//! Real scene-target input: locked click, drag/cancel, multi-touch, later and fight.
use bevy::{
    camera::RenderTarget,
    input::{
        ButtonState,
        mouse::MouseButtonInput,
        touch::{TouchInput, TouchPhase},
    },
    prelude::*,
    window::WindowRef,
    window::{CursorMoved, PrimaryWindow},
};
use rogue_commander::game::{
    ActiveSkill, AdventureBossState, AttackClock, Autoplay, CampArrival, ControlScheme,
    EncounterBoss, GameSession, PlayerBuild, RecruitmentProgress, SelectedHero, SpawnDirector,
};
#[derive(Resource, Default)]
struct Probe {
    frame: u32,
    point: Vec2,
    mouse_down: bool,
    targets: Vec<(Entity, RenderTarget)>,
}
pub fn configure(world: &mut World) {
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SelectedHero>().locked = true;
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
    world
        .resource_mut::<super::CaptureControls>()
        .suppress_drafts = true;
    world
        .resource_mut::<RecruitmentProgress>()
        .claimed_milestones = 7;
}
pub fn install(app: &mut bevy::app::SubApp) {
    app.world_mut().resource_mut::<PlayerBuild>().skills.fill(0);
    app.world_mut().resource_mut::<AttackClock>().0.pause();
    app.world_mut().resource_mut::<ActiveSkill>().remaining = 10000.;
    app.init_resource::<Probe>()
        .add_systems(First, drive)
        .add_systems(PreUpdate, restore_targets.after(bevy::ui::UiSystems::Focus))
        .add_systems(Last, audit);
}
fn target(world: &mut World, name: &str) -> Vec2 {
    let scale = world
        .query_filtered::<&Window, With<PrimaryWindow>>()
        .single(world)
        .unwrap()
        .scale_factor();
    let (node, transform) = world
        .query::<(&Name, &ComputedNode, &UiGlobalTransform)>()
        .iter(world)
        .find(|(n, _, _)| n.as_str() == name)
        .map(|(_, n, t)| (*n, *t))
        .expect("real scene/control target");
    assert!(
        node.size().min_element() > 0.,
        "target has real visible layout: {name}"
    );
    if name.starts_with("Adventure camp entry") {
        // Tap the exposed camp ground when the leader is partly behind the HUD.
        let blockers: Vec<_> = world
            .query::<(
                Option<&Name>,
                Has<Button>,
                &ComputedNode,
                &UiGlobalTransform,
                &InheritedVisibility,
            )>()
            .iter(world)
            .filter(|(n, button, _, _, v)| {
                v.get()
                    && !n.is_some_and(|n| n.as_str().starts_with("Adventure camp entry"))
                    && (*button
                        || n.is_some_and(|n| {
                            matches!(
                                n.as_str(),
                                "Battle status" | "Battle speed" | "XP readout" | "Trial drawer"
                            )
                        }))
            })
            .map(|(_, _, n, t, _)| (*n, *t))
            .collect();
        for offset in [
            Vec2::ZERO,
            Vec2::new(0., 0.4),
            Vec2::new(-0.4, 0.4),
            Vec2::new(0.4, 0.4),
            Vec2::new(-0.4, 0.),
            Vec2::new(0.4, 0.),
        ] {
            let point = transform.translation + node.size() * offset;
            if !blockers.iter().any(|(n, t)| n.contains_point(*t, point)) {
                println!("[capture] visible camp tap {name} at {:?}", point / scale);
                return point / scale;
            }
        }
        panic!("camp must have an exposed scene target: {name}");
    }
    transform.translation / scale
}
fn touch(world: &mut World, window: Entity, id: u64, phase: TouchPhase, position: Vec2) {
    world.write_message(TouchInput {
        window,
        id,
        phase,
        position,
        force: None,
    });
}
fn drive(world: &mut World) {
    let frame = world.resource::<Probe>().frame + 1;
    world.resource_mut::<Probe>().frame = frame;
    let window = world
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(world)
        .unwrap();
    let targets: Vec<_> = world
        .query_filtered::<(Entity, &RenderTarget), With<Camera>>()
        .iter(world)
        .map(|(e, t)| (e, t.clone()))
        .collect();
    for (e, _) in &targets {
        *world.get_mut::<RenderTarget>(*e).unwrap() =
            RenderTarget::Window(WindowRef::Entity(window));
    }
    world.resource_mut::<Probe>().targets = targets;
    // Virtual renderer has an offscreen target; UI focus must still receive real
    // pointer coordinates, as the other capture input probes do.
    if world.resource::<Probe>().mouse_down {
        world.write_message(MouseButtonInput {
            window,
            button: MouseButton::Left,
            state: ButtonState::Released,
        });
        world.resource_mut::<Probe>().mouse_down = false;
    }
    if frame == 30 {
        world.resource_mut::<GameSession>().elapsed = 481.;
    }
    let click = match frame {
        12 => Some("Adventure camp entry 1"),
        80 => Some("Encounter boss close"),
        100 => Some("Encounter boss fight"),
        _ => None,
    };
    if let Some(name) = click {
        let p = target(world, name);
        world
            .get_mut::<Window>(window)
            .unwrap()
            .set_cursor_position(Some(p));
        world.write_message(CursorMoved {
            window,
            position: p,
            delta: None,
        });
        world.write_message(MouseButtonInput {
            window,
            button: MouseButton::Left,
            state: ButtonState::Pressed,
        });
        world.resource_mut::<Probe>().mouse_down = true;
    }
    if [40, 50, 61, 70, 90].contains(&frame) {
        let p = target(world, "Adventure camp entry 1");
        world.resource_mut::<Probe>().point = p;
        touch(world, window, 7, TouchPhase::Started, p);
    }
    let p = world.resource::<Probe>().point;
    match frame {
        41 => touch(world, window, 7, TouchPhase::Moved, p + Vec2::X * 35.),
        42 => touch(world, window, 7, TouchPhase::Moved, p),
        43 | 62 | 71 | 91 => touch(world, window, 7, TouchPhase::Ended, p),
        51 => touch(world, window, 7, TouchPhase::Canceled, p),
        60 => {
            let p = target(world, "Movement pad");
            touch(world, window, 9, TouchPhase::Started, p);
        }
        63 => {
            let p = target(world, "Movement pad");
            touch(world, window, 9, TouchPhase::Ended, p);
        }
        _ => {}
    }
}
fn restore_targets(world: &mut World) {
    let targets = std::mem::take(&mut world.resource_mut::<Probe>().targets);
    for (e, target) in targets {
        *world.get_mut::<RenderTarget>(e).unwrap() = target;
    }
}
fn audit(world: &mut World) {
    let frame = world.resource::<Probe>().frame;
    let state = world.resource::<AdventureBossState>();
    match frame {
        16 | 46 | 54 | 66 => {
            assert!(
                state.pending.is_none(),
                "locked, dragged, cancelled or joystick touch must not open camp (frame {frame})"
            );
            println!("[capture] PASS camp rejected gesture frame={frame}");
        }
        75 | 95 => {
            assert_eq!(state.pending, Some(1), "tap the actual camp to open");
            println!("[capture] PASS camp scene tap opens preview frame={frame}");
        }
        85 => {
            assert!(state.pending.is_none());
            assert_eq!(state.resolved, 0);
            assert!(!world.resource::<GameSession>().paused);
            println!("[capture] PASS later preserves camp");
        }
        105 => {
            let active = state.active.expect("fight started");
            assert!(world.get::<CampArrival>(active).is_some());
            println!("[capture] PASS leader jumps from camp");
        }
        150 => {
            let active = state.active.expect("fight active");
            assert!(world.get::<EncounterBoss>(active).is_some());
            assert!(world.get::<CampArrival>(active).is_none());
            println!("[capture] PASS camp interaction end-to-end");
        }
        _ => {}
    }
    if frame == 10 {
        // Every target follows its scene footprint, not a 40px dock at right.
        for (name, node) in world.query::<(&Name, &ComputedNode)>().iter(world) {
            if name.as_str() == "Adventure camp entry 1" {
                assert!(node.size().x > 48.);
            }
        }
    }
    if frame >= 45 {
        assert!(world.resource::<ControlScheme>().touch());
    }
}
