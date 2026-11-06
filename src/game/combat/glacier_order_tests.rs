fn glacier_order_outcome(reverse: bool, reallocate: bool) -> ([f32; 2], u64) {
    let mut app = projectile_test_app();
    app.world_mut().resource_mut::<PlayerBuild>().skills.fill(0);
    app.world_mut().resource_mut::<PlayerBuild>().skills[14] = 1;
    app.world_mut()
        .insert_resource(RunRng(12_628_136_444_139_491_497));
    let mut targets = [Entity::PLACEHOLDER; 2];
    for i in if reverse && reallocate {
        [1, 0]
    } else {
        [0, 1]
    } {
        let mut enemy = test_enemy(if i == 0 { 10000. } else { 15000. });
        enemy.max_hp = enemy.hp;
        enemy.elite = false;
        targets[i] = app
            .world_mut()
            .spawn((enemy, Transform::from_xyz(1. + i as f32, 0., 0.)))
            .id();
    }
    for target in if reverse {
        [targets[1], targets[0]]
    } else {
        targets
    } {
        app.world_mut().entity_mut(target).insert(CosmeticReaction);
    }
    let observed_order: Vec<_> = app
        .world_mut()
        .query_filtered::<Entity, With<Enemy>>()
        .iter(app.world())
        .collect();
    assert_eq!(
        observed_order,
        if reverse {
            vec![targets[1], targets[0]]
        } else {
            targets.to_vec()
        }
    );
    let (strikes, pulse, remaining, casts, damage) =
        crate::game::skills::test_glacier_order_pulse(app.world_mut(), targets[0], true);
    assert_eq!((strikes, pulse, remaining, casts), (2, 0.8, 4.2, 1));
    assert!(damage > 0.);
    assert_eq!(
        app.world_mut()
            .query::<&Projectile>()
            .iter(app.world())
            .count(),
        2
    );
    assert_eq!(
        app.world_mut()
            .query::<&ProjectileTarget>()
            .iter(app.world())
            .count(),
        2,
        "real nonarea strikes lock each designated enemy"
    );
    app.world_mut().run_system_once(projectile_hits).unwrap();
    let hp = targets.map(|entity| app.world().get::<Enemy>(entity).unwrap().hp);
    let losses = [10000. - hp[0], 15000. - hp[1]];
    assert!(
        losses[0].max(losses[1]) > losses[0].min(losses[1]) * 1.5,
        "recorded seed must cause one actual critical and one actual noncritical hit"
    );
    assert_eq!(app.world().resource::<GameSession>().total_kills, 0);
    assert_eq!(
        app.world_mut()
            .query::<&Projectile>()
            .iter(app.world())
            .count(),
        0
    );
    let rng = app.world().resource::<RunRng>().state();
    assert_eq!(
        rng, 16_263_334_312_487_546_840,
        "same four real critical/proc RNG draws"
    );
    println!(
        "GLACIER_ORDER reverse={reverse} reallocate={reallocate} ids={targets:?} rows={observed_order:?} hp={hp:?} damage={damage} rng={rng} casts={casts} strikes={strikes} pulse={pulse} remaining={remaining}"
    );
    (hp, rng)
}

#[test]
fn glacier_same_ids_ignore_cosmetic_row_order() {
    let outcomes = [false, true].map(|reverse| glacier_order_outcome(reverse, false));
    assert_eq!(outcomes[0], outcomes[1]);
}

#[test]
fn glacier_critical_assignment_follows_geometry_across_allocations() {
    let outcomes = [false, true].map(|reverse| glacier_order_outcome(reverse, true));
    assert_eq!(
        outcomes[0], outcomes[1],
        "unrelated allocation must not exchange Glacier critical assignments"
    );
}

#[test]
fn glacier_third_real_pulse_freezes_both_targets() {
    for reverse in [false, true] {
        let mut app = projectile_test_app();
        app.world_mut().resource_mut::<PlayerBuild>().skills.fill(0);
        app.world_mut().resource_mut::<PlayerBuild>().skills[14] = 1;
        let mut targets = [Entity::PLACEHOLDER; 2];
        for i in if reverse { [1, 0] } else { [0, 1] } {
            let mut enemy = test_enemy(if i == 0 { 10000. } else { 15000. });
            enemy.max_hp = enemy.hp;
            enemy.elite = false;
            targets[i] = app
                .world_mut()
                .spawn((enemy, Transform::from_xyz(1. + i as f32, 0., 0.)))
                .id();
        }
        for pulse in 0..3 {
            let (strikes, _, _, casts, _) = crate::game::skills::test_glacier_order_pulse(
                app.world_mut(),
                targets[0],
                pulse == 0,
            );
            assert_eq!((strikes, casts), (2, 1));
            for target in targets {
                assert_eq!(
                    app.world()
                        .resource::<crate::game::skills::SkillRuntime>()
                        .actions_blocked(target),
                    pulse == 2,
                    "each designated target freezes on its third actual pulse"
                );
            }
            app.world_mut().run_system_once(projectile_hits).unwrap();
            assert_eq!(
                app.world_mut()
                    .query::<&Projectile>()
                    .iter(app.world())
                    .count(),
                0
            );
        }
    }
}
