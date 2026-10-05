//! Shared planar pursuit for mobile summons. Stop at reach even at high battle speed.
use bevy::prelude::*;
pub(crate) fn advance(
    transform: &mut Transform,
    target: Vec3,
    speed: f32,
    stop_range: f32,
    delta: f32,
) -> bool {
    let front = crate::game::resources::GATE_POSITION.x + 1.8;
    transform.translation.x = transform.translation.x.max(front);
    if target.x < front {
        transform.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, Vec3::X);
        return false;
    }
    let offset = (target - transform.translation).with_y(0.0);
    let distance = offset.length();
    if distance > 0.001 {
        let direction = offset / distance;
        transform.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, direction);
        transform.translation += direction
            * (distance - stop_range.max(0.0))
                .max(0.0)
                .min(speed.max(0.0) * delta.max(0.0));
    }
    (target - transform.translation).with_y(0.0).length() <= stop_range.max(0.0) + 0.001
}
pub(crate) fn follow(transform: &mut Transform, slot: u64, speed: f32, delta: f32) {
    let column = (slot % 3) as f32;
    let lane = ((slot / 3) % 9) as f32;
    let target = Vec3::new(
        crate::game::resources::GATE_POSITION.x + 2.3 + column * 1.1,
        0.0,
        -4.0 + lane,
    );
    advance(transform, target, speed, 0.1, delta);
    transform.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, Vec3::X);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accelerated_pursuit_stops_at_reach_and_faces_target_without_changing_height() {
        let mut t = Transform::from_xyz(0.0, 0.7, 0.0);
        assert!(advance(&mut t, Vec3::new(5.0, 0.0, 0.0), 5.2, 1.1, 32.0));
        assert!((t.translation.x - 3.9).abs() < 0.001);
        assert_eq!(t.translation.y, 0.7);
        assert!(t.forward().dot(Vec3::X) > 0.99);
        assert!(advance(&mut t, Vec3::new(5.0, 0.0, 0.0), 5.2, 1.1, 32.0));
        assert!((t.translation.x - 3.9).abs() < 0.001);
    }
    #[test]
    fn rear_targets_never_pull_summons_through_the_wall() {
        let mut summon = Transform::from_xyz(-8.0, 0.5, 0.0);
        assert!(!advance(
            &mut summon,
            Vec3::new(-10.0, 0.0, 0.0),
            5.0,
            1.0,
            32.0
        ));
        assert!(summon.translation.x >= crate::game::resources::GATE_POSITION.x + 1.8);
        assert!(summon.forward().dot(Vec3::X) > 0.99);
    }
    #[test]
    fn zero_delta_never_moves_and_follow_returns_to_formation() {
        let mut t = Transform::from_xyz(20.0, 0.0, 0.0);
        let before = t.translation;
        assert!(!advance(&mut t, Vec3::ZERO, 4.0, 1.0, 0.0));
        assert_eq!(t.translation, before);
        follow(&mut t, 0, 4.0, 30.0);
        assert!(t.translation.x > crate::game::resources::GATE_POSITION.x + 1.8);
        assert!(t.forward().dot(Vec3::X) > 0.99);
    }
}
