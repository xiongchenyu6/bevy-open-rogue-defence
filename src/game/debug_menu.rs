use bevy::prelude::*;

use super::combat::{
    CombatSet, DEBUG_BOSS_COUNT, DEBUG_ELITE_AFFIX_COUNT, DEBUG_ENEMY_COUNT, DebugSpawnRequest,
    debug_elite_affix_label, spawn_debug_request,
};
use super::content::{BOSSES, ENEMIES};
use super::pooling::CombatPools;
use super::{GameAssets, GameSession};

pub struct DebugMenuPlugin;

impl Plugin for DebugMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugSpawnMenu>()
            .add_systems(Startup, setup_debug_menu)
            .add_systems(Update, toggle_debug_menu.before(CombatSet))
            .add_systems(
                Update,
                handle_debug_menu
                    .run_if(debug_menu_open)
                    .after(toggle_debug_menu)
                    .before(CombatSet),
            );
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum DebugSpawnMode {
    #[default]
    Normal,
    Elite,
    Boss,
}

impl DebugSpawnMode {
    const fn next(self) -> Self {
        match self {
            Self::Normal => Self::Elite,
            Self::Elite => Self::Boss,
            Self::Boss => Self::Normal,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Normal => "NORMAL ENEMY",
            Self::Elite => "ELITE ENEMY",
            Self::Boss => "BOSS",
        }
    }
}

#[derive(Resource, Debug)]
struct DebugSpawnMenu {
    open: bool,
    mode: DebugSpawnMode,
    enemy_index: usize,
    elite_affix_index: usize,
    boss_index: usize,
    spawned: u32,
    status: String,
}

impl Default for DebugSpawnMenu {
    fn default() -> Self {
        Self {
            open: false,
            mode: DebugSpawnMode::Normal,
            enemy_index: 0,
            elite_affix_index: 0,
            boss_index: 0,
            spawned: 0,
            status: "Ready".to_string(),
        }
    }
}

impl DebugSpawnMenu {
    const fn selected_request(&self) -> DebugSpawnRequest {
        match self.mode {
            DebugSpawnMode::Normal => DebugSpawnRequest::Enemy {
                archetype: self.enemy_index,
                elite_affix: None,
            },
            DebugSpawnMode::Elite => DebugSpawnRequest::Enemy {
                archetype: self.enemy_index,
                elite_affix: Some(self.elite_affix_index),
            },
            DebugSpawnMode::Boss => DebugSpawnRequest::Boss {
                index: self.boss_index,
            },
        }
    }
}

#[derive(Component)]
struct DebugMenuRoot;

#[derive(Component)]
struct DebugMenuText;

fn setup_debug_menu(mut commands: Commands, state: Res<DebugSpawnMenu>) {
    commands
        .spawn((
            DebugMenuRoot,
            Node {
                position_type: PositionType::Absolute,
                left: px(18),
                top: px(92),
                width: px(520),
                min_height: px(190),
                padding: UiRect::all(px(16)),
                border: UiRect::all(px(2)),
                border_radius: BorderRadius::all(px(10)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.015, 0.025, 0.045, 0.96)),
            BorderColor::all(Color::srgba(0.25, 0.9, 0.76, 0.9)),
            Visibility::Hidden,
            Pickable::IGNORE,
            GlobalZIndex(200),
        ))
        .with_children(|root| {
            root.spawn((
                DebugMenuText,
                Text::new(menu_text(&state)),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::srgb(0.82, 0.95, 0.92)),
                TextLayout::linebreak(LineBreak::WordBoundary),
                Pickable::IGNORE,
            ));
        });
}

