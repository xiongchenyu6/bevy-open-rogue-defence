//! All twelve heroes' real basic attacks, isolated from learned skills and ultimates.
use bevy::prelude::*;
use rogue_commander::game::{
    ActiveSkill, Autoplay, Enemy, GameAssets, GameSession, Hero, HeroKind, HeroVitals, PlayerBuild,
    Projectile, RecruitmentProgress, SelectedHero,
};

#[derive(Component)]
pub(super) struct AttackTarget;

#[derive(Resource)]
struct WeaponGrowthProbe {
    scenario: String,
    frame: u32,
    advanced: bool,
    flight: bool,
    impact: bool,
    tier_material: bool,
    peak_emissive: f32,
}

pub fn configure_growth(world: &mut World, scenario: &str) {
    let (class, tier) = scenario
        .strip_suffix("-single")
        .unwrap_or(scenario)
        .strip_prefix("weapon-growth-")
        .unwrap()
        .rsplit_once('-')
        .unwrap();
    let kind = match class {
        "sword" => HeroKind::StoneWarden,
        "archer" => HeroKind::EmberRanger,
        "mage" => HeroKind::StormSavant,
        "void" => HeroKind::VoidReaper,
        "daggers" => HeroKind::RiftDancer,
        _ => panic!("unknown weapon class"),
    };
    assert!(matches!(tier, "starter" | "upgraded"));
    world.resource_mut::<SelectedHero>().kind = kind;
    world.resource_mut::<SelectedHero>().locked = true;
    world
        .resource_mut::<super::CaptureControls>()
        .suppress_drafts = true;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
    world
        .resource_mut::<rogue_commander::game::SpawnDirector>()
        .enabled = false;
    world.insert_resource(WeaponGrowthProbe {
        scenario: scenario.into(),
        frame: 0,
        advanced: tier == "upgraded",
        flight: false,
        impact: false,
        tier_material: false,
        peak_emissive: 0.,
    });
}

/// `-fragile` targets die in a few hits and revive, exercising kill feedback.
const FRAGILE_HP: f32 = 90.0;
const FRAGILE_REVIVE_FRAMES: u32 = 24;

#[derive(Component, Default)]
struct FragileRevive(u32);

#[derive(Resource)]
struct AttackCapture {
    frame: u32,
    route_fixture: bool,
    saw_damage: bool,
    saw_visual: bool,
    audio_verified: bool,
    audio_started: Option<f64>,
}

