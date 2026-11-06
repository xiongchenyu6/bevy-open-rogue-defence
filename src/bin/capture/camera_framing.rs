//! Controlled legal-corner poses exercise the production camera and renderer.
//! This is framing evidence, not physical keyboard or normal-play acceptance.
use bevy::prelude::*;
use bevy::{
    input::touch::{TouchInput, TouchPhase},
    window::PrimaryWindow,
};
use rogue_commander::game::{
    ActiveSkill, AttackClock, ControlScheme, Hero, PlayerBuild, SpawnDirector,
};

#[derive(Resource, Default)]
struct Probe(u32);

#[derive(Resource)]
struct TouchProbe {
    window: Entity,
    position: Vec2,
}

pub fn install(app: &mut bevy::app::SubApp, touch: bool) {
    app.world_mut().resource_mut::<PlayerBuild>().skills.fill(0);
    app.world_mut().resource_mut::<ActiveSkill>().remaining = 10_000.;
    app.world_mut().resource_mut::<AttackClock>().0.pause();
    app.world_mut().resource_mut::<SpawnDirector>().enabled = false;
    if touch {
        let world = app.world_mut();
        let (window, size) = world
            .query_filtered::<(Entity, &Window), With<PrimaryWindow>>()
            .single(world)
            .map(|(entity, window)| (entity, Vec2::new(window.width(), window.height())))
            .unwrap();
        world.insert_resource(TouchProbe {
            window,
            position: size * 0.5,
        });
    }
    app.init_resource::<Probe>()
        .add_systems(PreUpdate, touch_input.before(bevy::input::InputSystems))
        .add_systems(PreUpdate, pose.after(bevy::input::InputSystems))
        .add_systems(Last, audit);
}

fn touch_input(
    probe: Res<Probe>,
    touch: Option<Res<TouchProbe>>,
    mut messages: MessageWriter<TouchInput>,
) {
    let Some(touch) = touch else {
        return;
    };
    let phase = match probe.0 {
        0 => TouchPhase::Started,
        1 => TouchPhase::Ended,
        _ => return,
    };
    messages.write(TouchInput {
        phase,
        position: touch.position,
        window: touch.window,
        force: None,
        id: 77,
    });
    println!("[capture] CAMERA_TOUCH frame={} phase={phase:?}", probe.0);
}

fn pose(mut probe: ResMut<Probe>, mut heroes: Query<&mut Transform, With<Hero>>) {
    probe.0 += 1;
    let position = match probe.0 {
        30 => Vec3::new(-10., 0., -12.),
        90 => Vec3::new(10., 0., -12.),
        150 => Vec3::new(10., 0., 12.),
        210 => Vec3::new(-10., 0., 12.),
        _ => return,
    };
    heroes.single_mut().unwrap().translation = position;
}

fn audit(
    probe: Res<Probe>,
    heroes: Query<&GlobalTransform, With<Hero>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    touch: Option<Res<TouchProbe>>,
    scheme: Res<ControlScheme>,
) {
    if ![59, 119, 179, 239].contains(&probe.0) {
        return;
    }
    if touch.is_some() {
        assert!(
            scheme.touch(),
            "real TouchInput messages must activate the actual touch HUD"
        );
    }
    if probe.0 == 59 {
        println!("[capture] CAMERA_INPUT_MODE actual={:?}", *scheme);
    }
    let body = heroes.single().unwrap().translation() + Vec3::Y * 1.2;
    let (camera, transform) = cameras.single().unwrap();
    let size = camera.logical_viewport_size().unwrap();
    let point = camera.world_to_viewport(transform, body).unwrap();
    assert!(
        point.x >= 20. && point.x <= size.x - 20. && point.y >= 20. && point.y <= size.y - 20.,
        "legal hero body center outside actual viewport: {body:?} -> {point:?}/{size:?}"
    );
    println!(
        "[capture] CAMERA_CORNER frame={} world={body:?} actual_pixel={point:?} viewport={size:?}",
        probe.0
    );
    if probe.0 == 239 {
        println!(
            "[capture] PASS camera-framing: four legal corners remain onscreen; inspect pixels for HUD/model occlusion; controlled poses only"
        );
    }
}
