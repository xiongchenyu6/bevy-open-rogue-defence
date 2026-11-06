//! Controlled gate crossing poses and an explicit opaque-material render control.
//! Neither case represents physical input or a real combat/revival encounter.
use super::*;
use crate::game::{ActiveSkill, AttackClock, Autoplay, PlayerBuild, SpawnDirector};

#[derive(Resource)]
struct Probe {
    frame: u32,
    opaque_control: bool,
}

pub fn install(app: &mut bevy::app::SubApp, opaque_control: bool) {
    let world = app.world_mut();
    world.remove_resource::<Autoplay>();
    world.resource_mut::<PlayerBuild>().skills.fill(0);
    world.resource_mut::<ActiveSkill>().remaining = 10_000.;
    world.resource_mut::<AttackClock>().0.pause();
    world.resource_mut::<SpawnDirector>().enabled = false;
    app.insert_resource(Probe {
        frame: 0,
        opaque_control,
    })
    .add_systems(PreUpdate, pose)
    .add_systems(
        PostUpdate,
        opaque_render_control
            .after(update_cutaway)
            .before(VisibilitySystems::CheckVisibility),
    )
    .add_systems(Last, audit);
}

fn pose(mut probe: ResMut<Probe>, mut hero: Query<&mut Transform, With<Hero>>) {
    probe.frame += 1;
    let next = match probe.frame {
        1 | 330 | 510 => Vec3::new(-8.2, 0., 0.),
        90 => Vec3::new(-4.3, 0., 0.),
        150 => Vec3::new(-4.3, 0., 1.9),
        210 | 450 => Vec3::new(-4.3, 0., 4.),
        270 => Vec3::new(-4.3, 0., -1.9),
        390 => Vec3::new(-4.3, 0., 8.),
        _ => return,
    };
    hero.single_mut().unwrap().translation = next;
}

fn opaque_render_control(world: &mut World) {
    if !world.resource::<Probe>().opaque_control {
        return;
    }
    // A diagnostic render control, not an old-binary run: execute the real
    // detector, then restore original handles/shadow flags before visibility
    // and extraction. No production switch, geometry or gameplay state changes.
    let surfaces: Vec<_> = world
        .query_filtered::<Entity, With<GateSurface>>()
        .iter(world)
        .collect();
    for surface in surfaces {
        restore_surface(world, surface);
    }
}

fn audit(world: &mut World) {
    let probe = world.resource::<Probe>();
    let (frame, opaque_control) = (probe.frame, probe.opaque_control);
    if ![89, 149, 209, 269, 329, 389, 449, 509, 569].contains(&frame) {
        return;
    }
    let gate = world
        .query_filtered::<Entity, With<GateArt>>()
        .single(world)
        .unwrap();
    let opacity = world.get::<GateCutawayOpacity>(gate).unwrap().0;
    let blocked = matches!(frame, 149 | 209 | 269 | 329 | 509);
    assert!(
        (opacity - if blocked { FADED_OPACITY } else { 1. }).abs() < 0.0001,
        "expected settled gate opacity at controlled point {frame}, got {opacity}"
    );
    assert!(world.get::<InheritedVisibility>(gate).unwrap().get());
    let scenes: Vec<_> = world
        .query::<(Entity, &WorldAssetRoot)>()
        .iter(world)
        .filter(|(entity, _)| gate_ancestor(world, *entity) == Some(gate))
        .map(|(_, scene)| scene.0.clone())
        .collect();
    assert_eq!(scenes.len(), 9, "all actual gate scene roots are present");
    let surfaces: Vec<_> = world
        .query::<(Entity, &GateSurface, &MeshMaterial3d<StandardMaterial>)>()
        .iter(world)
        .filter(|(_, surface, _)| surface.gate == gate)
        .map(|(entity, surface, drawn)| {
            (
                entity,
                surface.original.clone(),
                surface.faded.clone(),
                drawn.0.clone(),
                surface.original_no_shadow,
            )
        })
        .collect();
    let server = world.resource::<AssetServer>();
    assert!(
        scenes
            .iter()
            .all(|scene| server.is_loaded_with_dependencies(scene.id()))
    );
    let imported = surfaces
        .iter()
        .filter(|(_, original, _, _, _)| {
            server
                .get_path(original.id())
                .is_some_and(|path| path.path().to_string_lossy().starts_with("models/"))
        })
        .count();
    assert!(imported >= 9, "actual imported gate meshes must be ready");
    for (entity, original, faded, drawn, original_no_shadow) in &surfaces {
        let no_shadow = world.get::<NotShadowCaster>(*entity).is_some();
        if opaque_control || !blocked {
            assert_eq!(drawn, original, "actual original material must be rendered");
            assert_eq!(no_shadow, *original_no_shadow);
        } else {
            assert_eq!(
                Some(drawn),
                faded.as_ref(),
                "actual faded material must be rendered"
            );
            assert_ne!(drawn, original);
            assert!(no_shadow);
        }
    }
    let hero = world
        .query_filtered::<&GlobalTransform, With<Hero>>()
        .single(world)
        .unwrap()
        .translation();
    let (camera, camera_pose) = world
        .query_filtered::<(&Camera, &GlobalTransform), With<BattleCamera>>()
        .single(world)
        .unwrap();
    let body_pixel = camera
        .world_to_viewport(camera_pose, hero + Vec3::Y * 1.2)
        .unwrap();
    assert_eq!(world.resource::<GameSession>().core_hp, 1000.);
    println!(
        "[capture] GATE_CUTAWAY frame={frame} opaque_control={opaque_control} blocked={blocked} opacity={opacity:.6} gate_meshes={} imported_meshes={imported} hero={hero:?} body_pixel={body_pixel:?} camera={:?}",
        surfaces.len(),
        camera_pose.translation()
    );
    if frame == 569 {
        println!(
            "[capture] PASS gate-cutaway: actual imported gate meshes, orthographic blocked poses, original material/shadow restoration, unchanged full gate HP; opaque_control={opaque_control}; inspect pixels, Aabb is not a visibility proof"
        );
    }
}
