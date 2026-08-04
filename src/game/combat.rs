use std::collections::{HashSet, VecDeque};

use bevy::{
    camera::{ClearColorConfig, RenderTarget, visibility::RenderLayers},
    prelude::*,
};

use super::archive::{ArchiveEcho, ArchiveOutcome, ArchiveState};
use super::bonds::{
    BondRuntime, DamageClass, DamageElement, DamageProfile, ReactionKind, ReactionProc,
    blood_execute_ready, boss_damage_multiplier, critical_multiplier,
    projectile_bond_damage_multiplier, queue_bond_strike,
};
use super::bosses::BossSummon;
use super::content::{BOSSES, ENEMIES, EnemyBehavior};
use super::market::MarketEffects;
use super::pooling::{CombatPools, DAMAGE_NUMBER_RENDER_LAYER, EnemySceneKey};
use super::skills::SkillRuntime;
use super::trial::{complete_gold_target, complete_tower_challenge};
use super::waves::{WAVES, WaveDef, choose_archetype};
use super::{
    ActiveSkill, AttackClock, BondId, BossEnemy, ChallengeKind, Core, DamageMeter, DodgeState,
    Enemy, EnemyArchetype, EnemyHeroThreat, GameAssets, GameSession, GoldChallengeState,
    HERO_RESPAWN_INVULNERABILITY, HERO_RESPAWN_POSITION, Hero, HeroKind, HeroVitals, ImpactFx,
    MoveIntent, PlayerBuild, Projectile, RunRng, SelectedHero, SpawnDirector, TrialEnemy,
    TrialState, revive_delay_for_death,
};

pub const BOSS_NAMES: [&str; 5] = [
    "CINDER COLOSSUS",
    "ABYSSAL LEVIATHAN",
    "DREAD VEIL",
    "SUN CROWN MAGUS",
    "THE GREEN REGENT",
];
const BOSS_TIMES: [f32; 5] = [300.0, 600.0, 900.0, 1_200.0, 1_500.0];
const BOSS_HP: [f32; 5] = [8_000.0, 28_000.0, 70_000.0, 160_000.0, 420_000.0];
const BOSS_SPEED: [f32; 5] = [0.85, 0.62, 0.92, 1.18, 0.78];
const BOSS_DAMAGE_PER_SECOND: [f32; 5] = [14.0, 20.0, 28.0, 38.0, 55.0];
pub(crate) const MAX_REGULAR_ENEMIES: usize = 130;
const ARCHETYPE_VISUALS: [usize; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
const HERO_DEATH_GATE_DAMAGE: f32 = 80.0;
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

pub(crate) const DEBUG_ENEMY_COUNT: usize = ENEMIES.len();
pub(crate) const DEBUG_ELITE_AFFIX_COUNT: usize = 5;
pub(crate) const DEBUG_BOSS_COUNT: usize = BOSS_NAMES.len();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DebugSpawnRequest {
    Enemy {
        archetype: usize,
        elite_affix: Option<usize>,
    },
    Boss {
        index: usize,
    },
}

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
enum EliteAffixKind {
    Frenzy,
    Bulwark,
    Volatile,
    Regeneration,
    Commander,
}

impl EliteAffixKind {
    const fn label(self) -> &'static str {
        match self {
            Self::Frenzy => "FRENZY",
            Self::Bulwark => "BULWARK",
            Self::Volatile => "VOLATILE",
            Self::Regeneration => "REGENERATION",
            Self::Commander => "COMMANDER",
        }
    }

    const fn visual_index(self) -> usize {
        match self {
            Self::Frenzy => 0,
            Self::Bulwark => 1,
            Self::Volatile => 2,
            Self::Regeneration => 3,
            Self::Commander => 4,
        }
    }
}

pub(crate) fn debug_elite_affix_label(index: usize) -> &'static str {
    elite_affix_from_index(index).label()
}

#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct EliteAffix {
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

    const fn is_volatile_pending(self) -> bool {
        self.volatile_remaining > 0.0
    }
}

