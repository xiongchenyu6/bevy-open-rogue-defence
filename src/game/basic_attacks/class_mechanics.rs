use super::{BasicWeapon, weapon};
use crate::game::{HeroKind, PlayerBuild};
use bevy::prelude::*;
use std::collections::HashSet;

pub const RICOCHET_RANGE: f32 = 5.0;
pub const RICOCHET_DAMAGE_RATIO: f32 = 0.75;

#[derive(Debug, Clone, Copy)]
pub struct ClassBasicStats {
    pub shots: u8,
    pub bounces: u8,
    pub cleave_targets: usize,
    pub cleave_ratio: f32,
    pub range: f32,
    pub half_angle: f32,
}

pub fn class_basic_stats(kind: HeroKind, level: u32, build: &PlayerBuild) -> ClassBasicStats {
    let tier = [10, 20, 40]
        .into_iter()
        .filter(|milestone| level >= *milestone)
        .count() as u8;
    let weapon = weapon(kind);
    let archer = weapon == BasicWeapon::Crossbow;
    let warrior = matches!(weapon, BasicWeapon::Sword | BasicWeapon::Axe);
    ClassBasicStats {
        shots: if archer {
            (2 + tier)
                .saturating_add(build.multishot.saturating_sub(1))
                .min(8)
        } else {
            build.multishot.max(1)
        },
        bounces: if archer {
            (1 + tier).saturating_add(build.ricochet).min(4)
        } else {
            0
        },
        cleave_targets: if weapon.melee() {
            usize::from(build.multishot.max(1)) * (usize::from(build.pierce) + 1)
                + if warrior {
                    2 + usize::from(build.cleave.min(6))
                } else {
                    0
                }
        } else {
            1
        },
        cleave_ratio: if warrior {
            0.6 + f32::from(tier) * 0.1
        } else {
            1.0
        },
        range: weapon.range(build.attack_range) + if warrior { f32::from(tier) * 0.2 } else { 0.0 },
        half_angle: weapon.half_angle(),
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Ricochet {
    pub remaining: u8,
    pub bounces_used: u8,
    pub target: Option<Entity>,
}
impl Ricochet {
    pub(crate) fn new(remaining: u8) -> Self {
        Self {
            remaining,
            bounces_used: 0,
            target: None,
        }
    }
}

pub(crate) fn next_ricochet_target(
    origin: Vec3,
    targets: &[(Entity, Vec3)],
    hit: &HashSet<Entity>,
) -> Option<(Entity, Vec3)> {
    targets
        .iter()
        .copied()
        .filter(|(entity, position)| {
            !hit.contains(entity)
                && position.xz().distance_squared(origin.xz()) <= RICOCHET_RANGE * RICOCHET_RANGE
        })
        .min_by(|a, b| {
            a.1.xz()
                .distance_squared(origin.xz())
                .total_cmp(&b.1.xz().distance_squared(origin.xz()))
                .then_with(|| a.0.to_bits().cmp(&b.0.to_bits()))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn class_milestones_and_bounded_equipment_feed_real_attack_profiles() {
        let build = PlayerBuild::default();
        for (level, shots, bounces, ratio) in [
            (1, 2, 1, 0.6),
            (10, 3, 2, 0.7),
            (20, 4, 3, 0.8),
            (40, 5, 4, 0.9),
        ] {
            let archer = class_basic_stats(HeroKind::EmberRanger, level, &build);
            assert_eq!((archer.shots, archer.bounces), (shots, bounces));
            let warrior = class_basic_stats(HeroKind::StoneWarden, level, &build);
            assert_eq!(warrior.cleave_targets, 3);
            assert!((warrior.cleave_ratio - ratio).abs() < 0.001);
        }
        let build = PlayerBuild {
            multishot: 255,
            ricochet: 255,
            cleave: 255,
            ..default()
        };
        assert_eq!(
            class_basic_stats(HeroKind::EmberRanger, 40, &build).shots,
            8
        );
        assert_eq!(
            class_basic_stats(HeroKind::EmberRanger, 40, &build).bounces,
            4
        );
        assert_eq!(
            class_basic_stats(
                HeroKind::IronBerserker,
                40,
                &PlayerBuild {
                    cleave: 255,
                    ..default()
                }
            )
            .cleave_targets,
            9
        );
        assert_eq!(
            class_basic_stats(HeroKind::TideWalker, 40, &PlayerBuild::default()).cleave_targets,
            1
        );
    }
    #[test]
    fn ricochet_chooses_nearest_unhit_target_with_stable_tie_and_true_range() {
        let a = Entity::from_bits(10);
        let b = Entity::from_bits(11);
        let c = Entity::from_bits(12);
        let targets = [
            (c, Vec3::X * 5.1),
            (b, Vec3::X * 2.0),
            (a, Vec3::NEG_X * 2.0),
        ];
        let mut hit = HashSet::new();
        assert_eq!(
            next_ricochet_target(Vec3::ZERO, &targets, &hit).unwrap().0,
            a
        );
        hit.insert(a);
        assert_eq!(
            next_ricochet_target(Vec3::ZERO, &targets, &hit).unwrap().0,
            b
        );
        hit.insert(b);
        assert!(next_ricochet_target(Vec3::ZERO, &targets, &hit).is_none());
    }
}
