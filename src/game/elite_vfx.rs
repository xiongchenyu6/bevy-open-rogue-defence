//! Readable elite signatures and a cave-local warning driven by real raid state.
use super::combat::{
    BLOODRITE_RADIUS, EliteAffix, EliteAffixKind, RaidWarning, YELLOW_SIGN_RADIUS,
};
use super::pooling::CombatPools;
use super::{Enemy, GameSession};
use bevy::{
    asset::RenderAssetUsages,
    light::{NotShadowCaster, NotShadowReceiver},
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

const CREST_TILT: f32 = 0.8;

pub(crate) struct EliteVfxPlugin;
impl Plugin for EliteVfxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameSession>()
            .init_resource::<RaidWarning>()
            .init_resource::<CombatPools>()
            .add_systems(Startup, setup)
            .add_systems(
                PostUpdate,
                (attach, update).chain().before(TransformSystems::Propagate),
            );
    }
}
#[derive(Resource)]
struct Art {
    glyphs: [Handle<Mesh>; 8],
    colors: [Handle<StandardMaterial>; 8],
    ring: Handle<Mesh>,
}
#[derive(Component)]
struct Crest {
    owner: Entity,
}
#[derive(Component)]
struct Field {
    owner: Entity,
}
#[derive(Component)]
struct RaidArt;
fn index(kind: EliteAffixKind) -> usize {
    match kind {
        EliteAffixKind::Frenzy => 0,
        EliteAffixKind::Bulwark => 1,
        EliteAffixKind::Volatile => 2,
        EliteAffixKind::Regeneration => 3,
        EliteAffixKind::Commander => 4,
        EliteAffixKind::YellowSign => 5,
        EliteAffixKind::Brood => 6,
        EliteAffixKind::Siege => 7,
    }
}
fn mesh(paths: &[Vec<Vec3>]) -> Mesh {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for path in paths {
        for pair in path.windows(2) {
            let side = (pair[1] - pair[0]).cross(Vec3::Y).normalize_or_zero() * 0.055;
            let start = vertices.len() as u32;
            vertices.extend(
                [
                    pair[0] - side,
                    pair[0] + side,
                    pair[1] + side,
                    pair[1] - side,
                ]
                .map(|p| p.to_array()),
            );
            indices.extend([start, start + 2, start + 1, start, start + 3, start + 2]);
        }
    }
    let count = vertices.len();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; count])
    .with_inserted_indices(Indices::U32(indices))
}
fn points(p: &[(f32, f32)]) -> Vec<Vec3> {
    p.iter().map(|&(x, z)| Vec3::new(x, 0.0, z)).collect()
}
fn circle(x: f32, z: f32, r: f32) -> Vec<Vec3> {
    (0..=40)
        .map(|i| {
            let a = i as f32 * std::f32::consts::TAU / 40.0;
            Vec3::new(x + a.cos() * r, 0.0, z + a.sin() * r)
        })
        .collect()
}
fn signature(i: usize) -> Mesh {
    let paths = match i {
        0 => vec![
            points(&[(-0.7, -0.7), (-0.2, 0.1), (-0.45, 0.15), (0.1, 0.75)]),
            points(&[(0.0, -0.7), (0.5, 0.1), (0.25, 0.15), (0.7, 0.75)]),
        ],
        1 => vec![
            points(&[
                (-0.65, -0.6),
                (0.65, -0.6),
                (0.55, 0.25),
                (0.0, 0.8),
                (-0.55, 0.25),
                (-0.65, -0.6),
            ]),
            points(&[(0.0, -0.55), (0.0, 0.5)]),
        ],
        2 => (0..8)
            .map(|n| {
                let a = n as f32 * std::f32::consts::TAU / 8.0;
                points(&[
                    (a.cos() * 0.2, a.sin() * 0.2),
                    (a.cos() * 0.75, a.sin() * 0.75),
                ])
            })
            .collect(),
        3 => vec![
            points(&[(0.0, 0.75), (0.0, -0.65)]),
            points(&[(-0.6, 0.15), (0.0, -0.25), (0.6, 0.15)]),
            points(&[(-0.45, -0.45), (0.0, -0.05), (0.45, -0.45)]),
        ],
        4 => vec![points(&[
            (-0.7, 0.45),
            (-0.7, -0.4),
            (-0.35, 0.0),
            (0.0, -0.75),
            (0.35, 0.0),
            (0.7, -0.4),
            (0.7, 0.45),
            (-0.7, 0.45),
        ])],
        5 => vec![
            circle(0.0, 0.0, 0.65),
            points(&[(-0.55, 0.55), (0.55, -0.55)]),
            points(&[(0.0, -0.5), (-0.3, 0.1), (0.3, 0.1), (0.0, 0.5)]),
        ],
        6 => vec![
            circle(-0.35, 0.2, 0.32),
            circle(0.35, 0.2, 0.32),
            circle(0.0, -0.4, 0.32),
        ],
        _ => vec![
            points(&[
                (-0.65, -0.65),
                (0.45, -0.65),
                (0.45, -0.2),
                (-0.65, -0.2),
                (-0.65, -0.65),
            ]),
            points(&[(-0.1, -0.2), (-0.1, 0.7)]),
            points(&[(-0.5, 0.7), (0.3, 0.7)]),
        ],
    };
    mesh(&paths)
}
fn setup(
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };
    let colors = [
        (1.0, 0.25, 0.05),
        (0.2, 0.85, 1.0),
        (1.0, 0.08, 0.03),
        (0.1, 1.0, 0.35),
        (1.0, 0.7, 0.08),
        (1.0, 0.95, 0.1),
        (0.9, 0.25, 1.0),
        (0.7, 0.8, 1.0),
    ]
    .map(|(r, g, b)| {
        materials.add(StandardMaterial {
            base_color: Color::srgba(r, g, b, 0.85),
            emissive: LinearRgba::rgb(r * 2.0, g * 2.0, b * 2.0),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        })
    });
    let ring = meshes.add(mesh(&[circle(0.0, 0.0, 0.945)]));
    let raid = meshes.add(mesh(
        &[-0.65, 0.0, 0.65].map(|x| points(&[(x - 0.25, -0.6), (x, 0.25), (x + 0.25, -0.6)])),
    ));
    commands
        .spawn((
            Name::new("Raid cave warning"),
            RaidArt,
            Mesh3d(raid),
            MeshMaterial3d(colors[2].clone()),
            Transform::default(),
            Visibility::Hidden,
            NotShadowCaster,
            NotShadowReceiver,
        ))
        .with_children(|parent| {
            parent.spawn((
                Name::new("Raid warning ground pulse"),
                Mesh3d(ring.clone()),
                MeshMaterial3d(colors[2].clone()),
                Transform::from_xyz(0.0, 0.015, 0.0),
                NotShadowCaster,
                NotShadowReceiver,
            ));
        });
    commands.insert_resource(Art {
        glyphs: std::array::from_fn(|i| meshes.add(signature(i))),
        colors,
        ring,
    });
}
fn attach(
    mut commands: Commands,
    art: Option<Res<Art>>,
    mut pools: ResMut<CombatPools>,
    elites: Query<(Entity, &EliteAffix), Added<EliteAffix>>,
) {
    let Some(art) = art else { return };
    for (owner, affix) in &elites {
        let i = index(affix.kind());
        let crest = commands
            .spawn((
                Name::new("Elite affix crest"),
                Crest { owner },
                Mesh3d(art.glyphs[i].clone()),
                MeshMaterial3d(art.colors[i].clone()),
                Transform::from_xyz(0.0, 2.0, 0.0).with_scale(Vec3::splat(0.65)),
                Visibility::Inherited,
                NotShadowCaster,
                NotShadowReceiver,
            ))
            .id();
        commands.entity(owner).add_child(crest);
        pools.track_enemy_decoration(owner, crest);
        let field = commands
            .spawn((
                Name::new("Elite affix field"),
                Field { owner },
                Mesh3d(art.ring.clone()),
                MeshMaterial3d(art.colors[i].clone()),
                Transform::from_xyz(0.0, 0.07, 0.0),
                Visibility::Hidden,
                NotShadowCaster,
                NotShadowReceiver,
            ))
            .id();
        commands.entity(owner).add_child(field);
        pools.track_enemy_decoration(owner, field);
    }
}
fn update(
    session: Res<GameSession>,
    raid: Res<RaidWarning>,
    elites: Query<
        (&Enemy, &EliteAffix, &Transform),
        (Without<Crest>, Without<Field>, Without<RaidArt>),
    >,
    mut crests: Query<
        (&Crest, &mut Transform, &mut Visibility),
        (Without<Field>, Without<RaidArt>),
    >,
    mut fields: Query<
        (&Field, &mut Transform, &mut Visibility),
        (Without<Crest>, Without<RaidArt>),
    >,
    mut warnings: Query<
        (&mut Transform, &mut Visibility),
        (With<RaidArt>, Without<Crest>, Without<Field>),
    >,
) {
    for (crest, mut transform, mut visibility) in &mut crests {
        let Ok((enemy, _, body)) = elites.get(crest.owner) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        if session.finished || enemy.hp <= 0.0 {
            *visibility = Visibility::Hidden;
            continue;
        }
        if session.paused || session.save_active {
            continue;
        }
        *visibility = Visibility::Inherited;
        transform.rotation = body.rotation.inverse() * Quat::from_rotation_x(CREST_TILT);
    }
    for (field, mut transform, mut visibility) in &mut fields {
        let Ok((enemy, affix, body)) = elites.get(field.owner) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        if session.finished || enemy.hp <= 0.0 {
            *visibility = Visibility::Hidden;
            continue;
        }
        if session.paused || session.save_active {
            continue;
        }
        let radius = field_radius(affix.kind(), affix.shield_active());
        *visibility = if radius.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if let Some(radius) = radius {
            transform.scale = Vec3::new(radius / body.scale.x, 1.0, radius / body.scale.z)
        }
    }
    for (mut transform, mut visibility) in &mut warnings {
        if session.finished || raid.remaining_secs <= 0.0 {
            *visibility = Visibility::Hidden;
            continue;
        }
        if session.paused || session.save_active {
            continue;
        }
        *visibility = Visibility::Visible;
        *transform = raid_transform(&raid);
    }
}
fn field_radius(kind: EliteAffixKind, shield_active: bool) -> Option<f32> {
    match kind {
        EliteAffixKind::YellowSign => Some(YELLOW_SIGN_RADIUS),
        EliteAffixKind::Regeneration => Some(BLOODRITE_RADIUS),
        EliteAffixKind::Bulwark if shield_active => Some(1.1),
        _ => None,
    }
}
fn raid_transform(raid: &RaidWarning) -> Transform {
    // Keep the warning on the open road, in front of the cave's rock silhouette.
    Transform::from_xyz(raid.origin.x - 7.5, 0.10, raid.origin.z)
        .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2))
        .with_scale(Vec3::splat(2.0 + (raid.remaining_secs * 12.0).sin() * 0.25))
}
pub(crate) fn reset_run(world: &mut World) {
    let entities = world
        .query_filtered::<Entity, With<RaidArt>>()
        .iter(world)
        .collect::<Vec<_>>();
    for entity in entities {
        world.entity_mut(entity).insert(Visibility::Hidden);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signatures_are_unique_and_bounded() {
        let meshes = (0..8).map(signature).collect::<Vec<_>>();
        for (i, a) in meshes.iter().enumerate() {
            let positions = a
                .attribute(Mesh::ATTRIBUTE_POSITION)
                .unwrap()
                .as_float3()
                .unwrap();
            for b in &meshes[..i] {
                assert_ne!(
                    positions,
                    b.attribute(Mesh::ATTRIBUTE_POSITION)
                        .unwrap()
                        .as_float3()
                        .unwrap()
                )
            }
        }
    }
    #[test]
    fn crest_normal_faces_the_production_camera_instead_of_becoming_a_thin_line() {
        let normal = Quat::from_rotation_x(CREST_TILT) * Vec3::Y;
        let camera_direction = Vec3::new(8.0, 24.0, 26.0).normalize();
        assert!(normal.dot(camera_direction) > 0.95);
        // The prior negative tilt put the glyph almost edge-on to this view.
        let prior_normal = Quat::from_rotation_x(-0.8) * Vec3::Y;
        assert!(prior_normal.dot(camera_direction).abs() < 0.10);
    }
    #[test]
    fn warning_is_in_front_of_the_cave_and_healing_uses_the_real_radius() {
        let raid = RaidWarning {
            wave: 3,
            remaining_secs: 2.0,
            count: 8,
            origin: Vec3::new(16.0, 0.0, 0.0),
        };
        let transform = raid_transform(&raid);
        assert_eq!(transform.translation, Vec3::new(8.5, 0.10, 0.0));
        assert!(transform.scale.x >= 1.75 && transform.scale.x <= 2.25);
        assert_eq!(
            field_radius(EliteAffixKind::Regeneration, false),
            Some(BLOODRITE_RADIUS)
        );
        assert_eq!(BLOODRITE_RADIUS, 3.5);
        assert_eq!(
            field_radius(EliteAffixKind::YellowSign, false),
            Some(YELLOW_SIGN_RADIUS)
        );
        assert_eq!(field_radius(EliteAffixKind::Bulwark, false), None);
    }
    #[test]
    fn reset_hides_the_reusable_warning_root() {
        let mut world = World::new();
        let entity = world.spawn((RaidArt, Visibility::Visible)).id();
        reset_run(&mut world);
        assert_eq!(world.get::<Visibility>(entity), Some(&Visibility::Hidden));
        assert!(world.get::<RaidArt>(entity).is_some());
    }
    #[test]
    fn raid_warning_freezes_on_pause_and_hides_on_finish() {
        let mut app = App::new();
        app.init_resource::<GameSession>()
            .init_resource::<RaidWarning>()
            .add_systems(Update, update);
        let entity = app
            .world_mut()
            .spawn((RaidArt, Transform::default(), Visibility::Hidden))
            .id();
        app.world_mut().insert_resource(RaidWarning {
            wave: 3,
            remaining_secs: 2.0,
            count: 9,
            origin: Vec3::new(16.0, 0.0, 0.0),
        });
        app.update();
        assert_eq!(
            app.world().get::<Visibility>(entity),
            Some(&Visibility::Visible)
        );
        let transform = *app.world().get::<Transform>(entity).unwrap();
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.world_mut().resource_mut::<RaidWarning>().remaining_secs = 1.0;
        app.update();
        assert_eq!(*app.world().get::<Transform>(entity).unwrap(), transform);
        app.world_mut().resource_mut::<GameSession>().finished = true;
        app.update();
        assert_eq!(
            app.world().get::<Visibility>(entity),
            Some(&Visibility::Hidden)
        );
    }
    #[test]
    fn partial_app_has_no_render_resource_dependency() {
        let mut app = App::new();
        app.add_plugins(EliteVfxPlugin);
        app.update();
    }
}
