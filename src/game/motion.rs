//! AAA 112-action full-body procedural motion system, zero-drift articulation,
//! hitstop victim micro-tremor, directional additive impact VFX, 3-hit combo
//! blade/projectile smears, footstep/skid-stop dust, and live bilingual HUD badge.
//!
//! Gameplay `Transform`s remain strictly authoritative and zero-drift:
//! `apply_motion_visuals` applies procedural pose + tremor deltas in `PostUpdate`
//! before `TransformSystems::Propagate`, and `restore_motion_visuals` removes
//! the exact applied delta in `First` before any gameplay or animation system runs.

use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};

use bevy::{
    asset::RenderAssetUsages,
    ecs::system::SystemParam,
    light::{NotShadowCaster, NotShadowReceiver},
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

use super::basic_attacks::{BasicAttackAim, BasicWeapon, PendingSwing, weapon};
use super::bonds::DamageElement;
use super::bosses::{BossAction, BossActionStage, BossPhaseCue};
use super::combat::{AttackVolleyRuntime, CombatSet, EliteAffix};
use super::hero_abilities::{TimedSpeedEffect, VoidPull, WetMark};
use super::hit_feedback::{HitFeedback, HitFeedbackSet, HitStop, HitWeight};
use super::pooling::damage_element_color;
use super::skills::SkillRuntime;
use super::skills::infection::Infection;
use super::ultimate_vfx::UltimateCues;
use super::{
    Autoplay, BossEnemy, DodgeState, Enemy, EnemyAttackCue, EnemyAttackKind, GameLanguage,
    GameSession, Hero, HeroKind, HeroVisual, HeroVitals, MoveIntent, PlayerSummon, SelectedHero,
};

/// 62.5 Hz victim micro-tremor frequency during hitstop / hit-react (within 60–65 Hz).
pub(crate) const HIT_TREMOR_HZ: f32 = 62.5;
/// Exact 85ms contact hit-flash core duration.
pub(crate) const CONTACT_FLASH_CORE_SECONDS: f32 = 0.085;
/// Maximum simultaneous additive visceral VFX entities (sparks, cores, smears, dust).
const MAX_VISCERAL_VFX: usize = 220;
/// Hero stride distance in meters between footstep dust puffs.
const FOOTSTEP_STRIDE_METERS: f32 = 1.45;
/// Combo chain window before resetting to step 0.
const COMBO_RESET_SECONDS: f32 = 1.35;
/// Minimum prior sprint speed to trigger skid-stop inertia when halting sharply.
const SKID_ENTRY_SPEED: f32 = 3.0;
/// Speed threshold below which a sharp halt enters `StopSkidRecover`.
const SKID_HALT_SPEED: f32 = 0.55;
/// Hero emote roster cycled with `Digit0`.
pub(crate) const HERO_EMOTE_CYCLE: [MotionId; 7] = [
    MotionId::EmoteSalute,
    MotionId::EmoteBattleTaunt,
    MotionId::EmoteVictoryFlourish,
    MotionId::EmoteBeckonRally,
    MotionId::EmoteBowRespect,
    MotionId::EmoteWeaponInspect,
    MotionId::EmoteWarhornCall,
];

pub(crate) struct MotionPlugin;

impl Plugin for MotionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HeroMotionTracker>()
            .init_resource::<HeroCastLock>()
            .init_resource::<VisceralVfxSequence>()
            .add_systems(Startup, (setup_visceral_vfx_art, setup_motion_hud_badge))
            .add_systems(First, restore_motion_visuals)
            .add_systems(
                Update,
                (
                    ensure_actor_motion_states,
                    drive_hero_motion_and_smears,
                    drive_enemy_boss_summon_motions,
                    spawn_visceral_hit_vfx,
                    tick_motion_states_and_tremors,
                    animate_visceral_vfx,
                    update_motion_hud_badge,
                )
                    .chain()
                    .after(HitFeedbackSet)
                    .after(CombatSet),
            )
            .add_systems(
                PostUpdate,
                apply_motion_visuals
                    .after(bevy::app::AnimationSystems)
                    .before(bevy::transform::TransformSystems::Propagate),
            );
    }
}

/// High-level action family for every named motion in the catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MotionCategory {
    Locomotion,
    ChargeTelegraph,
    ClassAttackCombo,
    EvadeRecovery,
    HitstunKnockback,
    StatusAilment,
    BuffInteraction,
    EnemySpecial,
    BossSpecial,
    AllyEmote,
}

/// Metadata for a single named full-body action.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionMeta {
    pub zh: &'static str,
    pub en: &'static str,
    pub category: MotionCategory,
    pub priority: u8,
    pub duration: f32,
    pub looping: bool,
    pub show_badge: bool,
}

/// 112 distinct named full-body combat, locomotion, status, enemy, boss, and emote actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum MotionId {
    // --- Locomotion & Stances (12) ---
    IdleReady,
    IdleCombatGuard,
    IdleExhaustedLowHp,
    WalkMarch,
    RunSprint,
    StrafeLeft,
    StrafeRight,
    BackpedalRetreat,
    TurnPivotLeft,
    TurnPivotRight,
    StopSkidRecover,
    LandingCrouch,

    // --- Charge, Windup & Telegraphs (10) ---
    ChargeHoldLight,
    ChargeHoldHeavy,
    ChargePeakOverload,
    SpellChannelLoop,
    BowDrawHold,
    CrossbowLockAim,
    HeavyOverheadWindup,
    ShieldBraceGuard,
    UltimateGatherPower,
    TelegraphGroundSlam,

    // --- Class Attacks & 3-Hit Combos (18) ---
    SwordCombo1Slash,
    SwordCombo2Cross,
    SwordCombo3Finisher,
    DaggerCombo1Flick,
    DaggerCombo2Twin,
    DaggerCombo3Spin,
    AxeCombo1Cleave,
    AxeCombo2Rend,
    AxeCombo3Crush,
    CrossbowCombo1Snap,
    CrossbowCombo2Burst,
    CrossbowCombo3Pierce,
    StormCastCombo1,
    StormCastCombo2,
    StormCastCombo3,
    FrostArcaneCast1,
    SpiritStarCast2,
    UltimateUnleashBurst,

    // --- Evades, Dashes & Recovery (10) ---
    DodgeRollForward,
    DodgeDashBackstep,
    DodgeSidestepLeft,
    DodgeSidestepRight,
    PhaseBlinkOut,
    PhaseBlinkIn,
    DodgeLandRecover,
    VaultLeapForward,
    CombatSlideCancel,
    GuardBreakRecover,

    // --- Directional Hitstun, Stagger & Knockback (10) ---
    HitReactFront,
    HitReactBack,
    HitReactLeft,
    HitReactRight,
    HitReactHeavy,
    StaggerKneel,
    StaggerDazeLoop,
    KnockbackLaunch,
    KnockbackSkid,
    GetupRecovery,

    // --- Status Ailments (10) ---
    AilmentBurning,
    AilmentChilled,
    AilmentFrozenRigid,
    AilmentStunnedDaze,
    AilmentBleeding,
    AilmentVulnerable,
    AilmentSlowedMud,
    AilmentJailedRooted,
    AilmentInfectedBlight,
    AilmentExhaustedGasp,

    // --- Buffs, Consumables, Interactions, Level-Up & Death (12) ---
    BuffBattleCry,
    BuffAuraPulse,
    ConsumeElixirDrink,
    InteractCampInspect,
    InteractForgeCraft,
    InteractGateRepair,
    LevelUpAscend,
    HeroRecruitAwaken,
    BondResonanceCheer,
    ReviveRiseUp,
    DeathCollapseForward,
    DeathCollapseBack,

    // --- Enemy Specialized Motions (10) ---
    EnemyLungeBite,
    EnemyShieldBash,
    EnemyRangedDrawFire,
    EnemyBurrowSubmerge,
    EnemyBurrowEmerge,
    EnemyChantHaste,
    EnemyHealPulse,
    EnemyVolatileSwell,
    EnemySiegeLob,
    EnemySplitSpawn,

    // --- Boss Specialized Motions (10) ---
    BossEntranceRoar,
    BossPhaseShiftErupt,
    BossHeavyOverheadSlam,
    BossSweepingCleave,
    BossLaserBeamFocus,
    BossShockwaveStomp,
    BossSummonCall,
    BossEnrageHowl,
    BossStaggerBroken,
    BossDefeatFall,

    // --- Ally, Summon, Tower & Co-op Emotes (10) ---
    EmoteSalute,
    EmoteBattleTaunt,
    EmoteVictoryFlourish,
    EmoteBeckonRally,
    EmoteBowRespect,
    EmoteWeaponInspect,
    EmoteWarhornCall,
    SummonSentryAim,
    SummonColossusSlam,
    TowerOverchargePulse,
}

impl MotionId {
    pub const ALL: [Self; 112] = [
        // Locomotion (12)
        Self::IdleReady,
        Self::IdleCombatGuard,
        Self::IdleExhaustedLowHp,
        Self::WalkMarch,
        Self::RunSprint,
        Self::StrafeLeft,
        Self::StrafeRight,
        Self::BackpedalRetreat,
        Self::TurnPivotLeft,
        Self::TurnPivotRight,
        Self::StopSkidRecover,
        Self::LandingCrouch,
        // Charge & Telegraph (10)
        Self::ChargeHoldLight,
        Self::ChargeHoldHeavy,
        Self::ChargePeakOverload,
        Self::SpellChannelLoop,
        Self::BowDrawHold,
        Self::CrossbowLockAim,
        Self::HeavyOverheadWindup,
        Self::ShieldBraceGuard,
        Self::UltimateGatherPower,
        Self::TelegraphGroundSlam,
        // Class Attacks & 3-Hit Combos (18)
        Self::SwordCombo1Slash,
        Self::SwordCombo2Cross,
        Self::SwordCombo3Finisher,
        Self::DaggerCombo1Flick,
        Self::DaggerCombo2Twin,
        Self::DaggerCombo3Spin,
        Self::AxeCombo1Cleave,
        Self::AxeCombo2Rend,
        Self::AxeCombo3Crush,
        Self::CrossbowCombo1Snap,
        Self::CrossbowCombo2Burst,
        Self::CrossbowCombo3Pierce,
        Self::StormCastCombo1,
        Self::StormCastCombo2,
        Self::StormCastCombo3,
        Self::FrostArcaneCast1,
        Self::SpiritStarCast2,
        Self::UltimateUnleashBurst,
        // Evades & Recovery (10)
        Self::DodgeRollForward,
        Self::DodgeDashBackstep,
        Self::DodgeSidestepLeft,
        Self::DodgeSidestepRight,
        Self::PhaseBlinkOut,
        Self::PhaseBlinkIn,
        Self::DodgeLandRecover,
        Self::VaultLeapForward,
        Self::CombatSlideCancel,
        Self::GuardBreakRecover,
        // Hitstun & Stagger (10)
        Self::HitReactFront,
        Self::HitReactBack,
        Self::HitReactLeft,
        Self::HitReactRight,
        Self::HitReactHeavy,
        Self::StaggerKneel,
        Self::StaggerDazeLoop,
        Self::KnockbackLaunch,
        Self::KnockbackSkid,
        Self::GetupRecovery,
        // Status Ailments (10)
        Self::AilmentBurning,
        Self::AilmentChilled,
        Self::AilmentFrozenRigid,
        Self::AilmentStunnedDaze,
        Self::AilmentBleeding,
        Self::AilmentVulnerable,
        Self::AilmentSlowedMud,
        Self::AilmentJailedRooted,
        Self::AilmentInfectedBlight,
        Self::AilmentExhaustedGasp,
        // Buffs & Interactions (12)
        Self::BuffBattleCry,
        Self::BuffAuraPulse,
        Self::ConsumeElixirDrink,
        Self::InteractCampInspect,
        Self::InteractForgeCraft,
        Self::InteractGateRepair,
        Self::LevelUpAscend,
        Self::HeroRecruitAwaken,
        Self::BondResonanceCheer,
        Self::ReviveRiseUp,
        Self::DeathCollapseForward,
        Self::DeathCollapseBack,
        // Enemy Specialized (10)
        Self::EnemyLungeBite,
        Self::EnemyShieldBash,
        Self::EnemyRangedDrawFire,
        Self::EnemyBurrowSubmerge,
        Self::EnemyBurrowEmerge,
        Self::EnemyChantHaste,
        Self::EnemyHealPulse,
        Self::EnemyVolatileSwell,
        Self::EnemySiegeLob,
        Self::EnemySplitSpawn,
        // Boss Specialized (10)
        Self::BossEntranceRoar,
        Self::BossPhaseShiftErupt,
        Self::BossHeavyOverheadSlam,
        Self::BossSweepingCleave,
        Self::BossLaserBeamFocus,
        Self::BossShockwaveStomp,
        Self::BossSummonCall,
        Self::BossEnrageHowl,
        Self::BossStaggerBroken,
        Self::BossDefeatFall,
        // Ally / Emotes (10)
        Self::EmoteSalute,
        Self::EmoteBattleTaunt,
        Self::EmoteVictoryFlourish,
        Self::EmoteBeckonRally,
        Self::EmoteBowRespect,
        Self::EmoteWeaponInspect,
        Self::EmoteWarhornCall,
        Self::SummonSentryAim,
        Self::SummonColossusSlam,
        Self::TowerOverchargePulse,
    ];

