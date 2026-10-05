use bevy::prelude::*;

use super::HeroKind;
pub use super::bond_catalog::BondCardId;
use super::bond_catalog::{BOND_FAMILIES, BondFamilyDef};
use super::content::{Attribute, HEROES, Rarity, SKILLS, SkillDef};

pub const MAX_HERO_LEVEL: u32 = 60;
pub const RECRUITMENT_LEVELS: [u32; 3] = [10, 20, 40];
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecruitmentSource {
    Milestone(u32),
    Fruit,
}
#[derive(Resource, Debug, Default)]
pub struct RecruitmentProgress {
    pub fruit_count: u32,
    pub claimed_milestones: u8,
    pub absorbed_strength: f32,
    pub absorbed_agility: f32,
    pub absorbed_intelligence: f32,
    pub pending: Option<RecruitmentSource>,
    pub requested: bool,
}
impl RecruitmentProgress {
    pub fn next_milestone(&self, level: u32) -> Option<u32> {
        RECRUITMENT_LEVELS
            .into_iter()
            .enumerate()
            .find_map(|(index, required)| {
                (level >= required && self.claimed_milestones & (1 << index) == 0)
                    .then_some(required)
            })
    }
    pub fn claim(&mut self, source: RecruitmentSource) -> bool {
        match source {
            RecruitmentSource::Fruit => {
                if self.fruit_count == 0 {
                    return false;
                }
                self.fruit_count -= 1;
            }
            RecruitmentSource::Milestone(level) => {
                let Some(index) = RECRUITMENT_LEVELS
                    .iter()
                    .position(|required| *required == level)
                else {
                    return false;
                };
                if self.claimed_milestones & (1 << index) != 0 {
                    return false;
                }
                self.claimed_milestones |= 1 << index;
            }
        }
        self.pending = None;
        true
    }
}

pub const HERO_REVIVE_BASE_SECONDS: f32 = 8.0;
pub const HERO_REVIVE_STEP_SECONDS: f32 = 3.0;
pub const HERO_TOTAL_LIVES: u32 = 3;
pub const GATE_REBUILD_SECONDS: f32 = 4.0;
pub const GATE_POSITION: Vec3 = Vec3::new(-5.2, 1.0, 0.0);
pub const HERO_RESPAWN_INVULNERABILITY: f32 = 1.2;
pub const HERO_RESPAWN_POSITION: Vec3 = Vec3::new(-8.2, 0.0, 0.0);
pub const DODGE_COOLDOWN_SECONDS: f32 = 4.5;
pub const DODGE_DISTANCE: f32 = 3.2;
pub const DODGE_INVULNERABILITY: f32 = 0.22;

#[derive(Resource)]
pub struct GameSession {
    pub elapsed: f32,
    pub duration: f32,
    pub wave: u32,
    pub core_hp: f32,
    pub core_max_hp: f32,
    pub gate_revive_remaining: f32,
    pub level: u32,
    pub xp: u32,
    pub xp_next: u32,
    pub xp_multiplier: f32,
    pub gold: u32,
    pub gold_income_accum: f32,
    pub wood: u32,
    pub bond_draws: u32,
    /// Lets autoplay open F drafts. Capture scenarios switch it off to keep
    /// bond drafts out of frame; manual F is unaffected.
    pub auto_bond_draws: bool,
    pub kills: u32,
    pub total_kills: u32,
    pub bosses_defeated: u8,
    pub save_active: bool,
    pub save_remaining: f32,
    pub skill_points: u32,
    pub skill_rerolls: u32,
    pub paused: bool,
    pub finished: bool,
    pub victory: bool,
}

impl GameSession {
    /// A collapsed gate must finish rebuilding; healing cannot skip its downtime.
    pub fn heal_gate(&mut self, amount: f32) -> f32 {
        if self.core_hp <= 0.0 || self.gate_revive_remaining > 0.0 {
            return 0.0;
        }
        let before = self.core_hp;
        self.core_hp = (self.core_hp + amount.max(0.0)).min(self.core_max_hp);
        self.core_hp - before
    }
}

