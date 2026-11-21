//! Static kinematic road collision: a circular footprint swept against the
//! inward-offset road edges. No rigid-body simulation is needed for flat paving.
use super::Hero;
use bevy::{prelude::*, transform::TransformSystems};

pub(crate) const TILE_SIZE: f32 = 2.0;
pub(crate) const COLUMNS: u32 = 18;
pub(crate) const ROWS: u32 = 5;
pub(crate) const FIRST_X: f32 = -16.5;
pub(crate) const HALF_WIDTH: f32 = ROWS as f32 * TILE_SIZE * 0.5;
pub(crate) const HERO_RADIUS: f32 = 0.6;
// The commander combat area ends before the enemy entrance and rear scenery.
pub const HERO_MIN_X: f32 = -10.0;
pub const HERO_MAX_X: f32 = 10.0;
pub const HERO_MIN_Z: f32 = -HALF_WIDTH + HERO_RADIUS;
pub const HERO_MAX_Z: f32 = HALF_WIDTH - HERO_RADIUS;

pub(crate) fn tile_center(column: u32, row: u32) -> Vec3 {
    Vec3::new(
        FIRST_X + column as f32 * TILE_SIZE,
        -0.15,
        -HALF_WIDTH + TILE_SIZE * 0.5 + row as f32 * TILE_SIZE,
    )
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct HeroBoundarySet;

#[derive(Resource, Default)]
pub(crate) struct BoundaryRecovery(pub bool);

pub(crate) fn install(app: &mut App) {
    app.init_resource::<BoundaryRecovery>().add_systems(
        PostUpdate,
        enforce_hero_boundary
            .in_set(HeroBoundarySet)
            .before(TransformSystems::Propagate),
    );
}

pub(crate) fn constrain(mut position: Vec3) -> Vec3 {
    // Recover invalid saved/external positions as well as ordinary penetrations.
    if !position.x.is_finite() {
        position.x = -8.2;
    }
    if !position.z.is_finite() {
        position.z = 0.;
    }
    position.x = position.x.clamp(HERO_MIN_X, HERO_MAX_X);
    position.z = position.z.clamp(HERO_MIN_Z, HERO_MAX_Z);
    position
}

/// Sweep to the first wall, discard only the inward normal's blocked movement,
/// then spend the remaining displacement along that wall. Two contacts suffice
/// for this convex axis-aligned road, including very large timestep/dodge motion.
pub(crate) fn sweep_and_slide(origin: Vec3, displacement: Vec3) -> Vec3 {
    let mut point = constrain(origin);
    let mut remaining = Vec3::new(displacement.x, 0., displacement.z);
    if !remaining.is_finite() {
        return point;
    }
    for _ in 0..2 {
        let mut time = 1.0;
        let mut axis = None;
        for (index, min, max) in [(0, HERO_MIN_X, HERO_MAX_X), (2, HERO_MIN_Z, HERO_MAX_Z)] {
            let velocity = remaining[index];
            if velocity == 0. {
                continue;
            }
            let distance = if velocity > 0. {
                max - point[index]
            } else {
                min - point[index]
            };
            let contact = (distance / velocity).max(0.);
            if contact < time {
                time = contact;
                axis = Some(index);
            }
        }
        point += remaining * time;
        let Some(axis) = axis else {
            break;
        };
        remaining *= 1.0 - time;
        remaining[axis] = 0.;
    }
    constrain(point)
}

fn enforce_hero_boundary(
    mut heroes: Query<&mut Transform, With<Hero>>,
    mut recovered: ResMut<BoundaryRecovery>,
) {
    recovered.0 = false;
    // Deliberately independent of session pause, life state and input. Any Update
    // writer (skills, respawn, imported state) must obey the same rendered boundary.
    for mut transform in &mut heroes {
        let corrected = constrain(transform.translation);
        if corrected != transform.translation {
            transform.translation = corrected;
            recovered.0 = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radius_matches_the_actual_paving_rows() {
        let north = tile_center(0, 0).z - TILE_SIZE * 0.5;
        let south = tile_center(0, ROWS - 1).z + TILE_SIZE * 0.5;
        assert_eq!(HERO_MIN_Z - HERO_RADIUS, north);
        assert_eq!(HERO_MAX_Z + HERO_RADIUS, south);
        assert!(HERO_MIN_X - HERO_RADIUS >= FIRST_X - TILE_SIZE * 0.5);
        assert!(HERO_MAX_X + HERO_RADIUS <= tile_center(COLUMNS - 1, 0).x + TILE_SIZE * 0.5);
    }

    #[test]
    fn large_displacements_stop_at_corners_and_slide_along_walls() {
        for x in [-1., 1.] {
            for z in [-1., 1.] {
                let p = sweep_and_slide(Vec3::ZERO, Vec3::new(x * 10000., 0., z * 10000.));
                assert!((p.x - x * HERO_MAX_X).abs() < 0.001);
                assert!((p.z - z * HERO_MAX_Z).abs() < 0.001);
                let slide = sweep_and_slide(
                    Vec3::new(0., 0., z * HERO_MAX_Z),
                    Vec3::new(3., 0., z * 50.),
                );
                assert!((slide - Vec3::new(3., 0., z * HERO_MAX_Z)).length() < 0.001);
            }
        }
    }

    #[test]
    fn collision_is_independent_of_step_subdivision() {
        let origin = Vec3::new(-9., 0., -4.);
        let delta = Vec3::new(50., 0., 17.);
        let whole = sweep_and_slide(origin, delta);
        let mut stepped = origin;
        for _ in 0..100 {
            stepped = sweep_and_slide(stepped, delta / 100.);
        }
        assert!(whole.distance(stepped) < 0.001);
        assert_eq!(
            constrain(Vec3::new(f32::NAN, -1000., f32::INFINITY)),
            Vec3::new(-8.2, -1000., 0.)
        );
    }
}
