//! Portal warnings and both concentrated arrivals through the real wave director.
use bevy::prelude::*;
use rogue_commander::game::{
    ActiveSkill, AttackClock, Autoplay, Enemy, GameSession, HeroVitals, PlayerBuild, RaidWarning,
    SelectedHero, SpawnDirector,
};
#[derive(Resource)]
struct RaidProbe {
    frame: u32,
    wave: u32,
    warnings: u32,
    bursts: u32,
    warning_at: Option<u32>,
    end_at: Option<u32>,
    prior_remaining: f32,
    count: u16,
    enemies_before: usize,
    frozen: f32,
}
pub fn configure(world: &mut World, scenario: &str) {
    let wave = scenario
        .strip_prefix("raid-pressure-")
        .unwrap()
        .parse::<u32>()
        .unwrap();
    assert!((3..=25).contains(&wave));
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SelectedHero>().locked = true;
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
    world
        .resource_mut::<super::CaptureControls>()
        .suppress_drafts = true;
    world.insert_resource(RaidProbe {
        frame: 0,
        wave,
        warnings: 0,
        bursts: 0,
        warning_at: None,
        end_at: None,
        prior_remaining: 0.,
        count: 0,
        enemies_before: 0,
        frozen: 0.,
    });
}
pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    let wave = world.resource::<RaidProbe>().wave;
    world.remove_resource::<Autoplay>();
    world.resource_mut::<PlayerBuild>().skills.fill(0);
    world.resource_mut::<AttackClock>().0.pause();
    world.resource_mut::<ActiveSkill>().remaining = 10000.;
    let mut session = world.resource_mut::<GameSession>();
    session.elapsed = (wave - 1) as f32 * 60. + 16.4;
    session.core_max_hp = 100000.;
    session.core_hp = 100000.;
    let mut vitals = world.resource_mut::<HeroVitals>();
    vitals.max_health = 100000.;
    vitals.health = 100000.;
    let mut director = world.resource_mut::<SpawnDirector>();
    director.enabled = true;
    director.boss_spawned.fill(true);
    director.next_elite_at = 10000.;
    app.add_systems(PreUpdate, observe);
}
fn observe(world: &mut World) {
    world.resource_mut::<RaidProbe>().frame += 1;
    let frame = world.resource::<RaidProbe>().frame;
    let raid = world.resource::<RaidWarning>();
    let (wave, remaining, count) = (raid.wave, raid.remaining_secs, raid.count);
    let enemies = world.query::<&Enemy>().iter(world).count();
    if remaining > 0. && world.resource::<RaidProbe>().prior_remaining <= 0. {
        let mut state = world.resource_mut::<RaidProbe>();
        state.warnings += 1;
        state.warning_at = Some(frame);
        state.count = count;
        state.enemies_before = enemies;
        assert_eq!(wave, state.wave);
        assert!(count > 0);
        println!(
            "[capture] raid-stage warning{} frame={frame} count={count}",
            state.warnings
        );
    }
    if let Some(warning) = world.resource::<RaidProbe>().warning_at {
        if world.resource::<RaidProbe>().warnings == 1 && frame == warning + 2 {
            world.resource_mut::<RaidProbe>().frozen = remaining;
            world.resource_mut::<GameSession>().paused = true;
        }
        if world.resource::<RaidProbe>().warnings == 1 && frame == warning + 14 {
            assert_eq!(
                remaining,
                world.resource::<RaidProbe>().frozen,
                "real pause freezes raid deadline"
            );
            world.resource_mut::<GameSession>().paused = false;
        }
    }
    if remaining <= 0. && world.resource::<RaidProbe>().prior_remaining > 0. {
        world.resource_mut::<RaidProbe>().end_at = Some(frame);
    }
    if let Some(end) = world.resource::<RaidProbe>().end_at
        && frame == end + 10
    {
        let state = world.resource::<RaidProbe>();
        assert!(
            enemies >= state.enemies_before + usize::from(state.count),
            "actual arrival releases announced concentrated count: before={} count={} after={enemies}",
            state.enemies_before,
            state.count
        );
        if state.wave == 4 {
            assert!(
                world.query::<&Enemy>().iter(world).any(|enemy| enemy.elite),
                "fourth-wave raid promotes a real elite"
            );
        }
        let mut state = world.resource_mut::<RaidProbe>();
        state.bursts += 1;
        println!(
            "[capture] raid-stage burst{} frame={frame} live_enemies={enemies}",
            state.bursts
        );
    }
    world.resource_mut::<RaidProbe>().prior_remaining = remaining;
    if frame == 1790 {
        let state = world.resource::<RaidProbe>();
        assert_eq!(state.warnings, 2);
        assert_eq!(state.bursts, 2);
        println!(
            "[capture] PASS raid-pressure-{}: two real portal warnings, paused countdown, two announced arrival counts, bounded live actors",
            state.wave
        );
    }
}
