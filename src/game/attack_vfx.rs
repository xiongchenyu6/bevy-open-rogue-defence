//! Bounded, cosmetic-only weapon trails. Damage and collision stay in combat.rs.
use super::basic_attacks::{BasicAttackArt, BasicWeapon, weapon};
use super::{GameSession, HeroKind, Projectile};
use bevy::prelude::*;

const MAX_MOTES: usize = 320;
const MAX_CUES: usize = 48;

#[derive(Component)]
pub(crate) struct WeaponFlight {
    pub kind: HeroKind,
    age: f32,
    next_trail: f32,
}
impl WeaponFlight {
    pub fn new(kind: HeroKind) -> Self {
        Self {
            kind,
            age: 0.0,
            next_trail: 0.0,
        }
    }
}

enum SpecialCue {
    Ricochet { target: Vec3, hop: u8 },
    Cleave { range: f32, half_angle: f32 },
}

struct Cue {
    kind: HeroKind,
    position: Vec3,
    direction: Vec3,
    impact: bool,
    special: Option<SpecialCue>,
}
#[derive(Resource, Default)]
pub(crate) struct WeaponCues(Vec<Cue>);
impl WeaponCues {
    pub(crate) fn ricochet(&mut self, kind: HeroKind, from: Vec3, to: Vec3, hop: u8) {
        if self.0.len() < MAX_CUES {
            self.0.push(Cue {
                kind,
                position: from,
                direction: to - from,
                impact: true,
                special: Some(SpecialCue::Ricochet { target: to, hop }),
            });
        }
    }
    pub(crate) fn cleave(
        &mut self,
        kind: HeroKind,
        origin: Vec3,
        direction: Vec3,
        range: f32,
        half_angle: f32,
    ) {
        if self.0.len() < MAX_CUES {
            self.0.push(Cue {
                kind,
                position: origin,
                direction,
                impact: true,
                special: Some(SpecialCue::Cleave { range, half_angle }),
            });
        }
    }
    pub fn push(&mut self, kind: HeroKind, position: Vec3, direction: Vec3, impact: bool) {
        if self.0.len() < MAX_CUES {
            self.0.push(Cue {
                kind,
                position,
                direction,
                impact,
                special: None,
            });
        }
    }
}

#[derive(Resource)]
pub(crate) struct TrailArt {
    streak: Handle<Mesh>,
    mote: Handle<Mesh>,
    ring: Handle<Mesh>,
    core: Handle<StandardMaterial>,
    shard: Handle<Mesh>,
    chunk: Handle<Mesh>,
}
#[derive(Component)]
pub(crate) struct WeaponMote {
    remaining: f32,
    duration: f32,
    velocity: Vec3,
    scale: Vec3,
    expand: bool,
    spin: f32,
}

pub(crate) fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(TrailArt {
        streak: meshes.add(Cuboid::new(0.055, 0.055, 1.0)),
        mote: meshes.add(Sphere::new(1.0).mesh().ico(1).unwrap()),
        ring: meshes.add(Torus::new(0.91, 1.0)),
        shard: meshes.add(Cone::new(1.0, 2.0).mesh().resolution(4)),
        chunk: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        core: materials.add(StandardMaterial {
            base_color: Color::srgb(0.88, 0.97, 1.0),
            emissive: LinearRgba::new(12.0, 14.0, 16.0, 1.0),
            unlit: false,
            ..default()
        }),
    });
}

