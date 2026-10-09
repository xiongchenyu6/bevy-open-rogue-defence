//! Thin, tapered wind ribbons; empty space preserves the combat silhouettes.
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

pub(super) fn mesh() -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();
    for strand in 0..3 {
        let start = positions.len() as u32;
        for step in 0..=64 {
            let t = step as f32 / 64.0;
            let taper = (t * std::f32::consts::PI).sin();
            let angle =
                strand as f32 * std::f32::consts::TAU / 3.0 + t * std::f32::consts::TAU * 1.35;
            let radius = 0.24 + 0.64 * t;
            let half_width = 0.003 + 0.013 * taper;
            for edge in [-half_width, half_width] {
                positions.push([
                    angle.cos() * (radius + edge),
                    0.08 + t * 2.25,
                    angle.sin() * (radius + edge),
                ]);
                normals.push([0.0, 1.0, 0.0]);
                colors.push([0.75 + 0.25 * t, 0.95, 1.0, 0.18 + 0.78 * taper]);
            }
            // Short breaks prevent the silhouette becoming a solid funnel.
            if step < 64 && step % 11 != 10 {
                let a = start + step * 2;
                indices.extend([a, a + 1, a + 2, a + 1, a + 3, a + 2]);
            }
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(indices))
}

pub(super) fn material() -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgba(0.52, 0.88, 1.0, 0.92),
        emissive: LinearRgba::rgb(5.2, 8.8, 10.0),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    }
}

pub(super) fn ground_material() -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgba(0.16, 0.45, 0.55, 0.08),
        emissive: LinearRgba::rgb(0.04, 0.12, 0.16),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    }
}
