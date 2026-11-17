//! Hold real keyboard/touch input against both road edges, including dodges.
use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
        touch::{TouchInput, TouchPhase},
    },
    prelude::*,
};
use rogue_commander::game::{
    ActiveSkill, AttackClock, Autoplay, DodgeState, GameSession, Hero, MoveIntent, PlayerBuild,
    SelectedHero, SpawnDirector,
};
#[derive(Resource, Default)]
struct Probe {
    frame: u32,
    pad: Vec2,
    first_edge: f32,
}
pub fn configure(world: &mut World) {
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SelectedHero>().locked = true;
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
    world
        .resource_mut::<super::CaptureControls>()
        .suppress_drafts = true;
}
pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    world.remove_resource::<Autoplay>();
    world.resource_mut::<PlayerBuild>().skills.fill(0);
    world.resource_mut::<AttackClock>().0.pause();
    world.resource_mut::<ActiveSkill>().remaining = 10000.;
    world
        .query_filtered::<&mut Transform, With<Hero>>()
        .single_mut(world)
        .unwrap()
        .translation = Vec3::ZERO;
    app.init_resource::<Probe>()
        .add_systems(PreUpdate, input.before(bevy::input::InputSystems))
        .add_systems(Last, verify);
}
fn point(world: &mut World, name: &str) -> Vec2 {
    world
        .query::<(&Name, &UiGlobalTransform)>()
        .iter(world)
        .find(|(n, _)| n.as_str() == name)
        .expect("named input control")
        .1
        .translation
}
fn touch(world: &mut World, window: Entity, id: u64, phase: TouchPhase, position: Vec2) {
    world.write_message(TouchInput {
        phase,
        position,
        window,
        id,
        force: None,
    });
}
fn input(world: &mut World) {
    world.resource_mut::<Probe>().frame += 1;
    let frame = world.resource::<Probe>().frame;
    let window = world.query::<(Entity, &Window)>().single(world).unwrap().0;
    let event = match frame {
        1 => Some((KeyCode::ArrowUp, Key::ArrowUp, ButtonState::Pressed)),
        171 | 371 => Some((KeyCode::ShiftLeft, Key::Shift, ButtonState::Pressed)),
        172 | 372 => Some((KeyCode::ShiftLeft, Key::Shift, ButtonState::Released)),
        190 => Some((KeyCode::ArrowUp, Key::ArrowUp, ButtonState::Released)),
        200 => Some((KeyCode::ArrowDown, Key::ArrowDown, ButtonState::Pressed)),
        390 => Some((KeyCode::ArrowDown, Key::ArrowDown, ButtonState::Released)),
        _ => None,
    };
    if let Some((key_code, logical_key, state)) = event {
        world.write_message(KeyboardInput {
            key_code,
            logical_key,
            state,
            text: None,
            repeat: false,
            window,
        });
    }
    match frame {
        420 => touch(
            world,
            window,
            90,
            TouchPhase::Started,
            Vec2::new(400., 180.),
        ),
        421 => touch(world, window, 90, TouchPhase::Ended, Vec2::new(400., 180.)),
        440 | 680 => {
            let pad = point(world, "Movement pad");
            world.resource_mut::<Probe>().pad = pad;
            touch(world, window, 91, TouchPhase::Started, pad);
        }
        441 | 681 => {
            let pad = world.resource::<Probe>().pad;
            touch(
                world,
                window,
                91,
                TouchPhase::Moved,
                pad + Vec2::Y * if frame == 441 { -60. } else { 60. },
            );
        }
        631 | 871 => {
            let p = point(world, "Dodge control");
            touch(world, window, 92, TouchPhase::Started, p);
        }
        632 | 872 => {
            let p = point(world, "Dodge control");
            touch(world, window, 92, TouchPhase::Ended, p);
        }
        660 | 900 => {
            let p = world.resource::<Probe>().pad;
            touch(world, window, 91, TouchPhase::Canceled, p);
        }
        _ => {}
    }
}
fn verify(world: &mut World) {
    let frame = world.resource::<Probe>().frame;
    let position = world
        .query_filtered::<&Transform, With<Hero>>()
        .single(world)
        .unwrap()
        .translation;
    assert!(
        position.z.abs() <= 4.4001,
        "hero left paved road at frame {frame}: {position:?}"
    );
    if [171, 371, 631, 871].contains(&frame) {
        assert!(
            world.resource::<DodgeState>().cooldown_remaining > 0.,
            "actual dodge must activate frame={frame} position={position:?} intent={:?} paused={} pressed={:?}",
            world.resource::<MoveIntent>().0,
            world.resource::<GameSession>().paused,
            world
                .resource::<ButtonInput<KeyCode>>()
                .get_pressed()
                .collect::<Vec<_>>()
        );
        println!("[capture] PASS real edge dodge frame={frame} position={position:?}");
    }
    if [180, 380, 650, 890].contains(&frame) {
        assert!(
            (position.z.abs() - 4.4).abs() < 0.001,
            "held input must reach road edge: {position:?}"
        );
        if frame == 180 {
            world.resource_mut::<Probe>().first_edge = position.z.signum();
        }
        let sign = world.resource::<Probe>().first_edge
            * if frame == 180 || frame == 650 {
                1.
            } else {
                -1.
            };
        assert_eq!(
            position.z.signum(),
            sign,
            "both road sides must be exercised"
        );
        println!("[capture] PASS held road edge frame={frame} position={position:?}");
    }
    if frame == 910 {
        assert_eq!(
            world.resource::<MoveIntent>().0,
            Vec2::ZERO,
            "cancelled touch must stop movement"
        );
        println!(
            "[capture] PASS road-boundary: keyboard/touch holds and four real dodges stay on paving; cancellation clears movement"
        );
    }
}