fn toggle_debug_menu(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<DebugSpawnMenu>,
    mut roots: Query<&mut Visibility, With<DebugMenuRoot>>,
    mut labels: Query<&mut Text, With<DebugMenuText>>,
) {
    if !keyboard.just_pressed(KeyCode::F7) {
        return;
    }
    state.open = !state.open;
    for mut visibility in &mut roots {
        *visibility = if state.open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if state.open {
        update_label(&state, &mut labels);
    }
}

fn debug_menu_open(state: Res<DebugSpawnMenu>) -> bool {
    state.open
}

fn handle_debug_menu(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    session: Res<GameSession>,
    assets: Option<Res<GameAssets>>,
    mut pools: ResMut<CombatPools>,
    mut state: ResMut<DebugSpawnMenu>,
    mut labels: Query<&mut Text, With<DebugMenuText>>,
) {
    let mut changed = false;
    if keyboard.just_pressed(KeyCode::Tab) {
        state.mode = state.mode.next();
        changed = true;
    }

    let previous =
        keyboard.just_pressed(KeyCode::BracketLeft) || keyboard.just_pressed(KeyCode::Numpad4);
    let next =
        keyboard.just_pressed(KeyCode::BracketRight) || keyboard.just_pressed(KeyCode::Numpad6);
    if previous || next {
        let direction = if previous { -1 } else { 1 };
        match state.mode {
            DebugSpawnMode::Normal | DebugSpawnMode::Elite => {
                state.enemy_index = wrap_index(state.enemy_index, direction, DEBUG_ENEMY_COUNT);
            }
            DebugSpawnMode::Boss => {
                state.boss_index = wrap_index(state.boss_index, direction, DEBUG_BOSS_COUNT);
            }
        }
        changed = true;
    }

    if state.mode == DebugSpawnMode::Elite {
        let previous_affix =
            keyboard.just_pressed(KeyCode::Minus) || keyboard.just_pressed(KeyCode::Numpad2);
        let next_affix =
            keyboard.just_pressed(KeyCode::Equal) || keyboard.just_pressed(KeyCode::Numpad8);
        if previous_affix || next_affix {
            let direction = if previous_affix { -1 } else { 1 };
            state.elite_affix_index =
                wrap_index(state.elite_affix_index, direction, DEBUG_ELITE_AFFIX_COUNT);
            changed = true;
        }
    }

    let spawn_requested =
        keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::NumpadEnter);
    if spawn_requested {
        if !spawn_allowed(&session) {
            state.status = "BLOCKED: spawning is disabled while paused or finished".to_string();
        } else if let Some(assets) = assets.as_deref() {
            let request = state.selected_request();
            if spawn_debug_request(&mut commands, &mut pools, assets, request, session.wave)
                .is_some()
            {
                state.spawned = state.spawned.saturating_add(1);
                state.status = format!("Spawned {}", selection_label(&state));
            } else {
                state.status = "BLOCKED: enemy scene pool is at capacity".to_string();
            }
        } else {
            state.status = "BLOCKED: game assets are not ready".to_string();
        }
        changed = true;
    }

    if changed {
        update_label(&state, &mut labels);
    }
}

fn update_label(state: &DebugSpawnMenu, labels: &mut Query<&mut Text, With<DebugMenuText>>) {
    let text = menu_text(state);
    for mut label in labels {
        **label = text.clone();
    }
}

fn menu_text(state: &DebugSpawnMenu) -> String {
    let affix_help = if state.mode == DebugSpawnMode::Elite {
        "\n- / = or NUM2 / NUM8  affix"
    } else {
        ""
    };
    format!(
        "F7 DEBUG SPAWNER  |  F7 close\n\
         TAB  mode     [ / ] or NUM4 / NUM6  selection{affix_help}\n\
         ENTER / NUM ENTER  spawn\n\n\
         MODE    {}\n\
         TARGET  {}\n\
         STATUS  {}\n\
         SPAWNED {}",
        state.mode.label(),
        selection_label(state),
        state.status,
        state.spawned,
    )
}

