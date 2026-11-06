//! Brief per-instance body feedback; imported source materials remain immutable.
use super::{Enemy, GameSession, HitPunch};
use bevy::prelude::*;
use std::collections::HashMap;

const FLASH_SECONDS: f32 = 0.070;

#[derive(Resource, Default)]
pub(super) struct EnemyFlash {
    pending: Vec<(Entity, u32)>,
    variants: HashMap<AssetId<StandardMaterial>, Handle<StandardMaterial>>,
}

#[derive(Component, Clone)]
struct FlashedLeaf {
    owner: Entity,
    epoch: u32,
    original: Handle<StandardMaterial>,
    flash: Handle<StandardMaterial>,
    remaining: f32,
}

/// Collect before punch animation: a long frame can complete/remove the punch.
pub(super) fn collect(
    mut flash: ResMut<EnemyFlash>,
    session: Res<GameSession>,
    punched: Query<(Entity, Ref<Enemy>), Added<HitPunch>>,
) {
    if session.finished || session.paused {
        return;
    }
    flash.pending.extend(
        punched
            .iter()
            .filter(|(_, enemy)| enemy.hp > 0.)
            .map(|(entity, enemy)| (entity, enemy.added().get())),
    );
}

fn owner_epoch(world: &World, owner: Entity) -> Option<u32> {
    let entity = world.get_entity(owner).ok()?;
    if entity.get::<Visibility>() == Some(&Visibility::Hidden) {
        return None;
    }
    let enemy = entity.get::<Enemy>()?;
    if enemy.hp <= 0. {
        return None;
    }
    Some(entity.get_change_ticks::<Enemy>()?.added.get())
}

