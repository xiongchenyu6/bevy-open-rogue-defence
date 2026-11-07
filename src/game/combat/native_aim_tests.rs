fn native_aim_outcome(points: &[Vec3], reverse_allocation: bool) -> (Vec3, Vec2) {
    let mut app = projectile_test_app();
    app.init_resource::<AttackClock>()
        .init_resource::<AttackVolleyRuntime>()
        .init_resource::<BasicAttackCounter>()
        .init_resource::<BasicAttackAim>();
    app.world_mut().resource_mut::<HeroVitals>().kind = HeroKind::StormSavant;
    {
        let mut build = app.world_mut().resource_mut::<PlayerBuild>();
        build.attacks_per_second = 1.0;
        build.attack_range = 10.0;
        build.multishot = 1;
    }
    app.world_mut()
        .spawn((Hero, Transform::from_xyz(-8.2, 0., 0.)));
    let mut positions = points.to_vec();
    if reverse_allocation {
        positions.reverse();
    }
    let actors: Vec<_> = positions
        .iter()
        .map(|point| {
            let mut enemy = test_enemy(192.5);
            enemy.elite = false;
            (
                app.world_mut()
                    .spawn((enemy, Transform::from_translation(*point)))
                    .id(),
                *point,
            )
        })
        .collect();
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs(1));
    app.world_mut().run_system_once(auto_attack).unwrap();
    assert_eq!(app.world().resource::<BasicAttackCounter>().count(), 1);
    let aim = app.world().resource::<BasicAttackAim>().0.unwrap();
    release_native_for_metadata(&mut app, 1, 1);
    let projectile = app
        .world_mut()
        .query::<&Projectile>()
        .single(app.world())
        .unwrap();
    let direction = Vec2::new(projectile.velocity.x, projectile.velocity.z).normalize();
    println!(
        "NATIVE_AIM reverse_allocation={reverse_allocation} actors={actors:?} aim={aim:?} projectile_xz={direction:?}"
    );
    (aim, direction)
}

fn assert_native_aim(actual: (Vec3, Vec2), expected: Vec3) {
    assert_eq!(
        actual.0, expected,
        "actual BasicAttackAim must follow visible target policy"
    );
    let origin = Vec3::new(-8.2, 0., 0.);
    let direction = Vec2::new(expected.x - origin.x, expected.z - origin.z).normalize();
    assert!(
        (actual.1 - direction).length() < 0.000_001,
        "real spawned native projectile must follow the chosen XZ direction"
    );
}

#[test]
fn equal_front_native_attacks_prefer_closest_visible_target() {
    let points = [
        Vec3::new(-3.396_132_7, 0., 0.394_025_92),
        Vec3::new(-3.396_132_7, 0., 1.044_025_9),
        Vec3::new(-3.396_132_7, 0., 1.694_025_9),
    ];
    let forward = native_aim_outcome(&points, false);
    let reverse = native_aim_outcome(&points, true);
    assert_native_aim(forward, points[0]);
    assert_native_aim(reverse, points[0]);
}

#[test]
fn native_front_priority_still_beats_a_closer_enemy_behind_it() {
    let points = [Vec3::new(-4., 0., 3.), Vec3::new(-3.8, 0., 0.)];
    for reverse in [false, true] {
        assert_native_aim(native_aim_outcome(&points, reverse), points[0]);
    }
}

#[test]
fn equal_front_symmetric_native_aim_uses_visible_coordinate_tie() {
    let points = [
        Vec3::new(-3.396_132_7, 0., -1.),
        Vec3::new(-3.396_132_7, 0., 1.),
    ];
    let forward = native_aim_outcome(&points, false);
    let reverse = native_aim_outcome(&points, true);
    assert_native_aim(forward, points[0]);
    assert_native_aim(reverse, points[0]);
}

#[test]
fn dagger_native_contacts_wait_for_slice_and_keep_one_real_damage_event() {
    for kind in [HeroKind::RiftDancer, HeroKind::TideWalker] {
        let mut app = projectile_test_app();
        app.init_resource::<AttackClock>()
            .init_resource::<AttackVolleyRuntime>()
            .init_resource::<BasicAttackCounter>()
            .init_resource::<BasicAttackAim>();
        app.world_mut().resource_mut::<HeroVitals>().kind = kind;
        {
            let mut build = app.world_mut().resource_mut::<PlayerBuild>();
            build.attacks_per_second = 1.;
            build.attack_damage = 25.;
        }
        app.world_mut().spawn((Hero, Transform::default()));
        let mut enemy = test_enemy(100.);
        enemy.elite = false;
        let target = app
            .world_mut()
            .spawn((enemy, Transform::from_xyz(1., 0., 0.)))
            .id();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(1));
        app.world_mut().run_system_once(auto_attack).unwrap();
        assert_eq!(app.world().resource::<BasicAttackCounter>().count(), 1);
        let window = app
            .world()
            .resource::<AttackClock>()
            .0
            .duration()
            .as_secs_f32()
            .min(0.45);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(window * 0.46));
        app.world_mut()
            .run_system_once(basic_attacks::resolve_swings)
            .unwrap();
        assert_eq!(
            app.world_mut()
                .query::<&Projectile>()
                .iter(app.world())
                .count(),
            0,
            "no apex contact carrier"
        );
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(window * 0.05));
        app.world_mut()
            .run_system_once(basic_attacks::resolve_swings)
            .unwrap();
        assert_eq!(
            app.world_mut()
                .query::<&Projectile>()
                .iter(app.world())
                .count(),
            1,
            "one actual slice carrier"
        );
        app.world_mut().run_system_once(projectile_hits).unwrap();
        assert_eq!(app.world().get::<Enemy>(target).unwrap().hp, 75.);
        app.world_mut()
            .run_system_once(basic_attacks::resolve_swings)
            .unwrap();
        app.world_mut().run_system_once(projectile_hits).unwrap();
        assert_eq!(
            app.world().get::<Enemy>(target).unwrap().hp,
            75.,
            "no duplicate damage during recovery"
        );
        assert_eq!(app.world().resource::<BasicAttackCounter>().count(), 1);
    }
}
