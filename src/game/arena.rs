use bevy::{
    camera::ScalingMode,
    core_pipeline::tonemapping::Tonemapping,
    post_process::bloom::{Bloom, BloomPrefilter},
    prelude::*,
};

use super::environment::spawn_gate_art;
use super::gate_feedback::GateArt;
use super::{ArenaEntity, CaptureTarget, Core, GameAssets, Hero, HeroVisual};

pub struct ArenaPlugin;

impl Plugin for ArenaPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_arena);
    }
}

fn setup_arena(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    capture_target: Option<Res<CaptureTarget>>,
    selected: Res<super::SelectedHero>,
) {
    let hero_scenes = [
        asset_server.load("models/hero_storm_savant.glb#Scene0"),
        asset_server.load("models/hero_ember_ranger.glb#Scene0"),
        asset_server.load("models/hero_stone_warden.glb#Scene0"),
        asset_server.load("models/hero_spirit_keeper.glb#Scene0"),
        asset_server.load("models/hero_tide_walker.glb#Scene0"),
        asset_server.load("models/hero_star_wagerer.glb#Scene0"),
        asset_server.load("models/hero_frost_oracle.glb#Scene0"),
        asset_server.load("models/hero_iron_berserker.glb#Scene0"),
        asset_server.load("models/hero_rift_dancer.glb#Scene0"),
        asset_server.load("models/hero_grove_artificer.glb#Scene0"),
        asset_server.load("models/hero_solar_vanguard.glb#Scene0"),
        asset_server.load("models/hero_void_reaper.glb#Scene0"),
    ];
    let enemy_scenes = [
        asset_server.load("models/enemy_rift_sapper.glb#Scene0"),
        asset_server.load("models/enemy_gnawling.glb#Scene0"),
        asset_server.load("models/enemy_shell_guard.glb#Scene0"),
        asset_server.load("models/enemy_void_caster.glb#Scene0"),
        asset_server.load("models/enemy_core_rusher.glb#Scene0"),
        asset_server.load("models/enemy_chanting_acolyte.glb#Scene0"),
        asset_server.load("models/enemy_iron_colossus.glb#Scene0"),
        asset_server.load("models/enemy_burrow_stalker.glb#Scene0"),
        asset_server.load("models/enemy_field_mender.glb#Scene0"),
        asset_server.load("models/enemy_split_spore.glb#Scene0"),
        asset_server.load("models/enemy_null_eye.glb#Scene0"),
        asset_server.load("models/enemy_siege_lobber.glb#Scene0"),
    ];
    let boss_scenes = [
        asset_server.load("models/boss_cinder_colossus.glb#Scene0"),
        asset_server.load("models/boss_abyssal_leviathan.glb#Scene0"),
        asset_server.load("models/boss_dread_veil.glb#Scene0"),
        asset_server.load("models/boss_sun_crown_magus.glb#Scene0"),
        asset_server.load("models/boss_green_regent.glb#Scene0"),
    ];
    let summon_scenes = [
        asset_server.load("models/summon_bone_sentry.glb#Scene0"),
        asset_server.load("models/summon_spirit_hound.glb#Scene0"),
        asset_server.load("models/summon_tide_elemental.glb#Scene0"),
        asset_server.load("models/summon_bolt_turret.glb#Scene0"),
        asset_server.load("models/summon_frost_wyrm.glb#Scene0"),
        asset_server.load("models/summon_ancestor_colossus.glb#Scene0"),
    ];
    let adventure_scenes = [
        asset_server.load("models/adventure_embermaw.glb#Scene0"),
        asset_server.load("models/adventure_tidehorn.glb#Scene0"),
        asset_server.load("models/adventure_plaguejaw.glb#Scene0"),
    ];
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.48, 0.58, 0.68),
        brightness: 220.0,
        ..default()
    });
    let ground = materials.add(StandardMaterial {
        base_color: Color::srgb(0.055, 0.12, 0.13),
        perceptual_roughness: 1.0,
        reflectance: 0.18,
        ..default()
    });
    let hero_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.72, 0.9),
        emissive: LinearRgba::new(0.0, 0.45, 0.8, 1.0),
        metallic: 0.15,
        ..default()
    });
    let hero_materials = [
        hero_material.clone(),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.92, 0.32, 0.12),
            emissive: LinearRgba::new(1.4, 0.12, 0.02, 1.0),
            metallic: 0.12,
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.58, 0.62, 0.68),
            emissive: LinearRgba::new(0.18, 0.22, 0.28, 1.0),
            metallic: 0.5,
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.46, 0.2, 0.76),
            emissive: LinearRgba::new(0.55, 0.08, 1.2, 1.0),
            metallic: 0.1,
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.12, 0.66, 0.58),
            emissive: LinearRgba::new(0.03, 0.7, 0.58, 1.0),
            metallic: 0.18,
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.88, 0.66, 0.16),
            emissive: LinearRgba::new(1.4, 0.62, 0.03, 1.0),
            metallic: 0.28,
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.48, 0.82, 0.94),
            emissive: LinearRgba::new(0.18, 1.2, 2.4, 1.0),
            metallic: 0.2,
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.56, 0.12, 0.08),
            emissive: LinearRgba::new(1.35, 0.08, 0.02, 1.0),
            metallic: 0.62,
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.72, 0.18, 0.86),
            emissive: LinearRgba::new(1.35, 0.08, 2.1, 1.0),
            metallic: 0.22,
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.22, 0.66, 0.25),
            emissive: LinearRgba::new(0.16, 1.15, 0.18, 1.0),
            metallic: 0.18,
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.96, 0.63, 0.12),
            emissive: LinearRgba::new(2.2, 0.68, 0.04, 1.0),
            metallic: 0.42,
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.26, 0.08, 0.42),
            emissive: LinearRgba::new(0.55, 0.04, 1.45, 1.0),
            metallic: 0.3,
            ..default()
        }),
    ];
    let enemy_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.75, 0.12, 0.25),
        emissive: LinearRgba::new(0.3, 0.0, 0.03, 1.0),
        ..default()
    });
    let enemy_materials = [
        enemy_material.clone(),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.86, 0.38, 0.12),
            emissive: LinearRgba::new(0.48, 0.08, 0.01, 1.0),
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.44, 0.16, 0.64),
            emissive: LinearRgba::new(0.5, 0.02, 0.9, 1.0),
            metallic: 0.18,
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.12, 0.62, 0.34),
            emissive: LinearRgba::new(0.02, 0.55, 0.12, 1.0),
            ..default()
        }),
    ];
    let elite_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.68, 0.18, 0.88),
        emissive: LinearRgba::new(1.0, 0.05, 1.8, 1.0),
        metallic: 0.25,
        ..default()
    });
    let boss_materials = [
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.95, 0.28, 0.08),
            emissive: LinearRgba::new(2.6, 0.18, 0.02, 1.0),
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.15, 0.52, 0.82),
            emissive: LinearRgba::new(0.05, 0.75, 2.0, 1.0),
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.48, 0.16, 0.62),
            emissive: LinearRgba::new(0.9, 0.05, 1.5, 1.0),
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.95, 0.62, 0.12),
            emissive: LinearRgba::new(2.3, 0.72, 0.02, 1.0),
            ..default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.18, 0.86, 0.42),
            emissive: LinearRgba::new(0.02, 2.5, 0.55, 1.0),
            ..default()
        }),
    ];
    let projectile_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.38, 0.92, 1.0),
        emissive: LinearRgba::new(0.15, 3.0, 5.0, 1.0),
        unlit: true,
        ..default()
    });
    let storm_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.08, 0.75, 1.0, 0.32),
        emissive: LinearRgba::new(0.0, 2.0, 5.5, 1.0),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    let contact_shadow = materials.add(StandardMaterial {
        base_color: Color::srgba(0.0, 0.0, 0.0, 0.34),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    let projectile_mesh = meshes.add(Sphere::new(0.16).mesh().ico(2).unwrap());
    let fx_mesh = meshes.add(super::energy_art::halo_mesh());
    let hero_shadow_mesh = meshes.add(Circle::new(0.92));
    let core_shadow_mesh = meshes.add(Circle::new(1.65));

    commands.insert_resource(GameAssets {
        hero_scenes: hero_scenes.clone(),
        enemy_scenes,
        boss_scenes,
        adventure_scenes,
        summon_scenes,
        projectile_mesh,
        fx_mesh,
        enemy_material,
        enemy_materials,
        elite_material,
        boss_materials,
        hero_materials: hero_materials.clone(),
        projectile_material,
        storm_material,
    });

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(80.0, 80.0))),
        MeshMaterial3d(ground.clone()),
        Transform::from_xyz(0.0, -0.16, 0.0),
        ArenaEntity,
    ));

    let paving = asset_server.load("models/environment_floor.glb#Scene0");
    for column in 0..18 {
        for row in 0..5 {
            commands.spawn((
                WorldAssetRoot(paving.clone()),
                Transform::from_xyz(-16.5 + column as f32 * 2.0, -0.15, -4.0 + row as f32 * 2.0)
                    .with_scale(Vec3::splat(0.15)),
                ArenaEntity,
            ));
        }
    }
    let border = asset_server.load("models/environment_border.glb#Scene0");
    for z in [-5.65_f32, 5.65] {
        for i in 0..12 {
            let x: f32 = -12.5 + i as f32 * 2.55;
            if super::encounter_bosses::ADVENTURE_SITES
                .iter()
                .any(|site| site.z.signum() == z.signum() && (site.x - x).abs() < 2.0)
            {
                continue;
            }
            commands.spawn((
                WorldAssetRoot(border.clone()),
                Transform::from_xyz(-12.5 + i as f32 * 2.55, 0.0, z).with_scale(Vec3::splat(0.64)),
                ArenaEntity,
            ));
        }
    }

    commands
        .spawn((
            Core,
            Transform::from_translation(super::resources::GATE_POSITION),
            Visibility::default(),
            ArenaEntity,
        ))
        .with_children(|parent| {
            parent.spawn((
                Mesh3d(core_shadow_mesh),
                MeshMaterial3d(contact_shadow.clone()),
                Transform::from_xyz(0.0, -0.95, 0.0)
                    .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
                    .with_scale(Vec3::new(0.65, 1.45, 1.0)),
            ));
            parent.spawn((
                WorldAssetRoot(paving.clone()),
                Transform::from_xyz(0.0, -0.98, 0.0).with_scale(Vec3::splat(0.18)),
            ));
            parent
                .spawn((
                    Name::new("Gate art"),
                    GateArt,
                    Transform::default(),
                    Visibility::default(),
                ))
                .with_children(|art| {
                    spawn_gate_art(art, &asset_server, &mut meshes, &mut materials);
                });
        });

    commands
        .spawn((
            Hero,
            Transform::from_xyz(-8.2, 0.0, 0.0),
            Visibility::default(),
            ArenaEntity,
        ))
        .with_children(|parent| {
            parent.spawn((
                Mesh3d(hero_shadow_mesh),
                MeshMaterial3d(contact_shadow),
                Transform::from_xyz(0.0, 0.025, 0.0)
                    .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
                    .with_scale(Vec3::new(1.0, 0.68, 1.0)),
            ));
            parent.spawn((
                HeroVisual,
                WorldAssetRoot(hero_scenes[selected.kind.index()].clone()),
                Transform::default(),
            ));
        });

    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.91, 0.80),
            illuminance: 4_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        bevy::light::CascadeShadowConfigBuilder {
            num_cascades: 2,
            maximum_distance: 65.0,
            first_cascade_far_bound: 32.0,
            ..default()
        }
        .build(),
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, -0.35, -0.7, -0.9)),
    ));

    // A dim, unshadowed sky fill keeps shaded model faces readable without
    // adding another cascade map or flattening the warmer sunlit surfaces.
    commands.spawn((
        Name::new("Cool sky fill"),
        DirectionalLight {
            color: Color::srgb(0.66, 0.79, 1.0),
            illuminance: 650.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(8.0, 15.0, 18.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let camera_projection = Projection::from(OrthographicProjection {
        scaling_mode: ScalingMode::FixedVertical {
            viewport_height: 22.0,
        },
        ..OrthographicProjection::default_3d()
    });
    let camera_transform =
        Transform::from_xyz(8.0, 24.0, 21.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y);
    if let Some(target) = capture_target {
        commands.spawn((
            Camera3d::default(),
            combat_bloom(),
            Tonemapping::TonyMcMapface,
            bevy::camera::RenderTarget::Image(target.0.clone().into()),
            camera_projection,
            camera_transform,
            Msaa::Off,
        ));
    } else {
        commands.spawn((
            Camera3d::default(),
            combat_bloom(),
            Tonemapping::TonyMcMapface,
            camera_projection,
            camera_transform,
            Msaa::Off,
        ));
    }
}

// Extract emissive spell light rather than blurring the ordinary battlefield.
fn combat_bloom() -> Bloom {
    Bloom {
        intensity: 0.16,
        low_frequency_boost: 0.35,
        prefilter: BloomPrefilter {
            threshold: 1.4,
            threshold_softness: 0.4,
        },
        ..Bloom::OLD_SCHOOL
    }
}