pub fn install(app: &mut bevy::app::SubApp, scenario: &str) {
    let world = app.world_mut();
    world.remove_resource::<Autoplay>();
    if std::env::var_os("ROGUE_CAPTURE_MELEE_AUDIO").is_some() {
        if !cfg!(any(debug_assertions, feature = "dev-tools")) {
            panic!("melee audio capture requires debug or dev-tools pending-count diagnostic");
        }
        world.insert_resource(bevy::time::TimeUpdateStrategy::Automatic);
        // Controlled recording fixture only: preserve a dodgeable windup even
        // on the first slow render. This is not the default performance case.
        world.resource_mut::<PlayerBuild>().attacks_per_second = 1.0;
        world
            .resource_mut::<Time<Virtual>>()
            .set_max_delta(std::time::Duration::from_millis(50));
        let mut preferences = world.resource_mut::<rogue_commander::game::AudioPreferences>();
        preferences.music = 0;
        preferences.effects = 100;
        preferences.muted = false;
    }
    world.resource_mut::<PlayerBuild>().skills.fill(0);
    world.resource_mut::<ActiveSkill>().remaining = 10_000.0;
    // Keep the pre-existing optional bond-route comparisons meaningful.
    if scenario == "attack-arcanum" {
        world.resource_mut::<PlayerBuild>().skills[0] = 1;
    }
    let growth = scenario.starts_with("weapon-growth-");
    if growth {
        let kind = world.resource::<SelectedHero>().kind;
        let level = if world.resource::<WeaponGrowthProbe>().advanced {
            40
        } else {
            1
        };
        *world.resource_mut::<HeroVitals>() = HeroVitals::for_hero(kind, level);
        world.resource_mut::<GameSession>().level = level;
        world
            .resource_mut::<RecruitmentProgress>()
            .claimed_milestones = 7;
    }
    let kind = world.resource::<SelectedHero>().kind;
    let melee = matches!(
        kind,
        HeroKind::StoneWarden
            | HeroKind::SolarVanguard
            | HeroKind::TideWalker
            | HeroKind::RiftDancer
            | HeroKind::IronBerserker
            | HeroKind::VoidReaper
    );
    let mut heroes = world.query_filtered::<&mut Transform, With<Hero>>();
    for mut hero in heroes.iter_mut(world) {
        hero.translation = Vec3::new(-2.0, 0.0, 0.0);
    }
    let center = if melee { -1.0 } else { 0.5 };
    if !growth {
        if std::env::var_os("ROGUE_CAPTURE_RELEASE_TRACE").is_some() {
            // Own this diagnostic view: responsive production framing otherwise
            // replaces the close-up before hands and release poses can be seen.
            let cameras: Vec<_> = world
                .query_filtered::<Entity, (With<Camera3d>, With<rogue_commander::game::BattleCamera>)>()
                .iter(world)
                .collect();
            assert!(
                !cameras.is_empty(),
                "release diagnostic requires battle camera"
            );
            for camera in cameras {
                world
                    .entity_mut(camera)
                    .remove::<rogue_commander::game::BattleCamera>();
            }
        }
        let mut cameras =
            world.query_filtered::<(&mut Transform, &mut Projection), With<Camera3d>>();
        for (mut transform, mut projection) in cameras.iter_mut(world) {
            *transform = Transform::from_xyz(center, 13.0, 11.0)
                .looking_at(Vec3::new(center, 0.0, 0.0), Vec3::Y);
            if let Projection::Orthographic(projection) = &mut *projection {
                projection.scaling_mode = bevy::camera::ScalingMode::FixedVertical {
                    viewport_height: 10.0,
                };
            }
        }
    }
    let scene = world.resource::<GameAssets>().enemy_scenes[0].clone();
    let fragile = scenario.ends_with("-fragile");
    let target_hp = if fragile { FRAGILE_HP } else { 100_000.0 };
    for index in 0..if scenario.ends_with("-single") { 1 } else { 3 } {
        let position = Vec3::new(
            if melee { -0.7 } else { 3.0 },
            0.0,
            (index as f32 - 1.0) * 1.2,
        );
        let mut target = world.spawn((
            Name::new(format!("Basic attack capture target {index}")),
            AttackTarget,
            WorldAssetRoot(scene.clone()),
            Transform::from_translation(position).with_scale(Vec3::splat(0.8)),
            Enemy {
                hp: target_hp,
                max_hp: target_hp,
                speed: 0.0,
                damage: 0.0,
                xp: 0,
                wood: 0,
                gold: 0,
                kill_credit: 0,
                elite: false,
            },
        ));
        if fragile {
            target.insert(FragileRevive::default());
        }
    }
    println!(
        "[capture] {scenario}: centered battlefield, real {kind:?} basic attacks; no Autoplay or ultimate"
    );
    if growth {
        app.add_systems(Last, audit_growth);
    }
    app.insert_resource(AttackCapture {
        frame: 0,
        route_fixture: scenario == "attack-arcanum",
        saw_damage: false,
        saw_visual: false,
        audio_verified: false,
        audio_started: None,
    })
    .add_systems(Last, (verify, revive_fragile_targets));
    if std::env::var_os("ROGUE_CAPTURE_HIT_MATERIAL_TRACE").is_some() {
        super::hit_material::install(app);
    }
    if std::env::var_os("ROGUE_CAPTURE_RELEASE_TRACE").is_some() {
        app.init_resource::<ReleaseTrace>()
            .add_systems(Last, trace_projectile_release);
    }
    if std::env::var_os("ROGUE_CAPTURE_ATTACK_TRACE").is_some() {
        app.init_resource::<AttackTimingTrace>()
            .add_systems(Last, trace_attack_timing);
    }
}

/// Opt-in observation at Last: public active projectile components only.
/// A carrier born and retired earlier in the same update is not observable here.
#[derive(Resource, Default)]
struct ReleaseTrace {
    frame: u32,
    seen: std::collections::HashSet<(Entity, u32)>,
}

