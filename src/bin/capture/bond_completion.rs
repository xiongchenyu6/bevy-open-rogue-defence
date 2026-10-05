//! Explicit prerequisites fixture; the final draw and confirmation use actual UI systems.
use bevy::prelude::*;
use rogue_commander::game::{
    Autoplay, BondDrawButton, BondId, DodgeButton, DraftChoice, DraftState, GameSession,
    PlayerBuild, RecruitmentProgress, SelectedHero, SkillDrawButton, available_bond_cards,
};

#[derive(Resource)]
struct CompletionCapture {
    frame: u32,
    target: BondId,
}

pub fn configure(world: &mut World, scenario: &str) {
    let target = match scenario {
        "bond-completion" => BondId::Journey,
        "bond-completion-fruit" => BondId::Warpath,
        _ => panic!("unknown completion fixture: {scenario}"),
    };
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SelectedHero>().locked = true;
    {
        let mut session = world.resource_mut::<GameSession>();
        session.elapsed = 20.0;
        session.wood = 60;
        session.auto_bond_draws = false;
    }
    let mut build = world.resource_mut::<PlayerBuild>();
    build.owned_bond_cards.fill(false);
    for family in BondId::ALL {
        let mut ancestor = Some(family);
        let mut descendant = false;
        while let Some(id) = ancestor {
            if id == target {
                descendant = true;
                break;
            }
            ancestor = id.definition().prerequisite;
        }
        if !descendant {
            for card in family.definition().members {
                build.owned_bond_cards[card.index()] = true;
            }
        }
    }
    let members = target.definition().members;
    for card in &members[..members.len() - 1] {
        build.owned_bond_cards[card.index()] = true;
    }
    assert_eq!(available_bond_cards(&build), vec![*members.last().unwrap()]);
    assert_eq!(build.active_bond_count(), 1);
    world.resource_mut::<RecruitmentProgress>().fruit_count = 0;
    world.insert_resource(CompletionCapture { frame: 0, target });
    println!(
        "[capture] {scenario}: explicit pre-owned prerequisites; real final-card draw and confirm"
    );
}