fn selection_label(state: &DebugSpawnMenu) -> String {
    match state.mode {
        DebugSpawnMode::Normal => format!(
            "{:02}/{:02} {}",
            state.enemy_index + 1,
            DEBUG_ENEMY_COUNT,
            ENEMIES[state.enemy_index].name,
        ),
        DebugSpawnMode::Elite => format!(
            "{:02}/{:02} {} + {:02}/{:02} {}",
            state.enemy_index + 1,
            DEBUG_ENEMY_COUNT,
            ENEMIES[state.enemy_index].name,
            state.elite_affix_index + 1,
            DEBUG_ELITE_AFFIX_COUNT,
            debug_elite_affix_label(state.elite_affix_index),
        ),
        DebugSpawnMode::Boss => format!(
            "{:02}/{:02} {}",
            state.boss_index + 1,
            DEBUG_BOSS_COUNT,
            BOSSES[state.boss_index].name,
        ),
    }
}

fn wrap_index(index: usize, direction: isize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    (index as isize + direction).rem_euclid(len as isize) as usize
}

fn spawn_allowed(session: &GameSession) -> bool {
    !session.paused && !session.finished
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tap_key(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        let mut keyboard = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keyboard.release(key);
        keyboard.clear();
    }

    #[test]
    fn selection_wraps_in_both_directions() {
        assert_eq!(wrap_index(0, -1, 12), 11);
        assert_eq!(wrap_index(11, 1, 12), 0);
        assert_eq!(wrap_index(2, -1, 5), 1);
        assert_eq!(wrap_index(0, 1, 0), 0);
    }

    #[test]
    fn dispatch_preserves_enemy_while_cycling_all_five_affixes() {
        let mut state = DebugSpawnMenu {
            mode: DebugSpawnMode::Elite,
            enemy_index: 7,
            ..default()
        };
        for affix in 0..DEBUG_ELITE_AFFIX_COUNT {
            state.elite_affix_index = affix;
            assert_eq!(
                state.selected_request(),
                DebugSpawnRequest::Enemy {
                    archetype: 7,
                    elite_affix: Some(affix),
                }
            );
        }

        state.mode = DebugSpawnMode::Normal;
        assert_eq!(
            state.selected_request(),
            DebugSpawnRequest::Enemy {
                archetype: 7,
                elite_affix: None,
            }
        );
        state.mode = DebugSpawnMode::Boss;
        state.boss_index = 4;
        assert_eq!(
            state.selected_request(),
            DebugSpawnRequest::Boss { index: 4 }
        );
    }

    #[test]
    fn paused_and_finished_sessions_block_debug_spawns() {
        assert!(spawn_allowed(&GameSession::default()));
        assert!(!spawn_allowed(&GameSession {
            paused: true,
            ..default()
        }));
        assert!(!spawn_allowed(&GameSession {
            finished: true,
            ..default()
        }));
    }

    #[test]
    fn f7_toggles_overlay_and_closed_or_paused_menu_never_spawns() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<GameSession>()
            .init_resource::<CombatPools>()
            .add_plugins(DebugMenuPlugin);
        app.update();

        let visibility = {
            let world = app.world_mut();
            let mut query = world.query_filtered::<&Visibility, With<DebugMenuRoot>>();
            *query.single(world).expect("one debug overlay")
        };
        assert_eq!(visibility, Visibility::Hidden);

        tap_key(&mut app, KeyCode::F7);
        assert!(app.world().resource::<DebugSpawnMenu>().open);
        let visibility = {
            let world = app.world_mut();
            let mut query = world.query_filtered::<&Visibility, With<DebugMenuRoot>>();
            *query.single(world).expect("one debug overlay")
        };
        assert_eq!(visibility, Visibility::Visible);
        app.world_mut().resource_mut::<GameSession>().paused = true;
        tap_key(&mut app, KeyCode::Enter);
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<super::super::Enemy>>()
                .iter(app.world())
                .count(),
            0
        );
        assert!(
            app.world()
                .resource::<DebugSpawnMenu>()
                .status
                .contains("BLOCKED")
        );

        tap_key(&mut app, KeyCode::F7);
        let status = app.world().resource::<DebugSpawnMenu>().status.clone();
        tap_key(&mut app, KeyCode::Enter);
        assert_eq!(app.world().resource::<DebugSpawnMenu>().status, status);
    }
}
