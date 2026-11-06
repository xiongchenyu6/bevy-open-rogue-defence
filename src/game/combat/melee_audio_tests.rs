use super::*;

#[test]
fn real_melee_producer_emits_one_precontact_whoosh() {
    let mut app = projectile_test_app();
    crate::game::audio::tests::install_feedback(&mut app);
    app.init_resource::<AttackClock>()
        .init_resource::<AttackVolleyRuntime>()
        .init_resource::<BasicAttackCounter>();
    app.world_mut().resource_mut::<HeroVitals>().kind = HeroKind::StoneWarden;
    app.world_mut()
        .resource_mut::<PlayerBuild>()
        .attacks_per_second = 1.0;
    app.world_mut().spawn((Hero, Transform::default()));
    let enemy = app
        .world_mut()
        .spawn((test_enemy(100.0), Transform::from_xyz(1.5, 0.0, 0.0)))
        .id();
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs(1));
    app.world_mut().run_system_once(auto_attack).unwrap();
    assert_eq!(app.world().resource::<BasicAttackCounter>().0, 1);
    assert_eq!(
        app.world_mut()
            .query::<&PendingSwing>()
            .iter(app.world())
            .count(),
        1
    );
    // Real audio Update consumer, before resolving this pending contact.
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_millis(230));
    app.world_mut().run_schedule(Update);
    assert_eq!(app.world().get::<Enemy>(enemy).unwrap().hp, 100.0);
    assert_eq!(
        app.world_mut()
            .query::<&bevy::audio::AudioPlayer>()
            .iter(app.world())
            .count(),
        1,
        "actual pending melee attack must have a precontact swing voice"
    );
}

fn produced_whoosh_app(kind: HeroKind) -> (App, Entity, Entity) {
    let mut app = projectile_test_app();
    crate::game::audio::tests::install_feedback(&mut app);
    app.init_resource::<AttackClock>()
        .init_resource::<AttackVolleyRuntime>()
        .init_resource::<BasicAttackCounter>();
    app.world_mut().resource_mut::<HeroVitals>().kind = kind;
    app.world_mut()
        .resource_mut::<PlayerBuild>()
        .attacks_per_second = 1.0;
    let hero = app.world_mut().spawn((Hero, Transform::default())).id();
    let target = app
        .world_mut()
        .spawn((test_enemy(100.0), Transform::from_xyz(1.3, 0.0, 0.0)))
        .id();
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs(1));
    app.world_mut().run_system_once(auto_attack).unwrap();
    assert_eq!(app.world().resource::<BasicAttackCounter>().0, 1);
    (app, hero, target)
}

fn whoosh_frame(app: &mut App, dt: f32, speed: f32) {
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs_f32(dt));
    app.world_mut()
        .resource_mut::<Time<Real>>()
        .advance_by(std::time::Duration::from_secs_f32(dt / speed));
    app.world_mut()
        .resource_mut::<Time<Virtual>>()
        .set_relative_speed(speed);
    app.world_mut()
        .run_system_once(crate::game::audio::play_melee_swings)
        .unwrap();
    crate::game::audio::tests::drain_feedback(app);
}

fn whoosh_count(app: &mut App) -> usize {
    app.world_mut()
        .query::<&Name>()
        .iter(app.world())
        .filter(|n| n.as_str() == "Melee swing whoosh")
        .count()
}

#[test]
fn melee_whoosh_waits_for_contact_lead_and_does_not_repeat_per_target() {
    for kind in [
        HeroKind::StoneWarden,
        HeroKind::IronBerserker,
        HeroKind::RiftDancer,
    ] {
        let (mut app, _, target) = produced_whoosh_app(kind);
        let second = app
            .world_mut()
            .spawn((test_enemy(100.0), Transform::from_xyz(1.2, 0.0, 0.15)))
            .id();
        for mut swing in app
            .world_mut()
            .query::<&mut PendingSwing>()
            .iter_mut(app.world_mut())
        {
            swing.hit_limit = 3;
        }
        whoosh_frame(&mut app, 0.001, 1.0);
        assert_eq!(
            whoosh_count(&mut app),
            0,
            "no sound immediately at long windup start"
        );
        let mut sounded = false;
        for _ in 0..65 {
            whoosh_frame(&mut app, 0.005, 1.0);
            if whoosh_count(&mut app) > 0 && !sounded {
                assert_eq!(app.world().get::<Enemy>(target).unwrap().hp, 100.0);
                assert_eq!(app.world().get::<Enemy>(second).unwrap().hp, 100.0);
                let remaining = app
                    .world_mut()
                    .query::<&PendingSwing>()
                    .single(app.world())
                    .unwrap()
                    .remaining;
                assert!(remaining > 0.005, "ordinary 1x whoosh precedes contact");
                sounded = true;
            }
            app.world_mut()
                .run_system_once(basic_attacks::resolve_swings)
                .unwrap();
            app.world_mut().run_system_once(projectile_hits).unwrap();
        }
        assert!(sounded);
        assert_eq!(
            whoosh_count(&mut app),
            1,
            "multi-target contact and later updates must not duplicate swing"
        );
        assert!(app.world().get::<Enemy>(target).unwrap().hp < 100.0);
        assert!(app.world().get::<Enemy>(second).unwrap().hp < 100.0);
    }
}

