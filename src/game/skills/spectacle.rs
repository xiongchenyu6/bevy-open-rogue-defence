//! Skill identities layered onto real impacts, zones and summoned actors.
//! Uses the existing presentation budget; there is no additional damage or timer.
use super::*;
#[path = "spectacle/reference_geometry.rs"]
pub(in super::super) mod reference_geometry;
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
};

pub(super) fn blade_mesh() -> Mesh {
    let p = vec![
        [-0.11, 0.0, 0.0],
        [0.11, 0.0, 0.0],
        [0.09, 1.05, 0.0],
        [0.0, 1.55, 0.0],
        [-0.09, 1.05, 0.0],
        [-0.33, 0.25, 0.0],
        [0.33, 0.25, 0.0],
        [0.33, 0.36, 0.0],
        [-0.33, 0.36, 0.0],
    ];
    let indices = vec![
        0, 1, 2, 0, 2, 4, 4, 2, 3, 5, 6, 7, 5, 7, 8, 2, 1, 0, 4, 2, 0, 3, 2, 4, 7, 6, 5, 8, 7, 5,
    ];
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, p)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 9])
    .with_inserted_indices(Indices::U32(indices))
}
pub(super) fn crescent_mesh() -> Mesh {
    let mut p = Vec::new();
    let mut indices = Vec::new();
    for i in 0..=32 {
        let t = i as f32 / 32.0;
        let angle = t * std::f32::consts::TAU;
        let outer = 0.72 + if i % 4 == 0 { 0.10 } else { 0.0 };
        for r in [0.49, outer] {
            p.push([angle.cos() * r, angle.sin() * r, 0.0]);
        }
        if i < 32 {
            let a = i * 2;
            indices.extend([
                a,
                a + 1,
                a + 2,
                a + 1,
                a + 3,
                a + 2,
                a + 2,
                a + 1,
                a,
                a + 2,
                a + 3,
                a + 1,
            ]);
        }
    }
    let len = p.len();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, p)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; len])
    .with_inserted_indices(Indices::U32(indices))
}
fn lightning(p: &mut Painter, from: Vec3, to: Vec3, motif: Motif, width: f32) {
    let delta = to - from;
    // Three pinned, octave-noise filaments in one mesh instead of six cuboid elbows.
    p.emit(
        motif,
        p.art.lightning.clone(),
        from,
        Vec3::new(width * 5.0, delta.length().max(0.05), width * 5.0),
        Quat::from_rotation_arc(Vec3::Y, delta.normalize_or(Vec3::Y)),
        Vec3::ZERO,
        0.23,
        0.0,
        0.0,
    );
}
fn fissure(p: &mut Painter, motif: Motif, at: Vec3, radius: f32, phase: f32) {
    p.emit(
        motif,
        p.art.fissure.clone(),
        at.with_y(0.065),
        Vec3::new(radius, 1.0, radius),
        Quat::from_rotation_y(phase),
        Vec3::ZERO,
        0.65,
        0.0,
        0.08,
    );
}
fn coil(p: &mut Painter, motif: Motif, from: Vec3, to: Vec3, radius: f32) {
    let delta = to - from;
    p.emit(
        motif,
        p.art.coil.clone(),
        from,
        Vec3::new(radius, delta.length().max(0.1), radius),
        Quat::from_rotation_arc(Vec3::Y, delta.normalize_or(Vec3::Y)),
        Vec3::ZERO,
        0.4,
        0.0,
        -0.1,
    );
}

