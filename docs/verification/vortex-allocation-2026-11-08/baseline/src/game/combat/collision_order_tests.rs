// Test-only cosmetic marker models HitPunch's enemy archetype migration without
// exposing that production component's private construction fields.
#[derive(Component)]
struct CosmeticReaction;

fn cosmetic_collision_outcome(reverse: bool, moving: bool) -> ([Entity; 2], [f32; 2], u64) {
    let mut app = projectile_test_app();
    let targets = std::array::from_fn(|_| {
        let mut enemy = test_enemy(100.);
        enemy.elite = false;
        app.world_mut().spawn((enemy, Transform::default())).id()
    });
    let insertion = if reverse {
        [targets[1], targets[0]]
    } else {
        targets
    };
    for target in insertion {
        app.world_mut().entity_mut(target).insert(CosmeticReaction);
    }
    let actual_order: Vec<_> = app
        .world_mut()
        .query_filtered::<Entity, (With<Enemy>, With<CosmeticReaction>)>()
        .iter(app.world())
        .collect();
    assert_eq!(
        actual_order, insertion,
        "fixture must change only cosmetic archetype row order"
    );
    app.world_mut()
        .run_system_once(
            move |mut commands: Commands, mut pools: ResMut<CombatPools>| {
                let projectile = pools
                    .rent_projectile(
                        &mut commands,
                        Handle::default(),
                        Handle::default(),
                        Transform::from_translation(if moving { Vec3::X } else { Vec3::ZERO }),
                        Projectile {
                            velocity: if moving { Vec3::X * 20. } else { Vec3::ZERO },
                            damage: 25.,
                            radius: 0.1,
                            remaining_hits: 0,
                            lifetime: 2.,
                        },
                    )
                    .expect("real pooled projectile should rent");
                commands
                    .entity(projectile)
                    .insert(DamageProfile::PHYSICAL_ATTACK);
            },
        )
        .unwrap();
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_millis(100));
    app.world_mut().run_system_once(projectile_hits).unwrap();
    let health = targets.map(|target| app.world().get::<Enemy>(target).unwrap().hp);
    let rng = app.world().resource::<RunRng>().state();
    assert_eq!(
        rng, 11_083_784_628_768_264_341,
        "opt-in observation must preserve the pre-trace single-contact RNG budget"
    );
    println!(
        "COSMETIC_COLLISION reverse={reverse} moving={moving} targets={targets:?} query_order={actual_order:?} hp={health:?} rng={rng}"
    );
    (targets, health, rng)
}

#[test]
fn stationary_collision_ignores_cosmetic_archetype_insertion_order() {
    let forward = cosmetic_collision_outcome(false, false);
    let reverse = cosmetic_collision_outcome(true, false);
    assert_eq!(
        forward.0, reverse.0,
        "identical actor allocation in both worlds"
    );
    assert_eq!(
        forward.1, reverse.1,
        "cosmetic migration must not change the actual single-hit victim"
    );
    assert_eq!(
        forward.2, reverse.2,
        "cosmetic migration must not change authoritative RNG consumption"
    );
    assert_eq!(forward.1.iter().filter(|hp| **hp < 100.).count(), 1);
}

#[test]
fn moving_collision_ignores_cosmetic_archetype_insertion_order() {
    let forward = cosmetic_collision_outcome(false, true);
    let reverse = cosmetic_collision_outcome(true, true);
    assert_eq!(forward.0, reverse.0);
    assert_eq!(
        forward.1, reverse.1,
        "moving sweep retains stable entity tie ordering"
    );
    assert_eq!(forward.2, reverse.2);
    assert_eq!(forward.1.iter().filter(|hp| **hp < 100.).count(), 1);
}

