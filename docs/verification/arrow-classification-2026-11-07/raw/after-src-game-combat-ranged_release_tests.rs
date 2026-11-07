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
    // Advance the real producer without a volley, then commit on a small
    // action frame. A one-second frame would already complete the windup.
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_millis(975));
    app.world_mut().run_system_once(auto_attack).unwrap();
    assert_eq!(app.world().resource::<BasicAttackCounter>().0, 0);
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_millis(25));
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
        app.world_mut()
            .resource_mut::<PlayerBuild>()
            .projectile_speed = 10.;
        let near = app
            .world_mut()
            .query_filtered::<Entity, With<Enemy>>()
            .single(app.world())
            .unwrap();
        // This control is an ordinary target; the shared helper defaults to
        // elite, whose larger collision radius legitimately intercepts earlier.
        {
            let mut enemy = app.world_mut().get_mut::<Enemy>(near).unwrap();
            enemy.elite = false;
            enemy.max_hp = 10000.;
        }
        let far = app
            .world_mut()
            .spawn((test_enemy(10000.), Transform::from_xyz(20., 0., 0.)))
            .id();
        commit_ranged(&mut app);
        let rng = app.world().resource::<RunRng>().state();
        app.world_mut()
            .run_system_once(ranged_release::release_pending)
            .unwrap();
        assert_eq!(ranged_active(&mut app), 0);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.15));
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
        // Real frame order consumes FirstFrameTravel in collision resolution,
        // even when the initial partial travel has not reached any target.
        let carrier = app
            .world_mut()
            .query_filtered::<Entity, With<Projectile>>()
            .single(app.world())
            .unwrap();
        assert!(
            app.world()
                .get::<ProjectileFirstFrameTravel>(carrier)
                .unwrap()
                .0
                < 0.1
        );
        app.world_mut().run_system_once(move_projectiles).unwrap();
        app.world_mut().run_system_once(projectile_hits).unwrap();
        assert_eq!(app.world().get::<Enemy>(near).unwrap().hp, 10000.);
        assert!(
            app.world()
                .get::<ProjectileFirstFrameTravel>(carrier)
                .is_none()
        );
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.2));
        app.world_mut().run_system_once(move_projectiles).unwrap();
        app.world_mut().run_system_once(projectile_hits).unwrap();
        assert!(app.world().get::<Enemy>(near).unwrap().hp < 10000.);
        assert_eq!(app.world().get::<Enemy>(far).unwrap().hp, 10000.);
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

#[test]
fn ranged_skill_runtime_freeze_cancels_pending_even_when_vitals_can_act() {
    let (mut app, hero) = ranged_release_app(HeroKind::StormSavant);
    commit_ranged(&mut app);
    app.world_mut()
        .resource_mut::<SkillRuntime>()
        .freeze_relic(hero, 1., false, false, 1.);
    assert!(app.world().resource::<HeroVitals>().can_act());
    assert!(app.world().resource::<SkillRuntime>().actions_blocked(hero));
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

#[test]
fn ranged_delayed_catchup_uses_distinct_partial_travel_then_full_next_frame() {
    let (mut app, _) = ranged_release_app(HeroKind::StormSavant);
    app.world_mut()
        .resource_mut::<PlayerBuild>()
        .attacks_per_second = 20.;
    app.world_mut()
        .resource_mut::<PlayerBuild>()
        .projectile_speed = 10.;
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_millis(200));
    app.world_mut().run_system_once(auto_attack).unwrap();
    app.world_mut()
        .run_system_once(ranged_release::release_pending)
        .unwrap();
    let shots: Vec<_> = app
        .world_mut()
        .query::<(Entity, &Projectile, &ProjectileFirstFrameTravel)>()
        .iter(app.world())
        .map(|(e, p, t)| (e, p.velocity, t.0))
        .collect();
    assert!(shots.len() > 1);
    let mut travel: Vec<_> = shots.iter().map(|(_, _, t)| t.to_bits()).collect();
    travel.sort_unstable();
    travel.dedup();
    assert_eq!(
        travel.len(),
        shots.len(),
        "different completion ages retain distinct travel"
    );
    // Preserve the real emissions but move the fixture target out of the sweep,
    // so marker consumption is tested independently from projectile retirement.
    for mut transform in app
        .world_mut()
        .query_filtered::<&mut Transform, With<Enemy>>()
        .iter_mut(app.world_mut())
    {
        transform.translation.x = 50.;
    }
    app.world_mut().run_system_once(move_projectiles).unwrap();
    app.world_mut().run_system_once(projectile_hits).unwrap();
    for (e, v, t) in &shots {
        assert!(
            (app.world().get::<Transform>(*e).unwrap().translation.x - v.x * t).abs() < 0.00001
        );
        assert!(app.world().get::<ProjectileFirstFrameTravel>(*e).is_none());
    }
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_millis(10));
    app.world_mut().run_system_once(move_projectiles).unwrap();
    app.world_mut().run_system_once(projectile_hits).unwrap();
    for (e, v, t) in shots {
        assert!(
            (app.world().get::<Transform>(e).unwrap().translation.x - v.x * (t + 0.01)).abs()
                < 0.00001
        );
    }
}