fn trace_projectile_release(world: &mut World) {
    let frame = {
        let mut trace = world.resource_mut::<ReleaseTrace>();
        trace.frame += 1;
        trace.frame
    };
    if frame > 210 {
        return;
    }
    let animations: Vec<_> = world
        .query::<(Entity, &AnimationPlayer)>()
        .iter(world)
        .filter(|(entity, _)| {
            let mut ancestor = Some(*entity);
            while let Some(entity) = ancestor {
                if world.get::<Hero>(entity).is_some() {
                    return true;
                }
                ancestor = world.get::<ChildOf>(entity).map(ChildOf::parent);
            }
            false
        })
        .flat_map(|(_, player)| {
            player
                .playing_animations()
                .map(|(index, active)| (index.index(), active.seek_time(), active.speed()))
        })
        .collect();
    let projectiles: Vec<_> = world
        .query::<(Entity, &Projectile, &Transform)>()
        .iter(world)
        .map(|(entity, projectile, transform)| {
            let epoch = world
                .entity(entity)
                .get_change_ticks::<Projectile>()
                .expect("queried projectile has component ticks")
                .added
                .get();
            (entity, epoch, transform.translation, projectile.velocity)
        })
        .collect();
    let kind = world.resource::<HeroVitals>().kind;
    let mut trace = world.resource_mut::<ReleaseTrace>();
    println!("RELEASE_PHASE frame={frame} hero={kind:?} clips={animations:?}");
    for (entity, epoch, position, velocity) in projectiles {
        if trace.seen.insert((entity, epoch)) {
            // DamageProfile is gameplay-private; no new API is exposed for this probe.
            println!(
                "PROJECTILE_RELEASE frame={frame} entity={entity:?} added_tick={epoch} hero={kind:?} position={position:?} velocity={velocity:?} clips={animations:?}"
            );
        }
    }
}

#[derive(Resource, Default)]
struct AttackTimingTrace {
    frame: u32,
    previous_health: Option<f32>,
    miss_started: Option<f64>,
    miss_restored: bool,
    displaced: Vec<(Entity, Vec3)>,
}

/// Observe actual damage and authored playback together, without injecting hits.
fn trace_attack_timing(world: &mut World) {
    let health: f32 = world
        .query_filtered::<&Enemy, With<AttackTarget>>()
        .iter(world)
        .map(|enemy| enemy.hp)
        .sum();
    let (frame, damage) = {
        let mut trace = world.resource_mut::<AttackTimingTrace>();
        trace.frame += 1;
        let damage = trace.previous_health.map_or(0.0, |last| last - health);
        trace.previous_health = Some(health);
        (trace.frame, damage)
    };
    if std::env::var_os("ROGUE_CAPTURE_MELEE_AUDIO").is_some() {
        let now = world.resource::<Time<Real>>().elapsed_secs_f64();
        let wall = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();
        if damage > 0.0 {
            println!(
                "MELEE_DAMAGE_WALL frame={frame} wall={wall:.6} damage={damage:.3} health={health:.3}"
            );
        }
        let committed = world
            .resource::<rogue_commander::game::BasicAttackCounter>()
            .count()
            > 0;
        let (started, restored) = {
            let p = world.resource::<AttackTimingTrace>();
            (p.miss_started, p.miss_restored)
        };
        if committed && started.is_none() && !restored {
            assert_eq!(
                damage, 0.0,
                "first controlled miss must move targets before contact"
            );
            let displaced: Vec<_> = world
                .query_filtered::<(Entity, &Transform), With<AttackTarget>>()
                .iter(world)
                .map(|(e, t)| (e, t.translation))
                .collect();
            for (e, _) in &displaced {
                world.get_mut::<Transform>(*e).unwrap().translation.z += 100.0;
            }
            let mut p = world.resource_mut::<AttackTimingTrace>();
            p.displaced = displaced;
            p.miss_started = Some(now);
            println!(
                "MELEE_MISS_BEGIN frame={frame} real={now:.6} wall={wall:.6} health={health:.3}"
            );
        } else if let Some(started) = started
            && !restored
        {
            assert_eq!(
                damage, 0.0,
                "controlled displaced targets must not take hits"
            );
            #[cfg(any(debug_assertions, feature = "dev-tools"))]
            let pending = rogue_commander::game::pending_melee_count(world);
            #[cfg(not(any(debug_assertions, feature = "dev-tools")))]
            let pending = usize::MAX; // Optional probe rejected explicitly at installation.
            if now - started >= 0.65 && pending == 0 {
                let displaced =
                    std::mem::take(&mut world.resource_mut::<AttackTimingTrace>().displaced);
                for (e, position) in displaced {
                    world.get_mut::<Transform>(e).unwrap().translation = position;
                }
                world.resource_mut::<AttackTimingTrace>().miss_restored = true;
                println!(
                    "MELEE_MISS_END frame={frame} real={now:.6} wall={wall:.6} health={health:.3} pending={pending}"
                );
            }
        }
        let real = world.resource::<Time<Real>>().elapsed_secs_f64();
        let virtual_time = world.resource::<Time>().elapsed_secs_f64();
        let voices: Vec<_> = world
            .query::<(Entity, &Name, &AudioPlayer, Option<&bevy::audio::AudioSink>)>()
            .iter(world)
            .filter(|(_, name, _, _)| name.as_str() == "Melee swing whoosh")
            .map(|(entity, _, player, sink)| (entity, player.0.clone(), sink.is_some()))
            .collect();
        let assets = world.resource::<AssetServer>();
        for (entity, handle, sink) in voices {
            println!(
                "MELEE_AUDIO frame={frame} real={real:.6} virtual={virtual_time:.6} entity={entity:?} source={:?} sink={sink} health={health:.3} damage={damage:.3}",
                assets.get_path(handle.id())
            );
        }
        println!(
            "MELEE_CONTACT frame={frame} real={real:.6} virtual={virtual_time:.6} health={health:.3} damage={damage:.3}"
        );
    }
    if frame > 80 {
        return;
    }
    let trails: Vec<_> = world
        .query::<(
            Entity,
            &Name,
            &MeshMaterial3d<StandardMaterial>,
            &Visibility,
        )>()
        .iter(world)
        .filter(|(_, name, _, visibility)| {
            name.as_str() == "Weapon swing trail" && **visibility != Visibility::Hidden
        })
        .map(|(entity, _, material, _)| (entity, material.0.clone()))
        .collect();
    let materials = world.resource::<Assets<StandardMaterial>>();
    for (entity, handle) in trails {
        if let Some(material) = materials.get(&handle) {
            println!(
                "SWORD_TRAIL frame={frame} entity={entity:?} alpha={:.6} mode={:?} material_count={}",
                material.base_color.alpha(),
                material.alpha_mode,
                materials.len()
            );
        }
    }
    let mut players = world.query::<(Entity, &AnimationPlayer)>();
    let animations: Vec<_> = players
        .iter(world)
        .filter(|(entity, _)| {
            let mut current = Some(*entity);
            while let Some(entity) = current {
                if world.get::<Hero>(entity).is_some() {
                    return true;
                }
                current = world.get::<ChildOf>(entity).map(ChildOf::parent);
            }
            false
        })
        .flat_map(|(_, player)| {
            player
                .playing_animations()
                .map(|(index, active)| (index.index(), active.seek_time(), active.speed()))
        })
        .collect();
    println!(
        "ATTACK_TIMING frame={frame} health={health:.3} damage={damage:.3} clips={animations:?}"
    );
}

