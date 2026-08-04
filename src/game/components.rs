use bevy::prelude::*;

#[derive(Component)]
pub struct Hero;

/// Marks every active player-owned summon, regardless of which gameplay
/// module created it. The marker is removed while a pooled shell is idle.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlayerSummon;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum HeroKind {
    StormSavant,
    EmberRanger,
    StoneWarden,
    SpiritKeeper,
    TideWalker,
    StarWagerer,
}

impl HeroKind {
    pub const ALL: [Self; 6] = [
        Self::StormSavant,
        Self::EmberRanger,
        Self::StoneWarden,
        Self::SpiritKeeper,
        Self::TideWalker,
        Self::StarWagerer,
    ];

    pub const fn from_index(index: usize) -> Self {
        let clamped = if index < Self::ALL.len() {
            index
        } else {
            Self::ALL.len() - 1
        };
        Self::ALL[clamped]
    }

    pub const fn index(self) -> usize {
        self as usize
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeroIdentity(pub HeroKind);

#[derive(Component)]
pub struct HeroVisual;

#[derive(Component)]
pub struct Core;

#[derive(Component)]
pub struct ArenaEntity;

#[derive(Component)]
pub struct Enemy {
    pub hp: f32,
    pub max_hp: f32,
    pub speed: f32,
    pub damage: f32,
    pub xp: u32,
    pub wood: u32,
    pub gold: u32,
    pub kill_credit: u32,
    pub elite: bool,
}

#[derive(Component)]
pub struct EnemyArchetype(pub u8);

/// Per-enemy cadence for archetypes that leave the gate lane to pressure the hero.
#[derive(Component, Debug, Clone, Copy)]
pub struct EnemyHeroThreat {
    pub attack_remaining: f32,
    pub emerged: bool,
    pub stun_applied: bool,
}

impl EnemyHeroThreat {
    pub const fn ranged() -> Self {
        Self {
            attack_remaining: 0.35,
            emerged: true,
            stun_applied: false,
        }
    }

    pub const fn burrowed() -> Self {
        Self {
            attack_remaining: 0.0,
            emerged: false,
            stun_applied: false,
        }
    }
}

#[derive(Component)]
pub struct BossEnemy {
    pub index: u8,
}

#[derive(Component)]
pub struct TrialEnemy {
    pub level: u8,
    pub reward: u32,
    pub kind: ChallengeKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChallengeKind {
    Tower,
    Gold,
}

#[derive(Component)]
pub struct Projectile {
    pub velocity: Vec3,
    pub damage: f32,
    pub radius: f32,
    pub remaining_hits: u8,
    pub lifetime: f32,
}

#[derive(Component)]
pub struct ImpactFx {
    pub lifetime: f32,
    pub start_scale: f32,
    pub end_scale: f32,
}

#[derive(Component)]
pub struct RunStatsText;

#[derive(Component)]
pub struct TimerText;

#[derive(Component)]
pub struct LevelText;

#[derive(Component)]
pub struct CoreBar;

#[derive(Component)]
pub struct XpBar;

#[derive(Component)]
pub struct SkillButtonLabel;

#[derive(Component)]
pub struct HeroVitalsText;

#[derive(Component)]
pub struct DodgeButton;

#[derive(Component)]
pub struct DodgeButtonLabel;

#[derive(Component)]
pub struct UltimateButton;

#[derive(Component)]
pub struct BondDrawButton;

#[derive(Component)]
pub struct BondStatusText;

#[derive(Component)]
pub struct BondButtonLabel;

#[derive(Component)]
pub struct SkillDrawButton;

#[derive(Component)]
pub struct SkillDrawLabel;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrialAction {
    StartTower,
    StartGold,
    ToggleTowerAuto,
    ToggleGoldAuto,
    CancelAll,
}

#[derive(Component)]
pub struct TrialButton(pub TrialAction);

#[derive(Component)]
pub struct TrialStatusText;

#[derive(Component)]
pub struct GoldChallengeStatusText;

#[derive(Component)]
pub struct DraftRoot;

#[derive(Component)]
pub struct DraftButton(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DraftAction {
    Refresh,
    Abandon,
    Later,
}

#[derive(Component)]
pub struct DraftActionButton(pub DraftAction);

#[derive(Component)]
pub struct ResultRoot;
