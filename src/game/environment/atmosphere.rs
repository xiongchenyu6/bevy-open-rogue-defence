//! Quiet environmental motion and worn material transitions, below combat contrast.
use super::*;
use bevy::{
    asset::RenderAssetUsages,
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    light::{NotShadowCaster, NotShadowReceiver},
    mesh::{Indices, PrimitiveTopology},
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

#[derive(Component)]
struct CampAtmosphere {
    camp: usize,
    index: usize,
    center: Vec3,
}
#[derive(Component)]
struct CampLamp {
    camp: usize,
    base: f32,
}
#[derive(Resource, Default)]
struct AtmosphereClock(f32);

fn grain(x: u32, y: u32) -> f32 {
    let mut n = x
        .wrapping_mul(374761393)
        .wrapping_add(y.wrapping_mul(668265263));
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    (n ^ (n >> 16)) as f32 / u32::MAX as f32
}
// Periodic, smoothly interpolated noise avoids square pixels and repeat seams.
fn soft_grain(u: f32, v: f32, cells: u32) -> f32 {
    let p = Vec2::new(u, v) * cells as f32;
    let i = p.floor().as_uvec2();
    let f = p.fract();
    let t = f * f * (Vec2::splat(3.0) - 2.0 * f);
    let a = grain(i.x % cells, i.y);
    let b = grain((i.x + 1) % cells, i.y);
    let c = grain(i.x % cells, i.y + 1);
    let d = grain((i.x + 1) % cells, i.y + 1);
    (a + (b - a) * t.x) * (1.0 - t.y) + (c + (d - c) * t.x) * t.y
}
fn image(patina: bool) -> Image {
    const SIZE: u32 = 128;
    let mut pixels = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let u = x as f32 / (SIZE - 1) as f32;
            let v = y as f32 / (SIZE - 1) as f32;
            let noise = soft_grain(u, v, 8) * 0.65 + soft_grain(u, v, 3) * 0.35;
            let alpha = if patina {
                (v * (1.0 - v) * 4.0).powf(1.5) * (noise * 2.5 - 0.9).clamp(0.0, 1.0) * 0.54
            } else {
                let r = ((u - 0.5).powi(2) * 4.0 + (v - 0.5).powi(2) * 4.0).sqrt();
                (1.0 - r).max(0.0).powi(2) * (0.5 + noise * 0.5)
            };
            pixels.extend([255, 255, 255, (alpha * 255.0) as u8]);
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    if patina {
        image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            address_mode_u: ImageAddressMode::Repeat,
            address_mode_v: ImageAddressMode::ClampToEdge,
            mag_filter: ImageFilterMode::Linear,
            min_filter: ImageFilterMode::Linear,
            ..default()
        });
    }
    image
}
fn verge_mesh(center: f32, width: f32) -> Mesh {
    let mut vertices = Vec::new();
    let mut uv = Vec::new();
    let mut indices = Vec::new();
    for step in 0..=48 {
        let x = -16.5 + step as f32 * 33.0 / 48.0;
        let offset = (x * 0.63 + center).sin() * 0.18 + (x * 1.37).cos() * 0.07;
        let half = width * (0.40 + (x * 0.91 + center).sin() * 0.08 + grain(step, 19) * 0.10);
        for side in [-1.0, 1.0] {
            vertices.push([x, 0.012, center + offset + half * side]);
            // One texture spans roughly the same metres along and across the road.
            uv.push([(x + 16.5) / width, if side < 0.0 { 0.0 } else { 1.0 }]);
        }
        if step < 48 {
            let i = step * 2;
            indices.extend([i, i + 1, i + 2, i + 1, i + 3, i + 2]);
        }
    }
    let count = vertices.len();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; count])
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
    .with_inserted_indices(Indices::U32(indices))
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<AtmosphereClock>()
        .add_systems(Startup, setup)
        .add_systems(Update, animate);
}
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let patina = images.add(image(true));
    for (center, width, color, name) in [
        (
            -4.45,
            1.4,
            Color::srgb(0.16, 0.20, 0.12),
            "Moss at southern road verge",
        ),
        (
            4.45,
            1.4,
            Color::srgb(0.16, 0.20, 0.12),
            "Moss at northern road verge",
        ),
        (
            0.0,
            2.6,
            Color::srgb(0.42, 0.36, 0.25),
            "Worn dusty road center",
        ),
    ] {
        commands.spawn((
            Name::new(name),
            Mesh3d(meshes.add(verge_mesh(center, width))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                base_color_texture: Some(patina.clone()),
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: 1.0,
                reflectance: 0.10,
                depth_bias: 1.0,
                ..default()
            })),
            Transform::default(),
            ArenaEntity,
            NotShadowCaster,
        ));
    }
    let mote = meshes.add(Rectangle::new(1.0, 1.0));
    let veil = images.add(image(false));
    for (camp, center) in super::super::encounter_bosses::ADVENTURE_SITES
        .into_iter()
        .enumerate()
    {
        let color = [
            Color::srgb(1.0, 0.34, 0.06),
            Color::srgb(0.16, 0.62, 0.72),
            Color::srgb(0.46, 0.68, 0.12),
        ][camp];
        let mat = materials.add(StandardMaterial {
            base_color: color.with_alpha(0.48),
            base_color_texture: Some(veil.clone()),
            emissive: LinearRgba::from(color) * 1.2,
            alpha_mode: AlphaMode::Add,
            unlit: true,
            cull_mode: None,
            ..default()
        });
        for index in 0..8 {
            commands.spawn((
                Name::new(format!("Camp atmosphere {camp}/{index}")),
                CampAtmosphere {
                    camp,
                    index,
                    center,
                },
                Mesh3d(mote.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_translation(center),
                ArenaEntity,
                NotShadowCaster,
                NotShadowReceiver,
            ));
        }
        // Match the existing torch prop position, then add a weak identity fill
        // at ground level. Neither light casts another expensive shadow map.
        for (position, base, lamp_color) in [
            (
                center + Vec3::new(-2.5, 1.45, -2.0),
                35_000.0,
                Color::srgb(1.0, 0.58, 0.22),
            ),
            (
                center + Vec3::new(0.0, 1.4, 0.0),
                [45_000.0, 32_000.0, 30_000.0][camp],
                color,
            ),
        ] {
            commands.spawn((
                Name::new(format!("Camp {camp} local light")),
                CampLamp { camp, base },
                PointLight {
                    color: lamp_color,
                    intensity: base,
                    range: 5.0,
                    radius: 0.4,
                    shadow_maps_enabled: false,
                    ..default()
                },
                Transform::from_translation(position),
                ArenaEntity,
            ));
        }
    }
}

