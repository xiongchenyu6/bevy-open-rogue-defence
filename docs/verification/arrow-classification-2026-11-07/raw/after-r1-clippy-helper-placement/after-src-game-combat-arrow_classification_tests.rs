fn arrow_card_app(kind: HeroKind, acquire: bool) -> (App, Entity) {
    let (mut app, _) = ranged_release_app(kind);
    app.add_plugins(super::super::bonds::BondsPlugin)
        .init_resource::<super::super::hit_feedback::HitFeedback>();
    app.world_mut()
        .run_system_once(
            move |mut session: ResMut<GameSession>, mut build: ResMut<PlayerBuild>| {
                for family in [
                    super::super::BondId::Journey,
                    super::super::BondId::Agility,
                    super::super::BondId::Ranger,
                ] {
                    for card in family.definition().members {
                        super::super::progression::test_acquire_bond_card(
                            *card,
                            &mut session,
                            &mut build,
                        );
                    }
                }
                if acquire {
                    super::super::progression::test_acquire_bond_card(
                        super::super::BondCardId::ArcheryPiercing,
                        &mut session,
                        &mut build,
                    );
                }
            },
        )
        .unwrap();
    app.update();
    while app.world().resource::<BondRuntime>().arrows_remaining > 0 {
        app.world_mut()
            .resource_mut::<BondRuntime>()
            .consume_arrow();
    }
    assert_eq!(
        app.world().resource::<BondRuntime>().archery_active(),
        acquire
    );
    let target = app
        .world_mut()
        .query_filtered::<Entity, With<Enemy>>()
        .single(app.world())
        .unwrap();
    {
        let mut enemy = app.world_mut().get_mut::<Enemy>(target).unwrap();
        enemy.hp = 100000.;
        enemy.max_hp = 100000.;
        enemy.elite = false;
    }
    // A single native arrow is isolated after actual card stats settle.
    app.world_mut().resource_mut::<PlayerBuild>().multishot = 1;
    app.world_mut().resource_mut::<PlayerBuild>().pierce = 0;
    app.world_mut().resource_mut::<RunRng>().0 = 12628136444139491497;
    (app, target)
}
fn arrow_native_loss(kind: HeroKind, acquire: bool) -> f32 {
    let (mut app, target) = arrow_card_app(kind, acquire);
    commit_ranged(&mut app);
    app.world_mut()
        .run_system_once(ranged_release::release_pending)
        .unwrap();
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_millis(200));
    app.world_mut()
        .run_system_once(ranged_release::release_pending)
        .unwrap();
    assert_eq!(ranged_active(&mut app), 1);
    let profiles: Vec<_> = app
        .world_mut()
        .query::<&DamageProfile>()
        .iter(app.world())
        .copied()
        .collect();
    app.world_mut().run_system_once(move_projectiles).unwrap();
    app.world_mut().run_system_once(projectile_hits).unwrap();
    let loss = 100000. - app.world().get::<Enemy>(target).unwrap().hp;
    let events = app
        .world()
        .resource::<super::super::hit_feedback::HitFeedback>()
        .events();
    assert_eq!(events.len(), 1);
    assert!(
        !events[0].critical,
        "recorded native pair must be non-critical"
    );
    println!(
        "ARROW_NATIVE kind={kind:?} card={acquire} loss={loss} profiles={profiles:?} rng={}",
        app.world().resource::<RunRng>().state()
    );
    assert!(loss > 0.);
    loss
}
#[test]
fn arrow_classification_native_ember_earns_true_shot_damage() {
    let outcomes = [false, true].map(|card| arrow_native_loss(HeroKind::EmberRanger, card));
    assert!(
        (outcomes[1] - outcomes[0] * 1.15).abs() < 0.03,
        "actual native HP losses {outcomes:?}"
    );
}

#[test]
fn arrow_classification_native_grove_crossbow_earns_true_shot_damage() {
    let outcomes = [false, true].map(|card| arrow_native_loss(HeroKind::GroveArtificer, card));
    assert!(
        (outcomes[1] - outcomes[0] * 1.15).abs() < 0.03,
        "actual Grove HP losses {outcomes:?}"
    );
}

#[test]
fn arrow_classification_native_mage_is_excluded_from_true_shot_damage() {
    let outcomes = [false, true].map(|card| arrow_native_loss(HeroKind::StormSavant, card));
    assert_eq!(outcomes[0], outcomes[1]);
}

