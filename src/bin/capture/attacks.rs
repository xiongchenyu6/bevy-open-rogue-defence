//! All twelve heroes' real basic attacks, isolated from learned skills and ultimates.
use bevy::prelude::*;
use rogue_commander::game::{
    ActiveSkill, Autoplay, Enemy, GameAssets, Hero, HeroKind, PlayerBuild, SelectedHero,
};

#[derive(Component)]
struct AttackTarget;

/// `-fragile` targets die in a few hits and revive, exercising kill feedback.
const FRAGILE_HP: f32 = 90.0;
const FRAGILE_REVIVE_FRAMES: u32 = 24;

#[derive(Component, Default)]
struct FragileRevive(u32);

#[derive(Resource)]
struct AttackCapture {
    frame: u32,
    route_fixture: bool,
    saw_damage: bool,
    saw_visual: bool,
}

pub fn install(app: &mut bevy::app::SubApp, scenario: &str) {
    let world = app.world_mut();
    world.remove_resource::<Autoplay>();
    world.resource_mut::<PlayerBuild>().skills.fill(0);
    world.resource_mut::<ActiveSkill>().remaining = 10_000.0;
    // Keep the pre-existing optional bond-route comparisons meaningful.
    if scenario == "attack-arcanum" {
        world.resource_mut::<PlayerBuild>().skills[0] = 1;
    }
    let kind = world.resource::<SelectedHero>().kind;
    let melee = matches!(
        kind,
        HeroKind::StoneWarden
            | HeroKind::SolarVanguard
            | HeroKind::TideWalker
            | HeroKind::RiftDancer
            | HeroKind::IronBerserker
            | HeroKind::VoidReaper
    );
    let mut heroes = world.query_filtered::<&mut Transform, With<Hero>>();
    for mut hero in heroes.iter_mut(world) {
        hero.translation = Vec3::new(-2.0, 0.0, 0.0);
    }
    let center = if melee { -1.0 } else { 0.5 };
    let mut cameras = world.query_filtered::<(&mut Transform, &mut Projection), With<Camera3d>>();
    for (mut transform, mut projection) in cameras.iter_mut(world) {
        *transform = Transform::from_xyz(center, 13.0, 11.0)
            .looking_at(Vec3::new(center, 0.0, 0.0), Vec3::Y);
        if let Projection::Orthographic(projection) = &mut *projection {
            projection.scaling_mode = bevy::camera::ScalingMode::FixedVertical {
                viewport_height: 10.0,
            };
        }
    }
    let scene = world.resource::<GameAssets>().enemy_scenes[0].clone();
    let fragile = scenario.ends_with("-fragile");
    let target_hp = if fragile { FRAGILE_HP } else { 100_000.0 };
    for index in 0..3 {
        let position = Vec3::new(
            if melee { -0.7 } else { 3.0 },
            0.0,
            (index as f32 - 1.0) * 1.2,
        );
        let mut target = world.spawn((
            Name::new(format!("Basic attack capture target {index}")),
            AttackTarget,
            WorldAssetRoot(scene.clone()),
            Transform::from_translation(position).with_scale(Vec3::splat(0.8)),
            Enemy {
                hp: target_hp,
                max_hp: target_hp,
                speed: 0.0,
                damage: 0.0,
                xp: 0,
                wood: 0,
                gold: 0,
                kill_credit: 0,
                elite: false,
            },
        ));
        if fragile {
            target.insert(FragileRevive::default());
        }
    }
    println!(
        "[capture] {scenario}: centered battlefield, real {kind:?} basic attacks; no Autoplay or ultimate"
    );
    app.insert_resource(AttackCapture {
        frame: 0,
        route_fixture: scenario == "attack-arcanum",
        saw_damage: false,
        saw_visual: false,
    })
    .add_systems(Last, (verify, revive_fragile_targets));
}

fn revive_fragile_targets(mut targets: Query<(&mut Enemy, &mut FragileRevive)>) {
    for (mut enemy, mut revive) in &mut targets {
        if enemy.hp > 0.0 {
            continue;
        }
        revive.0 += 1;
        if revive.0 >= FRAGILE_REVIVE_FRAMES {
            enemy.hp = enemy.max_hp;
            revive.0 = 0;
        }
    }
}

fn verify(
    mut capture: ResMut<AttackCapture>,
    names: Query<&Name>,
    enemies: Query<&Enemy, With<AttackTarget>>,
    build: Res<PlayerBuild>,
    active: Res<ActiveSkill>,
    autoplay: Option<Res<Autoplay>>,
) {
    capture.frame += 1;
    capture.saw_damage |= enemies.iter().any(|enemy| enemy.hp < enemy.max_hp);
    capture.saw_damage |= capture.frame > 1 && enemies.iter().any(|enemy| enemy.hp <= 0.0);
    capture.saw_visual |= names.iter().any(|name| {
        matches!(
            name.as_str(),
            "Weapon trail / impact" | "Weapon swing trail"
        )
    });
    if capture.frame == 205 {
        assert!(capture.saw_damage, "real basic attacks must damage enemies");
        assert!(
            capture.saw_visual,
            "real basic attacks must produce weapon visuals"
        );
        assert!(autoplay.is_none());
        assert!(
            active.remaining > 9_000.0,
            "automatic ultimate must remain suppressed"
        );
        if !capture.route_fixture {
            assert!(build.skills.iter().all(|rank| *rank == 0));
        }
        println!(
            "[capture] PASS: real basic attack damage and weapon visuals; learned skills/ultimate/Autoplay suppressed"
        );
    }
}
