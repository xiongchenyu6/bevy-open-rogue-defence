use bevy::{prelude::*, window::PrimaryWindow};

use super::combat::SurvivalSet;
use super::content::Rarity as BuildRarity;
use super::mobile_layout::hud_metrics;
use super::rarity::rarity_color;
use super::{Autoplay, GameLanguage, GameSession, HeroVitals, PlayerBuild, RunRng};

pub struct MarketPlugin;

impl Plugin for MarketPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameLanguage>()
            .init_resource::<MarketState>()
            .init_resource::<MarketEffects>()
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
                    update_market_tooltip,
                    refresh_market_language,
                )
                    .chain()
                    .before(SurvivalSet)
                    .before(super::bonds::BondSet),
            );
    }
}

const MARKET_UNLOCK_AT: f32 = 180.0;
const STOCK_INTERVAL: f32 = 180.0;
// The current public match footage shows five stock rows before any equipment
// is purchased. All five are always visible; MarketBadge is valuable for its
// price reduction instead of fabricating a sixth row outside the observed UI.
const OFFER_COUNT: usize = 5;
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
const MARKET_COMPACT_TOGGLE_WIDTH: f32 = 96.0;
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
enum MarketQuality {
    White,
    Blue,
    Purple,
    Orange,
    Red,
}

impl MarketQuality {
    const ALL: [Self; 5] = [
        Self::White,
        Self::Blue,
        Self::Purple,
        Self::Orange,
        Self::Red,
    ];

    const fn index(self) -> usize {
        match self {
            Self::White => 0,
            Self::Blue => 1,
            Self::Purple => 2,
            Self::Orange => 3,
            Self::Red => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum MarketItem {
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
    pub(crate) fn icon_path(self) -> &'static str {
        match self {
            Self::SerratedDisc => "ui/icons/orbit_blade.png",
            Self::ArcLens => "ui/icons/arcane_loop.png",
            Self::HardenedPlate => "ui/icons/shield.png",
            Self::SpiritDrum => "ui/icons/spirit_pact.png",
            Self::WindBoots => "ui/icons/windrunner.png",
            Self::EchoCore => "ui/icons/prism_echo.png",
            Self::EmberHeart => "ui/icons/cinder_aura.png",
            Self::StormCoil => "ui/icons/storm_conduit.png",
            Self::FrostMirror => "ui/icons/glacier_core.png",
            Self::ThreefoldRelic => "ui/icons/triune_engine.png",
            Self::BloodContract => "ui/icons/blood_array.png",
            Self::MarketBadge => "ui/icons/raider_fortune.png",
            Self::GuardianSeed => "ui/icons/guardian_oath.png",
            Self::AscendantCrown => "ui/icons/qi_ascension.png",
            Self::ChaosDie => "ui/icons/fate_cube.png",
            Self::WorldAnchor => "ui/icons/mountain.png",
            Self::LegionStandard => "ui/icons/oath_banner.png",
            Self::PrismaticEngine => "ui/icons/element_master.png",
            Self::LastOath => "ui/icons/ember_oath.png",
            _ => "ui/icons/market.png",
        }
    }

    pub(crate) fn name(self, language: GameLanguage) -> &'static str {
        let (english, chinese) = match self {
            Self::CorePatch => ("Gate Patch", "城门修补包"),
            Self::SkillTuner => ("Skill Tuner", "技能调谐器"),
            Self::BondCompass => ("Bond Compass", "羁绊罗盘"),
            Self::SlowHourglass => ("Slow Hourglass", "缓时沙漏"),
            Self::RevivalSeed => ("Revival Seed", "复生种子"),
            Self::SerratedDisc => ("Serrated Disc", "锯齿圆盘"),
            Self::ArcLens => ("Arc Lens", "奥术透镜"),
            Self::HardenedPlate => ("Hardened Plate", "淬硬甲片"),
            Self::SpiritDrum => ("Spirit Drum", "灵鼓"),
            Self::WindBoots => ("Wind Boots", "疾风靴"),
            Self::EchoCore => ("Echo Core", "回响核心"),
            Self::EmberHeart => ("Ember Heart", "余烬之心"),
            Self::StormCoil => ("Storm Coil", "雷暴线圈"),
            Self::FrostMirror => ("Frost Mirror", "霜镜"),
            Self::ThreefoldRelic => ("Threefold Relic", "三元遗物"),
            Self::BloodContract => ("Blood Contract", "血契"),
            Self::MarketBadge => ("Market Badge", "黑市徽章"),
            Self::GuardianSeed => ("Guardian Seed", "守护种子"),
            Self::AscendantCrown => ("Ascendant Crown", "登临之冠"),
            Self::ChaosDie => ("Chaos Die", "混沌骰"),
            Self::WorldAnchor => ("World Anchor", "世界之锚"),
            Self::LegionStandard => ("Legion Standard", "军团战旗"),
            Self::PrismaticEngine => ("Prismatic Engine", "棱镜引擎"),
            Self::LastOath => ("Last Oath", "最后誓约"),
        };
        language.pick(english, chinese)
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

    fn market_quality(self) -> MarketQuality {
        if self == Self::CorePatch || WHITE_EQUIPMENT.contains(&self) {
            MarketQuality::White
        } else if self == Self::SkillTuner || BLUE_EQUIPMENT.contains(&self) {
            MarketQuality::Blue
        } else if self == Self::BondCompass || PURPLE_EQUIPMENT.contains(&self) {
            MarketQuality::Purple
        } else if self == Self::SlowHourglass || ORANGE_EQUIPMENT.contains(&self) {
            MarketQuality::Orange
        } else {
            MarketQuality::Red
        }
    }

    fn equipment_index(self) -> Option<usize> {
        ALL_EQUIPMENT.iter().position(|item| *item == self)
    }

    fn is_consumable(self) -> bool {
        self.equipment_index().is_none()
    }

    fn tag(self) -> &'static str {
        self.build_rarity().label()
    }

    fn color(self) -> Color {
        rarity_color(self.build_rarity())
    }

    /// Keep the observed five internal stock bands while presenting the same
    /// four build tiers used by heroes, skills and bonds.
    pub(crate) fn build_rarity(self) -> BuildRarity {
        match self.market_quality() {
            MarketQuality::White => BuildRarity::R,
            MarketQuality::Blue => BuildRarity::Sr,
            MarketQuality::Purple | MarketQuality::Orange => BuildRarity::Ssr,
            MarketQuality::Red => BuildRarity::Ur,
        }
    }

    pub(crate) fn effect_text(self, language: GameLanguage) -> &'static str {
        let (english, chinese) = match self {
            Self::CorePatch => ("restore 200 gate integrity", "城门耐久+200"),
            Self::SkillTuner => ("+1 skill refresh charge", "技能刷新次数+1"),
            Self::BondCompass => (
                "next bond draft favors strongest bond",
                "下次羁绊偏向最高进度",
            ),
            Self::SlowHourglass => ("all enemies -50% speed for 15s", "敌人减速50%，持续15秒"),
            Self::RevivalSeed => ("prevent the next fatal defeat", "抵消下一次致命失败"),
            Self::SerratedDisc => (
                "+22% attack, +3 armor penetration; attacks splash",
                "攻击+22%，物理穿透+3点；普攻溅射",
            ),
            Self::ArcLens => (
                "+18% spell range, +10% spell power, +10% spell penetration",
                "法术范围+18%，威力+10%，法术穿透+10%",
            ),
            Self::HardenedPlate => ("+6 armor, +20% maximum health", "护甲+6，最大生命+20%"),
            Self::SpiritDrum => ("+25% summon attack speed", "召唤物攻速+25%"),
            Self::WindBoots => (
                "+15% move speed, -15% dodge cooldown, +8% evasion",
                "移速+15%，闪避冷却-15%，闪避率+8%",
            ),
            Self::EchoCore => (
                "20% chance to echo skills at 45%",
                "技能20%概率以45%效果回响",
            ),
            Self::EmberHeart => (
                "+8% critical chance; burns can critically strike",
                "暴击率+8%；燃烧可以暴击",
            ),
            Self::StormCoil => ("every 5th attack chains lightning", "每5次攻击触发连锁闪电"),
            Self::FrostMirror => (
                "frozen deaths create an ice burst",
                "冻结敌人死亡时触发冰爆",
            ),
            Self::ThreefoldRelic => (
                "attack, spell and summon empower each other",
                "攻击、法术、召唤互相强化",
            ),
            Self::BloodContract => (
                "+40% low-health damage, +5% leech",
                "低生命伤害+40%，吸血+5%",
            ),
            Self::MarketBadge => ("-20% market prices", "黑市价格-20%"),
            Self::GuardianSeed => ("+1 gate/sec; intercept every 30s", "城门每秒+1；每30秒拦截"),
            Self::AscendantCrown => (
                "top skill gains rank 6 and +30% power",
                "最高技能升至6级，威力+30%",
            ),
            Self::ChaosDie => ("damage rolls 0.7x / 1.0x / 1.8x", "伤害随机为0.7/1.0/1.8倍"),
            Self::WorldAnchor => (
                "bosses move -15%; dodge grants control guard",
                "首领移速-15%；闪避抗控制",
            ),
            Self::LegionStandard => (
                "+1 summon cap, +30% inherited stats",
                "召唤上限+1，继承属性+30%",
            ),
            Self::PrismaticEngine => (
                "+80% elemental aftereffects and +40% area",
                "元素持续与连锁伤害+80%，范围+40%",
            ),
            Self::LastOath => (
                "negate one fatal hit; empower for 8s",
                "抵消一次致命伤；强化8秒",
            ),
        };
        language.pick(english, chinese)
    }

