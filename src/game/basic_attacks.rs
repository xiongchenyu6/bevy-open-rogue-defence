//! Weapon-led basic attacks. Melee contact uses the same damage resolver as ranged hits.
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

use super::combat::{EliteAffix, queue_melee_contact};
use super::pooling::CombatPools;
use super::{Enemy, GameAssets, GameSession, Hero, HeroKind, HeroVitals};

mod class_mechanics;
pub(crate) use class_mechanics::next_ricochet_target;
pub use class_mechanics::{
    ClassBasicStats, RICOCHET_DAMAGE_RATIO, RICOCHET_RANGE, Ricochet, class_basic_stats,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BasicWeapon {
    Sword,
    Daggers,
    Axe,
    Crossbow,
    Storm,
    Spirit,
    Star,
    Frost,
}

pub(crate) const fn weapon(kind: HeroKind) -> BasicWeapon {
    match kind {
        HeroKind::StoneWarden | HeroKind::SolarVanguard => BasicWeapon::Sword,
        HeroKind::TideWalker | HeroKind::RiftDancer => BasicWeapon::Daggers,
        HeroKind::IronBerserker | HeroKind::VoidReaper => BasicWeapon::Axe,
        HeroKind::EmberRanger | HeroKind::GroveArtificer => BasicWeapon::Crossbow,
        HeroKind::StormSavant => BasicWeapon::Storm,
        HeroKind::SpiritKeeper => BasicWeapon::Spirit,
        HeroKind::StarWagerer => BasicWeapon::Star,
        HeroKind::FrostOracle => BasicWeapon::Frost,
    }
}

impl BasicWeapon {
    pub(crate) const fn melee(self) -> bool {
        matches!(self, Self::Sword | Self::Daggers | Self::Axe)
    }
    pub(crate) fn range(self, ranged_range: f32) -> f32 {
        match self {
            Self::Sword => 2.2,
            Self::Daggers => 1.8,
            Self::Axe => 2.6,
            _ => ranged_range,
        }
    }
    fn half_angle(self) -> f32 {
        match self {
            Self::Daggers => 0.72,
            Self::Axe => 1.05,
            _ => 0.88,
        }
    }
}

/// Visual growth follows the same level milestones as class basic attacks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BasicAttackVisualTier {
    pub(crate) index: usize,
    pub(crate) emissive: f32,
    pub(crate) projectile_scale: f32,
    pub(crate) intensity: f32,
}

pub(crate) const fn basic_attack_visual_tier(level: u32) -> BasicAttackVisualTier {
    let index = if level >= 40 {
        3
    } else if level >= 20 {
        2
    } else if level >= 10 {
        1
    } else {
        0
    };
    BasicAttackVisualTier {
        index,
        emissive: [0.15, 1.2, 3.5, 7.0][index],
        projectile_scale: [0.65, 0.8, 0.95, 1.1][index],
        intensity: [0.2, 0.45, 0.7, 1.0][index],
    }
}

#[derive(Resource, Default)]
pub(crate) struct BasicAttackAim(pub(crate) Option<Vec3>);

#[derive(Resource)]
pub(crate) struct BasicAttackArt {
    arrow: Handle<Mesh>,
    mechanical_bolt: Handle<Mesh>,
    storm: Handle<Mesh>,
    spirit: Handle<Mesh>,
    star: Handle<Mesh>,
    frost: Handle<Mesh>,
    melee_shapes: [Handle<Mesh>; 6],
    pub(crate) materials: [Handle<StandardMaterial>; HeroKind::COUNT],
    tier_materials: [[Handle<StandardMaterial>; HeroKind::COUNT]; 4],
}

impl BasicAttackArt {
    pub(crate) fn material_at_level(&self, kind: HeroKind, level: u32) -> Handle<StandardMaterial> {
        self.tier_materials[basic_attack_visual_tier(level).index][kind.index()].clone()
    }