#[test]
fn vortex_pulse_damage_ignores_cosmetic_enemy_archetype_order() {
    fn outcome(reverse: bool) -> ([Entity; 2], [Option<f32>; 2], u64) {
        let mut app = projectile_test_app();
        app.init_resource::<crate::game::SkillCastTelemetry>();
        // Recorded production stream: one noncritical draw, a spell-proc draw,
        // one critical draw, a spell-proc draw. No extra RNG fixture calls.
        app.world_mut()
            .insert_resource(RunRng(12_628_136_444_139_491_497));
        let targets = [
            app.world_mut()
                .spawn((test_enemy(1000.), Transform::from_xyz(-1., 0., 0.)))
                .id(),
            app.world_mut()
                .spawn((test_enemy(50.), Transform::from_xyz(1., 0., 0.)))
                .id(),
        ];
        for target in targets {
            app.world_mut().get_mut::<Enemy>(target).unwrap().elite = false;
        }
        for target in if reverse {
            [targets[1], targets[0]]
        } else {
            targets
        } {
            app.world_mut().entity_mut(target).insert(CosmeticReaction);
        }
        let query_order: Vec<_> = app
            .world_mut()
            .query_filtered::<Entity, With<Enemy>>()
            .iter(app.world())
            .collect();
        assert_eq!(
            query_order,
            if reverse {
                vec![targets[1], targets[0]]
            } else {
                targets.to_vec()
            }
        );
        let (strikes, pulse_remaining) =
            crate::game::skills::test_vortex_order_pulse(app.world_mut());
        assert_eq!(
            strikes, 2,
            "one real pulse schedules one typed strike per target"
        );
        assert_eq!(pulse_remaining, 0.5, "exactly one production Vortex pulse");
        assert_eq!(
            app.world_mut()
                .query::<&Projectile>()
                .iter(app.world())
                .count(),
            2,
            "two real pooled projectiles before authoritative collision"
        );
        app.world_mut().run_system_once(projectile_hits).unwrap();
        assert_eq!(
            app.world()
                .resource::<crate::game::SkillCastTelemetry>()
                .total,
            0,
            "fixture starts an existing zone, it does not fabricate a new skill cast"
        );
        assert_eq!(app.world().resource::<GameSession>().total_kills, 1);
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
            "same recorded four-draw RNG budget"
        );
        let health = targets.map(|target| app.world().get::<Enemy>(target).map(|enemy| enemy.hp));
        println!(
            "VORTEX_ORDER reverse={reverse} targets={targets:?} enemy_query={query_order:?} health={health:?} strikes={strikes} pulse={pulse_remaining} rng={rng}"
        );
        (targets, health, rng)
    }
    let forward = outcome(false);
    let reverse = outcome(true);
    assert_eq!(forward.0, reverse.0);
    assert_eq!(forward.2, reverse.2);
    assert_eq!(
        forward.1, reverse.1,
        "a purely cosmetic archetype migration must not reassign a zone's critical damage"
    );
}

fn overlap_geometry_outcome(
    reverse: bool,
    points: [Vec3; 2],
    start: Vec3,
    end: Vec3,
) -> ([f32; 2], u64) {
    let mut app = projectile_test_app();
    let mut targets = [Entity::PLACEHOLDER; 2];
    for i in if reverse { [1, 0] } else { [0, 1] } {
        let mut enemy = test_enemy(100.);
        enemy.elite = false;
        targets[i] = app
            .world_mut()
            .spawn((enemy, Transform::from_translation(points[i])))
            .id();
    }
    app.world_mut()
        .run_system_once(
            move |mut commands: Commands, mut pools: ResMut<CombatPools>| {
                let entity = pools
                    .rent_projectile(
                        &mut commands,
                        Handle::default(),
                        Handle::default(),
                        Transform::from_translation(end),
                        Projectile {
                            velocity: (end - start) * 10.,
                            damage: 25.,
                            radius: 0.48,
                            remaining_hits: 0,
                            lifetime: 2.,
                        },
                    )
                    .unwrap();
                commands
                    .entity(entity)
                    .insert(DamageProfile::PHYSICAL_ATTACK);
            },
        )
        .unwrap();
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_millis(100));
    app.world_mut().run_system_once(projectile_hits).unwrap();
    let hp = targets.map(|entity| app.world().get::<Enemy>(entity).unwrap().hp);
    let rng = app.world().resource::<RunRng>().state();
    assert_eq!(
        rng, 11_083_784_628_768_264_341,
        "one actual noncritical attack contact retains RNG budget"
    );
    assert_eq!(
        app.world_mut()
            .query::<&Projectile>()
            .iter(app.world())
            .count(),
        0,
        "single-hit carrier retires"
    );
    println!("OVERLAP_GEOMETRY reverse={reverse} points={points:?} hp={hp:?} rng={rng}");
    (hp, rng)
}

