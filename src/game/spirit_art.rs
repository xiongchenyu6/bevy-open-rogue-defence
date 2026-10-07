//! A single pooled, vertex-colored companion: hood, recessed mask and curling cloth tail.
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

const CLOTH: [f32; 4] = [0.36, 0.40, 0.58, 1.0];
const HEM: [f32; 4] = [0.48, 0.53, 0.70, 1.0];
const MASK: [f32; 4] = [0.035, 0.055, 0.10, 1.0];
const EYE: [f32; 4] = [0.70, 0.98, 1.0, 1.0];
const SIDES: usize = 8;

pub(crate) fn companion_material() -> StandardMaterial {
    StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.92,
        metallic: 0.0,
        reflectance: 0.12,
        emissive: LinearRgba::rgb(0.008, 0.012, 0.018),
        ..default()
    }
}

pub(crate) fn companion_mesh() -> Mesh {
    let mut art = Sculpt::default();
    // The changing X/Z centers form a real three-dimensional curl, not a flat cone.
    art.tube(
        &[
            (Vec3::new(0.0, 0.70, 0.015), Vec2::new(0.23, 0.15)),
            (Vec3::new(0.0, 0.49, 0.015), Vec2::new(0.19, 0.125)),
            (Vec3::new(0.025, 0.28, 0.045), Vec2::new(0.11, 0.075)),
            (Vec3::new(0.095, 0.12, 0.12), Vec2::new(0.055, 0.04)),
            (Vec3::new(0.18, 0.0, 0.21), Vec2::ZERO),
        ],
        CLOTH,
    );

    // Hood opening lies in XY and looks along -Z; its bevel surrounds an inset mask.
    let center = Vec3::new(0.0, 0.86, 0.0);
    let ring = |i: usize, radius: Vec2, z: f32| {
        let angle = i as f32 * std::f32::consts::TAU / SIDES as f32;
        center + Vec3::new(angle.cos() * radius.x, angle.sin() * radius.y, z)
    };
    for i in 0..SIDES {
        let j = (i + 1) % SIDES;
        let a = ring(i, Vec2::new(0.235, 0.29), -0.15);
        let b = ring(j, Vec2::new(0.235, 0.29), -0.15);
        let c = ring(j, Vec2::new(0.145, 0.20), -0.235);
        let d = ring(i, Vec2::new(0.145, 0.20), -0.235);
        art.quad(a, b, c, d, HEM, Vec3::NEG_Z);
        let back_a = ring(i, Vec2::new(0.17, 0.22), 0.12);
        let back_b = ring(j, Vec2::new(0.17, 0.22), 0.12);
        let radial = (a + b) * 0.5 - center;
        art.quad(a, back_a, back_b, b, CLOTH, radial);
        art.triangle(
            back_a,
            center + Vec3::new(0.0, 0.02, 0.18),
            back_b,
            CLOTH,
            Vec3::Z,
        );
        let mask_a = ring(i, Vec2::new(0.145, 0.20), -0.195);
        let mask_b = ring(j, Vec2::new(0.145, 0.20), -0.195);
        art.quad(d, c, mask_b, mask_a, MASK, center - (c + d) * 0.5);
        art.triangle(
            center + Vec3::new(0.0, 0.0, -0.195),
            mask_a,
            mask_b,
            MASK,
            Vec3::NEG_Z,
        );
    }
    for side in [-1.0, 1.0] {
        let eye = Vec3::new(side * 0.065, 0.87, -0.203);
        // Slanted, angular eyes stay inside the hood rather than becoming floating orbs.
        let points = [
            eye + Vec3::new(-0.038, 0.008 * side, 0.0),
            eye + Vec3::new(0.0, 0.023, -0.002),
            eye + Vec3::new(0.038, -0.008 * side, 0.0),
            eye + Vec3::new(0.0, -0.018, -0.002),
        ];
        art.quad(points[0], points[1], points[2], points[3], EYE, Vec3::NEG_Z);
        art.tube(
            &[
                (
                    Vec3::new(side * 0.205, 0.66, 0.005),
                    Vec2::new(0.055, 0.035),
                ),
                (
                    Vec3::new(side * 0.305, 0.55, -0.025),
                    Vec2::new(0.045, 0.025),
                ),
                (Vec3::new(side * 0.30, 0.38, 0.065), Vec2::new(0.03, 0.02)),
                (Vec3::new(side * 0.24, 0.26, 0.16), Vec2::ZERO),
            ],
            HEM,
        );
    }
    art.finish()
}

