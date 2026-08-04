use bevy::{prelude::*, window::PrimaryWindow};

use super::bonds::FortuneMarketRules;
use super::combat::SurvivalSet;
use super::mobile_layout::hud_metrics;
use super::{Autoplay, GameSession, HeroVitals, PlayerBuild, RunRng};

pub struct MarketPlugin;

impl Plugin for MarketPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MarketState>()
            .init_resource::<MarketEffects>()
            .init_resource::<FortuneMarketRules>()
            .add_systems(Startup, setup_market)
            .add_systems(
                Update,
                (
                    maintain_market_stock,
                    handle_market_actions,
                    tick_market_effects,
                    tick_market_notice,
                    apply_market_layout,
                    update_market_ui,
                )
                    .chain()
                    .before(SurvivalSet),
            );
    }
}

const MARKET_UNLOCK_AT: f32 = 180.0;
const STOCK_INTERVAL: f32 = 180.0;
// The current public match footage shows five stock rows before any equipment
// is purchased. Keep all five visible; MarketBadge remains valuable for its
// price reduction instead of fabricating a sixth row outside the observed UI.
const BASE_OFFER_COUNT: usize = 5;
const MAX_OFFER_COUNT: usize = 5;
const AUTOPLAY_MAX_PURCHASES_PER_GENERATION: usize = 2;
const AUTOPLAY_KILL_RESERVE: u32 = 100;
const REFRESH_COSTS: [u32; 4] = [60, 80, 100, 120];
const MARKET_LEVEL_COUNT: u8 = 5;
const FATAL_RECOVERY_RATIO: f32 = 0.35;
const FATAL_INVULNERABILITY_SECONDS: f32 = 1.0;
const LAST_OATH_EMPOWERMENT_SECONDS: f32 = 8.0;
const LAST_OATH_DAMAGE_MULTIPLIER: f32 = 1.50;
const WORLD_ANCHOR_CONTROL_GUARD_SECONDS: f32 = 1.0;
const GUARDIAN_INTERCEPT_COOLDOWN_SECONDS: f32 = 30.0;
const MARKET_PANEL_GAP: f32 = 6.0;
const MARKET_COMPACT_PANEL_PADDING: f32 = 8.0;
const MARKET_COMPACT_HEADER_HEIGHT: f32 = 28.0;
const MARKET_COMPACT_ROW_HEIGHT: f32 = 54.0;
const MARKET_COMPACT_ROW_WIDTH_PERCENT: f32 = 49.0;
const MARKET_COMPACT_MARK_WIDTH: f32 = 64.0;
const MARKET_COMPACT_TOGGLE_WIDTH: f32 = 168.0;
const MARKET_COMPACT_TOGGLE_HEIGHT: f32 = 52.0;
const MARKET_REFRESH_HEIGHT: f32 = 48.0;
const _: () = assert!(MARKET_REFRESH_HEIGHT >= 48.0);

// Public footage exposes these exact odds for market levels 1 through 5.
// Columns are White, Blue, Purple, Orange and Red.
const RARITY_WEIGHTS: [[u8; 5]; MARKET_LEVEL_COUNT as usize] = [
    [80, 15, 5, 0, 0],
    [50, 30, 10, 7, 3],
    [40, 25, 20, 9, 6],
    [20, 30, 25, 15, 10],
    [10, 25, 30, 20, 15],
];

const CONSUMABLES: [MarketItem; 5] = [
    MarketItem::CorePatch,
    MarketItem::SkillTuner,
    MarketItem::BondCompass,
    MarketItem::SlowHourglass,
    MarketItem::RevivalSeed,
];

const WHITE_EQUIPMENT: [MarketItem; 5] = [
    MarketItem::SerratedDisc,
    MarketItem::ArcLens,
    MarketItem::HardenedPlate,
    MarketItem::SpiritDrum,
    MarketItem::WindBoots,
];

const BLUE_EQUIPMENT: [MarketItem; 4] = [
    MarketItem::EchoCore,
    MarketItem::EmberHeart,
    MarketItem::StormCoil,
    MarketItem::FrostMirror,
];

const PURPLE_EQUIPMENT: [MarketItem; 4] = [
    MarketItem::ThreefoldRelic,
    MarketItem::BloodContract,
    MarketItem::MarketBadge,
    MarketItem::GuardianSeed,
];

const ORANGE_EQUIPMENT: [MarketItem; 3] = [
    MarketItem::AscendantCrown,
    MarketItem::WorldAnchor,
    MarketItem::LegionStandard,
];

const RED_EQUIPMENT: [MarketItem; 3] = [
    MarketItem::ChaosDie,
    MarketItem::PrismaticEngine,
    MarketItem::LastOath,
];

const ALL_EQUIPMENT: [MarketItem; 19] = [
    MarketItem::SerratedDisc,
    MarketItem::ArcLens,
    MarketItem::HardenedPlate,
    MarketItem::SpiritDrum,
    MarketItem::WindBoots,
    MarketItem::EchoCore,
    MarketItem::EmberHeart,
    MarketItem::StormCoil,
    MarketItem::FrostMirror,
    MarketItem::ThreefoldRelic,
    MarketItem::BloodContract,
    MarketItem::MarketBadge,
    MarketItem::GuardianSeed,
    MarketItem::AscendantCrown,
    MarketItem::ChaosDie,
    MarketItem::WorldAnchor,
    MarketItem::LegionStandard,
    MarketItem::PrismaticEngine,
    MarketItem::LastOath,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Rarity {
    White,
    Blue,
    Purple,
    Orange,
    Red,
}

impl Rarity {
    const ALL: [Self; 5] = [
        Self::White,
        Self::Blue,
        Self::Purple,
        Self::Orange,
        Self::Red,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::White => "WHITE",
            Self::Blue => "BLUE",
            Self::Purple => "PURPLE",
            Self::Orange => "ORANGE",
            Self::Red => "RED",
        }
    }

    const fn index(self) -> usize {
        match self {
            Self::White => 0,
            Self::Blue => 1,
            Self::Purple => 2,
            Self::Orange => 3,
            Self::Red => 4,
        }
    }

    fn color(self) -> Color {
        match self {
            Self::White => Color::srgb(0.82, 0.84, 0.86),
            Self::Blue => Color::srgb(0.24, 0.62, 1.0),
            Self::Purple => Color::srgb(0.72, 0.38, 1.0),
            Self::Orange => Color::srgb(1.0, 0.58, 0.16),
            Self::Red => Color::srgb(1.0, 0.24, 0.28),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MarketItem {
    CorePatch,
    SkillTuner,
    BondCompass,
    SlowHourglass,
    RevivalSeed,
    SerratedDisc,
    ArcLens,
    HardenedPlate,
    SpiritDrum,
    WindBoots,
    EchoCore,
    EmberHeart,
    StormCoil,
    FrostMirror,
    ThreefoldRelic,
    BloodContract,
    MarketBadge,
    GuardianSeed,
    AscendantCrown,
    ChaosDie,
    WorldAnchor,
    LegionStandard,
    PrismaticEngine,
    LastOath,
}

impl MarketItem {
    fn name(self) -> &'static str {
        match self {
            Self::CorePatch => "Core Patch",
            Self::SkillTuner => "Skill Tuner",
            Self::BondCompass => "Bond Compass",
            Self::SlowHourglass => "Slow Hourglass",
            Self::RevivalSeed => "Revival Seed",
            Self::SerratedDisc => "Serrated Disc",
            Self::ArcLens => "Arc Lens",
            Self::HardenedPlate => "Hardened Plate",
            Self::SpiritDrum => "Spirit Drum",
            Self::WindBoots => "Wind Boots",
            Self::EchoCore => "Echo Core",
            Self::EmberHeart => "Ember Heart",
            Self::StormCoil => "Storm Coil",
            Self::FrostMirror => "Frost Mirror",
            Self::ThreefoldRelic => "Threefold Relic",
            Self::BloodContract => "Blood Contract",
            Self::MarketBadge => "Market Badge",
            Self::GuardianSeed => "Guardian Seed",
            Self::AscendantCrown => "Ascendant Crown",
            Self::ChaosDie => "Chaos Die",
            Self::WorldAnchor => "World Anchor",
            Self::LegionStandard => "Legion Standard",
            Self::PrismaticEngine => "Prismatic Engine",
            Self::LastOath => "Last Oath",
        }
    }

    fn price(self) -> u32 {
        match self {
            Self::CorePatch => 120,
            Self::SkillTuner => 140,
            Self::BondCompass
            | Self::RevivalSeed
            | Self::SerratedDisc
            | Self::ArcLens
            | Self::HardenedPlate => 180,
            Self::SlowHourglass | Self::SpiritDrum => 200,
            Self::WindBoots => 160,
            Self::EchoCore | Self::GuardianSeed => 300,
            Self::EmberHeart | Self::StormCoil | Self::FrostMirror => 280,
            Self::ThreefoldRelic => 340,
            Self::BloodContract => 320,
            Self::MarketBadge => 260,
            Self::AscendantCrown | Self::LegionStandard => 480,
            Self::ChaosDie | Self::LastOath => 450,
            Self::WorldAnchor => 500,
            Self::PrismaticEngine => 520,
        }
    }

    fn rarity(self) -> Rarity {
        if self == Self::CorePatch || WHITE_EQUIPMENT.contains(&self) {
            Rarity::White
        } else if self == Self::SkillTuner || BLUE_EQUIPMENT.contains(&self) {
            Rarity::Blue
        } else if self == Self::BondCompass || PURPLE_EQUIPMENT.contains(&self) {
            Rarity::Purple
        } else if self == Self::SlowHourglass || ORANGE_EQUIPMENT.contains(&self) {
            Rarity::Orange
        } else {
            Rarity::Red
        }
    }

    fn equipment_index(self) -> Option<usize> {
        ALL_EQUIPMENT.iter().position(|item| *item == self)
    }

    fn is_consumable(self) -> bool {
        self.equipment_index().is_none()
    }

    fn tag(self) -> &'static str {
        self.rarity().label()
    }

    fn color(self) -> Color {
        self.rarity().color()
    }

    fn effect_text(self) -> &'static str {
        match self {
            Self::CorePatch => "restore 200 core integrity",
            Self::SkillTuner => "+1 skill refresh charge",
            Self::BondCompass => "next bond draft favors strongest bond",
            Self::SlowHourglass => "all enemies -50% speed for 15s",
            Self::RevivalSeed => "prevent the next fatal defeat",
            Self::SerratedDisc => "+22% attack; attacks gain splash",
            Self::ArcLens => "+18% spell range, +10% spell power",
            Self::HardenedPlate => "+6 armor, +20% maximum health",
            Self::SpiritDrum => "+25% summon attack speed",
            Self::WindBoots => "+15% move speed, -15% dodge cooldown",
            Self::EchoCore => "20% chance to echo skills at 45%",
            Self::EmberHeart => "burns can critically strike",
            Self::StormCoil => "every 5th attack chains lightning",
            Self::FrostMirror => "frozen deaths create an ice burst",
            Self::ThreefoldRelic => "attack, spell and summon empower each other",
            Self::BloodContract => "+40% low-health damage, +5% leech",
            Self::MarketBadge => "-20% market prices",
            Self::GuardianSeed => "+1 core/sec; intercept every 30s",
            Self::AscendantCrown => "top skill gains rank 6 and +30% power",
            Self::ChaosDie => "damage rolls 0.7x / 1.0x / 1.8x",
            Self::WorldAnchor => "bosses move -15%; dodge grants control guard",
            Self::LegionStandard => "+1 summon cap, +30% inherited stats",
            Self::PrismaticEngine => "+80% reactions and +40% area",
            Self::LastOath => "negate one fatal hit; empower for 8s",
        }
    }

    fn apply(
        self,
        session: &mut GameSession,
        build: &mut PlayerBuild,
        vitals: &mut HeroVitals,
        effects: &mut MarketEffects,
    ) {
        if let Some(index) = self.equipment_index() {
            effects.owned_equipment |= 1 << index;
        }

        match self {
            Self::CorePatch => session.core_hp = (session.core_hp + 200.0).min(session.core_max_hp),
            Self::SkillTuner => {
                session.skill_rerolls = session.skill_rerolls.saturating_add(1);
                effects.skill_tuner_uses = effects.skill_tuner_uses.saturating_add(1);
            }
            Self::BondCompass => {
                effects.bond_compass_uses = effects.bond_compass_uses.saturating_add(1)
            }
            Self::SlowHourglass => effects.enemy_slow_remaining = 15.0,
            Self::RevivalSeed => effects.revival_seeds = effects.revival_seeds.saturating_add(1),
            Self::SerratedDisc => {
                build.attack_damage *= 1.22;
                effects.splash_ratio = effects.splash_ratio.max(0.30);
            }
            Self::ArcLens => {
                effects.spell_area_multiplier *= 1.18;
                effects.spell_power_multiplier *= 1.10;
            }
            Self::HardenedPlate => {
                let health_ratio = if vitals.max_health > 0.0 {
                    (vitals.health / vitals.max_health).clamp(0.0, 1.0)
                } else {
                    1.0
                };
                vitals.command_health_multiplier *= 1.20;
                vitals.max_health *= 1.20;
                vitals.health = vitals.max_health * health_ratio;
                vitals.armor += 6.0;
            }
            Self::SpiritDrum => {
                effects.summon_attack_speed_multiplier *= 1.25;
            }
            Self::WindBoots => {
                build.move_speed *= 1.15;
                effects.dodge_cooldown_multiplier *= 0.85;
            }
            Self::EchoCore => effects.spell_echo_chance = 0.20,
            Self::EmberHeart => effects.burns_can_crit = true,
            Self::StormCoil => effects.storm_coil = true,
            Self::FrostMirror => effects.frost_mirror = true,
            Self::ThreefoldRelic => effects.threefold_multiplier = 1.12,
            Self::BloodContract => {
                effects.low_health_damage_multiplier = 1.40;
                effects.life_leech = 0.05;
            }
            Self::MarketBadge => {
                effects.market_discount = 0.20;
            }
            Self::GuardianSeed => effects.activate_guardian_seed(),
            Self::AscendantCrown => {
                if let Some(index) = highest_ranked_skill_index(build) {
                    build.skills[index] = build.skills[index].saturating_add(1).min(6);
                }
                effects.top_skill_power_multiplier = 1.30;
            }
            Self::ChaosDie => effects.chaos_die = true,
            Self::WorldAnchor => {
                effects.boss_speed_multiplier = 0.85;
                effects.world_anchor_control_guard = true;
            }
            Self::LegionStandard => {
                effects.summon_cap_bonus = effects.summon_cap_bonus.saturating_add(1);
                effects.summon_inherit_bonus += 0.30;
            }
            Self::PrismaticEngine => {
                effects.element_reaction_multiplier = 1.80;
                effects.element_reaction_area_multiplier = 1.40;
            }
            Self::LastOath => effects.last_oath_available = true,
        }
    }
}

