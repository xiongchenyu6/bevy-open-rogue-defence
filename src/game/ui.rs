use bevy::{prelude::*, time::Virtual, window::PrimaryWindow};

use super::archive::ArchiveEcho;
use super::encounters::EncounterState;
use super::mobile_layout::{ControlScheme, hud_metrics};
use super::progression::{SkillEconomy, bond_cost, skill_cost};
use super::trial::{
    GOLD_LEVEL_COUNT, GOLD_TARGET_COUNT, GOLD_UNLOCK_AT, TOWER_FLOOR_COUNT, TOWER_UNLOCK_AT,
};
use super::{
    BondButtonLabel, BondDrawButton, CoreBar, DodgeButton, DodgeButtonLabel, DodgeState,
    EncounterButton, EncounterButtonLabel, Enemy, GameLanguage, GameSession, GoldChallengeState,
    GoldChallengeStatusText, HeroVitals, LevelText, PlayerBuild, RunSeed, SkillDrawButton,
    SkillDrawLabel, TimerText, TrialAction, TrialButton, TrialState, TrialStatusText, XpBar,
};

#[path = "ui_art.rs"]
mod ui_art;
use ui_art::*;

#[path = "bond_collection_ui.rs"]
mod bond_collection_ui;
#[path = "hero_stats_ui.rs"]
pub(crate) mod hero_stats_ui;
#[path = "survival_ui.rs"]
mod survival_ui;

pub struct GameUiPlugin;

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        hero_stats_ui::install(app);
        super::equipment_ui::install(app);
        survival_ui::install(app);
        bond_collection_ui::install(app);
        app.init_resource::<ManualPauseState>()
            .init_resource::<TrialDrawerState>()
            .init_resource::<BattleSpeed>()
            .add_systems(Startup, setup_hud)
            .add_systems(
                Update,
                (
                    apply_hud_layout,
                    handle_trial_drawer,
                    handle_battle_speed,
                    update_battle_speed_ui.after(handle_battle_speed),
                    handle_manual_pause,
                    update_static_ui_language,
                    update_trial_auto_feedback.after(update_static_ui_language),
                    update_joystick_visual,
                    update_hud,
                    update_icon_hud,
                    update_bond_strip,
                    update_small_controls,
                    update_fruit_button,
                    update_encounter_button,
                    update_seed_language,
                ),
            );
    }
}

#[derive(Component)]
struct HudRoot;

#[derive(Component)]
struct HudLeft;

#[derive(Component)]
struct HudBottom;

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
struct XpReadout;

#[derive(Component)]
struct DodgeArt;

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
struct SpeedControl;

#[derive(Component)]
struct ArchiveHudText;

#[derive(Component, Clone, Copy)]
struct SpeedButton(SpeedAction);

#[derive(Component)]
struct SpeedLabel(SpeedAction);

#[derive(Clone, Copy, PartialEq, Eq)]
enum SpeedAction {
    Decrease,
    Increase,
    Reset,
}

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

#[derive(Resource, Default)]
struct ManualPauseState {
    root: Option<Entity>,
    was_focused: bool,
}

#[derive(Resource, Default)]
struct TrialDrawerState {
    open: bool,
}

const BATTLE_SPEEDS: [f32; 6] = [1.0, 2.0, 4.0, 8.0, 16.0, 32.0];

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
struct BattleSpeed {
    selected: u8,
}

impl BattleSpeed {
    fn multiplier(self) -> f32 {
        BATTLE_SPEEDS[usize::from(self.selected.min(5))]
    }

    fn apply(&mut self, action: SpeedAction) {
        self.selected = match action {
            SpeedAction::Decrease => self.selected.saturating_sub(1),
            SpeedAction::Increase => (self.selected + 1).min(5),
            SpeedAction::Reset => 0,
        };
    }
}