    pub const fn meta(self) -> MotionMeta {
        use MotionCategory as C;
        const fn m(
            zh: &'static str,
            en: &'static str,
            category: MotionCategory,
            priority: u8,
            duration: f32,
            looping: bool,
            show_badge: bool,
        ) -> MotionMeta {
            MotionMeta {
                zh,
                en,
                category,
                priority,
                duration,
                looping,
                show_badge,
            }
        }
        match self {
            // Locomotion & Stances
            Self::IdleReady => m(
                "待机警戒",
                "Ready Idle",
                C::Locomotion,
                10,
                1.8,
                true,
                false,
            ),
            Self::IdleCombatGuard => m(
                "战斗持架",
                "Combat Guard",
                C::Locomotion,
                12,
                1.4,
                true,
                false,
            ),
            Self::IdleExhaustedLowHp => m(
                "残血喘息",
                "Low-HP Exhausted",
                C::Locomotion,
                18,
                1.3,
                true,
                true,
            ),
            Self::WalkMarch => m(
                "行军步进",
                "March Walk",
                C::Locomotion,
                14,
                0.72,
                true,
                false,
            ),
            Self::RunSprint => m(
                "疾驰冲刺",
                "Sprint Run",
                C::Locomotion,
                16,
                0.48,
                true,
                false,
            ),
            Self::StrafeLeft => m(
                "左侧滑步",
                "Strafe Left",
                C::Locomotion,
                16,
                0.52,
                true,
                false,
            ),
            Self::StrafeRight => m(
                "右侧滑步",
                "Strafe Right",
                C::Locomotion,
                16,
                0.52,
                true,
                false,
            ),
            Self::BackpedalRetreat => m(
                "后撤警戒",
                "Backpedal",
                C::Locomotion,
                16,
                0.58,
                true,
                false,
            ),
            Self::TurnPivotLeft => m(
                "左转轴步",
                "Pivot Left",
                C::Locomotion,
                15,
                0.24,
                false,
                false,
            ),
            Self::TurnPivotRight => m(
                "右转轴步",
                "Pivot Right",
                C::Locomotion,
                15,
                0.24,
                false,
                false,
            ),
            Self::StopSkidRecover => m(
                "急停刹步",
                "Skid-Stop",
                C::Locomotion,
                28,
                0.30,
                false,
                true,
            ),
            Self::LandingCrouch => m(
                "落地缓冲",
                "Landing Crouch",
                C::Locomotion,
                26,
                0.24,
                false,
                false,
            ),

            // Charge, Windup & Telegraphs
            Self::ChargeHoldLight => m(
                "轻蓄聚力",
                "Light Charge",
                C::ChargeTelegraph,
                46,
                0.55,
                true,
                true,
            ),
            Self::ChargeHoldHeavy => m(
                "重蓄满弓",
                "Heavy Charge",
                C::ChargeTelegraph,
                52,
                0.70,
                true,
                true,
            ),
            Self::ChargePeakOverload => m(
                "极意过载",
                "Overload Peak",
                C::ChargeTelegraph,
                56,
                0.50,
                true,
                true,
            ),
            Self::SpellChannelLoop => m(
                "法术引导",
                "Spell Channel",
                C::ChargeTelegraph,
                48,
                0.80,
                true,
                true,
            ),
            Self::BowDrawHold => m(
                "拉弦瞄准",
                "Bow Draw",
                C::ChargeTelegraph,
                45,
                0.55,
                true,
                true,
            ),
            Self::CrossbowLockAim => m(
                "弩机锁定",
                "Crossbow Lock",
                C::ChargeTelegraph,
                45,
                0.45,
                true,
                true,
            ),
            Self::HeavyOverheadWindup => m(
                "举刃蓄劈",
                "Overhead Windup",
                C::ChargeTelegraph,
                54,
                0.42,
                false,
                true,
            ),
            Self::ShieldBraceGuard => m(
                "盾阵格挡",
                "Shield Brace",
                C::ChargeTelegraph,
                50,
                0.65,
                true,
                true,
            ),
            Self::UltimateGatherPower => m(
                "终结蓄能",
                "Ultimate Charge",
                C::ChargeTelegraph,
                74,
                0.55,
                false,
                true,
            ),
            Self::TelegraphGroundSlam => m(
                "震地前摇",
                "Slam Telegraph",
                C::ChargeTelegraph,
                72,
                0.85,
                false,
                true,
            ),

            // Class Attacks & 3-Hit Combos
            Self::SwordCombo1Slash => m(
                "剑势一斩",
                "Sword Slash I",
                C::ClassAttackCombo,
                58,
                0.28,
                false,
                false,
            ),
            Self::SwordCombo2Cross => m(
                "剑势二交",
                "Sword Cross II",
                C::ClassAttackCombo,
                59,
                0.28,
                false,
                false,
            ),
            Self::SwordCombo3Finisher => m(
                "剑势三破",
                "Sword Finisher III",
                C::ClassAttackCombo,
                62,
                0.36,
                false,
                true,
            ),
            Self::DaggerCombo1Flick => m(
                "影刃一闪",
                "Dagger Flick I",
                C::ClassAttackCombo,
                58,
                0.20,
                false,
                false,
            ),
            Self::DaggerCombo2Twin => m(
                "影刃双割",
                "Dagger Twin II",
                C::ClassAttackCombo,
                59,
                0.22,
                false,
                false,
            ),
            Self::DaggerCombo3Spin => m(
                "影刃旋杀",
                "Dagger Spin III",
                C::ClassAttackCombo,
                62,
                0.28,
                false,
                true,
            ),
            Self::AxeCombo1Cleave => m(
                "战斧一劈",
                "Axe Cleave I",
                C::ClassAttackCombo,
                58,
                0.34,
                false,
                false,
            ),
            Self::AxeCombo2Rend => m(
                "战斧二裂",
                "Axe Rend II",
                C::ClassAttackCombo,
                59,
                0.34,
                false,
                false,
            ),
            Self::AxeCombo3Crush => m(
                "战斧三崩",
                "Axe Crush III",
                C::ClassAttackCombo,
                63,
                0.42,
                false,
                true,
            ),
            Self::CrossbowCombo1Snap => m(
                "连弩点射",
                "Bolt Snap I",
                C::ClassAttackCombo,
                57,
                0.22,
                false,
                false,
            ),
            Self::CrossbowCombo2Burst => m(
                "连弩速射",
                "Bolt Burst II",
                C::ClassAttackCombo,
                58,
                0.24,
                false,
                false,
            ),
            Self::CrossbowCombo3Pierce => m(
                "穿心重矢",
                "Piercing Shot III",
                C::ClassAttackCombo,
                61,
                0.30,
                false,
                true,
            ),
            Self::StormCastCombo1 => m(
                "雷霆初引",
                "Storm Cast I",
                C::ClassAttackCombo,
                57,
                0.24,
                false,
                false,
            ),
            Self::StormCastCombo2 => m(
                "雷弧连发",
                "Storm Cast II",
                C::ClassAttackCombo,
                58,
                0.26,
                false,
                false,
            ),
            Self::StormCastCombo3 => m(
                "天雷震落",
                "Storm Surge III",
                C::ClassAttackCombo,
                61,
                0.34,
                false,
                true,
            ),
            Self::FrostArcaneCast1 => m(
                "霜奥凝射",
                "Frost Arcane",
                C::ClassAttackCombo,
                58,
                0.28,
                false,
                false,
            ),
            Self::SpiritStarCast2 => m(
                "灵星齐放",
                "Spirit Star Volley",
                C::ClassAttackCombo,
                59,
                0.28,
                false,
                false,
            ),
            Self::UltimateUnleashBurst => m(
                "绝技释放",
                "Ultimate Unleash",
                C::ClassAttackCombo,
                78,
                0.58,
                false,
                true,
            ),

            // Evades, Dashes & Recovery
            Self::DodgeRollForward => m(
                "前翻闪避",
                "Forward Roll",
                C::EvadeRecovery,
                70,
                0.32,
                false,
                false,
            ),
            Self::DodgeDashBackstep => m(
                "后撤疾闪",
                "Backstep Dash",
                C::EvadeRecovery,
                70,
                0.28,
                false,
                false,
            ),
            Self::DodgeSidestepLeft => m(
                "左侧疾闪",
                "Sidestep Left",
                C::EvadeRecovery,
                70,
                0.28,
                false,
                false,
            ),
            Self::DodgeSidestepRight => m(
                "右侧疾闪",
                "Sidestep Right",
                C::EvadeRecovery,
                70,
                0.28,
                false,
                false,
            ),
            Self::PhaseBlinkOut => m(
                "相位隐遁",
                "Phase Blink Out",
                C::EvadeRecovery,
                72,
                0.18,
                false,
                false,
            ),
            Self::PhaseBlinkIn => m(
                "相位现身",
                "Phase Blink In",
                C::EvadeRecovery,
                72,
                0.20,
                false,
                false,
            ),
            Self::DodgeLandRecover => m(
                "闪避收势",
                "Evade Recover",
                C::EvadeRecovery,
                40,
                0.20,
                false,
                false,
            ),
            Self::VaultLeapForward => m(
                "跃步突进",
                "Vault Leap",
                C::EvadeRecovery,
                68,
                0.34,
                false,
                false,
            ),
            Self::CombatSlideCancel => m(
                "滑铲取消",
                "Slide Cancel",
                C::EvadeRecovery,
                68,
                0.28,
                false,
                false,
            ),
            Self::GuardBreakRecover => m(
                "破防回气",
                "Guard Recover",
                C::EvadeRecovery,
                66,
                0.45,
                false,
                true,
            ),

            // Directional Hitstun, Stagger & Knockback
            Self::HitReactFront => m(
                "正面受击",
                "Front Hitstun",
                C::HitstunKnockback,
                80,
                0.24,
                false,
                false,
            ),
            Self::HitReactBack => m(
                "背后受击",
                "Back Hitstun",
                C::HitstunKnockback,
                80,
                0.24,
                false,
                false,
            ),
            Self::HitReactLeft => m(
                "左侧受击",
                "Left Hitstun",
                C::HitstunKnockback,
                80,
                0.24,
                false,
                false,
            ),
            Self::HitReactRight => m(
                "右侧受击",
                "Right Hitstun",
                C::HitstunKnockback,
                80,
                0.24,
                false,
                false,
            ),
            Self::HitReactHeavy => m(
                "重创僵直",
                "Heavy Hitstun",
                C::HitstunKnockback,
                84,
                0.34,
                false,
                true,
            ),
            Self::StaggerKneel => m(
                "失衡单膝",
                "Stagger Kneel",
                C::HitstunKnockback,
                86,
                0.55,
                false,
                true,
            ),
            Self::StaggerDazeLoop => m(
                "眩晕晃神",
                "Stagger Daze",
                C::HitstunKnockback,
                85,
                0.75,
                true,
                true,
            ),
            Self::KnockbackLaunch => m(
                "击飞掀翻",
                "Knockback Launch",
                C::HitstunKnockback,
                88,
                0.42,
                false,
                true,
            ),
            Self::KnockbackSkid => m(
                "击退滑行",
                "Knockback Skid",
                C::HitstunKnockback,
                82,
                0.32,
                false,
                false,
            ),
            Self::GetupRecovery => m(
                "起身整备",
                "Getup Recovery",
                C::HitstunKnockback,
                76,
                0.38,
                false,
                false,
            ),

            // Status Ailments
            Self::AilmentBurning => m(
                "灼烧痛楚",
                "Burning",
                C::StatusAilment,
                64,
                0.60,
                true,
                true,
            ),
            Self::AilmentChilled => m(
                "寒霜战栗",
                "Chilled",
                C::StatusAilment,
                62,
                0.85,
                true,
                true,
            ),
            Self::AilmentFrozenRigid => m(
                "冰封僵结",
                "Frozen Solid",
                C::StatusAilment,
                90,
                1.00,
                true,
                true,
            ),
            Self::AilmentStunnedDaze => m(
                "眩晕失神",
                "Stunned",
                C::StatusAilment,
                89,
                0.75,
                true,
                true,
            ),
            Self::AilmentBleeding => m(
                "流血虚弱",
                "Bleeding",
                C::StatusAilment,
                60,
                0.70,
                true,
                true,
            ),
            Self::AilmentVulnerable => m(
                "易伤破甲",
                "Vulnerable",
                C::StatusAilment,
                58,
                0.75,
                true,
                true,
            ),
            Self::AilmentSlowedMud => {
                m("迟缓受阻", "Slowed", C::StatusAilment, 56, 0.90, true, true)
            }
            Self::AilmentJailedRooted => m(
                "禁锢缠缚",
                "Rooted / Jailed",
                C::StatusAilment,
                84,
                0.70,
                true,
                true,
            ),
            Self::AilmentInfectedBlight => m(
                "瘟疫侵蚀",
                "Blighted",
                C::StatusAilment,
                63,
                0.72,
                true,
                true,
            ),
            Self::AilmentExhaustedGasp => m(
                "力竭喘息",
                "Exhausted",
                C::StatusAilment,
                55,
                1.10,
                true,
                true,
            ),

            // Buffs, Consumables, Interactions, Level-Up & Death
            Self::BuffBattleCry => m(
                "战吼振奋",
                "Battle Cry",
                C::BuffInteraction,
                65,
                0.55,
                false,
                true,
            ),
            Self::BuffAuraPulse => m(
                "光环涌动",
                "Aura Pulse",
                C::BuffInteraction,
                60,
                0.45,
                false,
                false,
            ),
            Self::ConsumeElixirDrink => m(
                "饮药回血",
                "Drink Elixir",
                C::BuffInteraction,
                62,
                0.50,
                false,
                true,
            ),
            Self::InteractCampInspect => m(
                "侦查营地",
                "Inspect Camp",
                C::BuffInteraction,
                40,
                0.50,
                false,
                false,
            ),
            Self::InteractForgeCraft => m(
                "熔炉锻造",
                "Forge Craft",
                C::BuffInteraction,
                44,
                0.55,
                false,
                false,
            ),
            Self::InteractGateRepair => m(
                "修筑城门",
                "Repair Gate",
                C::BuffInteraction,
                44,
                0.60,
                false,
                false,
            ),
            Self::LevelUpAscend => m(
                "升级耀光",
                "Level Up",
                C::BuffInteraction,
                73,
                0.62,
                false,
                true,
            ),
            Self::HeroRecruitAwaken => m(
                "英雄觉醒",
                "Hero Awaken",
                C::BuffInteraction,
                75,
                0.65,
                false,
                true,
            ),
            Self::BondResonanceCheer => m(
                "羁绊共鸣",
                "Bond Resonance",
                C::BuffInteraction,
                66,
                0.55,
                false,
                true,
            ),
            Self::ReviveRiseUp => m(
                "重返战场",
                "Revive Rise",
                C::BuffInteraction,
                94,
                0.65,
                false,
                true,
            ),
            Self::DeathCollapseForward => m(
                "力竭倒地",
                "Death Collapse",
                C::BuffInteraction,
                100,
                0.75,
                false,
                true,
            ),
            Self::DeathCollapseBack => m(
                "重伤仰倒",
                "Death Fall Back",
                C::BuffInteraction,
                100,
                0.75,
                false,
                true,
            ),

            // Enemy Specialized Motions
            Self::EnemyLungeBite => m(
                "凶兽扑咬",
                "Enemy Lunge",
                C::EnemySpecial,
                64,
                0.34,
                false,
                false,
            ),
            Self::EnemyShieldBash => m(
                "重盾冲撞",
                "Shield Bash",
                C::EnemySpecial,
                66,
                0.38,
                false,
                false,
            ),
            Self::EnemyRangedDrawFire => m(
                "毒箭齐射",
                "Ranged Volley",
                C::EnemySpecial,
                62,
                0.36,
                false,
                false,
            ),
            Self::EnemyBurrowSubmerge => m(
                "遁地潜行",
                "Burrow Dive",
                C::EnemySpecial,
                72,
                0.42,
                false,
                false,
            ),
            Self::EnemyBurrowEmerge => m(
                "破土奇袭",
                "Burrow Ambush",
                C::EnemySpecial,
                74,
                0.40,
                false,
                false,
            ),
            Self::EnemyChantHaste => m(
                "狂热吟唱",
                "Haste Chant",
                C::EnemySpecial,
                60,
                0.55,
                false,
                false,
            ),
            Self::EnemyHealPulse => m(
                "战地愈合",
                "Heal Pulse",
                C::EnemySpecial,
                60,
                0.50,
                false,
                false,
            ),
            Self::EnemyVolatileSwell => m(
                "自爆膨胀",
                "Volatile Swell",
                C::EnemySpecial,
                87,
                0.95,
                true,
                false,
            ),
            Self::EnemySiegeLob => m(
                "攻城抛投",
                "Siege Lob",
                C::EnemySpecial,
                65,
                0.48,
                false,
                false,
            ),
            Self::EnemySplitSpawn => m(
                "分裂增殖",
                "Split Spawn",
                C::EnemySpecial,
                70,
                0.35,
                false,
                false,
            ),

            // Boss Specialized Motions
            Self::BossEntranceRoar => m(
                "首领咆哮",
                "Boss Entrance Roar",
                C::BossSpecial,
                92,
                1.05,
                false,
                false,
            ),
            Self::BossPhaseShiftErupt => m(
                "狂暴转阶段",
                "Boss Phase Shift",
                C::BossSpecial,
                95,
                1.10,
                false,
                false,
            ),
            Self::BossHeavyOverheadSlam => m(
                "灭世重砸",
                "Boss Overhead Slam",
                C::BossSpecial,
                82,
                0.78,
                false,
                false,
            ),
            Self::BossSweepingCleave => m(
                "横扫千军",
                "Boss Sweep Cleave",
                C::BossSpecial,
                80,
                0.68,
                false,
                false,
            ),
            Self::BossLaserBeamFocus => m(
                "射线聚焦",
                "Boss Beam Focus",
                C::BossSpecial,
                84,
                0.90,
                false,
                false,
            ),
            Self::BossShockwaveStomp => m(
                "震荡重踏",
                "Boss Shockwave",
                C::BossSpecial,
                83,
                0.72,
                false,
                false,
            ),
            Self::BossSummonCall => m(
                "号令召唤",
                "Boss Summon Call",
                C::BossSpecial,
                78,
                0.80,
                false,
                false,
            ),
            Self::BossEnrageHowl => m(
                "狂怒震嚎",
                "Boss Enrage Howl",
                C::BossSpecial,
                91,
                0.95,
                false,
                false,
            ),
            Self::BossStaggerBroken => m(
                "首领破防",
                "Boss Staggered",
                C::BossSpecial,
                93,
                0.85,
                false,
                false,
            ),
            Self::BossDefeatFall => m(
                "首领陨落",
                "Boss Defeat Fall",
                C::BossSpecial,
                100,
                1.20,
                false,
                false,
            ),

            // Ally, Summon, Tower & Co-op Emotes
            Self::EmoteSalute => m("致敬军礼", "Salute", C::AllyEmote, 28, 1.05, false, true),
            Self::EmoteBattleTaunt => m(
                "挑衅迎敌",
                "Battle Taunt",
                C::AllyEmote,
                28,
                1.15,
                false,
                true,
            ),
            Self::EmoteVictoryFlourish => m(
                "胜利挥舞",
                "Victory Flourish",
                C::AllyEmote,
                28,
                1.20,
                false,
                true,
            ),
            Self::EmoteBeckonRally => m(
                "集结号令",
                "Rally Beckon",
                C::AllyEmote,
                28,
                1.10,
                false,
                true,
            ),
            Self::EmoteBowRespect => m("躬身致意", "Bow", C::AllyEmote, 28, 1.15, false, true),
            Self::EmoteWeaponInspect => m(
                "擦拭兵刃",
                "Inspect Weapon",
                C::AllyEmote,
                28,
                1.30,
                false,
                true,
            ),
            Self::EmoteWarhornCall => m(
                "吹响战角",
                "Warhorn Call",
                C::AllyEmote,
                28,
                1.25,
                false,
                true,
            ),
            Self::SummonSentryAim => m(
                "哨兵瞄准",
                "Sentry Aim",
                C::AllyEmote,
                50,
                0.35,
                false,
                false,
            ),
            Self::SummonColossusSlam => m(
                "巨像震击",
                "Colossus Slam",
                C::AllyEmote,
                56,
                0.50,
                false,
                false,
            ),
            Self::TowerOverchargePulse => m(
                "哨塔过载",
                "Tower Overcharge",
                C::AllyEmote,
                52,
                0.45,
                false,
                false,
            ),
        }
    }

    pub const fn zh_name(self) -> &'static str {
        self.meta().zh
    }

    pub const fn en_name(self) -> &'static str {
        self.meta().en
    }

    pub fn bilingual_badge(self, language: GameLanguage) -> String {
        let meta = self.meta();
        match language {
            GameLanguage::SimplifiedChinese => format!("{} · {}", meta.zh, meta.en),
            GameLanguage::English => format!("{} · {}", meta.en, meta.zh),
        }
    }
}

/// Non-linear AAA anticipation -> explosive release -> overshoot recovery curve.
/// Returns a signed energy scalar in roughly `[-0.38, +1.18]` over `t in [0, 1]`.
pub fn anticipation_release_overshoot(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.24 {
        // Coil back into anticipation
        let u = t / 0.24;
        -0.36 * (u * FRAC_PI_2).sin()
    } else if t < 0.50 {
        // Explosive snap from anticipation to overshoot peak
        let u = (t - 0.24) / 0.26;
        let snap = 1.0 - (1.0 - u).powi(3);
        -0.36 + 1.52 * snap
    } else {
        // Critically damped elastic settle from +1.16 back to 0.0
        let u = (t - 0.50) / 0.50;
        let decay = (-4.2 * u).exp();
        let ring = (u * PI * 1.45).cos();
        1.16 * decay * ring * (1.0 - u)
    }
}

/// Critically-damped exponential blend factor for frame-rate independent pose smoothing.
pub fn critically_damped_alpha(dt: f32, stiffness: f32) -> f32 {
    if dt <= 0.0 || stiffness <= 0.0 {
        return 0.0;
    }
    1.0 - (-stiffness * dt).exp()
}

/// Articulated full-body procedural pose delta (local to actor facing).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionPose {
    pub root_offset: Vec3,
    pub torso_pitch: f32,
    pub torso_yaw: f32,
    pub torso_roll: f32,
    pub scale_mul: Vec3,
    pub weapon_swing_arc: f32,
}

