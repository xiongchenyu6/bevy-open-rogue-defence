//! Finite commander lives and an endlessly rebuilding gate have distinct HUD states.
use super::*;

#[derive(Component)]
struct LivesLabel(bool);

pub(super) fn install(app: &mut App) {
    app.add_systems(Startup, spawn_gate_countdown)
        .add_systems(Update, update.after(update_static_ui_language))
        .add_systems(
            PostUpdate,
            update_gate_countdown.after(bevy::transform::TransformSystems::Propagate),
        );
}

pub(super) fn spawn(parent: &mut ChildSpawnerCommands, detail: bool) {
    parent
        .spawn((
            Button,
            HudHint::Lives,
            Name::new(if detail {
                "Hero lives explanation"
            } else {
                "Hero lives HUD"
            }),
            Node {
                flex_shrink: 0.0,
                padding: UiRect::vertical(px(2)),
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .with_children(|row| {
            row.spawn((
                LivesLabel(detail),
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(if detail { 14.0 } else { 12.0 }),
                    ..default()
                },
                TextColor(Color::srgb(0.96, 0.79, 0.48)),
                Pickable::IGNORE,
            ));
        });
}

fn lives_copy(language: GameLanguage, lives: u32, detail: bool) -> String {
    let count = if language.is_chinese() {
        format!("剩余生命 {lives}")
    } else {
        format!("LIVES {lives}")
    };
    if detail {
        format!(
            "{count} · {}",
            language.pick(
                "The current life is included. No lives left ends the expedition.",
                "包含当前生命，耗尽后本次远征失败。"
            )
        )
    } else if lives == 1 {
        format!("{count} · {}", language.pick("LAST LIFE", "最后生命"))
    } else {
        count
    }
}
fn gate_copy(language: GameLanguage, down: bool, remaining: f32, restored: bool) -> String {
    if down {
        format!(
            "{} {:.0}{}",
            language.pick("GATE DOWN", "城门重建"),
            remaining.max(0.0).ceil(),
            language.pick("s", "秒")
        )
    } else if restored {
        language.pick("GATE RESTORED", "城门已重建").into()
    } else {
        language.pick("GATE", "城门").into()
    }
}
fn update(
    language: Res<GameLanguage>,
    session: Res<GameSession>,
    vitals: Res<HeroVitals>,
    time: Res<Time>,
    mut gate_state: Local<(bool, f32)>,
    mut labels: Query<(
        &mut Text,
        &mut TextColor,
        Option<&LivesLabel>,
        Option<&CoreIntegrityLabel>,
    )>,
) {
    let down = session.core_hp <= 0.0;
    if !session.paused {
        if gate_state.0 && !down {
            gate_state.1 = 2.0;
        }
        gate_state.0 = down;
        gate_state.1 = (gate_state.1 - time.delta_secs()).max(0.0);
    }
    for (mut text, mut color, lives, gate) in &mut labels {
        if let Some(lives) = lives {
            text.reborrow()
                .map_unchanged(|value| &mut value.0)
                .set_if_neq(lives_copy(*language, vitals.lives_remaining(), lives.0));
            color
                .reborrow()
                .map_unchanged(|value| &mut value.0)
                .set_if_neq(if vitals.lives_remaining() <= 1 {
                    Color::srgb(1.0, 0.36, 0.28)
                } else {
                    Color::srgb(0.96, 0.79, 0.48)
                });
        } else if gate.is_some() {
            let mut value = gate_copy(
                *language,
                down,
                session.gate_revive_remaining,
                gate_state.1 > 0.0,
            );
            if !down {
                value.push_str(&format!(
                    "  {:.0} / {:.0}",
                    session.core_hp, session.core_max_hp
                ));
            }
            text.reborrow()
                .map_unchanged(|text| &mut text.0)
                .set_if_neq(value);
            color
                .reborrow()
                .map_unchanged(|value| &mut value.0)
                .set_if_neq(if down {
                    Color::srgb(1.0, 0.57, 0.25)
                } else {
                    Color::srgb(0.5, 0.94, 0.77)
                });
        }
    }
}

#[derive(Component)]
struct GateCountdownRoot;
#[derive(Component)]
struct GateCountdownDigit;
#[derive(Component)]
struct GateCountdownCaption;

fn spawn_gate_countdown(mut commands: Commands) {
    commands
        .spawn((
            Name::new("Gate rebuild countdown"),
            GateCountdownRoot,
            Node {
                position_type: PositionType::Absolute,
                width: px(120),
                height: px(120),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                display: Display::None,
                border_radius: BorderRadius::all(px(18)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.025, 0.035, 0.045, 0.86)),
            GlobalZIndex(5),
            Pickable::IGNORE,
        ))
        .with_children(|parent| {
            parent.spawn((
                Name::new("Gate rebuild countdown digit"),
                GateCountdownDigit,
                Text::new("4"),
                TextFont {
                    font_size: FontSize::Px(64.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.76, 0.34)),
                Pickable::IGNORE,
            ));
            parent.spawn((
                GateCountdownCaption,
                Text::new("REBUILDING"),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.93, 0.88, 0.75)),
                Pickable::IGNORE,
            ));
        });
}
fn countdown_digit(remaining: f32) -> String {
    remaining
        .ceil()
        .clamp(1.0, crate::game::resources::GATE_REBUILD_SECONDS)
        .to_string()
}
fn update_gate_countdown(
    session: Res<GameSession>,
    language: Res<GameLanguage>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut roots: Query<&mut Node, With<GateCountdownRoot>>,
    mut digits: Query<&mut Text, (With<GateCountdownDigit>, Without<GateCountdownCaption>)>,
    mut captions: Query<&mut Text, (With<GateCountdownCaption>, Without<GateCountdownDigit>)>,
) {
    let down = session.core_hp <= 0.0 && !session.finished;
    let position = cameras.single().ok().and_then(|(camera, transform)| {
        camera
            .world_to_viewport(
                transform,
                crate::game::resources::GATE_POSITION + Vec3::Y * 0.5,
            )
            .ok()
    });
    for mut node in &mut roots {
        node.reborrow()
            .map_unchanged(|value| &mut value.display)
            .set_if_neq(if down && position.is_some() {
                Display::Flex
            } else {
                Display::None
            });
        if let Some(position) = position {
            node.reborrow()
                .map_unchanged(|value| &mut value.left)
                .set_if_neq(px(position.x - 60.0));
            node.reborrow()
                .map_unchanged(|value| &mut value.top)
                .set_if_neq(px(position.y - 60.0));
        }
    }
    for mut digit in &mut digits {
        digit
            .reborrow()
            .map_unchanged(|value| &mut value.0)
            .set_if_neq(countdown_digit(if session.gate_revive_remaining > 0.0 {
                session.gate_revive_remaining
            } else {
                crate::game::resources::GATE_REBUILD_SECONDS
            }));
    }
    for mut caption in &mut captions {
        caption
            .reborrow()
            .map_unchanged(|value| &mut value.0)
            .set_if_neq(language.pick("REBUILDING", "重建中").into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn countdown_shows_four_integer_steps() {
        for (remaining, expected) in [
            (4.0, "4"),
            (3.01, "4"),
            (3.0, "3"),
            (2.0, "2"),
            (1.0, "1"),
            (0.01, "1"),
        ] {
            assert_eq!(countdown_digit(remaining), expected);
        }
    }
    #[test]
    fn life_and_gate_messages_distinguish_loss_from_rebuilding() {
        assert_eq!(
            lives_copy(GameLanguage::English, 1, false),
            "LIVES 1 · LAST LIFE"
        );
        assert!(lives_copy(GameLanguage::English, 0, true).contains("ends the expedition"));
        assert_eq!(
            gate_copy(GameLanguage::English, true, 2.4, false),
            "GATE DOWN 3s"
        );
        assert_eq!(
            gate_copy(GameLanguage::English, false, 0.0, true),
            "GATE RESTORED"
        );
    }
}