pub fn install(app: &mut bevy::app::SubApp) {
    app.add_systems(PreUpdate, exercise.after(bevy::ui::UiSystems::Focus));
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn exercise(
    mut state: ResMut<CompletionCapture>,
    mut buttons: Query<(Option<&Name>, &mut Interaction, Has<BondDrawButton>), With<Button>>,
    draft: Res<DraftState>,
    build: Res<PlayerBuild>,
    recruitment: Res<RecruitmentProgress>,
    session: Res<GameSession>,
    layout: Query<(&Name, &Node, &ComputedNode, &UiGlobalTransform)>,
    names: Query<(Entity, &Name)>,
    children: Query<&Children>,
    nodes: Query<(&ComputedNode, &UiGlobalTransform)>,
    windows: Query<&Window>,
    controls: Query<
        (
            Option<&Name>,
            &ComputedNode,
            &UiGlobalTransform,
            Has<SkillDrawButton>,
            Has<DodgeButton>,
        ),
        With<Button>,
    >,
) {
    state.frame += 1;
    let window = windows.single().unwrap();
    let viewport = Vec2::new(
        window.physical_width() as f32,
        window.physical_height() as f32,
    );
    let badge_name = format!("Bond badge {}", state.target.definition().id);
    let target = match state.frame {
        60 => Some("Build card 0"),
        240 | 260 => Some(badge_name.as_str()),
        _ => None,
    };
    let mut pressed = false;
    for (name, mut interaction, bond_draw) in &mut buttons {
        if (state.frame == 5 && bond_draw)
            || target.is_some_and(|target| name.is_some_and(|name| name.as_str() == target))
        {
            *interaction = Interaction::Pressed;
            pressed = true;
        } else if matches!(state.frame, 6 | 31 | 61 | 241 | 261) {
            *interaction = Interaction::None;
        }
    }
    if state.frame == 5 || target.is_some() {
        assert!(pressed, "completion capture action missing: {target:?}");
    }
    if state.frame == 50 {
        assert!(session.paused && draft.root.is_some());
        assert_eq!(
            draft.choices,
            vec![DraftChoice::Bond(
                *state.target.definition().members.last().unwrap()
            )]
        );
        assert!(!build.bond_complete(state.target));
        assert!(
            !names
                .iter()
                .any(|(_, name)| name.as_str() == "Bond completion celebration")
        );
        println!("[capture] final-card preview does not complete family or trigger celebration");
    }
    if matches!(state.frame, 250 | 265) {
        let (_, node, computed, transform) = layout
            .iter()
            .find(|(name, _, _, _)| name.as_str() == "Bond detail tooltip")
            .expect("bond detail tooltip root");
        if state.frame == 250 {
            assert_ne!(
                node.display,
                Display::None,
                "touch press pins completed bond details after release"
            );
            assert!(computed.size().min_element() > 0.0);
            assert_fits("pinned bond detail tooltip", computed, transform, viewport);
            let (root, _) = names
                .iter()
                .find(|(_, name)| name.as_str() == "Bond detail tooltip")
                .unwrap();
            for descendant in children.iter_descendants(root) {
                if let Ok((child, transform)) = nodes.get(descendant)
                    && child.size().min_element() > 0.0
                {
                    assert_fits("pinned tooltip content", child, transform, viewport);
                }
            }
            println!(
                "[capture] touch-pinned completion details remain visible after release and fit viewport"
            );
        } else {
            assert_eq!(
                node.display,
                Display::None,
                "second tap closes pinned bond details"
            );
            println!("[capture] second badge tap closes completion details");
        }
    }
    if matches!(state.frame, 90 | 265) {
        assert!(build.bond_complete(state.target));
        assert_eq!(
            build.active_bond_count(),
            0,
            "completed family releases slot"
        );
        assert!(draft.root.is_none() && !session.paused);
        assert_eq!(
            recruitment.fruit_count,
            u32::from(state.target == BondId::Warpath)
        );
        if state.target == BondId::Journey {
            let pool = available_bond_cards(&build);
            for child in [BondId::Strength, BondId::Agility, BondId::Intelligence] {
                assert!(pool.iter().any(|card| card.family() == child));
            }
        }
        let celebration = layout
            .iter()
            .find(|(name, _, _, _)| name.as_str() == "Bond completion celebration");
        let mut action_count = 0;
        let mut speed_count = 0;
        for (name, node, transform, skill, dodge) in &controls {
            let speed = name.is_some_and(|name| name.as_str().starts_with("Speed "));
            if !(skill || dodge || speed) {
                continue;
            }
            let label = if skill {
                "Skill draw"
            } else if dodge {
                "Dodge"
            } else {
                name.unwrap().as_str()
            };
            assert_fits(label, node, transform, viewport);
            action_count += usize::from(skill || dodge);
            speed_count += usize::from(speed);
            if let Some((_, _, banner, banner_transform)) = celebration {
                let delta = (transform.translation - banner_transform.translation).abs();
                let half_sizes = (node.size() + banner.size()) * 0.5;
                assert!(
                    delta.x >= half_sizes.x || delta.y >= half_sizes.y,
                    "celebration overlaps {label}"
                );
            }
        }
        assert_eq!(action_count, 2, "skill and dodge actions present");
        assert_eq!(speed_count, 3, "all three speed controls present");
        println!(
            "[capture] bottom actions and speed controls fit viewport; celebration does not cover controls"
        );
        if state.frame == 90 {
            let (_, _, node, transform) = celebration.expect("completion celebration visible");
            let window = windows.single().unwrap();
            let viewport = Vec2::new(
                window.physical_width() as f32,
                window.physical_height() as f32,
            );
            let min = transform.translation - node.size() * 0.5;
            let max = transform.translation + node.size() * 0.5;
            assert!(node.size().min_element() > 0.0);
            assert!(
                min.cmpge(Vec2::splat(-1.0)).all() && max.cmple(viewport + Vec2::ONE).all(),
                "celebration outside viewport: {min:?}..{max:?}"
            );
            let (root, _) = names
                .iter()
                .find(|(_, name)| name.as_str() == "Bond completion celebration")
                .unwrap();
            for descendant in children.iter_descendants(root) {
                let Ok((child, transform)) = nodes.get(descendant) else {
                    continue;
                };
                if child.size().min_element() == 0.0 {
                    continue;
                }
                let min = transform.translation - child.size() * 0.5;
                let max = transform.translation + child.size() * 0.5;
                assert!(
                    min.cmpge(Vec2::splat(-1.0)).all() && max.cmple(viewport + Vec2::ONE).all(),
                    "celebration child outside viewport: {min:?}..{max:?}"
                );
            }
            println!(
                "[capture] actual confirmation completed {:?}: celebration fits viewport; pool/reward/slot verified",
                state.target
            );
        } else {
            assert!(
                celebration.is_none(),
                "celebration expired and did not repeat"
            );
            let badge_name = format!("Bond badge {}", state.target.definition().id);
            let (_, node, computed, _) = layout
                .iter()
                .find(|(name, _, _, _)| name.as_str() == badge_name)
                .expect("persistent completed-family badge");
            assert_ne!(node.display, Display::None);
            assert!(computed.size().min_element() > 0.0);
            println!(
                "[capture] celebration expired; completed badge retained; fruit reward remains exactly once"
            );
        }
    }
}

fn assert_fits(label: &str, node: &ComputedNode, transform: &UiGlobalTransform, viewport: Vec2) {
    let min = transform.translation - node.size() * 0.5;
    let max = transform.translation + node.size() * 0.5;
    assert!(
        node.size().min_element() > 0.0,
        "{label} has no visible area"
    );
    assert!(
        min.cmpge(Vec2::splat(-1.0)).all() && max.cmple(viewport + Vec2::ONE).all(),
        "{label} outside viewport: {min:?}..{max:?}"
    );
}
