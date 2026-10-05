//! Uses real opening/hover/immediate-click buttons. Only level and fruit awards are
//! fixture-controlled; this does not claim to simulate earning forty levels.
use bevy::prelude::*;
use rogue_commander::game::{
    Autoplay, BondCardId, GameSession, HeroKind, PlayerBuild, RecruitmentProgress,
    RecruitmentSource, SelectedHero,
};

#[derive(Resource)]
struct RecruitmentCapture {
    scenario: String,
    frame: u32,
    old: Option<HeroKind>,
    skills: [u8; rogue_commander::game::SkillId::COUNT],
    cards: [bool; BondCardId::COUNT],
}

pub fn configure(world: &mut World, scenario: &str) {
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SelectedHero>().locked = false;
    let mut session = world.resource_mut::<GameSession>();
    session.level = 1;
    session.skill_points = 0;
    session.auto_bond_draws = false;
    world.insert_resource(RecruitmentCapture {
        scenario: scenario.into(),
        frame: 0,
        old: None,
        skills: [0; rogue_commander::game::SkillId::COUNT],
        cards: [false; BondCardId::COUNT],
    });
}

pub fn install(app: &mut bevy::app::SubApp) {
    app.add_systems(PreUpdate, exercise.after(bevy::ui::UiSystems::Focus));
}

#[allow(clippy::too_many_arguments)]
fn exercise(
    mut capture: ResMut<RecruitmentCapture>,
    mut buttons: Query<(&Name, &mut Interaction), With<Button>>,
    selected: Res<SelectedHero>,
    mut session: ResMut<GameSession>,
    mut recruitment: ResMut<RecruitmentProgress>,
    build: Res<PlayerBuild>,
    layout: Query<(&Name, &ComputedNode, &UiGlobalTransform), With<Button>>,
    windows: Query<&Window>,
) {
    capture.frame += 1;
    let frame = capture.frame;
    let base_only = capture.scenario == "recruit-base";
    let fruit = capture.scenario.contains("fruit");
    let level = if capture.scenario.contains("40") {
        40
    } else if capture.scenario.contains("20") {
        20
    } else {
        10
    };
    if frame == 15 {
        assert!(!selected.locked && session.paused);
        assert_eq!(
            buttons
                .iter()
                .filter(|(name, _)| name.as_str().starts_with("Commander card "))
                .count(),
            3
        );
        println!("[recruit capture] three basic opening offers; no commitment before click");
    }
    if frame == 35 && !base_only {
        assert!(selected.locked);
        capture.old = Some(selected.kind);
        capture.skills = build.skills;
        capture.cards = build.owned_bond_cards;
        if fruit {
            recruitment.fruit_count = 1;
        } else {
            recruitment.claimed_milestones = match level {
                40 => 3,
                20 => 1,
                _ => 0,
            };
            session.level = level;
        }
        println!(
            "[recruit capture] harness awards {}",
            if fruit {
                "one fruit".to_string()
            } else {
                format!("level {level}; preceding milestones marked claimed")
            }
        );
    }
    let target = match frame {
        20 if !base_only => Some("Commander card 0"),
        42 if fruit => Some("Fruit recruit"),
        65 if !base_only => Some("Commander card 1"),
        _ => None,
    };
    if let Some(target) = target {
        let mut found = false;
        for (name, mut interaction) in &mut buttons {
            if name.as_str() == target {
                *interaction = if frame == 65 && !capture.scenario.ends_with("-confirm") {
                    Interaction::Hovered
                } else {
                    Interaction::Pressed
                };
                found = true;
            }
        }
        assert!(found, "missing real recruitment button {target}");
    }
    if frame == 90 && !base_only && !capture.scenario.ends_with("-confirm") {
        assert!(session.paused);
        assert_eq!(
            Some(selected.kind),
            capture.old,
            "preview cannot replace hero"
        );
        assert_eq!(
            recruitment.pending,
            Some(if fruit {
                RecruitmentSource::Fruit
            } else {
                RecruitmentSource::Milestone(level)
            })
        );
        if fruit {
            assert_eq!(recruitment.fruit_count, 1, "preview cannot consume fruit");
        }
        assert_eq!(
            buttons
                .iter()
                .filter(|(name, _)| name.as_str().starts_with("Commander card "))
                .count(),
            3
        );
        assert_eq!(capture.skills, build.skills);
        assert_eq!(capture.cards, build.owned_bond_cards);
        let window = windows.single().unwrap();
        let viewport = Vec2::new(
            window.physical_width() as f32,
            window.physical_height() as f32,
        );
        for (name, node, transform) in &layout {
            if !name.as_str().starts_with("Commander ") || node.size().min_element() == 0.0 {
                continue;
            }
            let min = transform.translation - node.size() * 0.5;
            let max = transform.translation + node.size() * 0.5;
            assert!(
                min.cmpge(Vec2::splat(-1.)).all() && max.cmple(viewport + Vec2::ONE).all(),
                "{name} outside viewport"
            );
        }
        println!(
            "[recruit capture] three later offers fit; preview preserves hero, bank and build"
        );
    }
    if frame == 140 && capture.scenario.ends_with("-confirm") {
        assert_ne!(Some(selected.kind), capture.old);
        assert!(recruitment.pending.is_none());
        assert!(
            recruitment.absorbed_strength > 0.
                && recruitment.absorbed_agility > 0.
                && recruitment.absorbed_intelligence > 0.
        );
        assert!(
            capture
                .skills
                .iter()
                .zip(build.skills)
                .all(|(old, new)| new >= *old)
        );
        assert_eq!(capture.cards, build.owned_bond_cards);
        if fruit {
            assert_eq!(recruitment.fruit_count, 0);
        } else {
            assert_ne!(
                recruitment.claimed_milestones
                    & match level {
                        40 => 4,
                        20 => 2,
                        _ => 1,
                    },
                0
            );
        }
        println!(
            "[recruit capture] confirmed replacement, absorbed innate attributes, retained learned skills/bonds; token claimed once"
        );
    }
}
