fn gate_break_movement_outcome(reverse: bool) -> (Vec3, Vec3, f32) {
    let mut app = enemy_movement_test_app();
    app.world_mut().resource_mut::<GameSession>().core_hp = 8.813_354_5;
    let hero = app
        .world_mut()
        .query_filtered::<Entity, With<Hero>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .get_mut::<Transform>(hero)
        .unwrap()
        .translation = Vec3::new(-8., 0., 0.);
    let mut attacker = test_enemy(100.);
    attacker.elite = false;
    attacker.speed = 0.;
    attacker.damage = 100.;
    let near = app
        .world_mut()
        .spawn((attacker, Transform::from_xyz(1., 0., 0.)))
        .id();
    let mut mover = test_enemy(100.);
    mover.elite = false;
    mover.speed = 1.;
    mover.damage = 0.;
    let far = app
        .world_mut()
        .spawn((mover, Transform::from_xyz(15., 0., -2.5)))
        .id();
    let insertion = if reverse { [far, near] } else { [near, far] };
    for entity in insertion {
        app.world_mut().entity_mut(entity).insert(CosmeticReaction);
    }
    let rows: Vec<_> = app
        .world_mut()
        .query_filtered::<Entity, With<Enemy>>()
        .iter(app.world())
        .collect();
    assert_eq!(rows, insertion);
    let system = app.world_mut().register_system(move_enemies);
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_millis(100));
    app.world_mut().run_system(system).unwrap();
    let first = app.world().get::<Transform>(far).unwrap().translation;
    let hp = app.world().resource::<GameSession>().core_hp;
    assert_eq!(
        hp, 0.,
        "real near-gate DPS must immediately knock down gate"
    );
    app.world_mut().run_system(system).unwrap();
    let second = app.world().get::<Transform>(far).unwrap().translation;
    println!(
        "GATE_STATE reverse={reverse} rows={rows:?} first={first:?} second={second:?} gate={hp}"
    );
    (first, second, hp)
}

#[test]
fn gate_state_is_consistent_for_every_enemy_in_one_movement_pass() {
    let outcomes = [false, true].map(gate_break_movement_outcome);
    assert_eq!(
        outcomes[0], outcomes[1],
        "cosmetic row order must not change gate state seen by the same mover"
    );
    assert_eq!(outcomes[0].0.z, -2.5, "first movement uses live gate lane");
    assert!(
        outcomes[0].1.z > outcomes[0].0.z,
        "next movement sees broken gate and pursues hero"
    );
}

#[test]
fn gate_damage_ignores_unrelated_enemy_archetype_relocation() {
    #[derive(Component)]
    struct UnrelatedMarker;

    let run = |relocate: bool| {
        let mut app = enemy_movement_test_app();
        app.world_mut().resource_mut::<GameSession>().core_hp = 11.94;
        let mut actors = Vec::new();
        for (index, damage) in [1.0, 1.0, 1000.0].into_iter().enumerate() {
            let mut enemy = test_enemy(100.0);
            enemy.speed = 0.0;
            enemy.damage = damage;
            actors.push(
                app.world_mut()
                    .spawn((
                        enemy,
                        EnemyArchetype(11),
                        Transform::from_xyz(2.0, 0.0, index as f32 - 1.0),
                    ))
                    .id(),
            );
        }
        if relocate {
            // Change storage layout only: same IDs, stats and positions.
            app.world_mut()
                .entity_mut(actors[0])
                .insert(UnrelatedMarker);
            app.world_mut()
                .entity_mut(actors[0])
                .remove::<UnrelatedMarker>();
        }
        let raw_order: Vec<_> = app
            .world_mut()
            .query_filtered::<Entity, With<Enemy>>()
            .iter(app.world())
            .collect();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(1.0 / 60.0));
        app.world_mut().run_system_once(move_enemies).unwrap();
        (
            raw_order,
            app.world().resource::<GameSession>().core_hp.to_bits(),
        )
    };
    let a = run(false);
    let b = run(true);
    assert_ne!(a.0, b.0, "fixture must change raw query order");
    assert_eq!(
        a.1, b.1,
        "storage-only changes must not alter gate damage bits"
    );
}
