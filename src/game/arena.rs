use bevy::{camera::ScalingMode, prelude::*};

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
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.26, 0.38, 0.48),
        brightness: 90.0,
        ..default()
    });
    let ground = materials.add(StandardMaterial {
        base_color: Color::srgb(0.055, 0.12, 0.13),
        perceptual_roughness: 0.95,
        ..default()
    });
    let path = materials.add(StandardMaterial {
        base_color: Color::srgb(0.13, 0.19, 0.18),
        perceptual_roughness: 0.9,
        ..default()
    });
    let rune = materials.add(StandardMaterial {
        base_color: Color::srgb(0.05, 0.38, 0.44),
        emissive: LinearRgba::new(0.0, 1.8, 2.4, 1.0),
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
    let core_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.9, 0.78),
        emissive: LinearRgba::new(0.0, 1.8, 1.2, 1.0),
        metallic: 0.3,
        ..default()
    });
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

    let enemy_mesh = meshes.add(Capsule3d::new(0.36, 0.7));
    let enemy_meshes = [
        enemy_mesh.clone(),
        meshes.add(Sphere::new(0.58).mesh().ico(1).unwrap()),
        meshes.add(Cuboid::new(0.9, 1.05, 0.72)),
        meshes.add(ConicalFrustum {
            radius_top: 0.22,
            radius_bottom: 0.64,
            height: 1.25,
        }),
    ];
    let elite_mesh = meshes.add(ConicalFrustum {
        radius_top: 0.38,
        radius_bottom: 0.72,
        height: 1.6,
    });
    let projectile_mesh = meshes.add(Sphere::new(0.16).mesh().ico(2).unwrap());
    let fx_mesh = meshes.add(Torus::new(0.86, 1.0));

    commands.insert_resource(GameAssets {
        hero_scenes: hero_scenes.clone(),
        enemy_scenes,
        boss_scenes,
        summon_scenes,
        enemy_mesh,
        enemy_meshes,
        elite_mesh,
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
        Mesh3d(meshes.add(Plane3d::default().mesh().size(48.0, 30.0))),
        MeshMaterial3d(ground.clone()),
        ArenaEntity,
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(35.0, 0.12, 10.5))),
        MeshMaterial3d(path.clone()),
        Transform::from_xyz(0.5, 0.055, 0.0),
        ArenaEntity,
    ));

    let tile_mesh = meshes.add(Cuboid::new(1.45, 0.06, 4.55));
    for i in 0..23 {
        let x = -15.5 + i as f32 * 1.43;
        commands.spawn((
            Mesh3d(tile_mesh.clone()),
            MeshMaterial3d(if i % 2 == 0 {
                path.clone()
            } else {
                ground.clone()
            }),
            Transform::from_xyz(x, 0.12, if i % 2 == 0 { -2.45 } else { 2.45 }),
            ArenaEntity,
        ));
    }

    for z in [-5.65, 5.65] {
        for i in 0..12 {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(2.35, 0.65, 0.42))),
                MeshMaterial3d(path.clone()),
                Transform::from_xyz(-12.5 + i as f32 * 2.55, 0.28, z),
                ArenaEntity,
            ));
        }
    }

    for z in [-3.2, 0.0, 3.2] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(6.0, 0.035, 0.1))),
            MeshMaterial3d(rune.clone()),
            Transform::from_xyz(-8.8, 0.14, z),
            ArenaEntity,
        ));
    }

    commands
        .spawn((
            Core,
            Transform::from_xyz(-14.8, 1.0, 0.0),
            Visibility::default(),
            ArenaEntity,
        ))
        .with_children(|parent| {
            parent.spawn((
                Mesh3d(meshes.add(Cylinder::new(1.4, 0.35))),
                MeshMaterial3d(path.clone()),
                Transform::from_xyz(0.0, -0.8, 0.0),
            ));
            parent.spawn((
                Mesh3d(meshes.add(Tetrahedron::new(
                    Vec3::new(0.0, 1.15, 0.0),
                    Vec3::new(-0.9, -0.75, 0.72),
                    Vec3::new(0.9, -0.75, 0.72),
                    Vec3::new(0.0, -0.75, -0.95),
                ))),
                MeshMaterial3d(core_material.clone()),
                Transform::from_rotation(Quat::from_rotation_z(0.45)),
            ));
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
                HeroVisual,
                WorldAssetRoot(hero_scenes[0].clone()),
                Transform::from_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
            ));
        });

    let portal_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.08, 0.82, 0.43, 0.72),
        emissive: LinearRgba::new(0.05, 4.5, 1.2, 1.0),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    commands
        .spawn((
            Transform::from_xyz(14.6, 2.35, 0.0),
            Visibility::default(),
            ArenaEntity,
        ))
        .with_children(|portal| {
            portal.spawn((
                Mesh3d(meshes.add(Torus::new(1.65, 2.18))),
                MeshMaterial3d(portal_material.clone()),
                Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
            ));
            portal.spawn((
                Mesh3d(meshes.add(Cylinder::new(1.62, 0.08))),
                MeshMaterial3d(portal_material.clone()),
                Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2))
                    .with_scale(Vec3::new(1.0, 1.0, 0.72)),
            ));
        });

    for z in [-2.65, 2.65] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(1.1, 4.8, 1.1))),
            MeshMaterial3d(path.clone()),
            Transform::from_xyz(14.6, 2.25, z),
            ArenaEntity,
        ));
    }

    commands.spawn((
        DirectionalLight {
            illuminance: 4_500.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, -0.35, -0.7, -0.9)),
    ));

    let camera_projection = Projection::from(OrthographicProjection {
        scaling_mode: ScalingMode::FixedVertical {
            viewport_height: 22.0,
        },
        ..OrthographicProjection::default_3d()
    });
    let camera_transform =
        Transform::from_xyz(0.0, 24.0, 21.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y);
    if let Some(target) = capture_target {
        commands.spawn((
            Camera3d::default(),
            bevy::camera::RenderTarget::Image(target.0.clone().into()),
            camera_projection,
            camera_transform,
            Msaa::Off,
            IsDefaultUiCamera,
        ));
    } else {
        commands.spawn((
            Camera3d::default(),
            camera_projection,
            camera_transform,
            Msaa::Off,
            IsDefaultUiCamera,
        ));
    }
}