#[test]
fn melee_whoosh_survives_same_frame_short_windup_and_real_miss() {
    for miss in [false, true] {
        let (mut app, _, target) = produced_whoosh_app(HeroKind::StoneWarden);
        if miss {
            app.world_mut()
                .get_mut::<Transform>(target)
                .unwrap()
                .translation
                .x = 20.0;
        }
        whoosh_frame(&mut app, 0.5, 32.0);
        assert_eq!(whoosh_count(&mut app), 1);
        app.world_mut()
            .run_system_once(basic_attacks::resolve_swings)
            .unwrap();
        app.world_mut().run_system_once(projectile_hits).unwrap();
        assert_eq!(
            app.world_mut()
                .query::<&PendingSwing>()
                .iter(app.world())
                .count(),
            0
        );
        assert_eq!(app.world().get::<Enemy>(target).unwrap().hp == 100.0, miss);
        whoosh_frame(&mut app, 0.5, 32.0);
        assert_eq!(whoosh_count(&mut app), 1);
    }
}

#[test]
fn melee_whoosh_cancel_and_suppressed_windows_never_replay() {
    for mode in 0..6 {
        let (mut app, hero, _) = produced_whoosh_app(HeroKind::StoneWarden);
        match mode {
            0 => {
                app.world_mut().entity_mut(hero).despawn();
            }
            1 => {
                app.world_mut().resource_mut::<HeroVitals>().kind = HeroKind::StormSavant;
            }
            2 => {
                app.world_mut().resource_mut::<GameSession>().finished = true;
            }
            3 => {
                app.insert_resource(crate::game::Autoplay);
            }
            4 => {
                app.init_resource::<bevy::audio::AudioOutputStatus>();
            }
            _ => {
                app.world_mut().resource_mut::<GameSession>().paused = true;
                for mut swing in app
                    .world_mut()
                    .query::<&mut PendingSwing>()
                    .iter_mut(app.world_mut())
                {
                    swing.remaining = 0.01;
                }
            }
        }
        whoosh_frame(&mut app, 0.5, 1.0);
        assert_eq!(whoosh_count(&mut app), 0);
        app.world_mut().resource_mut::<GameSession>().paused = false;
        app.world_mut().resource_mut::<GameSession>().finished = false;
        app.world_mut().resource_mut::<HeroVitals>().kind = HeroKind::StoneWarden;
        app.world_mut().remove_resource::<crate::game::Autoplay>();
        app.world_mut()
            .remove_resource::<bevy::audio::AudioOutputStatus>();
        whoosh_frame(&mut app, 0.5, 1.0);
        assert_eq!(
            whoosh_count(&mut app),
            0,
            "suppressed/canceled swing cannot replay on recovery"
        );
    }
}

