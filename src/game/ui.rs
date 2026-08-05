use bevy::{prelude::*, window::PrimaryWindow};

use super::archive::{ArchiveEcho, ArchiveOutcome, ArchiveState};
use super::content::HEROES;
use super::localization::{bond_name, boss_name, hero_ultimate_name};
use super::mobile_layout::hud_metrics;
use super::progression::{SkillEconomy, bond_cost, skill_cost};
use super::trial::{GOLD_LEVEL_COUNT, GOLD_TARGET_COUNT, GOLD_UNLOCK_AT, TOWER_UNLOCK_AT};
use super::{
    ActiveSkill, BondButtonLabel, BondDrawButton, BondId, BondStatusText, BossEnemy, CoreBar,
    DamageMeter, DodgeButton, DodgeButtonLabel, DodgeState, Enemy, GameLanguage, GameSession,
    GoldChallengeState, GoldChallengeStatusText, HeroVitals, HeroVitalsText, LevelText,
    PlayerBuild, ResultRoot, RunSeed, RunStatsText, SkillButtonLabel, SkillDrawButton,
    SkillDrawLabel, TimerText, TrialAction, TrialButton, TrialState, TrialStatusText,
    UltimateButton, XpBar,
};

pub struct GameUiPlugin;

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ManualPauseState>()
            .init_resource::<TrialDrawerState>()
            .add_systems(Startup, setup_hud)
            .add_systems(
                Update,
                (
                    apply_hud_layout,
                    handle_trial_drawer,
                    handle_manual_pause,
                    update_static_ui_language,
                    update_joystick_visual,
                    update_hud,
                    show_result,
                    update_result_text,
                ),
            );
    }
}

#[derive(Component)]
struct HudRoot;

#[derive(Component)]
struct HudLeft;

#[derive(Component)]
struct CoreIntegrityLabel;

#[derive(Component)]
struct CoreBarTrack;

#[derive(Component)]
struct MovementPad;

#[derive(Component)]
struct MovementKnob;

#[derive(Component)]
struct MovementLabel;

#[derive(Component)]
struct XpPanel;

#[derive(Component)]
struct XpBarTrack;

#[derive(Component)]
struct ActionPanel;

#[derive(Component)]
struct TrialPanel;

#[derive(Component)]
struct TrialButtonRow;

#[derive(Component)]
struct TrialToggleButton;

#[derive(Component)]
struct TrialToggleLabel;

#[derive(Component)]
struct TrialActionLabel(TrialAction);

#[derive(Component)]
struct PauseButton;

#[derive(Component)]
struct PauseResumeButton;

#[derive(Component)]
struct PauseRoot;

#[derive(Component, Clone, Copy)]
enum PauseCopyLabel {
    Title,
    Description,
    Resume,
}

#[derive(Component, Clone, Copy)]
enum ResultCopyLabel {
    Title,
    Summary,
    Archive,
}

#[derive(Resource, Default)]
struct ManualPauseState {
    root: Option<Entity>,
    was_focused: bool,
}

#[derive(Resource, Default)]
struct TrialDrawerState {
    open: bool,
}

