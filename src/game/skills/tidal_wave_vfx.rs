//! A single broad curling water front follows the authoritative moving wave.
//! Meshes and materials are shared; each wave has 29 children and no gameplay damage.
use super::{GameSession, tidal_wave::TidalWave};
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};
const MAX_DECORATED_WAVES: usize = 8;
const SEGMENTS: usize = 64;
#[derive(Resource)]
struct WaveArt {
    wall: Handle<Mesh>,
    lip: Handle<Mesh>,
    foam: Handle<Mesh>,
    wake: Handle<Mesh>,
    splash: Handle<Mesh>,
    water: Handle<StandardMaterial>,
    glass: Handle<StandardMaterial>,
    white: Handle<StandardMaterial>,
    wash: Handle<StandardMaterial>,
}
#[derive(Component)]
struct Decorated;
#[derive(Component)]
struct WavePart {
    owner: Entity,
    kind: PartKind,
    base: Transform,
}
#[derive(Clone, Copy)]
enum PartKind {
    Wall,
    Lip,
    Foam(f32),
    Wake,
    Ripple(f32),
    Splash(u8),
}
pub(super) fn install(app: &mut App) {
    app.add_systems(Startup, setup).add_systems(
        PostUpdate,
        (attach, animate)
            .chain()
            .before(TransformSystems::Propagate),
    );
}
// Local -Z is forward. The middle of the crest leads its trailing wings.
fn crest(t: f32) -> Vec3 {
    Vec3::new(
        t * 0.5,
        1.65 - 0.32 * t.abs() + 0.07 * (t * 19.0).sin(),
        -0.65 * (1.0 - t * t),
    )
}
fn surface(kind: u8) -> Mesh {
    let rows = if kind == 2 { 2 } else { 9 };
    let mut positions = Vec::<[f32; 3]>::new();
    let mut normals = Vec::<[f32; 3]>::new();
    let mut colors = Vec::<[f32; 4]>::new();
    let mut uv = Vec::<[f32; 2]>::new();
    let mut indices = Vec::new();
    for i in 0..=SEGMENTS {
        let t = i as f32 / SEGMENTS as f32 * 2.0 - 1.0;
        let top = crest(t);
        for j in 0..rows {
            let v = j as f32 / (rows - 1) as f32;
            let (position, color) = match kind {
                // Climbing, concave water sheet from the ground into the lip.
                0 => (
                    Vec3::new(
                        top.x,
                        0.04 + top.y * v.powf(1.45),
                        top.z + 2.7 * (1.0 - v).powi(2),
                    ),
                    [0.015, 0.24 + v * 0.32, 0.68 + v * 0.24, 0.62 + v * 0.16],
                ),
                // Curl rolls forward and slightly down over the top edge.
                1 => (
                    Vec3::new(
                        top.x,
                        top.y + 0.32 * (v * std::f32::consts::PI).sin() - v * 0.2,
                        top.z - v * 0.68,
                    ),
                    [0.035, 0.58, 0.92, 0.76],
                ),
                // A broad, irregular but continuous whitewater ribbon.
                2 => (
                    Vec3::new(
                        top.x,
                        top.y + 0.05 + (v - 0.5) * 0.14,
                        top.z + (v - 0.5) * (0.3 + 0.09 * (t * 31.0).sin()),
                    ),
                    [0.48, 0.88, 1.0, 0.48 + 0.28 * (t * 37.0).sin().powi(2)],
                ),
                // Broad visible blue wash: fade only the trailing fringe, not
                // the whole sheet (the previous squared 0.24 alpha vanished).
                _ => (
                    Vec3::new(
                        top.x * (1.0 - 0.25 * v),
                        0.035 + 0.025 * (t * 13.0 + v * 9.0).sin(),
                        top.z + 0.4 + v * 7.0,
                    ),
                    [
                        0.015,
                        0.38 + 0.06 * (t * 13.0 + v * 9.0).sin(),
                        0.79,
                        (0.70 - v * 0.20) * ((1.0 - v) * 5.0).min(1.0),
                    ],
                ),
            };
            positions.push(position.to_array());
            normals.push([0.0, 0.8, -0.6]);
            colors.push(color);
            uv.push([i as f32 / SEGMENTS as f32, v]);
        }
    }
    for i in 0..SEGMENTS {
        for j in 0..rows - 1 {
            let a = (i * rows + j) as u32;
            let b = a + rows as u32;
            indices.extend([a, b, a + 1, a + 1, b, b + 1]);
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
    .with_inserted_indices(Indices::U32(indices))
}
fn material(
    materials: &mut Assets<StandardMaterial>,
    emission: LinearRgba,
) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive: emission,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        perceptual_roughness: 0.18,
        metallic: 0.05,
        ..default()
    })
}
fn setup(
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };
    commands.insert_resource(WaveArt {
        wall: meshes.add(surface(0)),
        lip: meshes.add(surface(1)),
        foam: meshes.add(surface(2)),
        wake: meshes.add(surface(3)),
        splash: meshes.add(Cone::new(0.06, 0.65)),
        water: material(&mut materials, LinearRgba::rgb(0.005, 0.12, 0.30)),
        glass: material(&mut materials, LinearRgba::rgb(0.015, 0.28, 0.48)),
        white: material(&mut materials, LinearRgba::rgb(0.18, 0.65, 0.85)),
        wash: material(&mut materials, LinearRgba::rgb(0.005, 0.15, 0.36)),
    });
}
#[allow(clippy::too_many_arguments)]
fn child(
    parent: &mut ChildSpawnerCommands,
    owner: Entity,
    name: &'static str,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    kind: PartKind,
    base: Transform,
) {
    parent.spawn((
        Name::new(name),
        Mesh3d(mesh),
        MeshMaterial3d(material),
        base,
        WavePart { owner, kind, base },
    ));
}
fn attach(
    mut commands: Commands,
    session: Res<GameSession>,
    art: Option<Res<WaveArt>>,
    waves: Query<(Entity, &TidalWave), Without<Decorated>>,
    existing: Query<(), With<Decorated>>,
) {
    if session.paused || session.finished {
        return;
    }
    let Some(art) = art else {
        return;
    };
    let mut capacity = MAX_DECORATED_WAVES.saturating_sub(existing.iter().count());
    for (owner, wave) in &waves {
        if capacity == 0 {
            break;
        }
        capacity -= 1;
        let width = wave.half_width * 2.0;
        commands
            .entity(owner)
            .insert(Decorated)
            .with_children(|parent| {
                let height = 1.0 + f32::from(wave.evolution) * 0.15;
                let broad = Transform::from_scale(Vec3::new(width, height, 1.0));
                child(
                    parent,
                    owner,
                    "Tidal wave curved wall",
                    art.wall.clone(),
                    art.water.clone(),
                    PartKind::Wall,
                    broad,
                );
                child(
                    parent,
                    owner,
                    "Tidal wave curling lip",
                    art.lip.clone(),
                    art.glass.clone(),
                    PartKind::Lip,
                    broad,
                );
                child(
                    parent,
                    owner,
                    "Tidal wave whitewater crest",
                    art.foam.clone(),
                    art.white.clone(),
                    PartKind::Foam(0.0),
                    broad,
                );
                child(
                    parent,
                    owner,
                    "Tidal wave lower whitewater",
                    art.foam.clone(),
                    art.white.clone(),
                    PartKind::Foam(1.7),
                    Transform::from_xyz(0.0, -0.35 * height, 0.58).with_scale(Vec3::new(
                        width,
                        0.73 * height,
                        0.7,
                    )),
                );
                child(
                    parent,
                    owner,
                    "Tidal wave water carpet",
                    art.wake.clone(),
                    art.wash.clone(),
                    PartKind::Wake,
                    broad,
                );
                for i in 0..4 {
                    child(
                        parent,
                        owner,
                        "Tidal wave trailing ripple",
                        art.foam.clone(),
                        art.wash.clone(),
                        PartKind::Ripple(i as f32),
                        Transform::from_xyz(0.0, 0.06, 0.8 + i as f32 * 1.15)
                            .with_scale(Vec3::new(width * (0.95 - i as f32 * 0.07), 0.025, 0.65)),
                    );
                }
                for i in 0..20 {
                    let t = (f32::from(i) / 19.0) * 2.0 - 1.0;
                    let p = crest(t);
                    let base = Transform::from_xyz(p.x * width, p.y * height, p.z)
                        .with_scale(Vec3::new(1.0, 0.5, 1.0));
                    child(
                        parent,
                        owner,
                        "Tidal wave crest spray",
                        art.splash.clone(),
                        art.white.clone(),
                        PartKind::Splash(i),
                        base,
                    );
                }
            });
    }
}
fn animate(
    session: Res<GameSession>,
    waves: Query<&TidalWave>,
    mut parts: Query<(&WavePart, &mut Transform)>,
) {
    if session.paused || session.finished {
        return;
    }
    for (part, mut transform) in &mut parts {
        let Ok(wave) = waves.get(part.owner) else {
            continue;
        };
        let age = wave.travelled / 14.0;
        let envelope =
            (wave.travelled / 1.2).clamp(0.1, 1.0) * (wave.remaining / 0.16).clamp(0.1, 1.0);
        *transform = part.base;
        match part.kind {
            PartKind::Wall | PartKind::Lip => transform.scale.y *= envelope,
            PartKind::Foam(offset) => {
                transform.translation.y += (age * 11.0 + offset).sin() * 0.045;
                transform.scale.y *= envelope;
            }
            PartKind::Wake => transform.scale.z = (wave.travelled / 7.0).clamp(0.05, 1.0),
            PartKind::Ripple(index) => {
                transform.translation.z += ((age * 1.8 + index * 0.23) % 1.0) * 0.7;
                transform.scale.x *= 1.0 + 0.015 * (age * 7.0 + index).sin();
            }
            PartKind::Splash(index) => {
                let phase = (age * 2.8 + f32::from(index) * 0.173).fract();
                let side = f32::from(index % 2) * 2.0 - 1.0;
                transform.translation.y += phase * 1.5 - phase * phase * 1.1;
                transform.translation.z -= phase * 0.9;
                transform.translation.x += side * phase * 0.18;
                transform.rotation =
                    Quat::from_rotation_z(side * 0.25) * Quat::from_rotation_x(-phase * 1.1);
                transform.scale = Vec3::new(0.5, 0.25 + 0.5 * (1.0 - phase), 0.5) * envelope;
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn water_children_share_assets_freeze_and_follow_owner_cleanup() {
        use bevy::ecs::system::RunSystemOnce;
        let mut app = App::new();
        app.init_resource::<GameSession>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>();
        app.world_mut().run_system_once(setup).unwrap();
        let mesh_count = app.world().resource::<Assets<Mesh>>().len();
        let material_count = app.world().resource::<Assets<StandardMaterial>>().len();
        app.world_mut()
            .run_system_once(|mut commands: Commands| {
                super::super::tidal_wave::spawn(&mut commands, Vec3::ZERO, Vec3::X, 5, 10.0, 0)
            })
            .unwrap();
        app.world_mut().run_system_once(attach).unwrap();
        let owner = app
            .world_mut()
            .query_filtered::<Entity, With<TidalWave>>()
            .single(app.world())
            .unwrap();
        assert_eq!(
            app.world_mut()
                .query::<&WavePart>()
                .iter(app.world())
                .count(),
            29
        );
        assert_eq!(app.world().resource::<Assets<Mesh>>().len(), mesh_count);
        assert_eq!(
            app.world().resource::<Assets<StandardMaterial>>().len(),
            material_count
        );
        let before: Vec<_> = app
            .world_mut()
            .query_filtered::<(Entity, &Transform), With<WavePart>>()
            .iter(app.world())
            .map(|(e, t)| (e, *t))
            .collect();
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.world_mut()
            .get_mut::<TidalWave>(owner)
            .unwrap()
            .travelled = 5.0;
        app.world_mut().run_system_once(animate).unwrap();
        for (entity, transform) in before {
            assert_eq!(*app.world().get::<Transform>(entity).unwrap(), transform);
        }
        app.world_mut().entity_mut(owner).despawn();
        assert_eq!(
            app.world_mut()
                .query::<&WavePart>()
                .iter(app.world())
                .count(),
            0
        );
    }
    #[test]
    fn authored_water_is_a_wide_continuous_surface_not_individual_projectiles() {
        use bevy::mesh::VertexAttributeValues;
        let mesh = surface(0);
        let Some(VertexAttributeValues::Float32x3(points)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("water positions");
        };
        assert_eq!(points.len(), (SEGMENTS + 1) * 9);
        let min_x = points.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
        let max_x = points
            .iter()
            .map(|p| p[0])
            .fold(f32::NEG_INFINITY, f32::max);
        assert_eq!((min_x, max_x), (-0.5, 0.5));
        assert!(points.iter().any(|p| p[1] > 1.6));
        assert_eq!(mesh.indices().unwrap().len(), SEGMENTS * 8 * 6);
    }
}