#[derive(Default)]
struct Sculpt {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    uv: Vec<[f32; 2]>,
}
impl Sculpt {
    fn triangle(&mut self, a: Vec3, mut b: Vec3, mut c: Vec3, color: [f32; 4], outward: Vec3) {
        let cross = (b - a).cross(c - a);
        if cross.length_squared() < 1e-12 {
            return;
        }
        if cross.dot(outward) < 0.0 {
            std::mem::swap(&mut b, &mut c);
        }
        let normal = (b - a).cross(c - a).normalize().to_array();
        for p in [a, b, c] {
            self.positions.push(p.to_array());
            self.normals.push(normal);
            self.colors.push(color);
            self.uv.push([p.x + 0.5, p.y / 1.15]);
        }
    }
    fn quad(&mut self, a: Vec3, b: Vec3, c: Vec3, d: Vec3, color: [f32; 4], outward: Vec3) {
        self.triangle(a, b, c, color, outward);
        self.triangle(a, c, d, color, outward);
    }
    fn tube(&mut self, rings: &[(Vec3, Vec2)], color: [f32; 4]) {
        let point = |ring: &(Vec3, Vec2), i: usize| {
            let angle = i as f32 * std::f32::consts::TAU / SIDES as f32;
            ring.0 + Vec3::new(angle.cos() * ring.1.x, 0.0, angle.sin() * ring.1.y)
        };
        for pair in rings.windows(2) {
            for i in 0..SIDES {
                let j = (i + 1) % SIDES;
                let outward = (point(&pair[0], i) + point(&pair[0], j)) * 0.5 - pair[0].0;
                self.quad(
                    point(&pair[0], i),
                    point(&pair[1], i),
                    point(&pair[1], j),
                    point(&pair[0], j),
                    color,
                    outward,
                );
            }
        }
        for i in 0..SIDES {
            self.triangle(
                rings[0].0,
                point(&rings[0], i),
                point(&rings[0], (i + 1) % SIDES),
                color,
                Vec3::Y,
            );
        }
    }
    fn finish(self) -> Mesh {
        let indices = (0..self.positions.len() as u32).collect();
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uv)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
        .with_inserted_indices(Indices::U32(indices))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::mesh::VertexAttributeValues;

    #[test]
    fn companion_has_valid_flat_shaded_geometry_and_requested_bounds() {
        let mesh = companion_mesh();
        let Some(VertexAttributeValues::Float32x3(p)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("positions");
        };
        let Some(VertexAttributeValues::Float32x3(n)) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("normals");
        };
        assert!(p.len() < 2000);
        assert_eq!(p.len(), n.len());
        assert_eq!(mesh.attribute(Mesh::ATTRIBUTE_UV_0).unwrap().len(), p.len());
        assert_eq!(
            mesh.attribute(Mesh::ATTRIBUTE_COLOR).unwrap().len(),
            p.len()
        );
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for (position, normal) in p.iter().zip(n) {
            let v = Vec3::from_array(*position);
            assert!(v.is_finite());
            let normal = Vec3::from_array(*normal);
            assert!(normal.is_finite());
            assert!((normal.length() - 1.0).abs() < 1e-5);
            min = min.min(v);
            max = max.max(v);
        }
        assert!(((max - min).y - 1.15).abs() < 1e-5);
        assert!(((max - min).x - 0.70).abs() < 1e-5);
        let Indices::U32(indices) = mesh.indices().unwrap() else {
            panic!("u32 indices");
        };
        for face in indices.as_chunks::<3>().0 {
            assert!(face.iter().all(|i| (*i as usize) < p.len()));
            let [a, b, c] = [
                p[face[0] as usize],
                p[face[1] as usize],
                p[face[2] as usize],
            ]
            .map(Vec3::from_array);
            assert!(
                (b - a)
                    .cross(c - a)
                    .dot(Vec3::from_array(n[face[0] as usize]))
                    > 0.0
            );
        }
    }

    #[test]
    fn eyes_face_negative_z_and_tail_curls_in_three_dimensions() {
        let mesh = companion_mesh();
        let Some(VertexAttributeValues::Float32x3(p)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("positions");
        };
        let Some(VertexAttributeValues::Float32x3(n)) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("normals");
        };
        let Some(VertexAttributeValues::Float32x4(colors)) = mesh.attribute(Mesh::ATTRIBUTE_COLOR)
        else {
            panic!("colors");
        };
        let eyes: Vec<_> = p
            .iter()
            .zip(n)
            .zip(colors)
            .filter(|(_, color)| **color == EYE)
            .collect();
        assert_eq!(eyes.len(), 12);
        assert!(eyes.iter().all(|((p, n), _)| p[2] < -0.19 && n[2] < -0.98));
        assert!(eyes.iter().any(|((p, _), _)| p[0] < 0.0));
        assert!(eyes.iter().any(|((p, _), _)| p[0] > 0.0));
        assert!(p.iter().any(|p| p[1] < 0.01 && p[0] > 0.17 && p[2] > 0.20));
        assert!(p.iter().any(|p| p[1] > 0.27 && p[1] < 0.29 && p[2] < 0.0));
    }
}