impl Default for MotionPose {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl MotionPose {
    pub const IDENTITY: Self = Self {
        root_offset: Vec3::ZERO,
        torso_pitch: 0.0,
        torso_yaw: 0.0,
        torso_roll: 0.0,
        scale_mul: Vec3::ONE,
        weapon_swing_arc: 0.0,
    };

    pub fn blend_toward(&mut self, target: Self, alpha: f32) {
        let a = alpha.clamp(0.0, 1.0);
        self.root_offset = self.root_offset.lerp(target.root_offset, a);
        self.torso_pitch += (target.torso_pitch - self.torso_pitch) * a;
        self.torso_yaw += (target.torso_yaw - self.torso_yaw) * a;
        self.torso_roll += (target.torso_roll - self.torso_roll) * a;
        self.scale_mul = self.scale_mul.lerp(target.scale_mul, a);
        self.weapon_swing_arc += (target.weapon_swing_arc - self.weapon_swing_arc) * a;
    }

    pub fn rotation(self) -> Quat {
        Quat::from_euler(
            EulerRot::YXZ,
            self.torso_yaw,
            self.torso_pitch,
            self.torso_roll,
        )
    }

    /// Evaluates a distinct, anatomically expressive procedural pose for every `MotionId`.
    pub fn sample(id: MotionId, phase: f32, elapsed: f32, intensity: f32) -> Self {
        let t = phase.clamp(0.0, 1.0);
        let power = intensity.clamp(0.25, 2.0);
        let snap = anticipation_release_overshoot(t) * power;
        let env = (t * PI).sin().max(0.0);
        let pulse = (elapsed * TAU).sin();
        let fast = (elapsed * TAU * 4.5).sin();
        let mut p = Self::IDENTITY;

        match id {
            // --- Locomotion & Stances ---
            MotionId::IdleReady => {
                let breathe = (t * TAU).sin();
                p.root_offset.y = breathe * 0.016;
                p.scale_mul = Vec3::new(1.0 - breathe * 0.01, 1.0 + breathe * 0.015, 1.0);
            }
            MotionId::IdleCombatGuard => {
                let breathe = (t * TAU).sin();
                p.root_offset = Vec3::new(0.0, -0.045 + breathe * 0.015, -0.03);
                p.torso_pitch = -0.10;
                p.torso_yaw = 0.14;
                p.scale_mul = Vec3::new(1.03, 0.96, 1.03);
            }
            MotionId::IdleExhaustedLowHp | MotionId::AilmentExhaustedGasp => {
                let gasp = (t * TAU).sin();
                p.root_offset = Vec3::new(0.0, -0.12 - gasp.abs() * 0.03, -0.05);
                p.torso_pitch = -0.26 - gasp * 0.07;
                p.torso_roll = (t * TAU * 0.5).sin() * 0.06;
                p.scale_mul = Vec3::new(1.04, 0.92 + gasp * 0.03, 1.04);
            }
            MotionId::WalkMarch => {
                let stride = (t * TAU).sin();
                p.root_offset.y = stride.abs() * 0.04;
                p.torso_pitch = -0.07;
                p.torso_roll = stride * 0.045;
                p.torso_yaw = -stride * 0.04;
            }
            MotionId::RunSprint => {
                let stride = (t * TAU).sin();
                p.root_offset = Vec3::new(0.0, stride.abs() * 0.075, -0.06);
                p.torso_pitch = -0.18;
                p.torso_roll = stride * 0.08;
                p.torso_yaw = -stride * 0.06;
                p.scale_mul = Vec3::new(0.98, 1.03, 1.02);
            }
            MotionId::StrafeLeft => {
                p.root_offset.y = (t * TAU).sin().abs() * 0.05;
                p.torso_roll = 0.16;
                p.torso_yaw = 0.14;
            }
            MotionId::StrafeRight => {
                p.root_offset.y = (t * TAU).sin().abs() * 0.05;
                p.torso_roll = -0.16;
                p.torso_yaw = -0.14;
            }
            MotionId::BackpedalRetreat => {
                p.root_offset = Vec3::new(0.0, -0.03 + (t * TAU).sin().abs() * 0.035, 0.06);
                p.torso_pitch = 0.12;
            }
            MotionId::TurnPivotLeft => {
                p.torso_yaw = 0.28 * env;
                p.torso_roll = 0.12 * env;
            }
            MotionId::TurnPivotRight => {
                p.torso_yaw = -0.28 * env;
                p.torso_roll = -0.12 * env;
            }
            MotionId::StopSkidRecover => {
                // Lean sharply back against prior sprint momentum, crouch low, then pop upright
                let brace = (1.0 - t).powf(1.4) * power;
                p.root_offset = Vec3::new(0.0, -0.14 * brace, 0.16 * brace);
                p.torso_pitch = 0.29 * brace;
                p.torso_roll = -0.11 * brace;
                p.scale_mul = Vec3::new(1.0 + 0.08 * brace, 1.0 - 0.11 * brace, 1.0 + 0.08 * brace);
            }
            MotionId::LandingCrouch => {
                p.root_offset.y = -0.16 * env;
                p.torso_pitch = -0.16 * env;
                p.scale_mul = Vec3::new(1.0 + 0.11 * env, 1.0 - 0.14 * env, 1.0 + 0.11 * env);
            }

            // --- Charge, Windup & Telegraphs ---
            MotionId::ChargeHoldLight => {
                let hum = (elapsed * 22.0).sin() * 0.015;
                p.root_offset = Vec3::new(hum, -0.09, 0.08);
                p.torso_pitch = 0.14;
                p.torso_yaw = -0.24;
                p.scale_mul = Vec3::new(1.05, 0.94, 1.05);
            }
            MotionId::ChargeHoldHeavy => {
                let hum = (elapsed * 32.0).sin() * 0.028;
                p.root_offset = Vec3::new(hum, -0.16, 0.14);
                p.torso_pitch = 0.24;
                p.torso_yaw = -0.38;
                p.torso_roll = -0.10;
                p.scale_mul = Vec3::new(1.10, 0.89, 1.10);
            }
            MotionId::ChargePeakOverload => {
                let hum = (elapsed * 48.0).sin() * 0.036;
                p.root_offset = Vec3::new(hum, -0.20, 0.17);
                p.torso_pitch = 0.30;
                p.torso_yaw = -0.44;
                p.torso_roll = -0.14 + hum * 1.4;
                p.scale_mul = Vec3::new(1.14, 0.86, 1.14);
            }
            MotionId::SpellChannelLoop => {
                let float_y = 0.08 + (elapsed * 6.0).sin() * 0.04;
                p.root_offset = Vec3::new(0.0, float_y, 0.04);
                p.torso_pitch = 0.16;
                p.scale_mul = Vec3::new(0.96, 1.06, 0.96);
            }
            MotionId::BowDrawHold | MotionId::CrossbowLockAim => {
                p.root_offset = Vec3::new(0.0, -0.07, 0.06);
                p.torso_yaw = -0.34;
                p.torso_pitch = 0.05;
                p.scale_mul = Vec3::new(1.03, 0.96, 1.03);
            }
            MotionId::HeavyOverheadWindup => {
                let ramp = t.powf(0.75);
                p.root_offset = Vec3::new(0.0, 0.08 * ramp, 0.14 * ramp);
                p.torso_pitch = 0.36 * ramp;
                p.torso_yaw = -0.28 * ramp;
                p.scale_mul = Vec3::new(0.95, 1.08 * ramp + (1.0 - ramp), 0.95);
            }
            MotionId::ShieldBraceGuard => {
                p.root_offset = Vec3::new(0.0, -0.12, 0.05);
                p.torso_pitch = -0.15;
                p.torso_yaw = 0.26;
                p.scale_mul = Vec3::new(1.09, 0.91, 1.09);
            }
            MotionId::UltimateGatherPower => {
                let rise = (t * FRAC_PI_2).sin();
                p.root_offset = Vec3::new((elapsed * 42.0).sin() * 0.025, 0.16 * rise, 0.08 * rise);
                p.torso_pitch = 0.26 * rise;
                p.scale_mul = Vec3::new(1.0 + 0.08 * rise, 1.0 + 0.10 * rise, 1.0 + 0.08 * rise);
            }
            MotionId::TelegraphGroundSlam => {
                let coil = (t * FRAC_PI_2).sin();
                p.root_offset = Vec3::new(0.0, 0.18 * coil, 0.12 * coil);
                p.torso_pitch = 0.34 * coil;
                p.scale_mul = Vec3::new(1.06, 1.12 * coil + (1.0 - coil), 1.06);
            }

            // --- Class Attacks & 3-Hit Combos ---
            MotionId::SwordCombo1Slash => {
                p.root_offset = Vec3::new(0.0, -0.05 * snap.max(0.0), -0.22 * snap);
                p.torso_yaw = 0.54 * snap;
                p.torso_pitch = -0.18 * snap;
                p.torso_roll = -0.14 * snap;
                p.weapon_swing_arc = -0.38;
            }
            MotionId::SwordCombo2Cross => {
                p.root_offset = Vec3::new(0.0, -0.06 * snap.max(0.0), -0.26 * snap);
                p.torso_yaw = -0.60 * snap;
                p.torso_pitch = -0.22 * snap;
                p.torso_roll = 0.18 * snap;
                p.weapon_swing_arc = 0.42;
            }
            MotionId::SwordCombo3Finisher => {
                p.root_offset = Vec3::new(0.0, 0.10 * env - 0.12 * snap.max(0.0), -0.38 * snap);
                p.torso_yaw = 0.78 * snap;
                p.torso_pitch = -0.32 * snap;
                p.scale_mul = Vec3::new(
                    1.0 + 0.10 * env,
                    1.0 - 0.08 * snap.max(0.0),
                    1.0 + 0.10 * env,
                );
                p.weapon_swing_arc = 0.0;
            }
            MotionId::DaggerCombo1Flick => {
                p.root_offset = Vec3::new(0.06 * snap, -0.04, -0.20 * snap);
                p.torso_yaw = 0.42 * snap;
                p.torso_pitch = -0.20 * snap;
                p.weapon_swing_arc = -0.45;
            }
            MotionId::DaggerCombo2Twin => {
                p.root_offset = Vec3::new(-0.08 * snap, -0.05, -0.24 * snap);
                p.torso_yaw = -0.52 * snap;
                p.torso_roll = 0.20 * snap;
                p.weapon_swing_arc = 0.45;
            }
            MotionId::DaggerCombo3Spin => {
                p.root_offset = Vec3::new(0.0, 0.08 * env, -0.32 * env);
                p.torso_yaw = t * TAU * 0.85;
                p.torso_pitch = -0.24 * env;
                p.weapon_swing_arc = 0.0;
            }
            MotionId::AxeCombo1Cleave => {
                p.root_offset = Vec3::new(0.0, -0.07 * snap.max(0.0), -0.26 * snap);
                p.torso_yaw = 0.66 * snap;
                p.torso_pitch = -0.24 * snap;
                p.torso_roll = -0.18 * snap;
                p.weapon_swing_arc = -0.32;
            }
            MotionId::AxeCombo2Rend => {
                p.root_offset = Vec3::new(0.0, -0.08 * snap.max(0.0), -0.30 * snap);
                p.torso_yaw = -0.70 * snap;
                p.torso_pitch = -0.26 * snap;
                p.torso_roll = 0.22 * snap;
                p.weapon_swing_arc = 0.36;
            }
            MotionId::AxeCombo3Crush => {
                p.root_offset = Vec3::new(0.0, 0.16 * env - 0.16 * snap.max(0.0), -0.40 * snap);
                p.torso_pitch = -0.42 * snap;
                p.torso_yaw = 0.25 * snap;
                p.scale_mul = Vec3::new(
                    1.0 + 0.14 * snap.max(0.0),
                    1.0 - 0.14 * snap.max(0.0),
                    1.0 + 0.14 * snap.max(0.0),
                );
                p.weapon_swing_arc = 0.0;
            }
            MotionId::CrossbowCombo1Snap
            | MotionId::CrossbowCombo2Burst
            | MotionId::CrossbowCombo3Pierce => {
                let kick = match id {
                    MotionId::CrossbowCombo1Snap => 1.0,
                    MotionId::CrossbowCombo2Burst => 1.2,
                    _ => 1.55,
                };
                p.root_offset = Vec3::new(0.0, -0.03 * kick * env, 0.14 * kick * snap.max(0.0));
                p.torso_pitch = 0.16 * kick * snap;
                p.torso_yaw = -0.18 * kick * env;
                p.weapon_swing_arc = match id {
                    MotionId::CrossbowCombo1Snap => -0.25,
                    MotionId::CrossbowCombo2Burst => 0.25,
                    _ => 0.0,
                };
            }
            MotionId::StormCastCombo1
            | MotionId::StormCastCombo2
            | MotionId::StormCastCombo3
            | MotionId::FrostArcaneCast1
            | MotionId::SpiritStarCast2 => {
                let step = match id {
                    MotionId::StormCastCombo3 => 1.45,
                    MotionId::StormCastCombo2 | MotionId::SpiritStarCast2 => 1.15,
                    _ => 1.0,
                };
                p.root_offset = Vec3::new(0.0, 0.08 * step * env, -0.16 * step * snap);
                p.torso_pitch = -0.20 * step * snap;
                p.torso_yaw =
                    (if matches!(id, MotionId::StormCastCombo2 | MotionId::SpiritStarCast2) {
                        -0.36
                    } else {
                        0.36
                    }) * snap;
                p.weapon_swing_arc = if matches!(id, MotionId::StormCastCombo2) {
                    0.35
                } else if matches!(id, MotionId::StormCastCombo3) {
                    0.0
                } else {
                    -0.35
                };
            }
            MotionId::UltimateUnleashBurst => {
                p.root_offset = Vec3::new(0.0, 0.22 * env, -0.34 * snap);
                p.torso_pitch = -0.34 * snap;
                p.scale_mul = Vec3::new(1.0 + 0.14 * env, 1.0 + 0.12 * env, 1.0 + 0.14 * env);
            }

            // --- Evades, Dashes & Recovery ---
            MotionId::DodgeRollForward => {
                p.root_offset = Vec3::new(0.0, 0.14 * env - 0.06, -0.20 * env);
                p.torso_pitch = -0.62 * env;
                p.scale_mul = Vec3::new(0.92, 0.88 + 0.12 * (1.0 - env), 1.12);
            }
            MotionId::DodgeDashBackstep => {
                p.root_offset = Vec3::new(0.0, 0.10 * env, 0.22 * env);
                p.torso_pitch = 0.36 * env;
            }
            MotionId::DodgeSidestepLeft => {
                p.root_offset = Vec3::new(-0.18 * env, 0.08 * env, 0.0);
                p.torso_roll = 0.42 * env;
            }
            MotionId::DodgeSidestepRight => {
                p.root_offset = Vec3::new(0.18 * env, 0.08 * env, 0.0);
                p.torso_roll = -0.42 * env;
            }
            MotionId::PhaseBlinkOut => {
                p.scale_mul = Vec3::new(0.65, 1.32, 0.65).lerp(Vec3::ONE, 1.0 - env);
            }
            MotionId::PhaseBlinkIn => {
                p.scale_mul = Vec3::new(1.22, 0.84, 1.22).lerp(Vec3::ONE, t);
            }
            MotionId::DodgeLandRecover | MotionId::CombatSlideCancel => {
                p.root_offset = Vec3::new(0.0, -0.14 * env, -0.10 * env);
                p.torso_pitch = -0.22 * env;
            }
            MotionId::VaultLeapForward => {
                p.root_offset = Vec3::new(0.0, 0.28 * env, -0.18 * env);
                p.torso_pitch = -0.30 * env;
            }
            MotionId::GuardBreakRecover => {
                p.root_offset = Vec3::new(0.0, -0.12 * env, 0.16 * env);
                p.torso_pitch = 0.28 * env;
                p.torso_roll = (t * PI * 3.0).sin() * 0.12 * (1.0 - t);
            }

            // --- Directional Hitstun, Stagger & Knockback ---
            MotionId::HitReactFront => {
                let recoil = env * power;
                p.root_offset = Vec3::new(0.0, -0.04 * recoil, 0.22 * recoil);
                p.torso_pitch = 0.34 * recoil;
                p.scale_mul = Vec3::new(
                    1.0 + 0.09 * recoil,
                    1.0 - 0.09 * recoil,
                    1.0 + 0.06 * recoil,
                );
            }
            MotionId::HitReactBack => {
                let recoil = env * power;
                p.root_offset = Vec3::new(0.0, -0.05 * recoil, -0.22 * recoil);
                p.torso_pitch = -0.36 * recoil;
                p.scale_mul = Vec3::new(
                    1.0 + 0.09 * recoil,
                    1.0 - 0.09 * recoil,
                    1.0 + 0.06 * recoil,
                );
            }
            MotionId::HitReactLeft => {
                let recoil = env * power;
                p.root_offset = Vec3::new(0.20 * recoil, -0.04 * recoil, 0.04 * recoil);
                p.torso_roll = -0.34 * recoil;
                p.torso_yaw = -0.22 * recoil;
            }
            MotionId::HitReactRight => {
                let recoil = env * power;
                p.root_offset = Vec3::new(-0.20 * recoil, -0.04 * recoil, 0.04 * recoil);
                p.torso_roll = 0.34 * recoil;
                p.torso_yaw = 0.22 * recoil;
            }
            MotionId::HitReactHeavy => {
                let recoil = env * power;
                p.root_offset = Vec3::new(0.0, -0.10 * recoil, 0.30 * recoil);
                p.torso_pitch = 0.46 * recoil;
                p.torso_roll = 0.16 * recoil;
                p.scale_mul = Vec3::new(
                    1.0 + 0.14 * recoil,
                    1.0 - 0.16 * recoil,
                    1.0 + 0.14 * recoil,
                );
            }
            MotionId::StaggerKneel | MotionId::BossStaggerBroken => {
                let kneel = if t < 0.25 {
                    t / 0.25
                } else if t < 0.78 {
                    1.0
                } else {
                    1.0 - (t - 0.78) / 0.22
                };
                p.root_offset = Vec3::new(0.0, -0.26 * kneel, 0.10 * kneel);
                p.torso_pitch = -0.38 * kneel;
                p.torso_roll = (elapsed * 9.0).sin() * 0.09 * kneel;
                p.scale_mul = Vec3::new(1.0 + 0.08 * kneel, 1.0 - 0.18 * kneel, 1.0 + 0.08 * kneel);
            }
            MotionId::StaggerDazeLoop => {
                p.root_offset = Vec3::new((elapsed * 5.0).sin() * 0.06, -0.12, 0.0);
                p.torso_pitch = -0.18 + (elapsed * 4.0).cos() * 0.08;
                p.torso_roll = (elapsed * 5.0).sin() * 0.22;
                p.torso_yaw = (elapsed * 3.5).cos() * 0.18;
            }
            MotionId::KnockbackLaunch => {
                p.root_offset = Vec3::new(0.0, 0.32 * env, 0.36 * env);
                p.torso_pitch = 0.58 * env;
            }
            MotionId::KnockbackSkid => {
                let k = (1.0 - t) * power;
                p.root_offset = Vec3::new(0.0, -0.09 * k, 0.24 * k);
                p.torso_pitch = 0.30 * k;
            }
            MotionId::GetupRecovery => {
                let r = 1.0 - t;
                p.root_offset.y = -0.22 * r;
                p.torso_pitch = -0.32 * r;
            }

            // --- Status Ailments (each distinct!) ---
            MotionId::AilmentBurning => {
                // Frantic writhing + rapid torso twist
                p.root_offset = Vec3::new(fast * 0.055, 0.03 * fast.abs(), 0.0);
                p.torso_yaw = fast * 0.26;
                p.torso_roll = (elapsed * 16.0).cos() * 0.16;
                p.torso_pitch = -0.12 + fast * 0.08;
            }
            MotionId::AilmentChilled => {
                // High-frequency shivering crouch
                let shiver = (elapsed * 36.0).sin();
                p.root_offset = Vec3::new(shiver * 0.028, -0.08, 0.0);
                p.torso_pitch = -0.16;
                p.torso_roll = shiver * 0.055;
                p.scale_mul = Vec3::new(0.97, 0.96, 0.97);
            }
            MotionId::AilmentFrozenRigid => {
                // Rigid crystal lock with subtle icy tension
                p.root_offset = Vec3::new(0.0, 0.02, 0.0);
                p.torso_pitch = 0.09;
                p.torso_roll = 0.08;
                p.scale_mul = Vec3::new(1.04, 1.04, 1.04);
            }
            MotionId::AilmentStunnedDaze => {
                // Heavy wobbling daze sway
                p.root_offset = Vec3::new((elapsed * 6.5).sin() * 0.075, -0.14, 0.03);
                p.torso_pitch = -0.22;
                p.torso_roll = (elapsed * 6.5).sin() * 0.26;
                p.torso_yaw = (elapsed * 4.5).cos() * 0.24;
            }
            MotionId::AilmentBleeding => {
                // Clutching side, asymmetrical droop
                p.root_offset = Vec3::new(0.04, -0.10 - pulse.abs() * 0.03, 0.0);
                p.torso_pitch = -0.24;
                p.torso_roll = 0.19;
                p.torso_yaw = -0.14;
            }
            MotionId::AilmentVulnerable => {
                // Exposed open-chest recoil stagger
                p.root_offset = Vec3::new(0.0, -0.05, 0.09);
                p.torso_pitch = 0.22;
                p.torso_roll = (elapsed * 7.0).sin() * 0.08;
                p.scale_mul = Vec3::new(1.06, 0.96, 0.94);
            }
            MotionId::AilmentSlowedMud => {
                // Heavy weighted drag forward
                p.root_offset = Vec3::new(0.0, -0.15, 0.04);
                p.torso_pitch = -0.28;
                p.scale_mul = Vec3::new(1.06, 0.90, 1.06);
            }
            MotionId::AilmentJailedRooted => {
                // Straining upward against binding roots
                let strain = (elapsed * 11.0).sin();
                p.root_offset = Vec3::new(strain * 0.035, -0.18, 0.0);
                p.torso_pitch = 0.18 + strain * 0.07;
                p.torso_yaw = strain * 0.20;
                p.scale_mul = Vec3::new(1.08, 0.88, 1.08);
            }
            MotionId::AilmentInfectedBlight => {
                // Contorted blight hunch + spasmodic twitch
                let twitch = (elapsed * 19.0).sin();
                p.root_offset = Vec3::new(twitch * 0.03, -0.13, -0.02);
                p.torso_pitch = -0.31;
                p.torso_roll = -0.18 + twitch * 0.06;
            }

            // --- Buffs, Consumables, Interactions, Level-Up & Death ---
            MotionId::BuffBattleCry | MotionId::BossEntranceRoar | MotionId::BossEnrageHowl => {
                let roar = env * power;
                p.root_offset = Vec3::new(
                    (elapsed * 45.0).sin() * 0.02 * roar,
                    0.10 * roar,
                    0.08 * roar,
                );
                p.torso_pitch = 0.36 * roar;
                p.scale_mul = Vec3::new(1.0 + 0.14 * roar, 1.0 + 0.12 * roar, 1.0 + 0.14 * roar);
            }
            MotionId::BuffAuraPulse | MotionId::BondResonanceCheer => {
                p.root_offset.y = 0.14 * env;
                p.torso_pitch = 0.18 * env;
                p.scale_mul = Vec3::splat(1.0 + 0.09 * env);
            }
            MotionId::ConsumeElixirDrink => {
                p.torso_pitch = 0.26 * env;
                p.torso_roll = -0.10 * env;
            }
            MotionId::InteractCampInspect
            | MotionId::InteractForgeCraft
            | MotionId::InteractGateRepair => {
                p.root_offset.y = -0.08 * env;
                p.torso_pitch = -0.26 * env;
                p.torso_yaw = (t * TAU).sin() * 0.16;
            }
            MotionId::LevelUpAscend | MotionId::HeroRecruitAwaken | MotionId::ReviveRiseUp => {
                p.root_offset.y = 0.24 * env;
                p.torso_pitch = 0.24 * env;
                p.torso_yaw = (1.0 - t) * 0.45;
                p.scale_mul = Vec3::new(0.95 + 0.12 * env, 1.0 + 0.16 * env, 0.95 + 0.12 * env);
            }
            MotionId::DeathCollapseForward => {
                let fall = t.powf(1.5);
                p.root_offset = Vec3::new(0.0, -0.45 * fall, -0.35 * fall);
                p.torso_pitch = -1.25 * fall;
            }
            MotionId::DeathCollapseBack | MotionId::BossDefeatFall => {
                let fall = t.powf(1.5);
                p.root_offset = Vec3::new(0.0, -0.45 * fall, 0.35 * fall);
                p.torso_pitch = 1.25 * fall;
                p.torso_roll = 0.25 * fall;
            }

            // --- Enemy Specialized Motions ---
            MotionId::EnemyLungeBite => {
                p.root_offset = Vec3::new(0.0, 0.06 * env, -0.30 * snap);
                p.torso_pitch = -0.34 * snap;
            }
            MotionId::EnemyShieldBash => {
                p.root_offset = Vec3::new(0.0, -0.06 * env, -0.26 * snap);
                p.torso_yaw = 0.40 * snap;
                p.scale_mul = Vec3::new(1.0 + 0.10 * env, 0.94, 1.0 + 0.10 * env);
            }
            MotionId::EnemyRangedDrawFire | MotionId::EnemySiegeLob => {
                p.root_offset = Vec3::new(0.0, 0.04 * env, 0.16 * snap.max(0.0));
                p.torso_pitch = 0.24 * snap;
            }
            MotionId::EnemyBurrowSubmerge => {
                p.root_offset.y = -0.48 * t;
                p.torso_yaw = t * TAU;
                p.scale_mul = Vec3::splat(1.0 - 0.35 * t);
            }
            MotionId::EnemyBurrowEmerge => {
                p.root_offset.y = -0.48 * (1.0 - t) + 0.14 * env;
                p.torso_pitch = 0.22 * env;
            }
            MotionId::EnemyChantHaste | MotionId::EnemyHealPulse => {
                p.root_offset.y = 0.10 * env;
                p.torso_pitch = 0.22 * env;
                p.torso_roll = (t * TAU).sin() * 0.14;
            }
            MotionId::EnemyVolatileSwell => {
                let swell = 1.0 + 0.28 * (0.5 + 0.5 * (elapsed * 14.0).sin());
                p.scale_mul = Vec3::splat(swell);
                p.root_offset.x = (elapsed * 44.0).sin() * 0.05;
            }
            MotionId::EnemySplitSpawn => {
                p.scale_mul = Vec3::new(1.0 + 0.24 * env, 1.0 - 0.18 * env, 1.0 + 0.24 * env);
            }

            // --- Boss Specialized Motions ---
            MotionId::BossPhaseShiftErupt => {
                let surge = env * power;
                p.root_offset =
                    Vec3::new((elapsed * 52.0).sin() * 0.045 * surge, 0.26 * surge, 0.0);
                p.torso_pitch = 0.42 * surge;
                p.scale_mul = Vec3::splat(1.0 + 0.20 * surge);
            }
            MotionId::BossHeavyOverheadSlam => {
                p.root_offset = Vec3::new(0.0, 0.20 * env - 0.22 * snap.max(0.0), -0.34 * snap);
                p.torso_pitch = -0.48 * snap;
                p.scale_mul = Vec3::new(
                    1.0 + 0.14 * snap.max(0.0),
                    1.0 - 0.16 * snap.max(0.0),
                    1.0 + 0.14 * snap.max(0.0),
                );
            }
            MotionId::BossSweepingCleave => {
                p.root_offset = Vec3::new(0.0, -0.08 * env, -0.28 * snap);
                p.torso_yaw = 0.85 * snap;
                p.torso_roll = -0.22 * snap;
            }
            MotionId::BossLaserBeamFocus => {
                p.root_offset = Vec3::new((elapsed * 38.0).sin() * 0.025, 0.10 * env, 0.12 * env);
                p.torso_pitch = 0.18 * env;
            }
            MotionId::BossShockwaveStomp => {
                let stomp = env * power;
                p.root_offset =
                    Vec3::new(0.0, 0.24 * (1.0 - t) * stomp - 0.14 * snap.max(0.0), 0.0);
                p.scale_mul = Vec3::new(
                    1.0 + 0.18 * snap.max(0.0),
                    1.0 - 0.18 * snap.max(0.0),
                    1.0 + 0.18 * snap.max(0.0),
                );
            }
            MotionId::BossSummonCall => {
                p.root_offset.y = 0.14 * env;
                p.torso_pitch = 0.30 * env;
                p.torso_yaw = (t * TAU).sin() * 0.25;
            }

            // --- Ally, Summon, Tower & Emotes ---
            MotionId::EmoteSalute => {
                p.root_offset.y = 0.05 * env;
                p.torso_pitch = 0.12 * env;
                p.torso_yaw = 0.16 * env;
            }
            MotionId::EmoteBattleTaunt => {
                p.root_offset = Vec3::new(0.0, -0.06 * env, -0.10 * env);
                p.torso_pitch = -0.18 * env;
                p.torso_yaw = (t * PI * 3.0).sin() * 0.30 * env;
            }
            MotionId::EmoteVictoryFlourish => {
                p.root_offset.y = 0.22 * env;
                p.torso_yaw = t * TAU;
                p.scale_mul = Vec3::new(1.0 + 0.06 * env, 1.0 + 0.10 * env, 1.0 + 0.06 * env);
            }
            MotionId::EmoteBeckonRally => {
                p.root_offset.y = 0.08 * env;
                p.torso_pitch = 0.18 * (t * PI * 2.0).sin().abs();
                p.torso_roll = (t * PI * 2.0).sin() * 0.18;
            }
            MotionId::EmoteBowRespect => {
                p.root_offset.y = -0.10 * env;
                p.torso_pitch = -0.44 * env;
            }
            MotionId::EmoteWeaponInspect => {
                p.torso_yaw = (t * TAU).sin() * 0.35;
                p.torso_roll = (t * TAU).cos() * 0.12 * env;
            }
            MotionId::EmoteWarhornCall => {
                p.root_offset.y = 0.12 * env;
                p.torso_pitch = 0.38 * env;
                p.scale_mul = Vec3::new(1.0 + 0.09 * env, 1.0 + 0.09 * env, 1.0 + 0.09 * env);
            }
            MotionId::SummonSentryAim => {
                p.root_offset.z = 0.12 * snap.max(0.0);
                p.torso_pitch = 0.15 * snap;
            }
            MotionId::SummonColossusSlam => {
                p.root_offset = Vec3::new(0.0, 0.14 * env - 0.12 * snap.max(0.0), -0.22 * snap);
                p.torso_pitch = -0.36 * snap;
            }
            MotionId::TowerOverchargePulse => {
                p.root_offset.y = 0.10 * env;
                p.scale_mul = Vec3::splat(1.0 + 0.12 * env);
            }
        }
        p
    }
}

/// Per-actor procedural motion state machine with critically-damped pose blending.
#[derive(Component, Debug, Clone)]
pub struct MotionState {
    pub current: MotionId,
    pub elapsed: f32,
    pub intensity: f32,
    pub combo_step: u8,
    pub combo_reset_timer: f32,
    pub blended: MotionPose,
    pub prev_speed: f32,
    pub prev_pos: Option<Vec3>,
    pub footstep_accum: f32,
    pub charge_elapsed: f32,
    pub emote_cursor: usize,
}

impl Default for MotionState {
    fn default() -> Self {
        Self {
            current: MotionId::IdleReady,
            elapsed: 0.0,
            intensity: 1.0,
            combo_step: 0,
            combo_reset_timer: 0.0,
            blended: MotionPose::IDENTITY,
            prev_speed: 0.0,
            prev_pos: None,
            footstep_accum: 0.0,
            charge_elapsed: 0.0,
            emote_cursor: 0,
        }
    }
}

impl MotionState {
    pub fn is_finished(&self) -> bool {
        let meta = self.current.meta();
        !meta.looping && self.elapsed >= meta.duration
    }

