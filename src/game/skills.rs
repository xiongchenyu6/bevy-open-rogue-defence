use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use super::archive::ArchiveEcho;
use super::bonds::{BondSet, DamageClass, DamageElement, DamageProfile, summon_cap};
use super::combat::{AttackVolleyRuntime, CombatSet, SurvivalSet};
use super::content::{SKILLS, Tag};
use super::market::MarketEffects;
use super::pooling::{CombatPools, SummonVisual};
use super::{
    BossEnemy, Enemy, GameAssets, GameSession, Hero, HeroVitals, ImpactFx, PlayerBuild,
    PlayerSummon, Projectile, RunRng, SelectedHero, SkillId, TrialEnemy,
};

#[path = "skills/evolution_mechanics.rs"]
mod evolution_mechanics;
#[path = "skills/growth.rs"]
mod growth;
#[path = "skills/infection.rs"]
pub(crate) mod infection;
#[path = "skills/orbit_art.rs"]
mod orbit_art;
#[path = "skills/presentation.rs"]
mod presentation;
#[path = "skills/reference_skills.rs"]
mod reference_skills;
#[path = "skills/ur_evolutions.rs"]
pub(crate) mod ur_evolutions;

#[path = "skills/summons.rs"]
mod summons;
use summons::{update_epic_summons, update_rare_summons, update_skill_summons};

#[path = "skills/tidal_wave.rs"]
pub(crate) mod tidal_wave;
#[path = "skills/tidal_wave_vfx.rs"]
mod tidal_wave_vfx;

const BURN_TICK_SECONDS: f32 = 0.5;
const MARKET_ECHO_DAMAGE_SCALE: f32 = 0.45;

pub struct SkillsPlugin;

impl Plugin for SkillsPlugin {
    fn build(&self, app: &mut App) {
        presentation::install(app);
        tidal_wave::install(app);
        tidal_wave_vfx::install(app);
        orbit_art::install(app);
        evolution_mechanics::install(app);
        ur_evolutions::install(app);
        reference_skills::install(app);
        app.init_resource::<SkillRuntime>()
            .add_systems(
                Update,
                (
                    tick_skill_runtime,
                    update_triune_engine,
                    sync_orbit_blades,
                    update_skill_summons,
                    update_rare_summons,
                    update_epic_summons,
                    update_oath_banners,
                    cleanup_skill_summons,
                    update_spell_zones,
                    infection::update,
                    update_execution_brands,
                    update_homing_swords,
                    update_blood_wheels,
                    update_orbit_blades,
                    resolve_skill_strikes,
                    animate_skill_visuals,
                )
                    .chain()
                    .before(BondSet)
                    .before(CombatSet),
            )
            .add_systems(Update, intercept_ember_rebirth.before(SurvivalSet))
            .add_systems(
                Update,
                growth::update_summon_growth.after(cast_learned_skills),
            )
            .add_systems(
                Update,
                (
                    resolve_transmute_kills,
                    intercept_ember_rebirth,
                    cast_learned_skills,
                    resolve_market_echoes,
                )
                    .chain()
                    .after(CombatSet),
            );
    }
}

#[derive(Resource)]
pub(crate) struct SkillRuntime {
    cooldowns: [f32; SkillId::COUNT],
    evolution_tiers: [u8; SkillId::COUNT],
    attack_count: u64,
    burns: HashMap<Entity, BurnState>,
    controls: HashMap<Entity, ControlState>,
    seals: HashMap<Entity, u8>,
    orbit_hit_cooldowns: HashMap<Entity, f32>,
    wet: HashMap<Entity, f32>,
    tide_repairs: HashMap<Entity, f32>,
    spell_cast_count: u64,
    last_spell: Option<SpellRecord>,
    triune: TriuneState,
    last_hero_position: Vec3,
    spell_power_multiplier: f32,
    summon_inherit_multiplier: f32,
    market_echo_chance: f32,
}

impl Default for SkillRuntime {
    fn default() -> Self {
        Self {
            cooldowns: [0.0; SkillId::COUNT],
            evolution_tiers: [0; SkillId::COUNT],
            attack_count: 0,
            burns: HashMap::new(),
            controls: HashMap::new(),
            seals: HashMap::new(),
            orbit_hit_cooldowns: HashMap::new(),
            wet: HashMap::new(),
            tide_repairs: HashMap::new(),
            spell_cast_count: 0,
            last_spell: None,
            triune: TriuneState::default(),
            last_hero_position: Vec3::new(-8.2, 0.0, 0.0),
            spell_power_multiplier: 1.0,
            summon_inherit_multiplier: 1.0,
            market_echo_chance: 0.0,
        }
    }
}

impl SkillRuntime {
    pub(crate) fn freeze_relic(
        &mut self,
        entity: Entity,
        speed: f32,
        elite: bool,
        boss: bool,
        seconds: f32,
    ) {
        let control = self.controls.entry(entity).or_insert(ControlState {
            base_speed: speed,
            slow_remaining: 0.0,
            frozen_remaining: 0.0,
            stunned_remaining: 0.0,
        });
        control.frozen_remaining = control
            .frozen_remaining
            .max(seconds * control_duration_scale(elite, boss));
    }
    pub(crate) fn ignite_relic(&mut self, entity: Entity, damage: f32) {
        add_burn(&mut self.burns, entity, damage, 3);
    }
    pub(crate) fn is_frozen(&self, target: Entity) -> bool {
        self.controls
            .get(&target)
            .is_some_and(|control| control.frozen_remaining > 0.0)
    }
}

#[derive(Debug, Clone, Copy)]
struct BurnState {
    layers: u8,
    damage_per_tick: f32,
    remaining: f32,
    tick_remaining: f32,
}

