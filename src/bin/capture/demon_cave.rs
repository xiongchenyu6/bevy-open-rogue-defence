//! Production-camera proof of the demon entrance, quiet and during real waves.
use bevy::prelude::*;
use rogue_commander::game::{
    ActiveSkill, AttackClock, Autoplay, CampaignMenu, Enemy, EnemyArchetype, GameAssets,
    GameSession, PlayerBuild, SelectedHero, SpawnDirector,
};
#[derive(Resource, Default)]
struct CaveCapture {
    frame: u32,
    saw_enemy: bool,
    saw_pulse: bool,
}
pub fn configure(world: &mut World) {
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SelectedHero>().locked = true;
    world
        .resource_mut::<super::CaptureControls>()
        .suppress_drafts = true;
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
}
pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<PlayerBuild>().skills.fill(0);
    world.resource_mut::<ActiveSkill>().remaining = 10_000.;
    world.resource_mut::<AttackClock>().0.pause();
    app.init_resource::<CaveCapture>()
        .add_systems(PreUpdate, exercise);
}
fn exercise(
    mut state: ResMut<CaveCapture>,
    mut director: ResMut<SpawnDirector>,
    enemies: Query<(&Enemy, &Transform)>,
    names: Query<&Name>,
) {
    state.frame += 1;
    let frame = state.frame;
    state.saw_enemy |= enemies
        .iter()
        .any(|(enemy, transform)| enemy.hp > 0. && transform.translation.x > 10.);
    state.saw_pulse |= names
        .iter()
        .any(|name| name.as_str() == "Demon den emergence pulse");
    if frame == 30 {
        assert!(
            enemies.is_empty(),
            "quiet entrance scene has no hand-spawned or real enemies"
        );
        let count = |label: &str| names.iter().filter(|name| name.as_str() == label).count();
        assert_eq!(
            count("Demon den mouth"),
            1,
            "one giant dark mouth, rather than scattered portal circles"
        );
        assert!(count("Demon den rock arch") >= 10);
        assert!(count("Demon den fang") >= 6);
        assert!(count("Demon den breathing mist") >= 6);
        println!(
            "[capture] PASS demon cave quiet: one giant mouth, stone arch, solid fangs and breathing mist, production camera"
        );
    }
    if frame == 60 {
        director.enabled = true;
    }
    if frame == 295 {
        assert!(
            state.saw_enemy,
            "actual SpawnDirector wave must emerge at the den"
        );
        assert!(
            state.saw_pulse,
            "actual enemy addition must trigger emergence pulse"
        );
        println!(
            "[capture] PASS demon-cave: quiet cave, real wave enemies and actual emergence pulse; production camera unchanged"
        );
    }
}

// Candidate preview only: clone assets and never alter the shipped entrance.
const MODEL: &str = "models/environment_demon_cavern_preview.glb#Scene0";

#[derive(Resource)]
struct Preview {
    root: Entity,
    scene: Handle<WorldAsset>,
    pose: Transform,
    removed: Vec<Entity>,
    frame: u32,
    actors: Vec<Entity>,
    camera: &'static str,
}

