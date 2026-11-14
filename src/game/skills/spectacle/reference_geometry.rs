//! CPU ports of LinearAbiltyCastingThreeJS crystal, coil, bolt and fissure geometry.
//! Source revision and MIT notice are in THIRD_PARTY_NOTICES/LinearAbilityCasting.txt.
//! Meshes are built once. Every cast shares them and stays in the presentation budget.
use super::*;
use std::f32::consts::{PI, TAU};

fn hash(x: f32) -> f32 {
    (x.sin() * 43_758.547).fract().abs()
}
fn mesh(points: Vec<[f32; 3]>, colors: Vec<[f32; 4]>) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, points)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.compute_flat_normals();
    mesh
}
fn triangle(
    points: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    a: Vec3,
    b: Vec3,
    c: Vec3,
    color: [f32; 4],
) {
    points.extend([a.to_array(), b.to_array(), c.to_array()]);
    colors.extend([color; 3]);
}

/// ProceduralGeometry.js createCrystalGeometry: irregular rings, bent axis, chipped apex.
pub(in super::super::super) fn crystal_mesh() -> Mesh {
    let seed = 7.3;
    let facets = 6;
    let bend = hash(seed * 1.77) * TAU;
    let axis = Vec3::new(bend.cos(), 0.0, bend.sin());
    let heights: [f32; 5] = [0.0, 0.22, 0.5, 0.75, 0.92];
    let rings: Vec<Vec<Vec3>> = heights
        .iter()
        .enumerate()
        .map(|(ring, &t)| {
            (0..facets)
                .map(|i| {
                    let angle = i as f32 / facets as f32 * TAU
                        + (hash(seed * 3.13 + i as f32 * 7.7) - 0.5) * TAU / facets as f32 * 0.46;
                    let radius = (0.13 + 0.87 * (1.0 - t).powf(1.15))
                        * 0.5
                        * (1.0
                            + (hash(seed * 11.1 + ring as f32 * 13.7 + i as f32 * 3.9) - 0.5)
                                * 0.36
                                * (0.35 + 0.65 * t));
                    Vec3::new(angle.cos() * radius, t, angle.sin() * radius)
                        + axis * 0.11 * t.powf(1.6)
                })
                .collect()
        })
        .collect();
    let mut points = Vec::new();
    let mut colors = Vec::new();
    for ring in 0..4 {
        for i in 0..facets {
            let j = (i + 1) % facets;
            let light = 0.40 + hash(i as f32 * 2.7) * 0.6;
            let color = [light, light, light, 1.0];
            triangle(
                &mut points,
                &mut colors,
                rings[ring][i],
                rings[ring][j],
                rings[ring + 1][i],
                color,
            );
            triangle(
                &mut points,
                &mut colors,
                rings[ring][j],
                rings[ring + 1][j],
                rings[ring + 1][i],
                color,
            );
        }
    }
    for i in 0..facets {
        let j = (i + 1) % facets;
        triangle(
            &mut points,
            &mut colors,
            rings[4][i],
            rings[4][j],
            Vec3::Y + axis * 0.11,
            [1.0; 4],
        );
        triangle(
            &mut points,
            &mut colors,
            Vec3::ZERO,
            rings[0][j],
            rings[0][i],
            [0.3, 0.3, 0.3, 1.0],
        );
    }
    mesh(points, colors)
}

fn ribbon(
    points: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    path: &[(Vec3, f32)],
    normal: Vec3,
) {
    for pair in path.windows(2) {
        let side = (pair[1].0 - pair[0].0).cross(normal).normalize_or(Vec3::X);
        let a = pair[0].0 - side * pair[0].1;
        let b = pair[0].0 + side * pair[0].1;
        let c = pair[1].0 - side * pair[1].1;
        let d = pair[1].0 + side * pair[1].1;
        triangle(points, colors, a, b, c, [1.0; 4]);
        triangle(points, colors, b, d, c, [1.0; 4]);
    }
}