    pub(crate) fn arrow(&self) -> (Handle<Mesh>, Handle<StandardMaterial>) {
        (
            self.arrow.clone(),
            self.materials[HeroKind::EmberRanger.index()].clone(),
        )
    }
    pub(crate) fn projectile(&self, kind: HeroKind) -> (Handle<Mesh>, Handle<StandardMaterial>) {
        self.projectile_at_level(kind, 1)
    }
    pub(crate) fn projectile_at_level(
        &self,
        kind: HeroKind,
        level: u32,
    ) -> (Handle<Mesh>, Handle<StandardMaterial>) {
        let mesh = match weapon(kind) {
            BasicWeapon::Crossbow if kind == HeroKind::GroveArtificer => &self.mechanical_bolt,
            BasicWeapon::Crossbow => &self.arrow,
            BasicWeapon::Storm => &self.storm,
            BasicWeapon::Spirit => &self.spirit,
            BasicWeapon::Star => &self.star,
            BasicWeapon::Frost => &self.frost,
            _ => unreachable!("melee weapons do not emit visible projectiles"),
        };
        (
            mesh.clone(),
            self.tier_materials[basic_attack_visual_tier(level).index][kind.index()].clone(),
        )
    }
}

pub(crate) fn setup_attack_art(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut arrow = Mesh::from(Cuboid::new(0.075, 0.075, 1.12));
    arrow
        .merge(&crystal(Vec3::new(0.26, 0.12, 0.52)).translated_by(Vec3::new(0.0, 0.0, -0.65)))
        .expect("flame arrowhead");
    for side in [-1.0, 1.0] {
        arrow
            .merge(&bolt_mesh(
                &[
                    Vec3::new(side * 0.08, 0.0, -0.2),
                    Vec3::new(side * 0.32, 0.0, 0.22),
                    Vec3::new(side * 0.12, 0.0, 0.56),
                ],
                0.09,
            ))
            .expect("flame arrow barbs");
    }
    let mut mechanical_bolt = Mesh::from(Cuboid::new(0.22, 0.18, 0.92));
    mechanical_bolt
        .merge(
            &Mesh::from(Cone::new(0.26, 0.48))
                .rotated_by(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
                .translated_by(Vec3::new(0.0, 0.0, -0.66)),
        )
        .expect("mechanical bolt head");
    for angle in [
        0.0,
        std::f32::consts::TAU / 3.0,
        std::f32::consts::TAU * 2.0 / 3.0,
    ] {
        mechanical_bolt
            .merge(
                &Mesh::from(Cuboid::new(0.55, 0.065, 0.4))
                    .rotated_by(Quat::from_rotation_z(angle))
                    .translated_by(Vec3::new(0.0, 0.0, 0.34)),
            )
            .expect("mechanical bolt fins");
    }
    let mut storm = bolt_mesh(
        &[
            Vec3::new(0.0, 0.0, -0.78),
            Vec3::new(-0.22, 0.0, -0.25),
            Vec3::new(0.16, 0.0, -0.30),
            Vec3::new(-0.13, 0.0, 0.30),
            Vec3::new(0.22, 0.0, 0.20),
            Vec3::new(0.0, 0.0, 0.73),
        ],
        0.10,
    );
    storm
        .merge(&bolt_mesh(
            &[
                Vec3::new(-0.13, 0.0, 0.30),
                Vec3::new(-0.40, 0.0, 0.05),
                Vec3::new(-0.32, 0.0, -0.15),
            ],
            0.05,
        ))
        .expect("lightning fork attributes agree");
    storm
        .merge(&crystal(Vec3::new(0.27, 0.23, 0.42)))
        .expect("storm seed mesh");
    let mut spirit = Mesh::from(Sphere::new(0.34));
    for side in [-1.0, 1.0] {
        let tail = bolt_mesh(
            &[
                Vec3::new(side * 0.09, 0.0, 0.02),
                Vec3::new(side * 0.25, 0.03, 0.29),
                Vec3::new(side * 0.17, 0.06, 0.55),
                Vec3::new(side * 0.33, 0.08, 0.83),
            ],
            0.065,
        );
        spirit.merge(&tail).expect("spirit tail attributes agree");
    }
    let mut star = star_mesh();
    star.merge(&Mesh::from(Torus::new(0.55, 0.60)))
        .expect("star orbit mesh");
    let mut frost = crystal(Vec3::new(0.16, 0.15, 0.72));
    for side in [-1.0, 1.0] {
        frost
            .merge(
                &crystal(Vec3::new(0.09, 0.10, 0.38))
                    .rotated_by(Quat::from_rotation_y(side * 0.3))
                    .translated_by(Vec3::new(side * 0.21, 0.03, 0.24)),
            )
            .expect("frost crystal attributes agree");
    }
    let colors = [
        [0.22, 0.80, 1.0],
        [1.0, 0.67, 0.30],
        [0.72, 0.93, 0.85],
        [0.40, 1.0, 0.56],
        [0.30, 0.84, 1.0],
        [1.0, 0.77, 0.28],
        [0.58, 0.91, 1.0],
        [1.0, 0.55, 0.30],
        [0.78, 0.32, 1.0],
        [0.63, 0.95, 0.42],
        [1.0, 0.83, 0.36],
        [0.65, 0.38, 0.95],
    ];
    let tier_materials = [1, 10, 20, 40].map(|level| {
        let tier = basic_attack_visual_tier(level);
        colors.map(|[r, g, b]| {
            materials.add(StandardMaterial {
                base_color: Color::srgb(r, g, b),
                emissive: LinearRgba::new(
                    r * tier.emissive,
                    g * tier.emissive,
                    b * tier.emissive,
                    1.0,
                ),
                unlit: false,
                alpha_mode: AlphaMode::Opaque,
                cull_mode: None,
                ..default()
            })
        })
    });
    let palette = tier_materials[0].clone();
    commands.insert_resource(BasicAttackArt {
        arrow: meshes.add(arrow),
        mechanical_bolt: meshes.add(mechanical_bolt),
        storm: meshes.add(storm.scaled_by(Vec3::splat(1.55))),
        spirit: meshes.add(spirit.scaled_by(Vec3::splat(1.50))),
        star: meshes.add(star.scaled_by(Vec3::splat(1.45))),
        frost: meshes.add(frost.scaled_by(Vec3::splat(1.65))),
        melee_shapes: melee_meshes().map(|mesh| meshes.add(mesh)),
        materials: palette,
        tier_materials,
    });
}

/// All projectile shapes face local -Z, matching the pooled projectile orientation.
fn bolt_mesh(points: &[Vec3], width: f32) -> Mesh {
    let segment = |a: Vec3, b: Vec3| {
        Mesh::from(Cuboid::new(width, width, a.distance(b)))
            .rotated_by(Quat::from_rotation_arc(Vec3::Z, (b - a).normalize()))
            .translated_by((a + b) * 0.5)
    };
    let mut mesh = segment(points[0], points[1]);
    for pair in points[1..].windows(2) {
        mesh.merge(&segment(pair[0], pair[1]))
            .expect("bolt segment attributes agree");
    }
    mesh
}

fn crystal(scale: Vec3) -> Mesh {
    Sphere::new(1.0)
        .mesh()
        .ico(0)
        .expect("icosahedron")
        .scaled_by(scale)
}

fn star_mesh() -> Mesh {
    let mut vertices = vec![[0.0, 0.0, 0.0]];
    let mut indices = Vec::new();
    for i in 0..12 {
        let angle = i as f32 * std::f32::consts::TAU / 12.0;
        let radius = if i % 2 == 0 { 0.48 } else { 0.16 };
        vertices.push([angle.sin() * radius, 0.0, angle.cos() * radius]);
        indices.extend([0, i + 1, (i + 1) % 12 + 1]);
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        vec![[0.0, 1.0, 0.0]; vertices.len()],
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; vertices.len()])
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
    .with_inserted_indices(Indices::U32(indices))
}