fn atmosphere_transform(camp: usize, index: usize, center: Vec3, clock: f32) -> Transform {
    let phase = index as f32 * 0.618;
    let angle = phase * std::f32::consts::TAU;
    let offset = Vec3::new(angle.cos(), 0.0, angle.sin()) * (1.4 + (index % 3) as f32 * 0.35);
    let (height, scale, drift) = match camp {
        0 => {
            let life = (clock * 0.32 + phase).fract();
            (
                0.2 + life * 1.8,
                Vec3::new(0.11, 0.25, 1.0) * (1.0 - life * 0.65),
                Vec3::X * (clock * 0.7 + phase).sin() * 0.12,
            )
        }
        1 => (
            0.12 + (clock * 0.4 + phase).sin() * 0.06,
            Vec3::new(1.2, 0.26, 1.0),
            Vec3::X * (clock * 0.22 + phase).sin() * 0.5,
        ),
        _ => (
            0.55 + (clock * 0.55 + phase).sin() * 0.32,
            Vec3::new(0.17, 0.17, 1.0),
            Vec3::Z * (clock * 0.32 + phase).cos() * 0.25,
        ),
    };
    Transform::from_translation(center + offset + drift + Vec3::Y * height).with_scale(scale)
}
fn animate(
    time: Res<Time<Real>>,
    session: Res<GameSession>,
    state: Option<Res<super::super::AdventureBossState>>,
    mut clock: ResMut<AtmosphereClock>,
    cameras: Query<&GlobalTransform, With<Camera3d>>,
    mut motes: Query<(&CampAtmosphere, &mut Transform)>,
    mut lamps: Query<(&CampLamp, &mut PointLight)>,
) {
    if session.paused || session.save_active || session.finished {
        return;
    }
    clock.0 += time.delta_secs().min(0.05);
    let camera = cameras.iter().next().map(|t| t.translation());
    for (mote, mut transform) in &mut motes {
        *transform = atmosphere_transform(mote.camp, mote.index, mote.center, clock.0);
        if let Some(camera) = camera {
            transform.look_at(camera, Vec3::Y);
        }
        if state
            .as_ref()
            .is_some_and(|state| state.resolved & (1 << mote.camp) != 0)
        {
            transform.scale *= 0.25;
        }
    }
    for (lamp, mut light) in &mut lamps {
        let flicker =
            (clock.0 * 7.3 + lamp.camp as f32).sin() * 0.05 + (clock.0 * 13.9).cos() * 0.025;
        let quiet = if state
            .as_ref()
            .is_some_and(|state| state.resolved & (1 << lamp.camp) != 0)
        {
            0.35
        } else {
            1.0
        };
        light.intensity = lamp.base * (1.0 + flicker) * quiet;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn road_surface_faces_the_overhead_camera() {
        let mesh = verge_mesh(0.0, 2.6);
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(vertices)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("road requires positions");
        };
        let indices: Vec<_> = mesh.indices().unwrap().iter().collect();
        for triangle in indices.chunks_exact(3) {
            let a = Vec3::from_array(vertices[triangle[0]]);
            let b = Vec3::from_array(vertices[triangle[1]]);
            let c = Vec3::from_array(vertices[triangle[2]]);
            assert!((b - a).cross(c - a).y > 0.0);
        }
    }
    #[test]
    fn road_grain_repeats_at_world_scale_without_a_stretched_single_tile() {
        let mesh = verge_mesh(4.45, 1.4);
        let Some(bevy::mesh::VertexAttributeValues::Float32x2(uv)) =
            mesh.attribute(Mesh::ATTRIBUTE_UV_0)
        else {
            panic!("road requires UV");
        };
        assert!((uv.last().unwrap()[0] - 33.0 / 1.4).abs() < 0.001);
        let texture = image(true);
        let ImageSampler::Descriptor(sampler) = texture.sampler else {
            panic!("repeat sampler required");
        };
        assert_eq!(sampler.address_mode_u, ImageAddressMode::Repeat);
        for y in 0..128 {
            assert!(
                (soft_grain(0.0, y as f32 / 127.0, 8) - soft_grain(1.0, y as f32 / 127.0, 8)).abs()
                    < 0.0001
            );
        }
    }
    #[test]
    fn camp_motion_stays_inside_its_own_site_and_profiles_differ() {
        for camp in 0..3 {
            for i in 0..8 {
                for tick in 0..100 {
                    let t = atmosphere_transform(camp, i, Vec3::ZERO, tick as f32 * 0.1);
                    assert!(t.translation.xz().length() < 3.2);
                    assert!((0.0..2.1).contains(&t.translation.y));
                    assert!(t.scale.x > 0.0 && t.scale.y > 0.0);
                }
            }
        }
        assert_ne!(
            atmosphere_transform(0, 0, Vec3::ZERO, 1.0).scale,
            atmosphere_transform(1, 0, Vec3::ZERO, 1.0).scale
        );
    }
    #[test]
    fn patina_has_soft_edges_and_transparent_gaps() {
        let image = image(true);
        let bytes = image.data.as_ref().unwrap();
        assert!(bytes.chunks_exact(4).take(128).all(|p| p[3] == 0));
        assert!(bytes.chunks_exact(4).any(|p| p[3] > 100));
        assert!(bytes.chunks_exact(4).all(|p| p[3] < 170));
    }
}
