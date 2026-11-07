fn arrow_card_app(kind: HeroKind, acquire: bool) -> (App, Entity) {
    let (mut app, _) = ranged_release_app(kind);
    app.add_plugins(super::super::bonds::BondsPlugin);
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
