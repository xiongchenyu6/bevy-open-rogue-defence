//! Observe actual boss warnings and hazards with a vulnerable, unlocked front-line hero.
use bevy::prelude::*;
use rogue_commander::game::{
    ActiveSkill, AttackClock, Autoplay, BossDangerShape, BossEnemy, BossTelegraph, Enemy,
    GameAssets, GameSession, Hero, HeroVitals, PlayerBuild, SelectedHero, SpawnDirector,
    content::BOSSES,
};
#[derive(Resource)]
struct BossProbe {
    index: usize,
    frame: u32,
    boss: Option<Entity>,
    first_warning: Option<u32>,
    frozen: Vec<(Entity, f32, Vec3, bool)>,
    hit: bool,
    phase: bool,
    dodged: bool,
    dodged_at: Option<u32>,
    death_at: Option<u32>,
    before_hit: f32,
    before_dodge: f32,
    saw_primary: bool,
    saw_secondary: bool,
    dodge_locked: bool,
    maximum_summons: usize,
}
pub fn configure(world: &mut World, scenario: &str) {
    let index = scenario
        .strip_prefix("boss-battle-")
        .unwrap()
        .parse::<usize>()
        .unwrap();
    assert!(index < 5);
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SelectedHero>().locked = true;
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
    world
        .resource_mut::<super::CaptureControls>()
        .suppress_drafts = true;
    world.insert_resource(BossProbe {
        index,
        frame: 0,
        boss: None,
        first_warning: None,
        frozen: vec![],
        hit: false,
        phase: false,
        dodged: false,
        dodged_at: None,
        death_at: None,
        before_hit: 0.,
        before_dodge: 0.,
        saw_primary: false,
        saw_secondary: false,
        dodge_locked: false,
        maximum_summons: 0,
    });
}
pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<PlayerBuild>().skills.fill(0);
    world.resource_mut::<AttackClock>().0.pause();
    world.resource_mut::<ActiveSkill>().remaining = 10000.;
    {
        let mut session = world.resource_mut::<GameSession>();
        session.elapsed = 1.;
        session.core_hp = 0.;
        session.gate_revive_remaining = 10000.;
    }
    {
        let mut vitals = world.resource_mut::<HeroVitals>();
        vitals.max_health = 100000.;
        vitals.health = 100000.;
    }
    for mut transform in world
        .query_filtered::<&mut Transform, With<Hero>>()
        .iter_mut(world)
    {
        transform.translation = Vec3::new(4., 0., 0.);
    }
    let index = world.resource::<BossProbe>().index;
    let profile = &BOSSES[index];
    let scene = world.resource::<GameAssets>().boss_scenes[index].clone();
    let boss = world
        .spawn((
            Name::new(format!("Boss battle probe {index}")),
            WorldAssetRoot(scene),
            Transform::from_xyz(9., 0., 0.).with_scale(Vec3::splat(1.15)),
            Visibility::Visible,
            BossEnemy { index: index as u8 },
            Enemy {
                hp: profile.hp,
                max_hp: profile.hp,
                speed: profile.speed,
                damage: profile.damage_per_second,
                xp: 0,
                wood: 0,
                gold: 0,
                kill_credit: 0,
                elite: true,
            },
        ))
        .id();
    world.resource_mut::<BossProbe>().boss = Some(boss);
    app.add_systems(PreUpdate, observe);
}
fn place_hero(world: &mut World, position: Vec3) {
    for mut transform in world
        .query_filtered::<&mut Transform, With<Hero>>()
        .iter_mut(world)
    {
        transform.translation = position;
    }
}
fn stage(name: &str, frame: u32) {
    println!("[capture] boss-stage {name} frame={frame}");
}
fn observe(world: &mut World) {
    world.resource_mut::<BossProbe>().frame += 1;
    let state = world.resource::<BossProbe>();
    let (index, frame, boss) = (state.index, state.frame, state.boss.unwrap());
    let hazards: Vec<_> = world
        .query::<(Entity, &BossTelegraph)>()
        .iter(world)
        .filter(|(_, hazard)| hazard.boss == boss)
        .map(|(entity, hazard)| (entity, *hazard))
        .collect();
    if frame == 25 {
        assert!(
            world.get::<Enemy>(boss).unwrap().max_hp >= BOSSES[index].hp,
            "actual boss registration applies catalog health modifiers"
        );
        stage("spawn", frame);
    }
    if world.resource::<BossProbe>().first_warning.is_none()
        && let Some((_, hazard)) = hazards.iter().find(|(_, hazard)| !hazard.active)
    {
        assert_eq!(usize::from(hazard.index), index);
        assert!(!hazard.enraged);
        let hp = world.resource::<HeroVitals>().health;
        let mut state = world.resource_mut::<BossProbe>();
        state.first_warning = Some(frame);
        state.before_hit = hp;
        state.saw_primary = hazard.kind == 0;
        place_hero(world, hazard.target);
        stage("primary-warning", frame);
    }
    if let Some(warning) = world.resource::<BossProbe>().first_warning {
        if frame == warning + 2 {
            let frozen = hazards
                .iter()
                .map(|(entity, h)| (*entity, h.remaining, h.target, h.active))
                .collect();
            world.resource_mut::<BossProbe>().frozen = frozen;
            world.resource_mut::<GameSession>().paused = true;
            stage("warning-paused", frame);
        }
        if frame == warning + 14 {
            let current: Vec<_> = hazards
                .iter()
                .map(|(entity, h)| (*entity, h.remaining, h.target, h.active))
                .collect();
            assert_eq!(
                current,
                world.resource::<BossProbe>().frozen,
                "pause freezes actual warning timers and target lock"
            );
            world.resource_mut::<GameSession>().paused = false;
        }
    }
    if !world.resource::<BossProbe>().hit && hazards.iter().any(|(_, h)| h.active && !h.enraged) {
        let hp = world.resource::<HeroVitals>().health;
        assert!(
            hp < world.resource::<BossProbe>().before_hit,
            "real primary hazard damages an exposed hero"
        );
        world.resource_mut::<BossProbe>().hit = true;
        stage("primary-hit", frame);
        let mut enemy = world.get_mut::<Enemy>(boss).unwrap();
        enemy.hp = enemy.max_hp * 0.49;
    }
    if world.resource::<BossProbe>().hit && !world.resource::<BossProbe>().phase {
        let crests: Vec<_> = world
            .query::<(&Name, &Visibility)>()
            .iter(world)
            .filter(|(name, visibility)| {
                name.as_str() == "Boss phase crest" && **visibility == Visibility::Visible
            })
            .collect();
        if !crests.is_empty() {
            world.resource_mut::<BossProbe>().phase = true;
            stage("phase-two", frame);
            place_hero(world, Vec3::new(4., 0., 0.));
        }
    }
    if world.resource::<BossProbe>().phase
        && !world.resource::<BossProbe>().dodge_locked
        && let Some((_, hazard)) = hazards
            .iter()
            .find(|(_, h)| h.enraged && !h.active && h.kind == 1)
    {
        let position = match hazard.shape {
            BossDangerShape::Circle => Vec3::new(hazard.target.x - hazard.radius - 6., 0., 0.),
            BossDangerShape::Lane { .. } => Vec3::new(hazard.target.x, 0., 5.1),
        };
        let hp = world.resource::<HeroVitals>().health;
        let mut state = world.resource_mut::<BossProbe>();
        state.before_dodge = hp;
        state.dodge_locked = true;
        state.saw_secondary = true;
        place_hero(world, position);
        stage("secondary-warning-dodge", frame);
    }
    if world.resource::<BossProbe>().dodge_locked
        && !world.resource::<BossProbe>().dodged
        && hazards
            .iter()
            .any(|(_, h)| h.enraged && !h.active && h.kind == 1)
    {
        let hp = world.resource::<HeroVitals>().health;
        world.resource_mut::<BossProbe>().before_dodge = hp;
    }
    if world.resource::<BossProbe>().dodge_locked
        && !world.resource::<BossProbe>().dodged
        && hazards
            .iter()
            .any(|(_, h)| h.enraged && h.active && h.kind == 1)
    {
        let hp = world.resource::<HeroVitals>().health;
        assert!(
            (hp - world.resource::<BossProbe>().before_dodge).abs() < 0.01,
            "moving outside exact secondary warning geometry avoids real hazard damage"
        );
        let mut state = world.resource_mut::<BossProbe>();
        state.dodged = true;
        state.dodged_at = Some(frame);
        stage("secondary-dodged", frame);
    }
    if matches!(world.resource::<BossProbe>().index, 2 | 4) {
        let count = world
            .query::<(Entity, &Enemy)>()
            .iter(world)
            .filter(|(entity, enemy)| *entity != boss && enemy.hp > 0.0)
            .count();
        assert!(
            count <= 12,
            "real boss-owned pressure actors respect the twelve-actor cap"
        );
        let mut probe = world.resource_mut::<BossProbe>();
        if count > probe.maximum_summons {
            probe.maximum_summons = count;
            println!("[capture] boss-stage summons frame={frame} actual={count} cap=12");
        }
    }
    let summons_ready =
        !matches!(index, 2 | 4) || world.resource::<BossProbe>().maximum_summons == 12;
    if world.resource::<BossProbe>().dodged
        && world.resource::<BossProbe>().death_at.is_none()
        && summons_ready
        && world
            .resource::<BossProbe>()
            .dodged_at
            .is_some_and(|at| frame >= at + 5)
    {
        world.resource_mut::<BossProbe>().death_at = Some(frame);
        world.resource_mut::<GameSession>().paused = true;
        world.get_mut::<Enemy>(boss).unwrap().hp = 0.;
    }
    if let Some(death) = world.resource::<BossProbe>().death_at
        && frame == death + 10
    {
        assert!(
            hazards.is_empty(),
            "boss death removes true damaging entities even paused"
        );
        assert!(
            !world
                .query::<&Name>()
                .iter(world)
                .any(|name| name.as_str() == "Boss danger art"),
            "boss death removes warning art attached to hazards"
        );
        stage("death-cleanup", frame);
        world.resource_mut::<GameSession>().paused = false;
    }
    if frame == 1390 {
        let state = world.resource::<BossProbe>();
        assert!(
            state.saw_primary
                && state.saw_secondary
                && state.hit
                && state.phase
                && state.dodged
                && state.death_at.is_some(),
            "all real boss stages reached for {index}"
        );
        println!(
            "[capture] PASS boss-battle-{index}: true registered boss, primary warning/hit, secondary target-lock dodge, half-health enrage, paused warning, death hazard/art cleanup"
        );
    }
}
