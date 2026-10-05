//! Shows real learned-skill casts and status effects against a durable formation.
use super::CaptureControls;
use bevy::prelude::*;
use rogue_commander::game::{
    ActiveSkill, Autoplay, Enemy, GameAssets, GameSession, Hero, HeroKind, PlayerBuild,
    SelectedHero, SpawnDirector,
};

#[derive(Resource)]
struct SkillSpectacleCapture {
    frame: u32,
    scenario: String,
    saw_effect: bool,
    saw_damage: bool,
}

#[derive(Component)]
struct SpectacleTarget;

pub fn configure(world: &mut World, scenario: &str) {
    let kind = match scenario.strip_prefix("skill-spectacle-").unwrap() {
        "fire" => HeroKind::EmberRanger,
        "frost" => HeroKind::FrostOracle,
        "virus" => HeroKind::VoidReaper,
        "buff" | "combo" => HeroKind::StormSavant,
        unknown => panic!("unknown skill spectacle capture: {unknown}"),
    };
    let mut selected = world.resource_mut::<SelectedHero>();
    selected.kind = kind;
    selected.locked = true;
    world.resource_mut::<CaptureControls>().suppress_drafts = true;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.insert_resource(SkillSpectacleCapture {
        frame: 0,
        scenario: scenario.to_owned(),
        saw_effect: false,
        saw_damage: false,
    });
}

pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    world.remove_resource::<Autoplay>();
    let scenario = world.resource::<SkillSpectacleCapture>().scenario.clone();
    let skills: &[usize] = match scenario.strip_prefix("skill-spectacle-").unwrap() {
        "fire" => &[1, 11, 13],
        "frost" => &[2, 14],
        "virus" => &[18],
        "buff" => &[0, 20, 21, 28],
        "combo" => &[0, 1, 2, 11, 13, 14, 18, 20, 21, 28],
        _ => unreachable!(),
    };
    let mut build = world.resource_mut::<PlayerBuild>();
    build.skills.fill(0);
    for &skill in skills {
        build.skills[skill] = 5;
    }
    build.attacks_per_second = 2.0;
    world.resource_mut::<ActiveSkill>().remaining = 10_000.0;
    let mut heroes = world.query_filtered::<&mut Transform, With<Hero>>();
    for mut hero in heroes.iter_mut(world) {
        hero.translation = Vec3::new(-5.0, 0.0, 0.0);
    }
    let mut cameras = world.query_filtered::<(&mut Transform, &mut Projection), With<Camera3d>>();
    for (mut transform, mut projection) in cameras.iter_mut(world) {
        *transform =
            Transform::from_xyz(-2.5, 18.0, 15.0).looking_at(Vec3::new(-2.5, 0.0, 0.0), Vec3::Y);
        if let Projection::Orthographic(projection) = &mut *projection {
            projection.scaling_mode = bevy::camera::ScalingMode::FixedVertical {
                viewport_height: 16.0,
            };
        }
    }
    let scene = world.resource::<GameAssets>().enemy_scenes[0].clone();
    for (index, position) in [
        Vec3::new(-3.0, 0.0, 0.0),
        Vec3::new(-1.5, 0.0, -1.8),
        Vec3::new(-1.5, 0.0, 1.8),
        Vec3::new(0.0, 0.0, -0.8),
        Vec3::new(0.0, 0.0, 0.8),
        Vec3::new(2.0, 0.0, -1.8),
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 1.8),
    ]
    .into_iter()
    .enumerate()
    {
        world.spawn((
            Name::new(format!("Skill spectacle target {index}")),
            SpectacleTarget,
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
    println!("[capture] {scenario}: rank-5 skills {skills:?}, real casts, ultimates suppressed");
    app.add_systems(Last, verify);
}

fn verify(
    mut capture: ResMut<SkillSpectacleCapture>,
    names: Query<&Name>,
    targets: Query<&Enemy, With<SpectacleTarget>>,
    active: Res<ActiveSkill>,
) {
    capture.frame += 1;
    capture.saw_effect |= names
        .iter()
        .any(|name| name.as_str() == "Skill spectacle effect");
    capture.saw_damage |= targets.iter().any(|enemy| enemy.hp < enemy.max_hp);
    if capture.frame == 265 {
        assert!(
            capture.saw_effect,
            "real skill casts must create spectacle effects"
        );
        assert!(
            capture.saw_damage,
            "real combat must damage the fixture enemies"
        );
        assert!(
            active.remaining > 9_000.0,
            "ultimate must remain suppressed"
        );
        println!(
            "[capture] PASS {}: real damage and skill spectacle effects; no ultimate",
            capture.scenario
        );
    }
}