/// PostUpdate sees the matte imported finishes. New swaps are applied after
/// aging existing ones, guaranteeing one rendered frame even at low frame rate.
pub(super) fn animate(world: &mut World) {
    let mut state = world.remove_resource::<EnemyFlash>().unwrap();
    let session = world.resource::<GameSession>();
    let (paused, finished) = (session.paused, session.finished);
    let delta = if paused {
        0.
    } else {
        world.resource::<Time<Real>>().delta_secs()
    };
    let active: Vec<_> = world
        .query::<(Entity, &FlashedLeaf)>()
        .iter(world)
        .map(|(entity, flash)| (entity, flash.clone()))
        .collect();
    for (entity, mut flash) in active {
        flash.remaining -= delta;
        let owned = world
            .get::<MeshMaterial3d<StandardMaterial>>(entity)
            .is_some_and(|material| material.0 == flash.flash);
        if finished
            || owner_epoch(world, flash.owner) != Some(flash.epoch)
            || flash.remaining <= 0.
            || !owned
        {
            if owned {
                world
                    .entity_mut(entity)
                    .insert(MeshMaterial3d(flash.original));
            }
            world.entity_mut(entity).remove::<FlashedLeaf>();
        } else {
            world.entity_mut(entity).insert(flash);
        }
    }
    state
        .variants
        .retain(|id, _| world.resource::<Assets<StandardMaterial>>().contains(*id));
    for (owner, epoch) in state.pending.drain(..) {
        if finished || paused || owner_epoch(world, owner) != Some(epoch) {
            continue;
        }
        let mut leaves = Vec::new();
        let mut descendants = vec![owner];
        while let Some(entity) = descendants.pop() {
            if let Some(children) = world.get::<Children>(entity) {
                descendants.extend(children.iter());
            }
            if world.get::<FlashedLeaf>(entity).is_some() {
                continue;
            }
            let Some(material) = world.get::<MeshMaterial3d<StandardMaterial>>(entity) else {
                continue;
            };
            let Some(path) = world.resource::<AssetServer>().get_path(material.0.id()) else {
                continue;
            };
            let path = path.path().to_string_lossy();
            if path.starts_with("models/") && path.ends_with(".glb") {
                leaves.push((entity, material.0.clone()));
            }
        }
        for (entity, original) in leaves {
            let flash = if let Some(handle) = state.variants.get(&original.id()) {
                handle.clone()
            } else {
                let Some(source) = world.resource::<Assets<StandardMaterial>>().get(&original)
                else {
                    continue;
                };
                let mut variant = source.clone();
                variant.base_color = Color::srgba(1., 1., 1., source.base_color.alpha());
                variant.emissive = LinearRgba::rgb(1., 1., 1.);
                let handle = world
                    .resource_mut::<Assets<StandardMaterial>>()
                    .add(variant);
                state.variants.insert(original.id(), handle.clone());
                handle
            };
            world.entity_mut(entity).insert((
                MeshMaterial3d(flash.clone()),
                FlashedLeaf {
                    owner,
                    epoch,
                    original,
                    flash,
                    remaining: FLASH_SECONDS,
                },
            ));
        }
    }
    world.insert_resource(state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::AssetPlugin;
    use std::time::Duration;

    fn enemy() -> Enemy {
        Enemy {
            hp: 100.,
            max_hp: 100.,
            speed: 0.,
            damage: 0.,
            xp: 0,
            wood: 0,
            gold: 0,
            kill_credit: 0,
            elite: false,
        }
    }
    fn fixture() -> (App, Entity, Entity, Entity, Handle<StandardMaterial>) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<StandardMaterial>()
            .init_resource::<GameSession>()
            .init_resource::<EnemyFlash>()
            .add_systems(Update, (collect, super::super::animate_hit_punch).chain())
            .add_systems(PostUpdate, animate);
        let world = app.world_mut();
        let source: Handle<StandardMaterial> = world
            .resource::<AssetServer>()
            .load("models/flash-test.glb#Material0");
        world
            .resource_mut::<Assets<StandardMaterial>>()
            .insert(
                source.id(),
                StandardMaterial {
                    base_color: Color::srgba(0.2, 0.4, 0.1, 0.8),
                    alpha_mode: AlphaMode::Mask(0.4),
                    perceptual_roughness: 0.9,
                    base_color_texture: Some(Handle::default()),
                    ..default()
                },
            )
            .unwrap();
        let owner = world
            .spawn((enemy(), Transform::default(), Visibility::Visible))
            .id();
        let leaf = world
            .spawn((ChildOf(owner), MeshMaterial3d(source.clone())))
            .id();
        let control = world.spawn(MeshMaterial3d(source.clone())).id();
        (app, owner, leaf, control, source)
    }
    fn step(app: &mut App, seconds: f32) {
        let world = app.world_mut();
        world.increment_change_tick();
        world
            .resource_mut::<Time<Real>>()
            .advance_by(Duration::from_secs_f32(seconds));
        world.run_schedule(Update);
        world.run_schedule(PostUpdate);
        world.clear_trackers();
    }
    fn hit(app: &mut App, owner: Entity) {
        app.world_mut().entity_mut(owner).insert(HitPunch {
            elapsed: 0.,
            presented: false,
            base_scale: Vec3::ONE,
            strength: 0.1,
        });
    }
    fn current(app: &App, leaf: Entity) -> Handle<StandardMaterial> {
        app.world()
            .get::<MeshMaterial3d<StandardMaterial>>(leaf)
            .unwrap()
            .0
            .clone()
    }

    #[test]
    fn hit_flash_is_per_instance_and_preserves_texture_alpha_and_source() {
        let (mut app, owner, leaf, control, source) = fixture();
        let before = format!(
            "{:?}",
            app.world()
                .resource::<Assets<StandardMaterial>>()
                .get(&source)
                .unwrap()
        );
        hit(&mut app, owner);
        step(&mut app, 0.016);
        assert_ne!(current(&app, leaf), source);
        assert_eq!(current(&app, control), source);
        let assets = app.world().resource::<Assets<StandardMaterial>>();
        let variant = assets.get(current(&app, leaf).id()).unwrap();
        assert_eq!(variant.alpha_mode, AlphaMode::Mask(0.4));
        assert_eq!(variant.base_color.alpha(), 0.8);
        assert_eq!(variant.base_color_texture, Some(Handle::default()));
        assert_eq!(variant.emissive, LinearRgba::WHITE);
        assert_eq!(format!("{:?}", assets.get(&source).unwrap()), before);
    }

    #[test]
    fn hit_flash_and_punch_share_first_large_frame_then_restore() {
        let (mut app, owner, leaf, _, source) = fixture();
        hit(&mut app, owner);
        step(&mut app, 0.2);
        assert!(app.world().get::<HitPunch>(owner).is_some());
        assert!(app.world().get::<Transform>(owner).unwrap().scale.y < 1.);
        assert_ne!(
            current(&app, leaf),
            source,
            "first rendered frame must show impact"
        );
        step(&mut app, 0.2);
        assert_eq!(current(&app, leaf), source);
    }

    #[test]
    fn hit_flash_lifetime_uses_real_time_at_all_battle_speeds() {
        for speed in [1., 8., 32.] {
            let (mut app, owner, leaf, _, source) = fixture();
            app.world_mut()
                .resource_mut::<Time<Virtual>>()
                .set_relative_speed(speed);
            hit(&mut app, owner);
            step(&mut app, 0.01);
            step(&mut app, 0.04);
            assert_ne!(current(&app, leaf), source);
            step(&mut app, 0.04);
            assert_eq!(current(&app, leaf), source);
        }
    }

    #[test]
    fn hit_flash_pause_freezes_but_finish_and_retirement_restore() {
        for finish in [false, true] {
            let (mut app, owner, leaf, _, source) = fixture();
            hit(&mut app, owner);
            step(&mut app, 0.01);
            app.world_mut().resource_mut::<GameSession>().paused = true;
            step(&mut app, 1.);
            assert_ne!(current(&app, leaf), source);
            if finish {
                app.world_mut().resource_mut::<GameSession>().finished = true;
            } else {
                app.world_mut().entity_mut(owner).remove::<Enemy>();
            }
            step(&mut app, 0.01);
            assert_eq!(current(&app, leaf), source);
        }
    }

    #[test]
    fn hit_flash_reused_actor_and_external_phase_material_are_not_overwritten() {
        let (mut app, owner, leaf, _, source) = fixture();
        hit(&mut app, owner);
        step(&mut app, 0.01);
        app.world_mut().increment_change_tick();
        app.world_mut()
            .entity_mut(owner)
            .remove::<Enemy>()
            .insert(enemy());
        step(&mut app, 0.01);
        assert_eq!(current(&app, leaf), source);
        app.world_mut().entity_mut(owner).remove::<HitPunch>();
        hit(&mut app, owner);
        step(&mut app, 0.01);
        let phase = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        app.world_mut()
            .entity_mut(leaf)
            .insert(MeshMaterial3d(phase.clone()));
        step(&mut app, 0.2);
        assert_eq!(current(&app, leaf), phase);
        assert!(app.world().get::<FlashedLeaf>(leaf).is_none());
    }

    #[test]
    fn hit_flash_active_cycle_does_not_restart_on_another_punch() {
        let (mut app, owner, leaf, _, source) = fixture();
        hit(&mut app, owner);
        step(&mut app, 0.01);
        app.world_mut().entity_mut(owner).remove::<HitPunch>();
        hit(&mut app, owner);
        step(&mut app, 0.04);
        assert_ne!(current(&app, leaf), source);
        step(&mut app, 0.04);
        assert_eq!(
            current(&app, leaf),
            source,
            "active flash must not refresh its timer"
        );
    }

    #[test]
    fn hit_flash_cache_reuses_variants_and_prunes_removed_sources() {
        let (mut app, owner, leaf, _, source) = fixture();
        let initial = app.world().resource::<Assets<StandardMaterial>>().len();
        for _ in 0..5 {
            hit(&mut app, owner);
            step(&mut app, 0.01);
            step(&mut app, 0.2);
            assert_eq!(current(&app, leaf), source);
            assert_eq!(
                app.world().resource::<Assets<StandardMaterial>>().len(),
                initial + 1
            );
        }
        app.world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .remove(source.id());
        step(&mut app, 0.01);
        assert!(app.world().resource::<EnemyFlash>().variants.is_empty());
    }
}