    pub fn phase(&self) -> f32 {
        let meta = self.current.meta();
        let dur = meta.duration.max(0.001);
        if meta.looping {
            (self.elapsed / dur).fract()
        } else {
            (self.elapsed / dur).clamp(0.0, 1.0)
        }
    }

    /// Requests a motion transition respecting priority & active non-looping locks.
    pub fn try_play(&mut self, id: MotionId, intensity: f32) -> bool {
        let incoming = id.meta();
        let active = self.current.meta();
        if self.current == id {
            if incoming.looping {
                self.intensity = intensity.max(self.intensity * 0.9);
                return false;
            }
            // Allow refreshing same one-shot if past 55% or finished
            if self.elapsed >= active.duration * 0.55 {
                self.elapsed = 0.0;
                self.intensity = intensity;
                return true;
            }
            return false;
        }
        if !self.is_finished() && !active.looping && incoming.priority < active.priority {
            return false;
        }
        self.current = id;
        self.elapsed = 0.0;
        self.intensity = intensity;
        true
    }

    pub fn force_play(&mut self, id: MotionId, intensity: f32) {
        self.current = id;
        self.elapsed = 0.0;
        self.intensity = intensity;
    }

    pub fn advance(&mut self, dt: f32) {
        let dt = dt.max(0.0);
        self.elapsed += dt;
        if self.combo_reset_timer > 0.0 {
            self.combo_reset_timer = (self.combo_reset_timer - dt).max(0.0);
            if self.combo_reset_timer <= 0.0 {
                self.combo_step = 0;
            }
        }
        if self.is_finished() {
            self.current = MotionId::IdleReady;
            self.elapsed = 0.0;
            self.intensity = 1.0;
        }
        let target = MotionPose::sample(self.current, self.phase(), self.elapsed, self.intensity);
        let stiffness = match self.current.meta().category {
            MotionCategory::HitstunKnockback | MotionCategory::ClassAttackCombo => 28.0,
            MotionCategory::EvadeRecovery => 26.0,
            MotionCategory::ChargeTelegraph | MotionCategory::BossSpecial => 18.0,
            _ => 15.0,
        };
        let alpha = critically_damped_alpha(dt, stiffness);
        self.blended.blend_toward(target, alpha);
    }

    /// Evaluates sprint-to-halt skid inertia; returns `true` when `StopSkidRecover` triggers.
    pub fn evaluate_skid_stop(&mut self, speed: f32) -> bool {
        let triggered = self.prev_speed >= SKID_ENTRY_SPEED && speed <= SKID_HALT_SPEED;
        self.prev_speed = speed;
        if triggered {
            self.try_play(MotionId::StopSkidRecover, 1.15)
        } else {
            false
        }
    }
}

/// 62.5 Hz hitstop victim micro-tremor (zero-drift via `MotionVisualDelta`).
#[derive(Component, Debug, Clone, Copy)]
pub struct HitTremor {
    pub elapsed: f32,
    pub duration: f32,
    pub amplitude: f32,
    pub axis: Vec3,
}

impl HitTremor {
    pub fn new(direction: Vec3, amplitude: f32, duration: f32) -> Self {
        let forward = direction.with_y(0.0).normalize_or(Vec3::X);
        let lateral = forward.cross(Vec3::Y).normalize_or(Vec3::Z);
        Self {
            elapsed: 0.0,
            duration: duration.max(0.05),
            amplitude: amplitude.clamp(0.01, 0.16),
            axis: (lateral * 0.82 + forward * 0.35).normalize_or(Vec3::Z),
        }
    }