    pub(super) fn apply(
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
            Self::CorePatch => {
                session.heal_gate(200.0);
            }
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
                effects.physical_penetration = 3.0;
                build.attack_damage *= 1.22;
                effects.splash_ratio = effects.splash_ratio.max(0.30);
            }
            Self::ArcLens => {
                effects.spell_penetration = 0.10;
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
                effects.evasion = 0.08;
                build.move_speed *= 1.15;
                effects.dodge_cooldown_multiplier *= 0.85;
            }
            Self::EchoCore => effects.spell_echo_chance = 0.20,
            Self::EmberHeart => {
                effects.burns_can_crit = true;
                effects.critical_chance = 0.08;
            }
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
    offers: [MarketOffer; OFFER_COUNT],
    marked: [Option<MarketItem>; OFFER_COUNT],
    level: u8,
    manual_refreshes: u32,
    next_stock_at: f32,
    stock_generation: u32,
    autoplay_planned_generation: Option<u32>,
    autoplay_plan: [Option<AutoplayPurchasePlan>; AUTOPLAY_MAX_PURCHASES_PER_GENERATION],
    autoplay_purchases_generation: u8,
    open: bool,
    paused_by_market: bool,
    notice: [String; 2],
    notice_timer: f32,
}

impl Default for MarketState {
    fn default() -> Self {
        Self {
            offers: [MarketOffer::PLACEHOLDER; OFFER_COUNT],
            marked: [None; OFFER_COUNT],
            level: 1,
            manual_refreshes: 0,
            next_stock_at: MARKET_UNLOCK_AT + STOCK_INTERVAL,
            stock_generation: 0,
            autoplay_planned_generation: None,
            autoplay_plan: [None; AUTOPLAY_MAX_PURCHASES_PER_GENERATION],
            autoplay_purchases_generation: 0,
            open: false,
            paused_by_market: false,
            notice: [String::new(), String::new()],
            notice_timer: 0.0,
        }
    }
}