fn arrow_fusion_loss(kind: HeroKind, card: bool) -> (f32, bool) {
    let (mut app, target) = arrow_card_app(HeroKind::StormSavant, card);
    app.init_resource::<super::super::hero_fusion::HeroFusion>()
        .init_resource::<WeaponCues>();
    let hero = app
        .world_mut()
        .query_filtered::<Entity, With<Hero>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .spawn(super::super::hero_fusion::FusionRequest {
            hero,
            kind,
            target,
            damage: 100.,
            speed: 10.,
        });
    super::super::hero_fusion::test_emit_fusion_requests(app.world_mut());
    let profile = *app
        .world_mut()
        .query::<&DamageProfile>()
        .single(app.world())
        .unwrap();
    assert!(profile.card_proc && !profile.melee);
    assert_eq!(
        profile.element,
        if kind == HeroKind::EmberRanger {
            DamageElement::Flame
        } else if kind == HeroKind::StormSavant {
            DamageElement::Storm
        } else {
            DamageElement::Physical
        }
    );
    assert_eq!(profile.applies_burn, kind == HeroKind::EmberRanger);
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_millis(200));
    app.world_mut().run_system_once(move_projectiles).unwrap();
    app.world_mut().run_system_once(projectile_hits).unwrap();
    let loss = 100000. - app.world().get::<Enemy>(target).unwrap().hp;
    assert!(loss > 0.);
    let events = app
        .world()
        .resource::<super::super::hit_feedback::HitFeedback>()
        .events();
    assert_eq!(events.len(), 1);
    assert!(!events[0].critical);
    println!(
        "ARROW_FUSION kind={kind:?} card={card} loss={loss} profile={profile:?} rng={}",
        app.world().resource::<RunRng>().state()
    );
    (loss, profile.arrow)
}

#[test]
fn arrow_classification_real_fusion_crossbows_gain_bonus_and_mage_does_not() {
    for kind in [
        HeroKind::EmberRanger,
        HeroKind::GroveArtificer,
        HeroKind::StormSavant,
    ] {
        let outcomes = [false, true].map(|card| arrow_fusion_loss(kind, card));
        let arrow = kind != HeroKind::StormSavant;
        assert_eq!(outcomes[0].1, arrow);
        assert_eq!(outcomes[1].1, arrow);
        assert!((outcomes[1].0 - outcomes[0].0 * if arrow { 1.15 } else { 1. }).abs() < 0.03);
    }
}

fn piercing_arrow_loss(card: bool) -> (f32, Vec<bool>, u64) {
    let (mut app, target) = arrow_card_app(HeroKind::StormSavant, card);
    app.world_mut().resource_mut::<PlayerBuild>().skills.fill(0);
    app.world_mut().resource_mut::<PlayerBuild>().skills[7] = 1;
    super::super::skills::test_cast_piercing_volley(app.world_mut());
    let profiles: Vec<_> = app
        .world_mut()
        .query::<(&Projectile, &DamageProfile)>()
        .iter(app.world())
        .map(|(p, d)| (p.damage, *d))
        .collect();
    assert!(!profiles.is_empty());
    assert!(
        profiles
            .iter()
            .all(|(_, p)| p.arrow && p.skill_index == Some(7))
    );
    let world = app.world();
    let build = world.resource::<PlayerBuild>();
    let vitals = world.resource::<HeroVitals>();
    let runtime = world.resource::<SkillRuntime>();
    let raw = super::super::skills::skill_preview_damage(7, 1, build, vitals, runtime).unwrap();
    assert!(profiles.iter().all(|(damage, _)| *damage == raw));
    let expected = raw
        * world
            .resource::<BondRuntime>()
            .damage_multiplier(profiles[0].1)
        * market_outgoing_damage_multiplier(
            world.resource::<MarketEffects>(),
            profiles[0].1,
            build,
            vitals,
        )
        * world
            .resource::<EncounterState>()
            .outgoing_damage_multiplier();
    let text = super::super::skill_details::detailed(
        super::super::GameLanguage::English,
        super::super::SkillId(7),
        1,
        build,
        vitals,
        runtime,
        super::super::skill_details::DamagePreviewContext {
            bonds: world.resource::<BondRuntime>(),
            market: world.resource::<MarketEffects>(),
            encounters: world.resource::<EncounterState>(),
        },
    );
    assert!(
        text.contains(&format!(
            "With current bonuses (non-critical): {expected:.1} → {expected:.1}"
        )),
        "{text}"
    );
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_millis(200));
    app.world_mut().run_system_once(move_projectiles).unwrap();
    app.world_mut().run_system_once(projectile_hits).unwrap();
    let loss = 100000. - app.world().get::<Enemy>(target).unwrap().hp;
    let critical: Vec<_> = app
        .world()
        .resource::<super::super::hit_feedback::HitFeedback>()
        .events()
        .iter()
        .map(|event| event.critical)
        .collect();
    assert_eq!(critical.len(), profiles.len());
    let rng = app.world().resource::<RunRng>().state();
    println!(
        "ARROW_PIERCING card={card} loss={loss} profiles={profiles:?} critical={critical:?} rng={rng} preview={expected}"
    );
    (loss, critical, rng)
}

#[test]
fn arrow_classification_real_fourth_attack_skill_and_preview_use_arrow_bonuses() {
    let outcomes = [false, true].map(piercing_arrow_loss);
    assert!((outcomes[1].0 - outcomes[0].0 * 1.15).abs() < 0.04);
    assert_eq!(outcomes[0].1, outcomes[1].1);
    assert_eq!(outcomes[0].2, outcomes[1].2);
}