fn highest_ranked_skill_index(build: &PlayerBuild) -> Option<usize> {
    build
        .skills
        .iter()
        .enumerate()
        .filter(|(_, rank)| **rank > 0)
        .max_by_key(|(index, rank)| (**rank, std::cmp::Reverse(*index)))
        .map(|(index, _)| index)
}

const fn chaos_multiplier_for_roll(roll: u32) -> f32 {
    match roll % 3 {
        0 => 0.7,
        1 => 1.0,
        _ => 1.8,
    }
}

fn roll_chaos_multiplier(rng: &mut RunRng) -> f32 {
    loop {
        let roll = rng.next_u32();
        // 2^32 is not divisible by three. Rejecting the single extra value
        // keeps all three outcomes exactly equiprobable.
        if roll != u32::MAX {
            return chaos_multiplier_for_roll(roll);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MarketOffer {
    item: MarketItem,
}

impl MarketOffer {
    const PLACEHOLDER: Self = Self {
        item: MarketItem::CorePatch,
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AutoplayPurchasePlan {
    index: usize,
    item: MarketItem,
}

#[derive(Resource)]
struct MarketState {
    offers: [MarketOffer; MAX_OFFER_COUNT],
    marked: [Option<MarketItem>; MAX_OFFER_COUNT],
    level: u8,
    manual_refreshes: u32,
    next_stock_at: f32,
    stock_generation: u32,
    free_refresh_used_generation: Option<u32>,
    autoplay_planned_generation: Option<u32>,
    autoplay_plan: [Option<AutoplayPurchasePlan>; AUTOPLAY_MAX_PURCHASES_PER_GENERATION],
    autoplay_purchases_generation: u8,
    open: bool,
    paused_by_market: bool,
    notice: String,
    notice_timer: f32,
}

impl Default for MarketState {
    fn default() -> Self {
        Self {
            offers: [MarketOffer::PLACEHOLDER; MAX_OFFER_COUNT],
            marked: [None; MAX_OFFER_COUNT],
            level: 1,
            manual_refreshes: 0,
            next_stock_at: MARKET_UNLOCK_AT + STOCK_INTERVAL,
            stock_generation: 0,
            free_refresh_used_generation: None,
            autoplay_planned_generation: None,
            autoplay_plan: [None; AUTOPLAY_MAX_PURCHASES_PER_GENERATION],
            autoplay_purchases_generation: 0,
            open: false,
            paused_by_market: false,
            notice: String::new(),
            notice_timer: 0.0,
        }
    }
}

impl MarketState {
    fn refresh_price(&self, effects: &MarketEffects, fortune: &FortuneMarketRules) -> u32 {
        if fortune.free_refresh_available
            && self.free_refresh_used_generation != Some(self.stock_generation)
        {
            return 0;
        }
        let tier = (self.manual_refreshes as usize).min(REFRESH_COSTS.len() - 1);
        discounted_cost(REFRESH_COSTS[tier], effects, fortune)
    }
}

fn market_level(bosses_defeated: u8) -> u8 {
    bosses_defeated
        .saturating_add(1)
        .clamp(1, MARKET_LEVEL_COUNT)
}

#[derive(Resource)]
pub(crate) struct MarketEffects {
    owned_equipment: u32,
    skill_tuner_uses: u8,
    bond_compass_uses: u8,
    revival_seeds: u8,
    enemy_slow_remaining: f32,
    splash_ratio: f32,
    spell_area_multiplier: f32,
    spell_power_multiplier: f32,
    summon_attack_speed_multiplier: f32,
    dodge_cooldown_multiplier: f32,
    spell_echo_chance: f32,
    burns_can_crit: bool,
    storm_coil: bool,
    storm_coil_attacks: u64,
    frost_mirror: bool,
    threefold_multiplier: f32,
    low_health_damage_multiplier: f32,
    life_leech: f32,
    market_discount: f32,
    core_regen_per_second: f32,
    guardian_seed_owned: bool,
    guardian_intercept_remaining: f32,
    guardian_intercept_ready: bool,
    top_skill_power_multiplier: f32,
    chaos_die: bool,
    boss_speed_multiplier: f32,
    world_anchor_control_guard: bool,
    control_guard_remaining: f32,
    summon_cap_bonus: u8,
    summon_inherit_bonus: f32,
    element_reaction_multiplier: f32,
    element_reaction_area_multiplier: f32,
    last_oath_available: bool,
    last_oath_empowerment_remaining: f32,
}

impl Default for MarketEffects {
    fn default() -> Self {
        Self {
            owned_equipment: 0,
            skill_tuner_uses: 0,
            bond_compass_uses: 0,
            revival_seeds: 0,
            enemy_slow_remaining: 0.0,
            splash_ratio: 0.0,
            spell_area_multiplier: 1.0,
            spell_power_multiplier: 1.0,
            summon_attack_speed_multiplier: 1.0,
            dodge_cooldown_multiplier: 1.0,
            spell_echo_chance: 0.0,
            burns_can_crit: false,
            storm_coil: false,
            storm_coil_attacks: 0,
            frost_mirror: false,
            threefold_multiplier: 1.0,
            low_health_damage_multiplier: 1.0,
            life_leech: 0.0,
            market_discount: 0.0,
            core_regen_per_second: 0.0,
            guardian_seed_owned: false,
            guardian_intercept_remaining: 0.0,
            guardian_intercept_ready: false,
            top_skill_power_multiplier: 1.0,
            chaos_die: false,
            boss_speed_multiplier: 1.0,
            world_anchor_control_guard: false,
            control_guard_remaining: 0.0,
            summon_cap_bonus: 0,
            summon_inherit_bonus: 0.0,
            element_reaction_multiplier: 1.0,
            element_reaction_area_multiplier: 1.0,
            last_oath_available: false,
            last_oath_empowerment_remaining: 0.0,
        }
    }
}

impl MarketEffects {
    fn owns(&self, item: MarketItem) -> bool {
        item.equipment_index()
            .is_some_and(|index| self.owned_equipment & (1 << index) != 0)
    }

    fn visible_offer_count(&self) -> usize {
        BASE_OFFER_COUNT
    }

    pub(crate) fn enemy_movement_multiplier(&self, is_boss: bool) -> f32 {
        let slow = if self.enemy_slow_remaining > 0.0 {
            0.5
        } else {
            1.0
        };
        let anchor = if is_boss {
            self.boss_speed_multiplier
        } else {
            1.0
        };
        slow * anchor
    }

    pub(crate) fn dodge_cooldown_multiplier(&self) -> f32 {
        self.dodge_cooldown_multiplier
    }

    pub(crate) fn splash_ratio(&self) -> f32 {
        self.splash_ratio
    }

    pub(crate) fn spell_area_multiplier(&self) -> f32 {
        self.spell_area_multiplier
    }

    pub(crate) fn effective_spell_power(&self, base: f32) -> f32 {
        base * self.spell_power_multiplier
    }

    pub(crate) fn effective_summon_power(&self, base: f32) -> f32 {
        base * (1.0 + self.summon_inherit_bonus)
    }

    pub(crate) fn summon_attack_speed_multiplier(&self) -> f32 {
        self.summon_attack_speed_multiplier
    }

    pub(crate) fn spell_echo_chance(&self) -> f32 {
        self.spell_echo_chance
    }

    pub(crate) fn burns_can_crit(&self) -> bool {
        self.burns_can_crit
    }

    pub(crate) fn record_basic_attack(&mut self) -> bool {
        if !self.storm_coil {
            return false;
        }
        self.storm_coil_attacks = self.storm_coil_attacks.wrapping_add(1);
        self.storm_coil_attacks.is_multiple_of(5)
    }

    pub(crate) fn frost_mirror_enabled(&self) -> bool {
        self.frost_mirror
    }

    pub(crate) fn threefold_multiplier(&self) -> f32 {
        self.threefold_multiplier
    }

    pub(crate) fn low_health_damage_multiplier(&self, vitals: &HeroVitals) -> f32 {
        if vitals.alive && vitals.max_health > 0.0 && vitals.health / vitals.max_health <= 0.50 {
            self.low_health_damage_multiplier
        } else {
            1.0
        }
    }

    pub(crate) fn life_leech(&self) -> f32 {
        self.life_leech
    }

    pub(crate) fn top_skill_multiplier(&self, skill_index: Option<u8>, build: &PlayerBuild) -> f32 {
        if highest_ranked_skill_index(build)
            .is_some_and(|highest| skill_index.map(usize::from) == Some(highest))
        {
            self.top_skill_power_multiplier
        } else {
            1.0
        }
    }

    pub(crate) fn chaos_damage_multiplier(
        &self,
        rng: &mut RunRng,
        fortune_take_better: bool,
    ) -> f32 {
        if !self.chaos_die {
            return 1.0;
        }
        let first = roll_chaos_multiplier(rng);
        if fortune_take_better {
            first.max(roll_chaos_multiplier(rng))
        } else {
            first
        }
    }

    pub(crate) fn summon_cap_bonus(&self) -> u8 {
        self.summon_cap_bonus
    }

    pub(crate) fn element_reaction_multiplier(&self) -> f32 {
        self.element_reaction_multiplier
    }

    pub(crate) fn element_reaction_area_multiplier(&self) -> f32 {
        self.element_reaction_area_multiplier
    }

    pub(crate) fn last_oath_damage_multiplier(&self) -> f32 {
        if self.last_oath_empowerment_remaining > 0.0 {
            LAST_OATH_DAMAGE_MULTIPLIER
        } else {
            1.0
        }
    }

    pub(crate) fn begin_dodge_control_guard(&mut self) {
        if self.world_anchor_control_guard {
            self.control_guard_remaining = WORLD_ANCHOR_CONTROL_GUARD_SECONDS;
        }
    }

    pub(crate) fn try_apply_hero_stun(&self, vitals: &mut HeroVitals, duration: f32) -> bool {
        if self.control_guard_remaining > 0.0 || duration <= 0.0 {
            return false;
        }
        vitals.stunned_remaining = vitals.stunned_remaining.max(duration);
        true
    }

    pub(crate) fn bond_compass_available(&self) -> bool {
        self.bond_compass_uses > 0
    }

    pub(crate) fn consume_bond_compass(&mut self) -> bool {
        if self.bond_compass_uses == 0 {
            return false;
        }
        self.bond_compass_uses -= 1;
        true
    }

    pub(crate) fn intercept_hero_fatal(&mut self, vitals: &mut HeroVitals) -> bool {
        if self.last_oath_available {
            self.last_oath_available = false;
            self.last_oath_empowerment_remaining = LAST_OATH_EMPOWERMENT_SECONDS;
        } else if self.revival_seeds > 0 {
            self.revival_seeds -= 1;
        } else {
            return false;
        }

        vitals.alive = true;
        vitals.health =
            (vitals.max_health * FATAL_RECOVERY_RATIO).clamp(1.0, vitals.max_health.max(1.0));
        vitals.revive_remaining = 0.0;
        vitals.invulnerable_remaining = vitals
            .invulnerable_remaining
            .max(FATAL_INVULNERABILITY_SECONDS);
        true
    }

    #[allow(dead_code)]
    pub(crate) fn last_oath_empowerment_remaining(&self) -> f32 {
        self.last_oath_empowerment_remaining
    }

    pub(crate) fn try_intercept_ordinary_breach(&mut self) -> bool {
        if !self.guardian_seed_owned || !self.guardian_intercept_ready {
            return false;
        }
        self.guardian_intercept_ready = false;
        self.guardian_intercept_remaining = GUARDIAN_INTERCEPT_COOLDOWN_SECONDS;
        true
    }

    fn activate_guardian_seed(&mut self) {
        self.core_regen_per_second = 1.0;
        self.guardian_seed_owned = true;
        self.guardian_intercept_ready = false;
        self.guardian_intercept_remaining = GUARDIAN_INTERCEPT_COOLDOWN_SECONDS;
    }

    fn advance(&mut self, delta: f32) {
        self.enemy_slow_remaining = (self.enemy_slow_remaining - delta).max(0.0);
        self.control_guard_remaining = (self.control_guard_remaining - delta).max(0.0);
        self.last_oath_empowerment_remaining =
            (self.last_oath_empowerment_remaining - delta).max(0.0);
        if self.guardian_seed_owned && !self.guardian_intercept_ready {
            self.guardian_intercept_remaining =
                (self.guardian_intercept_remaining - delta).max(0.0);
            if self.guardian_intercept_remaining <= 0.0001 {
                self.guardian_intercept_remaining = 0.0;
                self.guardian_intercept_ready = true;
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn with_revival_seed_for_test() -> Self {
        Self {
            revival_seeds: 1,
            ..default()
        }
    }
}

#[derive(Component)]
struct MarketOfferButton(usize);

#[derive(Component)]
struct MarketOfferRow(usize);

#[derive(Component)]
struct MarketOfferGrid;

#[derive(Component)]
struct MarketHeader;

#[derive(Component)]
struct MarketOfferLabel(usize);

#[derive(Component)]
struct MarketMarkButton(usize);

#[derive(Component)]
struct MarketMarkLabel(usize);

#[derive(Component)]
struct MarketRefreshButton;

#[derive(Component)]
struct MarketRefreshLabel;

#[derive(Component)]
struct MarketWalletLabel;

#[derive(Component)]
struct MarketPanel;

#[derive(Component)]
struct MarketToggleButton;

#[derive(Component)]
struct MarketToggleLabel;

#[derive(Clone, Copy, Debug, PartialEq)]
struct MarketLayout {
    compact: bool,
    safe_x: f32,
    safe_y: f32,
    panel_top: f32,
    row_width_percent: f32,
    row_height: f32,
    mark_width: f32,
    toggle_right: f32,
    toggle_width: f32,
    toggle_height: f32,
}

impl MarketLayout {
    fn minimum_compact_panel_height(self) -> f32 {
        let grid_rows = MAX_OFFER_COUNT.div_ceil(2) as f32;
        MARKET_COMPACT_PANEL_PADDING * 2.0
            + MARKET_COMPACT_HEADER_HEIGHT
            + grid_rows * self.row_height
            + (grid_rows - 1.0) * MARKET_PANEL_GAP
            + MARKET_REFRESH_HEIGHT
            + MARKET_PANEL_GAP * 2.0
    }
}

fn market_layout(width: f32, height: f32) -> MarketLayout {
    let metrics = hud_metrics(width, height);
    if metrics.compact {
        MarketLayout {
            compact: true,
            safe_x: metrics.safe_x,
            safe_y: metrics.safe_y,
            panel_top: metrics.safe_y + MARKET_COMPACT_TOGGLE_HEIGHT + MARKET_PANEL_GAP,
            row_width_percent: MARKET_COMPACT_ROW_WIDTH_PERCENT,
            row_height: MARKET_COMPACT_ROW_HEIGHT,
            mark_width: MARKET_COMPACT_MARK_WIDTH,
            toggle_right: metrics.safe_x + 164.0,
            toggle_width: MARKET_COMPACT_TOGGLE_WIDTH,
            toggle_height: MARKET_COMPACT_TOGGLE_HEIGHT,
        }
    } else {
        MarketLayout {
            compact: false,
            safe_x: 18.0,
            safe_y: 18.0,
            panel_top: 0.0,
            row_width_percent: 100.0,
            row_height: 52.0,
            mark_width: 72.0,
            toggle_right: 18.0,
            toggle_width: 230.0,
            toggle_height: 48.0,
        }
    }
}

fn setup_market(
    mut commands: Commands,
    mut state: ResMut<MarketState>,
    effects: Res<MarketEffects>,
    fortune: Res<FortuneMarketRules>,
    mut rng: ResMut<RunRng>,
) {
    let level = state.level;
    reroll_all(&mut state, &effects, &fortune, &mut rng, level);

    commands
        .spawn((
            Button,
            MarketToggleButton,
            Node {
                position_type: PositionType::Absolute,
                right: px(18),
                bottom: px(18),
                width: px(230),
                height: px(48),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(9)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.19, 0.105, 0.035, 0.94)),
            BorderColor::all(Color::srgba(0.92, 0.58, 0.20, 0.72)),
            GlobalZIndex(21),
        ))
        .with_children(|button| {
            button.spawn((
                MarketToggleLabel,
                Text::new("B  BLACK MARKET  [LOCKED 03:00]"),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.78, 0.38)),
            ));
        });

    commands
        .spawn((
            MarketPanel,
            Node {
                position_type: PositionType::Absolute,
                right: px(18),
                bottom: px(70),
                width: px(360),
                display: Display::None,
                padding: UiRect::all(px(10)),
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(10)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.025, 0.02, 0.035, 0.94)),
            BorderColor::all(Color::srgba(0.7, 0.45, 0.18, 0.65)),
            GlobalZIndex(20),
        ))
        .with_children(|panel| {
            panel
                .spawn((
                    MarketHeader,
                    Node {
                        width: percent(100),
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                ))
                .with_children(|header| {
                    header.spawn((
                        Text::new("BLACK MARKET"),
                        TextFont {
                            font_size: FontSize::Px(15.0),
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.72, 0.3)),
                    ));
                    header.spawn((
                        MarketWalletLabel,
                        Text::new("KILLS 0000"),
                        TextFont {
                            font_size: FontSize::Px(11.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.88, 0.9, 0.94)),
                        TextLayout::justify(Justify::Right),
                    ));
                });

            panel
                .spawn((
                    MarketOfferGrid,
                    Node {
                        width: percent(100),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(MARKET_PANEL_GAP),
                        ..default()
                    },
                ))
                .with_children(|grid| {
                    for index in 0..MAX_OFFER_COUNT {
                        grid.spawn((
                            MarketOfferRow(index),
                            Node {
                                width: percent(100),
                                min_height: px(52),
                                display: if index < BASE_OFFER_COUNT {
                                    Display::Flex
                                } else {
                                    Display::None
                                },
                                column_gap: px(MARKET_PANEL_GAP),
                                align_items: AlignItems::Center,
                                ..default()
                            },
                        ))
                        .with_children(|row| {
                            row.spawn((
                                Button,
                                MarketOfferButton(index),
                                Node {
                                    flex_grow: 1.0,
                                    min_height: px(52),
                                    padding: UiRect::axes(px(8), px(5)),
                                    align_items: AlignItems::Center,
                                    border_radius: BorderRadius::all(px(6)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.09, 0.075, 0.11)),
                            ))
                            .with_children(|button| {
                                button.spawn((
                                    MarketOfferLabel(index),
                                    Text::new(""),
                                    TextFont {
                                        font_size: FontSize::Px(11.0),
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                            });
                            row.spawn((
                                Button,
                                MarketMarkButton(index),
                                Node {
                                    width: px(72),
                                    min_height: px(52),
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    border_radius: BorderRadius::all(px(6)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.10, 0.16, 0.20)),
                            ))
                            .with_children(|button| {
                                button.spawn((
                                    MarketMarkLabel(index),
                                    Text::new("AUTO"),
                                    TextFont {
                                        font_size: FontSize::Px(11.0),
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.68, 0.84, 0.92)),
                                ));
                            });
                        });
                    }
                });

            panel
                .spawn((
                    Button,
                    MarketRefreshButton,
                    Node {
                        width: percent(100),
                        height: px(MARKET_REFRESH_HEIGHT),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        border_radius: BorderRadius::all(px(6)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.24, 0.14, 0.055)),
                ))
                .with_children(|button| {
                    button.spawn((
                        MarketRefreshLabel,
                        Text::new("REFRESH"),
                        TextFont {
                            font_size: FontSize::Px(12.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        });
}

fn maintain_market_stock(
    session: Res<GameSession>,
    effects: Res<MarketEffects>,
    fortune: Res<FortuneMarketRules>,
    mut state: ResMut<MarketState>,
    mut rng: ResMut<RunRng>,
) {
    state.level = market_level(session.bosses_defeated);
    if session.elapsed < MARKET_UNLOCK_AT {
        return;
    }
    while session.elapsed >= state.next_stock_at {
        let level = state.level;
        reroll_all(&mut state, &effects, &fortune, &mut rng, level);
        state.next_stock_at += STOCK_INTERVAL;
        state.stock_generation = state.stock_generation.saturating_add(1);
        if state.open {
            set_notice(&mut state, "Automatic restock".to_string());
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_market_actions(
    keyboard: Res<ButtonInput<KeyCode>>,
    autoplay: Option<Res<Autoplay>>,
    mut state: ResMut<MarketState>,
    mut effects: ResMut<MarketEffects>,
    mut session: ResMut<GameSession>,
    mut build: ResMut<PlayerBuild>,
    mut vitals: ResMut<HeroVitals>,
    fortune: Res<FortuneMarketRules>,
    mut rng: ResMut<RunRng>,
    offer_interactions: Query<(&Interaction, &MarketOfferButton), Changed<Interaction>>,
    mark_interactions: Query<(&Interaction, &MarketMarkButton), Changed<Interaction>>,
    refresh_interactions: Query<&Interaction, (With<MarketRefreshButton>, Changed<Interaction>)>,
    toggle_interactions: Query<&Interaction, (With<MarketToggleButton>, Changed<Interaction>)>,
) {
    let toggle_requested = keyboard.just_pressed(KeyCode::KeyB)
        || toggle_interactions
            .iter()
            .any(|interaction| *interaction == Interaction::Pressed);
    if toggle_requested {
        if state.open {
            close_market(&mut state, &mut session);
        } else if session.elapsed < MARKET_UNLOCK_AT {
            let remaining = (MARKET_UNLOCK_AT - session.elapsed).ceil() as u32;
            set_notice(
                &mut state,
                format!("Unlocks in {:02}:{:02}", remaining / 60, remaining % 60),
            );
        } else if session.finished {
            set_notice(&mut state, "Market closed after the run".to_string());
        } else if session.paused {
            set_notice(&mut state, "Close the current choice first".to_string());
        } else {
            state.open = true;
            state.paused_by_market = true;
            session.paused = true;
        }
    }

    if state.open && session.elapsed >= MARKET_UNLOCK_AT {
        for (interaction, button) in &mark_interactions {
            if *interaction != Interaction::Pressed || button.0 >= effects.visible_offer_count() {
                continue;
            }
            let item = state.offers[button.0].item;
            if effects.owns(item) {
                state.marked[button.0] = None;
                set_notice(&mut state, "Owned equipment cannot be marked".to_string());
            } else if state.marked[button.0] == Some(item) {
                state.marked[button.0] = None;
                set_notice(&mut state, format!("Auto-buy canceled: {}", item.name()));
            } else {
                state.marked[button.0] = Some(item);
                set_notice(&mut state, format!("Auto-buy marked: {}", item.name()));
            }
        }

        for (interaction, button) in &offer_interactions {
            if *interaction != Interaction::Pressed {
                continue;
            }
            let outcome = purchase_offer(
                button.0,
                &mut state,
                &mut effects,
                &mut session,
                &mut build,
                &mut vitals,
                &fortune,
                &mut rng,
            );
            match outcome {
                PurchaseOutcome::Bought(item) => {
                    set_notice(&mut state, format!("Bought {}", item.name()));
                }
                PurchaseOutcome::Unaffordable(shortfall) => {
                    set_notice(&mut state, format!("Need {shortfall} more kills"));
                }
                PurchaseOutcome::Owned => set_notice(
                    &mut state,
                    "Equipment already owned; stock replaced".to_string(),
                ),
                PurchaseOutcome::Hidden => {}
            }
        }

        for interaction in &refresh_interactions {
            if *interaction != Interaction::Pressed {
                continue;
            }
            let price = state.refresh_price(&effects, &fortune);
            if session.kills < price {
                set_notice(
                    &mut state,
                    format!("Refresh needs {} more kills", price - session.kills),
                );
                continue;
            }
            session.kills -= price;
            state.manual_refreshes = state.manual_refreshes.saturating_add(1);
            if fortune.free_refresh_available {
                state.free_refresh_used_generation = Some(state.stock_generation);
            }
            let level = state.level;
            reroll_all(&mut state, &effects, &fortune, &mut rng, level);
            set_notice(
                &mut state,
                if price == 0 {
                    "Fortune refresh used".to_string()
                } else {
                    "Stock refreshed".to_string()
                },
            );
        }
    }

    if session.elapsed >= MARKET_UNLOCK_AT && !session.finished {
        if autoplay.is_some() {
            prepare_autoplay_plan(&mut state, &effects, &session);
            process_autoplay_purchases(
                &mut state,
                &mut effects,
                &mut session,
                &mut build,
                &mut vitals,
                &fortune,
                &mut rng,
            );
        }
        process_auto_purchases(
            &mut state,
            &mut effects,
            &mut session,
            &mut build,
            &mut vitals,
            &fortune,
            &mut rng,
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PurchaseOutcome {
    Bought(MarketItem),
    Unaffordable(u32),
    Owned,
    Hidden,
}

fn prepare_autoplay_plan(state: &mut MarketState, effects: &MarketEffects, session: &GameSession) {
    if state.autoplay_planned_generation == Some(state.stock_generation) {
        return;
    }

    state.autoplay_planned_generation = Some(state.stock_generation);
    state.autoplay_plan = [None; AUTOPLAY_MAX_PURCHASES_PER_GENERATION];
    state.autoplay_purchases_generation = 0;

    let mut candidates = Vec::with_capacity(effects.visible_offer_count());
    for index in 0..effects.visible_offer_count() {
        let item = state.offers[index].item;
        if effects.owns(item) {
            continue;
        }
        if let Some(priority) = autoplay_purchase_priority(item, session, state.stock_generation) {
            candidates.push((priority, index, item));
        }
    }
    candidates
        .sort_unstable_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));

    for (slot, (_, index, item)) in candidates
        .into_iter()
        .take(AUTOPLAY_MAX_PURCHASES_PER_GENERATION)
        .enumerate()
    {
        state.autoplay_plan[slot] = Some(AutoplayPurchasePlan { index, item });
    }
}

fn autoplay_purchase_priority(
    item: MarketItem,
    session: &GameSession,
    stock_generation: u32,
) -> Option<u16> {
    let core_ratio = if session.core_max_hp > 0.0 {
        session.core_hp / session.core_max_hp
    } else {
        1.0
    };
    let priority = match item {
        MarketItem::CorePatch if core_ratio <= 0.60 => 1_000,
        MarketItem::CorePatch if core_ratio <= 0.85 => 930,
        MarketItem::CorePatch => return None,
        MarketItem::GuardianSeed => 980,
        MarketItem::LastOath => 970,
        MarketItem::WorldAnchor => 960,
        MarketItem::MarketBadge if stock_generation <= 2 => 920,
        MarketItem::HardenedPlate => 900,
        MarketItem::SerratedDisc => 880,
        MarketItem::ArcLens => 870,
        MarketItem::RevivalSeed => 860,
        MarketItem::PrismaticEngine => 850,
        MarketItem::BloodContract => 830,
        MarketItem::EchoCore => 820,
        MarketItem::LegionStandard => 810,
        MarketItem::ThreefoldRelic => 800,
        MarketItem::StormCoil => 780,
        MarketItem::FrostMirror => 770,
        MarketItem::EmberHeart => 760,
        MarketItem::SpiritDrum => 750,
        MarketItem::AscendantCrown => 740,
        MarketItem::WindBoots => 730,
        MarketItem::SlowHourglass => 700,
        MarketItem::ChaosDie => 680,
        MarketItem::SkillTuner => 620,
        MarketItem::BondCompass => 600,
        MarketItem::MarketBadge => 560,
    };
    Some(priority)
}

#[allow(clippy::too_many_arguments)]
fn process_autoplay_purchases(
    state: &mut MarketState,
    effects: &mut MarketEffects,
    session: &mut GameSession,
    build: &mut PlayerBuild,
    vitals: &mut HeroVitals,
    fortune: &FortuneMarketRules,
    rng: &mut RunRng,
) {
    let mut bought = Vec::new();
    for plan_index in 0..AUTOPLAY_MAX_PURCHASES_PER_GENERATION {
        let Some(plan) = state.autoplay_plan[plan_index] else {
            continue;
        };
        if state.offers.get(plan.index).map(|offer| offer.item) != Some(plan.item)
            || effects.owns(plan.item)
            || autoplay_purchase_priority(plan.item, session, state.stock_generation).is_none()
        {
            state.autoplay_plan[plan_index] = None;
            continue;
        }

        let price = discounted_cost(plan.item.price(), effects, fortune);
        if session.kills.saturating_sub(AUTOPLAY_KILL_RESERVE) < price {
            continue;
        }
        match purchase_offer(
            plan.index, state, effects, session, build, vitals, fortune, rng,
        ) {
            PurchaseOutcome::Bought(item) => {
                state.autoplay_plan[plan_index] = None;
                state.autoplay_purchases_generation =
                    state.autoplay_purchases_generation.saturating_add(1);
                bought.push(item.name());
            }
            PurchaseOutcome::Owned | PurchaseOutcome::Hidden => {
                state.autoplay_plan[plan_index] = None;
            }
            PurchaseOutcome::Unaffordable(_) => {}
        }
    }
    if !bought.is_empty() {
        set_notice(state, format!("Commander bought {}", bought.join(", ")));
    }
}

fn purchase_offer(
    index: usize,
    state: &mut MarketState,
    effects: &mut MarketEffects,
    session: &mut GameSession,
    build: &mut PlayerBuild,
    vitals: &mut HeroVitals,
    fortune: &FortuneMarketRules,
    rng: &mut RunRng,
) -> PurchaseOutcome {
    if index >= effects.visible_offer_count() {
        return PurchaseOutcome::Hidden;
    }
    let item = state.offers[index].item;
    if effects.owns(item) {
        state.marked[index] = None;
        let level = state.level;
        replace_offer(index, state, effects, fortune, rng, level);
        return PurchaseOutcome::Owned;
    }

    let price = discounted_cost(item.price(), effects, fortune);
    if session.kills < price {
        return PurchaseOutcome::Unaffordable(price - session.kills);
    }

    session.kills -= price;
    item.apply(session, build, vitals, effects);
    state.marked[index] = None;
    let level = state.level;
    replace_offer(index, state, effects, fortune, rng, level);
    PurchaseOutcome::Bought(item)
}

fn process_auto_purchases(
    state: &mut MarketState,
    effects: &mut MarketEffects,
    session: &mut GameSession,
    build: &mut PlayerBuild,
    vitals: &mut HeroVitals,
    fortune: &FortuneMarketRules,
    rng: &mut RunRng,
) {
    let mut bought = Vec::new();
    let visible = effects.visible_offer_count();
    for index in 0..visible {
        let Some(marked_item) = state.marked[index] else {
            continue;
        };
        if state.offers[index].item != marked_item {
            state.marked[index] = None;
            continue;
        }
        match purchase_offer(index, state, effects, session, build, vitals, fortune, rng) {
            PurchaseOutcome::Bought(item) => bought.push(item.name()),
            PurchaseOutcome::Owned => {}
            PurchaseOutcome::Unaffordable(_) | PurchaseOutcome::Hidden => continue,
        }
    }
    if !bought.is_empty() {
        set_notice(state, format!("Auto-bought {}", bought.join(", ")));
    }
}

fn tick_market_effects(
    time: Res<Time>,
    mut session: ResMut<GameSession>,
    mut effects: ResMut<MarketEffects>,
) {
    if session.paused || session.finished {
        return;
    }
    let delta = time.delta_secs();
    effects.advance(delta);
    if effects.core_regen_per_second > 0.0 {
        session.core_hp =
            (session.core_hp + effects.core_regen_per_second * delta).min(session.core_max_hp);
    }
}

fn tick_market_notice(time: Res<Time>, session: Res<GameSession>, mut state: ResMut<MarketState>) {
    if session.paused || session.finished || state.notice_timer <= 0.0 {
        return;
    }
    state.notice_timer = (state.notice_timer - time.delta_secs()).max(0.0);
    if state.notice_timer == 0.0 {
        state.notice.clear();
    }
}

#[allow(clippy::type_complexity)]
fn apply_market_layout(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut nodes: Query<
        (
            &mut Node,
            Option<&MarketPanel>,
            Option<&MarketHeader>,
            Option<&MarketOfferGrid>,
            Option<&MarketOfferRow>,
            Option<&MarketOfferButton>,
            Option<&MarketMarkButton>,
            Option<&MarketRefreshButton>,
            Option<&MarketToggleButton>,
        ),
        Or<(
            With<MarketPanel>,
            With<MarketHeader>,
            With<MarketOfferGrid>,
            With<MarketOfferRow>,
            With<MarketOfferButton>,
            With<MarketMarkButton>,
            With<MarketRefreshButton>,
            With<MarketToggleButton>,
        )>,
    >,
) {
    let layout = windows
        .single()
        .map(|window| market_layout(window.width(), window.height()))
        .unwrap_or_else(|_| market_layout(1280.0, 720.0));

    for (mut node, panel, header, grid, row, offer, mark, refresh, toggle) in &mut nodes {
        if panel.is_some() {
            node.padding = UiRect::all(px(if layout.compact {
                MARKET_COMPACT_PANEL_PADDING
            } else {
                10.0
            }));
            node.row_gap = px(MARKET_PANEL_GAP);
            node.height = Val::Auto;
            node.min_height = px(if layout.compact {
                layout.minimum_compact_panel_height()
            } else {
                0.0
            });
            if layout.compact {
                node.left = px(layout.safe_x);
                node.right = px(layout.safe_x);
                node.top = px(layout.panel_top);
                node.bottom = px(layout.safe_y);
                node.width = Val::Auto;
            } else {
                node.left = Val::Auto;
                node.right = px(18);
                node.top = Val::Auto;
                node.bottom = px(70);
                node.width = px(360);
            }
        } else if header.is_some() {
            node.width = percent(100);
            node.min_height = px(if layout.compact {
                MARKET_COMPACT_HEADER_HEIGHT
            } else {
                0.0
            });
        } else if grid.is_some() {
            node.width = percent(100);
            node.height = Val::Auto;
            node.flex_direction = if layout.compact {
                FlexDirection::Row
            } else {
                FlexDirection::Column
            };
            node.flex_wrap = if layout.compact {
                FlexWrap::Wrap
            } else {
                FlexWrap::NoWrap
            };
            node.justify_content = JustifyContent::FlexStart;
            node.align_items = AlignItems::Stretch;
            node.column_gap = px(if layout.compact {
                MARKET_PANEL_GAP
            } else {
                0.0
            });
            node.row_gap = px(MARKET_PANEL_GAP);
        } else if row.is_some() {
            node.width = percent(layout.row_width_percent);
            node.height = if layout.compact {
                px(layout.row_height)
            } else {
                Val::Auto
            };
            node.min_height = px(layout.row_height);
            node.flex_grow = 0.0;
            node.flex_shrink = 0.0;
        } else if offer.is_some() {
            node.height = if layout.compact {
                px(layout.row_height)
            } else {
                Val::Auto
            };
            node.min_height = px(layout.row_height);
        } else if mark.is_some() {
            node.width = px(layout.mark_width);
            node.height = if layout.compact {
                px(layout.row_height)
            } else {
                Val::Auto
            };
            node.min_height = px(layout.row_height);
        } else if refresh.is_some() {
            node.width = percent(100);
            node.height = px(MARKET_REFRESH_HEIGHT);
            node.min_height = px(MARKET_REFRESH_HEIGHT);
        } else if toggle.is_some() {
            node.left = Val::Auto;
            node.right = px(layout.toggle_right);
            node.width = px(layout.toggle_width);
            node.height = px(layout.toggle_height);
            node.min_height = px(layout.toggle_height);
            if layout.compact {
                node.top = px(layout.safe_y);
                node.bottom = Val::Auto;
            } else {
                node.top = Val::Auto;
                node.bottom = px(layout.safe_y);
            }
        }
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn update_market_ui(
    windows: Query<&Window, With<PrimaryWindow>>,
    state: Res<MarketState>,
    effects: Res<MarketEffects>,
    fortune: Res<FortuneMarketRules>,
    session: Res<GameSession>,
    mut item_labels: Query<
        (
            Option<&MarketOfferLabel>,
            Option<&MarketMarkLabel>,
            &mut Text,
            &mut TextColor,
            &mut TextFont,
        ),
        (
            Or<(With<MarketOfferLabel>, With<MarketMarkLabel>)>,
            Without<MarketWalletLabel>,
            Without<MarketRefreshLabel>,
            Without<MarketToggleLabel>,
        ),
    >,
    mut wallet_labels: Query<
        (&mut Text, &mut TextFont),
        (
            With<MarketWalletLabel>,
            Without<MarketOfferLabel>,
            Without<MarketMarkLabel>,
            Without<MarketRefreshLabel>,
            Without<MarketToggleLabel>,
        ),
    >,
    mut refresh_labels: Query<
        (&mut Text, &mut TextFont),
        (
            With<MarketRefreshLabel>,
            Without<MarketOfferLabel>,
            Without<MarketMarkLabel>,
            Without<MarketWalletLabel>,
            Without<MarketToggleLabel>,
        ),
    >,
    mut action_buttons: Query<
        (
            Option<&MarketOfferButton>,
            Option<&MarketMarkButton>,
            &mut BackgroundColor,
        ),
        (
            Or<(With<MarketOfferButton>, With<MarketMarkButton>)>,
            Without<MarketPanel>,
            Without<MarketOfferRow>,
        ),
    >,
    mut rows: Query<
        (&MarketOfferRow, &mut Node),
        (
            Without<MarketPanel>,
            Without<MarketOfferButton>,
            Without<MarketMarkButton>,
        ),
    >,
    mut panels: Query<
        &mut Node,
        (
            With<MarketPanel>,
            Without<MarketOfferRow>,
            Without<MarketOfferButton>,
            Without<MarketMarkButton>,
        ),
    >,
    mut toggle_labels: Query<
        (&mut Text, &mut TextFont),
        (
            With<MarketToggleLabel>,
            Without<MarketOfferLabel>,
            Without<MarketMarkLabel>,
            Without<MarketWalletLabel>,
            Without<MarketRefreshLabel>,
        ),
    >,
) {
    let layout = windows
        .single()
        .map(|window| market_layout(window.width(), window.height()))
        .unwrap_or_else(|_| market_layout(1280.0, 720.0));

    if let Ok(mut panel) = panels.single_mut() {
        panel.display = if state.open {
            Display::Flex
        } else {
            Display::None
        };
    }
    if let Ok((mut label, mut font)) = toggle_labels.single_mut() {
        **label = market_toggle_label(layout.compact, session.elapsed, state.open, state.level);
        font.font_size = FontSize::Px(if layout.compact { 14.0 } else { 13.0 });
    }

    for (offer_label, mark_label, mut text, mut color, mut font) in &mut item_labels {
        font.font_size = FontSize::Px(if layout.compact { 12.0 } else { 11.0 });
        if let Some(label) = offer_label {
            let item = state.offers[label.0].item;
            let price = discounted_cost(item.price(), &effects, &fortune);
            let kind = if item.is_consumable() { "USE" } else { "GEAR" };
            **text = if layout.compact {
                format!(
                    "{}   {} K\n[{} / {}] {}",
                    item.name(),
                    price,
                    item.tag(),
                    kind,
                    item.effect_text()
                )
            } else {
                format!(
                    "[{} / {}] {}   {} KILLS\n{}",
                    item.tag(),
                    kind,
                    item.name(),
                    price,
                    item.effect_text()
                )
            };
            color.0 = item.color();
        } else if let Some(label) = mark_label {
            let item = state.offers[label.0].item;
            let marked = state.marked[label.0] == Some(item);
            **text = if marked {
                "AUTO\nON".to_string()
            } else {
                "AUTO".to_string()
            };
            color.0 = if marked {
                Color::srgb(0.30, 1.0, 0.68)
            } else {
                Color::srgb(0.68, 0.84, 0.92)
            };
        }
    }

    let next_stock = (state.next_stock_at - session.elapsed).max(0.0).ceil() as u32;
    for (mut text, mut font) in &mut wallet_labels {
        font.font_size = FontSize::Px(if layout.compact { 12.0 } else { 11.0 });
        **text = if state.notice_timer > 0.0 {
            if layout.compact {
                format!("LV{} | {}", state.level, state.notice)
            } else {
                format!("LV{}  {}", state.level, state.notice)
            }
        } else if layout.compact {
            format!(
                "LV{} | KILLS {} | STOCK {:02}:{:02}",
                state.level,
                session.kills,
                next_stock / 60,
                next_stock % 60
            )
        } else {
            format!(
                "LV{}  KILLS {:04}\nRESTOCK {:02}:{:02}",
                state.level,
                session.kills,
                next_stock / 60,
                next_stock % 60
            )
        };
    }

    for (mut text, mut font) in &mut refresh_labels {
        font.font_size = FontSize::Px(if layout.compact { 13.0 } else { 12.0 });
        let price = state.refresh_price(&effects, &fortune);
        **text = if layout.compact && price == 0 {
            "REFRESH | FREE".to_string()
        } else if layout.compact {
            format!("REFRESH | {price} K")
        } else if price == 0 {
            "REFRESH STOCK   FREE".to_string()
        } else {
            format!("REFRESH STOCK   {price} KILLS")
        };
    }

    for (row, mut node) in &mut rows {
        node.display = if row.0 < effects.visible_offer_count() {
            Display::Flex
        } else {
            Display::None
        };
    }

    for (offer_button, mark_button, mut background) in &mut action_buttons {
        if let Some(button) = offer_button {
            let item = state.offers[button.0].item;
            let affordable = session.kills >= discounted_cost(item.price(), &effects, &fortune);
            background.0 = if effects.owns(item) {
                Color::srgb(0.05, 0.045, 0.055)
            } else if affordable {
                Color::srgb(0.12, 0.105, 0.15)
            } else {
                Color::srgb(0.065, 0.055, 0.075)
            };
        } else if let Some(button) = mark_button {
            let item = state.offers[button.0].item;
            background.0 = if effects.owns(item) {
                Color::srgb(0.05, 0.045, 0.055)
            } else if state.marked[button.0] == Some(item) {
                Color::srgb(0.06, 0.36, 0.25)
            } else {
                Color::srgb(0.10, 0.16, 0.20)
            };
        }
    }
}

fn market_toggle_label(compact: bool, elapsed: f32, open: bool, level: u8) -> String {
    if elapsed < MARKET_UNLOCK_AT {
        let remaining = (MARKET_UNLOCK_AT - elapsed).ceil() as u32;
        if compact {
            format!("MARKET LOCKED {:02}:{:02}", remaining / 60, remaining % 60)
        } else {
            format!(
                "B  BLACK MARKET  [LOCKED {:02}:{:02}]",
                remaining / 60,
                remaining % 60
            )
        }
    } else if open {
        if compact {
            format!("CLOSE MARKET  LV{level}")
        } else {
            format!("B  CLOSE BLACK MARKET  LV{level}")
        }
    } else if compact {
        format!("MARKET  LV{level}")
    } else {
        format!("B  OPEN BLACK MARKET  LV{level}")
    }
}

fn close_market(state: &mut MarketState, session: &mut GameSession) {
    state.open = false;
    if state.paused_by_market {
        if !session.finished {
            session.paused = false;
        }
        state.paused_by_market = false;
    }
}

fn discounted_cost(price: u32, effects: &MarketEffects, fortune: &FortuneMarketRules) -> u32 {
    (price as f32 * (1.0 - effects.market_discount) * fortune.price_multiplier).ceil() as u32
}

fn set_notice(state: &mut MarketState, message: String) {
    state.notice = message;
    state.notice_timer = 2.2;
}

fn reroll_all(
    state: &mut MarketState,
    effects: &MarketEffects,
    fortune: &FortuneMarketRules,
    rng: &mut RunRng,
    level: u8,
) {
    state.marked = [None; MAX_OFFER_COUNT];
    let mut selected = [MarketItem::CorePatch; MAX_OFFER_COUNT];
    for index in 0..MAX_OFFER_COUNT {
        let offer = roll_slot(index, effects, fortune, rng, level, &selected[..index]);
        state.offers[index] = offer;
        selected[index] = offer.item;
    }
}

fn replace_offer(
    index: usize,
    state: &mut MarketState,
    effects: &MarketEffects,
    fortune: &FortuneMarketRules,
    rng: &mut RunRng,
    level: u8,
) {
    let selected = state.offers.map(|offer| offer.item);
    state.offers[index] = roll_slot(index, effects, fortune, rng, level, &selected);
    state.marked[index] = None;
}

fn roll_slot(
    index: usize,
    effects: &MarketEffects,
    fortune: &FortuneMarketRules,
    rng: &mut RunRng,
    level: u8,
    avoid: &[MarketItem],
) -> MarketOffer {
    let rarity = roll_rarity(level, fortune.rarity_bonus, rng);
    let item = match index {
        0 | 1 => roll_equipment(rarity, effects, rng, avoid),
        2 => roll_consumable(rarity, rng, avoid),
        _ if rng.next_u32().is_multiple_of(4) => roll_consumable(rarity, rng, avoid),
        _ => roll_equipment(rarity, effects, rng, avoid),
    };
    MarketOffer { item }
}

fn roll_consumable(rarity: Rarity, rng: &mut RunRng, avoid: &[MarketItem]) -> MarketItem {
    let preferred = CONSUMABLES[rarity.index()];
    if !avoid.contains(&preferred) {
        return preferred;
    }
    let start = rng.next_u32() as usize % CONSUMABLES.len();
    for offset in 0..CONSUMABLES.len() {
        let candidate = CONSUMABLES[(start + offset) % CONSUMABLES.len()];
        if !avoid.contains(&candidate) {
            return candidate;
        }
    }
    preferred
}

fn roll_equipment(
    rarity: Rarity,
    effects: &MarketEffects,
    rng: &mut RunRng,
    avoid: &[MarketItem],
) -> MarketItem {
    let pool: &[MarketItem] = match rarity {
        Rarity::White => &WHITE_EQUIPMENT,
        Rarity::Blue => &BLUE_EQUIPMENT,
        Rarity::Purple => &PURPLE_EQUIPMENT,
        Rarity::Orange => &ORANGE_EQUIPMENT,
        Rarity::Red => &RED_EQUIPMENT,
    };
    for _ in 0..pool.len() * 2 {
        let candidate = pool[rng.next_u32() as usize % pool.len()];
        if !effects.owns(candidate) && !avoid.contains(&candidate) {
            return candidate;
        }
    }
    if let Some(candidate) = ALL_EQUIPMENT
        .iter()
        .find(|candidate| !effects.owns(**candidate) && !avoid.contains(candidate))
    {
        *candidate
    } else {
        roll_consumable(rarity, rng, avoid)
    }
}

fn rarity_for_roll(level: u8, roll: u8) -> Rarity {
    let weights = RARITY_WEIGHTS[(level.clamp(1, MARKET_LEVEL_COUNT) - 1) as usize];
    let mut upper = 0_u8;
    for (rarity, weight) in Rarity::ALL.into_iter().zip(weights) {
        upper = upper.saturating_add(weight);
        if roll < upper {
            return rarity;
        }
    }
    Rarity::Red
}

fn roll_rarity(level: u8, rarity_bonus: f32, rng: &mut RunRng) -> Rarity {
    let roll = (rng.next_u32() % 100) as u8;
    rarity_for_fortune_roll(level, roll, rarity_bonus)
}

fn rarity_for_fortune_roll(level: u8, roll: u8, rarity_bonus: f32) -> Rarity {
    let shift = (rarity_bonus.clamp(0.0, 1.0) * 100.0).round() as u8;
    rarity_for_roll(level, roll.saturating_add(shift).min(99))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply_item_for_test(
        item: MarketItem,
    ) -> (GameSession, PlayerBuild, HeroVitals, MarketEffects) {
        let mut session = GameSession::default();
        let mut build = PlayerBuild::default();
        let mut vitals = HeroVitals::default();
        let mut effects = MarketEffects::default();
        item.apply(&mut session, &mut build, &mut vitals, &mut effects);
        (session, build, vitals, effects)
    }

    fn test_app(elapsed: f32) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<GameSession>()
            .init_resource::<PlayerBuild>()
            .init_resource::<HeroVitals>()
            .init_resource::<RunRng>()
            .add_plugins(MarketPlugin);
        app.world_mut().resource_mut::<GameSession>().elapsed = elapsed;
        app.update();
        app
    }

    fn tap_b(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyB);
        app.update();
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        input.release(KeyCode::KeyB);
        input.clear();
    }

    #[test]
    fn compact_market_grid_fits_supported_landscape_viewports() {
        for (width, height) in [
            (667.0, 375.0),
            (740.0, 360.0),
            (844.0, 390.0),
            (915.0, 412.0),
        ] {
            let layout = market_layout(width, height);
            let available_height = height - layout.panel_top - layout.safe_y;
            let panel_width = width - layout.safe_x * 2.0;
            let inner_width = panel_width - MARKET_COMPACT_PANEL_PADDING * 2.0;
            let row_width = inner_width * layout.row_width_percent / 100.0;
            let purchase_width = row_width - MARKET_PANEL_GAP - layout.mark_width;

            assert!(layout.compact, "{width}x{height} should use compact layout");
            assert_eq!(MAX_OFFER_COUNT.div_ceil(2), 3);
            assert!(layout.minimum_compact_panel_height() <= available_height + f32::EPSILON);
            assert!(row_width * 2.0 + MARKET_PANEL_GAP <= inner_width + f32::EPSILON);
            assert!(layout.row_height >= 48.0);
            assert!(purchase_width >= 48.0);
            assert!(layout.mark_width >= 48.0);
            assert!(layout.toggle_height >= 48.0);
            // The pause button occupies right=safe_x+104 with a 52px target.
            assert!(layout.toggle_right >= layout.safe_x + 104.0 + 52.0 + 8.0);
        }
    }

    #[test]
    fn desktop_market_stays_single_column_and_mobile_labels_drop_keyboard_hints() {
        let desktop = market_layout(1280.0, 720.0);
        assert!(!desktop.compact);
        assert_eq!(desktop.row_width_percent, 100.0);
        assert_eq!(desktop.row_height, 52.0);
        assert_eq!(desktop.toggle_height, 48.0);

        let mobile_locked = market_toggle_label(true, 0.0, false, 1);
        let mobile_open = market_toggle_label(true, MARKET_UNLOCK_AT, true, 1);
        assert!(!mobile_locked.starts_with("B  "));
        assert!(!mobile_open.starts_with("B  "));
        assert!(market_toggle_label(false, 0.0, false, 1).starts_with("B  "));
    }

    #[test]
    fn initializes_ui_and_market_owns_its_pause() {
        let mut app = test_app(MARKET_UNLOCK_AT - 1.0);
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<MarketToggleButton>>()
                .iter(app.world())
                .count(),
            1
        );
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<MarketOfferButton>>()
                .iter(app.world())
                .count(),
            MAX_OFFER_COUNT
        );
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<MarketMarkButton>>()
                .iter(app.world())
                .count(),
            MAX_OFFER_COUNT
        );
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<MarketOfferGrid>>()
                .iter(app.world())
                .count(),
            1
        );

        tap_b(&mut app);
        assert!(!app.world().resource::<MarketState>().open);
        assert!(!app.world().resource::<GameSession>().paused);

        app.world_mut().resource_mut::<GameSession>().elapsed = MARKET_UNLOCK_AT;
        tap_b(&mut app);
        assert!(app.world().resource::<MarketState>().open);
        assert!(app.world().resource::<MarketState>().paused_by_market);
        assert!(app.world().resource::<GameSession>().paused);

        tap_b(&mut app);
        assert!(!app.world().resource::<MarketState>().open);
        assert!(!app.world().resource::<MarketState>().paused_by_market);
        assert!(!app.world().resource::<GameSession>().paused);
    }

    #[test]
    fn stock_is_deterministic_and_refresh_cost_caps() {
        let effects = MarketEffects::default();
        let fortune = FortuneMarketRules::default();
        let mut first_state = MarketState::default();
        let mut second_state = MarketState::default();
        let mut first_rng = RunRng::default();
        let mut second_rng = RunRng::default();
        assert_eq!(first_state.refresh_price(&effects, &fortune), 60);
        reroll_all(&mut first_state, &effects, &fortune, &mut first_rng, 5);
        reroll_all(&mut second_state, &effects, &fortune, &mut second_rng, 5);
        assert_eq!(first_state.offers, second_state.offers);
        assert!(!first_state.offers[0].item.is_consumable());
        assert!(!first_state.offers[1].item.is_consumable());
        assert!(first_state.offers[2].item.is_consumable());
        for left in 0..BASE_OFFER_COUNT {
            for right in left + 1..BASE_OFFER_COUNT {
                assert_ne!(first_state.offers[left], first_state.offers[right]);
            }
        }

        first_state.marked[0] = Some(first_state.offers[0].item);
        reroll_all(&mut first_state, &effects, &fortune, &mut first_rng, 5);
        assert!(first_state.marked.iter().all(Option::is_none));
        first_state.manual_refreshes = 99;
        assert_eq!(first_state.refresh_price(&effects, &fortune), 120);
    }

    #[test]
    fn fortune_refresh_is_free_once_per_stock_generation_and_stays_deterministic() {
        let effects = MarketEffects::default();
        let fortune = FortuneMarketRules {
            price_multiplier: 0.90,
            rarity_bonus: 0.15,
            free_refresh_available: true,
        };
        let mut state = MarketState::default();
        assert_eq!(state.refresh_price(&effects, &fortune), 0);
        state.free_refresh_used_generation = Some(state.stock_generation);
        assert_eq!(state.refresh_price(&effects, &fortune), 54);
        state.stock_generation += 1;
        assert_eq!(state.refresh_price(&effects, &fortune), 0);

        assert_eq!(rarity_for_roll(1, 70), Rarity::White);
        assert_eq!(rarity_for_fortune_roll(1, 70, 0.15), Rarity::Blue);
        let mut first = MarketState::default();
        let mut second = MarketState::default();
        reroll_all(&mut first, &effects, &fortune, &mut RunRng::default(), 3);
        reroll_all(&mut second, &effects, &fortune, &mut RunRng::default(), 3);
        assert_eq!(first.offers, second.offers);
    }

    #[test]
    fn all_five_market_levels_match_public_rarity_probabilities() {
        for (index, expected) in RARITY_WEIGHTS.into_iter().enumerate() {
            let level = index as u8 + 1;
            let mut actual = [0_u8; 5];
            for roll in 0..100 {
                let rarity = rarity_for_roll(level, roll);
                actual[rarity.index()] += 1;
            }
            assert_eq!(actual, expected, "market level {level}");
        }
    }

    #[test]
    fn market_level_tracks_checkpoint_bosses_and_caps_at_five() {
        assert_eq!(market_level(0), 1);
        assert_eq!(market_level(1), 2);
        assert_eq!(market_level(2), 3);
        assert_eq!(market_level(3), 4);
        assert_eq!(market_level(4), 5);
        assert_eq!(market_level(u8::MAX), 5);
    }

    #[test]
    fn automatic_restock_uses_exact_180_second_boundaries() {
        let mut app = test_app(MARKET_UNLOCK_AT);
        assert_eq!(app.world().resource::<MarketState>().stock_generation, 0);
        assert_eq!(
            app.world().resource::<MarketState>().next_stock_at,
            MARKET_UNLOCK_AT + STOCK_INTERVAL
        );

        let first_item = app.world().resource::<MarketState>().offers[0].item;
        app.world_mut().resource_mut::<MarketState>().marked[0] = Some(first_item);
        app.world_mut().resource_mut::<GameSession>().elapsed =
            MARKET_UNLOCK_AT + STOCK_INTERVAL - 0.01;
        app.update();
        assert_eq!(app.world().resource::<MarketState>().stock_generation, 0);
        assert!(app.world().resource::<MarketState>().marked[0].is_some());

        app.world_mut().resource_mut::<GameSession>().elapsed = MARKET_UNLOCK_AT + STOCK_INTERVAL;
        app.update();
        assert_eq!(app.world().resource::<MarketState>().stock_generation, 1);
        assert_eq!(
            app.world().resource::<MarketState>().next_stock_at,
            MARKET_UNLOCK_AT + STOCK_INTERVAL * 2.0
        );
        assert!(
            app.world()
                .resource::<MarketState>()
                .marked
                .iter()
                .all(Option::is_none)
        );

        app.world_mut().resource_mut::<GameSession>().elapsed =
            MARKET_UNLOCK_AT + STOCK_INTERVAL * 4.0;
        app.update();
        assert_eq!(app.world().resource::<MarketState>().stock_generation, 4);
        assert_eq!(
            app.world().resource::<MarketState>().next_stock_at,
            MARKET_UNLOCK_AT + STOCK_INTERVAL * 5.0
        );
    }

    #[test]
    fn marked_offer_auto_buys_only_after_balance_is_sufficient() {
        let mut app = test_app(MARKET_UNLOCK_AT);
        {
            let mut session = app.world_mut().resource_mut::<GameSession>();
            session.kills = MarketItem::CorePatch.price() - 1;
            session.core_hp = 100.0;
        }
        {
            let mut state = app.world_mut().resource_mut::<MarketState>();
            state.offers[0] = MarketOffer {
                item: MarketItem::CorePatch,
            };
            state.marked[0] = Some(MarketItem::CorePatch);
        }

        app.update();
        assert_eq!(app.world().resource::<GameSession>().core_hp, 100.0);
        assert_eq!(
            app.world().resource::<MarketState>().marked[0],
            Some(MarketItem::CorePatch)
        );

        app.world_mut().resource_mut::<GameSession>().kills += 1;
        app.update();
        assert_eq!(app.world().resource::<GameSession>().kills, 0);
        assert_eq!(app.world().resource::<GameSession>().core_hp, 300.0);
        assert_eq!(app.world().resource::<MarketState>().marked[0], None);
        assert_ne!(
            app.world().resource::<MarketState>().offers[0].item,
            MarketItem::CorePatch
        );
    }

    fn install_autoplay_priority_stock(app: &mut App, kills: u32) {
        {
            let mut session = app.world_mut().resource_mut::<GameSession>();
            session.kills = kills;
            session.core_hp = session.core_max_hp;
        }
        app.world_mut().resource_mut::<MarketState>().offers = [
            MarketOffer {
                item: MarketItem::CorePatch,
            },
            MarketOffer {
                item: MarketItem::GuardianSeed,
            },
            MarketOffer {
                item: MarketItem::SerratedDisc,
            },
            MarketOffer {
                item: MarketItem::ArcLens,
            },
            MarketOffer {
                item: MarketItem::WorldAnchor,
            },
        ];
    }

    #[test]
    fn autoplay_buys_two_priorities_once_per_generation_and_keeps_a_reserve() {
        let mut app = test_app(MARKET_UNLOCK_AT);
        install_autoplay_priority_stock(&mut app, 900);
        app.insert_resource(Autoplay);

        app.update();

        let effects = app.world().resource::<MarketEffects>();
        assert!(effects.owns(MarketItem::GuardianSeed));
        assert!(effects.owns(MarketItem::WorldAnchor));
        assert!(!effects.owns(MarketItem::SerratedDisc));
        assert_eq!(
            app.world().resource::<GameSession>().kills,
            AUTOPLAY_KILL_RESERVE
        );
        assert_eq!(
            app.world()
                .resource::<MarketState>()
                .autoplay_purchases_generation,
            AUTOPLAY_MAX_PURCHASES_PER_GENERATION as u8
        );

        let owned_after_plan = effects.owned_equipment;
        app.world_mut().resource_mut::<GameSession>().kills += 1_000;
        app.update();
        assert_eq!(
            app.world().resource::<MarketEffects>().owned_equipment,
            owned_after_plan
        );
        assert_eq!(
            app.world()
                .resource::<MarketState>()
                .autoplay_purchases_generation,
            AUTOPLAY_MAX_PURCHASES_PER_GENERATION as u8
        );
        assert_eq!(app.world().resource::<GameSession>().kills, 1_100);
    }

    #[test]
    fn market_never_buys_without_autoplay_or_a_player_mark() {
        let mut app = test_app(MARKET_UNLOCK_AT);
        install_autoplay_priority_stock(&mut app, 900);
        let rng_before = app.world().resource::<RunRng>().state();

        app.update();

        assert_eq!(app.world().resource::<GameSession>().kills, 900);
        assert_eq!(app.world().resource::<MarketEffects>().owned_equipment, 0);
        assert_eq!(app.world().resource::<RunRng>().state(), rng_before);
        assert_eq!(
            app.world()
                .resource::<MarketState>()
                .autoplay_planned_generation,
            None
        );
    }

    #[test]
    fn autoplay_waits_until_price_plus_reserve_is_available() {
        let mut app = test_app(MARKET_UNLOCK_AT);
        install_autoplay_priority_stock(
            &mut app,
            MarketItem::GuardianSeed.price() + AUTOPLAY_KILL_RESERVE - 1,
        );
        app.insert_resource(Autoplay);

        app.update();
        assert_eq!(
            app.world().resource::<GameSession>().kills,
            MarketItem::GuardianSeed.price() + AUTOPLAY_KILL_RESERVE - 1
        );
        assert!(
            !app.world()
                .resource::<MarketEffects>()
                .owns(MarketItem::GuardianSeed)
        );

        app.world_mut().resource_mut::<GameSession>().kills += 1;
        app.update();
        assert_eq!(
            app.world().resource::<GameSession>().kills,
            AUTOPLAY_KILL_RESERVE
        );
        assert!(
            app.world()
                .resource::<MarketEffects>()
                .owns(MarketItem::GuardianSeed)
        );
    }

    #[test]
    fn autoplay_market_results_are_deterministic_and_stay_five_rows() {
        fn run_once() -> (u32, u32, [MarketOffer; MAX_OFFER_COUNT], u64, usize) {
            let mut app = test_app(MARKET_UNLOCK_AT);
            install_autoplay_priority_stock(&mut app, 900);
            app.insert_resource(Autoplay);
            app.update();
            (
                app.world().resource::<GameSession>().kills,
                app.world().resource::<MarketEffects>().owned_equipment,
                app.world().resource::<MarketState>().offers,
                app.world().resource::<RunRng>().state(),
                app.world_mut()
                    .query_filtered::<Entity, With<MarketOfferRow>>()
                    .iter(app.world())
                    .count(),
            )
        }

        let first = run_once();
        let second = run_once();
        assert_eq!(first, second);
        assert_eq!(first.4, MAX_OFFER_COUNT);
    }

    #[test]
    fn owned_equipment_is_replaced_without_being_bought_again() {
        let mut app = test_app(MARKET_UNLOCK_AT);
        let item = MarketItem::SerratedDisc;
        let equipment_bit = 1 << item.equipment_index().expect("equipment index");
        app.world_mut()
            .resource_mut::<MarketEffects>()
            .owned_equipment |= equipment_bit;
        {
            let mut state = app.world_mut().resource_mut::<MarketState>();
            state.offers[0] = MarketOffer { item };
            state.marked[0] = Some(item);
        }
        app.world_mut().resource_mut::<GameSession>().kills = 1_000;

        app.update();
        assert_eq!(app.world().resource::<GameSession>().kills, 1_000);
        assert_eq!(app.world().resource::<MarketState>().marked[0], None);
        assert_ne!(app.world().resource::<MarketState>().offers[0].item, item);
    }

    #[test]
    fn mark_button_toggles_a_specific_offer() {
        let mut app = test_app(MARKET_UNLOCK_AT);
        app.world_mut().resource_mut::<GameSession>().kills = 0;
        app.world_mut().resource_mut::<MarketState>().offers[0] = MarketOffer {
            item: MarketItem::CorePatch,
        };
        tap_b(&mut app);
        let mark_button = {
            let world = app.world_mut();
            let mut buttons = world.query::<(Entity, &MarketMarkButton)>();
            buttons
                .iter(world)
                .find_map(|(entity, button)| (button.0 == 0).then_some(entity))
                .expect("first market mark button")
        };

        app.world_mut()
            .entity_mut(mark_button)
            .insert(Interaction::Pressed);
        app.update();
        assert_eq!(
            app.world().resource::<MarketState>().marked[0],
            Some(MarketItem::CorePatch)
        );

        app.world_mut()
            .entity_mut(mark_button)
            .insert(Interaction::None);
        app.update();
        app.world_mut()
            .entity_mut(mark_button)
            .insert(Interaction::Pressed);
        app.update();
        assert_eq!(app.world().resource::<MarketState>().marked[0], None);
    }

    #[test]
    fn runtime_purchase_only_spends_kills() {
        let mut app = test_app(MARKET_UNLOCK_AT);
        {
            let mut session = app.world_mut().resource_mut::<GameSession>();
            session.kills = 1_000;
            session.core_hp = 100.0;
        }
        app.world_mut().resource_mut::<MarketState>().offers[0] = MarketOffer {
            item: MarketItem::CorePatch,
        };
        let gold = app.world().resource::<GameSession>().gold;
        let wood = app.world().resource::<GameSession>().wood;

        tap_b(&mut app);
        let button = {
            let world = app.world_mut();
            let mut buttons = world.query::<(Entity, &MarketOfferButton)>();
            buttons
                .iter(world)
                .find_map(|(entity, button)| (button.0 == 0).then_some(entity))
                .expect("first market offer button")
        };
        app.world_mut()
            .entity_mut(button)
            .insert(Interaction::Pressed);
        app.update();

        let session = app.world().resource::<GameSession>();
        assert_eq!(session.kills, 1_000 - MarketItem::CorePatch.price());
        assert_eq!(session.gold, gold);
        assert_eq!(session.wood, wood);
        assert_eq!(session.core_hp, 300.0);
    }

    #[test]
    fn hardened_plate_buffs_hero_without_changing_core_or_drifting() {
        let item = MarketItem::HardenedPlate;
        let mut state = MarketState::default();
        state.offers[0] = MarketOffer { item };
        let mut effects = MarketEffects::default();
        let mut session = GameSession {
            kills: 1_000,
            ..default()
        };
        let original_core = (session.core_hp, session.core_max_hp);
        let mut build = PlayerBuild::default();
        let mut vitals = HeroVitals::default();
        let fortune = FortuneMarketRules::default();
        let original_max = vitals.max_health;
        vitals.health = original_max * 0.40;
        let mut rng = RunRng::default();

        assert_eq!(
            purchase_offer(
                0,
                &mut state,
                &mut effects,
                &mut session,
                &mut build,
                &mut vitals,
                &fortune,
                &mut rng,
            ),
            PurchaseOutcome::Bought(item)
        );
        assert_eq!((session.core_hp, session.core_max_hp), original_core);
        assert!((vitals.max_health - original_max * 1.20).abs() < 0.001);
        assert!((vitals.health / vitals.max_health - 0.40).abs() < 0.001);
        assert!((vitals.command_health_multiplier - 1.20).abs() < 0.001);
        assert!((vitals.armor - 6.0).abs() < 0.001);

        let hero_snapshot = (vitals.health, vitals.max_health, vitals.armor);
        state.offers[0] = MarketOffer { item };
        assert_eq!(
            purchase_offer(
                0,
                &mut state,
                &mut effects,
                &mut session,
                &mut build,
                &mut vitals,
                &fortune,
                &mut rng,
            ),
            PurchaseOutcome::Owned
        );
        assert_eq!(
            hero_snapshot,
            (vitals.health, vitals.max_health, vitals.armor)
        );
    }

    #[test]
    fn speed_effects_are_read_only_and_expire_without_drift() {
        let mut effects = MarketEffects::default();
        let mut session = GameSession::default();
        let mut build = PlayerBuild::default();
        let mut vitals = HeroVitals::default();
        MarketItem::SlowHourglass.apply(&mut session, &mut build, &mut vitals, &mut effects);
        MarketItem::WorldAnchor.apply(&mut session, &mut build, &mut vitals, &mut effects);

        let base_speed = 3.0;
        for _ in 0..10 {
            assert!((base_speed * effects.enemy_movement_multiplier(false) - 1.5).abs() < 0.001);
            assert!((base_speed * effects.enemy_movement_multiplier(true) - 1.275).abs() < 0.001);
        }
        assert_eq!(base_speed, 3.0);

        effects.advance(15.0);
        assert_eq!(effects.enemy_movement_multiplier(false), 1.0);
        assert_eq!(effects.enemy_movement_multiplier(true), 0.85);
    }

    #[test]
    fn serrated_disc_sets_thirty_percent_attack_splash() {
        let (_, build, _, effects) = apply_item_for_test(MarketItem::SerratedDisc);
        assert!((build.attack_damage - PlayerBuild::default().attack_damage * 1.22).abs() < 0.001);
        assert!((effects.splash_ratio() - 0.30).abs() < f32::EPSILON);
    }

    #[test]
    fn arc_lens_scales_spell_power_and_spell_area_independently() {
        let (_, _, _, effects) = apply_item_for_test(MarketItem::ArcLens);
        assert!((effects.effective_spell_power(100.0) - 110.0).abs() < 0.001);
        assert!((effects.spell_area_multiplier() - 1.18).abs() < 0.001);
    }

    #[test]
    fn spirit_drum_advances_summon_attack_timers_twenty_five_percent_faster() {
        let (_, _, _, effects) = apply_item_for_test(MarketItem::SpiritDrum);
        assert!((effects.summon_attack_speed_multiplier() - 1.25).abs() < 0.001);
    }

    #[test]
    fn echo_core_stores_twenty_percent_chance_for_forty_five_percent_replays() {
        let (_, _, _, effects) = apply_item_for_test(MarketItem::EchoCore);
        assert!((effects.spell_echo_chance() - 0.20).abs() < f32::EPSILON);
    }

    #[test]
    fn ember_heart_enables_burn_critical_strikes() {
        let (_, _, _, effects) = apply_item_for_test(MarketItem::EmberHeart);
        assert!(effects.burns_can_crit());
    }

    #[test]
    fn storm_coil_triggers_exactly_every_fifth_basic_attack() {
        let (_, _, _, mut effects) = apply_item_for_test(MarketItem::StormCoil);
        for _ in 0..4 {
            assert!(!effects.record_basic_attack());
        }
        assert!(effects.record_basic_attack());
        for _ in 0..4 {
            assert!(!effects.record_basic_attack());
        }
        assert!(effects.record_basic_attack());
    }

    #[test]
    fn frost_mirror_arms_only_the_frozen_death_burst() {
        let (_, _, _, effects) = apply_item_for_test(MarketItem::FrostMirror);
        assert!(effects.frost_mirror_enabled());
        assert!(!MarketEffects::default().frost_mirror_enabled());
    }

    #[test]
    fn threefold_relic_sets_one_twelve_for_attack_spell_and_summon_paths() {
        let (_, _, _, effects) = apply_item_for_test(MarketItem::ThreefoldRelic);
        assert!((effects.threefold_multiplier() - 1.12).abs() < 0.001);
    }

    #[test]
    fn blood_contract_uses_half_health_boundary_and_five_percent_leech() {
        let (_, _, mut vitals, effects) = apply_item_for_test(MarketItem::BloodContract);
        vitals.health = vitals.max_health * 0.50;
        assert!((effects.low_health_damage_multiplier(&vitals) - 1.40).abs() < 0.001);
        vitals.health = vitals.max_health * 0.501;
        assert_eq!(effects.low_health_damage_multiplier(&vitals), 1.0);
        assert!((effects.life_leech() - 0.05).abs() < f32::EPSILON);
    }

    #[test]
    fn ascendant_crown_uses_stable_highest_skill_and_thirty_percent_power() {
        let mut session = GameSession::default();
        let mut build = PlayerBuild::default();
        build.skills[2] = 5;
        build.skills[3] = 5;
        let mut vitals = HeroVitals::default();
        let mut effects = MarketEffects::default();
        MarketItem::AscendantCrown.apply(&mut session, &mut build, &mut vitals, &mut effects);
        assert_eq!((build.skills[2], build.skills[3]), (6, 5));
        assert!((effects.top_skill_multiplier(Some(2), &build) - 1.30).abs() < 0.001);
        assert_eq!(effects.top_skill_multiplier(Some(3), &build), 1.0);
        assert_eq!(
            effects.top_skill_multiplier(None, &PlayerBuild::default()),
            1.0
        );
    }

    #[test]
    fn chaos_die_has_three_equal_outcomes_and_fortune_takes_the_better_roll() {
        assert_eq!(chaos_multiplier_for_roll(0), 0.7);
        assert_eq!(chaos_multiplier_for_roll(1), 1.0);
        assert_eq!(chaos_multiplier_for_roll(2), 1.8);
        assert_eq!(chaos_multiplier_for_roll(3), 0.7);

        let (_, _, _, effects) = apply_item_for_test(MarketItem::ChaosDie);
        let mut expected_rng = RunRng::from_seed(0xCA05);
        let first = roll_chaos_multiplier(&mut expected_rng);
        let second = roll_chaos_multiplier(&mut expected_rng);
        let expected_state = expected_rng.state();
        let mut actual_rng = RunRng::from_seed(0xCA05);
        assert_eq!(
            effects.chaos_damage_multiplier(&mut actual_rng, true),
            first.max(second)
        );
        assert_eq!(actual_rng.state(), expected_state);
    }

    #[test]
    fn world_anchor_slows_only_bosses_and_guards_control_for_one_second() {
        let (_, _, mut vitals, mut effects) = apply_item_for_test(MarketItem::WorldAnchor);
        assert_eq!(effects.enemy_movement_multiplier(false), 1.0);
        assert_eq!(effects.enemy_movement_multiplier(true), 0.85);
        effects.begin_dodge_control_guard();
        assert!(!effects.try_apply_hero_stun(&mut vitals, 1.0));
        effects.advance(0.999);
        assert!(!effects.try_apply_hero_stun(&mut vitals, 1.0));
        effects.advance(0.001);
        assert!(effects.try_apply_hero_stun(&mut vitals, 1.0));
        assert_eq!(vitals.stunned_remaining, 1.0);
    }

    #[test]
    fn legion_standard_adds_one_slot_and_thirty_percent_summon_inheritance() {
        let (_, _, _, effects) = apply_item_for_test(MarketItem::LegionStandard);
        assert_eq!(effects.summon_cap_bonus(), 1);
        assert!((effects.effective_summon_power(100.0) - 130.0).abs() < 0.001);
    }

    #[test]
    fn prismatic_engine_sets_one_eight_damage_and_one_four_area() {
        let (_, _, _, effects) = apply_item_for_test(MarketItem::PrismaticEngine);
        assert!((effects.element_reaction_multiplier() - 1.80).abs() < 0.001);
        assert!((effects.element_reaction_area_multiplier() - 1.40).abs() < 0.001);
    }

    #[test]
    fn guardian_seed_intercepts_once_per_thirty_seconds() {
        let mut effects = MarketEffects::default();
        effects.activate_guardian_seed();

        effects.advance(GUARDIAN_INTERCEPT_COOLDOWN_SECONDS - 0.01);
        assert!(!effects.try_intercept_ordinary_breach());
        effects.advance(0.01);
        assert!(effects.try_intercept_ordinary_breach());
        assert!(!effects.try_intercept_ordinary_breach());
        effects.advance(GUARDIAN_INTERCEPT_COOLDOWN_SECONDS);
        assert!(effects.try_intercept_ordinary_breach());
    }

    #[test]
    fn bond_compass_is_consumed_exactly_once() {
        let mut effects = MarketEffects::default();
        let mut session = GameSession::default();
        let mut build = PlayerBuild::default();
        let mut vitals = HeroVitals::default();
        MarketItem::BondCompass.apply(&mut session, &mut build, &mut vitals, &mut effects);

        assert!(effects.bond_compass_available());
        assert!(effects.consume_bond_compass());
        assert!(!effects.bond_compass_available());
        assert!(!effects.consume_bond_compass());
    }

    #[test]
    fn last_oath_restores_hero_and_stores_eight_second_empowerment() {
        let mut effects = MarketEffects::default();
        let mut session = GameSession::default();
        let mut build = PlayerBuild::default();
        let mut vitals = HeroVitals::default();
        MarketItem::LastOath.apply(&mut session, &mut build, &mut vitals, &mut effects);
        vitals.health = 0.0;

        assert!(effects.intercept_hero_fatal(&mut vitals));
        assert!((vitals.health / vitals.max_health - FATAL_RECOVERY_RATIO).abs() < 0.001);
        assert!(vitals.invulnerable_remaining >= FATAL_INVULNERABILITY_SECONDS);
        assert_eq!(
            effects.last_oath_empowerment_remaining(),
            LAST_OATH_EMPOWERMENT_SECONDS
        );
        assert_eq!(effects.last_oath_damage_multiplier(), 1.50);
        effects.advance(LAST_OATH_EMPOWERMENT_SECONDS - 0.001);
        assert_eq!(effects.last_oath_damage_multiplier(), 1.50);
        effects.advance(0.001);
        assert_eq!(effects.last_oath_damage_multiplier(), 1.0);
        assert!(!effects.intercept_hero_fatal(&mut vitals));
    }
}
