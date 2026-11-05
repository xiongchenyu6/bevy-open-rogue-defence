#[test]
fn nonarea_typed_strike_cannot_be_intercepted_by_overlapping_elite() {
    fn outcome(reverse_allocation: bool) -> [f32; 2] {
        let mut app = projectile_test_app();
        let mut normal = test_enemy(1000.);
        normal.elite = false;
        let elite = test_enemy(1000.);
        let mut actors = Vec::new();
        for elite_first in if reverse_allocation {
            [true, false]
        } else {
            [false, true]
        } {
            let data = if elite_first {
                Enemy { ..elite }
            } else {
                Enemy { ..normal }
            };
            let position = if elite_first {
                Vec3::X * 0.7
            } else {
                Vec3::ZERO
            };
            actors.push((
                elite_first,
                app.world_mut()
                    .spawn((data, Transform::from_translation(position)))
                    .id(),
            ));
        }
        let normal_id = actors.iter().find(|(elite, _)| !elite).unwrap().1;
        let elite_id = actors.iter().find(|(elite, _)| *elite).unwrap().1;
        crate::game::skills::test_resolve_typed_strike(app.world_mut(), normal_id, false);
        assert_eq!(
            app.world_mut()
                .query::<&Projectile>()
                .iter(app.world())
                .count(),
            1
        );
        app.world_mut().run_system_once(projectile_hits).unwrap();
        let health =
            [normal_id, elite_id].map(|entity| app.world().get::<Enemy>(entity).unwrap().hp);
        println!(
            "DESIGNATED_STRIKE reverse_allocation={reverse_allocation} normal={normal_id:?} elite={elite_id:?} hp={health:?} rng={}",
            app.world().resource::<RunRng>().state()
        );
        health
    }
    let forward = outcome(false);
    let reverse = outcome(true);
    assert_eq!(
        forward,
        [900., 1000.],
        "non-area strike must hit only its designated normal target"
    );
    assert_eq!(
        reverse,
        [900., 1000.],
        "allocation order must not let an overlapping elite intercept"
    );
}

#[test]
fn area_and_ordinary_unrestricted_carriers_keep_spatial_collisions() {
    for area in [true, false] {
        let mut app = projectile_test_app();
        let mut normal = test_enemy(1000.);
        normal.elite = false;
        let normal = app.world_mut().spawn((normal, Transform::default())).id();
        let elite = app
            .world_mut()
            .spawn((test_enemy(1000.), Transform::from_xyz(0.7, 0., 0.)))
            .id();
        if area {
            crate::game::skills::test_resolve_typed_strike(app.world_mut(), normal, true);
        } else {
            app.world_mut()
                .run_system_once(|mut commands: Commands, mut pools: ResMut<CombatPools>| {
                    pools
                        .rent_projectile(
                            &mut commands,
                            Handle::default(),
                            Handle::default(),
                            Transform::from_xyz(0., 0.18, 0.),
                            Projectile {
                                velocity: Vec3::ZERO,
                                damage: 100.,
                                radius: 0.05,
                                remaining_hits: 0,
                                lifetime: 0.12,
                            },
                        )
                        .unwrap();
                })
                .unwrap();
        }
        assert_eq!(
            app.world_mut()
                .query::<&ProjectileTarget>()
                .iter(app.world())
                .count(),
            0
        );
        app.world_mut().run_system_once(projectile_hits).unwrap();
        assert!(app.world().get::<Enemy>(elite).unwrap().hp < 1000.);
        if area {
            assert!(
                app.world().get::<Enemy>(normal).unwrap().hp < 1000.,
                "area still hits both overlapping bodies"
            );
        } else {
            assert_eq!(
                app.world().get::<Enemy>(normal).unwrap().hp,
                1000.,
                "ordinary unrestricted single-hit carrier retains interception"
            );
        }
    }
}

#[test]
fn designated_target_missing_or_dead_never_falls_back_to_neighbor() {
    for missing in [true, false] {
        let mut app = projectile_test_app();
        let mut normal = test_enemy(1000.);
        normal.elite = false;
        let target = app.world_mut().spawn((normal, Transform::default())).id();
        let neighbor = app
            .world_mut()
            .spawn((test_enemy(1000.), Transform::from_xyz(0.7, 0., 0.)))
            .id();
        crate::game::skills::test_resolve_typed_strike(app.world_mut(), target, false);
        let carrier = app
            .world_mut()
            .query_filtered::<Entity, With<Projectile>>()
            .single(app.world())
            .unwrap();
        assert_eq!(
            app.world().get::<ProjectileTarget>(carrier).unwrap().0,
            target
        );
        if missing {
            app.world_mut().despawn(target);
        } else {
            app.world_mut().get_mut::<Enemy>(target).unwrap().hp = 0.;
        }
        let rng = app.world().resource::<RunRng>().state();
        app.world_mut().run_system_once(projectile_hits).unwrap();
        assert_eq!(app.world().get::<Enemy>(neighbor).unwrap().hp, 1000.);
        assert_eq!(
            app.world().resource::<RunRng>().state(),
            rng,
            "no replacement contact or RNG draw"
        );
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(200));
        app.world_mut().run_system_once(move_projectiles).unwrap();
        app.world_mut().run_system_once(projectile_hits).unwrap();
        assert!(app.world().get::<Projectile>(carrier).is_none());
        assert!(app.world().get::<ProjectileTarget>(carrier).is_none());
    }
}

#[test]
fn recycled_designated_carrier_can_hit_a_new_unrestricted_target() {
    let mut app = projectile_test_app();
    let old = app
        .world_mut()
        .spawn((test_enemy(1000.), Transform::from_xyz(10., 0., 0.)))
        .id();
    let next = app
        .world_mut()
        .spawn((test_enemy(1000.), Transform::default()))
        .id();
    crate::game::skills::test_resolve_typed_strike(app.world_mut(), old, false);
    let carrier = app
        .world_mut()
        .query_filtered::<Entity, With<Projectile>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .run_system_once(
            move |mut commands: Commands, mut pools: ResMut<CombatPools>| {
                pools.retire_projectile(&mut commands, carrier);
            },
        )
        .unwrap();
    assert!(app.world().get::<ProjectileTarget>(carrier).is_none());
    app.world_mut()
        .run_system_once(crate::game::pooling::finalize_recycles)
        .unwrap();
    let reused = app
        .world_mut()
        .run_system_once(|mut commands: Commands, mut pools: ResMut<CombatPools>| {
            pools
                .rent_projectile(
                    &mut commands,
                    Handle::default(),
                    Handle::default(),
                    Transform::default(),
                    Projectile {
                        velocity: Vec3::ZERO,
                        damage: 25.,
                        radius: 0.1,
                        remaining_hits: 0,
                        lifetime: 1.,
                    },
                )
                .unwrap()
        })
        .unwrap();
    assert_eq!(reused, carrier);
    assert!(app.world().get::<ProjectileTarget>(reused).is_none());
    app.world_mut().run_system_once(projectile_hits).unwrap();
    assert_eq!(app.world().get::<Enemy>(next).unwrap().hp, 975.);
    assert_eq!(app.world().get::<Enemy>(old).unwrap().hp, 1000.);
}