    /// Evaluates the exact zero-mean 62.5 Hz damped vibration offset.
    pub fn offset(&self, hitstop_active: bool) -> Vec3 {
        if self.elapsed >= self.duration {
            return Vec3::ZERO;
        }
        let t = (self.elapsed / self.duration).clamp(0.0, 1.0);
        let decay = (1.0 - t).powf(1.35);
        let boost = if hitstop_active { 1.45 } else { 1.0 };
        let wave = (self.elapsed * TAU * HIT_TREMOR_HZ).sin();
        self.axis * (self.amplitude * boost * decay * wave)
    }
}

/// Tracks the exact visual transform delta applied on the last rendered frame
/// so `restore_motion_visuals` guarantees zero cumulative drift on gameplay transforms.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct MotionVisualDelta {
    translation: Vec3,
    rotation: Quat,
    scale_mul: Vec3,
    active: bool,
}

impl Default for MotionVisualDelta {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale_mul: Vec3::ONE,
            active: false,
        }
    }
}

/// Resolves directional hitstun (`Front`, `Back`, `Left`, `Right`) from blow travel direction
/// and victim facing forward vector.
pub fn directional_hit_motion(
    blow_direction: Vec3,
    victim_forward: Vec3,
    critical: bool,
    boss: bool,
) -> MotionId {
    if critical {
        return if boss {
            MotionId::BossStaggerBroken
        } else {
            MotionId::StaggerKneel
        };
    }
    let dir = blow_direction.with_y(0.0).normalize_or_zero();
    let fwd = victim_forward.with_y(0.0).normalize_or(Vec3::NEG_Z);
    if dir == Vec3::ZERO {
        return MotionId::HitReactFront;
    }
    // `incoming` points from victim toward attacker (`-blow_direction`).
    let incoming = -dir;
    let right = fwd.cross(Vec3::Y).normalize_or(Vec3::X);
    let front_dot = incoming.dot(fwd);
    let right_dot = incoming.dot(right);
    if front_dot.abs() >= right_dot.abs() {
        if front_dot >= 0.0 {
            MotionId::HitReactFront
        } else {
            MotionId::HitReactBack
        }
    } else if right_dot >= 0.0 {
        MotionId::HitReactRight
    } else {
        MotionId::HitReactLeft
    }
}

/// Maps hero class + combo step (0, 1, 2) to the matching 3-hit combo `MotionId`.
pub const fn class_combo_motion(kind: HeroKind, step: u8) -> MotionId {
    match (weapon(kind), step % 3) {
        (BasicWeapon::Sword, 0) => MotionId::SwordCombo1Slash,
        (BasicWeapon::Sword, 1) => MotionId::SwordCombo2Cross,
        (BasicWeapon::Sword, _) => MotionId::SwordCombo3Finisher,
        (BasicWeapon::Daggers, 0) => MotionId::DaggerCombo1Flick,
        (BasicWeapon::Daggers, 1) => MotionId::DaggerCombo2Twin,
        (BasicWeapon::Daggers, _) => MotionId::DaggerCombo3Spin,
        (BasicWeapon::Axe, 0) => MotionId::AxeCombo1Cleave,
        (BasicWeapon::Axe, 1) => MotionId::AxeCombo2Rend,
        (BasicWeapon::Axe, _) => MotionId::AxeCombo3Crush,
        (BasicWeapon::Crossbow, 0) => MotionId::CrossbowCombo1Snap,
        (BasicWeapon::Crossbow, 1) => MotionId::CrossbowCombo2Burst,
        (BasicWeapon::Crossbow, _) => MotionId::CrossbowCombo3Pierce,
        (BasicWeapon::Frost, 0) => MotionId::FrostArcaneCast1,
        (BasicWeapon::Spirit | BasicWeapon::Star, 1) => MotionId::SpiritStarCast2,
        (_, 0) => MotionId::StormCastCombo1,
        (_, 1) => MotionId::StormCastCombo2,
        (_, _) => MotionId::StormCastCombo3,
    }
}

/// Angled 3-hit combo smear roll angle in radians (`-22°`, `+24°`, `0°` wide finisher).
pub const fn combo_smear_roll_angle(step: u8) -> f32 {
    match step % 3 {
        0 => -0.384,
        1 => 0.419,
        _ => 0.0,
    }
}

/// Builds a tapered crescent blade/projectile smear arc mesh (local -Z forward, Y up).
pub fn build_combo_smear_mesh(
    arc_radians: f32,
    inner_radius: f32,
    outer_radius: f32,
    segments: usize,
) -> Mesh {
    let segments = segments.max(6);
    let half = arc_radians * 0.5;
    let mut positions = Vec::with_capacity((segments + 1) * 2);
    let mut normals = Vec::with_capacity((segments + 1) * 2);
    let mut uvs = Vec::with_capacity((segments + 1) * 2);
    let mut indices = Vec::with_capacity(segments * 6);
    for i in 0..=segments {
        let u = i as f32 / segments as f32;
        let angle = -half + u * arc_radians;
        // Taper thickness cleanly at both leading & trailing tips.
        let taper = (u * PI).sin().max(0.0).powf(0.55);
        let r_out = outer_radius;
        let r_in = outer_radius - (outer_radius - inner_radius) * taper;
        let sin_a = angle.sin();
        let cos_a = angle.cos();
        positions.push([sin_a * r_in, 0.0, -cos_a * r_in]);
        positions.push([sin_a * r_out, 0.0, -cos_a * r_out]);
        normals.push([0.0, 1.0, 0.0]);
        normals.push([0.0, 1.0, 0.0]);
        uvs.push([u, 0.0]);
        uvs.push([u, 1.0]);
        if i < segments {
            let base = (i * 2) as u32;
            indices.extend_from_slice(&[base, base + 1, base + 2, base + 1, base + 3, base + 2]);
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}

/// Category of active cast commitment / cast stiffness ("施法僵滞") on the hero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum CastLockKind {
    #[default]
    None,
    /// Brief tactile attack-step windup & follow-through on basic attacks / swings.
    BasicAttack,
    /// Heavy spell cast lock (near-rooted windup -> recovery) on learned active skills.
    SkillSpell,
    /// Heroic full-root ultimate cast commitment & follow-through.
    Ultimate,
    /// Active `KeyC` focus-charge stance.
    ChargeHold,
}

impl CastLockKind {
    const fn priority(self) -> u8 {
        match self {
            Self::None => 0,
            Self::BasicAttack => 1,
            Self::ChargeHold => 2,
            Self::SkillSpell => 3,
            Self::Ultimate => 4,
        }
    }
}

/// Authoritative presentation + movement cast-stiffness state ("施法僵滞").
///
/// Engages windup root/slowdown, target facing lock, skeletal cast-apex hold,
/// and follow-through easing whenever the hero swings, casts a learned spell,
/// unleashes an ultimate, or holds charge. Always instantly cancellable via Dodge.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub(crate) struct HeroCastLock {
    pub(crate) kind: CastLockKind,
    pub(crate) elapsed: f32,
    pub(crate) total: f32,
    pub(crate) windup: f32,
    pub(crate) aim_dir: Vec3,
    pub(crate) skill_index: Option<usize>,
    pub(crate) cast_serial: u64,
}

impl HeroCastLock {
    pub(crate) fn trigger(
        &mut self,
        kind: CastLockKind,
        total: f32,
        windup: f32,
        aim_dir: Vec3,
        skill_index: Option<usize>,
    ) {
        if self.is_active() && kind.priority() < self.kind.priority() {
            return;
        }
        let dir = aim_dir.with_y(0.0).normalize_or_zero();
        self.kind = kind;
        self.elapsed = 0.0;
        self.total = total.max(0.02);
        self.windup = windup.clamp(0.0, self.total);
        self.aim_dir = dir;
        self.skill_index = skill_index;
        self.cast_serial = self.cast_serial.wrapping_add(1);
    }

    pub(crate) fn maintain_charge(&mut self, aim_dir: Vec3) {
        if self.is_active() && self.kind.priority() > CastLockKind::ChargeHold.priority() {
            return;
        }
        self.kind = CastLockKind::ChargeHold;
        self.elapsed = 0.0;
        self.total = 0.2;
        self.windup = 0.2;
        self.aim_dir = aim_dir.with_y(0.0).normalize_or_zero();
        self.skill_index = None;
    }

    pub(crate) fn release_charge(&mut self) {
        if self.kind == CastLockKind::ChargeHold {
            self.clear();
        }
    }

    /// Dodge roll immediately breaks out of cast stiffness / follow-through recovery.
    pub(crate) fn cancel_by_dodge(&mut self) {
        self.clear();
    }

    pub(crate) fn clear(&mut self) {
        self.kind = CastLockKind::None;
        self.elapsed = 0.0;
        self.total = 0.0;
        self.windup = 0.0;
        self.aim_dir = Vec3::ZERO;
        self.skill_index = None;
    }

    pub(crate) fn tick(&mut self, dt: f32) {
        if self.kind == CastLockKind::None || self.kind == CastLockKind::ChargeHold {
            return;
        }
        self.elapsed += dt.max(0.0);
        if self.elapsed >= self.total {
            self.clear();
        }
    }

    pub(crate) fn is_active(&self) -> bool {
        self.kind != CastLockKind::None
            && (self.kind == CastLockKind::ChargeHold || self.elapsed < self.total)
    }

    pub(crate) fn triggers_cast_pose(&self) -> bool {
        matches!(self.kind, CastLockKind::SkillSpell | CastLockKind::Ultimate)
    }

    /// Horizontal movement speed multiplier during cast windup & recovery.
    pub(crate) fn move_speed_scale(&self) -> f32 {
        if !self.is_active() {
            return 1.0;
        }
        let rec_len = (self.total - self.windup).max(0.001);
        let rec_t = ((self.elapsed - self.windup) / rec_len).clamp(0.0, 1.0);
        let smooth = rec_t * rec_t * (3.0 - 2.0 * rec_t);
        match self.kind {
            CastLockKind::None => 1.0,
            CastLockKind::ChargeHold => 0.22,
            CastLockKind::BasicAttack => {
                if self.elapsed < self.windup {
                    0.30
                } else {
                    0.30 + (0.88 - 0.30) * smooth
                }
            }
            CastLockKind::SkillSpell => {
                if self.elapsed < self.windup {
                    0.08
                } else {
                    0.15 + (0.74 - 0.15) * smooth
                }
            }
            CastLockKind::Ultimate => {
                if self.elapsed < self.windup {
                    0.0
                } else {
                    0.10 + (0.68 - 0.10) * smooth
                }
            }
        }
    }

    /// Locks hero facing toward the spell/attack target during the cast commitment window.
    pub(crate) fn locked_aim_dir(&self) -> Option<Vec3> {
        if self.is_active() && self.aim_dir.length_squared() > 0.01 {
            Some(self.aim_dir)
        } else {
            None
        }
    }

    /// Brief apex stiffness slowdown on the authored GLTF Attack/Cast clip so
    /// spell releases and ultimates read with unmistakable physical weight.
    pub(crate) fn animation_speed_scale(&self) -> f32 {
        if !self.is_active() {
            return 1.0;
        }
        let in_apex = self.elapsed >= self.windup * 0.65 && self.elapsed <= self.windup + 0.06;
        match self.kind {
            CastLockKind::Ultimate if in_apex => 0.36,
            CastLockKind::SkillSpell if in_apex => 0.52,
            CastLockKind::BasicAttack if in_apex => 0.78,
            _ => 1.0,
        }
    }
}

#[derive(Resource, Default)]
struct HeroMotionTracker {
    prev_level: u32,
    prev_kind: Option<HeroKind>,
    prev_alive: bool,
    prev_dodge_cooldown: f32,
    prev_hero_pos: Option<Vec3>,
    had_pending_swing: bool,
    last_ultimate_seen: u64,
    last_spell_cast_seen: u64,
}

#[derive(Resource, Default)]
struct VisceralVfxSequence(u64);

#[derive(Resource)]
struct VisceralVfxArt {
    flash_core: Handle<Mesh>,
    razor_spark: Handle<Mesh>,
    glint_beam: Handle<Mesh>,
    shock_ring: Handle<Mesh>,
    dust_puff: Handle<Mesh>,
    smear_arc: Handle<Mesh>,
    flash_core_mat: Handle<StandardMaterial>,
    crit_glint_mat: Handle<StandardMaterial>,
    dust_mat: Handle<StandardMaterial>,
    element_mats: [Handle<StandardMaterial>; 7],
    hero_smear_mats: [Handle<StandardMaterial>; HeroKind::COUNT],
}

#[derive(Component, Debug, Clone, Copy)]
struct VisceralVfx {
    elapsed: f32,
    duration: f32,
    velocity: Vec3,
    drag: f32,
    start_scale: Vec3,
    end_scale: Vec3,
    spin_y: f32,
}

#[derive(Component)]
struct MotionHudBadge;

#[derive(Component)]
struct MotionHudText;

fn additive_unlit_material(color: Color, emissive_boost: f32) -> StandardMaterial {
    let linear = color.to_linear();
    StandardMaterial {
        base_color: color.with_alpha(0.85),
        emissive: LinearRgba::rgb(
            linear.red * emissive_boost,
            linear.green * emissive_boost,
            linear.blue * emissive_boost,
        ),
        unlit: true,
        alpha_mode: AlphaMode::Add,
        cull_mode: None,
        ..default()
    }
}

const ELEMENT_ORDER: [DamageElement; 7] = [
    DamageElement::Physical,
    DamageElement::Storm,
    DamageElement::Flame,
    DamageElement::Frost,
    DamageElement::Tide,
    DamageElement::Arcane,
    DamageElement::Pure,
];

const fn element_slot(element: DamageElement) -> usize {
    match element {
        DamageElement::Physical => 0,
        DamageElement::Storm => 1,
        DamageElement::Flame => 2,
        DamageElement::Frost => 3,
        DamageElement::Tide => 4,
        DamageElement::Arcane => 5,
        DamageElement::Pure => 6,
    }
}

fn setup_visceral_vfx_art(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let element_mats = ELEMENT_ORDER
        .map(|el| materials.add(additive_unlit_material(damage_element_color(el), 4.2)));
    let hero_colors = [
        Color::srgb(0.26, 0.86, 1.0),
        Color::srgb(1.0, 0.62, 0.24),
        Color::srgb(0.75, 0.94, 0.86),
        Color::srgb(0.42, 1.0, 0.62),
        Color::srgb(0.28, 0.90, 1.0),
        Color::srgb(1.0, 0.82, 0.30),
        Color::srgb(0.62, 0.94, 1.0),
        Color::srgb(1.0, 0.44, 0.22),
        Color::srgb(0.84, 0.34, 1.0),
        Color::srgb(0.60, 0.96, 0.42),
        Color::srgb(1.0, 0.86, 0.38),
        Color::srgb(0.70, 0.36, 1.0),
    ];
    let hero_smear_mats =
        hero_colors.map(|color| materials.add(additive_unlit_material(color, 3.8)));

    commands.insert_resource(VisceralVfxArt {
        flash_core: meshes.add(Sphere::new(0.45).mesh().ico(1).expect("valid icosphere")),
        razor_spark: meshes.add(Cuboid::new(0.055, 0.045, 0.72)),
        glint_beam: meshes.add(Cuboid::new(0.075, 0.055, 1.65)),
        shock_ring: meshes.add(Torus::new(0.88, 1.0)),
        dust_puff: meshes.add(Sphere::new(0.32).mesh().ico(1).expect("valid dust puff")),
        smear_arc: meshes.add(build_combo_smear_mesh(2.15, 0.56, 1.15, 18)),
        flash_core_mat: materials.add(additive_unlit_material(Color::srgb(1.0, 0.98, 0.92), 7.0)),
        crit_glint_mat: materials.add(additive_unlit_material(Color::srgb(1.0, 0.86, 0.24), 6.2)),
        dust_mat: materials.add(additive_unlit_material(Color::srgb(0.42, 0.36, 0.28), 0.75)),
        element_mats,
        hero_smear_mats,
    });
}

fn setup_motion_hud_badge(mut commands: Commands) {
    commands
        .spawn((
            Name::new("MOTION STATUS BADGE"),
            MotionHudBadge,
            Node {
                position_type: PositionType::Absolute,
                left: percent(50),
                bottom: px(108),
                margin: UiRect::left(px(-108)),
                min_width: px(216),
                padding: UiRect::axes(px(12), px(4)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(10)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.03, 0.06, 0.11, 0.72)),
            BorderColor::all(Color::srgba(0.32, 0.78, 0.96, 0.55)),
            GlobalZIndex(22),
            Pickable::IGNORE,
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            parent.spawn((
                MotionHudText,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::srgb(0.92, 0.97, 1.0)),
                Pickable::IGNORE,
            ));
        });
}

/// Restores every zero-drift visual transform delta at the start of the frame
/// so gameplay, collision, hit-punch, and camera systems observe pure transforms.
fn restore_motion_visuals(mut actors: Query<(&mut Transform, &mut MotionVisualDelta)>) {
    for (mut transform, mut delta) in &mut actors {
        if !delta.active {
            continue;
        }
        transform.translation -= delta.translation;
        transform.rotation = (transform.rotation * delta.rotation.inverse()).normalize();
        let inv_scale = Vec3::new(
            if delta.scale_mul.x.abs() > 1e-4 {
                1.0 / delta.scale_mul.x
            } else {
                1.0
            },
            if delta.scale_mul.y.abs() > 1e-4 {
                1.0 / delta.scale_mul.y
            } else {
                1.0
            },
            if delta.scale_mul.z.abs() > 1e-4 {
                1.0 / delta.scale_mul.z
            } else {
                1.0
            },
        );
        transform.scale *= inv_scale;
        *delta = MotionVisualDelta::default();
    }
}

/// Ensures `HeroVisual`, active `Enemy`, and `PlayerSummon` entities have `MotionState`
/// and `MotionVisualDelta` attached.
fn ensure_actor_motion_states(
    mut commands: Commands,
    hero_visuals: Query<Entity, (With<HeroVisual>, Without<MotionState>)>,
    enemies: Query<Entity, (With<Enemy>, Without<MotionState>)>,
    summons: Query<Entity, (With<PlayerSummon>, Without<MotionState>)>,
) {
    for entity in hero_visuals
        .iter()
        .chain(enemies.iter())
        .chain(summons.iter())
    {
        commands
            .entity(entity)
            .insert((MotionState::default(), MotionVisualDelta::default()));
    }
}

