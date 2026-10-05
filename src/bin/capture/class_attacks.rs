//! Genuine class attacks and two actual fruit recruitments, with production camera.
use bevy::prelude::*;
use rogue_commander::game::{
    ActiveSkill, AttackClock, Autoplay, Enemy, EnemyArchetype, GameAssets, GameSession, Hero,
    HeroFusion, HeroKind, HeroVitals, PlayerBuild, Projectile, RecruitmentProgress, Ricochet,
    SelectedHero, SpawnDirector, class_basic_stats, content::HEROES,
};
#[derive(Component)]
struct ClassTarget(usize);
#[derive(Resource)]
struct ClassProbe {
    scenario: String,
    frame: u32,
    kind: HeroKind,
    level: u32,
    legacy: bool,
    saw_volley: bool,
    saw_bounce: bool,
    saw_cleave: bool,
    paused_at: Option<u32>,
    frozen: Vec<(Entity, Transform)>,
    frozen_hp: Vec<f32>,
}
pub fn configure(world: &mut World, scenario: &str) {
    let (kind, level, legacy) = match scenario {
        "class-archer-1" => (HeroKind::EmberRanger, 1, false),
        "class-archer-40" => (HeroKind::EmberRanger, 40, false),
        "class-sword" => (HeroKind::StoneWarden, 1, false),
        "class-axe" => (HeroKind::IronBerserker, 1, false),
        "class-inheritance" => (HeroKind::StoneWarden, 1, true),
        _ => panic!("unknown class attack scene {scenario}"),
    };
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SelectedHero>().kind = kind;
    world.resource_mut::<SelectedHero>().locked = scenario != "class-archer-1";
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
    world
        .resource_mut::<super::CaptureControls>()
        .suppress_drafts = true;
    world.insert_resource(ClassProbe {
        scenario: scenario.into(),
        frame: 0,
        kind,
        level,
        legacy,
        saw_volley: false,
        saw_bounce: false,
        saw_cleave: false,
        paused_at: None,
        frozen: vec![],
        frozen_hp: vec![],
    });
}
pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    let (level, legacy, kind) = {
        let p = world.resource::<ClassProbe>();
        (p.level, p.legacy, p.kind)
    };
    world.resource_mut::<PlayerBuild>().skills.fill(0);
    world.resource_mut::<ActiveSkill>().remaining = 10000.;
    world.resource_mut::<AttackClock>().0.pause();
    world.resource_mut::<GameSession>().level = level;
    world
        .resource_mut::<RecruitmentProgress>()
        .claimed_milestones = 7;
    if legacy {
        world.resource_mut::<RecruitmentProgress>().fruit_count = 2;
    }
    for mut hero in world
        .query_filtered::<&mut Transform, With<Hero>>()
        .iter_mut(world)
    {
        hero.translation = Vec3::ZERO;
    }
    let scenes = world.resource::<GameAssets>().enemy_scenes.clone();
    let positions = if kind == HeroKind::EmberRanger {
        vec![
            (5., 0.),
            (3.5, -2.5),
            (5.5, 0.8),
            (6., 1.8),
            (6., -1.8),
            (7., 3.5),
            (7., -3.5),
        ]
    } else {
        vec![(1.2, 0.), (1.5, 0.65), (1.5, -0.65), (-1.2, 0.), (4., 0.)]
    };
    for (index, (x, z)) in positions.into_iter().enumerate() {
        let archetype = if index == 0 { 4 } else { 0 };
        world.spawn((
            Name::new(format!("Class attack target {index}")),
            ClassTarget(index),
            WorldAssetRoot(scenes[archetype].clone()),
            EnemyArchetype(archetype as u8),
            Transform::from_xyz(x, 0., z).with_scale(Vec3::splat(0.7)),
            Enemy {
                hp: 100000.,
                max_hp: 100000.,
                speed: 0.,
                damage: 0.,
                xp: 0,
                wood: 0,
                gold: 0,
                kill_credit: 0,
                elite: false,
            },
        ));
    }
    app.add_systems(
        PreUpdate,
        exercise
            .after(bevy::ui::UiSystems::Focus)
            .before(rogue_commander::game::SettingsInputSet),
    );
    app.add_systems(Last, observe);
}
fn press(world: &mut World, label: &str) {
    let mut found = false;
    for (name, mut interaction) in world
        .query_filtered::<(&Name, &mut Interaction), With<Button>>()
        .iter_mut(world)
    {
        if name.as_str() == label {
            *interaction = Interaction::Pressed;
            found = true;
        }
    }
    assert!(found, "missing real control {label}");
}
fn portrait(world: &World, entity: Entity, handle: &Handle<Image>) -> bool {
    if world
        .get::<ImageNode>(entity)
        .is_some_and(|image| image.image == *handle)
    {
        return true;
    }
    world
        .get::<Children>(entity)
        .is_some_and(|children| children.iter().any(|child| portrait(world, child, handle)))
}
fn choose(world: &mut World, kind: HeroKind) {
    let handle = world
        .resource::<AssetServer>()
        .load(format!("ui/portraits/hero_{}.png", HEROES[kind.index()].id));
    let cards: Vec<_> = world
        .query_filtered::<(Entity, &Name), With<Button>>()
        .iter(world)
        .filter(|(_, name)| name.as_str().starts_with("Commander card "))
        .map(|(entity, name)| (entity, name.as_str().to_owned()))
        .collect();
    let name = cards
        .into_iter()
        .find(|(entity, _)| portrait(world, *entity, &handle))
        .expect("fixed seeded real recruitment must offer requested class")
        .1;
    press(world, &name);
}
fn exercise(world: &mut World) {
    world.resource_mut::<ClassProbe>().frame += 1;
    let (frame, legacy) = {
        let p = world.resource::<ClassProbe>();
        (p.frame, p.legacy)
    };
    if world.resource::<ClassProbe>().scenario == "class-archer-1" {
        if frame == 5 {
            choose(world, HeroKind::EmberRanger);
        }
        if matches!(frame, 6 | 16) {
            for mut interaction in world
                .query_filtered::<&mut Interaction, With<Button>>()
                .iter_mut(world)
            {
                *interaction = Interaction::None;
            }
        }
        if frame == 19 {
            assert_eq!(world.resource::<HeroVitals>().kind, HeroKind::EmberRanger);
            world.resource_mut::<PlayerBuild>().skills.fill(0);
            world.resource_mut::<ActiveSkill>().remaining = 10000.;
        }
    }
    if frame == 450 {
        press(world, "Hero attributes open");
    }
    if frame == 451 {
        for mut interaction in world
            .query_filtered::<&mut Interaction, With<Button>>()
            .iter_mut(world)
        {
            *interaction = Interaction::None;
        }
    }
    if legacy {
        if matches!(frame, 5 | 60) {
            world.resource_mut::<RecruitmentProgress>().requested = true;
        }
        if matches!(frame, 15 | 75) {
            choose(
                world,
                if frame == 15 {
                    HeroKind::EmberRanger
                } else {
                    HeroKind::StormSavant
                },
            );
        }
        if matches!(frame, 16 | 36 | 76 | 96) {
            for mut interaction in world
                .query_filtered::<&mut Interaction, With<Button>>()
                .iter_mut(world)
            {
                *interaction = Interaction::None;
            }
        }
        if frame == 110 {
            assert_eq!(world.resource::<SelectedHero>().kind, HeroKind::StormSavant);
            let fusion = world.resource::<HeroFusion>();
            assert!(fusion.has(HeroKind::StoneWarden) && fusion.has(HeroKind::EmberRanger));
            assert_eq!(world.resource::<RecruitmentProgress>().fruit_count, 0);
            world.resource_mut::<PlayerBuild>().skills.fill(0);
            world.resource_mut::<ActiveSkill>().remaining = 10000.;
            println!(
                "[capture] class-stage absorbed frame={frame} actual=StoneWarden+EmberRanger current=StormSavant"
            );
        }
    }
    if frame == if legacy { 120 } else { 20 } {
        let mut clock = world.resource_mut::<AttackClock>();
        clock.0.unpause();
        let duration = clock.0.duration();
        clock.0.set_elapsed(duration);
    }
}
fn health(world: &mut World) -> Vec<f32> {
    let mut targets: Vec<_> = world
        .query::<(&ClassTarget, &Enemy)>()
        .iter(world)
        .map(|(target, enemy)| (target.0, enemy.hp))
        .collect();
    targets.sort_by_key(|(index, _)| *index);
    targets.into_iter().map(|(_, hp)| hp).collect()
}
fn observe(world: &mut World) {
    let (frame, kind, legacy, level) = {
        let p = world.resource::<ClassProbe>();
        (p.frame, p.kind, p.legacy, p.level)
    };
    let arrows: Vec<_> = world
        .query::<(&Projectile, &Ricochet)>()
        .iter(world)
        .map(|(projectile, bounce)| (projectile.velocity, *bounce))
        .collect();
    if !arrows.is_empty() && !world.resource::<ClassProbe>().saw_volley {
        if !legacy {
            let stats = class_basic_stats(kind, level, world.resource::<PlayerBuild>());
            assert_eq!(arrows.len(), usize::from(stats.shots));
            assert!(
                arrows
                    .iter()
                    .all(|(_, bounce)| bounce.remaining == stats.bounces)
            );
            assert!(
                arrows
                    .windows(2)
                    .any(|pair| pair[0].0.normalize().distance(pair[1].0.normalize()) > 0.03)
            );
            world.resource_mut::<AttackClock>().0.pause();
        }
        world.resource_mut::<ClassProbe>().saw_volley = true;
        println!(
            "[capture] class-stage volley frame={frame} actual_arrows={}",
            arrows.len()
        );
    }
    let bounced = arrows
        .iter()
        .any(|(_, bounce)| bounce.bounces_used > 0 && bounce.target.is_some());
    let names: Vec<_> = world
        .query::<&Name>()
        .iter(world)
        .map(|name| name.as_str().to_owned())
        .collect();
    let bounce_art = names.iter().any(|name| name == "Archer ricochet trail");
    let cleave_art = names.iter().any(|name| name == "Warrior cleave sweep");
    if bounced && bounce_art && !world.resource::<ClassProbe>().saw_bounce {
        world.resource_mut::<ClassProbe>().saw_bounce = true;
        println!("[capture] class-stage ricochet frame={frame} actual_target_lock=true");
    }
    if cleave_art && !world.resource::<ClassProbe>().saw_cleave {
        world.resource_mut::<ClassProbe>().saw_cleave = true;
        println!("[capture] class-stage cleave frame={frame}");
        if !legacy {
            world.resource_mut::<AttackClock>().0.pause();
        }
    }
    if (bounce_art || cleave_art) && world.resource::<ClassProbe>().paused_at.is_none() {
        let frozen = world
            .query::<(Entity, &Name, &Transform)>()
            .iter(world)
            .filter(|(_, name, _)| {
                matches!(
                    name.as_str(),
                    "Archer ricochet trail" | "Warrior cleave sweep"
                )
            })
            .map(|(entity, _, transform)| (entity, *transform))
            .collect();
        let hp = health(world);
        let mut p = world.resource_mut::<ClassProbe>();
        p.paused_at = Some(frame);
        p.frozen = frozen;
        p.frozen_hp = hp;
        world.resource_mut::<GameSession>().paused = true;
        println!("[capture] class-stage paused frame={frame}");
    }
    if world
        .resource::<ClassProbe>()
        .paused_at
        .is_some_and(|at| frame == at + 8)
    {
        let frozen = world.resource::<ClassProbe>().frozen.clone();
        for (entity, transform) in frozen {
            assert_eq!(
                *world
                    .get::<Transform>(entity)
                    .expect("paused attack cue remains alive"),
                transform
            );
        }
        let hp = health(world);
        assert_eq!(hp, world.resource::<ClassProbe>().frozen_hp);
        world.resource_mut::<GameSession>().paused = false;
    }
    if frame == 470 {
        assert!(world.resource::<GameSession>().paused);
        assert!(
            world
                .query::<&Name>()
                .iter(world)
                .any(|name| name.as_str() == "Hero basic attack details")
        );
        println!("[capture] class-stage attributes frame={frame} actual_class_description=true");
    }
    if frame == 440 {
        let hp = health(world);
        let p = world.resource::<ClassProbe>();
        assert!(hp.iter().any(|hp| *hp < 100000.));
        if legacy {
            let fusion = world.resource::<HeroFusion>();
            assert!(
                fusion.attacks_emitted[HeroKind::StoneWarden.index()] > 0
                    && fusion.attacks_emitted[HeroKind::EmberRanger.index()] > 0
            );
            assert!(p.saw_bounce && p.saw_cleave);
            println!(
                "[capture] class-stage inherited frame={frame} actual={:?}",
                fusion.attacks_emitted
            );
        } else if kind == HeroKind::EmberRanger {
            assert!(p.saw_volley && p.saw_bounce);
            println!("[capture] class-stage archer-damage frame={frame} actual_hp={hp:?}");
            assert!(
                hp.iter().filter(|hp| **hp < 100000.).count() >= 3,
                "true ricochet damages beyond a two-arrow volley"
            );
        } else {
            assert!(p.saw_cleave);
            assert!(hp[..3].iter().all(|hp| *hp < 100000.));
            assert_eq!(hp[3], 100000., "behind actor outside forward cleave");
            assert_eq!(hp[4], 100000., "far actor outside real melee range");
            println!(
                "[capture] class-stage damage frame={frame} primary={} secondary={}/{} behind=0 outside=0",
                100000. - hp[0],
                100000. - hp[1],
                100000. - hp[2]
            );
        }
        println!(
            "[capture] PASS {}: actual class attack, authoritative damage, real trails, paused transforms/HP, production camera",
            p.scenario
        );
    }
}