fn shards(
    p: &mut Painter,
    motif: Motif,
    at: Vec3,
    radius: f32,
    height: f32,
    count: usize,
    phase: f32,
) {
    for i in 0..count {
        let angle = phase + i as f32 * std::f32::consts::TAU / count as f32;
        let offset = radial(angle + (i as f32 * 2.7).sin() * 0.15) * radius;
        let height = height * (0.72 + (i as f32 * 1.73).sin().abs() * 0.45);
        p.emit(
            motif,
            p.art.crystal.clone(),
            at + offset,
            Vec3::new(0.38, height, 0.38),
            Quat::from_rotation_arc(
                Vec3::Y,
                (Vec3::Y + offset.normalize_or_zero() * 0.25).normalize(),
            ),
            Vec3::Y * 0.2,
            0.65,
            0.0,
            0.15,
        );
    }
}
fn star(p: &mut Painter, motif: Motif, at: Vec3, radius: f32, phase: f32, arms: usize) {
    for i in 0..arms {
        let direction = radial(phase + i as f32 * std::f32::consts::TAU / arms as f32);
        p.emit(
            motif,
            p.art.shard.clone(),
            at + direction * radius * 0.42,
            Vec3::new(0.07, radius * 0.65, 0.07),
            Quat::from_rotation_arc(Vec3::Y, direction),
            direction * radius * 0.6,
            0.30,
            0.0,
            -0.35,
        );
    }
}
#[allow(clippy::too_many_arguments)]
pub(super) fn impact(
    p: &mut Painter,
    index: usize,
    origin: Vec3,
    at: Vec3,
    radius: f32,
    rank: u8,
    phase: f32,
    path: Option<super::super::super::skill_evolution::UrEvolutionPath>,
) {
    let previous = p.identity.replace(index);
    let scale = rank_size(rank);
    let reach = if radius > 0.05 { radius } else { 0.65 };
    match index {
        0 => lightning(
            p,
            origin + Vec3::Y,
            at + Vec3::Y * 0.6,
            Motif::Shock,
            0.10 * scale,
        ),
        1 => {
            shards(p, Motif::Fire, at, 0.35, 0.8 * scale, 4, phase);
            fissure(p, Motif::Fire, at, 0.55 * scale, phase);
            p.beam(
                Motif::Buff,
                at + Vec3::Y * 0.3,
                at + Vec3::Y * 2.0,
                0.10,
                0.22,
            );
        }
        2 => {
            p.emit(
                Motif::Ice,
                p.art.crystal.clone(),
                at,
                Vec3::new(0.32, 1.5, 0.32) * scale,
                Quat::from_rotation_z(0.3),
                Vec3::ZERO,
                0.5,
                0.0,
                0.0,
            );
            star(p, Motif::Ice, at + Vec3::Y * 0.1, 0.7, phase, 6);
        }
        3 => fissure(p, Motif::Water, at, reach * 0.6, phase),
        6 => coil(p, Motif::Steel, at, at + Vec3::Y * 0.75, 0.9 * scale),
        30 => coil(
            p,
            Motif::Arcane,
            origin + Vec3::Y,
            at + Vec3::Y * 0.6,
            0.7 * scale,
        ),
        4 => {
            fissure(p, Motif::Steel, at, 0.9 * scale, phase);
            for side in [-1.0, 1.0] {
                for rib in 0..3 {
                    let y = 0.25 + rib as f32 * 0.3;
                    p.beam(
                        Motif::Steel,
                        at + Vec3::new(side * 0.55, y, 0.0),
                        at + Vec3::new(side * 0.2, y + 0.35, 0.0),
                        0.09,
                        0.35,
                    );
                }
            }
        }
        5 => {
            coil(
                p,
                Motif::Arcane,
                at + Vec3::Y * 0.2,
                at + Vec3::Y * 1.3,
                scale,
            );
            for i in 0..3 {
                let x = (i as f32 - 1.0) * 0.25;
                p.beam(
                    Motif::Arcane,
                    at + Vec3::new(x, 1.5, -0.35),
                    at + Vec3::new(x - 0.25, 0.2, 0.35),
                    0.09,
                    0.30,
                );
            }
        }
        7 => {
            fissure(p, Motif::Steel, at, 0.45 * scale, phase);
            for i in 0..3 {
                let end = at + Vec3::new((i as f32 - 1.0) * 0.25, 0.3, 0.0);
                p.beam(Motif::Steel, end - Vec3::X * 1.2, end, 0.06 * scale, 0.25);
            }
        }
        8 => {
            coil(p, Motif::Arcane, at, at + Vec3::Y * 1.5, scale);
            star(p, Motif::Arcane, at + Vec3::Y * 0.6, 0.9 * scale, phase, 8);
        }
        9 => fissure(p, Motif::Buff, at, reach, phase),
        10 => {
            let corners = [
                at + Vec3::X * 0.7,
                at + Vec3::new(-0.35, 0.0, 0.6),
                at + Vec3::new(-0.35, 0.0, -0.6),
            ];
            for i in 0..3 {
                lightning(
                    p,
                    corners[i] + Vec3::Y,
                    corners[(i + 1) % 3] + Vec3::Y,
                    Motif::Shock,
                    0.045,
                );
            }
        }
        11 => shards(
            p,
            Motif::Fire,
            at,
            reach.min(3.5) * 0.8,
            0.9 * scale,
            8,
            phase,
        ),
        12 => {
            lightning(p, at + Vec3::Y * 6.0, at, Motif::Shock, 0.18 * scale);
            fissure(p, Motif::Shock, at, reach, phase);
        }
        13 => {
            p.emit(
                Motif::Fire,
                p.art.shard.clone(),
                at + Vec3::Y * 4.0,
                Vec3::new(0.6, 1.8, 0.6) * scale,
                Quat::from_rotation_z(std::f32::consts::PI),
                Vec3::NEG_Y * 12.0,
                0.32,
                1.0,
                0.0,
            );
            shards(p, Motif::Fire, at, reach * 0.7, 0.8, 6, phase);
            fissure(p, Motif::Fire, at, reach, phase);
        }
        14 => {
            shards(
                p,
                Motif::Ice,
                at,
                reach * 0.72,
                1.15 * scale,
                7,
                phase * 0.1,
            );
            fissure(p, Motif::Ice, at, reach, phase * 0.1);
        }
        15 => vortex(p, at, reach, phase),
        16 => {
            p.emit(
                Motif::Water,
                p.art.coil.clone(),
                at,
                Vec3::new(0.65, 1.35, 0.65) * scale,
                Quat::IDENTITY,
                Vec3::Y * 0.8,
                0.45,
                4.0,
                0.1,
            );
        }
        17 => {
            for side in [-1.0, 1.0] {
                p.beam(
                    Motif::Buff,
                    at + Vec3::new(side * 0.55, 0.3, -0.4),
                    at + Vec3::new(side * 0.55, 0.3, 0.4),
                    0.1,
                    0.3,
                );
            }
            lightning(p, at + Vec3::Y * 1.3, at, Motif::Shock, 0.08);
        }
        18 => {
            coil(p, Motif::Virus, at, at + Vec3::Y * 1.2, 0.8 * scale);
            p.ring(Motif::Virus, at.with_y(0.12), 0.7 * scale, 0.4, 0.7);
            p.burst(Motif::Virus, at, rank, phase, true);
        }
        19 => {
            fissure(p, Motif::Steel, at, 0.6 * scale, phase);
            for i in 0..3 {
                p.emit(
                    Motif::Steel,
                    p.art.blade.clone(),
                    at + radial(i as f32 * 2.1) * 0.4 + Vec3::Y,
                    Vec3::splat(0.65 * scale),
                    Quat::from_rotation_z(std::f32::consts::PI),
                    Vec3::NEG_Y * 3.0,
                    0.35,
                    0.0,
                    0.0,
                );
            }
        }
        20 => {
            coil(
                p,
                Motif::Arcane,
                at + Vec3::Y * 0.1,
                at + Vec3::Y * 1.5,
                scale,
            );
            for i in 0..3 {
                let a = at + radial(phase + i as f32 * 2.1) * 0.9 + Vec3::Y;
                p.emit(
                    Motif::Arcane,
                    p.art.shard.clone(),
                    a,
                    Vec3::new(0.22, 0.5, 0.22),
                    Quat::from_rotation_z(0.6),
                    Vec3::ZERO,
                    0.5,
                    2.0,
                    0.0,
                );
            }
        }
        21 => {
            coil(p, Motif::Buff, at, at + Vec3::Y * 2.2, 0.65 * scale);
            for side in [-1.0, 1.0] {
                p.beam(
                    Motif::Buff,
                    at + Vec3::X * side * 0.9,
                    at + Vec3::X * side * 0.9 + Vec3::Y * 2.2,
                    0.12,
                    0.6,
                );
            }
            p.beam(
                Motif::Buff,
                at + Vec3::new(-0.9, 1.5, 0.0),
                at + Vec3::new(0.9, 1.5, 0.0),
                0.10,
                0.6,
            );
        }
        22 => {
            star(p, Motif::Buff, at + Vec3::Y * 0.5, 0.8 * scale, phase, 4);
            p.emit(
                Motif::Buff,
                p.art.shard.clone(),
                at + Vec3::Y,
                Vec3::new(0.4, 0.45, 0.4),
                Quat::IDENTITY,
                Vec3::Y * 0.4,
                0.5,
                2.0,
                0.0,
            );
        }
        23 => {
            fissure(p, Motif::Fire, at, 0.6 * scale, phase);
            p.emit(
                Motif::Fire,
                p.art.crescent.clone(),
                at + Vec3::Y * 0.4,
                Vec3::splat(0.9),
                Quat::from_rotation_x(0.5),
                Vec3::ZERO,
                0.35,
                8.0,
                0.0,
            );
        }
        24 => {
            p.beam(Motif::Buff, at + Vec3::Y * 8.0, at, 0.28 * scale, 0.45);
            p.beam(Motif::Steel, at + Vec3::Y * 8.0, at, 0.08, 0.5);
            coil(p, Motif::Buff, at, at + Vec3::Y * 8.0, scale);
            fissure(p, Motif::Buff, at, reach, phase);
        }
        25 => {
            for side in [-1.0, 1.0] {
                for feather in 0..5 {
                    let end = at
                        + Vec3::new(
                            side * (0.7 + feather as f32 * 0.3),
                            1.3 + feather as f32 * 0.25,
                            0.0,
                        );
                    let direction = end - (at + Vec3::Y * 0.5);
                    p.emit(
                        Motif::Fire,
                        p.art.blade.clone(),
                        at + Vec3::Y * 0.5,
                        Vec3::new(0.28, direction.length() * 0.65, 0.28),
                        Quat::from_rotation_arc(Vec3::Y, direction.normalize()),
                        direction * 0.12,
                        0.7,
                        0.0,
                        0.1,
                    );
                }
            }
        }
        26 => {
            shards(p, Motif::Ice, at, reach * 0.7, 1.5, 5, phase);
            lightning(p, at + Vec3::Y * 2.0, at, Motif::Ice, 0.15);
        }
        27 => {
            shards(p, Motif::Buff, at, reach * 0.8, 0.7, 6, phase);
            fissure(p, Motif::Steel, at, reach, phase);
        }
        28 => {
            for (i, motif) in [Motif::Fire, Motif::Shock, Motif::Ice]
                .into_iter()
                .enumerate()
            {
                let at = at + radial(phase + i as f32 * 2.1) * 0.95;
                p.emit(
                    motif,
                    p.art.coil.clone(),
                    at,
                    Vec3::splat(0.28),
                    Quat::IDENTITY,
                    Vec3::ZERO,
                    0.4,
                    3.0,
                    0.0,
                );
            }
        }
        29 => {
            coil(
                p,
                Motif::Arcane,
                at + Vec3::Y * 0.1,
                at + Vec3::Y * 1.5,
                1.25 * scale,
            );
            for y in [0.25, 1.25] {
                let corners = [
                    Vec3::new(-0.5, y, -0.5),
                    Vec3::new(0.5, y, -0.5),
                    Vec3::new(0.5, y, 0.5),
                    Vec3::new(-0.5, y, 0.5),
                ];
                for i in 0..4 {
                    p.beam(
                        Motif::Arcane,
                        at + corners[i],
                        at + corners[(i + 1) % 4],
                        0.075,
                        0.5,
                    );
                }
            }
            for x in [-0.5, 0.5] {
                for z in [-0.5, 0.5] {
                    p.beam(
                        Motif::Buff,
                        at + Vec3::new(x, 0.25, z),
                        at + Vec3::new(x, 1.25, z),
                        0.06,
                        0.5,
                    );
                }
            }
        }
        31 => {
            shards(p, Motif::Fire, at, reach * 0.7, 1.1, 6, phase);
            fissure(p, Motif::Fire, at, reach, phase);
        }
        _ => {}
    }
    if let Some(path) = path {
        use super::super::super::skill_evolution::UrEvolutionPath;
        match path {
            UrEvolutionPath::Surge => {
                for i in 0..3 {
                    p.beam(
                        Motif::Fire,
                        at + Vec3::Y * (0.3 + i as f32 * 0.2),
                        at + radial(phase + i as f32 * 2.1) * reach + Vec3::Y * 0.5,
                        0.16,
                        0.45,
                    );
                }
            }
            UrEvolutionPath::Control => {
                for i in 0..4 {
                    let offset = radial(i as f32 * std::f32::consts::FRAC_PI_2) * reach.min(1.8);
                    p.beam(
                        Motif::Ice,
                        at + offset,
                        at + offset + Vec3::Y * 2.0,
                        0.10,
                        0.65,
                    );
                }
            }
            UrEvolutionPath::Sustain => {
                p.emit(
                    Motif::Water,
                    p.art.coil.clone(),
                    at,
                    Vec3::new(0.85, 0.65, 0.85),
                    Quat::IDENTITY,
                    Vec3::Y * 0.2,
                    0.75,
                    2.0,
                    0.1,
                );
            }
        }
    }
    p.identity = previous;
}
pub(super) fn zone(p: &mut Painter, zone: &SkillZone, position: Vec3, rank: u8, phase: f32) {
    let previous = p.identity;
    p.identity = zone.profile.skill_index.map(usize::from);
    let radius = zone.radius;
    match zone.kind {
        ZoneKind::StormMatrix => {
            // Voltaic Snare's boundary restrikes curl inward, leaving the enemy readable.
            for strand in 0..3 {
                let angle = phase + strand as f32 * std::f32::consts::TAU / 3.0;
                let edge = position + radial(angle) * radius * 0.85;
                lightning(
                    p,
                    edge + Vec3::Y * (1.0 + f32::from(rank) * 0.15),
                    position + radial(angle + 0.8) * radius * 0.55,
                    Motif::Shock,
                    0.12,
                );
            }
            fissure(p, Motif::Shock, position, radius, phase * 0.03);
        }
        ZoneKind::GlacierCore => {
            shards(
                p,
                Motif::Ice,
                position,
                radius * 0.8,
                0.65 + rank as f32 * 0.18,
                6,
                phase * 0.03,
            );
            fissure(p, Motif::Ice, position, radius, 0.0);
        }
        ZoneKind::VortexColumn => vortex(p, position, radius, phase),
    }
    p.identity = previous;
}