fn spawn_visceral_entity(
    commands: &mut Commands,
    budget: &mut usize,
    name: &'static str,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    transform: Transform,
    vfx: VisceralVfx,
) {
    if *budget == 0 {
        return;
    }
    *budget -= 1;
    commands.spawn((
        Name::new(name),
        Mesh3d(mesh),
        MeshMaterial3d(material),
        NotShadowCaster,
        NotShadowReceiver,
        transform,
        vfx,
    ));
}

#[derive(SystemParam)]
struct HeroMotionInputs<'w> {
    session: Res<'w, GameSession>,
    vitals: Res<'w, HeroVitals>,
    selected: Res<'w, SelectedHero>,
    dodge: Res<'w, DodgeState>,
    move_intent: Res<'w, MoveIntent>,
    keyboard: Option<Res<'w, ButtonInput<KeyCode>>>,
    volleys: Option<Res<'w, AttackVolleyRuntime>>,
    attack_aim: Option<Res<'w, BasicAttackAim>>,
    skills: Option<Res<'w, SkillRuntime>>,
    feedback: Option<Res<'w, HitFeedback>>,
    ultimates: Option<Res<'w, UltimateCues>>,
    autoplay: Option<Res<'w, Autoplay>>,
    art: Option<Res<'w, VisceralVfxArt>>,
}

fn spell_cast_motion_for_skill(skill_index: usize) -> (MotionId, DamageElement) {
    match skill_index % 6 {
        0 => (MotionId::StormCastCombo1, DamageElement::Storm),
        1 => (MotionId::FrostArcaneCast1, DamageElement::Frost),
        2 => (MotionId::StormCastCombo2, DamageElement::Flame),
        3 => (MotionId::SpiritStarCast2, DamageElement::Tide),
        4 => (MotionId::StormCastCombo3, DamageElement::Arcane),
        _ => (MotionId::SpellChannelLoop, DamageElement::Pure),
    }
}

#[allow(clippy::too_many_arguments)]
fn drive_hero_motion_and_smears(
    mut commands: Commands,
    time: Res<Time>,
    inputs: HeroMotionInputs,
    mut tracker: ResMut<HeroMotionTracker>,
    mut cast_lock: ResMut<HeroCastLock>,
    existing_vfx: Query<(), With<VisceralVfx>>,
    pending_swings: Query<&PendingSwing>,
    heroes: Query<&Transform, With<Hero>>,
    enemies: Query<&Transform, (With<Enemy>, Without<Hero>)>,
    mut visuals: Query<(Entity, &mut MotionState), With<HeroVisual>>,
) {
    let HeroMotionInputs {
        session,
        vitals,
        selected,
        dodge,
        move_intent,
        keyboard,
        volleys,
        attack_aim,
        skills,
        feedback,
        ultimates,
        autoplay,
        art,
    } = inputs;
    if session.paused || session.finished {
        return;
    }
    let Ok(hero_transform) = heroes.single() else {
        return;
    };
    let Ok((visual_entity, mut motion)) = visuals.single_mut() else {
        return;
    };
    let dt = time.delta_secs().max(0.0);
    cast_lock.tick(dt);
    let mut budget = MAX_VISCERAL_VFX.saturating_sub(existing_vfx.iter().len());
    let hero_pos = hero_transform.translation;
    let hero_fwd = hero_transform
        .forward()
        .with_y(0.0)
        .normalize_or(Vec3::NEG_Z);
    let hero_right = hero_fwd.cross(Vec3::Y).normalize_or(Vec3::X);

    // 1. Death / Revive transitions
    if !vitals.alive {
        cast_lock.clear();
        if tracker.prev_alive {
            motion.force_play(MotionId::DeathCollapseBack, 1.2);
        }
        tracker.prev_alive = false;
        tracker.prev_hero_pos = Some(hero_pos);
        return;
    } else if !tracker.prev_alive && tracker.prev_kind.is_some() {
        motion.force_play(MotionId::ReviveRiseUp, 1.15);
    }
    tracker.prev_alive = true;

    // 2. Hero recruitment / Level-up celebration
    if tracker.prev_kind.is_some_and(|prev| prev != selected.kind) {
        motion.try_play(MotionId::HeroRecruitAwaken, 1.2);
    }
    tracker.prev_kind = Some(selected.kind);

    if tracker.prev_level > 0 && session.level > tracker.prev_level {
        motion.try_play(MotionId::LevelUpAscend, 1.15);
    }
    tracker.prev_level = session.level;

    // 3. Hero Stunned / Hurt directional hitstun + micro-tremor
    if vitals.stunned_remaining > 0.0 {
        cast_lock.clear();
        motion.try_play(MotionId::AilmentStunnedDaze, 1.15);
    }
    if let Some(feedback) = feedback.as_deref()
        && feedback.hero_hurt()
    {
        let hit_dir = -feedback.hero_push();
        let react = directional_hit_motion(hit_dir, hero_fwd, false, false);
        motion.try_play(react, 1.1);
        commands
            .entity(visual_entity)
            .insert(HitTremor::new(hit_dir, 0.055, 0.14));
    }

    // 4. Dodge detection (cooldown spike or large instant dash step)
    let displacement = tracker
        .prev_hero_pos
        .map_or(Vec3::ZERO, |prev| (hero_pos - prev).with_y(0.0));
    let speed = if dt > 1e-4 {
        displacement.length() / dt
    } else {
        0.0
    };
    let dodged = dodge.cooldown_remaining > tracker.prev_dodge_cooldown + 0.5
        || displacement.length() > 1.45;
    tracker.prev_dodge_cooldown = dodge.cooldown_remaining;
    if dodged {
        cast_lock.cancel_by_dodge();
        let dir = displacement.normalize_or(hero_fwd);
        let fwd_dot = dir.dot(hero_fwd);
        let right_dot = dir.dot(hero_right);
        let dodge_motion = if fwd_dot >= right_dot.abs() {
            MotionId::DodgeRollForward
        } else if -fwd_dot >= right_dot.abs() {
            MotionId::DodgeDashBackstep
        } else if right_dot >= 0.0 {
            MotionId::DodgeSidestepRight
        } else {
            MotionId::DodgeSidestepLeft
        };
        motion.force_play(dodge_motion, 1.15);
        if let Some(art) = art.as_deref() {
            for i in 0..3 {
                let spread = (i as f32 - 1.0) * 0.32;
                spawn_visceral_entity(
                    &mut commands,
                    &mut budget,
                    "Dodge dust puff",
                    art.dust_puff.clone(),
                    art.dust_mat.clone(),
                    Transform::from_translation(
                        hero_pos + Vec3::Y * 0.14 - dir * 0.35 + hero_right * spread,
                    )
                    .with_scale(Vec3::splat(0.32)),
                    VisceralVfx {
                        elapsed: 0.0,
                        duration: 0.28,
                        velocity: -dir * 1.4 + hero_right * spread * 1.2 + Vec3::Y * 0.65,
                        drag: 4.5,
                        start_scale: Vec3::splat(0.32),
                        end_scale: Vec3::splat(0.75),
                        spin_y: 2.0,
                    },
                );
            }
        }
    }

    // 5. Ultimate burst cues -> Full-root ultimate cast lock ("终极施法僵直") + ground sigil
    if let Some(ultimates) = ultimates.as_deref() {
        if ultimates.serial > tracker.last_ultimate_seen {
            let aim_dir = ultimates
                .last_cast
                .map(|(_, origin, target)| (target - origin).with_y(0.0))
                .filter(|v| v.length_squared() > 0.01)
                .map_or(hero_fwd, Vec3::normalize);
            cast_lock.trigger(CastLockKind::Ultimate, 0.40, 0.16, aim_dir, None);
            motion.force_play(MotionId::UltimateUnleashBurst, 1.38);
            if let Some(art) = art.as_deref() {
                spawn_visceral_entity(
                    &mut commands,
                    &mut budget,
                    "Ultimate cast lock sigil",
                    art.shock_ring.clone(),
                    art.hero_smear_mats[vitals.kind.index()].clone(),
                    Transform::from_translation(hero_pos.with_y(0.14))
                        .with_scale(Vec3::new(0.45, 0.35, 0.45)),
                    VisceralVfx {
                        elapsed: 0.0,
                        duration: 0.38,
                        velocity: Vec3::ZERO,
                        drag: 0.0,
                        start_scale: Vec3::new(0.45, 0.35, 0.45),
                        end_scale: Vec3::new(2.35, 0.08, 2.35),
                        spin_y: 4.2,
                    },
                );
                for i in 0..4 {
                    let angle = i as f32 * FRAC_PI_2 + 0.35;
                    let dir = Vec3::new(angle.cos(), 0.75, angle.sin()).normalize();
                    spawn_visceral_entity(
                        &mut commands,
                        &mut budget,
                        "Ultimate cast spark",
                        art.razor_spark.clone(),
                        art.crit_glint_mat.clone(),
                        Transform::from_translation(hero_pos + Vec3::Y * 0.7)
                            .looking_to(dir, Vec3::Y)
                            .with_scale(Vec3::new(1.0, 1.0, 1.2)),
                        VisceralVfx {
                            elapsed: 0.0,
                            duration: 0.24,
                            velocity: dir * 4.6,
                            drag: 5.0,
                            start_scale: Vec3::new(1.0, 1.0, 1.2),
                            end_scale: Vec3::new(0.15, 0.15, 0.2),
                            spin_y: 0.0,
                        },
                    );
                }
            }
        }
        tracker.last_ultimate_seen = ultimates.serial;
    }

    // 5b. Learned Skill Spell Casts -> Authentic Spell Cast Lock ("施法僵滞") + Cast Sigil Ring
    if let Some(skills) = skills.as_deref() {
        let spell_count = skills.spell_cast_count();
        if spell_count > tracker.last_spell_cast_seen && vitals.can_act() && !dodged {
            let (skill_index, aim_dir) = skills
                .last_spell_info()
                .map(|(idx, center)| {
                    let dir = (center - hero_pos)
                        .with_y(0.0)
                        .try_normalize()
                        .unwrap_or(hero_fwd);
                    (idx, dir)
                })
                .unwrap_or((0, hero_fwd));
            let (cast_motion, element) = spell_cast_motion_for_skill(skill_index);
            cast_lock.trigger(
                CastLockKind::SkillSpell,
                0.22,
                0.08,
                aim_dir,
                Some(skill_index),
            );
            motion.force_play(cast_motion, 1.22);
            if let Some(art) = art.as_deref() {
                let mat = art.element_mats[element_slot(element)].clone();
                spawn_visceral_entity(
                    &mut commands,
                    &mut budget,
                    "Spell cast commitment ring",
                    art.shock_ring.clone(),
                    mat.clone(),
                    Transform::from_translation(hero_pos.with_y(0.12))
                        .with_scale(Vec3::new(0.32, 0.28, 0.32)),
                    VisceralVfx {
                        elapsed: 0.0,
                        duration: 0.22,
                        velocity: Vec3::ZERO,
                        drag: 0.0,
                        start_scale: Vec3::new(0.32, 0.28, 0.32),
                        end_scale: Vec3::new(1.45, 0.06, 1.45),
                        spin_y: 3.4,
                    },
                );
                spawn_visceral_entity(
                    &mut commands,
                    &mut budget,
                    "Spell release hand flash",
                    art.flash_core.clone(),
                    mat,
                    Transform::from_translation(hero_pos + aim_dir * 0.55 + Vec3::Y * 0.92)
                        .with_scale(Vec3::splat(0.34)),
                    VisceralVfx {
                        elapsed: 0.0,
                        duration: CONTACT_FLASH_CORE_SECONDS,
                        velocity: aim_dir * 1.4,
                        drag: 6.0,
                        start_scale: Vec3::splat(0.34),
                        end_scale: Vec3::splat(0.04),
                        spin_y: 0.0,
                    },
                );
            }
        }
        tracker.last_spell_cast_seen = spell_count;
    }

    // 6. Active combat focus charge (`KeyC` hold) + Melee swing windup
    let holding_charge = keyboard
        .as_deref()
        .is_some_and(|keys| keys.pressed(KeyCode::KeyC))
        && vitals.can_act();
    let released_charge = keyboard
        .as_deref()
        .is_some_and(|keys| keys.just_released(KeyCode::KeyC));
    if holding_charge {
        cast_lock.maintain_charge(hero_fwd);
        motion.charge_elapsed += dt;
        let charge_id = if motion.charge_elapsed >= 1.15 {
            MotionId::ChargePeakOverload
        } else if motion.charge_elapsed >= 0.45 {
            MotionId::ChargeHoldHeavy
        } else {
            MotionId::ChargeHoldLight
        };
        let charge_intensity = 1.0 + (motion.charge_elapsed * 0.35).min(0.5);
        motion.try_play(charge_id, charge_intensity);
    } else if released_charge && motion.charge_elapsed >= 0.25 {
        cast_lock.release_charge();
        cast_lock.trigger(CastLockKind::SkillSpell, 0.24, 0.06, hero_fwd, None);
        let finisher = class_combo_motion(vitals.kind, 2);
        motion.force_play(finisher, 1.35);
        if let Some(art) = art.as_deref() {
            spawn_combo_smear_arc(
                &mut commands,
                &mut budget,
                art,
                vitals.kind,
                hero_pos,
                hero_fwd,
                2,
                1.35,
            );
        }
        motion.charge_elapsed = 0.0;
    } else {
        cast_lock.release_charge();
        motion.charge_elapsed = 0.0;
    }

    let has_pending_swing = !pending_swings.is_empty();
    let aim_dir = attack_aim
        .as_deref()
        .and_then(|aim| aim.0)
        .map(|target| (target - hero_pos).with_y(0.0))
        .and_then(Vec3::try_normalize)
        .unwrap_or(hero_fwd);
    if has_pending_swing && !tracker.had_pending_swing && !holding_charge {
        motion.try_play(MotionId::HeavyOverheadWindup, 1.05);
        if autoplay.is_none() {
            cast_lock.trigger(CastLockKind::BasicAttack, 0.12, 0.05, aim_dir, None);
        }
    }
    tracker.had_pending_swing = has_pending_swing;

    // 7. 3-Hit Class Combos + Angled Blade/Projectile Smear Arcs + Attack-Step Commitment
    if vitals.can_act()
        && volleys
            .as_deref()
            .is_some_and(|v| v.resolved_this_frame > 0)
    {
        if autoplay.is_none() {
            cast_lock.trigger(CastLockKind::BasicAttack, 0.11, 0.04, aim_dir, None);
        }
        let step = motion.combo_step;
        let combo_id = class_combo_motion(vitals.kind, step);
        motion.try_play(combo_id, if step == 2 { 1.28 } else { 1.05 });
        if let Some(art) = art.as_deref() {
            spawn_combo_smear_arc(
                &mut commands,
                &mut budget,
                art,
                vitals.kind,
                hero_pos,
                aim_dir,
                step,
                if step == 2 { 1.28 } else { 1.0 },
            );
        }
        motion.combo_step = (step + 1) % 3;
        motion.combo_reset_timer = COMBO_RESET_SECONDS;
    }

    // 8. Emote cycle (`Digit0`)
    if let Some(keys) = keyboard.as_deref()
        && keys.just_pressed(KeyCode::Digit0)
        && vitals.can_act()
    {
        let emote = HERO_EMOTE_CYCLE[motion.emote_cursor % HERO_EMOTE_CYCLE.len()];
        motion.emote_cursor = (motion.emote_cursor + 1) % HERO_EMOTE_CYCLE.len();
        motion.force_play(emote, 1.1);
    }

    // 9. Skid-stop inertia, footstep dust puffs, and grounded locomotion
    if !dodged && !holding_charge {
        let skidded = motion.evaluate_skid_stop(speed);
        if skidded && let Some(art) = art.as_deref() {
            for sign in [-1.0_f32, 1.0] {
                spawn_visceral_entity(
                    &mut commands,
                    &mut budget,
                    "Skid-stop dust puff",
                    art.dust_puff.clone(),
                    art.dust_mat.clone(),
                    Transform::from_translation(
                        hero_pos + Vec3::Y * 0.12 + hero_fwd * 0.25 + hero_right * sign * 0.28,
                    )
                    .with_scale(Vec3::splat(0.30)),
                    VisceralVfx {
                        elapsed: 0.0,
                        duration: 0.30,
                        velocity: hero_fwd * 1.1 + hero_right * sign * 0.8 + Vec3::Y * 0.55,
                        drag: 4.8,
                        start_scale: Vec3::splat(0.30),
                        end_scale: Vec3::splat(0.72),
                        spin_y: sign * 2.5,
                    },
                );
            }
        }
        if speed > 0.45 || move_intent.0.length_squared() > 0.04 {
            motion.footstep_accum += displacement.length();
            if motion.footstep_accum >= FOOTSTEP_STRIDE_METERS {
                motion.footstep_accum -= FOOTSTEP_STRIDE_METERS;
                if let Some(art) = art.as_deref() {
                    let side = if (motion.elapsed * 4.0).sin() >= 0.0 {
                        0.22
                    } else {
                        -0.22
                    };
                    spawn_visceral_entity(
                        &mut commands,
                        &mut budget,
                        "Footstep dust puff",
                        art.dust_puff.clone(),
                        art.dust_mat.clone(),
                        Transform::from_translation(hero_pos + Vec3::Y * 0.09 + hero_right * side)
                            .with_scale(Vec3::splat(0.20)),
                        VisceralVfx {
                            elapsed: 0.0,
                            duration: 0.24,
                            velocity: -displacement.normalize_or_zero() * 0.5 + Vec3::Y * 0.38,
                            drag: 5.0,
                            start_scale: Vec3::splat(0.20),
                            end_scale: Vec3::splat(0.46),
                            spin_y: 1.4,
                        },
                    );
                }
            }
            let move_dir = displacement.normalize_or(hero_fwd);
            let fwd_dot = move_dir.dot(hero_fwd);
            let right_dot = move_dir.dot(hero_right);
            let loco = if speed >= 3.6 {
                MotionId::RunSprint
            } else if fwd_dot < -0.55 {
                MotionId::BackpedalRetreat
            } else if right_dot.abs() > 0.72 && fwd_dot.abs() < 0.55 {
                if right_dot > 0.0 {
                    MotionId::StrafeRight
                } else {
                    MotionId::StrafeLeft
                }
            } else {
                MotionId::WalkMarch
            };
            motion.try_play(loco, 1.0);
        } else {
            let low_hp = vitals.max_health > 0.0 && vitals.health / vitals.max_health < 0.30;
            let enemy_near = enemies
                .iter()
                .any(|t| t.translation.distance_squared(hero_pos) < 81.0);
            let idle = if low_hp {
                MotionId::IdleExhaustedLowHp
            } else if enemy_near {
                MotionId::IdleCombatGuard
            } else {
                MotionId::IdleReady
            };
            motion.try_play(idle, 1.0);
        }
    }
    tracker.prev_hero_pos = Some(hero_pos);
}

