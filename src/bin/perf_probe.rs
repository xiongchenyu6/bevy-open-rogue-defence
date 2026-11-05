//! Finite real-window probe. Samples swapchain present-call cadence, not GPU timestamps.
use bevy::{
    app::AppExit,
    asset::{LoadState, RecursiveDependencyLoadState, UntypedAssetId},
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        renderer::{RenderAdapterInfo, render_system},
        view::window::ExtractedWindows,
    },
    window::PresentMode,
};
use rogue_commander::game::*;
use std::{
    fs,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
#[derive(Resource, Clone, Default)]
struct Shared(Arc<Mutex<Samples>>);
#[derive(Default)]
struct Samples {
    measuring: bool,
    ready: bool,
    startup_frames: u64,
    last: Option<Instant>,
    intervals: Vec<f64>,
    warmup_frames: u64,
    adapter: String,
    surface: String,
}
#[derive(Resource, Default)]
struct PendingSurface(bool);
#[derive(Component)]
struct PerfEnemy;
#[derive(Resource)]
struct Probe {
    speed: f32,
    warmup: f64,
    seconds: f64,
    output: String,
    created: Instant,
    ready: Option<Instant>,
    sample_start: Option<Instant>,
    next_counts: Instant,
    counts: Vec<String>,
    initialized: bool,
    asset_ids: Vec<UntypedAssetId>,
    font: Option<Handle<Font>>,
    atlas: Option<Handle<Image>>,
    damage_seen: bool,
    projectile_seen: bool,
    peak_projectiles: usize,
    sample_health: Option<f64>,
    sample_casts_start: [u64; 6],
    sample_skill_casts: [u64; 6],
    loaded: usize,
    imported_enemies: usize,
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(
        args.len(),
        5,
        "perf_probe SPEED WARMUP_SECONDS SAMPLE_SECONDS OUTPUT_PREFIX"
    );
    let speed: f32 = args[1].parse().unwrap();
    let warmup: f64 = args[2].parse().unwrap();
    let seconds: f64 = args[3].parse().unwrap();
    assert!(
        matches!(speed, 1. | 32.)
            && (0. ..=30.).contains(&warmup)
            && (1. ..=60.).contains(&seconds)
    );
    let shared = Shared::default();
    shared.0.lock().unwrap().intervals.reserve(16_384);
    let mut app = rogue_commander::build_app();
    app.insert_resource(shared.clone())
        .insert_resource(Probe {
            speed,
            warmup,
            seconds,
            output: args[4].clone(),
            created: Instant::now(),
            ready: None,
            sample_start: None,
            next_counts: Instant::now(),
            counts: vec![],
            initialized: false,
            asset_ids: vec![],
            font: None,
            atlas: None,
            damage_seen: false,
            projectile_seen: false,
            peak_projectiles: 0,
            sample_health: None,
            sample_casts_start: [0; 6],
            sample_skill_casts: [0; 6],
            loaded: 0,
            imported_enemies: 0,
        })
        .insert_resource(bevy::winit::WinitSettings::continuous())
        .insert_resource(CampaignMenu(false))
        .add_systems(Startup, setup)
        .add_systems(Last, drive);
    app.world_mut().remove_resource::<Autoplay>();
    app.world_mut().resource_mut::<SelectedHero>().locked = true;
    app.world_mut().resource_mut::<OnboardingState>().dismissed = true;
    app.world_mut().resource_mut::<AudioPreferences>().muted = true;
    app.sub_app_mut(RenderApp)
        .insert_resource(shared)
        .init_resource::<PendingSurface>()
        .add_systems(
            Render,
            (
                before_present.before(render_system),
                after_present.after(render_system),
            )
                .in_set(RenderSystems::Render),
        );
    app.run();
}
fn setup(
    mut windows: Query<&mut Window>,
    mut selected: ResMut<SelectedHero>,
    mut guide: ResMut<OnboardingState>,
    mut director: ResMut<SpawnDirector>,
) {
    for mut w in &mut windows {
        w.resizable = false;
        w.title = "Rogue Commander · Frame-time probe".into();
        w.resolution.set(1280., 720.);
        w.present_mode = match std::env::var("ROGUE_PERF_PRESENT_MODE").as_deref() {
            Ok("immediate") => PresentMode::Immediate,
            Ok("fifo") | Err(_) => PresentMode::Fifo,
            Ok(other) => panic!("Unknown probe present mode: {other}"),
        };
    }
    selected.locked = true;
    guide.dismissed = true;
    director.enabled = false;
    println!(
        "PERF_FIXTURE injected stationary180-enemy mid/late battle; six skills rank3; not earned progression; no screenshot readback"
    );
}
fn before_present(windows: Res<ExtractedWindows>, mut pending: ResMut<PendingSurface>) {
    pending.0 = windows.values().any(|w| w.swap_chain_texture.is_some());
}
fn after_present(
    windows: Res<ExtractedWindows>,
    adapter: Res<RenderAdapterInfo>,
    pending: Res<PendingSurface>,
    shared: Res<Shared>,
) {
    if !pending.0 || windows.values().any(|w| w.swap_chain_texture.is_some()) {
        return;
    }
    let now = Instant::now();
    let mut data = shared.0.lock().unwrap();
    if data.adapter.is_empty() {
        data.adapter = format!("{:?}", **adapter);
        data.surface = windows
            .values()
            .map(|w| {
                format!(
                    "{}x{} {:?} format={:?} frame_latency={:?}",
                    w.physical_width,
                    w.physical_height,
                    w.present_mode,
                    w.swap_chain_texture_format,
                    w.desired_maximum_frame_latency
                )
            })
            .collect::<Vec<_>>()
            .join(";");
    }
    if data.measuring {
        if let Some(last) = data.last {
            data.intervals
                .push(now.duration_since(last).as_secs_f64() * 1000.);
        }
        data.last = Some(now);
    } else if data.ready {
        data.warmup_frames += 1;
    } else {
        data.startup_frames += 1;
    }
}
fn drive(world: &mut World) {
    let mut p = world.remove_resource::<Probe>().unwrap();
    let now = Instant::now();
    assert!(
        p.created.elapsed().as_secs_f64() < 60. + p.warmup + p.seconds,
        "PERF_TIMEOUT: startup exceeded60seconds plus requested warmup/sample budget"
    );
    if p.asset_ids.is_empty() {
        let assets = world.resource::<GameAssets>();
        p.asset_ids
            .extend(assets.enemy_scenes.iter().map(|h| h.id().untyped()));
        // Bone Sentry is the summon among the six selected skills.
        p.asset_ids.push(assets.summon_scenes[0].id().untyped());
        p.asset_ids.push(
            assets.hero_scenes[world.resource::<SelectedHero>().kind.index()]
                .id()
                .untyped(),
        );
        for root in world.query::<&WorldAssetRoot>().iter(world) {
            if !p.asset_ids.contains(&root.0.id().untyped()) {
                p.asset_ids.push(root.0.id().untyped());
            }
        }
        let font = world
            .resource::<AssetServer>()
            .load::<Font>("fonts/rogue_commander_ui.ttf");
        p.asset_ids.push(font.id().untyped());
        p.font = Some(font);
        let atlas = world
            .resource::<AssetServer>()
            .load::<Image>("ui/market/equipment-atlas.png");
        p.asset_ids.push(atlas.id().untyped());
        p.atlas = Some(atlas);
    }
    if !p.initialized {
        let server = world.resource::<AssetServer>();
        for id in &p.asset_ids {
            if let Some((state, _, deps)) = server.get_load_states(*id) {
                assert!(
                    !matches!(state, LoadState::Failed(_))
                        && !matches!(deps, RecursiveDependencyLoadState::Failed(_)),
                    "PERF_ASSET_FAILURE {:?}",
                    server.get_path(*id)
                );
            }
        }
        p.loaded = p
            .asset_ids
            .iter()
            .filter(|id| server.is_loaded_with_dependencies(**id))
            .count();
    }
    let ready =
        p.loaded == p.asset_ids.len() && p.loaded >= 16 && world.resource::<AssetReadiness>().ready;
    if ready && !p.initialized {
        let models = world.resource::<GameAssets>().enemy_scenes.clone();
        for i in 0..180usize {
            world.spawn((
                Name::new(format!("Perf fixture enemy {i}")),
                PerfEnemy,
                EnemyArchetype((i % 12) as u8),
                Enemy {
                    hp: 1e7,
                    max_hp: 1e7,
                    speed: 0.,
                    damage: 0.,
                    xp: 0,
                    gold: 0,
                    wood: 0,
                    kill_credit: 0,
                    elite: false,
                },
                WorldAssetRoot(models[i % 12].clone()),
                Transform::from_xyz(0.5 + (i % 18) as f32 * 0.42, 0., -4.5 + (i / 18) as f32)
                    .with_scale(Vec3::splat(0.7)),
            ));
        }
        {
            let mut s = world.resource_mut::<GameSession>();
            s.elapsed = 720.;
            s.duration = 100000.;
            s.core_hp = 1e9;
            s.core_max_hp = 1e9;
            s.auto_bond_draws = false;
            s.paused = false;
        }
        for mut hero in world
            .query_filtered::<&mut Transform, With<Hero>>()
            .iter_mut(world)
        {
            hero.translation = Vec3::new(-2.0, 0.0, 0.0);
        }
        world
            .resource_mut::<RecruitmentProgress>()
            .claimed_milestones = 7;
        world.resource_mut::<TrialState>().auto = false;
        world.resource_mut::<GoldChallengeState>().auto = false;
        world.resource_mut::<HeroVitals>().invulnerable_remaining = 1e6;
        world.resource_mut::<PlayerBuild>().skills[..6].fill(3);
        world
            .resource_mut::<Time<Virtual>>()
            .set_relative_speed(p.speed);
        p.initialized = true;
        println!(
            "PERF_ASSETS_READY speed={} readiness={}/{} warmup={}s sample={}s",
            p.speed,
            p.loaded,
            p.asset_ids.len(),
            p.warmup,
            p.seconds
        );
    }
    if p.initialized && p.ready.is_none() {
        p.imported_enemies = imported_targets(world);
        if p.imported_enemies == 180 {
            p.ready = Some(now);
            world.resource::<Shared>().0.lock().unwrap().ready = true;
            println!("PERF_MODELS_READY imported=180; warmup begins");
        }
    }
    if let Some(ready_at) = p.ready {
        if p.sample_start.is_none()
            && p.imported_enemies == 180
            && now.duration_since(ready_at).as_secs_f64() >= p.warmup
        {
            {
                let mut data = world.resource::<Shared>().0.lock().unwrap();
                data.measuring = true;
                data.last = None;
            }
            p.sample_start = Some(now);
            p.sample_health = Some(target_health(world));
            p.sample_casts_start
                .copy_from_slice(&world.resource::<SkillCastTelemetry>().casts[..6]);
            println!("PERF_SAMPLE_BEGIN");
        }
        if now >= p.next_counts {
            assert!(
                world.resource::<PlayerBuild>().skills[..6]
                    .iter()
                    .all(|rank| *rank == 3),
                "PERF_INVALID: fixture skill ranks changed"
            );
            let entities = world.entities().len();
            let enemies = world
                .query_filtered::<Entity, With<Enemy>>()
                .iter(world)
                .count();
            let projectiles = active_projectiles(world);
            p.imported_enemies = imported_targets(world);
            assert_eq!(
                enemies, 180,
                "PERF_INVALID: unexpected enemy spawn or fixture death"
            );
            assert_eq!(
                p.imported_enemies, 180,
                "PERF_INVALID: fixture model retired"
            );
            if p.sample_start.is_some() {
                p.peak_projectiles = p.peak_projectiles.max(projectiles);
            }
            let meshes = world
                .query_filtered::<Entity, With<Mesh3d>>()
                .iter(world)
                .count();
            let summons = world
                .query_filtered::<Entity, With<PlayerSummon>>()
                .iter(world)
                .count();
            let skill_casts = world.resource::<SkillCastTelemetry>().total;
            let s = world.resource::<GameSession>();
            p.counts.push(format!(
                "{:.6},{entities},{enemies},{projectiles},{meshes},{:.4},{},{},{},{},{skill_casts},{summons}",
                now.duration_since(ready_at).as_secs_f64(),
                s.elapsed,
                s.paused,
                s.finished,
                p.imported_enemies,
                world.resource::<Time<Virtual>>().relative_speed()
            ));
            p.next_counts = now + Duration::from_secs(1);
        }
        if p.sample_start.is_some() {
            if !p.damage_seen {
                let health = target_health(world);
                p.damage_seen = p.sample_health.is_some_and(|previous| health < previous);
                p.sample_health = Some(health);
            }
            if !p.projectile_seen {
                p.projectile_seen = active_projectiles(world) > 0;
            }
        }
        assert_eq!(
            world.resource::<Time<Virtual>>().relative_speed(),
            p.speed,
            "PERF_INVALID: requested virtual speed changed"
        );
        assert!(
            !world.resource::<GameSession>().paused && !world.resource::<GameSession>().finished,
            "PERF_INVALID: gameplay paused/finished"
        );
        if let Some(start) = p.sample_start
            && now.duration_since(start).as_secs_f64() >= p.seconds
        {
            for (index, count) in p.sample_skill_casts.iter_mut().enumerate() {
                *count = world.resource::<SkillCastTelemetry>().casts[index]
                    .saturating_sub(p.sample_casts_start[index]);
            }
            assert!(
                p.sample_skill_casts.iter().sum::<u64>() > 0,
                "PERF_INVALID: no production skill cast during measurement"
            );
            assert!(
                p.damage_seen && p.projectile_seen,
                "PERF_INVALID: no observed natural combat damage/projectile activity"
            );
            finish(
                &p,
                world.resource::<Shared>(),
                now.duration_since(start).as_secs_f64(),
            );
            world.write_message(AppExit::Success);
        }
    }
    world.insert_resource(p);
}
fn imported_targets(world: &mut World) -> usize {
    let mut instances = world.query_filtered::<
        &bevy::world_serialization::WorldInstance,
        (With<PerfEnemy>, With<Enemy>, With<Children>),
    >();
    let spawner = world.resource::<WorldInstanceSpawner>();
    instances
        .iter(world)
        .map(|instance| **instance)
        .filter(|id| spawner.instance_is_ready(*id))
        .count()
}

fn active_projectiles(world: &mut World) -> usize {
    world
        .query::<&Projectile>()
        .iter(world)
        .filter(|p| p.lifetime > 0.0)
        .count()
}
fn target_health(world: &mut World) -> f64 {
    world
        .query_filtered::<&Enemy, With<PerfEnemy>>()
        .iter(world)
        .map(|enemy| f64::from(enemy.hp))
        .sum()
}
fn quantile(sorted: &[f64], q: f64) -> f64 {
    sorted[((sorted.len() as f64 * q).ceil() as usize)
        .saturating_sub(1)
        .min(sorted.len() - 1)]
}
fn quote(s: &str) -> String {
    format!(
        "\"{}\"",
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
    )
}
fn finish(p: &Probe, shared: &Shared, wall: f64) {
    let mut d = shared.0.lock().unwrap();
    d.measuring = false;
    assert!(
        !d.intervals.is_empty(),
        "no actual swapchain present samples"
    );
    let mut sorted = d.intervals.clone();
    sorted.sort_by(f64::total_cmp);
    let summary = format!(
        "{{\n\"metric\":\"wall-clock intervals after actual swapchain present calls; not GPU timestamps or compositor displayed frames\",\n\"purpose\":{},\n\"concurrent_workload\":{},\n\"display\":{},\n\"adapter\":{},\n\"surface\":{},\n\"seed\":{},\n\"speed\":{},\n\"warmup_seconds\":{},\n\"startup_present_frames\":{},\n\"startup_wall_seconds\":{},\n\"warmup_wall_seconds\":{},\n\"warmup_present_frames\":{},\n\"sample_wall_seconds\":{},\n\"sample_present_frames\":{},\n\"sample_intervals\":{},\n\"p50_ms\":{},\n\"p95_ms\":{},\n\"p99_ms\":{},\n\"max_ms\":{},\n\"assets_ready\":{},\n\"assets_loaded\":{},\n\"assets_total\":{},\n\"build\":{},\n\"damage_observed\":{},\n\"projectiles_observed\":{},\n\"peak_projectiles_1hz\":{},\n\"sample_skill_casts\":{:?},\n\"fixture\":\"injected180 stationary 1e7HP enemies, hero at(-2,0,0), six skills rank3, elapsed720; no director/autoplay/auto trials, audio muted; not earned progression\"\n}}\n",
        quote(
            &std::env::var("ROGUE_PERF_PURPOSE")
                .unwrap_or_else(|_| "diagnostic; purpose not recorded".into())
        ),
        quote(
            &std::env::var("ROGUE_PERF_CONCURRENT_WORKLOAD")
                .unwrap_or_else(|_| "not recorded".into())
        ),
        quote(&format!(
            "DISPLAY={:?} WAYLAND_DISPLAY={:?}",
            std::env::var("DISPLAY"),
            std::env::var("WAYLAND_DISPLAY")
        )),
        quote(&d.adapter),
        quote(&d.surface),
        quote(&std::env::var("ROGUE_COMMANDER_SEED").unwrap_or_default()),
        p.speed,
        p.warmup,
        d.startup_frames,
        p.ready.unwrap().duration_since(p.created).as_secs_f64(),
        p.sample_start
            .unwrap()
            .duration_since(p.ready.unwrap())
            .as_secs_f64(),
        d.warmup_frames,
        wall,
        sorted.len() + 1,
        sorted.len(),
        quantile(&sorted, 0.5),
        quantile(&sorted, 0.95),
        quantile(&sorted, 0.99),
        sorted.last().unwrap(),
        p.initialized,
        p.loaded,
        p.asset_ids.len(),
        quote(
            &std::env::var("ROGUE_PERF_BUILD_PROFILE").unwrap_or_else(|_| {
                format!(
                    "unrecorded optimization profile; debug_assertions={}",
                    cfg!(debug_assertions)
                )
            })
        ),
        p.damage_seen,
        p.projectile_seen,
        p.peak_projectiles,
        p.sample_skill_casts
    );
    fs::write(format!("{}.json", p.output), summary).unwrap();
    fs::write(
        format!("{}-frames.csv", p.output),
        format!(
            "present_interval_ms\n{}",
            d.intervals
                .iter()
                .map(|v| format!("{v:.6}\n"))
                .collect::<String>()
        ),
    )
    .unwrap();
    fs::write(format!("{}-entities.csv",p.output),format!("seconds_since_readiness,entities,enemies,projectiles,meshes,virtual_elapsed,paused,finished,imported_enemy_roots,actual_virtual_speed,skill_casts_total,summons\n{}\n",p.counts.join("\n"))).unwrap();
    println!("PERF_PASS intervals={} output={}", sorted.len(), p.output);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nearest_rank_retains_tail_outliers() {
        let values: Vec<_> = (1..=1000).map(f64::from).collect();
        assert_eq!(quantile(&values, 0.5), 500.);
        assert_eq!(quantile(&values, 0.95), 950.);
        assert_eq!(quantile(&values, 0.99), 990.);
        assert_eq!(quantile(&[120.], 0.99), 120.);
    }
}
