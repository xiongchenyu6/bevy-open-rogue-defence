// Include inside combat.rs's existing tests module; not a parallel fixture app.
fn ranged_release_app(kind: HeroKind) -> (App, Entity) {
    let mut app = projectile_test_app();
    app.init_resource::<AttackClock>()
        .init_resource::<AttackVolleyRuntime>()
        .init_resource::<BasicAttackCounter>();
    app.world_mut().resource_mut::<HeroVitals>().kind = kind;
    app.world_mut().resource_mut::<HeroVitals>().level = 1;
    app.world_mut().resource_mut::<PlayerBuild>().attacks_per_second = 1.0;
    let hero = app.world_mut().spawn((Hero, Transform::default())).id();
    app.world_mut().spawn((test_enemy(10000.), Transform::from_xyz(2., 0., 0.)));
    (app, hero)
}
fn ranged_active(app: &mut App) -> usize {
    app.world_mut().query::<&Projectile>().iter(app.world()).count()
}
fn commit_ranged(app: &mut App) {
    app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_secs_f32(1.));
    app.world_mut().run_system_once(auto_attack).unwrap();
}

// BEFORE baseline: this test uses existing production symbols only. Append this
// fixture and test unchanged before applying runtime patch; expected immediate
// projectile count is1, violating the desired zero-until-raised-phase contract.
#[test]
fn native_ranged_does_not_emit_at_attack_commit() {
    for kind in [HeroKind::EmberRanger, HeroKind::StormSavant] {
        let (mut app, _) = ranged_release_app(kind);
        commit_ranged(&mut app);
        assert_eq!(app.world().resource::<BasicAttackCounter>().0, 1);
        assert_eq!(app.world().resource::<AttackVolleyRuntime>().resolved_this_frame, 1);
        assert_eq!(ranged_active(&mut app), 0, "real native projectile emitted before candidate phase: {kind:?}");
    }
}

