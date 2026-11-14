//! Brief contact feedback on struck enemies; imported source materials remain immutable.
//!
//! Every fresh contact lights only the struck side of the body with a short,
//! unshadowed point light at the contact point. A critical contact additionally
//! swaps the body to a warm per-instance variant, so the two tiers stay distinct.
use super::{Enemy, GameSession, HitFeedback, HitPunch};
use bevy::prelude::*;
use std::collections::HashMap;

const FLASH_SECONDS: f32 = 0.070;
/// Cosmetic budget: a barrage never adds more dynamic lights than this.
const MAX_CONTACT_LIGHTS: usize = 12;

#[derive(Clone, Copy)]
struct Contact {
    owner: Entity,
    epoch: u32,
    critical: bool,
    /// Horizontal travel direction of the blow; zero when unknown.
    direction: Vec3,
}

#[derive(Resource, Default)]
pub(super) struct EnemyFlash {
    pending: Vec<Contact>,
    /// Critical body variants, keyed by their imported source material.
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

/// One pooled light. Reusing a fixed set keeps combat entity allocation
/// independent of how many contacts were presented.
#[derive(Component, Clone, Copy, Default)]
struct ContactLight {
    owner: Option<Entity>,
    remaining: f32,
    peak: f32,
}

pub(super) fn setup(mut commands: Commands) {
    for _ in 0..MAX_CONTACT_LIGHTS {
        commands.spawn((
            Name::new("Hit contact light"),
            PointLight {
                intensity: 0.0,
                radius: 0.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::default(),
            Visibility::Hidden,
            ContactLight::default(),
        ));
    }
}

/// Collect before punch animation: a long frame can complete/remove the punch.
pub(super) fn collect(
    mut flash: ResMut<EnemyFlash>,
    session: Res<GameSession>,
    feedback: Res<HitFeedback>,
    punched: Query<(Entity, Ref<Enemy>), Added<HitPunch>>,
) {
    if session.finished || session.paused {
        return;
    }
    flash
        .pending
        .extend(
            punched
                .iter()
                .filter(|(_, enemy)| enemy.hp > 0.)
                .map(|(entity, enemy)| {
                    let hits = || {
                        feedback
                            .events()
                            .iter()
                            .filter(move |event| event.target == entity && !event.killed)
                    };
                    // Same-frame mixed contacts present the critical one.
                    let strongest = hits()
                        .find(|event| event.critical)
                        .or_else(|| hits().next());
                    Contact {
                        owner: entity,
                        epoch: enemy.added().get(),
                        critical: strongest.is_some_and(|event| event.critical),
                        direction: strongest.map_or(Vec3::ZERO, |event| event.direction),
                    }
                }),
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

/// Just off the attacker-facing side of the upper body, where the blow lands.
fn contact_point(body: &Transform, direction: Vec3) -> Vec3 {
    let toward_attacker = -direction.with_y(0.0).normalize_or_zero();
    body.translation
        + Vec3::Y * 0.9 * body.scale.y
        + toward_attacker * 0.6 * body.scale.x.max(body.scale.z)
}

/// Colour, peak intensity and reach of the highlight for a body of this size.
fn contact_light(body: &Transform, critical: bool) -> (Color, f32, f32) {
    let size = body.scale.max_element().max(0.5);
    let (color, peak) = if critical {
        (Color::srgb(1.0, 0.78, 0.35), 150_000.0)
    } else {
        (Color::srgb(1.0, 0.96, 0.88), 70_000.0)
    };
    // Short reach keeps the highlight on the struck region, off the floor.
    (color, peak * size * size, 1.7 * size)
}

/// PostUpdate sees the matte imported finishes. New feedback is applied after
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
    let lights: Vec<_> = world
        .query::<(Entity, &ContactLight)>()
        .iter(world)
        .map(|(entity, light)| (entity, *light))
        .collect();
    let mut lit: Vec<Entity> = Vec::with_capacity(lights.len());
    let mut free: Vec<Entity> = Vec::with_capacity(lights.len());
    for (entity, mut light) in lights {
        let Some(owner) = light.owner else {
            free.push(entity);
            continue;
        };
        light.remaining -= delta;
        if finished || light.remaining <= 0. {
            world.get_mut::<PointLight>(entity).unwrap().intensity = 0.0;
            world
                .entity_mut(entity)
                .insert((Visibility::Hidden, ContactLight::default()));
            free.push(entity);
            continue;
        }
        lit.push(owner);
        // Fade out instead of popping off.
        world.get_mut::<PointLight>(entity).unwrap().intensity =
            light.peak * (light.remaining / FLASH_SECONDS);
        world.entity_mut(entity).insert(light);
    }
    state
        .variants
        .retain(|id, _| world.resource::<Assets<StandardMaterial>>().contains(*id));
    for contact in state.pending.drain(..) {
        let Contact { owner, epoch, .. } = contact;
        if finished || paused || owner_epoch(world, owner) != Some(epoch) {
            continue;
        }
        // An active cycle is not restarted by another punch on the same body.
        if !lit.contains(&owner)
            && let Some(mut body) = world.get::<Transform>(owner).copied()
            && let Some(entity) = free.pop()
        {
            // The punch may already have squashed this frame's scale.
            if let Some(punch) = world.get::<HitPunch>(owner) {
                body.scale = punch.base_scale;
            }
            let (color, peak, range) = contact_light(&body, contact.critical);
            let mut light = world.get_mut::<PointLight>(entity).unwrap();
            light.color = color;
            light.intensity = peak;
            light.range = range;
            world.entity_mut(entity).insert((
                Transform::from_translation(contact_point(&body, contact.direction)),
                Visibility::Visible,
                ContactLight {
                    owner: Some(owner),
                    remaining: FLASH_SECONDS,
                    peak,
                },
            ));
            lit.push(owner);
        }
        if !contact.critical {
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
                // A warm, strong accent rather than a uniform white silhouette.
                let base = source.base_color.to_srgba();
                let (amount, tint) = (0.55, [1.0, 0.82, 0.38]);
                variant.base_color = Color::srgba(
                    base.red + (tint[0] - base.red) * amount,
                    base.green + (tint[1] - base.green) * amount,
                    base.blue + (tint[2] - base.blue) * amount,
                    base.alpha,
                );
                variant.emissive.red += 0.4;
                variant.emissive.green += 0.18;
                variant.emissive.blue += 0.03;
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
            .init_resource::<HitFeedback>()
            .add_systems(Startup, setup)
            .add_systems(Update, (collect, super::super::animate_hit_punch).chain())
            .add_systems(PostUpdate, animate);
        let world = app.world_mut();
        world.run_schedule(Startup);
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
    fn contact(app: &mut App, owner: Entity, critical: bool) {
        app.world_mut()
            .resource_mut::<HitFeedback>()
            .push(super::super::HitEvent {
                target: owner,
                position: Vec3::ZERO,
                direction: Vec3::X,
                element: crate::game::bonds::DamageElement::Physical,
                weight: super::super::HitWeight::Normal,
                critical,
                killed: false,
            });
    }
    /// A fresh critical punch cycle: the tier that flashes the whole body.
    fn hit(app: &mut App, owner: Entity) {
        app.world_mut().resource_mut::<HitFeedback>().events.clear();
        contact(app, owner, true);
        punch(app, owner);
    }
    fn punch(app: &mut App, owner: Entity) {
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

    /// Lights currently presenting a contact.
    fn lights(app: &mut App) -> Vec<(PointLight, Vec3)> {
        app.world_mut()
            .query::<(&PointLight, &Transform, &ContactLight, &Visibility)>()
            .iter(app.world())
            .filter(|(light, _, contact, visibility)| {
                contact.owner.is_some()
                    && light.intensity > 0.0
                    && **visibility == Visibility::Visible
            })
            .map(|(light, transform, ..)| (*light, transform.translation))
            .collect()
    }

    #[test]
    fn ordinary_contact_lights_the_struck_side_without_touching_body_materials() {
        let (mut app, owner, leaf, _, source) = fixture();
        app.world_mut()
            .get_mut::<Transform>(owner)
            .unwrap()
            .translation = Vec3::new(4., 0., 1.);
        contact(&mut app, owner, false);
        punch(&mut app, owner);
        step(&mut app, 0.016);
        assert_eq!(current(&app, leaf), source);
        assert!(app.world().resource::<EnemyFlash>().variants.is_empty());
        let lights = lights(&mut app);
        assert_eq!(lights.len(), 1);
        let (light, position) = &lights[0];
        // The blow travels +X, so the highlight sits on the -X side of the upper body.
        assert!(position.distance(Vec3::new(3.4, 0.9, 1.)) < 0.0001);
        assert!(!light.shadow_maps_enabled);
        assert!(light.range < 2.0, "the highlight must stay local");
        step(&mut app, 0.08);
        assert!(
            lights_len(&mut app) == 0,
            "contact light expires with the flash"
        );
    }
    fn lights_len(app: &mut App) -> usize {
        lights(app).len()
    }

    #[test]
    fn critical_contact_light_is_warmer_and_stronger_than_ordinary() {
        let mut seen = Vec::new();
        for critical in [false, true] {
            let (mut app, owner, ..) = fixture();
            contact(&mut app, owner, critical);
            punch(&mut app, owner);
            step(&mut app, 0.0);
            seen.push(lights(&mut app).remove(0).0);
        }
        assert!(seen[1].intensity > seen[0].intensity * 2.0);
        let (ordinary, critical) = (seen[0].color.to_srgba(), seen[1].color.to_srgba());
        assert!(critical.blue < ordinary.blue * 0.5);
    }

    #[test]
    fn contact_lights_freeze_on_pause_clear_on_finish_and_never_stack_on_one_body() {
        let (mut app, owner, ..) = fixture();
        contact(&mut app, owner, false);
        punch(&mut app, owner);
        step(&mut app, 0.01);
        app.world_mut().entity_mut(owner).remove::<HitPunch>();
        punch(&mut app, owner);
        step(&mut app, 0.01);
        assert_eq!(lights_len(&mut app), 1);
        app.world_mut().resource_mut::<GameSession>().paused = true;
        step(&mut app, 1.);
        assert_eq!(lights_len(&mut app), 1);
        app.world_mut().resource_mut::<GameSession>().finished = true;
        step(&mut app, 0.01);
        assert_eq!(lights_len(&mut app), 0);
    }

    #[test]
    fn a_barrage_never_exceeds_the_contact_light_budget() {
        let (mut app, ..) = fixture();
        for _ in 0..MAX_CONTACT_LIGHTS + 8 {
            let owner = app
                .world_mut()
                .spawn((enemy(), Transform::default(), Visibility::Visible))
                .id();
            contact(&mut app, owner, false);
            punch(&mut app, owner);
        }
        step(&mut app, 0.01);
        assert_eq!(lights_len(&mut app), MAX_CONTACT_LIGHTS);
        // The pool is fixed: presenting contacts never allocates entities.
        assert_eq!(
            app.world_mut()
                .query::<&ContactLight>()
                .iter(app.world())
                .count(),
            MAX_CONTACT_LIGHTS
        );
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
        assert_eq!(variant.emissive, LinearRgba::rgb(0.4, 0.18, 0.03));
        let color = variant.base_color.to_srgba();
        // Warm accent blended over, not replacing, the creature's own palette.
        assert!(color.red > color.green && color.green > color.blue);
        assert!(color.blue < 0.3);
        assert_eq!(format!("{:?}", assets.get(&source).unwrap()), before);
    }

    #[test]
    fn simultaneous_ordinary_and_critical_contacts_keep_the_critical_tier() {
        for critical_first in [false, true] {
            let (mut app, owner, leaf, control, source) = fixture();
            for critical in [critical_first, !critical_first] {
                contact(&mut app, owner, critical);
            }
            punch(&mut app, owner);
            step(&mut app, 0.016);
            let assets = app.world().resource::<Assets<StandardMaterial>>();
            let material = assets.get(current(&app, leaf).id()).unwrap();
            assert_eq!(material.emissive, LinearRgba::rgb(0.4, 0.18, 0.03));
            assert_eq!(material.base_color.alpha(), 0.8);
            assert_eq!(material.base_color_texture, Some(Handle::default()));
            assert_eq!(current(&app, control), source);
            step(&mut app, 0.08);
            assert_eq!(current(&app, leaf), source);
            // The next ordinary cycle does not inherit the critical body flash.
            app.world_mut().resource_mut::<HitFeedback>().events.clear();
            app.world_mut().entity_mut(owner).remove::<HitPunch>();
            contact(&mut app, owner, false);
            punch(&mut app, owner);
            step(&mut app, 0.016);
            assert_eq!(current(&app, leaf), source);
            assert_eq!(app.world().resource::<EnemyFlash>().variants.len(), 1);
        }
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
