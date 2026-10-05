//! Wild creature attacks render from their real damage areas, not main-boss AI.
use super::encounter_bosses::{
    ADVENTURE_BOSSES, AdventureDanger, AdventureDangerShape, AdventureImpactCue,
    AdventureSkillClock,
};
use super::{ArenaEntity, EncounterBoss, Enemy, GameLanguage, GameSession};
use bevy::{
    asset::RenderAssetUsages,
    light::{NotShadowCaster, NotShadowReceiver},
    mesh::Indices,
    prelude::*,
};

pub(super) struct AdventureVfxPlugin;
impl Plugin for AdventureVfxPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(PostUpdate, (spawn, impacts, update, status).chain());
    }
}
#[derive(Resource)]
struct Art {
    circle: Handle<Mesh>,
    fan: Handle<Mesh>,
    wave: Handle<Mesh>,
    shard: Handle<Mesh>,
    cloud: Handle<Mesh>,
    warning: Handle<StandardMaterial>,
    colors: [Handle<StandardMaterial>; 3],
}
#[derive(Component)]
struct Visual {
    danger: Entity,
    owner: Entity,
    linger: f32,
}
#[derive(Component)]
struct Burst;
#[derive(Component)]
struct Flash {
    owner: Entity,
    remaining: f32,
}
#[derive(Component)]
struct BossStatus;
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let warning = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.23, 0.07, 0.20),
        emissive: LinearRgba::rgb(0.8, 0.12, 0.015),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    let colors = [(1.0, 0.24, 0.025), (0.07, 0.68, 1.0), (0.30, 0.95, 0.09)].map(|(r, g, b)| {
        materials.add(StandardMaterial {
            base_color: Color::srgba(r, g, b, 0.65),
            emissive: LinearRgba::rgb(r * 3.0, g * 3.0, b * 3.0),
            alpha_mode: AlphaMode::Add,
            unlit: true,
            cull_mode: None,
            ..default()
        })
    });
    let fan = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![[0.0, 0.0, 0.0], [-1.0, 0.0, -1.0], [1.0, 0.0, -1.0]],
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; 3])
    .with_inserted_indices(Indices::U32(vec![0, 2, 1]));
    commands.insert_resource(Art {
        circle: meshes.add(Cylinder::new(1.0, 0.025)),
        fan: meshes.add(fan),
        wave: meshes.add(Cuboid::new(2.0, 1.0, 1.0)),
        shard: meshes.add(Cone::new(0.12, 1.0)),
        cloud: meshes.add(Sphere::new(0.18)),
        warning,
        colors,
    });
    commands.spawn((
        Name::new("Adventure boss status"),
        BossStatus,
        Node {
            position_type: PositionType::Absolute,
            left: percent(32.0),
            bottom: px(78.0),
            width: percent(45.0),
            ..default()
        },
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.8, 0.52)),
        Visibility::Hidden,
    ));
}
fn spawn(
    mut commands: Commands,
    art: Res<Art>,
    dangers: Query<(Entity, &AdventureDanger), Added<AdventureDanger>>,
) {
    for (entity, danger) in &dangers {
        let mesh = match danger.shape {
            AdventureDangerShape::Circle { .. } => art.circle.clone(),
            AdventureDangerShape::Cone { .. } => art.fan.clone(),
            AdventureDangerShape::Wave { .. } => art.wave.clone(),
        };
        commands
            .spawn((
                Name::new(format!(
                    "Wild attack visual {}/{}",
                    danger.index, danger.kind
                )),
                ArenaEntity,
                NotShadowCaster,
                NotShadowReceiver,
                Visual {
                    danger: entity,
                    owner: danger.owner,
                    linger: 0.35,
                },
                Mesh3d(mesh),
                MeshMaterial3d(art.warning.clone()),
                Transform::default(),
            ))
            .with_children(|root| {
                for n in 0..9 {
                    let x = (n % 3) as f32 - 1.0;
                    let z = (n / 3) as f32 - 1.0;
                    let mesh = if danger.index == 2 {
                        art.cloud.clone()
                    } else {
                        art.shard.clone()
                    };
                    root.spawn((
                        Burst,
                        NotShadowCaster,
                        NotShadowReceiver,
                        Mesh3d(mesh),
                        MeshMaterial3d(art.colors[danger.index as usize].clone()),
                        Transform::from_xyz(x * 0.55, 0.25 + (n % 3) as f32 * 0.18, z * 0.45)
                            .with_scale(Vec3::new(
                                0.7,
                                if danger.index == 1 { 1.6 } else { 0.8 },
                                0.7,
                            )),
                        Visibility::Hidden,
                    ));
                }
            });
    }
}
fn update(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    art: Res<Art>,
    dangers: Query<&AdventureDanger>,
    bosses: Query<(), With<EncounterBoss>>,
    mut visuals: Query<(
        Entity,
        &mut Visual,
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &Children,
    )>,
    mut bursts: Query<&mut Visibility, With<Burst>>,
    mut flashes: Query<(Entity, &mut Flash)>,
) {
    for (entity, mut flash) in &mut flashes {
        if !session.paused {
            flash.remaining -= time.delta_secs();
        }
        if session.finished || flash.remaining <= 0.0 || !bosses.contains(flash.owner) {
            commands.entity(entity).despawn();
        }
    }
    for (entity, mut visual, mut transform, mut material, children) in &mut visuals {
        if session.finished || !bosses.contains(visual.owner) {
            commands.entity(entity).despawn();
            continue;
        }
        let Ok(danger) = dangers.get(visual.danger) else {
            if !session.paused {
                visual.linger -= time.delta_secs();
            }
            if visual.linger <= 0.0 {
                commands.entity(entity).despawn();
            }
            continue;
        };
        let direction = (danger.target - danger.origin)
            .with_y(0.0)
            .normalize_or(Vec3::NEG_X);
        let (position, scale) = match danger.shape {
            AdventureDangerShape::Circle { radius } => {
                (danger.target.with_y(0.05), Vec3::new(radius, 1.0, radius))
            }
            AdventureDangerShape::Cone { range, half_angle } => (
                danger.origin.with_y(0.05),
                Vec3::new(range * half_angle.tan(), 1.0, range),
            ),
            AdventureDangerShape::Wave { half_width, length } => (
                (danger.origin + direction * length * 0.5).with_y(if danger.active {
                    0.45
                } else {
                    0.035
                }),
                Vec3::new(
                    half_width,
                    if danger.active { 0.8 } else { 0.04 },
                    length.max(0.05),
                ),
            ),
        };
        *transform = Transform::from_translation(position)
            .looking_to(direction, Vec3::Y)
            .with_scale(scale);
        material.0 = if danger.active {
            art.colors[danger.index as usize].clone()
        } else {
            art.warning.clone()
        };
        for child in children.iter() {
            if let Ok(mut visibility) = bursts.get_mut(child) {
                *visibility = if danger.active {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
            }
        }
    }
}
fn impacts(mut commands: Commands, art: Res<Art>, mut cues: MessageReader<AdventureImpactCue>) {
    for cue in cues.read() {
        let direction = (cue.target - cue.origin)
            .with_y(0.0)
            .normalize_or(Vec3::NEG_X);
        let (mesh, position, scale) = match cue.shape {
            AdventureDangerShape::Circle { radius } => (
                art.circle.clone(),
                cue.target.with_y(0.06),
                Vec3::new(radius, 1.0, radius),
            ),
            AdventureDangerShape::Cone { range, half_angle } => (
                art.fan.clone(),
                cue.origin.with_y(0.07),
                Vec3::new(range * half_angle.tan(), 1.0, range),
            ),
            AdventureDangerShape::Wave { half_width, length } => (
                art.wave.clone(),
                (cue.origin + direction * length * 0.5).with_y(0.4),
                Vec3::new(half_width, 0.65, length.max(0.1)),
            ),
        };
        commands
            .spawn((
                Name::new(format!("Wild attack impact {}/{}", cue.index, cue.kind)),
                ArenaEntity,
                NotShadowCaster,
                NotShadowReceiver,
                Flash {
                    owner: cue.owner,
                    remaining: 0.4,
                },
                Mesh3d(mesh),
                MeshMaterial3d(art.colors[cue.index as usize].clone()),
                Transform::from_translation(position)
                    .looking_to(direction, Vec3::Y)
                    .with_scale(scale),
            ))
            .with_children(|root| {
                for n in 0..12 {
                    let angle = n as f32 * std::f32::consts::TAU / 12.0;
                    root.spawn((
                        NotShadowCaster,
                        NotShadowReceiver,
                        Mesh3d(if cue.index == 2 {
                            art.cloud.clone()
                        } else {
                            art.shard.clone()
                        }),
                        MeshMaterial3d(art.colors[cue.index as usize].clone()),
                        Transform::from_xyz(
                            angle.cos() * 0.8,
                            0.4 + (n % 3) as f32 * 0.3,
                            angle.sin() * 0.8,
                        )
                        .with_rotation(Quat::from_rotation_z(angle * 0.3)),
                    ));
                }
            });
    }
}
fn status(
    session: Res<GameSession>,
    language: Res<GameLanguage>,
    bosses: Query<(&EncounterBoss, &Enemy, &AdventureSkillClock)>,
    mut labels: Query<(&mut Text, &mut Visibility), With<BossStatus>>,
) {
    let Ok((mut text, mut visibility)) = labels.single_mut() else {
        return;
    };
    let Some((boss, enemy, phase)) = bosses.iter().next().filter(|_| !session.finished) else {
        *visibility = Visibility::Hidden;
        return;
    };
    let def = &ADVENTURE_BOSSES[boss.index as usize];
    *visibility = Visibility::Visible;
    text.0 = format!(
        "{}  {:.0}%{}",
        language.pick(def.name_en, def.name_zh),
        100.0 * enemy.hp.max(0.0) / enemy.max_hp.max(1.0),
        if phase.enraged {
            language.pick(" · ENRAGED", " · 狂暴")
        } else {
            ""
        }
    );
}
