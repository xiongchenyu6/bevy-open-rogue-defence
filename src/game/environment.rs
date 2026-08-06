use bevy::prelude::*;

use super::{ArenaEntity, GameSession};

pub struct EnvironmentArtPlugin;

impl Plugin for EnvironmentArtPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ArtClock>()
            .add_systems(Startup, setup_environment_art)
            .add_systems(Update, animate_environment_art);
    }
}

#[derive(Resource, Default)]
struct ArtClock {
    elapsed: f32,
}

impl ArtClock {
    fn advance(&mut self, delta: f32, session: &GameSession) -> f32 {
        if !session.paused && !session.finished {
            self.elapsed += delta.max(0.0);
        }
        self.elapsed
    }
}

#[derive(Component)]
pub(super) struct ArtSpin {
    pub(super) axis: Vec3,
    pub(super) speed: f32,
}

#[derive(Component)]
pub(super) struct ArtBob {
    pub(super) base_y: f32,
    pub(super) amplitude: f32,
    pub(super) speed: f32,
    pub(super) phase: f32,
}

#[derive(Component)]
pub(super) struct ArtPulse {
    pub(super) base_scale: Vec3,
    pub(super) amplitude: f32,
    pub(super) speed: f32,
    pub(super) phase: f32,
}

struct BrazierMeshes {
    bowl: Handle<Mesh>,
    flame: Handle<Mesh>,
}

fn setup_environment_art(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let fortress_stone = materials.add(StandardMaterial {
        base_color: Color::srgb(0.085, 0.15, 0.16),
        metallic: 0.08,
        perceptual_roughness: 0.86,
        ..default()
    });
    let fortress_trim = materials.add(StandardMaterial {
        base_color: Color::srgb(0.32, 0.24, 0.10),
        metallic: 0.72,
        perceptual_roughness: 0.34,
        ..default()
    });
    let banner = materials.add(StandardMaterial {
        base_color: Color::srgb(0.025, 0.42, 0.46),
        metallic: 0.12,
        perceptual_roughness: 0.62,
        ..default()
    });
    let core_glow = materials.add(StandardMaterial {
        base_color: Color::srgb(0.08, 0.76, 0.68),
        emissive: LinearRgba::new(0.04, 4.2, 3.2, 1.0),
        metallic: 0.28,
        perceptual_roughness: 0.22,
        ..default()
    });
    let corruption = materials.add(StandardMaterial {
        base_color: Color::srgba(0.20, 0.025, 0.31, 0.82),
        emissive: LinearRgba::new(0.42, 0.015, 1.25, 1.0),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.58,
        ..default()
    });
    let rift_glow = materials.add(StandardMaterial {
        base_color: Color::srgba(0.62, 0.10, 0.92, 0.72),
        emissive: LinearRgba::new(3.4, 0.10, 6.4, 1.0),
        alpha_mode: AlphaMode::Add,
        unlit: true,
        ..default()
    });
    let rune_glow = materials.add(StandardMaterial {
        base_color: Color::srgba(0.06, 0.62, 0.70, 0.78),
        emissive: LinearRgba::new(0.04, 2.4, 3.4, 1.0),
        alpha_mode: AlphaMode::Add,
        unlit: true,
        ..default()
    });
    let brazier_metal = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.13, 0.08),
        metallic: 0.82,
        perceptual_roughness: 0.32,
        ..default()
    });
    let flame = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.30, 0.035, 0.92),
        emissive: LinearRgba::new(7.2, 0.65, 0.025, 1.0),
        alpha_mode: AlphaMode::Add,
        unlit: true,
        ..default()
    });
    let vegetation = materials.add(StandardMaterial {
        base_color: Color::srgb(0.055, 0.20, 0.13),
        perceptual_roughness: 0.96,
        ..default()
    });
    let rock = materials.add(StandardMaterial {
        base_color: Color::srgb(0.065, 0.095, 0.10),
        perceptual_roughness: 0.98,
        ..default()
    });
    let brazier_meshes = BrazierMeshes {
        bowl: meshes.add(ConicalFrustum {
            radius_top: 0.44,
            radius_bottom: 0.24,
            height: 0.36,
        }),
        flame: meshes.add(ConicalFrustum {
            radius_top: 0.02,
            radius_bottom: 0.24,
            height: 0.72,
        }),
    };

    spawn_fortress(
        &mut commands,
        &mut meshes,
        &fortress_stone,
        &fortress_trim,
        &banner,
        &core_glow,
        &brazier_meshes,
        &brazier_metal,
        &flame,
    );
    spawn_rift_dressing(
        &mut commands,
        &mut meshes,
        &fortress_stone,
        &corruption,
        &rift_glow,
    );
    spawn_lane_runes(&mut commands, &mut meshes, &rune_glow, &fortress_trim);
    spawn_border_dressing(
        &mut commands,
        &mut meshes,
        &fortress_stone,
        &brazier_meshes,
        &brazier_metal,
        &flame,
        &vegetation,
        &rock,
    );
}

