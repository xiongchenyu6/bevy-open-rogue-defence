//! Skill identities layered onto real impacts, zones and summoned actors.
//! Uses the existing presentation budget; there is no additional damage or timer.
use super::*;
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
};

pub(super) fn helix_mesh() -> Mesh {
    let mut points = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    for step in 0..=96 {
        let t = step as f32 / 96.0;
        let angle = t * std::f32::consts::TAU * 2.5;
        let radius = 0.25 + 0.75 * t;
        for edge in [-0.07, 0.07] {
            points.push([
                angle.cos() * (radius + edge),
                t * 2.7,
                angle.sin() * (radius + edge),
            ]);
            normals.push([0.0, 1.0, 0.0]);
        }
        if step < 96 {
            let a = step * 2;
            indices.extend([a, a + 1, a + 2, a + 1, a + 3, a + 2]);
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, points)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_indices(Indices::U32(indices))
}
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
    let side = delta.cross(Vec3::Y).normalize_or(Vec3::Z);
    let mut previous = from;
    for i in 1..=6 {
        let t = i as f32 / 6.0;
        let next = from
            + delta * t
            + if i < 6 {
                side * if i % 2 == 0 { 0.34 } else { -0.34 } + Vec3::Y * 0.12
            } else {
                Vec3::ZERO
            };
        p.beam(motif, previous, next, width, 0.26);
        previous = next;
    }
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
        let offset = radial(angle) * radius;
        p.emit(
            motif,
            p.art.shard.clone(),
            at + offset,
            Vec3::new(0.23, height, 0.23),
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
        p.beam(motif, at, at + direction * radius, 0.10, 0.38);
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
                p.art.shard.clone(),
                at + Vec3::Y * 0.9,
                Vec3::new(0.32, 1.5, 0.32) * scale,
                Quat::from_rotation_z(0.3),
                Vec3::ZERO,
                0.5,
                0.0,
                0.0,
            );
            star(p, Motif::Ice, at + Vec3::Y * 0.1, 0.7, phase, 6);
        }
        3 | 6 | 30 => {} // Dedicated water front, chakram, and continuous beam own their silhouette.
        4 => {
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
            for i in 0..3 {
                let end = at + Vec3::new((i as f32 - 1.0) * 0.25, 0.3, 0.0);
                p.beam(Motif::Steel, end - Vec3::X * 1.2, end, 0.06 * scale, 0.25);
            }
        }
        8 => star(p, Motif::Arcane, at + Vec3::Y * 0.6, 0.9 * scale, phase, 8),
        9 => {
            for i in 0..6 {
                let direction = radial(i as f32 * std::f32::consts::TAU / 6.0);
                let knee = at + direction * reach * 0.55 + Vec3::Y * 0.03;
                p.beam(Motif::Buff, at.with_y(0.05), knee, 0.12, 0.55);
                p.beam(
                    Motif::Buff,
                    knee,
                    at + direction * reach + radial(i as f32 + 1.0) * 0.14,
                    0.065,
                    0.55,
                );
            }
        }
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
            star(p, Motif::Shock, at + Vec3::Y * 0.05, reach, phase, 5);
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
            star(p, Motif::Ice, at.with_y(0.07), reach, phase * 0.1, 6);
        }
        15 => p.emit(
            Motif::Water,
            p.art.helix.clone(),
            at,
            Vec3::new(reach, 1.3, reach),
            Quat::IDENTITY,
            Vec3::ZERO,
            0.45,
            6.0,
            0.05,
        ),
        16 => {
            p.emit(
                Motif::Water,
                p.art.helix.clone(),
                at,
                Vec3::new(0.65, 0.5, 0.65) * scale,
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
            p.ring(Motif::Virus, at.with_y(0.12), 0.7 * scale, 0.4, 0.7);
            p.burst(Motif::Virus, at, rank, phase, true);
        }
        19 => {
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
            star(p, Motif::Buff, at.with_y(0.08), reach, 0.0, 8);
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
                    p.beam(Motif::Fire, at + Vec3::Y * 0.5, end, 0.14, 0.7);
                }
            }
        }
        26 => {
            shards(p, Motif::Ice, at, reach * 0.7, 1.5, 5, phase);
            lightning(p, at + Vec3::Y * 2.0, at, Motif::Ice, 0.15);
        }
        27 => {
            shards(p, Motif::Buff, at, reach * 0.8, 0.7, 6, phase);
            star(p, Motif::Steel, at.with_y(0.1), reach, phase, 6);
        }
        28 => {
            for (i, motif) in [Motif::Fire, Motif::Shock, Motif::Ice]
                .into_iter()
                .enumerate()
            {
                let at = at + radial(phase + i as f32 * 2.1) * 0.95;
                p.emit(
                    motif,
                    p.art.helix.clone(),
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
            star(p, Motif::Buff, at.with_y(0.1), reach, phase, 5);
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
                    p.art.helix.clone(),
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
            for row in -2_i32..=2 {
                let x = row as f32 * radius / 3.0;
                let z = (radius * radius - x * x).sqrt();
                p.beam(
                    Motif::Shock,
                    position + Vec3::new(x, 0.1, -z),
                    position + Vec3::new(x, 0.1, z),
                    0.06,
                    0.4,
                );
                p.beam(
                    Motif::Shock,
                    position + Vec3::new(-z, 0.1, x),
                    position + Vec3::new(z, 0.1, x),
                    0.06,
                    0.4,
                );
            }
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
            star(p, Motif::Ice, position.with_y(0.05), radius, 0.0, 6);
        }
        ZoneKind::VortexColumn => p.emit(
            Motif::Water,
            p.art.helix.clone(),
            position,
            Vec3::new(radius, 1.6, radius),
            Quat::from_rotation_y(phase),
            Vec3::ZERO,
            0.4,
            5.0,
            0.0,
        ),
    }
    p.identity = previous;
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
            p.art.shard.clone(),
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
