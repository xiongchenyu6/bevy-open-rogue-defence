// Controlled inherited-contact boundary, followed by real item splash creation
// and resolution. This does not stand in for hero-emission cadence or UI buying.
fn frost_market_splash_outcome(reverse: bool, reallocate: bool, tied: bool) -> ([f32; 2], u64) {
    let mut app = projectile_test_app();
    app.world_mut()
        .run_system_once(
            |mut session: ResMut<GameSession>,
             mut build: ResMut<PlayerBuild>,
             mut vitals: ResMut<HeroVitals>,
             mut effects: ResMut<MarketEffects>| {
                super::super::market::MarketItem::SerratedDisc.apply(
                    &mut session,
                    &mut build,
                    &mut vitals,
                    &mut effects,
                );
            },
        )
        .unwrap();
    assert_eq!(app.world().resource::<MarketEffects>().splash_ratio(), 0.30);
    let origin = Vec3::new(f32::from_bits(1092943774), 0., f32::from_bits(1068289229));
    let positions = [
        Vec3::new(f32::from_bits(1093132518), 0., f32::from_bits(1076677837)),
        if tied {
            Vec3::new(f32::from_bits(1092755030), 0., 0.)
        } else {
            origin + Vec3::X * 2.0
        },
    ];
    let distances = positions.map(|p| planar_distance_squared(origin, p));
    if tied {
        assert_eq!(distances.map(f32::to_bits), [1072524639; 2]);
    } else {
        assert!(distances[0] < distances[1]);
    }
    let source = app
        .world_mut()
        .spawn((test_enemy(20000.), Transform::from_translation(origin)))
        .id();
    let mut targets = [Entity::PLACEHOLDER; 2];
    for i in if reverse && reallocate {
        [1, 0]
    } else {
        [0, 1]
    } {
        targets[i] = app
            .world_mut()
            .spawn((
                test_enemy(if i == 0 { 10000. } else { 15000. }),
                Transform::from_translation(positions[i]),
            ))
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
        .filter(|e| *e != source)
        .collect();
    assert_eq!(rows, insertion);
    app.world_mut()
        .insert_resource(RunRng(12_628_136_444_139_491_497));
    app.world_mut()
        .run_system_once(
            move |mut commands: Commands,
                  assets: Res<GameAssets>,
                  mut pools: ResMut<CombatPools>| {
                let profile = super::super::hero_fusion::profile(HeroKind::FrostOracle);
                assert_eq!(profile.element, DamageElement::Frost);
                assert!(profile.card_proc && !profile.market_proc);
                let carrier = rent_market_projectile(
                    &mut commands,
                    &mut pools,
                    &assets,
                    origin,
                    100.,
                    0.05,
                    0,
                    profile,
                )
                .unwrap();
                commands
                    .entity(carrier)
                    .insert(super::super::hero_fusion::FusionAttack {
                        kind: HeroKind::FrostOracle,
                        origin,
                    });
                restrict_projectile_to_target(&mut commands, carrier, source);
            },
        )
        .unwrap();
    app.world_mut().run_system_once(projectile_hits).unwrap();
    let source_hp = app.world().get::<Enemy>(source).unwrap().hp;
    assert!(source_hp < 20000.);
    assert_eq!(
        targets.map(|e| app.world().get::<Enemy>(e).unwrap().hp),
        [10000., 15000.]
    );
    let frozen_source = app
        .world()
        .get::<super::super::hero_abilities::TimedSpeedEffect>(source)
        .unwrap();
    assert_eq!(
        (frozen_source.multiplier, frozen_source.remaining),
        (0., 0.45)
    );
    let children: Vec<_> = app
        .world_mut()
        .query::<(&Projectile, &DamageProfile, &ProjectileTarget)>()
        .iter(app.world())
        .map(|(p, d, t)| (p.damage, *d, t.0))
        .collect();
    assert_eq!(children.len(), 2);
    for (damage, profile, target) in &children {
        // Preserve the actual f32 product rather than rounding the expected value.
        assert_eq!(*damage, 100.0_f32 * 0.30_f32);
        assert_eq!(profile.element, DamageElement::Frost);
        assert_eq!(profile.class, DamageClass::Attack);
        assert!(profile.market_proc && profile.card_proc);
        assert!(targets.contains(target) && *target != source);
    }
    app.world_mut().run_system_once(projectile_hits).unwrap();
    let hp = targets.map(|e| app.world().get::<Enemy>(e).unwrap().hp);
    let losses = [10000. - hp[0], 15000. - hp[1]];
    let rng = app.world().resource::<RunRng>().state();
    println!(
        "FROST_MARKET_SPLASH reverse={reverse} reallocate={reallocate} tied={tied} ids={targets:?} rows={rows:?} distances={distances:?} children={children:?} hp={hp:?} source_hp={source_hp} rng={rng}"
    );
    assert!(
        losses[0].max(losses[1]) > losses[0].min(losses[1]) * 1.5,
        "actual child resolutions must include one critical and one ordinary hit"
    );
    assert_eq!(
        rng, 5_218_726_287_330_236_675,
        "six actual parent/child crit/proc draws"
    );
    assert_eq!(app.world().get::<Enemy>(source).unwrap().hp, source_hp);
    assert_eq!(
        app.world_mut()
            .query::<&Projectile>()
            .iter(app.world())
            .count(),
        0,
        "market splash children retire without recursively splashing"
    );
    assert_eq!(app.world().resource::<GameSession>().total_kills, 0);
    (hp, rng)
}

#[test]
fn frost_market_splash_same_ids_ignore_cosmetic_rows() {
    let result = [false, true].map(|reverse| frost_market_splash_outcome(reverse, false, true));
    assert_eq!(result[0], result[1]);
}

#[test]
fn frost_market_splash_tied_targets_keep_critical_assignment_across_allocations() {
    let result = [false, true].map(|reverse| frost_market_splash_outcome(reverse, true, true));
    assert_eq!(
        result[0], result[1],
        "physical neighbors must retain their splash critical rolls"
    );
}

#[test]
fn frost_market_splash_preserves_unequal_distance_priority() {
    let result = [false, true].map(|reverse| frost_market_splash_outcome(reverse, true, false));
    assert_eq!(result[0], result[1]);
}
