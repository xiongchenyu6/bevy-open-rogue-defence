use bevy::prelude::*;

use super::archive::{ArchiveEcho, ArchiveOutcome, ArchiveState};
use super::combat::BOSS_NAMES;
use super::progression::{SkillEconomy, bond_cost, skill_cost};
use super::trial::{GOLD_LEVEL_COUNT, GOLD_TARGET_COUNT, GOLD_UNLOCK_AT, TOWER_UNLOCK_AT};
use super::{
    ActiveSkill, BondButtonLabel, BondDrawButton, BondId, BondStatusText, BossEnemy, CoreBar,
    DamageMeter, DodgeButton, DodgeButtonLabel, DodgeState, Enemy, GameSession, GoldChallengeState,
    GoldChallengeStatusText, HeroVitals, HeroVitalsText, LevelText, PlayerBuild, ResultRoot,
    RunSeed, RunStatsText, SkillButtonLabel, SkillDrawButton, SkillDrawLabel, TimerText,
    TrialAction, TrialButton, TrialState, TrialStatusText, UltimateButton, XpBar,
};

pub struct GameUiPlugin;

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_hud)
            .add_systems(Update, (update_hud, show_result));
    }
}

fn setup_hud(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                padding: UiRect::all(px(22)),
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
                top.spawn(Node {
                    width: px(350),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(7),
                    ..default()
                })
                .with_children(|left| {
                    left.spawn((
                        Text::new("CORE INTEGRITY"),
                        TextFont {
                            font_size: FontSize::Px(15.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.62, 0.86, 0.88)),
                    ));
                    left.spawn((
                        Node {
                            width: px(330),
                            height: px(18),
                            padding: UiRect::all(px(3)),
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
                        Text::new("HERO HP"),
                        TextFont {
                            font_size: FontSize::Px(17.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.35, 1.0, 0.62)),
                    ));
                    left.spawn((
                        RunStatsText,
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(17.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                    left.spawn((
                        BondStatusText,
                        Text::new("BONDS 0/7"),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.96, 0.78, 0.3)),
                    ));
                    left.spawn((
                        Button,
                        BondDrawButton,
                        Node {
                            width: px(185),
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
                            Text::new("F  DRAW BOND  [10 WOOD]"),
                            TextFont {
                                font_size: FontSize::Px(12.0),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                    });
                });
                top.spawn((
                    Text::new("25:00\nWAVE 1"),
                    TimerText,
                    TextFont {
                        font_size: FontSize::Px(26.0),
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
                    .spawn(Node {
                        width: px(150),
                        height: px(150),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        border_radius: BorderRadius::MAX,
                        border: UiRect::all(px(2)),
                        ..default()
                    })
                    .insert(BackgroundColor(Color::srgba(0.05, 0.14, 0.18, 0.32)))
                    .insert(BorderColor::all(Color::srgba(0.2, 0.8, 0.9, 0.22)))
                    .with_children(|joy| {
                        joy.spawn((
                            Text::new("MOVE\nARROWS / TOUCH"),
                            TextFont {
                                font_size: FontSize::Px(15.0),
                                ..default()
                            },
                            TextColor(Color::srgba(0.65, 0.86, 0.9, 0.7)),
                            TextLayout::justify(Justify::Center),
                        ));
                    });

                bottom
                    .spawn((Node {
                        width: px(360),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: px(7),
                        ..default()
                    },))
                    .with_children(|xp| {
                        xp.spawn((
                            Text::new("LEVEL 1"),
                            LevelText,
                            TextFont {
                                font_size: FontSize::Px(15.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.78, 0.84, 0.96)),
                        ));
                        xp.spawn((
                            Node {
                                width: px(340),
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
                                width: px(170),
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
                                Text::new("G  SKILL  [80 GOLD]"),
                                TextFont {
                                    font_size: FontSize::Px(13.0),
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });
                    });

                bottom
                    .spawn(Node {
                        width: px(260),
                        height: px(150),
                        margin: UiRect::right(px(225)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceBetween,
                        column_gap: px(12),
                        ..default()
                    })
                    .with_children(|controls| {
                        controls
                            .spawn((
                                Button,
                                DodgeButton,
                                Node {
                                    width: px(96),
                                    height: px(96),
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
                                    Text::new("DODGE\nSHIFT / TAP"),
                                    TextFont {
                                        font_size: FontSize::Px(14.0),
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
                                    width: px(150),
                                    height: px(150),
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
                                    Text::new("STORM\nSPACE / TAP"),
                                    TextFont {
                                        font_size: FontSize::Px(16.0),
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
                Text::new("TOWER 1 LOCKED  01:30"),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.9, 1.0)),
            ));
            panel
                .spawn(Node {
                    width: percent(100),
                    height: px(54),
                    column_gap: px(6),
                    ..default()
                })
                .with_children(|row| {
                    spawn_trial_button(row, TrialAction::StartTower, "Q  TOWER", 102.0);
                    spawn_trial_button(row, TrialAction::ToggleTowerAuto, "AUTO", 82.0);
                });
            panel.spawn((
                GoldChallengeStatusText,
                Text::new("GOLD 1 LOCKED  01:00"),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.82, 0.38)),
            ));
            panel
                .spawn(Node {
                    width: percent(100),
                    height: px(54),
                    column_gap: px(6),
                    ..default()
                })
                .with_children(|row| {
                    spawn_trial_button(row, TrialAction::StartGold, "W  GOLD", 102.0);
                    spawn_trial_button(row, TrialAction::ToggleGoldAuto, "AUTO", 82.0);
                });
            spawn_trial_button(panel, TrialAction::CancelAll, "F4  CLEAR", 196.0);
        });
}

fn spawn_trial_button(
    parent: &mut ChildSpawnerCommands,
    action: TrialAction,
    label: &'static str,
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
                Text::new(label),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

fn update_hud(
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
    mut core_bar: Query<&mut Node, (With<CoreBar>, Without<XpBar>)>,
    mut xp_bar: Query<&mut Node, (With<XpBar>, Without<CoreBar>)>,
) {
    let remaining = (session.duration - session.elapsed).max(0.0) as u32;
    let boss_status = bosses.iter().next().map(|(enemy, boss)| {
        format!(
            "{}  {:.0}%",
            BOSS_NAMES[boss.index as usize],
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
        "TOWER SEALED — ARCHIVE ACTIVE".to_string()
    } else if trial.active {
        format!("TOWER {}  {:.1}s", trial.level + 1, trial.remaining)
    } else if usize::from(trial.level) >= TOWER_UNLOCK_AT.len() {
        "TOWER COMPLETE".to_string()
    } else if session.elapsed < TOWER_UNLOCK_AT[trial.level as usize] {
        let unlock = (TOWER_UNLOCK_AT[trial.level as usize] - session.elapsed).ceil() as u32;
        format!(
            "TOWER {} LOCKED  {:02}:{:02}",
            trial.level + 1,
            unlock / 60,
            unlock % 60
        )
    } else if trial.cooldown > 0.0 {
        format!("TOWER {}  CD {:.0}s", trial.level + 1, trial.cooldown)
    } else {
        format!(
            "TOWER {} READY{}",
            trial.level + 1,
            if trial.auto { "  [AUTO]" } else { "" }
        )
    };
    let next_growth = super::CommandGrowth::from_point(trial.command_points);
    let last_growth = trial.last_growth.map_or("—", |growth| growth.label());
    let trial_status = format!(
        "{trial_base}\nCMD {}  A{} H{} X{}  LAST {}  NEXT {}",
        trial.command_points,
        trial.growth_ranks[0],
        trial.growth_ranks[1],
        trial.growth_ranks[2],
        last_growth,
        next_growth.label(),
    );
    let gold_status = if session.save_active {
        "GOLD SEALED — ARCHIVE ACTIVE".to_string()
    } else if gold_trial.active {
        format!(
            "GOLD {}  {:.1}s  TARGETS {}/{}",
            gold_trial.level + 1,
            gold_trial.remaining,
            gold_trial.remaining_targets,
            GOLD_TARGET_COUNT,
        )
    } else if usize::from(gold_trial.level) >= GOLD_LEVEL_COUNT {
        "GOLD COMPLETE".to_string()
    } else if session.elapsed < GOLD_UNLOCK_AT {
        let unlock = (GOLD_UNLOCK_AT - session.elapsed).ceil() as u32;
        format!(
            "GOLD {} LOCKED  {:02}:{:02}",
            gold_trial.level + 1,
            unlock / 60,
            unlock % 60
        )
    } else if gold_trial.cooldown > 0.0 {
        format!(
            "GOLD {}  CD {:.0}s",
            gold_trial.level + 1,
            gold_trial.cooldown
        )
    } else {
        format!(
            "GOLD {} READY{}",
            gold_trial.level + 1,
            if gold_trial.auto { "  [AUTO]" } else { "" }
        )
    };
    let mut bond_lines = vec![format!("BONDS {}/7", build.active_bond_count())];
    for id in BondId::ALL {
        let rank = build.bond(id);
        if rank > 0 && !build.bond_complete(id) {
            bond_lines.push(format!("{}  {}/{}", id.title(), rank, id.required()));
        }
    }
    if bond_lines.len() == 1 {
        bond_lines.push("No active bond — press F".to_string());
    }

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
                    format!("STUNNED {:.1}s", vitals.stunned_remaining)
                } else if vitals.invulnerable_remaining > 0.0 {
                    format!("INVULNERABLE {:.1}s", vitals.invulnerable_remaining)
                } else {
                    "ACTIVE".to_string()
                };
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
            } else {
                format!(
                    "HERO DOWN — REVIVE {:.1}s\nDEATH {}  |  CORE -80",
                    vitals.revive_remaining, vitals.deaths
                )
            }
        } else if dodge_label.is_some() {
            if !vitals.alive {
                "DODGE\nLOCKED".to_string()
            } else if vitals.stunned_remaining > 0.0 {
                "DODGE\nSTUNNED".to_string()
            } else if dodge.cooldown_remaining <= 0.0 {
                "DODGE READY\nSHIFT / TAP".to_string()
            } else {
                format!("DODGE\n{:.1}s", dodge.cooldown_remaining)
            }
        } else if stats.is_some() {
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
        } else if timer.is_some() {
            if session.save_active {
                format!(
                    "ARCHIVE {:02}s\nECHO {:03.0}%  HERO {:03.0}%",
                    session.save_remaining.ceil() as u32,
                    archive_hp,
                    hero_hp
                )
            } else if let Some(boss) = &boss_status {
                format!(
                    "{:02}:{:02}\nROUND {}/5\n{}",
                    remaining / 60,
                    remaining % 60,
                    session.bosses_defeated,
                    boss
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
                format!("LEVEL {}   MAX", session.level)
            } else {
                format!(
                    "LEVEL {}   {}/{} XP",
                    session.level, session.xp, session.xp_next
                )
            }
        } else if skill.is_some() {
            if active.remaining <= 0.0 {
                "STORM READY\nSPACE / TAP".to_string()
            } else {
                format!("STORM\n{:.1}s", active.remaining)
            }
        } else if trial_marker.is_some() {
            trial_status.clone()
        } else if gold_marker.is_some() {
            gold_status.clone()
        } else if bonds.is_some() {
            bond_lines.join("\n")
        } else if bond_button.is_some() {
            if session.save_active {
                "F  BOND SEALED".to_string()
            } else {
                format!("F  DRAW BOND  [{} WOOD]", bond_cost(&session))
            }
        } else if skill_draw.is_some() {
            if session.save_active {
                "G  SKILL SEALED".to_string()
            } else if session.skill_points > 0 {
                format!("G  SKILL  [FREE {}]", session.skill_points)
            } else {
                format!("G  SKILL  [{} GOLD]", skill_cost(&skill_economy))
            }
        } else {
            continue;
        };
    }

    if let Ok(mut node) = core_bar.single_mut() {
        node.width = percent((session.core_hp / session.core_max_hp * 100.0).clamp(0.0, 100.0));
    }
    if let Ok(mut node) = xp_bar.single_mut() {
        node.width =
            percent((session.xp as f32 / session.xp_next as f32 * 100.0).clamp(0.0, 100.0));
    }
}

fn show_result(
    mut commands: Commands,
    session: Res<GameSession>,
    archive: Res<ArchiveState>,
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
            let title = match archive.outcome {
                ArchiveOutcome::Captured => "ARCHIVE ECHO CAPTURED",
                ArchiveOutcome::TimedOut | ArchiveOutcome::HeroFelled => "ARCHIVE RUN COMPLETE",
                ArchiveOutcome::Pending if session.victory => "THE CORE ENDURES",
                ArchiveOutcome::Pending => "THE CORE HAS FALLEN",
            };
            root.spawn((
                Text::new(title),
                TextFont {
                    font_size: FontSize::Px(46.0),
                    ..default()
                },
                TextColor(if session.victory {
                    Color::srgb(0.25, 1.0, 0.72)
                } else {
                    Color::srgb(1.0, 0.24, 0.3)
                }),
            ));
            root.spawn((
                Text::new(format!(
                    "Level {}  |  {} total kills  |  {} wood",
                    session.level, session.total_kills, session.wood
                )),
                TextFont {
                    font_size: FontSize::Px(21.0),
                    ..default()
                },
                TextColor(Color::srgb(0.78, 0.85, 0.92)),
            ));
            if archive.outcome != ArchiveOutcome::Pending {
                root.spawn((
                    Text::new(format!(
                        "{}  |  +{} ARCHIVE SHARD{}",
                        archive.outcome.label(),
                        archive.shards(),
                        if archive.shards() == 1 { "" } else { "S" }
                    )),
                    TextFont {
                        font_size: FontSize::Px(19.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.38, 0.9, 1.0)),
                ));
            }
        });
}
