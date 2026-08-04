use std::collections::HashMap;

use bevy::prelude::*;

use super::market::MarketEffects;
use super::pooling::CombatPools;

use super::combat::CombatSet;
use super::content::{BossModifiers, SKILLS, Tag};
use super::{
    BondId, Core, Enemy, GameAssets, GameSession, Hero, HeroVitals, ImpactFx, PlayerBuild,
    Projectile, RunRng,
};

const BASE_SUMMON_CAP: usize = 18;
const GLOBAL_SUMMON_CAP: usize = 20;
const REACTION_COOLDOWN_SECONDS: f32 = 3.0;
const REACTION_TIER_SIX_COOLDOWN_SECONDS: f32 = 1.0;

pub(crate) struct BondsPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct BondSet;

impl Plugin for BondsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BondRuntime>()
            .init_resource::<FortuneMarketRules>()
            .add_systems(
                Update,
                (tick_bond_effects, resolve_bond_strikes)
                    .chain()
                    .in_set(BondSet)
                    .before(CombatSet),
            )
            .add_systems(Update, observe_bond_results.after(CombatSet));
    }
}

/// Element used by the final damage resolver. `Pure` bypasses armor and boss
/// resistances and is reserved for the true-damage share of Triune Blast.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum DamageElement {
    Physical,
    Storm,
    Flame,
    Frost,
    Tide,
    Arcane,
    Pure,
}

