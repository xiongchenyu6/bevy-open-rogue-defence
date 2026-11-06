//! Opt-in observation of real sword hits and an untargetable shared-material control.
use bevy::{light::NotShadowCaster, prelude::*};
use rogue_commander::game::{AttackClock, Enemy, GameAssets};
use std::collections::HashMap;

#[derive(Component)]
struct Control;
#[derive(Resource, Default)]
struct Probe {
    frame: u32,
    started: bool,
    baseline: HashMap<Entity, Handle<StandardMaterial>>,
    sources: HashMap<AssetId<StandardMaterial>, String>,
    shadows: HashMap<Entity, bool>,
    hp: HashMap<Entity, f32>,
    hits: u32,
    swapped: bool,
    restored: bool,
    changed: Vec<Entity>,
    low_fps: bool,
    first_punch: Option<bool>,
    base_scales: HashMap<Entity, Vec3>,
}

pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    world.resource_mut::<AttackClock>().0.pause();
    let speed: f32 = std::env::var("ROGUE_CAPTURE_HIT_MATERIAL_SPEED")
        .unwrap_or_else(|_| "1".into())
        .parse()
        .expect("numeric material trace speed");
    assert!(speed == 1.0 || speed == 8.0);
    world
        .resource_mut::<Time<Virtual>>()
        .set_relative_speed(speed);
    let scene = world.resource::<GameAssets>().enemy_scenes[0].clone();
    world.spawn((
        Name::new("UNHIT shared-material reference"),
        Control,
        WorldAssetRoot(scene),
        Transform::from_xyz(-2.5, 0., 2.7).with_scale(Vec3::splat(0.8)),
    ));
    app.init_resource::<Probe>().add_systems(Last, observe);
    app.world_mut().resource_mut::<Probe>().low_fps =
        std::env::var("ROGUE_CAPTURE_HIT_PUNCH_LOW_FPS").is_ok_and(|v| v == "1");
}

fn owner(world: &World, mut entity: Entity) -> Option<(Entity, bool)> {
    loop {
        if world.get::<Control>(entity).is_some() {
            return Some((entity, true));
        }
        if world
            .get::<super::attack_capture::AttackTarget>(entity)
            .is_some()
        {
            return Some((entity, false));
        }
        entity = world.get::<ChildOf>(entity)?.parent();
    }
}