#[test]
fn ranged_real_release_preserves_committed_metadata_and_starter_boundaries() {
    for kind in [HeroKind::EmberRanger, HeroKind::StormSavant] {
        for level in [1, 10] {
            let (mut app, _) = ranged_release_app(kind);
            app.world_mut().resource_mut::<HeroVitals>().level = level;
            {
                let mut build = app.world_mut().resource_mut::<PlayerBuild>();
                build.attack_damage = 73.;
                build.projectile_speed = 11.;
                build.pierce = 2;
                build.ricochet = 1;
            }
            commit_ranged(&mut app);
            app.world_mut()
                .run_system_once(ranged_release::release_pending)
                .unwrap();
            // Changes after commit must not rewrite the pending attack's data.
            app.world_mut().resource_mut::<PlayerBuild>().attack_damage = 999.;
            app.world_mut()
                .resource_mut::<PlayerBuild>()
                .projectile_speed = 99.;
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_millis(200));
            app.world_mut()
                .run_system_once(ranged_release::release_pending)
                .unwrap();
            let entity = app
                .world_mut()
                .query_filtered::<Entity, With<Projectile>>()
                .single(app.world())
                .unwrap();
            let projectile = app.world().get::<Projectile>(entity).unwrap();
            assert_eq!(
                (
                    projectile.damage,
                    projectile.radius,
                    projectile.remaining_hits,
                    projectile.lifetime
                ),
                (73., 0.48, 2, 1.4)
            );
            assert!((projectile.velocity.length() - 11.).abs() < 0.00001);
            assert_eq!(
                app.world().get::<DamageProfile>(entity),
                Some(
                    &if basic_attacks::weapon(kind) == basic_attacks::BasicWeapon::Crossbow {
                        DamageProfile::PHYSICAL_ATTACK.as_arrow()
                    } else {
                        DamageProfile::PHYSICAL_ATTACK
                    }
                )
            );
            assert_eq!(app.world().get::<WeaponFlight>(entity).unwrap().kind, kind);
            assert_eq!(
                app.world()
                    .get::<super::super::skill_lighting::NoGroundProjection>(entity)
                    .is_some(),
                kind == HeroKind::EmberRanger && level == 1
            );
            let bounce = app.world().get::<basic_attacks::Ricochet>(entity);
            if kind == HeroKind::EmberRanger {
                assert_eq!(bounce.unwrap().remaining, if level == 1 { 1 } else { 2 });
            } else {
                assert!(bounce.is_none());
            }
            assert!(
                app.world()
                    .get::<ProjectileFirstFrameTravel>(entity)
                    .is_some()
            );
        }
    }
}

#[test]
fn ranged_regular_action_age_is_independent_of_timer_completion_remainder() {
    for (kind, expected_release) in [
        (HeroKind::EmberRanger, 0.1125_f32),
        (HeroKind::StormSavant, 0.126_f32),
    ] {
        for precharge in [975, 980, 990] {
            let (mut app, _) = ranged_release_app(kind);
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_millis(precharge));
            app.world_mut().run_system_once(auto_attack).unwrap();
            assert_eq!(app.world().resource::<BasicAttackCounter>().0, 0);
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_millis(25));
            app.world_mut().run_system_once(auto_attack).unwrap();
            assert_eq!(app.world().resource::<BasicAttackCounter>().0, 1);
            app.world_mut()
                .run_system_once(ranged_release::release_pending)
                .unwrap();
            assert_eq!(ranged_active(&mut app), 0);
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_millis(150));
            app.world_mut()
                .run_system_once(ranged_release::release_pending)
                .unwrap();
            let travel = app
                .world_mut()
                .query::<&ProjectileFirstFrameTravel>()
                .single(app.world())
                .unwrap()
                .0;
            let release_after_action_start = 0.025 + 0.15 - travel;
            assert!(
                (release_after_action_start - expected_release).abs() < 0.000001,
                "{kind:?} precharge={precharge}: actual action release {release_after_action_start}"
            );
        }
    }
}