fn setup_hud(mut commands: Commands, assets: Res<AssetServer>, language: Res<GameLanguage>) {
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
                    Name::new("Battle status"),
                    Node {
                        width: px(240),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(4),
                        ..default()
                    },
                ))
                .with_children(|left| {
                    left.spawn((
                        Button,
                        HudHint::Gate,
                        Node {
                            align_items: AlignItems::Center,
                            column_gap: px(7),
                            ..default()
                        },
                    ))
                    .with_children(|row| {
                        hud_icon(row, &assets, "tower", 24.0);
                        row.spawn((
                            CoreIntegrityLabel,
                            Text::new(language.pick("GATE · ∞", "城门 · ∞")),
                            TextFont {
                                font_size: FontSize::Px(11.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.86, 0.76, 0.52)),
                        ));
                        hero_stats_ui::spawn_entry(row, &assets);
                    });
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
                    survival_ui::spawn(left, false);
                    for (icon, value, color) in [
                        ("shield", HudValue::Health, Color::srgb(0.25, 0.82, 0.55)),
                        ("energy", HudValue::Energy, Color::srgb(0.30, 0.59, 0.94)),
                    ] {
                        left.spawn(Node {
                            align_items: AlignItems::Center,
                            column_gap: px(7),
                            ..default()
                        })
                        .with_children(|row| {
                            hud_icon(row, &assets, icon, 23.0);
                            row.spawn(Node {
                                width: px(176),
                                flex_direction: FlexDirection::Column,
                                row_gap: px(3),
                                ..default()
                            })
                            .with_children(|gauge| {
                                gauge.spawn((
                                    value,
                                    Text::new(""),
                                    TextFont {
                                        font_size: FontSize::Px(12.0),
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.86, 0.9, 0.94)),
                                ));
                                gauge
                                    .spawn((
                                        Node {
                                            width: percent(100),
                                            height: px(5),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgba(0.01, 0.02, 0.04, 0.9)),
                                    ))
                                    .with_children(|track| {
                                        track.spawn((
                                            HudGauge(value),
                                            Node {
                                                width: percent(100),
                                                height: percent(100),
                                                ..default()
                                            },
                                            BackgroundColor(color),
                                        ));
                                    });
                            });
                        });
                    }
                    left.spawn(Node {
                        align_items: AlignItems::Center,
                        column_gap: px(12),
                        margin: UiRect::vertical(px(4)),
                        ..default()
                    })
                    .with_children(|row| {
                        for (icon, value) in [
                            ("gold", HudValue::Gold),
                            ("wood", HudValue::Wood),
                            ("skull", HudValue::Kills),
                        ] {
                            row.spawn(Node {
                                align_items: AlignItems::Center,
                                column_gap: px(4),
                                ..default()
                            })
                            .with_children(|counter| {
                                hud_icon(counter, &assets, icon, 22.0);
                                counter.spawn((
                                    value,
                                    Text::new("0"),
                                    TextFont {
                                        font_size: FontSize::Px(13.0),
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.95, 0.85, 0.61)),
                                ));
                            });
                        }
                    });
                    spawn_bond_strip(left, &assets);
                    left.spawn(Node {
                        column_gap: px(4),
                        row_gap: px(4),
                        flex_wrap: FlexWrap::Wrap,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|actions| {
                        spawn_fruit_button(actions, &assets);
                        bond_collection_ui::spawn_entry(actions, &assets);
                        super::equipment_ui::spawn_entry(actions, &assets);
                        actions
                            .spawn((
                                Button,
                                BondDrawButton,
                                HudHint::Bond,
                                Node {
                                    width: px(104),
                                    height: px(44),
                                    flex_shrink: 0.0,
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    border_radius: BorderRadius::all(px(7)),
                                    column_gap: px(8),
                                    border: UiRect::all(px(1)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.055, 0.065, 0.085, 0.94)),
                                BorderColor::all(Color::srgba(0.65, 0.49, 0.25, 0.8)),
                            ))
                            .with_children(|button| {
                                hud_icon(button, &assets, "bond", 30.0);
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
                        spawn_encounter_button(actions, &assets);
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

            root.spawn((
                ArchiveHudText,
                Name::new("Archive battle progress"),
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::srgb(0.88, 0.95, 1.0)),
                TextLayout::justify(Justify::Center),
                BackgroundColor(Color::srgba(0.025, 0.035, 0.045, 0.88)),
                Node {
                    position_type: PositionType::Absolute,
                    left: percent(50),
                    margin: UiRect::left(px(-100)),
                    width: px(200),
                    top: px(80),
                    display: Display::None,
                    ..default()
                },
            ));

            root.spawn((
                HudBottom,
                Name::new("Battle bottom controls"),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(12),
                    right: px(12),
                    bottom: px(12),
                    align_items: AlignItems::FlexEnd,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                },
            ))
            .with_children(|bottom| {
                bottom
                    .spawn((
                        MovementPad,
                        Name::new("Movement pad"),
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
                            XpReadout,
                            Name::new("XP readout"),
                            Node {
                                width: percent(100),
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                row_gap: px(7),
                                ..default()
                            },
                        ))
                        .with_children(|readout| {
                            readout.spawn((
                                Text::new(language.pick("LEVEL 1", "等级 1")),
                                LevelText,
                                TextFont {
                                    font_size: FontSize::Px(12.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.78, 0.84, 0.96)),
                            ));
                            readout
                                .spawn((
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
                        });
                        xp.spawn((
                            Button,
                            SkillDrawButton,
                            Name::new("Skill draw control"),
                            HudHint::Skill,
                            Node {
                                width: px(112),
                                height: px(48),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                border_radius: BorderRadius::all(px(8)),
                                column_gap: px(8),
                                border: UiRect::all(px(1)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.055, 0.065, 0.085, 0.94)),
                            BorderColor::all(Color::srgba(0.65, 0.49, 0.25, 0.8)),
                        ))
                        .with_children(|button| {
                            hud_icon(button, &assets, "skill", 30.0);
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
                                Name::new("Dodge control"),
                                HudHint::Dodge,
                                Node {
                                    width: px(78),
                                    height: px(78),
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    border_radius: BorderRadius::all(px(9)),
                                    border: UiRect::all(px(1)),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: px(3),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.045, 0.045, 0.075, 0.95)),
                                BorderColor::all(Color::srgb(0.53, 0.42, 0.27)),
                            ))
                            .with_children(|button| {
                                button.spawn((
                                    DodgeArt,
                                    ImageNode::new(assets.load("ui/icons/dodge.png")),
                                    Node {
                                        width: px(44),
                                        height: px(44),
                                        ..default()
                                    },
                                    Pickable::IGNORE,
                                ));
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
                Text::new(language.pick("GOLD 1 LOCKED  01:00", "金币挑战 1 未解锁 01:00")),
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
            Name::new("Trial drawer"),
            HudHint::Trials,
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
            BackgroundColor(Color::srgba(0.035, 0.045, 0.065, 0.95)),
            BorderColor::all(Color::srgba(0.65, 0.49, 0.25, 0.75)),
            GlobalZIndex(22),
        ))
        .with_children(|button| {
            hud_icon(button, &assets, "tower", 26.0);
            button.spawn((
                TrialToggleLabel,
                Text::new(language.pick("CHALLENGES", "挑战")),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::srgb(0.94, 0.85, 0.63)),
            ));
        });

    commands
        .spawn((
            Button,
            PauseButton,
            HudHint::Pause,
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
            BackgroundColor(Color::srgba(0.035, 0.045, 0.065, 0.95)),
            BorderColor::all(Color::srgba(0.65, 0.49, 0.25, 0.75)),
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

    commands
        .spawn((
            SpeedControl,
            Name::new("Battle speed"),
            Node {
                position_type: PositionType::Absolute,
                right: px(208),
                top: px(12),
                width: px(152),
                height: px(48),
                column_gap: px(4),
                ..default()
            },
            // MarketPanel uses z=20; keep speed below it so the compact market
            // overlay cannot leave live battle controls over its header.
            GlobalZIndex(19),
        ))
        .with_children(|control| {
            for (action, label) in [
                (SpeedAction::Decrease, "−"),
                (SpeedAction::Increase, "+"),
                (SpeedAction::Reset, "1×"),
            ] {
                spawn_speed_button(control, action, label);
            }
        });
    commands
        .spawn((
            HudTooltip,
            Name::new("Bond detail tooltip"),
            Node {
                position_type: PositionType::Absolute,
                right: px(24),
                bottom: px(160),
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
                HudTooltipText,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::srgb(0.92, 0.87, 0.74)),
                Pickable::IGNORE,
            ));
        });
}

fn spawn_speed_button(parent: &mut ChildSpawnerCommands, action: SpeedAction, label: &'static str) {
    parent
        .spawn((
            Button,
            SpeedButton(action),
            Name::new(match action {
                SpeedAction::Decrease => "Speed decrease",
                SpeedAction::Increase => "Speed increase",
                SpeedAction::Reset => "Speed reset",
            }),
            HudHint::Speed(action),
            Node {
                width: px(48),
                height: px(48),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(9)),
                ..default()
            },
            BackgroundColor(if action == SpeedAction::Reset {
                Color::srgba(0.26, 0.18, 0.075, 0.96)
            } else {
                Color::srgba(0.035, 0.045, 0.065, 0.95)
            }),
            BorderColor::all(if action == SpeedAction::Reset {
                Color::srgb(0.92, 0.72, 0.34)
            } else {
                Color::srgba(0.65, 0.49, 0.25, 0.75)
            }),
        ))
        .with_children(|button| {
            button.spawn((
                SpeedLabel(action),
                Text::new(label),
                TextFont {
                    font_size: FontSize::Px(14.0),
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
            HudHint::Trial(action),
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

#[allow(clippy::type_complexity)]
fn apply_hud_layout(
    scheme: Res<ControlScheme>,
    windows: Query<&Window, With<PrimaryWindow>>,
    drawer: Res<TrialDrawerState>,
    session: Res<GameSession>,
    mut archive_nodes: Query<
        &mut Node,
        (
            With<ArchiveHudText>,
            Without<SpeedControl>,
            Without<HudBottom>,
            Without<DodgeArt>,
        ),
    >,
    mut nodes: Query<
        (
            &mut Node,
            Option<&HudRoot>,
            Option<&HudLeft>,
            Option<&CoreBarTrack>,
            Option<&MovementPad>,
            (Option<&XpPanel>, Option<&XpReadout>, Option<&XpBarTrack>),
            Option<&SkillDrawButton>,
            Option<&ActionPanel>,
            Option<&DodgeButton>,
            Option<&TrialPanel>,
            Option<&TrialButtonRow>,
            Option<&TrialButton>,
            Option<&TrialToggleButton>,
            Option<&PauseButton>,
        ),
        (
            Without<SpeedControl>,
            Without<HudBottom>,
            Without<DodgeArt>,
            Without<ArchiveHudText>,
        ),
    >,
    mut bottom_rows: Query<&mut Node, (With<HudBottom>, Without<SpeedControl>)>,
    mut speed_controls: Query<&mut Node, (With<SpeedControl>, Without<DodgeArt>)>,
    mut dodge_art: Query<&mut Node, (With<DodgeArt>, Without<HudBottom>, Without<SpeedControl>)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let layout = hud_metrics(window.width(), window.height());
    let compact = layout.compact;
    let xp_width = compact_xp_width(window.width());
    let compact_dodge_size = compact_dodge_size(layout.joystick_size);
    let action_width = compact_action_width(layout.joystick_size);
    let desktop_width = (window.width() - layout.safe_x * 2.0).min(420.0);
    let desktop_xp_width = desktop_width - 100.0;
    let desktop_readout_width = desktop_xp_width - 102.0;

    for (
        mut node,
        root,
        left,
        core_track,
        movement_pad,
        (xp_panel, xp_readout, xp_track),
        skill_button,
        action_panel,
        dodge_button,
        trial_panel,
        trial_row,
        trial_button,
        trial_toggle,
        pause_button,
    ) in &mut nodes
    {
        if root.is_some() {
            node.reborrow()
                .map_unchanged(|value| &mut value.padding)
                .set_if_neq(UiRect {
                    left: px(layout.safe_x),
                    right: px(layout.safe_x),
                    top: px(layout.safe_y),
                    bottom: px(layout.safe_y),
                });
        } else if left.is_some() {
            node.reborrow()
                .map_unchanged(|value| &mut value.width)
                .set_if_neq(px(220));
            node.reborrow()
                .map_unchanged(|value| &mut value.row_gap)
                .set_if_neq(px(if compact { 1 } else { 4 }));
        } else if core_track.is_some() {
            node.reborrow()
                .map_unchanged(|value| &mut value.width)
                .set_if_neq(px(206));
            node.reborrow()
                .map_unchanged(|value| &mut value.height)
                .set_if_neq(px(8));
            node.reborrow()
                .map_unchanged(|value| &mut value.padding)
                .set_if_neq(UiRect::all(px(if compact { 2 } else { 3 })));
        } else if movement_pad.is_some() {
            node.reborrow()
                .map_unchanged(|value| &mut value.display)
                .set_if_neq(if scheme.touch {
                    Display::Flex
                } else {
                    Display::None
                });
            node.reborrow()
                .map_unchanged(|value| &mut value.width)
                .set_if_neq(px(layout.joystick_size));
            node.reborrow()
                .map_unchanged(|value| &mut value.height)
                .set_if_neq(px(layout.joystick_size));
        } else if xp_panel.is_some() {
            node.reborrow()
                .map_unchanged(|value| &mut value.height)
                .set_if_neq(if scheme.touch { Val::Auto } else { px(40) });
            node.reborrow()
                .map_unchanged(|value| &mut value.width)
                .set_if_neq(px(if !scheme.touch {
                    desktop_xp_width
                } else if compact {
                    xp_width
                } else {
                    180.0
                }));
            node.reborrow()
                .map_unchanged(|value| &mut value.flex_direction)
                .set_if_neq(if scheme.touch {
                    FlexDirection::Column
                } else {
                    FlexDirection::Row
                });
            node.reborrow()
                .map_unchanged(|value| &mut value.column_gap)
                .set_if_neq(px(10));
            node.reborrow()
                .map_unchanged(|value| &mut value.row_gap)
                .set_if_neq(px(if compact { 5 } else { 7 }));
        } else if xp_readout.is_some() {
            node.reborrow()
                .map_unchanged(|value| &mut value.height)
                .set_if_neq(if scheme.touch { Val::Auto } else { px(28) });
            node.reborrow()
                .map_unchanged(|value| &mut value.justify_content)
                .set_if_neq(if scheme.touch {
                    JustifyContent::FlexStart
                } else {
                    JustifyContent::Center
                });
            node.reborrow()
                .map_unchanged(|value| &mut value.width)
                .set_if_neq(if scheme.touch {
                    percent(100)
                } else {
                    px(desktop_readout_width)
                });
            node.reborrow()
                .map_unchanged(|value| &mut value.row_gap)
                .set_if_neq(px(if scheme.touch { 7 } else { 4 }));
        } else if xp_track.is_some() {
            node.reborrow()
                .map_unchanged(|value| &mut value.width)
                .set_if_neq(px(if !scheme.touch {
                    desktop_readout_width
                } else if compact {
                    xp_width - 16.0
                } else {
                    170.0
                }));
            node.reborrow()
                .map_unchanged(|value| &mut value.height)
                .set_if_neq(px(if scheme.touch { 13 } else { 8 }));
        } else if skill_button.is_some() {
            node.reborrow()
                .map_unchanged(|value| &mut value.width)
                .set_if_neq(px(if scheme.touch { 112 } else { 92 }));
            node.reborrow()
                .map_unchanged(|value| &mut value.height)
                .set_if_neq(px(if scheme.touch { 48 } else { 40 }));
        } else if action_panel.is_some() {
            node.reborrow()
                .map_unchanged(|value| &mut value.width)
                .set_if_neq(px(if !scheme.touch {
                    88.0
                } else if compact {
                    action_width
                } else {
                    76.0
                }));
            node.reborrow()
                .map_unchanged(|value| &mut value.height)
                .set_if_neq(px(if !scheme.touch {
                    40.0
                } else if compact {
                    layout.joystick_size
                } else {
                    94.0
                }));
            node.reborrow()
                .map_unchanged(|value| &mut value.margin)
                .set_if_neq(UiRect::right(px(if scheme.touch {
                    action_panel_margin_right(compact)
                } else {
                    0.0
                })));
        } else if dodge_button.is_some() {
            let size = if compact { compact_dodge_size } else { 76.0 };
            node.reborrow()
                .map_unchanged(|value| &mut value.width)
                .set_if_neq(px(if scheme.touch { size } else { 88.0 }));
            node.reborrow()
                .map_unchanged(|value| &mut value.height)
                .set_if_neq(px(if scheme.touch { size } else { 40.0 }));
            node.reborrow()
                .map_unchanged(|value| &mut value.flex_direction)
                .set_if_neq(if scheme.touch {
                    FlexDirection::Column
                } else {
                    FlexDirection::Row
                });
            node.reborrow()
                .map_unchanged(|value| &mut value.column_gap)
                .set_if_neq(px(6));
        } else if trial_panel.is_some() {
            node.reborrow()
                .map_unchanged(|value| &mut value.display)
                .set_if_neq(if !drawer.open {
                    Display::None
                } else {
                    Display::Flex
                });
            node.reborrow()
                .map_unchanged(|value| &mut value.right)
                .set_if_neq(px(if compact { layout.safe_x } else { 22.0 }));
            node.reborrow()
                .map_unchanged(|value| &mut value.top)
                .set_if_neq(px(if compact { 118 } else { 112 }));
            node.reborrow()
                .map_unchanged(|value| &mut value.width)
                .set_if_neq(px(if compact { 205 } else { 225 }));
            node.reborrow()
                .map_unchanged(|value| &mut value.padding)
                .set_if_neq(UiRect::all(px(if compact { 8 } else { 12 })));
            node.reborrow()
                .map_unchanged(|value| &mut value.row_gap)
                .set_if_neq(px(if compact { 4 } else { 8 }));
        } else if trial_row.is_some() {
            node.reborrow()
                .map_unchanged(|value| &mut value.height)
                .set_if_neq(px(if compact { 48 } else { 54 }));
        } else if let Some(button) = trial_button {
            node.reborrow()
                .map_unchanged(|value| &mut value.height)
                .set_if_neq(px(48));
            node.reborrow()
                .map_unchanged(|value| &mut value.width)
                .set_if_neq(px(match (compact, button.0) {
                    (true, TrialAction::StartTower | TrialAction::StartGold) => 102.0,
                    (true, TrialAction::ToggleTowerAuto | TrialAction::ToggleGoldAuto) => 81.0,
                    (true, TrialAction::CancelAll) => 189.0,
                    (false, TrialAction::StartTower | TrialAction::StartGold) => 102.0,
                    (false, TrialAction::ToggleTowerAuto | TrialAction::ToggleGoldAuto) => 82.0,
                    (false, TrialAction::CancelAll) => 196.0,
                }));
        } else if trial_toggle.is_some() {
            node.reborrow()
                .map_unchanged(|value| &mut value.display)
                .set_if_neq(Display::Flex);
            node.reborrow()
                .map_unchanged(|value| &mut value.right)
                .set_if_neq(px(layout.safe_x));
            node.reborrow()
                .map_unchanged(|value| &mut value.top)
                .set_if_neq(px(64));
        } else if pause_button.is_some() {
            node.reborrow()
                .map_unchanged(|value| &mut value.right)
                .set_if_neq(px(if compact {
                    layout.safe_x + 104.0
                } else {
                    150.0
                }));
            node.reborrow()
                .map_unchanged(|value| &mut value.top)
                .set_if_neq(px(layout.safe_y));
        }
    }
    for mut node in &mut bottom_rows {
        node.reborrow()
            .map_unchanged(|value| &mut value.height)
            .set_if_neq(if scheme.touch { Val::Auto } else { px(40) });
        node.reborrow()
            .map_unchanged(|value| &mut value.left)
            .set_if_neq(if scheme.touch {
                px(layout.safe_x)
            } else {
                px((window.width() - desktop_width) * 0.5)
            });
        node.reborrow()
            .map_unchanged(|value| &mut value.right)
            .set_if_neq(if scheme.touch {
                px(layout.safe_x)
            } else {
                Val::Auto
            });
        node.reborrow()
            .map_unchanged(|value| &mut value.width)
            .set_if_neq(if scheme.touch {
                Val::Auto
            } else {
                px(desktop_width)
            });
        node.reborrow()
            .map_unchanged(|value| &mut value.bottom)
            .set_if_neq(px(layout.safe_y));
        node.reborrow()
            .map_unchanged(|value| &mut value.align_items)
            .set_if_neq(if scheme.touch {
                AlignItems::FlexEnd
            } else {
                AlignItems::Center
            });
        node.reborrow()
            .map_unchanged(|value| &mut value.justify_content)
            .set_if_neq(if scheme.touch {
                JustifyContent::SpaceBetween
            } else {
                JustifyContent::FlexStart
            });
        node.reborrow()
            .map_unchanged(|value| &mut value.column_gap)
            .set_if_neq(px(if scheme.touch { 0 } else { 12 }));
    }
    for mut node in &mut dodge_art {
        node.reborrow()
            .map_unchanged(|value| &mut value.width)
            .set_if_neq(px(if scheme.touch { 44 } else { 22 }));
        node.reborrow()
            .map_unchanged(|value| &mut value.height)
            .set_if_neq(px(if scheme.touch { 44 } else { 22 }));
    }
    for mut node in &mut archive_nodes {
        let width = if compact { 128.0 } else { 200.0 };
        node.reborrow()
            .map_unchanged(|value| &mut value.width)
            .set_if_neq(px(width));
        node.reborrow()
            .map_unchanged(|value| &mut value.margin)
            .set_if_neq(UiRect::left(px(-width * 0.5)));
        node.reborrow()
            .map_unchanged(|value| &mut value.display)
            .set_if_neq(if session.save_active {
                Display::Flex
            } else {
                Display::None
            });
        node.reborrow()
            .map_unchanged(|value| &mut value.top)
            .set_if_neq(px(if compact { 118.0 } else { 80.0 }));
    }
    for mut node in &mut speed_controls {
        node.reborrow()
            .map_unchanged(|value| &mut value.right)
            .set_if_neq(px(speed_control_right(layout.safe_x, compact)));
        node.reborrow()
            .map_unchanged(|value| &mut value.top)
            .set_if_neq(px(speed_control_top(layout.safe_y, compact)));
    }
}

fn compact_xp_width(window_width: f32) -> f32 {
    if window_width < 750.0 { 190.0 } else { 220.0 }
}

fn compact_dodge_size(joystick_size: f32) -> f32 {
    (joystick_size * 0.7).max(72.0)
}

fn compact_action_width(joystick_size: f32) -> f32 {
    compact_dodge_size(joystick_size)
}

fn action_panel_margin_right(compact: bool) -> f32 {
    if compact { 0.0 } else { 230.0 }
}

fn speed_control_right(safe_x: f32, compact: bool) -> f32 {
    if compact { safe_x + 162.0 } else { 208.0 }
}

fn speed_control_top(safe_y: f32, compact: bool) -> f32 {
    if compact { 64.0 } else { safe_y }
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
        label
            .reborrow()
            .map_unchanged(|value| &mut value.0)
            .set_if_neq(if drawer.open {
                language.pick("CLOSE", "关闭").to_string()
            } else {
                language.pick("CHALLENGES", "挑战").to_string()
            });
    }
}

fn handle_battle_speed(
    keyboard: Res<ButtonInput<KeyCode>>,
    interactions: Query<(&Interaction, &SpeedButton), Changed<Interaction>>,
    mut battle_speed: ResMut<BattleSpeed>,
    mut virtual_time: ResMut<Time<Virtual>>,
) {
    let selected = interactions.iter().find_map(|(interaction, button)| {
        (*interaction == Interaction::Pressed).then_some(button.0)
    });
    if let Some(selected) = selected {
        battle_speed.apply(selected);
    } else if keyboard.just_pressed(KeyCode::KeyT) {
        battle_speed.apply(SpeedAction::Increase);
    } else {
        return;
    }
    virtual_time.set_relative_speed(battle_speed.multiplier());
}

fn update_battle_speed_ui(
    battle_speed: Res<BattleSpeed>,
    mut buttons: Query<(
        &SpeedButton,
        &Interaction,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
    mut labels: Query<(&SpeedLabel, &mut Text)>,
) {
    for (label, mut text) in &mut labels {
        text.reborrow()
            .map_unchanged(|value| &mut value.0)
            .set_if_neq(match label.0 {
                SpeedAction::Decrease => "−".into(),
                SpeedAction::Increase => "+".into(),
                SpeedAction::Reset => format!("{}×", battle_speed.multiplier() as u8),
            });
    }
    for (button, interaction, mut background, mut border) in &mut buttons {
        let selected = button.0 == SpeedAction::Reset || *interaction != Interaction::None;
        background.0 = if selected {
            Color::srgba(0.26, 0.18, 0.075, 0.96)
        } else {
            Color::srgba(0.035, 0.045, 0.065, 0.95)
        };
        *border = BorderColor::all(if selected {
            Color::srgb(0.92, 0.72, 0.34)
        } else {
            Color::srgba(0.65, 0.49, 0.25, 0.75)
        });
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
        text.reborrow()
            .map_unchanged(|value| &mut value.0)
            .set_if_neq(language.pick("GATE · ∞", "城门 · ∞").to_string());
    }
    for mut text in &mut movement_labels {
        text.reborrow()
            .map_unchanged(|value| &mut value.0)
            .set_if_neq(language.pick("MOVE", "移动").to_string());
    }
    for (label, mut text) in &mut trial_labels {
        text.reborrow()
            .map_unchanged(|value| &mut value.0)
            .set_if_neq(trial_action_label(*language, label.0).to_string());
    }
    for (label, mut text) in &mut pause_labels {
        text.reborrow()
            .map_unchanged(|value| &mut value.0)
            .set_if_neq(
                match label {
                    PauseCopyLabel::Title => language.pick("PAUSED", "已暂停"),
                    PauseCopyLabel::Description => {
                        language.pick("The battlefield is frozen.", "战场已冻结。")
                    }
                    PauseCopyLabel::Resume => language.pick("RESUME", "继续战斗"),
                }
                .to_string(),
            );
    }
}

fn handle_manual_pause(
    mut commands: Commands,
    settings: Option<Res<super::settings::SettingsState>>,
    attributes: Option<Res<hero_stats_ui::HeroStatsPanelState>>,
    collection: Option<Res<bond_collection_ui::CollectionState>>,
    equipment: Option<Res<super::equipment_ui::EquipmentPanelState>>,
    adventure: Option<Res<super::AdventureBossState>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    pause_buttons: Query<&Interaction, (With<PauseButton>, Changed<Interaction>)>,
    resume_buttons: Query<&Interaction, (With<PauseResumeButton>, Changed<Interaction>)>,
    mut session: ResMut<GameSession>,
    mut state: ResMut<ManualPauseState>,
    run_seed: Res<RunSeed>,
    language: Res<GameLanguage>,
) {
    if adventure.is_some_and(|state| state.pending.is_some())
        || equipment.is_some_and(|state| state.open())
        || collection.is_some_and(|state| state.open())
        || attributes.is_some_and(|state| state.open())
        || settings.is_some_and(|state| state.open)
    {
        return;
    }
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
                RunSeedLabel(run_seed.0),
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
        node.reborrow()
            .map_unchanged(|value| &mut value.width)
            .set_if_neq(px(knob_size));
        node.reborrow()
            .map_unchanged(|value| &mut value.height)
            .set_if_neq(px(knob_size));
        node.reborrow()
            .map_unchanged(|value| &mut value.left)
            .set_if_neq(px(center + intent.0.x * travel));
        node.reborrow()
            .map_unchanged(|value| &mut value.top)
            .set_if_neq(px(center + intent.0.y * travel));
    }
}

fn update_hud(
    scheme: Res<ControlScheme>,
    build: Res<PlayerBuild>,
    language: Res<GameLanguage>,
    session: Res<GameSession>,
    skill_economy: Res<SkillEconomy>,
    encounters: Res<EncounterState>,
    vitals: Res<HeroVitals>,
    dodge: Res<DodgeState>,
    trial: Res<TrialState>,
    gold_trial: Res<GoldChallengeState>,
    archive_echo: Query<&Enemy, With<ArchiveEcho>>,
    mut text_nodes: Query<(
        &mut Text,
        (Option<&TimerText>, Option<&ArchiveHudText>),
        Option<&LevelText>,
        Option<&TrialStatusText>,
        Option<&GoldChallengeStatusText>,
        Option<&BondButtonLabel>,
        Option<&SkillDrawLabel>,
        Option<&DodgeButtonLabel>,
    )>,
    mut bars: ParamSet<(
        Query<&mut Node, (With<CoreBar>, Without<XpBar>)>,
        Query<&mut Node, (With<XpBar>, Without<CoreBar>)>,
    )>,
) {
    let language = *language;
    let remaining = (session.duration - session.elapsed).max(0.0) as u32;
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
    } else if usize::from(trial.level) >= TOWER_FLOOR_COUNT {
        language.pick("TOWER COMPLETE", "爬塔完成").to_string()
    } else if session.elapsed < TOWER_UNLOCK_AT {
        let unlock = (TOWER_UNLOCK_AT - session.elapsed).ceil() as u32;
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
    let trial_status = trial_base;
    let gold_status = if session.save_active {
        language
            .pick("GOLD SEALED - ARCHIVE ACTIVE", "金币挑战封印 · 战果刻印中")
            .to_string()
    } else if gold_trial.active {
        if language.is_chinese() {
            format!(
                "金币挑战 {} · {:.1}秒 · 目标 {}/{}",
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
        language.pick("GOLD COMPLETE", "金币挑战完成").to_string()
    } else if session.elapsed < GOLD_UNLOCK_AT {
        let unlock = (GOLD_UNLOCK_AT - session.elapsed).ceil() as u32;
        if language.is_chinese() {
            format!(
                "金币挑战 {} 未解锁 {:02}:{:02}",
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
                "金币挑战 {} · 冷却 {:.0}秒",
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
                "金币挑战 {} 就绪{}",
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
    for (
        mut text,
        (timer, archive_text),
        level,
        trial_marker,
        gold_marker,
        bond_button,
        skill_draw,
        dodge_label,
    ) in &mut text_nodes
    {
        text.reborrow()
            .map_unchanged(|value| &mut value.0)
            .set_if_neq(if dodge_label.is_some() {
                if !vitals.alive || vitals.stunned_remaining > 0.0 {
                    "—".to_string()
                } else if dodge.cooldown_remaining > 0.0 {
                    format!(
                        "{:.1}{}",
                        dodge.cooldown_remaining,
                        language.pick("s", "秒")
                    )
                } else {
                    if scheme.touch {
                        language.pick("DODGE", "闪避").to_string()
                    } else {
                        "Shift".to_string()
                    }
                }
            } else if archive_text.is_some() {
                if session.save_active {
                    if language.is_chinese() {
                        format!(
                            "刻印 {:02}秒\n回响 {:03.0}%\n英雄 {:03.0}%",
                            session.save_remaining.ceil() as u32,
                            archive_hp,
                            hero_hp
                        )
                    } else {
                        format!(
                            "ARCHIVE {:02}s\nECHO {:03.0}%\nHERO {:03.0}%",
                            session.save_remaining.ceil() as u32,
                            archive_hp,
                            hero_hp
                        )
                    }
                } else {
                    String::new()
                }
            } else if timer.is_some() {
                if session.save_active {
                    String::new()
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
            } else if trial_marker.is_some() {
                trial_status.clone()
            } else if gold_marker.is_some() {
                gold_status.clone()
            } else if bond_button.is_some() {
                if super::available_bond_cards_at(&build, &session).is_empty() {
                    if super::available_bond_cards(&build).is_empty() {
                        language.pick("DONE", "集齐").to_string()
                    } else {
                        language.pick("LOCKED", "待解锁").to_string()
                    }
                } else if session.save_active {
                    "—".to_string()
                } else {
                    format!("{}", bond_cost(&session, &encounters))
                }
            } else if skill_draw.is_some() {
                if session.save_active {
                    "—".to_string()
                } else if session.skill_points > 0 {
                    format!("+{}", session.skill_points)
                } else {
                    format!("{}", skill_cost(&skill_economy))
                }
            } else {
                continue;
            });
    }

    if let Ok(mut node) = bars.p0().single_mut() {
        node.reborrow()
            .map_unchanged(|value| &mut value.width)
            .set_if_neq(percent(
                (session.core_hp / session.core_max_hp * 100.0).clamp(0.0, 100.0),
            ));
    }
    if let Ok(mut node) = bars.p1().single_mut() {
        node.reborrow()
            .map_unchanged(|value| &mut value.width)
            .set_if_neq(percent(
                (session.xp as f32 / session.xp_next as f32 * 100.0).clamp(0.0, 100.0),
            ));
    }
}

#[derive(Component)]
struct RunSeedLabel(u64);

fn update_seed_language(
    language: Res<GameLanguage>,
    mut labels: Query<(&RunSeedLabel, &mut Text)>,
) {
    for (seed, mut text) in &mut labels {
        text.reborrow()
            .map_unchanged(|value| &mut value.0)
            .set_if_neq(if language.is_chinese() {
                format!("种子 0x{:016X}", seed.0)
            } else {
                format_run_seed(seed.0)
            });
    }
}

fn format_run_seed(seed: u64) -> String {
    format!("SEED 0x{seed:016X}")
}

fn update_trial_auto_feedback(
    language: Res<GameLanguage>,
    tower: Res<TrialState>,
    gold: Res<GoldChallengeState>,
    mut buttons: Query<(&TrialButton, &mut BackgroundColor)>,
    mut labels: Query<(&TrialActionLabel, &mut Text)>,
) {
    let enabled = |action| match action {
        TrialAction::ToggleTowerAuto => Some(tower.auto),
        TrialAction::ToggleGoldAuto => Some(gold.auto),
        _ => None,
    };
    for (button, mut background) in &mut buttons {
        if let Some(on) = enabled(button.0) {
            background.0 = if on {
                Color::srgb(0.36, 0.26, 0.10)
            } else {
                Color::srgb(0.055, 0.11, 0.15)
            };
        }
    }
    for (label, mut text) in &mut labels {
        if let Some(on) = enabled(label.0) {
            text.reborrow()
                .map_unchanged(|value| &mut value.0)
                .set_if_neq(
                    if on {
                        language.pick("AUTO ON", "自动开启")
                    } else {
                        language.pick("AUTO OFF", "自动关闭")
                    }
                    .to_string(),
                );
        }
    }
}

pub(crate) fn reset_run(world: &mut World) {
    world.insert_resource(ManualPauseState::default());
    world.insert_resource(TrialDrawerState::default());
    world.insert_resource(BattleSpeed::default());
    hero_stats_ui::reset_run(world);
    bond_collection_ui::reset_run(world);
    super::equipment_ui::reset_run(world);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn narrow_desktop_hides_touch_controls_until_real_touch_mode() {
        let mut app = App::new();
        app.insert_resource(ControlScheme { touch: false })
            .init_resource::<TrialDrawerState>()
            .init_resource::<GameSession>()
            .add_systems(Update, apply_hud_layout);
        app.world_mut().spawn((
            Window {
                resolution: (667, 375).into(),
                ..default()
            },
            PrimaryWindow,
        ));
        let pad = app.world_mut().spawn((MovementPad, Node::default())).id();
        let dodge = app.world_mut().spawn((DodgeButton, Node::default())).id();
        let bottom = app.world_mut().spawn((HudBottom, Node::default())).id();
        for touch in [false, true, false] {
            app.world_mut().resource_mut::<ControlScheme>().touch = touch;
            app.update();
            let pad_node = app.world().get::<Node>(pad).unwrap();
            let dodge_node = app.world().get::<Node>(dodge).unwrap();
            assert_eq!(
                app.world().get::<Node>(bottom).unwrap().height,
                if touch { Val::Auto } else { px(40) }
            );
            assert_eq!(
                pad_node.display,
                if touch { Display::Flex } else { Display::None }
            );
            if !touch {
                assert_eq!(dodge_node.height, px(40));
                assert_eq!(app.world().get::<Node>(bottom).unwrap().width, px(420));
            } else {
                assert_ne!(dodge_node.height, px(40));
            }
        }
    }

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
    fn compact_speed_control_fits_beside_challenges_below_market() {
        for (width, height) in [
            (667.0, 375.0),
            (740.0, 360.0),
            (844.0, 390.0),
            (915.0, 412.0),
        ] {
            let layout = hud_metrics(width, height);
            let speed_right = speed_control_right(layout.safe_x, true);
            let speed_left = width - speed_right - 152.0;
            let speed_right_edge = width - speed_right;
            let challenge_left = width - layout.safe_x - 154.0;
            let market_bottom = layout.safe_y + 52.0;
            let speed_top = speed_control_top(layout.safe_y, true);

            assert!(
                speed_left >= layout.safe_x,
                "speed left at {width}x{height}"
            );
            assert!(
                speed_right_edge + 8.0 <= challenge_left,
                "speed/challenge gap at {width}x{height}"
            );
            assert!(speed_top >= market_bottom, "speed below market");
            assert_eq!((152.0 - 8.0) / 3.0, 48.0);
        }
    }

    #[test]
    fn battle_speed_doubles_halves_and_clamps_without_rng() {
        let mut speed = BattleSpeed::default();
        for expected in [1., 2., 4., 8., 16., 32., 32.] {
            assert_eq!(speed.multiplier(), expected);
            speed.apply(SpeedAction::Increase);
        }
        for expected in [32., 16., 8., 4., 2., 1., 1.] {
            assert_eq!(speed.multiplier(), expected);
            speed.apply(SpeedAction::Decrease);
        }
        speed.apply(SpeedAction::Increase);
        speed.apply(SpeedAction::Reset);
        assert_eq!(speed.multiplier(), 1.);
    }

    #[test]
    fn speed_button_updates_bevy_virtual_time() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<BattleSpeed>()
            .insert_resource(Time::<Virtual>::default())
            .add_systems(Update, handle_battle_speed);
        app.world_mut()
            .spawn((SpeedButton(SpeedAction::Increase), Interaction::Pressed));
        app.update();
        assert_eq!(app.world().resource::<BattleSpeed>().selected, 1);
        assert_eq!(
            app.world().resource::<Time<Virtual>>().relative_speed(),
            2.0
        );
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