pub fn install_preview(app: &mut bevy::app::SubApp, scenario: &str) {
    assert!(matches!(
        scenario,
        "den-preview"
            | "den-preview-reframed"
            | "den-preview-front"
            | "den-preview-wide"
            | "den-preview-front-board"
            | "den-preview-frontal-c"
    ));
    let world = app.world_mut();
    assert!(
        !world.resource::<CampaignMenu>().0,
        "preview requires the actual arena, not its chapter menu"
    );
    assert!(
        !world.resource::<GameSession>().paused,
        "preview must enter active battle before installing actors"
    );
    world.remove_resource::<Autoplay>();
    world.resource_mut::<PlayerBuild>().skills.fill(0);
    world.resource_mut::<ActiveSkill>().remaining = 10_000.0;
    world.resource_mut::<AttackClock>().0.reset();
    world.resource_mut::<AttackClock>().0.pause();

    let old: Vec<_> = world
        .query::<(Entity, &Name, &Transform)>()
        .iter(world)
        .filter(|(_, name, _)| name.as_str() == "Demon den")
        .map(|(entity, _, pose)| (entity, *pose))
        .collect();
    assert_eq!(old.len(), 1, "replace exactly the actual arena den");
    let (old_root, pose) = old[0];
    let removed = descendants(world, old_root);
    world.entity_mut(old_root).despawn();
    let scene = world.resource::<AssetServer>().load(MODEL);
    let root = world
        .spawn((
            Name::new("Candidate demon den preview"),
            WorldAssetRoot(scene.clone()),
            pose,
        ))
        .id();
    let (camera, camera_proposal) = match scenario {
        "den-preview-front" => (
            "front-side proposal",
            Some((Vec3::new(-6.0, 24.0, 21.0), Vec3::new(2.0, 0.0, 0.0), 24.0)),
        ),
        "den-preview-reframed" => (
            "shifted original direction proposal",
            Some((Vec3::new(10.0, 24.0, 21.0), Vec3::new(2.0, 0.0, 0.0), 24.0)),
        ),
        "den-preview-wide" => (
            "whole battlefield proposal",
            Some((Vec3::new(15.0, 24.0, 21.0), Vec3::new(7.0, 0.0, 0.0), 26.0)),
        ),
        "den-preview-front-board" => (
            "entrance-facing battlefield proposal",
            Some((Vec3::new(-9.0, 25.2, 21.0), Vec3::new(5.0, 1.2, 0.0), 28.0)),
        ),
        "den-preview-frontal-c" => (
            "frontal cave and battlefield proposal C",
            Some((Vec3::new(-20.0, 25.2, 10.0), Vec3::new(4.0, 1.2, 0.0), 28.0)),
        ),
        _ => ("production", None),
    };
    if let Some((eye, focus, height)) = camera_proposal {
        for (mut transform, mut projection) in world
            .query_filtered::<(&mut Transform, &mut Projection), With<Camera3d>>()
            .iter_mut(world)
        {
            *transform = Transform::from_translation(eye).looking_at(focus, Vec3::Y);
            if let Projection::Orthographic(projection) = &mut *projection {
                projection.scaling_mode = bevy::camera::ScalingMode::FixedVertical {
                    viewport_height: height,
                };
            }
        }
        println!(
            "[capture] den camera: eye={eye:?} center_x={} vertical={height} target_y={} target_z={}",
            focus.x, focus.y, focus.z
        );
    }
    world.insert_resource(Preview {
        root,
        scene,
        pose,
        removed,
        frame: 0,
        actors: Vec::new(),
        camera,
    });
    app.add_systems(Last, observe);
    println!(
        "[capture] den candidate: real arena and lighting, old den removed; fixture actors, not earned gameplay"
    );
}

fn descendants(world: &World, root: Entity) -> Vec<Entity> {
    let mut pending = vec![root];
    let mut result = Vec::new();
    while let Some(entity) = pending.pop() {
        result.push(entity);
        if let Some(children) = world.get::<Children>(entity) {
            pending.extend(children.iter());
        }
    }
    result
}

fn observe(world: &mut World) {
    let mut preview = world.remove_resource::<Preview>().unwrap();
    preview.frame += 1;
    let loaded = world
        .resource::<AssetServer>()
        .is_loaded_with_dependencies(preview.scene.id());
    let meshes = descendants(world, preview.root)
        .into_iter()
        .filter(|entity| world.get::<Mesh3d>(*entity).is_some())
        .count();
    if preview.frame >= 100 && preview.actors.is_empty() && loaded && meshes > 0 {
        let scenes = world.resource::<GameAssets>().enemy_scenes.clone();
        // Source-audited maximum spawn/merge scales. Three actual animated models
        // sample both birth-band edges and the tallest central colossus. The
        // separate triangle/animated-envelope audit covers the remaining models.
        for (index, z, scale) in [(3, -4.25, 1.85), (6, 0.0, 1.91808), (11, 4.25, 1.8615212)] {
            let entity = world
                .spawn((
                    Name::new(format!("Den preview maximum-scale actor {index}")),
                    WorldAssetRoot(scenes[index].clone()),
                    Transform::from_xyz(16.4, 0.0, z).with_scale(Vec3::splat(scale)),
                    EnemyArchetype(index as u8),
                    Enemy {
                        hp: 100_000.0,
                        max_hp: 100_000.0,
                        speed: 1.2,
                        damage: 0.0,
                        xp: 0,
                        wood: 0,
                        gold: 0,
                        kill_credit: 0,
                        elite: false,
                    },
                ))
                .id();
            preview.actors.push(entity);
        }
        println!(
            "[capture] den preview loaded: imported_meshes={meshes}; three maximum-scale fixture actors start actual production movement"
        );
    }
    if preview.frame == 350 {
        assert!(loaded && meshes > 0, "candidate must actually import");
        assert_eq!(world.get::<Transform>(preview.root), Some(&preview.pose));
        assert!(
            preview
                .removed
                .iter()
                .all(|entity| !world.entities().contains(*entity)),
            "the entire original den subtree must be absent"
        );
        assert_eq!(preview.actors.len(), 3);
        for entity in &preview.actors {
            let position = world.get::<Transform>(*entity).unwrap().translation;
            assert!(
                position.x < 10.0,
                "actual movement must leave the entrance: {position:?}"
            );
            assert!(world.get::<Enemy>(*entity).unwrap().hp > 0.0);
        }
        assert!(
            !world
                .query::<&Name>()
                .iter(world)
                .any(|name| name.as_str() == "Demon den")
        );
        println!(
            "[capture] PASS den-preview: candidate imported, old den absent, actual parent retained, three moving maximum-scale models, camera={}",
            preview.camera
        );
    }
    world.insert_resource(preview);
}