impl DamageElement {
    pub(crate) const fn is_elemental(self) -> bool {
        matches!(
            self,
            Self::Storm | Self::Flame | Self::Frost | Self::Tide | Self::Arcane
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum DamageClass {
    Attack,
    Spell,
    Summon,
    Reaction,
}

/// Metadata carried by every hero-owned projectile. It deliberately contains
/// no damage number, so combat remains the single source of truth for damage,
/// death rewards, and the DPS meter.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DamageProfile {
    pub(crate) element: DamageElement,
    pub(crate) class: DamageClass,
    pub(crate) applies_burn: bool,
    pub(crate) burn_tick: bool,
    pub(crate) skill_index: Option<u8>,
    /// Secondary equipment projectiles still use the authoritative damage
    /// resolver, but cannot recursively trigger another equipment proc.
    pub(crate) market_proc: bool,
}

impl DamageProfile {
    pub(crate) const PHYSICAL_ATTACK: Self =
        Self::new(DamageElement::Physical, DamageClass::Attack);
    pub(crate) const FLAME_REACTION: Self = Self::new(DamageElement::Flame, DamageClass::Reaction);
    pub(crate) const PURE_REACTION: Self = Self::new(DamageElement::Pure, DamageClass::Reaction);

    pub(crate) const fn new(element: DamageElement, class: DamageClass) -> Self {
        Self {
            element,
            class,
            applies_burn: false,
            burn_tick: false,
            skill_index: None,
            market_proc: false,
        }
    }

    pub(crate) const fn with_burn(mut self) -> Self {
        self.applies_burn = true;
        self
    }

    pub(crate) const fn as_burn_tick(mut self) -> Self {
        self.burn_tick = true;
        self
    }

    pub(crate) const fn as_market_proc(mut self) -> Self {
        self.market_proc = true;
        self
    }

    pub(crate) fn for_skill(index: usize) -> Self {
        let tags = SKILLS[index].tags;
        let element = if tags.contains(&Tag::Storm) {
            DamageElement::Storm
        } else if tags.contains(&Tag::Frost) {
            DamageElement::Frost
        } else if tags.contains(&Tag::Flame) {
            DamageElement::Flame
        } else if tags.contains(&Tag::Tide) {
            DamageElement::Tide
        } else if tags.contains(&Tag::Arcane) {
            DamageElement::Arcane
        } else {
            DamageElement::Physical
        };
        let class = if tags.contains(&Tag::Summon) {
            DamageClass::Summon
        } else if tags.contains(&Tag::Spell) {
            DamageClass::Spell
        } else {
            DamageClass::Attack
        };
        let mut profile = Self::new(element, class);
        profile.skill_index = Some(index as u8);
        profile
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ReactionKind {
    WetStormChain,
    WetFrostFreeze,
    BurnStormOverload,
    FrozenPhysicalShatter,
    StormJudgment,
}

impl ReactionKind {
    const fn index(self) -> usize {
        match self {
            Self::WetStormChain => 0,
            Self::WetFrostFreeze => 1,
            Self::BurnStormOverload => 2,
            Self::FrozenPhysicalShatter => 3,
            Self::StormJudgment => 0,
        }
    }

    pub(crate) const fn is_element_reaction(self) -> bool {
        !matches!(self, Self::StormJudgment)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ReactionProc {
    pub(crate) kind: ReactionKind,
    pub(crate) damage: f32,
    pub(crate) control_seconds: f32,
    pub(crate) echo: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ElementTarget {
    pub(crate) entity: Entity,
    pub(crate) max_hp: f32,
    pub(crate) speed: f32,
    pub(crate) boss: bool,
}

#[derive(Debug, Clone)]
struct ElementState {
    wet_remaining: f32,
    burn_remaining: f32,
    burn_layers: u8,
    frozen_remaining: f32,
    base_speed: f32,
    reaction_cooldowns: [f32; 4],
    detonation_remaining: f32,
}

impl ElementState {
    fn new(base_speed: f32) -> Self {
        Self {
            wet_remaining: 0.0,
            burn_remaining: 0.0,
            burn_layers: 0,
            frozen_remaining: 0.0,
            base_speed,
            reaction_cooldowns: [0.0; 4],
            detonation_remaining: 2.0,
        }
    }

    fn active(&self) -> bool {
        self.wet_remaining > 0.0 || self.burn_remaining > 0.0 || self.frozen_remaining > 0.0
    }
}

#[derive(Debug, Clone, Copy)]
struct TargetSnapshot {
    position: Vec3,
    max_hp: f32,
    elite: bool,
}

#[derive(Resource)]
pub(crate) struct BondRuntime {
    initialized: bool,
    last_kills: u32,
    last_hero_deaths: u32,
    last_gold: u32,
    last_core_hp: f32,
    fortune_gold_fraction: f32,
    blood_kills: u32,
    blood_frenzy_remaining: f32,
    frenzy_attack_bonus: f32,
    guardian_intercept_remaining: f32,
    guardian_last_stand_used: bool,
    guardian_armor_bonus: f32,
    guardian_magic_resistance_bonus: f32,
    elemental_states: HashMap<Entity, ElementState>,
    target_snapshots: HashMap<Entity, TargetSnapshot>,
    storm_hits: u64,
    spirit_marks: HashMap<Entity, f32>,
    triune_windows: [f32; 3],
    pub(crate) fortune_consumables_collected: u32,
}

impl Default for BondRuntime {
    fn default() -> Self {
        Self {
            initialized: false,
            last_kills: 0,
            last_hero_deaths: 0,
            last_gold: 0,
            last_core_hp: 0.0,
            fortune_gold_fraction: 0.0,
            blood_kills: 0,
            blood_frenzy_remaining: 0.0,
            frenzy_attack_bonus: 0.0,
            guardian_intercept_remaining: 0.0,
            guardian_last_stand_used: false,
            guardian_armor_bonus: 0.0,
            guardian_magic_resistance_bonus: 0.0,
            elemental_states: HashMap::new(),
            target_snapshots: HashMap::new(),
            storm_hits: 0,
            spirit_marks: HashMap::new(),
            triune_windows: [0.0; 3],
            fortune_consumables_collected: 0,
        }
    }
}

impl BondRuntime {
    pub(crate) fn blood_cast_speed_multiplier(&self, build: &PlayerBuild) -> f32 {
        if build.bond(BondId::Blood) >= 6 && self.blood_frenzy_remaining > 0.0 {
            1.35
        } else {
            1.0
        }
    }

    pub(crate) fn is_frozen(&self, target: Entity) -> bool {
        self.elemental_states
            .get(&target)
            .is_some_and(|state| state.frozen_remaining > 0.0)
    }

    #[cfg(test)]
    fn register_burn(
        &mut self,
        target: ElementTarget,
        duration: f32,
        layers: u8,
        ember_points: u8,
    ) {
        let cap = if ember_points >= 2 { 5 } else { 3 };
        let state = self
            .elemental_states
            .entry(target.entity)
            .or_insert_with(|| ElementState::new(target.speed));
        state.base_speed = target.speed.max(state.base_speed);
        state.burn_remaining = state.burn_remaining.max(duration);
        state.burn_layers = state.burn_layers.saturating_add(layers).min(cap);
    }

    #[cfg(test)]
    fn register_wet(&mut self, target: ElementTarget, duration: f32) {
        let state = self
            .elemental_states
            .entry(target.entity)
            .or_insert_with(|| ElementState::new(target.speed));
        state.base_speed = target.speed.max(state.base_speed);
        state.wet_remaining = state.wet_remaining.max(duration);
    }

    #[cfg(test)]
    fn register_frozen(&mut self, target: ElementTarget, duration: f32) {
        let state = self
            .elemental_states
            .entry(target.entity)
            .or_insert_with(|| ElementState::new(target.speed));
        state.base_speed = target.speed.max(state.base_speed);
        state.frozen_remaining = state.frozen_remaining.max(duration);
    }

    /// Registers an actual projectile hit and returns any deterministic bond
    /// procs that combat should route back through `Projectile`.
    pub(crate) fn register_element_hit(
        &mut self,
        profile: DamageProfile,
        target: ElementTarget,
        build: &PlayerBuild,
        spell_power: f32,
        attack_damage: f32,
    ) -> Vec<ReactionProc> {
        let element_points = build.bond(BondId::Elements);
        let status_scale = reaction_status_multiplier(element_points);
        let state = self
            .elemental_states
            .entry(target.entity)
            .or_insert_with(|| ElementState::new(target.speed));
        state.base_speed = target.speed.max(state.base_speed);

        let mut candidates = Vec::with_capacity(3);
        if element_points >= 2 && profile.class != DamageClass::Reaction {
            if profile.element == DamageElement::Storm && state.wet_remaining > 0.0 {
                candidates.push(ReactionKind::WetStormChain);
            }
            if profile.element == DamageElement::Frost && state.wet_remaining > 0.0 {
                candidates.push(ReactionKind::WetFrostFreeze);
            }
            if profile.element == DamageElement::Storm && state.burn_remaining > 0.0 {
                candidates.push(ReactionKind::BurnStormOverload);
            }
            if profile.element == DamageElement::Physical && state.frozen_remaining > 0.0 {
                candidates.push(ReactionKind::FrozenPhysicalShatter);
            }
        }

        let mut procs = Vec::with_capacity(candidates.len() + 1);
        for kind in candidates {
            let cooldown = &mut state.reaction_cooldowns[kind.index()];
            if *cooldown > 0.0 {
                continue;
            }
            *cooldown = reaction_cooldown(element_points);
            let (damage, control_seconds) = match kind {
                ReactionKind::WetStormChain => (50.0 + 0.45 * spell_power, 0.0),
                ReactionKind::WetFrostFreeze => {
                    state.wet_remaining = 0.0;
                    state.frozen_remaining = 1.2 * status_scale;
                    (0.0, state.frozen_remaining)
                }
                ReactionKind::BurnStormOverload => {
                    (60.0 + 0.5 * spell_power + 0.4 * attack_damage, 0.0)
                }
                ReactionKind::FrozenPhysicalShatter => {
                    state.frozen_remaining = 0.0;
                    let fraction = if target.boss { 0.005 } else { 0.08 };
                    (target.max_hp * fraction, 0.0)
                }
                ReactionKind::StormJudgment => unreachable!("judgment is not a status reaction"),
            };
            procs.push(ReactionProc {
                kind,
                damage: damage * reaction_damage_multiplier(element_points),
                control_seconds,
                echo: element_points >= 6,
            });
        }

        if profile.element == DamageElement::Tide {
            state.wet_remaining = state.wet_remaining.max(3.0 * status_scale);
        }
        if profile.applies_burn {
            let ember_points = build.bond(BondId::Ember);
            let cap = if ember_points >= 2 { 5 } else { 3 };
            state.burn_layers = state.burn_layers.saturating_add(1).min(cap);
            state.burn_remaining = 3.0 * status_scale;
        }

        if profile.class != DamageClass::Reaction
            && profile.element == DamageElement::Storm
            && build.bond(BondId::Thunder) >= 6
        {
            self.storm_hits = self.storm_hits.wrapping_add(1);
            if self.storm_hits.is_multiple_of(8) {
                procs.push(ReactionProc {
                    kind: ReactionKind::StormJudgment,
                    damage: 220.0 + 2.2 * spell_power,
                    control_seconds: 0.0,
                    echo: false,
                });
            }
        }
        procs
    }

    pub(crate) fn flame_taken_multiplier(&self, target: Entity, ember_points: u8) -> f32 {
        if ember_points < 2 {
            return 1.0;
        }
        1.0 + 0.04
            * f32::from(
                self.elemental_states
                    .get(&target)
                    .map_or(0, |state| state.burn_layers),
            )
    }

    pub(crate) fn spirit_mark_multiplier_and_refresh(
        &mut self,
        target: Entity,
        spirit_points: u8,
        profile: DamageProfile,
    ) -> f32 {
        if spirit_points < 4 || profile.class != DamageClass::Summon {
            return 1.0;
        }
        let multiplier = if self
            .spirit_marks
            .get(&target)
            .is_some_and(|remaining| *remaining > 0.0)
        {
            1.12
        } else {
            1.0
        };
        self.spirit_marks.insert(target, 4.0);
        multiplier
    }

    pub(crate) fn register_triune_hit(
        &mut self,
        triune_points: u8,
        profile: DamageProfile,
        source_damage: f32,
    ) -> Option<TriuneProc> {
        if triune_points < 4 || profile.class == DamageClass::Reaction {
            return None;
        }
        let index = match profile.class {
            DamageClass::Attack => 0,
            DamageClass::Spell => 1,
            DamageClass::Summon => 2,
            DamageClass::Reaction => return None,
        };
        self.triune_windows[index] = 4.0;
        if !self.triune_windows.iter().all(|remaining| *remaining > 0.0) {
            return None;
        }
        self.triune_windows.fill(0.0);
        Some(TriuneProc {
            damage: 100.0 + source_damage * 1.5,
            pure_share: if triune_points >= 6 { 0.25 } else { 0.0 },
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TriuneProc {
    pub(crate) damage: f32,
    pub(crate) pure_share: f32,
}

/// Runtime rules consumed by the market module. Keeping them in one resource
/// makes the 2/4/6 Fortune behavior testable without duplicating thresholds.
#[derive(Resource, Debug, Clone)]
pub(crate) struct FortuneMarketRules {
    pub(crate) price_multiplier: f32,
    pub(crate) rarity_bonus: f32,
    pub(crate) free_refresh_available: bool,
}

impl Default for FortuneMarketRules {
    fn default() -> Self {
        Self {
            price_multiplier: 1.0,
            rarity_bonus: 0.0,
            free_refresh_available: false,
        }
    }
}

#[derive(Component)]
pub(crate) struct BondStrike {
    pub(crate) target: Option<Entity>,
    pub(crate) position: Vec3,
    pub(crate) damage: f32,
    pub(crate) radius: f32,
    pub(crate) remaining_hits: u8,
    pub(crate) profile: DamageProfile,
    pub(crate) delay: f32,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn queue_bond_strike(
    commands: &mut Commands,
    target: Option<Entity>,
    position: Vec3,
    damage: f32,
    radius: f32,
    remaining_hits: u8,
    profile: DamageProfile,
    delay: f32,
) {
    commands.spawn(BondStrike {
        target,
        position,
        damage,
        radius,
        remaining_hits,
        profile,
        delay,
    });
}

#[allow(clippy::too_many_arguments)]
fn tick_bond_effects(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    mut build: ResMut<PlayerBuild>,
    mut vitals: ResMut<HeroVitals>,
    mut runtime: ResMut<BondRuntime>,
    mut fortune: ResMut<FortuneMarketRules>,
    market_effects: Res<MarketEffects>,
    hero: Query<&Transform, (With<Hero>, Without<Core>)>,
    core: Query<&Transform, (With<Core>, Without<Hero>)>,
    mut enemies: Query<(Entity, &mut Enemy, &Transform)>,
) {
    if session.finished {
        remove_frenzy_bonus(&mut runtime, &mut build);
        remove_guardian_reduction(&mut runtime, &mut vitals);
        return;
    }
    sync_fortune_rules(&build, session.wave, &mut fortune);
    if session.paused {
        return;
    }
    let delta = time.delta_secs();
    runtime.guardian_intercept_remaining = (runtime.guardian_intercept_remaining - delta).max(0.0);
    runtime.blood_frenzy_remaining = (runtime.blood_frenzy_remaining - delta).max(0.0);
    for remaining in runtime.spirit_marks.values_mut() {
        *remaining = (*remaining - delta).max(0.0);
    }
    runtime.spirit_marks.retain(|_, remaining| *remaining > 0.0);
    for remaining in &mut runtime.triune_windows {
        *remaining = (*remaining - delta).max(0.0);
    }

    let near_gate = hero
        .single()
        .ok()
        .zip(core.single().ok())
        .is_some_and(|(hero, core)| hero.translation.distance(core.translation) <= 8.0);
    if build.bond(BondId::Bastion) >= 2 && near_gate {
        apply_guardian_reduction(&mut runtime, &mut vitals);
    } else {
        remove_guardian_reduction(&mut runtime, &mut vitals);
    }
    if build.bond(BondId::Bastion) >= 6
        && !runtime.guardian_last_stand_used
        && session.core_max_hp > 0.0
        && session.core_hp / session.core_max_hp <= 0.25
    {
        runtime.guardian_last_stand_used = true;
        vitals.invulnerable_remaining = vitals.invulnerable_remaining.max(6.0);
    }

    let ember_detonation_damage =
        120.0 + market_effects.effective_spell_power(vitals.spell_power()) + build.attack_damage;
    runtime.target_snapshots.clear();
    for (entity, mut enemy, transform) in &mut enemies {
        runtime.target_snapshots.insert(
            entity,
            TargetSnapshot {
                position: transform.translation,
                max_hp: enemy.max_hp,
                elite: enemy.elite,
            },
        );
        let Some(state) = runtime.elemental_states.get_mut(&entity) else {
            continue;
        };
        state.wet_remaining = (state.wet_remaining - delta).max(0.0);
        state.burn_remaining = (state.burn_remaining - delta).max(0.0);
        state.frozen_remaining = (state.frozen_remaining - delta).max(0.0);
        for cooldown in &mut state.reaction_cooldowns {
            *cooldown = (*cooldown - delta).max(0.0);
        }
        if state.burn_remaining <= 0.0 {
            state.burn_layers = 0;
            state.detonation_remaining = 2.0;
        }
        if state.frozen_remaining > 0.0 {
            enemy.speed = 0.0;
        } else if enemy.speed == 0.0 {
            enemy.speed = state.base_speed;
        }
        if ember_detonation_ready(build.bond(BondId::Ember), state.burn_layers) {
            state.detonation_remaining -= delta;
            while state.detonation_remaining <= 0.0 {
                state.detonation_remaining += 2.0;
                queue_bond_strike(
                    &mut commands,
                    Some(entity),
                    transform.translation,
                    ember_detonation_damage,
                    3.0,
                    u8::MAX,
                    DamageProfile::FLAME_REACTION,
                    0.0,
                );
            }
        } else {
            state.detonation_remaining = 2.0;
        }
    }
    runtime
        .elemental_states
        .retain(|entity, state| enemies.contains(*entity) && state.active());
}

#[allow(clippy::too_many_arguments)]
fn observe_bond_results(
    mut commands: Commands,
    mut session: ResMut<GameSession>,
    mut build: ResMut<PlayerBuild>,
    mut vitals: ResMut<HeroVitals>,
    mut runtime: ResMut<BondRuntime>,
    mut rng: ResMut<RunRng>,
    living: Query<(Entity, &Transform, &Enemy)>,
) {
    if !runtime.initialized {
        runtime.initialized = true;
        runtime.last_kills = session.total_kills;
        runtime.last_hero_deaths = vitals.deaths;
        runtime.last_gold = session.gold;
        runtime.last_core_hp = session.core_hp;
        return;
    }

    let kill_delta = session.total_kills.saturating_sub(runtime.last_kills);
    if kill_delta > 0 {
        if build.bond(BondId::Blood) >= 2 {
            let healing = vitals.max_health * 0.01 * kill_delta as f32;
            vitals.heal(healing);
        }
        if build.bond(BondId::Blood) >= 6 {
            runtime.blood_kills = runtime.blood_kills.saturating_add(kill_delta);
            while runtime.blood_kills >= 20 {
                runtime.blood_kills -= 20;
                runtime.blood_frenzy_remaining = 10.0;
            }
        }
    }
    runtime.last_kills = session.total_kills;

    let frenzy_active = build.bond(BondId::Blood) >= 6 && runtime.blood_frenzy_remaining > 0.0;
    if frenzy_active && runtime.frenzy_attack_bonus == 0.0 {
        runtime.frenzy_attack_bonus = build.attacks_per_second * 0.35;
        build.attacks_per_second += runtime.frenzy_attack_bonus;
    } else if !frenzy_active && runtime.frenzy_attack_bonus != 0.0 {
        build.attacks_per_second =
            (build.attacks_per_second - runtime.frenzy_attack_bonus).max(0.2);
        runtime.frenzy_attack_bonus = 0.0;
    }

    if session.gold >= runtime.last_gold {
        let gained = session.gold - runtime.last_gold;
        if build.bond(BondId::Fortune) >= 2 && gained > 0 {
            runtime.fortune_gold_fraction += gained as f32 * 0.20;
            let bonus = runtime.fortune_gold_fraction.floor() as u32;
            runtime.fortune_gold_fraction -= bonus as f32;
            session.gold = session.gold.saturating_add(bonus);
        }
    }
    runtime.last_gold = session.gold;

    let new_hero_deaths = vitals.deaths.saturating_sub(runtime.last_hero_deaths);
    if session.core_hp < runtime.last_core_hp
        && build.bond(BondId::Bastion) >= 4
        && runtime.guardian_intercept_remaining <= 0.0
    {
        let breach_damage =
            (runtime.last_core_hp - session.core_hp - new_hero_deaths as f32 * 80.0).max(0.0);
        if breach_damage > 0.0 {
            session.core_hp = (session.core_hp + breach_damage).min(session.core_max_hp);
            runtime.guardian_intercept_remaining = 12.0;
        }
    }
    runtime.last_hero_deaths = vitals.deaths;
    runtime.last_core_hp = session.core_hp;

    let living_entities: HashMap<Entity, Vec3> = living
        .iter()
        .filter(|(_, _, enemy)| enemy.hp > 0.0)
        .map(|(entity, transform, _)| (entity, transform.translation))
        .collect();
    let mut defeated: Vec<(Entity, TargetSnapshot)> = runtime
        .target_snapshots
        .iter()
        .filter(|(entity, _)| !living_entities.contains_key(entity))
        .map(|(entity, snapshot)| (*entity, *snapshot))
        .collect();
    defeated.sort_by_key(|(entity, _)| entity.to_bits());
    defeated.truncate(kill_delta as usize);
    for (entity, snapshot) in defeated {
        if build.bond(BondId::Ember) >= 4
            && let Some(burn) = runtime.elemental_states.get(&entity).cloned()
            && burn.burn_layers > 0
        {
            let mut nearby: Vec<(Entity, f32)> = living_entities
                .iter()
                .filter_map(|(candidate, position)| {
                    let distance = snapshot.position.distance_squared(*position);
                    (distance <= 3.0_f32.powi(2)).then_some((*candidate, distance))
                })
                .collect();
            nearby.sort_by(|left, right| {
                left.1
                    .total_cmp(&right.1)
                    .then_with(|| left.0.to_bits().cmp(&right.0.to_bits()))
            });
            for (candidate, _) in nearby.into_iter().take(3) {
                let state = runtime
                    .elemental_states
                    .entry(candidate)
                    .or_insert_with(|| ElementState::new(1.0));
                state.burn_layers = state.burn_layers.max(burn.burn_layers).min(5);
                state.burn_remaining = state.burn_remaining.max(burn.burn_remaining.max(1.5));
            }
        }
        if build.bond(BondId::Blood) >= 4 {
            queue_bond_strike(
                &mut commands,
                None,
                snapshot.position,
                (snapshot.max_hp * 0.08).clamp(30.0, 1_200.0),
                2.4,
                u8::MAX,
                DamageProfile::new(DamageElement::Physical, DamageClass::Reaction),
                0.0,
            );
        }
        if build.bond(BondId::Fortune) >= 6 && snapshot.elite {
            runtime.fortune_consumables_collected =
                runtime.fortune_consumables_collected.saturating_add(1);
            match rng.next_u32() % 3 {
                0 => session.core_hp = (session.core_hp + 200.0).min(session.core_max_hp),
                1 => session.skill_rerolls = session.skill_rerolls.saturating_add(1),
                _ => session.wood = session.wood.saturating_add(10),
            }
        }
        runtime.elemental_states.remove(&entity);
    }
}

fn resolve_bond_strikes(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    assets: Option<Res<GameAssets>>,
    mut pools: ResMut<CombatPools>,
    mut strikes: Query<(Entity, &mut BondStrike)>,
    targets: Query<&Transform, With<Enemy>>,
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
        let position = strike
            .target
            .and_then(|target| {
                targets
                    .get(target)
                    .ok()
                    .map(|transform| transform.translation)
            })
            .unwrap_or(strike.position);
        let Some(projectile_entity) = pools.rent_projectile(
            &mut commands,
            assets.projectile_mesh.clone(),
            assets.projectile_material.clone(),
            Transform::from_translation(position + Vec3::Y * 0.18).with_scale(Vec3::splat(
                if strike.remaining_hits == u8::MAX {
                    0.8
                } else {
                    0.34
                },
            )),
            Projectile {
                velocity: Vec3::ZERO,
                damage: strike.damage.max(1.0),
                radius: strike.radius,
                remaining_hits: strike.remaining_hits,
                lifetime: 0.12,
            },
        ) else {
            continue;
        };
        commands.entity(projectile_entity).insert(strike.profile);
        pools.rent_impact_fx(
            &mut commands,
            assets.fx_mesh.clone(),
            assets.storm_material.clone(),
            Transform::from_xyz(position.x, 0.16, position.z).with_scale(Vec3::splat(0.12)),
            ImpactFx {
                lifetime: 0.24,
                start_scale: 0.12,
                end_scale: strike.radius.max(0.5),
            },
        );
        commands.entity(entity).despawn();
    }
}

fn sync_fortune_rules(build: &PlayerBuild, _wave: u32, rules: &mut FortuneMarketRules) {
    let points = build.bond(BondId::Fortune);
    rules.price_multiplier = if points >= 2 { 0.90 } else { 1.0 };
    rules.rarity_bonus = if points >= 4 { 0.15 } else { 0.0 };
    rules.free_refresh_available = points >= 4;
}

fn apply_guardian_reduction(runtime: &mut BondRuntime, vitals: &mut HeroVitals) {
    if runtime.guardian_armor_bonus != 0.0 {
        return;
    }
    runtime.guardian_armor_bonus = armor_bonus_for_reduction(vitals.armor, 0.10);
    runtime.guardian_magic_resistance_bonus = (1.0 - vitals.magic_resistance) * 0.10;
    vitals.armor += runtime.guardian_armor_bonus;
    vitals.magic_resistance =
        (vitals.magic_resistance + runtime.guardian_magic_resistance_bonus).min(0.95);
}

fn remove_guardian_reduction(runtime: &mut BondRuntime, vitals: &mut HeroVitals) {
    if runtime.guardian_armor_bonus == 0.0 {
        return;
    }
    vitals.armor -= runtime.guardian_armor_bonus;
    vitals.magic_resistance =
        (vitals.magic_resistance - runtime.guardian_magic_resistance_bonus).clamp(0.0, 0.95);
    runtime.guardian_armor_bonus = 0.0;
    runtime.guardian_magic_resistance_bonus = 0.0;
}

fn remove_frenzy_bonus(runtime: &mut BondRuntime, build: &mut PlayerBuild) {
    if runtime.frenzy_attack_bonus != 0.0 {
        build.attacks_per_second =
            (build.attacks_per_second - runtime.frenzy_attack_bonus).max(0.2);
    }
    runtime.frenzy_attack_bonus = 0.0;
}

fn armor_bonus_for_reduction(base_armor: f32, reduction: f32) -> f32 {
    let effective_base = base_armor.max(0.0);
    let target_armor =
        ((100.0 + 5.0 * effective_base) / (1.0 - reduction.clamp(0.0, 0.9)) - 100.0) / 5.0;
    target_armor - base_armor
}

pub(crate) const fn summon_cap(spirit_points: u8, equipment_bonus: u8) -> usize {
    let bond_bonus = if spirit_points >= 2 { 2 } else { 0 };
    let cap = BASE_SUMMON_CAP + bond_bonus + equipment_bonus as usize;
    if cap > GLOBAL_SUMMON_CAP {
        GLOBAL_SUMMON_CAP
    } else {
        cap
    }
}

const fn ember_detonation_ready(ember_points: u8, burn_layers: u8) -> bool {
    ember_points >= 6 && burn_layers >= 5
}

pub(crate) const fn reaction_cooldown(element_points: u8) -> f32 {
    if element_points >= 6 {
        REACTION_TIER_SIX_COOLDOWN_SECONDS
    } else {
        REACTION_COOLDOWN_SECONDS
    }
}

pub(crate) const fn reaction_damage_multiplier(element_points: u8) -> f32 {
    if element_points >= 4 { 1.60 } else { 1.0 }
}

pub(crate) const fn reaction_status_multiplier(element_points: u8) -> f32 {
    if element_points >= 4 { 1.35 } else { 1.0 }
}

pub(crate) fn skill_cooldown_recovery(
    index: usize,
    build: &PlayerBuild,
    runtime: &BondRuntime,
) -> f32 {
    let tags = SKILLS[index].tags;
    let mut recovery = runtime.blood_cast_speed_multiplier(build);
    if build.bond(BondId::Arcane) >= 2 && tags.contains(&Tag::Spell) {
        recovery /= 0.90;
    }
    if build.bond(BondId::Thunder) >= 4 && tags.contains(&Tag::Storm) {
        recovery /= 0.88;
    }
    if build.bond(BondId::Ascension) >= 6 && ascension_ready(build) {
        recovery /= 0.80;
    }
    recovery
}

pub(crate) fn projectile_bond_damage_multiplier(
    profile: DamageProfile,
    build: &PlayerBuild,
    session: &GameSession,
) -> f32 {
    let mut multiplier = 1.0;
    if build.bond(BondId::Thunder) >= 2 && profile.element == DamageElement::Storm {
        multiplier *= 1.15;
    }
    if build.bond(BondId::Spirits) >= 2 && profile.class == DamageClass::Summon {
        multiplier *= 1.15;
    }
    if build.bond(BondId::Triune) >= 2 && profile.class != DamageClass::Reaction {
        multiplier *= 1.08;
    }
    if build.bond(BondId::Ascension) >= 4
        && profile
            .skill_index
            .is_some_and(|index| build.skills[index as usize] >= 4)
    {
        multiplier *= 1.25;
    }
    multiplier *= guardian_damage_multiplier(
        build.bond(BondId::Bastion),
        session.core_hp,
        session.core_max_hp,
    );
    multiplier
}

pub(crate) fn ascension_ready(build: &PlayerBuild) -> bool {
    build.skills.iter().filter(|rank| **rank >= 5).count() >= 4
}

pub(crate) const fn arcane_echo_scale(arcane_points: u8) -> f32 {
    if arcane_points >= 6 {
        1.0
    } else if arcane_points >= 4 {
        0.5
    } else {
        0.0
    }
}

pub(crate) const fn guardian_damage_multiplier(
    bastion_points: u8,
    core_hp: f32,
    core_max_hp: f32,
) -> f32 {
    if bastion_points >= 6 && core_max_hp > 0.0 && core_hp / core_max_hp > 0.75 {
        1.30
    } else {
        1.0
    }
}

pub(crate) const fn blood_execute_ready(
    blood_points: u8,
    current_hp: f32,
    max_hp: f32,
    special: bool,
) -> bool {
    blood_points >= 4 && !special && max_hp > 0.0 && current_hp / max_hp <= 0.12
}

pub(crate) fn critical_multiplier(
    blood_points: u8,
    ember_points: u8,
    equipment_burns_can_crit: bool,
    profile: DamageProfile,
    roll: f32,
) -> f32 {
    let chance = 0.05 + if blood_points >= 2 { 0.10 } else { 0.0 };
    let burn_can_crit = !profile.burn_tick || ember_points >= 4 || equipment_burns_can_crit;
    if burn_can_crit && roll < chance {
        1.75
    } else {
        1.0
    }
}

pub(crate) fn boss_damage_multiplier(profile: DamageProfile, modifiers: BossModifiers) -> f32 {
    if profile.element == DamageElement::Pure {
        return 1.0;
    }
    let mut multiplier = 1.0 - modifiers.damage_reduction.clamp(0.0, 0.95);
    if profile.element.is_elemental() {
        multiplier *= 1.0 - modifiers.elemental_resistance.clamp(0.0, 0.95);
    }
    if profile.element == DamageElement::Flame {
        multiplier *= 1.0 - modifiers.flame_resistance.clamp(0.0, 0.95);
    }
    if profile.element == DamageElement::Frost {
        multiplier *= modifiers.frost_damage_taken_multiplier.max(0.0);
    }
    multiplier
}

#[cfg(test)]
mod tests {
    use super::super::content::{BONDS, BossModifiers};
    use super::*;

    #[test]
    fn every_bond_exposes_all_three_runtime_tiers() {
        assert_eq!(BONDS.len(), BondId::COUNT);
        for bond in BONDS {
            assert_eq!(bond.tiers.map(|tier| tier.threshold), [2, 4, 6]);
            assert!(bond.tiers.iter().all(|tier| !tier.description.is_empty()));
        }
        assert_eq!(summon_cap(0, 0), 18);
        assert_eq!(summon_cap(0, 1), 19);
        assert_eq!(summon_cap(2, 1), 20);
        assert_eq!(arcane_echo_scale(4), 0.5);
        assert_eq!(arcane_echo_scale(6), 1.0);
        assert_eq!(reaction_damage_multiplier(4), 1.6);
        assert_eq!(reaction_cooldown(6), 1.0);
        assert_eq!(guardian_damage_multiplier(6, 800.0, 1_000.0), 1.3);
        assert!(blood_execute_ready(4, 12.0, 100.0, false));
    }

    #[test]
    fn ember_heart_allows_burns_to_use_the_normal_critical_roll() {
        let burn = DamageProfile::FLAME_REACTION.as_burn_tick();
        assert_eq!(critical_multiplier(0, 0, false, burn, 0.01), 1.0);
        assert_eq!(critical_multiplier(0, 0, true, burn, 0.01), 1.75);
        assert_eq!(critical_multiplier(0, 0, true, burn, 0.05), 1.0);
    }

    #[test]
    fn all_ten_bond_families_have_runtime_boundaries() {
        let target = ElementTarget {
            entity: Entity::from_bits(9),
            max_hp: 1_000.0,
            speed: 2.0,
            boss: false,
        };
        let session = GameSession::default();

        let mut thunder = PlayerBuild::default();
        thunder.bonds[BondId::Thunder.index()] = 6;
        assert_eq!(
            projectile_bond_damage_multiplier(DamageProfile::for_skill(0), &thunder, &session),
            1.15
        );
        let mut thunder_runtime = BondRuntime::default();
        let mut judgment = false;
        for _ in 0..8 {
            judgment |= thunder_runtime
                .register_element_hit(DamageProfile::for_skill(0), target, &thunder, 100.0, 80.0)
                .iter()
                .any(|proc| proc.kind == ReactionKind::StormJudgment);
        }
        assert!(judgment);

        let mut ember_runtime = BondRuntime::default();
        ember_runtime.register_burn(target, 3.0, 5, 6);
        assert_eq!(ember_runtime.flame_taken_multiplier(target.entity, 2), 1.2);
        assert!(ember_detonation_ready(6, 5));
        assert_eq!(
            critical_multiplier(
                0,
                4,
                false,
                DamageProfile::FLAME_REACTION.as_burn_tick(),
                0.01,
            ),
            1.75
        );

        let mut spirit_runtime = BondRuntime::default();
        assert_eq!(summon_cap(0, 0), 18);
        assert_eq!(summon_cap(2, 0), 20);
        assert_eq!(
            spirit_runtime.spirit_mark_multiplier_and_refresh(
                target.entity,
                4,
                DamageProfile::new(DamageElement::Physical, DamageClass::Summon),
            ),
            1.0
        );
        assert_eq!(
            spirit_runtime.spirit_mark_multiplier_and_refresh(
                target.entity,
                4,
                DamageProfile::new(DamageElement::Physical, DamageClass::Summon),
            ),
            1.12
        );

        assert_eq!(reaction_damage_multiplier(4), 1.6);
        assert_eq!(reaction_cooldown(6), 1.0);
        assert_eq!(arcane_echo_scale(4), 0.5);
        assert_eq!(arcane_echo_scale(6), 1.0);

        let mut triune_runtime = BondRuntime::default();
        assert!(
            triune_runtime
                .register_triune_hit(6, DamageProfile::PHYSICAL_ATTACK, 100.0)
                .is_none()
        );
        assert!(
            triune_runtime
                .register_triune_hit(
                    6,
                    DamageProfile::new(DamageElement::Arcane, DamageClass::Spell),
                    100.0,
                )
                .is_none()
        );
        let blast = triune_runtime
            .register_triune_hit(
                6,
                DamageProfile::new(DamageElement::Physical, DamageClass::Summon),
                100.0,
            )
            .expect("three categories trigger the blast");
        assert_eq!(blast.pure_share, 0.25);

        let mut ascension = PlayerBuild::default();
        ascension.bonds[BondId::Ascension.index()] = 6;
        ascension.skills[..4].fill(5);
        assert!(ascension_ready(&ascension));
        assert!(skill_cooldown_recovery(0, &ascension, &BondRuntime::default()) > 1.2);

        assert_eq!(
            critical_multiplier(2, 0, false, DamageProfile::PHYSICAL_ATTACK, 0.10),
            1.75
        );
        assert!(blood_execute_ready(4, 11.9, 100.0, false));

        let mut fortune = FortuneMarketRules::default();
        let mut fortunate_build = PlayerBuild::default();
        fortunate_build.bonds[BondId::Fortune.index()] = 4;
        sync_fortune_rules(&fortunate_build, 1, &mut fortune);
        assert_eq!(fortune.price_multiplier, 0.90);
        assert_eq!(fortune.rarity_bonus, 0.15);
        assert!(fortune.free_refresh_available);

        assert_eq!(guardian_damage_multiplier(6, 800.0, 1_000.0), 1.3);
        assert_eq!(guardian_damage_multiplier(6, 250.0, 1_000.0), 1.0);
    }

    #[test]
    fn all_four_element_reactions_are_detected() {
        let target = ElementTarget {
            entity: Entity::from_bits(1),
            max_hp: 1_000.0,
            speed: 2.0,
            boss: false,
        };
        let mut build = PlayerBuild::default();
        build.bonds[BondId::Elements.index()] = 6;
        build.bonds[BondId::Ember.index()] = 2;

        let mut wet_storm = BondRuntime::default();
        wet_storm.register_wet(target, 3.0);
        let kinds: Vec<_> = wet_storm
            .register_element_hit(
                DamageProfile::new(DamageElement::Storm, DamageClass::Spell),
                target,
                &build,
                100.0,
                80.0,
            )
            .into_iter()
            .map(|proc| proc.kind)
            .collect();
        assert!(kinds.contains(&ReactionKind::WetStormChain));

        let mut wet_frost = BondRuntime::default();
        wet_frost.register_wet(target, 3.0);
        let proc = wet_frost.register_element_hit(
            DamageProfile::new(DamageElement::Frost, DamageClass::Spell),
            target,
            &build,
            100.0,
            80.0,
        )[0];
        assert_eq!(proc.kind, ReactionKind::WetFrostFreeze);
        assert!((proc.control_seconds - 1.62).abs() < 0.001);

        let mut burn_storm = BondRuntime::default();
        burn_storm.register_burn(target, 3.0, 1, 2);
        let kinds: Vec<_> = burn_storm
            .register_element_hit(
                DamageProfile::new(DamageElement::Storm, DamageClass::Spell),
                target,
                &build,
                100.0,
                80.0,
            )
            .into_iter()
            .map(|proc| proc.kind)
            .collect();
        assert!(kinds.contains(&ReactionKind::BurnStormOverload));

        let mut frozen_physical = BondRuntime::default();
        frozen_physical.register_frozen(target, 1.2);
        let proc = frozen_physical.register_element_hit(
            DamageProfile::PHYSICAL_ATTACK,
            target,
            &build,
            100.0,
            80.0,
        )[0];
        assert_eq!(proc.kind, ReactionKind::FrozenPhysicalShatter);
        assert_eq!(proc.damage, 128.0);
    }

    #[test]
    fn boss_modifiers_apply_by_damage_element() {
        let cinder = BossModifiers {
            flame_resistance: 0.70,
            frost_damage_taken_multiplier: 1.35,
            ..BossModifiers::NEUTRAL
        };
        let flame = DamageProfile::new(DamageElement::Flame, DamageClass::Spell);
        let frost = DamageProfile::new(DamageElement::Frost, DamageClass::Spell);
        assert!((boss_damage_multiplier(flame, cinder) - 0.30).abs() < 0.001);
        assert!((boss_damage_multiplier(frost, cinder) - 1.35).abs() < 0.001);

        let veil = BossModifiers {
            damage_reduction: 0.15,
            elemental_resistance: 0.10,
            ..BossModifiers::NEUTRAL
        };
        assert!((boss_damage_multiplier(frost, veil) - 0.765).abs() < 0.001);
        assert!((boss_damage_multiplier(DamageProfile::PURE_REACTION, veil) - 1.0).abs() < 0.001);
    }
}
