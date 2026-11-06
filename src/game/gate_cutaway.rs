//! Reveal the main hero when the actual gate lies on its orthographic view ray.
//! This changes only dedicated gate materials and mesh shadow flags.
use std::collections::{HashMap, HashSet};

use bevy::{
    camera::{primitives::Aabb, visibility::VisibilitySystems},
    light::NotShadowCaster,
    prelude::*,
    transform::TransformSystems,
};

use super::{BattleCamera, GameSession, Hero, gate_feedback::GateArt};

#[cfg(any(debug_assertions, feature = "dev-tools"))]
#[path = "gate_cutaway/capture.rs"]
pub mod capture;

// The authored facade overlaps procedural masonry and door details. A stronger
// cutaway keeps the hero readable through the accumulated transparent layers.
const FADED_OPACITY: f32 = 0.10;
const FADE_SECONDS: f32 = 0.15;

pub(super) struct GateCutawayPlugin;

impl Plugin for GateCutawayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GateMaterials>().add_systems(
            PostUpdate,
            update_cutaway
                .after(TransformSystems::Propagate)
                .after(VisibilitySystems::VisibilityPropagate)
                .after(VisibilitySystems::CalculateBounds)
                .before(VisibilitySystems::CheckVisibility),
        );
    }
}

#[derive(Component)]
pub(super) struct GateCutawayOpacity(pub(super) f32);

/// Mesh-level ownership is required: imported scene children share materials,
/// and NotShadowCaster does not inherit from their GateArt parent.
#[derive(Component)]
pub(super) struct GateSurface {
    pub(super) gate: Entity,
    pub(super) original: Handle<StandardMaterial>,
    pub(super) faded: Option<Handle<StandardMaterial>>,
    pub(super) original_no_shadow: bool,
}

#[derive(Resource, Default)]
struct GateMaterials(HashMap<(Entity, AssetId<StandardMaterial>), Handle<StandardMaterial>>);

fn gate_ancestor(world: &World, entity: Entity) -> Option<Entity> {
    let mut cursor = entity;
    while let Some(parent) = world.get::<ChildOf>(cursor) {
        cursor = parent.parent();
        if world.get::<GateArt>(cursor).is_some() {
            return Some(cursor);
        }
    }
    None
}

fn hero_view_segment(world: &mut World) -> Option<(Vec3, Vec3)> {
    let session = world.resource::<GameSession>();
    if session.finished {
        return None;
    }
    let (hero, visible) = world
        .query_filtered::<(&GlobalTransform, &InheritedVisibility), With<Hero>>()
        .single(world)
        .ok()?;
    if !visible.get() {
        return None;
    }
    let end = hero.translation() + Vec3::Y * 1.2;
    let (camera, projection, pose) = world
        .query_filtered::<(&Camera, &Projection, &GlobalTransform), With<BattleCamera>>()
        .single(world)
        .ok()?;
    if !camera.is_active || !matches!(projection, Projection::Orthographic(_)) {
        return None;
    }
    // Orthographic rays are parallel, not eye-to-hero perspective rays. Start
    // on the camera plane at the same screen position as the hero's body.
    let forward = *pose.forward();
    let depth = (end - pose.translation()).dot(forward);
    (depth > 0.0).then_some((end - forward * depth, end))
}

fn intersects_surface(start: Vec3, end: Vec3, pose: &GlobalTransform, bounds: &Aabb) -> bool {
    let inverse = pose.affine().inverse();
    let start = inverse.transform_point3(start);
    let end = inverse.transform_point3(end);
    if !start.is_finite() || !end.is_finite() {
        return false;
    }
    let min: Vec3 = (bounds.center - bounds.half_extents).into();
    let max: Vec3 = (bounds.center + bounds.half_extents).into();
    let delta = end - start;
    let mut near = 0.0_f32;
    let mut far = 1.0_f32;
    for axis in 0..3 {
        if delta[axis].abs() < 0.00001 {
            if start[axis] < min[axis] || start[axis] > max[axis] {
                return false;
            }
        } else {
            let a = (min[axis] - start[axis]) / delta[axis];
            let b = (max[axis] - start[axis]) / delta[axis];
            near = near.max(a.min(b));
            far = far.min(a.max(b));
            if near > far {
                return false;
            }
        }
    }
    near < 1.0 && far > 0.0
}

