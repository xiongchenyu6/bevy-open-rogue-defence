//! Read-only world presentation of the optional encounter lifecycle.
use super::*;
use crate::game::encounter_bosses::{ADVENTURE_SITES, AdventureBossState};

const WAKE_SECONDS: f32 = 1.8;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct CampPresenceSet;

#[derive(Resource, Default)]
pub(super) struct CampPresence {
    unlocked: u8,
    wake: [f32; 3],
    elapsed: f32,
}

impl CampPresence {
    fn update(&mut self, state: &AdventureBossState, delta: f32) {
        let newly_ready = state.unlocked & !self.unlocked & !state.resolved;
        for (index, wake) in self.wake.iter_mut().enumerate() {
            *wake = if state.unlocked & (1 << index) == 0 {
                0.0
            } else if newly_ready & (1 << index) != 0 {
                WAKE_SECONDS
            } else {
                (*wake - delta).max(0.0)
            };
        }
        self.unlocked = state.unlocked;
        self.elapsed += delta;
    }

    fn wake_pulse(&self, index: usize) -> f32 {
        let remaining = self.wake[index];
        if remaining <= 0.0 {
            return 0.0;
        }
        let progress = 1.0 - remaining / WAKE_SECONDS;
        (progress * std::f32::consts::PI).sin().max(0.0)
    }

    pub(super) fn intensity(&self, index: usize, state: &AdventureBossState) -> f32 {
        let bit = 1 << index;
        if state.completed & bit != 0 {
            0.52
        } else if state.resolved & bit != 0 {
            0.16
        } else if state.unlocked & bit == 0 {
            0.20
        } else {
            0.85 + self.wake_pulse(index) * 1.6
        }
    }
}

#[derive(Component)]
pub(super) struct CampLeaderPose(pub(super) u8);
#[derive(Component)]
struct CampVictoryFlag(u8);

pub(super) fn install(app: &mut App) {
    app.init_resource::<CampPresence>().add_systems(
        Update,
        (advance, pose_leaders, show_victories)
            .chain()
            .in_set(CampPresenceSet),
    );
}

pub(super) fn spawn_victory_flag(
    commands: &mut Commands,
    assets: &AssetServer,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    index: u8,
    center: Vec3,
) {
    let bronze = materials.add(StandardMaterial {
        base_color: Color::srgb(0.54, 0.36, 0.13),
        metallic: 0.35,
        perceptual_roughness: 0.72,
        ..default()
    });
    let stone = materials.add(StandardMaterial {
        base_color: Color::srgb(0.31, 0.34, 0.32),
        perceptual_roughness: 0.95,
        ..default()
    });
    commands
        .spawn((
            Name::new(format!("Conquered wild camp {index}")),
            // The imported banner is a hanging cloth, not a planted flag. Its front
            // faces +Z; turn it toward the battle camera and suspend it from a pole.
            Transform::from_translation(center).with_rotation(Quat::from_rotation_y(-1.2)),
            Visibility::Hidden,
            CampVictoryFlag(index),
            ArenaEntity,
        ))
        .with_children(|flag| {
            flag.spawn((
                WorldAssetRoot(assets.load("models/environment_banner_blue.glb#Scene0")),
                Transform::from_xyz(0.57, 0.86, 0.02).with_scale(Vec3::splat(2.0)),
            ));
            flag.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.055, 3.1))),
                MeshMaterial3d(bronze.clone()),
                Transform::from_xyz(0.0, 1.55, 0.0),
            ));
            flag.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.04, 1.25))),
                MeshMaterial3d(bronze.clone()),
                Transform::from_xyz(0.56, 2.9, 0.0)
                    .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
            ));
            flag.spawn((
                Mesh3d(meshes.add(Cone::new(0.11, 0.32))),
                MeshMaterial3d(bronze),
                Transform::from_xyz(0.0, 3.24, 0.0),
            ));
            flag.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.26, 0.18))),
                MeshMaterial3d(stone),
                Transform::from_xyz(0.0, 0.09, 0.0),
            ));
        });
}

