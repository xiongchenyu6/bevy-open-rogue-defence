// Include inside combat.rs's existing tests module; not a parallel fixture app.
fn ranged_release_app(kind: HeroKind) -> (App, Entity) {
    let mut app = projectile_test_app();
    app.init_resource::<AttackClock>()
        .init_resource::<AttackVolleyRuntime>()
        .init_resource::<BasicAttackCounter>();
    app.world_mut().resource_mut::<HeroVitals>().kind = kind;
    app.world_mut().resource_mut::<HeroVitals>().level = 1;
    app.world_mut()
        .resource_mut::<PlayerBuild>()
        .attacks_per_second = 1.0;
    let hero = app.world_mut().spawn((Hero, Transform::default())).id();
    app.world_mut()
        .spawn((test_enemy(10000.), Transform::from_xyz(2., 0., 0.)));
    (app, hero)
}
fn ranged_active(app: &mut App) -> usize {
    app.world_mut()
        .query::<&Projectile>()
        .iter(app.world())
        .count()
}
fn commit_ranged(app: &mut App) {
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs_f32(1.));
    app.world_mut().run_system_once(auto_attack).unwrap();
}

// BEFORE baseline: this test uses existing production symbols only. Append this
// fixture and test unchanged before applying runtime patch; expected immediate
// projectile count is1, violating the desired zero-until-raised-phase contract.
#[test]
fn native_ranged_does_not_emit_at_attack_commit() {
    for kind in [HeroKind::EmberRanger, HeroKind::StormSavant] {
        let (mut app, _) = ranged_release_app(kind);
        commit_ranged(&mut app);
        assert_eq!(app.world().resource::<BasicAttackCounter>().0, 1);
        assert_eq!(
            app.world()
                .resource::<AttackVolleyRuntime>()
                .resolved_this_frame,
            1
        );
        assert_eq!(
            ranged_active(&mut app),
            0,
            "real native projectile emitted before candidate phase: {kind:?}"
        );
    }
}

#[test]
fn ranged_pending_releases_real_pool_once_and_deals_real_damage() {
    for kind in [HeroKind::EmberRanger, HeroKind::StormSavant] {
        let (mut app, _) = ranged_release_app(kind);
        commit_ranged(&mut app);
        let rng = app.world().resource::<RunRng>().state();
        app.world_mut()
            .run_system_once(ranged_release::release_pending)
            .unwrap();
        assert_eq!(ranged_active(&mut app), 0);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.2));
        app.world_mut()
            .run_system_once(ranged_release::release_pending)
            .unwrap();
        assert_eq!(ranged_active(&mut app), 1);
        assert_eq!(
            app.world().resource::<RunRng>().state(),
            rng,
            "release must not reroll a committed attack"
        );
        assert_eq!(app.world().resource::<BasicAttackCounter>().0, 1);
        app.world_mut().run_system_once(move_projectiles).unwrap();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.2));
        app.world_mut().run_system_once(move_projectiles).unwrap();
        app.world_mut().run_system_once(projectile_hits).unwrap();
        assert!(
            app.world_mut()
                .query::<&Enemy>()
                .iter(app.world())
                .any(|enemy| enemy.hp < 10000.)
        );
        assert_eq!(
            app.world_mut()
                .query::<&ranged_release::PendingRangedShot>()
                .iter(app.world())
                .count(),
            0
        );
    }
}

#[test]
fn ranged_pending_pause_freezes_and_invalid_owner_cancels() {
    for reason in 0..5 {
        let (mut app, hero) = ranged_release_app(HeroKind::EmberRanger);
        commit_ranged(&mut app);
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(1));
        app.world_mut()
            .run_system_once(ranged_release::release_pending)
            .unwrap();
        assert_eq!(ranged_active(&mut app), 0);
        assert_eq!(
            app.world_mut()
                .query::<&ranged_release::PendingRangedShot>()
                .iter(app.world())
                .count(),
            1
        );
        app.world_mut().resource_mut::<GameSession>().paused = false;
        match reason {
            0 => app.world_mut().resource_mut::<GameSession>().finished = true,
            1 => app.world_mut().resource_mut::<HeroVitals>().alive = false,
            2 => app.world_mut().resource_mut::<HeroVitals>().kind = HeroKind::StoneWarden,
            3 => {
                app.world_mut()
                    .resource_mut::<HeroVitals>()
                    .stunned_remaining = 1.
            }
            _ => {
                app.world_mut().despawn(hero);
            }
        }
        app.world_mut()
            .run_system_once(ranged_release::release_pending)
            .unwrap();
        assert_eq!(ranged_active(&mut app), 0);
        assert_eq!(
            app.world_mut()
                .query::<&ranged_release::PendingRangedShot>()
                .iter(app.world())
                .count(),
            0
        );
    }
}

