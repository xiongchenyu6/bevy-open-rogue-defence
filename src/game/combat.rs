use super::attack_vfx::{self, WeaponCues, WeaponFlight};
use std::collections::{HashMap, HashSet, VecDeque};

use bevy::{
    camera::{ClearColorConfig, Hdr, RenderTarget, visibility::RenderLayers},
    core_pipeline::tonemapping::Tonemapping,
    ecs::system::SystemParam,
    prelude::*,
};

use super::archive::{ArchiveEcho, ArchiveOutcome, ArchiveState};
use super::basic_attacks::{self, BasicAttackAim, BasicAttackArt, PendingSwing};
use super::bonds::{BondRuntime, DamageClass, DamageElement, DamageProfile, critical_multiplier};
use super::bosses::BossSummon;
use super::content::{BOSSES, ENEMIES, EnemyBehavior};
use super::encounters::EncounterState;
use super::hero_abilities::{BasicAttackCounter, HeroAttackSpeedModifier, TimedSpeedEffect};
use super::hit_feedback::{HitEvent, HitFeedback, HitWeight};
use super::market::MarketEffects;
use super::pooling::{CombatPools, DAMAGE_NUMBER_RENDER_LAYER, EnemySceneKey};
use super::skills::SkillRuntime;
use super::trial::{complete_gold_target, complete_tower_challenge};
use super::waves::{
    WAVES, WaveDef, choose_archetype, rush_champion, rush_warning, scheduled_spawn_count,
};
mod enemy_pressure;
pub use enemy_pressure::{EnemyPressure, EnemyPressureSettings};
mod elite_pressure;
pub use elite_pressure::{BLOODRITE_RADIUS, ElitePressure, YELLOW_SIGN_RADIUS};
#[derive(Resource, Default)]
pub struct RaidWarning {
    pub wave: u32,
    pub remaining_secs: f32,
    pub count: u16,
    pub origin: Vec3,
}
use super::{
    ActiveSkill, AttackClock, BossEnemy, ChallengeKind, Core, DamageMeter, DodgeState, Enemy,
    EnemyArchetype, EnemyAttackCue, EnemyAttackKind, EnemyHeroThreat, GameAssets, GameSession,
    GoldChallengeState, HERO_RESPAWN_INVULNERABILITY, HERO_RESPAWN_POSITION, Hero, HeroKind,
    HeroVitals, ImpactFx, MoveIntent, PlayerBuild, Projectile, RunRng, SpawnDirector, TrialEnemy,
    TrialState, revive_delay_for_death,
};

