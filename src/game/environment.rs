use bevy::prelude::*;

use super::{ArenaEntity, GameSession};

pub struct EnvironmentArtPlugin;

impl Plugin for EnvironmentArtPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ArtClock>()
            .add_systems(Startup, setup_environment_art)
            .add_systems(Update, (animate_environment_art, update_camp_previews));
    }
}

#[derive(Resource, Default)]
struct ArtClock {
    elapsed: f32,
}
#[derive(Component)]
pub(super) struct CampPreview(pub(super) u8);
fn update_camp_previews(
    state: Res<super::AdventureBossState>,
    mut previews: Query<(&CampPreview, &mut Visibility)>,
) {
    for (camp, mut visibility) in &mut previews {
        *visibility = if state.resolved & (1 << camp.0) != 0
            || state.active.is_some() && state.active_index == camp.0
        {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
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

// Props are CC0 KayKit dungeon artwork, normalized to unit height on import.
// Gate masonry and timber detailing share a unit box; silhouettes and ornament
// retain the authored CC0 tower and gate models.
struct EnvironmentModels {
    pillar: Handle<WorldAsset>,
    torch: Handle<WorldAsset>,
    rubble: Handle<WorldAsset>,
    rubble_half: Handle<WorldAsset>,
}

fn setup_environment_art(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let models = EnvironmentModels {
        pillar: assets.load("models/environment_pillar.glb#Scene0"),
        torch: assets.load("models/environment_torch_lit.glb#Scene0"),
        rubble: assets.load("models/environment_rubble_large.glb#Scene0"),
        rubble_half: assets.load("models/environment_rubble_half.glb#Scene0"),
    };
    spawn_side_camps(&mut commands, &assets, &mut meshes, &mut materials, &models);
    spawn_border_dressing(&mut commands, &models);
}

fn spawn_prop(
    commands: &mut Commands,
    scene: &Handle<WorldAsset>,
    position: Vec3,
    height: f32,
    yaw: f32,
) {
    commands.spawn((
        WorldAssetRoot(scene.clone()),
        Transform::from_translation(position)
            .with_scale(Vec3::splat(height))
            .with_rotation(Quat::from_rotation_y(yaw)),
        ArenaEntity,
    ));
}

/// The entire defensive structure belongs to Core: its visibility and revival
/// therefore follow the actual tank, rather than a separate rear-wall prop.
pub(super) fn spawn_gate_art(
    parent: &mut ChildSpawnerCommands,
    assets: &AssetServer,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    // The wall follows the road's Z axis, perpendicular to enemies travelling
    // towards decreasing X. All spans share one plane and touch both borders.
    let facing = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
    let block = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let stone = materials.add(StandardMaterial {
        base_color: Color::srgb(0.40, 0.47, 0.49),
        emissive: LinearRgba::new(0.07, 0.09, 0.10, 1.0),
        perceptual_roughness: 0.98,
        reflectance: 0.20,
        ..default()
    });
    let timber = materials.add(StandardMaterial {
        base_color: Color::srgb(0.42, 0.22, 0.085),
        emissive: LinearRgba::new(0.10, 0.045, 0.012, 1.0),
        perceptual_roughness: 0.94,
        reflectance: 0.16,
        ..default()
    });
    let iron = materials.add(StandardMaterial {
        base_color: Color::srgb(0.29, 0.35, 0.39),
        emissive: LinearRgba::new(0.035, 0.045, 0.055, 1.0),
        metallic: 0.78,
        perceptual_roughness: 0.62,
        reflectance: 0.38,
        ..default()
    });
    // The fixed sun lights the rear of this perpendicular wall. Two soft
    // gatehouse lamps reveal the invader-facing masonry without changing the
    // entire arena's lighting or making the stone bloom like a spell.
    for z in [-2.8, 2.8] {
        parent.spawn((
            Name::new("Gatehouse facade fill light"),
            PointLight {
                color: Color::srgb(1.0, 0.90, 0.78),
                intensity: 130_000.0,
                range: 13.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_xyz(3.4, 4.0, z),
        ));
    }
    // Courses have real depth and readable joints, even when the imported gate
    // facade faces away from the directional light. Their X plane never tilts.
    for side in [-1.0, 1.0] {
        for course in 0..4 {
            for column in 0..4 {
                let z = side * (2.0 + 0.9 * column as f32);
                parent.spawn((
                    Name::new("Gate masonry course"),
                    Mesh3d(block.clone()),
                    MeshMaterial3d(stone.clone()),
                    Transform::from_xyz(0.0, -0.67 + 0.64 * course as f32, z)
                        .with_scale(Vec3::new(1.1, 0.60, 0.86)),
                ));
            }
        }
    }
    // Solid double doors sit inside the existing iron frame. Eight narrow
    // boards, two horizontal straps and a central seam read as a closed gate.
    for board in 0..8 {
        let z = -1.54 + 0.44 * board as f32;
        parent.spawn((
            Name::new("Gate timber door plank"),
            Mesh3d(block.clone()),
            MeshMaterial3d(timber.clone()),
            Transform::from_xyz(0.45, 0.31, z).with_scale(Vec3::new(0.30, 2.62, 0.41)),
        ));
    }
    for y in [-0.25, 0.90] {
        parent.spawn((
            Name::new("Gate iron door brace"),
            Mesh3d(block.clone()),
            MeshMaterial3d(iron.clone()),
            Transform::from_xyz(0.64, y, 0.0).with_scale(Vec3::new(0.10, 0.18, 3.46)),
        ));
    }
    for z in [-0.18, 0.18] {
        parent.spawn((
            Name::new("Gate iron door handle"),
            Mesh3d(block.clone()),
            MeshMaterial3d(iron.clone()),
            Transform::from_xyz(0.71, 0.25, z).with_scale(Vec3::new(0.14, 0.34, 0.10)),
        ));
    }
    for (name, model, z, width, height) in [
        ("central iron gate", "wall_gated", 0.0, 3.8, 3.0),
        ("north curtain wall", "wall", -3.8, 3.8, 2.6),
        ("south curtain wall", "wall", 3.8, 3.8, 2.6),
    ] {
        parent.spawn((
            Name::new(format!("Gate {name}")),
            WorldAssetRoot(assets.load(format!("models/environment_{model}.glb#Scene0"))),
            Transform::from_xyz(0.0, -1.0, z)
                .with_rotation(facing)
                .with_scale(Vec3::new(width, height, 3.2)),
        ));
    }
    for (side, z) in [
        ("north border", -5.65),
        ("north gatehouse", -1.9),
        ("south gatehouse", 1.9),
        ("south border", 5.65),
    ] {
        parent.spawn((
            Name::new(format!("Gate {side} tower")),
            WorldAssetRoot(assets.load("models/environment_pillar_decorated.glb#Scene0")),
            Transform::from_xyz(0.0, -1.0, z)
                .with_rotation(facing)
                .with_scale(Vec3::new(4.2, 3.65, 4.2)),
        ));
        // Banners face the invaders; their anchors sit on the gatehouse facade.
        if z.abs() < 2.0 {
            parent.spawn((
                Name::new(format!("Gate {side} blue banner")),
                WorldAssetRoot(assets.load("models/environment_banner_blue.glb#Scene0")),
                Transform::from_xyz(0.66, 0.65, z)
                    .with_rotation(facing)
                    .with_scale(Vec3::splat(1.35)),
            ));
        }
    }
}

fn spawn_side_camps(
    commands: &mut Commands,
    assets: &AssetServer,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    models: &EnvironmentModels,
) {
    let floor = assets.load("models/environment_floor.glb#Scene0");
    let stone = meshes.add(Sphere::new(1.0));
    let grass = meshes.add(Cone::new(0.12, 0.65));
    for (index, center) in super::encounter_bosses::ADVENTURE_SITES
        .into_iter()
        .enumerate()
    {
        let boss = assets.load(
            [
                "models/adventure_embermaw.glb#Scene0",
                "models/adventure_tidehorn.glb#Scene0",
                "models/adventure_plaguejaw.glb#Scene0",
            ][index],
        );
        commands.spawn((
            Name::new(format!("Resting side camp leader {index}")),
            WorldAssetRoot(boss),
            Transform::from_translation(center)
                .with_scale(Vec3::splat(2.7))
                .with_rotation(Quat::from_rotation_y(1.0)),
            CampPreview(index as u8),
            ArenaEntity,
        ));
        for n in 0..3 {
            let guard = assets.load(
                [
                    "models/enemy_rift_sapper.glb#Scene0",
                    "models/enemy_void_caster.glb#Scene0",
                    "models/enemy_split_spore.glb#Scene0",
                ][n],
            );
            commands.spawn((
                Name::new(format!("Resting camp wild guard {index}/{n}")),
                WorldAssetRoot(guard),
                Transform::from_translation(center + Vec3::new(-1.3 + n as f32 * 1.3, 0.0, 1.8))
                    .with_scale(Vec3::splat(0.9 + n as f32 * 0.05)),
                CampPreview(index as u8),
                ArenaEntity,
            ));
        }
        let soil = materials.add(StandardMaterial {
            base_color: [
                Color::srgb(0.22, 0.16, 0.12),
                Color::srgb(0.12, 0.21, 0.22),
                Color::srgb(0.15, 0.21, 0.11),
            ][index],
            perceptual_roughness: 1.0,
            reflectance: 0.15,
            ..default()
        });
        let moss = materials.add(StandardMaterial {
            base_color: Color::srgb(0.17, 0.24, 0.13),
            perceptual_roughness: 1.0,
            reflectance: 0.15,
            ..default()
        });
        commands.spawn((
            Name::new(format!("Side camp terrain {index}")),
            Mesh3d(meshes.add(Cylinder::new(3.6, 0.12))),
            MeshMaterial3d(soil.clone()),
            Transform::from_translation(center - Vec3::Y * 0.1),
            ArenaEntity,
        ));
        for n in 0..24 {
            let angle = n as f32 * 2.39996;
            let radius = 2.4 + (n % 4) as f32 * 0.28;
            let position = center + Vec3::new(angle.cos() * radius, -0.02, angle.sin() * radius);
            spawn_static(
                commands,
                stone.clone(),
                soil.clone(),
                Transform::from_translation(position)
                    .with_scale(Vec3::new(0.32 + (n % 3) as f32 * 0.08, 0.13, 0.24))
                    .with_rotation(Quat::from_rotation_y(angle)),
            );
            if n % 2 == 0 {
                spawn_static(
                    commands,
                    grass.clone(),
                    moss.clone(),
                    Transform::from_translation(position + Vec3::Y * 0.2)
                        .with_rotation(Quat::from_rotation_z(0.28)),
                );
            }
        }
        for step in 0..4 {
            let z = center.z.signum() * (5.2 + step as f32 * 0.9);
            commands.spawn((
                Name::new("Side camp stone approach"),
                WorldAssetRoot(floor.clone()),
                Transform::from_xyz(center.x + (step % 2) as f32 * 0.16, -0.14, z)
                    .with_scale(Vec3::splat(0.11))
                    .with_rotation(Quat::from_rotation_y(step as f32 * 0.12)),
                ArenaEntity,
            ));
        }
        spawn_prop(
            commands,
            &models.rubble,
            center + Vec3::new(2.7, 0.0, 1.6),
            1.3,
            index as f32,
        );
        spawn_prop(
            commands,
            &models.pillar,
            center + Vec3::new(-2.5, 0.0, -2.0),
            1.1,
            0.4,
        );
        spawn_prop(
            commands,
            &models.torch,
            center + Vec3::new(-2.5, 1.0, -2.0),
            0.55,
            0.0,
        );
    }
}

fn spawn_border_dressing(commands: &mut Commands, models: &EnvironmentModels) {
    for z in [-6.35, 6.35] {
        for (index, x) in [-11.8, -5.6, 0.6, 6.8, 12.0].into_iter().enumerate() {
            spawn_prop(commands, &models.pillar, Vec3::new(x, 0.0, z), 1.55, 0.0);
            if index % 2 == 0 {
                spawn_prop(commands, &models.torch, Vec3::new(x, 1.55, z), 0.95, 0.0);
            }
        }
    }
    for (index, (x, z, height)) in [
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
        let scene = if index % 2 == 0 {
            &models.rubble
        } else {
            &models.rubble_half
        };
        spawn_prop(
            commands,
            scene,
            Vec3::new(x, 0.0, z),
            height,
            index as f32 * 0.58,
        );
    }
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
}
