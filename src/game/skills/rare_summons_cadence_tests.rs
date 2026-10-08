//! Finite pooled summon output under equal simulated time, including expiry.
use super::{
    tests::{app, enemy},
    *,
};
use bevy::ecs::system::RunSystemOnce;
use std::time::Duration;

// Outgoing output only: no projectile movement/collision, casting, or enemy AI.
// update_epic_summons only needs the resources installed by app(); apply_control
// writes SkillRuntime directly, without EnemyStatus or message resources.
#[derive(Clone, Copy, Debug)]
enum FiniteSummon {
    Tide,
    Turret,
    Wyrm,
    Colossus,
}
impl FiniteSummon {
    fn skill(self) -> usize {
        match self {
            Self::Tide => 16,
            Self::Turret => 17,
            Self::Wyrm => 26,
            Self::Colossus => 27,
        }
    }
    fn interval(self) -> f32 {
        match self {
            Self::Tide => 1.0,
            Self::Turret => 0.72,
            Self::Wyrm => 1.15,
            Self::Colossus => 3.0,
        }
    }
    fn visual(self) -> SummonVisual {
        match self {
            Self::Tide => SummonVisual::TideElemental,
            Self::Turret => SummonVisual::BoltTurret,
            Self::Wyrm => SummonVisual::FrostWyrm,
            Self::Colossus => SummonVisual::AncestorColossus,
        }
    }
    fn update(self, app: &mut App) {
        match self {
            Self::Tide | Self::Turret => {
                app.world_mut()
                    .run_system_once(update_rare_summons)
                    .unwrap();
            }
            Self::Wyrm | Self::Colossus => {
                app.world_mut()
                    .run_system_once(update_epic_summons)
                    .unwrap();
            }
        }
    }
}
const FINITE_SUMMONS: [FiniteSummon; 4] = [
    FiniteSummon::Tide,
    FiniteSummon::Turret,
    FiniteSummon::Wyrm,
    FiniteSummon::Colossus,
];
fn finite_fixture(kind: FiniteSummon, lifetime: f32) -> (App, Entity) {
    let mut app = app();
    app.world_mut().resource_mut::<PlayerBuild>().skills[kind.skill()] = 1;
    let entity = app
        .world_mut()
        .run_system_once(
            move |mut commands: Commands, mut pools: ResMut<CombatPools>| {
                let entity = pools
                    .rent_scene_summon(
                        &mut commands,
                        kind.visual(),
                        Handle::default(),
                        Transform::default(),
                    )
                    .unwrap();
                let mut entity_commands = commands.entity(entity);
                entity_commands.insert(SkillSummon);
                match kind {
                    FiniteSummon::Tide => {
                        entity_commands.insert(TideElemental {
                            remaining: lifetime,
                            attack_remaining: 0.0,
                            damage: 50.0,
                        });
                    }
                    FiniteSummon::Turret => {
                        entity_commands.insert(BoltTurret {
                            remaining: lifetime,
                            attack_remaining: 0.0,
                            damage: 50.0,
                        });
                    }
                    FiniteSummon::Wyrm => {
                        entity_commands.insert(FrostWyrm {
                            remaining: lifetime,
                            attack_remaining: 0.0,
                            damage: 50.0,
                            frost_hits: default(),
                        });
                    }
                    FiniteSummon::Colossus => {
                        entity_commands.insert(AncestorColossus {
                            remaining: lifetime,
                            slam_remaining: 0.0,
                            damage: 50.0,
                        });
                    }
                }
                entity
            },
        )
        .unwrap();
    (app, entity)
}
fn take_finite_output(app: &mut App, kind: FiniteSummon) -> (usize, f64) {
    let projectiles: Vec<_> = app
        .world_mut()
        .query::<(Entity, &Projectile, &DamageProfile)>()
        .iter(app.world())
        .map(|(entity, shot, profile)| {
            assert_eq!(
                profile.skill_index,
                Some(u8::try_from(kind.skill()).unwrap())
            );
            assert_eq!(shot.damage, 50.0);
            (entity, f64::from(shot.damage))
        })
        .collect();
    let strikes: Vec<_> = app
        .world_mut()
        .query::<(Entity, &SkillStrike)>()
        .iter(app.world())
        .map(|(entity, strike)| {
            assert_eq!(
                strike.profile.skill_index,
                Some(u8::try_from(kind.skill()).unwrap())
            );
            assert_eq!(strike.damage, 50.0);
            (entity, f64::from(strike.damage))
        })
        .collect();
    let output = (
        projectiles.len() + strikes.len(),
        projectiles
            .iter()
            .chain(strikes.iter())
            .map(|(_, damage)| damage)
            .sum(),
    );
    // Return projectiles to their real pool; despawn would strand active slots.
    app.world_mut()
        .run_system_once(
            move |mut commands: Commands, mut pools: ResMut<CombatPools>| {
                for &(entity, _) in &projectiles {
                    pools.retire_projectile(&mut commands, entity);
                }
                for &(entity, _) in &strikes {
                    commands.entity(entity).despawn();
                }
            },
        )
        .unwrap();
    app.world_mut()
        .run_system_once(crate::game::pooling::finalize_recycles)
        .unwrap();
    assert_eq!(
        app.world_mut()
            .query::<&Projectile>()
            .iter(app.world())
            .count(),
        0,
        "retired shells must not retain Projectile"
    );
    assert_eq!(
        app.world_mut()
            .query::<&SkillStrike>()
            .iter(app.world())
            .count(),
        0
    );
    output
}
fn finite_step(app: &mut App, kind: FiniteSummon, delta: Duration) -> (usize, f64) {
    // Every previous output was drained, so all queried gameplay components
    // after this update belong to emissions from this step, including overflow.
    // ActivePooledProjectile is private to pooling; visibility would wrongly
    // exclude active logic-only overflow projectiles.
    assert_eq!(
        app.world_mut()
            .query::<&Projectile>()
            .iter(app.world())
            .count(),
        0
    );
    assert_eq!(
        app.world_mut()
            .query::<&SkillStrike>()
            .iter(app.world())
            .count(),
        0
    );
    app.world_mut().resource_mut::<Time>().advance_by(delta);
    kind.update(app);
    take_finite_output(app, kind)
}
fn finite_output(kind: FiniteSummon, step: Duration) -> (usize, f64) {
    let (mut app, summon) = finite_fixture(kind, 8.0);
    // Colossus only pulls distances >1.55; this target remains stationary.
    let target = enemy(&mut app, 0.5);
    let mut elapsed = Duration::ZERO;
    let total = Duration::from_secs(10);
    let mut result = (0, 0.0);
    while elapsed < total {
        let delta = step.min(total - elapsed);
        let output = finite_step(&mut app, kind, delta);
        result.0 += output.0;
        result.1 += output.1;
        assert_eq!(
            app.world().get::<Transform>(target).unwrap().translation,
            Vec3::X * 0.5
        );
        elapsed += delta;
    }
    assert!(
        app.world().get::<SkillSummon>(summon).is_none(),
        "{kind:?} must retire through the real pool"
    );
    assert_eq!(app.world().get::<Enemy>(target).unwrap().hp, 1000.0);
    assert_eq!(result.1, result.0 as f64 * 50.0);
    result
}
#[test]
fn finite_summons_preserve_output_across_32x_steps_and_lifetime_overshoot() {
    let mut discrepancies = Vec::new();
    for kind in FINITE_SUMMONS {
        let normal = finite_output(kind, Duration::from_secs_f64(1.0 / 60.0));
        assert!(
            normal.0 >= (8.0 / kind.interval()).floor() as usize,
            "{kind:?}: nearby durable target must receive attacks"
        );
        for step in [
            Duration::from_secs_f64(32.0 / 60.0),
            Duration::from_secs_f64(32.0 / 15.0),
            Duration::from_secs(10),
        ] {
            let accelerated = finite_output(kind, step);
            println!(
                "FINITE_SUMMON {kind:?} lifetime=8s step={step:?} normal={normal:?} accelerated={accelerated:?}; outgoing count/damage, no collision"
            );
            if normal.0.abs_diff(accelerated.0) > 1 || (normal.1 - accelerated.1).abs() > 50.0 {
                discrepancies.push(format!(
                    "{kind:?}: normal={normal:?}, accelerated={accelerated:?}, step={step:?}"
                ));
            }
        }
    }
    assert!(discrepancies.is_empty(), "{}", discrepancies.join("; "));
}
#[test]
fn finite_summons_cannot_bank_attacks_without_targets() {
    for kind in FINITE_SUMMONS {
        let (mut app, summon) = finite_fixture(kind, 100.0);
        assert_eq!(
            finite_step(&mut app, kind, Duration::from_secs(60)),
            (0, 0.0)
        );
        let position = app.world().get::<Transform>(summon).unwrap().translation;
        let target = enemy(&mut app, position.x + 0.5);
        app.world_mut()
            .get_mut::<Transform>(target)
            .unwrap()
            .translation = position + Vec3::X * 0.5;
        assert_eq!(
            finite_step(&mut app, kind, Duration::from_secs_f32(0.1)),
            (1, 50.0),
            "{kind:?}: idle time cannot produce a burst"
        );
    }
}
#[test]
fn mobile_finite_summons_cannot_spend_pursuit_time_on_attacks() {
    for (kind, speed, reach) in [
        (FiniteSummon::Tide, 3.5, 6.0),
        (FiniteSummon::Wyrm, 4.2, 7.0),
        (FiniteSummon::Colossus, 3.0, 2.6),
    ] {
        let (mut app, summon) = finite_fixture(kind, 100.0);
        // 1.5s travelling, then 0.5s in reach: only the ready attack is due.
        enemy(&mut app, reach + speed * 1.5);
        assert_eq!(
            finite_step(&mut app, kind, Duration::from_secs(2)),
            (1, 50.0),
            "{kind:?}: travel cannot bank attacks"
        );
        let timer = match kind {
            FiniteSummon::Tide => {
                app.world()
                    .get::<TideElemental>(summon)
                    .unwrap()
                    .attack_remaining
            }
            FiniteSummon::Wyrm => {
                app.world()
                    .get::<FrostWyrm>(summon)
                    .unwrap()
                    .attack_remaining
            }
            FiniteSummon::Colossus => {
                app.world()
                    .get::<AncestorColossus>(summon)
                    .unwrap()
                    .slam_remaining
            }
            FiniteSummon::Turret => unreachable!(),
        };
        assert!(
            (timer - (kind.interval() - 0.5)).abs() < 0.001,
            "{kind:?}: actual timer={timer}"
        );
    }
}
