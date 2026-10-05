use super::*;

/// Absolute root scale: pooled summons and live upgrades share the same baseline.
/// Larger creatures grow less so their silhouettes remain readable in a crowd.
pub(super) fn summon_scale(skill: usize, rank: u8) -> Vec3 {
    let (base, growth) = match skill {
        4 => (Vec3::splat(0.7), 0.20),
        5 => (Vec3::splat(0.82), 0.13),
        16 | 17 => (Vec3::splat(0.82), 0.15),
        21 => (Vec3::new(0.32, 1.9, 0.32), 0.12),
        26 => (Vec3::splat(0.9), 0.12),
        27 => (Vec3::splat(0.92), 0.15),
        29 => (Vec3::splat(0.78), 0.13),
        _ => return Vec3::ONE,
    };
    base * (1.0 + f32::from(rank.clamp(1, 5) - 1) * growth)
}

// Fixed defense formation keeps the growing skeletons clear of the starting hero.
pub(super) fn bone_anchor(slot: usize) -> Vec3 {
    [
        Vec3::new(super::super::resources::GATE_POSITION.x + 1.8, 0.0, 2.2),
        Vec3::new(super::super::resources::GATE_POSITION.x + 1.8, 0.0, -2.2),
        Vec3::new(super::super::resources::GATE_POSITION.x + 2.8, 0.0, 0.0),
    ][slot.min(2)]
}

pub(super) fn update_summon_growth(
    time: Res<Time>,
    session: Res<GameSession>,
    build: Res<PlayerBuild>,
    mut summons: Query<
        (
            &mut Transform,
            Option<&BoneSentry>,
            Option<&SpiritHound>,
            Option<&TideElemental>,
            Option<&BoltTurret>,
            Option<&OathBanner>,
            Option<&FrostWyrm>,
            Option<&AncestorColossus>,
        ),
        With<SkillSummon>,
    >,
    evolution: Option<Res<super::super::skill_evolution::SkillEvolution>>,
) {
    if session.paused || session.finished {
        return;
    }
    let blend = 1.0 - (-8.0 * time.delta_secs()).exp();
    for (mut transform, bone, hound, tide, turret, banner, wyrm, colossus) in &mut summons {
        let skill = if bone.is_some() {
            4
        } else if let Some(hound) = hound {
            hound.source_skill
        } else if tide.is_some() {
            16
        } else if turret.is_some() {
            17
        } else if banner.is_some() {
            21
        } else if wyrm.is_some() {
            26
        } else if colossus.is_some() {
            27
        } else {
            continue;
        };
        let tier = evolution.as_ref().map_or(0, |e| e.tiers[skill]);
        let growth = if tier == 0 {
            1.0
        } else if skill == 4 {
            1.8 + f32::from(tier) * 0.25
        } else {
            1.2 + f32::from(tier) * 0.25
        };
        let target = summon_scale(skill, build.skills[skill]) * growth;
        transform.scale = transform.scale.lerp(target, blend);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn advance(app: &mut App) {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(0.1));
        app.update();
    }

    #[test]
    fn live_skeleton_upgrade_grows_smoothly_without_compounding_or_moving() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<GameSession>()
            .init_resource::<PlayerBuild>()
            .add_systems(Update, update_summon_growth);
        let location = Vec3::new(-8.6, 0.0, 1.65);
        let entity = app
            .world_mut()
            .spawn((
                SkillSummon,
                BoneSentry {
                    attack_remaining: 0.0,
                    damage: 1.0,
                },
                Transform::from_translation(location).with_scale(summon_scale(4, 1)),
            ))
            .id();
        app.world_mut().resource_mut::<PlayerBuild>().skills[4] = 5;
        advance(&mut app);
        let scale = app.world().get::<Transform>(entity).unwrap().scale.x;
        assert!(scale > 0.7 && scale < 1.26);
        for _ in 0..30 {
            advance(&mut app);
        }
        let transform = app.world().get::<Transform>(entity).unwrap();
        assert!((transform.scale.x - 1.26).abs() < 0.0001);
        assert_eq!(transform.translation, location);
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.world_mut().resource_mut::<PlayerBuild>().skills[4] = 1;
        advance(&mut app);
        assert!((app.world().get::<Transform>(entity).unwrap().scale.x - 1.26).abs() < 0.0001);
    }

    #[test]
    fn each_summon_grows_at_every_rank_and_fate_uses_its_own_skill() {
        for skill in [4, 5, 16, 17, 21, 26, 27, 29] {
            for rank in 2..=5 {
                assert!(
                    summon_scale(skill, rank)
                        .cmpgt(summon_scale(skill, rank - 1))
                        .all()
                );
            }
            assert_eq!(summon_scale(skill, 0), summon_scale(skill, 1));
            assert_eq!(summon_scale(skill, 255), summon_scale(skill, 5));
        }
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<GameSession>()
            .init_resource::<PlayerBuild>()
            .add_systems(Update, update_summon_growth);
        app.world_mut().resource_mut::<PlayerBuild>().skills[5] = 1;
        app.world_mut().resource_mut::<PlayerBuild>().skills[29] = 5;
        let entity = app
            .world_mut()
            .spawn((
                SkillSummon,
                SpiritHound {
                    source_skill: 29,
                    remaining: 8.0,
                    attack_remaining: 0.0,
                    damage: 1.0,
                },
                Transform::from_scale(summon_scale(29, 5)),
            ))
            .id();
        for _ in 0..10 {
            advance(&mut app);
        }
        assert_eq!(
            app.world().get::<Transform>(entity).unwrap().scale,
            summon_scale(29, 5)
        );
        // A recycled hound overwrites its source and absolute transform on rent.
        app.world_mut().entity_mut(entity).insert((
            SpiritHound {
                source_skill: 5,
                remaining: 10.0,
                attack_remaining: 0.0,
                damage: 1.0,
            },
            Transform::from_scale(summon_scale(5, 1)),
        ));
        advance(&mut app);
        assert_eq!(
            app.world().get::<Transform>(entity).unwrap().scale,
            summon_scale(5, 1)
        );
    }
}