#[allow(clippy::too_many_arguments)]
fn spawn_combo_smear_arc(
    commands: &mut Commands,
    budget: &mut usize,
    art: &VisceralVfxArt,
    kind: HeroKind,
    hero_pos: Vec3,
    forward: Vec3,
    combo_step: u8,
    scale_boost: f32,
) {
    let roll = combo_smear_roll_angle(combo_step);
    let base_rot = Transform::IDENTITY.looking_to(forward, Vec3::Y).rotation;
    let rot = base_rot * Quat::from_rotation_z(roll);
    let melee = weapon(kind).melee();
    let radius = (if melee { 1.45 } else { 0.92 }) * scale_boost;
    let mat = art.hero_smear_mats[kind.index()].clone();
    let origin = hero_pos + Vec3::Y * 0.92 + forward * (if melee { 0.45 } else { 0.62 });
    spawn_visceral_entity(
        commands,
        budget,
        "Combo smear arc",
        art.smear_arc.clone(),
        mat.clone(),
        Transform::from_translation(origin)
            .with_rotation(rot)
            .with_scale(Vec3::new(radius * 0.85, 1.0, radius * 0.85)),
        VisceralVfx {
            elapsed: 0.0,
            duration: if combo_step == 2 { 0.21 } else { 0.15 },
            velocity: forward * (if melee { 1.4 } else { 2.8 }),
            drag: 6.0,
            start_scale: Vec3::new(radius * 0.85, 1.0, radius * 0.85),
            end_scale: Vec3::new(radius * 1.25, 0.25, radius * 1.25),
            spin_y: if combo_step.is_multiple_of(2) {
                -6.5
            } else {
                6.5
            },
        },
    );
    // 3rd-hit combo finisher gets an extra bright white-hot core echo arc!
    if combo_step == 2 {
        spawn_visceral_entity(
            commands,
            budget,
            "Combo finisher echo arc",
            art.smear_arc.clone(),
            art.flash_core_mat.clone(),
            Transform::from_translation(origin + Vec3::Y * 0.08)
                .with_rotation(base_rot)
                .with_scale(Vec3::new(radius * 0.72, 1.0, radius * 0.72)),
            VisceralVfx {
                elapsed: 0.0,
                duration: 0.14,
                velocity: forward * 2.2,
                drag: 7.0,
                start_scale: Vec3::new(radius * 0.72, 1.0, radius * 0.72),
                end_scale: Vec3::new(radius * 1.38, 0.15, radius * 1.38),
                spin_y: -9.0,
            },
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn drive_enemy_boss_summon_motions(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    skills: Option<Res<SkillRuntime>>,
    feedback: Option<Res<HitFeedback>>,
    mut attack_cues: MessageReader<EnemyAttackCue>,
    mut phase_cues: MessageReader<BossPhaseCue>,
    new_bosses: Query<Entity, Added<BossEnemy>>,
    mut enemies: Query<
        (
            Entity,
            &Transform,
            &Enemy,
            &mut MotionState,
            Option<&BossEnemy>,
            Option<&BossAction>,
            Option<&EliteAffix>,
            Option<&Infection>,
            Option<&WetMark>,
            Option<&VoidPull>,
            Option<&TimedSpeedEffect>,
        ),
        Without<HeroVisual>,
    >,
    mut summons: Query<
        (&Transform, &mut MotionState),
        (With<PlayerSummon>, Without<Enemy>, Without<HeroVisual>),
    >,
) {
    if session.paused || session.finished {
        attack_cues.clear();
        phase_cues.clear();
        return;
    }
    let dt = time.delta_secs().max(1e-4);

    // Collect enemy attack cues & boss phase shifts
    let mut attack_map: HashMap<Entity, EnemyAttackKind> = HashMap::new();
    for cue in attack_cues.read() {
        attack_map.insert(cue.source, cue.kind);
    }
    let mut phase_bosses = Vec::new();
    for cue in phase_cues.read() {
        phase_bosses.push(cue.boss);
    }

    // 1. Boss entrance roar
    for entity in &new_bosses {
        if let Ok((_, _, _, mut motion, ..)) = enemies.get_mut(entity) {
            motion.force_play(MotionId::BossEntranceRoar, 1.3);
        }
    }

    // 2. Boss phase shift eruption
    for boss in phase_bosses {
        if let Ok((_, _, _, mut motion, ..)) = enemies.get_mut(boss) {
            motion.force_play(MotionId::BossPhaseShiftErupt, 1.45);
        }
    }

    // 3. Directional hitstun + Hitstop Victim Micro-Tremor from `HitFeedback`
    if let Some(feedback) = feedback.as_deref() {
        for event in feedback.events().iter().filter(|e| !e.killed) {
            if let Ok((entity, transform, _, mut motion, boss, ..)) = enemies.get_mut(event.target)
            {
                let fwd = transform.forward().with_y(0.0).normalize_or(Vec3::NEG_Z);
                let react =
                    directional_hit_motion(event.direction, fwd, event.critical, boss.is_some());
                let power = if event.critical { 1.35 } else { 1.0 };
                motion.try_play(react, power);
                let amp = match (event.weight, event.critical) {
                    (HitWeight::Boss, true) => 0.085,
                    (HitWeight::Boss, false) => 0.045,
                    (HitWeight::Elite, true) => 0.095,
                    (_, true) => 0.11,
                    _ => 0.055,
                };
                let dur = if event.critical { 0.14 } else { 0.095 };
                commands
                    .entity(entity)
                    .insert(HitTremor::new(event.direction, amp, dur));
            }
        }
    }

    // 4. Per-enemy status ailments, boss windups/recoveries, enemy attacks, and locomotion
    for (
        entity,
        transform,
        enemy,
        mut motion,
        boss,
        boss_action,
        affix,
        infection,
        wet_mark,
        void_pull,
        speed_effect,
    ) in &mut enemies
    {
        if enemy.hp <= 0.0 {
            motion.force_play(
                if boss.is_some() {
                    MotionId::BossDefeatFall
                } else {
                    MotionId::DeathCollapseForward
                },
                1.2,
            );
            continue;
        }
        // Status ailments from SkillRuntime + ECS components
        let frozen = skills.as_deref().is_some_and(|s| s.is_frozen(entity));
        let stunned = skills.as_deref().is_some_and(|s| s.is_stunned(entity));
        let burning = skills.as_deref().is_some_and(|s| s.is_burning(entity));
        let slowed =
            skills.as_deref().is_some_and(|s| s.is_slowed(entity)) || speed_effect.is_some();
        let vulnerable = skills
            .as_deref()
            .is_some_and(|s| s.is_wet(entity) || s.has_seal(entity))
            || wet_mark.is_some();

        if frozen {
            motion.try_play(MotionId::AilmentFrozenRigid, 1.0);
        } else if stunned {
            motion.try_play(MotionId::AilmentStunnedDaze, 1.15);
        } else if affix.is_some_and(|a| a.is_volatile_pending()) {
            motion.try_play(MotionId::EnemyVolatileSwell, 1.3);
        } else if void_pull.is_some() {
            motion.try_play(MotionId::AilmentJailedRooted, 1.1);
        } else if let Some(action) = boss_action {
            let id = match action.stage {
                BossActionStage::Windup => MotionId::TelegraphGroundSlam,
                BossActionStage::Recovery => MotionId::BossSweepingCleave,
            };
            motion.try_play(id, 1.25);
        } else if let Some(&kind) = attack_map.get(&entity) {
            let id = match (boss.is_some(), kind) {
                (true, EnemyAttackKind::Slam) => MotionId::BossHeavyOverheadSlam,
                (true, EnemyAttackKind::Beam) => MotionId::BossLaserBeamFocus,
                (true, EnemyAttackKind::Burst) => MotionId::BossShockwaveStomp,
                (true, _) => MotionId::BossSweepingCleave,
                (false, EnemyAttackKind::Shot) => MotionId::EnemyRangedDrawFire,
                (false, EnemyAttackKind::Slam) => MotionId::EnemyShieldBash,
                (false, EnemyAttackKind::Heal) => MotionId::EnemyHealPulse,
                (false, EnemyAttackKind::Burst) => MotionId::EnemyChantHaste,
                (false, _) => MotionId::EnemyLungeBite,
            };
            motion.try_play(id, 1.1);
        } else if burning {
            motion.try_play(MotionId::AilmentBurning, 1.05);
        } else if infection.is_some() {
            motion.try_play(MotionId::AilmentInfectedBlight, 1.05);
        } else if slowed {
            motion.try_play(MotionId::AilmentSlowedMud, 1.0);
        } else if vulnerable {
            motion.try_play(MotionId::AilmentVulnerable, 1.0);
        } else {
            let speed = motion.prev_pos.map_or(0.0, |prev| {
                (transform.translation - prev).with_y(0.0).length() / dt
            });
            if speed > 2.8 {
                motion.try_play(MotionId::RunSprint, 0.9);
            } else if speed > 0.2 {
                motion.try_play(MotionId::WalkMarch, 0.9);
            } else {
                motion.try_play(MotionId::IdleCombatGuard, 0.85);
            }
        }
        motion.prev_pos = Some(transform.translation);
    }

    // 5. Player summons
    for (transform, mut motion) in &mut summons {
        let speed = motion.prev_pos.map_or(0.0, |prev| {
            (transform.translation - prev).with_y(0.0).length() / dt
        });
        if speed > 0.35 {
            motion.try_play(MotionId::WalkMarch, 0.85);
        } else {
            motion.try_play(MotionId::IdleReady, 0.8);
        }
        motion.prev_pos = Some(transform.translation);
    }
}

/// Spawns strictly additive (`AlphaMode::Add`, `unlit: true`, `NotShadowCaster`, `NotShadowReceiver`)
/// directional razor hit-sparks, 85ms contact hit-flash cores, crit cross-glints, and kill finisher
/// shock-rings on every landed combat hit.
fn spawn_visceral_hit_vfx(
    mut commands: Commands,
    session: Res<GameSession>,
    feedback: Option<Res<HitFeedback>>,
    art: Option<Res<VisceralVfxArt>>,
    mut seq: ResMut<VisceralVfxSequence>,
    existing: Query<(), With<VisceralVfx>>,
) {
    if session.paused || session.finished {
        return;
    }
    let (Some(feedback), Some(art)) = (feedback.as_deref(), art.as_deref()) else {
        return;
    };
    let mut budget = MAX_VISCERAL_VFX.saturating_sub(existing.iter().len());
    for event in feedback.events() {
        if budget == 0 {
            break;
        }
        let body = event.position.with_y(0.95);
        let blow_dir = event.direction.with_y(0.0).normalize_or(Vec3::X);
        let side = blow_dir.cross(Vec3::Y).normalize_or(Vec3::Z);
        let el_mat = art.element_mats[element_slot(event.element)].clone();

        // 1. 85ms Contact Hit-Flash Core
        let core_scale = if event.critical { 0.58 } else { 0.36 };
        spawn_visceral_entity(
            &mut commands,
            &mut budget,
            "Contact hit-flash core",
            art.flash_core.clone(),
            art.flash_core_mat.clone(),
            Transform::from_translation(body - blow_dir * 0.18).with_scale(Vec3::splat(core_scale)),
            VisceralVfx {
                elapsed: 0.0,
                duration: CONTACT_FLASH_CORE_SECONDS,
                velocity: Vec3::ZERO,
                drag: 0.0,
                start_scale: Vec3::splat(core_scale),
                end_scale: Vec3::splat(core_scale * 2.1),
                spin_y: 0.0,
            },
        );

        // 2. Directional Razor Hit-Sparks
        let spark_count = if event.killed {
            6
        } else if event.critical {
            5
        } else {
            3
        };
        for i in 0..spark_count {
            seq.0 = seq.0.wrapping_add(1);
            let jitter = ((seq.0 as f32 * 2.399_963).sin()) * 0.55;
            let fan = (i as f32 - (spark_count as f32 - 1.0) * 0.5) * 0.36 + jitter * 0.25;
            let dir = (blow_dir * (1.0 + jitter.abs() * 0.5)
                + side * fan
                + Vec3::Y * (0.28 + (i as f32 * 0.19).sin().abs() * 0.55))
                .normalize_or(Vec3::Y);
            let speed = if event.critical { 8.5 } else { 6.2 } + (i as f32 * 0.9);
            let mat = if event.critical && i % 2 == 0 {
                art.crit_glint_mat.clone()
            } else {
                el_mat.clone()
            };
            spawn_visceral_entity(
                &mut commands,
                &mut budget,
                "Directional razor spark",
                art.razor_spark.clone(),
                mat,
                Transform::from_translation(body)
                    .looking_to(dir, Vec3::Y)
                    .with_scale(Vec3::new(1.0, 1.0, if event.critical { 1.45 } else { 1.0 })),
                VisceralVfx {
                    elapsed: 0.0,
                    duration: if event.critical { 0.22 } else { 0.16 },
                    velocity: dir * speed,
                    drag: 7.5,
                    start_scale: Vec3::new(1.0, 1.0, if event.critical { 1.45 } else { 1.0 }),
                    end_scale: Vec3::new(0.15, 0.15, 0.20),
                    spin_y: 0.0,
                },
            );
        }

        // 3. Crit Cross-Glint (dual orthogonal razor beams)
        if event.critical {
            let base = Transform::IDENTITY.looking_to(blow_dir, Vec3::Y).rotation;
            for roll in [FRAC_PI_4, -FRAC_PI_4] {
                spawn_visceral_entity(
                    &mut commands,
                    &mut budget,
                    "Crit cross-glint",
                    art.glint_beam.clone(),
                    art.crit_glint_mat.clone(),
                    Transform::from_translation(body)
                        .with_rotation(base * Quat::from_rotation_z(roll))
                        .with_scale(Vec3::new(1.2, 1.2, 1.15)),
                    VisceralVfx {
                        elapsed: 0.0,
                        duration: 0.14,
                        velocity: Vec3::ZERO,
                        drag: 0.0,
                        start_scale: Vec3::new(1.25, 1.25, 1.2),
                        end_scale: Vec3::new(0.12, 0.12, 2.2),
                        spin_y: 0.0,
                    },
                );
            }
        }

        // 4. Kill Finisher Shock-Ring (additive, independent of CombatPools::impact_fx)
        if event.killed {
            let (ring_end, dur) = match event.weight {
                HitWeight::Normal => (1.85, 0.24),
                HitWeight::Elite => (3.1, 0.34),
                HitWeight::Boss => (5.2, 0.48),
            };
            spawn_visceral_entity(
                &mut commands,
                &mut budget,
                "Kill finisher shock-ring",
                art.shock_ring.clone(),
                el_mat,
                Transform::from_translation(event.position.with_y(0.18))
                    .with_scale(Vec3::splat(0.32)),
                VisceralVfx {
                    elapsed: 0.0,
                    duration: dur,
                    velocity: Vec3::ZERO,
                    drag: 0.0,
                    start_scale: Vec3::new(0.32, 0.45, 0.32),
                    end_scale: Vec3::new(ring_end, 0.06, ring_end),
                    spin_y: 2.5,
                },
            );
        }
    }
}

fn tick_motion_states_and_tremors(
    mut commands: Commands,
    time: Res<Time<Real>>,
    session: Res<GameSession>,
    mut motions: Query<&mut MotionState>,
    mut tremors: Query<(Entity, &mut HitTremor)>,
) {
    if session.paused || session.finished {
        return;
    }
    let dt = time.delta_secs().min(0.05);
    for mut motion in &mut motions {
        motion.advance(dt);
    }
    for (entity, mut tremor) in &mut tremors {
        tremor.elapsed += dt;
        if tremor.elapsed >= tremor.duration {
            commands.entity(entity).remove::<HitTremor>();
        }
    }
}

fn animate_visceral_vfx(
    mut commands: Commands,
    time: Res<Time<Real>>,
    session: Res<GameSession>,
    mut effects: Query<(Entity, &mut VisceralVfx, &mut Transform)>,
) {
    if session.finished {
        for (entity, ..) in &effects {
            commands.entity(entity).despawn();
        }
        return;
    }
    if session.paused {
        return;
    }
    let dt = time.delta_secs().min(0.05);
    for (entity, mut vfx, mut transform) in &mut effects {
        vfx.elapsed += dt;
        if vfx.elapsed >= vfx.duration {
            commands.entity(entity).despawn();
            continue;
        }
        let t = (vfx.elapsed / vfx.duration).clamp(0.0, 1.0);
        let damp = (-vfx.drag * dt).exp();
        vfx.velocity *= damp;
        transform.translation += vfx.velocity * dt;
        if vfx.spin_y != 0.0 {
            transform.rotate_local_y(vfx.spin_y * dt);
        }
        let ease = 1.0 - (1.0 - t) * (1.0 - t);
        transform.scale = vfx.start_scale.lerp(vfx.end_scale, ease);
    }
}

/// Applies blended procedural poses + 62.5 Hz victim micro-tremor right before
/// transform propagation and records exact applied deltas for zero-drift restoration.
fn apply_motion_visuals(
    session: Res<GameSession>,
    hit_stop: Option<Res<HitStop>>,
    mut actors: Query<(
        &MotionState,
        Option<&HitTremor>,
        &mut Transform,
        &mut MotionVisualDelta,
    )>,
) {
    if session.finished {
        return;
    }
    let hitstop_active = hit_stop.as_deref().is_some_and(HitStop::active);
    for (motion, tremor, mut transform, mut delta) in &mut actors {
        let pose = motion.blended;
        let local_offset = transform.rotation * pose.root_offset;
        let tremor_offset = tremor.map_or(Vec3::ZERO, |t| t.offset(hitstop_active));
        let total_translation = local_offset + tremor_offset;
        let pose_rot = pose.rotation();
        let scale_mul = Vec3::new(
            pose.scale_mul.x.clamp(0.55, 1.55),
            pose.scale_mul.y.clamp(0.55, 1.55),
            pose.scale_mul.z.clamp(0.55, 1.55),
        );
        transform.translation += total_translation;
        transform.rotation = (transform.rotation * pose_rot).normalize();
        transform.scale *= scale_mul;
        *delta = MotionVisualDelta {
            translation: total_translation,
            rotation: pose_rot,
            scale_mul,
            active: true,
        };
    }
}

fn update_motion_hud_badge(
    session: Res<GameSession>,
    language: Res<GameLanguage>,
    cast_lock: Option<Res<HeroCastLock>>,
    hero_motion: Query<&MotionState, With<HeroVisual>>,
    mut badge: Query<&mut Visibility, With<MotionHudBadge>>,
    mut label: Query<&mut Text, With<MotionHudText>>,
) {
    let Ok(mut vis) = badge.single_mut() else {
        return;
    };
    let Ok(mut text) = label.single_mut() else {
        return;
    };
    let Ok(motion) = hero_motion.single() else {
        vis.set_if_neq(Visibility::Hidden);
        return;
    };
    if session.finished {
        vis.set_if_neq(Visibility::Hidden);
        return;
    }
    let meta = motion.current.meta();
    let cast_lock_label = cast_lock.as_deref().and_then(|lock| {
        if !lock.is_active() {
            return None;
        }
        match lock.kind {
            CastLockKind::Ultimate => Some(match *language {
                GameLanguage::SimplifiedChinese => format!("施法僵直 · {}", meta.zh),
                GameLanguage::English => format!("Cast Lock · {}", meta.en),
            }),
            CastLockKind::SkillSpell => Some(match *language {
                GameLanguage::SimplifiedChinese => format!("施法僵直 · {}", meta.zh),
                GameLanguage::English => format!("Cast Lock · {}", meta.en),
            }),
            _ => None,
        }
    });
    if cast_lock_label.is_none() && !meta.show_badge {
        vis.set_if_neq(Visibility::Hidden);
        return;
    }
    let next = if let Some(lock_text) = cast_lock_label {
        lock_text
    } else if meta.category == MotionCategory::AllyEmote {
        format!("[0] {}", motion.current.bilingual_badge(*language))
    } else {
        motion.current.bilingual_badge(*language)
    };
    text.reborrow().map_unchanged(|t| &mut t.0).set_if_neq(next);
    vis.set_if_neq(Visibility::Visible);
}

pub(crate) fn reset_run(world: &mut World) {
    world.insert_resource(HeroMotionTracker::default());
    world.insert_resource(HeroCastLock::default());
    world.insert_resource(VisceralVfxSequence::default());
    // Restore any active visual deltas before clearing motion states.
    for (mut transform, mut delta) in world
        .query::<(&mut Transform, &mut MotionVisualDelta)>()
        .iter_mut(world)
    {
        if delta.active {
            transform.translation -= delta.translation;
            transform.rotation = (transform.rotation * delta.rotation.inverse()).normalize();
            transform.scale *= Vec3::new(
                if delta.scale_mul.x.abs() > 1e-4 {
                    1.0 / delta.scale_mul.x
                } else {
                    1.0
                },
                if delta.scale_mul.y.abs() > 1e-4 {
                    1.0 / delta.scale_mul.y
                } else {
                    1.0
                },
                if delta.scale_mul.z.abs() > 1e-4 {
                    1.0 / delta.scale_mul.z
                } else {
                    1.0
                },
            );
            *delta = MotionVisualDelta::default();
        }
    }
    for mut motion in world.query::<&mut MotionState>().iter_mut(world) {
        *motion = MotionState::default();
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn catalog_has_at_least_100_unique_valid_bilingual_motions() {
        assert!(
            MotionId::ALL.len() >= 100,
            "expected >= 100 motions, got {}",
            MotionId::ALL.len()
        );
        let mut ids = HashSet::new();
        let mut zh_names = HashSet::new();
        let mut en_names = HashSet::new();
        let mut categories = HashSet::new();
        for id in MotionId::ALL {
            assert!(ids.insert(id), "duplicate MotionId {id:?}");
            let meta = id.meta();
            assert!(!meta.zh.is_empty(), "{id:?} missing Chinese name");
            assert!(!meta.en.is_empty(), "{id:?} missing English name");
            assert!(
                zh_names.insert(meta.zh),
                "duplicate Chinese motion name: {}",
                meta.zh
            );
            assert!(
                en_names.insert(meta.en),
                "duplicate English motion name: {}",
                meta.en
            );
            assert!(
                meta.duration.is_finite() && meta.duration > 0.0,
                "{id:?} invalid duration {}",
                meta.duration
            );
            categories.insert(meta.category);
        }
        assert_eq!(
            categories.len(),
            10,
            "all 10 motion categories must be covered"
        );
    }

    #[test]
    fn charge_stagger_and_ailments_produce_distinct_articulated_poses() {
        let sample_ids = [
            MotionId::IdleReady,
            MotionId::ChargeHoldHeavy,
            MotionId::ChargePeakOverload,
            MotionId::StaggerKneel,
            MotionId::AilmentBurning,
            MotionId::AilmentChilled,
            MotionId::AilmentFrozenRigid,
            MotionId::AilmentStunnedDaze,
            MotionId::AilmentBleeding,
            MotionId::AilmentVulnerable,
            MotionId::AilmentSlowedMud,
            MotionId::AilmentJailedRooted,
            MotionId::AilmentExhaustedGasp,
        ];
        let poses: Vec<_> = sample_ids
            .iter()
            .map(|&id| MotionPose::sample(id, 0.42, 0.37, 1.0))
            .collect();
        for i in 0..poses.len() {
            for j in (i + 1)..poses.len() {
                let a = poses[i];
                let b = poses[j];
                let diff = (a.root_offset - b.root_offset).length()
                    + (a.torso_pitch - b.torso_pitch).abs()
                    + (a.torso_yaw - b.torso_yaw).abs()
                    + (a.torso_roll - b.torso_roll).abs()
                    + (a.scale_mul - b.scale_mul).length();
                assert!(
                    diff > 0.04,
                    "{:?} and {:?} poses are too similar (diff={diff})",
                    sample_ids[i],
                    sample_ids[j]
                );
            }
        }
    }

    #[test]
    fn directional_hitstun_resolves_front_back_left_right_and_crit_stagger() {
        let forward = Vec3::NEG_Z;
        // Blow travelling +Z comes from in front of a -Z facing actor -> HitReactFront
        assert_eq!(
            directional_hit_motion(Vec3::Z, forward, false, false),
            MotionId::HitReactFront
        );
        // Blow travelling -Z strikes from behind -> HitReactBack
        assert_eq!(
            directional_hit_motion(Vec3::NEG_Z, forward, false, false),
            MotionId::HitReactBack
        );
        // Right vector of -Z is +X; blow travelling -X hits the +X (right) side -> HitReactRight
        assert_eq!(
            directional_hit_motion(Vec3::NEG_X, forward, false, false),
            MotionId::HitReactRight
        );
        // Blow travelling +X hits the -X (left) side -> HitReactLeft
        assert_eq!(
            directional_hit_motion(Vec3::X, forward, false, false),
            MotionId::HitReactLeft
        );
        assert_eq!(
            directional_hit_motion(Vec3::Z, forward, true, false),
            MotionId::StaggerKneel
        );
        assert_eq!(
            directional_hit_motion(Vec3::Z, forward, true, true),
            MotionId::BossStaggerBroken
        );
    }

    #[test]
    fn anticipation_release_overshoot_curve_coils_peaks_and_settles() {
        let windup = anticipation_release_overshoot(0.18);
        let peak = anticipation_release_overshoot(0.49);
        let end = anticipation_release_overshoot(1.0);
        assert!(windup < -0.2, "windup must coil negative: {windup}");
        assert!(peak > 1.0, "release must overshoot above 1.0: {peak}");
        assert!(end.abs() < 1e-4, "curve must settle cleanly to 0: {end}");
    }

    #[test]
    fn skid_stop_inertia_triggers_when_halting_from_sprint() {
        let mut motion = MotionState::default();
        assert!(!motion.evaluate_skid_stop(4.5));
        assert!(motion.evaluate_skid_stop(0.1));
        assert_eq!(motion.current, MotionId::StopSkidRecover);
        let pose = MotionPose::sample(MotionId::StopSkidRecover, 0.15, 0.05, 1.15);
        assert!(
            pose.torso_pitch > 0.2,
            "skid-stop must lean back against momentum"
        );
        assert!(pose.root_offset.y < -0.05, "skid-stop must crouch low");
    }

    #[test]
    fn combo_smear_mesh_is_valid_tapered_crescent_with_three_distinct_angles() {
        let mesh = build_combo_smear_mesh(2.15, 0.56, 1.15, 18);
        let positions = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .expect("smear mesh has positions");
        assert_eq!(positions.len(), 38);
        let a0 = combo_smear_roll_angle(0);
        let a1 = combo_smear_roll_angle(1);
        let a2 = combo_smear_roll_angle(2);
        assert!(a0 < 0.0 && a1 > 0.0 && a2 == 0.0);
    }

    #[test]
    fn hit_tremor_and_motion_visual_delta_restore_with_zero_drift() {
        let mut app = App::new();
        app.init_resource::<GameSession>()
            .init_resource::<HitStop>()
            .add_systems(First, restore_motion_visuals)
            .add_systems(PostUpdate, apply_motion_visuals);
        let base = Transform::from_xyz(3.5, 0.0, -2.25)
            .with_rotation(Quat::from_rotation_y(0.6))
            .with_scale(Vec3::splat(1.2));
        let mut state = MotionState::default();
        state.force_play(MotionId::SwordCombo3Finisher, 1.2);
        state.advance(0.14);
        let mut tremor = HitTremor::new(Vec3::X, 0.09, 0.14);
        tremor.elapsed = 0.004; // quarter-cycle of 62.5 Hz -> peak lateral displacement
        let entity = app
            .world_mut()
            .spawn((base, state, tremor, MotionVisualDelta::default()))
            .id();
        app.update();
        let posed = *app.world().get::<Transform>(entity).unwrap();
        assert!(
            posed.translation.distance(base.translation) > 0.02,
            "visual pose + micro-tremor must displace rendered transform"
        );
        // Run First restore only to verify exact restoration before gameplay runs
        app.world_mut().run_schedule(First);
        let restored = *app.world().get::<Transform>(entity).unwrap();
        assert!(
            restored.translation.distance(base.translation) < 1e-5,
            "translation drifted: {:?} vs {:?}",
            restored.translation,
            base.translation
        );
        assert!(
            restored.rotation.angle_between(base.rotation) < 1e-4,
            "rotation drifted"
        );
        assert!(restored.scale.distance(base.scale) < 1e-5, "scale drifted");
    }

    #[test]
    fn visceral_vfx_materials_are_strictly_additive_and_unlit() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_systems(Startup, setup_visceral_vfx_art);
        app.update();
        let mats = app.world().resource::<Assets<StandardMaterial>>();
        assert!(!mats.is_empty());
        for (_, mat) in mats.iter() {
            assert_eq!(mat.alpha_mode, AlphaMode::Add);
            assert!(mat.unlit);
        }
    }

    #[test]
    fn digit0_emote_cycle_and_motion_hud_badge_reflect_active_state() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<GameSession>()
            .init_resource::<GameLanguage>()
            .init_resource::<HeroVitals>()
            .init_resource::<SelectedHero>()
            .init_resource::<DodgeState>()
            .init_resource::<MoveIntent>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<HeroMotionTracker>()
            .init_resource::<HeroCastLock>()
            .add_systems(Startup, setup_motion_hud_badge)
            .add_systems(
                Update,
                (drive_hero_motion_and_smears, update_motion_hud_badge).chain(),
            );
        app.world_mut().spawn((Hero, Transform::default()));
        let visual = app
            .world_mut()
            .spawn((HeroVisual, MotionState::default(), Transform::default()))
            .id();
        app.update();
        // Neutral IdleReady keeps badge hidden (non-intrusive).
        let badge_vis = app
            .world_mut()
            .query_filtered::<&Visibility, With<MotionHudBadge>>()
            .single(app.world())
            .copied()
            .unwrap();
        assert_eq!(badge_vis, Visibility::Hidden);

        // Press Digit0 to trigger first emote and reveal bilingual badge.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Digit0);
        app.update();
        assert_eq!(
            app.world().get::<MotionState>(visual).unwrap().current,
            HERO_EMOTE_CYCLE[0]
        );
        let badge_vis = app
            .world_mut()
            .query_filtered::<&Visibility, With<MotionHudBadge>>()
            .single(app.world())
            .copied()
            .unwrap();
        assert_eq!(badge_vis, Visibility::Visible);
        let text = app
            .world_mut()
            .query_filtered::<&Text, With<MotionHudText>>()
            .single(app.world())
            .unwrap()
            .0
            .clone();
        assert!(text.contains(HERO_EMOTE_CYCLE[0].zh_name()));
        assert!(text.contains(HERO_EMOTE_CYCLE[0].en_name()));
    }

    #[test]
    fn enemy_ailments_and_boss_phase_cues_wire_live_into_motion_state() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<GameSession>()
            .init_resource::<SkillRuntime>()
            .add_message::<EnemyAttackCue>()
            .add_message::<BossPhaseCue>()
            .add_systems(Update, drive_enemy_boss_summon_motions);
        let boss = app
            .world_mut()
            .spawn((
                Enemy {
                    hp: 500.0,
                    max_hp: 500.0,
                    speed: 2.0,
                    damage: 25.0,
                    xp: 0,
                    wood: 0,
                    gold: 0,
                    kill_credit: 0,
                    elite: true,
                },
                BossEnemy { index: 0 },
                Transform::default(),
                MotionState::default(),
            ))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<MotionState>(boss).unwrap().current,
            MotionId::BossEntranceRoar
        );
        app.world_mut().write_message(BossPhaseCue {
            boss,
            index: 0,
            origin: Vec3::ZERO,
        });
        app.update();
        assert_eq!(
            app.world().get::<MotionState>(boss).unwrap().current,
            MotionId::BossPhaseShiftErupt
        );

        let burned = app
            .world_mut()
            .spawn((
                Enemy {
                    hp: 100.0,
                    max_hp: 100.0,
                    speed: 1.5,
                    damage: 10.0,
                    xp: 0,
                    wood: 0,
                    gold: 0,
                    kill_credit: 0,
                    elite: false,
                },
                Transform::default(),
                MotionState::default(),
            ))
            .id();
        app.world_mut()
            .resource_mut::<SkillRuntime>()
            .ignite_relic(burned, 30.0);
        app.update();
        assert_eq!(
            app.world().get::<MotionState>(burned).unwrap().current,
            MotionId::AilmentBurning
        );
    }

    #[test]
    fn cast_lock_enforces_windup_stiffness_priority_and_dodge_cancel() {
        let mut lock = HeroCastLock::default();
        assert!(!lock.is_active());
        assert!((lock.move_speed_scale() - 1.0).abs() < 1e-5);
        assert!(lock.locked_aim_dir().is_none());

        // Spell cast engages near-rooted windup, locks facing, and slows cast apex clip.
        lock.trigger(CastLockKind::SkillSpell, 0.22, 0.08, Vec3::X, Some(2));
        assert!(lock.is_active());
        assert!(lock.triggers_cast_pose());
        assert!(lock.move_speed_scale() < 0.15);
        assert_eq!(lock.locked_aim_dir(), Some(Vec3::X));

        // Lower-priority basic attack cannot overwrite an active spell cast lock.
        let serial_before = lock.cast_serial;
        lock.trigger(CastLockKind::BasicAttack, 0.11, 0.04, Vec3::Z, None);
        assert_eq!(lock.kind, CastLockKind::SkillSpell);
        assert_eq!(lock.cast_serial, serial_before);

        // Apex window applies tactile cast-stiffness animation hold.
        lock.tick(0.07);
        assert!(lock.animation_speed_scale() < 0.7);

        // Recovery eases movement back up smoothly before expiring.
        lock.tick(0.10);
        assert!(lock.is_active());
        assert!(lock.move_speed_scale() > 0.35 && lock.move_speed_scale() < 0.8);

        // Higher-priority Ultimate overrides spell recovery and roots at 0.0 during windup.
        lock.trigger(CastLockKind::Ultimate, 0.40, 0.16, Vec3::NEG_Z, None);
        assert_eq!(lock.kind, CastLockKind::Ultimate);
        assert_eq!(lock.move_speed_scale(), 0.0);
        assert_eq!(lock.locked_aim_dir(), Some(Vec3::NEG_Z));

        // Dodge roll immediately breaks out of cast stiffness.
        lock.cancel_by_dodge();
        assert!(!lock.is_active());
        assert!((lock.move_speed_scale() - 1.0).abs() < 1e-5);
        assert!(lock.locked_aim_dir().is_none());
    }
}
