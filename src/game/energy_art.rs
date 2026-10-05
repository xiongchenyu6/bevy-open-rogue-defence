//! Soft, open energy silhouettes; physical orbiting weapons keep their own meshes.
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

pub(crate) fn halo_mesh() -> Mesh {
    let mut positions = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();
    for arc in 0..3 {
        let start = arc as f32 * std::f32::consts::TAU / 3.0;
        let base = positions.len() as u32;
        for step in 0..=32 {
            let t = step as f32 / 32.0;
            let a = start + t * 1.6;
            let taper = (t * std::f32::consts::PI).sin().clamp(0.0, 1.0);
            for edge in 0..3 {
                let radius = 0.94 + (edge as f32 - 1.0) * 0.075 * taper;
                positions.push([a.cos() * radius, 0.0, a.sin() * radius]);
                colors.push([1.0, 1.0, 1.0, if edge == 1 { taper * 0.85 } else { 0.0 }]);
            }
            if step < 32 {
                for edge in 0..2 {
                    let v = base + step * 3 + edge;
                    indices.extend_from_slice(&[v, v + 3, v + 1, v + 1, v + 3, v + 4]);
                }
            }
        }
    }
    finish(positions, colors, indices)
}

pub(crate) fn wisp_mesh() -> Mesh {
    let mut positions = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();
    for plane in 0..3 {
        let a = plane as f32 * std::f32::consts::PI / 3.0;
        let base = positions.len() as u32;
        positions.push([0.0, 0.0, 0.0]);
        colors.push([1.0, 1.0, 1.0, 0.7]);
        for step in 0..=24 {
            let t = step as f32 * std::f32::consts::TAU / 24.0;
            let width = t.cos() * 0.5;
            positions.push([width * a.cos(), t.sin(), width * a.sin()]);
            colors.push([1.0, 1.0, 1.0, 0.0]);
            if step < 24 {
                indices.extend_from_slice(&[base, base + step + 1, base + step + 2]);
            }
        }
    }
    finish(positions, colors, indices)
}
fn finish(positions: Vec<[f32; 3]>, colors: Vec<[f32; 4]>, indices: Vec<u32>) -> Mesh {
    let n = positions.len();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; n])
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.5, 0.5]; n])
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(indices))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::mesh::VertexAttributeValues;
    #[test]
    fn energy_has_transparent_edges_and_bounded_geometry() {
        for mesh in [halo_mesh(), wisp_mesh()] {
            let Some(VertexAttributeValues::Float32x4(colors)) =
                mesh.attribute(Mesh::ATTRIBUTE_COLOR)
            else {
                panic!("vertex alpha absent");
            };
            assert!(colors.iter().any(|v| v[3] == 0.0));
            assert!(colors.iter().any(|v| v[3] > 0.5));
            assert!(colors.iter().all(|v| (0.0..=1.0).contains(&v[3])));
            assert!(mesh.count_vertices() < 400);
        }
    }
}