#[test]
fn ranged_catchup_keeps_commit_count_and_releases_due_shots_once() {
    let (mut app, _) = ranged_release_app(HeroKind::StormSavant);
    app.world_mut()
        .resource_mut::<PlayerBuild>()
        .attacks_per_second = 20.;
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs_f32(0.2));
    app.world_mut().run_system_once(auto_attack).unwrap();
    let commits = app.world().resource::<BasicAttackCounter>().0;
    let rng = app.world().resource::<RunRng>().state();
    assert!(commits > 1);
    app.world_mut()
        .run_system_once(ranged_release::release_pending)
        .unwrap();
    let pending = app
        .world_mut()
        .query::<&ranged_release::PendingRangedShot>()
        .iter(app.world())
        .count();
    assert_eq!(ranged_active(&mut app) + pending, commits as usize);
    assert!(
        ranged_active(&mut app) > 0,
        "older completions must not acquire an extra whole-frame delay"
    );
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs_f32(0.1));
    app.world_mut()
        .run_system_once(ranged_release::release_pending)
        .unwrap();
    assert_eq!(ranged_active(&mut app), commits as usize);
    assert_eq!(app.world().resource::<BasicAttackCounter>().0, commits);
    assert_eq!(app.world().resource::<RunRng>().state(), rng);
}

#[test]
fn ranged_pending_resumes_once_without_consuming_paused_time() {
    for kind in [HeroKind::EmberRanger, HeroKind::StormSavant] {
        let (mut app, _) = ranged_release_app(kind);
        commit_ranged(&mut app);
        app.world_mut()
            .run_system_once(ranged_release::release_pending)
            .unwrap();
        let rng = app.world().resource::<RunRng>().state();
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(8));
        app.world_mut()
            .run_system_once(ranged_release::release_pending)
            .unwrap();
        assert_eq!(ranged_active(&mut app), 0);
        app.world_mut().resource_mut::<GameSession>().paused = false;
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(50));
        app.world_mut()
            .run_system_once(ranged_release::release_pending)
            .unwrap();
        assert_eq!(
            ranged_active(&mut app),
            0,
            "paused time cannot make launch overdue"
        );
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(100));
        app.world_mut()
            .run_system_once(ranged_release::release_pending)
            .unwrap();
        assert_eq!(ranged_active(&mut app), 1);
        app.world_mut()
            .run_system_once(ranged_release::release_pending)
            .unwrap();
        assert_eq!(ranged_active(&mut app), 1, "resumption cannot double emit");
        assert_eq!(app.world().resource::<BasicAttackCounter>().0, 1);
        assert_eq!(app.world().resource::<RunRng>().state(), rng);
    }
}

#[test]
fn ranged_real_virtual_1x_8x_catchup_preserves_commits_and_pending_accounting() {
    for kind in [HeroKind::EmberRanger, HeroKind::StormSavant] {
        let outcomes = [(1., 32), (8., 4)].map(|(speed, frames)| {
            let (mut app, _) = ranged_release_app(kind);
            app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_millis(25),
            ));
            // Initialize actual TimePlugin once before registering attack systems.
            app.update();
            app.world_mut()
                .resource_mut::<Time<Virtual>>()
                .set_relative_speed(speed);
            app.world_mut()
                .resource_mut::<PlayerBuild>()
                .attacks_per_second = 20.;
            app.add_systems(
                Update,
                (auto_attack, ranged_release::release_pending).chain(),
            );
            for _ in 0..frames {
                app.update();
                let committed = app.world().resource::<BasicAttackCounter>().0 as usize;
                let waiting = app
                    .world_mut()
                    .query::<&ranged_release::PendingRangedShot>()
                    .iter(app.world())
                    .count();
                assert_eq!(ranged_active(&mut app) + waiting, committed);
            }
            let committed = app.world().resource::<BasicAttackCounter>().0;
            assert!(committed > 1);
            assert!(ranged_active(&mut app) > 0);
            (committed, app.world().resource::<RunRng>().state())
        });
        assert_eq!(
            outcomes[0], outcomes[1],
            "equal virtual duration must preserve commit/RNG cadence: {kind:?}"
        );
    }
}