#[allow(clippy::too_many_arguments)]
fn spawn_fortress(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    stone: &Handle<StandardMaterial>,
    trim: &Handle<StandardMaterial>,
    banner: &Handle<StandardMaterial>,
    core_glow: &Handle<StandardMaterial>,
    brazier_meshes: &BrazierMeshes,
    brazier: &Handle<StandardMaterial>,
    flame: &Handle<StandardMaterial>,
) {
    let wall = meshes.add(Cuboid::new(1.2, 3.6, 8.8));
    let tower = meshes.add(Cylinder::new(1.05, 3.8));
    let tower_cap = meshes.add(ConicalFrustum {
        radius_top: 0.08,
        radius_bottom: 1.22,
        height: 1.25,
    });
    let crenel = meshes.add(Cuboid::new(0.75, 0.72, 0.68));
    let trim_bar = meshes.add(Cuboid::new(0.24, 0.20, 8.9));
    let banner_mesh = meshes.add(Cuboid::new(0.10, 1.55, 0.82));
    let crystal = meshes.add(ConicalFrustum {
        radius_top: 0.02,
        radius_bottom: 0.32,
        height: 1.35,
    });

    spawn_static(
        commands,
        wall,
        stone.clone(),
        Transform::from_xyz(-17.0, 1.72, 0.0),
    );
    for z in [-4.65, 4.65] {
        spawn_static(
            commands,
            tower.clone(),
            stone.clone(),
            Transform::from_xyz(-16.65, 1.85, z),
        );
        spawn_static(
            commands,
            tower_cap.clone(),
            trim.clone(),
            Transform::from_xyz(-16.65, 4.37, z),
        );
    }
    spawn_static(
        commands,
        trim_bar,
        trim.clone(),
        Transform::from_xyz(-16.34, 2.70, 0.0),
    );
    for index in 0..8 {
        let z = -3.60 + index as f32 * 1.03;
        spawn_static(
            commands,
            crenel.clone(),
            if index % 2 == 0 {
                stone.clone()
            } else {
                trim.clone()
            },
            Transform::from_xyz(-16.30, 3.88, z),
        );
    }
    for (z, tilt) in [(-2.80, -0.08), (2.80, 0.08)] {
        spawn_static(
            commands,
            banner_mesh.clone(),
            banner.clone(),
            Transform::from_xyz(-16.28, 2.05, z).with_rotation(Quat::from_rotation_x(tilt)),
        );
    }
    for (index, z) in [-1.55, 0.0, 1.55].into_iter().enumerate() {
        commands.spawn((
            Mesh3d(crystal.clone()),
            MeshMaterial3d(core_glow.clone()),
            Transform::from_xyz(-16.15, 2.05 + (index % 2) as f32 * 0.35, z)
                .with_rotation(Quat::from_rotation_z(0.10 * (index as f32 - 1.0))),
            ArtBob {
                base_y: 2.05 + (index % 2) as f32 * 0.35,
                amplitude: 0.16,
                speed: 1.55,
                phase: index as f32 * 1.7,
            },
            ArtSpin {
                axis: Vec3::Y,
                speed: 0.42 + index as f32 * 0.08,
            },
            ArenaEntity,
        ));
    }
    for z in [-5.55, 5.55] {
        spawn_brazier(
            commands,
            brazier_meshes,
            Vec3::new(-14.6, 0.0, z),
            brazier,
            flame,
        );
    }
}

