//! Boss identities drawn from the exact warning geometry, never cosmetic hitboxes.
use super::bosses::{BossDangerShape, BossPhaseCue, BossTelegraph};
use super::pooling::CombatPools;
use super::{BossEnemy, Enemy, GameSession, ImpactFx};
use bevy::{
    asset::RenderAssetUsages,
    light::{NotShadowCaster, NotShadowReceiver},
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

#[path = "boss_status.rs"]
mod status;
pub(crate) struct BossVfxPlugin;
impl Plugin for BossVfxPlugin {
    fn build(&self, app: &mut App) {
        status::install(app);
        app.add_message::<BossPhaseCue>()
            .init_resource::<GameSession>()
            .init_resource::<CombatPools>()
            .add_systems(Startup, setup)
            .add_systems(
                PostUpdate,
                (attach, sync, phases)
                    .chain()
                    .before(TransformSystems::Propagate),
            );
    }
}

#[derive(Resource)]
struct Art {
    circle: Handle<Mesh>,
    disc: Handle<Mesh>,
    lane: Handle<Mesh>,
    rectangle: Handle<Mesh>,
    identities: [[Handle<Mesh>; 2]; 5],
    colors: [[Handle<StandardMaterial>; 4]; 5],
    spike: Handle<Mesh>,
}
#[derive(Component)]
struct Drawn;
#[derive(Component)]
struct DangerArt;
#[derive(Component)]
struct DangerLayer(u8);
#[derive(Component)]
struct PhaseCrest {
    boss: Entity,
    index: u8,
}

#[derive(Component)]
pub(crate) struct BossPhaseBurstOwner(pub(crate) Entity);

pub(crate) fn reset_run(world: &mut World) {
    if let Some(mut cues) = world.get_resource_mut::<Messages<BossPhaseCue>>() {
        cues.clear();
    }
    let stale = world
        .query_filtered::<Entity, Or<(With<PhaseCrest>, With<BossTelegraph>)>>()
        .iter(world)
        .collect::<Vec<_>>();
    for entity in stale {
        world.despawn(entity);
    }
}

fn ribbon(paths: &[Vec<Vec3>], width: f32) -> Mesh {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for path in paths {
        for pair in path.windows(2) {
            let side = (pair[1] - pair[0]).cross(Vec3::Y).normalize_or_zero() * width * 0.5;
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
fn polar(angle: f32, radius: f32) -> Vec3 {
    Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius)
}
fn loop_path(sides: usize, radius: f32) -> Vec<Vec3> {
    (0..=sides)
        .map(|i| polar(i as f32 * std::f32::consts::TAU / sides as f32, radius))
        .collect()
}
fn identity(index: usize, lane: bool) -> Mesh {
    let mut paths = Vec::new();
    if lane {
        for z in [-0.65, 0.0, 0.65] {
            let path = if index == 1 {
                (0..=16)
                    .map(|i| {
                        let x = -0.80 + i as f32 * 0.1;
                        Vec3::new(x, 0.0, z + (x * 8.0).sin() * 0.06)
                    })
                    .collect()
            } else {
                vec![
                    Vec3::new(-0.60, 0.0, z - 0.12),
                    Vec3::new(0.0, 0.0, z + 0.12),
                    Vec3::new(0.60, 0.0, z - 0.12),
                ]
            };
            paths.push(path);
        }
    } else {
        match index {
            1 => {
                for radius in [0.35, 0.65] {
                    paths.push(
                        (0..=48)
                            .map(|i| {
                                let a = i as f32 * std::f32::consts::TAU / 48.0;
                                polar(a, radius + (a * 5.0).sin() * 0.04)
                            })
                            .collect(),
                    );
                }
            }
            2 => {
                for offset in [0.0, std::f32::consts::PI] {
                    paths.push(
                        (0..=3)
                            .map(|i| polar(offset + i as f32 * std::f32::consts::TAU / 3.0, 0.78))
                            .collect(),
                    );
                }
            }
            _ => {
                let arms = [6, 0, 0, 8, 5][index];
                for i in 0..arms {
                    let a = i as f32 * std::f32::consts::TAU / arms as f32;
                    paths.push(vec![polar(a, 0.18), polar(a + 0.12, 0.46), polar(a, 0.80)]);
                    if index == 4 {
                        paths.push(vec![
                            polar(a - 0.22, 0.64),
                            polar(a + 0.12, 0.46),
                            polar(a + 0.25, 0.67),
                        ]);
                    }
                }
                if index == 3 {
                    paths.push(loop_path(8, 0.23));
                }
            }
        }
    }
    ribbon(&paths, 0.045)
}
fn fill_circle() -> Mesh {
    let mut points = vec![[0.0, 0.0, 0.0]];
    points.extend(loop_path(64, 0.96).iter().map(|p| p.to_array()));
    let mut indices = Vec::new();
    for i in 1..=64 {
        indices.extend([0, i + 1, i]);
    }
    let count = points.len();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, points)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; count])
    .with_inserted_indices(Indices::U32(indices))
}
fn setup(
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };
    let palette = [
        (1.0, 0.18, 0.04),
        (0.05, 0.72, 1.0),
        (0.74, 0.10, 1.0),
        (1.0, 0.70, 0.06),
        (0.12, 1.0, 0.22),
    ];
    let colors = palette.map(|(r, g, b)| {
        [0.12, 0.28, 0.75, 1.0].map(|alpha| {
            materials.add(StandardMaterial {
                base_color: Color::srgba(r, g, b, alpha),
                emissive: LinearRgba::rgb(r * 2.8, g * 2.8, b * 2.8),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                cull_mode: None,
                ..default()
            })
        })
    });
    let rectangle_path = vec![
        Vec3::new(-0.97, 0.0, -0.97),
        Vec3::new(0.97, 0.0, -0.97),
        Vec3::new(0.97, 0.0, 0.97),
        Vec3::new(-0.97, 0.0, 0.97),
        Vec3::new(-0.97, 0.0, -0.97),
    ];
    let rectangle = meshes.add(Cuboid::new(1.92, 0.006, 1.92));
    commands.insert_resource(Art {
        circle: meshes.add(ribbon(&[loop_path(64, 0.97)], 0.06)),
        disc: meshes.add(fill_circle()),
        lane: meshes.add(ribbon(&[rectangle_path], 0.06)),
        rectangle,
        identities: std::array::from_fn(|i| {
            [
                meshes.add(identity(i, false)),
                meshes.add(identity(i, true)),
            ]
        }),
        colors,
        spike: meshes.add(Cone::new(0.10, 0.75)),
    });
}

