//! Real mouse choices settle immediately, including consecutive hands and held clicks.
use bevy::prelude::*;
use bevy::{
    camera::RenderTarget,
    input::{ButtonState, mouse::MouseButtonInput},
    window::{CursorMoved, WindowRef},
};
use rogue_commander::game::{
    ActiveSkill, AttackClock, Autoplay, BondCardId, BondDrawButton, DraftChoice, DraftState,
    GameSession, PlayerBuild, SelectedHero, SkillDrawButton, SkillEconomy, SpawnDirector,
};
#[derive(Resource, Default)]
struct Pointer {
    saved_targets: Vec<(Entity, RenderTarget)>,
    down: bool,
    release_at: u32,
}
#[derive(Resource)]
struct Probe {
    frame: u32,
    bond: bool,
    before: u32,
    round: u32,
    payment: u32,
}
pub fn configure(world: &mut World, scenario: &str) {
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SelectedHero>().locked = true;
    world.resource_mut::<SpawnDirector>().enabled = false;
    world
        .resource_mut::<super::CaptureControls>()
        .suppress_drafts = false;
    let mut session = world.resource_mut::<GameSession>();
    session.elapsed = 20.;
    session.auto_bond_draws = false;
    session.wood = 100;
    session.skill_points = 3;
    world.insert_resource(Probe {
        frame: 0,
        bond: scenario == "immediate-bond",
        before: 0,
        round: 0,
        payment: 0,
    });
}
pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    world.resource_mut::<AttackClock>().0.pause();
    world.resource_mut::<ActiveSkill>().remaining = 10000.;
    world.resource_mut::<GameSession>().skill_points = 3;
    world.resource_mut::<SkillEconomy>().first_free_granted = true;
    app.init_resource::<Pointer>()
        .add_systems(First, input)
        .add_systems(PreUpdate, restore.after(bevy::ui::UiSystems::Focus))
        .add_systems(Last, audit);
}
fn input(world: &mut World) {
    world.resource_mut::<Probe>().frame += 1;
    let frame = world.resource::<Probe>().frame;
    let bond = world.resource::<Probe>().bond;
    let label = if frame == 10 {
        if bond {
            Some("__actual_bond_draw".to_owned())
        } else {
            world
                .query::<(&Name, &SkillDrawButton)>()
                .iter(world)
                .next()
                .map(|(name, _)| name.as_str().to_owned())
        }
    } else if bond && frame == 205 {
        Some("Build later".to_owned())
    } else if matches!(frame, 35 | 105 | 175) {
        Some(format!(
            "Build card {}",
            match frame {
                35 => 2,
                105 => 1,
                _ => 0,
            }
        ))
    } else {
        None
    };
    let window = world
        .query::<Entity>()
        .iter(world)
        .find(|entity| world.get::<Window>(*entity).is_some())
        .expect("virtual capture window");
    if world.resource::<Pointer>().down && frame >= world.resource::<Pointer>().release_at {
        world.write_message(MouseButtonInput {
            button: MouseButton::Left,
            state: ButtonState::Released,
            window,
        });
        world.resource_mut::<Pointer>().down = false;
    }
    if world.resource::<Pointer>().down && label.is_none() {
        let targets: Vec<_> = world
            .query_filtered::<(Entity, &RenderTarget), With<Camera>>()
            .iter(world)
            .map(|(entity, target)| (entity, target.clone()))
            .collect();
        for (entity, _) in &targets {
            *world.get_mut::<RenderTarget>(*entity).unwrap() =
                RenderTarget::Window(WindowRef::Entity(window));
        }
        world.resource_mut::<Pointer>().saved_targets = targets;
    }
    if let Some(label) = label {
        let position = world
            .query_filtered::<(
                Option<&Name>,
                Option<&BondDrawButton>,
                &ComputedNode,
                &UiGlobalTransform,
            ), With<Button>>()
            .iter(world)
            .find(|(name, bond, _, _)| {
                if label == "__actual_bond_draw" {
                    bond.is_some()
                } else {
                    name.is_some_and(|name| name.as_str() == label)
                }
            })
            .map(|(_, _, node, t)| {
                assert!(
                    node.size().min_element() > 0.,
                    "actual button must have layout"
                );
                t.translation
            })
            .unwrap_or_else(|| panic!("missing actual three-choice control {label}"));
        let scale = world.get::<Window>(window).unwrap().scale_factor();
        let logical = position / scale;
        world
            .get_mut::<Window>(window)
            .unwrap()
            .set_cursor_position(Some(logical));
        world.write_message(CursorMoved {
            window,
            position: logical,
            delta: None,
        });
        world.write_message(MouseButtonInput {
            button: MouseButton::Left,
            state: ButtonState::Pressed,
            window,
        });
        let targets: Vec<_> = world
            .query_filtered::<(Entity, &RenderTarget), With<Camera>>()
            .iter(world)
            .map(|(entity, target)| (entity, target.clone()))
            .collect();
        for (entity, _) in &targets {
            *world.get_mut::<RenderTarget>(*entity).unwrap() =
                RenderTarget::Window(WindowRef::Entity(window));
        }
        world.resource_mut::<Pointer>().saved_targets = targets;
        world.resource_mut::<Pointer>().down = true;
        world.resource_mut::<Pointer>().release_at = frame + 4;
        println!("[capture] actual virtual-window pointer {label} at {position:?} frame={frame}");
    }
}
fn restore(world: &mut World) {
    for (entity, target) in std::mem::take(&mut world.resource_mut::<Pointer>().saved_targets) {
        if let Some(mut current) = world.get_mut::<RenderTarget>(entity) {
            *current = target;
        }
    }
}
fn count(world: &World) -> u32 {
    let p = world.resource::<Probe>();
    let b = world.resource::<PlayerBuild>();
    if p.bond {
        b.owned_bond_cards.iter().filter(|owned| **owned).count() as u32
    } else {
        b.skills.iter().map(|rank| u32::from(*rank)).sum()
    }
}
fn audit(world: &mut World) {
    let frame = world.resource::<Probe>().frame;
    let bond = world.resource::<Probe>().bond;
    if matches!(frame, 30 | 100 | 170) {
        let choices = world.resource::<DraftState>().choices.len();
        assert!(
            world.resource::<DraftState>().root.is_some() && world.resource::<GameSession>().paused
        );
        assert!(
            !world
                .query::<&Name>()
                .iter(world)
                .any(|name| name.as_str() == "Build confirm")
        );
        assert_eq!(
            choices,
            if bond {
                match frame {
                    30 => 3,
                    100 => 2,
                    _ => 1,
                }
            } else {
                3
            }
        );
        if bond {
            assert!(
                world
                    .resource::<DraftState>()
                    .choices
                    .iter()
                    .all(|choice| matches!(
                        choice,
                        DraftChoice::Bond(
                            BondCardId::JourneyStrength
                                | BondCardId::JourneyAgility
                                | BondCardId::JourneyIntelligence
                        )
                    )),
                "fixture choices must be the three authored Journey cards"
            );
        }
        world.resource_mut::<Probe>().before = count(world);
        world.resource_mut::<Probe>().payment = world.resource::<GameSession>().wood;
        println!("[capture] immediate-stage offered frame={frame}");
    }
    if matches!(frame, 36 | 106 | 176) {
        assert_eq!(
            count(world),
            world.resource::<Probe>().before + 1,
            "actual click settles once"
        );
        if !bond && frame == 176 {
            assert!(
                world.resource::<DraftState>().root.is_none()
                    && !world.resource::<GameSession>().paused
            );
        }
        world.resource_mut::<Probe>().round += 1;
        if bond {
            assert_eq!(
                world.resource::<GameSession>().wood,
                world.resource::<Probe>().payment + 20 + if frame == 176 { 100 } else { 0 }
                    - (5 + 5 * world.resource::<GameSession>().bond_draws),
                "Journey reward, completion bonus and exactly one automatically queued paid hand"
            );
        } else {
            assert_eq!(world.resource::<SkillEconomy>().paid_draws, 0);
        }
        println!("[capture] immediate-stage settled frame={frame}");
    }
    if matches!(frame, 45 | 115 | 185) {
        assert_eq!(
            count(world),
            world.resource::<Probe>().before + 1,
            "held native mouse cannot grant duplicate choice"
        );
        if bond {
            assert!(
                world.resource::<DraftState>().root.is_some(),
                "bond opens next hand without another draw click"
            );
        } else {
            let round = world.resource::<Probe>().round;
            assert_eq!(
                world.resource::<GameSession>().skill_points,
                if round < 3 { 2 - round } else { 0 }
            );
            assert_eq!(
                world.resource::<DraftState>().root.is_some(),
                round < 3,
                "queued free choice opens automatically without another draw"
            );
        }
    }
    if frame == 290 {
        assert_eq!(world.resource::<Probe>().round, 3);
        assert!(world.resource::<DraftState>().root.is_none());
        println!(
            "[capture] PASS immediate-{}: three real mouse choices2/1/0, one settlement/payment per hand, held mouse no duplicate",
            if bond { "bond" } else { "skill" }
        );
    }
}
