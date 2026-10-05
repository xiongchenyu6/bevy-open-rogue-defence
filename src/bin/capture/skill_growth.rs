//! Grants ranks at known times; summons and feedback use the real game systems.
use super::CaptureControls;
use bevy::prelude::*;
use rogue_commander::game::{Autoplay, GameAssets, GameSession, PlayerBuild, SelectedHero};

#[derive(Resource, Default)]
struct GrowthCapture {
    frame: u32,
    skeleton: Option<Entity>,
}

pub fn configure(world: &mut World) {
    world.resource_mut::<SelectedHero>().locked = true;
    world.resource_mut::<CaptureControls>().suppress_drafts = true;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
}
pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    world.remove_resource::<Autoplay>();
    world.resource_mut::<PlayerBuild>().skills.fill(0);
    world.resource_mut::<PlayerBuild>().skills[4] = 1;
    let mut cameras = world.query_filtered::<(&mut Transform, &mut Projection), With<Camera3d>>();
    for (mut transform, mut projection) in cameras.iter_mut(world) {
        *transform =
            Transform::from_xyz(-8.6, 10.0, 9.0).looking_at(Vec3::new(-8.6, 0.0, 0.0), Vec3::Y);
        if let Projection::Orthographic(projection) = &mut *projection {
            projection.scaling_mode = bevy::camera::ScalingMode::FixedVertical {
                viewport_height: 8.0,
            };
        }
    }
    app.init_resource::<GrowthCapture>()
        .add_systems(PreUpdate, exercise);
}
fn exercise(
    mut state: ResMut<GrowthCapture>,
    mut build: ResMut<PlayerBuild>,
    assets: Res<GameAssets>,
    models: Query<(Entity, &WorldAssetRoot, &Transform)>,
    names: Query<&Name>,
) {
    state.frame += 1;
    if state.frame == 90 {
        build.skills[4] = 3;
        println!("[capture] upgrade Bone Sentry 1 -> 3");
    }
    if state.frame == 180 {
        build.skills[4] = 5;
        println!("[capture] upgrade Bone Sentry 3 -> 5");
    }
    if state.frame == 265 {
        assert!(
            !names
                .iter()
                .any(|name| name.as_str() == "Skill upgrade notice")
        );
        println!("[capture] upgrade notice expires after feedback window");
    }
    if state.frame == 60 {
        let (entity, _, transform) = models
            .iter()
            .find(|(_, model, _)| model.0 == assets.summon_scenes[0])
            .expect("skeleton spawned by real skill");
        assert!((transform.scale.x - 0.7).abs() < 0.005);
        state.skeleton = Some(entity);
        println!("[capture] level 1 skeleton scale=0.70");
    }
    if let Some(expected) = match state.frame {
        135 => Some(0.98),
        225 => Some(1.26),
        _ => None,
    } {
        let entity = state.skeleton.unwrap();
        let (_, _, transform) = models
            .get(entity)
            .expect("existing skeleton survives upgrade");
        assert!((transform.scale.x - expected).abs() < 0.005);
        assert!(
            names
                .iter()
                .any(|name| name.as_str() == "Skill upgrade notice")
        );
        println!("[capture] same skeleton grows to {expected}; upgrade notice visible");
    }
}