fn attach(
    mut commands: Commands,
    art: Option<Res<Art>>,
    dangers: Query<(Entity, &BossTelegraph), Without<Drawn>>,
    bosses: Query<(Entity, &BossEnemy), Added<BossEnemy>>,
    crests: Query<(Entity, &PhaseCrest)>,
) {
    let Some(art) = art else { return };
    for (entity, danger) in &dangers {
        let index = usize::from(danger.index.min(4));
        let lane = matches!(danger.shape, BossDangerShape::Lane { .. });
        commands
            .entity(entity)
            .insert(Drawn)
            .with_children(|parent| {
                parent
                    .spawn((
                        Name::new("Boss danger art"),
                        DangerArt,
                        Transform::default(),
                        Visibility::default(),
                    ))
                    .with_children(|parent| {
                        for (layer, mesh, name) in [
                            (
                                0,
                                if lane { &art.lane } else { &art.circle },
                                "Boss danger border",
                            ),
                            (
                                1,
                                if lane { &art.rectangle } else { &art.disc },
                                "Boss danger fill",
                            ),
                            (
                                2,
                                &art.identities[index][usize::from(lane)],
                                "Boss danger identity",
                            ),
                        ] {
                            parent.spawn((
                                Name::new(name),
                                DangerLayer(layer),
                                Mesh3d(mesh.clone()),
                                MeshMaterial3d(
                                    art.colors[index][if layer == 1 { 0 } else { 2 }].clone(),
                                ),
                                Transform::from_xyz(0.0, layer as f32 * 0.005, 0.0),
                                NotShadowCaster,
                                NotShadowReceiver,
                            ));
                        }
                    });
            });
    }
    for (boss, identity) in &bosses {
        if let Some((entity, _)) = crests.iter().find(|(_, crest)| crest.boss == boss) {
            commands.entity(entity).insert(Visibility::Hidden);
            continue;
        }
        let index = identity.index.min(4);
        commands.entity(boss).with_children(|parent| {
            parent
                .spawn((
                    Name::new("Boss phase crest"),
                    PhaseCrest { boss, index },
                    Transform::from_xyz(0.0, 2.3, 0.0),
                    Visibility::Hidden,
                    Mesh3d(art.identities[index as usize][0].clone()),
                    MeshMaterial3d(art.colors[index as usize][3].clone()),
                    NotShadowCaster,
                    NotShadowReceiver,
                ))
                .with_children(|parent| {
                    for side in [-1.0, 1.0] {
                        parent.spawn((
                            Mesh3d(art.spike.clone()),
                            MeshMaterial3d(art.colors[index as usize][3].clone()),
                            Transform::from_xyz(side * 0.52, 0.2, 0.0),
                            NotShadowCaster,
                        ));
                    }
                });
        });
    }
}