fn slash_mesh(half_angle: f32, thickness: f32, ripple: f32, teeth: f32) -> Mesh {
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    for step in 0..=24 {
        let t = step as f32 / 24.0;
        let angle = -half_angle + t * half_angle * 2.0;
        // sin(PI) can round slightly below zero; fractional powers would
        // otherwise create NaN vertices at the tapered blade endpoint.
        let width = thickness * (std::f32::consts::PI * t).sin().max(0.0).powf(0.7);
        let edge = 1.0 - ripple * (t * std::f32::consts::PI * teeth).sin().abs();
        for radius in [edge - width, edge] {
            vertices.push([angle.sin() * radius, 0.0, -angle.cos() * radius]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([step as f32 / 24.0, radius]);
        }
        if step < 24 {
            let i = step * 2;
            indices.extend([i, i + 1, i + 2, i + 1, i + 3, i + 2]);
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}

/// Geometry is authored per hero; the combat weapon categories stay unchanged.
fn melee_meshes() -> [Mesh; 6] {
    let stone = slash_mesh(1.16, 0.36, 0.13, 9.0);
    let mut solar = slash_mesh(1.0, 0.18, 0.0, 1.0);
    solar
        .merge(&crystal(Vec3::new(0.13, 0.09, 0.33)).translated_by(Vec3::new(0.0, 0.0, -0.84)))
        .expect("solar blade tip");
    let mut tide = slash_mesh(0.92, 0.16, 0.12, 3.0);
    tide.merge(
        &slash_mesh(0.82, 0.09, 0.09, 3.0)
            .scaled_by(Vec3::splat(0.74))
            .rotated_by(Quat::from_rotation_y(0.16)),
    )
    .expect("twin wave ribbon");
    let rift = bolt_mesh(
        &[
            Vec3::new(-0.78, 0.0, -0.45),
            Vec3::new(0.18, 0.0, -1.0),
            Vec3::new(0.75, 0.0, -0.55),
        ],
        0.13,
    );
    let iron = slash_mesh(1.45, 0.48, 0.05, 5.0);
    let mut void = slash_mesh(1.72, 0.18, 0.0, 1.0);
    void.merge(&bolt_mesh(
        &[
            Vec3::new(0.92, 0.0, 0.1),
            Vec3::new(0.67, 0.0, -0.25),
            Vec3::new(0.3, 0.0, -0.2),
        ],
        0.1,
    ))
    .expect("void scythe hook");
    [stone, solar, tide, rift, iron, void]
}

const fn melee_shape_index(kind: HeroKind) -> usize {
    match kind {
        HeroKind::StoneWarden => 0,
        HeroKind::SolarVanguard => 1,
        HeroKind::TideWalker => 2,
        HeroKind::RiftDancer => 3,
        HeroKind::IronBerserker => 4,
        HeroKind::VoidReaper => 5,
        _ => panic!("ranged hero has no swing mesh"),
    }
}

#[derive(Clone, Copy)]
struct SwingStyle {
    duration: f32,
    sweep: f32,
    layers: u8,
    start: f32,
    end: f32,
    tilt: f32,
    advance: f32,
}

const fn swing_style(kind: HeroKind) -> SwingStyle {
    match kind {
        HeroKind::StoneWarden => SwingStyle {
            duration: 0.28,
            sweep: 1.25,
            layers: 1,
            start: 0.60,
            end: 1.0,
            tilt: 0.0,
            advance: 0.0,
        },
        HeroKind::SolarVanguard => SwingStyle {
            duration: 0.22,
            sweep: 2.1,
            layers: 2,
            start: 0.78,
            end: 1.0,
            tilt: 0.42,
            advance: 0.0,
        },
        HeroKind::TideWalker => SwingStyle {
            duration: 0.22,
            sweep: 1.65,
            layers: 2,
            start: 0.58,
            end: 1.0,
            tilt: 0.12,
            advance: 0.12,
        },
        HeroKind::RiftDancer => SwingStyle {
            duration: 0.16,
            sweep: 2.6,
            layers: 3,
            start: 0.95,
            end: 0.68,
            tilt: 0.38,
            advance: 0.2,
        },
        HeroKind::IronBerserker => SwingStyle {
            duration: 0.32,
            sweep: 2.4,
            layers: 1,
            start: 0.66,
            end: 1.0,
            tilt: 0.18,
            advance: 0.0,
        },
        HeroKind::VoidReaper => SwingStyle {
            duration: 0.30,
            sweep: -2.25,
            layers: 2,
            start: 1.0,
            end: 0.34,
            tilt: 0.16,
            advance: 0.0,
        },
        _ => panic!("ranged hero has no swing style"),
    }
}

#[derive(Component)]
pub(crate) struct PendingSwing {
    pub(crate) hero: Entity,
    pub(crate) target: Entity,
    pub(crate) kind: HeroKind,
    pub(crate) direction: Vec3,
    pub(crate) damage: f32,
    pub(crate) hit_limit: usize,
    pub(crate) remaining: f32,
}

#[derive(Component)]
pub(crate) struct SwingTrail {
    remaining: f32,
    duration: f32,
    radius: f32,
    delay: f32,
    rotation: Quat,
    kind: HeroKind,
    layer: u8,
    origin: Vec3,
    direction: Vec3,
}

pub(crate) fn swing_windup(interval: f32) -> f32 {
    interval.clamp(0.03, 0.45) * 0.42
}

/// Targets are checked at weapon contact, not at attack start; a dodged swing misses.
pub(crate) fn melee_targets(
    origin: Vec3,
    direction: Vec3,
    range: f32,
    half_angle: f32,
    limit: usize,
    primary: Entity,
    candidates: &[(Entity, Vec3)],
) -> Vec<(Entity, Vec3)> {
    let forward = direction.with_y(0.0).normalize_or_zero();
    let mut hits: Vec<_> = candidates
        .iter()
        .copied()
        .filter(|(_, position)| {
            let delta = (*position - origin).with_y(0.0);
            delta.length_squared() <= range * range
                && (delta.length_squared() < 0.0001
                    || forward.dot(delta.normalize_or_zero()) >= half_angle.cos())
        })
        .collect();
    hits.sort_by(|a, b| {
        (a.0 != primary)
            .cmp(&(b.0 != primary))
            .then_with(|| {
                a.1.distance_squared(origin)
                    .total_cmp(&b.1.distance_squared(origin))
            })
            .then_with(|| a.0.to_bits().cmp(&b.0.to_bits()))
    });
    hits.truncate(limit);
    hits
}

pub(crate) fn resolve_swings(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    vitals: Res<HeroVitals>,
    build: Option<Res<super::PlayerBuild>>,
    mut cues: Option<ResMut<super::attack_vfx::WeaponCues>>,
    assets: Option<Res<GameAssets>>,
    art: Option<Res<BasicAttackArt>>,
    mut pools: ResMut<CombatPools>,
    heroes: Query<&Transform, With<Hero>>,
    enemies: Query<(Entity, &Transform, &Enemy, Option<&EliteAffix>)>,
    mut swings: Query<(
        Entity,
        &mut PendingSwing,
        Option<&super::hero_fusion::FusionAttack>,
    )>,
) {
    let candidates: Vec<_> = enemies
        .iter()
        .filter(|(_, _, enemy, affix)| {
            enemy.hp > 0.0 && !affix.is_some_and(|affix| affix.is_volatile_pending())
        })
        .map(|(entity, transform, ..)| (entity, transform.translation))
        .collect();
    for (entity, mut swing, fusion) in &mut swings {
        if session.finished
            || !vitals.can_act()
            || (swing.kind != vitals.kind && fusion.is_none())
            || !heroes.contains(swing.hero)
        {
            commands.entity(entity).despawn();
            continue;
        }
        if session.paused {
            continue;
        }
        swing.remaining -= time.delta_secs();
        if swing.remaining > 0.0 {
            continue;
        }
        let Ok(hero) = heroes.get(swing.hero) else {
            commands.entity(entity).despawn();
            continue;
        };
        let origin = fusion.map_or(hero.translation, |fusion| fusion.origin);
        let profile = weapon(swing.kind);
        let fallback = super::PlayerBuild::default();
        let stats = class_basic_stats(
            swing.kind,
            vitals.level,
            build.as_deref().unwrap_or(&fallback),
        );
        let range = stats.range;
        if let Some(art) = art.as_deref() {
            let style = swing_style(swing.kind);
            let mesh = &art.melee_shapes[melee_shape_index(swing.kind)];
            let duration = style.duration;
            for layer in 0..style
                .layers
                .min(basic_attack_visual_tier(vitals.level).index as u8 + 1)
            {
                let delay = layer as f32 * 0.045;
                let transform =
                    Transform::from_translation(origin + Vec3::Y * (0.65 + layer as f32 * 0.16))
                        .looking_to(swing.direction, Vec3::Y);
                commands.spawn((
                    Name::new("Weapon swing trail"),
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(
                        art.tier_materials[basic_attack_visual_tier(vitals.level).index]
                            [swing.kind.index()]
                        .clone(),
                    ),
                    transform.with_scale(Vec3::splat(range * style.start)),
                    if delay > 0.0 {
                        Visibility::Hidden
                    } else {
                        Visibility::Visible
                    },
                    SwingTrail {
                        remaining: duration + delay,
                        duration,
                        radius: range,
                        delay,
                        rotation: transform.rotation,
                        kind: swing.kind,
                        layer,
                        origin: transform.translation,
                        direction: swing.direction,
                    },
                ));
            }
        }
        if let Some(assets) = assets.as_deref() {
            let hits = melee_targets(
                origin,
                swing.direction,
                range,
                profile.half_angle(),
                swing.hit_limit,
                swing.target,
                &candidates,
            );
            if hits.len() > 1
                && let Some(cues) = cues.as_deref_mut()
            {
                cues.cleave(swing.kind, origin, swing.direction, range, stats.half_angle);
            }
            for (index, (target, position)) in hits.into_iter().enumerate() {
                let contact = queue_melee_contact(
                    &mut commands,
                    &mut pools,
                    assets,
                    target,
                    position,
                    swing.damage * if index == 0 { 1.0 } else { stats.cleave_ratio },
                    &candidates,
                );
                if let (Some(contact), Some(fusion)) = (contact, fusion) {
                    commands.entity(contact).insert((
                        *fusion,
                        super::hero_fusion::profile(swing.kind),
                        super::attack_vfx::WeaponFlight::new(swing.kind),
                    ));
                }
            }
        }
        commands.entity(entity).despawn();
    }
}

pub(crate) fn animate_swings(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    mut trails: Query<(Entity, &mut SwingTrail, &mut Transform, &mut Visibility)>,
) {
    if session.paused && !session.finished {
        return;
    }
    for (entity, mut trail, mut transform, mut visibility) in &mut trails {
        trail.remaining -= time.delta_secs();
        if trail.remaining <= 0.0 || session.finished {
            commands.entity(entity).despawn();
            continue;
        }
        trail.delay = (trail.delay - time.delta_secs()).max(0.0);
        if trail.delay > 0.0 {
            continue;
        }
        *visibility = Visibility::Visible;
        let progress = (1.0 - trail.remaining / trail.duration).clamp(0.0, 1.0);
        let ease = 1.0 - (1.0 - progress).powi(3);
        let style = swing_style(trail.kind);
        let radius = trail.radius * (style.start + (style.end - style.start) * ease);
        let reverse = if trail.layer % 2 == 0 { 1.0 } else { -1.0 };
        transform.scale = Vec3::new(radius, 1.0, radius * (1.0 - progress.powi(3) * 0.4));
        transform.translation = trail.origin
            + trail.direction * style.advance * (progress * std::f32::consts::PI).sin();
        transform.rotation = trail.rotation
            * Quat::from_rotation_y(style.sweep * reverse * (ease - 0.5))
            * Quat::from_rotation_z(style.tilt * reverse);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn visual_tiers_grow_at_class_milestones_and_materials_are_not_shared() {
        for (level, expected) in [(1, 0), (9, 0), (10, 1), (19, 1), (20, 2), (39, 2), (40, 3)] {
            assert_eq!(super::basic_attack_visual_tier(level).index, expected);
        }
        let mut app = bevy::prelude::App::new();
        app.init_resource::<bevy::prelude::Assets<bevy::prelude::Mesh>>()
            .init_resource::<bevy::prelude::Assets<bevy::prelude::StandardMaterial>>()
            .add_systems(bevy::prelude::Startup, super::setup_attack_art);
        app.update();
        let art = app.world().resource::<super::BasicAttackArt>();
        let materials = app
            .world()
            .resource::<bevy::prelude::Assets<bevy::prelude::StandardMaterial>>();
        let mut previous = 0.0;
        for level in [1, 10, 20, 40] {
            let (_, handle) = art.projectile_at_level(super::HeroKind::StormSavant, level);
            let emission = materials.get(&handle).unwrap().emissive.blue;
            assert!(emission > previous);
            assert_eq!(emission, super::basic_attack_visual_tier(level).emissive);
            previous = emission;
        }
        assert_ne!(
            art.projectile_at_level(super::HeroKind::StormSavant, 1).1,
            art.projectile_at_level(super::HeroKind::StormSavant, 40).1
        );
    }
    use super::*;
    #[test]
    fn attack_meshes_build_with_valid_merge_attributes() {
        use bevy::ecs::system::RunSystemOnce;
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>();
        app.world_mut().run_system_once(setup_attack_art).unwrap();
        let art = app.world().resource::<BasicAttackArt>();
        let mesh_assets = app.world().resource::<Assets<Mesh>>();
        for handle in [
            &art.arrow,
            &art.mechanical_bolt,
            &art.storm,
            &art.spirit,
            &art.star,
            &art.frost,
            &art.melee_shapes[0],
            &art.melee_shapes[1],
            &art.melee_shapes[2],
            &art.melee_shapes[3],
            &art.melee_shapes[4],
            &art.melee_shapes[5],
        ] {
            let mesh = mesh_assets.get(handle).unwrap();
            assert!(mesh.count_vertices() >= 13);
            assert!(mesh.indices().is_some());
        }
        // Starter mage attacks retain a readable silhouette without advanced HDR glow.
        // Unlit materials silently ignore emissive, which previously reduced the
        // effect to a tiny flat cyan mark even with Bloom enabled.
        for kind in [
            HeroKind::StormSavant,
            HeroKind::SpiritKeeper,
            HeroKind::StarWagerer,
            HeroKind::FrostOracle,
        ] {
            let (mesh, material) = art.projectile(kind);
            let positions = mesh_assets
                .get(&mesh)
                .unwrap()
                .attribute(Mesh::ATTRIBUTE_POSITION)
                .unwrap()
                .as_float3()
                .unwrap();
            let mut min = Vec3::splat(f32::MAX);
            let mut max = Vec3::splat(f32::MIN);
            for point in positions {
                let point = Vec3::from_array(*point);
                min = min.min(point);
                max = max.max(point);
            }
            assert!(
                (max - min).max_element() >= 1.4,
                "mage silhouette too small: {kind:?}"
            );
            let material = app
                .world()
                .resource::<Assets<StandardMaterial>>()
                .get(&material)
                .unwrap();
            assert!(!material.unlit);
            assert!(
                material
                    .emissive
                    .red
                    .max(material.emissive.green)
                    .max(material.emissive.blue)
                    < 0.2
            );
        }
        let (ember, _) = art.projectile(HeroKind::EmberRanger);
        let (grove, _) = art.projectile(HeroKind::GroveArtificer);
        assert_ne!(ember.id(), grove.id());
        assert_ne!(
            mesh_assets.get(&ember).unwrap().count_vertices(),
            mesh_assets.get(&grove).unwrap().count_vertices()
        );
        assert_eq!(mesh_assets.len(), 12);
        assert_eq!(
            app.world().resource::<Assets<StandardMaterial>>().len(),
            HeroKind::COUNT * 4
        );
    }

    #[test]
    fn physical_heroes_have_distinct_geometry_and_directional_swing_motion() {
        let meshes = melee_meshes();
        for (index, mesh) in meshes.iter().enumerate() {
            let vertices = mesh
                .attribute(Mesh::ATTRIBUTE_POSITION)
                .unwrap()
                .as_float3()
                .unwrap();
            assert!(vertices.iter().all(|v| Vec3::from_array(*v).is_finite()));
            for previous in &meshes[..index] {
                assert_ne!(
                    vertices,
                    previous
                        .attribute(Mesh::ATTRIBUTE_POSITION)
                        .unwrap()
                        .as_float3()
                        .unwrap(),
                    "different heroes must not share a recolored silhouette"
                );
            }
        }
        use bevy::ecs::system::RunSystemOnce;
        let mut app = App::new();
        app.init_resource::<Time>().init_resource::<GameSession>();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.12));
        let mut entities = Vec::new();
        for kind in [HeroKind::IronBerserker, HeroKind::VoidReaper] {
            let style = swing_style(kind);
            entities.push(
                app.world_mut()
                    .spawn((
                        SwingTrail {
                            remaining: style.duration,
                            duration: style.duration,
                            radius: 2.6,
                            delay: 0.0,
                            rotation: Quat::IDENTITY,
                            kind,
                            layer: 0,
                            origin: Vec3::ZERO,
                            direction: Vec3::NEG_Z,
                        },
                        Transform::default(),
                        Visibility::Visible,
                    ))
                    .id(),
            );
        }
        app.world_mut().run_system_once(animate_swings).unwrap();
        let outward = app.world().get::<Transform>(entities[0]).unwrap();
        let inward = app.world().get::<Transform>(entities[1]).unwrap();
        assert!(outward.scale.x > 2.6 * swing_style(HeroKind::IronBerserker).start);
        assert!(inward.scale.x < 2.6 * swing_style(HeroKind::VoidReaper).start);
        assert_ne!(outward.rotation, inward.rotation);
    }

    #[test]
    fn decorative_layers_do_not_expand_any_melee_heros_contact_budget() {
        let primary = Entity::from_bits(1);
        let other = Entity::from_bits(2);
        let outside = Entity::from_bits(3);
        for kind in HeroKind::ALL
            .into_iter()
            .filter(|kind| weapon(*kind).melee())
        {
            let profile = weapon(kind);
            let range = profile.range(99.0);
            let candidates = [
                (primary, Vec3::NEG_Z * range * 0.9),
                (other, Vec3::NEG_Z * range * 0.5),
                (outside, Vec3::NEG_Z * (range + 0.01)),
            ];
            let hits = melee_targets(
                Vec3::ZERO,
                Vec3::NEG_Z,
                range,
                profile.half_angle(),
                1,
                primary,
                &candidates,
            );
            assert_eq!(
                hits,
                vec![candidates[0]],
                "{kind:?}: visual layers cannot add hits"
            );
        }
    }

    #[test]
    fn weapon_profiles_match_the_authored_equipment() {
        assert_eq!(
            HeroKind::ALL
                .into_iter()
                .filter(|kind| weapon(*kind).melee())
                .count(),
            6
        );
        assert_eq!(weapon(HeroKind::TideWalker), BasicWeapon::Daggers);
        assert_eq!(weapon(HeroKind::GroveArtificer), BasicWeapon::Crossbow);
        assert_eq!(weapon(HeroKind::FrostOracle), BasicWeapon::Frost);
        assert!(weapon(HeroKind::IronBerserker).range(99.0) < 3.0);
    }
    #[test]
    fn melee_contact_rejects_far_and_behind_targets_and_respects_hit_budget() {
        let ids = [
            Entity::from_bits(1),
            Entity::from_bits(2),
            Entity::from_bits(3),
            Entity::from_bits(4),
        ];
        let candidates = [
            (ids[0], Vec3::new(0.0, 0.0, -1.5)),
            (ids[1], Vec3::new(0.0, 0.0, -8.0)),
            (ids[2], Vec3::new(0.0, 0.0, 1.0)),
            (ids[3], Vec3::new(0.4, 0.0, -1.0)),
        ];
        let hits = melee_targets(Vec3::ZERO, Vec3::NEG_Z, 2.2, 0.88, 1, ids[0], &candidates);
        assert_eq!(hits, vec![candidates[0]]);
        let hits = melee_targets(Vec3::ZERO, Vec3::NEG_Z, 2.2, 0.88, 2, ids[0], &candidates);
        assert_eq!(hits, vec![candidates[0], candidates[3]]);
    }
    #[test]
    fn faster_attacks_keep_contact_inside_the_animation_window() {
        assert!((swing_windup(1.0) - 0.189).abs() < 0.0001);
        assert!(swing_windup(0.1) < 0.1);
    }

    #[test]
    fn pending_swings_freeze_on_pause_but_cancel_on_hero_change_death_or_finish() {
        use bevy::ecs::system::RunSystemOnce;
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<GameSession>()
            .init_resource::<HeroVitals>()
            .init_resource::<CombatPools>();
        let hero = app.world_mut().spawn((Hero, Transform::default())).id();
        app.world_mut().resource_mut::<HeroVitals>().kind = HeroKind::StoneWarden;
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(1));
        let spawn = |world: &mut World| {
            world
                .spawn(PendingSwing {
                    hero,
                    target: hero,
                    kind: HeroKind::StoneWarden,
                    direction: Vec3::X,
                    damage: 10.0,
                    hit_limit: 1,
                    remaining: 0.12,
                })
                .id()
        };
        let swing = spawn(app.world_mut());
        app.world_mut().run_system_once(resolve_swings).unwrap();
        assert_eq!(
            app.world().get::<PendingSwing>(swing).unwrap().remaining,
            0.12
        );
        app.world_mut().resource_mut::<HeroVitals>().kind = HeroKind::FrostOracle;
        app.world_mut().run_system_once(resolve_swings).unwrap();
        assert!(
            app.world().get_entity(swing).is_err(),
            "hero changes invalidate old weapon attacks even while paused"
        );
        app.world_mut().resource_mut::<HeroVitals>().kind = HeroKind::StoneWarden;
        let swing = spawn(app.world_mut());
        app.world_mut().resource_mut::<HeroVitals>().alive = false;
        app.world_mut().run_system_once(resolve_swings).unwrap();
        assert!(app.world().get_entity(swing).is_err());
        app.world_mut().resource_mut::<HeroVitals>().alive = true;
        let swing = spawn(app.world_mut());
        app.world_mut().resource_mut::<GameSession>().finished = true;
        app.world_mut().run_system_once(resolve_swings).unwrap();
        assert!(app.world().get_entity(swing).is_err());
    }
}