impl Default for GameSession {
    fn default() -> Self {
        Self {
            elapsed: 0.0,
            duration: 25.0 * 60.0,
            wave: 1,
            core_hp: 1_000.0,
            core_max_hp: 1_000.0,
            gate_revive_remaining: 0.0,
            level: 1,
            xp: 0,
            xp_next: 60,
            xp_multiplier: 1.0,
            gold: 10,
            gold_income_accum: 0.0,
            wood: 60,
            bond_draws: 0,
            auto_bond_draws: true,
            kills: 0,
            total_kills: 0,
            bosses_defeated: 0,
            save_active: false,
            save_remaining: 60.0,
            skill_points: 0,
            skill_rerolls: 1,
            paused: false,
            finished: false,
            victory: false,
        }
    }
}

pub const DAMAGE_WINDOW_SECONDS: usize = 10;

/// Fixed one-second buckets for the run's measured damage output.
///
/// Damage is recorded by combat after mitigation and overkill clamping. The
/// clock is advanced only while gameplay is running, so pausing preserves the
/// exact contents of the ten-second window.
#[derive(Resource, Debug, Clone)]
pub struct DamageMeter {
    buckets: [f32; DAMAGE_WINDOW_SECONDS],
    current: usize,
    bucket_elapsed: f32,
}

impl Default for DamageMeter {
    fn default() -> Self {
        Self {
            buckets: [0.0; DAMAGE_WINDOW_SECONDS],
            current: 0,
            bucket_elapsed: 0.0,
        }
    }
}

impl DamageMeter {
    pub fn record(&mut self, damage: f32) {
        if damage > 0.0 {
            self.buckets[self.current] += damage;
        }
    }

    pub fn advance(&mut self, delta_seconds: f32) {
        self.bucket_elapsed += delta_seconds.max(0.0);
        // Frame deltas are f32; tolerate their sub-microsecond representation
        // error so an exact logical second always rotates the bucket.
        let elapsed_buckets = (self.bucket_elapsed + 0.000_01).floor() as usize;
        if elapsed_buckets == 0 {
            return;
        }
        self.bucket_elapsed = (self.bucket_elapsed - elapsed_buckets as f32).max(0.0);
        if elapsed_buckets >= DAMAGE_WINDOW_SECONDS {
            self.buckets.fill(0.0);
            self.current = (self.current + elapsed_buckets) % DAMAGE_WINDOW_SECONDS;
            return;
        }
        for _ in 0..elapsed_buckets {
            self.current = (self.current + 1) % DAMAGE_WINDOW_SECONDS;
            self.buckets[self.current] = 0.0;
        }
    }

    pub fn recent_dps(&self) -> f32 {
        self.buckets.iter().sum::<f32>() / DAMAGE_WINDOW_SECONDS as f32
    }
}