fn danger_transform(origin: Vec3, target: Vec3, shape: BossDangerShape, radius: f32) -> Transform {
    match shape {
        BossDangerShape::Circle => Transform::from_translation(target.with_y(0.06))
            .with_scale(Vec3::new(radius, 1.0, radius)),
        BossDangerShape::Lane { half_width, length } => {
            let direction = (target - origin).with_y(0.0).normalize_or(Vec3::NEG_X);
            Transform::from_translation((origin + direction * length * 0.5).with_y(0.06))
                .with_rotation(Quat::from_rotation_arc(Vec3::Z, direction))
                .with_scale(Vec3::new(half_width, 1.0, length * 0.5))
        }
    }
}
fn sync(
    session: Res<GameSession>,
    art: Option<Res<Art>>,
    dangers: Query<(&BossTelegraph, &Children)>,
    mut roots: Query<(&mut Transform, &Children), With<DangerArt>>,
    mut layers: Query<(&DangerLayer, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    if session.paused || session.save_active {
        return;
    }
    let Some(art) = art else { return };
    for (danger, children) in &dangers {
        for child in children {
            let Ok((mut transform, layers_children)) = roots.get_mut(*child) else {
                continue;
            };
            *transform =
                danger_transform(danger.origin, danger.target, danger.shape, danger.radius);
            let index = usize::from(danger.index.min(4));
            for child in layers_children {
                if let Ok((layer, mut material)) = layers.get_mut(*child) {
                    let color = if layer.0 == 1 {
                        usize::from(danger.active)
                    } else if danger.active || danger.enraged {
                        3
                    } else {
                        2
                    };
                    material.0 = art.colors[index][color].clone();
                }
            }
        }
    }
}
fn phases(
    mut commands: Commands,
    session: Res<GameSession>,
    art: Option<Res<Art>>,
    mut cues: MessageReader<BossPhaseCue>,
    mut pools: ResMut<CombatPools>,
    bosses: Query<&Enemy, With<BossEnemy>>,
    mut crests: Query<(&PhaseCrest, &mut Visibility)>,
    bursts: Query<(Entity, &BossPhaseBurstOwner)>,
) {
    for (entity, owner) in &bursts {
        if session.finished || !bosses.get(owner.0).is_ok_and(|enemy| enemy.hp > 0.0) {
            pools.retire_impact_fx(&mut commands, entity);
            commands.entity(entity).remove::<BossPhaseBurstOwner>();
        }
    }
    for (crest, mut visibility) in &mut crests {
        if session.finished || !bosses.get(crest.boss).is_ok_and(|enemy| enemy.hp > 0.0) {
            *visibility = Visibility::Hidden;
        }
    }
    if session.paused || session.finished || session.save_active {
        return;
    }
    let Some(art) = art else { return };
    for cue in cues.read() {
        if !bosses.get(cue.boss).is_ok_and(|enemy| enemy.hp > 0.0) {
            continue;
        }
        let index = usize::from(cue.index.min(4));
        for (crest, mut visibility) in &mut crests {
            if crest.boss == cue.boss && crest.index == cue.index {
                *visibility = Visibility::Visible;
            }
        }
        for (mesh, position, scale) in [
            (
                art.identities[index][0].clone(),
                cue.origin.with_y(0.18),
                Vec3::splat(0.8),
            ),
            (
                art.circle.clone(),
                cue.origin.with_y(0.08),
                Vec3::splat(0.5),
            ),
        ] {
            if let Some(entity) = pools.rent_named_impact_fx(
                &mut commands,
                mesh,
                art.colors[index][3].clone(),
                Transform::from_translation(position).with_scale(scale),
                ImpactFx {
                    lifetime: 0.85,
                    start_scale: 0.5,
                    end_scale: 3.8,
                },
                Some(Name::new("Boss phase burst")),
            ) {
                commands
                    .entity(entity)
                    .insert(BossPhaseBurstOwner(cue.boss));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    #[test]
    fn paused_death_retires_owned_burst_and_reuse_cannot_kill_another_effect() {
        let mut app = App::new();
        app.add_plugins(BossVfxPlugin)
            .add_systems(Last, super::super::pooling::finalize_recycles);
        let boss = app
            .world_mut()
            .spawn((
                BossEnemy { index: 0 },
                Enemy {
                    hp: 100.0,
                    max_hp: 100.0,
                    speed: 1.0,
                    damage: 1.0,
                    xp: 0,
                    wood: 0,
                    gold: 0,
                    kill_credit: 1,
                    elite: true,
                },
            ))
            .id();
        let effect = app
            .world_mut()
            .run_system_once(
                move |mut commands: Commands, mut pools: ResMut<CombatPools>| {
                    let entity = pools
                        .rent_named_impact_fx(
                            &mut commands,
                            Handle::default(),
                            Handle::default(),
                            Transform::default(),
                            ImpactFx {
                                lifetime: 0.85,
                                start_scale: 0.5,
                                end_scale: 3.8,
                            },
                            Some(Name::new("Boss phase burst")),
                        )
                        .unwrap();
                    commands.entity(entity).insert(BossPhaseBurstOwner(boss));
                    entity
                },
            )
            .unwrap();
        app.update();
        assert!(app.world().get::<ImpactFx>(effect).is_some());
        app.world_mut().entity_mut(boss).remove::<BossEnemy>();
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.update();
        assert!(app.world().get::<BossPhaseBurstOwner>(effect).is_none());
        assert!(app.world().get::<ImpactFx>(effect).is_none());
        assert_eq!(
            app.world().get::<Visibility>(effect),
            Some(&Visibility::Hidden)
        );
        // Rent also clears stale owner state defensively, before any late cleanup
        // can interpret a new effect as belonging to the defeated boss.
        app.world_mut()
            .entity_mut(effect)
            .insert(BossPhaseBurstOwner(boss));
        let reused = app
            .world_mut()
            .run_system_once(|mut commands: Commands, mut pools: ResMut<CombatPools>| {
                pools
                    .rent_named_impact_fx(
                        &mut commands,
                        Handle::default(),
                        Handle::default(),
                        Transform::default(),
                        ImpactFx {
                            lifetime: 1.0,
                            start_scale: 0.1,
                            end_scale: 1.0,
                        },
                        Some(Name::new("OTHER EFFECT")),
                    )
                    .unwrap()
            })
            .unwrap();
        assert_eq!(reused, effect);
        app.update();
        assert!(app.world().get::<BossPhaseBurstOwner>(reused).is_none());
        assert!(app.world().get::<ImpactFx>(reused).is_some());
        assert_eq!(
            app.world().get::<Name>(reused).unwrap().as_str(),
            "OTHER EFFECT"
        );
    }
    #[test]
    fn partial_app_needs_no_renderer_and_hides_retired_crest_even_while_paused() {
        let mut app = App::new();
        app.add_plugins(BossVfxPlugin);
        let boss = app.world_mut().spawn_empty().id();
        let crest = app
            .world_mut()
            .spawn((PhaseCrest { boss, index: 0 }, Visibility::Visible))
            .id();
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.update();
        assert_eq!(
            app.world().get::<Visibility>(crest),
            Some(&Visibility::Hidden)
        );
    }
    #[test]
    fn warning_geometry_is_the_real_circle_and_forward_lane() {
        let origin = Vec3::new(5.0, 0.0, 2.0);
        let target = Vec3::new(-5.0, 0.0, 2.0);
        let circle = danger_transform(origin, target, BossDangerShape::Circle, 3.4);
        assert_eq!(circle.translation, target.with_y(0.06));
        assert_eq!(circle.scale.x, 3.4);
        let lane = danger_transform(
            origin,
            target,
            BossDangerShape::Lane {
                half_width: 2.2,
                length: 16.0,
            },
            2.2,
        );
        assert!(lane.transform_point(Vec3::NEG_Z).xz().distance(origin.xz()) < 0.0001);
        assert!(
            lane.transform_point(Vec3::Z)
                .xz()
                .distance((origin - Vec3::X * 16.0).xz())
                < 0.0001
        );
        assert!(
            (lane
                .transform_point(Vec3::X)
                .xz()
                .distance(lane.translation.xz())
                - 2.2)
                .abs()
                < 0.0001
        );
    }
    #[test]
    fn new_run_removes_old_crests_and_phase_messages() {
        let mut world = World::new();
        world.init_resource::<Messages<BossPhaseCue>>();
        let boss = world.spawn_empty().id();
        let crest = world.spawn(PhaseCrest { boss, index: 0 }).id();
        world
            .resource_mut::<Messages<BossPhaseCue>>()
            .write(BossPhaseCue {
                boss,
                index: 0,
                origin: Vec3::ZERO,
            });
        reset_run(&mut world);
        assert!(world.get_entity(crest).is_err());
        assert!(world.resource::<Messages<BossPhaseCue>>().is_empty());
        assert!(world.get_entity(boss).is_ok());
    }
    #[test]
    fn five_element_signatures_have_distinct_meshes_inside_the_real_boundary() {
        let signatures: Vec<_> = (0..5).map(|i| identity(i, false)).collect();
        for (i, mesh) in signatures.iter().enumerate() {
            let positions = mesh
                .attribute(Mesh::ATTRIBUTE_POSITION)
                .unwrap()
                .as_float3()
                .unwrap();
            assert!(positions.iter().all(|p| p[0].hypot(p[2]) < 0.9));
            for other in &signatures[..i] {
                assert_ne!(
                    mesh.attribute(Mesh::ATTRIBUTE_POSITION),
                    other.attribute(Mesh::ATTRIBUTE_POSITION)
                );
            }
        }
    }
}
