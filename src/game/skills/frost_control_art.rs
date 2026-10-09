//! Shared thin ice shell sampled from authoritative frozen control timers.
use super::*;
use bevy::{asset::RenderAssetUsages, mesh::PrimitiveTopology};

const MAX_FROZEN_ART: usize = 32;
#[derive(Resource)]
struct FrostArt {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}
#[derive(Component)]
struct FrozenArt(Entity);

pub(super) fn install(app: &mut App) {
    app.add_systems(Startup, setup)
        .add_systems(PostUpdate, sync.before(TransformSystems::Propagate));
}
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(FrostArt {
        mesh: meshes.add(mesh()),
        // Dedicated low-emission material: existing Motif::Ice is opaque and
        // emissive x10, which erases silhouettes. Vertex alpha multiplies this.
        material: materials.add(StandardMaterial {
            base_color: Color::srgba(0.64, 0.91, 1.0, 1.0),
            emissive: LinearRgba::new(0.36, 0.74, 1.0, 1.0),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        }),
    });
}

// One shared mesh/material, one entity per frozen target, <=32 draw objects.
// No per-frame assets/particles or emission timer. Frozen actors keep their actual
// world translation; rotation/scale stay stable so pause does not need an art clock.
fn sync(
    mut commands: Commands,
    session: Res<GameSession>,
    runtime: Res<SkillRuntime>,
    art: Option<Res<FrostArt>>,
    enemies: Query<(Entity, &Transform, &Enemy), Without<FrozenArt>>,
    mut effects: Query<(Entity, &FrozenArt, &mut Transform), Without<Enemy>>,
) {
    let alive_frozen = |target: Entity| {
        !session.finished
            && enemies
                .get(target)
                .is_ok_and(|(_, _, enemy)| enemy.hp > 0.0)
            && runtime
                .controls
                .get(&target)
                .is_some_and(|c| c.frozen_remaining > 0.0)
    };
    let mut occupied = Vec::with_capacity(MAX_FROZEN_ART);
    for (entity, frozen, mut transform) in &mut effects {
        if !alive_frozen(frozen.0) {
            commands.entity(entity).despawn();
            continue;
        }
        occupied.push(frozen.0);
        if !session.paused {
            let target = enemies.get(frozen.0).unwrap().1;
            transform.translation = target.translation;
            transform.scale = target.scale * 1.12;
        }
    }
    // Cleanup works while paused/finished, but pause never creates or animates art.
    if session.paused || session.finished {
        return;
    }
    let Some(art) = art else {
        return;
    };
    for (target, transform, _) in &enemies {
        if occupied.len() >= MAX_FROZEN_ART {
            break;
        }
        if !alive_frozen(target) || occupied.contains(&target) {
            continue;
        }
        commands.spawn((
            Name::new("Frozen ice shell"),
            FrozenArt(target),
            Mesh3d(art.mesh.clone()),
            MeshMaterial3d(art.material.clone()),
            Transform::from_translation(transform.translation).with_scale(transform.scale * 1.12),
        ));
        occupied.push(target);
    }
}

fn mesh() -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut colors: Vec<[f32; 4]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut triangle = |a: Vec3, b: Vec3, c: Vec3, alpha: f32| {
        let normal = (b - a).cross(c - a).normalize_or_zero().to_array();
        for point in [a, b, c] {
            positions.push(point.to_array());
            normals.push(normal);
            colors.push([0.75, 0.96, 1.0, alpha]);
        }
    };
    // Eight separated curved panels around the outline. No radial crossbars.
    // 28-degree gaps leave the actor readable; each panel covers 18 degrees.
    // Four small quads per panel avoid a giant flat polygon across the torso.
    for panel in 0..8 {
        let angle = panel as f32 * std::f32::consts::TAU / 8.0;
        for row in 0..4 {
            let low = row as f32 / 4.0;
            let high = (row + 1) as f32 / 4.0;
            let point = |t: f32, edge: f32| {
                let radius = 0.48 + (t * std::f32::consts::PI).sin() * 0.16;
                Vec3::new(
                    (angle + edge).cos() * radius,
                    0.10 + t * 1.35,
                    (angle + edge).sin() * radius,
                )
            };
            let a = point(low, -0.15);
            let b = point(low, 0.15);
            let c = point(high, -0.15);
            let d = point(high, 0.15);
            triangle(a, b, c, 0.34);
            triangle(b, d, c, 0.34);
        }
        // Small triangular crystals grow upward at the OUTSIDE edge, not through
        // the body; their three sides retain crystalline facets at game distance.
        let radial = Vec3::new(angle.cos(), 0.0, angle.sin());
        let tangent = Vec3::new(-angle.sin(), 0.0, angle.cos());
        let base = radial * 0.67 + Vec3::Y * 0.08;
        let tip = base + radial * 0.08 + Vec3::Y * (0.28 + (panel % 3) as f32 * 0.06);
        let a = base + tangent * 0.07;
        let b = base - tangent * 0.07;
        let c = base + radial * 0.09;
        triangle(a, b, tip, 0.62);
        triangle(b, c, tip, 0.46);
        triangle(c, a, tip, 0.72);
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
}

