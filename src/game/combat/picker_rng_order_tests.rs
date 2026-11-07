// Controlled single-tick ordering probe. This does not attribute the full replay mismatch.
#[derive(Debug)]
struct PickerOrderOutcome {
    health: f32,
    rng: u64,
    choices: Vec<crate::game::DraftChoice>,
    active_projectiles: usize,
    gold: u32,
}
fn picker_contact_order_outcome(contact_first: bool) -> PickerOrderOutcome {
    let mut app = projectile_test_app();
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<crate::game::DraftState>()
        .init_resource::<crate::game::SelectedHero>()
        .add_plugins(crate::game::progression::ProgressionPlugin);
    app.insert_resource(RunRng::from_seed(0xA11CE));
    app.world_mut()
        .resource_mut::<crate::game::SelectedHero>()
        .locked = true;
    {
        let mut session = app.world_mut().resource_mut::<GameSession>();
        session.level = 8;
        session.elapsed = 100.;
        session.gold = 80;
        session.skill_points = 0;
    }
    {
        let mut economy = app.world_mut().resource_mut::<crate::game::SkillEconomy>();
        economy.first_free_granted = true;
        economy.level_grants = 1;
    }
    app.world_mut().resource_mut::<PlayerBuild>().skills[..6].fill(4);
    let mut enemy = test_enemy(1000.);
    enemy.elite = false;
    let target = app.world_mut().spawn((enemy, Transform::default())).id();
    app.world_mut()
        .run_system_once(|mut commands: Commands, mut pools: ResMut<CombatPools>| {
            let entity = pools
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
                        lifetime: 2.,
                    },
                )
                .unwrap();
            commands
                .entity(entity)
                .insert(DamageProfile::PHYSICAL_ATTACK);
        })
        .unwrap();
    assert_eq!(
        app.world_mut()
            .query::<&Projectile>()
            .iter(app.world())
            .count(),
        1
    );
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyG);
    let mut schedule = Schedule::default();
    if contact_first {
        schedule.add_systems(
            (
                projectile_hits,
                crate::game::progression::test_request_skill_draft,
            )
                .chain(),
        );
    } else {
        schedule.add_systems(
            (
                crate::game::progression::test_request_skill_draft,
                projectile_hits,
            )
                .chain(),
        );
    }
    schedule.run(app.world_mut());
    let draft = app.world().resource::<crate::game::DraftState>();
    assert!(draft.root.is_some());
    assert_eq!(draft.choices.len(), 3);
    assert!(app.world().resource::<GameSession>().paused);
    assert_eq!(
        app.world()
            .resource::<crate::game::SkillEconomy>()
            .paid_draws,
        1
    );
    let choices = draft.choices.clone();
    let outcome = PickerOrderOutcome {
        health: app.world().get::<Enemy>(target).unwrap().hp,
        rng: app.world().resource::<RunRng>().state(),
        choices,
        active_projectiles: app
            .world_mut()
            .query::<&Projectile>()
            .iter(app.world())
            .count(),
        gold: app.world().resource::<GameSession>().gold,
    };
    println!("PICKER_CONTACT_ORDER contact_first={contact_first} outcome={outcome:?}");
    outcome
}
#[test]
fn real_g_draw_is_sensitive_to_actual_contact_order_in_the_same_tick() {
    let contact_first = picker_contact_order_outcome(true);
    let draft_first = picker_contact_order_outcome(false);
    assert!(contact_first.health < 1000.);
    assert_eq!(draft_first.health, 1000.);
    assert_eq!(contact_first.active_projectiles, 0);
    assert_eq!(draft_first.active_projectiles, 1);
    assert_eq!([contact_first.gold, draft_first.gold], [0, 0]);
    assert_ne!(contact_first.rng, draft_first.rng);
    assert_ne!(contact_first.choices, draft_first.choices);
}
