//! Renderer-free balance run using the complete production GamePlugin.
use bevy::{
    audio::AudioPlugin,
    ecs::schedule::{ScheduleLabel, SingleThreadedExecutor},
    prelude::*,
    render::RenderPlugin,
    time::TimeUpdateStrategy,
    window::ExitCondition,
    winit::WinitPlugin,
};
use rogue_commander::game::{
    BasicAttackCounter, BondCardId, BondDrawButton, CombatPools, DraftButton, DraftState,
    EnemyPressureSettings, GamePlugin, GameSession, Hero, HeroVitals, PlayerBuild,
    RecruitmentProgress, RunRng, RunSeed, SelectedHero, SkillCastTelemetry, SkillDrawButton,
    SkillEconomy,
};
use std::{env, time::Duration};

#[derive(Resource)]
struct Policy {
    chapter: usize,
    training: u8,
    active: bool,
    starter: usize,
    forward: bool,
    frame: u32,
    settled: u32,
    market_step: Option<usize>,
    next_market_at: f32,
}
fn press(world: &mut World, entity: Entity) {
    if let Some(mut interaction) = world.get_mut::<Interaction>(entity) {
        *interaction = Interaction::Pressed;
    }
}
fn policy(world: &mut World) {
    let (frame, active, starter, forward) = {
        let mut p = world.resource_mut::<Policy>();
        p.frame += 1;
        (p.frame, p.active, p.starter, p.forward)
    };
    world
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::KeyB);
    // Release between choices: Changed<Interaction> must represent a fresh input.
    let entities: Vec<Entity> = world
        .query::<(Entity, &Interaction)>()
        .iter(world)
        .filter_map(|(e, i)| (*i == Interaction::Pressed).then_some(e))
        .collect();
    for e in entities {
        *world.get_mut::<Interaction>(e).unwrap() = Interaction::None;
    }
    if frame == 1 {
        let requested = world.resource::<Policy>();
        let (chapter, training) = (requested.chapter, requested.training);
        let mut progress = world.resource_mut::<rogue_commander::game::CampaignProgress>();
        // Isolated chapter fixture, not a claim that this strategy earned prior clears.
        progress.unlocked = (chapter + 1) as u8;
        progress.cleared = std::array::from_fn(|index| index < chapter);
        progress.upgrades = [training; 6];
    }
    if frame < 10 || !frame.is_multiple_of(8) {
        return;
    }
    if world.resource::<rogue_commander::game::CampaignMenu>().0 {
        let wanted = format!("Campaign chapter {}", world.resource::<Policy>().chapter);
        let e = world
            .query::<(Entity, &Name)>()
            .iter(world)
            .find_map(|(e, n)| (n.as_str() == wanted).then_some(e));
        if let Some(e) = e {
            press(world, e);
        }
        return;
    }
    if !world.resource::<SelectedHero>().locked {
        let paths = [
            "ui/portraits/hero_stone_warden.png",
            "ui/portraits/hero_ember_ranger.png",
            "ui/portraits/hero_storm_savant.png",
        ];
        let wanted = world
            .resource::<AssetServer>()
            .load::<Image>(paths[starter])
            .id();
        let images: Vec<Entity> = world
            .query::<(Entity, &ImageNode)>()
            .iter(world)
            .filter_map(|(e, i)| (i.image.id() == wanted).then_some(e))
            .collect();
        for image in images {
            let mut card = Some(image);
            while let Some(e) = card {
                if world
                    .get::<Name>(e)
                    .is_some_and(|n| n.as_str().starts_with("Commander card "))
                {
                    press(world, e);
                    return;
                }
                card = world.get::<ChildOf>(e).map(ChildOf::parent);
            }
        }
        return;
    }
    if world.resource::<Policy>().settled == 0 {
        if forward {
            for mut t in world
                .query_filtered::<&mut Transform, With<Hero>>()
                .iter_mut(world)
            {
                t.translation.x = -2.0;
            }
        }
        world.resource_mut::<Policy>().settled = 1;
    }
    if world.resource::<GameSession>().finished {
        return;
    }
    // Real recruitment cards and real authoritative draft settlements; no stat grants.
    let hero_card = world
        .query::<(Entity, &Name, &Interaction)>()
        .iter(world)
        .find_map(|(e, n, _)| (n.as_str() == "Commander card 0").then_some(e));
    if let Some(e) = hero_card {
        press(world, e);
        return;
    }
    if !active {
        return;
    }
    if let Some(step) = world.resource::<Policy>().market_step {
        if step < 5 {
            let label = format!("Market offer {step}");
            let e = world
                .query::<(Entity, &Name)>()
                .iter(world)
                .find_map(|(e, n)| (n.as_str() == label).then_some(e));
            if let Some(e) = e {
                press(world, e);
            }
            world.resource_mut::<Policy>().market_step = Some(step + 1);
        } else {
            world
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::KeyB);
            world.resource_mut::<Policy>().market_step = None;
        }
        return;
    }
    if world.resource::<DraftState>().root.is_some() {
        let e = world
            .query::<(Entity, &DraftButton)>()
            .iter(world)
            .find_map(|(e, b)| (b.0 == 0).then_some(e));
        if let Some(e) = e {
            press(world, e);
        }
        return;
    }
    let s = world.resource::<GameSession>();
    if s.paused {
        return;
    }
    if s.elapsed >= world.resource::<Policy>().next_market_at && s.kills > 0 {
        let next = s.elapsed + 60.0;
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyB);
        let mut p = world.resource_mut::<Policy>();
        p.market_step = Some(0);
        p.next_market_at = next;
        return;
    }
    let price = 80 + 15 * world.resource::<SkillEconomy>().paid_draws;
    let skill = s.skill_points > 0 || s.gold >= price.min(260);
    let bond = s.wood >= 40;
    let e = if skill {
        world
            .query_filtered::<Entity, With<SkillDrawButton>>()
            .iter(world)
            .next()
    } else if bond {
        world
            .query_filtered::<Entity, With<BondDrawButton>>()
            .iter(world)
            .next()
    } else {
        None
    };
    if let Some(e) = e {
        press(world, e);
    }
}
struct Metrics {
    gate_breaks: u32,
    gate_down: bool,
    exposed_secs: f32,
    last_elapsed: f32,
    min_gate_hp: f32,
    min_hero_hp: f32,
    min_alive_health_fraction: f32,
    max_health_deficit: f32,
    injured_secs: f32,
}
impl Metrics {
    fn observe(&mut self, world: &World) {
        let s = world.resource::<GameSession>();
        let h = world.resource::<HeroVitals>();
        let down = s.core_hp <= 0.0 || s.gate_revive_remaining > 0.0;
        if down && !self.gate_down {
            self.gate_breaks += 1;
        }
        let seconds = (s.elapsed - self.last_elapsed).max(0.0);
        if down {
            self.exposed_secs += seconds;
        }
        if h.alive && h.max_health > 0.0 {
            self.min_alive_health_fraction = self
                .min_alive_health_fraction
                .min((h.health / h.max_health).clamp(0.0, 1.0));
            let deficit = (h.max_health - h.health).max(0.0);
            self.max_health_deficit = self.max_health_deficit.max(deficit);
            if deficit > 0.01 {
                self.injured_secs += seconds;
            }
        }
        self.gate_down = down;
        self.last_elapsed = s.elapsed;
        self.min_gate_hp = self.min_gate_hp.min(s.core_hp);
        self.min_hero_hp = self.min_hero_hp.min(h.health);
    }
}
fn checkpoint(world: &mut World, frame: u32, metrics: &Metrics) {
    let s = world.resource::<GameSession>();
    let h = world.resource::<HeroVitals>();
    let actual_chapter = world.resource::<rogue_commander::game::CampaignRun>().stage;
    let permanent_upgrades = world
        .resource::<rogue_commander::game::CampaignProgress>()
        .upgrades;
    let actual_seed = world.resource::<RunSeed>().0;
    let actual_hero = h.kind.index();
    let absorbed_strength = world.resource::<RecruitmentProgress>().absorbed_strength;
    let command_health_multiplier = h.command_health_multiplier;
    let max_hero_hp = h.max_health;
    let owned_bond_ids = BondCardId::ALL
        .iter()
        .filter(|id| world.resource::<PlayerBuild>().owned_bond_cards[id.index()])
        .map(|id| id.index().to_string())
        .collect::<Vec<_>>()
        .join(",");
    let attacks = world
        .get_resource::<BasicAttackCounter>()
        .map_or(0, |v| v.count());
    let casts = world
        .get_resource::<SkillCastTelemetry>()
        .map_or("null".to_owned(), |v| v.total.to_string());
    let skills: u32 = world
        .resource::<PlayerBuild>()
        .skills
        .iter()
        .map(|v| u32::from(*v))
        .sum();
    let pools = world.resource::<CombatPools>().snapshot();
    let gate_breaks = metrics.gate_breaks;
    let exposed_secs = metrics.exposed_secs;
    let min_gate_hp = metrics.min_gate_hp;
    let min_hero_hp = metrics.min_hero_hp;
    let min_alive_health_fraction = metrics.min_alive_health_fraction;
    let max_health_deficit = metrics.max_health_deficit;
    let injured_secs = metrics.injured_secs;
    println!(
        "{{\"type\":\"checkpoint\",\"frame\":{frame},\"actual_chapter\":{actual_chapter},\"permanent_upgrades\":{permanent_upgrades:?},\"actual_run_seed\":{actual_seed},\"actual_hero_id\":{actual_hero},\"owned_bond_ids\":[{owned_bond_ids}],\"absorbed_strength\":{absorbed_strength},\"command_health_multiplier\":{command_health_multiplier},\"max_hero_hp\":{max_hero_hp},\"elapsed\":{:.3},\"wave\":{},\"level\":{},\"kills\":{},\"gate_hp\":{:.2},\"gate_breaks\":{gate_breaks},\"exposed_secs\":{exposed_secs:.3},\"min_gate_hp\":{min_gate_hp:.2},\"min_hero_hp\":{min_hero_hp:.2},\"min_alive_health_fraction\":{min_alive_health_fraction:.6},\"max_health_deficit\":{max_health_deficit:.3},\"injured_secs\":{injured_secs:.3},\"hero_hp\":{:.2},\"deaths\":{},\"gold\":{},\"wood\":{},\"bosses\":{},\"attacks\":{attacks},\"damage_skill_casts\":{casts},\"skill_ranks\":{skills},\"enemy_peak\":{},\"enemy_rejected\":{},\"paused\":{},\"finished\":{},\"victory\":{}}}",
        s.elapsed,
        s.wave,
        s.level,
        s.total_kills,
        s.core_hp,
        h.health,
        h.deaths,
        s.gold,
        s.wood,
        s.bosses_defeated,
        pools.enemies.peak(),
        pools.enemies.rejected(),
        s.paused,
        s.finished,
        s.victory
    );
}
fn main() {
    let args: Vec<String> = env::args().collect();
    let chapter: usize = env::var("ROGUE_SIM_CHAPTER")
        .unwrap_or_else(|_| "0".into())
        .parse()
        .expect("chapter must be an integer");
    let training: u8 = env::var("ROGUE_SIM_TRAINING")
        .unwrap_or_else(|_| "0".into())
        .parse()
        .expect("training must be an integer");
    assert!(
        chapter < rogue_commander::game::CHAPTER_COUNT,
        "invalid chapter"
    );
    assert!(
        training <= rogue_commander::game::MAX_UPGRADE_LEVEL,
        "invalid training"
    );
    let active = args.get(1).is_some_and(|s| s == "active");
    let seed = u64::from_str_radix(args.get(2).map_or("1", |s| s.trim_start_matches("0x")), 16)
        .expect("hex seed");
    let starter: usize = args.get(3).map_or("2", String::as_str).parse().unwrap();
    assert!(starter < 3, "invalid starter");
    let forward = args.get(4).is_some_and(|s| s == "forward");
    let frames: u32 = args.get(5).map_or("90000", String::as_str).parse().unwrap();
    assert!(
        args.len() <= 6 || args.len() == 12,
        "pressure override requires exactly six values"
    );
    let mut pressure = EnemyPressureSettings::default();
    if let Some(v) = args.get(6) {
        pressure.movement_growth = v.parse().expect("movement growth");
    }
    if let Some(v) = args.get(7) {
        pressure.attack_growth = v.parse().expect("attack growth");
    }
    if let Some(v) = args.get(8) {
        pressure.movement_cap = v.parse().expect("movement cap");
    }
    if let Some(v) = args.get(9) {
        pressure.attack_cap = v.parse().expect("attack cap");
    }
    if let Some(v) = args.get(10) {
        pressure.health_growth = v.parse().expect("health growth");
    }
    if let Some(v) = args.get(11) {
        pressure.health_cap = v.parse().expect("health cap");
    }
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: format!("{}/assets", env!("CARGO_MANIFEST_DIR")),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: (1280, 720).into(),
                    ..default()
                }),
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .disable::<RenderPlugin>()
            .disable::<WinitPlugin>()
            .disable::<AudioPlugin>()
            .disable::<bevy::core_pipeline::CorePipelinePlugin>()
            .disable::<bevy::post_process::PostProcessPlugin>()
            .disable::<bevy::anti_alias::AntiAliasPlugin>()
            .disable::<bevy::sprite_render::SpriteRenderPlugin>()
            .disable::<bevy::ui_render::UiRenderPlugin>()
            .disable::<bevy::pbr::PbrPlugin>()
            .disable::<bevy::gizmos_render::GizmoRenderPlugin>()
            .disable::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>(),
    )
    .init_asset::<StandardMaterial>()
    .init_asset::<bevy::audio::AudioSource>()
    .init_asset_loader::<bevy::audio::AudioLoader>()
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / 60.0,
    )))
    .insert_resource(pressure)
    .insert_resource(RunSeed(seed))
    .insert_resource(RunRng::from_seed(seed))
    .insert_resource(Policy {
        chapter,
        training,
        active,
        starter,
        forward,
        frame: 0,
        settled: 0,
        market_step: None,
        next_market_at: 180.0,
    })
    .add_plugins(GamePlugin)
    .add_systems(PreUpdate, policy.after(bevy::ui::UiSystems::Focus));
    // Keep the complete production systems, but make their conflict ordering
    // repeatable. Presentation hit-stop also reads the same fixed clock.
    for label in [
        First.intern(),
        PreUpdate.intern(),
        Update.intern(),
        PostUpdate.intern(),
        Last.intern(),
    ] {
        app.edit_schedule(label, |schedule| {
            schedule.set_executor(SingleThreadedExecutor::new());
        });
    }
    app.finish();
    app.cleanup();
    println!(
        "{{\"type\":\"metadata\",\"policy\":\"{}\",\"seed\":{seed},\"chapter\":{chapter},\"training\":{training},\"profile_fixture\":\"prior-chapters-cleared-six-equal-upgrades\",\"starter\":{starter},\"forward\":{forward},\"frame_cap\":{frames},\"renderer\":false,\"executor\":\"single-threaded\",\"fixed_step_secs\":0.016666666666666666}}",
        if active {
            "greedy-card-zero"
        } else {
            "passive"
        }
    );
    println!(
        "{{\"type\":\"pressure\",\"movement_growth\":{},\"attack_growth\":{},\"movement_cap\":{},\"attack_cap\":{},\"health_growth\":{},\"health_cap\":{}}}",
        pressure.movement_growth,
        pressure.attack_growth,
        pressure.movement_cap,
        pressure.attack_cap,
        pressure.health_growth,
        pressure.health_cap
    );
    let mut metrics = Metrics {
        gate_breaks: 0,
        gate_down: false,
        exposed_secs: 0.0,
        last_elapsed: 0.0,
        min_gate_hp: f32::MAX,
        min_hero_hp: f32::MAX,
        min_alive_health_fraction: 1.0,
        max_health_deficit: 0.0,
        injured_secs: 0.0,
    };
    for frame in 1..=frames {
        app.update();
        metrics.observe(app.world());
        if frame == 32
            || frame.is_multiple_of(1800)
            || frame == frames
            || app.world().resource::<GameSession>().finished
        {
            checkpoint(app.world_mut(), frame, &metrics);
        }
        if app.world().resource::<GameSession>().finished {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_pressure_tracks_fraction_after_growth_and_excludes_dead_time() {
        let mut world = World::new();
        world.insert_resource(GameSession::default());
        world.insert_resource(HeroVitals::default());
        let mut metrics = Metrics {
            gate_breaks: 0,
            gate_down: false,
            exposed_secs: 0.0,
            last_elapsed: 0.0,
            min_gate_hp: f32::MAX,
            min_hero_hp: f32::MAX,
            min_alive_health_fraction: 1.0,
            max_health_deficit: 0.0,
            injured_secs: 0.0,
        };
        metrics.observe(&world);
        world.resource_mut::<GameSession>().elapsed = 2.0;
        {
            let mut h = world.resource_mut::<HeroVitals>();
            h.max_health = 10000.0;
            h.health = 2500.0;
        }
        metrics.observe(&world);
        assert_eq!(metrics.min_alive_health_fraction, 0.25);
        assert_eq!(metrics.max_health_deficit, 7500.0);
        assert_eq!(metrics.injured_secs, 2.0);
        world.resource_mut::<GameSession>().elapsed = 5.0;
        {
            let mut h = world.resource_mut::<HeroVitals>();
            h.alive = false;
            h.health = 0.0;
        }
        metrics.observe(&world);
        assert_eq!(metrics.min_alive_health_fraction, 0.25);
        assert_eq!(metrics.injured_secs, 2.0);
        assert_eq!(metrics.min_hero_hp, 0.0);
    }
}
