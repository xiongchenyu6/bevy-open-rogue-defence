//! Compare real outgoing contact strikes under equal simulated time, without
//! target movement, target attacks, collision, skill casts or summon expiry.
use super::{
    tests::{app, enemy},
    *,
};
use bevy::ecs::system::RunSystemOnce;
use std::time::Duration;

fn sentry_output(step: Duration) -> (usize, f64) {
    let mut app = app();
    app.init_resource::<HeroVitals>();
    app.world_mut().resource_mut::<PlayerBuild>().skills[4] = 1;
    let target = enemy(&mut app, 0.5);
    let sentry = app
        .world_mut()
        .spawn((
            SkillSummon,
            FormationSlot(0),
            BoneSentry {
                attack_remaining: 0.0,
                damage: 50.0,
            },
            Transform::default(),
        ))
        .id();
    // BoneSentry is persistent (has no expiry timer), so it survives all 60 s.
    let initial_vitals = app.world().resource::<HeroVitals>().clone();
    let total = Duration::from_secs(60);
    let mut elapsed = Duration::ZERO;
    let mut shots = 0usize;
    let mut damage = 0.0f64;
    while elapsed < total {
        let delta = step.min(total - elapsed);
        app.world_mut().resource_mut::<Time>().advance_by(delta);
        app.world_mut()
            .run_system_once(update_skill_summons)
            .unwrap();
        let strikes: Vec<_> = app
            .world_mut()
            .query::<(Entity, &SkillStrike)>()
            .iter(app.world())
            .map(|(entity, strike)| {
                assert_eq!(strike.target, target);
                assert_eq!(strike.profile.skill_index, Some(4));
                assert_eq!(strike.damage, 50.0);
                assert_eq!(strike.delay, 0.0);
                (entity, f64::from(strike.damage))
            })
            .collect();
        for (entity, amount) in strikes {
            shots += 1;
            damage += amount;
            app.world_mut().despawn(entity);
        }
        elapsed += delta;
    }
    assert_eq!(elapsed, total);
    assert_eq!(app.world().resource::<Time>().elapsed(), total);
    assert_eq!(
        app.world().get::<Transform>(sentry).unwrap().translation,
        Vec3::ZERO
    );
    assert_eq!(
        app.world().get::<Transform>(target).unwrap().translation,
        Vec3::X * 0.5
    );
    assert_eq!(app.world().get::<Enemy>(target).unwrap().hp, 1000.0);
    let vitals = app.world().resource::<HeroVitals>();
    assert_eq!(
        (vitals.kind, vitals.level, vitals.health),
        (
            initial_vitals.kind,
            initial_vitals.level,
            initial_vitals.health
        )
    );
    assert_eq!(app.world().resource::<PlayerBuild>().skills[4], 1);
    assert_eq!(damage, shots as f64 * 50.0);
    (shots, damage)
}

#[test]
fn bone_sentry_equal_simulated_time_preserves_contact_cadence_at_32x() {
    let normal = sentry_output(Duration::from_secs_f64(1.0 / 60.0));
    let fast = sentry_output(Duration::from_secs_f64(32.0 / 60.0));
    let slow_render = sentry_output(Duration::from_secs_f64(32.0 / 15.0));
    println!(
        "BONE_SENTRY_EQUAL_60S normal={normal:?} speed32_60fps={fast:?} speed32_15fps={slow_render:?}; outputs=(actual SkillStrike count, summed outgoing damage), no collision or HP damage"
    );
    assert!(
        normal.0 >= 60,
        "durable nearby target must receive real scheduled contacts"
    );
    for (label, output) in [
        ("32x at 60 render FPS", fast),
        ("32x at 15 render FPS", slow_render),
    ] {
        assert!(
            normal.0.abs_diff(output.0) <= 1 && (normal.1 - output.1).abs() <= 50.0,
            "{label}: equal 60 simulated seconds should preserve BoneSentry cadence within one contact; normal={normal:?}, accelerated={output:?}"
        );
    }
}

fn ready_sentry(app: &mut App) -> Entity {
    app.init_resource::<HeroVitals>();
    app.world_mut().resource_mut::<PlayerBuild>().skills[4] = 1;
    app.world_mut()
        .spawn((
            SkillSummon,
            FormationSlot(0),
            BoneSentry {
                attack_remaining: 0.0,
                damage: 50.0,
            },
            Transform::default(),
        ))
        .id()
}

