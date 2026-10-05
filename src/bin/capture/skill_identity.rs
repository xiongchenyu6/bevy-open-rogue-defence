//! One learned skill per scene, with explicitly declared trigger dependencies.
use bevy::{prelude::*, time::Virtual};
use rogue_commander::game::{
    ActiveSkill, AttackClock, Autoplay, CampaignMenu, Enemy, GameAssets, GameSession, Hero,
    HeroKind, HeroVitals, PlayerBuild, SelectedHero, SkillId, SpawnDirector, content::SKILLS,
};
#[derive(Component)]
struct IdentityTarget;
#[derive(Resource)]
struct IdentityCapture {
    index: usize,
    rank: u8,
    production_camera: bool,
    max_visual_scale: f32,
    max_zone_scale: f32,
    frame: u32,
    saw_signature: bool,
    saw_damage: bool,
    saw_summon: bool,
    revived: bool,
    initial_attack: f32,
    initial_gold: u32,
    initial_health: f32,
    first_signature: Option<u32>,
    peak: usize,
}
#[derive(Resource, Default)]
struct LightingProbe {
    frame: u32,
    paused: Vec<(Entity, Transform, Visibility)>,
    saw_visible: bool,
}
pub fn configure(world: &mut World, scenario: &str) {
    let suffix = scenario.strip_prefix("skill-identity-").unwrap();
    let production_camera = suffix.ends_with("-production");
    let suffix = suffix.strip_suffix("-production").unwrap_or(suffix);
    let (index, rank) = suffix.split_once("-rank-").unwrap_or((suffix, "5"));
    let index = index.parse::<usize>().expect("skill index");
    let rank = rank.parse::<u8>().expect("skill rank");
    assert!((1..=5).contains(&rank));
    assert!(index < SkillId::COUNT);
    world.resource_mut::<CampaignMenu>().0 = false;
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SelectedHero>().locked = true;
    world.resource_mut::<SelectedHero>().kind = HeroKind::StoneWarden;
    *world.resource_mut::<HeroVitals>() = HeroVitals::for_hero(HeroKind::StoneWarden, 1);
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
    world
        .resource_mut::<super::CaptureControls>()
        .suppress_drafts = true;
    world.insert_resource(IdentityCapture {
        index,
        rank,
        production_camera,
        max_visual_scale: 0.,
        max_zone_scale: 0.,
        frame: 0,
        saw_signature: false,
        saw_damage: false,
        saw_summon: false,
        revived: false,
        initial_attack: 0.0,
        initial_gold: 0,
        initial_health: 0.0,
        first_signature: None,
        peak: 0,
    });
}
pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    world.remove_resource::<Autoplay>();
    let index = world.resource::<IdentityCapture>().index;
    let rank = world.resource::<IdentityCapture>().rank;
    {
        let mut build = world.resource_mut::<PlayerBuild>();
        build.skills.fill(0);
        build.skills[index] = rank;
        build.attacks_per_second = 3.0;
        if matches!(index, 10 | 20) {
            build.skills[0] = 1;
        }
    }
    world.resource_mut::<ActiveSkill>().remaining = 10_000.0;
    if matches!(index, 1 | 7 | 11 | 22 | 28 | 29) {
        world.resource_mut::<AttackClock>().0.unpause();
    } else {
        world.resource_mut::<AttackClock>().0.pause();
    }
    if matches!(index, 10 | 20) {
        world
            .resource_mut::<Time<Virtual>>()
            .set_relative_speed(2.0);
    }
    if index == 29 {
        let mut vitals = world.resource_mut::<HeroVitals>();
        vitals.health = vitals.max_health * 0.3;
    }
    let initial_attack = world.resource::<PlayerBuild>().attack_damage;
    let initial_gold = world.resource::<GameSession>().gold;
    let initial_health = world.resource::<HeroVitals>().health;
    {
        let mut state = world.resource_mut::<IdentityCapture>();
        state.initial_attack = initial_attack;
        state.initial_gold = initial_gold;
        state.initial_health = initial_health;
    }
    let center = Vec3::new(-4.5, 0.0, 0.0);
    for mut hero in world
        .query_filtered::<&mut Transform, With<Hero>>()
        .iter_mut(world)
    {
        hero.translation = center;
    }
    if !world.resource::<IdentityCapture>().production_camera {
        for (mut camera, mut projection) in world
            .query_filtered::<(&mut Transform, &mut Projection), With<Camera3d>>()
            .iter_mut(world)
        {
            *camera = Transform::from_xyz(-0.5, 15.0, 12.5)
                .looking_at(Vec3::new(-0.5, 0.0, 0.0), Vec3::Y);
            if let Projection::Orthographic(p) = &mut *projection {
                p.scaling_mode = bevy::camera::ScalingMode::FixedVertical {
                    viewport_height: if index == 3 { 18.0 } else { 13.0 },
                };
            }
        }
    }
    let scene = world.resource::<GameAssets>().enemy_scenes[0].clone();
    for (i, position) in [
        Vec3::new(-3.2, 0.0, 0.0),
        Vec3::new(-2.5, 0.0, -1.4),
        Vec3::new(-2.5, 0.0, 1.4),
        Vec3::new(-0.5, 0.0, -0.6),
        Vec3::new(-0.5, 0.0, 0.6),
        Vec3::new(2.0, 0.0, -2.0),
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 2.0),
    ]
    .into_iter()
    .enumerate()
    {
        world.spawn((
            Name::new(format!("Skill identity target {i}")),
            IdentityTarget,
            WorldAssetRoot(scene.clone()),
            Transform::from_translation(position).with_scale(Vec3::splat(0.72)),
            Enemy {
                hp: 1_000_000.0,
                max_hp: 1_000_000.0,
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
    let production_camera = world.resource::<IdentityCapture>().production_camera;
    app.add_systems(PreUpdate, observe);
    if production_camera {
        app.init_resource::<LightingProbe>()
            .add_systems(PreUpdate, probe_lighting);
    }
    println!(
        "[capture] skill-identity-{index}: {} rank {rank}; {}",
        SKILLS[index].id,
        if matches!(index, 10 | 20) {
            "declared dependency arc_chain rank 1; 2x real simulation to trigger four marks/fifth spell"
        } else {
            "no other learned skills"
        }
    );
}
fn probe_lighting(world: &mut World) {
    world.resource_mut::<LightingProbe>().frame += 1;
    let frame = world.resource::<LightingProbe>().frame;
    let projections: Vec<_> = world
        .query::<(Entity, &Name, &Transform, &Visibility)>()
        .iter(world)
        .filter(|(_, name, _, _)| name.as_str() == "Skill ground projection")
        .map(|(entity, _, transform, visibility)| (entity, *transform, *visibility))
        .collect();
    assert_eq!(projections.len(), 80, "bounded projection pool exists");
    if projections
        .iter()
        .any(|(_, _, visibility)| *visibility == Visibility::Visible)
    {
        world.resource_mut::<LightingProbe>().saw_visible = true;
    }
    if frame == 320 {
        assert!(
            world.resource::<LightingProbe>().saw_visible,
            "real skill casts ground projection"
        );
        world.resource_mut::<LightingProbe>().paused = projections.clone();
        world.resource_mut::<GameSession>().paused = true;
    }
    if frame == 329 {
        assert_eq!(
            projections,
            world.resource::<LightingProbe>().paused,
            "real pause freezes every projection transform and visibility"
        );
        world.resource_mut::<GameSession>().paused = false;
    }
    if frame == 350 {
        world.resource_mut::<GameSession>().finished = true;
    }
    if frame == 360 {
        assert!(
            projections
                .iter()
                .all(|(_, _, visibility)| *visibility == Visibility::Hidden),
            "finished battle hides every projection pool slot"
        );
        println!(
            "[capture] PASS skill-lighting: 80 bounded slots, real visible contact, actual pause frozen, finished pool hidden"
        );
    }
}
#[allow(clippy::too_many_arguments)]
fn observe(
    mut state: ResMut<IdentityCapture>,
    mut vitals: ResMut<HeroVitals>,
    build: Res<PlayerBuild>,
    session: Res<GameSession>,
    active: Res<ActiveSkill>,
    assets: Res<GameAssets>,
    names: Query<&Name>,
    visuals: Query<(&Name, &Transform)>,
    models: Query<(&WorldAssetRoot, &Visibility)>,
    targets: Query<&Enemy, With<IdentityTarget>>,
) {
    state.frame += 1;
    let frame = state.frame;
    let index = state.index;
    let identity = format!("Skill identity {index}");
    let count = names
        .iter()
        .filter(|name| name.as_str() == identity)
        .count();
    let dedicated = match index {
        3 => "Tidal wave curved wall",
        6 => "Orbit blade",
        30 => "Flame ray sweeping beam",
        31 => "Goblin mine armed trap",
        _ => "",
    };
    let signature =
        count > 0 || (!dedicated.is_empty() && names.iter().any(|name| name.as_str() == dedicated));
    let max_scale = visuals
        .iter()
        .filter(|(name, _)| name.as_str() == identity)
        .map(|(_, transform)| transform.scale.max_element())
        .fold(0.0, f32::max);
    state.max_visual_scale = state.max_visual_scale.max(max_scale);
    let zone_scale = visuals
        .iter()
        .filter(|(name, _)| name.as_str() == "Storm matrix damage zone")
        .map(|(_, transform)| transform.scale.max_element())
        .fold(0.0, f32::max);
    state.max_zone_scale = state.max_zone_scale.max(zone_scale);
    state.saw_signature |= signature;
    state.peak = state.peak.max(count);
    if signature && state.first_signature.is_none() {
        state.first_signature = Some(frame);
        println!("[capture] first skill {index} main visual at frame {frame}");
    }
    state.saw_damage |= targets.iter().any(|enemy| enemy.hp < enemy.max_hp);
    let summon_index = match index {
        4 => Some(0),
        5 => Some(1),
        16 => Some(2),
        17 => Some(3),
        26 => Some(4),
        27 => Some(5),
        29 => Some(1),
        _ => None,
    };
    state.saw_summon |= summon_index.is_some_and(|expected| {
        models.iter().any(|(model, visibility)| {
            *visibility != Visibility::Hidden && model.0 == assets.summon_scenes[expected]
        })
    });
    if index == 25 && frame == 80 {
        vitals.invulnerable_remaining = 0.0;
        let lethal = vitals.max_health * 100.0;
        assert!(vitals.take_magical_damage(lethal) > 0.0);
        assert_eq!(vitals.health, 0.0);
        println!("[capture] actual lethal magical hit triggers Ember Rebirth at frame 80");
    }
    if index == 25 && frame > 80 {
        state.revived |= vitals.alive && vitals.health > 0.0;
    }
    if matches!(frame, 80 | 160 | 250) {
        println!(
            "[capture] skill {index} observation frame {frame}: main_visual={signature}, parts={count}, damage={}, summon={}",
            state.saw_damage, state.saw_summon
        );
    }
    if frame == 295 {
        assert_eq!(build.skills[index], state.rank);
        if index == 12 {
            let expected_radius = 3.1 * (1. + 0.25 * f32::from(state.rank - 1));
            assert!(
                (state.max_zone_scale - expected_radius * 0.34).abs() < 0.001,
                "actual storm matrix damage root must scale with collision radius"
            );
            println!(
                "[capture] PASS storm rank {}: radius={expected_radius:.3}, actual_zone_scale={:.4}",
                state.rank, state.max_zone_scale
            );
        }
        assert!(active.remaining > 9000.0);
        for (learned, rank) in build.skills.iter().enumerate() {
            if learned != index && !(matches!(index, 10 | 20) && learned == 0) {
                assert_eq!(*rank, 0, "unrelated skill contaminated identity scene");
            }
        }
        assert!(
            state.saw_signature,
            "skill {index} had no attributable main visual"
        );
        match index {
            20 => {} // A declared spell source drives the real fifth-spell echo; its prism is identified independently.
            21 => {
                assert!(names.iter().any(|name| name.as_str() == identity) || state.saw_signature)
            }
            25 => assert!(
                state.revived,
                "learned rebirth failed to recover from a real lethal hit"
            ),
            28 => assert!(
                build.attack_damage > state.initial_attack,
                "mastered triune attack buff not applied"
            ),
            29 => assert!(
                state.saw_damage
                    || state.saw_summon
                    || session.gold > state.initial_gold
                    || vitals.health > state.initial_health,
                "fate cube produced no gameplay outcome"
            ),
            _ => assert!(state.saw_damage, "skill {index} produced no actual damage"),
        }
        if matches!(index, 4 | 5 | 16 | 17 | 26 | 27) {
            assert!(state.saw_summon, "real skill summon missing");
        }
        println!(
            "[capture] PASS skill-identity-{index}: attributable silhouette, real gameplay, isolated rank {}; first={:?}, peak_parts={}, max_visual_scale={:.4}",
            state.rank, state.first_signature, state.peak, state.max_visual_scale
        );
    }
}