fn spawn_rift_dressing(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    stone: &Handle<StandardMaterial>,
    corruption: &Handle<StandardMaterial>,
    glow: &Handle<StandardMaterial>,
) {
    let stain = meshes.add(Cylinder::new(1.0, 0.035));
    let monolith = meshes.add(ConicalFrustum {
        radius_top: 0.22,
        radius_bottom: 0.62,
        height: 3.0,
    });
    let shard = meshes.add(ConicalFrustum {
        radius_top: 0.025,
        radius_bottom: 0.26,
        height: 1.15,
    });
    let ring = meshes.add(Torus::new(1.95, 2.10));

    for (index, (x, z, sx, sz)) in [
        (11.7, -2.2, 2.8, 1.4),
        (12.8, 2.1, 2.1, 1.2),
        (14.6, 0.0, 3.8, 2.0),
        (16.4, -1.6, 2.2, 1.1),
        (17.2, 2.2, 2.6, 1.3),
    ]
    .into_iter()
    .enumerate()
    {
        commands.spawn((
            Mesh3d(stain.clone()),
            MeshMaterial3d(corruption.clone()),
            Transform::from_xyz(x, 0.095 + index as f32 * 0.002, z)
                .with_scale(Vec3::new(sx, 1.0, sz)),
            ArtPulse {
                base_scale: Vec3::new(sx, 1.0, sz),
                amplitude: 0.035,
                speed: 1.15,
                phase: index as f32 * 0.8,
            },
            ArenaEntity,
        ));
    }
    for (z, tilt) in [(-4.35, -0.18), (4.35, 0.18)] {
        spawn_static(
            commands,
            monolith.clone(),
            stone.clone(),
            Transform::from_xyz(14.8, 1.5, z).with_rotation(Quat::from_rotation_z(tilt)),
        );
    }
    commands.spawn((
        Mesh3d(ring),
        MeshMaterial3d(glow.clone()),
        Transform::from_xyz(14.62, 2.35, 0.0)
            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
        ArtSpin {
            axis: Vec3::X,
            speed: -0.52,
        },
        ArtPulse {
            base_scale: Vec3::ONE,
            amplitude: 0.055,
            speed: 2.1,
            phase: 0.0,
        },
        ArenaEntity,
    ));
    for index in 0..7 {
        let angle = index as f32 / 7.0 * std::f32::consts::TAU;
        let radius = if index % 2 == 0 { 2.65 } else { 2.25 };
        let y = 2.35 + angle.cos() * radius;
        let z = angle.sin() * radius;
        commands.spawn((
            Mesh3d(shard.clone()),
            MeshMaterial3d(glow.clone()),
            Transform::from_xyz(14.6, y, z)
                .with_rotation(Quat::from_euler(
                    EulerRot::XYZ,
                    angle * 0.35,
                    angle,
                    -angle * 0.28,
                ))
                .with_scale(Vec3::splat(if index % 3 == 0 { 1.25 } else { 0.82 })),
            ArtBob {
                base_y: y,
                amplitude: 0.13,
                speed: 1.2 + index as f32 * 0.08,
                phase: angle,
            },
            ArtSpin {
                axis: Vec3::Y,
                speed: 0.45 + index as f32 * 0.05,
            },
            ArenaEntity,
        ));
    }
}