impl MarketState {
    fn refresh_price(&self, effects: &MarketEffects) -> u32 {
        let tier = (self.manual_refreshes as usize).min(REFRESH_COSTS.len() - 1);
        discounted_cost(REFRESH_COSTS[tier], effects)
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
    consumed_equipment: u32,
    skill_tuner_uses: u8,
    bond_compass_uses: u8,
    revival_seeds: u8,
    enemy_slow_remaining: f32,
    splash_ratio: f32,
    physical_penetration: f32,
    spell_penetration: f32,
    evasion: f32,
    critical_chance: f32,
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
            consumed_equipment: 0,
            skill_tuner_uses: 0,
            bond_compass_uses: 0,
            revival_seeds: 0,
            enemy_slow_remaining: 0.0,
            splash_ratio: 0.0,
            physical_penetration: 0.0,
            spell_penetration: 0.0,
            evasion: 0.0,
            critical_chance: 0.0,
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
    pub(crate) fn equipment(&self) -> Vec<MarketItem> {
        ALL_EQUIPMENT
            .iter()
            .copied()
            .filter(|item| {
                self.owns(*item)
                    && item
                        .equipment_index()
                        .is_some_and(|index| self.consumed_equipment & (1 << index) == 0)
            })
            .collect()
    }
    pub(crate) fn consume_equipment(&mut self, item: MarketItem) {
        if let Some(index) = item.equipment_index() {
            self.consumed_equipment |= 1 << index;
        }
    }
    pub(crate) fn apply_crafting_bonus(
        &mut self,
        recipe: super::crafting::RecipeId,
        vitals: &mut HeroVitals,
    ) {
        use super::crafting::RecipeId;
        match recipe {
            RecipeId::Bloodreaver => {
                self.physical_penetration += 3.0;
                self.life_leech += 0.05;
            }
            RecipeId::AstralPrism => {
                self.spell_penetration += 0.10;
                self.spell_power_multiplier *= 1.15;
            }
            RecipeId::StormBastion => {
                vitals.armor += 4.0;
                self.evasion += 0.05;
            }
            RecipeId::Suncoil => {
                self.critical_chance += 0.05;
                self.spell_power_multiplier *= 1.15;
            }
            RecipeId::NightCrown => {
                self.physical_penetration += 4.0;
                self.life_leech += 0.05;
                vitals.armor += 4.0;
            }
            RecipeId::WorldEngine => {
                self.spell_penetration += 0.10;
                self.spell_echo_chance = (self.spell_echo_chance + 0.10).min(1.0);
            }
        }
    }

    fn owns(&self, item: MarketItem) -> bool {
        item.equipment_index()
            .is_some_and(|index| self.owned_equipment & (1 << index) != 0)
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

    pub(crate) fn physical_penetration(&self) -> f32 {
        self.physical_penetration
    }
    pub(crate) fn spell_penetration(&self) -> f32 {
        self.spell_penetration
    }
    pub(crate) fn evasion(&self) -> f32 {
        self.evasion
    }
    pub(crate) fn critical_chance(&self) -> f32 {
        self.critical_chance
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

    pub(crate) fn chaos_damage_bounds(&self) -> (f32, f32) {
        if self.chaos_die {
            (chaos_multiplier_for_roll(0), chaos_multiplier_for_roll(2))
        } else {
            (1.0, 1.0)
        }
    }

    pub(crate) fn chaos_damage_multiplier(&self, rng: &mut RunRng) -> f32 {
        if !self.chaos_die {
            return 1.0;
        }
        roll_chaos_multiplier(rng)
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
struct MarketOfferRow;

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

#[derive(Component)]
struct MarketTooltip;

#[derive(Component)]
struct MarketTooltipText;

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
        let grid_rows = OFFER_COUNT.div_ceil(2) as f32;
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
            toggle_width: 96.0,
            toggle_height: 48.0,
        }
    }
}

fn setup_market(
    mut commands: Commands,
    assets: Option<Res<AssetServer>>,
    language: Res<GameLanguage>,
    mut state: ResMut<MarketState>,
    effects: Res<MarketEffects>,

    mut rng: ResMut<RunRng>,
) {
    let level = state.level;
    reroll_all(&mut state, &effects, &mut rng, level);

    commands
        .spawn((
            Button,
            MarketToggleButton,
            Node {
                position_type: PositionType::Absolute,
                right: px(18),
                bottom: px(18),
                width: px(96),
                height: px(48),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(9)),
                column_gap: px(6),
                ..default()
            },
            BackgroundColor(Color::srgba(0.045, 0.055, 0.075, 0.95)),
            BorderColor::all(Color::srgba(0.71, 0.55, 0.29, 0.8)),
            GlobalZIndex(21),
        ))
        .with_children(|button| {
            if let Some(assets) = assets.as_ref() {
                button.spawn((
                    ImageNode::new(assets.load("ui/icons/market.png")),
                    Node {
                        width: px(32),
                        height: px(32),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
            }
            button.spawn((
                MarketToggleLabel,
                Text::new("03:00"),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.78, 0.38)),
            ));
        });

    commands
        .spawn((
            MarketTooltip,
            Node {
                position_type: PositionType::Absolute,
                right: px(18),
                bottom: px(76),
                width: px(260),
                padding: UiRect::all(px(12)),
                display: Display::None,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(8)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.018, 0.027, 0.045, 0.97)),
            BorderColor::all(Color::srgba(0.71, 0.55, 0.29, 0.8)),
            GlobalZIndex(25),
            Pickable::IGNORE,
        ))
        .with_children(|tip| {
            tip.spawn((
                MarketTooltipText,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::srgb(0.92, 0.87, 0.74)),
                Pickable::IGNORE,
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
                        MarketTitle,
                        Text::new(language.pick("BLACK MARKET", "黑市")),
                        TextFont {
                            font_size: FontSize::Px(15.0),
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.72, 0.3)),
                    ));
                    header.spawn((
                        MarketWalletLabel,
                        Text::new(language.pick("KILLS 0000", "击杀 0000")),
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
                    for index in 0..OFFER_COUNT {
                        grid.spawn((
                            MarketOfferRow,
                            Node {
                                width: percent(100),
                                min_height: px(52),
                                column_gap: px(MARKET_PANEL_GAP),
                                align_items: AlignItems::Center,
                                ..default()
                            },
                        ))
                        .with_children(|row| {
                            row.spawn((
                                Button,
                                Name::new(format!("Market offer {index}")),
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
                                    TextLayout::linebreak(LineBreak::WordOrCharacter),
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
                                    Text::new(language.pick("AUTO", "自动")),
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
                        Text::new(language.pick("REFRESH", "刷新")),
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

    mut state: ResMut<MarketState>,
    mut rng: ResMut<RunRng>,
) {
    state.level = market_level(session.bosses_defeated);
    if session.elapsed < MARKET_UNLOCK_AT {
        return;
    }
    while session.elapsed >= state.next_stock_at {
        let level = state.level;
        reroll_all(&mut state, &effects, &mut rng, level);
        state.next_stock_at += STOCK_INTERVAL;
        state.stock_generation = state.stock_generation.saturating_add(1);
        if state.open {
            set_notice(&mut state, |language: GameLanguage| {
                language.pick("Automatic restock", "自动补货").to_string()
            });
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_market_actions(
    keyboard: Res<ButtonInput<KeyCode>>,
    autoplay: Option<Res<Autoplay>>,
    language: Res<GameLanguage>,
    mut state: ResMut<MarketState>,
    mut effects: ResMut<MarketEffects>,
    mut session: ResMut<GameSession>,
    mut build: ResMut<PlayerBuild>,
    mut vitals: ResMut<HeroVitals>,

    mut rng: ResMut<RunRng>,
    offer_interactions: Query<(&Interaction, &MarketOfferButton), Changed<Interaction>>,
    mark_interactions: Query<(&Interaction, &MarketMarkButton), Changed<Interaction>>,
    refresh_interactions: Query<&Interaction, (With<MarketRefreshButton>, Changed<Interaction>)>,
    toggle_interactions: Query<&Interaction, (With<MarketToggleButton>, Changed<Interaction>)>,
) {
    let language = *language;
    let toggle_requested = keyboard.just_pressed(KeyCode::KeyB)
        || toggle_interactions
            .iter()
            .any(|interaction| *interaction == Interaction::Pressed);
    if toggle_requested {
        if state.open {
            close_market(&mut state, &mut session);
        } else if session.elapsed < MARKET_UNLOCK_AT {
            let remaining = (MARKET_UNLOCK_AT - session.elapsed).ceil() as u32;
            set_notice(&mut state, |language: GameLanguage| {
                if language.is_chinese() {
                    format!("{:02}:{:02} 后解锁", remaining / 60, remaining % 60)
                } else {
                    format!("Unlocks in {:02}:{:02}", remaining / 60, remaining % 60)
                }
            });
        } else if session.finished {
            set_notice(&mut state, |language: GameLanguage| {
                language
                    .pick("Market closed after the run", "对局结束后黑市已关闭")
                    .to_string()
            });
        } else if session.paused {
            set_notice(&mut state, |language: GameLanguage| {
                language
                    .pick("Close the current choice first", "请先关闭当前选择")
                    .to_string()
            });
        } else {
            state.open = true;
            state.paused_by_market = true;
            session.paused = true;
        }
    }

    if state.open && session.elapsed >= MARKET_UNLOCK_AT {
        for (interaction, button) in &mark_interactions {
            if *interaction != Interaction::Pressed || button.0 >= OFFER_COUNT {
                continue;
            }
            let item = state.offers[button.0].item;
            if effects.owns(item) {
                state.marked[button.0] = None;
                set_notice(&mut state, |language: GameLanguage| {
                    language
                        .pick("Owned equipment cannot be marked", "已拥有装备，无法标记")
                        .to_string()
                });
            } else if state.marked[button.0] == Some(item) {
                state.marked[button.0] = None;
                set_notice(&mut state, |language: GameLanguage| {
                    if language.is_chinese() {
                        format!("取消自动购买：{}", item.name(language))
                    } else {
                        format!("Auto-buy canceled: {}", item.name(language))
                    }
                });
            } else {
                state.marked[button.0] = Some(item);
                set_notice(&mut state, |language: GameLanguage| {
                    if language.is_chinese() {
                        format!("标记自动购买：{}", item.name(language))
                    } else {
                        format!("Auto-buy marked: {}", item.name(language))
                    }
                });
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
                &mut rng,
            );
            match outcome {
                PurchaseOutcome::Bought(item) => {
                    set_notice(&mut state, |language: GameLanguage| {
                        if language.is_chinese() {
                            format!("已购买 {}", item.name(language))
                        } else {
                            format!("Bought {}", item.name(language))
                        }
                    });
                }
                PurchaseOutcome::Unaffordable(shortfall) => {
                    set_notice(&mut state, |language: GameLanguage| {
                        if language.is_chinese() {
                            format!("还差 {shortfall} 击杀")
                        } else {
                            format!("Need {shortfall} more kills")
                        }
                    });
                }
                PurchaseOutcome::Owned => set_notice(&mut state, |language: GameLanguage| {
                    language
                        .pick(
                            "Equipment already owned; stock replaced",
                            "装备已拥有，货品已替换",
                        )
                        .to_string()
                }),
                PurchaseOutcome::Hidden => {}
            }
        }

        for interaction in &refresh_interactions {
            if *interaction != Interaction::Pressed {
                continue;
            }
            let price = state.refresh_price(&effects);
            if session.kills < price {
                set_notice(&mut state, |language: GameLanguage| {
                    if language.is_chinese() {
                        format!("刷新还差 {} 击杀", price - session.kills)
                    } else {
                        format!("Refresh needs {} more kills", price - session.kills)
                    }
                });
                continue;
            }
            session.kills -= price;
            state.manual_refreshes = state.manual_refreshes.saturating_add(1);

            let level = state.level;
            reroll_all(&mut state, &effects, &mut rng, level);
            set_notice(&mut state, |language: GameLanguage| {
                language.pick("Stock refreshed", "货品已刷新").to_string()
            });
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
                &mut rng,
                language,
            );
        }
        process_auto_purchases(
            &mut state,
            &mut effects,
            &mut session,
            &mut build,
            &mut vitals,
            &mut rng,
            language,
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

    let mut candidates = Vec::with_capacity(OFFER_COUNT);
    for index in 0..OFFER_COUNT {
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

    rng: &mut RunRng,
    _language: GameLanguage,
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

        let price = discounted_cost(plan.item.price(), effects);
        if session.kills.saturating_sub(AUTOPLAY_KILL_RESERVE) < price {
            continue;
        }
        match purchase_offer(plan.index, state, effects, session, build, vitals, rng) {
            PurchaseOutcome::Bought(item) => {
                state.autoplay_plan[plan_index] = None;
                state.autoplay_purchases_generation =
                    state.autoplay_purchases_generation.saturating_add(1);
                bought.push(item);
            }
            PurchaseOutcome::Owned | PurchaseOutcome::Hidden => {
                state.autoplay_plan[plan_index] = None;
            }
            PurchaseOutcome::Unaffordable(_) => {}
        }
    }
    if !bought.is_empty() {
        set_notice(state, |language: GameLanguage| {
            if language.is_chinese() {
                format!(
                    "指挥官购买：{}",
                    bought
                        .iter()
                        .map(|item| item.name(language))
                        .collect::<Vec<_>>()
                        .join("、")
                )
            } else {
                format!(
                    "Commander bought {}",
                    bought
                        .iter()
                        .map(|item| item.name(language))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        });
    }
}

fn purchase_offer(
    index: usize,
    state: &mut MarketState,
    effects: &mut MarketEffects,
    session: &mut GameSession,
    build: &mut PlayerBuild,
    vitals: &mut HeroVitals,

    rng: &mut RunRng,
) -> PurchaseOutcome {
    if index >= OFFER_COUNT {
        return PurchaseOutcome::Hidden;
    }
    let item = state.offers[index].item;
    if effects.owns(item) {
        state.marked[index] = None;
        let level = state.level;
        replace_offer(index, state, effects, rng, level);
        return PurchaseOutcome::Owned;
    }

    let price = discounted_cost(item.price(), effects);
    if session.kills < price {
        return PurchaseOutcome::Unaffordable(price - session.kills);
    }

    session.kills -= price;
    item.apply(session, build, vitals, effects);
    state.marked[index] = None;
    let level = state.level;
    replace_offer(index, state, effects, rng, level);
    PurchaseOutcome::Bought(item)
}

fn process_auto_purchases(
    state: &mut MarketState,
    effects: &mut MarketEffects,
    session: &mut GameSession,
    build: &mut PlayerBuild,
    vitals: &mut HeroVitals,

    rng: &mut RunRng,
    _language: GameLanguage,
) {
    let mut bought = Vec::new();
    for index in 0..OFFER_COUNT {
        let Some(marked_item) = state.marked[index] else {
            continue;
        };
        if state.offers[index].item != marked_item {
            state.marked[index] = None;
            continue;
        }
        match purchase_offer(index, state, effects, session, build, vitals, rng) {
            PurchaseOutcome::Bought(item) => bought.push(item),
            PurchaseOutcome::Owned => {}
            PurchaseOutcome::Unaffordable(_) | PurchaseOutcome::Hidden => continue,
        }
    }
    if !bought.is_empty() {
        set_notice(state, |language: GameLanguage| {
            if language.is_chinese() {
                format!(
                    "自动购买：{}",
                    bought
                        .iter()
                        .map(|item| item.name(language))
                        .collect::<Vec<_>>()
                        .join("、")
                )
            } else {
                format!(
                    "Auto-bought {}",
                    bought
                        .iter()
                        .map(|item| item.name(language))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        });
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
        session.heal_gate(effects.core_regen_per_second * delta);
    }
}

fn tick_market_notice(time: Res<Time>, session: Res<GameSession>, mut state: ResMut<MarketState>) {
    if session.paused || session.finished || state.notice_timer <= 0.0 {
        return;
    }
    state.notice_timer = (state.notice_timer - time.delta_secs()).max(0.0);
    if state.notice_timer == 0.0 {
        state.notice.iter_mut().for_each(String::clear);
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
            node.reborrow()
                .map_unchanged(|value| &mut value.padding)
                .set_if_neq(UiRect::all(px(if layout.compact {
                    MARKET_COMPACT_PANEL_PADDING
                } else {
                    10.0
                })));
            node.reborrow()
                .map_unchanged(|value| &mut value.row_gap)
                .set_if_neq(px(MARKET_PANEL_GAP));
            node.reborrow()
                .map_unchanged(|value| &mut value.height)
                .set_if_neq(Val::Auto);
            node.reborrow()
                .map_unchanged(|value| &mut value.min_height)
                .set_if_neq(px(if layout.compact {
                    layout.minimum_compact_panel_height()
                } else {
                    0.0
                }));
            if layout.compact {
                node.reborrow()
                    .map_unchanged(|value| &mut value.left)
                    .set_if_neq(px(layout.safe_x));
                node.reborrow()
                    .map_unchanged(|value| &mut value.right)
                    .set_if_neq(px(layout.safe_x));
                node.reborrow()
                    .map_unchanged(|value| &mut value.top)
                    .set_if_neq(px(layout.panel_top));
                node.reborrow()
                    .map_unchanged(|value| &mut value.bottom)
                    .set_if_neq(px(layout.safe_y));
                node.reborrow()
                    .map_unchanged(|value| &mut value.width)
                    .set_if_neq(Val::Auto);
            } else {
                node.reborrow()
                    .map_unchanged(|value| &mut value.left)
                    .set_if_neq(Val::Auto);
                node.reborrow()
                    .map_unchanged(|value| &mut value.right)
                    .set_if_neq(px(18));
                node.reborrow()
                    .map_unchanged(|value| &mut value.top)
                    .set_if_neq(Val::Auto);
                node.reborrow()
                    .map_unchanged(|value| &mut value.bottom)
                    .set_if_neq(px(70));
                node.reborrow()
                    .map_unchanged(|value| &mut value.width)
                    .set_if_neq(px(360));
            }
        } else if header.is_some() {
            node.reborrow()
                .map_unchanged(|value| &mut value.width)
                .set_if_neq(percent(100));
            node.reborrow()
                .map_unchanged(|value| &mut value.min_height)
                .set_if_neq(px(if layout.compact {
                    MARKET_COMPACT_HEADER_HEIGHT
                } else {
                    0.0
                }));
        } else if grid.is_some() {
            node.reborrow()
                .map_unchanged(|value| &mut value.width)
                .set_if_neq(percent(100));
            node.reborrow()
                .map_unchanged(|value| &mut value.height)
                .set_if_neq(Val::Auto);
            node.reborrow()
                .map_unchanged(|value| &mut value.flex_direction)
                .set_if_neq(if layout.compact {
                    FlexDirection::Row
                } else {
                    FlexDirection::Column
                });
            node.reborrow()
                .map_unchanged(|value| &mut value.flex_wrap)
                .set_if_neq(if layout.compact {
                    FlexWrap::Wrap
                } else {
                    FlexWrap::NoWrap
                });
            node.reborrow()
                .map_unchanged(|value| &mut value.justify_content)
                .set_if_neq(JustifyContent::FlexStart);
            node.reborrow()
                .map_unchanged(|value| &mut value.align_items)
                .set_if_neq(AlignItems::Stretch);
            node.reborrow()
                .map_unchanged(|value| &mut value.column_gap)
                .set_if_neq(px(if layout.compact {
                    MARKET_PANEL_GAP
                } else {
                    0.0
                }));
            node.reborrow()
                .map_unchanged(|value| &mut value.row_gap)
                .set_if_neq(px(MARKET_PANEL_GAP));
        } else if row.is_some() {
            node.reborrow()
                .map_unchanged(|value| &mut value.width)
                .set_if_neq(percent(layout.row_width_percent));
            node.reborrow()
                .map_unchanged(|value| &mut value.height)
                .set_if_neq(if layout.compact {
                    px(layout.row_height)
                } else {
                    Val::Auto
                });
            node.reborrow()
                .map_unchanged(|value| &mut value.min_height)
                .set_if_neq(px(layout.row_height));
            node.reborrow()
                .map_unchanged(|value| &mut value.flex_grow)
                .set_if_neq(0.0);
            node.reborrow()
                .map_unchanged(|value| &mut value.flex_shrink)
                .set_if_neq(0.0);
        } else if offer.is_some() {
            node.reborrow()
                .map_unchanged(|value| &mut value.height)
                .set_if_neq(if layout.compact {
                    px(layout.row_height)
                } else {
                    Val::Auto
                });
            node.reborrow()
                .map_unchanged(|value| &mut value.min_height)
                .set_if_neq(px(layout.row_height));
        } else if mark.is_some() {
            node.reborrow()
                .map_unchanged(|value| &mut value.width)
                .set_if_neq(px(layout.mark_width));
            node.reborrow()
                .map_unchanged(|value| &mut value.height)
                .set_if_neq(if layout.compact {
                    px(layout.row_height)
                } else {
                    Val::Auto
                });
            node.reborrow()
                .map_unchanged(|value| &mut value.min_height)
                .set_if_neq(px(layout.row_height));
        } else if refresh.is_some() {
            node.reborrow()
                .map_unchanged(|value| &mut value.width)
                .set_if_neq(percent(100));
            node.reborrow()
                .map_unchanged(|value| &mut value.height)
                .set_if_neq(px(MARKET_REFRESH_HEIGHT));
            node.reborrow()
                .map_unchanged(|value| &mut value.min_height)
                .set_if_neq(px(MARKET_REFRESH_HEIGHT));
        } else if toggle.is_some() {
            node.reborrow()
                .map_unchanged(|value| &mut value.left)
                .set_if_neq(Val::Auto);
            node.reborrow()
                .map_unchanged(|value| &mut value.right)
                .set_if_neq(px(layout.toggle_right));
            node.reborrow()
                .map_unchanged(|value| &mut value.width)
                .set_if_neq(px(layout.toggle_width));
            node.reborrow()
                .map_unchanged(|value| &mut value.height)
                .set_if_neq(px(layout.toggle_height));
            node.reborrow()
                .map_unchanged(|value| &mut value.min_height)
                .set_if_neq(px(layout.toggle_height));
            if layout.compact {
                node.reborrow()
                    .map_unchanged(|value| &mut value.top)
                    .set_if_neq(px(layout.safe_y));
                node.reborrow()
                    .map_unchanged(|value| &mut value.bottom)
                    .set_if_neq(Val::Auto);
            } else {
                node.reborrow()
                    .map_unchanged(|value| &mut value.top)
                    .set_if_neq(Val::Auto);
                node.reborrow()
                    .map_unchanged(|value| &mut value.bottom)
                    .set_if_neq(px(layout.safe_y));
            }
        }
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn update_market_ui(
    windows: Query<&Window, With<PrimaryWindow>>,
    language: Res<GameLanguage>,
    state: Res<MarketState>,
    effects: Res<MarketEffects>,

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
    let language = *language;
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
        label
            .reborrow()
            .map_unchanged(|value| &mut value.0)
            .set_if_neq(if session.elapsed < MARKET_UNLOCK_AT {
                let remaining = (MARKET_UNLOCK_AT - session.elapsed).ceil() as u32;
                format!("{:02}:{:02}", remaining / 60, remaining % 60)
            } else if state.open {
                "×".to_string()
            } else {
                "B".to_string()
            });
        font.reborrow()
            .map_unchanged(|value| &mut value.font_size)
            .set_if_neq(FontSize::Px(13.0));
    }

    for (offer_label, mark_label, mut text, mut color, mut font) in &mut item_labels {
        font.reborrow()
            .map_unchanged(|value| &mut value.font_size)
            .set_if_neq(FontSize::Px(if layout.compact { 12.0 } else { 11.0 }));
        if let Some(label) = offer_label {
            let item = state.offers[label.0].item;
            let price = discounted_cost(item.price(), &effects);
            let kind = if item.is_consumable() {
                language.pick("USE", "使用")
            } else {
                language.pick("GEAR", "装备")
            };
            text.reborrow()
                .map_unchanged(|value| &mut value.0)
                .set_if_neq(if language.is_chinese() && layout.compact {
                    format!(
                        "{}   {} 击杀\n[{} / {}] {}",
                        item.name(language),
                        price,
                        item.tag(),
                        kind,
                        item.effect_text(language)
                    )
                } else if language.is_chinese() {
                    format!(
                        "[{} / {}] {}   {} 击杀\n{}",
                        item.tag(),
                        kind,
                        item.name(language),
                        price,
                        item.effect_text(language)
                    )
                } else if layout.compact {
                    format!(
                        "{}   {} K\n[{} / {}] {}",
                        item.name(language),
                        price,
                        item.tag(),
                        kind,
                        item.effect_text(language)
                    )
                } else {
                    format!(
                        "[{} / {}] {}   {} KILLS\n{}",
                        item.tag(),
                        kind,
                        item.name(language),
                        price,
                        item.effect_text(language)
                    )
                });
            color
                .reborrow()
                .map_unchanged(|value| &mut value.0)
                .set_if_neq(item.color());
        } else if let Some(label) = mark_label {
            let item = state.offers[label.0].item;
            let marked = state.marked[label.0] == Some(item);
            text.reborrow()
                .map_unchanged(|value| &mut value.0)
                .set_if_neq(if marked {
                    language.pick("AUTO\nON", "自动\n开启").to_string()
                } else {
                    language.pick("AUTO", "自动").to_string()
                });
            color
                .reborrow()
                .map_unchanged(|value| &mut value.0)
                .set_if_neq(if marked {
                    Color::srgb(0.30, 1.0, 0.68)
                } else {
                    Color::srgb(0.68, 0.84, 0.92)
                });
        }
    }

    let next_stock = (state.next_stock_at - session.elapsed).max(0.0).ceil() as u32;
    for (mut text, mut font) in &mut wallet_labels {
        font.reborrow()
            .map_unchanged(|value| &mut value.font_size)
            .set_if_neq(FontSize::Px(if layout.compact { 12.0 } else { 11.0 }));
        text.reborrow()
            .map_unchanged(|value| &mut value.0)
            .set_if_neq(if state.notice_timer > 0.0 {
                if language.is_chinese() && layout.compact {
                    format!(
                        "等级{}｜{}",
                        state.level,
                        state.notice[usize::from(language.is_chinese())]
                    )
                } else if language.is_chinese() {
                    format!(
                        "等级{}  {}",
                        state.level,
                        state.notice[usize::from(language.is_chinese())]
                    )
                } else if layout.compact {
                    format!(
                        "LV{} | {}",
                        state.level,
                        state.notice[usize::from(language.is_chinese())]
                    )
                } else {
                    format!(
                        "LV{}  {}",
                        state.level,
                        state.notice[usize::from(language.is_chinese())]
                    )
                }
            } else if language.is_chinese() && layout.compact {
                format!(
                    "等级{}｜击杀 {}｜补货 {:02}:{:02}",
                    state.level,
                    session.kills,
                    next_stock / 60,
                    next_stock % 60
                )
            } else if language.is_chinese() {
                format!(
                    "等级{}  击杀 {:04}\n补货 {:02}:{:02}",
                    state.level,
                    session.kills,
                    next_stock / 60,
                    next_stock % 60
                )
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
            });
    }

    for (mut text, mut font) in &mut refresh_labels {
        font.reborrow()
            .map_unchanged(|value| &mut value.font_size)
            .set_if_neq(FontSize::Px(if layout.compact { 13.0 } else { 12.0 }));
        let price = state.refresh_price(&effects);
        text.reborrow()
            .map_unchanged(|value| &mut value.0)
            .set_if_neq(if language.is_chinese() && price == 0 {
                if layout.compact {
                    "刷新｜免费".to_string()
                } else {
                    "刷新货品   免费".to_string()
                }
            } else if language.is_chinese() {
                if layout.compact {
                    format!("刷新｜{price} 击杀")
                } else {
                    format!("刷新货品   {price} 击杀")
                }
            } else if layout.compact && price == 0 {
                "REFRESH | FREE".to_string()
            } else if layout.compact {
                format!("REFRESH | {price} K")
            } else if price == 0 {
                "REFRESH STOCK   FREE".to_string()
            } else {
                format!("REFRESH STOCK   {price} KILLS")
            });
    }

    for (offer_button, mark_button, mut background) in &mut action_buttons {
        if let Some(button) = offer_button {
            let item = state.offers[button.0].item;
            let affordable = session.kills >= discounted_cost(item.price(), &effects);
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

fn update_market_tooltip(
    windows: Query<&Window, With<PrimaryWindow>>,
    language: Res<GameLanguage>,
    session: Res<GameSession>,
    state: Res<MarketState>,
    buttons: Query<&Interaction, With<MarketToggleButton>>,
    mut tips: Query<&mut Node, With<MarketTooltip>>,
    mut labels: Query<&mut Text, With<MarketTooltipText>>,
) {
    let layout = windows
        .single()
        .map(|w| market_layout(w.width(), w.height()))
        .unwrap_or_else(|_| market_layout(1280.0, 720.0));
    let visible = !state.open && buttons.iter().any(|i| *i != Interaction::None);
    for mut node in &mut tips {
        node.reborrow()
            .map_unchanged(|value| &mut value.display)
            .set_if_neq(if visible {
                Display::Flex
            } else {
                Display::None
            });
        node.reborrow()
            .map_unchanged(|value| &mut value.right)
            .set_if_neq(px(layout.safe_x));
        node.reborrow()
            .map_unchanged(|value| &mut value.top)
            .set_if_neq(if layout.compact {
                px(layout.safe_y + 60.0)
            } else {
                Val::Auto
            });
        node.reborrow()
            .map_unchanged(|value| &mut value.bottom)
            .set_if_neq(if layout.compact { Val::Auto } else { px(76) });
    }
    if !visible {
        return;
    }
    for mut text in &mut labels {
        text.reborrow()
            .map_unchanged(|value| &mut value.0)
            .set_if_neq(format!(
                "{}\n{}",
                market_toggle_label(
                    *language,
                    layout.compact,
                    session.elapsed,
                    state.open,
                    state.level
                ),
                language.pick(
                    "Spend kills on equipment and supplies. Preview each offer before buying.",
                    "使用击杀数兑换装备与补给，购买前可查看商品详情。"
                )
            ));
    }
}

fn market_toggle_label(
    language: GameLanguage,
    compact: bool,
    elapsed: f32,
    open: bool,
    level: u8,
) -> String {
    if elapsed < MARKET_UNLOCK_AT {
        let remaining = (MARKET_UNLOCK_AT - elapsed).ceil() as u32;
        if language.is_chinese() && compact {
            format!("黑市锁定 {:02}:{:02}", remaining / 60, remaining % 60)
        } else if language.is_chinese() {
            format!(
                "B  黑市  [锁定 {:02}:{:02}]",
                remaining / 60,
                remaining % 60
            )
        } else if compact {
            format!("MARKET LOCKED {:02}:{:02}", remaining / 60, remaining % 60)
        } else {
            format!(
                "B  BLACK MARKET  [LOCKED {:02}:{:02}]",
                remaining / 60,
                remaining % 60
            )
        }
    } else if open {
        if language.is_chinese() && compact {
            format!("关闭黑市  等级{level}")
        } else if language.is_chinese() {
            format!("B  关闭黑市  等级{level}")
        } else if compact {
            format!("CLOSE MARKET  LV{level}")
        } else {
            format!("B  CLOSE BLACK MARKET  LV{level}")
        }
    } else if language.is_chinese() && compact {
        format!("黑市  等级{level}")
    } else if language.is_chinese() {
        format!("B  打开黑市  等级{level}")
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

fn discounted_cost(price: u32, effects: &MarketEffects) -> u32 {
    (price as f32 * (1.0 - effects.market_discount)).ceil() as u32
}

fn set_notice(state: &mut MarketState, message: impl Fn(GameLanguage) -> String) {
    state.notice = [
        message(GameLanguage::English),
        message(GameLanguage::SimplifiedChinese),
    ];
    state.notice_timer = 2.2;
}

fn reroll_all(state: &mut MarketState, effects: &MarketEffects, rng: &mut RunRng, level: u8) {
    state.marked = [None; OFFER_COUNT];
    let mut selected = [MarketItem::CorePatch; OFFER_COUNT];
    for index in 0..OFFER_COUNT {
        let offer = roll_slot(index, effects, rng, level, &selected[..index]);
        state.offers[index] = offer;
        selected[index] = offer.item;
    }
}

fn replace_offer(
    index: usize,
    state: &mut MarketState,
    effects: &MarketEffects,

    rng: &mut RunRng,
    level: u8,
) {
    let selected = state.offers.map(|offer| offer.item);
    state.offers[index] = roll_slot(index, effects, rng, level, &selected);
    state.marked[index] = None;
}

fn roll_slot(
    index: usize,
    effects: &MarketEffects,

    rng: &mut RunRng,
    level: u8,
    avoid: &[MarketItem],
) -> MarketOffer {
    let rarity = roll_market_quality(level, rng);
    let item = match index {
        0 | 1 => roll_equipment(rarity, effects, rng, avoid),
        2 => roll_consumable(rarity, rng, avoid),
        _ if rng.next_u32().is_multiple_of(4) => roll_consumable(rarity, rng, avoid),
        _ => roll_equipment(rarity, effects, rng, avoid),
    };
    MarketOffer { item }
}

fn roll_consumable(rarity: MarketQuality, rng: &mut RunRng, avoid: &[MarketItem]) -> MarketItem {
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
    rarity: MarketQuality,
    effects: &MarketEffects,
    rng: &mut RunRng,
    avoid: &[MarketItem],
) -> MarketItem {
    let pool: &[MarketItem] = match rarity {
        MarketQuality::White => &WHITE_EQUIPMENT,
        MarketQuality::Blue => &BLUE_EQUIPMENT,
        MarketQuality::Purple => &PURPLE_EQUIPMENT,
        MarketQuality::Orange => &ORANGE_EQUIPMENT,
        MarketQuality::Red => &RED_EQUIPMENT,
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

fn market_quality_for_roll(level: u8, roll: u8) -> MarketQuality {
    let weights = RARITY_WEIGHTS[(level.clamp(1, MARKET_LEVEL_COUNT) - 1) as usize];
    let mut upper = 0_u8;
    for (rarity, weight) in MarketQuality::ALL.into_iter().zip(weights) {
        upper = upper.saturating_add(weight);
        if roll < upper {
            return rarity;
        }
    }
    MarketQuality::Red
}

fn roll_market_quality(level: u8, rng: &mut RunRng) -> MarketQuality {
    let roll = (rng.next_u32() % 100) as u8;
    market_quality_for_roll(level, roll)
}

pub(crate) fn reset_run(world: &mut World) {
    world.insert_resource(MarketState::default());
    world.insert_resource(MarketEffects::default());
}

pub fn prepare_crafting_market(world: &mut World, phase: u8) {
    let items = if phase == 0 {
        [
            MarketItem::SerratedDisc,
            MarketItem::BloodContract,
            MarketItem::HardenedPlate,
            MarketItem::WindBoots,
            MarketItem::AscendantCrown,
        ]
    } else {
        [
            MarketItem::ArcLens,
            MarketItem::EchoCore,
            MarketItem::EmberHeart,
            MarketItem::StormCoil,
            MarketItem::PrismaticEngine,
        ]
    };
    let mut state = world.resource_mut::<MarketState>();
    state.offers = items.map(|item| MarketOffer { item });
    state.level = 1;
    state.next_stock_at = 10000.0;
    state.open = true;
    state.paused_by_market = true;
}

/// Deterministic stock for the native input capture harness; purchases still
/// pass through the normal button handler, prices and ownership validation.
pub fn prepare_attribute_growth_market(world: &mut World) {
    let mut state = world.resource_mut::<MarketState>();
    state.offers = [
        MarketItem::SerratedDisc,
        MarketItem::ArcLens,
        MarketItem::WindBoots,
        MarketItem::HardenedPlate,
        MarketItem::BloodContract,
    ]
    .map(|item| MarketOffer { item });
    state.level = 1;
    state.next_stock_at = 10000.0;
    state.open = true;
    state.paused_by_market = true;
}

#[cfg(test)]
pub(crate) fn purchased_attribute_equipment_for_test() -> (PlayerBuild, HeroVitals, MarketEffects) {
    let mut state = MarketState::default();
    let mut effects = MarketEffects::default();
    let mut session = GameSession {
        kills: 10000,
        ..default()
    };
    let mut build = PlayerBuild::default();
    let mut vitals = HeroVitals::default();
    let mut rng = RunRng::default();
    for item in [
        MarketItem::SerratedDisc,
        MarketItem::ArcLens,
        MarketItem::WindBoots,
        MarketItem::HardenedPlate,
        MarketItem::BloodContract,
        MarketItem::EmberHeart,
    ] {
        state.offers[0] = MarketOffer { item };
        let balance = session.kills;
        assert_eq!(
            purchase_offer(
                0,
                &mut state,
                &mut effects,
                &mut session,
                &mut build,
                &mut vitals,
                &mut rng
            ),
            PurchaseOutcome::Bought(item)
        );
        assert_eq!(session.kills, balance - item.price());
        state.offers[0] = MarketOffer { item };
        assert_eq!(
            purchase_offer(
                0,
                &mut state,
                &mut effects,
                &mut session,
                &mut build,
                &mut vitals,
                &mut rng
            ),
            PurchaseOutcome::Owned
        );
        assert_eq!(session.kills, balance - item.price());
    }
    (build, vitals, effects)
}

#[cfg(test)]
pub(crate) fn purchased_crafting_materials_for_test() -> (PlayerBuild, HeroVitals, MarketEffects) {
    let mut state = MarketState::default();
    let mut effects = MarketEffects::default();
    let mut session = GameSession {
        kills: 10000,
        ..default()
    };
    let mut build = PlayerBuild::default();
    let mut vitals = HeroVitals::default();
    let mut rng = RunRng::default();
    for item in [
        MarketItem::SerratedDisc,
        MarketItem::BloodContract,
        MarketItem::HardenedPlate,
        MarketItem::WindBoots,
        MarketItem::AscendantCrown,
        MarketItem::ArcLens,
        MarketItem::EchoCore,
        MarketItem::EmberHeart,
        MarketItem::StormCoil,
        MarketItem::PrismaticEngine,
    ] {
        state.offers[0] = MarketOffer { item };
        assert_eq!(
            purchase_offer(
                0,
                &mut state,
                &mut effects,
                &mut session,
                &mut build,
                &mut vitals,
                &mut rng
            ),
            PurchaseOutcome::Bought(item)
        );
    }
    (build, vitals, effects)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn consumed_crafting_material_cannot_be_bought_again_or_reapply_its_stats() {
        let (mut build, vitals, effects) = purchased_attribute_equipment_for_test();
        let mut world = World::new();
        world.insert_resource(vitals);
        world.insert_resource(effects);
        world.init_resource::<super::super::crafting::CraftingState>();
        assert!(super::super::crafting::craft_recipe(
            &mut world,
            super::super::crafting::RecipeId::Bloodreaver
        ));
        let mut state = MarketState::default();
        state.offers[0] = MarketOffer {
            item: MarketItem::SerratedDisc,
        };
        let mut session = GameSession {
            kills: 1000,
            ..default()
        };
        let mut vitals = world.resource::<HeroVitals>().clone();
        let before = build.attack_damage;
        world.resource_scope(|_world, mut effects: Mut<MarketEffects>| {
            assert_eq!(
                purchase_offer(
                    0,
                    &mut state,
                    &mut effects,
                    &mut session,
                    &mut build,
                    &mut vitals,
                    &mut RunRng::default()
                ),
                PurchaseOutcome::Owned
            );
            assert_eq!(session.kills, 1000);
            assert_eq!(build.attack_damage, before);
            assert_eq!(effects.physical_penetration(), 6.0);
            assert!(!effects.equipment().contains(&MarketItem::SerratedDisc));
        });
    }

    #[test]
    fn purchased_equipment_unlocks_real_stats_once_and_bond_runtime_matches_the_sheet() {
        let (build, vitals, effects) = purchased_attribute_equipment_for_test();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<GameSession>()
            .insert_resource(build)
            .insert_resource(vitals)
            .insert_resource(effects)
            .add_plugins(super::super::bonds::BondsPlugin);
        app.update();
        let runtime = app.world().resource::<super::super::bonds::BondRuntime>();
        let snapshot = super::super::hero_stats::HeroStatsSnapshot::capture(
            app.world().resource::<HeroVitals>(),
            app.world().resource::<PlayerBuild>(),
            runtime,
            app.world().resource::<MarketEffects>(),
            &super::super::hero_abilities::HeroAttackSpeedModifier::default(),
        );
        assert_eq!(snapshot.armor, 6.0);
        assert_eq!(snapshot.physical_penetration, 3.0);
        assert!((snapshot.spell_penetration - 0.10).abs() < 0.0001);
        assert!((snapshot.evasion - 0.08).abs() < 0.0001);
        assert!((snapshot.crit_chance - 0.13).abs() < 0.0001);
        assert!((snapshot.life_leech - 0.05).abs() < 0.0001);
        assert!(runtime.evade(0.079));
        assert!(!runtime.evade(0.08));
        assert_eq!(
            super::super::bonds::critical_multiplier(
                runtime,
                false,
                super::super::bonds::DamageProfile::PHYSICAL_ATTACK,
                0.12
            ),
            runtime.critical_damage()
        );
        let mut vitals = app.world().resource::<HeroVitals>().clone();
        let expected = 100.0 * 100.0 / 130.0;
        assert!((vitals.take_physical_damage(100.0) - expected).abs() < 0.001);
        let kind = super::super::HeroKind::StoneWarden;
        vitals.synchronize_level(kind, 10);
        assert_eq!(vitals.armor, 6.0);
    }

    #[test]
    fn live_market_language_keeps_stock_wallet_and_translates_existing_notice() {
        let mut app = test_app(MARKET_UNLOCK_AT);
        {
            let mut state = app.world_mut().resource_mut::<MarketState>();
            state.open = true;
            set_notice(&mut state, |language| {
                language.pick("Stock refreshed", "货品已刷新").into()
            });
        }
        app.world_mut().resource_mut::<GameSession>().paused = true;
        let offers = app
            .world()
            .resource::<MarketState>()
            .offers
            .map(|offer| offer.item);
        let rng = app.world().resource::<RunRng>().0;
        let kills = app.world().resource::<GameSession>().kills;
        *app.world_mut().resource_mut::<GameLanguage>() = GameLanguage::SimplifiedChinese;
        app.update();
        assert_eq!(
            app.world()
                .resource::<MarketState>()
                .offers
                .map(|offer| offer.item),
            offers
        );
        assert_eq!(app.world().resource::<RunRng>().0, rng);
        assert_eq!(app.world().resource::<GameSession>().kills, kills);
        assert!(app.world().resource::<MarketState>().open);
        let titles: Vec<_> = app
            .world_mut()
            .query_filtered::<&Text, With<MarketTitle>>()
            .iter(app.world())
            .map(|text| text.0.clone())
            .collect();
        assert_eq!(titles, ["黑市"]);
        assert!(
            app.world_mut()
                .query_filtered::<&Text, With<MarketWalletLabel>>()
                .iter(app.world())
                .any(|text| text.0.contains("货品已刷新"))
        );
    }

    #[test]
    fn every_market_item_has_distinct_complete_bilingual_copy() {
        let items: Vec<_> = CONSUMABLES.into_iter().chain(ALL_EQUIPMENT).collect();
        assert_eq!(items.len(), 24);
        assert_eq!(items.iter().copied().collect::<HashSet<_>>().len(), 24);

        for item in items {
            let english_name = item.name(GameLanguage::English);
            let chinese_name = item.name(GameLanguage::SimplifiedChinese);
            let english_effect = item.effect_text(GameLanguage::English);
            let chinese_effect = item.effect_text(GameLanguage::SimplifiedChinese);
            assert!(!english_name.is_empty());
            assert!(!chinese_name.is_empty());
            assert!(!english_effect.is_empty());
            assert!(!chinese_effect.is_empty());
            assert_ne!(english_name, chinese_name);
            assert_ne!(english_effect, chinese_effect);
        }
    }

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
    fn five_internal_market_bands_present_the_shared_four_build_tiers() {
        assert_eq!(MarketItem::CorePatch.build_rarity(), BuildRarity::R);
        assert_eq!(MarketItem::SkillTuner.build_rarity(), BuildRarity::Sr);
        assert_eq!(MarketItem::BondCompass.build_rarity(), BuildRarity::Ssr);
        assert_eq!(MarketItem::SlowHourglass.build_rarity(), BuildRarity::Ssr);
        assert_eq!(MarketItem::RevivalSeed.build_rarity(), BuildRarity::Ur);
        assert_eq!(MarketItem::ThreefoldRelic.build_rarity(), BuildRarity::Ssr);
        assert_eq!(MarketItem::AscendantCrown.build_rarity(), BuildRarity::Ssr);

        for item in CONSUMABLES.into_iter().chain(ALL_EQUIPMENT) {
            assert!(matches!(item.tag(), "R" | "SR" | "SSR" | "UR"));
            assert_eq!(item.color(), rarity_color(item.build_rarity()));
        }
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
            assert_eq!(OFFER_COUNT.div_ceil(2), 3);
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

        let mobile_locked = market_toggle_label(GameLanguage::English, true, 0.0, false, 1);
        let mobile_open =
            market_toggle_label(GameLanguage::English, true, MARKET_UNLOCK_AT, true, 1);
        assert!(!mobile_locked.starts_with("B  "));
        assert!(!mobile_open.starts_with("B  "));
        assert!(
            market_toggle_label(GameLanguage::English, false, 0.0, false, 1).starts_with("B  ")
        );
        assert_eq!(
            market_toggle_label(GameLanguage::SimplifiedChinese, true, 0.0, false, 1),
            "黑市锁定 03:00"
        );
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
            OFFER_COUNT
        );
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<MarketMarkButton>>()
                .iter(app.world())
                .count(),
            OFFER_COUNT
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
        let mut first_state = MarketState::default();
        let mut second_state = MarketState::default();
        let mut first_rng = RunRng::default();
        let mut second_rng = RunRng::default();
        assert_eq!(first_state.refresh_price(&effects), 60);
        reroll_all(&mut first_state, &effects, &mut first_rng, 5);
        reroll_all(&mut second_state, &effects, &mut second_rng, 5);
        assert_eq!(first_state.offers, second_state.offers);
        assert!(!first_state.offers[0].item.is_consumable());
        assert!(!first_state.offers[1].item.is_consumable());
        assert!(first_state.offers[2].item.is_consumable());
        for left in 0..OFFER_COUNT {
            for right in left + 1..OFFER_COUNT {
                assert_ne!(first_state.offers[left], first_state.offers[right]);
            }
        }

        first_state.marked[0] = Some(first_state.offers[0].item);
        reroll_all(&mut first_state, &effects, &mut first_rng, 5);
        assert!(first_state.marked.iter().all(Option::is_none));
        first_state.manual_refreshes = 99;
        assert_eq!(first_state.refresh_price(&effects), 120);
    }

    #[test]
    fn all_five_market_levels_match_public_rarity_probabilities() {
        for (index, expected) in RARITY_WEIGHTS.into_iter().enumerate() {
            let level = index as u8 + 1;
            let mut actual = [0_u8; 5];
            for roll in 0..100 {
                let rarity = market_quality_for_roll(level, roll);
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
        fn run_once() -> (u32, u32, [MarketOffer; OFFER_COUNT], u64, usize) {
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
        assert_eq!(first.4, OFFER_COUNT);
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
    fn gate_items_call_the_gate_a_gate_in_both_languages() {
        for item in [MarketItem::CorePatch, MarketItem::GuardianSeed] {
            let english = item.effect_text(GameLanguage::English);
            assert!(english.contains("gate") && !english.contains("core"));
            let chinese = item.effect_text(GameLanguage::SimplifiedChinese);
            assert!(chinese.contains("城门") && !chinese.contains("核心"));
        }
        assert_eq!(
            MarketItem::CorePatch.name(GameLanguage::English),
            "Gate Patch"
        );
        assert_eq!(
            MarketItem::CorePatch.name(GameLanguage::SimplifiedChinese),
            "城门修补包"
        );
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
    fn hero_sheet_tracks_equipment_leech_low_health_damage_and_chaos_range() {
        use crate::game::{
            bonds::BondRuntime, hero_abilities::HeroAttackSpeedModifier,
            hero_stats::HeroStatsSnapshot,
        };
        let (_, build, mut vitals, mut effects) = apply_item_for_test(MarketItem::BloodContract);
        let bonds = BondRuntime::default();
        let speed = HeroAttackSpeedModifier::default();
        let full = HeroStatsSnapshot::capture(&vitals, &build, &bonds, &effects, &speed);
        assert_eq!(full.life_leech, 0.05);
        assert_eq!(full.basic_hit_damage, build.attack_damage);
        vitals.health = vitals.max_health * 0.5;
        let low = HeroStatsSnapshot::capture(&vitals, &build, &bonds, &effects, &speed);
        assert!((low.basic_hit_damage - full.basic_hit_damage * 1.4).abs() < 0.001);
        assert_eq!(low.life_leech, full.life_leech);
        effects.chaos_die = true;
        let chaos = HeroStatsSnapshot::capture(&vitals, &build, &bonds, &effects, &speed);
        assert!((chaos.basic_hit_min - low.basic_hit_damage * 0.7).abs() < 0.001);
        assert!((chaos.basic_hit_max - low.basic_hit_damage * 1.8).abs() < 0.001);
        // The sheet reports the owned rate, while the actual heal path still
        // rejects a dead hero and never heals from armor-absorbed damage.
        vitals.alive = false;
        vitals.health = 0.0;
        assert_eq!(vitals.heal(100.0 * chaos.life_leech), 0.0);
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
    fn chaos_die_has_three_equal_outcomes_and_consumes_one_roll() {
        assert_eq!(chaos_multiplier_for_roll(0), 0.7);
        assert_eq!(chaos_multiplier_for_roll(1), 1.0);
        assert_eq!(chaos_multiplier_for_roll(2), 1.8);
        assert_eq!(chaos_multiplier_for_roll(3), 0.7);

        let (_, _, _, effects) = apply_item_for_test(MarketItem::ChaosDie);
        let mut expected_rng = RunRng::from_seed(0xCA05);
        let first = roll_chaos_multiplier(&mut expected_rng);
        let expected_state = expected_rng.state();
        let mut actual_rng = RunRng::from_seed(0xCA05);
        assert_eq!(effects.chaos_damage_multiplier(&mut actual_rng), first);
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

#[derive(Component)]
struct MarketTitle;

fn refresh_market_language(
    language: Res<GameLanguage>,
    mut titles: Query<&mut Text, With<MarketTitle>>,
) {
    if !language.is_changed() {
        return;
    }
    for mut title in &mut titles {
        title.set_if_neq(Text::new(language.pick("BLACK MARKET", "黑市")));
    }
}