#[test]
fn overlap_geometry_prefers_nearest_sweep_start_center() {
    let start = Vec3::new(-8.507_8, 0.25, 1.457_371_5);
    let points = [
        Vec3::new(-8.478_959, 0., 2.158_846),
        Vec3::new(-8.519_881, 0., 2.369_858_7),
    ];
    let end = start + Vec3::new(1.703_585, 0., 58.303_28) * 0.1;
    let outcomes =
        [false, true].map(|reverse| overlap_geometry_outcome(reverse, points, start, end));
    for outcome in outcomes {
        assert_eq!(outcome.0, [75., 100.]);
    }
}

#[test]
fn overlap_geometry_symmetric_tie_uses_visible_coordinates() {
    let outcomes = [false, true].map(|reverse| {
        overlap_geometry_outcome(
            reverse,
            [Vec3::new(0., 0., -0.3), Vec3::new(0., 0., 0.3)],
            Vec3::ZERO,
            Vec3::X,
        )
    });
    for outcome in outcomes {
        assert_eq!(outcome.0, [75., 100.]);
    }
}

#[test]
fn overlap_geometry_preserves_nonzero_entry_progress_before_distance() {
    // The first sphere is entered at x=.34, while the closer-center
    // second sphere is entered at x≈.469. Start-center distance must not override entry.
    for reverse in [false, true] {
        assert_eq!(
            overlap_geometry_outcome(
                reverse,
                [Vec3::new(1.3, 0., 0.), Vec3::new(1., 0., 0.8)],
                Vec3::ZERO,
                Vec3::X * 3.
            )
            .0,
            [75., 100.]
        );
    }
}

fn ray_order_outcome(reverse: bool, reallocate: bool) -> ([f32; 2], u64) {
    let mut app = projectile_test_app();
    app.world_mut().resource_mut::<PlayerBuild>().skills[30] = 1;
    app.world_mut()
        .insert_resource(RunRng(12_628_136_444_139_491_497));
    let mut targets = [Entity::PLACEHOLDER; 2];
    for i in if reverse && reallocate {
        [1, 0]
    } else {
        [0, 1]
    } {
        let mut enemy = test_enemy(if i == 0 { 1000. } else { 1500. });
        enemy.elite = false;
        targets[i] = app
            .world_mut()
            .spawn((enemy, Transform::from_xyz(3. + i as f32, 0., 0.)))
            .id();
    }
    for target in if reverse {
        [targets[1], targets[0]]
    } else {
        targets
    } {
        app.world_mut().entity_mut(target).insert(CosmeticReaction);
    }
    let (strikes, remaining, pulse) = crate::game::skills::test_ray_order_damage(app.world_mut());
    assert_eq!((strikes, remaining, pulse), (2, 2., 0.25));
    assert_eq!(
        app.world_mut()
            .query::<&Projectile>()
            .iter(app.world())
            .count(),
        2
    );
    app.world_mut().run_system_once(projectile_hits).unwrap();
    let hp = targets.map(|entity| app.world().get::<Enemy>(entity).unwrap().hp);
    let rng = app.world().resource::<RunRng>().state();
    assert_eq!(
        rng, 16_263_334_312_487_546_840,
        "exact four production critical/proc draws"
    );
    assert_eq!(
        app.world_mut()
            .query::<&Projectile>()
            .iter(app.world())
            .count(),
        0
    );
    println!(
        "RAY_ORDER reverse={reverse} reallocate={reallocate} targets={targets:?} hp={hp:?} rng={rng}"
    );
    (hp, rng)
}

#[test]
fn ray_pulse_damage_ignores_cosmetic_query_order() {
    let outcomes = [false, true].map(|reverse| ray_order_outcome(reverse, false));
    assert_eq!(outcomes[0], outcomes[1]);
}

#[test]
fn ray_pulse_damage_follows_geometry_across_entity_allocations() {
    let outcomes = [false, true].map(|reverse| ray_order_outcome(reverse, true));
    assert_eq!(outcomes[0], outcomes[1]);
}