fn spawn_lane_runes(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    glow: &Handle<StandardMaterial>,
    trim: &Handle<StandardMaterial>,
) {
    let ring = meshes.add(Torus::new(0.58, 0.68));
    let hub = meshes.add(Cylinder::new(0.22, 0.055));
    let tick = meshes.add(Cuboid::new(0.62, 0.035, 0.08));
    for (index, x) in [-11.2, -6.1, -1.0, 4.1, 9.2].into_iter().enumerate() {
        commands.spawn((
            Mesh3d(ring.clone()),
            MeshMaterial3d(glow.clone()),
            Transform::from_xyz(x, 0.19, 0.0),
            ArtSpin {
                axis: Vec3::Y,
                speed: if index % 2 == 0 { 0.42 } else { -0.42 },
            },
            ArtPulse {
                base_scale: Vec3::ONE,
                amplitude: 0.08,
                speed: 1.6,
                phase: index as f32 * 0.7,
            },
            ArenaEntity,
        ));
        spawn_static(
            commands,
            hub.clone(),
            trim.clone(),
            Transform::from_xyz(x, 0.165, 0.0),
        );
        for rotation in [0.0, std::f32::consts::FRAC_PI_2] {
            spawn_static(
                commands,
                tick.clone(),
                glow.clone(),
                Transform::from_xyz(x, 0.18, 0.0).with_rotation(Quat::from_rotation_y(rotation)),
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_border_dressing(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    stone: &Handle<StandardMaterial>,
    brazier_meshes: &BrazierMeshes,
    brazier: &Handle<StandardMaterial>,
    flame: &Handle<StandardMaterial>,
    vegetation: &Handle<StandardMaterial>,
    rock: &Handle<StandardMaterial>,
) {
    let pillar = meshes.add(Cylinder::new(0.42, 1.55));
    let pillar_cap = meshes.add(Cylinder::new(0.57, 0.18));
    let rock_mesh = meshes.add(Sphere::new(1.0).mesh().ico(1).unwrap());
    let shrub = meshes.add(ConicalFrustum {
        radius_top: 0.05,
        radius_bottom: 0.58,
        height: 1.15,
    });
    for z in [-6.35, 6.35] {
        for (index, x) in [-11.8, -5.6, 0.6, 6.8, 12.0].into_iter().enumerate() {
            spawn_static(
                commands,
                pillar.clone(),
                stone.clone(),
                Transform::from_xyz(x, 0.78, z),
            );
            spawn_static(
                commands,
                pillar_cap.clone(),
                brazier.clone(),
                Transform::from_xyz(x, 1.58, z),
            );
            if index % 2 == 0 {
                spawn_brazier(
                    commands,
                    brazier_meshes,
                    Vec3::new(x, 1.58, z),
                    brazier,
                    flame,
                );
            }
        }
    }
    for (index, (x, z, scale)) in [
        (-13.8, -8.3, 1.4),
        (-9.4, 8.1, 1.1),
        (-3.8, -7.7, 0.9),
        (2.4, 8.3, 1.2),
        (7.7, -8.0, 1.5),
        (12.3, 7.9, 1.0),
        (16.8, -7.8, 1.3),
    ]
    .into_iter()
    .enumerate()
    {
        spawn_static(
            commands,
            rock_mesh.clone(),
            rock.clone(),
            Transform::from_xyz(x, scale * 0.42, z)
                .with_scale(Vec3::new(scale, scale * 0.72, scale * 0.86))
                .with_rotation(Quat::from_euler(
                    EulerRot::YXZ,
                    index as f32 * 0.58,
                    0.0,
                    index as f32 * 0.13,
                )),
        );
        if index % 2 == 1 {
            spawn_static(
                commands,
                shrub.clone(),
                vegetation.clone(),
                Transform::from_xyz(x + 1.15, 0.58, z * 0.98)
                    .with_rotation(Quat::from_rotation_z((index as f32 - 2.0) * 0.08)),
            );
        }
    }
}

fn spawn_brazier(
    commands: &mut Commands,
    meshes: &BrazierMeshes,
    position: Vec3,
    metal: &Handle<StandardMaterial>,
    flame: &Handle<StandardMaterial>,
) {
    spawn_static(
        commands,
        meshes.bowl.clone(),
        metal.clone(),
        Transform::from_translation(position + Vec3::Y * 0.24),
    );
    let base_y = position.y + 0.78;
    commands.spawn((
        Mesh3d(meshes.flame.clone()),
        MeshMaterial3d(flame.clone()),
        Transform::from_xyz(position.x, base_y, position.z),
        ArtBob {
            base_y,
            amplitude: 0.08,
            speed: 4.2,
            phase: position.x * 0.37 + position.z,
        },
        ArtPulse {
            base_scale: Vec3::ONE,
            amplitude: 0.14,
            speed: 5.1,
            phase: position.z * 0.5,
        },
        ArenaEntity,
    ));
}

fn spawn_static(
    commands: &mut Commands,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    transform: Transform,
) {
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        transform,
        ArenaEntity,
    ));
}

fn animate_environment_art(
    time: Res<Time>,
    session: Res<GameSession>,
    mut clock: ResMut<ArtClock>,
    mut transforms: ParamSet<(
        Query<(&ArtSpin, &mut Transform)>,
        Query<(&ArtBob, &mut Transform)>,
        Query<(&ArtPulse, &mut Transform)>,
    )>,
) {
    let running = !session.paused && !session.finished;
    let delta = if running { time.delta_secs() } else { 0.0 };
    let elapsed = clock.advance(delta, &session);
    if running {
        for (spin, mut transform) in &mut transforms.p0() {
            transform.rotate(Quat::from_axis_angle(spin.axis, spin.speed * delta));
        }
    }
    for (bob, mut transform) in &mut transforms.p1() {
        transform.translation.y = bob_height(bob, elapsed);
    }
    for (pulse, mut transform) in &mut transforms.p2() {
        transform.scale = pulse_scale(pulse, elapsed);
    }
}

fn bob_height(bob: &ArtBob, elapsed: f32) -> f32 {
    bob.base_y + (elapsed * bob.speed + bob.phase).sin() * bob.amplitude
}

fn pulse_scale(pulse: &ArtPulse, elapsed: f32) -> Vec3 {
    let multiplier = 1.0 + (elapsed * pulse.speed + pulse.phase).sin() * pulse.amplitude;
    pulse.base_scale * multiplier
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::*;

    #[test]
    fn animated_art_is_bounded_and_never_owns_gameplay_state() {
        let pulse = ArtPulse {
            base_scale: Vec3::new(2.0, 1.0, 3.0),
            amplitude: 0.08,
            speed: 1.6,
            phase: 0.7,
        };
        for step in 0..1_000 {
            let elapsed = step as f32 / 60.0;
            let scale = pulse_scale(&pulse, elapsed);
            assert!(scale.x >= pulse.base_scale.x * 0.92);
            assert!(scale.x <= pulse.base_scale.x * 1.08);
            assert!(scale.z >= pulse.base_scale.z * 0.92);
            assert!(scale.z <= pulse.base_scale.z * 1.08);
        }
    }

    #[test]
    fn art_clock_freezes_phase_and_resumes_with_only_the_scaled_virtual_delta() {
        let bob = ArtBob {
            base_y: 2.0,
            amplitude: 0.16,
            speed: 1.55,
            phase: 0.7,
        };
        let pulse = ArtPulse {
            base_scale: Vec3::new(2.0, 1.0, 3.0),
            amplitude: 0.08,
            speed: 1.6,
            phase: 0.7,
        };
        let mut clock = ArtClock::default();
        let mut session = GameSession::default();

        let before_pause = clock.advance(0.12, &session);
        let bob_before_pause = bob_height(&bob, before_pause);
        let pulse_before_pause = pulse_scale(&pulse, before_pause);

        session.paused = true;
        let during_pause = clock.advance(30.0, &session);
        assert_eq!(during_pause, before_pause);
        assert_eq!(bob_height(&bob, during_pause), bob_before_pause);
        assert_eq!(pulse_scale(&pulse, during_pause), pulse_before_pause);

        session.paused = false;
        let after_resume = clock.advance(0.06, &session);
        assert!((after_resume - 0.18).abs() < f32::EPSILON);
        assert_eq!(bob_height(&bob, after_resume), bob_height(&bob, 0.18));
        assert_eq!(pulse_scale(&pulse, after_resume), pulse_scale(&pulse, 0.18));

        session.finished = true;
        assert_eq!(clock.advance(30.0, &session), after_resume);
    }

    #[test]
    fn art_module_has_no_rng_or_gameplay_mutation() {
        let source = include_str!("environment.rs");
        for forbidden in [
            ["Run", "Rng"].concat(),
            ["ResMut<", "GameSession>"].concat(),
            ["Enemy", " {"].concat(),
        ] {
            assert!(!source.contains(&forbidden), "{forbidden}");
        }
    }

    #[test]
    fn cast_models_ship_real_emissive_accents_except_the_plain_gnawling() {
        let model_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/models");
        let mut model_count = 0;
        let mut emissive_count = 0;
        for entry in fs::read_dir(model_dir).expect("read model directory") {
            let path = entry.expect("read model entry").path();
            if path.extension().and_then(|value| value.to_str()) != Some("glb") {
                continue;
            }
            model_count += 1;
            let bytes = fs::read(&path).expect("read GLB");
            let emissive = bytes
                .windows(b"emissiveFactor".len())
                .any(|window| window == b"emissiveFactor");
            if emissive {
                emissive_count += 1;
            } else {
                assert_eq!(
                    path.file_name().and_then(|value| value.to_str()),
                    Some("enemy_gnawling.glb"),
                    "unexpected model without emissive art: {}",
                    path.display()
                );
            }
        }
        assert_eq!(model_count, 35);
        assert_eq!(emissive_count, 34);
    }
}
