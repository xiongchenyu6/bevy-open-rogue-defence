//! Thin, tapered wind ribbons; empty space preserves the combat silhouettes.
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

// FrostFieldMaterial's falling veil treatment adapted into a bounded wind mesh.
// Copyright (c) 2026 mohamedachrefelouafi, MIT.
// See THIRD_PARTY_NOTICES/LinearAbilityCasting.txt.
// Feathered bands leave the combat silhouette visible inside the funnel.
pub(super) fn mesh() -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();
    for strand in 0..5 {
        for feather in 0..3 {
            let start = positions.len() as u32;
            for step in 0..=96 {
                let t = step as f32 / 96.0;
                let taper = (t * std::f32::consts::PI).sin().max(0.0);
                let phase = strand as f32 * std::f32::consts::TAU / 5.0;
                let angle = phase + t * std::f32::consts::TAU * 1.65;
                let billow =
                    (angle * 3.0 + t * 11.0).sin() * 0.035 + (angle * 5.0 - t * 19.0).sin() * 0.018;
                let radius = 0.20 + 0.67 * t.powf(0.82) + billow * taper;
                let feather_offset = feather as f32 * 0.035;
                let half_width = (0.012 + 0.045 * taper) * (1.0 - feather as f32 * 0.22);
                for edge in [-half_width, half_width] {
                    positions.push([
                        angle.cos() * (radius + edge + feather_offset),
                        0.04 + t * 2.25 + billow * 1.5,
                        angle.sin() * (radius + edge + feather_offset),
                    ]);
                    normals.push([angle.cos(), 0.4, angle.sin()]);
                    let fade = taper * (1.0 - feather as f32 * 0.28);
                    colors.push([0.48 + 0.40 * t, 0.80 + 0.18 * t, 1.0, fade * 0.58]);
                }
                // Uneven breaks at the high rim act as the reference veil's
                // height-dependent erosion, instead of a hard cylinder edge.
                let torn = t > 0.6 && (step + strand * 7 + feather * 3) % 13 > 9;
                if step < 96 && !torn {
                    let a = start + step * 2;
                    indices.extend([a, a + 1, a + 2, a + 1, a + 3, a + 2]);
                }
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
        emissive: LinearRgba::rgb(1.5, 3.2, 4.2),
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
