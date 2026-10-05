#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use bevy::prelude::*;

mod archive;
mod arena;
mod attack_vfx;
mod audio;
mod basic_attack_details;
mod basic_attacks;
mod basic_relics;
mod encounter_boss_ui;
mod encounter_bosses;
mod energy_art;
pub use basic_attacks::{ClassBasicStats, Ricochet, class_basic_stats};
pub use basic_relics::{BasicRelics, RelicCast};
pub use encounter_bosses::{
    ADVENTURE_BATTLE_SECONDS, ADVENTURE_BOSSES, ADVENTURE_LANDING_SITES, ADVENTURE_LEADER_VISUALS,
    ADVENTURE_OFFER_TIMES, ADVENTURE_SITES, AdventureAbility, AdventureBossChoice,
    AdventureBossDef, AdventureBossState, AdventureCampRequest, CampArrival, EncounterBoss,
    EncounterGuard,
};
mod bond_catalog;
mod bond_feedback;
mod bonds;
mod boss_vfx;
mod bosses;
mod campaign;
mod campaign_progress;
pub(crate) mod campaign_ui;
mod combat;
mod components;
pub mod content;
mod crafting;
mod cultivation;
#[cfg(any(debug_assertions, feature = "dev-tools"))]
mod debug_menu;
mod demon_den;
mod elite_vfx;
mod encounters;
mod enemy_vfx;
mod environment;
mod equipment_ui;
mod gate_feedback;
mod hero;
mod hero_abilities;
mod hero_fusion;
pub mod hero_stats;
mod hit_feedback;
mod localization;
mod market;
mod material_finish;
mod mobile_input;
mod mobile_layout;
mod model_animation;
mod pooling;
mod progression;
mod rarity;
mod resources;
mod run_reset;
mod selection_detail;
pub use selection_detail::SelectionDetail;
mod settings;
mod skill_details;
pub mod skill_evolution;
mod skill_feedback;
mod skill_lighting;
mod skills;
pub(crate) mod summon_movement;
mod trial;
mod ui;
mod ultimate_vfx;
pub mod ur_evolution;
mod waves;

#[cfg(test)]
mod tests;

pub use archive::{ArchiveOutcome, ArchiveState};
pub use bosses::{BossDangerShape, BossPhaseCue, BossTelegraph};
pub use campaign::{CHAPTER_COUNT, CampaignRun, RunRequest, chapter};
pub use campaign_progress::{
    CampaignProgress, CampaignReward, MAX_UPGRADE_LEVEL, RunReward, UPGRADE_PER_LEVEL,
};
pub use campaign_ui::CampaignMenu;
pub use combat::{EliteAffix, EliteAffixKind, ElitePressure, RaidWarning, YELLOW_SIGN_RADIUS};
pub use components::*;
pub use crafting::{CraftingState, RecipeId, craft_recipe};
pub use encounters::{EncounterId, EncounterState};
pub use hero_fusion::HeroFusion;
pub use hero_stats::HeroStatsSnapshot;
pub use localization::{GameLanguage, LocalizationPlugin};
pub use market::{prepare_attribute_growth_market, prepare_crafting_market};
pub use mobile_layout::ControlScheme;
pub use pooling::{CombatPoolSnapshot, CombatPools, DamageNumberPoolMetrics, PoolMetrics};
pub use progression::{SkillEconomy, available_bond_cards, available_bond_cards_at};
pub use resources::*;
pub use settings::SettingsInputSet;
pub use skills::tidal_wave::TidalWave;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        hero_stats::install(app);
        app.init_resource::<CraftingState>()
            .init_resource::<GameSession>()
            .init_resource::<PlayerBuild>()
            .init_resource::<SpawnDirector>()
            .init_resource::<AttackClock>()
            .init_resource::<MoveIntent>()
            .init_resource::<ActiveSkill>()
            .init_resource::<SelectedHero>()
            .init_resource::<HeroVitals>()
            .init_resource::<DodgeState>()
            .init_resource::<TrialState>()
            .init_resource::<GoldChallengeState>()
            .init_resource::<DraftState>()
            .init_resource::<RunSeed>()
            .init_resource::<RunRng>()
            .add_plugins(campaign::CampaignPlugin)
            .add_plugins(campaign_progress::CampaignProgressPlugin)
            .add_plugins(campaign_ui::CampaignUiPlugin)
            .add_plugins(localization::LocalizationPlugin)
            .add_plugins(settings::SettingsPlugin)
            .add_plugins(pooling::PoolPlugin)
            .add_plugins(model_animation::ModelAnimationPlugin)
            .add_plugins(material_finish::MaterialFinishPlugin)
            .add_plugins(environment::EnvironmentArtPlugin)
            .add_plugins(basic_relics::BasicRelicsPlugin)
            .add_plugins(encounter_bosses::EncounterBossesPlugin)
            .add_plugins(encounter_boss_ui::EncounterBossUiPlugin)
            .add_plugins(demon_den::DemonDenPlugin)
            .add_plugins(bond_feedback::BondFeedbackPlugin)
            .add_plugins(skill_feedback::SkillFeedbackPlugin)
            .add_plugins(skill_lighting::SkillLightingPlugin)
            .add_plugins(ultimate_vfx::UltimateVfxPlugin)
            .add_plugins(enemy_vfx::EnemyVfxPlugin)
            .add_plugins(elite_vfx::EliteVfxPlugin)
            .add_plugins(boss_vfx::BossVfxPlugin)
            .add_plugins(cultivation::CultivationPlugin)
            .add_plugins(hero_fusion::HeroFusionPlugin)
            .add_plugins(hit_feedback::HitFeedbackPlugin)
            .add_plugins(gate_feedback::GateFeedbackPlugin)
            .add_plugins((
                arena::ArenaPlugin,
                hero::HeroPlugin,
                hero_abilities::HeroAbilitiesPlugin,
                mobile_input::MobileInputPlugin,
                audio::GameAudioPlugin,
                combat::CombatPlugin,
                encounters::EncountersPlugin,
                bonds::BondsPlugin,
                bosses::BossesPlugin,
                archive::ArchivePlugin,
                skills::SkillsPlugin,
                progression::ProgressionPlugin,
                trial::TrialPlugin,
                market::MarketPlugin,
                ui::GameUiPlugin,
            ));
        #[cfg(any(debug_assertions, feature = "dev-tools"))]
        app.add_plugins(debug_menu::DebugMenuPlugin);
    }
}

pub use skill_evolution::UrEvolutionPath;
pub use skills::ur_evolutions::UrBranchCue;
