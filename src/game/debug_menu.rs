use bevy::prelude::*;

use super::combat::CombatSet;
use super::combat::debug_tools::{
    DEBUG_BOSS_COUNT, DEBUG_ELITE_AFFIX_COUNT, DEBUG_ENEMY_COUNT, DebugSpawnRequest,
    debug_elite_affix_label, spawn_debug_request,
};
use super::content::{BOSSES, ENEMIES};
use super::pooling::CombatPools;
use super::{GameAssets, GameLanguage, GameSession};

const ELITE_AFFIX_ZH: [&str; DEBUG_ELITE_AFFIX_COUNT] = [
    "狂乱", "硬壳", "爆裂", "血祭", "统御", "黄印", "孵化", "攻城",
];

pub struct DebugMenuPlugin;

impl Plugin for DebugMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugSpawnMenu>()
            .init_resource::<GameLanguage>()
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

    const fn label(self, language: GameLanguage) -> &'static str {
        match self {
            Self::Normal => language.pick("NORMAL ENEMY", "普通敌人"),
            Self::Elite => language.pick("ELITE ENEMY", "精英敌人"),
            Self::Boss => language.pick("BOSS", "首领"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DebugStatus {
    Ready,
    Spawned(DebugSpawnRequest),
    SessionBlocked,
    PoolFull,
    AssetsLoading,
}

#[derive(Resource, Debug)]
struct DebugSpawnMenu {
    open: bool,
    mode: DebugSpawnMode,
    enemy_index: usize,
    elite_affix_index: usize,
    boss_index: usize,
    spawned: u32,
    status: DebugStatus,
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
            status: DebugStatus::Ready,
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

fn setup_debug_menu(
    mut commands: Commands,
    state: Res<DebugSpawnMenu>,
    language: Res<GameLanguage>,
) {
    commands
        .spawn((
            DebugMenuRoot,
            Name::new("Debug menu"),
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
                Text::new(menu_text(&state, *language)),
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
    language: Res<GameLanguage>,
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
        update_label(&state, *language, &mut labels);
    }
}

fn debug_menu_open(state: Res<DebugSpawnMenu>) -> bool {
    state.open
}

fn handle_debug_menu(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    language: Res<GameLanguage>,
    session: Res<GameSession>,
    assets: Option<Res<GameAssets>>,
    mut pools: ResMut<CombatPools>,
    mut state: ResMut<DebugSpawnMenu>,
    mut labels: Query<&mut Text, With<DebugMenuText>>,
) {
    let mut changed = language.is_changed();
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
            state.status = DebugStatus::SessionBlocked;
        } else if let Some(assets) = assets.as_deref() {
            let request = state.selected_request();
            if spawn_debug_request(&mut commands, &mut pools, assets, request, session.wave)
                .is_some()
            {
                state.spawned = state.spawned.saturating_add(1);
                state.status = DebugStatus::Spawned(request);
            } else {
                state.status = DebugStatus::PoolFull;
            }
        } else {
            state.status = DebugStatus::AssetsLoading;
        }
        changed = true;
    }

    if changed {
        update_label(&state, *language, &mut labels);
    }
}

fn update_label(
    state: &DebugSpawnMenu,
    language: GameLanguage,
    labels: &mut Query<&mut Text, With<DebugMenuText>>,
) {
    let text = menu_text(state, language);
    for mut label in labels {
        **label = text.clone();
    }
}

fn status_label(status: DebugStatus, language: GameLanguage) -> String {
    match status {
        DebugStatus::Ready => language.pick("Ready", "就绪").to_string(),
        DebugStatus::Spawned(request) => format!(
            "{} {}",
            language.pick("Spawned", "已生成"),
            request_label(request, language),
        ),
        DebugStatus::SessionBlocked => language
            .pick(
                "BLOCKED: spawning is disabled while paused or finished",
                "无法生成：游戏已暂停或结束",
            )
            .to_string(),
        DebugStatus::PoolFull => language
            .pick(
                "BLOCKED: enemy scene pool is at capacity",
                "无法生成：敌人数量已达上限",
            )
            .to_string(),
        DebugStatus::AssetsLoading => language
            .pick(
                "BLOCKED: game assets are not ready",
                "无法生成：游戏资源尚未就绪",
            )
            .to_string(),
    }
}

fn menu_text(state: &DebugSpawnMenu, language: GameLanguage) -> String {
    let affix_help = if state.mode == DebugSpawnMode::Elite {
        language.pick(
            "\n- / = or NUM2 / NUM8  affix",
            "\n- / = 或小键盘 2 / 8  切换词缀",
        )
    } else {
        ""
    };
    let header = language.pick(
        "F7 DEBUG SPAWNER  |  F7 close\nTAB  mode     [ / ] or NUM4 / NUM6  selection",
        "F7 调试生成器  |  F7 关闭\nTAB 切换模式    [ / ] 或小键盘 4 / 6  选择目标",
    );
    format!(
        "{header}{affix_help}\n{}\n\n{}    {}\n{}  {}\n{}  {}\n{} {}",
        language.pick("ENTER / NUM ENTER  spawn", "回车 / 小键盘回车  生成"),
        language.pick("MODE", "模式"),
        state.mode.label(language),
        language.pick("TARGET", "目标"),
        request_label(state.selected_request(), language),
        language.pick("STATUS", "状态"),
        status_label(state.status, language),
        language.pick("SPAWNED", "已生成数量"),
        state.spawned,
    )
}

fn request_label(request: DebugSpawnRequest, language: GameLanguage) -> String {
    match request {
        DebugSpawnRequest::Enemy {
            archetype,
            elite_affix,
        } => {
            let enemy = &ENEMIES[archetype];
            let name = if language.is_chinese() {
                enemy.name.to_string()
            } else {
                enemy.id.replace('_', " ").to_ascii_uppercase()
            };
            let mut label = format!("{:02}/{:02} {}", archetype + 1, DEBUG_ENEMY_COUNT, name);
            if let Some(index) = elite_affix {
                let affix = if language.is_chinese() {
                    ELITE_AFFIX_ZH[index % DEBUG_ELITE_AFFIX_COUNT]
                } else {
                    debug_elite_affix_label(index)
                };
                label.push_str(&format!(
                    " + {:02}/{:02} {}",
                    index + 1,
                    DEBUG_ELITE_AFFIX_COUNT,
                    affix
                ));
            }
            label
        }
        DebugSpawnRequest::Boss { index } => {
            let boss = &BOSSES[index];
            let name = language.pick(boss.name_en, boss.name);
            format!("{:02}/{:02} {}", index + 1, DEBUG_BOSS_COUNT, name)
        }
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

pub(crate) fn reset_run(world: &mut World) {
    world.insert_resource(DebugSpawnMenu::default());
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
    fn spawned_status_retains_target_and_translates_affix_after_language_switch() {
        let request = DebugSpawnRequest::Enemy {
            archetype: 2,
            elite_affix: Some(0),
        };
        let status = DebugStatus::Spawned(request);
        let english = status_label(status, GameLanguage::English);
        let chinese = status_label(status, GameLanguage::SimplifiedChinese);
        assert!(english.contains("Spawned"));
        assert!(english.contains("FRENZY"));
        assert!(!english.contains(ENEMIES[2].name));
        assert!(chinese.contains("已生成"));
        assert!(chinese.contains(ENEMIES[2].name));
        assert!(chinese.contains("狂乱"));
        assert!(!chinese.contains("FRENZY"));
    }

    #[test]
    fn all_eight_elites_can_be_selected_and_reported_in_both_languages() {
        let chinese: Vec<_> = (0..DEBUG_ELITE_AFFIX_COUNT)
            .map(|index| {
                let request = DebugSpawnRequest::Enemy {
                    archetype: 0,
                    elite_affix: Some(index),
                };
                let english = request_label(request, GameLanguage::English);
                assert!(english.contains(debug_elite_affix_label(index)));
                let label = request_label(request, GameLanguage::SimplifiedChinese);
                assert_eq!(
                    status_label(
                        DebugStatus::Spawned(request),
                        GameLanguage::SimplifiedChinese
                    ),
                    format!("已生成 {label}")
                );
                label
            })
            .collect();
        assert!(chinese[5].contains("黄印"));
        assert!(chinese[6].contains("孵化"));
        assert!(chinese[7].contains("攻城"));
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
        assert_eq!(
            app.world().resource::<DebugSpawnMenu>().status,
            DebugStatus::SessionBlocked,
        );

        *app.world_mut().resource_mut::<GameLanguage>() = GameLanguage::SimplifiedChinese;
        app.update();
        let localized = {
            let world = app.world_mut();
            let mut labels = world.query_filtered::<&Text, With<DebugMenuText>>();
            labels.single(world).expect("one debug label").0.clone()
        };
        assert!(localized.contains("调试生成器"));
        assert!(localized.contains("无法生成：游戏已暂停或结束"));
        assert!(localized.contains(ENEMIES[0].name));
        assert!(!localized.contains("BLOCKED"));

        *app.world_mut().resource_mut::<GameLanguage>() = GameLanguage::English;
        app.update();
        let localized = {
            let world = app.world_mut();
            let mut labels = world.query_filtered::<&Text, With<DebugMenuText>>();
            labels.single(world).expect("one debug label").0.clone()
        };
        assert!(localized.contains("DEBUG SPAWNER"));
        assert!(localized.contains("BLOCKED"));
        assert!(!localized.contains(ENEMIES[0].name));

        tap_key(&mut app, KeyCode::F7);
        let status = app.world().resource::<DebugSpawnMenu>().status;
        tap_key(&mut app, KeyCode::Enter);
        assert_eq!(app.world().resource::<DebugSpawnMenu>().status, status);
    }
}