fn vortex(p: &mut Painter, position: Vec3, radius: f32, phase: f32) {
    p.emit(
        Motif::Water,
        p.art.vortex.clone(),
        position,
        Vec3::new(radius, 1.4, radius),
        Quat::from_rotation_y(phase),
        Vec3::ZERO,
        0.38,
        2.4,
        0.0,
    );
    let size = (radius / 4.0).clamp(0.75, 2.0);
    for strand in 0..3 {
        let angle = phase + strand as f32 * std::f32::consts::TAU / 3.0;
        let offset = Vec3::new(angle.cos(), 0.0, angle.sin()) * radius * 0.64;
        let tangent = Vec3::new(-angle.sin(), 0.0, angle.cos());
        p.emit(
            Motif::Water,
            p.art.shard.clone(),
            position + offset + Vec3::Y * 0.3,
            Vec3::new(0.16, 0.65, 0.16) * size,
            Quat::from_rotation_y(-angle),
            tangent * 1.4 + Vec3::Y * 0.7,
            0.32,
            1.8,
            0.0,
        );
    }
}

pub(super) fn projectile(
    p: &mut Painter,
    index: usize,
    at: Vec3,
    velocity: Vec3,
    rank: u8,
    phase: f32,
) {
    let previous = p.identity.replace(index);
    let forward = velocity.normalize_or(Vec3::X);
    let rotation = Quat::from_rotation_arc(Vec3::Y, forward);
    let scale = rank_size(rank);
    match index {
        1 => {
            p.emit(
                Motif::Fire,
                p.art.shard.clone(),
                at,
                Vec3::new(0.18, 0.7, 0.18) * scale,
                rotation,
                Vec3::ZERO,
                0.16,
                0.0,
                0.0,
            );
            p.beam(Motif::Buff, at - forward * 1.1, at, 0.06, 0.16);
        }
        2 => p.emit(
            Motif::Ice,
            p.art.crystal.clone(),
            at,
            Vec3::new(0.24, 0.9, 0.24) * scale,
            rotation,
            Vec3::ZERO,
            0.15,
            0.0,
            0.0,
        ),
        7 => p.emit(
            Motif::Steel,
            p.art.blade.clone(),
            at - forward * 0.4,
            Vec3::new(0.5, 0.7, 0.5),
            rotation,
            Vec3::ZERO,
            0.12,
            0.0,
            0.0,
        ),
        8 => star(p, Motif::Arcane, at, 0.55 * scale, phase, 5),
        _ => {}
    }
    p.identity = previous;
}
