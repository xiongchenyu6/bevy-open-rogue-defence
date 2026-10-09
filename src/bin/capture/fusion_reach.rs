//! Explicit fixture legacies; actual clock/emitter/contact, never earned progression.
use bevy::{app::MainScheduleOrder, ecs::schedule::ScheduleLabel, prelude::*};

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct CaptureAfterCombat;
use rogue_commander::game::{
    ActiveSkill, Autoplay, BasicAttackCounter, Enemy, GameAssets, GameSession, Hero, HeroFusion,
    HeroKind, HeroVitals, PlayerBuild, SelectedHero, SpawnDirector,
};
#[derive(Component)]
struct ReachTarget;
#[derive(Resource, Default)]
struct ReachCapture {
    frame: u32,
    damaged: bool,
    saw_projectile: bool,
}
pub fn configure(world: &mut World) {
    world.remove_resource::<Autoplay>();
    {
        let mut selected = world.resource_mut::<SelectedHero>();
        selected.locked = true;
        selected.kind = HeroKind::StoneWarden;
    }
    world.resource_mut::<SpawnDirector>().enabled = false;
    {
        let mut session = world.resource_mut::<GameSession>();
        session.level = 1;
        session.elapsed = 0.0;
        session.skill_points = 0;
        session.auto_bond_draws = false;
    }
    world.insert_resource(ReachCapture::default());
}
pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    world.remove_resource::<Autoplay>();
    world.resource_mut::<ActiveSkill>().remaining = 10_000.0;
    {
        let mut build = world.resource_mut::<PlayerBuild>();
        build.skills.fill(0);
        build.attacks_per_second = 2.0;
    }
    for kind in [HeroKind::EmberRanger, HeroKind::StormSavant] {
        world.resource_mut::<HeroFusion>().record_absorbed(kind);
    }
    for mut hero in world
        .query_filtered::<&mut Transform, With<Hero>>()
        .iter_mut(world)
    {
        hero.translation = Vec3::new(-8.2, 0.0, 0.0);
    }
    let scene = world.resource::<GameAssets>().enemy_scenes[0].clone();
    world.spawn((
        Name::new("Legacy reach durable target"),
        ReachTarget,
        WorldAssetRoot(scene),
        Transform::from_xyz(-3.2, 0.0, 0.0).with_scale(Vec3::splat(0.85)),
        Enemy {
            hp: 100_000.0,
            max_hp: 100_000.0,
            speed: 0.0,
            damage: 0.0,
            xp: 0,
            wood: 0,
            gold: 0,
            kill_credit: 0,
            elite: false,
        },
    ));
    for (mut camera, mut projection) in world
        .query_filtered::<(&mut Transform, &mut Projection), With<Camera3d>>()
        .iter_mut(world)
    {
        *camera =
            Transform::from_xyz(-5.7, 12.0, 10.0).looking_at(Vec3::new(-5.7, 0.0, 0.0), Vec3::Y);
        if let Projection::Orthographic(p) = &mut *projection {
            p.scaling_mode = bevy::camera::ScalingMode::FixedVertical {
                viewport_height: 11.0,
            };
        }
    }
    // This explicit fixture has no ability/passive inheritance. Keep selection
    // locked during production combat/UI, then suppress the PostUpdate ability
    // synchronization/cast path without altering private production cooldowns.
    app.world_mut()
        .resource_mut::<MainScheduleOrder>()
        .insert_after(Update, CaptureAfterCombat);
    app.add_systems(First, |mut selected: ResMut<SelectedHero>| {
        selected.locked = true
    });
    app.add_systems(CaptureAfterCombat, |mut selected: ResMut<SelectedHero>| {
        selected.locked = false
    });
    app.add_systems(PostUpdate, exercise);
}
#[allow(clippy::too_many_arguments)]
fn exercise(
    mut state: ResMut<ReachCapture>,
    fusion: Res<HeroFusion>,
    vitals: Res<HeroVitals>,
    native: Res<BasicAttackCounter>,
    build: Res<PlayerBuild>,
    heroes: Query<&Transform, With<Hero>>,
    targets: Query<(&Enemy, &Transform), With<ReachTarget>>,
    projectiles: Query<&Name, With<rogue_commander::game::Projectile>>,
    names: Query<&Name>,
) {
    state.frame += 1;
    assert!(
        names
            .iter()
            .all(|name| name.as_str() != "Hero ultimate effect"),
        "isolated fixture must emit no ultimate visual entity"
    );
    assert_eq!(vitals.kind, HeroKind::StoneWarden);
    assert!(build.skills.iter().all(|rank| *rank == 0));
    assert_eq!(native.count(), 0, "no fake native hit outside host reach");
    let hero = heroes.single().unwrap();
    let (enemy, target) = targets.single().unwrap();
    let distance = hero.translation.distance(target.translation);
    assert!(distance > 2.2 && distance <= build.attack_range);
    state.damaged |= enemy.hp < enemy.max_hp;
    state.saw_projectile |= projectiles
        .iter()
        .any(|name| name.as_str() == "Fusion legacy projectile");
    if state.frame == 390 {
        for kind in [HeroKind::EmberRanger, HeroKind::StormSavant] {
            assert!(
                fusion.attacks_emitted[kind.index()] > 0,
                "missing {kind:?} legacy"
            );
        }
        assert!(state.damaged && state.saw_projectile);
        println!(
            "[capture] PASS: explicit fixture legacies bow={} staff={}, real targetHP={}/{}, native attacks=0, reach={}m, skills=0, ultimate visual entities=0, normal clock",
            fusion.attacks_emitted[HeroKind::EmberRanger.index()],
            fusion.attacks_emitted[HeroKind::StormSavant.index()],
            enemy.hp,
            enemy.max_hp,
            distance
        );
    }
}
