//! Production runtime control-time accounting with manually advanced real Time.
use super::*;
use std::time::Duration;

fn fixture(freeze: f32, stun: f32) -> (App, Entity) {
    let mut app = App::new();
    app.init_resource::<Time>()
        .init_resource::<PlayerBuild>()
        .init_resource::<GameSession>()
        .init_resource::<MarketEffects>()
        .init_resource::<super::super::encounters::EncounterState>()
        .init_resource::<SkillRuntime>()
        .add_systems(Update, tick_skill_runtime);
    let target = app
        .world_mut()
        .spawn((
            Enemy {
                hp: 100.,
                max_hp: 100.,
                speed: 2.,
                damage: 1.,
                xp: 0,
                wood: 0,
                gold: 0,
                kill_credit: 0,
                elite: false,
            },
            Transform::default(),
        ))
        .id();
    app.world_mut()
        .resource_mut::<SkillRuntime>()
        .controls
        .insert(
            target,
            ControlState {
                base_speed: 2.,
                slow_remaining: 0.,
                frozen_remaining: freeze,
                stunned_remaining: stun,
            },
        );
    (app, target)
}
fn step(app: &mut App, seconds: f32) {
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_secs_f32(seconds));
    app.update();
}
#[test]
fn control_time_expiry_preserves_spent_time_for_every_consumer_only_this_step() {
    let (mut app, target) = fixture(0.1, 0.05);
    step(&mut app, 32. / 60.);
    let runtime = app.world().resource::<SkillRuntime>();
    assert!(!runtime.actions_blocked(target));
    for _ in 0..3 {
        assert!((runtime.action_delta(target, 32. / 60.) - (32. / 60. - 0.1)).abs() < 0.00001);
    }
    assert_eq!(app.world().get::<Enemy>(target).unwrap().speed, 2.);
    step(&mut app, 32. / 60.);
    assert!(
        (app.world()
            .resource::<SkillRuntime>()
            .action_delta(target, 32. / 60.)
            - 32. / 60.)
            .abs()
            < 0.00001
    );
}
#[test]
fn control_time_overlap_spends_maximum_duration_across_small_and_large_steps() {
    for step_size in [1.0_f32 / 60., 32. / 60., 32. / 15., 1.6] {
        let (mut app, target) = fixture(0.75, 1.2);
        let mut elapsed: f32 = 0.;
        let mut available = 0.;
        while elapsed < 1.6 - 0.000001 {
            let delta = step_size.min(1.6 - elapsed);
            step(&mut app, delta);
            available += app
                .world()
                .resource::<SkillRuntime>()
                .action_delta(target, delta);
            elapsed += delta;
        }
        assert!(
            (available - 0.4).abs() < 0.00002,
            "step={step_size} available={available}"
        );
    }
}
#[test]
fn control_time_new_freeze_pause_removal_and_reset_do_not_leak_a_budget() {
    let (mut app, target) = fixture(0.1, 0.);
    step(&mut app, 0.5);
    assert!(
        (app.world()
            .resource::<SkillRuntime>()
            .action_delta(target, 0.5)
            - 0.4)
            .abs()
            < 0.00001
    );
    app.world_mut()
        .resource_mut::<SkillRuntime>()
        .freeze_relic(target, 2., false, false, 1.);
    assert_eq!(
        app.world()
            .resource::<SkillRuntime>()
            .action_delta(target, 0.5),
        0.
    );
    app.world_mut().resource_mut::<GameSession>().paused = true;
    step(&mut app, 0.5);
    assert_eq!(
        app.world().resource::<SkillRuntime>().controls[&target].frozen_remaining,
        1.
    );
    assert!(
        app.world()
            .resource::<SkillRuntime>()
            .control_blocked_seconds
            .is_empty()
    );
    app.world_mut().resource_mut::<GameSession>().paused = false;
    app.world_mut().despawn(target);
    step(&mut app, 0.5);
    assert!(
        !app.world()
            .resource::<SkillRuntime>()
            .controls
            .contains_key(&target)
    );
    assert!(
        app.world()
            .resource::<SkillRuntime>()
            .control_blocked_seconds
            .is_empty()
    );
    app.world_mut().insert_resource(SkillRuntime::default());
    assert_eq!(
        app.world()
            .resource::<SkillRuntime>()
            .action_delta(target, 0.5),
        0.5
    );
}
