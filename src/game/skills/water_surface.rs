//! Flow texture and spring emergence adapted from LinearAbiltyCastingThreeJS.
//! Copyright (c) 2026 mohamedachrefelouafi, MIT. See third-party notice.
//! The reference's ridged flowing veil is baked once into a small repeating map.
//! Shared scrolling UVs avoid one shader/material allocation per moving wave.
use bevy::{
    asset::RenderAssetUsages,
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

// Port of noise.glsl.js hash11. Input lattice indices are nonnegative.
fn hash(mut p: f32) -> f32 {
    p = (p * 0.1031).fract();
    p *= p + 33.33;
    p *= p + p;
    p.fract()
}
fn noise(x: f32, y: f32, period: i32) -> f32 {
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let fx = x - x.floor();
    let fy = y - y.floor();
    let sx = fx * fx * (3.0 - 2.0 * fx);
    let sy = fy * fy * (3.0 - 2.0 * fy);
    let sample = |dx: i32, dy: i32| {
        hash(((ix + dx).rem_euclid(period) + (iy + dy).rem_euclid(period) * period) as f32)
    };
    let a = sample(0, 0) + (sample(1, 0) - sample(0, 0)) * sx;
    let b = sample(0, 1) + (sample(1, 1) - sample(0, 1)) * sx;
    a + (b - a) * sy
}

pub(super) fn flow_texture(foam: bool) -> Image {
    const SIZE: u32 = 128;
    let mut bytes = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let u = x as f32 / SIZE as f32;
            let v = y as f32 / SIZE as f32;
            let warp = noise(u * 4.0, v * 4.0, 4) * 0.7;
            let mut ridge = 0.0;
            let mut weight = 0.57;
            for period in [8, 16, 32] {
                let n = noise(u * period as f32 + warp, v * period as f32, period);
                ridge += (1.0 - (n * 2.0 - 1.0).abs()).powi(3) * weight;
                weight *= 0.5;
            }
            // FrostFieldMaterial's bright seams and eroded veil become foam
            // filaments. Leave gaps so the curled sheet remains translucent.
            let filaments = ((ridge - 0.32) / 0.43).clamp(0.0, 1.0);
            let alpha = if foam {
                0.48 + filaments * 0.48
            } else {
                0.22 + filaments * 0.65
            };
            let light = if foam {
                (218.0 + filaments * 37.0) as u8
            } else {
                (155.0 + filaments * 100.0) as u8
            };
            bytes.extend([light, light, 255, (alpha * 255.0) as u8]);
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        bytes,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    });
    image
}

// GlacierAbility._emergence, retimed for a travelling water crest.
pub(super) fn emergence(age: f32) -> f32 {
    let rise_time = 0.12;
    let rise = 1.0 - (1.0 - (age / rise_time).clamp(0.0, 1.0)).powi(5);
    if age <= rise_time {
        return rise;
    }
    let after = age - rise_time;
    1.0 + 0.12 * (after * 14.0).sin() * (-after / 0.22).exp()
}