pub(super) fn reset_run(world: &mut World) {
    let entities: Vec<_> = world
        .query_filtered::<Entity, With<FrozenArt>>()
        .iter(world)
        .collect();
    for entity in entities {
        world.despawn(entity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<GameSession>()
            .init_resource::<SkillRuntime>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_systems(Update, sync);
        app.world_mut().run_system_once(setup).unwrap();
        app
    }
    fn freeze(app: &mut App, target: Entity) {
        app.world_mut()
            .resource_mut::<SkillRuntime>()
            .freeze_relic(target, 1.0, false, false, 1.0);
    }
    fn target(app: &mut App) -> Entity {
        let entity = app
            .world_mut()
            .spawn((
                Enemy {
                    hp: 100.0,
                    max_hp: 100.0,
                    speed: 1.0,
                    damage: 1.0,
                    xp: 1,
                    wood: 0,
                    gold: 1,
                    kill_credit: 1,
                    elite: false,
                },
                Transform::from_xyz(2.0, 0.0, 3.0),
            ))
            .id();
        freeze(app, entity);
        entity
    }
    fn effects(app: &mut App) -> Vec<(Entity, Entity)> {
        app.world_mut()
            .query::<(Entity, &FrozenArt)>()
            .iter(app.world())
            .map(|(entity, frozen)| (entity, frozen.0))
            .collect()
    }

    #[test]
    fn repeated_freeze_shares_assets_and_missing_targets_leave_no_ghosts() {
        let mut app = app();
        let target = target(&mut app);
        let missing = app.world_mut().spawn_empty().id();
        freeze(&mut app, missing);
        app.world_mut().despawn(missing);
        app.update();
        let first = effects(&mut app);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].1, target);
        let counts = (
            app.world().resource::<Assets<Mesh>>().len(),
            app.world().resource::<Assets<StandardMaterial>>().len(),
        );
        for _ in 0..8 {
            freeze(&mut app, target);
            app.update();
        }
        assert_eq!(effects(&mut app), first);
        assert_eq!(
            counts,
            (
                app.world().resource::<Assets<Mesh>>().len(),
                app.world().resource::<Assets<StandardMaterial>>().len()
            )
        );
        app.world_mut().despawn(target);
        app.update();
        assert!(effects(&mut app).is_empty());
    }

    #[test]
    fn freeze_cap_and_pause_preserve_transforms_and_stop_new_shells() {
        let mut app = app();
        let first_target = target(&mut app);
        app.update();
        let effect = effects(&mut app)[0].0;
        let original = *app.world().get::<Transform>(effect).unwrap();
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.world_mut()
            .get_mut::<Transform>(first_target)
            .unwrap()
            .translation
            .x += 10.0;
        for _ in 0..40 {
            target(&mut app);
        }
        app.update();
        assert_eq!(effects(&mut app).len(), 1);
        assert_eq!(*app.world().get::<Transform>(effect).unwrap(), original);
        app.world_mut().resource_mut::<GameSession>().paused = false;
        app.update();
        assert_eq!(effects(&mut app).len(), MAX_FROZEN_ART);
        assert_eq!(
            app.world().get::<Transform>(effect).unwrap().translation,
            app.world()
                .get::<Transform>(first_target)
                .unwrap()
                .translation
        );
        let art = app.world().resource::<FrostArt>();
        let mesh = art.mesh.clone();
        let material = art.material.clone();
        assert!(
            app.world_mut()
                .query::<(&FrozenArt, &Mesh3d, &MeshMaterial3d<StandardMaterial>)>()
                .iter(app.world())
                .all(|(_, m, mat)| m.0 == mesh && mat.0 == material)
        );
        app.update();
        assert_eq!(effects(&mut app).len(), MAX_FROZEN_ART);
    }

    #[test]
    fn expiration_death_finish_and_reset_clean_up_even_while_paused() {
        let mut app = app();
        let expired = target(&mut app);
        let dead = target(&mut app);
        let live = target(&mut app);
        app.update();
        assert_eq!(effects(&mut app).len(), 3);
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.world_mut()
            .resource_mut::<SkillRuntime>()
            .controls
            .get_mut(&expired)
            .unwrap()
            .frozen_remaining = 0.0;
        app.world_mut().get_mut::<Enemy>(dead).unwrap().hp = 0.0;
        app.update();
        let remaining = effects(&mut app);
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].1, live);
        app.world_mut().resource_mut::<GameSession>().finished = true;
        app.update();
        assert!(effects(&mut app).is_empty());
        {
            let mut session = app.world_mut().resource_mut::<GameSession>();
            session.finished = false;
            session.paused = false;
        }
        app.update();
        assert_eq!(effects(&mut app).len(), 1);
        reset_run(app.world_mut());
        assert!(effects(&mut app).is_empty());
    }
}