pub(crate) const MAX_REGULAR_ENEMIES: usize = 130;
const ARCHETYPE_VISUALS: [usize; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
const ACID_ARCHER_RANGE: f32 = 6.0;
const ACID_ARCHER_ATTACK_INTERVAL: f32 = 1.0;
const BURROW_REVEAL_RANGE: f32 = 1.6;
const BURROW_ATTACK_RANGE: f32 = 1.35;
const BURROW_ATTACK_INTERVAL: f32 = 1.1;
const BURROW_STUN_SECONDS: f32 = 1.0;
const ELITE_ACTION_SPEED_MULTIPLIER: f32 = 1.35;
const BULWARK_SHIELD_INTERVAL: f32 = 8.0;
const BULWARK_SHIELD_FRACTION: f32 = 0.25;
const VOLATILE_WARNING_SECONDS: f32 = 1.0;
const VOLATILE_BLAST_RADIUS: f32 = 3.75;
const VOLATILE_DAMAGE_MULTIPLIER: f32 = 2.5;
const REGENERATION_HEALTH_FRACTION: f32 = 0.35;
const COMMANDER_AURA_RADIUS: f32 = 5.5;
const COMMANDER_AURA_MULTIPLIER: f32 = 1.2;
const BACKLOG_MERGE_WAIT_SECONDS: f32 = 8.0;

#[cfg(any(debug_assertions, feature = "dev-tools"))]
pub(crate) mod debug_tools;

#[derive(Resource, Default)]
struct WaveSpawnRuntime {
    wave: u32,
    spawned: u16,
    accumulator: f32,
    backlog: VecDeque<SpawnBudget>,
    capped_for: f32,
    merge_next_slot: bool,
}

#[derive(Debug, Clone, Copy)]
struct SpawnBudget {
    wave: WaveDef,
    archetype: usize,
    elite: bool,
    /// Elite Hunt encounter: tougher, pays extra wood.
    hunted: bool,
    affix: Option<EliteAffixKind>,
    spawn_x: f32,
    spawn_z: f32,
    scale_jitter: f32,
    speed_jitter: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BacklogSpawnDecision {
    Idle,
    Hold,
    Individuals(usize),
    Merged,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct EnemyArmor(pub(crate) f32);

#[derive(Component, Clone, Copy)]
pub(crate) struct EnemyBreachDamage(pub(crate) f32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EliteAffixKind {
    Frenzy,
    Bulwark,
    Volatile,
    Regeneration,
    Commander,
    YellowSign,
    Brood,
    Siege,
}

impl EliteAffixKind {
    const fn label(self) -> &'static str {
        match self {
            Self::Frenzy => "FRENZY",
            Self::Bulwark => "BULWARK",
            Self::Volatile => "VOLATILE",
            Self::Regeneration => "REGENERATION",
            Self::Commander => "COMMANDER",
            Self::YellowSign => "YELLOW SIGN",
            Self::Brood => "BROOD",
            Self::Siege => "SIEGE",
        }
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct EliteAffix {
    kind: EliteAffixKind,
    shield: f32,
    shield_remaining: f32,
    regenerated: bool,
    volatile_remaining: f32,
}

impl EliteAffix {
    const fn new(kind: EliteAffixKind) -> Self {
        Self {
            kind,
            shield: 0.0,
            shield_remaining: BULWARK_SHIELD_INTERVAL,
            regenerated: false,
            volatile_remaining: 0.0,
        }
    }

    pub const fn kind(&self) -> EliteAffixKind {
        self.kind
    }
    pub const fn shield_active(&self) -> bool {
        self.shield > 0.0
    }
    pub const fn is_volatile_pending(self) -> bool {
        self.volatile_remaining > 0.0
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct CommanderAuraState {
    base_max_hp: f32,
    active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DamageOutcome {
    Alive,
    Regenerated,
    VolatileStarted,
    Defeated,
    Pending,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct DamageResolution {
    outcome: DamageOutcome,
    hp_damage: f32,
}

impl DamageResolution {
    const fn new(outcome: DamageOutcome, hp_damage: f32) -> Self {
        Self { outcome, hp_damage }
    }
}

#[derive(Component, Default)]
pub(crate) struct ProjectileHitMemory {
    enemies: HashSet<Entity>,
}

/// Per-frame work budget. Excess completions are retained by
/// [`AttackVolleyRuntime`] and resolved on later active frames.
pub(crate) const MAX_ATTACK_CATCH_UP_VOLLEYS: u32 = 8;

#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct ProjectileFirstFrameTravel(pub(crate) f32);

#[derive(Resource, Default, Debug, Clone, Copy)]
pub(crate) struct AttackVolleyRuntime {
    pub(crate) pending: u64,
    pub(crate) resolved_this_frame: u32,
}

#[derive(SystemParam)]
struct ProjectileHitFrame<'w, 's> {
    heroes: Query<'w, 's, &'static Transform, With<Hero>>,
    cues: Option<ResMut<'w, WeaponCues>>,
    hit_feedback: Option<ResMut<'w, HitFeedback>>,
    time: Res<'w, Time>,
    session: ResMut<'w, GameSession>,
    encounters: ResMut<'w, EncounterState>,
}

fn catch_up_volley_travel_seconds(
    clock: &AttackClock,
    raw_volley_count: u32,
    volley_index: u32,
    delta: f32,
) -> f32 {
    let older_completions = raw_volley_count
        .saturating_sub(1)
        .saturating_sub(volley_index);
    (clock.0.elapsed_secs() + clock.0.duration().as_secs_f32() * older_completions as f32)
        .clamp(0.0, delta.max(0.0))
}

pub struct CombatPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct CombatSet;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct SurvivalSet;

#[derive(Component)]
struct DamageNumberCamera;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EnemyPressureSettings>()
            .init_resource::<WaveSpawnRuntime>()
            .init_resource::<RaidWarning>()
            .init_resource::<ElitePressure>()
            .init_resource::<DamageMeter>()
            .init_resource::<AttackVolleyRuntime>()
            .init_resource::<BasicAttackAim>()
            .init_resource::<WeaponCues>()
            .add_systems(Startup, attack_vfx::setup)
            .add_systems(Startup, basic_attacks::setup_attack_art)
            .add_systems(PostStartup, setup_damage_number_camera)
            .add_systems(Update, tick_hero_survival.in_set(SurvivalSet))
            .add_systems(
                Update,
                (
                    tick_gate_rebuild,
                    advance_run,
                    tick_damage_meter,
                    spawn_wave,
                    enemy_auras,
                    elite_pressure::tick_pressure,
                    commander_aura,
                    tick_elite_affixes,
                    move_enemies,
                    auto_attack,
                    basic_attacks::resolve_swings,
                    move_projectiles,
                    projectile_hits,
                    cast_active_skill,
                    animate_effects,
                    basic_attacks::animate_swings,
                    attack_vfx::animate,
                    cleanup_finished_enemies,
                )
                    .chain()
                    .after(SurvivalSet)
                    .in_set(CombatSet),
            )
            .add_systems(
                Update,
                update_damage_numbers
                    .after(projectile_hits)
                    .before(cast_active_skill)
                    .in_set(CombatSet),
            );
        #[cfg(any(debug_assertions, feature = "dev-tools"))]
        app.add_systems(
            Update,
            debug_tools::debug_shortcuts
                .after(attack_vfx::animate)
                .before(cleanup_finished_enemies)
                .in_set(CombatSet),
        );
    }
}

fn setup_damage_number_camera(
    mut commands: Commands,
    source: Query<(&Camera, &RenderTarget), With<Camera3d>>,
) {
    let Ok((source, target)) = source.single() else {
        return;
    };
    commands.spawn((
        Name::new("DAMAGE NUMBER CAMERA"),
        DamageNumberCamera,
        Camera2d,
        // Camera stacking shares main textures only when HDR and MSAA match.
        // The world camera already applied tonemapping before this overlay.
        Hdr,
        Tonemapping::None,
        // Render interface text after floating combat numbers, including tooltips.
        IsDefaultUiCamera,
        Camera {
            order: source.order.saturating_add(1),
            clear_color: ClearColorConfig::None,
            ..default()
        },
        target.clone(),
        RenderLayers::layer(DAMAGE_NUMBER_RENDER_LAYER),
        Msaa::Off,
    ));
}

#[allow(clippy::too_many_arguments)]
fn tick_hero_survival(
    time: Res<Time>,
    mut session: ResMut<GameSession>,
    mut vitals: ResMut<HeroVitals>,
    mut market_effects: ResMut<MarketEffects>,
    mut bond_runtime: ResMut<BondRuntime>,
    mut dodge: ResMut<DodgeState>,
    mut intent: ResMut<MoveIntent>,
    mut hero: Query<(&mut Transform, &mut Visibility), With<Hero>>,
) {
    if session.paused || session.finished {
        return;
    }
    let delta = time.delta_secs();
    vitals.invulnerable_remaining = (vitals.invulnerable_remaining - delta).max(0.0);
    vitals.stunned_remaining = (vitals.stunned_remaining - delta).max(0.0);

    let Ok((mut transform, mut visibility)) = hero.single_mut() else {
        return;
    };
    if vitals.alive && vitals.health <= 0.0 {
        begin_hero_death(
            &mut session,
            &mut vitals,
            &mut market_effects,
            &mut bond_runtime,
            &mut dodge,
            &mut intent,
            &mut transform,
            &mut visibility,
        );
        return;
    }
    if vitals.alive {
        return;
    }

    vitals.revive_remaining = (vitals.revive_remaining - delta).max(0.0);
    if vitals.revive_remaining > 0.0 {
        return;
    }
    vitals.alive = true;
    vitals.health = vitals.max_health;
    vitals.energy = vitals.max_energy;
    vitals.invulnerable_remaining = HERO_RESPAWN_INVULNERABILITY;
    vitals.stunned_remaining = 0.0;
    transform.translation = HERO_RESPAWN_POSITION;
    transform.rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
    *visibility = Visibility::Visible;
}

fn begin_hero_death(
    session: &mut GameSession,
    vitals: &mut HeroVitals,
    market_effects: &mut MarketEffects,
    bond_runtime: &mut BondRuntime,
    dodge: &mut DodgeState,
    intent: &mut MoveIntent,
    transform: &mut Transform,
    visibility: &mut Visibility,
) {
    if !vitals.alive {
        return;
    }
    if vitals.health <= 0.0
        && (bond_runtime.intercept_fatal(vitals) || market_effects.intercept_hero_fatal(vitals))
    {
        return;
    }
    vitals.alive = false;
    vitals.health = 0.0;
    vitals.deaths = vitals.deaths.saturating_add(1);
    vitals.revive_remaining = if vitals.lives_remaining() > 0 {
        revive_delay_for_death(vitals.deaths)
    } else {
        0.0
    };
    if vitals.lives_remaining() == 0 {
        session.finished = true;
        session.victory = false;
        session.paused = true;
    }
    vitals.invulnerable_remaining = 0.0;
    vitals.stunned_remaining = 0.0;
    dodge.requested = false;
    intent.0 = Vec2::ZERO;
    // The fallen hero is hidden until the surviving life revives.
    transform.translation = Vec3::new(0.0, -1_000.0, 0.0);
    *visibility = Visibility::Hidden;
}

fn tick_gate_rebuild(
    time: Res<Time>,
    mut session: ResMut<GameSession>,
    mut gates: Query<&mut Visibility, With<Core>>,
) {
    if session.paused || session.finished {
        return;
    }
    if session.gate_revive_remaining > 0.0 {
        session.gate_revive_remaining =
            (session.gate_revive_remaining - time.delta_secs()).max(0.0);
        session.core_hp = if session.gate_revive_remaining == 0.0 {
            session.core_max_hp
        } else {
            0.0
        };
    } else if session.core_hp <= 0.0 {
        session.gate_revive_remaining = super::resources::GATE_REBUILD_SECONDS;
    }
    for mut visible in &mut gates {
        *visible = if session.gate_revive_remaining > 0.0 {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }
}

fn advance_run(time: Res<Time>, mut session: ResMut<GameSession>) {
    if session.paused || session.finished {
        return;
    }
    if session.save_active {
        return;
    }
    session.elapsed += time.delta_secs();
    session.gold_income_accum += time.delta_secs() * 2.0;
    let income = session.gold_income_accum.floor() as u32;
    if income > 0 {
        session.gold += income;
        session.gold_income_accum -= income as f32;
    }
    session.wave = ((session.elapsed / 60.0).floor() as u32 + 1).min(25);
}

fn tick_damage_meter(
    time: Res<Time>,
    session: Res<GameSession>,
    mut damage_meter: ResMut<DamageMeter>,
) {
    if session.paused || session.finished {
        return;
    }
    damage_meter.advance(time.delta_secs());
}

fn cleanup_finished_enemies(
    mut commands: Commands,
    session: Res<GameSession>,
    mut pools: ResMut<CombatPools>,
    projectile_markers: Query<Entity, With<ProjectileFirstFrameTravel>>,
) {
    if session.finished {
        for entity in &projectile_markers {
            commands
                .entity(entity)
                .remove::<ProjectileFirstFrameTravel>();
        }
        pools.retire_all_enemies(&mut commands);
        pools.retire_all_projectiles(&mut commands);
        pools.retire_all_effects(&mut commands);
        pools.retire_all_damage_numbers(&mut commands);
    }
}

fn update_damage_numbers(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    archive: Res<ArchiveState>,
    mut pools: ResMut<CombatPools>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut archive_was_active: Local<bool>,
) {
    let archive_started = archive.active && !*archive_was_active;
    *archive_was_active = archive.active;
    if session.finished {
        pools.retire_all_damage_numbers(&mut commands);
        return;
    }
    if archive_started {
        pools.retire_all_damage_numbers(&mut commands);
    }
    if session.paused {
        return;
    }
    pools.tick_damage_numbers(&mut commands, time.delta_secs(), cameras.single().ok());
}

fn pressure_wave(
    mut wave: WaveDef,
    number: u32,
    settings: Option<&EnemyPressureSettings>,
) -> WaveDef {
    wave.hp_multiplier *= settings.map_or(1.0, |settings| settings.at_wave(number).health);
    wave
}

fn spawn_wave(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    mut director: ResMut<SpawnDirector>,
    mut wave_runtime: ResMut<WaveSpawnRuntime>,
    mut raid: Option<ResMut<RaidWarning>>,
    mut rng: ResMut<RunRng>,
    mut encounters: ResMut<EncounterState>,
    mut pools: ResMut<CombatPools>,
    assets: Option<Res<GameAssets>>,
    pressure: Option<Res<EnemyPressureSettings>>,
    living_enemies: Query<
        (),
        (
            With<Enemy>,
            Without<BossEnemy>,
            Without<BossSummon>,
            Without<super::encounter_bosses::EncounterGuard>,
            Without<TrialEnemy>,
            Without<ArchiveEcho>,
        ),
    >,
) {
    if !director.enabled || session.paused || session.finished || session.save_active {
        return;
    }
    let Some(assets) = assets else {
        return;
    };

    for (index, boss) in BOSSES.iter().enumerate() {
        if session.elapsed + encounters.boss_lead_seconds() >= boss.spawn_seconds
            && !director.boss_spawned[index]
            && spawn_boss(&mut commands, &mut pools, &assets, index).is_some()
        {
            director.boss_spawned[index] = true;
            encounters.note_boss_spawned(index as u8);
        }
    }
    drain_splitling_backlog(&mut commands, &mut pools, &assets);
    let occupied = living_enemies
        .iter()
        .count()
        .max(pools.regular_enemy_active_count());

    let delta = time.delta_secs();
    let wave_index = session.wave.saturating_sub(1).min(24) as usize;
    let wave = pressure_wave(WAVES[wave_index], session.wave, pressure.as_deref());
    let interval = wave_spawn_interval(wave);
    let minute = session.elapsed.rem_euclid(60.0);
    if let Some(raid) = raid.as_deref_mut() {
        let warning = rush_warning(session.wave, wave, minute);
        raid.wave = session.wave;
        raid.remaining_secs = warning.map_or(0.0, |value| value.0);
        raid.count = warning.map_or(0, |value| value.1);
        raid.origin = Vec3::new(16.0, 0.0, 0.0);
    }
    if wave_runtime.wave != session.wave {
        if wave_runtime.wave > 0 {
            let previous = pressure_wave(
                WAVES[wave_runtime.wave.saturating_sub(1).min(24) as usize],
                wave_runtime.wave,
                pressure.as_deref(),
            );
            while wave_runtime.spawned < previous.count {
                let mut budget = next_spawn_budget(
                    previous,
                    session.elapsed,
                    &mut director,
                    &mut encounters,
                    &mut rng,
                );
                promote_pressure_elite(
                    &mut budget,
                    wave_runtime.wave,
                    wave_runtime.spawned,
                    &mut rng,
                );
                wave_runtime.backlog.push_back(budget);
                wave_runtime.spawned += 1;
            }
            for skipped_wave in wave_runtime.wave + 1..session.wave {
                let skipped = pressure_wave(
                    WAVES[skipped_wave.saturating_sub(1).min(24) as usize],
                    skipped_wave,
                    pressure.as_deref(),
                );
                for spawned in 0..skipped.count {
                    let mut budget = next_spawn_budget(
                        skipped,
                        session.elapsed,
                        &mut director,
                        &mut encounters,
                        &mut rng,
                    );
                    promote_pressure_elite(&mut budget, skipped_wave, spawned, &mut rng);
                    wave_runtime.backlog.push_back(budget);
                }
            }
        }
        wave_runtime.wave = session.wave;
        wave_runtime.spawned = 0;
        // The first unit enters at the start of each minute, followed by an
        // even count/60 cadence. This keeps all N entries inside that minute.
        wave_runtime.accumulator = interval;
    }
    wave_runtime.accumulator = minute;
    let due = scheduled_spawn_count(session.wave, wave, wave_runtime.accumulator);
    while wave_runtime.spawned < due {
        let mut budget = next_spawn_budget(
            wave,
            session.elapsed,
            &mut director,
            &mut encounters,
            &mut rng,
        );
        promote_pressure_elite(&mut budget, session.wave, wave_runtime.spawned, &mut rng);
        wave_runtime.backlog.push_back(budget);
        wave_runtime.spawned += 1;
    }

    match backlog_spawn_decision(
        wave_runtime.backlog.len(),
        occupied,
        wave_runtime.merge_next_slot,
    ) {
        BacklogSpawnDecision::Idle => {
            wave_runtime.capped_for = 0.0;
            wave_runtime.merge_next_slot = false;
        }
        BacklogSpawnDecision::Hold => {
            wave_runtime.capped_for += delta;
            if backlog_merge_ready(wave_runtime.capped_for) {
                wave_runtime.merge_next_slot = true;
            }
        }
        BacklogSpawnDecision::Merged => {
            let merged: Vec<_> = wave_runtime.backlog.drain(..).collect();
            if spawn_merged_enemy(&mut commands, &mut pools, &assets, &merged).is_some() {
                wave_runtime.capped_for = 0.0;
                wave_runtime.merge_next_slot = false;
            } else {
                for budget in merged.into_iter().rev() {
                    wave_runtime.backlog.push_front(budget);
                }
                wave_runtime.capped_for += delta;
            }
        }
        BacklogSpawnDecision::Individuals(slots) => {
            let mut pool_blocked = false;
            for _ in 0..slots {
                let Some(budget) = wave_runtime.backlog.front().copied() else {
                    break;
                };
                if spawn_enemy(&mut commands, &mut pools, &assets, budget).is_none() {
                    pool_blocked = true;
                    break;
                }
                wave_runtime.backlog.pop_front();
            }
            if wave_runtime.backlog.is_empty() {
                wave_runtime.capped_for = 0.0;
                wave_runtime.merge_next_slot = false;
            } else if pool_blocked {
                wave_runtime.capped_for += delta;
                if backlog_merge_ready(wave_runtime.capped_for) {
                    wave_runtime.merge_next_slot = true;
                }
            }
        }
    }
}

fn backlog_merge_ready(capped_for: f32) -> bool {
    capped_for >= BACKLOG_MERGE_WAIT_SECONDS
}

fn backlog_spawn_decision(
    pending: usize,
    occupied: usize,
    merge_next_slot: bool,
) -> BacklogSpawnDecision {
    if pending == 0 {
        BacklogSpawnDecision::Idle
    } else if occupied >= MAX_REGULAR_ENEMIES {
        BacklogSpawnDecision::Hold
    } else if merge_next_slot {
        BacklogSpawnDecision::Merged
    } else {
        BacklogSpawnDecision::Individuals(pending.min(MAX_REGULAR_ENEMIES - occupied))
    }
}

fn promote_pressure_elite(
    budget: &mut SpawnBudget,
    wave_number: u32,
    spawned: u16,
    rng: &mut RunRng,
) {
    if budget.elite || wave_number < 4 {
        return;
    }
    let champion = rush_champion(wave_number, budget.wave, spawned);
    let chance = (0.025 + 0.003 * wave_number.saturating_sub(4) as f32).min(0.09);
    if champion || rng.unit() < chance {
        budget.elite = true;
        budget.affix = Some(elite_affix_from_roll(rng.next_u32()));
    }
}

fn next_spawn_budget(
    wave: WaveDef,
    elapsed: f32,
    director: &mut SpawnDirector,
    encounters: &mut EncounterState,
    rng: &mut RunRng,
) -> SpawnBudget {
    let elite = elapsed >= director.next_elite_at;
    if elite {
        director.next_elite_at += 120.0;
    }
    let hunted = elite && encounters.take_hunted_elite();
    let archetype = choose_archetype(wave, rng.next_u32());
    let affix = elite.then(|| elite_affix_from_roll(rng.next_u32()));
    let (spawn_min_x, spawn_max_x) = spawn_x_range(archetype);
    SpawnBudget {
        wave,
        archetype,
        elite,
        hunted,
        affix,
        spawn_x: rng.range(spawn_min_x, spawn_max_x),
        spawn_z: rng.range(-4.25, 4.25),
        scale_jitter: rng.range(0.92, 1.08),
        speed_jitter: rng.range(0.94, 1.08),
    }
}

fn spawn_enemy(
    commands: &mut Commands,
    pools: &mut CombatPools,
    assets: &GameAssets,
    budget: SpawnBudget,
) -> Option<Entity> {
    spawn_enemy_group(commands, pools, assets, std::slice::from_ref(&budget))
}

fn spawn_merged_enemy(
    commands: &mut Commands,
    pools: &mut CombatPools,
    assets: &GameAssets,
    budgets: &[SpawnBudget],
) -> Option<Entity> {
    (!budgets.is_empty())
        .then(|| spawn_enemy_group(commands, pools, assets, budgets))
        .flatten()
}

#[derive(Debug, Clone, Copy, Default)]
struct SpawnBudgetTotals {
    hp: f32,
    breach_damage: f32,
    xp: u32,
    wood: u32,
    gold: u32,
    kill_credit: u32,
}

fn spawn_budget_totals(budgets: &[SpawnBudget]) -> SpawnBudgetTotals {
    budgets
        .iter()
        .fold(SpawnBudgetTotals::default(), |mut total, budget| {
            let def = ENEMIES[budget.archetype.min(ENEMIES.len() - 1)];
            let elite_health = if budget.elite { 6.0 } else { 1.0 };
            let hunted_health = if budget.hunted {
                super::encounters::ELITE_HUNT_HEALTH_MULTIPLIER
            } else {
                1.0
            };
            let elite_damage = if budget.elite { 1.8 } else { 1.0 };
            let elite_reward = if budget.elite { 8 } else { 1 };
            total.hp += def.hp * budget.wave.hp_multiplier * elite_health * hunted_health;
            total.breach_damage +=
                def.breach_damage as f32 * budget.wave.damage_multiplier * elite_damage;
            total.xp = total
                .xp
                .saturating_add(u32::from(def.xp).saturating_mul(elite_reward));
            total.wood = total.wood.saturating_add(if budget.elite { 16 } else { 0 });
            if budget.hunted {
                total.wood = total
                    .wood
                    .saturating_add(super::encounters::ELITE_HUNT_WOOD);
            }
            total.gold = total
                .gold
                .saturating_add(u32::from(def.gold_bounty).saturating_mul(elite_reward));
            total.kill_credit = total
                .kill_credit
                .saturating_add(u32::from(def.kill_credit).saturating_mul(elite_reward));
            total
        })
}

fn spawn_enemy_group(
    commands: &mut Commands,
    pools: &mut CombatPools,
    assets: &GameAssets,
    budgets: &[SpawnBudget],
) -> Option<Entity> {
    let representative = budgets
        .iter()
        .find(|budget| budget.elite)
        .or_else(|| budgets.first())
        .copied()?;
    let archetype = representative.archetype.min(ENEMIES.len() - 1);
    let def = ENEMIES[archetype];
    let totals = spawn_budget_totals(budgets);
    let elite = representative.elite;
    let affix = representative.affix;
    let base_scale = enemy_base_scale(def.hp, elite, representative.scale_jitter);
    let merge_scale = if budgets.len() > 1 {
        1.0 + (budgets.len() as f32).log2().min(6.0) * 0.08
    } else {
        1.0
    };
    let scale = base_scale * merge_scale;
    let visual = visual_for_archetype(archetype);
    let display_name = match (budgets.len(), affix) {
        (1, Some(kind)) => format!("{} · {}", def.name, kind.label()),
        (1, None) => def.name.to_owned(),
        (count, Some(kind)) => format!("MERGED x{count} · {}", kind.label()),
        (count, None) => format!("MERGED x{count}"),
    };
    let entity = pools.rent_enemy_scene(
        commands,
        EnemySceneKey::Regular(visual as u8),
        assets.enemy_scenes[visual].clone(),
        Transform::from_xyz(representative.spawn_x, 0.0, representative.spawn_z)
            .with_scale(Vec3::splat(scale)),
    )?;
    commands.entity(entity).insert((
        Name::new(display_name),
        Enemy {
            hp: totals.hp,
            max_hp: totals.hp,
            speed: def.speed
                * if elite {
                    1.0
                } else {
                    representative.speed_jitter
                },
            damage: def.damage
                * representative.wave.damage_multiplier
                * if elite { 1.8 } else { 1.0 },
            xp: totals.xp,
            wood: totals.wood,
            gold: totals.gold,
            kill_credit: totals.kill_credit,
            elite,
        },
        EnemyArchetype(archetype as u8),
        EnemyArmor(def.armor),
        EnemyBreachDamage(totals.breach_damage),
    ));
    if let Some(kind) = affix {
        commands.entity(entity).insert(EliteAffix::new(kind));
        if kind == EliteAffixKind::Brood {
            commands.entity(entity).insert(EnemyArchetype(9));
        }
        if kind == EliteAffixKind::Bulwark {
            commands.entity(entity).insert(EnemyArmor(def.armor + 6.0));
        }
    }
    match def.behavior {
        EnemyBehavior::HuntHero => {
            commands.entity(entity).insert(EnemyHeroThreat::ranged());
        }
        EnemyBehavior::BurrowAmbush => {
            commands.entity(entity).insert(EnemyHeroThreat::burrowed());
        }
        _ => {}
    }
    Some(entity)
}

fn enemy_base_scale(base_hp: f32, elite: bool, scale_jitter: f32) -> f32 {
    if elite {
        1.25
    } else {
        (0.78 + base_hp / 520.0).clamp(0.78, 1.2) * scale_jitter
    }
}

const fn elite_affix_from_roll(roll: u32) -> EliteAffixKind {
    let bucket = ((roll as u64 * 8) >> 32) as u8;
    match bucket {
        0 => EliteAffixKind::Frenzy,
        1 => EliteAffixKind::Bulwark,
        2 => EliteAffixKind::Volatile,
        3 => EliteAffixKind::Regeneration,
        4 => EliteAffixKind::Commander,
        5 => EliteAffixKind::YellowSign,
        6 => EliteAffixKind::Brood,
        _ => EliteAffixKind::Siege,
    }
}

fn wave_spawn_interval(wave: WaveDef) -> f32 {
    60.0 / f32::from(wave.count.max(1))
}

const fn visual_for_archetype(archetype: usize) -> usize {
    if archetype < ARCHETYPE_VISUALS.len() {
        ARCHETYPE_VISUALS[archetype]
    } else {
        0
    }
}

const fn spawn_x_range(archetype: usize) -> (f32, f32) {
    if archetype == 7 {
        (-9.0, -6.8)
    } else {
        (15.3, 16.4)
    }
}

fn spawn_boss(
    commands: &mut Commands,
    pools: &mut CombatPools,
    assets: &GameAssets,
    index: usize,
) -> Option<Entity> {
    let boss = &BOSSES[index];
    let entity = pools.rent_enemy_scene(
        commands,
        EnemySceneKey::Boss(index as u8),
        assets.boss_scenes[index].clone(),
        Transform::from_xyz(15.2, 0.0, 0.0).with_scale(Vec3::splat(1.15)),
    )?;
    commands.entity(entity).insert((
        Enemy {
            hp: boss.hp,
            max_hp: boss.hp,
            speed: boss.speed,
            damage: boss.damage_per_second,
            xp: 240 + index as u32 * 160,
            wood: 30 + index as u32 * 18,
            gold: 250 + index as u32 * 180,
            kill_credit: 120 + index as u32 * 80,
            elite: true,
        },
        BossEnemy { index: index as u8 },
        EnemyArmor(0.0),
        EnemyBreachDamage(boss.damage_per_second),
    ));
    Some(entity)
}

/// Boss lunges and teleports cannot tunnel through a standing road barrier.
pub(crate) fn stop_at_live_gate(
    session: &GameSession,
    gate: Option<Vec3>,
    origin: Vec3,
    mut destination: Vec3,
) -> Vec3 {
    if session.core_hp > 0.0
        && session.gate_revive_remaining <= 0.0
        && let Some(gate) = gate
        && origin.x >= gate.x
        && origin.z.abs() <= 5.65
    {
        destination.x = destination.x.max(gate.x + 1.9);
    }
    destination
}

pub(crate) fn move_towards_radius(
    position: Vec3,
    target: Vec3,
    speed: f32,
    delta: f32,
    radius: f32,
) -> (Vec3, f32, bool) {
    let offset = Vec3::new(target.x - position.x, 0.0, target.z - position.z);
    let distance = offset.length();
    let radius = radius.max(0.0);
    let delta = delta.max(0.0);
    if distance <= radius {
        return (position, delta, true);
    }
    let speed = speed.max(0.0);
    if speed <= f32::EPSILON || delta <= 0.0 {
        return (position, 0.0, false);
    }
    let distance_to_radius = distance - radius;
    let max_travel = speed * delta;
    if max_travel + f32::EPSILON >= distance_to_radius {
        let travel_seconds = (distance_to_radius / speed).min(delta);
        (
            position + offset / distance * distance_to_radius,
            (delta - travel_seconds).max(0.0),
            true,
        )
    } else {
        (position + offset / distance * max_travel, 0.0, false)
    }
}

fn move_enemies(
    pressure_and_fusion: (
        Option<Res<EnemyPressureSettings>>,
        Option<Res<super::hero_fusion::HeroFusion>>,
    ),
    mut commands: Commands,
    mut attack_cues: MessageWriter<EnemyAttackCue>,
    mut cue_clocks: Local<HashMap<Entity, f32>>,
    mut pools: ResMut<CombatPools>,
    time: Res<Time>,
    mut session: ResMut<GameSession>,
    mut vitals: ResMut<HeroVitals>,
    guards: HeroDamageGuards,
    mut rng: ResMut<RunRng>,
    mut dodge: ResMut<DodgeState>,
    mut intent: ResMut<MoveIntent>,
    adventure_bosses: Query<(), With<super::encounter_bosses::EncounterBoss>>,
    core_query: Query<&Transform, (With<Core>, Without<Enemy>)>,
    mut hero_query: Query<
        (&mut Transform, &mut Visibility),
        (With<Hero>, Without<Enemy>, Without<Core>),
    >,
    mut enemies: ParamSet<(
        Query<
            (Entity, &Transform, &EnemyArchetype, Option<&EliteAffix>),
            (With<Enemy>, Without<TrialEnemy>, Without<ArchiveEcho>),
        >,
        Query<
            (
                Entity,
                &Enemy,
                Option<&BossEnemy>,
                Option<&EnemyArchetype>,
                Option<&EnemyBreachDamage>,
                Option<&mut EnemyHeroThreat>,
                Option<&EliteAffix>,
                Option<&CommanderAuraState>,
                Option<&TimedSpeedEffect>,
                Option<&super::encounter_bosses::CampCombatant>,
                &mut Transform,
            ),
            (Without<TrialEnemy>, Without<ArchiveEcho>),
        >,
    )>,
) {
    let (pressure_settings, fusion) = pressure_and_fusion;
    let HeroDamageGuards {
        mut market_effects,
        mut bond_runtime,
        encounters,
    } = guards;
    if session.finished {
        cue_clocks.clear();
        return;
    }
    if session.paused {
        return;
    }
    let delta = time.delta_secs().max(0.0);
    // Bounty encounter: every lane attacker hits harder while it runs.
    let bounty = encounters.enemy_damage_multiplier();
    cue_clocks.retain(|entity, remaining| {
        *remaining -= delta;
        enemies.p1().contains(*entity)
    });
    let Ok(core) = core_query.single() else {
        return;
    };
    let Ok((mut hero_transform, mut hero_visibility)) = hero_query.single_mut() else {
        return;
    };
    let hero_position = hero_transform.translation;
    let core_position = core.translation;
    let chanters: Vec<(Entity, Vec3)> = enemies
        .p0()
        .iter()
        .filter(|(_, _, archetype, affix)| {
            archetype.0 == 5 && !affix.is_some_and(|affix| affix.is_volatile_pending())
        })
        .map(|(entity, transform, _, _)| (entity, transform.translation))
        .collect();
    for (
        entity,
        enemy,
        boss,
        archetype,
        breach_damage,
        mut threat,
        affix,
        commander_aura,
        speed_effect,
        camp,
        mut transform,
    ) in &mut enemies.p1()
    {
        if affix.is_some_and(|affix| affix.is_volatile_pending()) {
            continue;
        }
        if camp.is_some_and(|camp| hero_position.xz().distance(camp.center.xz()) > 6.0) {
            continue;
        }
        let is_boss = boss.is_some() || adventure_bosses.contains(entity);
        let pressure = if !is_boss && camp.is_none() {
            pressure_settings
                .as_deref()
                .map_or(EnemyPressure::BASELINE, |settings| {
                    settings.at_wave(session.wave)
                })
        } else {
            EnemyPressure::BASELINE
        };
        let action_speed =
            elite_action_speed_multiplier(affix.map(|affix| affix.kind)) * pressure.attack;
        let commander_multiplier = commander_aura_multiplier(commander_aura);
        let movement_speed = enemy.speed
            * elite_action_speed_multiplier(affix.map(|affix| affix.kind))
            * pressure.movement
            * commander_multiplier
            * speed_effect.map_or(1.0, |effect| effect.multiplier)
            * market_effects.enemy_movement_multiplier(is_boss);
        let attack_damage = enemy.damage * commander_multiplier * bounty;
        let siege = affix.is_some_and(|affix| affix.kind == EliteAffixKind::Siege);
        let behavior = if siege {
            Some(EnemyBehavior::SiegeGate)
        } else {
            archetype.map(|kind| ENEMIES[usize::from(kind.0).min(ENEMIES.len() - 1)].behavior)
        };
        if (camp.is_some() && hero_position.x >= core_position.x)
            || transform.translation.x < core_position.x
            || session.core_hp <= 0.0
            || session.gate_revive_remaining > 0.0
        {
            // Rushers carry their damage in the breach profile; ordinary attackers
            // keep their existing native damage when switching to the exposed hero.
            let hero_damage = if attack_damage > 0.0 {
                attack_damage
            } else {
                breach_damage.map_or(0.0, |breach| breach.0 * commander_multiplier * bounty)
            };
            if vitals.can_be_targeted() {
                let (next, attack_seconds, reached) = move_towards_radius(
                    transform.translation,
                    hero_position,
                    movement_speed,
                    delta,
                    if is_boss { 2.0 } else { 1.35 },
                );
                transform.translation = next;
                if reached && attack_seconds > 0.0 && hero_damage > 0.0 {
                    if continuous_enemy_cue_due(&mut cue_clocks, entity) {
                        attack_cues.write(EnemyAttackCue {
                            source: entity,
                            origin: next,
                            target: hero_position,
                            kind: if is_boss {
                                EnemyAttackKind::Slam
                            } else {
                                EnemyAttackKind::Slash
                            },
                        });
                    }
                    if !bond_runtime.evade(rng.unit()) {
                        bond_runtime.take_physical_damage(
                            &mut vitals,
                            hero_damage * action_speed * attack_seconds,
                        );
                    }
                    if vitals.health <= 0.0 {
                        begin_hero_death(
                            &mut session,
                            &mut vitals,
                            &mut market_effects,
                            &mut bond_runtime,
                            &mut dodge,
                            &mut intent,
                            &mut hero_transform,
                            &mut hero_visibility,
                        );
                    }
                }
            }
            continue;
        }

        // A rebuilt wall blocks new arrivals; enemies already through its
        // opening keep pursuing the hero instead of being teleported outside.
        // A live wall spans the road: keep each attacker's lane instead of
        // funneling it into the center, and protect heroes on its rear side.
        let target = Vec3::new(core.translation.x, 0.0, transform.translation.z);
        let gate_protects_hero =
            hero_position.x < core.translation.x && transform.translation.x >= core.translation.x;
        if vitals.can_be_targeted()
            && !gate_protects_hero
            && matches!(
                behavior,
                Some(EnemyBehavior::HuntHero | EnemyBehavior::BurrowAmbush)
            )
            && let Some(threat) = threat.as_deref_mut()
        {
            let hastened = chanters.iter().any(|(chanter, position)| {
                *chanter != entity && position.distance_squared(transform.translation) <= 12.25
            });
            let speed_multiplier = chanter_speed_multiplier(hastened);
            match behavior {
                Some(EnemyBehavior::HuntHero) => {
                    let (next, attack_seconds, reached) = move_towards_radius(
                        transform.translation,
                        hero_position,
                        movement_speed * speed_multiplier,
                        delta,
                        ACID_ARCHER_RANGE,
                    );
                    transform.translation = next;
                    if reached {
                        threat.attack_remaining -= attack_seconds * action_speed;
                        while threat.attack_remaining <= 0.0 && vitals.alive {
                            threat.attack_remaining += ACID_ARCHER_ATTACK_INTERVAL;
                            attack_cues.write(EnemyAttackCue {
                                source: entity,
                                origin: transform.translation,
                                target: hero_position,
                                kind: EnemyAttackKind::Shot,
                            });
                            let guard = hero_guard_damage_multiplier(
                                &vitals,
                                hero_position,
                                core_position,
                                fusion.as_deref(),
                            );
                            if !bond_runtime.evade(rng.unit()) {
                                bond_runtime
                                    .take_physical_damage(&mut vitals, attack_damage * guard);
                            }
                            if vitals.health <= 0.0 {
                                begin_hero_death(
                                    &mut session,
                                    &mut vitals,
                                    &mut market_effects,
                                    &mut bond_runtime,
                                    &mut dodge,
                                    &mut intent,
                                    &mut hero_transform,
                                    &mut hero_visibility,
                                );
                            }
                        }
                    }
                }
                Some(EnemyBehavior::BurrowAmbush) if !threat.emerged => {
                    transform.translation.y = -0.32;
                    let (next, _, reached) = move_towards_radius(
                        transform.translation,
                        hero_position,
                        movement_speed * 1.55 * speed_multiplier,
                        delta,
                        BURROW_REVEAL_RANGE,
                    );
                    transform.translation = next;
                    if reached {
                        threat.emerged = true;
                        transform.translation.y = 0.0;
                        if !threat.stun_applied {
                            threat.stun_applied = true;
                            attack_cues.write(EnemyAttackCue {
                                source: entity,
                                origin: transform.translation,
                                target: hero_position,
                                kind: EnemyAttackKind::Slam,
                            });
                            market_effects.try_apply_hero_stun(&mut vitals, BURROW_STUN_SECONDS);
                            let guard = hero_guard_damage_multiplier(
                                &vitals,
                                hero_position,
                                core_position,
                                fusion.as_deref(),
                            );
                            if !bond_runtime.evade(rng.unit()) {
                                bond_runtime
                                    .take_physical_damage(&mut vitals, attack_damage * guard);
                            }
                            if vitals.health <= 0.0 {
                                begin_hero_death(
                                    &mut session,
                                    &mut vitals,
                                    &mut market_effects,
                                    &mut bond_runtime,
                                    &mut dodge,
                                    &mut intent,
                                    &mut hero_transform,
                                    &mut hero_visibility,
                                );
                            }
                        }
                        threat.attack_remaining = BURROW_ATTACK_INTERVAL;
                    }
                }
                Some(EnemyBehavior::BurrowAmbush) => {
                    transform.translation.y = 0.0;
                    let (next, attack_seconds, reached) = move_towards_radius(
                        transform.translation,
                        hero_position,
                        movement_speed * speed_multiplier,
                        delta,
                        BURROW_ATTACK_RANGE,
                    );
                    transform.translation = next;
                    if reached {
                        threat.attack_remaining -= attack_seconds * action_speed;
                        while threat.attack_remaining <= 0.0 && vitals.alive {
                            threat.attack_remaining += BURROW_ATTACK_INTERVAL;
                            attack_cues.write(EnemyAttackCue {
                                source: entity,
                                origin: transform.translation,
                                target: hero_position,
                                kind: EnemyAttackKind::Slash,
                            });
                            let guard = hero_guard_damage_multiplier(
                                &vitals,
                                hero_position,
                                core_position,
                                fusion.as_deref(),
                            );
                            if !bond_runtime.evade(rng.unit()) {
                                bond_runtime
                                    .take_physical_damage(&mut vitals, attack_damage * guard);
                            }
                            if vitals.health <= 0.0 {
                                begin_hero_death(
                                    &mut session,
                                    &mut vitals,
                                    &mut market_effects,
                                    &mut bond_runtime,
                                    &mut dodge,
                                    &mut intent,
                                    &mut hero_transform,
                                    &mut hero_visibility,
                                );
                            }
                        }
                    }
                }
                _ => {}
            }
            continue;
        }

        if matches!(behavior, Some(EnemyBehavior::BurrowAmbush)) {
            transform.translation.y = 0.0;
        }
        let hastened = chanters.iter().any(|(chanter, position)| {
            *chanter != entity && position.distance_squared(transform.translation) <= 12.25
        });
        let speed_multiplier = chanter_speed_multiplier(hastened);
        if matches!(behavior, Some(EnemyBehavior::SiegeGate)) {
            let siege_damage = if attack_damage > 0.0 {
                attack_damage
            } else {
                breach_damage.map_or(0.0, |breach| breach.0 * commander_multiplier * bounty)
            };
            let (next, attack_seconds, reached) = move_towards_radius(
                transform.translation,
                target,
                movement_speed * speed_multiplier,
                delta,
                8.5,
            );
            transform.translation = next;
            if reached {
                if attack_seconds > 0.0
                    && siege_damage > 0.0
                    && continuous_enemy_cue_due(&mut cue_clocks, entity)
                {
                    attack_cues.write(EnemyAttackCue {
                        source: entity,
                        origin: transform.translation,
                        target: core_position,
                        kind: EnemyAttackKind::Shot,
                    });
                }
                session.core_hp = (session.core_hp
                    - siege_damage
                        * action_speed
                        * if siege { 0.7 } else { 0.35 }
                        * attack_seconds)
                    .max(0.0);
            }
            continue;
        }

        let breach_radius = if enemy.elite { 1.9 } else { 1.25 };
        let (next, attack_seconds, reached) = move_towards_radius(
            transform.translation,
            target,
            movement_speed * speed_multiplier,
            delta,
            breach_radius,
        );
        transform.translation = next;
        if reached {
            if is_boss {
                if attack_seconds > 0.0
                    && attack_damage > 0.0
                    && continuous_enemy_cue_due(&mut cue_clocks, entity)
                {
                    attack_cues.write(EnemyAttackCue {
                        source: entity,
                        origin: transform.translation,
                        target: core_position,
                        kind: EnemyAttackKind::Slam,
                    });
                }
                session.core_hp =
                    (session.core_hp - attack_damage * action_speed * attack_seconds).max(0.0);
            } else if attack_seconds > 0.0 {
                if continuous_enemy_cue_due(&mut cue_clocks, entity) {
                    attack_cues.write(EnemyAttackCue {
                        source: entity,
                        origin: transform.translation,
                        target: core_position,
                        kind: EnemyAttackKind::Slash,
                    });
                }
                if market_effects.try_intercept_ordinary_breach() {
                    pools.retire_enemy(&mut commands, entity);
                } else {
                    session.core_hp = (session.core_hp
                        - breach_damage.map_or(attack_damage, |breach| {
                            breach.0 * commander_multiplier * bounty
                        }) * action_speed
                            * attack_seconds)
                        .max(0.0);
                }
            }
        }
    }
}

/// Continuous DPS/healing stays unchanged; presentation pulses have a bounded cadence.
fn continuous_enemy_cue_due(clocks: &mut HashMap<Entity, f32>, source: Entity) -> bool {
    if clocks
        .get(&source)
        .is_some_and(|remaining| *remaining > 0.0)
    {
        return false;
    }
    if clocks.len() >= 256 && !clocks.contains_key(&source) {
        return false;
    }
    clocks.insert(source, 0.8);
    true
}

fn hero_guard_damage_multiplier(
    vitals: &HeroVitals,
    hero_position: Vec3,
    core_position: Vec3,
    fusion: Option<&super::hero_fusion::HeroFusion>,
) -> f32 {
    if (vitals.kind == HeroKind::StoneWarden
        || fusion.is_some_and(|fusion| fusion.has(HeroKind::StoneWarden)))
        && hero_position.distance(core_position) <= 8.0
    {
        0.85
    } else {
        1.0
    }
}

const fn chanter_speed_multiplier(hastened: bool) -> f32 {
    if hastened { 1.2 } else { 1.0 }
}

const fn elite_action_speed_multiplier(affix: Option<EliteAffixKind>) -> f32 {
    if matches!(affix, Some(EliteAffixKind::Frenzy)) {
        ELITE_ACTION_SPEED_MULTIPLIER
    } else {
        1.0
    }
}

fn commander_aura_multiplier(state: Option<&CommanderAuraState>) -> f32 {
    if state.is_some_and(|state| state.active) {
        COMMANDER_AURA_MULTIPLIER
    } else {
        1.0
    }
}

fn enemy_auras(
    time: Res<Time>,
    mut attack_cues: MessageWriter<EnemyAttackCue>,
    mut cue_clocks: Local<HashMap<Entity, f32>>,
    session: Res<GameSession>,
    mut enemies: ParamSet<(
        Query<(Entity, &Transform, &EnemyArchetype), With<Enemy>>,
        Query<(&mut Enemy, &Transform, Option<&EnemyArchetype>)>,
    )>,
) {
    if session.finished {
        cue_clocks.clear();
        return;
    }
    if session.paused {
        return;
    }
    cue_clocks.retain(|entity, remaining| {
        *remaining -= time.delta_secs();
        enemies.p0().contains(*entity)
    });
    let healers: Vec<(Entity, Vec3)> = enemies
        .p0()
        .iter()
        .filter(|(_, _, kind)| kind.0 == 8)
        .map(|(entity, transform, _)| (entity, transform.translation))
        .collect();
    if healers.is_empty() {
        return;
    }
    for (mut enemy, transform, kind) in &mut enemies.p1() {
        if kind.is_some_and(|kind| kind.0 == 8) {
            continue;
        }
        if let Some((source, origin)) = healers
            .iter()
            .find(|(_, position)| position.distance_squared(transform.translation) <= 12.25)
        {
            let before = enemy.hp;
            enemy.hp = (enemy.hp + enemy.max_hp * 0.02 * time.delta_secs()).min(enemy.max_hp);
            if enemy.hp > before && continuous_enemy_cue_due(&mut cue_clocks, *source) {
                attack_cues.write(EnemyAttackCue {
                    source: *source,
                    origin: *origin,
                    target: transform.translation,
                    kind: EnemyAttackKind::Heal,
                });
            }
        }
    }
}

fn commander_aura(
    mut commands: Commands,
    session: Res<GameSession>,
    mut enemies: ParamSet<(
        Query<
            (Entity, &Enemy, &Transform, &EliteAffix),
            (
                Without<BossEnemy>,
                Without<TrialEnemy>,
                Without<ArchiveEcho>,
            ),
        >,
        Query<
            (
                Entity,
                &mut Enemy,
                &Transform,
                Option<&mut CommanderAuraState>,
            ),
            (
                Without<BossEnemy>,
                Without<TrialEnemy>,
                Without<ArchiveEcho>,
            ),
        >,
    )>,
) {
    if session.paused || session.finished {
        return;
    }
    let commanders: Vec<(Entity, Vec3)> = enemies
        .p0()
        .iter()
        .filter(|(_, enemy, _, affix)| {
            enemy.hp > 0.0
                && affix.kind == EliteAffixKind::Commander
                && !affix.is_volatile_pending()
        })
        .map(|(entity, _, transform, _)| (entity, transform.translation))
        .collect();

    for (entity, mut enemy, transform, state) in &mut enemies.p1() {
        let active = commanders.iter().any(|(commander, position)| {
            *commander != entity && within_commander_aura(*position, transform.translation)
        });
        match state {
            Some(mut state) if state.active != active => {
                let health_fraction = (enemy.hp / enemy.max_hp.max(f32::EPSILON)).clamp(0.0, 1.0);
                enemy.max_hp = state.base_max_hp
                    * if active {
                        COMMANDER_AURA_MULTIPLIER
                    } else {
                        1.0
                    };
                enemy.hp = enemy.max_hp * health_fraction;
                state.active = active;
            }
            Some(_) => {}
            None => {
                let base_max_hp = enemy.max_hp;
                if active {
                    enemy.max_hp *= COMMANDER_AURA_MULTIPLIER;
                    enemy.hp *= COMMANDER_AURA_MULTIPLIER;
                }
                commands.entity(entity).insert(CommanderAuraState {
                    base_max_hp,
                    active,
                });
            }
        }
    }
}

fn within_commander_aura(commander: Vec3, target: Vec3) -> bool {
    planar_distance_squared(commander, target) <= COMMANDER_AURA_RADIUS * COMMANDER_AURA_RADIUS
}

/// Resources that scale damage between enemies and the hero.
#[derive(SystemParam)]
struct HeroDamageGuards<'w> {
    market_effects: ResMut<'w, MarketEffects>,
    bond_runtime: ResMut<'w, BondRuntime>,
    encounters: ResMut<'w, EncounterState>,
}

#[allow(clippy::too_many_arguments)]
fn tick_elite_affixes(
    mut commands: Commands,
    mut attack_cues: MessageWriter<EnemyAttackCue>,
    time: Res<Time>,
    mut session: ResMut<GameSession>,
    mut archive: ResMut<ArchiveState>,
    mut trial: ResMut<TrialState>,
    mut gold_trial: ResMut<GoldChallengeState>,
    mut build: ResMut<PlayerBuild>,
    mut vitals: ResMut<HeroVitals>,
    guards: HeroDamageGuards,
    mut dodge: ResMut<DodgeState>,
    mut intent: ResMut<MoveIntent>,
    assets: Option<Res<GameAssets>>,
    mut pools: ResMut<CombatPools>,
    mut hero_query: Query<(&mut Transform, &mut Visibility), (With<Hero>, Without<Enemy>)>,
    mut enemies: Query<
        (
            Entity,
            &mut Enemy,
            &Transform,
            &mut EliteAffix,
            Option<&TrialEnemy>,
            Option<&BossEnemy>,
            Option<&super::encounter_bosses::EncounterBoss>,
            Option<&EnemyArchetype>,
            Option<&ArchiveEcho>,
            Option<&CommanderAuraState>,
        ),
        (With<Enemy>, Without<Hero>, Without<Projectile>),
    >,
) {
    let HeroDamageGuards {
        mut market_effects,
        mut bond_runtime,
        mut encounters,
    } = guards;
    if session.paused || session.finished {
        return;
    }
    let Some(assets) = assets else {
        return;
    };
    let delta = time.delta_secs();
    for (
        entity,
        mut enemy,
        transform,
        mut affix,
        trial_enemy,
        boss_enemy,
        adventure_boss,
        archetype,
        archive_echo,
        commander_state,
    ) in &mut enemies
    {
        if affix.is_volatile_pending() {
            affix.volatile_remaining = (affix.volatile_remaining - delta).max(0.0);
            if affix.volatile_remaining > 0.0 {
                continue;
            }
            let position = transform.translation;
            attack_cues.write(EnemyAttackCue {
                source: entity,
                origin: position,
                target: position,
                kind: EnemyAttackKind::Burst,
            });
            spawn_affix_trigger_fx(
                &mut commands,
                &mut pools,
                &assets,
                position,
                EliteAffixKind::Volatile,
                false,
            );
            if let Ok((mut hero_transform, mut hero_visibility)) = hero_query.single_mut()
                && vitals.can_be_targeted()
                && planar_distance_squared(hero_transform.translation, position)
                    <= VOLATILE_BLAST_RADIUS * VOLATILE_BLAST_RADIUS
            {
                let damage = enemy.damage
                    * commander_aura_multiplier(commander_state)
                    * encounters.enemy_damage_multiplier()
                    * VOLATILE_DAMAGE_MULTIPLIER;
                bond_runtime.take_physical_damage(&mut vitals, damage);
                if vitals.health <= 0.0 {
                    begin_hero_death(
                        &mut session,
                        &mut vitals,
                        &mut market_effects,
                        &mut bond_runtime,
                        &mut dodge,
                        &mut intent,
                        &mut hero_transform,
                        &mut hero_visibility,
                    );
                }
            }
            enemy.hp = 0.0;
            settle_enemy_defeat(
                &mut commands,
                &mut pools,
                &assets,
                &mut session,
                &mut archive,
                &mut trial,
                &mut gold_trial,
                &mut build,
                &mut vitals,
                &market_effects,
                &mut encounters,
                false,
                entity,
                &enemy,
                position,
                trial_enemy,
                boss_enemy,
                adventure_boss,
                archetype,
                archive_echo,
            );
            continue;
        }

        if refresh_bulwark_shield(&enemy, &mut affix, delta) {
            spawn_affix_trigger_fx(
                &mut commands,
                &mut pools,
                &assets,
                transform.translation,
                EliteAffixKind::Bulwark,
                false,
            );
        }
    }
}

fn refresh_bulwark_shield(enemy: &Enemy, affix: &mut EliteAffix, delta: f32) -> bool {
    if affix.kind != EliteAffixKind::Bulwark {
        return false;
    }
    affix.shield_remaining -= delta.max(0.0);
    if affix.shield_remaining > 0.0 {
        return false;
    }
    while affix.shield_remaining <= 0.0 {
        affix.shield_remaining += BULWARK_SHIELD_INTERVAL;
    }
    affix.shield = enemy.max_hp * BULWARK_SHIELD_FRACTION;
    true
}

fn planar_distance_squared(left: Vec3, right: Vec3) -> f32 {
    Vec2::new(left.x - right.x, left.z - right.z).length_squared()
}

#[derive(SystemParam)]
struct AttackState<'w> {
    fusion: Option<ResMut<'w, super::hero_fusion::HeroFusion>>,
    rng: ResMut<'w, RunRng>,
    cues: Option<ResMut<'w, WeaponCues>>,
    clock: ResMut<'w, AttackClock>,
    volley_runtime: ResMut<'w, AttackVolleyRuntime>,
    bonds: ResMut<'w, BondRuntime>,
}
fn auto_attack(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    vitals: Res<HeroVitals>,
    build: Res<PlayerBuild>,
    attack_speed_modifier: Option<Res<HeroAttackSpeedModifier>>,
    mut basic_attacks: Option<ResMut<BasicAttackCounter>>,
    mut market_effects: ResMut<MarketEffects>,
    assets: Option<Res<GameAssets>>,
    attack_art: Option<Res<BasicAttackArt>>,
    mut attack_aim: Option<ResMut<BasicAttackAim>>,
    mut pools: ResMut<CombatPools>,
    state: AttackState,
    hero_query: Query<(Entity, &Transform), With<Hero>>,
    enemies: Query<
        (
            Entity,
            &Enemy,
            &Transform,
            Option<&BossEnemy>,
            Option<&super::encounter_bosses::EncounterBoss>,
            Option<&ArchiveEcho>,
            Option<&TrialEnemy>,
            Option<&EnemyArchetype>,
            Option<&EliteAffix>,
        ),
        With<Enemy>,
    >,
) {
    let AttackState {
        mut fusion,
        mut rng,
        mut cues,
        mut clock,
        mut volley_runtime,
        mut bonds,
    } = state;
    volley_runtime.resolved_this_frame = 0;
    if let Some(aim) = attack_aim.as_deref_mut() {
        aim.0 = None;
    }
    if session.finished || !vitals.can_act() {
        volley_runtime.pending = 0;
        return;
    }
    if session.paused {
        return;
    }
    let Some(assets) = assets else {
        volley_runtime.pending = 0;
        return;
    };
    let Ok((hero_entity, hero)) = hero_query.single() else {
        volley_runtime.pending = 0;
        return;
    };
    let attacks_per_second = effective_basic_attack_rate(
        build.attacks_per_second,
        attack_speed_modifier
            .as_deref()
            .map_or(1.0, |modifier| modifier.0),
    );
    clock
        .0
        .set_duration(std::time::Duration::from_secs_f32(1.0 / attacks_per_second));
    clock.0.tick(time.delta());
    let raw_volley_count = clock.0.times_finished_this_tick();
    let previous_pending = volley_runtime.pending;
    let available = previous_pending.saturating_add(u64::from(raw_volley_count));
    let planned_volley_count = available.min(u64::from(MAX_ATTACK_CATCH_UP_VOLLEYS)) as u32;
    if planned_volley_count == 0 {
        return;
    }

    let mut target: Option<(u8, f32, Entity, Vec3)> = None;
    let mut target_positions = Vec::new();
    let weapon = basic_attacks::weapon(vitals.kind);
    let class_stats = basic_attacks::class_basic_stats(vitals.kind, vitals.level, &build);
    let attack_range = class_stats.range;
    let range_squared = attack_range * attack_range;
    for (entity, enemy, enemy_transform, boss, adventure, archive, trial, archetype, affix) in
        &enemies
    {
        if enemy.hp <= 0.0 || affix.is_some_and(|affix| affix.is_volatile_pending()) {
            continue;
        }
        target_positions.push((entity, enemy_transform.translation));
        let dist_sq = hero
            .translation
            .distance_squared(enemy_transform.translation);
        if dist_sq > range_squared {
            continue;
        }
        let tier = target_priority(
            archive.is_some(),
            boss.is_some() || adventure.is_some(),
            trial.is_some(),
            archetype.map(|archetype| archetype.0),
        );
        let within_tier_score = if tier <= 1 {
            dist_sq
        } else {
            enemy_transform.translation.x
        };
        let candidate = (tier, within_tier_score, entity, enemy_transform.translation);
        if target.is_none_or(|current| {
            candidate.0 < current.0
                || (candidate.0 == current.0
                    && (candidate.1 < current.1
                        || (candidate.1 == current.1
                            && candidate.2.to_bits() < current.2.to_bits())))
        }) {
            target = Some(candidate);
        }
    }
    let Some((_, _, target_entity, target)) = target else {
        volley_runtime.pending = 0;
        return;
    };

    let mut archer_targets = target_positions.clone();
    archer_targets
        .retain(|(_, position)| position.distance_squared(hero.translation) <= range_squared);
    archer_targets.sort_by(|a, b| {
        (a.0 != target_entity)
            .cmp(&(b.0 != target_entity))
            .then_with(|| {
                a.1.distance_squared(hero.translation)
                    .total_cmp(&b.1.distance_squared(hero.translation))
            })
            .then_with(|| a.0.to_bits().cmp(&b.0.to_bits()))
    });
    let backlog_volley_count = previous_pending.min(u64::from(planned_volley_count)) as u32;
    let new_volley_count = planned_volley_count - backlog_volley_count;
    volley_runtime.pending = available - u64::from(planned_volley_count);

    for volley_index in 0..planned_volley_count {
        // Roll once for the scheduled volley: bonus attacks use the same weapon
        // and on-attack effects, but can never roll another bonus attack.
        let chance = bonds.extra_attack_chance();
        let extra = chance > 0.0 && rng.unit() < chance;
        for attack_index in 0..=u32::from(extra) {
            if market_effects.record_basic_attack() {
                for (entity, position) in
                    storm_coil_targets(target_entity, target, &target_positions)
                {
                    if let Some(projectile_entity) = rent_market_projectile(
                        &mut commands,
                        &mut pools,
                        &assets,
                        position,
                        build.attack_damage * 0.60,
                        0.30,
                        0,
                        DamageProfile::new(DamageElement::Storm, DamageClass::Attack)
                            .as_market_proc(),
                    ) {
                        restrict_projectile_to_target(
                            &mut commands,
                            projectile_entity,
                            entity,
                            &target_positions,
                        );
                    }
                }
            }

            let base_direction = (target - hero.translation).normalize_or_zero();
            let shot_count = if weapon == basic_attacks::BasicWeapon::Crossbow {
                class_stats
                    .shots
                    .min(archer_targets.len().min(usize::from(u8::MAX)) as u8)
            } else {
                class_stats.shots
            };
            let mut fired = false;
            if weapon.melee() {
                commands.spawn(PendingSwing {
                    hero: hero_entity,
                    target: target_entity,
                    kind: vitals.kind,
                    direction: base_direction,
                    damage: build.attack_damage,
                    hit_limit: class_stats.cleave_targets + bonds.melee_cleave(),
                    remaining: basic_attacks::swing_windup(clock.0.duration().as_secs_f32()),
                });
                fired = true;
            }
            for i in 0..if weapon.melee() { 0 } else { shot_count } {
                let spread_index = i as f32 - (shot_count as f32 - 1.0) * 0.5;
                let spread = spread_index * 0.14;
                let direction = if weapon == basic_attacks::BasicWeapon::Crossbow {
                    archer_targets.get(usize::from(i)).map_or_else(
                        || Quat::from_rotation_y(spread) * base_direction,
                        |(_, position)| (*position - hero.translation).normalize_or(Vec3::X),
                    )
                } else {
                    Quat::from_rotation_y(spread) * base_direction
                };
                let (mesh, material) = attack_art
                    .as_deref()
                    .map(|art| art.projectile_at_level(vitals.kind, vitals.level))
                    .unwrap_or_else(|| {
                        (
                            assets.projectile_mesh.clone(),
                            assets.projectile_material.clone(),
                        )
                    });
                let Some(entity) = pools.rent_projectile(
                    &mut commands,
                    mesh,
                    material,
                    Transform::from_translation(hero.translation + Vec3::Y * 0.25)
                        .looking_to(direction, Vec3::Y)
                        .with_scale(Vec3::splat(
                            basic_attacks::basic_attack_visual_tier(vitals.level).projectile_scale,
                        )),
                    Projectile {
                        velocity: direction * build.projectile_speed,
                        damage: build.attack_damage,
                        radius: 0.48,
                        remaining_hits: build.pierce,
                        lifetime: 1.4,
                    },
                ) else {
                    break;
                };
                commands.entity(entity).insert((
                    DamageProfile::PHYSICAL_ATTACK,
                    WeaponFlight::new(vitals.kind),
                ));
                if class_stats.bounces > 0 {
                    commands
                        .entity(entity)
                        .insert(basic_attacks::Ricochet::new(class_stats.bounces));
                }
                let is_new_completion = volley_index >= backlog_volley_count;
                if is_new_completion && (raw_volley_count > 1 || backlog_volley_count > 0) {
                    let new_volley_index = volley_index - backlog_volley_count;
                    commands.entity(entity).insert(ProjectileFirstFrameTravel(
                        catch_up_volley_travel_seconds(
                            &clock,
                            raw_volley_count,
                            new_volley_index,
                            time.delta_secs(),
                        ),
                    ));
                }
                fired = true;
            }
            // Archery is a supporting volley. An empty quiver never stops the native weapon.
            if fired && bonds.archery_active() && bonds.arrows_remaining > 0 {
                let mut arrow_fired = false;
                for i in 0..build.multishot.max(1) {
                    let direction = Quat::from_rotation_y(
                        (i as f32 - (build.multishot.max(1) as f32 - 1.) * 0.5) * 0.14,
                    ) * base_direction;
                    let (mesh, material) = attack_art
                        .as_deref()
                        .map(BasicAttackArt::arrow)
                        .unwrap_or_else(|| {
                            (
                                assets.projectile_mesh.clone(),
                                assets.projectile_material.clone(),
                            )
                        });
                    if let Some(entity) = pools.rent_projectile(
                        &mut commands,
                        mesh,
                        material,
                        Transform::from_translation(hero.translation + Vec3::Y * 0.65)
                            .looking_to(direction, Vec3::Y),
                        Projectile {
                            velocity: direction * build.projectile_speed,
                            damage: build.attack_damage,
                            radius: 0.3,
                            remaining_hits: build.pierce,
                            lifetime: 1.4,
                        },
                    ) {
                        commands.entity(entity).insert((
                            DamageProfile::PHYSICAL_ATTACK.as_arrow(),
                            WeaponFlight::new(HeroKind::EmberRanger),
                        ));
                        if build.ricochet > 0 {
                            commands
                                .entity(entity)
                                .insert(basic_attacks::Ricochet::new(build.ricochet.min(4)));
                        }
                        arrow_fired = true;
                    }
                }
                if arrow_fired {
                    bonds.consume_arrow();
                }
            }
            if fired && let Some(attacks) = basic_attacks.as_deref_mut() {
                attacks.0 = attacks.0.wrapping_add(1);
            }
            if fired {
                super::basic_relics::on_attack(
                    &mut commands,
                    target,
                    build.attack_damage,
                    vitals.spell_power(),
                );
                if let Some(fusion) = fusion.as_deref_mut()
                    && let Some(kind) = fusion.on_attack(vitals.kind)
                {
                    commands.spawn(super::hero_fusion::FusionRequest {
                        hero: hero_entity,
                        kind,
                        target: target_entity,
                        damage: build.attack_damage,
                        speed: build.projectile_speed,
                    });
                }
                if !weapon.melee()
                    && let Some(cues) = cues.as_deref_mut()
                {
                    cues.push(vitals.kind, hero.translation, base_direction, false);
                }
                if let Some(aim) = attack_aim.as_deref_mut() {
                    aim.0 = Some(target);
                }
                volley_runtime.resolved_this_frame =
                    volley_runtime.resolved_this_frame.saturating_add(1);
            } else {
                if attack_index == 0 {
                    volley_runtime.pending = volley_runtime.pending.saturating_add(1);
                }
                break;
            }
        }
    }

    debug_assert!(new_volley_count <= raw_volley_count);
}

fn storm_coil_targets(
    primary: Entity,
    primary_position: Vec3,
    candidates: &[(Entity, Vec3)],
) -> Vec<(Entity, Vec3)> {
    let mut nearby: Vec<(Entity, Vec3, f32)> = candidates
        .iter()
        .filter_map(|(entity, position)| {
            let distance = planar_distance_squared(primary_position, *position);
            (*entity != primary && distance <= 5.5_f32.powi(2))
                .then_some((*entity, *position, distance))
        })
        .collect();
    nearby.sort_by(|left, right| {
        left.2
            .total_cmp(&right.2)
            .then_with(|| left.0.to_bits().cmp(&right.0.to_bits()))
    });
    let mut targets = Vec::with_capacity(3);
    targets.push((primary, primary_position));
    targets.extend(
        nearby
            .into_iter()
            .take(2)
            .map(|(entity, position, _)| (entity, position)),
    );
    targets
}

#[allow(clippy::too_many_arguments)]
fn rent_market_projectile(
    commands: &mut Commands,
    pools: &mut CombatPools,
    assets: &GameAssets,
    position: Vec3,
    damage: f32,
    radius: f32,
    remaining_hits: u8,
    profile: DamageProfile,
) -> Option<Entity> {
    let entity = pools.rent_projectile(
        commands,
        assets.projectile_mesh.clone(),
        assets.projectile_material.clone(),
        Transform::from_translation(position + Vec3::Y * 0.18),
        Projectile {
            velocity: Vec3::ZERO,
            damage,
            radius,
            remaining_hits,
            lifetime: 0.12,
        },
    )?;
    commands.entity(entity).insert(profile);
    Some(entity)
}

/// Feed an already validated weapon contact into the authoritative damage pipeline.
/// This carrier has no mesh, travel or area reach and can hit only its chosen enemy.
pub(crate) fn queue_melee_contact(
    commands: &mut Commands,
    pools: &mut CombatPools,
    assets: &GameAssets,
    target: Entity,
    position: Vec3,
    damage: f32,
    candidates: &[(Entity, Vec3)],
) -> Option<Entity> {
    if let Some(entity) = pools.rent_projectile(
        commands,
        assets.projectile_mesh.clone(),
        assets.projectile_material.clone(),
        Transform::from_translation(position),
        Projectile {
            velocity: Vec3::ZERO,
            damage,
            radius: 0.01,
            remaining_hits: 0,
            lifetime: 0.01,
        },
    ) {
        commands
            .entity(entity)
            .remove::<(Mesh3d, MeshMaterial3d<StandardMaterial>)>()
            .insert((
                Visibility::Hidden,
                DamageProfile::PHYSICAL_ATTACK.as_melee(),
            ));
        restrict_projectile_to_target(commands, entity, target, candidates);
        return Some(entity);
    }
    None
}

fn restrict_projectile_to_target(
    commands: &mut Commands,
    projectile: Entity,
    target: Entity,
    candidates: &[(Entity, Vec3)],
) {
    commands.entity(projectile).insert(ProjectileHitMemory {
        enemies: candidates
            .iter()
            .filter_map(|(entity, _)| (*entity != target).then_some(*entity))
            .collect(),
    });
}

const fn target_priority(archive: bool, boss: bool, trial: bool, archetype: Option<u8>) -> u8 {
    if archive {
        0
    } else if boss {
        1
    } else if trial {
        2
    } else if matches!(archetype, Some(4)) {
        3
    } else {
        4
    }
}

fn move_projectiles(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    mut pools: ResMut<CombatPools>,
    targets: Query<(&Enemy, &Transform, Option<&EliteAffix>), Without<Projectile>>,
    mut projectiles: Query<
        (
            Entity,
            &mut Projectile,
            &mut Transform,
            Option<&ProjectileFirstFrameTravel>,
            Option<&basic_attacks::Ricochet>,
        ),
        With<Projectile>,
    >,
) {
    if session.paused || session.finished {
        return;
    }
    let delta = time.delta_secs().max(0.0);
    for (entity, mut projectile, mut transform, first_frame, ricochet) in &mut projectiles {
        if let Some(target) = ricochet.and_then(|bounce| bounce.target) {
            let Ok((enemy, target, affix)) = targets.get(target) else {
                pools.retire_projectile(&mut commands, entity);
                continue;
            };
            if enemy.hp <= 0.0 || affix.is_some_and(|affix| affix.is_volatile_pending()) {
                pools.retire_projectile(&mut commands, entity);
                continue;
            }
            let direction = (target.translation - transform.translation).normalize_or(Vec3::X);
            projectile.velocity = direction * projectile.velocity.length();
            transform.look_to(direction, Vec3::Y);
        }
        let simulation_seconds = first_frame.map_or(delta, |travel| travel.0.clamp(0.0, delta));
        let travel_seconds = simulation_seconds.min(projectile.lifetime.max(0.0));
        transform.translation += projectile.velocity * travel_seconds;
        projectile.lifetime -= simulation_seconds;
    }
}

fn segment_sphere_hit_progress(start: Vec3, end: Vec3, center: Vec3, radius: f32) -> Option<f32> {
    let segment = end - start;
    let length_squared = segment.length_squared();
    if length_squared <= f32::EPSILON {
        return (end.distance_squared(center) <= radius.max(0.0).powi(2)).then_some(0.0);
    }
    let offset = start - center;
    let radius_squared = radius.max(0.0).powi(2);
    let offset_from_surface = offset.length_squared() - radius_squared;
    if offset_from_surface <= 0.0 {
        return Some(0.0);
    }
    let projected = offset.dot(segment);
    let discriminant = projected * projected - length_squared * offset_from_surface;
    if discriminant < 0.0 {
        return None;
    }
    let root = discriminant.sqrt();
    let entry = (-projected - root) / length_squared;
    let exit = (-projected + root) / length_squared;
    (exit >= 0.0 && entry <= 1.0).then_some(entry.clamp(0.0, 1.0))
}

fn apply_enemy_damage(
    enemy: &mut Enemy,
    mut affix: Option<&mut EliteAffix>,
    damage: f32,
    damage_meter: Option<&mut DamageMeter>,
) -> DamageResolution {
    if affix
        .as_deref()
        .is_some_and(|affix| affix.is_volatile_pending())
    {
        return DamageResolution::new(DamageOutcome::Pending, 0.0);
    }

    let mut remaining = damage.max(0.0);
    let mut applied = 0.0;
    if let Some(affix) = affix.as_deref_mut()
        && affix.shield > 0.0
    {
        let absorbed = remaining.min(affix.shield);
        affix.shield -= absorbed;
        remaining -= absorbed;
        applied += absorbed;
    }
    if remaining <= 0.0 {
        if let Some(meter) = damage_meter {
            meter.record(applied);
        }
        return DamageResolution::new(DamageOutcome::Alive, 0.0);
    }

    let hp_before = enemy.hp.max(0.0);
    let hp_damage = remaining.min(hp_before);
    applied += hp_damage;
    enemy.hp -= remaining;
    if let Some(meter) = damage_meter {
        meter.record(applied);
    }
    if enemy.hp > 0.0 {
        return DamageResolution::new(DamageOutcome::Alive, hp_damage);
    }
    let Some(affix) = affix else {
        return DamageResolution::new(DamageOutcome::Defeated, hp_damage);
    };
    let outcome = match affix.kind {
        EliteAffixKind::Regeneration if !affix.regenerated => {
            affix.regenerated = true;
            enemy.hp = enemy.max_hp * REGENERATION_HEALTH_FRACTION;
            DamageOutcome::Regenerated
        }
        EliteAffixKind::Volatile => {
            affix.volatile_remaining = VOLATILE_WARNING_SECONDS;
            // Keeping a positive sentinel prevents healing and targeting systems
            // from treating the warning entity as already settled.
            enemy.hp = f32::EPSILON;
            DamageOutcome::VolatileStarted
        }
        _ => DamageOutcome::Defeated,
    };
    DamageResolution::new(outcome, hp_damage)
}

#[allow(clippy::too_many_arguments)]
fn settle_enemy_defeat(
    commands: &mut Commands,
    pools: &mut CombatPools,
    assets: &GameAssets,
    session: &mut GameSession,
    archive: &mut ArchiveState,
    trial: &mut TrialState,
    gold_trial: &mut GoldChallengeState,
    build: &mut PlayerBuild,
    vitals: &mut HeroVitals,
    market_effects: &MarketEffects,
    encounters: &mut EncounterState,
    frost_mirror_kill: bool,
    enemy_entity: Entity,
    enemy: &Enemy,
    position: Vec3,
    trial_enemy: Option<&TrialEnemy>,
    boss_enemy: Option<&BossEnemy>,
    adventure_boss: Option<&super::encounter_bosses::EncounterBoss>,
    archetype: Option<&EnemyArchetype>,
    archive_echo: Option<&ArchiveEcho>,
) {
    pools.retire_enemy(commands, enemy_entity);
    if archive_echo.is_some() {
        pools.retire_all_projectiles(commands);
        pools.retire_all_effects(commands);
        pools.retire_all_damage_numbers(commands);
        archive.resolve(session, ArchiveOutcome::Captured);
    } else if let Some(challenge) = trial_enemy {
        match challenge.kind {
            ChallengeKind::Tower => {
                complete_tower_challenge(
                    challenge.level,
                    challenge.reward,
                    session,
                    trial,
                    build,
                    vitals,
                );
            }
            ChallengeKind::Gold => {
                complete_gold_target(challenge.level, challenge.reward, session, gold_trial);
            }
        }
    } else if let Some(adventure) = adventure_boss {
        let index = adventure.index;
        commands.queue(move |world: &mut World| {
            super::encounter_bosses::successful_defeat(world, enemy_entity, index);
        });
    } else if let Some(boss) = boss_enemy {
        // Forced March pays the marched boss one and a half times.
        let xp = encounters.boss_reward(boss.index, enemy.xp);
        session.xp = session
            .xp
            .saturating_add(xp_with_efficiency(xp, session.xp_multiplier));
        session.wood += encounters.boss_reward(boss.index, enemy.wood);
        session.gold += encounters.boss_reward(boss.index, enemy.gold);
        session.kills +=
            encounters.kill_credit(encounters.boss_reward(boss.index, enemy.kill_credit));
        session.total_kills += 1;
        session.bosses_defeated = session.bosses_defeated.max(boss.index + 1);
        encounters.bank_for_boss(boss.index);
        if boss.index == 4 {
            session.save_active = true;
            session.save_remaining = 60.0;
        }
    } else {
        session.xp = session
            .xp
            .saturating_add(xp_with_efficiency(enemy.xp, session.xp_multiplier));
        session.wood += enemy.wood;
        session.gold += enemy.gold;
        session.kills += encounters.kill_credit(enemy.kill_credit);
        session.total_kills += 1;
    }
    if trial_enemy.is_none() && boss_enemy.is_none() && archetype.is_some_and(|kind| kind.0 == 9) {
        enqueue_or_spawn_splitlings(commands, pools, assets, position, 3);
    }
    if frost_mirror_kill && market_effects.frost_mirror_enabled() {
        rent_market_projectile(
            commands,
            pools,
            assets,
            position,
            frost_mirror_damage(market_effects.effective_spell_power(vitals.spell_power())),
            3.0,
            u8::MAX,
            DamageProfile::new(DamageElement::Frost, DamageClass::Reaction).as_market_proc(),
        );
    }
}

fn frost_mirror_damage(effective_spell_power: f32) -> f32 {
    80.0 + 0.5 * effective_spell_power
}

fn xp_with_efficiency(base_xp: u32, multiplier: f32) -> u32 {
    (base_xp as f32 * multiplier.max(0.0)).round() as u32
}

fn affix_trigger_material(assets: &GameAssets, kind: EliteAffixKind) -> Handle<StandardMaterial> {
    match kind {
        EliteAffixKind::Frenzy => assets.storm_material.clone(),
        EliteAffixKind::Bulwark => assets.enemy_materials[2].clone(),
        EliteAffixKind::Volatile | EliteAffixKind::Siege => assets.enemy_materials[1].clone(),
        EliteAffixKind::Regeneration | EliteAffixKind::Brood => assets.enemy_materials[3].clone(),
        EliteAffixKind::Commander | EliteAffixKind::YellowSign => assets.elite_material.clone(),
    }
}

fn spawn_affix_trigger_fx(
    commands: &mut Commands,
    pools: &mut CombatPools,
    assets: &GameAssets,
    position: Vec3,
    kind: EliteAffixKind,
    warning: bool,
) {
    let (lifetime, start_scale, end_scale) = if warning {
        (VOLATILE_WARNING_SECONDS, 0.25, VOLATILE_BLAST_RADIUS)
    } else {
        match kind {
            EliteAffixKind::Volatile => (0.34, VOLATILE_BLAST_RADIUS, 0.2),
            EliteAffixKind::Regeneration => (0.45, 0.3, 2.2),
            EliteAffixKind::Bulwark => (0.42, 0.4, 1.8),
            _ => (0.3, 0.25, 1.2),
        }
    };
    let name = if warning {
        "VOLATILE · 1 SECOND WARNING"
    } else {
        kind.label()
    };
    pools.rent_named_impact_fx(
        commands,
        assets.fx_mesh.clone(),
        affix_trigger_material(assets, kind),
        Transform::from_xyz(position.x, 0.2, position.z).with_scale(Vec3::splat(start_scale)),
        ImpactFx {
            lifetime,
            start_scale,
            end_scale,
        },
        Some(Name::new(name)),
    );
}

fn projectile_hits(
    mut commands: Commands,
    frame: ProjectileHitFrame,
    mut damage_meter: ResMut<DamageMeter>,
    mut archive: ResMut<ArchiveState>,
    assets: Option<Res<GameAssets>>,
    mut trial: ResMut<TrialState>,
    mut gold_trial: ResMut<GoldChallengeState>,
    mut build: ResMut<PlayerBuild>,
    mut vitals: ResMut<HeroVitals>,
    mut rng: ResMut<RunRng>,
    mut bond_runtime: ResMut<BondRuntime>,
    skill_runtime: Res<SkillRuntime>,
    market_effects: Res<MarketEffects>,
    mut pools: ResMut<CombatPools>,
    mut projectiles: Query<
        (
            Entity,
            &mut Projectile,
            &mut Transform,
            Option<&DamageProfile>,
            Option<&mut ProjectileHitMemory>,
            Option<&ProjectileFirstFrameTravel>,
            Option<&WeaponFlight>,
            Option<&super::hero_fusion::FusionAttack>,
            Option<&mut basic_attacks::Ricochet>,
        ),
        (With<Projectile>, Without<Hero>),
    >,
    mut enemies: Query<
        (
            Entity,
            &mut Enemy,
            &Transform,
            Option<&TrialEnemy>,
            Option<&BossEnemy>,
            Option<&super::encounter_bosses::EncounterBoss>,
            Option<&EnemyArchetype>,
            Option<&ArchiveEcho>,
            Option<&EnemyArmor>,
            Option<&mut EliteAffix>,
            Option<&TimedSpeedEffect>,
        ),
        Without<Projectile>,
    >,
) {
    let delta = frame.time.delta_secs().max(0.0);
    let hero_position = frame
        .heroes
        .iter()
        .next()
        .map(|transform| transform.translation);
    let mut session = frame.session;
    let mut encounters = frame.encounters;
    let mut cues = frame.cues;
    let mut hit_feedback = frame.hit_feedback;
    if session.paused || session.finished {
        return;
    }
    let Some(assets) = assets else {
        return;
    };
    let collision_targets: Vec<(Entity, Vec3, bool)> = enemies
        .iter()
        .filter(|(_, enemy, _, _, _, _, _, _, _, _, _)| enemy.hp > 0.0)
        .map(|(entity, enemy, transform, _, _, _, _, _, _, _, _)| {
            (entity, transform.translation, enemy.elite)
        })
        .collect();
    let target_positions: Vec<(Entity, Vec3)> = collision_targets
        .iter()
        .map(|(entity, position, _)| (*entity, *position))
        .collect();
    for (
        projectile_entity,
        mut projectile,
        mut projectile_transform,
        profile,
        mut hit_memory,
        first_frame,
        flight,
        fusion,
        mut ricochet,
    ) in &mut projectiles
    {
        let profile = profile.copied().unwrap_or(DamageProfile::PHYSICAL_ATTACK);
        let simulation_seconds = first_frame.map_or(delta, |travel| travel.0.clamp(0.0, delta));
        let travel_seconds =
            (projectile.lifetime + simulation_seconds).clamp(0.0, simulation_seconds);
        let sweep_start = projectile_transform.translation - projectile.velocity * travel_seconds;
        let projectile_radius = if profile.class == DamageClass::Spell {
            projectile.radius * market_effects.spell_area_multiplier()
        } else if profile.class == DamageClass::Reaction && profile.element.is_elemental() {
            projectile.radius * market_effects.element_reaction_area_multiplier()
        } else {
            projectile.radius
        };
        let mut collision_order: Vec<(f32, Entity)> = collision_targets
            .iter()
            .filter_map(|(entity, position, elite)| {
                let already_hit = hit_memory
                    .as_deref()
                    .is_some_and(|memory| memory.enemies.contains(entity));
                if already_hit {
                    return None;
                }
                let hit_radius = projectile_radius + if *elite { 0.95 } else { 0.48 };
                segment_sphere_hit_progress(
                    sweep_start,
                    projectile_transform.translation,
                    *position,
                    hit_radius,
                )
                .map(|progress| (progress, *entity))
            })
            .collect();
        if sweep_start.distance_squared(projectile_transform.translation) > f32::EPSILON {
            collision_order.sort_by(|left, right| {
                left.0
                    .total_cmp(&right.0)
                    .then_with(|| left.1.to_bits().cmp(&right.1.to_bits()))
            });
        }
        let mut consumed = false;
        let mut first_frame_hits = HashSet::new();
        for (_, ordered_enemy) in collision_order {
            let Ok((
                enemy_entity,
                mut enemy,
                enemy_transform,
                trial_enemy,
                boss_enemy,
                adventure_boss,
                archetype,
                archive_echo,
                armor,
                mut affix,
                speed_effect,
            )) = enemies.get_mut(ordered_enemy)
            else {
                continue;
            };
            let already_hit = hit_memory
                .as_deref()
                .is_some_and(|memory| memory.enemies.contains(&enemy_entity))
                || first_frame_hits.contains(&enemy_entity);
            if enemy.hp <= 0.0
                || affix
                    .as_deref()
                    .is_some_and(|affix| affix.is_volatile_pending())
                || already_hit
            {
                continue;
            }
            let hit_radius = projectile_radius + if enemy.elite { 0.95 } else { 0.48 };
            if segment_sphere_hit_progress(
                sweep_start,
                projectile_transform.translation,
                enemy_transform.translation,
                hit_radius,
            )
            .is_some()
            {
                if let Some(memory) = hit_memory.as_deref_mut() {
                    memory.enemies.insert(enemy_entity);
                } else {
                    first_frame_hits.insert(enemy_entity);
                }
                let direct_shell_hit = profile.element == DamageElement::Physical
                    && projectile.velocity.length_squared() > 0.01
                    && archetype.is_some_and(|kind| kind.0 == 2);
                let frozen = skill_runtime.is_frozen(enemy_entity);
                let mut raw_damage = projectile.damage * bond_runtime.damage_multiplier(profile);
                let crit_multiplier = critical_multiplier(
                    &bond_runtime,
                    market_effects.burns_can_crit(),
                    profile,
                    rng.unit(),
                );
                let critical = crit_multiplier > 1.0;
                raw_damage *= crit_multiplier;
                raw_damage *= bond_runtime.target_damage_multiplier(
                    boss_enemy.is_some() || adventure_boss.is_some(),
                    trial_enemy.is_some_and(|trial| trial.kind == ChallengeKind::Tower),
                );
                if let Some(origin) = hero_position {
                    raw_damage *= bond_runtime
                        .distance_damage_multiplier(origin.distance(enemy_transform.translation));
                }
                raw_damage *=
                    market_outgoing_damage_multiplier(&market_effects, profile, &build, &vitals);
                raw_damage *= encounters.outgoing_damage_multiplier();
                raw_damage *= market_effects.chaos_damage_multiplier(&mut rng);
                if let Some(boss) = boss_enemy {
                    raw_damage *= super::bonds::boss_damage_multiplier_with_penetration(
                        profile,
                        BOSSES[usize::from(boss.index.min(4))].modifiers,
                        market_effects.spell_penetration(),
                    );
                }
                let damage = if profile.element == DamageElement::Physical {
                    physical_damage_after_armor(
                        raw_damage,
                        (armor.map_or(0.0, |armor| armor.0)
                            - market_effects.physical_penetration())
                        .max(0.0),
                        direct_shell_hit,
                    )
                } else {
                    raw_damage
                };
                let resolution = apply_enemy_damage(
                    &mut enemy,
                    affix.as_deref_mut(),
                    damage,
                    Some(&mut damage_meter),
                );
                if resolution.hp_damage > 0.0 {
                    pools.show_damage_number(
                        &mut commands,
                        enemy_entity,
                        enemy_transform.translation,
                        resolution.hp_damage,
                        profile.element,
                        critical,
                    );
                    if let Some(fusion) = fusion {
                        super::hero_fusion::apply_contact(
                            &mut commands,
                            enemy_entity,
                            fusion.kind,
                            boss_enemy.is_some() || adventure_boss.is_some(),
                            speed_effect,
                        );
                    }
                    vitals.heal(
                        resolution.hp_damage
                            * (market_effects.life_leech() + bond_runtime.life_leech()),
                    );
                }
                if profile.class == DamageClass::Attack
                    && !profile.market_proc
                    && market_effects.splash_ratio() > 0.0
                {
                    for (splash_target, splash_position) in
                        splash_targets(enemy_entity, enemy_transform.translation, &target_positions)
                    {
                        if let Some(projectile_entity) = rent_market_projectile(
                            &mut commands,
                            &mut pools,
                            &assets,
                            splash_position,
                            projectile.damage * market_effects.splash_ratio(),
                            0.30,
                            0,
                            profile.as_market_proc(),
                        ) {
                            restrict_projectile_to_target(
                                &mut commands,
                                projectile_entity,
                                splash_target,
                                &target_positions,
                            );
                        }
                    }
                }
                if resolution.hp_damage > 0. {
                    let (echo, chain_count) = bond_runtime.spell_procs(profile, rng.unit());
                    let mut proc_targets = Vec::new();
                    if echo && enemy.hp > 0. {
                        proc_targets.push((enemy_entity, enemy_transform.translation));
                    }
                    proc_targets.extend(
                        splash_targets(
                            enemy_entity,
                            enemy_transform.translation,
                            &target_positions,
                        )
                        .into_iter()
                        .take(chain_count as usize),
                    );
                    for (target, position) in proc_targets {
                        if let Some(entity) = rent_market_projectile(
                            &mut commands,
                            &mut pools,
                            &assets,
                            position,
                            projectile.damage,
                            0.2,
                            0,
                            profile.as_card_proc(),
                        ) {
                            restrict_projectile_to_target(
                                &mut commands,
                                entity,
                                target,
                                &target_positions,
                            );
                        }
                    }
                }
                if let Some(cues) = cues.as_deref_mut() {
                    if let Some(flight) = flight {
                        cues.push(
                            flight.kind,
                            enemy_transform.translation,
                            projectile.velocity,
                            true,
                        );
                    } else if profile.melee {
                        cues.push(vitals.kind, enemy_transform.translation, Vec3::X, true);
                    }
                }
                spawn_hit_fx(
                    &mut commands,
                    &mut pools,
                    &assets,
                    enemy_transform.translation,
                    enemy.elite,
                );
                if let Some(feedback) = hit_feedback.as_deref_mut()
                    && resolution.outcome != DamageOutcome::Pending
                {
                    feedback.push(HitEvent {
                        target: enemy_entity,
                        position: enemy_transform.translation,
                        element: profile.element,
                        weight: if boss_enemy.is_some() || adventure_boss.is_some() {
                            HitWeight::Boss
                        } else if enemy.elite {
                            HitWeight::Elite
                        } else {
                            HitWeight::Normal
                        },
                        critical,
                        killed: resolution.outcome == DamageOutcome::Defeated,
                    });
                }
                match resolution.outcome {
                    DamageOutcome::Regenerated => spawn_affix_trigger_fx(
                        &mut commands,
                        &mut pools,
                        &assets,
                        enemy_transform.translation,
                        EliteAffixKind::Regeneration,
                        false,
                    ),
                    DamageOutcome::VolatileStarted => spawn_affix_trigger_fx(
                        &mut commands,
                        &mut pools,
                        &assets,
                        enemy_transform.translation,
                        EliteAffixKind::Volatile,
                        true,
                    ),
                    DamageOutcome::Defeated => settle_enemy_defeat(
                        &mut commands,
                        &mut pools,
                        &assets,
                        &mut session,
                        &mut archive,
                        &mut trial,
                        &mut gold_trial,
                        &mut build,
                        &mut vitals,
                        &market_effects,
                        &mut encounters,
                        frozen && !profile.market_proc,
                        enemy_entity,
                        &enemy,
                        enemy_transform.translation,
                        trial_enemy,
                        boss_enemy,
                        adventure_boss,
                        archetype,
                        archive_echo,
                    ),
                    DamageOutcome::Alive | DamageOutcome::Pending => {}
                }
                if projectile.remaining_hits == 0 {
                    let origin = enemy_transform.translation;
                    let next = if ricochet
                        .as_deref()
                        .is_some_and(|bounce| bounce.remaining > 0)
                    {
                        let mut hit = first_frame_hits.clone();
                        if let Some(memory) = hit_memory.as_deref() {
                            hit.extend(memory.enemies.iter().copied());
                        }
                        let alive: Vec<_> = enemies
                            .iter()
                            .filter(|(_, enemy, _, _, _, _, _, _, _, affix, _)| {
                                enemy.hp > 0.0
                                    && !affix
                                        .as_deref()
                                        .is_some_and(|affix| affix.is_volatile_pending())
                            })
                            .map(|(entity, _, transform, ..)| (entity, transform.translation))
                            .collect();
                        basic_attacks::next_ricochet_target(origin, &alive, &hit)
                    } else {
                        None
                    };
                    if let Some((target, position)) = next {
                        let bounce = ricochet.as_deref_mut().expect("active ricochet");
                        bounce.remaining -= 1;
                        bounce.bounces_used += 1;
                        bounce.target = Some(target);
                        projectile.damage *= basic_attacks::RICOCHET_DAMAGE_RATIO;
                        let speed = projectile.velocity.length().max(1.0);
                        let direction = (position - origin).normalize_or(Vec3::X);
                        projectile.velocity = direction * speed;
                        projectile.lifetime = basic_attacks::RICOCHET_RANGE / speed + 0.2;
                        projectile_transform.translation = origin + Vec3::Y * 0.25;
                        projectile_transform.look_to(direction, Vec3::Y);
                        if let Some(cues) = cues.as_deref_mut() {
                            cues.ricochet(
                                flight.map_or(HeroKind::EmberRanger, |flight| flight.kind),
                                origin,
                                position,
                                bounce.bounces_used,
                            );
                        }
                        break;
                    }
                    consumed = true;
                    break;
                }
                projectile.remaining_hits -= 1;
            }
        }
        if first_frame.is_some() {
            commands
                .entity(projectile_entity)
                .remove::<ProjectileFirstFrameTravel>();
        }
        if !consumed && hit_memory.is_none() && !first_frame_hits.is_empty() {
            commands
                .entity(projectile_entity)
                .insert(ProjectileHitMemory {
                    enemies: first_frame_hits,
                });
        }
        if consumed || projectile.lifetime <= 0.0 {
            pools.retire_projectile(&mut commands, projectile_entity);
        }
    }
}

pub(crate) fn effective_basic_attack_rate(base: f32, modifier: f32) -> f32 {
    (base * modifier).max(0.2)
}

fn physical_damage_after_armor(damage: f32, armor: f32, shell_guard_direct_hit: bool) -> f32 {
    let armor_multiplier = 100.0 / (100.0 + 5.0 * armor.max(0.0));
    damage * armor_multiplier * if shell_guard_direct_hit { 0.6 } else { 1.0 }
}

pub(crate) fn market_outgoing_damage_multiplier(
    effects: &MarketEffects,
    profile: DamageProfile,
    build: &PlayerBuild,
    vitals: &HeroVitals,
) -> f32 {
    let threefold = if profile.class == DamageClass::Reaction {
        1.0
    } else {
        effects.threefold_multiplier()
    };
    let elemental_secondary =
        if profile.class == DamageClass::Reaction && profile.element.is_elemental() {
            effects.element_reaction_multiplier()
        } else {
            1.0
        };
    threefold
        * elemental_secondary
        * effects.low_health_damage_multiplier(vitals)
        * effects.top_skill_multiplier(profile.skill_index, build)
        * effects.last_oath_damage_multiplier()
}

fn splash_targets(
    source: Entity,
    position: Vec3,
    candidates: &[(Entity, Vec3)],
) -> Vec<(Entity, Vec3)> {
    let mut nearby: Vec<(Entity, Vec3, f32)> = candidates
        .iter()
        .filter_map(|(entity, target_position)| {
            let distance = planar_distance_squared(position, *target_position);
            (*entity != source && distance <= 3.0_f32.powi(2)).then_some((
                *entity,
                *target_position,
                distance,
            ))
        })
        .collect();
    nearby.sort_by(|left, right| {
        left.2
            .total_cmp(&right.2)
            .then_with(|| left.0.to_bits().cmp(&right.0.to_bits()))
    });
    nearby
        .into_iter()
        .take(6)
        .map(|(entity, target_position, _)| (entity, target_position))
        .collect()
}

fn cast_active_skill(
    time: Res<Time>,
    session: Res<GameSession>,
    mut active: ResMut<ActiveSkill>,
    pressure: Option<Res<ElitePressure>>,
    hero_query: Query<&Transform, With<Hero>>,
    enemies: Query<(
        &Enemy,
        &Transform,
        Option<&EnemyArchetype>,
        Option<&ArchiveEcho>,
        Option<&EliteAffix>,
    )>,
) {
    let hero_position = hero_query.single().ok().map(|hero| hero.translation);
    let null_eye_nearby = hero_position.is_some_and(|hero| {
        enemies
            .iter()
            .any(|(enemy, transform, archetype, archive_echo, affix)| {
                enemy.hp > 0.0
                    && !affix.is_some_and(|affix| affix.is_volatile_pending())
                    && archive_echo.is_none()
                    && archetype.is_some_and(|kind| kind.0 == 10)
                    && hero.distance_squared(transform.translation) <= 20.25
            })
    });
    if active.remaining > 0.0 && !session.paused && !session.finished {
        active.remaining = (active.remaining
            - time.delta_secs()
                * ultimate_recovery_rate(null_eye_nearby)
                * pressure
                    .as_ref()
                    .map_or(1.0, |p| p.cooldown_recovery_multiplier()))
        .max(0.0);
    }
}

const fn ultimate_recovery_rate(null_eye_nearby: bool) -> f32 {
    if null_eye_nearby { 0.85 } else { 1.0 }
}

fn enqueue_or_spawn_splitlings(
    commands: &mut Commands,
    pools: &mut CombatPools,
    assets: &GameAssets,
    position: Vec3,
    logical_count: usize,
) {
    let physical_slots = splitling_physical_slots(pools, logical_count);
    if physical_slots == 0
        || spawn_splitlings(
            commands,
            pools,
            assets,
            position,
            logical_count,
            physical_slots,
        ) == 0
    {
        pools.enqueue_splitlings(position, logical_count);
    }
}

fn drain_splitling_backlog(commands: &mut Commands, pools: &mut CombatPools, assets: &GameAssets) {
    while let Some((position, logical_count)) = pools.pop_pending_splitlings() {
        let physical_slots = splitling_physical_slots(pools, logical_count);
        if physical_slots == 0 {
            pools.push_front_pending_splitlings(position, logical_count);
            break;
        }
        if spawn_splitlings(
            commands,
            pools,
            assets,
            position,
            logical_count,
            physical_slots,
        ) == 0
        {
            pools.push_front_pending_splitlings(position, logical_count);
            break;
        }
    }
}

fn splitling_physical_slots(pools: &CombatPools, logical_count: usize) -> usize {
    MAX_REGULAR_ENEMIES
        .saturating_sub(pools.regular_enemy_active_count())
        .min(pools.enemy_active_capacity_remaining())
        .min(logical_count)
        .min(3)
}

fn spawn_splitlings(
    commands: &mut Commands,
    pools: &mut CombatPools,
    assets: &GameAssets,
    position: Vec3,
    logical_count: usize,
    physical_slots: usize,
) -> usize {
    let physical_slots = physical_slots.min(logical_count).min(3);
    if physical_slots == 0 || logical_count == 0 {
        return 0;
    }
    let transforms: Vec<_> = (0..physical_slots)
        .map(|slot| {
            let logical_count =
                logical_count / physical_slots + usize::from(slot < logical_count % physical_slots);
            let offset = (slot as f32 - (physical_slots as f32 - 1.0) * 0.5) * 0.65;
            let multiplier = logical_count as f32;
            Transform::from_xyz(position.x + 0.45, 0.0, position.z + offset)
                .with_scale(Vec3::splat(0.68 * (1.0 + (multiplier - 1.0) * 0.18)))
        })
        .collect();
    let Some(entities) = pools.rent_enemy_scene_group(
        commands,
        EnemySceneKey::Regular(1),
        assets.enemy_scenes[1].clone(),
        &transforms,
    ) else {
        return 0;
    };
    for (slot, entity) in entities.iter().copied().enumerate() {
        let logical_count =
            logical_count / physical_slots + usize::from(slot < logical_count % physical_slots);
        let multiplier = logical_count as f32;
        commands.entity(entity).insert((
            Name::new(if logical_count > 1 {
                format!("SPLITLING · MERGED x{logical_count}")
            } else {
                "SPLITLING".to_owned()
            }),
            Enemy {
                hp: 22.0 * multiplier,
                max_hp: 22.0 * multiplier,
                speed: 3.2,
                damage: 3.0 * multiplier,
                xp: 0,
                wood: 0,
                gold: 0,
                kill_credit: 0,
                elite: false,
            },
            EnemyArchetype(1),
            EnemyArmor(0.0),
            EnemyBreachDamage(4.0 * multiplier),
        ));
    }
    entities.len()
}

fn spawn_hit_fx(
    commands: &mut Commands,
    pools: &mut CombatPools,
    assets: &GameAssets,
    position: Vec3,
    elite: bool,
) {
    pools.rent_impact_fx(
        commands,
        assets.fx_mesh.clone(),
        assets.storm_material.clone(),
        Transform::from_xyz(position.x, 0.16, position.z).with_scale(Vec3::splat(0.18)),
        ImpactFx {
            lifetime: 0.18,
            start_scale: 0.18,
            end_scale: if elite { 1.6 } else { 0.75 },
        },
    );
}

fn animate_effects(
    mut commands: Commands,
    time: Res<Time>,
    mut pools: ResMut<CombatPools>,
    mut effects: Query<(Entity, &mut ImpactFx, &mut Transform)>,
) {
    for (entity, mut effect, mut transform) in &mut effects {
        effect.lifetime -= time.delta_secs();
        let total = 0.42_f32.max(effect.lifetime + time.delta_secs());
        let t = (1.0 - effect.lifetime / total).clamp(0.0, 1.0);
        let scale = effect.start_scale + (effect.end_scale - effect.start_scale) * t;
        transform.scale = Vec3::splat(scale);
        if effect.lifetime <= 0.0 {
            pools.retire_impact_fx(&mut commands, entity);
        }
    }
}

pub(crate) fn reset_run(world: &mut World) {
    world.insert_resource(WaveSpawnRuntime::default());
    world.insert_resource(RaidWarning::default());
    world.insert_resource(ElitePressure::default());
    world.insert_resource(AttackVolleyRuntime::default());
}

#[cfg(test)]
mod tests {
    #[test]
    fn damage_overlay_shares_hdr_target_without_tonemapping_world_twice() {
        let mut app = App::new();
        app.add_systems(Update, super::setup_damage_number_camera);
        app.world_mut().spawn((
            Camera3d::default(),
            Camera {
                order: 3,
                ..default()
            },
            Hdr,
            Msaa::Off,
            Tonemapping::TonyMcMapface,
            RenderTarget::default(),
        ));
        app.update();
        let world = app.world_mut();
        let mut cameras = world.query::<(
            &Camera,
            &Hdr,
            &Msaa,
            &Tonemapping,
            Option<&super::DamageNumberCamera>,
        )>();
        assert_eq!(cameras.iter(world).count(), 2);
        for (camera, _, msaa, tonemapping, overlay) in cameras.iter(world) {
            assert_eq!(*msaa, Msaa::Off);
            if overlay.is_some() {
                assert_eq!(camera.order, 4);
                assert_eq!(*tonemapping, Tonemapping::None);
                assert!(matches!(camera.clear_color, ClearColorConfig::None));
            } else {
                assert_eq!(*tonemapping, Tonemapping::TonyMcMapface);
            }
        }
    }

    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    fn test_game_assets() -> GameAssets {
        GameAssets {
            hero_scenes: std::array::from_fn(|_| Handle::default()),
            enemy_scenes: std::array::from_fn(|_| Handle::default()),
            boss_scenes: std::array::from_fn(|_| Handle::default()),
            adventure_scenes: default(),
            summon_scenes: std::array::from_fn(|_| Handle::default()),
            projectile_mesh: Handle::default(),
            spirit_mesh: default(),
            spirit_material: default(),
            fx_mesh: Handle::default(),
            enemy_material: Handle::default(),
            enemy_materials: std::array::from_fn(|_| Handle::default()),
            elite_material: Handle::default(),
            boss_materials: std::array::from_fn(|_| Handle::default()),
            hero_materials: std::array::from_fn(|_| Handle::default()),
            projectile_material: Handle::default(),
            storm_material: Handle::default(),
        }
    }

    fn test_enemy(hp: f32) -> Enemy {
        Enemy {
            hp,
            max_hp: 100.0,
            speed: 2.0,
            damage: 10.0,
            xp: 4,
            wood: 0,
            gold: 3,
            kill_credit: 1,
            elite: true,
        }
    }

    fn wave_test_app(elapsed: f32, wave: u32) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<EnemyAttackCue>()
            .init_resource::<SpawnDirector>()
            .init_resource::<WaveSpawnRuntime>()
            .init_resource::<RunRng>()
            .init_resource::<EncounterState>()
            .init_resource::<CombatPools>()
            .insert_resource(GameSession {
                elapsed,
                wave,
                ..default()
            })
            .insert_resource(test_game_assets());
        app
    }

    #[test]
    fn pressure_scales_actual_wave_spawns_once_but_not_campaign_bosses() {
        let mut baseline = wave_test_app(540.0, 10);
        let mut tuned = wave_test_app(540.0, 10);
        tuned.init_resource::<EnemyPressureSettings>();
        baseline
            .world_mut()
            .insert_resource(RunRng::from_seed(1234));
        tuned.world_mut().insert_resource(RunRng::from_seed(1234));
        baseline.world_mut().run_system_once(spawn_wave).unwrap();
        tuned.world_mut().run_system_once(spawn_wave).unwrap();
        let collect = |app: &mut App| {
            let mut entries = app
                .world_mut()
                .query::<(Option<&Name>, &Transform, &Enemy, Option<&BossEnemy>)>()
                .iter(app.world())
                .map(|(name, transform, enemy, boss)| {
                    // Entity reservation numbers belong to each world. Pair the
                    // same seeded spawn by its actual identity and position.
                    (
                        (
                            name.map_or("boss", Name::as_str).to_owned(),
                            transform.translation.x.to_bits(),
                            transform.translation.z.to_bits(),
                            boss.map(|boss| boss.index),
                        ),
                        enemy.hp,
                        enemy.damage,
                    )
                })
                .collect::<Vec<_>>();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            entries
        };
        let before = collect(&mut tuned);
        let original = collect(&mut baseline);
        assert_eq!(before.len(), original.len());
        assert!(before.iter().any(|entry| entry.0.3.is_some()));
        assert!(before.iter().any(|entry| entry.0.3.is_none()));
        for (actual, old) in before.iter().zip(&original) {
            assert_eq!(actual.0, old.0, "paired worlds must match seeded spawns");
            let multiplier = if actual.0.3.is_some() { 1.0 } else { 1.84 };
            assert!(
                (actual.1 - old.1 * multiplier).abs() < 0.01,
                "actual={actual:?}, baseline={old:?}; ordinary HP must scale once, bosses remain unchanged"
            );
            assert_eq!(actual.2, old.2);
        }
        tuned.world_mut().run_system_once(spawn_wave).unwrap();
        let after = collect(&mut tuned);
        assert_eq!(
            after, before,
            "existing HP must not multiply on subsequent frames"
        );
        for number in [4, 9, 25] {
            let old = WAVES[number - 1];
            let scaled = pressure_wave(old, number as u32, Some(&EnemyPressureSettings::default()));
            assert_eq!(scaled.count, old.count);
            assert_eq!(scaled.damage_multiplier, old.damage_multiplier);
            assert!(
                (scaled.hp_multiplier / old.hp_multiplier
                    - EnemyPressureSettings::default()
                        .at_wave(number as u32)
                        .health)
                    .abs()
                    < 0.001
            );
        }
    }

    #[test]
    fn disabled_director_blocks_ordinary_elites_and_due_bosses_without_advancing_state() {
        let mut app = wave_test_app(300.0, 6);
        app.world_mut().resource_mut::<SpawnDirector>().enabled = false;
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(30));
        let system = app.world_mut().register_system(spawn_wave);
        for _ in 0..3 {
            app.world_mut().run_system(system).unwrap();
        }
        assert_eq!(
            app.world_mut().query::<&Enemy>().iter(app.world()).count(),
            0
        );
        let director = app.world().resource::<SpawnDirector>();
        assert_eq!(director.boss_spawned, [false; 5]);
        assert_eq!(director.next_elite_at, 120.0);
        app.world_mut().resource_mut::<SpawnDirector>().enabled = true;
        app.world_mut().run_system(system).unwrap();
        assert!(app.world().resource::<SpawnDirector>().boss_spawned[0]);
        assert!(app.world_mut().query::<&Enemy>().iter(app.world()).count() > 0);
    }

    fn projectile_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<EnemyAttackCue>()
            .init_resource::<GameSession>()
            .init_resource::<DamageMeter>()
            .init_resource::<ArchiveState>()
            .init_resource::<TrialState>()
            .init_resource::<GoldChallengeState>()
            .init_resource::<PlayerBuild>()
            .init_resource::<HeroVitals>()
            .init_resource::<RunRng>()
            .init_resource::<BondRuntime>()
            .init_resource::<SkillRuntime>()
            .init_resource::<MarketEffects>()
            .init_resource::<BondRuntime>()
            .init_resource::<RunRng>()
            .init_resource::<EncounterState>()
            .init_resource::<CombatPools>()
            .insert_resource(test_game_assets());
        app
    }

    fn enemy_movement_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<EnemyAttackCue>()
            .init_resource::<GameSession>()
            .init_resource::<HeroVitals>()
            .init_resource::<MarketEffects>()
            .init_resource::<BondRuntime>()
            .init_resource::<EncounterState>()
            .init_resource::<RunRng>()
            .init_resource::<DodgeState>()
            .init_resource::<ActiveSkill>()
            .init_resource::<MoveIntent>()
            .init_resource::<CombatPools>();
        app.world_mut().spawn((Core, Transform::default()));
        app.world_mut()
            .spawn((Hero, Transform::default(), Visibility::Visible));
        app
    }

    #[test]
    fn enemy_attack_messages_follow_real_contact_and_freeze_when_paused() {
        let mut app = enemy_movement_test_app();
        let mut enemy = test_enemy(100.0);
        enemy.speed = 0.0;
        enemy.damage = 10.0;
        let source = app
            .world_mut()
            .spawn((
                enemy,
                EnemyArchetype(3),
                EnemyHeroThreat::ranged(),
                Transform::from_xyz(7.0, 0.0, 0.0),
            ))
            .id();
        let system = app.world_mut().register_system(move_enemies);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(1.0));
        let health = app.world().resource::<HeroVitals>().health;
        app.world_mut().run_system(system).unwrap();
        assert_eq!(app.world().resource::<Messages<EnemyAttackCue>>().len(), 0);
        assert_eq!(app.world().resource::<HeroVitals>().health, health);
        app.world_mut()
            .get_mut::<Transform>(source)
            .unwrap()
            .translation
            .x = 5.0;
        app.world_mut()
            .get_mut::<EnemyHeroThreat>(source)
            .unwrap()
            .attack_remaining = 0.5;
        app.world_mut().run_system(system).unwrap();
        assert_eq!(app.world().resource::<HeroVitals>().health, health - 10.0);
        let messages = app.world().resource::<Messages<EnemyAttackCue>>();
        let mut cursor = messages.get_cursor();
        let cues: Vec<_> = cursor.read(messages).collect();
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].source, source);
        assert!(matches!(cues[0].kind, EnemyAttackKind::Shot));
        assert_eq!(cues[0].target, Vec3::ZERO);
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.world_mut().run_system(system).unwrap();
        assert_eq!(app.world().resource::<Messages<EnemyAttackCue>>().len(), 1);
        assert_eq!(app.world().resource::<HeroVitals>().health, health - 10.0);
    }

    #[test]
    fn road_pressure_increases_real_gate_damage_but_not_camp_damage() {
        for camp in [false, true] {
            let mut app = enemy_movement_test_app();
            app.init_resource::<EnemyPressureSettings>();
            let mut heroes = app
                .world_mut()
                .query_filtered::<&mut Transform, With<Hero>>();
            heroes.single_mut(app.world_mut()).unwrap().translation.x = -3.0;
            app.world_mut().resource_mut::<GameSession>().wave = 25;
            let mut enemy = test_enemy(100.0);
            enemy.speed = 0.0;
            enemy.damage = 10.0;
            let entity = app
                .world_mut()
                .spawn((
                    enemy,
                    EnemyArchetype(11),
                    Transform::from_xyz(2.0, 0.0, 0.0),
                ))
                .id();
            if camp {
                app.world_mut()
                    .entity_mut(entity)
                    .insert(super::super::encounter_bosses::CampCombatant { center: Vec3::ZERO });
            }
            let initial = app.world().resource::<GameSession>().core_hp;
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_secs_f32(1.0));
            app.world_mut().run_system_once(move_enemies).unwrap();
            let dealt = initial - app.world().resource::<GameSession>().core_hp;
            assert!((dealt - if camp { 3.5 } else { 7.35 }).abs() < 0.001);
        }
    }

    #[test]
    fn siege_visual_cadence_does_not_change_continuous_gate_damage() {
        let mut app = enemy_movement_test_app();
        let mut enemy = test_enemy(100.0);
        enemy.speed = 0.0;
        enemy.damage = 10.0;
        app.world_mut().spawn((
            enemy,
            EnemyArchetype(11),
            Transform::from_xyz(8.0, 0.0, 0.0),
        ));
        let initial = app.world().resource::<GameSession>().core_hp;
        let system = app.world_mut().register_system(move_enemies);
        for _ in 0..5 {
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_secs_f32(0.25));
            app.world_mut().run_system(system).unwrap();
        }
        assert_eq!(app.world().resource::<Messages<EnemyAttackCue>>().len(), 2);
        assert!(
            (app.world().resource::<GameSession>().core_hp - (initial - 10.0 * 0.35 * 1.25)).abs()
                < 0.001
        );
    }

    #[test]
    fn siege_affix_prioritizes_live_gate_and_doubles_real_ranged_siege_damage() {
        let mut app = enemy_movement_test_app();
        let mut enemy = test_enemy(100.0);
        enemy.speed = 0.0;
        enemy.damage = 10.0;
        app.world_mut().spawn((
            enemy,
            EnemyArchetype(3),
            EnemyHeroThreat::ranged(),
            EliteAffix::new(EliteAffixKind::Siege),
            Transform::from_xyz(8.0, 0.0, 0.0),
        ));
        let mut query = app
            .world_mut()
            .query_filtered::<&mut Transform, With<Hero>>();
        query.single_mut(app.world_mut()).unwrap().translation = Vec3::X * 6.0;
        let hp = app.world().resource::<HeroVitals>().health;
        let gate = app.world().resource::<GameSession>().core_hp;
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(1));
        app.world_mut().run_system_once(move_enemies).unwrap();
        assert_eq!(app.world().resource::<HeroVitals>().health, hp);
        assert!((app.world().resource::<GameSession>().core_hp - (gate - 7.0)).abs() < 0.001);
    }

    #[test]
    fn brood_affix_uses_real_death_queue_once_and_children_cannot_hatch_again() {
        let mut app = wave_test_app(180.0, 4);
        app.init_resource::<ArchiveState>()
            .init_resource::<TrialState>()
            .init_resource::<GoldChallengeState>()
            .init_resource::<PlayerBuild>()
            .init_resource::<HeroVitals>()
            .init_resource::<MarketEffects>();
        app.world_mut()
            .run_system_once(
                |mut commands: Commands,
                 mut pools: ResMut<CombatPools>,
                 assets: Res<GameAssets>| {
                    let mut budget = test_budget(0, true);
                    budget.affix = Some(EliteAffixKind::Brood);
                    spawn_enemy(&mut commands, &mut pools, &assets, budget).unwrap();
                },
            )
            .unwrap();
        let settle = app.world_mut().register_system(
            |mut commands: Commands,
             mut pools: ResMut<CombatPools>,
             assets: Res<GameAssets>,
             (mut session, mut archive, mut trial, mut gold, mut build, mut vitals): (
                ResMut<GameSession>,
                ResMut<ArchiveState>,
                ResMut<TrialState>,
                ResMut<GoldChallengeState>,
                ResMut<PlayerBuild>,
                ResMut<HeroVitals>,
            ),
             market: Res<MarketEffects>,
             mut encounters: ResMut<EncounterState>,
             mut enemies: Query<(
                Entity,
                &mut Enemy,
                &Transform,
                &EnemyArchetype,
                &mut EliteAffix,
            )>| {
                for (entity, mut enemy, transform, archetype, mut affix) in &mut enemies {
                    if affix.kind == EliteAffixKind::Brood {
                        assert_eq!(
                            apply_enemy_damage(&mut enemy, Some(&mut affix), 100_000.0, None)
                                .outcome,
                            DamageOutcome::Defeated
                        );
                        settle_enemy_defeat(
                            &mut commands,
                            &mut pools,
                            &assets,
                            &mut session,
                            &mut archive,
                            &mut trial,
                            &mut gold,
                            &mut build,
                            &mut vitals,
                            &market,
                            &mut encounters,
                            false,
                            entity,
                            &enemy,
                            transform.translation,
                            None,
                            None,
                            None,
                            Some(archetype),
                            None,
                        );
                    }
                }
            },
        );
        app.world_mut().run_system(settle).unwrap();
        let mut children = app
            .world_mut()
            .query::<(&EnemyArchetype, Option<&EliteAffix>)>();
        let children: Vec<_> = children.iter(app.world()).collect();
        assert_eq!(children.len(), 3);
        assert!(
            children
                .iter()
                .all(|(kind, affix)| kind.0 == 1 && affix.is_none())
        );
        app.world_mut().run_system(settle).unwrap();
        let mut living = app.world_mut().query::<&Enemy>();
        assert_eq!(living.iter(app.world()).count(), 3);
    }

    #[test]
    fn heal_cues_require_actual_healing_and_keep_original_aura_rate() {
        let mut app = enemy_movement_test_app();
        app.world_mut().spawn((
            test_enemy(100.0),
            EnemyArchetype(8),
            Transform::from_xyz(1.0, 0.0, 0.0),
        ));
        let target = app
            .world_mut()
            .spawn((test_enemy(100.0), Transform::from_xyz(2.0, 0.0, 0.0)))
            .id();
        let system = app.world_mut().register_system(enemy_auras);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.25));
        app.world_mut().run_system(system).unwrap();
        assert_eq!(app.world().resource::<Messages<EnemyAttackCue>>().len(), 0);
        app.world_mut().get_mut::<Enemy>(target).unwrap().hp = 50.0;
        for _ in 0..5 {
            app.world_mut().run_system(system).unwrap();
        }
        assert_eq!(app.world().get::<Enemy>(target).unwrap().hp, 52.5);
        let messages = app.world().resource::<Messages<EnemyAttackCue>>();
        let mut cursor = messages.get_cursor();
        let cues: Vec<_> = cursor.read(messages).collect();
        assert_eq!(cues.len(), 2);
        assert!(
            cues.iter()
                .all(|cue| matches!(cue.kind, EnemyAttackKind::Heal))
        );
    }

    #[test]
    fn move_towards_radius_clamps_at_threshold_and_reports_leftover_time() {
        let (position, remaining, arrived) =
            move_towards_radius(Vec3::new(3.0, 0.0, 0.0), Vec3::ZERO, 2.0, 1.0, 1.25);
        assert!(arrived);
        assert!((position.x - 1.25).abs() < 0.000_001);
        assert!((remaining - 0.125).abs() < 0.000_001);

        let (position, remaining, arrived) =
            move_towards_radius(Vec3::new(3.0, 0.0, 0.0), Vec3::ZERO, 2.0, 0.75, 1.25);
        assert!(!arrived);
        assert!((position.x - 1.5).abs() < 0.000_001);
        assert_eq!(remaining, 0.0);
    }

    #[test]
    fn gate_rebuilds_repeatedly_without_losing_and_pause_preserves_timer() {
        let mut app = App::new();
        app.init_resource::<Time>().init_resource::<GameSession>();
        let gate = app.world_mut().spawn((Core, Visibility::Visible)).id();
        let system = app.world_mut().register_system(tick_gate_rebuild);
        for _ in 0..5 {
            app.world_mut().resource_mut::<GameSession>().core_hp = 0.0;
            app.world_mut().run_system(system).unwrap();
            assert_eq!(
                app.world().resource::<GameSession>().gate_revive_remaining,
                4.0
            );
            assert_eq!(
                *app.world().get::<Visibility>(gate).unwrap(),
                Visibility::Hidden
            );
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_secs(4));
            app.world_mut().resource_mut::<GameSession>().paused = true;
            app.world_mut().run_system(system).unwrap();
            assert_eq!(
                app.world().resource::<GameSession>().gate_revive_remaining,
                4.0
            );
            app.world_mut().resource_mut::<GameSession>().paused = false;
            app.world_mut().run_system(system).unwrap();
            assert_eq!(app.world().resource::<GameSession>().core_hp, 1000.0);
            assert!(!app.world().resource::<GameSession>().finished);
            assert_eq!(
                *app.world().get::<Visibility>(gate).unwrap(),
                Visibility::Visible
            );
        }
    }
    #[test]
    fn broken_gate_redirects_ordinary_enemies_into_real_hero_damage() {
        let mut app = enemy_movement_test_app();
        app.world_mut().resource_mut::<GameSession>().core_hp = 0.0;
        let hero_position = app
            .world_mut()
            .query_filtered::<&Transform, With<Hero>>()
            .single(app.world())
            .unwrap()
            .translation;
        let before = app.world().resource::<HeroVitals>().health;
        let entity = app
            .world_mut()
            .spawn((
                test_enemy(100.0),
                EnemyArchetype(0),
                Transform::from_translation(hero_position + Vec3::X),
            ))
            .id();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(1));
        app.world_mut().run_system_once(move_enemies).unwrap();
        assert!(app.world().resource::<HeroVitals>().health < before);
        assert_eq!(app.world().resource::<GameSession>().core_hp, 0.0);
        assert!(app.world().get::<Enemy>(entity).is_some());
        assert!(!app.world().resource::<GameSession>().finished);
    }

    #[test]
    fn broken_gate_redirects_zero_native_damage_rushers_using_their_breach_damage() {
        let mut app = enemy_movement_test_app();
        app.world_mut().resource_mut::<GameSession>().core_hp = 0.0;
        let hero_position = app
            .world_mut()
            .query_filtered::<&Transform, With<Hero>>()
            .single(app.world())
            .unwrap()
            .translation;
        let before = app.world().resource::<HeroVitals>().health;
        let entity = app
            .world_mut()
            .spawn((
                Enemy {
                    damage: 0.0,
                    ..test_enemy(100.0)
                },
                EnemyArchetype(0),
                EnemyBreachDamage(20.0),
                Transform::from_translation(hero_position + Vec3::X),
            ))
            .id();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(1));
        app.world_mut().run_system_once(move_enemies).unwrap();
        assert!(app.world().resource::<HeroVitals>().health < before);
        assert_eq!(app.world().resource::<GameSession>().core_hp, 0.0);
        assert!(app.world().get::<Enemy>(entity).is_some());
        assert!(!app.world().resource::<GameSession>().finished);
    }

    #[test]
    fn standing_gate_blocks_every_lane_and_protects_rear_hero_from_hunters_and_burrowers() {
        for (archetype, lane) in [(0, -4.3), (3, -2.0), (7, 2.0), (11, 4.3)] {
            let mut app = enemy_movement_test_app();
            let hero_entity = app
                .world_mut()
                .query_filtered::<Entity, With<Hero>>()
                .single(app.world())
                .unwrap();
            app.world_mut()
                .get_mut::<Transform>(hero_entity)
                .unwrap()
                .translation
                .x = -3.0;
            let original_health = app.world().resource::<HeroVitals>().health;
            let original_gate = app.world().resource::<GameSession>().core_hp;
            let mut enemy = test_enemy(100.0);
            enemy.damage = 10.0;
            enemy.speed = 100.0;
            enemy.elite = false;
            let entity = app
                .world_mut()
                .spawn((
                    enemy,
                    EnemyArchetype(archetype),
                    EnemyBreachDamage(10.0),
                    EnemyHeroThreat::burrowed(),
                    Transform::from_xyz(10.0, 0.0, lane),
                ))
                .id();
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_secs(1));
            app.world_mut().run_system_once(move_enemies).unwrap();
            let position = app.world().get::<Transform>(entity).unwrap().translation;
            assert!(
                position.x >= 1.25,
                "archetype {archetype} crossed the live wall"
            );
            assert_eq!(position.z, lane, "gate attackers must keep their lane");
            assert_eq!(app.world().resource::<HeroVitals>().health, original_health);
            assert!(app.world().resource::<GameSession>().core_hp < original_gate);
            assert!(!app.world().get::<EnemyHeroThreat>(entity).unwrap().emerged);
        }
    }

    #[test]
    fn rebuilt_gate_keeps_breached_enemies_behind_it_and_front_arrivals_blocked() {
        let mut app = enemy_movement_test_app();
        let mut heroes = app
            .world_mut()
            .query_filtered::<&mut Transform, With<Hero>>();
        heroes.single_mut(app.world_mut()).unwrap().translation.x = -3.0;
        let mut enemy = test_enemy(100.0);
        enemy.speed = 4.0;
        enemy.damage = 10.0;
        let breached = app
            .world_mut()
            .spawn((enemy, Transform::from_xyz(2.0, 0.0, 0.0)))
            .id();
        {
            let mut session = app.world_mut().resource_mut::<GameSession>();
            session.core_hp = 0.0;
            session.gate_revive_remaining = 4.0;
        }
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(1));
        app.world_mut().run_system_once(move_enemies).unwrap();
        assert!(
            app.world()
                .get::<Transform>(breached)
                .unwrap()
                .translation
                .x
                < 0.0
        );
        {
            let mut session = app.world_mut().resource_mut::<GameSession>();
            session.core_hp = session.core_max_hp;
            session.gate_revive_remaining = 0.0;
        }
        let mut enemy = test_enemy(100.0);
        enemy.speed = 4.0;
        enemy.damage = 10.0;
        let front = app
            .world_mut()
            .spawn((enemy, Transform::from_xyz(4.0, 0.0, 2.0)))
            .id();
        let health = app.world().resource::<HeroVitals>().health;
        app.world_mut().run_system_once(move_enemies).unwrap();
        assert!(
            app.world()
                .get::<Transform>(breached)
                .unwrap()
                .translation
                .x
                < 0.0
        );
        assert!(app.world().resource::<HeroVitals>().health < health);
        assert!(app.world().get::<Transform>(front).unwrap().translation.x >= 1.25);
        assert_eq!(
            app.world().resource::<GameSession>().gate_revive_remaining,
            0.0
        );
    }

    #[test]
    fn boss_charge_cannot_tunnel_through_live_gate_but_can_pass_collapsed_gate() {
        let mut session = GameSession::default();
        let gate = Some(Vec3::ZERO);
        let origin = Vec3::new(5.0, 0.0, -4.3);
        let target = Vec3::new(-3.0, 0.0, -4.3);
        assert_eq!(
            stop_at_live_gate(&session, gate, origin, target),
            Vec3::new(1.9, 0.0, -4.3)
        );
        session.core_hp = 0.0;
        assert_eq!(stop_at_live_gate(&session, gate, origin, target), target);
        session.core_hp = 1000.0;
        session.gate_revive_remaining = 2.0;
        assert_eq!(stop_at_live_gate(&session, gate, origin, target), target);
    }

    #[test]
    fn ordinary_enemy_crossing_gate_radius_breaches_in_the_same_frame() {
        let mut app = enemy_movement_test_app();
        let initial_core_hp = app.world().resource::<GameSession>().core_hp;
        let mut enemy = test_enemy(100.0);
        enemy.elite = false;
        let entity = app
            .world_mut()
            .spawn((
                enemy,
                EnemyArchetype(0),
                EnemyBreachDamage(17.0),
                Transform::from_xyz(2.0, 0.0, 0.0),
            ))
            .id();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(750));

        app.world_mut()
            .run_system_once(move_enemies)
            .expect("gate crossing should run");
        assert_eq!(app.world().resource::<Messages<EnemyAttackCue>>().len(), 1);

        assert_eq!(
            app.world().resource::<GameSession>().core_hp,
            initial_core_hp - 17.0 * 0.375
        );
        assert!(
            (app.world().get::<Transform>(entity).unwrap().translation.x - 1.25).abs() < 0.000_001,
            "the enemy must stop at the breach radius instead of overshooting"
        );
    }

    #[test]
    fn hunter_crossing_attack_radius_uses_leftover_frame_time() {
        let mut app = enemy_movement_test_app();
        let initial_health = app.world().resource::<HeroVitals>().health;
        let mut enemy = test_enemy(100.0);
        enemy.elite = false;
        enemy.damage = 10.0;
        let entity = app
            .world_mut()
            .spawn((
                enemy,
                EnemyArchetype(3),
                EnemyHeroThreat {
                    attack_remaining: 0.1,
                    ..EnemyHeroThreat::ranged()
                },
                Transform::from_xyz(7.0, 0.0, 0.0),
            ))
            .id();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(750));

        app.world_mut()
            .run_system_once(move_enemies)
            .expect("hunter crossing should run");

        assert_eq!(
            app.world().resource::<HeroVitals>().health,
            initial_health - 10.0
        );
        assert!(
            (app.world().get::<Transform>(entity).unwrap().translation.x - ACID_ARCHER_RANGE).abs()
                < 0.000_001
        );
        assert!(
            (app.world()
                .get::<EnemyHeroThreat>(entity)
                .unwrap()
                .attack_remaining
                - 0.85)
                .abs()
                < 0.000_001
        );
    }

    #[test]
    fn burrower_crossing_reveal_radius_emerges_and_strikes_in_the_same_frame() {
        let mut app = enemy_movement_test_app();
        let initial_health = app.world().resource::<HeroVitals>().health;
        let mut enemy = test_enemy(100.0);
        enemy.elite = false;
        enemy.damage = 10.0;
        let entity = app
            .world_mut()
            .spawn((
                enemy,
                EnemyArchetype(7),
                EnemyHeroThreat::burrowed(),
                Transform::from_xyz(3.0, 0.0, 0.0),
            ))
            .id();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(750));

        app.world_mut()
            .run_system_once(move_enemies)
            .expect("burrow crossing should run");
        let cues = app.world().resource::<Messages<EnemyAttackCue>>();
        assert_eq!(cues.len(), 1);
        assert!(
            cues.get_cursor()
                .read(cues)
                .all(|cue| matches!(cue.kind, EnemyAttackKind::Slam))
        );

        let threat = app.world().get::<EnemyHeroThreat>(entity).unwrap();
        let transform = app.world().get::<Transform>(entity).unwrap();
        assert!(threat.emerged && threat.stun_applied);
        assert_eq!(threat.attack_remaining, BURROW_ATTACK_INTERVAL);
        assert!((transform.translation.x - BURROW_REVEAL_RANGE).abs() < 0.000_001);
        assert_eq!(transform.translation.y, 0.0);
        assert_eq!(
            app.world().resource::<HeroVitals>().health,
            initial_health - 10.0
        );
        assert_eq!(
            app.world().resource::<HeroVitals>().stunned_remaining,
            BURROW_STUN_SECONDS
        );
    }

    #[test]
    fn basic_attack_counter_advances_once_per_successful_volley() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<EnemyAttackCue>()
            .init_resource::<GameSession>()
            .init_resource::<HeroVitals>()
            .init_resource::<PlayerBuild>()
            .init_resource::<MarketEffects>()
            .init_resource::<BondRuntime>()
            .init_resource::<RunRng>()
            .init_resource::<CombatPools>()
            .init_resource::<AttackClock>()
            .init_resource::<AttackVolleyRuntime>()
            .init_resource::<BasicAttackCounter>()
            .insert_resource(test_game_assets());
        app.world_mut().resource_mut::<PlayerBuild>().multishot = 3;
        app.world_mut().spawn((Hero, Transform::default()));
        app.world_mut()
            .spawn((test_enemy(100.0), Transform::from_xyz(2.0, 0.0, 0.0)));
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(1));

        app.world_mut()
            .run_system_once(auto_attack)
            .expect("auto attack should run");

        assert_eq!(app.world().resource::<BasicAttackCounter>().0, 1);
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<Projectile>>()
                .iter(app.world())
                .count(),
            3,
            "multishot projectiles still represent one basic-attack volley"
        );
    }

    #[test]
    fn archer_native_volley_aims_distinct_targets_and_attaches_real_bounces() {
        let mut app = projectile_test_app();
        app.init_resource::<AttackClock>()
            .init_resource::<AttackVolleyRuntime>();
        app.world_mut().resource_mut::<HeroVitals>().kind = HeroKind::EmberRanger;
        app.world_mut().resource_mut::<HeroVitals>().level = 10;
        app.world_mut()
            .resource_mut::<PlayerBuild>()
            .attacks_per_second = 1.0;
        app.world_mut().spawn((Hero, Transform::default()));
        app.world_mut()
            .spawn((test_enemy(1000.0), Transform::from_xyz(3.0, 0.0, -1.0)));
        app.world_mut()
            .spawn((test_enemy(1000.0), Transform::from_xyz(3.0, 0.0, 1.0)));
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(1));
        app.world_mut().run_system_once(auto_attack).unwrap();
        let mut arrows = app
            .world_mut()
            .query::<(&Projectile, &basic_attacks::Ricochet)>();
        let arrows: Vec<_> = arrows.iter(app.world()).collect();
        assert_eq!(arrows.len(), 2);
        assert!(arrows.iter().all(|(_, bounce)| bounce.remaining == 1));
        assert!(arrows[0].0.velocity.z * arrows[1].0.velocity.z < 0.0);
        assert_eq!(arrows[0].0.damage, arrows[1].0.damage);
    }

    #[test]
    fn archer_single_enemy_never_receives_duplicate_initial_arrows() {
        for level in [1, 10, 40] {
            let mut app = projectile_test_app();
            app.init_resource::<AttackClock>()
                .init_resource::<AttackVolleyRuntime>();
            {
                let mut vitals = app.world_mut().resource_mut::<HeroVitals>();
                vitals.kind = HeroKind::EmberRanger;
                vitals.level = level;
            }
            app.world_mut()
                .resource_mut::<PlayerBuild>()
                .attacks_per_second = 1.0;
            app.world_mut().spawn((Hero, Transform::default()));
            app.world_mut()
                .spawn((test_enemy(1000.0), Transform::from_xyz(3.0, 0.0, 0.0)));
            app.world_mut()
                .spawn((test_enemy(0.0), Transform::from_xyz(3.0, 0.0, 1.0)));
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_secs(1));
            app.world_mut().run_system_once(auto_attack).unwrap();
            let mut arrows = app
                .world_mut()
                .query::<(&Projectile, Option<&basic_attacks::Ricochet>)>();
            let arrows: Vec<_> = arrows.iter(app.world()).collect();
            assert_eq!(arrows.len(), 1, "level {level}");
            assert_eq!(arrows[0].1.is_some(), level >= 10);
            assert_eq!(
                arrows[0].0.damage,
                app.world().resource::<PlayerBuild>().attack_damage
            );
        }
    }

    #[test]
    fn ricochet_hits_alive_unvisited_targets_and_applies_falloff() {
        let mut app = projectile_test_app();
        app.world_mut()
            .insert_resource(RunRng::from_seed(123456789));
        let first = app
            .world_mut()
            .spawn((
                test_enemy(1000.0),
                EnemyArmor(20.0),
                Transform::from_xyz(2.0, 0.0, 0.0),
            ))
            .id();
        let second = app
            .world_mut()
            .spawn((
                test_enemy(1000.0),
                EnemyArmor(20.0),
                Transform::from_xyz(2.0, 0.0, 3.0),
            ))
            .id();
        let third = app
            .world_mut()
            .spawn((
                test_enemy(1000.0),
                EnemyArmor(20.0),
                Transform::from_xyz(2.0, 0.0, 6.0),
            ))
            .id();
        let dead = app
            .world_mut()
            .spawn((test_enemy(0.0), Transform::from_xyz(2.0, 0.0, 1.0)))
            .id();
        let projectile = app
            .world_mut()
            .run_system_once(|mut commands: Commands, mut pools: ResMut<CombatPools>| {
                let entity = pools
                    .rent_projectile(
                        &mut commands,
                        Handle::default(),
                        Handle::default(),
                        Transform::from_xyz(2.0, 0.0, 0.0),
                        Projectile {
                            velocity: Vec3::X * 20.0,
                            damage: 100.0,
                            radius: 0.1,
                            remaining_hits: 0,
                            lifetime: 1.0,
                        },
                    )
                    .unwrap();
                commands
                    .entity(entity)
                    .insert(basic_attacks::Ricochet::new(2));
                entity
            })
            .unwrap();
        app.world_mut().run_system_once(projectile_hits).unwrap();
        assert_eq!(app.world().get::<Enemy>(first).unwrap().hp, 950.0);
        assert_eq!(
            app.world()
                .get::<basic_attacks::Ricochet>(projectile)
                .unwrap()
                .target,
            Some(second)
        );
        assert_eq!(
            app.world().get::<Projectile>(projectile).unwrap().damage,
            75.0
        );
        app.world_mut()
            .get_mut::<Transform>(projectile)
            .unwrap()
            .translation = Vec3::new(2.0, 0.0, 3.0);
        app.world_mut().run_system_once(projectile_hits).unwrap();
        assert_eq!(app.world().get::<Enemy>(second).unwrap().hp, 962.5);
        assert_eq!(
            app.world()
                .get::<basic_attacks::Ricochet>(projectile)
                .unwrap()
                .target,
            Some(third)
        );
        assert_eq!(
            app.world().get::<Projectile>(projectile).unwrap().damage,
            56.25
        );
        app.world_mut()
            .get_mut::<Transform>(projectile)
            .unwrap()
            .translation = Vec3::new(2.0, 0.0, 6.0);
        app.world_mut().run_system_once(projectile_hits).unwrap();
        assert_eq!(app.world().get::<Enemy>(third).unwrap().hp, 971.875);
        assert_eq!(app.world().get::<Enemy>(dead).unwrap().hp, 0.0);
        assert_eq!(app.world().get::<Enemy>(first).unwrap().hp, 950.0);
        assert!(app.world().get::<Projectile>(projectile).is_none());
        assert!(
            app.world()
                .get::<basic_attacks::Ricochet>(projectile)
                .is_none()
        );
    }

    #[test]
    fn pierce_keeps_full_damage_until_final_contact_then_redirects_a_real_arrow() {
        let mut app = projectile_test_app();
        app.world_mut()
            .insert_resource(RunRng::from_seed(123456789));
        let first = app
            .world_mut()
            .spawn((
                test_enemy(1000.0),
                EnemyArmor(20.0),
                Transform::from_xyz(2.0, 0.0, 0.0),
            ))
            .id();
        let second = app
            .world_mut()
            .spawn((
                test_enemy(1000.0),
                EnemyArmor(20.0),
                Transform::from_xyz(4.0, 0.0, 0.0),
            ))
            .id();
        let next = app
            .world_mut()
            .spawn((test_enemy(1000.0), Transform::from_xyz(4.0, 0.0, 3.0)))
            .id();
        let projectile = app
            .world_mut()
            .run_system_once(|mut commands: Commands, mut pools: ResMut<CombatPools>| {
                let entity = pools
                    .rent_projectile(
                        &mut commands,
                        Handle::default(),
                        Handle::default(),
                        Transform::from_xyz(6.0, 0.0, 0.0),
                        Projectile {
                            velocity: Vec3::X * 20.0,
                            damage: 100.0,
                            radius: 0.1,
                            remaining_hits: 1,
                            lifetime: 1.0,
                        },
                    )
                    .unwrap();
                commands
                    .entity(entity)
                    .insert(basic_attacks::Ricochet::new(1));
                entity
            })
            .unwrap();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.3));
        app.world_mut().run_system_once(projectile_hits).unwrap();
        assert_eq!(app.world().get::<Enemy>(first).unwrap().hp, 950.0);
        assert_eq!(app.world().get::<Enemy>(second).unwrap().hp, 950.0);
        assert_eq!(app.world().get::<Enemy>(next).unwrap().hp, 1000.0);
        assert_eq!(
            app.world()
                .get::<basic_attacks::Ricochet>(projectile)
                .unwrap()
                .target,
            Some(next)
        );
        assert_eq!(
            app.world().get::<Projectile>(projectile).unwrap().damage,
            75.0
        );
    }

    #[test]
    fn mage_inherits_genuine_archer_volley_bounces_and_warrior_cleave_after_absorption() {
        let mut app = projectile_test_app();
        app.init_resource::<AttackClock>()
            .init_resource::<AttackVolleyRuntime>()
            .init_resource::<BasicAttackCounter>()
            .init_resource::<WeaponCues>()
            .add_plugins(super::super::hero_fusion::HeroFusionPlugin);
        app.world_mut().resource_mut::<HeroVitals>().kind = HeroKind::StormSavant;
        app.world_mut().resource_mut::<HeroVitals>().level = 40;
        app.world_mut()
            .resource_mut::<PlayerBuild>()
            .attacks_per_second = 1.0;
        {
            let mut fusion = app.world_mut().resource_mut::<super::super::HeroFusion>();
            fusion.record_absorbed(HeroKind::StoneWarden);
            fusion.record_absorbed(HeroKind::EmberRanger);
        }
        app.world_mut().spawn((Hero, Transform::default()));
        app.world_mut()
            .spawn((test_enemy(1000.0), Transform::from_xyz(2.0, 0.0, 0.0)));
        for _ in 0..8 {
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_secs(1));
            app.world_mut().run_system_once(auto_attack).unwrap();
        }
        app.update();
        let mut arrows=app.world_mut().query_filtered::<(&Projectile,&basic_attacks::Ricochet),With<super::super::hero_fusion::FusionAttack>>();
        let arrows: Vec<_> = arrows.iter(app.world()).collect();
        assert_eq!(arrows.len(), 1);
        assert!(arrows.iter().all(|(_, bounce)| bounce.remaining == 3));
        let base = app.world().resource::<PlayerBuild>().attack_damage;
        assert!(
            arrows
                .iter()
                .all(|(arrow, _)| (arrow.damage - base * 0.45).abs() < 0.001)
        );
        let mut swings = app
            .world_mut()
            .query_filtered::<&PendingSwing, With<super::super::hero_fusion::FusionAttack>>();
        let swing = swings.single(app.world()).unwrap();
        assert_eq!(swing.hit_limit, 3);
        assert!((swing.damage - base * 0.45).abs() < 0.001);
        assert_eq!(app.world().resource::<BasicAttackCounter>().0, 8);
    }

    #[test]
    fn bounced_arrow_steers_toward_moving_target_without_teleporting_and_retires_if_it_dies() {
        let mut app = projectile_test_app();
        let target = app
            .world_mut()
            .spawn((test_enemy(1000.0), Transform::from_xyz(8.0, 0.0, 0.0)))
            .id();
        let projectile = app
            .world_mut()
            .run_system_once(
                move |mut commands: Commands, mut pools: ResMut<CombatPools>| {
                    let entity = pools
                        .rent_projectile(
                            &mut commands,
                            Handle::default(),
                            Handle::default(),
                            Transform::default(),
                            Projectile {
                                velocity: Vec3::X * 10.0,
                                damage: 100.0,
                                radius: 0.1,
                                remaining_hits: 0,
                                lifetime: 1.0,
                            },
                        )
                        .unwrap();
                    commands.entity(entity).insert(basic_attacks::Ricochet {
                        remaining: 0,
                        bounces_used: 1,
                        target: Some(target),
                    });
                    entity
                },
            )
            .unwrap();
        app.world_mut()
            .get_mut::<Transform>(target)
            .unwrap()
            .translation
            .z = 4.0;
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.1));
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.world_mut().run_system_once(move_projectiles).unwrap();
        assert_eq!(
            app.world()
                .get::<Transform>(projectile)
                .unwrap()
                .translation,
            Vec3::ZERO
        );
        app.world_mut().resource_mut::<GameSession>().paused = false;
        app.world_mut().run_system_once(move_projectiles).unwrap();
        let position = app
            .world()
            .get::<Transform>(projectile)
            .unwrap()
            .translation;
        assert!(position.x > 0.0 && position.z > 0.0);
        assert!((position.length() - 1.0).abs() < 0.001);
        assert_eq!(app.world().get::<Enemy>(target).unwrap().hp, 1000.0);
        app.world_mut().get_mut::<Enemy>(target).unwrap().hp = 0.0;
        app.world_mut().run_system_once(move_projectiles).unwrap();
        assert!(app.world().get::<Projectile>(projectile).is_none());
        assert!(
            app.world()
                .get::<basic_attacks::Ricochet>(projectile)
                .is_none()
        );
    }

    #[test]
    fn melee_basics_require_weapon_reach_and_count_one_swing() {
        let mut app = projectile_test_app();
        app.init_resource::<AttackClock>()
            .init_resource::<AttackVolleyRuntime>()
            .init_resource::<BasicAttackCounter>();
        app.world_mut().resource_mut::<HeroVitals>().kind = HeroKind::StoneWarden;
        app.world_mut().resource_mut::<PlayerBuild>().multishot = 3;
        app.world_mut()
            .resource_mut::<PlayerBuild>()
            .attacks_per_second = 1.0;
        app.world_mut().spawn((Hero, Transform::default()));
        let enemy = app
            .world_mut()
            .spawn((test_enemy(100.0), Transform::from_xyz(7.0, 0.0, 0.0)))
            .id();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(1));
        app.world_mut().run_system_once(auto_attack).unwrap();
        assert_eq!(app.world().resource::<BasicAttackCounter>().0, 0);
        assert_eq!(
            app.world_mut()
                .query::<&PendingSwing>()
                .iter(app.world())
                .count(),
            0
        );
        app.world_mut()
            .get_mut::<Transform>(enemy)
            .unwrap()
            .translation
            .x = 1.5;
        app.world_mut().run_system_once(auto_attack).unwrap();
        assert_eq!(app.world().resource::<BasicAttackCounter>().0, 1);
        assert_eq!(
            app.world_mut()
                .query::<&PendingSwing>()
                .iter(app.world())
                .count(),
            1
        );
        assert_eq!(
            app.world_mut()
                .query::<&Projectile>()
                .iter(app.world())
                .count(),
            0,
            "starting a melee swing must not launch a projectile"
        );
    }

    #[test]
    fn weapon_contact_uses_shared_damage_resolution_and_cannot_hit_behind() {
        let mut app = projectile_test_app();
        app.world_mut().resource_mut::<HeroVitals>().kind = HeroKind::StoneWarden;
        app.world_mut()
            .insert_resource(RunRng::from_seed(123456789));
        let hero = app.world_mut().spawn((Hero, Transform::default())).id();
        let primary = app
            .world_mut()
            .spawn((test_enemy(100.0), Transform::from_xyz(1.5, 0.0, 0.0)))
            .id();
        let nearby = app
            .world_mut()
            .spawn((test_enemy(100.0), Transform::from_xyz(1.3, 0.0, 0.2)))
            .id();
        let behind = app
            .world_mut()
            .spawn((test_enemy(100.0), Transform::from_xyz(-1.0, 0.0, 0.0)))
            .id();
        app.world_mut().entity_mut(primary).insert(EnemyArmor(20.0));
        app.world_mut().entity_mut(nearby).insert(EnemyArmor(20.0));
        app.world_mut().spawn(PendingSwing {
            hero,
            target: primary,
            kind: HeroKind::StoneWarden,
            direction: Vec3::X,
            damage: 10.0,
            hit_limit: 2,
            remaining: 0.05,
        });
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(100));
        app.world_mut()
            .run_system_once(basic_attacks::resolve_swings)
            .unwrap();
        let contact_entities: Vec<_> = app
            .world_mut()
            .query_filtered::<Entity, With<Projectile>>()
            .iter(app.world())
            .collect();
        let mut contacts = app.world_mut().query::<(&Projectile, Option<&Mesh3d>)>();
        assert_eq!(contacts.iter(app.world()).count(), 2);
        for (contact, mesh) in contacts.iter(app.world()) {
            assert_eq!(contact.velocity, Vec3::ZERO);
            assert!(mesh.is_none(), "melee contacts are not visible bullets");
        }
        app.world_mut().run_system_once(move_projectiles).unwrap();
        app.world_mut().run_system_once(projectile_hits).unwrap();
        assert_eq!(app.world().get::<Enemy>(primary).unwrap().hp, 95.0);
        assert_eq!(app.world().get::<Enemy>(nearby).unwrap().hp, 97.0);
        assert_eq!(app.world().get::<Enemy>(behind).unwrap().hp, 100.0);
        assert_eq!(
            app.world_mut()
                .query::<&Projectile>()
                .iter(app.world())
                .count(),
            0,
            "contact carriers retire through the normal pool path"
        );
        app.world_mut()
            .run_system_once(super::super::pooling::finalize_recycles)
            .unwrap();
        let reused = app
            .world_mut()
            .run_system_once(|mut commands: Commands, mut pools: ResMut<CombatPools>| {
                pools
                    .rent_projectile(
                        &mut commands,
                        Handle::default(),
                        Handle::default(),
                        Transform::default(),
                        Projectile {
                            velocity: Vec3::X * 20.0,
                            damage: 10.0,
                            radius: 0.4,
                            remaining_hits: 0,
                            lifetime: 1.0,
                        },
                    )
                    .unwrap()
            })
            .unwrap();
        assert!(contact_entities.contains(&reused));
        assert!(app.world().get::<Mesh3d>(reused).is_some());
        assert!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(reused)
                .is_some()
        );
        assert_eq!(
            app.world().get::<Visibility>(reused),
            Some(&Visibility::Visible)
        );
        assert!(app.world().get::<ProjectileHitMemory>(reused).is_none());
    }

    #[test]
    fn melee_target_can_leave_reach_during_windup() {
        let mut app = projectile_test_app();
        app.world_mut().resource_mut::<HeroVitals>().kind = HeroKind::IronBerserker;
        let hero = app.world_mut().spawn((Hero, Transform::default())).id();
        let target = app
            .world_mut()
            .spawn((test_enemy(100.0), Transform::from_xyz(1.0, 0.0, 0.0)))
            .id();
        app.world_mut().spawn(PendingSwing {
            hero,
            target,
            kind: HeroKind::IronBerserker,
            direction: Vec3::X,
            damage: 10.0,
            hit_limit: 1,
            remaining: 0.12,
        });
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(50));
        app.world_mut()
            .run_system_once(basic_attacks::resolve_swings)
            .unwrap();
        assert_eq!(
            app.world_mut()
                .query::<&PendingSwing>()
                .iter(app.world())
                .count(),
            1
        );
        app.world_mut()
            .get_mut::<Transform>(target)
            .unwrap()
            .translation
            .x = 6.0;
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(100));
        app.world_mut()
            .run_system_once(basic_attacks::resolve_swings)
            .unwrap();
        assert_eq!(
            app.world_mut()
                .query::<&PendingSwing>()
                .iter(app.world())
                .count(),
            0
        );
        assert_eq!(
            app.world_mut()
                .query::<&Projectile>()
                .iter(app.world())
                .count(),
            0
        );
        assert_eq!(app.world().get::<Enemy>(target).unwrap().hp, 100.0);
    }

    #[test]
    fn auto_attack_catches_up_every_elapsed_volley_and_preserves_timer_remainder() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<EnemyAttackCue>()
            .init_resource::<GameSession>()
            .init_resource::<HeroVitals>()
            .init_resource::<PlayerBuild>()
            .init_resource::<MarketEffects>()
            .init_resource::<BondRuntime>()
            .init_resource::<RunRng>()
            .init_resource::<CombatPools>()
            .init_resource::<AttackClock>()
            .init_resource::<AttackVolleyRuntime>()
            .init_resource::<BasicAttackCounter>()
            .insert_resource(test_game_assets());
        {
            let mut build = app.world_mut().resource_mut::<PlayerBuild>();
            build.attacks_per_second = 2.0;
            build.multishot = 2;
        }
        app.world_mut().spawn((Hero, Transform::default()));
        app.world_mut()
            .spawn((test_enemy(100.0), Transform::from_xyz(2.0, 0.0, 0.0)));
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(1_750));

        app.world_mut()
            .run_system_once(auto_attack)
            .expect("large-delta auto attack should run");

        assert_eq!(app.world().resource::<BasicAttackCounter>().0, 3);
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<Projectile>>()
                .iter(app.world())
                .count(),
            6,
            "three elapsed volleys with two shots each must all be emitted"
        );
        assert!(
            (app.world().resource::<AttackClock>().0.elapsed_secs() - 0.25).abs() < 0.000_001,
            "the repeating timer must retain the quarter-second remainder"
        );
    }

    #[test]
    fn absorbed_melee_strikes_at_ranged_target_without_counting_another_basic_attack() {
        let mut app = projectile_test_app();
        app.init_resource::<BasicAttackCounter>();
        app.world_mut().resource_mut::<HeroVitals>().kind = HeroKind::StormSavant;
        let hero = app.world_mut().spawn((Hero, Transform::default())).id();
        let target = app
            .world_mut()
            .spawn((test_enemy(1000.0), Transform::from_xyz(5.0, 0.0, 0.0)))
            .id();
        app.world_mut().spawn((
            super::super::hero_fusion::FusionAttack {
                kind: HeroKind::StoneWarden,
                origin: Vec3::new(4.4, 0.0, 0.0),
            },
            PendingSwing {
                hero,
                target,
                kind: HeroKind::StoneWarden,
                direction: Vec3::X,
                damage: 45.0,
                hit_limit: 3,
                remaining: 0.0,
            },
        ));
        app.world_mut()
            .run_system_once(basic_attacks::resolve_swings)
            .unwrap();
        let contacts = app
            .world_mut()
            .query_filtered::<Entity, With<super::super::hero_fusion::FusionAttack>>()
            .iter(app.world())
            .count();
        assert_eq!(
            contacts, 1,
            "ranged host must emit the old sword's actual contact"
        );
        app.world_mut().run_system_once(projectile_hits).unwrap();
        assert!(app.world().get::<Enemy>(target).unwrap().hp < 1000.0);
        assert_eq!(
            app.world().resource::<BasicAttackCounter>().0,
            0,
            "a legacy cannot recursively advance the primary attack cadence"
        );
    }

    #[test]
    fn bamboo_bonus_is_a_real_weapon_volley_without_advancing_clock_or_recursing() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<GameSession>()
            .init_resource::<HeroVitals>()
            .init_resource::<PlayerBuild>()
            .init_resource::<MarketEffects>()
            .init_resource::<RunRng>()
            .init_resource::<CombatPools>()
            .init_resource::<AttackClock>()
            .init_resource::<AttackVolleyRuntime>()
            .init_resource::<BasicAttackCounter>()
            .insert_resource(test_game_assets())
            .add_plugins(crate::game::bonds::BondsPlugin);
        app.world_mut()
            .resource_mut::<PlayerBuild>()
            .owned_bond_cards[crate::game::bond_catalog::BondCardId::BambooCloudSword.index()] =
            true;
        app.update();
        app.world_mut().resource_mut::<HeroVitals>().kind = HeroKind::EmberRanger;
        app.world_mut()
            .resource_mut::<PlayerBuild>()
            .attacks_per_second = 2.0;
        let seed = (0..10000)
            .find(|seed| RunRng::from_seed(*seed).unit() < 0.15)
            .unwrap();
        app.insert_resource(RunRng::from_seed(seed));
        app.world_mut().spawn((Hero, Transform::default()));
        app.world_mut()
            .spawn((test_enemy(1000.0), Transform::from_xyz(2.0, 0.0, 0.0)));
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(500));
        app.world_mut().run_system_once(auto_attack).unwrap();
        assert_eq!(
            app.world()
                .resource::<AttackVolleyRuntime>()
                .resolved_this_frame,
            2
        );
        assert_eq!(app.world().resource::<BasicAttackCounter>().0, 2);
        assert_eq!(app.world().resource::<AttackVolleyRuntime>().pending, 0);
        assert!(app.world().resource::<AttackClock>().0.elapsed_secs() < 0.001);
        let flights: Vec<_> = app
            .world_mut()
            .query::<(&Projectile, &WeaponFlight)>()
            .iter(app.world())
            .collect();
        assert_eq!(
            flights.len(),
            2,
            "one native arrow plus one arrow in exactly one nonrecursive bonus volley"
        );
        assert!(
            flights
                .iter()
                .all(|(p, _)| p.damage == app.world().resource::<PlayerBuild>().attack_damage)
        );
    }

    #[test]
    fn accelerated_catch_up_volleys_receive_distinct_first_frame_travel_ages() {
        let mut app = projectile_test_app();
        app.init_resource::<AttackClock>()
            .init_resource::<AttackVolleyRuntime>()
            .init_resource::<BasicAttackCounter>();
        {
            let mut build = app.world_mut().resource_mut::<PlayerBuild>();
            build.attacks_per_second = 5.0;
            build.projectile_speed = 10.0;
        }
        app.world_mut()
            .resource_mut::<AttackClock>()
            .0
            .set_elapsed(std::time::Duration::from_millis(190));
        app.world_mut().spawn((Hero, Transform::default()));
        app.world_mut()
            .spawn((test_enemy(100.0), Transform::from_xyz(2.0, 0.0, 0.0)));
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(750));

        app.world_mut()
            .run_system_once(auto_attack)
            .expect("accelerated catch-up attack should run");
        assert_eq!(app.world().resource::<BasicAttackCounter>().0, 4);
        let volley_runtime = app.world().resource::<AttackVolleyRuntime>();
        assert_eq!(volley_runtime.resolved_this_frame, 4);
        assert_eq!(volley_runtime.pending, 0);
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<ProjectileFirstFrameTravel>>()
                .iter(app.world())
                .count(),
            4
        );

        app.world_mut()
            .run_system_once(move_projectiles)
            .expect("catch-up projectile movement should run");
        let mut positions: Vec<f32> = app
            .world_mut()
            .query_filtered::<&Transform, With<Projectile>>()
            .iter(app.world())
            .map(|transform| transform.translation.x)
            .collect();
        positions.sort_by(f32::total_cmp);
        let expected = [1.4, 3.4, 5.4, 7.4];
        assert_eq!(positions.len(), expected.len());
        for (actual, expected) in positions.into_iter().zip(expected) {
            assert!((actual - expected).abs() < 0.000_01);
        }
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<ProjectileFirstFrameTravel>>()
                .iter(app.world())
                .count(),
            4,
            "the allowance remains available to swept hit resolution"
        );
        app.world_mut()
            .run_system_once(projectile_hits)
            .expect("catch-up swept hits should run");
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<ProjectileFirstFrameTravel>>()
                .iter(app.world())
                .count(),
            0,
            "hit resolution removes allowance state only from catch-up shells"
        );
    }

    #[test]
    fn single_volley_keeps_legacy_full_frame_projectile_travel() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<EnemyAttackCue>()
            .init_resource::<GameSession>()
            .init_resource::<HeroVitals>()
            .init_resource::<PlayerBuild>()
            .init_resource::<MarketEffects>()
            .init_resource::<BondRuntime>()
            .init_resource::<RunRng>()
            .init_resource::<CombatPools>()
            .init_resource::<AttackClock>()
            .init_resource::<AttackVolleyRuntime>()
            .init_resource::<BasicAttackCounter>()
            .insert_resource(test_game_assets());
        {
            let mut build = app.world_mut().resource_mut::<PlayerBuild>();
            build.attacks_per_second = 2.0;
            build.projectile_speed = 10.0;
        }
        app.world_mut().spawn((Hero, Transform::default()));
        app.world_mut()
            .spawn((test_enemy(100.0), Transform::from_xyz(2.0, 0.0, 0.0)));
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(500));

        app.world_mut()
            .run_system_once(auto_attack)
            .expect("single attack should run");
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<ProjectileFirstFrameTravel>>()
                .iter(app.world())
                .count(),
            0
        );
        app.world_mut()
            .run_system_once(move_projectiles)
            .expect("legacy projectile movement should run");
        let transform = app
            .world_mut()
            .query_filtered::<&Transform, With<Projectile>>()
            .single(app.world())
            .expect("one projectile should exist");
        assert!((transform.translation.x - 5.0).abs() < 0.000_001);
    }

    #[test]
    fn attack_catch_up_backlog_carries_every_volley_without_dropping_over_cap_work() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<EnemyAttackCue>()
            .init_resource::<GameSession>()
            .init_resource::<HeroVitals>()
            .init_resource::<PlayerBuild>()
            .init_resource::<MarketEffects>()
            .init_resource::<BondRuntime>()
            .init_resource::<RunRng>()
            .init_resource::<CombatPools>()
            .init_resource::<AttackClock>()
            .init_resource::<AttackVolleyRuntime>()
            .init_resource::<BasicAttackCounter>()
            .insert_resource(test_game_assets());
        app.world_mut()
            .resource_mut::<PlayerBuild>()
            .attacks_per_second = 20.0;
        app.world_mut()
            .resource_mut::<AttackClock>()
            .0
            .set_elapsed(std::time::Duration::from_millis(49));
        app.world_mut().spawn((Hero, Transform::default()));
        app.world_mut()
            .spawn((test_enemy(100.0), Transform::from_xyz(2.0, 0.0, 0.0)));
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(1));

        app.world_mut()
            .run_system_once(auto_attack)
            .expect("first capped attack frame should run");
        {
            let runtime = app.world().resource::<AttackVolleyRuntime>();
            assert_eq!(runtime.resolved_this_frame, MAX_ATTACK_CATCH_UP_VOLLEYS);
            assert_eq!(runtime.pending, 12);
        }

        app.world_mut()
            .resource_mut::<PlayerBuild>()
            .attacks_per_second = 0.2;
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(10));
        app.world_mut()
            .run_system_once(auto_attack)
            .expect("second backlog frame should run");
        {
            let runtime = app.world().resource::<AttackVolleyRuntime>();
            assert_eq!(runtime.resolved_this_frame, MAX_ATTACK_CATCH_UP_VOLLEYS);
            assert_eq!(runtime.pending, 4);
        }

        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(10));
        app.world_mut()
            .run_system_once(auto_attack)
            .expect("final backlog frame should run");
        let runtime = app.world().resource::<AttackVolleyRuntime>();
        assert_eq!(runtime.resolved_this_frame, 4);
        assert_eq!(runtime.pending, 0);
        assert_eq!(app.world().resource::<BasicAttackCounter>().0, 20);
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<Projectile>>()
                .iter(app.world())
                .count(),
            20
        );
    }

    #[test]
    fn attack_backlog_is_discarded_when_no_valid_target_exists() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<EnemyAttackCue>()
            .init_resource::<GameSession>()
            .init_resource::<HeroVitals>()
            .init_resource::<PlayerBuild>()
            .init_resource::<MarketEffects>()
            .init_resource::<BondRuntime>()
            .init_resource::<RunRng>()
            .init_resource::<CombatPools>()
            .init_resource::<AttackClock>()
            .insert_resource(AttackVolleyRuntime {
                pending: 7,
                resolved_this_frame: 3,
            })
            .init_resource::<BasicAttackCounter>()
            .insert_resource(test_game_assets());
        app.world_mut().spawn((Hero, Transform::default()));
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(10));

        app.world_mut()
            .run_system_once(auto_attack)
            .expect("targetless attack frame should run");

        let runtime = app.world().resource::<AttackVolleyRuntime>();
        assert_eq!(runtime.resolved_this_frame, 0);
        assert_eq!(runtime.pending, 0);
        assert_eq!(app.world().resource::<BasicAttackCounter>().0, 0);
    }

    #[test]
    fn swept_segment_reports_entry_progress_and_handles_stationary_projectiles() {
        let start = Vec3::ZERO;
        let end = Vec3::new(10.0, 0.0, 0.0);
        let entry = segment_sphere_hit_progress(start, end, Vec3::new(5.0, 0.0, 0.0), 0.5)
            .expect("centered segment should cross the sphere");
        assert!((entry - 0.45).abs() < 0.000_001);
        assert_eq!(
            segment_sphere_hit_progress(start, end, Vec3::new(5.0, 1.0, 0.0), 0.5,),
            None
        );
        assert_eq!(
            segment_sphere_hit_progress(Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, 0.1),
            Some(0.0)
        );
        assert_eq!(
            segment_sphere_hit_progress(Vec3::ZERO, Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), 0.1,),
            None
        );
    }

    #[test]
    fn expired_projectile_resolves_its_final_sweep_before_pool_retirement() {
        let mut app = projectile_test_app();
        let mut enemy_data = test_enemy(100.0);
        enemy_data.elite = false;
        let enemy = app
            .world_mut()
            .spawn((enemy_data, Transform::from_xyz(4.0, 0.0, 0.0)))
            .id();
        let projectile = app
            .world_mut()
            .run_system_once(|mut commands: Commands, mut pools: ResMut<CombatPools>| {
                pools
                    .rent_projectile(
                        &mut commands,
                        Handle::default(),
                        Handle::default(),
                        Transform::default(),
                        Projectile {
                            velocity: Vec3::X * 20.0,
                            damage: 37.0,
                            radius: 0.1,
                            remaining_hits: 0,
                            lifetime: 0.4,
                        },
                    )
                    .expect("test projectile should rent")
            })
            .expect("projectile fixture should run");
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(500));

        app.world_mut()
            .run_system_once(move_projectiles)
            .expect("large projectile step should run");
        assert_eq!(
            app.world()
                .get::<Transform>(projectile)
                .unwrap()
                .translation
                .x,
            8.0,
            "travel is clamped to the remaining 0.4-second lifetime"
        );
        assert!(app.world().get::<Projectile>(projectile).is_some());

        app.world_mut()
            .run_system_once(projectile_hits)
            .expect("expired projectile final sweep should resolve");
        assert_eq!(app.world().get::<Enemy>(enemy).unwrap().hp, 63.0);
        assert!(app.world().get::<Projectile>(projectile).is_none());
        assert_eq!(
            app.world()
                .resource::<CombatPools>()
                .projectile_metrics()
                .active(),
            0
        );
    }

    #[test]
    fn swept_projectile_hits_nearest_enemy_first_even_when_spawned_last() {
        let mut app = projectile_test_app();
        let mut far_data = test_enemy(100.0);
        far_data.elite = false;
        let far = app
            .world_mut()
            .spawn((far_data, Transform::from_xyz(7.0, 0.0, 0.0)))
            .id();
        let mut near_data = test_enemy(100.0);
        near_data.elite = false;
        let near = app
            .world_mut()
            .spawn((near_data, Transform::from_xyz(3.0, 0.0, 0.0)))
            .id();
        app.world_mut()
            .run_system_once(|mut commands: Commands, mut pools: ResMut<CombatPools>| {
                pools
                    .rent_projectile(
                        &mut commands,
                        Handle::default(),
                        Handle::default(),
                        Transform::default(),
                        Projectile {
                            velocity: Vec3::X * 20.0,
                            damage: 25.0,
                            radius: 0.1,
                            remaining_hits: 0,
                            lifetime: 2.0,
                        },
                    )
                    .expect("test projectile should rent");
            })
            .expect("projectile fixture should run");
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(500));

        app.world_mut()
            .run_system_once(move_projectiles)
            .expect("projectile movement should run");
        app.world_mut()
            .run_system_once(projectile_hits)
            .expect("ordered swept collision should run");

        assert_eq!(app.world().get::<Enemy>(near).unwrap().hp, 75.0);
        assert_eq!(app.world().get::<Enemy>(far).unwrap().hp, 100.0);
    }

    #[test]
    fn announced_rush_enters_as_one_real_group_and_never_repeats_on_the_same_frame() {
        let mut app = wave_test_app(199.999, 4);
        app.init_resource::<RaidWarning>();
        app.world_mut().run_system_once(spawn_wave).unwrap();
        let mut living = app
            .world_mut()
            .query_filtered::<Entity, (With<Enemy>, Without<BossEnemy>)>();
        assert_eq!(living.iter(app.world()).count(), 5);
        assert!(app.world().resource::<RaidWarning>().remaining_secs > 0.0);
        assert_eq!(app.world().resource::<RaidWarning>().count, 9);
        app.world_mut().resource_mut::<GameSession>().elapsed = 200.0;
        app.world_mut().run_system_once(spawn_wave).unwrap();
        assert_eq!(living.iter(app.world()).count(), 14);
        assert_eq!(app.world().resource::<RaidWarning>().remaining_secs, 0.0);
        app.world_mut().run_system_once(spawn_wave).unwrap();
        assert_eq!(living.iter(app.world()).count(), 14);
        app.world_mut().resource_mut::<GameSession>().elapsed = 221.0;
        app.world_mut().run_system_once(spawn_wave).unwrap();
        assert_eq!(app.world().resource::<RaidWarning>().remaining_secs, 3.0);
        app.world_mut().resource_mut::<GameSession>().paused = true;
        let count = living.iter(app.world()).count();
        app.world_mut().run_system_once(spawn_wave).unwrap();
        assert_eq!(living.iter(app.world()).count(), count);
        assert_eq!(app.world().resource::<RaidWarning>().remaining_secs, 3.0);
    }

    #[test]
    fn checkpoint_boss_spawns_exactly_on_time_with_130_regular_shells_active() {
        let mut app = wave_test_app(BOSSES[0].spawn_seconds - 0.001, 5);
        app.world_mut()
            .run_system_once(|mut commands: Commands, mut pools: ResMut<CombatPools>| {
                pools
                    .rent_enemy_scene_group(
                        &mut commands,
                        EnemySceneKey::Regular(0),
                        Handle::default(),
                        &vec![Transform::default(); MAX_REGULAR_ENEMIES],
                    )
                    .expect("130 ordinary shells should fit")
            })
            .expect("ordinary pressure setup should run");
        {
            let mut runtime = app.world_mut().resource_mut::<WaveSpawnRuntime>();
            runtime.wave = 5;
            runtime.spawned = WAVES[4].count;
            runtime.accumulator = 0.0;
        }

        app.world_mut()
            .run_system_once(spawn_wave)
            .expect("pre-boundary wave system should run");
        assert!(!app.world().resource::<SpawnDirector>().boss_spawned[0]);
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<BossEnemy>>()
                .iter(app.world())
                .count(),
            0
        );

        app.world_mut().resource_mut::<GameSession>().elapsed = BOSSES[0].spawn_seconds;
        app.world_mut()
            .run_system_once(spawn_wave)
            .expect("checkpoint wave system should run");
        assert!(app.world().resource::<SpawnDirector>().boss_spawned[0]);
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<BossEnemy>>()
                .iter(app.world())
                .count(),
            1
        );
        let pools = app.world().resource::<CombatPools>();
        assert_eq!(pools.regular_enemy_active_count(), MAX_REGULAR_ENEMIES);
        assert_eq!(pools.enemy_metrics().active(), MAX_REGULAR_ENEMIES + 1);
    }

    #[test]
    fn failed_regular_pool_rent_keeps_the_front_wave_budget() {
        let mut app = wave_test_app(1.0, 1);
        app.world_mut()
            .run_system_once(|mut commands: Commands, mut pools: ResMut<CombatPools>| {
                pools
                    .rent_enemy_scene_group(
                        &mut commands,
                        EnemySceneKey::BossSummon(0),
                        Handle::default(),
                        &vec![Transform::default(); super::super::pooling::ENEMY_POOL_ACTIVE_CAP],
                    )
                    .expect("global pool should fill for the pressure test")
            })
            .expect("global pressure setup should run");
        let budget = SpawnBudget {
            wave: WAVES[0],
            archetype: 0,
            elite: false,
            hunted: false,
            affix: None,
            spawn_x: 15.75,
            spawn_z: 1.25,
            scale_jitter: 1.0,
            speed_jitter: 1.0,
        };
        {
            let mut runtime = app.world_mut().resource_mut::<WaveSpawnRuntime>();
            runtime.wave = 1;
            runtime.spawned = WAVES[0].count;
            runtime.accumulator = 0.0;
            runtime.backlog.push_back(budget);
        }

        app.world_mut()
            .run_system_once(spawn_wave)
            .expect("blocked wave system should run");
        let runtime = app.world().resource::<WaveSpawnRuntime>();
        assert_eq!(runtime.backlog.len(), 1);
        assert_eq!(
            runtime.backlog.front().map(|entry| entry.spawn_x),
            Some(15.75)
        );
        assert_eq!(
            app.world()
                .resource::<CombatPools>()
                .enemy_metrics()
                .rejected(),
            1
        );
    }

    #[test]
    fn splitling_budget_waits_at_full_capacity_then_merges_into_one_released_slot() {
        let mut app = wave_test_app(1.0, 1);
        let filled = app
            .world_mut()
            .run_system_once(|mut commands: Commands, mut pools: ResMut<CombatPools>| {
                pools
                    .rent_enemy_scene_group(
                        &mut commands,
                        EnemySceneKey::BossSummon(0),
                        Handle::default(),
                        &vec![Transform::default(); super::super::pooling::ENEMY_POOL_ACTIVE_CAP],
                    )
                    .expect("global active capacity should fill atomically")
            })
            .expect("full-capacity fixture should run");

        app.world_mut()
            .run_system_once(
                |mut commands: Commands,
                 mut pools: ResMut<CombatPools>,
                 assets: Res<GameAssets>| {
                    enqueue_or_spawn_splitlings(
                        &mut commands,
                        &mut pools,
                        &assets,
                        Vec3::new(3.0, 0.0, -2.0),
                        3,
                    );
                },
            )
            .expect("blocked splitling enqueue should run");
        assert_eq!(
            app.world()
                .resource::<CombatPools>()
                .pending_splitling_groups(),
            1
        );
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<EnemyArchetype>>()
                .iter(app.world())
                .count(),
            0,
            "all-or-none failure must not create a partial splitling group"
        );

        app.world_mut()
            .run_system_once(
                move |mut commands: Commands, mut pools: ResMut<CombatPools>| {
                    assert!(pools.retire_enemy(&mut commands, filled[0]));
                },
            )
            .expect("one physical slot should be released");
        app.world_mut()
            .run_system_once(
                |mut commands: Commands,
                 mut pools: ResMut<CombatPools>,
                 assets: Res<GameAssets>| {
                    drain_splitling_backlog(&mut commands, &mut pools, &assets);
                },
            )
            .expect("splitling backlog drain should run");

        assert_eq!(
            app.world()
                .resource::<CombatPools>()
                .pending_splitling_groups(),
            0
        );
        let mut splitlings = app.world_mut().query_filtered::<(
            &Enemy,
            &EnemyArchetype,
            &EnemyBreachDamage,
            &Name,
        ), Without<BossEnemy>>();
        let entries: Vec<_> = splitlings.iter(app.world()).collect();
        assert_eq!(entries.len(), 1);
        let (enemy, archetype, breach, name) = entries[0];
        assert_eq!(archetype.0, 1);
        assert_eq!(enemy.hp, 66.0);
        assert_eq!(enemy.max_hp, 66.0);
        assert_eq!(enemy.damage, 9.0);
        assert_eq!(breach.0, 12.0);
        assert_eq!(name.as_str(), "SPLITLING · MERGED x3");
        assert_eq!(
            app.world()
                .resource::<CombatPools>()
                .enemy_metrics()
                .active(),
            super::super::pooling::ENEMY_POOL_ACTIVE_CAP
        );
    }

    #[test]
    fn forged_bloodreaver_really_increases_armor_damage_and_inherited_lifesteal() {
        let mut app = projectile_test_app();
        let (build, mut vitals, effects) =
            super::super::market::purchased_attribute_equipment_for_test();
        vitals.health = vitals.max_health - 100.0;
        let before = vitals.health;
        app.insert_resource(build)
            .insert_resource(vitals)
            .insert_resource(effects)
            .init_resource::<super::super::crafting::CraftingState>()
            .add_plugins(super::super::bonds::BondsPlugin);
        assert!(super::super::crafting::craft_recipe(
            app.world_mut(),
            super::super::crafting::RecipeId::Bloodreaver
        ));
        app.update();
        let enemy = app
            .world_mut()
            .spawn((
                Enemy {
                    hp: 1000.0,
                    max_hp: 1000.0,
                    speed: 0.0,
                    damage: 0.0,
                    xp: 0,
                    wood: 0,
                    gold: 0,
                    kill_credit: 0,
                    elite: false,
                },
                EnemyArmor(8.0),
                Transform::default(),
            ))
            .id();
        app.world_mut().spawn((
            Projectile {
                velocity: Vec3::ZERO,
                damage: 100.0,
                radius: 0.3,
                remaining_hits: 0,
                lifetime: 1.0,
            },
            Transform::default(),
            DamageProfile::PHYSICAL_ATTACK,
        ));
        let mut rng = *app.world().resource::<RunRng>();
        let raw = 100.0
            * critical_multiplier(
                app.world().resource::<BondRuntime>(),
                false,
                DamageProfile::PHYSICAL_ATTACK,
                rng.unit(),
            );
        app.world_mut().run_system_once(projectile_hits).unwrap();
        let actual = 1000.0 - app.world().get::<Enemy>(enemy).unwrap().hp;
        assert!((actual - physical_damage_after_armor(raw, 2.0, false)).abs() < 0.001);
        assert!(actual > physical_damage_after_armor(raw, 5.0, false));
        assert!(
            (app.world().resource::<HeroVitals>().health - before - actual * 0.10).abs() < 0.001
        );
    }

    #[test]
    fn purchased_penetration_and_leech_change_real_armored_and_resistant_target_health() {
        for profile in [DamageProfile::PHYSICAL_ATTACK, DamageProfile::for_skill(3)] {
            let mut app = projectile_test_app();
            let (build, mut vitals, effects) =
                super::super::market::purchased_attribute_equipment_for_test();
            vitals.health = vitals.max_health - 100.0;
            let health_before = vitals.health;
            app.insert_resource(build)
                .insert_resource(vitals)
                .insert_resource(effects)
                .add_plugins(super::super::bonds::BondsPlugin);
            app.update();
            let enemy = app
                .world_mut()
                .spawn((
                    Enemy {
                        hp: 1000.0,
                        max_hp: 1000.0,
                        speed: 0.0,
                        damage: 0.0,
                        xp: 0,
                        wood: 0,
                        gold: 0,
                        kill_credit: 0,
                        elite: false,
                    },
                    EnemyArmor(8.0),
                    BossEnemy { index: 2 },
                    Transform::default(),
                ))
                .id();
            app.world_mut().spawn((
                Projectile {
                    velocity: Vec3::ZERO,
                    damage: 100.0,
                    radius: 0.3,
                    remaining_hits: 0,
                    lifetime: 1.0,
                },
                Transform::default(),
                profile,
            ));
            let mut rng = *app.world().resource::<RunRng>();
            let crit = critical_multiplier(
                app.world().resource::<BondRuntime>(),
                false,
                profile,
                rng.unit(),
            );
            let raw = 100.0 * crit;
            let resistance = super::super::bonds::boss_damage_multiplier_with_penetration(
                profile,
                BOSSES[2].modifiers,
                0.10,
            );
            let expected = if profile.element == DamageElement::Physical {
                physical_damage_after_armor(raw * resistance, 5.0, false)
            } else {
                raw * resistance
            };
            app.world_mut().run_system_once(projectile_hits).unwrap();
            let actual = 1000.0 - app.world().get::<Enemy>(enemy).unwrap().hp;
            assert!((actual - expected).abs() < 0.001, "{actual} != {expected}");
            assert!(
                (app.world().resource::<HeroVitals>().health - health_before - actual * 0.05).abs()
                    < 0.001
            );
            let without_penetration = if profile.element == DamageElement::Physical {
                physical_damage_after_armor(raw * resistance, 8.0, false)
            } else {
                raw * super::super::bonds::boss_damage_multiplier(profile, BOSSES[2].modifiers)
            };
            assert!(
                actual > without_penetration,
                "equipped penetration must increase real health loss"
            );
        }
    }

    #[test]
    fn expanded_bond_damage_distance_and_lifesteal_reach_actual_hp() {
        let mut app = projectile_test_app();
        app.add_plugins(super::super::bonds::BondsPlugin);
        for card in [
            super::super::BondCardId::JungleKnife,
            super::super::BondCardId::BloodFeast,
            super::super::BondCardId::ArcherTalent,
            super::super::BondCardId::WindFootwork,
            super::super::BondCardId::WindQuiver,
            super::super::BondCardId::WindAim,
        ] {
            app.world_mut()
                .resource_mut::<PlayerBuild>()
                .owned_bond_cards[card.index()] = true;
        }
        app.update();
        app.world_mut().resource_mut::<HeroVitals>().health = 100.;
        app.world_mut()
            .spawn((Hero, Transform::from_xyz(-12., 0., 0.)));
        let enemy = app
            .world_mut()
            .spawn((
                Enemy {
                    hp: 1000.,
                    max_hp: 1000.,
                    speed: 0.,
                    damage: 0.,
                    xp: 0,
                    wood: 0,
                    gold: 0,
                    kill_credit: 0,
                    elite: false,
                },
                Transform::default(),
            ))
            .id();
        app.world_mut().spawn((
            Projectile {
                velocity: Vec3::ZERO,
                damage: 100.,
                radius: 0.3,
                remaining_hits: 0,
                lifetime: 1.,
            },
            Transform::default(),
            DamageProfile::PHYSICAL_ATTACK,
        ));
        let mut rng = *app.world().resource::<RunRng>();
        let crit = critical_multiplier(
            app.world().resource::<BondRuntime>(),
            false,
            DamageProfile::PHYSICAL_ATTACK,
            rng.unit(),
        );
        let expected = 100. * 1.1 * 1.3 * crit;
        app.world_mut().run_system_once(projectile_hits).unwrap();
        let actual = 1000. - app.world().get::<Enemy>(enemy).unwrap().hp;
        assert!(
            (actual - expected).abs() < 0.01,
            "actual {actual}, expected {expected}"
        );
        assert!(
            (app.world().resource::<HeroVitals>().health - (100. + actual * 0.03)).abs() < 0.01
        );
    }

    #[test]
    fn logic_only_projectile_still_runs_the_authoritative_damage_resolver() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<EnemyAttackCue>()
            .init_resource::<GameSession>()
            .init_resource::<DamageMeter>()
            .init_resource::<ArchiveState>()
            .init_resource::<TrialState>()
            .init_resource::<GoldChallengeState>()
            .init_resource::<PlayerBuild>()
            .init_resource::<HeroVitals>()
            .init_resource::<RunRng>()
            .init_resource::<BondRuntime>()
            .init_resource::<SkillRuntime>()
            .init_resource::<MarketEffects>()
            .init_resource::<BondRuntime>()
            .init_resource::<RunRng>()
            .init_resource::<EncounterState>()
            .init_resource::<CombatPools>()
            .insert_resource(test_game_assets());
        let enemy = app
            .world_mut()
            .spawn((
                Enemy {
                    hp: 1_000.0,
                    max_hp: 1_000.0,
                    speed: 0.0,
                    damage: 0.0,
                    xp: 0,
                    wood: 0,
                    gold: 0,
                    kill_credit: 0,
                    elite: false,
                },
                Transform::from_xyz(2.0, 0.0, 1.0),
            ))
            .id();
        let overflow = app
            .world_mut()
            .run_system_once(|mut commands: Commands, mut pools: ResMut<CombatPools>| {
                for index in 0..super::super::pooling::PROJECTILE_POOL_CAPACITY {
                    assert!(
                        pools
                            .rent_projectile(
                                &mut commands,
                                Handle::default(),
                                Handle::default(),
                                Transform::from_xyz(100.0, 0.0, index as f32),
                                Projectile {
                                    velocity: Vec3::ZERO,
                                    damage: 1.0,
                                    radius: 0.1,
                                    remaining_hits: 0,
                                    lifetime: 10.0,
                                },
                            )
                            .is_some()
                    );
                }
                let entity = pools
                    .rent_projectile(
                        &mut commands,
                        Handle::default(),
                        Handle::default(),
                        Transform::from_xyz(2.0, 0.0, 1.0),
                        Projectile {
                            velocity: Vec3::ZERO,
                            damage: 37.0,
                            radius: 0.3,
                            remaining_hits: 0,
                            lifetime: 1.0,
                        },
                    )
                    .expect("257th logical projectile must exist");
                commands
                    .entity(entity)
                    .insert(DamageProfile::new(DamageElement::Storm, DamageClass::Spell));
                entity
            })
            .expect("overflow projectile fixture should run");
        assert!(app.world().get::<Mesh3d>(overflow).is_none());
        assert!(app.world().get::<DamageProfile>(overflow).is_some());

        app.world_mut()
            .run_system_once(projectile_hits)
            .expect("overflow projectile should resolve a hit");
        assert_eq!(
            app.world().get::<Enemy>(enemy).map(|enemy| enemy.hp),
            Some(963.0)
        );
        assert!(app.world().get::<Projectile>(overflow).is_none());
        let pools = app.world().resource::<CombatPools>();
        assert_eq!(pools.projectile_metrics().active(), 256);
        assert_eq!(pools.projectile_metrics().rejected(), 0);
        assert_eq!(pools.projectile_visual_fallbacks(), 1);
        assert_eq!(pools.damage_number_metrics().created(), 1);
        assert_eq!(pools.damage_number_metrics().active(), 1);
    }

    #[test]
    fn landed_hits_report_kill_weight_and_survivors_to_hit_feedback() {
        let mut app = projectile_test_app();
        app.init_resource::<HitFeedback>();
        let spawn_enemy = |app: &mut App, hp: f32, elite: bool, x: f32| {
            app.world_mut()
                .spawn((
                    Enemy {
                        hp,
                        max_hp: hp,
                        speed: 0.0,
                        damage: 0.0,
                        xp: 0,
                        wood: 0,
                        gold: 0,
                        kill_credit: 0,
                        elite,
                    },
                    Transform::from_xyz(x, 0.0, 0.0),
                ))
                .id()
        };
        let doomed_elite = spawn_enemy(&mut app, 10.0, true, 0.0);
        let survivor = spawn_enemy(&mut app, 1_000.0, false, 20.0);
        app.world_mut()
            .run_system_once(|mut commands: Commands, mut pools: ResMut<CombatPools>| {
                for x in [0.0, 20.0] {
                    pools.rent_projectile(
                        &mut commands,
                        Handle::default(),
                        Handle::default(),
                        Transform::from_xyz(x, 0.0, 0.0),
                        Projectile {
                            velocity: Vec3::ZERO,
                            damage: 50.0,
                            radius: 0.3,
                            remaining_hits: 0,
                            lifetime: 1.0,
                        },
                    );
                }
            })
            .expect("projectile fixture should run");
        app.world_mut()
            .run_system_once(projectile_hits)
            .expect("projectiles should resolve");

        let feedback = app.world().resource::<HitFeedback>();
        let find = |target| {
            *feedback
                .events()
                .iter()
                .find(|event| event.target == target)
                .expect("every landed hit is reported")
        };
        let kill = find(doomed_elite);
        assert!(kill.killed);
        assert_eq!(kill.weight, HitWeight::Elite);
        let hit = find(survivor);
        assert!(!hit.killed);
        assert_eq!(hit.weight, HitWeight::Normal);
    }

    #[test]
    fn finished_session_retires_every_active_enemy_shell() {
        let mut app = App::new();
        app.init_resource::<CombatPools>()
            .insert_resource(GameSession {
                finished: true,
                paused: true,
                ..default()
            });
        let enemy = app
            .world_mut()
            .run_system_once(|mut commands: Commands, mut pools: ResMut<CombatPools>| {
                let entity = pools
                    .rent_enemy_scene(
                        &mut commands,
                        EnemySceneKey::Boss(4),
                        Handle::default(),
                        Transform::default(),
                    )
                    .expect("cleanup fixture should fit");
                commands
                    .entity(entity)
                    .insert((test_enemy(10.0), BossEnemy { index: 4 }));
                entity
            })
            .expect("cleanup fixture should run");
        app.world_mut()
            .run_system_once(cleanup_finished_enemies)
            .expect("finished cleanup should run");
        assert_eq!(
            app.world()
                .resource::<CombatPools>()
                .enemy_metrics()
                .active(),
            0
        );
        assert!(app.world().get::<Enemy>(enemy).is_none());
        assert_eq!(
            app.world().get::<Visibility>(enemy),
            Some(&Visibility::Hidden)
        );
    }

    #[test]
    fn damage_number_lifetime_freezes_while_paused_and_archive_start_cleans_up() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<EnemyAttackCue>()
            .init_resource::<CombatPools>()
            .init_resource::<ArchiveState>()
            .insert_resource(GameSession {
                paused: true,
                ..default()
            });
        let target = Entity::from_bits(55);
        app.world_mut()
            .run_system_once(
                move |mut commands: Commands, mut pools: ResMut<CombatPools>| {
                    pools
                        .show_damage_number(
                            &mut commands,
                            target,
                            Vec3::ZERO,
                            20.0,
                            DamageElement::Frost,
                            false,
                        )
                        .expect("positive HP loss should show");
                },
            )
            .expect("damage-number fixture should run");
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(1.0));
        app.world_mut()
            .run_system_once(update_damage_numbers)
            .expect("paused number update should run");
        assert_eq!(
            app.world()
                .resource::<CombatPools>()
                .damage_number_metrics()
                .active(),
            1,
            "pause must freeze the 0.7-second visual lifetime"
        );

        app.world_mut().resource_mut::<GameSession>().paused = false;
        app.world_mut().resource_mut::<ArchiveState>().active = true;
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.01));
        app.world_mut()
            .run_system_once(update_damage_numbers)
            .expect("archive transition cleanup should run");
        assert_eq!(
            app.world()
                .resource::<CombatPools>()
                .damage_number_metrics()
                .active(),
            0
        );
    }

    #[test]
    fn finished_cleanup_retires_visible_and_logic_projectiles_and_damage_numbers() {
        let mut app = App::new();
        app.init_resource::<CombatPools>()
            .insert_resource(GameSession {
                finished: true,
                paused: true,
                ..default()
            });
        app.world_mut()
            .run_system_once(|mut commands: Commands, mut pools: ResMut<CombatPools>| {
                for index in 0..=super::super::pooling::PROJECTILE_POOL_CAPACITY {
                    pools
                        .rent_projectile(
                            &mut commands,
                            Handle::default(),
                            Handle::default(),
                            Transform::from_xyz(index as f32, 0.0, 0.0),
                            Projectile {
                                velocity: Vec3::ZERO,
                                damage: 1.0,
                                radius: 0.1,
                                remaining_hits: 0,
                                lifetime: 5.0,
                            },
                        )
                        .expect("logical projectile rent cannot reject");
                }
                pools
                    .show_damage_number(
                        &mut commands,
                        Entity::from_bits(8),
                        Vec3::ZERO,
                        5.0,
                        DamageElement::Physical,
                        false,
                    )
                    .expect("damage number should show");
            })
            .expect("finished cleanup fixture should run");
        assert_eq!(
            app.world()
                .resource::<CombatPools>()
                .projectile_metrics()
                .active(),
            257
        );
        app.world_mut()
            .run_system_once(cleanup_finished_enemies)
            .expect("finished combat cleanup should run");
        let pools = app.world().resource::<CombatPools>();
        assert_eq!(pools.projectile_metrics().active(), 0);
        assert_eq!(pools.projectile_metrics().rejected(), 0);
        assert_eq!(pools.damage_number_metrics().active(), 0);
    }

    fn test_budget(archetype: usize, elite: bool) -> SpawnBudget {
        SpawnBudget {
            wave: WAVES[0],
            archetype,
            elite,
            hunted: false,
            affix: elite.then_some(EliteAffixKind::Frenzy),
            spawn_x: 16.0,
            spawn_z: 0.0,
            scale_jitter: 1.0,
            speed_jitter: 1.0,
        }
    }

    #[test]
    fn storm_coil_selects_primary_then_two_nearest_targets_stably() {
        let primary = Entity::from_bits(10);
        let low_tie = Entity::from_bits(2);
        let high_tie = Entity::from_bits(3);
        let candidates = [
            (high_tie, Vec3::new(1.0, 0.0, 0.0)),
            (primary, Vec3::ZERO),
            (Entity::from_bits(4), Vec3::new(6.0, 0.0, 0.0)),
            (low_tie, Vec3::new(-1.0, 0.0, 0.0)),
        ];
        let targets = storm_coil_targets(primary, Vec3::ZERO, &candidates);
        assert_eq!(targets.len(), 3);
        assert_eq!(targets[0].0, primary);
        assert_eq!(targets[1].0, low_tie);
        assert_eq!(targets[2].0, high_tie);
    }

    #[test]
    fn serrated_splash_uses_three_meter_radius_and_caps_at_six_neighbors() {
        let source = Entity::from_bits(20);
        let mut candidates = vec![(source, Vec3::ZERO)];
        for index in 1..=7 {
            candidates.push((
                Entity::from_bits(index),
                Vec3::new(index as f32 * 0.4, 0.0, 0.0),
            ));
        }
        candidates.push((Entity::from_bits(30), Vec3::new(3.01, 0.0, 0.0)));
        let targets = splash_targets(source, Vec3::ZERO, &candidates);
        assert_eq!(targets.len(), 6);
        assert_eq!(
            targets
                .iter()
                .map(|(entity, _)| entity.to_bits())
                .collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 5, 6]
        );
    }

    #[test]
    fn damage_resolution_reports_only_actual_hp_loss_for_blood_leech() {
        let mut enemy = test_enemy(100.0);
        let mut affix = EliteAffix::new(EliteAffixKind::Bulwark);
        affix.shield = 25.0;
        let resolution = apply_enemy_damage(&mut enemy, Some(&mut affix), 40.0, None);
        assert_eq!(resolution.outcome, DamageOutcome::Alive);
        assert!((resolution.hp_damage - 15.0).abs() < 0.001);

        enemy.hp = 10.0;
        let lethal = apply_enemy_damage(&mut enemy, None, 999.0, None);
        assert_eq!(lethal.outcome, DamageOutcome::Defeated);
        assert!((lethal.hp_damage - 10.0).abs() < 0.001);
    }

    #[test]
    fn frost_mirror_uses_eighty_plus_half_effective_spell_power() {
        assert!((frost_mirror_damage(100.0) - 130.0).abs() < 0.001);
    }

    #[test]
    fn elite_affix_rolls_are_deterministic_and_cover_all_eight_kinds() {
        for (index, expected) in [
            EliteAffixKind::Frenzy,
            EliteAffixKind::Bulwark,
            EliteAffixKind::Volatile,
            EliteAffixKind::Regeneration,
            EliteAffixKind::Commander,
            EliteAffixKind::YellowSign,
            EliteAffixKind::Brood,
            EliteAffixKind::Siege,
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(elite_affix_from_roll((index as u32) * (1 << 29)), expected);
        }

        let mut first = RunRng::default();
        let mut second = RunRng::default();
        let first_sequence: Vec<_> = (0..24)
            .map(|_| elite_affix_from_roll(first.next_u32()))
            .collect();
        let second_sequence: Vec<_> = (0..24)
            .map(|_| elite_affix_from_roll(second.next_u32()))
            .collect();
        assert_eq!(first_sequence, second_sequence);
    }

    #[cfg(any(debug_assertions, feature = "dev-tools"))]
    #[test]
    fn debug_dispatch_spawns_every_runtime_enemy_affix_and_boss_component_set() {
        use super::debug_tools::{
            DEBUG_BOSS_COUNT, DEBUG_ELITE_AFFIX_COUNT, DEBUG_ENEMY_COUNT, DebugSpawnRequest,
            spawn_debug_request,
        };

        let mut world = World::new();
        let assets = test_game_assets();
        let mut pools = CombatPools::default();
        {
            let mut commands = world.commands();
            for archetype in 0..DEBUG_ENEMY_COUNT {
                spawn_debug_request(
                    &mut commands,
                    &mut pools,
                    &assets,
                    DebugSpawnRequest::Enemy {
                        archetype,
                        elite_affix: None,
                    },
                    1,
                );
            }
            for elite_affix in 0..DEBUG_ELITE_AFFIX_COUNT {
                spawn_debug_request(
                    &mut commands,
                    &mut pools,
                    &assets,
                    DebugSpawnRequest::Enemy {
                        archetype: 7,
                        elite_affix: Some(elite_affix),
                    },
                    1,
                );
            }
            for index in 0..DEBUG_BOSS_COUNT {
                spawn_debug_request(
                    &mut commands,
                    &mut pools,
                    &assets,
                    DebugSpawnRequest::Boss { index },
                    1,
                );
            }
        }
        world.flush();

        let mut regular_query = world.query_filtered::<(
            Entity,
            &Enemy,
            &EnemyArchetype,
            &EnemyArmor,
            &EnemyBreachDamage,
            Option<&EliteAffix>,
            &WorldAssetRoot,
        ), Without<BossEnemy>>();
        let regular: Vec<_> = regular_query.iter(&world).collect();
        assert_eq!(regular.len(), DEBUG_ENEMY_COUNT + DEBUG_ELITE_AFFIX_COUNT);
        for (archetype, definition) in ENEMIES.iter().enumerate() {
            let (entity, _, _, armor, breach, _, _) = regular
                .iter()
                .copied()
                .find(|(_, enemy, kind, _, _, affix, _)| {
                    !enemy.elite && affix.is_none() && usize::from(kind.0) == archetype
                })
                .expect("every normal archetype should use the runtime bundle");
            assert_eq!(armor.0, definition.armor);
            assert!(breach.0 > 0.0);
            if matches!(
                definition.behavior,
                EnemyBehavior::HuntHero | EnemyBehavior::BurrowAmbush
            ) {
                assert!(world.get::<EnemyHeroThreat>(entity).is_some());
            }
        }

        let expected_affixes = [
            EliteAffixKind::Frenzy,
            EliteAffixKind::Bulwark,
            EliteAffixKind::Volatile,
            EliteAffixKind::Regeneration,
            EliteAffixKind::Commander,
            EliteAffixKind::YellowSign,
            EliteAffixKind::Brood,
            EliteAffixKind::Siege,
        ];
        for expected in expected_affixes {
            assert!(regular.iter().any(|(_, enemy, kind, _, _, affix, _)| {
                enemy.elite
                    && kind.0
                        == if expected == EliteAffixKind::Brood {
                            9
                        } else {
                            7
                        }
                    && affix.is_some_and(|affix| affix.kind == expected)
            }));
        }
        drop(regular);

        let mut boss_query = world.query::<(
            &BossEnemy,
            &Enemy,
            &EnemyArmor,
            &EnemyBreachDamage,
            &WorldAssetRoot,
        )>();
        let bosses: Vec<_> = boss_query.iter(&world).collect();
        assert_eq!(bosses.len(), DEBUG_BOSS_COUNT);
        let expected = [
            (8_000.0, 0.85, 14.0),
            (28_000.0, 0.62, 20.0),
            (70_000.0, 0.92, 28.0),
            (160_000.0, 1.18, 38.0),
            (420_000.0, 0.78, 55.0),
        ];
        for (index, (hp, speed, damage)) in expected.into_iter().enumerate() {
            assert!(bosses.iter().any(|(boss, enemy, armor, breach, _)| {
                usize::from(boss.index) == index
                    && enemy.elite
                    && (enemy.hp, enemy.max_hp, enemy.speed, enemy.damage)
                        == (hp, hp, speed, damage)
                    && armor.0 == 0.0
                    && breach.0 == damage
            }));
        }
    }

    #[test]
    fn frenzy_and_commander_multipliers_do_not_stack_accidentally() {
        assert_eq!(
            elite_action_speed_multiplier(Some(EliteAffixKind::Frenzy)),
            1.35
        );
        assert_eq!(elite_action_speed_multiplier(None), 1.0);
        let state = CommanderAuraState {
            base_max_hp: 100.0,
            active: true,
        };
        assert_eq!(commander_aura_multiplier(Some(&state)), 1.2);
        assert!(within_commander_aura(
            Vec3::ZERO,
            Vec3::new(COMMANDER_AURA_RADIUS, 12.0, 0.0)
        ));
        assert!(!within_commander_aura(
            Vec3::ZERO,
            Vec3::new(COMMANDER_AURA_RADIUS + 0.01, 0.0, 0.0)
        ));
    }

    #[test]
    fn bulwark_refreshes_at_eight_seconds_and_absorbs_damage_first() {
        let mut enemy = test_enemy(100.0);
        let mut affix = EliteAffix::new(EliteAffixKind::Bulwark);
        assert!(!refresh_bulwark_shield(&enemy, &mut affix, 7.999));
        assert!(refresh_bulwark_shield(&enemy, &mut affix, 0.002));
        assert!((affix.shield - 25.0).abs() < 0.001);

        assert_eq!(
            apply_enemy_damage(&mut enemy, Some(&mut affix), 20.0, None).outcome,
            DamageOutcome::Alive
        );
        assert_eq!(enemy.hp, 100.0);
        assert!((affix.shield - 5.0).abs() < 0.001);
        assert_eq!(
            apply_enemy_damage(&mut enemy, Some(&mut affix), 12.0, None).outcome,
            DamageOutcome::Alive
        );
        assert_eq!(enemy.hp, 93.0);
        assert_eq!(affix.shield, 0.0);
    }

    #[test]
    fn regeneration_prevents_only_the_first_lethal_hit() {
        let mut enemy = test_enemy(10.0);
        let mut affix = EliteAffix::new(EliteAffixKind::Regeneration);
        assert_eq!(
            apply_enemy_damage(&mut enemy, Some(&mut affix), 20.0, None).outcome,
            DamageOutcome::Regenerated
        );
        assert!((enemy.hp - 35.0).abs() < 0.001);
        assert!(affix.regenerated);
        assert_eq!(
            apply_enemy_damage(&mut enemy, Some(&mut affix), 40.0, None).outcome,
            DamageOutcome::Defeated
        );
    }

    #[test]
    fn volatile_enters_one_warning_state_before_settlement() {
        let mut enemy = test_enemy(10.0);
        let mut affix = EliteAffix::new(EliteAffixKind::Volatile);
        assert_eq!(
            apply_enemy_damage(&mut enemy, Some(&mut affix), 20.0, None).outcome,
            DamageOutcome::VolatileStarted
        );
        assert_eq!(affix.volatile_remaining, VOLATILE_WARNING_SECONDS);
        assert!(enemy.hp > 0.0);
        assert_eq!(
            apply_enemy_damage(&mut enemy, Some(&mut affix), 999.0, None).outcome,
            DamageOutcome::Pending
        );
        assert_eq!(affix.volatile_remaining, VOLATILE_WARNING_SECONDS);
    }

    #[test]
    fn damage_meter_records_mitigated_non_overkill_damage_at_the_entry_point() {
        let mut enemy = test_enemy(10.0);
        let mut meter = DamageMeter::default();
        assert_eq!(
            apply_enemy_damage(&mut enemy, None, 999.0, Some(&mut meter)).outcome,
            DamageOutcome::Defeated
        );
        assert!((meter.recent_dps() - 1.0).abs() < 0.001);

        let mut shielded = test_enemy(100.0);
        let mut affix = EliteAffix::new(EliteAffixKind::Bulwark);
        affix.shield = 25.0;
        assert_eq!(
            apply_enemy_damage(&mut shielded, Some(&mut affix), 20.0, Some(&mut meter),).outcome,
            DamageOutcome::Alive
        );
        assert!((meter.recent_dps() - 3.0).abs() < 0.001);
    }

    #[test]
    fn command_insight_multiplier_is_consumed_by_enemy_xp_rewards() {
        assert_eq!(xp_with_efficiency(10, 1.0), 10);
        assert_eq!(xp_with_efficiency(10, 1.1), 11);
        assert_eq!(xp_with_efficiency(15, 1.1), 17);
    }

    #[test]
    fn backlog_wait_and_next_slot_boundaries_are_exact() {
        assert!(!backlog_merge_ready(7.999));
        assert!(backlog_merge_ready(8.0));
        assert_eq!(
            backlog_spawn_decision(1, MAX_REGULAR_ENEMIES - 1, false),
            BacklogSpawnDecision::Individuals(1)
        );
        assert_eq!(
            backlog_spawn_decision(3, MAX_REGULAR_ENEMIES, false),
            BacklogSpawnDecision::Hold
        );
        assert_eq!(
            backlog_spawn_decision(3, MAX_REGULAR_ENEMIES, true),
            BacklogSpawnDecision::Hold
        );
        assert_eq!(
            backlog_spawn_decision(3, MAX_REGULAR_ENEMIES - 1, true),
            BacklogSpawnDecision::Merged
        );
    }

    #[test]
    fn merged_spawn_preserves_total_health_breach_and_rewards() {
        let budgets = [test_budget(0, false), test_budget(1, true)];
        let total = spawn_budget_totals(&budgets);
        let first = ENEMIES[0];
        let second = ENEMIES[1];
        let expected_hp =
            first.hp * WAVES[0].hp_multiplier + second.hp * WAVES[0].hp_multiplier * 6.0;
        let expected_breach = first.breach_damage as f32 * WAVES[0].damage_multiplier
            + second.breach_damage as f32 * WAVES[0].damage_multiplier * 1.8;
        assert!((total.hp - expected_hp).abs() < 0.001);
        assert!((total.breach_damage - expected_breach).abs() < 0.001);
        assert_eq!(total.xp, u32::from(first.xp) + u32::from(second.xp) * 8);
        assert_eq!(
            total.gold,
            u32::from(first.gold_bounty) + u32::from(second.gold_bounty) * 8
        );
        assert_eq!(total.wood, 16);
        assert_eq!(enemy_base_scale(second.hp, true, 0.1), 1.25);
    }

    #[test]
    fn wave_pacing_uses_exact_count_over_sixty_seconds() {
        assert!((wave_spawn_interval(WAVES[0]) - 60.0 / 18.0).abs() < 0.0001);
        assert!((wave_spawn_interval(WAVES[24]) - 60.0 / 96.0).abs() < 0.0001);
        for wave in WAVES {
            assert!((wave_spawn_interval(wave) * f32::from(wave.count) - 60.0).abs() < 0.001);
        }
    }

    #[test]
    fn all_twelve_archetypes_use_distinct_models() {
        for archetype in 0..12 {
            assert_eq!(visual_for_archetype(archetype), archetype);
        }
        assert_eq!(spawn_x_range(7), (-9.0, -6.8));
        assert_eq!(spawn_x_range(0), (15.3, 16.4));
    }

    #[test]
    fn armor_and_shell_guard_reduction_follow_the_spec() {
        assert!((physical_damage_after_armor(100.0, 0.0, false) - 100.0).abs() < 0.0001);
        assert!((physical_damage_after_armor(100.0, 8.0, false) - 71.42857).abs() < 0.001);
        assert!((physical_damage_after_armor(100.0, 8.0, true) - 42.85714).abs() < 0.001);
        assert!((physical_damage_after_armor(100.0, -5.0, false) - 100.0).abs() < 0.0001);
    }

    #[test]
    fn targeting_keeps_archive_boss_rusher_gate_order() {
        assert_eq!(target_priority(true, false, false, None), 0);
        assert_eq!(target_priority(false, true, false, None), 1);
        assert_eq!(target_priority(false, false, true, None), 2);
        assert_eq!(target_priority(false, false, false, Some(4)), 3);
        assert_eq!(target_priority(false, false, false, Some(0)), 4);
    }

    #[test]
    fn null_eye_reduces_ultimate_recovery_by_fifteen_percent() {
        assert_eq!(ultimate_recovery_rate(false), 1.0);
        assert_eq!(ultimate_recovery_rate(true), 0.85);
        assert_eq!(chanter_speed_multiplier(false), 1.0);
        assert_eq!(chanter_speed_multiplier(true), 1.2);
    }

    #[test]
    fn deaths_consume_only_lives_and_escalate_revive_time() {
        let mut session = GameSession::default();
        let mut vitals = HeroVitals::default();
        let mut market_effects = MarketEffects::default();
        let mut bond_runtime = BondRuntime::default();
        let mut dodge = DodgeState::default();
        let mut intent = MoveIntent(Vec2::ONE);
        let mut transform = Transform::default();
        let mut visibility = Visibility::Visible;

        vitals.health = 0.0;
        begin_hero_death(
            &mut session,
            &mut vitals,
            &mut market_effects,
            &mut bond_runtime,
            &mut dodge,
            &mut intent,
            &mut transform,
            &mut visibility,
        );
        assert_eq!(session.core_hp, 1000.0);
        assert!(!vitals.alive);
        assert_eq!(vitals.deaths, 1);
        assert_eq!(vitals.revive_remaining, 8.0);
        assert_eq!(visibility, Visibility::Hidden);

        begin_hero_death(
            &mut session,
            &mut vitals,
            &mut market_effects,
            &mut bond_runtime,
            &mut dodge,
            &mut intent,
            &mut transform,
            &mut visibility,
        );
        assert_eq!(session.core_hp, 1000.0);

        vitals.alive = true;
        vitals.health = 0.0;
        begin_hero_death(
            &mut session,
            &mut vitals,
            &mut market_effects,
            &mut bond_runtime,
            &mut dodge,
            &mut intent,
            &mut transform,
            &mut visibility,
        );
        assert_eq!(session.core_hp, 1000.0);
        assert_eq!(vitals.revive_remaining, 11.0);
        assert_eq!(vitals.lives_remaining(), 1);
        assert!(!session.finished);
        vitals.alive = true;
        vitals.health = 0.0;
        begin_hero_death(
            &mut session,
            &mut vitals,
            &mut market_effects,
            &mut bond_runtime,
            &mut dodge,
            &mut intent,
            &mut transform,
            &mut visibility,
        );
        assert_eq!(vitals.lives_remaining(), 0);
        assert_eq!(vitals.revive_remaining, 0.0);
        assert_eq!(session.core_hp, 1000.0);
        assert!(session.finished && session.paused && !session.victory);
    }

    #[test]
    fn revival_seed_prevents_death_without_charging_the_gate() {
        let mut session = GameSession::default();
        let original_gate = session.core_hp;
        let mut vitals = HeroVitals {
            health: 0.0,
            ..default()
        };
        let expected_health = vitals.max_health * 0.35;
        let mut market_effects = MarketEffects::with_revival_seed_for_test();
        let mut bond_runtime = BondRuntime::default();
        let mut dodge = DodgeState::default();
        let mut intent = MoveIntent(Vec2::ONE);
        let mut transform = Transform::default();
        let mut visibility = Visibility::Visible;

        begin_hero_death(
            &mut session,
            &mut vitals,
            &mut market_effects,
            &mut bond_runtime,
            &mut dodge,
            &mut intent,
            &mut transform,
            &mut visibility,
        );

        assert_eq!(session.core_hp, original_gate);
        assert!(vitals.alive);
        assert_eq!(vitals.deaths, 0);
        assert!((vitals.health - expected_health).abs() < 0.001);
        assert_eq!(visibility, Visibility::Visible);
    }

    #[test]
    fn hero_hunters_have_distinct_reproducible_threat_profiles() {
        assert_eq!(ENEMIES[3].behavior, EnemyBehavior::HuntHero);
        assert_eq!(ENEMIES[7].behavior, EnemyBehavior::BurrowAmbush);
        assert_eq!(ACID_ARCHER_RANGE, 6.0);
        assert_eq!(ACID_ARCHER_ATTACK_INTERVAL, 1.0);
        assert_eq!(BURROW_REVEAL_RANGE, 1.6);
        assert_eq!(BURROW_STUN_SECONDS, 1.0);
    }

    #[test]
    fn stone_warden_guard_only_applies_near_the_gate() {
        let stone = HeroVitals::for_hero(HeroKind::StoneWarden, 1);
        assert_eq!(
            hero_guard_damage_multiplier(&stone, Vec3::ZERO, Vec3::new(7.9, 0.0, 0.0), None),
            0.85
        );
        assert_eq!(
            hero_guard_damage_multiplier(&stone, Vec3::ZERO, Vec3::new(8.1, 0.0, 0.0), None),
            1.0
        );
        let storm = HeroVitals::default();
        assert_eq!(
            hero_guard_damage_multiplier(&storm, Vec3::ZERO, Vec3::ZERO, None),
            1.0
        );
    }
    #[test]
    fn absorbed_stone_guard_survives_recruitment_without_double_stacking() {
        let mut fusion = super::super::HeroFusion::default();
        fusion.record_absorbed(HeroKind::StoneWarden);
        let storm = HeroVitals::for_hero(HeroKind::StormSavant, 1);
        assert_eq!(
            hero_guard_damage_multiplier(
                &storm,
                Vec3::ZERO,
                Vec3::new(7.9, 0.0, 0.0),
                Some(&fusion)
            ),
            0.85
        );
        assert_eq!(
            hero_guard_damage_multiplier(
                &storm,
                Vec3::ZERO,
                Vec3::new(8.1, 0.0, 0.0),
                Some(&fusion)
            ),
            1.0
        );
        let stone = HeroVitals::for_hero(HeroKind::StoneWarden, 1);
        assert_eq!(
            hero_guard_damage_multiplier(&stone, Vec3::ZERO, Vec3::ZERO, Some(&fusion)),
            0.85
        );
    }
}
