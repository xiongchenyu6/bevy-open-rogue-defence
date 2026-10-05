//! Repeated gate reconstruction and the hero's actual finite-life survival system.
use bevy::prelude::*;
use rogue_commander::game::{
    ActiveSkill, AttackClock, Autoplay, Core, Enemy, EnemyArchetype, GameAssets, GameSession, Hero,
    HeroKind, HeroVitals, PlayerBuild, SelectedHero, SpawnDirector,
};
#[derive(Component)]
struct BarrierProbe {
    lane: f32,
}

#[derive(Component)]
struct BreachedPressureProbe;

#[derive(Resource, Default)]
struct RevivalCapture {
    frame: u32,
    paused_countdown: f32,
    pressure_health: f32,
    countdown_seen: [bool; 4],
}
pub fn configure(world: &mut World) {
    world.remove_resource::<Autoplay>();
    world
        .resource_mut::<super::CaptureControls>()
        .suppress_drafts = true;
    world.resource_mut::<SelectedHero>().kind = HeroKind::StoneWarden;
    world.resource_mut::<SelectedHero>().locked = true;
    *world.resource_mut::<HeroVitals>() = HeroVitals::for_hero(HeroKind::StoneWarden, 1);
    world.resource_mut::<GameSession>().auto_bond_draws = false;
    world.resource_mut::<GameSession>().skill_points = 0;
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<PlayerBuild>().skills.fill(0);
}
pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    world.remove_resource::<Autoplay>();
    world.resource_mut::<PlayerBuild>().skills.fill(0);
    world.resource_mut::<ActiveSkill>().remaining = 10_000.;
    world.resource_mut::<AttackClock>().0.pause();
    world.resource_mut::<Time<Virtual>>().set_relative_speed(4.);
    for (mut transform, mut projection) in world
        .query_filtered::<(&mut Transform, &mut Projection), With<Camera3d>>()
        .iter_mut(world)
    {
        *transform = Transform::from_xyz(8., 24., 21.).looking_at(Vec3::ZERO, Vec3::Y);
        if let Projection::Orthographic(p) = &mut *projection {
            p.scaling_mode = bevy::camera::ScalingMode::FixedVertical {
                viewport_height: 22.,
            };
        }
    }
    let scenes = world.resource::<GameAssets>().enemy_scenes.clone();
    for archetype in 0..12u8 {
        let lane = -5.0 + f32::from(archetype) * (10.0 / 11.0);
        world.spawn((
            Name::new(format!("Barrier probe archetype {archetype}")),
            BarrierProbe { lane },
            EnemyArchetype(archetype),
            Enemy {
                hp: 100_000.,
                max_hp: 100_000.,
                speed: 100.,
                damage: 1.,
                xp: 0,
                wood: 0,
                gold: 0,
                kill_credit: 0,
                elite: false,
            },
            WorldAssetRoot(scenes[usize::from(archetype)].clone()),
            Transform::from_xyz(5., 0., lane).with_scale(Vec3::splat(0.75)),
        ));
    }
    app.init_resource::<RevivalCapture>()
        .add_systems(Last, exercise);
    println!(
        "[capture] gate-revival: 400 frames at 4x; two gate rebuilds and all three hero lives"
    );
}
fn lethal_hit(world: &mut World) {
    let mut vitals = world.resource_mut::<HeroVitals>();
    assert!(vitals.alive);
    assert_eq!(
        vitals.invulnerable_remaining, 0.,
        "wait for genuine respawn protection to expire"
    );
    let damage = vitals.max_health * 100.;
    assert!(vitals.take_magical_damage(damage) > 0.);
    assert_eq!(vitals.health, 0.);
}
fn exercise(world: &mut World) {
    world.resource_mut::<RevivalCapture>().frame += 1;
    let frame = world.resource::<RevivalCapture>().frame;
    if (81..120).contains(&frame) {
        let down = world.resource::<GameSession>().gate_revive_remaining > 0.;
        let labels: Vec<(String, String)> = world
            .query::<(&Name, &Text)>()
            .iter(world)
            .map(|(n, t)| (n.as_str().to_owned(), t.0.clone()))
            .collect();
        let countdown = labels
            .iter()
            .find(|(name, _)| name == "Gate rebuild countdown digit")
            .expect("world-space rebuild countdown must exist");
        if down {
            let digit: usize = countdown
                .1
                .parse()
                .expect("countdown is a visible integer digit");
            assert!((1..=4).contains(&digit));
            world.resource_mut::<RevivalCapture>().countdown_seen[digit - 1] = true;
            let visible = world
                .query::<(&Name, &Node)>()
                .iter(world)
                .find(|(name, _)| name.as_str() == "Gate rebuild countdown")
                .expect("countdown panel exists")
                .1
                .display;
            assert_eq!(visible, Display::Flex);
        }
    }
    match frame {
        10 => {
            let hero = world
                .query_filtered::<&Transform, With<Hero>>()
                .single(world)
                .unwrap()
                .translation;
            let gate = world
                .query_filtered::<&Transform, With<Core>>()
                .single(world)
                .unwrap()
                .translation;
            assert!(
                gate.x > hero.x + 1.,
                "gate must stand ahead of hero toward incoming enemies"
            );
            let masonry: Vec<(String, Vec3, Vec3)> = world
                .query::<(&Name, &Transform)>()
                .iter(world)
                .filter(|(name, _)| {
                    name.as_str().starts_with("Gate ")
                        && (name.as_str().contains("wall") || name.as_str().contains("iron gate"))
                })
                .map(|(name, t)| (name.as_str().to_owned(), t.translation, t.scale))
                .collect();
            assert_eq!(
                masonry.len(),
                3,
                "central iron gate and two road-spanning stone walls"
            );
            for (_, position, _) in &masonry {
                assert_eq!(position.x, 0., "wall segments share a transverse plane");
            }
            let northern = masonry
                .iter()
                .find(|(name, _, _)| name.contains("north"))
                .unwrap();
            let southern = masonry
                .iter()
                .find(|(name, _, _)| name.contains("south"))
                .unwrap();
            assert!(northern.1.z - northern.2.x / 2. <= -5.6);
            assert!(southern.1.z + southern.2.x / 2. >= 5.6);
            assert_eq!(world.resource::<HeroVitals>().lives_remaining(), 3);
            println!("[capture] gate ahead of hero PASS: gate={gate:?} hero={hero:?}");
        }
        15 => {
            let gate_x = world
                .query_filtered::<&Transform, With<Core>>()
                .single(world)
                .unwrap()
                .translation
                .x;
            let probes: Vec<(Entity, Vec3, f32)> = world
                .query::<(Entity, &Transform, &BarrierProbe)>()
                .iter(world)
                .map(|(e, t, p)| (e, t.translation, p.lane))
                .collect();
            assert_eq!(probes.len(), 12);
            for (entity, position, lane) in probes {
                assert!(
                    position.x > gate_x,
                    "archetype must not cross live wall: {position:?}"
                );
                assert!(
                    (position.z - lane).abs() < 0.001,
                    "wall must cover every lane without funneling"
                );
                world.despawn(entity);
            }
            let session = world.resource::<GameSession>();
            assert!(
                session.core_hp > 0. && session.core_hp < session.core_max_hp,
                "wall is finite-health tank, not invulnerable"
            );
            let vitals = world.resource::<HeroVitals>();
            assert_eq!(
                vitals.health, vitals.max_health,
                "live wall protects rear hero"
            );
            println!(
                "[capture] PASS full-road barrier: 12 archetypes blocked, all lane positions preserved, finite gate HP, hero protected"
            );
        }
        20 => {
            world.resource_mut::<GameSession>().core_hp = 0.;
            let health = world.resource::<HeroVitals>().health;
            world.resource_mut::<RevivalCapture>().pressure_health = health;
            let gate = world
                .query_filtered::<&Transform, With<Core>>()
                .single(world)
                .unwrap()
                .translation;
            world.spawn((
                Name::new("Persistent breached enemy pressure"),
                BreachedPressureProbe,
                Enemy {
                    hp: 100_000.,
                    max_hp: 100_000.,
                    speed: 6.,
                    damage: 1.,
                    xp: 0,
                    wood: 0,
                    gold: 0,
                    kill_credit: 0,
                    elite: false,
                },
                Transform::from_translation(gate + Vec3::X * 2.),
            ));
        }
        80 => world.resource_mut::<GameSession>().core_hp = 0.,
        25 | 85 => {
            let session = world.resource::<GameSession>();
            assert!(!session.finished && session.gate_revive_remaining > 0.);
            assert_eq!(
                world.resource::<HeroVitals>().deaths,
                0,
                "gate destruction must not spend hero lives"
            );
            println!(
                "[capture] gate down frame={frame}, rebuild={:.2}, expedition continues",
                session.gate_revive_remaining
            );
        }
        30 => {
            world.resource_mut::<GameSession>().paused = true;
            let remaining = world.resource::<GameSession>().gate_revive_remaining;
            world.resource_mut::<RevivalCapture>().paused_countdown = remaining;
        }
        44 => assert_eq!(
            world.resource::<GameSession>().gate_revive_remaining,
            world.resource::<RevivalCapture>().paused_countdown
        ),
        45 => world.resource_mut::<GameSession>().paused = false,
        70 | 120 => {
            let session = world.resource::<GameSession>();
            assert!(!session.finished);
            assert_eq!(session.gate_revive_remaining, 0.);
            assert_eq!(session.core_hp, session.core_max_hp);
            let panel = world
                .query::<(&Name, &Node)>()
                .iter(world)
                .find(|(name, _)| name.as_str() == "Gate rebuild countdown")
                .expect("countdown panel exists")
                .1
                .display;
            assert_eq!(panel, Display::None, "rebuilt wall removes countdown");
            if frame == 120 {
                assert!(
                    world
                        .resource::<RevivalCapture>()
                        .countdown_seen
                        .iter()
                        .all(|seen| *seen),
                    "real rebuild renders all digits 4, 3, 2, 1"
                );
                println!(
                    "[capture] PASS visible rebuild countdown: 4 / 3 / 2 / 1, hidden on restored full-health wall"
                );
            }
            if frame == 70 {
                let gate_x = world
                    .query_filtered::<&Transform, With<Core>>()
                    .single(world)
                    .unwrap()
                    .translation
                    .x;
                let mut probes =
                    world.query_filtered::<(Entity, &Transform), With<BreachedPressureProbe>>();
                let (entity, transform) = probes.single(world).unwrap();
                assert!(
                    transform.translation.x < gate_x,
                    "rebuilt wall must not teleport breached enemies outside"
                );
                assert!(
                    world.resource::<HeroVitals>().health
                        < world.resource::<RevivalCapture>().pressure_health,
                    "breached enemy must deal actual hero damage"
                );
                world.despawn(entity);
                println!(
                    "[capture] PASS persistent rear pressure: crossed opening, survived rebuild on hero side, actual hero damage"
                );
            }
            println!("[capture] full-health gate rebuild frame={frame} PASS");
        }
        130 | 225 | 350 => lethal_hit(world),
        150 | 245 => {
            let vitals = world.resource::<HeroVitals>();
            let expected_deaths = if frame == 150 { 1 } else { 2 };
            assert!(!vitals.alive && vitals.revive_remaining > 0.);
            assert_eq!(vitals.deaths, expected_deaths);
            assert_eq!(vitals.lives_remaining(), 3 - expected_deaths);
            assert!(!world.resource::<GameSession>().finished);
            assert_eq!(
                world.resource::<GameSession>().core_hp,
                world.resource::<GameSession>().core_max_hp,
                "hero death must not damage gate"
            );
            println!(
                "[capture] hero life spent deaths={} remaining={} revive={:.2}",
                vitals.deaths,
                vitals.lives_remaining(),
                vitals.revive_remaining
            );
        }
        200 | 320 => {
            let vitals = world.resource::<HeroVitals>();
            assert!(vitals.alive);
            assert_eq!(vitals.health, vitals.max_health);
            assert!(!world.resource::<GameSession>().finished);
            println!("[capture] hero respawn after death {} PASS", vitals.deaths);
        }
        375 => {
            let vitals = world.resource::<HeroVitals>();
            let session = world.resource::<GameSession>();
            assert_eq!(vitals.deaths, 3);
            assert_eq!(vitals.lives_remaining(), 0);
            assert!(!vitals.alive && session.finished && !session.victory);
            assert_eq!(vitals.revive_remaining, 0.);
            println!(
                "[capture] PASS gate-revival: two nonterminal rebuilds, frozen pause, two hero respawns, third death settles defeat"
            );
        }
        _ => {}
    }
}