struct Painter<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    available: usize,
    label: Option<&'static str>,
}
impl Painter<'_, '_, '_> {
    fn beam(
        &mut self,
        art: &TrailArt,
        color: &Handle<StandardMaterial>,
        a: Vec3,
        b: Vec3,
        width: f32,
        life: f32,
    ) {
        self.emit(
            art.streak.clone(),
            color.clone(),
            Transform::from_translation((a + b) * 0.5)
                .looking_to(b - a, Vec3::Y)
                .with_scale(Vec3::new(width / 0.055, width / 0.055, a.distance(b))),
            Vec3::ZERO,
            life,
            false,
            0.0,
        );
    }
    fn physical_contact(
        &mut self,
        art: &TrailArt,
        color: &Handle<StandardMaterial>,
        kind: HeroKind,
        center: Vec3,
        direction: Vec3,
    ) -> bool {
        let forward = direction.normalize_or(Vec3::X);
        let side = forward.cross(Vec3::Y);
        match kind {
            HeroKind::StoneWarden => {
                // Broad ground cracks and flying stone chips: a crushing sword blow.
                for i in -1..=1 {
                    let end = center + forward * 0.9 + side * i as f32 * 0.55;
                    self.beam(art, color, center - forward * 0.5, end, 0.09, 0.28);
                    self.emit(
                        art.chunk.clone(),
                        color.clone(),
                        Transform::from_translation(end)
                            .with_rotation(Quat::from_rotation_z(0.6))
                            .with_scale(Vec3::splat(0.18)),
                        side * i as f32 * 1.4 + Vec3::Y * 1.5,
                        0.35,
                        false,
                        4.0,
                    );
                }
            }
            HeroKind::SolarVanguard => {
                // A rising gold cross and four sun rays instead of stone debris.
                self.beam(
                    art,
                    color,
                    center - Vec3::Y * 0.2,
                    center + Vec3::Y * 1.4,
                    0.12,
                    0.28,
                );
                self.beam(
                    art,
                    color,
                    center - side * 0.65 + Vec3::Y * 0.45,
                    center + side * 0.65 + Vec3::Y * 0.45,
                    0.12,
                    0.28,
                );
                for i in 0..4 {
                    let angle = i as f32 * std::f32::consts::FRAC_PI_2 + 0.4;
                    let ray = Vec3::new(angle.cos(), 0.3, angle.sin());
                    self.beam(
                        art,
                        color,
                        center + ray * 0.3,
                        center + ray * 1.15,
                        0.065,
                        0.26,
                    );
                }
            }
            HeroKind::TideWalker => {
                // Paired foamy curls mark the two dagger contacts.
                for sign in [-1.0, 1.0] {
                    for i in 0..6 {
                        let a = i as f32 * 0.4;
                        let b = (i + 1) as f32 * 0.4;
                        self.beam(
                            art,
                            color,
                            center + forward * a.sin() * 0.8 + side * (a.cos() * 0.5 + 0.2) * sign,
                            center + forward * b.sin() * 0.8 + side * (b.cos() * 0.5 + 0.2) * sign,
                            0.10,
                            0.25,
                        );
                    }
                    self.emit(
                        art.mote.clone(),
                        color.clone(),
                        Transform::from_translation(center + side * sign * 0.5)
                            .with_scale(Vec3::new(0.12, 0.3, 0.12)),
                        side * sign + Vec3::Y,
                        0.35,
                        false,
                        0.0,
                    );
                }
            }
            HeroKind::RiftDancer => {
                // Violet crossed blades, with a displaced echo behind the target.
                for offset in [Vec3::ZERO, -forward * 0.45] {
                    for sign in [-1.0, 1.0] {
                        self.beam(
                            art,
                            color,
                            center + offset - side * 0.7 + Vec3::Y * (0.6 * sign),
                            center + offset + side * 0.7 - Vec3::Y * (0.6 * sign),
                            0.10,
                            if offset == Vec3::ZERO { 0.22 } else { 0.34 },
                        );
                    }
                }
            }
            HeroKind::IronBerserker => {
                self.beam(
                    art,
                    color,
                    center + Vec3::Y * 1.4 - forward * 0.4,
                    center - Vec3::Y * 0.2 + forward * 0.4,
                    0.22,
                    0.3,
                );
                for i in -2..=2 {
                    let out = (side * i as f32 * 0.45 + forward * 0.3 + Vec3::Y * 0.35).normalize();
                    self.emit(
                        art.shard.clone(),
                        color.clone(),
                        Transform::from_translation(center)
                            .with_rotation(Quat::from_rotation_arc(Vec3::Y, out))
                            .with_scale(Vec3::new(0.12, 0.4, 0.12)),
                        out * 3.0,
                        0.35,
                        false,
                        0.0,
                    );
                }
            }
            HeroKind::VoidReaper => {
                // A hooked inward crescent: fragments contract rather than explode.
                for i in 0..9 {
                    let a = i as f32 * 0.4;
                    let b = (i + 1) as f32 * 0.4;
                    self.beam(
                        art,
                        color,
                        center + side * a.cos() + forward * a.sin(),
                        center + side * b.cos() + forward * b.sin(),
                        0.13,
                        0.3,
                    );
                }
                for sign in [-1.0, 1.0] {
                    self.emit(
                        art.shard.clone(),
                        color.clone(),
                        Transform::from_translation(center + side * sign)
                            .with_scale(Vec3::splat(0.22)),
                        -side * sign * 2.0,
                        0.38,
                        false,
                        6.0,
                    );
                }
            }
            HeroKind::EmberRanger => {
                for i in -1..=1 {
                    self.emit(
                        art.shard.clone(),
                        color.clone(),
                        Transform::from_translation(center + side * i as f32 * 0.25)
                            .with_scale(Vec3::new(0.22, 0.6, 0.22)),
                        Vec3::Y * (1.8 + i as f32 * 0.2) - forward * 0.3,
                        0.34,
                        false,
                        2.0,
                    );
                }
            }
            HeroKind::GroveArtificer => {
                for i in -1..=1 {
                    self.beam(
                        art,
                        color,
                        center - forward * 0.55 + side * i as f32 * 0.25,
                        center + forward * 0.65 + side * i as f32 * 0.25,
                        0.07,
                        0.24,
                    );
                }
                for sign in [-1.0, 1.0] {
                    self.emit(
                        art.chunk.clone(),
                        color.clone(),
                        Transform::from_translation(center + side * sign * 0.4)
                            .with_rotation(Quat::from_rotation_z(0.7))
                            .with_scale(Vec3::new(0.18, 0.18, 0.07)),
                        side * sign * 1.5 + Vec3::Y * 0.4,
                        0.32,
                        false,
                        5.0,
                    );
                }
            }
            _ => return false,
        }
        self.emit(
            art.mote.clone(),
            art.core.clone(),
            Transform::from_translation(center).with_scale(Vec3::splat(0.17)),
            Vec3::ZERO,
            0.10,
            false,
            0.0,
        );
        true
    }
    fn emit(
        &mut self,
        mesh: Handle<Mesh>,
        material: Handle<StandardMaterial>,
        transform: Transform,
        velocity: Vec3,
        duration: f32,
        expand: bool,
        spin: f32,
    ) {
        if self.available == 0 {
            return;
        }
        self.available -= 1;
        self.commands.spawn((
            Name::new(self.label.unwrap_or("Weapon trail / impact")),
            Mesh3d(mesh),
            MeshMaterial3d(material),
            WeaponMote {
                remaining: duration,
                duration,
                velocity,
                scale: transform.scale,
                expand,
                spin,
            },
            transform,
        ));
    }
}