fn observe(world: &mut World) {
    let mut probe = world.remove_resource::<Probe>().unwrap();
    probe.frame += 1;
    let leaves: Vec<_> = world
        .query::<(Entity, &MeshMaterial3d<StandardMaterial>)>()
        .iter(world)
        .filter_map(|(entity, material)| {
            let (actor, control) = owner(world, entity)?;
            if !probe.started {
                let path = world.resource::<AssetServer>().get_path(material.0.id())?;
                let path = path.path().to_string_lossy();
                if !path.starts_with("models/") || !path.ends_with(".glb") {
                    return None;
                }
            } else if !probe.baseline.contains_key(&entity) {
                return None;
            }
            Some((
                entity,
                actor,
                control,
                material.0.clone(),
                world.get::<NotShadowCaster>(entity).is_some(),
            ))
        })
        .collect();
    let ready = leaves.iter().any(|x| x.2)
        && leaves
            .iter()
            .filter(|x| !x.2)
            .map(|x| x.1)
            .collect::<std::collections::HashSet<_>>()
            .len()
            == 3
        && leaves.iter().all(|x| {
            world
                .resource::<Assets<StandardMaterial>>()
                .get(&x.3)
                .is_some()
        });
    if !probe.started && ready {
        let materials = world.resource::<Assets<StandardMaterial>>();
        for (leaf, _, _, handle, shadow) in &leaves {
            probe.shadows.insert(*leaf, *shadow);
            probe.baseline.insert(*leaf, handle.clone());
            probe
                .sources
                .insert(handle.id(), format!("{:?}", materials.get(handle).unwrap()));
        }
        assert!(
            leaves.iter().filter(|x| x.2).any(|control| leaves
                .iter()
                .any(|target| !target.2 && target.3 == control.3)),
            "control must share original GLB materials"
        );
        for (actor, transform) in world
            .query_filtered::<(Entity, &Transform), With<super::attack_capture::AttackTarget>>()
            .iter(world)
        {
            probe.base_scales.insert(actor, transform.scale);
        }
        if probe.low_fps {
            world.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_millis(160),
            ));
        }
        probe.started = true;
        println!(
            "HIT_MATERIAL_BASELINE frame={} leaves={} sources={} shared_control=true",
            probe.frame,
            leaves.len(),
            probe.sources.len()
        );
        world.resource_mut::<AttackClock>().0.unpause();
    }
    if probe.started {
        let health: Vec<_> = world
            .query_filtered::<(Entity, &Enemy), With<super::attack_capture::AttackTarget>>()
            .iter(world)
            .map(|(entity, enemy)| (entity, enemy.hp))
            .collect();
        for (actor, hp) in health {
            if let Some(previous) = probe.hp.insert(actor, hp)
                && hp < previous
            {
                probe.hits += 1;
                if probe.low_fps && probe.first_punch.is_none() {
                    let base = probe.base_scales[&actor];
                    let scale = world.get::<Transform>(actor).unwrap().scale;
                    let visible = scale.y < base.y && scale.x > base.x;
                    probe.first_punch = Some(visible);
                    let expected =
                        std::env::var("EXPECT_HIT_PUNCH_FIRST_FRAME").is_ok_and(|v| v == "1");
                    println!(
                        "HIT_PUNCH_FIRST frame={} actor={actor:?} actual_hp={hp} base={base:?} scale={scale:?} visible={visible} real_delta={}",
                        probe.frame,
                        world.resource::<Time<Real>>().delta_secs()
                    );
                    assert_eq!(visible, expected, "real sword contact first-render squash");
                }
                println!(
                    "HIT_MATERIAL_CONTACT frame={} actor={actor:?} hp={hp:.3} damage={:.3}",
                    probe.frame,
                    previous - hp
                );
            }
        }
        let materials = world.resource::<Assets<StandardMaterial>>();
        for (id, original) in &probe.sources {
            assert_eq!(
                &format!(
                    "{:?}",
                    materials.get(*id).expect("original material retained")
                ),
                original,
                "source material mutated"
            );
        }
        for (leaf, actor, control, handle, shadow) in &leaves {
            let Some(original) = probe.baseline.get(leaf).cloned() else {
                continue;
            };
            assert_eq!(
                probe.shadows.get(leaf),
                Some(shadow),
                "body shadow flag changed"
            );
            let different = handle != &original;
            assert!(!*control || !different, "unhit control changed material");
            if different {
                assert!(probe.hits > 0, "flash before actual HP damage");
                probe.swapped = true;
                if !probe.changed.contains(leaf) {
                    probe.changed.push(*leaf);
                }
            } else if probe.changed.contains(leaf) {
                probe.restored = true;
            }
            if probe.frame <= 100 || different {
                let material = materials.get(handle).unwrap();
                println!(
                    "HIT_MATERIAL frame={} leaf={leaf:?} actor={actor:?} control={control} original={:?} current={:?} changed={different} base={:?} emissive={:?} alpha={:?} no_shadow={shadow}",
                    probe.frame,
                    original.id(),
                    handle.id(),
                    material.base_color,
                    material.emissive,
                    material.alpha_mode
                );
            }
        }
    }
    if probe.frame == 180 {
        assert!(
            probe.started && probe.hits > 0,
            "fixture requires ready models and real damage"
        );
        let expected = std::env::var("EXPECT_HIT_FLASH").is_ok_and(|v| v == "1");
        assert_eq!(
            probe.swapped, expected,
            "actual damaged model flash expectation"
        );
        assert!(
            !expected || probe.restored,
            "flash must restore original handle"
        );
        assert!(
            !probe.low_fps || probe.first_punch.is_some(),
            "low-FPS requires actual sword contact"
        );
        println!(
            "HIT_MATERIAL_PASS hits={} swapped={} restored={} source_and_control_unchanged=true",
            probe.hits, probe.swapped, probe.restored
        );
    }
    world.insert_resource(probe);
}