fn revive_fragile_targets(mut targets: Query<(&mut Enemy, &mut FragileRevive)>) {
    for (mut enemy, mut revive) in &mut targets {
        if enemy.hp > 0.0 {
            continue;
        }
        revive.0 += 1;
        if revive.0 >= FRAGILE_REVIVE_FRAMES {
            enemy.hp = enemy.max_hp;
            revive.0 = 0;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn verify(
    mut capture: ResMut<AttackCapture>,
    real: Res<Time<Real>>,
    audio_trace: Option<Res<AttackTimingTrace>>,
    names: Query<&Name>,
    enemies: Query<&Enemy, With<AttackTarget>>,
    build: Res<PlayerBuild>,
    active: Res<ActiveSkill>,
    autoplay: Option<Res<Autoplay>>,
) {
    capture.frame += 1;
    capture.saw_damage |= enemies.iter().any(|enemy| enemy.hp < enemy.max_hp);
    capture.saw_damage |= capture.frame > 1 && enemies.iter().any(|enemy| enemy.hp <= 0.0);
    capture.saw_visual |= names.iter().any(|name| {
        matches!(
            name.as_str(),
            "Weapon trail / impact" | "Weapon swing trail"
        )
    });
    let audio_probe = std::env::var_os("ROGUE_CAPTURE_MELEE_AUDIO").is_some();
    let audio_started = *capture.audio_started.get_or_insert(real.elapsed_secs_f64());
    if audio_probe && !capture.audio_verified {
        assert!(
            real.elapsed_secs_f64() - audio_started < 30.0,
            "controlled melee audio fixture exceeded 30 real seconds without restored-target damage"
        );
    }
    let audio_ready = audio_probe
        && !capture.audio_verified
        && capture.saw_damage
        && audio_trace
            .as_ref()
            .is_some_and(|trace| trace.miss_restored);
    if (!audio_probe && capture.frame == 205) || audio_ready {
        capture.audio_verified = true;
        assert!(capture.saw_damage, "real basic attacks must damage enemies");
        assert!(
            capture.saw_visual,
            "real basic attacks must produce weapon visuals"
        );
        assert!(autoplay.is_none());
        assert!(
            active.remaining > 9_000.0,
            "automatic ultimate must remain suppressed"
        );
        if !capture.route_fixture {
            assert!(build.skills.iter().all(|rank| *rank == 0));
        }
        println!(
            "[capture] PASS: real basic attack damage and weapon visuals; learned skills/ultimate/Autoplay suppressed"
        );
    }
}

fn audit_growth(world: &mut World) {
    world.resource_mut::<WeaponGrowthProbe>().frame += 1;
    let frame = world.resource::<WeaponGrowthProbe>().frame;
    let advanced = world.resource::<WeaponGrowthProbe>().advanced;
    let weapon_rows: Vec<_> = world
        .query::<(&Name, &MeshMaterial3d<StandardMaterial>)>()
        .iter(world)
        .filter(|(name, _)| {
            matches!(
                name.as_str(),
                "Weapon swing trail"
                    | "Weapon trail / impact"
                    | "Warrior cleave sweep"
                    | "Archer ricochet trail"
            )
        })
        .map(|(_, handle)| handle.0.clone())
        .collect();
    let mut emissions = Vec::new();
    for handle in weapon_rows {
        if let Some(material) = world.resource::<Assets<StandardMaterial>>().get(&handle) {
            emissions.push(
                material
                    .emissive
                    .red
                    .max(material.emissive.green)
                    .max(material.emissive.blue),
            );
        }
    }
    let projectile_rows: Vec<_> = world
        .query::<(&Projectile, &MeshMaterial3d<StandardMaterial>)>()
        .iter(world)
        .filter(|(projectile, _)| projectile.lifetime > 0.)
        .map(|(_, handle)| handle.0.clone())
        .collect();
    for handle in &projectile_rows {
        if let Some(material) = world.resource::<Assets<StandardMaterial>>().get(handle) {
            emissions.push(
                material
                    .emissive
                    .red
                    .max(material.emissive.green)
                    .max(material.emissive.blue),
            );
        }
    }
    let swing = world
        .query::<&Name>()
        .iter(world)
        .any(|name| name.as_str() == "Weapon swing trail");
    if !world.resource::<WeaponGrowthProbe>().flight && (swing || !projectile_rows.is_empty()) {
        if world.resource::<SelectedHero>().kind == HeroKind::EmberRanger {
            let expected = if !advanced
                || world
                    .resource::<WeaponGrowthProbe>()
                    .scenario
                    .ends_with("-single")
            {
                1
            } else {
                3
            };
            assert_eq!(
                projectile_rows.len(),
                expected,
                "initial volley uses one arrow per distinct target; starter has one arrow"
            );
            println!("[capture] PASS archer-first-volley arrows={expected}");
        }
        world.resource_mut::<WeaponGrowthProbe>().flight = true;
        println!("[capture] weapon-growth-stage actual-flight frame={frame}");
    }
    if !world.resource::<WeaponGrowthProbe>().impact
        && world
            .query_filtered::<&Enemy, With<AttackTarget>>()
            .iter(world)
            .any(|enemy| enemy.hp < enemy.max_hp)
    {
        world.resource_mut::<WeaponGrowthProbe>().impact = true;
        println!("[capture] weapon-growth-stage actual-impact frame={frame}");
    }
    if let Some(peak) = emissions.into_iter().reduce(f32::max) {
        let mut probe = world.resource_mut::<WeaponGrowthProbe>();
        probe.peak_emissive = probe.peak_emissive.max(peak);
        probe.tier_material |= if advanced { peak >= 3.5 } else { peak <= 0.2 };
    }
    if frame == 230 {
        let probe = world.resource::<WeaponGrowthProbe>();
        assert!(
            probe.flight && probe.impact && probe.tier_material,
            "real ordinary attack flight/impact/tier material observed"
        );
        if !advanced {
            assert!(
                probe.peak_emissive <= 0.2,
                "starter has no high-emission weapon material: {}",
                probe.peak_emissive
            );
        }
        assert_eq!(
            world.resource::<HeroVitals>().level,
            if advanced { 40 } else { 1 }
        );
        println!(
            "[capture] PASS {}: production attack flight+damage, actual material peak_emissive={}, no learnedskills/ultimate",
            probe.scenario, probe.peak_emissive
        );
    }
}