pub(crate) fn animate(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    art: Option<Res<TrailArt>>,
    weapons: Option<Res<BasicAttackArt>>,
    mut cues: ResMut<WeaponCues>,
    mut flights: Query<(&Projectile, &mut WeaponFlight, &mut Transform), With<Mesh3d>>,
    mut motes: Query<(Entity, &mut WeaponMote, &mut Transform), Without<Projectile>>,
) {
    if session.finished {
        cues.0.clear();
        for (entity, ..) in &mut motes {
            commands.entity(entity).despawn();
        }
        return;
    }
    if session.paused {
        return;
    }
    let (Some(art), Some(weapons)) = (art, weapons) else {
        cues.0.clear();
        return;
    };
    let dt = time.delta_secs();
    let mut active = 0;
    for (entity, mut mote, mut transform) in &mut motes {
        mote.remaining -= dt;
        if mote.remaining <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        active += 1;
        let t = 1.0 - mote.remaining / mote.duration;
        transform.translation += mote.velocity * dt;
        transform.rotate_local_y(mote.spin * dt);
        // Rings spread while their thickness collapses; sparks taper out cleanly.
        transform.scale = if mote.expand {
            mote.scale * Vec3::new(0.3 + t * 1.1, (1.0 - t).max(0.01), 0.3 + t * 1.1)
        } else {
            mote.scale * (1.0 - t).powf(0.65)
        };
    }
    let mut painter = Painter {
        commands: &mut commands,
        available: MAX_MOTES.saturating_sub(active),
        label: None,
    };
    for cue in cues.0.drain(..) {
        let profile = weapon(cue.kind);
        let color = weapons.materials[cue.kind.index()].clone();
        painter.label = None;
        if let Some(special) = cue.special {
            match special {
                SpecialCue::Ricochet { target, hop } => {
                    painter.label = Some("Archer ricochet trail");
                    let from = cue.position + Vec3::Y * 0.35;
                    let to = target + Vec3::Y * 0.35;
                    if from.distance_squared(to) > 0.001 {
                        painter.beam(&art, &color, from, to, 0.12, 0.22);
                        painter.beam(&art, &art.core, from, to, 0.035, 0.13);
                        // A fork at the real turn separates a bounce from a straight shot.
                        let side = (to - from).normalize().cross(Vec3::Y);
                        for sign in [-1.0, 1.0] {
                            painter.beam(
                                &art,
                                &color,
                                from - side * sign * 0.35,
                                from + (to - from).normalize() * 0.45,
                                0.10,
                                0.22,
                            );
                        }
                        painter.emit(
                            art.ring.clone(),
                            color.clone(),
                            Transform::from_translation(from)
                                .with_scale(Vec3::splat(0.30 + f32::from(hop.min(4)) * 0.04)),
                            Vec3::ZERO,
                            0.22,
                            true,
                            0.0,
                        );
                    }
                }
                SpecialCue::Cleave { range, half_angle } => {
                    painter.label = Some("Warrior cleave sweep");
                    let forward = cue.direction.with_y(0.0).normalize_or(Vec3::X);
                    let side = forward.cross(Vec3::Y);
                    let origin = cue.position.with_y(0.12);
                    for i in 0..16 {
                        let a = -half_angle + 2.0 * half_angle * i as f32 / 16.0;
                        let b = -half_angle + 2.0 * half_angle * (i + 1) as f32 / 16.0;
                        let point = |angle: f32| {
                            origin + (forward * angle.cos() + side * angle.sin()) * range
                        };
                        painter.beam(&art, &color, point(a), point(b), 0.16, 0.28);
                    }
                    for angle in [-half_angle, 0.0, half_angle] {
                        let end = origin + (forward * angle.cos() + side * angle.sin()) * range;
                        painter.beam(&art, &color, origin + forward * 0.4, end, 0.10, 0.22);
                        painter.emit(
                            art.shard.clone(),
                            color.clone(),
                            Transform::from_translation(end)
                                .with_scale(Vec3::new(0.14, 0.40, 0.14)),
                            Vec3::Y * 1.4,
                            0.27,
                            false,
                            4.0,
                        );
                    }
                }
            }
            continue;
        }
        let center = cue.position + Vec3::Y * 0.45;
        let direction = cue.direction.with_y(0.0).normalize_or_zero();
        if !cue.impact {
            // Preserve the magical release silhouette even for point-blank shots
            // which can collide before a single rendered flight frame.
            if matches!(
                profile,
                BasicWeapon::Storm | BasicWeapon::Spirit | BasicWeapon::Star | BasicWeapon::Frost
            ) {
                painter.emit(
                    weapons.projectile(cue.kind).0,
                    color.clone(),
                    Transform::from_translation(center + direction * 0.35)
                        .looking_to(direction, Vec3::Y)
                        .with_scale(Vec3::splat(0.7)),
                    direction * 0.6,
                    0.17,
                    false,
                    0.0,
                );
            }
            if profile == BasicWeapon::Crossbow {
                painter.emit(
                    weapons.projectile(cue.kind).0,
                    color.clone(),
                    Transform::from_translation(center + direction * 0.35)
                        .looking_to(direction, Vec3::Y)
                        .with_scale(Vec3::splat(0.7)),
                    direction * 0.5,
                    0.14,
                    false,
                    0.0,
                );
            }
            painter.emit(
                art.mote.clone(),
                art.core.clone(),
                Transform::from_translation(center).with_scale(Vec3::splat(0.14)),
                direction * 0.4,
                0.09,
                false,
                0.0,
            );
            continue;
        }
        if painter.physical_contact(&art, &color, cue.kind, center, direction) {
            continue;
        }
        let radius = match profile {
            BasicWeapon::Axe => 0.9,
            BasicWeapon::Storm => 1.05,
            BasicWeapon::Frost => 1.0,
            BasicWeapon::Spirit | BasicWeapon::Star => 0.9,
            _ => 0.5,
        };
        painter.emit(
            art.ring.clone(),
            color.clone(),
            Transform::from_translation(center).with_scale(Vec3::splat(radius)),
            Vec3::ZERO,
            0.24,
            true,
            0.0,
        );
        painter.emit(
            art.mote.clone(),
            art.core.clone(),
            Transform::from_translation(center).with_scale(Vec3::splat(0.20)),
            Vec3::ZERO,
            0.10,
            false,
            0.0,
        );
        if matches!(
            profile,
            BasicWeapon::Storm | BasicWeapon::Spirit | BasicWeapon::Star
        ) {
            let (mesh, _) = weapons.projectile(cue.kind);
            let count = if profile == BasicWeapon::Storm { 4 } else { 3 };
            for index in 0..count {
                let angle = index as f32 / count as f32 * std::f32::consts::TAU;
                let outward = Vec3::new(angle.cos(), 0.15, angle.sin());
                painter.emit(
                    mesh.clone(),
                    color.clone(),
                    Transform::from_translation(center)
                        .looking_to(outward, Vec3::Y)
                        .with_scale(Vec3::splat(0.45)),
                    outward * 2.4,
                    0.32,
                    false,
                    5.0,
                );
            }
        }
        let count = if profile == BasicWeapon::Daggers {
            3
        } else {
            5
        };
        for index in 0..count {
            let angle = index as f32 / count as f32 * std::f32::consts::TAU + 0.3;
            let outward = Vec3::new(angle.cos(), 0.22, angle.sin()).normalize();
            let (mesh, scale) = if profile == BasicWeapon::Frost {
                (weapons.projectile(cue.kind).0, Vec3::splat(0.40))
            } else {
                (art.streak.clone(), Vec3::new(1.2, 1.2, 0.40))
            };
            painter.emit(
                mesh,
                color.clone(),
                Transform::from_translation(center + outward * 0.12)
                    .looking_to(outward, Vec3::Y)
                    .with_scale(scale),
                outward * 2.2,
                0.22,
                false,
                2.0,
            );
        }
    }
    painter.label = None;
    for (projectile, mut flight, mut transform) in &mut flights {
        flight.age += dt;
        let direction = projectile.velocity.normalize_or_zero();
        if direction == Vec3::ZERO {
            continue;
        }
        let profile = weapon(flight.kind);
        let color = weapons.materials[flight.kind.index()].clone();
        // Spin the star around its normal; other projectiles retain their collision direction.
        if profile == BasicWeapon::Star {
            transform.rotation = Transform::IDENTITY.looking_to(direction, Vec3::Y).rotation
                * Quat::from_rotation_y(flight.age * 15.0);
        }
        flight.next_trail -= dt;
        if flight.next_trail > 0.0 {
            continue;
        }
        flight.next_trail = 0.035;
        // Preserve room for contact feedback when multishot fills the scene.
        if painter.available <= 64 {
            continue;
        }
        let position = transform.translation + Vec3::Y * 0.13;
        let side = direction.cross(Vec3::Y).normalize_or_zero();
        let (length, life) = match profile {
            BasicWeapon::Crossbow => (0.90, 0.14),
            BasicWeapon::Storm => (0.75, 0.20),
            BasicWeapon::Spirit => (0.95, 0.42),
            BasicWeapon::Star => (0.40, 0.23),
            BasicWeapon::Frost => (0.9, 0.30),
            _ => continue,
        };
        // Layer a bright seed over the colored body, with a moving orbital shell.
        // These travel at projectile velocity and expire quickly, so pooled reuse
        // never leaves attached children or a glow on the next rented projectile.
        if matches!(
            profile,
            BasicWeapon::Storm | BasicWeapon::Spirit | BasicWeapon::Star | BasicWeapon::Frost
        ) {
            painter.emit(
                art.mote.clone(),
                art.core.clone(),
                Transform::from_translation(position).with_scale(Vec3::splat(0.19)),
                projectile.velocity,
                0.055,
                false,
                0.0,
            );
            if profile != BasicWeapon::Frost {
                let rotation = Quat::from_rotation_z(flight.age * 8.0 + 0.6);
                painter.emit(
                    art.ring.clone(),
                    color.clone(),
                    Transform::from_translation(position)
                        .with_rotation(rotation)
                        .with_scale(Vec3::splat(0.64)),
                    projectile.velocity,
                    0.07,
                    false,
                    4.0,
                );
            }
        }
        match profile {
            BasicWeapon::Storm => {
                // Three connected, jittering segments form a broken lightning wake.
                let mut from = position;
                for i in 1..=3 {
                    let offset = ((flight.age * 63.0 + i as f32 * 2.1).sin()) * 0.22;
                    let to = position - direction * length * i as f32 + side * offset;
                    painter.emit(
                        art.streak.clone(),
                        if i == 1 {
                            art.core.clone()
                        } else {
                            color.clone()
                        },
                        Transform::from_translation((from + to) * 0.5)
                            .looking_to(to - from, Vec3::Y)
                            .with_scale(Vec3::new(2.8, 2.8, from.distance(to))),
                        Vec3::ZERO,
                        life,
                        false,
                        0.0,
                    );
                    from = to;
                }
            }
            BasicWeapon::Spirit => {
                for sign in [-1.0, 1.0] {
                    let offset = side * (flight.age * 19.0).sin() * 0.44 * sign;
                    painter.emit(
                        art.mote.clone(),
                        color.clone(),
                        Transform::from_translation(position + offset)
                            .with_scale(Vec3::splat(0.23)),
                        -direction * 0.7 + Vec3::Y * 0.3,
                        life,
                        false,
                        0.0,
                    );
                }
            }
            BasicWeapon::Star => {
                let (mesh, _) = weapons.projectile(flight.kind);
                painter.emit(
                    mesh,
                    color,
                    Transform::from_translation(position)
                        .with_rotation(transform.rotation)
                        .with_scale(Vec3::splat(0.64)),
                    Vec3::Y * 0.12,
                    life,
                    false,
                    12.0,
                );
            }
            BasicWeapon::Crossbow => {
                if flight.kind == HeroKind::EmberRanger {
                    for sign in [-1.0, 1.0] {
                        let offset = side * (flight.age * 32.0).sin() * 0.12 * sign;
                        painter.beam(
                            &art,
                            &color,
                            position + offset,
                            position - direction * 1.15 + offset * 1.8,
                            0.12,
                            0.20,
                        );
                        painter.emit(
                            art.shard.clone(),
                            color.clone(),
                            Transform::from_translation(position - direction * 0.4 + offset)
                                .with_scale(Vec3::new(0.1, 0.25, 0.1)),
                            -direction * 0.4 + Vec3::Y * 0.9,
                            0.27,
                            false,
                            2.0,
                        );
                    }
                } else {
                    for index in 0..3 {
                        let start = position - direction * (index as f32 * 0.4);
                        painter.beam(&art, &color, start, start - direction * 0.24, 0.10, 0.16);
                    }
                    painter.emit(
                        art.chunk.clone(),
                        color.clone(),
                        Transform::from_translation(position - direction * 0.5)
                            .with_rotation(Quat::from_rotation_z(flight.age * 15.0))
                            .with_scale(Vec3::new(0.22, 0.06, 0.22)),
                        Vec3::ZERO,
                        0.20,
                        false,
                        6.0,
                    );
                }
            }
            _ => {
                painter.emit(
                    art.streak.clone(),
                    color.clone(),
                    Transform::from_translation(position - direction * length * 0.5)
                        .looking_to(direction, Vec3::Y)
                        .with_scale(Vec3::new(
                            if profile == BasicWeapon::Frost {
                                2.8
                            } else {
                                1.2
                            },
                            if profile == BasicWeapon::Frost {
                                2.8
                            } else {
                                1.2
                            },
                            length,
                        )),
                    Vec3::ZERO,
                    life,
                    false,
                    0.0,
                );
                if profile == BasicWeapon::Frost {
                    let (mesh, _) = weapons.projectile(flight.kind);
                    painter.emit(
                        mesh,
                        color,
                        Transform::from_translation(
                            position + side * (flight.age * 31.0).sin() * 0.22,
                        )
                        .with_scale(Vec3::splat(0.34)),
                        Vec3::Y * -0.5,
                        life,
                        false,
                        5.0,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trails_freeze_on_pause_expire_and_clear_at_finish_without_asset_growth() {
        use bevy::ecs::system::RunSystemOnce;
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<GameSession>()
            .init_resource::<WeaponCues>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>();
        app.world_mut().run_system_once(setup).unwrap();
        app.world_mut()
            .run_system_once(super::super::basic_attacks::setup_attack_art)
            .unwrap();
        let assets = (
            app.world().resource::<Assets<Mesh>>().len(),
            app.world().resource::<Assets<StandardMaterial>>().len(),
        );
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(40));
        for kind in HeroKind::ALL {
            app.world_mut()
                .resource_mut::<WeaponCues>()
                .push(kind, Vec3::ZERO, Vec3::X, false);
        }
        app.world_mut().run_system_once(animate).unwrap();
        let releases = app
            .world_mut()
            .query::<&WeaponMote>()
            .iter(app.world())
            .count();
        assert!((1..=MAX_MOTES).contains(&releases));
        assert_eq!(
            app.world_mut()
                .query::<&Projectile>()
                .iter(app.world())
                .count(),
            0,
            "cosmetic releases must not create damage carriers"
        );
        app.world_mut().resource_mut::<GameSession>().finished = true;
        app.world_mut().run_system_once(animate).unwrap();
        app.world_mut().resource_mut::<GameSession>().finished = false;
        for index in 0..MAX_CUES {
            app.world_mut().resource_mut::<WeaponCues>().push(
                HeroKind::ALL[index % HeroKind::COUNT],
                Vec3::ZERO,
                Vec3::X,
                true,
            );
        }
        app.world_mut().run_system_once(animate).unwrap();
        let mut query = app.world_mut().query::<(Entity, &WeaponMote)>();
        let live: Vec<_> = query
            .iter(app.world())
            .map(|(entity, mote)| (entity, mote.remaining))
            .collect();
        assert_eq!(live.len(), MAX_MOTES);
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.world_mut().run_system_once(animate).unwrap();
        assert_eq!(
            app.world().get::<WeaponMote>(live[0].0).unwrap().remaining,
            live[0].1
        );
        app.world_mut().resource_mut::<GameSession>().paused = false;
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(1));
        app.world_mut().run_system_once(animate).unwrap();
        assert_eq!(query.iter(app.world()).count(), 0);
        app.world_mut().resource_mut::<WeaponCues>().push(
            HeroKind::StormSavant,
            Vec3::ZERO,
            Vec3::X,
            true,
        );
        app.world_mut().run_system_once(animate).unwrap();
        assert!(query.iter(app.world()).count() > 0);
        app.world_mut().resource_mut::<GameSession>().finished = true;
        app.world_mut().run_system_once(animate).unwrap();
        assert_eq!(query.iter(app.world()).count(), 0);
        assert_eq!(
            assets,
            (
                app.world().resource::<Assets<Mesh>>().len(),
                app.world().resource::<Assets<StandardMaterial>>().len()
            )
        );
    }
    #[test]
    fn special_trails_follow_real_cue_geometry_without_creating_damage_carriers() {
        use bevy::ecs::system::RunSystemOnce;
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<GameSession>()
            .init_resource::<WeaponCues>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>();
        app.world_mut().run_system_once(setup).unwrap();
        app.world_mut()
            .run_system_once(super::super::basic_attacks::setup_attack_art)
            .unwrap();
        let origin = Vec3::new(4.0, 0.0, 0.0);
        app.world_mut().resource_mut::<WeaponCues>().cleave(
            HeroKind::StoneWarden,
            origin,
            Vec3::X,
            3.0,
            0.8,
        );
        app.world_mut().resource_mut::<WeaponCues>().ricochet(
            HeroKind::EmberRanger,
            Vec3::ZERO,
            Vec3::new(2.0, 0.0, 2.0),
            1,
        );
        app.world_mut().run_system_once(animate).unwrap();
        let mut query = app.world_mut().query::<(&Name, &Transform)>();
        let cleave = query
            .iter(app.world())
            .filter(|(name, _)| name.as_str() == "Warrior cleave sweep")
            .collect::<Vec<_>>();
        assert_eq!(cleave.len(), 22);
        for (_, transform) in &cleave {
            assert!(transform.translation.xz().distance(origin.xz()) <= 3.001);
        }
        assert_eq!(
            query
                .iter(app.world())
                .filter(|(name, _)| name.as_str() == "Archer ricochet trail")
                .count(),
            5
        );
        assert_eq!(
            app.world_mut()
                .query::<&Projectile>()
                .iter(app.world())
                .count(),
            0
        );
        app.world_mut().resource_mut::<GameSession>().finished = true;
        app.world_mut().run_system_once(animate).unwrap();
        assert_eq!(
            app.world_mut()
                .query::<&WeaponMote>()
                .iter(app.world())
                .count(),
            0
        );
    }
    #[test]
    fn cosmetic_queue_is_bounded_under_mass_multishot() {
        let mut cues = WeaponCues::default();
        for _ in 0..10000 {
            cues.push(HeroKind::StormSavant, Vec3::ZERO, Vec3::X, true);
        }
        assert_eq!(cues.0.len(), MAX_CUES);
    }
}