/// Authoritative runtime state for hero attributes, health and action guards.
///
/// Systems that create hero-owned attacks should call [`Self::can_act`] before
/// doing work. This deliberately lives in the public game API so skills and
/// equipment can share the same death, stun and invulnerability rules.
#[derive(Resource, Debug, Clone)]
pub struct HeroVitals {
    pub kind: HeroKind,
    pub level: u32,
    pub strength: f32,
    pub agility: f32,
    pub intelligence: f32,
    pub health: f32,
    pub max_health: f32,
    pub command_health_multiplier: f32,
    pub permanent_spell_power: f32,
    pub energy: f32,
    pub max_energy: f32,
    pub armor: f32,
    pub magic_resistance: f32,
    pub alive: bool,
    pub deaths: u32,
    pub revive_remaining: f32,
    pub invulnerable_remaining: f32,
    pub stunned_remaining: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HeroProgressionDelta {
    pub primary: f32,
    pub agility: f32,
    pub intelligence: f32,
    pub max_health: f32,
    pub max_energy: f32,
}

impl HeroVitals {
    pub fn for_hero(kind: HeroKind, level: u32) -> Self {
        let level = level.clamp(1, MAX_HERO_LEVEL);
        let hero = HEROES[kind.index()];
        let rarity_multiplier = hero.rarity.hero_stat_multiplier();
        let level_index = level.saturating_sub(1) as f32;
        let strength =
            (hero.base_stats.strength + hero.growth.strength * level_index) * rarity_multiplier;
        let agility =
            (hero.base_stats.agility + hero.growth.agility * level_index) * rarity_multiplier;
        let intelligence = (hero.base_stats.intelligence + hero.growth.intelligence * level_index)
            * rarity_multiplier;
        let max_health = hero.base_health * rarity_multiplier + strength * 14.0;
        let max_energy = hero.base_energy * rarity_multiplier + intelligence * 8.0;
        Self {
            kind,
            level,
            strength,
            agility,
            intelligence,
            health: max_health,
            max_health,
            command_health_multiplier: 1.0,
            permanent_spell_power: 0.0,
            energy: max_energy,
            max_energy,
            // N1 heroes begin without innate armor; equipment may add it.
            armor: 0.0,
            magic_resistance: 0.10,
            alive: true,
            deaths: 0,
            revive_remaining: 0.0,
            invulnerable_remaining: 0.0,
            stunned_remaining: 0.0,
        }
    }

    pub fn reset_for_hero(&mut self, kind: HeroKind, level: u32) {
        *self = Self::for_hero(kind, level);
    }

    /// Applies catalog growth without discarding damage, death count or timed states.
    pub fn synchronize_level(&mut self, kind: HeroKind, level: u32) -> HeroProgressionDelta {
        if self.kind != kind {
            let previous_primary = self.primary_attribute();
            let previous_agility = self.agility;
            let previous_intelligence = self.intelligence;
            let previous_max_health = self.max_health;
            let previous_max_energy = self.max_energy;
            let command_health_multiplier = self.command_health_multiplier;
            let permanent_spell_power = self.permanent_spell_power;
            let armor = self.armor;
            let magic_resistance = self.magic_resistance;
            self.reset_for_hero(kind, level);
            self.armor = armor;
            self.magic_resistance = magic_resistance;
            self.command_health_multiplier = command_health_multiplier;
            self.permanent_spell_power = permanent_spell_power;
            self.max_health *= command_health_multiplier;
            self.health = self.max_health;
            return HeroProgressionDelta {
                primary: self.primary_attribute() - previous_primary,
                agility: self.agility - previous_agility,
                intelligence: self.intelligence - previous_intelligence,
                max_health: self.max_health - previous_max_health,
                max_energy: self.max_energy - previous_max_energy,
            };
        }
        let next = Self::for_hero(kind, level);
        if next.level == self.level {
            return HeroProgressionDelta::default();
        }

        let old = Self::for_hero(kind, self.level);
        let next_max_health =
            self.max_health + (next.max_health - old.max_health) * self.command_health_multiplier;
        let delta = HeroProgressionDelta {
            primary: next.primary_attribute() - old.primary_attribute(),
            agility: next.agility - old.agility,
            intelligence: next.intelligence - old.intelligence,
            max_health: next_max_health - self.max_health,
            max_energy: next.max_energy - old.max_energy,
        };
        self.level = next.level;
        self.strength += next.strength - old.strength;
        self.agility += next.agility - old.agility;
        self.intelligence += next.intelligence - old.intelligence;
        self.max_health = next_max_health;
        self.max_energy += delta.max_energy;
        if self.alive {
            self.health = (self.health + delta.max_health).clamp(0.0, self.max_health);
            self.energy = (self.energy + delta.max_energy).clamp(0.0, self.max_energy);
        }
        delta
    }

    pub fn primary_attribute(&self) -> f32 {
        match HEROES[self.kind.index()].primary {
            Attribute::Strength => self.strength,
            Attribute::Agility => self.agility,
            Attribute::Intelligence => self.intelligence,
        }
    }

    pub fn attack_damage(&self) -> f32 {
        20.0 + self.primary_attribute()
    }

