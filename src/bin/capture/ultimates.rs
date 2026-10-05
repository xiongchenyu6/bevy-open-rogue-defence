//! Real automatic ultimate casts, with stationary durable enemies for visual review.
use super::CaptureControls;
use bevy::prelude::*;
use rogue_commander::game::{
    ActiveSkill, Autoplay, Enemy, GameAssets, GameSession, Hero, HeroKind, PlayerBuild,
    SelectedHero, SpawnDirector,
};

#[derive(Resource)]
struct UltimateCapture {
    frame: u32,
    previous_remaining: f32,
    casts: u32,
}

pub fn configure(world: &mut World, scenario: &str) {
    let kind = match scenario.strip_prefix("ultimate-").unwrap() {
        "storm" => HeroKind::StormSavant,
        "ember" => HeroKind::EmberRanger,
        "stone" => HeroKind::StoneWarden,
        "spirit" => HeroKind::SpiritKeeper,
        "tide" => HeroKind::TideWalker,
        "star" => HeroKind::StarWagerer,
        "frost" => HeroKind::FrostOracle,
        "iron" => HeroKind::IronBerserker,
        "rift" => HeroKind::RiftDancer,
        "grove" => HeroKind::GroveArtificer,
        "solar" => HeroKind::SolarVanguard,
        "void" => HeroKind::VoidReaper,
        unknown => panic!("unknown ultimate capture: {unknown}"),
    };
    let mut selected = world.resource_mut::<SelectedHero>();
    selected.kind = kind;
    selected.locked = true;
    world.resource_mut::<CaptureControls>().suppress_drafts = true;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
    world.resource_mut::<SpawnDirector>().enabled = false;
    println!("[capture] {scenario}: automatic ultimate, no manual input");
}

pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    world.remove_resource::<Autoplay>();
    world.resource_mut::<PlayerBuild>().skills.fill(0);
    world.resource_mut::<PlayerBuild>().attacks_per_second = 0.05;
    world.resource_mut::<ActiveSkill>().remaining = 1.0;
    let mut heroes = world.query_filtered::<&mut Transform, With<Hero>>();
    for mut hero in heroes.iter_mut(world) {
        hero.translation = Vec3::new(-3.0, 0.0, 0.0);
    }
    let mut cameras = world.query_filtered::<(&mut Transform, &mut Projection), With<Camera3d>>();
    for (mut transform, mut projection) in cameras.iter_mut(world) {
        *transform = Transform::from_xyz(0.0, 18.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y);
        if let Projection::Orthographic(projection) = &mut *projection {
            projection.scaling_mode = bevy::camera::ScalingMode::FixedVertical {
                viewport_height: 16.0,
            };
        }
    }
    let scene = world.resource::<GameAssets>().enemy_scenes[0].clone();
    for (index, position) in [
        Vec3::new(-1.0, 0.0, 0.0),
        Vec3::new(0.8, 0.0, -2.4),
        Vec3::new(0.8, 0.0, 2.4),
        Vec3::new(2.8, 0.0, -1.2),
        Vec3::new(2.8, 0.0, 1.2),
        Vec3::new(5.0, 0.0, -2.4),
        Vec3::new(5.0, 0.0, 0.0),
        Vec3::new(5.0, 0.0, 2.4),
    ]
    .into_iter()
    .enumerate()
    {
        world.spawn((
            Name::new(format!("Ultimate capture target {index}")),
            WorldAssetRoot(scene.clone()),
            Transform::from_translation(position).with_scale(Vec3::splat(0.8)),
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
    }
    app.insert_resource(UltimateCapture {
        frame: 0,
        previous_remaining: 1.0,
        casts: 0,
    })
    .add_systems(Last, verify);
}

fn verify(
    mut capture: ResMut<UltimateCapture>,
    active: Res<ActiveSkill>,
    selected: Res<SelectedHero>,
) {
    capture.frame += 1;
    if active.remaining > capture.previous_remaining + 1.0 {
        capture.casts += 1;
        println!(
            "[capture] {:?} automatically cast at frame {}, cooldown {:.2}",
            selected.kind, capture.frame, active.remaining
        );
    }
    capture.previous_remaining = active.remaining;
    if capture.frame == 90 {
        assert_eq!(capture.casts, 1, "ultimate must cast without input");
        assert!(active.remaining > 0.0 && active.remaining < active.cooldown);
    }
    if capture.frame == 205 {
        assert_eq!(capture.casts, 1, "ultimate cannot repeat during cooldown");
        assert!(active.remaining > 0.0 && active.remaining < active.cooldown - 3.0);
        println!("[capture] PASS: one automatic cast; cooldown advances without repeated casts");
    }
}