fn advance(
    time: Res<Time<Real>>,
    session: Res<GameSession>,
    state: Res<AdventureBossState>,
    mut presence: ResMut<CampPresence>,
) {
    let delta = if session.paused || session.finished || session.save_active {
        0.0
    } else {
        time.delta_secs().min(0.1)
    };
    presence.update(&state, delta);
}

fn leader_pose(index: usize, state: &AdventureBossState, presence: &CampPresence) -> Transform {
    let center = ADVENTURE_SITES[index];
    // Native cast clips still animate the child rig. Root presentation supplies
    // only a low sleeping silhouette and a one-shot awakening, never attacks.
    let ready = state.unlocked & (1 << index) != 0;
    let pulse = presence.wake_pulse(index);
    let breath = (presence.elapsed * 1.25 + index as f32).sin() * 0.012;
    let height = if ready {
        1.0 + pulse * 0.13
    } else {
        0.68 + breath
    };
    let pitch = if ready { -pulse * 0.20 } else { 0.16 };
    Transform::from_translation(center + Vec3::Y * (pulse * 0.16))
        .with_scale(Vec3::new(2.7, 2.7 * height, 2.7))
        .with_rotation(Quat::from_rotation_y(1.0) * Quat::from_rotation_x(pitch))
}

fn pose_leaders(
    state: Res<AdventureBossState>,
    presence: Res<CampPresence>,
    mut leaders: Query<(&CampLeaderPose, &mut Transform)>,
) {
    for (leader, mut transform) in &mut leaders {
        *transform = leader_pose(usize::from(leader.0), &state, &presence);
    }
}

fn show_victories(
    state: Res<AdventureBossState>,
    mut flags: Query<(&CampVictoryFlag, &mut Visibility)>,
) {
    for (flag, mut visibility) in &mut flags {
        *visibility = if state.completed & (1 << flag.0) != 0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sleeping_wake_ready_and_reset_have_distinct_bounded_presentations() {
        let mut state = AdventureBossState::default();
        let mut presence = CampPresence::default();
        presence.update(&state, 0.1);
        let sleeping = leader_pose(0, &state, &presence);
        assert!(sleeping.scale.y < 2.0);
        assert_eq!(presence.intensity(0, &state), 0.2);
        state.unlocked = 1;
        presence.update(&state, 0.1);
        assert_eq!(presence.wake[0], WAKE_SECONDS);
        presence.update(&state, WAKE_SECONDS * 0.5);
        let awake = leader_pose(0, &state, &presence);
        assert!(awake.scale.y > 2.7 && awake.translation.y > 0.0);
        assert!(presence.intensity(0, &state) > 2.0);
        presence.update(&state, WAKE_SECONDS);
        assert_eq!(leader_pose(0, &state, &presence).scale, Vec3::splat(2.7));
        assert_eq!(presence.intensity(0, &state), 0.85);
        state.unlocked = 0;
        presence.update(&state, 0.1);
        assert_eq!(presence.wake, [0.0; 3]);
        assert!(leader_pose(0, &state, &presence).scale.y < 2.0);
    }

    #[test]
    fn victory_flag_requires_a_real_completion_not_merely_resolved() {
        let mut app = App::new();
        app.init_resource::<AdventureBossState>()
            .add_systems(Update, show_victories);
        let flag = app
            .world_mut()
            .spawn((CampVictoryFlag(1), Visibility::Hidden))
            .id();
        app.world_mut()
            .resource_mut::<AdventureBossState>()
            .resolved = 2;
        app.update();
        assert_eq!(
            app.world().get::<Visibility>(flag),
            Some(&Visibility::Hidden)
        );
        app.world_mut()
            .resource_mut::<AdventureBossState>()
            .completed = 2;
        app.update();
        assert_eq!(
            app.world().get::<Visibility>(flag),
            Some(&Visibility::Inherited)
        );
        app.world_mut()
            .insert_resource(AdventureBossState::default());
        app.update();
        assert_eq!(
            app.world().get::<Visibility>(flag),
            Some(&Visibility::Hidden)
        );
    }
}