fn setup_hud(mut commands: Commands, language: Res<GameLanguage>) {
    let language = *language;
    commands
        .spawn((
            HudRoot,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                padding: UiRect::all(px(12)),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            Pickable::IGNORE,
            GlobalZIndex(10),
        ))
        .with_children(|root| {
            root.spawn(Node {
                width: percent(100),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::FlexStart,
                ..default()
            })
            .with_children(|top| {
                top.spawn((
                    HudLeft,
                    Node {
                        width: px(240),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(4),
                        ..default()
                    },
                ))
                .with_children(|left| {
                    left.spawn((
                        CoreIntegrityLabel,
                        Text::new(language.pick("CORE INTEGRITY", "城门耐久")),
                        TextFont {
                            font_size: FontSize::Px(11.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.62, 0.86, 0.88)),
                    ));
                    left.spawn((
                        CoreBarTrack,
                        Node {
                            width: px(230),
                            height: px(12),
                            padding: UiRect::all(px(2)),
                            border_radius: BorderRadius::MAX,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.72)),
                    ))
                    .with_children(|bar| {
                        bar.spawn((
                            CoreBar,
                            Node {
                                width: percent(100),
                                height: percent(100),
                                border_radius: BorderRadius::MAX,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.12, 0.9, 0.72)),
                        ));
                    });
                    left.spawn((
                        HeroVitalsText,
                        Text::new(language.pick("HERO HP", "英雄生命")),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.35, 1.0, 0.62)),
                    ));
                    left.spawn((
                        RunStatsText,
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                    left.spawn((
                        BondStatusText,
                        Text::new(language.pick("BONDS 0/7", "羁绊 0/7")),
                        TextFont {
                            font_size: FontSize::Px(11.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.96, 0.78, 0.3)),
                    ));
                    left.spawn((
                        Button,
                        BondDrawButton,
                        Node {
                            width: px(146),
                            height: px(48),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            border_radius: BorderRadius::all(px(7)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.36, 0.19, 0.06)),
                    ))
                    .with_children(|button| {
                        button.spawn((
                            BondButtonLabel,
                            Text::new(language.pick("BOND | 10 WOOD", "羁绊 | 10 木材")),
                            TextFont {
                                font_size: FontSize::Px(12.0),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                    });
                });
                top.spawn((
                    Text::new(language.pick("25:00\nWAVE 1", "25:00\n第 1 波")),
                    TimerText,
                    TextFont {
                        font_size: FontSize::Px(20.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.88, 0.95, 1.0)),
                    TextLayout::justify(Justify::Right),
                ));
            });

            root.spawn(Node {
                width: percent(100),
                align_items: AlignItems::FlexEnd,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            })
            .with_children(|bottom| {
                bottom
                    .spawn((
                        MovementPad,
                        Node {
                            width: px(112),
                            height: px(112),
                            position_type: PositionType::Relative,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            border_radius: BorderRadius::MAX,
                            border: UiRect::all(px(2)),
                            ..default()
                        },
                    ))
                    .insert(BackgroundColor(Color::srgba(0.05, 0.14, 0.18, 0.32)))
                    .insert(BorderColor::all(Color::srgba(0.2, 0.8, 0.9, 0.22)))
                    .with_children(|joy| {
                        joy.spawn((
                            MovementKnob,
                            Node {
                                position_type: PositionType::Absolute,
                                left: px(35),
                                top: px(35),
                                width: px(42),
                                height: px(42),
                                border_radius: BorderRadius::MAX,
                                border: UiRect::all(px(2)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.22, 0.78, 0.88, 0.42)),
                            BorderColor::all(Color::srgba(0.55, 0.95, 1.0, 0.72)),
                        ));
                        joy.spawn((
                            MovementLabel,
                            Text::new(language.pick("MOVE", "移动")),
                            TextFont {
                                font_size: FontSize::Px(11.0),
                                ..default()
                            },
                            TextColor(Color::srgba(0.72, 0.9, 0.94, 0.82)),
                            TextLayout::justify(Justify::Center),
                        ));
                    });

                bottom
                    .spawn((
                        XpPanel,
                        Node {
                            width: px(220),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            row_gap: px(7),
                            ..default()
                        },
                    ))
                    .with_children(|xp| {
                        xp.spawn((
                            Text::new(language.pick("LEVEL 1", "等级 1")),
                            LevelText,
                            TextFont {
                                font_size: FontSize::Px(12.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.78, 0.84, 0.96)),
                        ));
                        xp.spawn((
                            XpBarTrack,
                            Node {
                                width: px(204),
                                height: px(13),
                                padding: UiRect::all(px(2)),
                                border_radius: BorderRadius::MAX,
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.72)),
                        ))
                        .with_children(|bar| {
                            bar.spawn((
                                XpBar,
                                Node {
                                    width: percent(0),
                                    height: percent(100),
                                    border_radius: BorderRadius::MAX,
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.42, 0.36, 1.0)),
                            ));
                        });
                        xp.spawn((
                            Button,
                            SkillDrawButton,
                            Node {
                                width: px(142),
                                height: px(48),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                border_radius: BorderRadius::all(px(8)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.12, 0.22, 0.48)),
                        ))
                        .with_children(|button| {
                            button.spawn((
                                SkillDrawLabel,
                                Text::new(language.pick("SKILL | 80 GOLD", "技能 | 80 金币")),
                                TextFont {
                                    font_size: FontSize::Px(13.0),
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });
                    });

                bottom
                    .spawn((
                        ActionPanel,
                        Node {
                            width: px(202),
                            height: px(112),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::SpaceBetween,
                            column_gap: px(12),
                            ..default()
                        },
                    ))
                    .with_children(|controls| {
                        controls
                            .spawn((
                                Button,
                                DodgeButton,
                                Node {
                                    width: px(78),
                                    height: px(78),
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    border_radius: BorderRadius::MAX,
                                    border: UiRect::all(px(3)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.42, 0.16, 0.58, 0.84)),
                                BorderColor::all(Color::srgb(0.78, 0.5, 1.0)),
                            ))
                            .with_children(|button| {
                                button.spawn((
                                    DodgeButtonLabel,
                                    Text::new(language.pick("DODGE\nREADY", "闪避\n就绪")),
                                    TextFont {
                                        font_size: FontSize::Px(12.0),
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                    TextLayout::justify(Justify::Center),
                                ));
                            });
                        controls
                            .spawn((
                                Button,
                                UltimateButton,
                                Node {
                                    width: px(112),
                                    height: px(112),
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    border_radius: BorderRadius::MAX,
                                    border: UiRect::all(px(3)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.02, 0.34, 0.55, 0.78)),
                                BorderColor::all(Color::srgb(0.25, 0.9, 1.0)),
                            ))
                            .with_children(|button| {
                                button.spawn((
                                    SkillButtonLabel,
                                    Text::new(language.pick("STORM\nREADY", "雷域\n就绪")),
                                    TextFont {
                                        font_size: FontSize::Px(14.0),
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                    TextLayout::justify(Justify::Center),
                                ));
                            });
                    });
            });
        });

    commands
        .spawn((
            TrialPanel,
            Node {
                position_type: PositionType::Absolute,
                right: px(22),
                top: px(112),
                width: px(225),
                padding: UiRect::all(px(12)),
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(12)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.025, 0.055, 0.09, 0.82)),
            BorderColor::all(Color::srgba(0.25, 0.75, 0.9, 0.5)),
            GlobalZIndex(15),
        ))
        .with_children(|panel| {
            panel.spawn((
                TrialStatusText,
                Text::new(language.pick("TOWER 1 LOCKED  01:30", "爬塔 1 未解锁 01:30")),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.9, 1.0)),
            ));
            panel
                .spawn((
                    TrialButtonRow,
                    Node {
                        width: percent(100),
                        height: px(54),
                        column_gap: px(6),
                        ..default()
                    },
                ))
                .with_children(|row| {
                    spawn_trial_button(row, TrialAction::StartTower, language, 102.0);
                    spawn_trial_button(row, TrialAction::ToggleTowerAuto, language, 82.0);
                });
            panel.spawn((
                GoldChallengeStatusText,
                Text::new(language.pick("GOLD 1 LOCKED  01:00", "金币试炼 1 未解锁 01:00")),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.82, 0.38)),
            ));
            panel
                .spawn((
                    TrialButtonRow,
                    Node {
                        width: percent(100),
                        height: px(54),
                        column_gap: px(6),
                        ..default()
                    },
                ))
                .with_children(|row| {
                    spawn_trial_button(row, TrialAction::StartGold, language, 102.0);
                    spawn_trial_button(row, TrialAction::ToggleGoldAuto, language, 82.0);
                });
            spawn_trial_button(panel, TrialAction::CancelAll, language, 196.0);
        });

    commands
        .spawn((
            Button,
            TrialToggleButton,
            Node {
                position_type: PositionType::Absolute,
                right: px(12),
                top: px(64),
                width: px(154),
                height: px(48),
                display: Display::None,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(9)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.035, 0.14, 0.20, 0.94)),
            BorderColor::all(Color::srgba(0.25, 0.75, 0.9, 0.65)),
            GlobalZIndex(22),
        ))
        .with_children(|button| {
            button.spawn((
                TrialToggleLabel,
                Text::new(language.pick("CHALLENGES", "挑战")),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::srgb(0.72, 0.92, 1.0)),
            ));
        });

    commands
        .spawn((
            Button,
            PauseButton,
            Node {
                position_type: PositionType::Absolute,
                right: px(150),
                top: px(12),
                width: px(52),
                height: px(48),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(9)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.055, 0.11, 0.17, 0.9)),
            BorderColor::all(Color::srgba(0.52, 0.76, 0.88, 0.65)),
            GlobalZIndex(22),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new("II"),
                TextFont {
                    font_size: FontSize::Px(17.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

fn spawn_trial_button(
    parent: &mut ChildSpawnerCommands,
    action: TrialAction,
    language: GameLanguage,
    width: f32,
) {
    parent
        .spawn((
            Button,
            TrialButton(action),
            Node {
                width: px(width),
                height: px(48),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px(8)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.07, 0.2, 0.28)),
        ))
        .with_children(|button| {
            button.spawn((
                TrialActionLabel(action),
                Text::new(trial_action_label(language, action)),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

fn trial_action_label(language: GameLanguage, action: TrialAction) -> &'static str {
    match action {
        TrialAction::StartTower => language.pick("TOWER", "爬塔"),
        TrialAction::StartGold => language.pick("GOLD", "金币"),
        TrialAction::ToggleTowerAuto | TrialAction::ToggleGoldAuto => language.pick("AUTO", "自动"),
        TrialAction::CancelAll => language.pick("CLEAR", "清除"),
    }
}

fn command_growth_label(language: GameLanguage, growth: super::CommandGrowth) -> &'static str {
    match growth {
        super::CommandGrowth::Attack => language.pick("ATTACK", "攻击"),
        super::CommandGrowth::Vitality => language.pick("MAX HP", "生命"),
        super::CommandGrowth::Insight => language.pick("XP EFF", "悟性"),
    }
}

#[allow(clippy::type_complexity)]
fn apply_hud_layout(
    windows: Query<&Window, With<PrimaryWindow>>,
    drawer: Res<TrialDrawerState>,
    mut nodes: Query<(
        &mut Node,
        Option<&HudRoot>,
        Option<&HudLeft>,
        Option<&CoreBarTrack>,
        Option<&MovementPad>,
        Option<&XpPanel>,
        Option<&XpBarTrack>,
        Option<&ActionPanel>,
        Option<&DodgeButton>,
        Option<&UltimateButton>,
        Option<&TrialPanel>,
        Option<&TrialButtonRow>,
        Option<&TrialButton>,
        Option<&TrialToggleButton>,
        Option<&PauseButton>,
    )>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let layout = hud_metrics(window.width(), window.height());
    let compact = layout.compact;
    let xp_width = compact_xp_width(window.width());
    let compact_dodge_size = compact_dodge_size(layout.joystick_size);
    let action_width = compact_action_width(layout.joystick_size);

    for (
        mut node,
        root,
        left,
        core_track,
        movement_pad,
        xp_panel,
        xp_track,
        action_panel,
        dodge_button,
        ultimate_button,
        trial_panel,
        trial_row,
        trial_button,
        trial_toggle,
        pause_button,
    ) in &mut nodes
    {
        if root.is_some() {
            node.padding = UiRect {
                left: px(layout.safe_x),
                right: px(layout.safe_x),
                top: px(layout.safe_y),
                bottom: px(layout.safe_y),
            };
        } else if left.is_some() {
            node.width = px(if compact { 240 } else { 350 });
            node.row_gap = px(if compact { 4 } else { 7 });
        } else if core_track.is_some() {
            node.width = px(if compact { 230 } else { 330 });
            node.height = px(if compact { 12 } else { 18 });
            node.padding = UiRect::all(px(if compact { 2 } else { 3 }));
        } else if movement_pad.is_some() {
            node.width = px(layout.joystick_size);
            node.height = px(layout.joystick_size);
        } else if xp_panel.is_some() {
            node.width = px(if compact { xp_width } else { 360.0 });
            node.row_gap = px(if compact { 5 } else { 7 });
        } else if xp_track.is_some() {
            node.width = px(if compact { xp_width - 16.0 } else { 340.0 });
        } else if action_panel.is_some() {
            node.width = px(if compact { action_width } else { 260.0 });
            node.height = px(if compact { layout.joystick_size } else { 150.0 });
            node.margin = UiRect::right(px(action_panel_margin_right(compact)));
        } else if dodge_button.is_some() {
            let size = if compact { compact_dodge_size } else { 96.0 };
            node.width = px(size);
            node.height = px(size);
        } else if ultimate_button.is_some() {
            let size = if compact { layout.joystick_size } else { 150.0 };
            node.width = px(size);
            node.height = px(size);
        } else if trial_panel.is_some() {
            node.display = if compact && !drawer.open {
                Display::None
            } else {
                Display::Flex
            };
            node.right = px(if compact { layout.safe_x } else { 22.0 });
            node.top = px(if compact { 118 } else { 112 });
            node.width = px(if compact { 205 } else { 225 });
            node.padding = UiRect::all(px(if compact { 8 } else { 12 }));
            node.row_gap = px(if compact { 4 } else { 8 });
        } else if trial_row.is_some() {
            node.height = px(if compact { 48 } else { 54 });
        } else if let Some(button) = trial_button {
            node.height = px(48);
            node.width = px(match (compact, button.0) {
                (true, TrialAction::StartTower | TrialAction::StartGold) => 102.0,
                (true, TrialAction::ToggleTowerAuto | TrialAction::ToggleGoldAuto) => 81.0,
                (true, TrialAction::CancelAll) => 189.0,
                (false, TrialAction::StartTower | TrialAction::StartGold) => 102.0,
                (false, TrialAction::ToggleTowerAuto | TrialAction::ToggleGoldAuto) => 82.0,
                (false, TrialAction::CancelAll) => 196.0,
            });
        } else if trial_toggle.is_some() {
            node.display = if compact {
                Display::Flex
            } else {
                Display::None
            };
            node.right = px(layout.safe_x);
            node.top = px(64);
        } else if pause_button.is_some() {
            node.right = px(if compact {
                layout.safe_x + 104.0
            } else {
                150.0
            });
            node.top = px(layout.safe_y);
        }
    }
}

fn compact_xp_width(window_width: f32) -> f32 {
    if window_width < 750.0 { 190.0 } else { 220.0 }
}

fn compact_dodge_size(joystick_size: f32) -> f32 {
    (joystick_size * 0.7).max(72.0)
}

fn compact_action_width(joystick_size: f32) -> f32 {
    joystick_size + compact_dodge_size(joystick_size) + 12.0
}

fn action_panel_margin_right(compact: bool) -> f32 {
    if compact { 0.0 } else { 230.0 }
}

fn handle_trial_drawer(
    interactions: Query<&Interaction, (With<TrialToggleButton>, Changed<Interaction>)>,
    mut labels: Query<&mut Text, With<TrialToggleLabel>>,
    mut drawer: ResMut<TrialDrawerState>,
    language: Res<GameLanguage>,
) {
    if interactions
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        drawer.open = !drawer.open;
    }
    for mut label in &mut labels {
        **label = if drawer.open {
            language.pick("CLOSE CHALLENGES", "关闭挑战").to_string()
        } else {
            language.pick("CHALLENGES", "挑战").to_string()
        };
    }
}

fn update_static_ui_language(
    language: Res<GameLanguage>,
    mut core_labels: Query<
        &mut Text,
        (
            With<CoreIntegrityLabel>,
            Without<MovementLabel>,
            Without<TrialActionLabel>,
            Without<PauseCopyLabel>,
        ),
    >,
    mut movement_labels: Query<
        &mut Text,
        (
            With<MovementLabel>,
            Without<CoreIntegrityLabel>,
            Without<TrialActionLabel>,
            Without<PauseCopyLabel>,
        ),
    >,
    mut trial_labels: Query<
        (&TrialActionLabel, &mut Text),
        (
            Without<CoreIntegrityLabel>,
            Without<MovementLabel>,
            Without<PauseCopyLabel>,
        ),
    >,
    mut pause_labels: Query<
        (&PauseCopyLabel, &mut Text),
        (
            Without<CoreIntegrityLabel>,
            Without<MovementLabel>,
            Without<TrialActionLabel>,
        ),
    >,
) {
    if !language.is_changed() {
        return;
    }
    for mut text in &mut core_labels {
        **text = language.pick("CORE INTEGRITY", "城门耐久").to_string();
    }
    for mut text in &mut movement_labels {
        **text = language.pick("MOVE", "移动").to_string();
    }
    for (label, mut text) in &mut trial_labels {
        **text = trial_action_label(*language, label.0).to_string();
    }
    for (label, mut text) in &mut pause_labels {
        **text = match label {
            PauseCopyLabel::Title => language.pick("PAUSED", "已暂停"),
            PauseCopyLabel::Description => {
                language.pick("The battlefield is frozen.", "战场已冻结。")
            }
            PauseCopyLabel::Resume => language.pick("RESUME", "继续战斗"),
        }
        .to_string();
    }
}

fn handle_manual_pause(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    pause_buttons: Query<&Interaction, (With<PauseButton>, Changed<Interaction>)>,
    resume_buttons: Query<&Interaction, (With<PauseResumeButton>, Changed<Interaction>)>,
    mut session: ResMut<GameSession>,
    mut state: ResMut<ManualPauseState>,
    run_seed: Res<RunSeed>,
    language: Res<GameLanguage>,
) {
    let focused = windows
        .single()
        .map(|window| window.focused)
        .unwrap_or(true);
    let lost_focus = state.was_focused && !focused;
    state.was_focused = focused;

    let pause_pressed = keyboard.just_pressed(KeyCode::KeyP)
        || keyboard.just_pressed(KeyCode::Escape)
        || pause_buttons
            .iter()
            .any(|interaction| *interaction == Interaction::Pressed);
    let resume_pressed = resume_buttons
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed);

    if let Some(root) = state.root {
        if pause_pressed || resume_pressed {
            commands.entity(root).despawn();
            state.root = None;
            session.paused = false;
        }
        return;
    }

    if !(pause_pressed || lost_focus) || session.paused || session.finished {
        return;
    }

    session.paused = true;
    let root = commands
        .spawn((
            PauseRoot,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: px(14),
                ..default()
            },
            BackgroundColor(Color::srgba(0.01, 0.025, 0.045, 0.9)),
            GlobalZIndex(90),
        ))
        .with_children(|overlay| {
            overlay.spawn((
                PauseCopyLabel::Title,
                Text::new(language.pick("PAUSED", "已暂停")),
                TextFont {
                    font_size: FontSize::Px(30.0),
                    ..default()
                },
                TextColor(Color::srgb(0.78, 0.94, 1.0)),
            ));
            overlay.spawn((
                PauseCopyLabel::Description,
                Text::new(language.pick("The battlefield is frozen.", "战场已冻结。")),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.68, 0.78, 0.86)),
                TextLayout::new(Justify::Center, LineBreak::WordOrCharacter),
            ));
            overlay.spawn((
                Text::new(format_run_seed(run_seed.0)),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.52, 0.76, 0.86)),
            ));
            overlay
                .spawn((
                    Button,
                    PauseResumeButton,
                    Node {
                        width: px(220),
                        height: px(56),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        border_radius: BorderRadius::all(px(10)),
                        border: UiRect::all(px(2)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.08, 0.32, 0.38)),
                    BorderColor::all(Color::srgb(0.3, 0.88, 0.92)),
                ))
                .with_children(|button| {
                    button.spawn((
                        PauseCopyLabel::Resume,
                        Text::new(language.pick("RESUME", "继续战斗")),
                        TextFont {
                            font_size: FontSize::Px(16.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        })
        .id();
    state.root = Some(root);
}

fn update_joystick_visual(
    windows: Query<&Window, With<PrimaryWindow>>,
    intent: Res<super::MoveIntent>,
    mut knobs: Query<&mut Node, With<MovementKnob>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let layout = hud_metrics(window.width(), window.height());
    let knob_size = if layout.compact { 42.0 } else { 50.0 };
    let center = (layout.joystick_size - knob_size) * 0.5;
    let travel = (center - 6.0).max(0.0);
    for mut node in &mut knobs {
        node.width = px(knob_size);
        node.height = px(knob_size);
        node.left = px(center + intent.0.x * travel);
        node.top = px(center + intent.0.y * travel);
    }
}

fn update_hud(
    windows: Query<&Window, With<PrimaryWindow>>,
    language: Res<GameLanguage>,
    session: Res<GameSession>,
    damage_meter: Res<DamageMeter>,
    run_seed: Res<RunSeed>,
    skill_economy: Res<SkillEconomy>,
    build: Res<PlayerBuild>,
    active: Res<ActiveSkill>,
    vitals: Res<HeroVitals>,
    dodge: Res<DodgeState>,
    trial: Res<TrialState>,
    gold_trial: Res<GoldChallengeState>,
    bosses: Query<(&Enemy, &BossEnemy)>,
    archive_echo: Query<&Enemy, With<ArchiveEcho>>,
    mut text_nodes: Query<(
        &mut Text,
        Option<&RunStatsText>,
        Option<&TimerText>,
        Option<&LevelText>,
        Option<&SkillButtonLabel>,
        Option<&TrialStatusText>,
        Option<&GoldChallengeStatusText>,
        Option<&BondStatusText>,
        Option<&BondButtonLabel>,
        Option<&SkillDrawLabel>,
        Option<&HeroVitalsText>,
        Option<&DodgeButtonLabel>,
    )>,
    mut bars: ParamSet<(
        Query<&mut Node, (With<CoreBar>, Without<XpBar>)>,
        Query<&mut Node, (With<XpBar>, Without<CoreBar>)>,
    )>,
) {
    let language = *language;
    let compact = windows
        .single()
        .map(|window| hud_metrics(window.width(), window.height()).compact)
        .unwrap_or(false);
    let remaining = (session.duration - session.elapsed).max(0.0) as u32;
    let boss_status = bosses.iter().next().map(|(enemy, boss)| {
        format!(
            "{}  {:.0}%",
            boss_name(language, boss.index as usize),
            (enemy.hp / enemy.max_hp * 100.0).clamp(0.0, 100.0)
        )
    });
    let archive_hp = archive_echo
        .iter()
        .next()
        .map(|enemy| (enemy.hp / enemy.max_hp * 100.0).clamp(0.0, 100.0))
        .unwrap_or(100.0);
    let hero_hp = (vitals.health / vitals.max_health.max(1.0) * 100.0).clamp(0.0, 100.0);
    let trial_base = if session.save_active {
        language
            .pick("TOWER SEALED - ARCHIVE ACTIVE", "爬塔封印 · 战果刻印中")
            .to_string()
    } else if trial.active {
        if language.is_chinese() {
            format!("爬塔 {} · {:.1}秒", trial.level + 1, trial.remaining)
        } else {
            format!("TOWER {}  {:.1}s", trial.level + 1, trial.remaining)
        }
    } else if usize::from(trial.level) >= TOWER_UNLOCK_AT.len() {
        language.pick("TOWER COMPLETE", "爬塔完成").to_string()
    } else if session.elapsed < TOWER_UNLOCK_AT[trial.level as usize] {
        let unlock = (TOWER_UNLOCK_AT[trial.level as usize] - session.elapsed).ceil() as u32;
        if language.is_chinese() {
            format!(
                "爬塔 {} 未解锁 {:02}:{:02}",
                trial.level + 1,
                unlock / 60,
                unlock % 60
            )
        } else {
            format!(
                "TOWER {} LOCKED  {:02}:{:02}",
                trial.level + 1,
                unlock / 60,
                unlock % 60
            )
        }
    } else if trial.cooldown > 0.0 {
        if language.is_chinese() {
            format!("爬塔 {} · 冷却 {:.0}秒", trial.level + 1, trial.cooldown)
        } else {
            format!("TOWER {}  CD {:.0}s", trial.level + 1, trial.cooldown)
        }
    } else {
        if language.is_chinese() {
            format!(
                "爬塔 {} 就绪{}",
                trial.level + 1,
                if trial.auto { " [自动]" } else { "" }
            )
        } else {
            format!(
                "TOWER {} READY{}",
                trial.level + 1,
                if trial.auto { "  [AUTO]" } else { "" }
            )
        }
    };
    let next_growth = super::CommandGrowth::from_point(trial.command_points);
    let last_growth = trial
        .last_growth
        .map_or("-", |growth| command_growth_label(language, growth));
    let trial_status = if compact {
        if language.is_chinese() {
            format!("{trial_base} | 指挥 {}", trial.command_points)
        } else {
            format!("{trial_base} | CMD {}", trial.command_points)
        }
    } else if language.is_chinese() {
        format!(
            "{trial_base}\n指挥 {}  攻{} 体{} 悟{}  上次 {}  下次 {}",
            trial.command_points,
            trial.growth_ranks[0],
            trial.growth_ranks[1],
            trial.growth_ranks[2],
            last_growth,
            command_growth_label(language, next_growth),
        )
    } else {
        format!(
            "{trial_base}\nCMD {}  A{} H{} X{}  LAST {}  NEXT {}",
            trial.command_points,
            trial.growth_ranks[0],
            trial.growth_ranks[1],
            trial.growth_ranks[2],
            last_growth,
            command_growth_label(language, next_growth),
        )
    };
    let gold_status = if session.save_active {
        language
            .pick("GOLD SEALED - ARCHIVE ACTIVE", "金币试炼封印 · 战果刻印中")
            .to_string()
    } else if gold_trial.active {
        if language.is_chinese() {
            format!(
                "金币试炼 {} · {:.1}秒 · 目标 {}/{}",
                gold_trial.level + 1,
                gold_trial.remaining,
                gold_trial.remaining_targets,
                GOLD_TARGET_COUNT,
            )
        } else {
            format!(
                "GOLD {}  {:.1}s  TARGETS {}/{}",
                gold_trial.level + 1,
                gold_trial.remaining,
                gold_trial.remaining_targets,
                GOLD_TARGET_COUNT,
            )
        }
    } else if usize::from(gold_trial.level) >= GOLD_LEVEL_COUNT {
        language.pick("GOLD COMPLETE", "金币试炼完成").to_string()
    } else if session.elapsed < GOLD_UNLOCK_AT {
        let unlock = (GOLD_UNLOCK_AT - session.elapsed).ceil() as u32;
        if language.is_chinese() {
            format!(
                "金币试炼 {} 未解锁 {:02}:{:02}",
                gold_trial.level + 1,
                unlock / 60,
                unlock % 60
            )
        } else {
            format!(
                "GOLD {} LOCKED  {:02}:{:02}",
                gold_trial.level + 1,
                unlock / 60,
                unlock % 60
            )
        }
    } else if gold_trial.cooldown > 0.0 {
        if language.is_chinese() {
            format!(
                "金币试炼 {} · 冷却 {:.0}秒",
                gold_trial.level + 1,
                gold_trial.cooldown
            )
        } else {
            format!(
                "GOLD {}  CD {:.0}s",
                gold_trial.level + 1,
                gold_trial.cooldown
            )
        }
    } else {
        if language.is_chinese() {
            format!(
                "金币试炼 {} 就绪{}",
                gold_trial.level + 1,
                if gold_trial.auto { " [自动]" } else { "" }
            )
        } else {
            format!(
                "GOLD {} READY{}",
                gold_trial.level + 1,
                if gold_trial.auto { "  [AUTO]" } else { "" }
            )
        }
    };
    let mut bond_lines = vec![if language.is_chinese() {
        format!("羁绊 {}/7", build.active_bond_count())
    } else {
        format!("BONDS {}/7", build.active_bond_count())
    }];
    for id in BondId::ALL {
        let rank = build.bond(id);
        if rank > 0 && !build.bond_complete(id) {
            bond_lines.push(format!(
                "{}  {}/{}",
                bond_name(language, id),
                rank,
                id.required()
            ));
        }
    }
    if bond_lines.len() == 1 {
        bond_lines.push(
            language
                .pick("No active bond - press F", "暂无羁绊 · 按 F 抽取")
                .to_string(),
        );
    }
    let compact_bonds = build.bonds.iter().filter(|rank| **rank > 0).count();

    for (
        mut text,
        stats,
        timer,
        level,
        skill,
        trial_marker,
        gold_marker,
        bonds,
        bond_button,
        skill_draw,
        hero_vitals_marker,
        dodge_label,
    ) in &mut text_nodes
    {
        **text = if hero_vitals_marker.is_some() {
            if vitals.alive {
                let condition = if vitals.stunned_remaining > 0.0 {
                    if language.is_chinese() {
                        format!("眩晕 {:.1}秒", vitals.stunned_remaining)
                    } else {
                        format!("STUNNED {:.1}s", vitals.stunned_remaining)
                    }
                } else if vitals.invulnerable_remaining > 0.0 {
                    if language.is_chinese() {
                        format!("无敌 {:.1}秒", vitals.invulnerable_remaining)
                    } else {
                        format!("INVULNERABLE {:.1}s", vitals.invulnerable_remaining)
                    }
                } else if language.is_chinese() {
                    "正常".to_string()
                } else {
                    "ACTIVE".to_string()
                };
                if compact && language.is_chinese() {
                    format!(
                        "血 {:.0}/{:.0} | 能 {:.0}/{:.0} | {condition}\n力 {:.0}  敏 {:.0}  智 {:.0}",
                        vitals.health,
                        vitals.max_health,
                        vitals.energy,
                        vitals.max_energy,
                        vitals.strength,
                        vitals.agility,
                        vitals.intelligence,
                    )
                } else if compact {
                    format!(
                        "HP {:.0}/{:.0} | EN {:.0}/{:.0} | {condition}\nSTR {:.0}  AGI {:.0}  INT {:.0}",
                        vitals.health,
                        vitals.max_health,
                        vitals.energy,
                        vitals.max_energy,
                        vitals.strength,
                        vitals.agility,
                        vitals.intelligence,
                    )
                } else if language.is_chinese() {
                    format!(
                        "英雄 {:04.0}/{:04.0} HP  {:03.0}/{:03.0} 能量  [{condition}]\n力量 {:.0}  敏捷 {:.0}  智力 {:.0}",
                        vitals.health,
                        vitals.max_health,
                        vitals.energy,
                        vitals.max_energy,
                        vitals.strength,
                        vitals.agility,
                        vitals.intelligence,
                    )
                } else {
                    format!(
                        "HERO {:04.0}/{:04.0} HP  {:03.0}/{:03.0} EN  [{condition}]\nSTR {:.0}  AGI {:.0}  INT {:.0}",
                        vitals.health,
                        vitals.max_health,
                        vitals.energy,
                        vitals.max_energy,
                        vitals.strength,
                        vitals.agility,
                        vitals.intelligence,
                    )
                }
            } else if language.is_chinese() {
                format!(
                    "英雄倒地 · {:.1}秒复活\n阵亡 {}  |  城门 -80",
                    vitals.revive_remaining, vitals.deaths
                )
            } else {
                format!(
                    "HERO DOWN - REVIVE {:.1}s\nDEATH {}  |  CORE -80",
                    vitals.revive_remaining, vitals.deaths
                )
            }
        } else if dodge_label.is_some() {
            if !vitals.alive {
                language.pick("DODGE\nLOCKED", "闪避\n锁定").to_string()
            } else if vitals.stunned_remaining > 0.0 {
                language.pick("DODGE\nSTUNNED", "闪避\n眩晕").to_string()
            } else if dodge.cooldown_remaining <= 0.0 {
                if compact {
                    language.pick("DODGE\nREADY", "闪避\n就绪").to_string()
                } else if language.is_chinese() {
                    "闪避就绪\nSHIFT / 轻触".to_string()
                } else {
                    "DODGE READY\nSHIFT / TAP".to_string()
                }
            } else if language.is_chinese() {
                format!("闪避\n{:.1}秒", dodge.cooldown_remaining)
            } else {
                format!("DODGE\n{:.1}s", dodge.cooldown_remaining)
            }
        } else if stats.is_some() {
            if compact && language.is_chinese() {
                format!(
                    "金 {:04}  杀 {:04}  木 {:03}\n秒伤 {:05.1}",
                    session.gold,
                    session.kills,
                    session.wood,
                    damage_meter.recent_dps(),
                )
            } else if compact {
                format!(
                    "G {:04}  K {:04}  W {:03}\nDPS {:05.1}",
                    session.gold,
                    session.kills,
                    session.wood,
                    damage_meter.recent_dps(),
                )
            } else if language.is_chinese() {
                format!(
                    "金币 {:04}   击杀 {:04}   木材 {:03}\n10秒伤害 {:06.1}   种子 {:08X}\n雷 {}  火 {}  灵 {}",
                    session.gold,
                    session.kills,
                    session.wood,
                    damage_meter.recent_dps(),
                    run_seed.0 as u32,
                    build.bond(BondId::Thunder),
                    build.bond(BondId::Ember),
                    build.bond(BondId::Spirits)
                )
            } else {
                format!(
                    "GOLD {:04}   KILLS {:04}   WOOD {:03}\nDPS(10s) {:06.1}   SEED {:08X}\nTHUNDER {}  EMBER {}  SPIRITS {}",
                    session.gold,
                    session.kills,
                    session.wood,
                    damage_meter.recent_dps(),
                    run_seed.0 as u32,
                    build.bond(BondId::Thunder),
                    build.bond(BondId::Ember),
                    build.bond(BondId::Spirits)
                )
            }
        } else if timer.is_some() {
            if session.save_active {
                if language.is_chinese() {
                    format!(
                        "刻印 {:02}秒\n回响 {:03.0}%  英雄 {:03.0}%",
                        session.save_remaining.ceil() as u32,
                        archive_hp,
                        hero_hp
                    )
                } else {
                    format!(
                        "ARCHIVE {:02}s\nECHO {:03.0}%  HERO {:03.0}%",
                        session.save_remaining.ceil() as u32,
                        archive_hp,
                        hero_hp
                    )
                }
            } else if let Some(boss) = &boss_status {
                if language.is_chinese() {
                    format!(
                        "{:02}:{:02}\n首领 {}/5\n{}",
                        remaining / 60,
                        remaining % 60,
                        session.bosses_defeated,
                        boss
                    )
                } else {
                    format!(
                        "{:02}:{:02}\nROUND {}/5\n{}",
                        remaining / 60,
                        remaining % 60,
                        session.bosses_defeated,
                        boss
                    )
                }
            } else if language.is_chinese() {
                format!(
                    "{:02}:{:02}\n首领 {}/5",
                    remaining / 60,
                    remaining % 60,
                    session.bosses_defeated
                )
            } else {
                format!(
                    "{:02}:{:02}\nROUND {}/5",
                    remaining / 60,
                    remaining % 60,
                    session.bosses_defeated
                )
            }
        } else if level.is_some() {
            if session.xp_next == u32::MAX {
                if language.is_chinese() {
                    format!("等级 {}   已满", session.level)
                } else {
                    format!("LEVEL {}   MAX", session.level)
                }
            } else if language.is_chinese() {
                format!(
                    "等级 {}   {}/{} 经验",
                    session.level, session.xp, session.xp_next
                )
            } else {
                format!(
                    "LEVEL {}   {}/{} XP",
                    session.level, session.xp, session.xp_next
                )
            }
        } else if skill.is_some() {
            ultimate_button_text(language, vitals.kind, active.remaining, compact)
        } else if trial_marker.is_some() {
            trial_status.clone()
        } else if gold_marker.is_some() {
            gold_status.clone()
        } else if bonds.is_some() {
            if compact {
                if language.is_chinese() {
                    format!(
                        "羁绊 {}/7 | {} 套",
                        build.active_bond_count(),
                        compact_bonds
                    )
                } else {
                    format!(
                        "BONDS {}/7 | {} ACTIVE",
                        build.active_bond_count(),
                        compact_bonds
                    )
                }
            } else {
                bond_lines.join("\n")
            }
        } else if bond_button.is_some() {
            if session.save_active {
                if compact {
                    language.pick("BOND SEALED", "羁绊封印").to_string()
                } else if language.is_chinese() {
                    "F  羁绊封印".to_string()
                } else {
                    "F  BOND SEALED".to_string()
                }
            } else if compact && language.is_chinese() {
                format!("羁绊 | {} 木材", bond_cost(&session))
            } else if compact {
                format!("BOND | {} WOOD", bond_cost(&session))
            } else if language.is_chinese() {
                format!("F  抽取羁绊  [{} 木材]", bond_cost(&session))
            } else {
                format!("F  DRAW BOND  [{} WOOD]", bond_cost(&session))
            }
        } else if skill_draw.is_some() {
            if session.save_active {
                if compact {
                    language.pick("SKILL SEALED", "技能封印").to_string()
                } else if language.is_chinese() {
                    "G  技能封印".to_string()
                } else {
                    "G  SKILL SEALED".to_string()
                }
            } else if session.skill_points > 0 {
                if compact && language.is_chinese() {
                    format!("技能 | 免费 {}", session.skill_points)
                } else if compact {
                    format!("SKILL | FREE {}", session.skill_points)
                } else if language.is_chinese() {
                    format!("G  技能  [免费 {}]", session.skill_points)
                } else {
                    format!("G  SKILL  [FREE {}]", session.skill_points)
                }
            } else if compact && language.is_chinese() {
                format!("技能 | {} 金币", skill_cost(&skill_economy))
            } else if compact {
                format!("SKILL | {} GOLD", skill_cost(&skill_economy))
            } else if language.is_chinese() {
                format!("G  技能  [{} 金币]", skill_cost(&skill_economy))
            } else {
                format!("G  SKILL  [{} GOLD]", skill_cost(&skill_economy))
            }
        } else {
            continue;
        };
    }

    if let Ok(mut node) = bars.p0().single_mut() {
        node.width = percent((session.core_hp / session.core_max_hp * 100.0).clamp(0.0, 100.0));
    }
    if let Ok(mut node) = bars.p1().single_mut() {
        node.width =
            percent((session.xp as f32 / session.xp_next as f32 * 100.0).clamp(0.0, 100.0));
    }
}

fn ultimate_button_text(
    language: GameLanguage,
    kind: super::HeroKind,
    remaining: f32,
    compact: bool,
) -> String {
    let ultimate = hero_ultimate_name(language, kind);
    let rarity = HEROES[kind.index()].rarity.label();
    if remaining <= 0.0 {
        if compact {
            format!("[{rarity}] {ultimate}\n{}", language.pick("READY", "就绪"))
        } else if language.is_chinese() {
            format!("[{rarity}] {ultimate} 就绪\n空格 / 轻触")
        } else {
            format!("[{rarity}] {ultimate} READY\nSPACE / TAP")
        }
    } else if language.is_chinese() {
        format!("[{rarity}] {ultimate}\n{remaining:.1}秒")
    } else {
        format!("[{rarity}] {ultimate}\n{remaining:.1}s")
    }
}

fn show_result(
    mut commands: Commands,
    language: Res<GameLanguage>,
    session: Res<GameSession>,
    archive: Res<ArchiveState>,
    run_seed: Res<RunSeed>,
    existing: Query<Entity, With<ResultRoot>>,
) {
    if !session.finished || !existing.is_empty() {
        return;
    }
    commands
        .spawn((
            ResultRoot,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: px(14),
                ..default()
            },
            BackgroundColor(Color::srgba(0.01, 0.02, 0.04, 0.84)),
            GlobalZIndex(100),
        ))
        .with_children(|root| {
            root.spawn((
                ResultCopyLabel::Title,
                Text::new(result_title(*language, archive.outcome, session.victory)),
                Node {
                    max_width: percent(94),
                    ..default()
                },
                TextFont {
                    font_size: FontSize::Px(46.0),
                    ..default()
                },
                TextColor(if session.victory {
                    Color::srgb(0.25, 1.0, 0.72)
                } else {
                    Color::srgb(1.0, 0.24, 0.3)
                }),
                TextLayout::new(Justify::Center, LineBreak::WordOrCharacter),
            ));
            root.spawn((
                ResultCopyLabel::Summary,
                Text::new(result_summary(*language, &session)),
                Node {
                    max_width: percent(94),
                    ..default()
                },
                TextFont {
                    font_size: FontSize::Px(21.0),
                    ..default()
                },
                TextColor(Color::srgb(0.78, 0.85, 0.92)),
                TextLayout::new(Justify::Center, LineBreak::WordOrCharacter),
            ));
            root.spawn((
                Text::new(format_run_seed(run_seed.0)),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::srgb(0.52, 0.76, 0.86)),
            ));
            if archive.outcome != ArchiveOutcome::Pending {
                root.spawn((
                    ResultCopyLabel::Archive,
                    Text::new(result_archive(*language, &archive)),
                    Node {
                        max_width: percent(94),
                        ..default()
                    },
                    TextFont {
                        font_size: FontSize::Px(19.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.38, 0.9, 1.0)),
                    TextLayout::new(Justify::Center, LineBreak::WordOrCharacter),
                ));
            }
        });
}

fn update_result_text(
    language: Res<GameLanguage>,
    session: Res<GameSession>,
    archive: Res<ArchiveState>,
    mut labels: Query<(&ResultCopyLabel, &mut Text)>,
) {
    for (label, mut text) in &mut labels {
        **text = match label {
            ResultCopyLabel::Title => result_title(*language, archive.outcome, session.victory),
            ResultCopyLabel::Summary => result_summary(*language, &session),
            ResultCopyLabel::Archive => result_archive(*language, &archive),
        };
    }
}

fn result_title(language: GameLanguage, outcome: ArchiveOutcome, victory: bool) -> String {
    match outcome {
        ArchiveOutcome::Captured => language
            .pick("ARCHIVE ECHO CAPTURED", "战果回响已捕获")
            .to_string(),
        ArchiveOutcome::TimedOut | ArchiveOutcome::HeroFelled => language
            .pick("ARCHIVE RUN COMPLETE", "战果刻印完成")
            .to_string(),
        ArchiveOutcome::Pending if victory => {
            language.pick("THE CORE ENDURES", "城门屹立").to_string()
        }
        ArchiveOutcome::Pending => language.pick("THE CORE HAS FALLEN", "城门陷落").to_string(),
    }
}

fn result_summary(language: GameLanguage, session: &GameSession) -> String {
    if language.is_chinese() {
        format!(
            "等级 {}  |  总击杀 {}  |  木材 {}",
            session.level, session.total_kills, session.wood
        )
    } else {
        format!(
            "Level {}  |  {} total kills  |  {} wood",
            session.level, session.total_kills, session.wood
        )
    }
}

fn archive_outcome_label(language: GameLanguage, outcome: ArchiveOutcome) -> &'static str {
    match outcome {
        ArchiveOutcome::Pending => language.pick("NOT REACHED", "未到达"),
        ArchiveOutcome::Captured => language.pick("ECHO CAPTURED", "回响已捕获"),
        ArchiveOutcome::TimedOut => language.pick("TIME EXPIRED", "时间耗尽"),
        ArchiveOutcome::HeroFelled => language.pick("HERO FELLED", "英雄倒地"),
    }
}

fn result_archive(language: GameLanguage, archive: &ArchiveState) -> String {
    if language.is_chinese() {
        format!(
            "{}  |  +{} 战果碎片",
            archive_outcome_label(language, archive.outcome),
            archive.shards(),
        )
    } else {
        format!(
            "{}  |  +{} ARCHIVE SHARD{}",
            archive_outcome_label(language, archive.outcome),
            archive.shards(),
            if archive.shards() == 1 { "" } else { "S" }
        )
    }
}

fn format_run_seed(seed: u64) -> String {
    format!("SEED 0x{seed:016X}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_bottom_controls_fit_common_phone_widths() {
        for (width, height) in [
            (667.0, 375.0),
            (740.0, 360.0),
            (844.0, 390.0),
            (915.0, 412.0),
        ] {
            let layout = hud_metrics(width, height);
            let required = layout.joystick_size
                + compact_xp_width(width)
                + compact_action_width(layout.joystick_size);
            let available = width - layout.safe_x * 2.0;

            assert!(layout.compact);
            assert!(
                required <= available,
                "{required} > {available} at {width}x{height}"
            );
            assert!(compact_dodge_size(layout.joystick_size) >= 72.0);
            assert!(layout.joystick_size >= 112.0);
        }
    }

    #[test]
    fn desktop_action_panel_stays_left_of_the_market_toggle() {
        let width = 1280.0;
        let layout = hud_metrics(width, 720.0);
        let action_right = width - layout.safe_x - action_panel_margin_right(false);
        let market_left = width - 18.0 - 230.0;

        assert!(action_right <= market_left);
    }

    #[test]
    fn run_seed_text_preserves_all_sixty_four_bits() {
        assert_eq!(
            format_run_seed(0x1234_5678_9ABC_DEF0),
            "SEED 0x123456789ABCDEF0"
        );
        assert_eq!(format_run_seed(0xA11CE), "SEED 0x00000000000A11CE");
    }

    #[test]
    fn ultimate_button_uses_the_selected_hero_profile() {
        assert_eq!(
            ultimate_button_text(
                GameLanguage::English,
                super::super::HeroKind::SolarVanguard,
                0.0,
                true,
            ),
            "[UR] DAWN BASTION\nREADY"
        );
        assert_eq!(
            ultimate_button_text(
                GameLanguage::English,
                super::super::HeroKind::VoidReaper,
                12.34,
                false,
            ),
            "[UR] EVENT HORIZON\n12.3s"
        );
        assert_eq!(
            ultimate_button_text(
                GameLanguage::SimplifiedChinese,
                super::super::HeroKind::SolarVanguard,
                0.0,
                true,
            ),
            "[UR] 曙光壁垒\n就绪"
        );
    }

    #[test]
    fn result_overlay_includes_the_complete_run_seed() {
        let seed = 0x1234_5678_9ABC_DEF0;
        let mut app = App::new();
        app.insert_resource(GameSession {
            finished: true,
            ..default()
        })
        .insert_resource(GameLanguage::English)
        .insert_resource(ArchiveState::default())
        .insert_resource(RunSeed(seed))
        .add_systems(Update, show_result);

        app.update();

        let world = app.world_mut();
        let mut texts = world.query::<&Text>();
        assert!(
            texts
                .iter(world)
                .any(|text| text.0 == format_run_seed(seed))
        );
    }

    #[test]
    fn pause_overlay_includes_the_complete_run_seed() {
        let seed = 0x0FED_CBA9_8765_4321;
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<GameSession>()
            .init_resource::<ManualPauseState>()
            .insert_resource(GameLanguage::English)
            .insert_resource(RunSeed(seed))
            .add_systems(Update, handle_manual_pause);
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyP);

        app.update();

        let world = app.world_mut();
        let mut roots = world.query_filtered::<Entity, With<PauseRoot>>();
        assert_eq!(roots.iter(world).count(), 1);
        let mut texts = world.query::<&Text>();
        assert!(
            texts
                .iter(world)
                .any(|text| text.0 == format_run_seed(seed))
        );
    }
}