#[test]
fn melee_whoosh_preserves_mixed_fusion_contact_order_health_and_rng() {
    fn exercise(audio: bool) -> (Vec<(Entity, f32)>, Vec<f32>, u64) {
        let (mut app, hero, first) = produced_whoosh_app(HeroKind::StoneWarden);
        let second = app
            .world_mut()
            .spawn((test_enemy(1000.0), Transform::from_xyz(1.1, 0.0, 0.2)))
            .id();
        app.world_mut().get_mut::<Enemy>(first).unwrap().hp = 1000.0;
        for (kind, target, damage) in [
            (HeroKind::RiftDancer, second, 13.0),
            (HeroKind::IronBerserker, first, 27.0),
        ] {
            app.world_mut().spawn((
                crate::game::hero_fusion::FusionAttack {
                    kind,
                    origin: Vec3::ZERO,
                },
                PendingSwing {
                    whoosh_consumed: false,
                    hero,
                    target,
                    kind,
                    direction: Vec3::X,
                    damage,
                    hit_limit: 2,
                    remaining: 0.01,
                },
            ));
        }
        let before: Vec<_> = app
            .world_mut()
            .query::<(Entity, &PendingSwing)>()
            .iter(app.world())
            .map(|(e, _)| (e, app.world().entity(e).archetype().id()))
            .collect();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(500));
        if audio {
            app.world_mut()
                .run_system_once(crate::game::audio::play_melee_swings)
                .unwrap();
        }
        let after: Vec<_> = app
            .world_mut()
            .query::<(Entity, &PendingSwing)>()
            .iter(app.world())
            .map(|(e, _)| (e, app.world().entity(e).archetype().id()))
            .collect();
        assert_eq!(
            before, after,
            "audio state must not migrate archetypes or reorder attacks"
        );
        app.world_mut()
            .run_system_once(basic_attacks::resolve_swings)
            .unwrap();
        let contacts: Vec<_> = app
            .world_mut()
            .query::<(&Projectile, &ProjectileTarget)>()
            .iter(app.world())
            .map(|(p, target)| (target.0, p.damage))
            .collect();
        app.world_mut().run_system_once(projectile_hits).unwrap();
        let mut rounds = contacts;
        if audio {
            crate::game::audio::tests::drain_feedback(&mut app);
        }
        app.world_mut().resource_mut::<AttackClock>().0.reset();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(1));
        app.world_mut().run_system_once(auto_attack).unwrap();
        if audio {
            app.world_mut()
                .run_system_once(crate::game::audio::play_melee_swings)
                .unwrap();
        }
        app.world_mut()
            .run_system_once(basic_attacks::resolve_swings)
            .unwrap();
        rounds.extend(
            app.world_mut()
                .query::<(&Projectile, &ProjectileTarget)>()
                .iter(app.world())
                .map(|(p, t)| (t.0, p.damage)),
        );
        app.world_mut().run_system_once(projectile_hits).unwrap();
        if audio {
            crate::game::audio::tests::drain_feedback(&mut app);
        }
        let hp = [first, second]
            .map(|e| app.world().get::<Enemy>(e).unwrap().hp)
            .to_vec();
        (rounds, hp, app.world().resource::<RunRng>().0)
    }
    assert_eq!(
        exercise(false),
        exercise(true),
        "audio must preserve real contact order, HP and RNG state"
    );
}

#[test]
fn melee_whoosh_fixed_attack_has_same_contact_at_30_60_120_hz_and_speed_lead() {
    for speed in [1.0, 2.0] {
        let mut outcomes = Vec::new();
        for hz in [30.0, 60.0, 120.0] {
            let (mut app, _, target) = produced_whoosh_app(HeroKind::StoneWarden);
            let mut lead = None;
            for _ in 0..120 {
                whoosh_frame(&mut app, speed / hz, speed);
                if whoosh_count(&mut app) > 0 && lead.is_none() {
                    let remaining = app
                        .world_mut()
                        .query::<&PendingSwing>()
                        .single(app.world())
                        .unwrap()
                        .remaining;
                    lead = Some(remaining / speed);
                }
                app.world_mut()
                    .run_system_once(basic_attacks::resolve_swings)
                    .unwrap();
                app.world_mut().run_system_once(projectile_hits).unwrap();
            }
            let lead = lead.unwrap();
            assert!(
                (0.050..=0.050 + 1.0 / hz + 0.0001).contains(&lead),
                "real lead must follow candidate peak, with one-frame quantization: {lead}"
            );
            assert_eq!(whoosh_count(&mut app), 1);
            outcomes.push((
                app.world().get::<Enemy>(target).unwrap().hp,
                app.world().resource::<RunRng>().0,
                app.world().resource::<BasicAttackCounter>().0,
            ));
        }
        assert!(outcomes.windows(2).all(|w| w[0] == w[1]));
    }
}

#[test]
fn melee_whoosh_postcombat_pause_and_finish_consume_without_replay() {
    for finish in [false, true] {
        let (mut app, _, _) = produced_whoosh_app(HeroKind::StoneWarden);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(500));
        app.world_mut()
            .run_system_once(crate::game::audio::play_melee_swings)
            .unwrap();
        if finish {
            app.world_mut().resource_mut::<GameSession>().finished = true;
        } else {
            app.world_mut().resource_mut::<GameSession>().paused = true;
        }
        crate::game::audio::tests::drain_feedback(&mut app);
        assert_eq!(whoosh_count(&mut app), 0);
        app.world_mut().resource_mut::<GameSession>().finished = false;
        app.world_mut().resource_mut::<GameSession>().paused = false;
        crate::game::audio::tests::drain_feedback(&mut app);
        assert_eq!(whoosh_count(&mut app), 0);
    }
}