impl BurnState {
    fn new(total_damage: f32) -> Self {
        Self {
            layers: 1,
            damage_per_tick: total_damage / 6.0,
            remaining: 3.0,
            tick_remaining: BURN_TICK_SECONDS,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ControlState {
    base_speed: f32,
    slow_remaining: f32,
    frozen_remaining: f32,
    stunned_remaining: f32,
}

#[derive(Component)]
struct SkillStrike {
    target: Entity,
    fixed_position: Option<Vec3>,
    delay: f32,
    damage: f32,
    radius: f32,
    area: bool,
    fx_scale: f32,
    transmute_rewards: Vec<TransmuteReward>,
    profile: DamageProfile,
}

#[derive(Component)]
struct MarketEchoRequest {
    record: SpellRecord,
    chance: f32,
}

#[derive(Component)]
struct SkillVisualBolt {
    velocity: Vec3,
    lifetime: f32,
}

#[derive(Component)]
pub(crate) struct SkillSummon;

#[derive(Component)]
pub(crate) struct BoneSentry {
    pub(crate) attack_remaining: f32,
    damage: f32,
}

#[derive(Component)]
pub(crate) struct SpiritHound {
    source_skill: usize,
    remaining: f32,
    pub(crate) attack_remaining: f32,
    damage: f32,
}

#[derive(Component)]
pub(crate) struct TideElemental {
    remaining: f32,
    pub(crate) attack_remaining: f32,
    damage: f32,
}

#[derive(Component)]
pub(crate) struct BoltTurret {
    remaining: f32,
    pub(crate) attack_remaining: f32,
    damage: f32,
}

#[derive(Component)]
pub(crate) struct FrostWyrm {
    remaining: f32,
    pub(crate) attack_remaining: f32,
    damage: f32,
    frost_hits: HashMap<Entity, u8>,
}

#[derive(Component)]
pub(crate) struct AncestorColossus {
    remaining: f32,
    pub(crate) slam_remaining: f32,
    damage: f32,
}

#[derive(Component)]
pub(crate) struct OathBanner {
    remaining: f32,
    repair_remaining: f32,
    damage_reduction: f32,
    applied: bool,
    armor_bonus: f32,
    attack_speed_bonus: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ZoneKind {
    StormMatrix,
    GlacierCore,
    VortexColumn,
}

#[derive(Component)]
pub(crate) struct SkillZone {
    kind: ZoneKind,
    remaining: f32,
    pulse_remaining: f32,
    damage_per_pulse: f32,
    radius: f32,
    glacier_hits: HashMap<Entity, u8>,
    profile: DamageProfile,
}

impl SkillZone {
    pub(crate) fn lighting_radius(&self) -> f32 {
        self.radius
    }
    pub(crate) fn lighting_profile(&self) -> &DamageProfile {
        &self.profile
    }
}

#[derive(Component)]
struct ExecutionBrand {
    infection_rank: u8,
    infection_damage: f32,
    target: Entity,
    remaining: f32,
    threshold: f32,
}

#[derive(Component)]
pub(crate) struct HomingSword {
    target: Entity,
    damage: f32,
    speed: f32,
    remaining: f32,
    profile: DamageProfile,
}

#[derive(Component)]
pub(crate) struct BloodWheel {
    origin: Vec3,
    direction: Vec3,
    speed: f32,
    max_distance: f32,
    traveled: f32,
    returning: bool,
    damage: f32,
    burn_cap: u8,
    outbound_hits: HashSet<Entity>,
    return_hits: HashSet<Entity>,
    outbound_profile: DamageProfile,
    return_profile: DamageProfile,
}

#[derive(Component)]
struct TransmuteKillWatch {
    target: Entity,
    expected_lethal: bool,
    chance: f32,
    reward: u32,
}

#[derive(Debug, Clone, Copy)]
struct TransmuteReward {
    target: Entity,
    expected_lethal: bool,
    chance: f32,
    reward: u32,
}

#[derive(Component)]
pub(crate) struct OrbitBlade {
    slot: u8,
}

#[derive(Clone, Copy)]
struct TargetInfo {
    entity: Entity,
    position: Vec3,
    speed: f32,
    elite: bool,
    boss: bool,
    special: bool,
    hp: f32,
    max_hp: f32,
    gold: u32,
}

#[derive(Debug, Clone, Copy)]
struct SpellRecord {
    skill_index: usize,
    target: Entity,
    center: Vec3,
    damage: f32,
}

#[derive(Debug, Clone, Copy, Default)]
struct TriuneState {
    rank: u8,
    phase: u8,
    phase_remaining: f32,
    attack_bonus: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FateOutcome {
    Lightning,
    FireRain,
    Heal,
    Gold,
    Summon,
}

#[derive(Clone, Copy)]
struct SummonBudget(usize);

impl SummonBudget {
    fn from_existing(existing: usize, equipment_bonus: u8) -> Self {
        Self(summon_cap(equipment_bonus).saturating_sub(existing))
    }

    fn take(&mut self) -> bool {
        if self.0 == 0 {
            false
        } else {
            self.0 -= 1;
            true
        }
    }
}

fn tick_skill_runtime(
    mut commands: Commands,
    time: Res<Time>,
    mut session: ResMut<GameSession>,
    market_effects: Res<MarketEffects>,
    pressure: Option<Res<super::combat::ElitePressure>>,
    encounters: Res<super::encounters::EncounterState>,
    mut runtime: ResMut<SkillRuntime>,
    mut enemies: Query<(Entity, &mut Enemy, &Transform)>,
    tide_elementals: Query<(), With<TideElemental>>,
    evolution: Option<Res<super::skill_evolution::SkillEvolution>>,
) {
    runtime.evolution_tiers = evolution.as_ref().map_or([0; SkillId::COUNT], |e| e.tiers);
    runtime.spell_power_multiplier = market_effects.effective_spell_power(1.0);
    runtime.summon_inherit_multiplier = market_effects.effective_summon_power(1.0);
    runtime.market_echo_chance = market_effects.spell_echo_chance();
    if session.paused || session.finished {
        return;
    }
    let delta = time.delta_secs();
    // Armory Donation shortens every skill cooldown by recovering it faster.
    let recovery = delta
        * encounters.cooldown_recovery_multiplier()
        * pressure
            .as_ref()
            .map_or(1.0, |p| p.cooldown_recovery_multiplier());
    for cooldown in &mut runtime.cooldowns {
        *cooldown = (*cooldown - recovery).max(0.0);
    }
    for cooldown in runtime.orbit_hit_cooldowns.values_mut() {
        *cooldown -= delta;
    }
    runtime
        .orbit_hit_cooldowns
        .retain(|entity, cooldown| *cooldown > 0.0 && enemies.contains(*entity));
    runtime.seals.retain(|entity, _| enemies.contains(*entity));
    for remaining in runtime.wet.values_mut() {
        *remaining -= delta;
    }
    runtime
        .wet
        .retain(|entity, remaining| *remaining > 0.0 && enemies.contains(*entity));

    let removed_tides: Vec<(Entity, f32)> = runtime
        .tide_repairs
        .iter()
        .filter(|(entity, _)| !tide_elementals.contains(**entity))
        .map(|(entity, repair)| (*entity, *repair))
        .collect();
    for (entity, repair) in removed_tides {
        session.heal_gate(repair);
        runtime.tide_repairs.remove(&entity);
    }

    let controlled: Vec<Entity> = runtime.controls.keys().copied().collect();
    let mut expired_controls = Vec::new();
    for entity in controlled {
        let Some(control) = runtime.controls.get_mut(&entity) else {
            continue;
        };
        control.slow_remaining = (control.slow_remaining - delta).max(0.0);
        control.frozen_remaining = (control.frozen_remaining - delta).max(0.0);
        control.stunned_remaining = (control.stunned_remaining - delta).max(0.0);
        let Ok((_, mut enemy, _)) = enemies.get_mut(entity) else {
            expired_controls.push(entity);
            continue;
        };
        if control.frozen_remaining > 0.0 || control.stunned_remaining > 0.0 {
            enemy.speed = 0.0;
        } else if control.slow_remaining > 0.0 {
            enemy.speed = control.base_speed * 0.7;
        } else {
            enemy.speed = control.base_speed;
            expired_controls.push(entity);
        }
    }
    for entity in expired_controls {
        runtime.controls.remove(&entity);
    }

    let burning: Vec<Entity> = runtime.burns.keys().copied().collect();
    let mut expired_burns = Vec::new();
    for entity in burning {
        if !enemies.contains(entity) {
            expired_burns.push(entity);
            continue;
        }
        let Some(burn) = runtime.burns.get_mut(&entity) else {
            continue;
        };
        burn.remaining -= delta;
        burn.tick_remaining -= delta;
        let mut due_damage = 0.0;
        while burn.tick_remaining <= 0.0 && burn.remaining >= -0.001 {
            due_damage += burn.damage_per_tick;
            burn.tick_remaining += BURN_TICK_SECONDS;
        }
        if due_damage > 0.0 {
            schedule_typed_strike(
                &mut commands,
                entity,
                due_damage,
                0.0,
                0.05,
                false,
                0.38,
                DamageProfile::new(DamageElement::Flame, DamageClass::Reaction).as_burn_tick(),
            );
        }
        if burn.remaining <= 0.0 {
            expired_burns.push(entity);
        }
    }
    for entity in expired_burns {
        runtime.burns.remove(&entity);
    }
}

fn update_triune_engine(
    time: Res<Time>,
    session: Res<GameSession>,
    mut build: ResMut<PlayerBuild>,
    mut runtime: ResMut<SkillRuntime>,
) {
    if session.finished {
        remove_triune_attack_bonus(&mut runtime.triune, &mut build);
        runtime.triune.rank = 0;
        return;
    }
    let rank = build.skills[28];
    if rank != runtime.triune.rank {
        remove_triune_attack_bonus(&mut runtime.triune, &mut build);
        runtime.triune.rank = rank;
        runtime.triune.phase = 0;
        runtime.triune.phase_remaining = 4.0;
        if triune_attack_active(&runtime.triune) {
            apply_triune_attack_bonus(&mut runtime.triune, &mut build);
        }
    }
    if session.paused || rank == 0 || rank >= 5 {
        return;
    }
    runtime.triune.phase_remaining -= time.delta_secs();
    while runtime.triune.phase_remaining <= 0.0 {
        remove_triune_attack_bonus(&mut runtime.triune, &mut build);
        runtime.triune.phase = (runtime.triune.phase + 1) % 3;
        runtime.triune.phase_remaining += 4.0;
        if triune_attack_active(&runtime.triune) {
            apply_triune_attack_bonus(&mut runtime.triune, &mut build);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn intercept_ember_rebirth(
    mut commands: Commands,
    session: Res<GameSession>,
    build: Res<PlayerBuild>,
    mut vitals: ResMut<HeroVitals>,
    mut runtime: ResMut<SkillRuntime>,
    mut hero: Query<(&mut Transform, &mut Visibility), With<Hero>>,
) {
    if session.paused || session.finished {
        return;
    }
    let Ok((mut transform, mut visibility)) = hero.single_mut() else {
        return;
    };
    if vitals.alive && vitals.health > 0.0 {
        runtime.last_hero_position = transform.translation;
        return;
    }
    let rank = build.skills[25];
    if !ember_rebirth_ready(rank, runtime.cooldowns[25], vitals.health) {
        return;
    }
    let had_settled_death = !vitals.alive;
    let position = if had_settled_death {
        runtime.last_hero_position
    } else {
        transform.translation
    };
    if had_settled_death {
        vitals.deaths = vitals.deaths.saturating_sub(1);
    }
    vitals.alive = true;
    vitals.health = vitals.max_health * (0.35 * skill_utility_multiplier(25)).min(0.9);
    vitals.revive_remaining = 0.0;
    vitals.invulnerable_remaining = vitals
        .invulnerable_remaining
        .max(0.8 * skill_utility_multiplier(25));
    transform.translation = position;
    *visibility = Visibility::Visible;
    runtime.cooldowns[25] = skill_cooldown(25);
    let damage = scaled_skill_damage(25, rank, &build, &vitals, &runtime);
    schedule_typed_fixed_strike(
        &mut commands,
        position,
        damage,
        0.0,
        skill_effect_radius(25, rank).unwrap(),
        true,
        skill_effect_radius(25, rank).unwrap(),
        DamageProfile::for_skill(25),
    );
}

fn sync_orbit_blades(
    mut commands: Commands,
    session: Res<GameSession>,
    build: Res<PlayerBuild>,
    vitals: Res<HeroVitals>,
    art: Option<Res<orbit_art::OrbitArt>>,
    hero: Query<&Transform, With<Hero>>,
    blades: Query<(Entity, &OrbitBlade)>,
    evolution: Option<Res<super::skill_evolution::SkillEvolution>>,
) {
    if session.paused || session.finished {
        return;
    }
    let tier = evolution.as_ref().map_or(0, |e| e.tiers[6]);
    let rank = build.skills[6];
    let desired = if vitals.alive && rank > 0 {
        orbit_blade_count(rank) + tier * 2
    } else {
        0
    };
    let mut occupied = [false; 10];
    for (entity, blade) in &blades {
        if blade.slot >= desired {
            commands.entity(entity).despawn();
        } else {
            occupied[blade.slot as usize] = true;
        }
    }
    if desired == 0 {
        return;
    }
    let Some(art) = art else {
        return;
    };
    let Ok(hero) = hero.single() else {
        return;
    };
    for slot in 0..desired {
        if occupied[slot as usize] {
            continue;
        }
        orbit_art::spawn(
            &mut commands,
            &art,
            hero.translation + Vec3::Y * 0.48,
            rank,
            slot,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn summon_attack_delta(delta: f32, attack_speed_multiplier: f32) -> f32 {
    delta * attack_speed_multiplier
}

fn update_oath_banners(
    mut commands: Commands,
    time: Res<Time>,
    mut session: ResMut<GameSession>,
    mut build: ResMut<PlayerBuild>,
    mut vitals: ResMut<HeroVitals>,
    selected: Res<SelectedHero>,
    mut pools: ResMut<CombatPools>,
    mut banners: Query<(Entity, &mut OathBanner)>,
) {
    if session.finished || selected.is_changed() || build.skills[21] == 0 {
        for (entity, mut banner) in &mut banners {
            retire_oath_banner(
                &mut commands,
                &mut pools,
                entity,
                &mut banner,
                &mut build,
                &mut vitals,
            );
        }
        return;
    }
    if session.paused {
        return;
    }
    let delta = time.delta_secs();
    for (entity, mut banner) in &mut banners {
        if !banner.applied {
            apply_banner_bonuses(&mut banner, &mut build, &mut vitals);
        }
        banner.remaining -= delta;
        banner.repair_remaining -= delta;
        while banner.repair_remaining <= 0.0 && banner.remaining >= -0.001 {
            session.heal_gate(skill_utility_multiplier(21));
            banner.repair_remaining += 1.0;
        }
        if banner.remaining <= 0.0 {
            retire_oath_banner(
                &mut commands,
                &mut pools,
                entity,
                &mut banner,
                &mut build,
                &mut vitals,
            );
        }
    }
}

fn retire_oath_banner(
    commands: &mut Commands,
    pools: &mut CombatPools,
    entity: Entity,
    banner: &mut OathBanner,
    build: &mut PlayerBuild,
    vitals: &mut HeroVitals,
) -> bool {
    remove_banner_bonuses(banner, build, vitals);
    pools.retire_summon(commands, entity)
}

fn cleanup_skill_summons(
    mut commands: Commands,
    session: Res<GameSession>,
    selected: Res<SelectedHero>,
    build: Res<PlayerBuild>,
    summons: Query<
        (
            Entity,
            Option<&BoneSentry>,
            Option<&SpiritHound>,
            Option<&TideElemental>,
            Option<&BoltTurret>,
            Option<&FrostWyrm>,
            Option<&AncestorColossus>,
        ),
        With<SkillSummon>,
    >,
    mut runtime: ResMut<SkillRuntime>,
    mut pools: ResMut<CombatPools>,
) {
    for (entity, bone, hound, tide, turret, wyrm, colossus) in &summons {
        let skill = if bone.is_some() {
            Some(4)
        } else if let Some(hound) = hound {
            Some(hound.source_skill)
        } else if tide.is_some() {
            Some(16)
        } else if turret.is_some() {
            Some(17)
        } else if wyrm.is_some() {
            Some(26)
        } else if colossus.is_some() {
            Some(27)
        } else {
            None
        };
        if !session.finished
            && !selected.is_changed()
            && !skill.is_some_and(|skill| build.skills[skill] == 0)
        {
            continue;
        }
        runtime.tide_repairs.remove(&entity);
        pools.retire_summon(&mut commands, entity);
    }
}

#[allow(clippy::too_many_arguments)]
fn update_spell_zones(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    build: Res<PlayerBuild>,
    vitals: Res<HeroVitals>,
    mut runtime: ResMut<SkillRuntime>,
    mut zones: Query<(Entity, &mut SkillZone, &mut Transform), (Without<Enemy>, Without<Hero>)>,
    mut enemies: Query<
        (
            Entity,
            &mut Transform,
            &Enemy,
            Option<&BossEnemy>,
            Option<&TrialEnemy>,
            Option<&ArchiveEcho>,
        ),
        (Without<SkillZone>, Without<Hero>),
    >,
) {
    if session.paused || session.finished {
        return;
    }
    let delta = time.delta_secs();
    for (zone_entity, mut zone, mut zone_transform) in &mut zones {
        zone.remaining -= delta;
        zone.pulse_remaining -= delta;
        zone_transform.rotate_y(delta * 1.6);
        let interval = zone_pulse_interval(zone.kind);
        let mut pulses = 0_u8;
        while zone.pulse_remaining <= 0.0 && zone.remaining >= -0.001 {
            zone.pulse_remaining += interval;
            pulses = pulses.saturating_add(1);
        }

        let center = zone_transform.translation;
        let radius_squared = zone.radius * zone.radius;
        let mut affected = Vec::new();
        for (entity, mut transform, enemy, boss, trial, archive) in &mut enemies {
            if enemy.hp <= 0.0 || center.distance_squared(transform.translation) > radius_squared {
                continue;
            }
            let target = target_info(entity, &transform, enemy, boss, trial, archive);
            if zone.kind == ZoneKind::VortexColumn {
                let pull_scale = forced_movement_scale(enemy.elite, boss.is_some());
                let to_center = center - transform.translation;
                transform.translation +=
                    to_center.normalize_or_zero() * 3.8 * pull_scale * delta.min(0.1);
            }
            affected.push(target);
        }

        for _ in 0..pulses {
            for target in &affected {
                schedule_typed_strike(
                    &mut commands,
                    target.entity,
                    zone.damage_per_pulse,
                    0.0,
                    0.05,
                    false,
                    match zone.kind {
                        ZoneKind::StormMatrix => 0.7,
                        ZoneKind::GlacierCore => 0.62,
                        ZoneKind::VortexColumn => 0.55,
                    },
                    zone.profile,
                );
                if zone.kind == ZoneKind::GlacierCore {
                    let hits = zone.glacier_hits.entry(target.entity).or_default();
                    *hits += 1;
                    if *hits >= 3 {
                        *hits = 0;
                        apply_control(&mut runtime, *target, 0.0, 1.0, 0.0);
                    }
                }
            }
            apply_static_seals(
                &mut commands,
                &mut runtime,
                affected.iter().copied(),
                &build,
                &vitals,
                0.0,
            );
        }
        zone.glacier_hits
            .retain(|entity, _| enemies.contains(*entity));
        if zone.remaining <= 0.0 {
            commands.entity(zone_entity).despawn();
        }
    }
}

fn update_execution_brands(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    mut brands: Query<(Entity, &mut ExecutionBrand)>,
    enemies: Query<&Enemy>,
) {
    if session.paused || session.finished {
        return;
    }
    for (entity, mut brand) in &mut brands {
        brand.remaining -= time.delta_secs();
        let Ok(enemy) = enemies.get(brand.target) else {
            commands.entity(entity).despawn();
            continue;
        };
        if execution_ready(enemy.hp, enemy.max_hp, brand.threshold) {
            schedule_typed_strike(
                &mut commands,
                brand.target,
                enemy.max_hp * 50.0,
                0.0,
                0.05,
                false,
                1.25,
                DamageProfile::for_skill(18),
            );
            commands.entity(entity).despawn();
        } else if brand.remaining <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn update_homing_swords(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    mut swords: Query<(Entity, &mut HomingSword, &mut Transform), Without<Enemy>>,
    enemies: Query<(Entity, &Transform, &Enemy), Without<HomingSword>>,
) {
    if session.paused || session.finished {
        return;
    }
    let delta = time.delta_secs();
    let targets: Vec<(Entity, Vec3)> = enemies
        .iter()
        .filter(|(_, _, enemy)| enemy.hp > 0.0)
        .map(|(entity, transform, _)| (entity, transform.translation))
        .collect();
    for (entity, mut sword, mut transform) in &mut swords {
        sword.remaining -= delta;
        if sword.remaining <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        let target = targets
            .iter()
            .find(|(candidate, _)| *candidate == sword.target)
            .copied()
            .or_else(|| nearest_position(transform.translation, &targets));
        let Some((target_entity, target_position)) = target else {
            continue;
        };
        sword.target = target_entity;
        let offset = target_position - transform.translation;
        if offset.length() <= 0.55 * (transform.scale.x / 0.62) {
            schedule_typed_strike(
                &mut commands,
                target_entity,
                sword.damage,
                0.0,
                0.05,
                false,
                0.58,
                sword.profile,
            );
            commands.entity(entity).despawn();
        } else {
            transform.translation += offset.normalize_or_zero() * sword.speed * delta;
            transform.rotate_y(delta * 12.0);
        }
    }
}

fn update_blood_wheels(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    mut runtime: ResMut<SkillRuntime>,
    mut wheels: Query<(Entity, &mut BloodWheel, &mut Transform), Without<Enemy>>,
    enemies: Query<(Entity, &Transform, &Enemy), Without<BloodWheel>>,
) {
    if session.paused || session.finished {
        return;
    }
    let delta = time.delta_secs();
    for (wheel_entity, mut wheel, mut transform) in &mut wheels {
        let step = wheel.speed * delta;
        let leg_returning = wheel.returning;
        if leg_returning {
            wheel.traveled -= step;
            transform.translation -= wheel.direction * step;
        } else {
            wheel.traveled += step;
            transform.translation += wheel.direction * step;
            if wheel.traveled >= wheel.max_distance {
                wheel.traveled = wheel.max_distance;
                wheel.returning = true;
            }
        }
        transform.rotate_z(delta * 10.0);
        for (enemy_entity, enemy_transform, enemy) in &enemies {
            if enemy.hp <= 0.0
                || transform
                    .translation
                    .distance_squared(enemy_transform.translation)
                    > (0.82 * transform.scale.x / 1.15).powi(2)
            {
                continue;
            }
            let first_leg_hit = if leg_returning {
                register_single_leg_hit(&mut wheel.return_hits, enemy_entity)
            } else {
                register_single_leg_hit(&mut wheel.outbound_hits, enemy_entity)
            };
            if !first_leg_hit {
                continue;
            }
            schedule_typed_strike(
                &mut commands,
                enemy_entity,
                wheel.damage,
                0.0,
                0.05,
                false,
                0.72,
                if leg_returning {
                    wheel.return_profile
                } else {
                    wheel.outbound_profile
                },
            );
            if leg_returning {
                add_burn(
                    &mut runtime.burns,
                    enemy_entity,
                    wheel.damage,
                    wheel.burn_cap,
                );
            }
        }
        if wheel.returning && wheel.traveled <= 0.0 {
            transform.translation = wheel.origin;
            commands.entity(wheel_entity).despawn();
        }
    }
}

fn resolve_transmute_kills(
    mut commands: Commands,
    mut session: ResMut<GameSession>,
    mut rng: ResMut<RunRng>,
    watches: Query<(Entity, &TransmuteKillWatch)>,
    enemies: Query<(), With<Enemy>>,
) {
    for (entity, watch) in &watches {
        if watch.expected_lethal && !enemies.contains(watch.target) && rng.unit() < watch.chance {
            session.gold = session.gold.saturating_add(watch.reward.max(1));
        }
        commands.entity(entity).despawn();
    }
}

fn update_orbit_blades(
    mut commands: Commands,
    session: Res<GameSession>,
    build: Res<PlayerBuild>,
    vitals: Res<HeroVitals>,
    mut runtime: ResMut<SkillRuntime>,
    hero: Query<&Transform, (With<Hero>, Without<OrbitBlade>)>,
    mut blades: Query<(&OrbitBlade, &mut Transform), (Without<Enemy>, Without<Hero>)>,
    enemies: Query<(Entity, &Transform, &Enemy), (Without<OrbitBlade>, Without<Hero>)>,
    evolution: Option<Res<super::skill_evolution::SkillEvolution>>,
) {
    if session.paused || session.finished || !vitals.can_act() {
        return;
    }
    let tier = evolution.as_ref().map_or(0, |e| e.tiers[6]);
    let rank = build.skills[6];
    if rank == 0 {
        return;
    }
    let Ok(hero) = hero.single() else {
        return;
    };
    let count = (orbit_blade_count(rank) + tier * 2).max(1);
    let damage = scaled_skill_damage(6, rank, &build, &vitals, &runtime);
    for (blade, mut transform) in &mut blades {
        let angle =
            session.elapsed * 2.8 + blade.slot as f32 * std::f32::consts::TAU / count as f32;
        let radius = if tier > 0 && blade.slot % 2 == 1 {
            2.65 + f32::from(tier) * 0.2
        } else {
            2.05
        } * rank_size(rank);
        transform.translation =
            hero.translation + Vec3::new(angle.cos() * radius, 0.48, angle.sin() * radius);
        transform.rotation = Quat::from_rotation_y(session.elapsed * 11.0 + blade.slot as f32);
        transform.scale = Vec3::splat(orbit_art::scale(rank));
        for (enemy_entity, enemy_transform, enemy) in &enemies {
            if enemy.hp <= 0.0
                || transform
                    .translation
                    .distance_squared(enemy_transform.translation)
                    > skill_effect_radius(6, rank).unwrap().powi(2)
                || runtime
                    .orbit_hit_cooldowns
                    .get(&enemy_entity)
                    .is_some_and(|remaining| *remaining > 0.0)
            {
                continue;
            }
            runtime.orbit_hit_cooldowns.insert(enemy_entity, 1.0);
            schedule_typed_strike(
                &mut commands,
                enemy_entity,
                damage,
                0.0,
                0.05,
                false,
                0.52,
                DamageProfile::for_skill(6),
            );
        }
    }
}

fn resolve_skill_strikes(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    assets: Option<Res<GameAssets>>,
    mut strikes: Query<(Entity, &mut SkillStrike)>,
    targets: Query<&Transform, With<Enemy>>,
    mut pools: ResMut<CombatPools>,
) {
    if session.paused || session.finished {
        return;
    }
    let Some(assets) = assets else {
        return;
    };
    for (entity, mut strike) in &mut strikes {
        strike.delay -= time.delta_secs();
        if strike.delay > 0.0 {
            continue;
        }
        let position = if let Some(position) = strike.fixed_position {
            position
        } else {
            let Ok(target) = targets.get(strike.target) else {
                commands.entity(entity).despawn();
                continue;
            };
            target.translation
        };
        let Some(projectile_entity) = pools.rent_projectile(
            &mut commands,
            assets.projectile_mesh.clone(),
            assets.projectile_material.clone(),
            Transform::from_translation(position + Vec3::Y * 0.18)
                .with_scale(Vec3::splat(if strike.area { 0.8 } else { 0.32 })),
            Projectile {
                velocity: Vec3::ZERO,
                damage: strike.damage.max(1.0),
                radius: strike.radius,
                remaining_hits: if strike.area { u8::MAX } else { 0 },
                lifetime: 0.12,
            },
        ) else {
            continue;
        };
        commands.entity(projectile_entity).insert(strike.profile);
        for reward in &strike.transmute_rewards {
            commands.spawn(TransmuteKillWatch {
                target: reward.target,
                expected_lethal: reward.expected_lethal,
                chance: reward.chance,
                reward: reward.reward,
            });
        }
        if let Some(impact) = pools.rent_impact_fx(
            &mut commands,
            assets.fx_mesh.clone(),
            assets.storm_material.clone(),
            Transform::from_xyz(position.x, 0.16, position.z).with_scale(Vec3::splat(0.12)),
            ImpactFx {
                lifetime: 0.24,
                start_scale: 0.12,
                end_scale: strike.fx_scale,
            },
        ) {
            commands
                .entity(impact)
                .insert(super::skill_lighting::GroundLightProfile(strike.profile));
        }
        commands.entity(entity).despawn();
    }
}

fn animate_skill_visuals(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    mut bolts: Query<(Entity, &mut SkillVisualBolt, &mut Transform)>,
) {
    if session.paused {
        return;
    }
    for (entity, mut bolt, mut transform) in &mut bolts {
        transform.translation += bolt.velocity * time.delta_secs();
        transform.rotate_y(time.delta_secs() * 9.0);
        bolt.lifetime -= time.delta_secs();
        if bolt.lifetime <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn cast_learned_skills(
    mut commands: Commands,
    mut session: ResMut<GameSession>,
    build: Res<PlayerBuild>,
    mut vitals: ResMut<HeroVitals>,
    mut rng: ResMut<RunRng>,
    assets: Option<Res<GameAssets>>,
    attack_volleys: Res<AttackVolleyRuntime>,
    hero_query: Query<&Transform, With<Hero>>,
    enemy_query: Query<(
        Entity,
        &Transform,
        &Enemy,
        Option<&BossEnemy>,
        Option<&TrialEnemy>,
        Option<&ArchiveEcho>,
    )>,
    sentries: Query<(), With<BoneSentry>>,
    summons: Query<(), With<PlayerSummon>>,
    mut runtime: ResMut<SkillRuntime>,
    mut pools: ResMut<CombatPools>,
    market_effects: Res<MarketEffects>,
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
    let origin = hero.translation;
    let mut targets: Vec<TargetInfo> = enemy_query
        .iter()
        .filter(|(_, transform, enemy, _, _, _)| {
            enemy.hp > 0.0
                && origin.distance_squared(transform.translation)
                    <= (build.attack_range + 5.0).powi(2)
        })
        .map(|(entity, transform, enemy, boss, trial, archive)| {
            target_info(entity, transform, enemy, boss, trial, archive)
        })
        .collect();
    targets.sort_by(|left, right| {
        right.boss.cmp(&left.boss).then_with(|| {
            origin
                .distance_squared(left.position)
                .total_cmp(&origin.distance_squared(right.position))
        })
    });
    let primary = targets.first().copied();
    let mut summon_budget =
        SummonBudget::from_existing(summons.iter().count(), market_effects.summon_cap_bonus());

    apply_resolved_attack_volleys(
        &mut runtime,
        attack_volleys.resolved_this_frame,
        |runtime| {
            if let Some(target) = primary {
                cast_attack_procs(
                    &mut commands,
                    &assets,
                    &mut pools,
                    &build,
                    &vitals,
                    origin,
                    target,
                    runtime,
                );
            }
        },
    );

    if let Some(target) = primary {
        cast_arc_chain(
            &mut commands,
            &assets,
            &mut pools,
            &build,
            &vitals,
            origin,
            target,
            &targets,
            &mut runtime,
        );
        cast_frost_lance(
            &mut commands,
            &assets,
            &mut pools,
            &build,
            &vitals,
            origin,
            target,
            &targets,
            &mut runtime,
        );
        cast_tidal_fan(
            &mut commands,
            &assets,
            &mut pools,
            &build,
            &vitals,
            origin,
            target,
            &targets,
            &mut runtime,
        );
        cast_arcane_star(
            &mut commands,
            &assets,
            &mut pools,
            &build,
            &vitals,
            origin,
            target,
            &targets,
            &mut runtime,
        );
        cast_ground_pulse(
            &mut commands,
            &assets,
            &mut pools,
            &build,
            &vitals,
            origin,
            target,
            &targets,
            &mut runtime,
        );
    }

    cast_bone_sentry(
        &mut commands,
        &assets,
        &mut pools,
        &build,
        &vitals,
        origin,
        sentries.iter().count(),
        &mut summon_budget,
        &mut runtime,
    );
    cast_spirit_hounds(
        &mut commands,
        &assets,
        &mut pools,
        &build,
        &vitals,
        origin,
        &mut summon_budget,
        &mut runtime,
    );
    cast_cinder_aura(
        &mut commands,
        &assets,
        &mut pools,
        &build,
        &vitals,
        origin,
        &targets,
        &mut runtime,
    );

    if let Some(target) = primary {
        cast_rare_skills(
            &mut commands,
            &assets,
            &mut pools,
            &build,
            &vitals,
            origin,
            target,
            &targets,
            &mut summon_budget,
            &mut runtime,
        );
        cast_epic_skills(
            &mut commands,
            &assets,
            &mut pools,
            &mut session,
            &build,
            &mut vitals,
            &mut rng,
            origin,
            target,
            &targets,
            &mut summon_budget,
            &mut runtime,
        );
    }
}

fn apply_resolved_attack_volleys(
    runtime: &mut SkillRuntime,
    resolved: u32,
    mut on_attack: impl FnMut(&mut SkillRuntime),
) {
    for _ in 0..resolved {
        runtime.attack_count = runtime.attack_count.wrapping_add(1);
        on_attack(runtime);
    }
}

#[allow(clippy::too_many_arguments)]
fn cast_attack_procs(
    commands: &mut Commands,
    assets: &GameAssets,
    pools: &mut CombatPools,
    build: &PlayerBuild,
    vitals: &HeroVitals,
    origin: Vec3,
    target: TargetInfo,
    runtime: &mut SkillRuntime,
) {
    let ember_rank = build.skills[1];
    if ember_rank > 0
        && runtime
            .attack_count
            .is_multiple_of(ember_attack_interval(ember_rank) as u64)
    {
        let damage = scaled_skill_damage(1, ember_rank, build, vitals, runtime);
        let direction = (target.position - origin).normalize_or_zero();
        if spawn_typed_projectile(
            commands,
            assets,
            pools,
            origin + Vec3::Y * 0.3,
            direction,
            build.projectile_speed,
            damage,
            0.46 * rank_size(ember_rank),
            0,
            1.5 * rank_size(ember_rank),
            0.94 * rank_size(ember_rank),
            DamageProfile::for_skill(1).with_burn(),
        ) {
            let burn_cap = 3;
            add_burn(&mut runtime.burns, target.entity, damage, burn_cap);
        }
    }

    let volley_rank = build.skills[7];
    if volley_rank > 0 && runtime.attack_count.is_multiple_of(4) {
        let count = piercing_volley_count(volley_rank);
        let base_direction = (target.position - origin).normalize_or_zero();
        for shot in 0..count {
            let spread = shot as f32 - (count as f32 - 1.0) * 0.5;
            let direction = Quat::from_rotation_y(spread * 0.105) * base_direction;
            spawn_typed_projectile(
                commands,
                assets,
                pools,
                origin + Vec3::Y * 0.3,
                direction,
                build.projectile_speed * 1.08,
                build.attack_damage * 0.6,
                0.4 * rank_size(volley_rank),
                4,
                1.7 * rank_size(volley_rank),
                0.66 * rank_size(volley_rank),
                DamageProfile::for_skill(7),
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn cast_arc_chain(
    commands: &mut Commands,
    assets: &GameAssets,
    pools: &mut CombatPools,
    build: &PlayerBuild,
    vitals: &HeroVitals,
    origin: Vec3,
    first: TargetInfo,
    targets: &[TargetInfo],
    runtime: &mut SkillRuntime,
) {
    let rank = build.skills[0];
    if rank == 0 || runtime.cooldowns[0] > 0.0 {
        return;
    }
    runtime.cooldowns[0] = skill_cooldown(0);
    let damage = scaled_skill_damage(0, rank, build, vitals, runtime);
    let chain = chain_target_sequence(
        targets,
        first,
        arc_chain_hit_count(rank),
        skill_effect_radius(0, rank).unwrap(),
    );
    let mut from = origin;
    for (hop, target) in chain.iter().enumerate() {
        schedule_typed_strike(
            commands,
            target.entity,
            damage,
            hop as f32 * 0.09,
            0.05,
            false,
            0.58,
            DamageProfile::for_skill(0),
        );
        spawn_visual_bolt(
            commands,
            assets,
            from,
            target.position,
            22.0,
            0.38 * rank_size(rank),
        );
        from = target.position;
    }
    apply_static_seals(commands, runtime, chain, build, vitals, 0.04);
    record_spell_and_echo(
        commands,
        assets,
        pools,
        runtime,
        build,
        SpellRecord {
            skill_index: 0,
            target: first.entity,
            center: first.position,
            damage,
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn cast_frost_lance(
    commands: &mut Commands,
    assets: &GameAssets,
    pools: &mut CombatPools,
    build: &PlayerBuild,
    vitals: &HeroVitals,
    origin: Vec3,
    first: TargetInfo,
    targets: &[TargetInfo],
    runtime: &mut SkillRuntime,
) {
    let rank = build.skills[2];
    if rank == 0 || runtime.cooldowns[2] > 0.0 {
        return;
    }
    runtime.cooldowns[2] = skill_cooldown(2);
    let damage = scaled_skill_damage(2, rank, build, vitals, runtime);
    let base_direction = (first.position - origin).normalize_or_zero();
    let shots = frost_lance_count(rank);
    let mut controlled = HashSet::new();
    let mut marked = Vec::new();
    for shot in 0..shots {
        let spread = if shots == 1 {
            0.0
        } else if shot == 0 {
            -0.075
        } else {
            0.075
        };
        let direction = Quat::from_rotation_y(spread) * base_direction;
        if !spawn_typed_projectile(
            commands,
            assets,
            pools,
            origin + Vec3::Y * 0.34,
            direction,
            build.projectile_speed * 0.92,
            damage,
            0.5 * rank_size(rank),
            1,
            1.8 * rank_size(rank),
            0.86 * rank_size(rank),
            DamageProfile::for_skill(2),
        ) {
            continue;
        }
        for target in line_targets(
            origin,
            direction,
            targets,
            (build.attack_range + 5.0) * rank_size(rank),
            0.75 * rank_size(rank),
            2,
        ) {
            if controlled.insert(target.entity) {
                apply_control(runtime, target, 2.0, if rank >= 5 { 1.0 } else { 0.0 }, 0.0);
                marked.push(target);
            }
        }
    }
    apply_static_seals(commands, runtime, marked, build, vitals, 0.05);
    record_spell_and_echo(
        commands,
        assets,
        pools,
        runtime,
        build,
        SpellRecord {
            skill_index: 2,
            target: first.entity,
            center: first.position,
            damage,
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn cast_tidal_fan(
    commands: &mut Commands,
    assets: &GameAssets,
    pools: &mut CombatPools,
    build: &PlayerBuild,
    vitals: &HeroVitals,
    origin: Vec3,
    first: TargetInfo,
    _targets: &[TargetInfo],
    runtime: &mut SkillRuntime,
) {
    let rank = build.skills[3];
    if rank == 0 || runtime.cooldowns[3] > 0.0 {
        return;
    }
    runtime.cooldowns[3] = skill_cooldown(3);
    let damage = scaled_skill_damage(3, rank, build, vitals, runtime) * 2.0;
    tidal_wave::spawn(
        commands,
        origin,
        first.position - origin,
        rank,
        damage,
        runtime.evolution_tiers[3],
    );
    record_spell_and_echo(
        commands,
        assets,
        pools,
        runtime,
        build,
        SpellRecord {
            skill_index: 3,
            target: first.entity,
            center: first.position,
            damage,
        },
    );
}

fn cast_bone_sentry(
    commands: &mut Commands,
    assets: &GameAssets,
    pools: &mut CombatPools,
    build: &PlayerBuild,
    vitals: &HeroVitals,
    _origin: Vec3,
    existing_sentries: usize,
    summon_budget: &mut SummonBudget,
    runtime: &mut SkillRuntime,
) {
    let rank = build.skills[4];
    if rank == 0 || runtime.cooldowns[4] > 0.0 {
        return;
    }
    runtime.cooldowns[4] = skill_cooldown(4);
    let cap = if runtime.evolution_tiers[4] > 0 {
        1
    } else {
        bone_sentry_cap(rank)
    };
    if existing_sentries >= cap as usize || !summon_budget.take() {
        return;
    }
    let damage = scaled_skill_damage(4, rank, build, vitals, runtime);

    let Some(entity) = pools.rent_scene_summon(
        commands,
        SummonVisual::BoneSentry,
        assets.summon_scenes[0].clone(),
        Transform::from_translation(growth::bone_anchor(existing_sentries))
            .with_scale(growth::summon_scale(4, rank)),
    ) else {
        return;
    };
    commands.entity(entity).insert((
        SkillSummon,
        BoneSentry {
            attack_remaining: 0.15,
            damage,
        },
    ));
}

fn cast_spirit_hounds(
    commands: &mut Commands,
    assets: &GameAssets,
    pools: &mut CombatPools,
    build: &PlayerBuild,
    vitals: &HeroVitals,
    origin: Vec3,
    summon_budget: &mut SummonBudget,
    runtime: &mut SkillRuntime,
) {
    let rank = build.skills[5];
    if rank == 0 || runtime.cooldowns[5] > 0.0 {
        return;
    }
    runtime.cooldowns[5] = skill_cooldown(5);
    let count = spirit_hound_count(rank) as usize;
    let damage = scaled_skill_damage(5, rank, build, vitals, runtime);
    for index in 0..count {
        if !summon_budget.take() {
            break;
        }
        let angle = index as f32 * std::f32::consts::TAU / count as f32;
        let Some(entity) = pools.rent_scene_summon(
            commands,
            SummonVisual::SpiritHound,
            assets.summon_scenes[1].clone(),
            Transform::from_translation(Vec3::new(
                origin.x.max(super::resources::GATE_POSITION.x + 1.8) + angle.cos() * 0.9,
                0.0,
                angle.sin() * 0.9,
            ))
            .with_scale(growth::summon_scale(5, rank)),
        ) else {
            break;
        };
        commands.entity(entity).insert((
            SkillSummon,
            SpiritHound {
                source_skill: 5,
                remaining: 10.0,
                attack_remaining: index as f32 * 0.12,
                damage,
            },
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn cast_arcane_star(
    commands: &mut Commands,
    assets: &GameAssets,
    pools: &mut CombatPools,
    build: &PlayerBuild,
    vitals: &HeroVitals,
    origin: Vec3,
    first: TargetInfo,
    targets: &[TargetInfo],
    runtime: &mut SkillRuntime,
) {
    let rank = build.skills[8];
    if rank == 0 || runtime.cooldowns[8] > 0.0 {
        return;
    }
    runtime.cooldowns[8] = skill_cooldown(8);
    let damage = scaled_skill_damage(8, rank, build, vitals, runtime);
    schedule_typed_strike(
        commands,
        first.entity,
        damage,
        0.18,
        skill_effect_radius(8, rank).unwrap(),
        true,
        skill_effect_radius(8, rank).unwrap(),
        DamageProfile::for_skill(8),
    );
    spawn_visual_bolt(commands, assets, origin, first.position, 15.5, 0.72);
    let marked: Vec<TargetInfo> = targets
        .iter()
        .copied()
        .filter(|target| {
            target.position.distance_squared(first.position)
                <= skill_effect_radius(8, rank).unwrap().powi(2)
        })
        .collect();
    apply_static_seals(commands, runtime, marked, build, vitals, 0.18);
    record_spell_and_echo(
        commands,
        assets,
        pools,
        runtime,
        build,
        SpellRecord {
            skill_index: 8,
            target: first.entity,
            center: first.position,
            damage,
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn cast_ground_pulse(
    commands: &mut Commands,
    assets: &GameAssets,
    pools: &mut CombatPools,
    build: &PlayerBuild,
    vitals: &HeroVitals,
    origin: Vec3,
    first: TargetInfo,
    targets: &[TargetInfo],
    runtime: &mut SkillRuntime,
) {
    let rank = build.skills[9];
    if rank == 0 || runtime.cooldowns[9] > 0.0 {
        return;
    }
    runtime.cooldowns[9] = skill_cooldown(9);
    let damage = scaled_skill_damage(9, rank, build, vitals, runtime);
    let forward = (first.position - origin).normalize_or_zero();
    let hit: Vec<TargetInfo> = targets
        .iter()
        .copied()
        .filter(|target| {
            inside_cone(
                origin,
                forward,
                target.position,
                skill_effect_radius(9, rank).unwrap(),
                36.0_f32.to_radians(),
            )
        })
        .collect();
    for target in &hit {
        let delay = origin.distance(target.position) / 22.0;
        schedule_typed_strike(
            commands,
            target.entity,
            damage,
            delay,
            0.05,
            false,
            0.7,
            DamageProfile::for_skill(9),
        );
        apply_control(runtime, *target, 0.0, 0.0, 0.6);
    }
    let center = origin + forward * 2.4;
    if let Some(impact) = pools.rent_impact_fx(
        commands,
        assets.fx_mesh.clone(),
        assets.storm_material.clone(),
        Transform::from_xyz(center.x, 0.12, center.z).with_scale(Vec3::splat(0.2)),
        ImpactFx {
            lifetime: 0.34,
            start_scale: 0.2,
            end_scale: 2.5 * rank_size(rank),
        },
    ) {
        commands
            .entity(impact)
            .insert(super::skill_lighting::GroundLightProfile(
                DamageProfile::for_skill(9),
            ));
    }
    apply_static_seals(commands, runtime, hit, build, vitals, 0.08);
    record_spell_and_echo(
        commands,
        assets,
        pools,
        runtime,
        build,
        SpellRecord {
            skill_index: 9,
            target: first.entity,
            center,
            damage,
        },
    );
}

fn cast_cinder_aura(
    commands: &mut Commands,
    assets: &GameAssets,
    pools: &mut CombatPools,
    build: &PlayerBuild,
    vitals: &HeroVitals,
    origin: Vec3,
    targets: &[TargetInfo],
    runtime: &mut SkillRuntime,
) {
    let rank = build.skills[11];
    if rank == 0 || runtime.cooldowns[11] > 0.0 {
        return;
    }
    runtime.cooldowns[11] = BURN_TICK_SECONDS;
    let tick_damage = scaled_skill_damage(11, rank, build, vitals, runtime) * BURN_TICK_SECONDS;
    let nearby: Vec<TargetInfo> = targets
        .iter()
        .copied()
        .filter(|target| {
            origin.distance_squared(target.position)
                <= skill_effect_radius(11, rank).unwrap().powi(2)
        })
        .collect();
    for target in &nearby {
        schedule_typed_strike(
            commands,
            target.entity,
            tick_damage,
            0.0,
            0.05,
            false,
            0.42,
            DamageProfile::for_skill(11),
        );
    }
    apply_static_seals(
        commands,
        runtime,
        nearby.iter().copied(),
        build,
        vitals,
        0.0,
    );
    if let Some(target) = nearby.first() {
        record_spell_and_echo(
            commands,
            assets,
            pools,
            runtime,
            build,
            SpellRecord {
                skill_index: 11,
                target: target.entity,
                center: origin,
                damage: tick_damage,
            },
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn cast_rare_skills(
    commands: &mut Commands,
    assets: &GameAssets,
    pools: &mut CombatPools,
    build: &PlayerBuild,
    vitals: &HeroVitals,
    origin: Vec3,
    primary: TargetInfo,
    targets: &[TargetInfo],
    summon_budget: &mut SummonBudget,
    runtime: &mut SkillRuntime,
) {
    let storm_rank = build.skills[12];
    if storm_rank > 0 && runtime.cooldowns[12] <= 0.0 {
        runtime.cooldowns[12] = skill_cooldown(12);
        let damage = scaled_skill_damage(12, storm_rank, build, vitals, runtime);
        spawn_skill_zone(
            commands,
            assets,
            primary.position,
            ZoneKind::StormMatrix,
            4.0,
            0.6,
            damage,
            skill_effect_radius(12, storm_rank).unwrap(),
        );
        record_spell_and_echo(
            commands,
            assets,
            pools,
            runtime,
            build,
            SpellRecord {
                skill_index: 12,
                target: primary.entity,
                center: primary.position,
                damage,
            },
        );
    }

    let meteor_rank = build.skills[13];
    if meteor_rank > 0 && runtime.cooldowns[13] <= 0.0 {
        runtime.cooldowns[13] = skill_cooldown(13);
        let radius = skill_effect_radius(13, meteor_rank).unwrap();
        let damage = scaled_skill_damage(13, meteor_rank, build, vitals, runtime);
        schedule_typed_fixed_strike(
            commands,
            primary.position,
            damage,
            0.8,
            radius,
            true,
            radius,
            DamageProfile::for_skill(13),
        );
        if let Some(warning) = pools.rent_impact_fx(
            commands,
            assets.fx_mesh.clone(),
            assets.storm_material.clone(),
            Transform::from_xyz(primary.position.x, 0.1, primary.position.z)
                .with_scale(Vec3::splat(0.18)),
            ImpactFx {
                lifetime: 0.8,
                start_scale: 0.18,
                end_scale: radius,
            },
        ) {
            commands
                .entity(warning)
                .insert(super::skill_lighting::GroundLightProfile(
                    DamageProfile::for_skill(13),
                ));
        }
        let affected = targets
            .iter()
            .copied()
            .filter(|target| target.position.distance_squared(primary.position) <= radius.powi(2));
        apply_static_seals(commands, runtime, affected, build, vitals, 0.8);
        record_spell_and_echo(
            commands,
            assets,
            pools,
            runtime,
            build,
            SpellRecord {
                skill_index: 13,
                target: primary.entity,
                center: primary.position,
                damage,
            },
        );
    }

    let glacier_rank = build.skills[14];
    if glacier_rank > 0 && runtime.cooldowns[14] <= 0.0 {
        runtime.cooldowns[14] = skill_cooldown(14);
        let damage = scaled_skill_damage(14, glacier_rank, build, vitals, runtime);
        spawn_skill_zone(
            commands,
            assets,
            primary.position,
            ZoneKind::GlacierCore,
            5.0,
            0.8,
            damage,
            skill_effect_radius(14, glacier_rank).unwrap(),
        );
        record_spell_and_echo(
            commands,
            assets,
            pools,
            runtime,
            build,
            SpellRecord {
                skill_index: 14,
                target: primary.entity,
                center: primary.position,
                damage,
            },
        );
    }

    let vortex_rank = build.skills[15];
    if vortex_rank > 0 && runtime.cooldowns[15] <= 0.0 {
        runtime.cooldowns[15] = skill_cooldown(15);
        let total_damage = scaled_skill_damage(15, vortex_rank, build, vitals, runtime);
        spawn_skill_zone(
            commands,
            assets,
            primary.position,
            ZoneKind::VortexColumn,
            4.0,
            0.5,
            total_damage / 8.0,
            skill_effect_radius(15, vortex_rank).unwrap(),
        );
        record_spell_and_echo(
            commands,
            assets,
            pools,
            runtime,
            build,
            SpellRecord {
                skill_index: 15,
                target: primary.entity,
                center: primary.position,
                damage: total_damage,
            },
        );
    }

    let tide_rank = build.skills[16];
    if tide_rank > 0 && runtime.cooldowns[16] <= 0.0 {
        runtime.cooldowns[16] = skill_cooldown(16);
        if summon_budget.take()
            && let Some(entity) = pools.rent_scene_summon(
                commands,
                SummonVisual::TideElemental,
                assets.summon_scenes[2].clone(),
                Transform::from_translation(Vec3::new(
                    origin.x.max(super::resources::GATE_POSITION.x + 1.8),
                    0.0,
                    -0.9,
                ))
                .with_scale(growth::summon_scale(16, tide_rank)),
            )
        {
            commands.entity(entity).insert((
                SkillSummon,
                TideElemental {
                    remaining: 12.0,
                    attack_remaining: 0.2,
                    damage: scaled_skill_damage(16, tide_rank, build, vitals, runtime),
                },
            ));
            runtime.tide_repairs.insert(
                entity,
                tide_elemental_repair(tide_rank) * skill_utility_multiplier(16),
            );
        }
    }

    let turret_rank = build.skills[17];
    if turret_rank > 0 && runtime.cooldowns[17] <= 0.0 {
        runtime.cooldowns[17] = skill_cooldown(17);
        if summon_budget.take()
            && let Some(entity) = pools.rent_scene_summon(
                commands,
                SummonVisual::BoltTurret,
                assets.summon_scenes[3].clone(),
                Transform::from_translation(super::resources::GATE_POSITION + Vec3::Y * 2.1)
                    .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2))
                    .with_scale(growth::summon_scale(17, turret_rank)),
            )
        {
            commands.entity(entity).insert((
                SkillSummon,
                BoltTurret {
                    remaining: 16.0,
                    attack_remaining: 0.15,
                    damage: scaled_skill_damage(17, turret_rank, build, vitals, runtime),
                },
            ));
        }
    }

    let brand_rank = build.skills[18];
    if brand_rank > 0 && runtime.cooldowns[18] <= 0.0 {
        runtime.cooldowns[18] = skill_cooldown(18);
        if let Some(target) = targets
            .iter()
            .copied()
            .max_by(|left, right| left.max_hp.total_cmp(&right.max_hp))
        {
            commands.spawn(ExecutionBrand {
                infection_rank: brand_rank,
                infection_damage: infection::damage_per_tick(brand_rank, vitals.attack_damage()),
                target: target.entity,
                remaining: 8.0,
                threshold: execution_threshold(target.special),
            });
            if let Some(impact) = pools.rent_impact_fx(
                commands,
                assets.fx_mesh.clone(),
                assets.storm_material.clone(),
                Transform::from_xyz(target.position.x, 0.2, target.position.z)
                    .with_scale(Vec3::splat(0.15)),
                ImpactFx {
                    lifetime: 0.72,
                    start_scale: 0.15,
                    end_scale: 1.3,
                },
            ) {
                commands
                    .entity(impact)
                    .insert(super::skill_lighting::GroundLightProfile(
                        DamageProfile::for_skill(18),
                    ));
            }
        }
    }

    let swords_rank = build.skills[19];
    if swords_rank > 0 && runtime.cooldowns[19] <= 0.0 {
        runtime.cooldowns[19] = skill_cooldown(19);
        let count = flying_sword_count(swords_rank);
        let damage = scaled_skill_damage(19, swords_rank, build, vitals, runtime);
        let mut marked = Vec::new();
        for index in 0..count {
            let target = targets[index as usize % targets.len()];
            marked.push(target);
            commands.spawn((
                Mesh3d(assets.projectile_mesh.clone()),
                MeshMaterial3d(assets.projectile_material.clone()),
                Transform::from_translation(
                    origin + Vec3::new(0.0, 0.38, (index as f32 - 1.0) * 0.28),
                )
                .with_scale(Vec3::splat(0.62 * rank_size(swords_rank))),
                HomingSword {
                    target: target.entity,
                    damage,
                    speed: 18.5,
                    remaining: 2.0 * rank_size(swords_rank),
                    profile: DamageProfile::for_skill(19),
                },
            ));
        }
        apply_static_seals(commands, runtime, marked, build, vitals, 0.1);
        record_spell_and_echo(
            commands,
            assets,
            pools,
            runtime,
            build,
            SpellRecord {
                skill_index: 19,
                target: primary.entity,
                center: primary.position,
                damage,
            },
        );
    }

    let banner_rank = build.skills[21];
    if banner_rank > 0 && runtime.cooldowns[21] <= 0.0 {
        runtime.cooldowns[21] = skill_cooldown(21);
        if summon_budget.take()
            && let Some(entity) = pools.rent_mesh_summon(
                commands,
                SummonVisual::OathBanner,
                assets.fx_mesh.clone(),
                assets.storm_material.clone(),
                Transform::from_xyz(-9.0, 0.5, -2.3)
                    .with_scale(growth::summon_scale(21, banner_rank)),
            )
        {
            commands.entity(entity).insert((
                SkillSummon,
                OathBanner {
                    remaining: 10.0,
                    repair_remaining: 1.0,
                    damage_reduction: (oath_banner_reduction(banner_rank)
                        * skill_utility_multiplier(21))
                    .min(0.75),
                    applied: false,
                    armor_bonus: 0.0,
                    attack_speed_bonus: 0.0,
                },
            ));
        }
    }

    let transmute_rank = build.skills[22];
    if transmute_rank > 0 && runtime.cooldowns[22] <= 0.0 {
        runtime.cooldowns[22] = skill_cooldown(22);
        let damage = scaled_skill_damage(22, transmute_rank, build, vitals, runtime);
        let radius = skill_effect_radius(22, transmute_rank).unwrap();
        let affected: Vec<TargetInfo> = targets
            .iter()
            .copied()
            .filter(|target| target.position.distance_squared(primary.position) <= radius * radius)
            .collect();
        let rewards = affected
            .iter()
            .map(|target| TransmuteReward {
                target: target.entity,
                expected_lethal: target.hp <= damage,
                chance: (transmute_gold_chance(transmute_rank) * skill_utility_multiplier(22))
                    .min(1.0),
                reward: target.gold,
            })
            .collect();
        schedule_transmute_strike(commands, primary.entity, damage, radius, radius, rewards);
        apply_static_seals(
            commands,
            runtime,
            affected.iter().copied(),
            build,
            vitals,
            0.0,
        );
        record_spell_and_echo(
            commands,
            assets,
            pools,
            runtime,
            build,
            SpellRecord {
                skill_index: 22,
                target: primary.entity,
                center: primary.position,
                damage,
            },
        );
    }

    let wheel_rank = build.skills[23];
    if wheel_rank > 0 && runtime.cooldowns[23] <= 0.0 {
        runtime.cooldowns[23] = skill_cooldown(23);
        commands.spawn((
            Mesh3d(assets.fx_mesh.clone()),
            MeshMaterial3d(assets.storm_material.clone()),
            Transform::from_translation(origin + Vec3::Y * 0.4)
                .with_scale(Vec3::splat(1.15 * rank_size(wheel_rank))),
            BloodWheel {
                origin: origin + Vec3::Y * 0.4,
                direction: (primary.position - origin).normalize_or_zero(),
                speed: 14.0,
                max_distance: (build.attack_range + 2.0).clamp(7.0, 16.0) * rank_size(wheel_rank),
                traveled: 0.0,
                returning: false,
                damage: build.attack_damage * 0.9 * skill_utility_multiplier(23),
                burn_cap: 3,
                outbound_hits: HashSet::new(),
                return_hits: HashSet::new(),
                outbound_profile: DamageProfile::new(DamageElement::Physical, DamageClass::Attack),
                return_profile: DamageProfile::for_skill(23).with_burn(),
            },
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn cast_epic_skills(
    commands: &mut Commands,
    assets: &GameAssets,
    pools: &mut CombatPools,
    session: &mut GameSession,
    build: &PlayerBuild,
    vitals: &mut HeroVitals,
    rng: &mut RunRng,
    origin: Vec3,
    primary: TargetInfo,
    targets: &[TargetInfo],
    summon_budget: &mut SummonBudget,
    runtime: &mut SkillRuntime,
) {
    let judgment_rank = build.skills[24];
    if judgment_rank > 0 && runtime.cooldowns[24] <= 0.0 {
        runtime.cooldowns[24] = skill_cooldown(24);
        let damage = scaled_skill_damage(24, judgment_rank, build, vitals, runtime);
        let count = sky_judgment_target_count(judgment_rank);
        let mut struck = Vec::with_capacity(count as usize);
        for strike in 0..count {
            let target = targets[strike as usize % targets.len()];
            struck.push(target);
            schedule_typed_strike(
                commands,
                target.entity,
                damage,
                strike as f32 * 0.075,
                0.32,
                false,
                1.15,
                DamageProfile::for_skill(24),
            );
        }
        apply_static_seals(
            commands,
            runtime,
            struck.iter().copied(),
            build,
            vitals,
            0.04,
        );
        record_spell_and_echo(
            commands,
            assets,
            pools,
            runtime,
            build,
            SpellRecord {
                skill_index: 24,
                target: primary.entity,
                center: primary.position,
                damage,
            },
        );
    }

    let wyrm_rank = build.skills[26];
    if wyrm_rank > 0 && runtime.cooldowns[26] <= 0.0 {
        runtime.cooldowns[26] = skill_cooldown(26);
        if summon_budget.take()
            && let Some(entity) = pools.rent_scene_summon(
                commands,
                SummonVisual::FrostWyrm,
                assets.summon_scenes[4].clone(),
                Transform::from_translation(Vec3::new(
                    origin.x.max(super::resources::GATE_POSITION.x + 1.8),
                    0.0,
                    1.0,
                ))
                .with_scale(growth::summon_scale(26, wyrm_rank)),
            )
        {
            commands.entity(entity).insert((
                SkillSummon,
                FrostWyrm {
                    remaining: 20.0,
                    attack_remaining: 0.25,
                    damage: scaled_skill_damage(26, wyrm_rank, build, vitals, runtime),
                    frost_hits: HashMap::new(),
                },
            ));
        }
    }

    let colossus_rank = build.skills[27];
    if colossus_rank > 0 && runtime.cooldowns[27] <= 0.0 {
        runtime.cooldowns[27] = skill_cooldown(27);
        if summon_budget.take()
            && let Some(entity) = pools.rent_scene_summon(
                commands,
                SummonVisual::AncestorColossus,
                assets.summon_scenes[5].clone(),
                Transform::from_xyz(super::resources::GATE_POSITION.x + 2.2, 0.0, 0.0)
                    .with_scale(growth::summon_scale(27, colossus_rank)),
            )
        {
            commands.entity(entity).insert((
                SkillSummon,
                AncestorColossus {
                    remaining: 18.0,
                    slam_remaining: 3.0,
                    damage: scaled_skill_damage(27, colossus_rank, build, vitals, runtime),
                },
            ));
        }
    }

    let fate_rank = build.skills[29];
    if fate_rank > 0 && runtime.cooldowns[29] <= 0.0 {
        runtime.cooldowns[29] = skill_cooldown(29);
        let outcome = roll_fate_cube(fate_rank, rng.next_u32());
        let rank_value = fate_rank.saturating_sub(1) as f32;
        let fx_center = match outcome {
            FateOutcome::Lightning | FateOutcome::FireRain => primary.position,
            FateOutcome::Heal | FateOutcome::Gold | FateOutcome::Summon => origin,
        };
        match outcome {
            FateOutcome::Lightning => {
                let damage =
                    (80.0 + 40.0 * rank_value + 0.7 * effective_spell_power(vitals, runtime))
                        * skill_utility_multiplier(29);
                for strike in 0..6 {
                    let target = targets[strike % targets.len()];
                    schedule_typed_strike(
                        commands,
                        target.entity,
                        damage,
                        strike as f32 * 0.08,
                        0.3,
                        false,
                        1.0,
                        DamageProfile::new(DamageElement::Storm, DamageClass::Spell),
                    );
                }
            }
            FateOutcome::FireRain => {
                let damage = (130.0 + 65.0 * rank_value + effective_spell_power(vitals, runtime))
                    * skill_utility_multiplier(29);
                schedule_typed_fixed_strike(
                    commands,
                    primary.position,
                    damage,
                    0.35,
                    3.2,
                    true,
                    3.35,
                    DamageProfile::new(DamageElement::Flame, DamageClass::Spell).with_burn(),
                );
            }
            FateOutcome::Heal => {
                vitals.heal(
                    vitals.max_health
                        * ((0.12 + 0.03 * rank_value) * skill_utility_multiplier(29)).min(0.8),
                );
            }
            FateOutcome::Gold => {
                let base_reward = 40 + 20 * u32::from(fate_rank.saturating_sub(1));
                session.gold = session.gold.saturating_add(
                    (base_reward as f32 * skill_utility_multiplier(29)).round() as u32,
                );
            }
            FateOutcome::Summon => {
                let count = 2 + usize::from(fate_rank >= 4);
                for summon in 0..count {
                    if !summon_budget.take() {
                        break;
                    }
                    let angle = summon as f32 * std::f32::consts::TAU / count as f32;
                    let Some(entity) = pools.rent_scene_summon(
                        commands,
                        SummonVisual::SpiritHound,
                        assets.summon_scenes[1].clone(),
                        Transform::from_translation(Vec3::new(
                            origin.x.max(super::resources::GATE_POSITION.x + 1.8) + angle.cos(),
                            0.0,
                            angle.sin(),
                        ))
                        .with_scale(growth::summon_scale(29, fate_rank)),
                    ) else {
                        break;
                    };
                    commands.entity(entity).insert((
                        SkillSummon,
                        SpiritHound {
                            source_skill: 29,
                            remaining: 8.0,
                            attack_remaining: summon as f32 * 0.1,
                            damage: (45.0
                                + 18.0 * rank_value
                                + 0.35 * effective_summon_power(vitals, runtime))
                                * skill_utility_multiplier(29),
                        },
                    ));
                }
            }
        }
        if let Some(impact) = pools.rent_impact_fx(
            commands,
            assets.fx_mesh.clone(),
            assets.storm_material.clone(),
            Transform::from_xyz(fx_center.x, 0.2, fx_center.z).with_scale(Vec3::splat(0.16)),
            ImpactFx {
                lifetime: 0.62,
                start_scale: 0.16,
                end_scale: 2.4,
            },
        ) {
            let profile = match outcome {
                FateOutcome::Lightning => {
                    DamageProfile::new(DamageElement::Storm, DamageClass::Spell)
                }
                FateOutcome::FireRain => {
                    DamageProfile::new(DamageElement::Flame, DamageClass::Spell)
                }
                _ => DamageProfile::for_skill(29),
            };
            commands
                .entity(impact)
                .insert(super::skill_lighting::GroundLightProfile(profile));
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_skill_zone(
    commands: &mut Commands,
    assets: &GameAssets,
    center: Vec3,
    kind: ZoneKind,
    duration: f32,
    pulse_interval: f32,
    damage_per_pulse: f32,
    radius: f32,
) {
    commands.spawn((
        Name::new(match kind {
            ZoneKind::StormMatrix => "Storm matrix damage zone",
            ZoneKind::GlacierCore => "Glacier core damage zone",
            ZoneKind::VortexColumn => "Vortex column damage zone",
        }),
        Mesh3d(assets.fx_mesh.clone()),
        MeshMaterial3d(assets.storm_material.clone()),
        Transform::from_xyz(center.x, 0.12, center.z).with_scale(Vec3::splat(radius * 0.34)),
        SkillZone {
            kind,
            remaining: duration,
            pulse_remaining: pulse_interval,
            damage_per_pulse,
            radius,
            glacier_hits: HashMap::new(),
            profile: match kind {
                ZoneKind::StormMatrix => DamageProfile::for_skill(12),
                ZoneKind::GlacierCore => DamageProfile::for_skill(14),
                ZoneKind::VortexColumn => DamageProfile::for_skill(15),
            },
        },
    ));
}

fn record_spell_and_echo(
    commands: &mut Commands,
    assets: &GameAssets,
    pools: &mut CombatPools,
    runtime: &mut SkillRuntime,
    build: &PlayerBuild,
    record: SpellRecord,
) {
    runtime.last_spell = Some(record);
    if runtime.market_echo_chance > 0.0 {
        commands.spawn(MarketEchoRequest {
            record,
            chance: runtime.market_echo_chance,
        });
    }
    let fifth_cast = record_non_echo_spell(&mut runtime.spell_cast_count);

    if !fifth_cast {
        return;
    }
    let prism_rank = build.skills[20];
    let prism_scale = if prism_rank > 0 {
        (prism_echo_scale(prism_rank) * skill_utility_multiplier(20)).min(1.0)
    } else {
        0.0
    };
    let scale = prism_scale;
    if scale > 0.0 {
        let replay = runtime
            .last_spell
            .expect("the current spell was just recorded");
        spawn_echo_replay(commands, pools, assets, replay, scale);
    }
}

fn resolve_market_echoes(
    mut commands: Commands,
    session: Res<GameSession>,
    assets: Option<Res<GameAssets>>,
    mut pools: ResMut<CombatPools>,
    mut rng: ResMut<RunRng>,
    requests: Query<(Entity, &MarketEchoRequest)>,
) {
    if session.paused || session.finished {
        return;
    }
    let Some(assets) = assets else {
        return;
    };
    let mut pending: Vec<(Entity, SpellRecord, f32)> = requests
        .iter()
        .map(|(entity, request)| (entity, request.record, request.chance))
        .collect();
    pending.sort_by_key(|(entity, _, _)| entity.to_bits());
    for (entity, record, chance) in pending {
        if market_echo_triggers(chance, rng.unit()) {
            spawn_echo_replay(
                &mut commands,
                &mut pools,
                &assets,
                record,
                MARKET_ECHO_DAMAGE_SCALE,
            );
        }
        commands.entity(entity).despawn();
    }
}

fn market_echo_triggers(chance: f32, roll: f32) -> bool {
    chance > 0.0 && roll < chance.clamp(0.0, 1.0)
}

fn spawn_echo_replay(
    commands: &mut Commands,
    pools: &mut CombatPools,
    assets: &GameAssets,
    replay: SpellRecord,
    scale: f32,
) {
    let damage = replay.damage * scale;
    match replay.skill_index {
        12 => spawn_skill_zone(
            commands,
            assets,
            replay.center,
            ZoneKind::StormMatrix,
            4.0,
            0.6,
            damage,
            3.1,
        ),
        13 => {
            schedule_typed_fixed_strike(
                commands,
                replay.center,
                damage,
                0.8,
                3.2,
                true,
                3.4,
                DamageProfile::for_skill(13),
            );
            if let Some(warning) = pools.rent_impact_fx(
                commands,
                assets.fx_mesh.clone(),
                assets.storm_material.clone(),
                Transform::from_xyz(replay.center.x, 0.1, replay.center.z)
                    .with_scale(Vec3::splat(0.16)),
                ImpactFx {
                    lifetime: 0.8,
                    start_scale: 0.16,
                    end_scale: 3.2,
                },
            ) {
                commands
                    .entity(warning)
                    .insert(super::skill_lighting::GroundLightProfile(
                        DamageProfile::for_skill(13),
                    ));
            }
        }
        14 => spawn_skill_zone(
            commands,
            assets,
            replay.center,
            ZoneKind::GlacierCore,
            5.0,
            0.8,
            damage,
            2.8,
        ),
        15 => spawn_skill_zone(
            commands,
            assets,
            replay.center,
            ZoneKind::VortexColumn,
            4.0,
            0.5,
            damage / 8.0,
            3.4,
        ),
        8 | 22 => schedule_typed_strike(
            commands,
            replay.target,
            damage,
            0.08,
            2.2,
            true,
            2.3,
            DamageProfile::for_skill(replay.skill_index),
        ),
        _ => schedule_typed_strike(
            commands,
            replay.target,
            damage,
            0.08,
            0.05,
            false,
            0.74,
            DamageProfile::for_skill(replay.skill_index),
        ),
    }
}

fn apply_banner_bonuses(banner: &mut OathBanner, build: &mut PlayerBuild, vitals: &mut HeroVitals) {
    if banner.applied {
        return;
    }
    banner.armor_bonus = armor_bonus_for_reduction(vitals.armor, banner.damage_reduction);
    banner.attack_speed_bonus = build.attacks_per_second * 0.15 * skill_utility_multiplier(21);
    vitals.armor += banner.armor_bonus;
    build.attacks_per_second += banner.attack_speed_bonus;
    banner.applied = true;
}

fn remove_banner_bonuses(
    banner: &mut OathBanner,
    build: &mut PlayerBuild,
    vitals: &mut HeroVitals,
) {
    if !banner.applied {
        return;
    }
    vitals.armor -= banner.armor_bonus;
    build.attacks_per_second = (build.attacks_per_second - banner.attack_speed_bonus).max(0.2);
    banner.applied = false;
    banner.armor_bonus = 0.0;
    banner.attack_speed_bonus = 0.0;
}

fn target_info(
    entity: Entity,
    transform: &Transform,
    enemy: &Enemy,
    boss: Option<&BossEnemy>,
    trial: Option<&TrialEnemy>,
    archive: Option<&ArchiveEcho>,
) -> TargetInfo {
    TargetInfo {
        entity,
        position: transform.translation,
        speed: enemy.speed,
        elite: enemy.elite,
        boss: boss.is_some(),
        special: enemy.elite || boss.is_some() || trial.is_some() || archive.is_some(),
        hp: enemy.hp,
        max_hp: enemy.max_hp,
        gold: enemy.gold,
    }
}

/// Shared collision and presentation growth; rank 5 doubles rank 1 geometry.
pub(crate) fn summon_size_multiplier(index: usize, rank: u8) -> f32 {
    growth::summon_scale(index, rank).x / growth::summon_scale(index, 1).x
}

pub(crate) fn rank_size(rank: u8) -> f32 {
    1.0 + f32::from(rank.clamp(1, 5) - 1) * 0.25
}

/// Radius (or cone reach) used by actual combat, not an illustrative UI value.
pub(crate) fn skill_effect_radius(index: usize, rank: u8) -> Option<f32> {
    let base = match index {
        0 => 5.5,
        6 => 0.85,
        8 => 2.0,
        9 => 5.4,
        10 => 2.1,
        11 => 2.6,
        12 => 3.1,
        13 => 3.2,
        14 => 2.8,
        15 => 3.4,
        22 => 2.8,
        25 => 3.6,
        27 => 5.2,
        18 => return Some(3.0 + 0.25 * f32::from(rank.clamp(1, 5))),
        31 => return Some(2.0 + 0.2 * f32::from(rank.clamp(1, 5))),
        _ => return None,
    };
    Some(base * rank_size(rank))
}

pub(crate) fn scaled_skill_damage(
    index: usize,
    rank: u8,
    build: &PlayerBuild,
    vitals: &HeroVitals,
    runtime: &SkillRuntime,
) -> f32 {
    let power = SKILLS[index].effect.power;
    let rank_value = rank.saturating_sub(1) as f32;
    let base_attack_damage = (build.attack_damage - runtime.triune.attack_bonus).max(1.0);
    ((power.base
        + power.per_rank * rank_value
        + power.attack_ratio * base_attack_damage
        + power.spell_ratio * effective_spell_power(vitals, runtime)
        + power.summon_ratio * effective_summon_power(vitals, runtime))
    .max(1.0)
        * SKILLS[index].rarity.skill_power_multiplier())
        * triune_skill_multiplier(index, &runtime.triune)
}

pub(crate) fn skill_preview_damage(
    index: usize,
    rank: u8,
    build: &PlayerBuild,
    vitals: &HeroVitals,
    runtime: &SkillRuntime,
) -> Option<f32> {
    match index {
        7 => Some(build.attack_damage * 0.6),
        18 => Some(infection::damage_per_tick(rank, vitals.attack_damage())),
        20 | 21 | 28 | 29 => None,
        _ => Some(scaled_skill_damage(index, rank, build, vitals, runtime)),
    }
}

pub(crate) fn skill_cooldown(index: usize) -> f32 {
    let skill = SKILLS[index];
    skill.cooldown_secs * skill.rarity.skill_cooldown_multiplier()
}

const fn skill_utility_multiplier(index: usize) -> f32 {
    SKILLS[index].rarity.skill_power_multiplier()
}

pub(crate) fn effective_spell_power(vitals: &HeroVitals, runtime: &SkillRuntime) -> f32 {
    vitals.spell_power() * runtime.spell_power_multiplier
}

pub(crate) fn effective_summon_power(vitals: &HeroVitals, runtime: &SkillRuntime) -> f32 {
    let inherited = effective_spell_power(vitals, runtime) * 0.7 + vitals.attack_damage() * 0.5;
    inherited * runtime.summon_inherit_multiplier
}

fn apply_static_seals(
    commands: &mut Commands,
    runtime: &mut SkillRuntime,
    targets: impl IntoIterator<Item = TargetInfo>,
    build: &PlayerBuild,
    vitals: &HeroVitals,
    delay: f32,
) {
    let rank = build.skills[10];
    if rank == 0 {
        return;
    }
    let explosion_damage = scaled_skill_damage(10, rank, build, vitals, runtime);
    let mut unique = HashSet::new();
    for target in targets {
        if !unique.insert(target.entity) {
            continue;
        }
        let stacks = runtime.seals.entry(target.entity).or_default();
        if advance_static_seal(stacks) {
            schedule_typed_strike(
                commands,
                target.entity,
                explosion_damage,
                delay + 0.04,
                skill_effect_radius(10, rank).unwrap(),
                true,
                skill_effect_radius(10, rank).unwrap(),
                DamageProfile::for_skill(10),
            );
        }
    }
}

fn apply_control(
    runtime: &mut SkillRuntime,
    target: TargetInfo,
    slow_seconds: f32,
    freeze_seconds: f32,
    stun_seconds: f32,
) {
    let duration_scale = control_duration_scale(target.elite, target.boss);
    let control = runtime
        .controls
        .entry(target.entity)
        .or_insert(ControlState {
            base_speed: target.speed,
            slow_remaining: 0.0,
            frozen_remaining: 0.0,
            stunned_remaining: 0.0,
        });
    control.slow_remaining = control.slow_remaining.max(slow_seconds * duration_scale);
    control.frozen_remaining = control
        .frozen_remaining
        .max(freeze_seconds * duration_scale);
    control.stunned_remaining = control.stunned_remaining.max(stun_seconds * duration_scale);
}

fn add_burn(burns: &mut HashMap<Entity, BurnState>, entity: Entity, total_damage: f32, cap: u8) {
    match burns.get_mut(&entity) {
        Some(burn) => add_burn_layer(burn, total_damage, cap),
        None => {
            burns.insert(entity, BurnState::new(total_damage));
        }
    }
}

fn add_burn_layer(burn: &mut BurnState, total_damage: f32, cap: u8) {
    if burn.layers < cap.max(1) {
        burn.layers += 1;
        burn.damage_per_tick += total_damage / 6.0;
    }
    burn.remaining = 3.0;
}

#[allow(clippy::too_many_arguments)]
fn schedule_typed_strike(
    commands: &mut Commands,
    target: Entity,
    damage: f32,
    delay: f32,
    radius: f32,
    area: bool,
    fx_scale: f32,
    profile: DamageProfile,
) {
    commands.spawn(SkillStrike {
        target,
        fixed_position: None,
        delay,
        damage,
        radius,
        area,
        fx_scale,
        transmute_rewards: Vec::new(),
        profile,
    });
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn schedule_typed_fixed_strike(
    commands: &mut Commands,
    position: Vec3,
    damage: f32,
    delay: f32,
    radius: f32,
    area: bool,
    fx_scale: f32,
    profile: DamageProfile,
) {
    commands.spawn(SkillStrike {
        target: Entity::PLACEHOLDER,
        fixed_position: Some(position),
        delay,
        damage,
        radius,
        area,
        fx_scale,
        transmute_rewards: Vec::new(),
        profile,
    });
}

fn schedule_transmute_strike(
    commands: &mut Commands,
    target: Entity,
    damage: f32,
    radius: f32,
    fx_scale: f32,
    transmute_rewards: Vec<TransmuteReward>,
) {
    commands.spawn(SkillStrike {
        target,
        fixed_position: None,
        delay: 0.0,
        damage,
        radius,
        area: true,
        fx_scale,
        transmute_rewards,
        profile: DamageProfile::for_skill(22),
    });
}

#[allow(clippy::too_many_arguments)]
fn spawn_typed_projectile(
    commands: &mut Commands,
    assets: &GameAssets,
    pools: &mut CombatPools,
    origin: Vec3,
    direction: Vec3,
    speed: f32,
    damage: f32,
    radius: f32,
    remaining_hits: u8,
    lifetime: f32,
    scale: f32,
    profile: DamageProfile,
) -> bool {
    let Some(entity) = pools.rent_projectile(
        commands,
        assets.projectile_mesh.clone(),
        assets.projectile_material.clone(),
        Transform::from_translation(origin).with_scale(Vec3::splat(scale)),
        Projectile {
            velocity: direction.normalize_or_zero() * speed,
            damage: damage.max(1.0),
            radius,
            remaining_hits,
            lifetime,
        },
    ) else {
        return false;
    };
    commands.entity(entity).insert(profile);
    true
}

fn spawn_visual_bolt(
    commands: &mut Commands,
    assets: &GameAssets,
    origin: Vec3,
    target: Vec3,
    speed: f32,
    scale: f32,
) {
    let delta = target - origin;
    let lifetime = (delta.length() / speed.max(0.1)).clamp(0.08, 0.45);
    spawn_free_visual_bolt(
        commands,
        assets,
        origin,
        delta.normalize_or_zero(),
        speed,
        scale,
        lifetime,
    );
}

fn spawn_free_visual_bolt(
    commands: &mut Commands,
    assets: &GameAssets,
    origin: Vec3,
    direction: Vec3,
    speed: f32,
    scale: f32,
    lifetime: f32,
) {
    commands.spawn((
        Mesh3d(assets.projectile_mesh.clone()),
        MeshMaterial3d(assets.projectile_material.clone()),
        Transform::from_translation(origin + Vec3::Y * 0.28).with_scale(Vec3::splat(scale)),
        SkillVisualBolt {
            velocity: direction.normalize_or_zero() * speed,
            lifetime,
        },
    ));
}

fn nearest_position(origin: Vec3, targets: &[(Entity, Vec3)]) -> Option<(Entity, Vec3)> {
    targets.iter().copied().min_by(|left, right| {
        origin
            .distance_squared(left.1)
            .total_cmp(&origin.distance_squared(right.1))
    })
}

fn line_targets(
    origin: Vec3,
    direction: Vec3,
    targets: &[TargetInfo],
    max_distance: f32,
    half_width: f32,
    limit: usize,
) -> Vec<TargetInfo> {
    let direction = Vec2::new(direction.x, direction.z).normalize_or_zero();
    let mut candidates: Vec<(f32, TargetInfo)> = targets
        .iter()
        .copied()
        .filter_map(|target| {
            let delta = Vec2::new(target.position.x - origin.x, target.position.z - origin.z);
            let projection = delta.dot(direction);
            let perpendicular = (delta - direction * projection).length();
            (projection >= 0.0 && projection <= max_distance && perpendicular <= half_width)
                .then_some((projection, target))
        })
        .collect();
    candidates.sort_by(|left, right| left.0.total_cmp(&right.0));
    candidates
        .into_iter()
        .take(limit)
        .map(|(_, target)| target)
        .collect()
}

fn chain_target_sequence(
    targets: &[TargetInfo],
    first: TargetInfo,
    max_hits: u8,
    hop_radius: f32,
) -> Vec<TargetInfo> {
    let mut chain = vec![first];
    let mut visited = HashSet::from([first.entity]);
    while chain.len() < max_hits as usize {
        let current = chain.last().expect("chain starts with one target");
        let Some(next) = targets
            .iter()
            .copied()
            .filter(|candidate| {
                !visited.contains(&candidate.entity)
                    && current.position.distance_squared(candidate.position) <= hop_radius.powi(2)
            })
            .min_by(|left, right| {
                current
                    .position
                    .distance_squared(left.position)
                    .total_cmp(&current.position.distance_squared(right.position))
            })
        else {
            break;
        };
        visited.insert(next.entity);
        chain.push(next);
    }
    chain
}

fn inside_cone(origin: Vec3, forward: Vec3, point: Vec3, range: f32, half_angle: f32) -> bool {
    let forward = Vec2::new(forward.x, forward.z).normalize_or_zero();
    let delta = Vec2::new(point.x - origin.x, point.z - origin.z);
    let distance = delta.length();
    distance <= range
        && distance > f32::EPSILON
        && forward.dot(delta / distance) >= half_angle.cos()
}

pub(crate) const fn arc_chain_hit_count(rank: u8) -> u8 {
    3 + rank.saturating_sub(1) / 2
}

const fn ember_attack_interval(rank: u8) -> u8 {
    if rank >= 5 { 2 } else { 3 }
}

pub(crate) const fn frost_lance_count(rank: u8) -> u8 {
    if rank >= 3 { 2 } else { 1 }
}

pub(crate) const fn bone_sentry_cap(rank: u8) -> u8 {
    rank.div_ceil(2)
}

pub(crate) const fn spirit_hound_count(rank: u8) -> u8 {
    1 + rank.div_ceil(2)
}

const fn forced_movement_scale(elite: bool, boss: bool) -> f32 {
    if boss {
        0.0
    } else if elite {
        0.5
    } else {
        1.0
    }
}

pub(crate) const fn orbit_blade_count(rank: u8) -> u8 {
    2 + rank.saturating_sub(1) / 2
}

pub(crate) const fn piercing_volley_count(rank: u8) -> u8 {
    3 + rank.saturating_sub(1) / 2
}

pub(crate) const fn flying_sword_count(rank: u8) -> u8 {
    1 + rank.saturating_sub(1) / 2
}

const fn tide_elemental_repair(rank: u8) -> f32 {
    8.0 + 4.0 * rank.saturating_sub(1) as f32
}

const fn oath_banner_reduction(rank: u8) -> f32 {
    0.10 + 0.03 * rank.saturating_sub(1) as f32
}

fn armor_bonus_for_reduction(base_armor: f32, reduction: f32) -> f32 {
    let effective_base = base_armor.max(0.0);
    let target_armor =
        ((100.0 + 5.0 * effective_base) / (1.0 - reduction.clamp(0.0, 0.9)) - 100.0) / 5.0;
    target_armor - base_armor
}

const fn zone_pulse_interval(kind: ZoneKind) -> f32 {
    match kind {
        ZoneKind::StormMatrix => 0.6,
        ZoneKind::GlacierCore => 0.8,
        ZoneKind::VortexColumn => 0.5,
    }
}

const fn execution_threshold(special: bool) -> f32 {
    let base = if special { 0.04 } else { 0.12 };
    base * skill_utility_multiplier(18)
}

fn execution_ready(health: f32, max_health: f32, threshold: f32) -> bool {
    max_health > 0.0 && health / max_health <= threshold
}

const fn transmute_gold_chance(rank: u8) -> f32 {
    0.04 + 0.02 * rank.saturating_sub(1) as f32
}

const fn prism_echo_scale(rank: u8) -> f32 {
    0.35 + 0.10 * rank.saturating_sub(1) as f32
}

pub(crate) const fn sky_judgment_target_count(rank: u8) -> u8 {
    6 + rank.saturating_sub(1)
}

fn ember_rebirth_ready(rank: u8, cooldown: f32, health: f32) -> bool {
    rank > 0 && cooldown <= 0.0 && health <= 0.0
}

const fn triune_boost(rank: u8) -> f32 {
    (0.20 + 0.05 * rank.saturating_sub(1) as f32) * skill_utility_multiplier(28)
}

fn triune_attack_active(state: &TriuneState) -> bool {
    state.rank >= 5 || (state.rank > 0 && state.phase == 0)
}

fn triune_category_active(state: &TriuneState, category: u8) -> bool {
    state.rank >= 5 || (state.rank > 0 && state.phase == category)
}

fn triune_skill_multiplier(index: usize, state: &TriuneState) -> f32 {
    let tags = SKILLS[index].tags;
    let category = if tags.contains(&Tag::Summon) {
        Some(2)
    } else if tags.contains(&Tag::Spell) {
        Some(1)
    } else if tags.contains(&Tag::Attack) {
        Some(0)
    } else {
        None
    };
    if category.is_some_and(|category| triune_category_active(state, category)) {
        1.0 + triune_boost(state.rank)
    } else {
        1.0
    }
}

fn apply_triune_attack_bonus(state: &mut TriuneState, build: &mut PlayerBuild) {
    if state.attack_bonus != 0.0 || !triune_attack_active(state) {
        return;
    }
    state.attack_bonus = build.attack_damage * triune_boost(state.rank);
    build.attack_damage += state.attack_bonus;
}

fn remove_triune_attack_bonus(state: &mut TriuneState, build: &mut PlayerBuild) {
    if state.attack_bonus == 0.0 {
        return;
    }
    build.attack_damage = (build.attack_damage - state.attack_bonus).max(1.0);
    state.attack_bonus = 0.0;
}

const fn fate_cube_weights(rank: u8) -> [u32; 5] {
    let high = 10 + 5 * rank.saturating_sub(1) as u32;
    [high, high, 25, 25, high]
}

fn roll_fate_cube(rank: u8, roll: u32) -> FateOutcome {
    let weights = fate_cube_weights(rank);
    let total: u32 = weights.iter().sum();
    let mut cursor = roll % total;
    for (index, weight) in weights.into_iter().enumerate() {
        if cursor < weight {
            return match index {
                0 => FateOutcome::Lightning,
                1 => FateOutcome::FireRain,
                2 => FateOutcome::Heal,
                3 => FateOutcome::Gold,
                _ => FateOutcome::Summon,
            };
        }
        cursor -= weight;
    }
    FateOutcome::Summon
}

fn record_non_echo_spell(spell_cast_count: &mut u64) -> bool {
    *spell_cast_count = spell_cast_count.wrapping_add(1);
    spell_cast_count.is_multiple_of(5)
}

fn register_single_leg_hit(hits: &mut HashSet<Entity>, target: Entity) -> bool {
    hits.insert(target)
}

const fn control_duration_scale(elite: bool, boss: bool) -> f32 {
    if boss {
        0.2
    } else if elite {
        0.5
    } else {
        1.0
    }
}

fn advance_static_seal(stacks: &mut u8) -> bool {
    *stacks += 1;
    if *stacks >= 4 {
        *stacks = 0;
        true
    } else {
        false
    }
}

pub(crate) fn reset_run(world: &mut World) {
    world.insert_resource(SkillRuntime::default());
    evolution_mechanics::reset_run(world);
    ur_evolutions::reset_run(world);
    presentation::reset_run(world);
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    #[test]
    fn yellow_sign_halves_real_skill_cooldown_recovery_and_releases_on_exit() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<GameSession>()
            .init_resource::<MarketEffects>()
            .init_resource::<super::super::encounters::EncounterState>()
            .init_resource::<SkillRuntime>()
            .insert_resource(super::super::combat::ElitePressure {
                yellow_sign_active: true,
            });
        app.world_mut().resource_mut::<SkillRuntime>().cooldowns[0] = 10.0;
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(2));
        app.world_mut().run_system_once(tick_skill_runtime).unwrap();
        assert_eq!(app.world().resource::<SkillRuntime>().cooldowns[0], 9.0);
        app.world_mut()
            .resource_mut::<super::super::combat::ElitePressure>()
            .yellow_sign_active = false;
        app.world_mut().run_system_once(tick_skill_runtime).unwrap();
        assert_eq!(app.world().resource::<SkillRuntime>().cooldowns[0], 7.0);
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.world_mut().run_system_once(tick_skill_runtime).unwrap();
        assert_eq!(app.world().resource::<SkillRuntime>().cooldowns[0], 7.0);
    }

    #[test]
    fn real_meteor_strike_transmits_cosmetic_element_without_changing_damage() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<GameSession>()
            .init_resource::<CombatPools>()
            .insert_resource(GameAssets {
                hero_scenes: std::array::from_fn(|_| Handle::default()),
                enemy_scenes: std::array::from_fn(|_| Handle::default()),
                boss_scenes: std::array::from_fn(|_| Handle::default()),
                summon_scenes: std::array::from_fn(|_| Handle::default()),
                projectile_mesh: Handle::default(),
                fx_mesh: Handle::default(),
                enemy_material: Handle::default(),
                enemy_materials: std::array::from_fn(|_| Handle::default()),
                elite_material: Handle::default(),
                boss_materials: std::array::from_fn(|_| Handle::default()),
                hero_materials: std::array::from_fn(|_| Handle::default()),
                projectile_material: Handle::default(),
                storm_material: Handle::default(),
            })
            .add_systems(Update, resolve_skill_strikes);
        app.world_mut().spawn(SkillStrike {
            target: Entity::PLACEHOLDER,
            fixed_position: Some(Vec3::new(2.0, 0.0, 1.0)),
            delay: 0.0,
            damage: 123.0,
            radius: 3.2,
            area: true,
            fx_scale: 3.2,
            transmute_rewards: Vec::new(),
            profile: DamageProfile::for_skill(13),
        });
        app.update();
        let projectile = app
            .world_mut()
            .query::<(&Projectile, &DamageProfile)>()
            .single(app.world())
            .unwrap();
        assert_eq!(projectile.0.damage, 123.0);
        assert_eq!(projectile.0.radius, 3.2);
        assert_eq!(
            projectile.1.element,
            super::super::bonds::DamageElement::Flame
        );
        let (impact, light) = app
            .world_mut()
            .query::<(&ImpactFx, &super::super::skill_lighting::GroundLightProfile)>()
            .single(app.world())
            .unwrap();
        assert_eq!(impact.end_scale, 3.2);
        assert_eq!(light.0, DamageProfile::for_skill(13));
        // The warning appears before damage resolves; exercise both real cast
        // and echo paths so the long pre-impact ring cannot silently stay blue.
        app.world_mut()
            .run_system_once(
                |mut commands: Commands,
                 mut pools: ResMut<CombatPools>,
                 assets: Res<GameAssets>| {
                    let mut build = PlayerBuild::default();
                    build.skills.fill(0);
                    build.skills[13] = 5;
                    let target = TargetInfo {
                        entity: Entity::PLACEHOLDER,
                        position: Vec3::new(2.0, 0.0, 1.0),
                        speed: 1.0,
                        elite: false,
                        boss: false,
                        special: false,
                        hp: 500.0,
                        max_hp: 500.0,
                        gold: 1,
                    };
                    cast_rare_skills(
                        &mut commands,
                        &assets,
                        &mut pools,
                        &build,
                        &HeroVitals::default(),
                        Vec3::ZERO,
                        target,
                        &[target],
                        &mut SummonBudget(20),
                        &mut SkillRuntime::default(),
                    );
                    spawn_echo_replay(
                        &mut commands,
                        &mut pools,
                        &assets,
                        SpellRecord {
                            skill_index: 13,
                            target: target.entity,
                            center: target.position,
                            damage: 100.0,
                        },
                        0.5,
                    );
                },
            )
            .unwrap();
        let effects = app
            .world_mut()
            .query::<(&ImpactFx, &super::super::skill_lighting::GroundLightProfile)>()
            .iter(app.world())
            .collect::<Vec<_>>();
        assert_eq!(effects.len(), 3);
        assert_eq!(
            effects.iter().filter(|(fx, _)| fx.lifetime == 0.8).count(),
            2
        );
        assert!(
            effects
                .iter()
                .all(|(_, p)| p.0 == DamageProfile::for_skill(13))
        );
    }

    #[test]
    fn skill_attack_procs_consume_exactly_the_combat_resolved_volley_count() {
        let combat = AttackVolleyRuntime {
            pending: 13,
            resolved_this_frame: 8,
        };
        let mut runtime = SkillRuntime::default();
        let mut observed = Vec::new();

        apply_resolved_attack_volleys(&mut runtime, combat.resolved_this_frame, |runtime| {
            observed.push(runtime.attack_count)
        });

        assert_eq!(runtime.attack_count, 8);
        assert_eq!(observed, (1..=8).collect::<Vec<_>>());
        assert_eq!(combat.pending, 13, "skills only observe combat state");
    }

    #[test]
    fn common_skill_rank_boundaries_match_the_spec() {
        assert_eq!((arc_chain_hit_count(1), arc_chain_hit_count(5)), (3, 5));
        assert_eq!((ember_attack_interval(1), ember_attack_interval(5)), (3, 2));
        assert_eq!((frost_lance_count(2), frost_lance_count(3)), (1, 2));
        assert_eq!((bone_sentry_cap(1), bone_sentry_cap(5)), (1, 3));
        assert_eq!((spirit_hound_count(1), spirit_hound_count(5)), (2, 4));
        assert_eq!((orbit_blade_count(1), orbit_blade_count(5)), (2, 4));
        assert_eq!((piercing_volley_count(1), piercing_volley_count(5)), (3, 5));
        assert_eq!(forced_movement_scale(false, false), 1.0);
        assert_eq!(forced_movement_scale(true, false), 0.5);
        assert_eq!(forced_movement_scale(true, true), 0.0);
    }

    #[test]
    fn upgraded_storm_zone_really_hits_outer_targets_that_rank_one_cannot() {
        use bevy::ecs::system::RunSystemOnce;
        for (rank, expected_hits) in [(1, 0), (3, 1), (5, 1)] {
            let mut app = App::new();
            app.init_resource::<Time>()
                .init_resource::<GameSession>()
                .init_resource::<PlayerBuild>()
                .init_resource::<HeroVitals>()
                .init_resource::<SkillRuntime>();
            app.world_mut().spawn((
                Enemy {
                    hp: 100.0,
                    max_hp: 100.0,
                    speed: 0.0,
                    damage: 0.0,
                    xp: 0,
                    wood: 0,
                    gold: 0,
                    kill_credit: 0,
                    elite: false,
                },
                Transform::from_xyz(4.5, 0.0, 0.0),
            ));
            app.world_mut().spawn((
                SkillZone {
                    kind: ZoneKind::StormMatrix,
                    remaining: 4.0,
                    pulse_remaining: 0.0,
                    damage_per_pulse: 10.0,
                    radius: skill_effect_radius(12, rank).unwrap(),
                    glacier_hits: HashMap::new(),
                    profile: DamageProfile::for_skill(12),
                },
                Transform::default(),
            ));
            app.world_mut().run_system_once(update_spell_zones).unwrap();
            assert_eq!(
                app.world_mut()
                    .query::<&SkillStrike>()
                    .iter(app.world())
                    .count(),
                expected_hits
            );
        }
    }

    #[test]
    fn ground_pulse_uses_a_real_finite_cone() {
        let origin = Vec3::ZERO;
        let forward = Vec3::X;
        let half_angle = 36.0_f32.to_radians();
        assert!(inside_cone(
            origin,
            forward,
            Vec3::new(4.0, 0.0, 1.0),
            5.4,
            half_angle
        ));
        assert!(!inside_cone(
            origin,
            forward,
            Vec3::new(1.0, 0.0, 4.0),
            5.4,
            half_angle
        ));
        assert!(!inside_cone(
            origin,
            forward,
            Vec3::new(6.0, 0.0, 0.0),
            5.4,
            half_angle
        ));
    }

    #[test]
    fn ember_burn_refreshes_one_timer_and_respects_the_layer_cap() {
        let mut burn = BurnState::new(60.0);
        burn.remaining = 0.2;
        add_burn_layer(&mut burn, 60.0, 3);
        add_burn_layer(&mut burn, 60.0, 3);
        add_burn_layer(&mut burn, 60.0, 3);
        assert_eq!(burn.layers, 3);
        assert!((burn.damage_per_tick - 30.0).abs() < 0.001);
        assert_eq!(burn.remaining, 3.0);
    }

    #[test]
    fn static_seal_explodes_on_four_and_resets() {
        let mut stacks = 0;
        assert!(!advance_static_seal(&mut stacks));
        assert!(!advance_static_seal(&mut stacks));
        assert!(!advance_static_seal(&mut stacks));
        assert!(advance_static_seal(&mut stacks));
        assert_eq!(stacks, 0);
    }

    #[test]
    fn control_duration_scaling_matches_normal_elite_and_boss_rules() {
        assert_eq!(control_duration_scale(false, false), 1.0);
        assert_eq!(control_duration_scale(true, false), 0.5);
        assert_eq!(control_duration_scale(false, true), 0.2);
        assert_eq!(control_duration_scale(true, true), 0.2);
    }

    #[test]
    fn rare_skill_rank_math_and_durations_match_the_spec() {
        assert_eq!((flying_sword_count(1), flying_sword_count(5)), (1, 3));
        assert_eq!(
            (tide_elemental_repair(1), tide_elemental_repair(5)),
            (8.0, 24.0)
        );
        assert_eq!(
            (oath_banner_reduction(1), oath_banner_reduction(5)),
            (0.10, 0.22)
        );
        assert_eq!(
            (transmute_gold_chance(1), transmute_gold_chance(5)),
            (0.04, 0.12)
        );
        assert_eq!(zone_pulse_interval(ZoneKind::StormMatrix), 0.6);
        assert_eq!(zone_pulse_interval(ZoneKind::GlacierCore), 0.8);
        assert_eq!(zone_pulse_interval(ZoneKind::VortexColumn), 0.5);
        assert_eq!((4.0_f32 / 0.6).floor() as u8, 6);
        assert_eq!((5.0_f32 / 0.8).floor() as u8, 6);
        assert_eq!((4.0_f32 / 0.5).floor() as u8, 8);
    }

    #[test]
    fn execution_threshold_consumes_its_sr_power_multiplier() {
        let mob_threshold = 0.12 * skill_utility_multiplier(18);
        let special_threshold = 0.04 * skill_utility_multiplier(18);
        assert!((execution_threshold(false) - mob_threshold).abs() < 0.0001);
        assert!((execution_threshold(true) - special_threshold).abs() < 0.0001);
        assert!(execution_ready(
            mob_threshold * 1_000.0,
            1_000.0,
            execution_threshold(false)
        ));
        assert!(!execution_ready(
            mob_threshold * 1_000.0 + 1.0,
            1_000.0,
            execution_threshold(false)
        ));
    }

    #[test]
    fn prism_echo_triggers_every_fifth_non_echo_spell_without_recursing() {
        let mut casts = 0;
        for _ in 0..4 {
            assert!(!record_non_echo_spell(&mut casts));
        }
        assert!(record_non_echo_spell(&mut casts));
        let echo_damage = 100.0 * prism_echo_scale(3);
        assert!((echo_damage - 55.0).abs() < 0.001);
        // Replaying uses `spawn_echo_replay` directly and never records a cast.
        assert_eq!(casts, 5);
        assert!(!record_non_echo_spell(&mut casts));
    }

    #[test]
    fn echo_core_uses_twenty_percent_chance_and_forty_five_percent_damage() {
        assert!(market_echo_triggers(0.20, 0.199));
        assert!(!market_echo_triggers(0.20, 0.20));
        assert!((100.0 * MARKET_ECHO_DAMAGE_SCALE - 45.0).abs() < 0.001);
    }

    #[test]
    fn spirit_drum_advances_only_summon_attack_time_at_one_twenty_five() {
        assert!((summon_attack_delta(1.0, 1.25) - 1.25).abs() < 0.001);
    }

    #[test]
    fn arc_lens_and_legion_scale_spell_and_inherited_summon_power() {
        let vitals = HeroVitals::default();
        let mut runtime = SkillRuntime {
            spell_power_multiplier: 1.10,
            summon_inherit_multiplier: 1.30,
            ..default()
        };
        let spell_power = vitals.spell_power() * 1.10;
        assert!((effective_spell_power(&vitals, &runtime) - spell_power).abs() < 0.001);
        let expected_summon = (spell_power * 0.7 + vitals.attack_damage() * 0.5) * 1.30;
        assert!((effective_summon_power(&vitals, &runtime) - expected_summon).abs() < 0.001);

        runtime.summon_inherit_multiplier = 1.0;
        assert!(effective_summon_power(&vitals, &runtime) < expected_summon);
    }

    #[test]
    fn oath_banner_bonus_restores_exactly_without_drift() {
        let mut build = PlayerBuild::default();
        let mut vitals = HeroVitals::default();
        build.attacks_per_second = 2.0;
        vitals.armor = 8.0;
        let base_attack_speed = build.attacks_per_second;
        let base_armor = vitals.armor;
        let reduction = oath_banner_reduction(5);
        let before_multiplier = 100.0 / (100.0 + 5.0 * base_armor);
        let mut banner = OathBanner {
            remaining: 10.0,
            repair_remaining: 1.0,
            damage_reduction: reduction,
            applied: false,
            armor_bonus: 0.0,
            attack_speed_bonus: 0.0,
        };
        for _ in 0..100 {
            apply_banner_bonuses(&mut banner, &mut build, &mut vitals);
            let during_multiplier = 100.0 / (100.0 + 5.0 * vitals.armor.max(0.0));
            assert!((during_multiplier / before_multiplier - (1.0 - reduction)).abs() < 0.0001);
            remove_banner_bonuses(&mut banner, &mut build, &mut vitals);
        }
        assert!((build.attacks_per_second - base_attack_speed).abs() < 0.0001);
        assert!((vitals.armor - base_armor).abs() < 0.0001);
    }

    #[test]
    fn pooled_oath_banner_retirement_removes_its_bonus_exactly_once() {
        let mut app = App::new();
        app.init_resource::<CombatPools>()
            .init_resource::<PlayerBuild>()
            .init_resource::<HeroVitals>();
        let base_attack_speed = app.world().resource::<PlayerBuild>().attacks_per_second;
        let base_armor = app.world().resource::<HeroVitals>().armor;
        let entity = app
            .world_mut()
            .run_system_once(|mut commands: Commands, mut pools: ResMut<CombatPools>| {
                let entity = pools
                    .rent_mesh_summon(
                        &mut commands,
                        SummonVisual::OathBanner,
                        Handle::default(),
                        Handle::default(),
                        Transform::default(),
                    )
                    .expect("banner shell should be available");
                commands.entity(entity).insert((
                    SkillSummon,
                    OathBanner {
                        remaining: 10.0,
                        repair_remaining: 1.0,
                        damage_reduction: 0.22,
                        applied: false,
                        armor_bonus: 0.0,
                        attack_speed_bonus: 0.0,
                    },
                ));
                entity
            })
            .expect("banner rent system should run");

        let retired = app
            .world_mut()
            .run_system_once(
                move |mut commands: Commands,
                      mut pools: ResMut<CombatPools>,
                      mut build: ResMut<PlayerBuild>,
                      mut vitals: ResMut<HeroVitals>,
                      mut banners: Query<&mut OathBanner>| {
                    let mut banner = banners.get_mut(entity).expect("active banner behavior");
                    apply_banner_bonuses(&mut banner, &mut build, &mut vitals);
                    let first = retire_oath_banner(
                        &mut commands,
                        &mut pools,
                        entity,
                        &mut banner,
                        &mut build,
                        &mut vitals,
                    );
                    let second = retire_oath_banner(
                        &mut commands,
                        &mut pools,
                        entity,
                        &mut banner,
                        &mut build,
                        &mut vitals,
                    );
                    (first, second)
                },
            )
            .expect("banner retire system should run");

        assert_eq!(retired, (true, false));
        assert!(
            (app.world().resource::<PlayerBuild>().attacks_per_second - base_attack_speed).abs()
                < 0.0001
        );
        assert!((app.world().resource::<HeroVitals>().armor - base_armor).abs() < 0.0001);
        assert!(app.world().get::<OathBanner>(entity).is_none());
        assert!(app.world().get::<PlayerSummon>(entity).is_none());
        assert_eq!(
            app.world().get::<Visibility>(entity),
            Some(&Visibility::Hidden)
        );
    }

    #[test]
    fn blood_wheel_records_one_hit_per_target_per_leg() {
        let target = Entity::PLACEHOLDER;
        let mut outbound = HashSet::new();
        let mut returning = HashSet::new();
        assert!(register_single_leg_hit(&mut outbound, target));
        assert!(!register_single_leg_hit(&mut outbound, target));
        assert!(register_single_leg_hit(&mut returning, target));
        assert!(!register_single_leg_hit(&mut returning, target));
    }

    #[test]
    fn summon_budget_hard_caps_every_summon_family_together() {
        let mut budget = SummonBudget::from_existing(18, 2);
        assert!(budget.take());
        assert!(budget.take());
        assert!(!budget.take());

        let mut legion_budget = SummonBudget::from_existing(18, 1);
        assert!(legion_budget.take());
        assert!(!legion_budget.take());

        let mut capped_legion_budget = SummonBudget::from_existing(19, 3);
        assert!(capped_legion_budget.take());
        assert!(!capped_legion_budget.take());
    }

    #[test]
    fn sky_judgment_scales_from_six_to_ten_visible_strikes() {
        assert_eq!(sky_judgment_target_count(1), 6);
        assert_eq!(sky_judgment_target_count(2), 7);
        assert_eq!(sky_judgment_target_count(5), 10);
    }

    #[test]
    fn ember_rebirth_only_intercepts_a_lethal_hit_when_ready() {
        assert!(!ember_rebirth_ready(0, 0.0, 0.0));
        assert!(!ember_rebirth_ready(1, 0.1, 0.0));
        assert!(!ember_rebirth_ready(1, 0.0, 1.0));
        assert!(ember_rebirth_ready(1, 0.0, 0.0));
        assert!(ember_rebirth_ready(5, 0.0, -20.0));
        assert!(!ember_rebirth_ready(5, 90.0, -20.0));
    }

    #[test]
    fn ember_rebirth_rolls_back_the_same_death_once_before_survival() {
        let mut app = App::new();
        app.init_resource::<GameSession>()
            .init_resource::<PlayerBuild>()
            .init_resource::<HeroVitals>()
            .init_resource::<SkillRuntime>()
            .add_systems(Update, intercept_ember_rebirth);
        app.world_mut().resource_mut::<PlayerBuild>().skills[25] = 1;
        {
            let mut session = app.world_mut().resource_mut::<GameSession>();
            session.core_hp = 920.0;
        }
        {
            let mut vitals = app.world_mut().resource_mut::<HeroVitals>();
            vitals.alive = false;
            vitals.health = 0.0;
            vitals.deaths = 1;
            vitals.revive_remaining = 8.0;
        }
        let return_position = Vec3::new(-4.0, 0.0, 1.5);
        app.world_mut()
            .resource_mut::<SkillRuntime>()
            .last_hero_position = return_position;
        let hero = app
            .world_mut()
            .spawn((
                Hero,
                Transform::from_xyz(0.0, -1_000.0, 0.0),
                Visibility::Hidden,
            ))
            .id();

        app.update();

        let session = app.world().resource::<GameSession>();
        let vitals = app.world().resource::<HeroVitals>();
        let runtime = app.world().resource::<SkillRuntime>();
        assert_eq!(session.core_hp, 920.0);
        assert!(vitals.alive);
        assert_eq!(vitals.deaths, 0);
        assert!(
            (vitals.health - vitals.max_health * 0.35 * skill_utility_multiplier(25)).abs() < 0.001
        );
        assert!((runtime.cooldowns[25] - skill_cooldown(25)).abs() < 0.001);
        assert_eq!(
            app.world()
                .entity(hero)
                .get::<Transform>()
                .unwrap()
                .translation,
            return_position
        );
        assert_eq!(
            *app.world().entity(hero).get::<Visibility>().unwrap(),
            Visibility::Visible
        );
        let strike_count = {
            let world = app.world_mut();
            let mut query = world.query::<&SkillStrike>();
            query.iter(world).count()
        };
        assert_eq!(strike_count, 1);

        app.update();
        let strike_count = {
            let world = app.world_mut();
            let mut query = world.query::<&SkillStrike>();
            query.iter(world).count()
        };
        assert_eq!(strike_count, 1);
    }

    #[test]
    fn triune_engine_cycles_without_stat_drift_and_rank_five_enables_all() {
        let mut build = PlayerBuild {
            attack_damage: 100.0,
            ..Default::default()
        };
        let mut state = TriuneState {
            rank: 3,
            phase: 0,
            phase_remaining: 4.0,
            attack_bonus: 0.0,
        };
        for _ in 0..100 {
            apply_triune_attack_bonus(&mut state, &mut build);
            assert!((build.attack_damage - 155.5).abs() < 0.0001);
            remove_triune_attack_bonus(&mut state, &mut build);
            assert!((build.attack_damage - 100.0).abs() < 0.0001);
        }
        state.phase = 1;
        assert!((triune_skill_multiplier(0, &state) - 1.555).abs() < 0.0001);
        assert_eq!(triune_skill_multiplier(1, &state), 1.0);
        state.phase = 2;
        assert!((triune_skill_multiplier(4, &state) - 1.555).abs() < 0.0001);

        state.rank = 5;
        state.phase = 0;
        assert!((triune_skill_multiplier(1, &state) - 1.74).abs() < 0.0001);
        assert!((triune_skill_multiplier(0, &state) - 1.74).abs() < 0.0001);
        assert!((triune_skill_multiplier(4, &state) - 1.74).abs() < 0.0001);
    }

    #[test]
    fn fate_cube_high_tier_weight_grows_and_seed_is_replayable() {
        let low_rank = fate_cube_weights(1);
        let high_rank = fate_cube_weights(5);
        let high_total = |weights: [u32; 5]| weights[0] + weights[1] + weights[4];
        assert!(high_total(high_rank) > high_total(low_rank));
        assert_eq!(low_rank[2], high_rank[2]);
        assert_eq!(low_rank[3], high_rank[3]);

        let mut first = RunRng::default();
        let mut second = RunRng::default();
        let first_results: Vec<FateOutcome> = (0..32)
            .map(|_| roll_fate_cube(4, first.next_u32()))
            .collect();
        let second_results: Vec<FateOutcome> = (0..32)
            .map(|_| roll_fate_cube(4, second.next_u32()))
            .collect();
        assert_eq!(first_results, second_results);
        assert!(first_results.iter().any(|outcome| matches!(
            outcome,
            FateOutcome::Lightning | FateOutcome::FireRain | FateOutcome::Summon
        )));
    }
}