#[test]
fn bone_sentry_idle_time_does_not_bank_contacts() {
    let mut app = app();
    let sentry = ready_sentry(&mut app);
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_secs(60));
    app.world_mut()
        .run_system_once(update_skill_summons)
        .unwrap();
    assert_eq!(
        app.world_mut()
            .query::<&SkillStrike>()
            .iter(app.world())
            .count(),
        0
    );
    let position = app.world().get::<Transform>(sentry).unwrap().translation;
    let target = enemy(&mut app, position.x + 0.5);
    app.world_mut()
        .get_mut::<Transform>(target)
        .unwrap()
        .translation
        .z = position.z;
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_secs_f32(0.1));
    app.world_mut()
        .run_system_once(update_skill_summons)
        .unwrap();
    assert_eq!(
        app.world_mut()
            .query::<&SkillStrike>()
            .iter(app.world())
            .count(),
        1
    );
}

#[test]
fn bone_sentry_pursuit_time_cannot_be_spent_as_contact_time() {
    let mut app = app();
    let sentry = ready_sentry(&mut app);
    enemy(&mut app, 6.8);
    // Reach 1.1 at speed3.8 takes1.5s; only0.5s remains in contact range.
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_secs(2));
    app.world_mut()
        .run_system_once(update_skill_summons)
        .unwrap();
    assert_eq!(
        app.world_mut()
            .query::<&SkillStrike>()
            .iter(app.world())
            .count(),
        1
    );
    assert!(
        (app.world()
            .get::<BoneSentry>(sentry)
            .unwrap()
            .attack_remaining
            - 0.4)
            .abs()
            < 0.001
    );
}

fn hound_output(step: Duration) -> (usize, f64) {
    let mut app = app();
    app.world_mut().resource_mut::<PlayerBuild>().skills[5] = 1;
    let target = enemy(&mut app, 0.5);
    let hound = app
        .world_mut()
        .spawn((
            SkillSummon,
            FormationSlot(0),
            SpiritHound {
                source_skill: 5,
                remaining: 8.0,
                attack_remaining: 0.0,
                damage: 50.0,
            },
            Transform::default(),
        ))
        .id();
    let total = Duration::from_secs(10);
    let mut elapsed = Duration::ZERO;
    let mut count = 0;
    let mut damage = 0.0;
    while elapsed < total {
        let delta = step.min(total - elapsed);
        app.world_mut().resource_mut::<Time>().advance_by(delta);
        app.world_mut()
            .run_system_once(update_skill_summons)
            .unwrap();
        let strikes: Vec<_> = app
            .world_mut()
            .query::<(Entity, &SkillStrike)>()
            .iter(app.world())
            .map(|(entity, strike)| {
                assert_eq!(strike.target, target);
                assert_eq!(strike.profile.skill_index, Some(5));
                (entity, f64::from(strike.damage))
            })
            .collect();
        for (entity, amount) in strikes {
            count += 1;
            damage += amount;
            app.world_mut().despawn(entity);
        }
        elapsed += delta;
    }
    assert!(app.world().get::<SpiritHound>(hound).unwrap().remaining <= 0.0);
    assert_eq!(app.world().get::<Enemy>(target).unwrap().hp, 1000.0);
    (count, damage)
}

#[test]
fn hound_equal_lifetime_preserves_contacts_at_accelerated_steps() {
    let normal = hound_output(Duration::from_secs_f64(1.0 / 60.0));
    let fast = hound_output(Duration::from_secs_f64(32.0 / 60.0));
    let slow = hound_output(Duration::from_secs_f64(32.0 / 15.0));
    println!(
        "HOUND_EQUAL_8S_LIFETIME normal={normal:?} fast={fast:?} slow={slow:?}; outgoing contacts, not HP damage"
    );
    assert!(normal.0 >= 10);
    let overshot = hound_output(Duration::from_secs(10));
    for output in [fast, slow, overshot] {
        assert!(
            normal.0.abs_diff(output.0) <= 1 && (normal.1 - output.1).abs() <= 50.0,
            "finite lifetime must preserve due contacts: normal={normal:?}, accelerated={output:?}"
        );
    }
}