#[derive(Component)]
struct EliteAffixVisual {
    kind: EliteAffixKind,
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

pub struct CombatPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct CombatSet;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct SurvivalSet;

#[derive(Component)]
struct DamageNumberCamera;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WaveSpawnRuntime>()
            .init_resource::<DamageMeter>()
            .add_systems(PostStartup, setup_damage_number_camera)
            .add_systems(Update, tick_hero_survival.in_set(SurvivalSet))
            .add_systems(
                Update,
                (
                    advance_run,
                    tick_damage_meter,
                    spawn_wave,
                    enemy_auras,
                    commander_aura,
                    tick_elite_affixes,
                    move_enemies,
                    auto_attack,
                    move_projectiles,
                    projectile_hits,
                    cast_active_skill,
                    animate_elite_affix_visuals,
                    animate_effects,
                    debug_shortcuts,
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
    mut dodge: ResMut<DodgeState>,
    mut active: ResMut<ActiveSkill>,
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
            &mut dodge,
            &mut active,
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
    dodge: &mut DodgeState,
    active: &mut ActiveSkill,
    intent: &mut MoveIntent,
    transform: &mut Transform,
    visibility: &mut Visibility,
) {
    if !vitals.alive {
        return;
    }
    if vitals.health <= 0.0 && market_effects.intercept_hero_fatal(vitals) {
        return;
    }
    vitals.alive = false;
    vitals.health = 0.0;
    vitals.deaths = vitals.deaths.saturating_add(1);
    vitals.revive_remaining = revive_delay_for_death(vitals.deaths);
    vitals.invulnerable_remaining = 0.0;
    vitals.stunned_remaining = 0.0;
    session.core_hp = (session.core_hp - HERO_DEATH_GATE_DAMAGE).max(0.0);
    dodge.requested = false;
    active.requested = false;
    intent.0 = Vec2::ZERO;
    // skills.rs cannot yet consume HeroVitals. Moving the hidden root outside
    // combat range is the compatibility guard until that module calls can_act().
    transform.translation = Vec3::new(0.0, -1_000.0, 0.0);
    *visibility = Visibility::Hidden;
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
    if session.core_hp <= 0.0 {
        session.finished = true;
        session.victory = false;
        session.paused = true;
    }
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
) {
    if session.finished {
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

fn spawn_wave(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    mut director: ResMut<SpawnDirector>,
    mut wave_runtime: ResMut<WaveSpawnRuntime>,
    mut rng: ResMut<RunRng>,
    mut pools: ResMut<CombatPools>,
    assets: Option<Res<GameAssets>>,
    living_enemies: Query<
        (),
        (
            With<Enemy>,
            Without<BossEnemy>,
            Without<BossSummon>,
            Without<TrialEnemy>,
            Without<ArchiveEcho>,
        ),
    >,
) {
    if session.paused || session.finished || session.save_active {
        return;
    }
    let Some(assets) = assets else {
        return;
    };

    for (index, spawn_at) in BOSS_TIMES.into_iter().enumerate() {
        if session.elapsed >= spawn_at
            && !director.boss_spawned[index]
            && spawn_boss(&mut commands, &mut pools, &assets, index).is_some()
        {
            director.boss_spawned[index] = true;
        }
    }
    drain_splitling_backlog(&mut commands, &mut pools, &assets);
    let occupied = living_enemies
        .iter()
        .count()
        .max(pools.regular_enemy_active_count());

    let delta = time.delta_secs();
    let wave_index = session.wave.saturating_sub(1).min(24) as usize;
    let wave = WAVES[wave_index];
    let interval = wave_spawn_interval(wave);
    if wave_runtime.wave != session.wave {
        if wave_runtime.wave > 0 {
            let previous = WAVES[wave_runtime.wave.saturating_sub(1).min(24) as usize];
            while wave_runtime.spawned < previous.count {
                let budget = next_spawn_budget(previous, session.elapsed, &mut director, &mut rng);
                wave_runtime.backlog.push_back(budget);
                wave_runtime.spawned += 1;
            }
            for skipped_wave in wave_runtime.wave + 1..session.wave {
                let skipped = WAVES[skipped_wave.saturating_sub(1).min(24) as usize];
                for _ in 0..skipped.count {
                    let budget =
                        next_spawn_budget(skipped, session.elapsed, &mut director, &mut rng);
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
    wave_runtime.accumulator += delta;

    while wave_runtime.spawned < wave.count && wave_runtime.accumulator >= interval {
        wave_runtime.accumulator -= interval;
        let budget = next_spawn_budget(wave, session.elapsed, &mut director, &mut rng);
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

fn next_spawn_budget(
    wave: WaveDef,
    elapsed: f32,
    director: &mut SpawnDirector,
    rng: &mut RunRng,
) -> SpawnBudget {
    let elite = elapsed >= director.next_elite_at;
    if elite {
        director.next_elite_at += 120.0;
    }
    let archetype = choose_archetype(wave, rng.next_u32());
    let affix = elite.then(|| elite_affix_from_roll(rng.next_u32()));
    let (spawn_min_x, spawn_max_x) = spawn_x_range(archetype);
    SpawnBudget {
        wave,
        archetype,
        elite,
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
            let elite_damage = if budget.elite { 1.8 } else { 1.0 };
            let elite_reward = if budget.elite { 8 } else { 1 };
            total.hp += def.hp * budget.wave.hp_multiplier * elite_health;
            total.breach_damage +=
                def.breach_damage as f32 * budget.wave.damage_multiplier * elite_damage;
            total.xp = total
                .xp
                .saturating_add(u32::from(def.xp).saturating_mul(elite_reward));
            total.wood = total.wood.saturating_add(if budget.elite { 16 } else { 0 });
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
        let decoration = commands
            .spawn((
                Name::new(format!("AFFIX · {}", kind.label())),
                Mesh3d(assets.fx_mesh.clone()),
                MeshMaterial3d(elite_affix_material(assets, kind)),
                Transform::from_xyz(0.0, 1.35, 0.0).with_scale(Vec3::splat(0.26)),
                EliteAffixVisual { kind },
            ))
            .id();
        commands.entity(entity).add_child(decoration);
        pools.track_enemy_decoration(entity, decoration);
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
    let bucket = ((roll as u64 * 5) >> 32) as u8;
    match bucket {
        0 => EliteAffixKind::Frenzy,
        1 => EliteAffixKind::Bulwark,
        2 => EliteAffixKind::Volatile,
        3 => EliteAffixKind::Regeneration,
        _ => EliteAffixKind::Commander,
    }
}

fn elite_affix_material(assets: &GameAssets, kind: EliteAffixKind) -> Handle<StandardMaterial> {
    match kind {
        EliteAffixKind::Frenzy => assets.storm_material.clone(),
        EliteAffixKind::Bulwark => assets.enemy_materials[2].clone(),
        EliteAffixKind::Volatile => assets.enemy_materials[1].clone(),
        EliteAffixKind::Regeneration => assets.enemy_materials[3].clone(),
        EliteAffixKind::Commander => assets.elite_material.clone(),
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
    let hp = BOSS_HP[index];
    let entity = pools.rent_enemy_scene(
        commands,
        EnemySceneKey::Boss(index as u8),
        assets.boss_scenes[index].clone(),
        Transform::from_xyz(15.2, 0.0, 0.0).with_scale(Vec3::splat(1.15)),
    )?;
    commands.entity(entity).insert((
        Enemy {
            hp,
            max_hp: hp,
            speed: BOSS_SPEED[index],
            damage: BOSS_DAMAGE_PER_SECOND[index],
            xp: 240 + index as u32 * 160,
            wood: 30 + index as u32 * 18,
            gold: 250 + index as u32 * 180,
            kill_credit: 120 + index as u32 * 80,
            elite: true,
        },
        BossEnemy { index: index as u8 },
        EnemyArmor(0.0),
        EnemyBreachDamage(BOSS_DAMAGE_PER_SECOND[index]),
    ));
    Some(entity)
}

/// Uses the same spawn builders as scheduled waves without advancing their RNG,
/// backlog, elapsed time, or checkpoint bookkeeping.
pub(crate) fn spawn_debug_request(
    commands: &mut Commands,
    pools: &mut CombatPools,
    assets: &GameAssets,
    request: DebugSpawnRequest,
    wave_number: u32,
) -> Option<Entity> {
    match request {
        DebugSpawnRequest::Enemy {
            archetype,
            elite_affix,
        } => {
            let archetype = archetype % DEBUG_ENEMY_COUNT;
            let wave = WAVES[wave_number.saturating_sub(1).min(24) as usize];
            let (min_x, max_x) = spawn_x_range(archetype);
            let affix = elite_affix.map(elite_affix_from_index);
            spawn_enemy(
                commands,
                pools,
                assets,
                SpawnBudget {
                    wave,
                    archetype,
                    elite: affix.is_some(),
                    affix,
                    spawn_x: (min_x + max_x) * 0.5,
                    spawn_z: (archetype % 5) as f32 * 1.55 - 3.1,
                    scale_jitter: 1.0,
                    speed_jitter: 1.0,
                },
            )
        }
        DebugSpawnRequest::Boss { index } => {
            spawn_boss(commands, pools, assets, index % DEBUG_BOSS_COUNT)
        }
    }
}

const fn elite_affix_from_index(index: usize) -> EliteAffixKind {
    match index % DEBUG_ELITE_AFFIX_COUNT {
        0 => EliteAffixKind::Frenzy,
        1 => EliteAffixKind::Bulwark,
        2 => EliteAffixKind::Volatile,
        3 => EliteAffixKind::Regeneration,
        _ => EliteAffixKind::Commander,
    }
}

fn move_enemies(
    mut commands: Commands,
    mut pools: ResMut<CombatPools>,
    time: Res<Time>,
    mut session: ResMut<GameSession>,
    mut vitals: ResMut<HeroVitals>,
    mut market_effects: ResMut<MarketEffects>,
    mut dodge: ResMut<DodgeState>,
    mut active: ResMut<ActiveSkill>,
    mut intent: ResMut<MoveIntent>,
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
                &mut Transform,
            ),
            (Without<TrialEnemy>, Without<ArchiveEcho>),
        >,
    )>,
) {
    if session.paused || session.finished {
        return;
    }
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
    let target = Vec3::new(core.translation.x, 0.0, core.translation.z);
    for (
        entity,
        enemy,
        boss,
        archetype,
        breach_damage,
        mut threat,
        affix,
        commander_aura,
        mut transform,
    ) in &mut enemies.p1()
    {
        if affix.is_some_and(|affix| affix.is_volatile_pending()) {
            continue;
        }
        let action_speed = elite_action_speed_multiplier(affix.map(|affix| affix.kind));
        let commander_multiplier = commander_aura_multiplier(commander_aura);
        let movement_speed = enemy.speed
            * action_speed
            * commander_multiplier
            * market_effects.enemy_movement_multiplier(boss.is_some());
        let attack_damage = enemy.damage * commander_multiplier;
        let behavior =
            archetype.map(|kind| ENEMIES[usize::from(kind.0).min(ENEMIES.len() - 1)].behavior);
        if vitals.can_be_targeted()
            && matches!(
                behavior,
                Some(EnemyBehavior::HuntHero | EnemyBehavior::BurrowAmbush)
            )
            && let Some(threat) = threat.as_deref_mut()
        {
            let to_hero = Vec3::new(
                hero_position.x - transform.translation.x,
                0.0,
                hero_position.z - transform.translation.z,
            );
            let distance = to_hero.length();
            let hastened = chanters.iter().any(|(chanter, position)| {
                *chanter != entity && position.distance_squared(transform.translation) <= 12.25
            });
            let speed_multiplier = chanter_speed_multiplier(hastened);
            match behavior {
                Some(EnemyBehavior::HuntHero) => {
                    if distance > ACID_ARCHER_RANGE {
                        transform.translation += to_hero.normalize_or_zero()
                            * movement_speed
                            * speed_multiplier
                            * time.delta_secs();
                    } else {
                        threat.attack_remaining -= time.delta_secs() * action_speed;
                        while threat.attack_remaining <= 0.0 && vitals.alive {
                            threat.attack_remaining += ACID_ARCHER_ATTACK_INTERVAL;
                            let guard =
                                hero_guard_damage_multiplier(&vitals, hero_position, core_position);
                            vitals.take_physical_damage(attack_damage * guard);
                            if vitals.health <= 0.0 {
                                begin_hero_death(
                                    &mut session,
                                    &mut vitals,
                                    &mut market_effects,
                                    &mut dodge,
                                    &mut active,
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
                    transform.translation += to_hero.normalize_or_zero()
                        * movement_speed
                        * 1.55
                        * speed_multiplier
                        * time.delta_secs();
                    if distance <= BURROW_REVEAL_RANGE {
                        threat.emerged = true;
                        transform.translation.y = 0.0;
                        if !threat.stun_applied {
                            threat.stun_applied = true;
                            market_effects.try_apply_hero_stun(&mut vitals, BURROW_STUN_SECONDS);
                            let guard =
                                hero_guard_damage_multiplier(&vitals, hero_position, core_position);
                            vitals.take_physical_damage(attack_damage * guard);
                            if vitals.health <= 0.0 {
                                begin_hero_death(
                                    &mut session,
                                    &mut vitals,
                                    &mut market_effects,
                                    &mut dodge,
                                    &mut active,
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
                    if distance > BURROW_ATTACK_RANGE {
                        transform.translation += to_hero.normalize_or_zero()
                            * movement_speed
                            * speed_multiplier
                            * time.delta_secs();
                    } else {
                        threat.attack_remaining -= time.delta_secs() * action_speed;
                        while threat.attack_remaining <= 0.0 && vitals.alive {
                            threat.attack_remaining += BURROW_ATTACK_INTERVAL;
                            let guard =
                                hero_guard_damage_multiplier(&vitals, hero_position, core_position);
                            vitals.take_physical_damage(attack_damage * guard);
                            if vitals.health <= 0.0 {
                                begin_hero_death(
                                    &mut session,
                                    &mut vitals,
                                    &mut market_effects,
                                    &mut dodge,
                                    &mut active,
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
            transform.rotate_y(time.delta_secs() * if enemy.elite { 0.8 } else { 2.2 });
            continue;
        }

        if matches!(behavior, Some(EnemyBehavior::BurrowAmbush)) {
            transform.translation.y = 0.0;
        }
        let to_core = target - transform.translation;
        let distance = to_core.length();
        if archetype.is_some_and(|kind| kind.0 == 11) && distance < 8.5 {
            session.core_hp = (session.core_hp
                - attack_damage * action_speed * 0.35 * time.delta_secs())
            .max(0.0);
            transform.rotate_y(time.delta_secs() * 0.7);
        } else if distance < if enemy.elite { 1.9 } else { 1.25 } {
            if boss.is_some() {
                session.core_hp =
                    (session.core_hp - attack_damage * action_speed * time.delta_secs()).max(0.0);
            } else {
                if !market_effects.try_intercept_ordinary_breach() {
                    session.core_hp = (session.core_hp
                        - breach_damage
                            .map_or(attack_damage, |breach| breach.0 * commander_multiplier))
                    .max(0.0);
                }
                pools.retire_enemy(&mut commands, entity);
            }
        } else {
            let hastened = chanters.iter().any(|(chanter, position)| {
                *chanter != entity && position.distance_squared(transform.translation) <= 12.25
            });
            let speed_multiplier = chanter_speed_multiplier(hastened);
            transform.translation +=
                to_core.normalize_or_zero() * movement_speed * speed_multiplier * time.delta_secs();
            transform.rotate_y(time.delta_secs() * if enemy.elite { 0.8 } else { 2.2 });
        }
    }
}

fn hero_guard_damage_multiplier(
    vitals: &HeroVitals,
    hero_position: Vec3,
    core_position: Vec3,
) -> f32 {
    if vitals.kind == HeroKind::StoneWarden && hero_position.distance(core_position) <= 8.0 {
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
    session: Res<GameSession>,
    mut enemies: ParamSet<(
        Query<(&Transform, &EnemyArchetype), With<Enemy>>,
        Query<(&mut Enemy, &Transform, Option<&EnemyArchetype>)>,
    )>,
) {
    if session.paused || session.finished {
        return;
    }
    let healers: Vec<Vec3> = enemies
        .p0()
        .iter()
        .filter(|(_, kind)| kind.0 == 8)
        .map(|(transform, _)| transform.translation)
        .collect();
    if healers.is_empty() {
        return;
    }
    for (mut enemy, transform, kind) in &mut enemies.p1() {
        if kind.is_some_and(|kind| kind.0 == 8) {
            continue;
        }
        if healers
            .iter()
            .any(|position| position.distance_squared(transform.translation) <= 12.25)
        {
            enemy.hp = (enemy.hp + enemy.max_hp * 0.02 * time.delta_secs()).min(enemy.max_hp);
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

#[allow(clippy::too_many_arguments)]
fn tick_elite_affixes(
    mut commands: Commands,
    time: Res<Time>,
    mut session: ResMut<GameSession>,
    mut archive: ResMut<ArchiveState>,
    mut trial: ResMut<TrialState>,
    mut gold_trial: ResMut<GoldChallengeState>,
    mut build: ResMut<PlayerBuild>,
    mut vitals: ResMut<HeroVitals>,
    mut market_effects: ResMut<MarketEffects>,
    mut dodge: ResMut<DodgeState>,
    mut active: ResMut<ActiveSkill>,
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
            Option<&EnemyArchetype>,
            Option<&ArchiveEcho>,
            Option<&CommanderAuraState>,
        ),
        (With<Enemy>, Without<Hero>, Without<Projectile>),
    >,
) {
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
                    * VOLATILE_DAMAGE_MULTIPLIER;
                vitals.take_physical_damage(damage);
                if vitals.health <= 0.0 {
                    begin_hero_death(
                        &mut session,
                        &mut vitals,
                        &mut market_effects,
                        &mut dodge,
                        &mut active,
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
                false,
                entity,
                &enemy,
                position,
                trial_enemy,
                boss_enemy,
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

fn auto_attack(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    vitals: Res<HeroVitals>,
    build: Res<PlayerBuild>,
    mut market_effects: ResMut<MarketEffects>,
    assets: Option<Res<GameAssets>>,
    mut pools: ResMut<CombatPools>,
    mut clock: ResMut<AttackClock>,
    hero_query: Query<&Transform, With<Hero>>,
    enemies: Query<
        (
            Entity,
            &Enemy,
            &Transform,
            Option<&BossEnemy>,
            Option<&ArchiveEcho>,
            Option<&TrialEnemy>,
            Option<&EnemyArchetype>,
            Option<&EliteAffix>,
        ),
        With<Enemy>,
    >,
) {
    if session.paused || session.finished || !vitals.can_act() {
        return;
    }
    let Some(assets) = assets else {
        return;
    };
    let Ok(hero) = hero_query.single() else {
        return;
    };
    clock.0.set_duration(std::time::Duration::from_secs_f32(
        1.0 / build.attacks_per_second.max(0.2),
    ));
    clock.0.tick(time.delta());
    if !clock.0.just_finished() {
        return;
    }

    let mut target: Option<(u8, f32, Entity, Vec3)> = None;
    let mut target_positions = Vec::new();
    let range_squared = build.attack_range * build.attack_range;
    for (entity, enemy, enemy_transform, boss, archive, trial, archetype, affix) in &enemies {
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
            boss.is_some(),
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
        return;
    };

    if market_effects.record_basic_attack() {
        for (entity, position) in storm_coil_targets(target_entity, target, &target_positions) {
            if let Some(projectile_entity) = rent_market_projectile(
                &mut commands,
                &mut pools,
                &assets,
                position,
                build.attack_damage * 0.60,
                0.30,
                0,
                DamageProfile::new(DamageElement::Storm, DamageClass::Attack).as_market_proc(),
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
    let shot_count = build.multishot.max(1);
    for i in 0..shot_count {
        let spread_index = i as f32 - (shot_count as f32 - 1.0) * 0.5;
        let spread = spread_index * 0.14;
        let direction = Quat::from_rotation_y(spread) * base_direction;
        let thunder = build.bond(BondId::Thunder);
        let Some(entity) = pools.rent_projectile(
            &mut commands,
            assets.projectile_mesh.clone(),
            assets.projectile_material.clone(),
            Transform::from_translation(hero.translation + Vec3::Y * 0.25)
                .with_scale(Vec3::splat(1.0 + thunder as f32 * 0.04)),
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
        commands
            .entity(entity)
            .insert(DamageProfile::PHYSICAL_ATTACK);
    }
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
    mut projectiles: Query<(Entity, &mut Projectile, &mut Transform)>,
) {
    if session.paused || session.finished {
        return;
    }
    for (entity, mut projectile, mut transform) in &mut projectiles {
        transform.translation += projectile.velocity * time.delta_secs();
        projectile.lifetime -= time.delta_secs();
        if projectile.lifetime <= 0.0 {
            pools.retire_projectile(&mut commands, entity);
        }
    }
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
    frost_mirror_kill: bool,
    enemy_entity: Entity,
    enemy: &Enemy,
    position: Vec3,
    trial_enemy: Option<&TrialEnemy>,
    boss_enemy: Option<&BossEnemy>,
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
    } else if let Some(boss) = boss_enemy {
        session.xp = session
            .xp
            .saturating_add(xp_with_efficiency(enemy.xp, session.xp_multiplier));
        session.wood += enemy.wood;
        session.gold += enemy.gold;
        session.kills += enemy.kill_credit;
        session.total_kills += 1;
        session.bosses_defeated = session.bosses_defeated.max(boss.index + 1);
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
        session.kills += enemy.kill_credit;
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
        elite_affix_material(assets, kind),
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
    mut session: ResMut<GameSession>,
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
    mut projectiles: Query<(
        Entity,
        &mut Projectile,
        &Transform,
        Option<&DamageProfile>,
        Option<&mut ProjectileHitMemory>,
    )>,
    mut enemies: Query<
        (
            Entity,
            &mut Enemy,
            &Transform,
            Option<&TrialEnemy>,
            Option<&BossEnemy>,
            Option<&EnemyArchetype>,
            Option<&ArchiveEcho>,
            Option<&EnemyArmor>,
            Option<&mut EliteAffix>,
        ),
        Without<Projectile>,
    >,
) {
    if session.paused || session.finished {
        return;
    }
    let Some(assets) = assets else {
        return;
    };
    let target_positions: Vec<(Entity, Vec3)> = enemies
        .iter()
        .filter(|(_, enemy, _, _, _, _, _, _, _)| enemy.hp > 0.0)
        .map(|(entity, _, transform, _, _, _, _, _, _)| (entity, transform.translation))
        .collect();
    for (projectile_entity, mut projectile, projectile_transform, profile, mut hit_memory) in
        &mut projectiles
    {
        let profile = profile.copied().unwrap_or(DamageProfile::PHYSICAL_ATTACK);
        let mut consumed = false;
        let mut first_frame_hits = HashSet::new();
        for (
            enemy_entity,
            mut enemy,
            enemy_transform,
            trial_enemy,
            boss_enemy,
            archetype,
            archive_echo,
            armor,
            mut affix,
        ) in &mut enemies
        {
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
            let projectile_radius = if profile.class == DamageClass::Spell {
                projectile.radius * market_effects.spell_area_multiplier()
            } else {
                projectile.radius
            };
            let hit_radius = projectile_radius + if enemy.elite { 0.95 } else { 0.48 };
            if projectile_transform
                .translation
                .distance_squared(enemy_transform.translation)
                <= hit_radius * hit_radius
            {
                if let Some(memory) = hit_memory.as_deref_mut() {
                    memory.enemies.insert(enemy_entity);
                } else {
                    first_frame_hits.insert(enemy_entity);
                }
                let direct_shell_hit = profile.element == DamageElement::Physical
                    && projectile.velocity.length_squared() > 0.01
                    && archetype.is_some_and(|kind| kind.0 == 2);
                let target = super::bonds::ElementTarget {
                    entity: enemy_entity,
                    max_hp: enemy.max_hp,
                    speed: enemy.speed,
                    boss: boss_enemy.is_some(),
                };
                let reaction_procs = bond_runtime.register_element_hit(
                    profile,
                    target,
                    &build,
                    market_effects.effective_spell_power(vitals.spell_power()),
                    build.attack_damage,
                );
                let frozen =
                    skill_runtime.is_frozen(enemy_entity) || bond_runtime.is_frozen(enemy_entity);
                queue_element_reactions(
                    &mut commands,
                    enemy_entity,
                    enemy_transform.translation,
                    &target_positions,
                    &reaction_procs,
                    market_effects.element_reaction_multiplier(),
                    market_effects.element_reaction_area_multiplier(),
                );
                if let Some(triune) = bond_runtime.register_triune_hit(
                    build.bond(BondId::Triune),
                    profile,
                    projectile.damage,
                ) {
                    let normal_share = 1.0 - triune.pure_share;
                    if normal_share > 0.0 {
                        queue_bond_strike(
                            &mut commands,
                            None,
                            enemy_transform.translation,
                            triune.damage * normal_share,
                            2.8,
                            u8::MAX,
                            DamageProfile::new(DamageElement::Arcane, DamageClass::Reaction),
                            0.0,
                        );
                    }
                    if triune.pure_share > 0.0 {
                        queue_bond_strike(
                            &mut commands,
                            None,
                            enemy_transform.translation,
                            triune.damage * triune.pure_share,
                            2.8,
                            u8::MAX,
                            DamageProfile::PURE_REACTION,
                            0.0,
                        );
                    }
                }

                let mut raw_damage = projectile.damage
                    * projectile_bond_damage_multiplier(profile, &build, &session)
                    * bond_runtime.spirit_mark_multiplier_and_refresh(
                        enemy_entity,
                        build.bond(BondId::Spirits),
                        profile,
                    );
                if profile.element == DamageElement::Flame {
                    raw_damage *= bond_runtime
                        .flame_taken_multiplier(enemy_entity, build.bond(BondId::Ember));
                }
                raw_damage *= critical_multiplier(
                    build.bond(BondId::Blood),
                    build.bond(BondId::Ember),
                    market_effects.burns_can_crit(),
                    profile,
                    rng.unit(),
                );
                raw_damage *=
                    market_outgoing_damage_multiplier(&market_effects, profile, &build, &vitals);
                raw_damage *= market_effects
                    .chaos_damage_multiplier(&mut rng, build.bond(BondId::Fortune) >= 6);
                if let Some(boss) = boss_enemy {
                    raw_damage *= boss_damage_multiplier(
                        profile,
                        BOSSES[usize::from(boss.index.min(4))].modifiers,
                    );
                }
                let mut damage = if profile.element == DamageElement::Physical {
                    physical_damage_after_armor(
                        raw_damage,
                        armor.map_or(0.0, |armor| armor.0),
                        direct_shell_hit,
                    )
                } else {
                    raw_damage
                };
                if blood_execute_ready(
                    build.bond(BondId::Blood),
                    enemy.hp,
                    enemy.max_hp,
                    enemy.elite
                        || boss_enemy.is_some()
                        || trial_enemy.is_some()
                        || archive_echo.is_some(),
                ) {
                    damage = damage.max(enemy.hp);
                }
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
                    );
                    vitals.heal(resolution.hp_damage * market_effects.life_leech());
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
                spawn_hit_fx(
                    &mut commands,
                    &mut pools,
                    &assets,
                    enemy_transform.translation,
                    enemy.elite,
                );
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
                        frozen && !profile.market_proc,
                        enemy_entity,
                        &enemy,
                        enemy_transform.translation,
                        trial_enemy,
                        boss_enemy,
                        archetype,
                        archive_echo,
                    ),
                    DamageOutcome::Alive | DamageOutcome::Pending => {}
                }
                if projectile.remaining_hits == 0 {
                    consumed = true;
                    break;
                }
                projectile.remaining_hits -= 1;
            }
        }
        if !consumed && hit_memory.is_none() && !first_frame_hits.is_empty() {
            commands
                .entity(projectile_entity)
                .insert(ProjectileHitMemory {
                    enemies: first_frame_hits,
                });
        }
        if consumed {
            pools.retire_projectile(&mut commands, projectile_entity);
        }
    }
}

fn physical_damage_after_armor(damage: f32, armor: f32, shell_guard_direct_hit: bool) -> f32 {
    let armor_multiplier = 100.0 / (100.0 + 5.0 * armor.max(0.0));
    damage * armor_multiplier * if shell_guard_direct_hit { 0.6 } else { 1.0 }
}

fn market_outgoing_damage_multiplier(
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
    threefold
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

fn queue_element_reactions(
    commands: &mut Commands,
    source: Entity,
    position: Vec3,
    targets: &[(Entity, Vec3)],
    procs: &[ReactionProc],
    damage_multiplier: f32,
    area_multiplier: f32,
) {
    for reaction in procs {
        let (reaction_damage_multiplier, reaction_area_multiplier) =
            reaction_market_scales(reaction.kind, damage_multiplier, area_multiplier);
        queue_element_reaction(
            commands,
            source,
            position,
            targets,
            *reaction,
            0.0,
            reaction_damage_multiplier,
            reaction_area_multiplier,
        );
        if reaction.echo && reaction.damage > 0.0 {
            queue_element_reaction(
                commands,
                source,
                position,
                targets,
                *reaction,
                0.12,
                reaction_damage_multiplier,
                reaction_area_multiplier,
            );
        }
    }
}

const fn reaction_market_scales(
    kind: ReactionKind,
    damage_multiplier: f32,
    area_multiplier: f32,
) -> (f32, f32) {
    if kind.is_element_reaction() {
        (damage_multiplier, area_multiplier)
    } else {
        (1.0, 1.0)
    }
}

#[allow(clippy::too_many_arguments)]
fn queue_element_reaction(
    commands: &mut Commands,
    source: Entity,
    position: Vec3,
    targets: &[(Entity, Vec3)],
    reaction: ReactionProc,
    delay: f32,
    damage_scale: f32,
    area_scale: f32,
) {
    match reaction.kind {
        ReactionKind::WetStormChain => {
            let mut nearby: Vec<(Entity, f32)> = targets
                .iter()
                .filter_map(|(entity, target_position)| {
                    let distance = position.distance_squared(*target_position);
                    (*entity != source && distance <= (5.5 * area_scale).powi(2))
                        .then_some((*entity, distance))
                })
                .collect();
            nearby.sort_by(|left, right| {
                left.1
                    .total_cmp(&right.1)
                    .then_with(|| left.0.to_bits().cmp(&right.0.to_bits()))
            });
            for (target, _) in nearby.into_iter().take(2) {
                queue_bond_strike(
                    commands,
                    Some(target),
                    position,
                    reaction.damage * damage_scale,
                    0.28,
                    0,
                    DamageProfile::new(DamageElement::Storm, DamageClass::Reaction),
                    delay,
                );
            }
        }
        ReactionKind::WetFrostFreeze => {}
        ReactionKind::BurnStormOverload => queue_bond_strike(
            commands,
            None,
            position,
            reaction.damage * damage_scale,
            3.0 * area_scale,
            u8::MAX,
            DamageProfile::new(DamageElement::Storm, DamageClass::Reaction),
            delay,
        ),
        ReactionKind::FrozenPhysicalShatter => queue_bond_strike(
            commands,
            Some(source),
            position,
            reaction.damage * damage_scale,
            0.18 * area_scale,
            0,
            DamageProfile::new(DamageElement::Physical, DamageClass::Reaction),
            delay,
        ),
        ReactionKind::StormJudgment => queue_bond_strike(
            commands,
            Some(source),
            position,
            reaction.damage * damage_scale,
            0.34,
            0,
            DamageProfile::new(DamageElement::Storm, DamageClass::Reaction),
            delay,
        ),
    }
}

fn cast_active_skill(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    vitals: Res<HeroVitals>,
    selected: Res<SelectedHero>,
    build: Res<PlayerBuild>,
    assets: Option<Res<GameAssets>>,
    mut pools: ResMut<CombatPools>,
    mut active: ResMut<ActiveSkill>,
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
    if active.remaining > 0.0 && !session.paused {
        active.remaining = (active.remaining
            - time.delta_secs() * ultimate_recovery_rate(null_eye_nearby))
        .max(0.0);
    }
    if !active.requested
        || active.remaining > 0.0
        || session.paused
        || session.finished
        || !vitals.can_act()
    {
        active.requested = false;
        return;
    }
    active.requested = false;
    active.remaining = active.cooldown;
    // Each hero's ultimate is resolved by `hero_abilities`. Storm Savant alone
    // keeps this immediate lightning burst as the opening hit of Thunder Domain.
    if selected.kind != HeroKind::StormSavant {
        return;
    }
    let Some(assets) = assets else {
        return;
    };
    let Ok(hero) = hero_query.single() else {
        return;
    };
    let thunder = build.bond(BondId::Thunder);
    let radius = 4.2 + thunder as f32 * 0.2;
    pools.rent_impact_fx(
        &mut commands,
        assets.fx_mesh.clone(),
        assets.storm_material.clone(),
        Transform::from_xyz(hero.translation.x, 0.18, hero.translation.z)
            .with_scale(Vec3::splat(0.3)),
        ImpactFx {
            lifetime: 0.42,
            start_scale: 0.3,
            end_scale: radius,
        },
    );
    rent_market_projectile(
        &mut commands,
        &mut pools,
        &assets,
        hero.translation,
        75.0 + thunder as f32 * 18.0,
        radius,
        u8::MAX,
        DamageProfile::new(DamageElement::Storm, DamageClass::Spell),
    );
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

fn animate_elite_affix_visuals(
    time: Res<Time>,
    mut visuals: Query<(&EliteAffixVisual, &mut Transform)>,
) {
    let elapsed = time.elapsed_secs();
    for (visual, mut transform) in &mut visuals {
        let index = visual.kind.visual_index() as f32;
        let pulse = 0.26 + ((elapsed * (3.2 + index * 0.35) + index).sin() + 1.0) * 0.045;
        transform.scale = Vec3::splat(pulse);
        transform.rotation = Quat::from_rotation_y(elapsed * (1.1 + index * 0.22))
            * Quat::from_rotation_z(index * 0.18);
    }
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

fn debug_shortcuts(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut session: ResMut<GameSession>,
    mut director: ResMut<SpawnDirector>,
) {
    if keyboard.just_pressed(KeyCode::F8) && !session.finished {
        if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
            session.elapsed = 1_499.0;
            director.boss_spawned[4] = false;
        } else {
            session.elapsed = 299.0;
            director.boss_spawned[0] = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    fn test_game_assets() -> GameAssets {
        GameAssets {
            hero_scenes: std::array::from_fn(|_| Handle::default()),
            enemy_scenes: std::array::from_fn(|_| Handle::default()),
            boss_scenes: std::array::from_fn(|_| Handle::default()),
            summon_scenes: std::array::from_fn(|_| Handle::default()),
            enemy_mesh: Handle::default(),
            enemy_meshes: std::array::from_fn(|_| Handle::default()),
            elite_mesh: Handle::default(),
            projectile_mesh: Handle::default(),
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
            .init_resource::<SpawnDirector>()
            .init_resource::<WaveSpawnRuntime>()
            .init_resource::<RunRng>()
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
    fn checkpoint_boss_spawns_exactly_on_time_with_130_regular_shells_active() {
        let mut app = wave_test_app(BOSS_TIMES[0] - 0.001, 5);
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

        app.world_mut().resource_mut::<GameSession>().elapsed = BOSS_TIMES[0];
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
    fn logic_only_projectile_still_runs_the_authoritative_damage_resolver() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
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
    fn prismatic_engine_scales_only_the_four_element_reactions() {
        for kind in [
            ReactionKind::WetStormChain,
            ReactionKind::WetFrostFreeze,
            ReactionKind::BurnStormOverload,
            ReactionKind::FrozenPhysicalShatter,
        ] {
            assert_eq!(reaction_market_scales(kind, 1.8, 1.4), (1.8, 1.4));
        }
        assert_eq!(
            reaction_market_scales(ReactionKind::StormJudgment, 1.8, 1.4),
            (1.0, 1.0)
        );
    }

    #[test]
    fn elite_affix_rolls_are_deterministic_and_cover_all_five_kinds() {
        assert_eq!(elite_affix_from_roll(0), EliteAffixKind::Frenzy);
        assert_eq!(
            elite_affix_from_roll(1_000_000_000),
            EliteAffixKind::Bulwark
        );
        assert_eq!(
            elite_affix_from_roll(2_000_000_000),
            EliteAffixKind::Volatile
        );
        assert_eq!(
            elite_affix_from_roll(3_000_000_000),
            EliteAffixKind::Regeneration
        );
        assert_eq!(
            elite_affix_from_roll(4_000_000_000),
            EliteAffixKind::Commander
        );

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

    #[test]
    fn debug_dispatch_spawns_every_runtime_enemy_affix_and_boss_component_set() {
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
        ];
        for expected in expected_affixes {
            assert!(regular.iter().any(|(_, enemy, kind, _, _, affix, _)| {
                enemy.elite && kind.0 == 7 && affix.is_some_and(|affix| affix.kind == expected)
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
        for (index, expected_damage) in BOSS_DAMAGE_PER_SECOND.into_iter().enumerate() {
            assert!(bosses.iter().any(|(boss, enemy, armor, breach, _)| {
                usize::from(boss.index) == index
                    && enemy.elite
                    && armor.0 == 0.0
                    && breach.0 == expected_damage
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
    fn one_death_charges_the_gate_once_and_escalates_revive_time() {
        let mut session = GameSession::default();
        let mut vitals = HeroVitals::default();
        let mut market_effects = MarketEffects::default();
        let mut dodge = DodgeState::default();
        let mut active = ActiveSkill::default();
        let mut intent = MoveIntent(Vec2::ONE);
        let mut transform = Transform::default();
        let mut visibility = Visibility::Visible;

        vitals.health = 0.0;
        begin_hero_death(
            &mut session,
            &mut vitals,
            &mut market_effects,
            &mut dodge,
            &mut active,
            &mut intent,
            &mut transform,
            &mut visibility,
        );
        assert_eq!(session.core_hp, 920.0);
        assert!(!vitals.alive);
        assert_eq!(vitals.deaths, 1);
        assert_eq!(vitals.revive_remaining, 8.0);
        assert_eq!(visibility, Visibility::Hidden);

        begin_hero_death(
            &mut session,
            &mut vitals,
            &mut market_effects,
            &mut dodge,
            &mut active,
            &mut intent,
            &mut transform,
            &mut visibility,
        );
        assert_eq!(session.core_hp, 920.0);

        vitals.alive = true;
        vitals.health = 0.0;
        begin_hero_death(
            &mut session,
            &mut vitals,
            &mut market_effects,
            &mut dodge,
            &mut active,
            &mut intent,
            &mut transform,
            &mut visibility,
        );
        assert_eq!(session.core_hp, 840.0);
        assert_eq!(vitals.revive_remaining, 11.0);
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
        let mut dodge = DodgeState::default();
        let mut active = ActiveSkill::default();
        let mut intent = MoveIntent(Vec2::ONE);
        let mut transform = Transform::default();
        let mut visibility = Visibility::Visible;

        begin_hero_death(
            &mut session,
            &mut vitals,
            &mut market_effects,
            &mut dodge,
            &mut active,
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
            hero_guard_damage_multiplier(&stone, Vec3::ZERO, Vec3::new(7.9, 0.0, 0.0)),
            0.85
        );
        assert_eq!(
            hero_guard_damage_multiplier(&stone, Vec3::ZERO, Vec3::new(8.1, 0.0, 0.0)),
            1.0
        );
        let storm = HeroVitals::default();
        assert_eq!(
            hero_guard_damage_multiplier(&storm, Vec3::ZERO, Vec3::ZERO),
            1.0
        );
    }
}