fn restore_surface(world: &mut World, entity: Entity) {
    let Some(surface) = world.get::<GateSurface>(entity) else {
        return;
    };
    let original = surface.original.clone();
    let original_no_shadow = surface.original_no_shadow;
    if world
        .get::<MeshMaterial3d<StandardMaterial>>(entity)
        .is_some_and(|material| material.0 != original)
    {
        world.entity_mut(entity).insert(MeshMaterial3d(original));
    }
    if original_no_shadow && world.get::<NotShadowCaster>(entity).is_none() {
        world.entity_mut(entity).insert(NotShadowCaster);
    } else if !original_no_shadow && world.get::<NotShadowCaster>(entity).is_some() {
        world.entity_mut(entity).remove::<NotShadowCaster>();
    }
}

fn update_cutaway(world: &mut World) {
    let gates: HashSet<_> = world
        .query_filtered::<Entity, With<GateArt>>()
        .iter(world)
        .collect();
    // Walk only the gate subtree: enemy/projectile meshes must not pay a
    // repeated ancestry search. Late imported scene children join next frame.
    let mut meshes = Vec::new();
    for gate in &gates {
        let mut pending = world
            .get::<Children>(*gate)
            .map_or_else(Vec::new, |children| children.iter().collect());
        while let Some(entity) = pending.pop() {
            if let Some(children) = world.get::<Children>(entity) {
                pending.extend(children.iter());
            }
            if world.get::<Mesh3d>(entity).is_some()
                && world.get::<GateSurface>(entity).is_none()
                && let Some(material) = world.get::<MeshMaterial3d<StandardMaterial>>(entity)
            {
                meshes.push((entity, *gate, material.0.clone()));
            }
        }
    }
    for (entity, gate, original) in meshes {
        let original_no_shadow = world.get::<NotShadowCaster>(entity).is_some();
        world.entity_mut(entity).insert(GateSurface {
            gate,
            original,
            faded: None,
            original_no_shadow,
        });
    }
    let detached: Vec<_> = world
        .query::<(Entity, &GateSurface)>()
        .iter(world)
        .filter(|(entity, surface)| {
            !gates.contains(&surface.gate) || gate_ancestor(world, *entity) != Some(surface.gate)
        })
        .map(|(entity, _)| entity)
        .collect();
    for entity in detached {
        restore_surface(world, entity);
        world.entity_mut(entity).remove::<GateSurface>();
    }
    let stale: Vec<_> = world
        .resource::<GateMaterials>()
        .0
        .iter()
        .filter(|((gate, _), _)| !gates.contains(gate))
        .map(|(key, handle)| (*key, handle.id()))
        .collect();
    for (key, id) in stale {
        world.resource_mut::<GateMaterials>().0.remove(&key);
        world.resource_mut::<Assets<StandardMaterial>>().remove(id);
    }

    let segment = hero_view_segment(world);
    let mut blocked = HashSet::new();
    if let Some((start, end)) = segment {
        for (surface, bounds, pose, visible) in world
            .query::<(&GateSurface, &Aabb, &GlobalTransform, &InheritedVisibility)>()
            .iter(world)
        {
            if visible.get() && intersects_surface(start, end, pose, bounds) {
                blocked.insert(surface.gate);
            }
        }
    }
    let step = (1.0 - FADED_OPACITY) / FADE_SECONDS * world.resource::<Time<Real>>().delta_secs();
    for gate in &gates {
        let old = world
            .get::<GateCutawayOpacity>(*gate)
            .map_or(1.0, |value| value.0);
        let target = if blocked.contains(gate) {
            FADED_OPACITY
        } else {
            1.0
        };
        let next = if old < target {
            (old + step).min(target)
        } else {
            (old - step).max(target)
        };
        if world.get::<GateCutawayOpacity>(*gate).is_none() || old != next {
            world.entity_mut(*gate).insert(GateCutawayOpacity(next));
        }
    }
    let surfaces: Vec<_> = world
        .query::<(Entity, &GateSurface)>()
        .iter(world)
        .map(|(entity, surface)| (entity, surface.gate, surface.original.clone()))
        .collect();
    for (entity, gate, original) in surfaces {
        let opacity = world
            .get::<GateCutawayOpacity>(gate)
            .map_or(1.0, |value| value.0);
        if opacity >= 1.0 {
            restore_surface(world, entity);
            continue;
        }
        let Some(source) = world
            .resource::<Assets<StandardMaterial>>()
            .get(&original)
            .cloned()
        else {
            continue;
        };
        let key = (gate, original.id());
        let faded = if let Some(handle) = world.resource::<GateMaterials>().0.get(&key) {
            handle.clone()
        } else {
            let mut material = source.clone();
            material.alpha_mode = AlphaMode::Blend;
            material
                .base_color
                .set_alpha(source.base_color.alpha() * opacity);
            let handle = world
                .resource_mut::<Assets<StandardMaterial>>()
                .add(material);
            world
                .resource_mut::<GateMaterials>()
                .0
                .insert(key, handle.clone());
            handle
        };
        // Inspect immutably first: idle cutaways must not upload materials each frame.
        let alpha = source.base_color.alpha() * opacity;
        if world
            .resource::<Assets<StandardMaterial>>()
            .get(&faded)
            .is_some_and(|material| material.base_color.alpha() != alpha)
        {
            world
                .resource_mut::<Assets<StandardMaterial>>()
                .get_mut(&faded)
                .unwrap()
                .base_color
                .set_alpha(alpha);
        }
        if world
            .get::<MeshMaterial3d<StandardMaterial>>(entity)
            .is_some_and(|material| material.0 != faded)
        {
            world
                .entity_mut(entity)
                .insert(MeshMaterial3d(faded.clone()));
        }
        if world.get::<NotShadowCaster>(entity).is_none() {
            world.entity_mut(entity).insert(NotShadowCaster);
        }
        if world
            .get::<GateSurface>(entity)
            .is_some_and(|surface| surface.faded.as_ref() != Some(&faded))
        {
            world.get_mut::<GateSurface>(entity).unwrap().faded = Some(faded);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    struct Fixture {
        world: World,
        gate: Entity,
        hero: Entity,
        camera: Entity,
        solid: Entity,
        shadowless: Entity,
        hidden_stage: Entity,
        unrelated: Entity,
        material: Handle<StandardMaterial>,
    }

    fn surface(
        world: &mut World,
        parent: Option<Entity>,
        material: Handle<StandardMaterial>,
        visible: bool,
    ) -> Entity {
        let entity = world
            .spawn((
                Mesh3d(Handle::default()),
                MeshMaterial3d(material),
                GlobalTransform::from_translation(Vec3::new(0.0, 1.2, 0.0)),
                Aabb::from_min_max(Vec3::new(-0.5, -1.5, -1.0), Vec3::new(0.5, 1.5, 1.0)),
                if visible {
                    InheritedVisibility::VISIBLE
                } else {
                    InheritedVisibility::HIDDEN
                },
                if visible {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                },
            ))
            .id();
        if let Some(parent) = parent {
            world.entity_mut(entity).insert(ChildOf(parent));
        }
        entity
    }

    fn fixture() -> Fixture {
        let mut world = World::new();
        world.init_resource::<GateMaterials>();
        world.init_resource::<Assets<StandardMaterial>>();
        world.init_resource::<GameSession>();
        let mut time = Time::<Real>::default();
        time.advance_by(Duration::from_secs_f32(0.05));
        world.insert_resource(time);
        let gate = world.spawn(GateArt).id();
        let nested_scene = world.spawn(ChildOf(gate)).id();
        let material = world
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color: Color::srgb(0.4, 0.45, 0.5),
                ..default()
            });
        let solid = surface(&mut world, Some(nested_scene), material.clone(), true);
        let shadowless = surface(&mut world, Some(gate), material.clone(), true);
        world.entity_mut(shadowless).insert(NotShadowCaster);
        let hidden_stage = surface(&mut world, Some(gate), material.clone(), false);
        let unrelated = surface(&mut world, None, material.clone(), true);
        let hero = world
            .spawn((
                Hero,
                GlobalTransform::from_translation(Vec3::new(2.0, 0.0, 0.0)),
                InheritedVisibility::VISIBLE,
            ))
            .id();
        let camera = world
            .spawn((
                BattleCamera::default(),
                Camera::default(),
                Projection::from(OrthographicProjection::default_3d()),
                GlobalTransform::from(
                    Transform::from_xyz(-10.0, 1.2, 0.0)
                        .looking_at(Vec3::new(2.0, 1.2, 0.0), Vec3::Y),
                ),
            ))
            .id();
        Fixture {
            world,
            gate,
            hero,
            camera,
            solid,
            shadowless,
            hidden_stage,
            unrelated,
            material,
        }
    }

    fn settle(world: &mut World) {
        for _ in 0..4 {
            update_cutaway(world);
        }
    }

    #[test]
    fn actual_gate_subtree_fades_with_private_material_and_preserves_hidden_damage_art() {
        let mut f = fixture();
        settle(&mut f.world);
        assert_eq!(
            f.world.get::<GateCutawayOpacity>(f.gate).unwrap().0,
            FADED_OPACITY
        );
        let faded = &f
            .world
            .get::<MeshMaterial3d<StandardMaterial>>(f.solid)
            .unwrap()
            .0;
        assert_ne!(*faded, f.material);
        assert_eq!(
            *faded,
            f.world
                .get::<MeshMaterial3d<StandardMaterial>>(f.shadowless)
                .unwrap()
                .0
        );
        let assets = f.world.resource::<Assets<StandardMaterial>>();
        assert!(matches!(
            assets.get(faded).unwrap().alpha_mode,
            AlphaMode::Blend
        ));
        assert_eq!(assets.get(faded).unwrap().base_color.alpha(), FADED_OPACITY);
        assert!(matches!(
            assets.get(&f.material).unwrap().alpha_mode,
            AlphaMode::Opaque
        ));
        assert_eq!(assets.get(&f.material).unwrap().base_color.alpha(), 1.0);
        assert_eq!(
            f.world
                .get::<MeshMaterial3d<StandardMaterial>>(f.unrelated)
                .unwrap()
                .0,
            f.material
        );
        assert_eq!(
            *f.world.get::<Visibility>(f.hidden_stage).unwrap(),
            Visibility::Hidden
        );
        assert!(
            !f.world
                .get::<InheritedVisibility>(f.hidden_stage)
                .unwrap()
                .get()
        );
        assert!(f.world.get::<NotShadowCaster>(f.solid).is_some());
        assert_eq!(f.world.resource::<GateMaterials>().0.len(), 1);
    }

    #[test]
    fn clear_view_restores_exact_handles_and_original_shadow_flags_without_cache_growth() {
        let mut f = fixture();
        settle(&mut f.world);
        let faded = f
            .world
            .get::<GateSurface>(f.solid)
            .unwrap()
            .faded
            .clone()
            .unwrap();
        *f.world.get_mut::<GlobalTransform>(f.hero).unwrap() =
            GlobalTransform::from_translation(Vec3::new(2.0, 0.0, 8.0));
        settle(&mut f.world);
        assert_eq!(f.world.get::<GateCutawayOpacity>(f.gate).unwrap().0, 1.0);
        assert_eq!(
            f.world
                .get::<MeshMaterial3d<StandardMaterial>>(f.solid)
                .unwrap()
                .0,
            f.material
        );
        assert!(f.world.get::<NotShadowCaster>(f.solid).is_none());
        assert!(f.world.get::<NotShadowCaster>(f.shadowless).is_some());
        *f.world.get_mut::<GlobalTransform>(f.hero).unwrap() =
            GlobalTransform::from_translation(Vec3::new(2.0, 0.0, 0.0));
        settle(&mut f.world);
        assert_eq!(
            f.world
                .get::<MeshMaterial3d<StandardMaterial>>(f.solid)
                .unwrap()
                .0,
            faded
        );
        assert_eq!(f.world.resource::<GateMaterials>().0.len(), 1);
    }

    #[test]
    fn hidden_gate_missing_hero_finished_run_and_perspective_do_not_leave_cutaway_active() {
        for mode in 0..4 {
            let mut f = fixture();
            settle(&mut f.world);
            match mode {
                0 => {
                    for entity in [f.solid, f.shadowless, f.hidden_stage] {
                        *f.world.get_mut::<InheritedVisibility>(entity).unwrap() =
                            InheritedVisibility::HIDDEN;
                    }
                }
                1 => {
                    f.world.entity_mut(f.hero).remove::<Hero>();
                }
                2 => {
                    f.world.resource_mut::<GameSession>().finished = true;
                }
                _ => {
                    *f.world.get_mut::<Projection>(f.camera).unwrap() =
                        Projection::Perspective(PerspectiveProjection::default());
                }
            }
            settle(&mut f.world);
            assert_eq!(f.world.get::<GateCutawayOpacity>(f.gate).unwrap().0, 1.0);
            assert_eq!(
                f.world
                    .get::<MeshMaterial3d<StandardMaterial>>(f.solid)
                    .unwrap()
                    .0,
                f.material
            );
        }
    }

    #[test]
    fn gate_removal_cleans_private_assets_and_detached_surface_restores_before_cleanup() {
        let mut f = fixture();
        settle(&mut f.world);
        let faded = f
            .world
            .get::<GateSurface>(f.solid)
            .unwrap()
            .faded
            .clone()
            .unwrap();
        f.world.entity_mut(f.solid).remove::<ChildOf>();
        f.world.entity_mut(f.gate).despawn();
        update_cutaway(&mut f.world);
        assert!(f.world.resource::<GateMaterials>().0.is_empty());
        assert!(
            f.world
                .resource::<Assets<StandardMaterial>>()
                .get(&faded)
                .is_none()
        );
        assert_eq!(
            f.world
                .get::<MeshMaterial3d<StandardMaterial>>(f.solid)
                .unwrap()
                .0,
            f.material
        );
        assert!(f.world.get::<GateSurface>(f.solid).is_none());
        assert!(f.world.get::<NotShadowCaster>(f.solid).is_none());
    }

    #[test]
    fn idle_materials_stay_unmodified_and_render_only_override_is_rebound_next_frame() {
        let mut f = fixture();
        settle(&mut f.world);
        f.world.clear_trackers();
        update_cutaway(&mut f.world);
        assert!(!f.world.is_resource_changed::<Assets<StandardMaterial>>());
        // A capture-only opaque control can restore the handle in Last. The
        // next production update must bind the fade again even at idle opacity.
        f.world
            .entity_mut(f.solid)
            .insert(MeshMaterial3d(f.material.clone()));
        f.world.entity_mut(f.solid).remove::<NotShadowCaster>();
        f.world.clear_trackers();
        update_cutaway(&mut f.world);
        assert_ne!(
            f.world
                .get::<MeshMaterial3d<StandardMaterial>>(f.solid)
                .unwrap()
                .0,
            f.material
        );
        assert!(f.world.get::<NotShadowCaster>(f.solid).is_some());
        assert!(!f.world.is_resource_changed::<Assets<StandardMaterial>>());
        *f.world.get_mut::<GlobalTransform>(f.hero).unwrap() =
            GlobalTransform::from_translation(Vec3::new(2.0, 0.0, 8.0));
        settle(&mut f.world);
        f.world.clear_trackers();
        update_cutaway(&mut f.world);
        assert!(!f.world.is_resource_changed::<Assets<StandardMaterial>>());
    }

    #[test]
    fn parallel_orthographic_ray_and_local_bounds_handle_off_axis_and_nonuniform_meshes() {
        let mut f = fixture();
        *f.world.get_mut::<GlobalTransform>(f.hero).unwrap() =
            GlobalTransform::from_translation(Vec3::new(2.0, 0.0, 8.0));
        let (start, end) = hero_view_segment(&mut f.world).unwrap();
        let bounds = Aabb::from_min_max(Vec3::splat(-0.5), Vec3::splat(0.5));
        let pose = GlobalTransform::from_translation(Vec3::new(0.0, 1.2, 8.0));
        assert!(intersects_surface(start, end, &pose, &bounds));
        assert!(!intersects_surface(
            Vec3::new(-10.0, 1.2, 0.0),
            end,
            &pose,
            &bounds
        ));
        let transform = Transform::from_xyz(4.0, 2.0, -3.0)
            .with_rotation(Quat::from_rotation_y(0.7))
            .with_scale(Vec3::new(0.3, 2.0, 3.5));
        let pose = GlobalTransform::from(transform);
        let start = pose.affine().transform_point3(Vec3::new(-2.0, 0.0, 0.0));
        let end = pose.affine().transform_point3(Vec3::new(2.0, 0.0, 0.0));
        assert!(intersects_surface(start, end, &pose, &bounds));
        let short_end = pose.affine().transform_point3(Vec3::new(-1.0, 0.0, 0.0));
        assert!(!intersects_surface(start, short_end, &pose, &bounds));
    }
}