    pub fn spell_power(&self) -> f32 {
        20.0 + self.intelligence * 2.0 + self.permanent_spell_power
    }

    pub fn summon_power(&self) -> f32 {
        self.spell_power() * 0.7 + self.attack_damage() * 0.5
    }

    pub fn lives_remaining(&self) -> u32 {
        HERO_TOTAL_LIVES.saturating_sub(self.deaths)
    }

    pub fn can_act(&self) -> bool {
        self.alive && self.stunned_remaining <= 0.0
    }

    pub fn can_be_targeted(&self) -> bool {
        self.alive
    }

    pub fn is_invulnerable(&self) -> bool {
        self.invulnerable_remaining > 0.0
    }

    pub fn take_physical_damage(&mut self, raw_damage: f32) -> f32 {
        if !self.alive || self.is_invulnerable() || raw_damage <= 0.0 {
            return 0.0;
        }
        let multiplier = 100.0 / (100.0 + 5.0 * self.armor.max(0.0));
        let damage = (raw_damage * multiplier).max(1.0);
        self.health = (self.health - damage).max(0.0);
        damage
    }

    pub fn take_magical_damage(&mut self, raw_damage: f32) -> f32 {
        if !self.alive || self.is_invulnerable() || raw_damage <= 0.0 {
            return 0.0;
        }
        let damage = (raw_damage * (1.0 - self.magic_resistance.clamp(0.0, 0.95))).max(1.0);
        self.health = (self.health - damage).max(0.0);
        damage
    }

    pub fn heal(&mut self, amount: f32) -> f32 {
        if !self.alive || amount <= 0.0 {
            return 0.0;
        }
        let before = self.health;
        self.health = (self.health + amount).min(self.max_health);
        self.health - before
    }
}

impl Default for HeroVitals {
    fn default() -> Self {
        Self::for_hero(HeroKind::StormSavant, 1)
    }
}

pub const fn revive_delay_for_death(death_number: u32) -> f32 {
    HERO_REVIVE_BASE_SECONDS + HERO_REVIVE_STEP_SECONDS * death_number.saturating_sub(1) as f32
}

#[derive(Resource, Debug, Clone)]
pub struct DodgeState {
    pub cooldown_remaining: f32,
    pub requested: bool,
    pub last_direction: Vec2,
}

impl DodgeState {
    pub fn ready(&self) -> bool {
        self.cooldown_remaining <= 0.0
    }

    pub fn request(&mut self) {
        self.requested = true;
    }
}

impl Default for DodgeState {
    fn default() -> Self {
        Self {
            cooldown_remaining: 0.0,
            requested: false,
            last_direction: Vec2::X,
        }
    }
}

#[derive(Resource)]
pub struct PlayerBuild {
    pub attack_damage: f32,
    pub attacks_per_second: f32,
    pub projectile_speed: f32,
    pub attack_range: f32,
    pub move_speed: f32,
    pub multishot: u8,
    pub pierce: u8,
    pub ricochet: u8,
    pub cleave: u8,
    pub owned_bond_cards: [bool; BondCardId::COUNT],
    pub skills: [u8; SkillId::COUNT],
}

impl Default for PlayerBuild {
    fn default() -> Self {
        Self {
            attack_damage: 48.0,
            attacks_per_second: 1.954,
            projectile_speed: 19.24,
            attack_range: 9.2,
            move_speed: 6.03,
            multishot: 1,
            pierce: 0,
            ricochet: 0,
            cleave: 0,
            owned_bond_cards: [false; BondCardId::COUNT],
            skills: [0; SkillId::COUNT],
        }
    }
}

#[derive(Resource)]
pub struct SpawnDirector {
    pub enabled: bool,
    pub next_elite_at: f32,
    pub boss_spawned: [bool; 5],
}

impl Default for SpawnDirector {
    fn default() -> Self {
        Self {
            enabled: true,
            next_elite_at: 120.0,
            boss_spawned: [false; 5],
        }
    }
}

#[derive(Resource)]
pub struct AttackClock(pub Timer);

impl Default for AttackClock {
    fn default() -> Self {
        Self(Timer::from_seconds(0.45, TimerMode::Repeating))
    }
}

#[derive(Resource, Default)]
pub struct MoveIntent(pub Vec2);

#[derive(Resource)]
pub struct ActiveSkill {
    pub cooldown: f32,
    pub remaining: f32,
}

#[derive(Resource)]
pub struct TrialState {
    pub level: u8,
    pub active: bool,
    pub remaining: f32,
    pub cooldown: f32,
    pub auto: bool,
    pub entity: Option<Entity>,
    pub command_points: u32,
    pub growth_ranks: [u8; CommandGrowth::COUNT],
    pub last_growth: Option<CommandGrowth>,
}

#[derive(Resource)]
pub struct GoldChallengeState {
    pub level: u8,
    pub active: bool,
    pub remaining: f32,
    pub cooldown: f32,
    pub auto: bool,
    pub entity: Option<Entity>,
    pub remaining_targets: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandGrowth {
    Attack,
    Vitality,
    Insight,
}

impl CommandGrowth {
    pub const COUNT: usize = 3;

