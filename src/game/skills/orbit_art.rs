//! The passive orbit weapon is a bladed chakram, never the shared bullet mesh.
use super::*;
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
};

const MAX_TRAILS: usize = 96;
#[derive(Resource)]
pub(super) struct OrbitArt {
    body: Handle<Mesh>,
    edge: Handle<Mesh>,
    metal: Handle<StandardMaterial>,
    glow: Handle<StandardMaterial>,
}
#[derive(Component)]
struct BladeWake {
    previous: Vec3,
    remaining: f32,
}
#[derive(Component)]
struct Trail {
    remaining: f32,
    scale: Vec3,
}
pub(super) fn install(app: &mut App) {
    app.add_systems(Startup, setup)
        .add_systems(PostUpdate, trail.before(TransformSystems::Propagate));
}
pub(super) fn scale(rank: u8) -> f32 {
    1.0 + f32::from(rank.clamp(1, 5) - 1) * 0.13
}

/// Extruded quadrilateral keeps broad metal faces readable from oblique cameras.
fn tooth(points: [Vec2; 4], height: f32) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    let vertex = |point: Vec2, y: f32| Vec3::new(point.x, y, point.y);
    let faces = [
        [
            vertex(points[3], height),
            vertex(points[2], height),
            vertex(points[1], height),
            vertex(points[0], height),
        ],
        [
            vertex(points[0], -height),
            vertex(points[1], -height),
            vertex(points[2], -height),
            vertex(points[3], -height),
        ],
        [
            vertex(points[0], -height),
            vertex(points[1], -height),
            vertex(points[1], height),
            vertex(points[0], height),
        ],
        [
            vertex(points[1], -height),
            vertex(points[2], -height),
            vertex(points[2], height),
            vertex(points[1], height),
        ],
        [
            vertex(points[2], -height),
            vertex(points[3], -height),
            vertex(points[3], height),
            vertex(points[2], height),
        ],
        [
            vertex(points[3], -height),
            vertex(points[0], -height),
            vertex(points[0], height),
            vertex(points[3], height),
        ],
    ];
    for face in faces {
        let base = positions.len() as u32;
        let normal = (face[1] - face[0])
            .cross(face[2] - face[0])
            .normalize_or(Vec3::Y);
        for point in face {
            positions.push(point.to_array());
            normals.push(normal.to_array());
        }
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    let count = positions.len();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; count])
    .with_inserted_indices(Indices::U32(indices))
}
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut body = Mesh::from(Torus::new(0.22, 0.37));
    let mut edge = Mesh::from(Torus::new(0.34, 0.37));
    for i in 0..4 {
        let rotation = Quat::from_rotation_y(i as f32 * std::f32::consts::FRAC_PI_2);
        body.merge(
            &tooth(
                [
                    Vec2::new(0.25, -0.12),
                    Vec2::new(0.76, -0.05),
                    Vec2::new(0.86, 0.42),
                    Vec2::new(0.38, 0.17),
                ],
                0.075,
            )
            .rotated_by(rotation),
        )
        .expect("blade face attributes");
        edge.merge(
            &tooth(
                [
                    Vec2::new(0.76, -0.05),
                    Vec2::new(0.86, 0.42),
                    Vec2::new(0.79, 0.34),
                    Vec2::new(0.70, -0.04),
                ],
                0.08,
            )
            .rotated_by(rotation),
        )
        .expect("blade edge attributes");
    }
    commands.insert_resource(OrbitArt {
        body: meshes.add(body),
        edge: meshes.add(edge),
        metal: materials.add(StandardMaterial {
            base_color: Color::srgb(0.58, 0.70, 0.80),
            metallic: 0.45,
            perceptual_roughness: 0.35,
            cull_mode: None,
            ..default()
        }),
        glow: materials.add(StandardMaterial {
            base_color: Color::srgb(0.40, 0.93, 1.0),
            emissive: LinearRgba::new(1.6, 4.8, 6.0, 1.0),
            cull_mode: None,
            ..default()
        }),
    });
}
pub(super) fn spawn(commands: &mut Commands, art: &OrbitArt, position: Vec3, rank: u8, slot: u8) {
    commands
        .spawn((
            Name::new("Orbit blade"),
            Mesh3d(art.body.clone()),
            MeshMaterial3d(art.metal.clone()),
            Transform::from_translation(position).with_scale(Vec3::splat(scale(rank))),
            OrbitBlade { slot },
            BladeWake {
                previous: position,
                remaining: 0.0,
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                Name::new("Orbit blade luminous edge"),
                Mesh3d(art.edge.clone()),
                MeshMaterial3d(art.glow.clone()),
                Transform::default(),
            ));
        });
}
fn trail(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    build: Res<PlayerBuild>,
    vitals: Res<HeroVitals>,
    art: Option<Res<OrbitArt>>,
    mut blades: Query<(Entity, &Transform, &mut BladeWake), With<OrbitBlade>>,
    mut trails: Query<(Entity, &mut Trail, &mut Transform), Without<OrbitBlade>>,
) {
    if session.finished || !vitals.alive || build.skills[6] == 0 {
        for (entity, ..) in &blades {
            commands.entity(entity).despawn();
        }
        for (entity, ..) in &trails {
            commands.entity(entity).despawn();
        }
        return;
    }
    if session.paused {
        return;
    }
    let delta = time.delta_secs();
    let mut count = 0;
    for (entity, mut tail, mut transform) in &mut trails {
        tail.remaining -= delta;
        if tail.remaining <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        count += 1;
        transform.scale = tail.scale * (tail.remaining / 0.20).sqrt();
    }
    let Some(art) = art else {
        return;
    };
    for (_, transform, mut wake) in &mut blades {
        wake.remaining -= delta;
        if wake.remaining > 0.0 {
            continue;
        }
        wake.remaining = 0.045;
        let displacement = transform.translation - wake.previous;
        // No false line across the battlefield after a teleport or initial spawn.
        if displacement.length_squared() > 0.0001
            && displacement.length_squared() < 4.0
            && count < MAX_TRAILS
        {
            count += 1;
            let size = transform.scale * 0.88;
            commands.spawn((
                Name::new("Orbit blade wake"),
                Mesh3d(art.edge.clone()),
                MeshMaterial3d(art.glow.clone()),
                Transform::from_translation(wake.previous)
                    .with_rotation(transform.rotation)
                    .with_scale(size),
                Trail {
                    remaining: 0.20,
                    scale: size,
                },
            ));
        }
        wake.previous = transform.translation;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    #[test]
    fn orbit_blades_use_authored_mesh_grow_from_two_to_four_and_pause_cleanly() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Time>()
            .init_resource::<GameSession>()
            .init_resource::<PlayerBuild>()
            .init_resource::<HeroVitals>()
            .init_resource::<SkillRuntime>();
        app.world_mut().run_system_once(setup).unwrap();
        app.world_mut().spawn((Hero, Transform::default()));
        for (rank, expected, orbit_radius) in [(1, 2, 2.05), (3, 3, 3.075), (5, 4, 4.10)] {
            app.world_mut().resource_mut::<PlayerBuild>().skills[6] = rank;
            app.world_mut().run_system_once(sync_orbit_blades).unwrap();
            app.world_mut()
                .run_system_once(update_orbit_blades)
                .unwrap();
            let art = app.world().resource::<OrbitArt>();
            let body = art.body.clone();
            let edge = art.edge.clone();
            assert_ne!(body, edge);
            let mut query = app
                .world_mut()
                .query_filtered::<(&Mesh3d, &Transform), With<OrbitBlade>>();
            assert_eq!(query.iter(app.world()).count(), expected);
            for (mesh, transform) in query.iter(app.world()) {
                assert_eq!(mesh.0, body);
                assert_eq!(transform.scale, Vec3::splat(scale(rank)));
                assert!((transform.translation.with_y(0.0).length() - orbit_radius).abs() < 0.001);
            }
        }
        let before: Vec<_> = app
            .world_mut()
            .query_filtered::<&Transform, With<OrbitBlade>>()
            .iter(app.world())
            .cloned()
            .collect();
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.world_mut().resource_mut::<GameSession>().elapsed += 1.0;
        app.world_mut()
            .run_system_once(update_orbit_blades)
            .unwrap();
        let after: Vec<_> = app
            .world_mut()
            .query_filtered::<&Transform, With<OrbitBlade>>()
            .iter(app.world())
            .cloned()
            .collect();
        assert_eq!(before, after);
        app.world_mut().resource_mut::<GameSession>().finished = true;
        app.world_mut().run_system_once(trail).unwrap();
        assert_eq!(
            app.world_mut()
                .query::<&OrbitBlade>()
                .iter(app.world())
                .count(),
            0
        );
        assert_eq!(
            app.world_mut().query::<&Mesh3d>().iter(app.world()).count(),
            0,
            "blade edges must be removed with the parent"
        );
    }
}
