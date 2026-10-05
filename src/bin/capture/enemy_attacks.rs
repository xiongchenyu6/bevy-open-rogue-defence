//! Opponent attack presentation driven exclusively by production combat systems.
use super::CaptureControls;
use bevy::prelude::*;
use rogue_commander::game::{
    ActiveSkill, AttackClock, Autoplay, BossEnemy, Core, Enemy, EnemyArchetype, EnemyHeroThreat,
    GameAssets, GameSession, Hero, HeroKind, HeroVitals, PlayerBuild, SelectedHero, SpawnDirector,
};
#[derive(Component)]
struct CaptureEnemy {
    target: Vec3,
    check_facing: bool,
}
#[derive(Resource, Default)]
struct EnemyCapture {
    frame: u32,
    initial_hero: f32,
    initial_core: f32,
    saw_flight: bool,
    saw_impact: bool,
    saw_slash: bool,
    paused: Vec<(Entity, Transform)>,
    orientations: Vec<(Entity, Quat)>,
}
pub fn configure(world: &mut World) {
    world.resource_mut::<SelectedHero>().kind = HeroKind::StoneWarden;
    world.resource_mut::<SelectedHero>().locked = true;
    world.resource_mut::<CaptureControls>().suppress_drafts = true;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
    world.resource_mut::<SpawnDirector>().enabled = false;
}
pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    world.remove_resource::<Autoplay>();
    world.resource_mut::<PlayerBuild>().skills.fill(0);
    world.resource_mut::<ActiveSkill>().remaining = 10_000.0;
    world.resource_mut::<AttackClock>().0.reset();
    world.resource_mut::<AttackClock>().0.pause();
    let core = world
        .query_filtered::<&Transform, With<Core>>()
        .single(world)
        .unwrap()
        .translation
        .with_y(0.0);
    // Keep the fixture inside the production movement bounds. At core.x + 4
    // (-10.8), movement clamps the hero to -10: the stationary burrower then
    // sits 1.360m away, just outside its 1.35m melee range after emerging.
    let hero = core + Vec3::new(5.0, 0.0, 2.0);
    for mut transform in world
        .query_filtered::<&mut Transform, With<Hero>>()
        .iter_mut(world)
    {
        transform.translation = hero;
    }
    let center = core + Vec3::new(3.0, 0.0, 0.6);
    for (mut transform, mut projection) in world
        .query_filtered::<(&mut Transform, &mut Projection), With<Camera3d>>()
        .iter_mut(world)
    {
        *transform = Transform::from_translation(center + Vec3::new(0.0, 14.0, 12.0))
            .looking_at(center, Vec3::Y);
        if let Projection::Orthographic(projection) = &mut *projection {
            projection.scaling_mode = bevy::camera::ScalingMode::FixedVertical {
                viewport_height: 12.0,
            };
        }
    }
    let assets = world.resource::<GameAssets>();
    let scenes = assets.enemy_scenes.clone();
    let boss_scene = assets.boss_scenes[0].clone();
    for (index, position, target, scale) in [
        (3, hero + Vec3::new(2.0, 0.0, 0.0), hero, 1.0),
        (7, hero + Vec3::new(0.0, 0.0, 1.1), hero, 0.85),
        (11, core + Vec3::new(6.0, 0.0, -2.0), core, 1.25),
    ] {
        let mut entity = world.spawn((
            Name::new(format!("Capture archetype {index}")),
            CaptureEnemy {
                target,
                check_facing: true,
            },
            WorldAssetRoot(scenes[index].clone()),
            Transform::from_translation(position).with_scale(Vec3::splat(scale)),
            EnemyArchetype(index as u8),
            enemy(8.0),
        ));
        if index == 3 {
            entity.insert(EnemyHeroThreat::ranged());
        }
        if index == 7 {
            entity.insert(EnemyHeroThreat::burrowed());
        }
    }
    world.spawn((
        Name::new("Capture boss"),
        CaptureEnemy {
            target: core,
            check_facing: false,
        },
        WorldAssetRoot(boss_scene),
        Transform::from_translation(core + Vec3::new(1.0, 0.0, -0.4)).with_scale(Vec3::splat(1.2)),
        BossEnemy { index: 0 },
        enemy(10.0),
    ));
    let initial_hero = world.resource::<HeroVitals>().health;
    let initial_core = world.resource::<GameSession>().core_hp;
    world.insert_resource(EnemyCapture {
        initial_hero,
        initial_core,
        ..default()
    });
    app.add_systems(PreUpdate, control)
        .add_systems(Last, verify);
    println!(
        "[capture] enemy-attacks: actual acid archer, burrower, siege and boss; player attacks disabled"
    );
}
fn enemy(damage: f32) -> Enemy {
    Enemy {
        hp: 100_000.0,
        max_hp: 100_000.0,
        speed: 0.0,
        damage,
        xp: 0,
        wood: 0,
        gold: 0,
        kill_credit: 0,
        elite: false,
    }
}
fn control(mut capture: ResMut<EnemyCapture>, mut session: ResMut<GameSession>) {
    capture.frame += 1;
    if capture.frame == 81 {
        session.paused = true;
        println!("[capture] pause enemy attacks for 30 frames");
    }
    if capture.frame == 111 {
        session.paused = false;
        println!("[capture] resume enemy attacks");
    }
}
#[allow(clippy::too_many_arguments)]
fn verify(
    mut capture: ResMut<EnemyCapture>,
    session: Res<GameSession>,
    vitals: Res<HeroVitals>,
    active: Res<ActiveSkill>,
    clock: Res<AttackClock>,
    models: Query<(Entity, &Name, &Transform)>,
    actors: Query<(Entity, &CaptureEnemy, &Transform)>,
    hero: Query<&Transform, With<Hero>>,
    threats: Query<&EnemyHeroThreat>,
) {
    let effects: Vec<_> = models
        .iter()
        .filter(|(_, name, _)| name.as_str().starts_with("Enemy attack "))
        .collect();
    assert!(
        effects.len() <= 240,
        "enemy attack effects must stay bounded"
    );
    capture.saw_flight |= effects
        .iter()
        .any(|(_, name, _)| name.as_str() == "Enemy attack flight");
    capture.saw_impact |= effects
        .iter()
        .any(|(_, name, _)| name.as_str() == "Enemy attack impact");
    capture.saw_slash |= effects
        .iter()
        .any(|(_, name, _)| name.as_str() == "Enemy attack slash");
    if capture.frame == 80 {
        capture.paused = models
            .iter()
            .filter(|(entity, name, _)| {
                actors.get(*entity).is_ok() || name.as_str().starts_with("Enemy attack ")
            })
            .map(|(entity, _, transform)| (entity, *transform))
            .collect();
    }
    if capture.frame == 110 {
        assert!(session.paused);
        for (entity, before) in &capture.paused {
            let (_, _, after) = models
                .get(*entity)
                .expect("paused actor/effect must survive");
            assert_eq!(
                before, after,
                "paused enemy effects must not move, spin or shrink"
            );
        }
        println!(
            "[capture] PASS pause freezes {} enemy/effect transforms",
            capture.paused.len()
        );
    }
    if capture.frame == 200 {
        for (entity, actor, transform) in &actors {
            println!(
                "[capture] enemy {:?} at {:?}, expected target {:?}, hero {:?}, threat {:?}; cues flight={} impact={} slash={}",
                entity,
                transform.translation,
                actor.target,
                hero.single().map(|h| h.translation),
                threats.get(entity),
                capture.saw_flight,
                capture.saw_impact,
                capture.saw_slash
            );
        }
        capture.orientations = actors
            .iter()
            .filter(|(_, actor, _)| actor.check_facing)
            .map(|(entity, _, transform)| (entity, transform.rotation))
            .collect();
    }
    if capture.frame == 295 {
        assert!(
            vitals.health < capture.initial_hero,
            "real enemy attacks must damage the hero"
        );
        assert!(
            session.core_hp < capture.initial_core,
            "siege/boss must damage the core"
        );
        assert!(
            capture.saw_flight && capture.saw_impact && capture.saw_slash,
            "enemy shot, hit and melee feedback must be observed: flight={} impact={} slash={}",
            capture.saw_flight,
            capture.saw_impact,
            capture.saw_slash
        );
        assert!(active.remaining > 9000.0 && clock.0.is_paused());
        for (entity, previous) in &capture.orientations {
            let (_, actor, transform) = actors.get(*entity).unwrap();
            assert!(
                previous.angle_between(transform.rotation) < 0.08,
                "stationary attacker must not continuously spin"
            );
            let direction = (actor.target - transform.translation)
                .with_y(0.0)
                .normalize_or(Vec3::X);
            assert!(
                transform.forward().dot(direction) > 0.8,
                "attacker must face its actual victim"
            );
        }
        println!(
            "[capture] PASS enemy-attacks: hero {:.1}->{:.1}, core {:.1}->{:.1}, shot/slash/impact, stable facing, cap 240",
            capture.initial_hero, vitals.health, capture.initial_core, session.core_hp
        );
    }
}