    pub const fn from_point(point: u32) -> Self {
        match point % Self::COUNT as u32 {
            0 => Self::Attack,
            1 => Self::Vitality,
            _ => Self::Insight,
        }
    }

    pub const fn index(self) -> usize {
        match self {
            Self::Attack => 0,
            Self::Vitality => 1,
            Self::Insight => 2,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Attack => "ATTACK",
            Self::Vitality => "MAX HP",
            Self::Insight => "XP EFF",
        }
    }
}

impl Default for TrialState {
    fn default() -> Self {
        Self {
            level: 0,
            active: false,
            remaining: 0.0,
            cooldown: 0.0,
            auto: false,
            entity: None,
            command_points: 0,
            growth_ranks: [0; CommandGrowth::COUNT],
            last_growth: None,
        }
    }
}

impl Default for GoldChallengeState {
    fn default() -> Self {
        Self {
            level: 0,
            active: false,
            remaining: 0.0,
            cooldown: 0.0,
            auto: false,
            entity: None,
            remaining_targets: 0,
        }
    }
}

impl Default for ActiveSkill {
    fn default() -> Self {
        Self {
            cooldown: 8.0,
            remaining: 0.0,
        }
    }
}

#[derive(Resource, Debug, Clone, Copy)]
pub struct SelectedHero {
    pub kind: HeroKind,
    pub locked: bool,
}

impl SelectedHero {
    pub fn rarity(&self) -> Rarity {
        HEROES[self.kind.index()].rarity
    }
}

impl Default for SelectedHero {
    fn default() -> Self {
        Self {
            kind: HeroKind::StormSavant,
            locked: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkillId(pub u8);

impl SkillId {
    pub const COUNT: usize = 32;

    pub fn definition(self) -> &'static SkillDef {
        &SKILLS[self.0 as usize]
    }

    pub fn title(self) -> String {
        self.definition().id.replace('_', " ").to_ascii_uppercase()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BondId {
    Journey,
    Agility,
    Ranger,
    Underworld,
    Ascension,
    Strength,
    Intelligence,
    Archery,
    Warpath,
    Arcanum,
    Cultivation,
    SpiritEnergy,
    ElementMaster,
    ElementArchmage,
    DemonHunter,
    Barbarian,
    MadScientist,
    Windrunner,
    ForbiddenArts,
    RevivalWind,
    Mountain,
    KillingIntent,
    Destruction,
    Requiem,
    Hunter,
    Sharpshooter,
}
impl BondId {
    pub const COUNT: usize = 26;
    pub const ALL: [Self; Self::COUNT] = [
        Self::Journey,
        Self::Agility,
        Self::Ranger,
        Self::Underworld,
        Self::Ascension,
        Self::Strength,
        Self::Intelligence,
        Self::Archery,
        Self::Warpath,
        Self::Arcanum,
        Self::Cultivation,
        Self::SpiritEnergy,
        Self::ElementMaster,
        Self::ElementArchmage,
        Self::DemonHunter,
        Self::Barbarian,
        Self::MadScientist,
        Self::Windrunner,
        Self::ForbiddenArts,
        Self::RevivalWind,
        Self::Mountain,
        Self::KillingIntent,
        Self::Destruction,
        Self::Requiem,
        Self::Hunter,
        Self::Sharpshooter,
    ];
    pub const fn index(self) -> usize {
        self as usize
    }
    pub fn definition(self) -> &'static BondFamilyDef {
        &BOND_FAMILIES[self.index()]
    }
    pub fn title(self) -> &'static str {
        self.definition().name_en
    }
    pub fn required(self) -> u8 {
        self.definition().members.len() as u8
    }
}

impl PlayerBuild {
    pub fn owns_bond_card(&self, card: BondCardId) -> bool {
        self.owned_bond_cards[card.index()]
    }

    pub fn bond(&self, id: BondId) -> u8 {
        id.definition()
            .members
            .iter()
            .filter(|card| self.owns_bond_card(**card))
            .count() as u8
    }

    pub fn bond_complete(&self, id: BondId) -> bool {
        self.bond(id) == id.required()
    }

    pub fn active_bond_count(&self) -> usize {
        BondId::ALL
            .into_iter()
            .filter(|id| self.bond(*id) > 0 && !self.bond_complete(*id))
            .count()
    }

    pub fn skill_rank(&self, id: SkillId) -> u8 {
        self.skills[id.0 as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DraftChoice {
    Skill(SkillId),
    Evolution {
        skill: SkillId,
        tier: u8,
    },
    EvolveUr {
        skill: SkillId,
        path: super::skill_evolution::UrEvolutionPath,
    },
    ReplaceSkill {
        old: SkillId,
        new: SkillId,
    },
    Bond(BondCardId),
    ReplaceBond(BondId),
    Encounter(super::encounters::EncounterId),
}

impl DraftChoice {
    /// Encounters and bond replacements use the neutral frame.
    pub fn rarity(self) -> Rarity {
        match self {
            Self::Skill(skill) | Self::Evolution { skill, .. } | Self::EvolveUr { skill, .. } => {
                skill.definition().rarity
            }
            Self::ReplaceSkill { new, .. } => new.definition().rarity,
            Self::Bond(bond) => bond.rarity(),
            Self::ReplaceBond(_) | Self::Encounter(_) => Rarity::R,
        }
    }

    pub fn title(self) -> String {
        match self {
            Self::Skill(skill) | Self::Evolution { skill, .. } | Self::EvolveUr { skill, .. } => {
                skill.title()
            }
            Self::ReplaceSkill { new, .. } => new.title(),
            Self::Bond(bond) => bond.title().to_string(),
            Self::ReplaceBond(bond) => bond.title().to_string(),
            Self::Encounter(event) => event.name(super::GameLanguage::English).to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DraftKind {
    Skill,
    Bond,
    BondReplacement,
    Encounter,
}

#[derive(Resource, Default)]
pub struct DraftState {
    pub choices: Vec<DraftChoice>,
    pub root: Option<Entity>,
    pub opened_for: f32,
    pub kind: Option<DraftKind>,
    /// The new family already selected from a full seven-slot bond hand.
    /// It is applied only after the player chooses an existing family to remove.
    pub pending_bond_replacement: Option<BondCardId>,
}

#[derive(Resource)]
pub struct CaptureTarget(pub Handle<Image>);

#[derive(Resource, Default)]
pub struct Autoplay;

pub const DEFAULT_RUN_SEED: u64 = 0x5EED_CAFE_1234_5678;

/// Immutable identity for a run. Keep this value with bug reports or captures
/// to replay draft, market, wave and affix rolls in the same version.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunSeed(pub u64);

impl RunSeed {
    pub const fn new(seed: u64) -> Self {
        if seed == 0 {
            Self(DEFAULT_RUN_SEED)
        } else {
            Self(seed)
        }
    }
}

impl Default for RunSeed {
    fn default() -> Self {
        Self(DEFAULT_RUN_SEED)
    }
}

#[derive(Resource, Debug, Clone, Copy)]
pub struct RunRng(pub u64);

impl Default for RunRng {
    fn default() -> Self {
        Self(DEFAULT_RUN_SEED)
    }
}

impl RunRng {
    pub const fn from_seed(seed: u64) -> Self {
        Self(RunSeed::new(seed).0)
    }

    pub const fn state(&self) -> u64 {
        self.0
    }

    pub fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 16) as u32
    }

    pub fn unit(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }

    pub fn range(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.unit()
    }
}

pub fn parse_run_seed(value: &str) -> Option<u64> {
    let trimmed = value.trim();
    let parsed = if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        u64::from_str_radix(hex, 16).ok()
    } else {
        trimmed.parse::<u64>().ok()
    }?;
    Some(RunSeed::new(parsed).0)
}

#[derive(Resource, Clone)]
pub struct GameAssets {
    pub hero_scenes: [Handle<WorldAsset>; HeroKind::COUNT],
    pub enemy_scenes: [Handle<WorldAsset>; 12],
    pub boss_scenes: [Handle<WorldAsset>; 5],
    pub adventure_scenes: [Handle<WorldAsset>; 3],
    pub summon_scenes: [Handle<WorldAsset>; 6],
    pub projectile_mesh: Handle<Mesh>,
    pub fx_mesh: Handle<Mesh>,
    pub enemy_material: Handle<StandardMaterial>,
    pub enemy_materials: [Handle<StandardMaterial>; 4],
    pub elite_material: Handle<StandardMaterial>,
    pub boss_materials: [Handle<StandardMaterial>; 5],
    pub hero_materials: [Handle<StandardMaterial>; HeroKind::COUNT],
    pub projectile_material: Handle<StandardMaterial>,
    pub storm_material: Handle<StandardMaterial>,
}

#[cfg(test)]
mod survival_tests {
    use super::*;

    fn close(left: f32, right: f32) {
        assert!((left - right).abs() < 0.001, "{left} != {right}");
    }

    #[test]
    fn damage_meter_uses_an_exact_ten_second_fixed_bucket_window() {
        let mut meter = DamageMeter::default();
        meter.record(100.0);
        close(meter.recent_dps(), 10.0);
        meter.advance(9.999);
        close(meter.recent_dps(), 10.0);
        // A paused frame does not call advance; the window therefore remains exact.
        close(meter.recent_dps(), 10.0);
        meter.advance(0.001);
        close(meter.recent_dps(), 0.0);

        meter.record(25.0);
        meter.advance(1.0);
        meter.record(75.0);
        close(meter.recent_dps(), 10.0);
        meter.advance(20.0);
        close(meter.recent_dps(), 0.0);
    }

    #[test]
    fn hero_formulas_use_catalog_base_and_growth_values() {
        let storm = HeroVitals::for_hero(HeroKind::StormSavant, 1);
        close(storm.strength, 20.0);
        close(storm.agility, 18.0);
        close(storm.intelligence, 28.0);
        close(storm.max_health, 800.0);
        close(storm.max_energy, 504.0);
        close(storm.attack_damage(), 48.0);
        close(storm.spell_power(), 76.0);
        close(storm.summon_power(), 77.2);

        let level_30 = HeroVitals::for_hero(HeroKind::StormSavant, 30);
        close(level_30.strength, 78.0);
        close(level_30.agility, 76.0);
        close(level_30.intelligence, 126.6);
        close(
            HeroVitals::for_hero(HeroKind::StormSavant, 99).intelligence,
            HeroVitals::for_hero(HeroKind::StormSavant, MAX_HERO_LEVEL).intelligence,
        );
    }

    #[test]
    fn every_hero_vitals_formula_consumes_its_catalog_rarity() {
        let level = 7;
        let level_index = 6.0;
        for kind in HeroKind::ALL {
            let def = HEROES[kind.index()];
            let multiplier = def.rarity.hero_stat_multiplier();
            let vitals = HeroVitals::for_hero(kind, level);
            close(
                vitals.strength,
                (def.base_stats.strength + def.growth.strength * level_index) * multiplier,
            );
            close(
                vitals.agility,
                (def.base_stats.agility + def.growth.agility * level_index) * multiplier,
            );
            close(
                vitals.intelligence,
                (def.base_stats.intelligence + def.growth.intelligence * level_index) * multiplier,
            );
            close(
                vitals.max_health,
                def.base_health * multiplier + vitals.strength * 14.0,
            );
            close(
                vitals.max_energy,
                def.base_energy * multiplier + vitals.intelligence * 8.0,
            );
            assert_eq!(SelectedHero { kind, locked: true }.rarity(), def.rarity);
        }
    }

    #[test]
    fn level_growth_preserves_missing_health_and_runtime_states() {
        let mut vitals = HeroVitals::for_hero(HeroKind::StoneWarden, 1);
        vitals.health -= 100.0;
        vitals.deaths = 2;
        vitals.invulnerable_remaining = 0.5;
        let old_max = vitals.max_health;
        let delta = vitals.synchronize_level(HeroKind::StoneWarden, 2);
        close(vitals.max_health, old_max + delta.max_health);
        close(vitals.health, vitals.max_health - 100.0);
        assert_eq!(vitals.deaths, 2);
        close(vitals.invulnerable_remaining, 0.5);
    }

    #[test]
    fn mitigation_invulnerability_and_healing_share_one_vitals_state() {
        let mut vitals = HeroVitals {
            armor: 8.0,
            ..default()
        };
        close(vitals.take_physical_damage(100.0), 71.42857);
        close(vitals.take_magical_damage(100.0), 90.0);
        vitals.invulnerable_remaining = 0.01;
        close(vitals.take_physical_damage(9_999.0), 0.0);
        vitals.invulnerable_remaining = 0.0;
        vitals.health = vitals.max_health - 10.0;
        close(vitals.heal(100.0), 10.0);
        close(vitals.health, vitals.max_health);
    }

    #[test]
    fn gate_healing_cannot_skip_reconstruction_or_overheal() {
        let mut session = GameSession {
            core_hp: 0.0,
            ..default()
        };
        assert_eq!(session.heal_gate(5000.0), 0.0);
        assert_eq!(session.core_hp, 0.0);
        session.core_hp = 20.0;
        session.gate_revive_remaining = 2.0;
        assert_eq!(session.heal_gate(5000.0), 0.0);
        session.gate_revive_remaining = 0.0;
        assert_eq!(session.heal_gate(5000.0), 980.0);
        assert_eq!(session.core_hp, 1000.0);
    }
    #[test]
    fn gate_stands_between_starting_hero_and_incoming_army() {
        const {
            assert!(GATE_POSITION.x > HERO_RESPAWN_POSITION.x);
        }
    }

    #[test]
    fn successive_deaths_add_three_seconds_to_revive_time() {
        close(revive_delay_for_death(1), 8.0);
        close(revive_delay_for_death(2), 11.0);
        close(revive_delay_for_death(3), 14.0);
        close(revive_delay_for_death(4), 17.0);
    }

    #[test]
    fn run_seed_is_replayable_and_accepts_decimal_or_hex() {
        let seed = parse_run_seed("0xA11CE").unwrap();
        assert_eq!(seed, 0xA11CE);
        assert_eq!(parse_run_seed("659918"), Some(659_918));
        assert_eq!(parse_run_seed("0"), Some(DEFAULT_RUN_SEED));
        assert_eq!(parse_run_seed("not-a-seed"), None);

        let mut first = RunRng::from_seed(seed);
        let mut second = RunRng::from_seed(seed);
        let first_sequence: Vec<u32> = (0..16).map(|_| first.next_u32()).collect();
        let second_sequence: Vec<u32> = (0..16).map(|_| second.next_u32()).collect();
        assert_eq!(first_sequence, second_sequence);
        assert_ne!(RunRng::from_seed(1).next_u32(), first_sequence[0]);
    }
}