/// BeamMaterial.js coilPoint: phase-separated ribbons with a flared downrange radius.
pub(in super::super::super) fn coil_mesh() -> Mesh {
    let mut points = Vec::new();
    let mut colors = Vec::new();
    for strand in 0..2 {
        let path: Vec<_> = (0..=80)
            .map(|i| {
                let t = i as f32 / 80.0;
                let angle = (t * 2.5 + strand as f32 * 0.5) * TAU;
                let radius = 0.32 * (1.0 + 0.65 * t.powi(3));
                (
                    Vec3::new(angle.cos() * radius, t, angle.sin() * radius),
                    0.024 * (PI * t).sin().max(0.08),
                )
            })
            .collect();
        ribbon(&mut points, &mut colors, &path, Vec3::Z);
    }
    mesh(points, colors)
}

fn value_noise(x: f32, seed: f32) -> f32 {
    let i = x.floor();
    let f = x - i;
    (hash(i + seed) * (1.0 - f) + hash(i + 1.0 + seed) * f) * 2.0 - 1.0
}
/// LightningMaterial.js vnoise/kink: octave linear noise retains sharp bolt corners.
pub(in super::super::super) fn lightning_mesh() -> Mesh {
    let mut points = Vec::new();
    let mut colors = Vec::new();
    for strand in 0..3 {
        let path: Vec<_> = (0..=48)
            .map(|i| {
                let t = i as f32 / 48.0;
                let mut offset = Vec3::ZERO;
                let mut amplitude = 0.11;
                let mut frequency = 7.0;
                for octave in 0..3 {
                    let seed = strand as f32 * 7.13 + octave as f32 * 13.0;
                    offset.x += amplitude * value_noise(t * frequency, seed);
                    offset.z += amplitude * value_noise(t * frequency, seed + 71.3);
                    amplitude *= 0.46;
                    frequency *= 2.0;
                }
                (
                    Vec3::Y * t + offset * (PI * t).sin(),
                    if strand == 0 { 0.018 } else { 0.007 },
                )
            })
            .collect();
        ribbon(&mut points, &mut colors, &path, Vec3::Z);
        ribbon(&mut points, &mut colors, &path, Vec3::X);
    }
    mesh(points, colors)
}

/// GroundFissures.js _generate: meandering uneven arms, tapered widths and side branches.
pub(in super::super::super) fn fissure_mesh() -> Mesh {
    let mut points = Vec::new();
    let mut colors = Vec::new();
    for arm in 0..7 {
        let heading = arm as f32 / 7.0 * TAU + (hash(arm as f32 * 4.1) - 0.5) * 0.9;
        let length = 0.6 + hash(arm as f32 + 8.1) * 0.4;
        let mut angle = heading;
        let mut pos = radial(angle) * 0.06;
        let mut path = Vec::new();
        for step in 0..=22 {
            let t = step as f32 / 22.0;
            let width = 0.024 * ((1.0 - t) / 0.22).clamp(0.0, 1.0).powf(0.65);
            path.push((pos, width));
            if step == 8 || step == 14 {
                let branch_angle = angle + if step == 8 { 0.85 } else { -1.1 };
                let branch: Vec<_> = (0..=6)
                    .map(|i| {
                        let t = i as f32 / 6.0;
                        (
                            pos + radial(branch_angle + t * 0.2) * t * 0.3,
                            0.014 * (1.0 - t).powf(0.7),
                        )
                    })
                    .collect();
                ribbon(&mut points, &mut colors, &branch, Vec3::Y);
            }
            pos += radial(angle) * length / 22.0;
            angle += (hash(arm as f32 * 17.0 + step as f32 * 3.7) - 0.5) * 0.45;
            angle += (heading - angle) * 0.06;
        }
        ribbon(&mut points, &mut colors, &path, Vec3::Y);
    }
    mesh(points, colors)
}
