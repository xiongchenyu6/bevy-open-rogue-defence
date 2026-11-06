fn cinder_tied_order_outcome(reverse: bool, reallocate: bool) -> ([f32; 2], u64) {
    let mut app = projectile_test_app();
    app.world_mut().resource_mut::<PlayerBuild>().skills.fill(0);
    app.world_mut().resource_mut::<PlayerBuild>().skills[11] = 4;
    let origin = Vec3::new(-8.2, 0., 0.);
    app.world_mut()
        .spawn((Hero, Transform::from_translation(origin)));
    let positions = [
        Vec3::new(f32::from_bits(3238177851), 0., f32::from_bits(3215769503)),
        Vec3::new(f32::from_bits(3236208368), 0., f32::from_bits(3210178573)),
    ];
    let d2 = positions.map(|position| origin.distance_squared(position).to_bits());
    assert_eq!(
        d2, [1072252846; 2],
        "actual source-f32 equality is the precondition"
    );
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
            .spawn((enemy, Transform::from_translation(positions[i])))
            .id();
    }
    let insertion = if reverse {
        [targets[1], targets[0]]
    } else {
        targets
    };
    for target in insertion {
        app.world_mut().entity_mut(target).insert(CosmeticReaction);
    }
    let rows: Vec<_> = app
        .world_mut()
        .query_filtered::<Entity, With<Enemy>>()
        .iter(app.world())
        .collect();
    assert_eq!(
        rows, insertion,
        "control must actually invert harmless archetype row order"
    );
    app.world_mut()
        .insert_resource(RunRng(12_628_136_444_139_491_497));
    let (strike_targets, damage, cooldown, casts) =
        crate::game::skills::test_cinder_order_cast(app.world_mut());
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
        2
    );
    app.world_mut().run_system_once(projectile_hits).unwrap();
    let hp = targets.map(|e| app.world().get::<Enemy>(e).unwrap().hp);
    let rng = app.world().resource::<RunRng>().state();
    // Print actual producer/HP/RNG states before semantic-pair comparisons.
    println!(
        "CINDER_TIED_ORDER reverse={reverse} reallocate={reallocate} ids={targets:?} rows={rows:?} positions={positions:?} distance_bits={d2:?} strike_targets={strike_targets:?} damage={damage} cooldown={cooldown} casts={casts} hp={hp:?} rng={rng}"
    );
    let losses = [10000. - hp[0], 15000. - hp[1]];
    assert!(
        losses[0].max(losses[1]) > losses[0].min(losses[1]) * 1.5,
        "seed must actually yield one critical and one noncritical result"
    );
    assert_eq!(
        rng, 16_263_334_312_487_546_840,
        "same four actual crit/proc draws"
    );
    assert_eq!(app.world().resource::<GameSession>().total_kills, 0);
    assert_eq!(
        app.world_mut()
            .query::<&Projectile>()
            .iter(app.world())
            .count(),
        0
    );
    (hp, rng)
}

#[test]
fn cinder_equal_distance_ignores_cosmetic_row_relocation() {
    let outcomes = [false, true].map(|reverse| cinder_tied_order_outcome(reverse, false));
    assert_eq!(
        outcomes[0], outcomes[1],
        "harmless relocation must not reassign skill11 critical damage"
    );
}

#[test]
fn cinder_equal_distance_follows_geometry_across_entity_allocations() {
    let outcomes = [false, true].map(|reverse| cinder_tied_order_outcome(reverse, true));
    assert_eq!(
        outcomes[0], outcomes[1],
        "same physical tied targets must retain critical assignment"
    );
}
