//! Developer-only spawn dispatch and time-skip hotkey. The parent module only
//! compiles this file in debug builds or with the `dev-tools` feature.

use bevy::prelude::*;

use super::super::content::{BOSSES, ENEMIES};
use super::super::pooling::CombatPools;
use super::super::waves::WAVES;
use super::super::{GameAssets, GameSession, SpawnDirector};
use super::{EliteAffixKind, SpawnBudget, spawn_boss, spawn_enemy, spawn_x_range};

pub(crate) const DEBUG_ENEMY_COUNT: usize = ENEMIES.len();
pub(crate) const DEBUG_ELITE_AFFIX_COUNT: usize = 8;
pub(crate) const DEBUG_BOSS_COUNT: usize = BOSSES.len();

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

pub(crate) fn debug_elite_affix_label(index: usize) -> &'static str {
    elite_affix_from_index(index).label()
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
                    hunted: false,
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
        4 => EliteAffixKind::Commander,
        5 => EliteAffixKind::YellowSign,
        6 => EliteAffixKind::Brood,
        _ => EliteAffixKind::Siege,
    }
}

/// F8 jumps to 4:59 and Shift+F8 to 24:59, re-arming that checkpoint's boss.
pub(super) fn debug_shortcuts(
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
