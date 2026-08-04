#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use bevy::prelude::*;

mod archive;
mod arena;
mod audio;
mod bonds;
mod bosses;
mod combat;
mod components;
pub mod content;
mod debug_menu;
mod hero;
mod hero_abilities;
mod market;
mod mobile_input;
mod pooling;
mod progression;
mod resources;
mod skills;
mod trial;
mod ui;
mod waves;

#[cfg(test)]
mod tests;

pub use archive::{ArchiveOutcome, ArchiveState};
pub use components::*;
pub use pooling::{CombatPoolSnapshot, CombatPools, DamageNumberPoolMetrics, PoolMetrics};
pub use resources::*;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameSession>()
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
            .add_plugins(pooling::PoolPlugin)
            .add_plugins((
                arena::ArenaPlugin,
                hero::HeroPlugin,
                hero_abilities::HeroAbilitiesPlugin,
                mobile_input::MobileInputPlugin,
                audio::GameAudioPlugin,
                combat::CombatPlugin,
                bonds::BondsPlugin,
                bosses::BossesPlugin,
                archive::ArchivePlugin,
                skills::SkillsPlugin,
                progression::ProgressionPlugin,
                trial::TrialPlugin,
                market::MarketPlugin,
                debug_menu::DebugMenuPlugin,
                ui::GameUiPlugin,
            ));
    }
}
