//! Independent music/effects controls shared with the actual audio mixer.
use super::{EDGE, GOLD, IVORY, MUTED, PANEL, SettingsInputSet, SettingsState, label};
use crate::game::{AudioPreferences, GameLanguage};
use bevy::prelude::*;

#[derive(Component, Clone, Copy)]
enum Control {
    Music(i16),
    Effects(i16),
    Mute,
}
#[derive(Component, Clone, Copy)]
enum Copy {
    Music,
    Effects,
    Sound,
    MusicValue,
    EffectsValue,
    Mute,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<AudioPreferences>()
        .add_systems(PreUpdate, handle.after(SettingsInputSet))
        .add_systems(Update, update);
}

pub(super) fn spawn(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn(Node {
            width: percent(100),
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(8),
            row_gap: px(8),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|row| {
            for (music, heading) in [(true, Copy::Music), (false, Copy::Effects)] {
                row.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    flex_basis: px(0),
                    flex_grow: 2.0,
                    min_width: px(142),
                    row_gap: px(6),
                    ..default()
                })
                .with_children(|channel| {
                    channel.spawn((heading, label("", 13.0, MUTED)));
                    channel
                        .spawn(Node {
                            align_items: AlignItems::Center,
                            width: percent(100),
                            ..default()
                        })
                        .with_children(|controls| {
                            button(
                                controls,
                                if music {
                                    Control::Music(-10)
                                } else {
                                    Control::Effects(-10)
                                },
                                if music {
                                    "Settings music down"
                                } else {
                                    "Settings effects down"
                                },
                                "−",
                            );
                            controls.spawn((
                                if music {
                                    Copy::MusicValue
                                } else {
                                    Copy::EffectsValue
                                },
                                label("", 15.0, IVORY),
                                TextLayout::default().with_justify(Justify::Center),
                                Node {
                                    min_width: px(42),
                                    flex_grow: 1.0,
                                    justify_content: JustifyContent::Center,
                                    ..default()
                                },
                            ));
                            button(
                                controls,
                                if music {
                                    Control::Music(10)
                                } else {
                                    Control::Effects(10)
                                },
                                if music {
                                    "Settings music up"
                                } else {
                                    "Settings effects up"
                                },
                                "+",
                            );
                        });
                });
            }
            row.spawn(Node {
                flex_direction: FlexDirection::Column,
                flex_basis: px(0),
                flex_grow: 1.0,
                min_width: px(88),
                row_gap: px(6),
                ..default()
            })
            .with_children(|mute| {
                mute.spawn((Copy::Sound, label("", 13.0, MUTED)));
                mute.spawn((
                    Button,
                    Control::Mute,
                    Name::new("Settings mute"),
                    Node {
                        min_height: px(44),
                        height: px(44),
                        width: percent(100),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        border: UiRect::all(px(1)),
                        ..default()
                    },
                    BackgroundColor(PANEL),
                    BorderColor::all(EDGE),
                ))
                .with_children(|button| {
                    button.spawn((Copy::Mute, label("", 13.0, IVORY)));
                });
            });
        });
}

fn button(parent: &mut ChildSpawnerCommands, control: Control, name: &'static str, text: &str) {
    parent
        .spawn((
            Button,
            control,
            Name::new(name),
            Node {
                width: px(44),
                min_width: px(44),
                height: px(44),
                min_height: px(44),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                ..default()
            },
            BackgroundColor(PANEL),
            BorderColor::all(EDGE),
        ))
        .with_children(|button| {
            button.spawn(label(text, 19.0, IVORY));
        });
}

fn handle(
    state: Res<SettingsState>,
    buttons: Query<(&Interaction, &Control), Changed<Interaction>>,
    mut audio: ResMut<AudioPreferences>,
) {
    if !state.open {
        return;
    }
    for (interaction, control) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match *control {
            Control::Music(step) => {
                audio.music = (i16::from(audio.music) + step).clamp(0, 100) as u8
            }
            Control::Effects(step) => {
                audio.effects = (i16::from(audio.effects) + step).clamp(0, 100) as u8
            }
            Control::Mute => audio.muted = !audio.muted,
        }
    }
}

fn update(
    audio: Res<AudioPreferences>,
    language: Res<GameLanguage>,
    mut texts: Query<(&Copy, &mut Text)>,
    mut buttons: Query<(
        &Control,
        &Interaction,
        &mut BorderColor,
        &mut BackgroundColor,
    )>,
) {
    for (copy, mut text) in &mut texts {
        text.set_if_neq(Text(match copy {
            Copy::Music => language.pick("MUSIC", "音乐").into(),
            Copy::Effects => language.pick("EFFECTS", "效果音").into(),
            Copy::Sound => language.pick("SOUND", "声音").into(),
            Copy::MusicValue => format!("{}%", audio.music),
            Copy::EffectsValue => format!("{}%", audio.effects),
            Copy::Mute => if audio.muted {
                language.pick("Unmute", "开启声音")
            } else {
                language.pick("Mute", "静音")
            }
            .into(),
        }));
    }
    for (control, interaction, mut border, mut background) in &mut buttons {
        let active = matches!(control, Control::Mute) && audio.muted;
        border.set_if_neq(BorderColor::all(if active {
            GOLD
        } else if *interaction != Interaction::None {
            IVORY
        } else {
            EDGE
        }));
        background
            .reborrow()
            .map_unchanged(|color| &mut color.0)
            .set_if_neq(if active {
                Color::srgb(0.18, 0.14, 0.07)
            } else if *interaction != Interaction::None {
                Color::srgb(0.11, 0.16, 0.15)
            } else {
                PANEL
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unchanged_audio_labels_are_clean_and_volume_language_changes_refresh() {
        use crate::game::settings::{TextRefreshProbe, count_text_refreshes};
        let mut app = App::new();
        app.insert_resource(AudioPreferences {
            music: 30,
            effects: 50,
            muted: false,
        })
        .init_resource::<GameLanguage>()
        .init_resource::<TextRefreshProbe>()
        .add_systems(Update, update)
        .add_systems(Last, count_text_refreshes);
        let volume = app
            .world_mut()
            .spawn((Copy::MusicValue, Text::default()))
            .id();
        let sound = app.world_mut().spawn((Copy::Sound, Text::default())).id();
        app.update();
        app.update();
        assert_eq!(app.world().resource::<TextRefreshProbe>().0, 0);
        app.world_mut().resource_mut::<AudioPreferences>().music = 40;
        app.update();
        assert_eq!(app.world().resource::<TextRefreshProbe>().0, 1);
        assert_eq!(app.world().get::<Text>(volume).unwrap().0, "40%");
        app.world_mut()
            .insert_resource(GameLanguage::SimplifiedChinese);
        app.update();
        assert_eq!(app.world().resource::<TextRefreshProbe>().0, 1);
        assert_eq!(app.world().get::<Text>(sound).unwrap().0, "声音");
        app.update();
        assert_eq!(app.world().resource::<TextRefreshProbe>().0, 0);
    }

    #[test]
    fn actual_settings_buttons_adjust_channels_independently_and_preserve_levels_when_muted() {
        let mut app = App::new();
        app.insert_resource(SettingsState {
            open: true,
            ..default()
        })
        .insert_resource(AudioPreferences {
            music: 50,
            effects: 70,
            muted: false,
        })
        .add_systems(PreUpdate, handle);
        let music = app
            .world_mut()
            .spawn((Control::Music(-10), Interaction::Pressed))
            .id();
        app.update();
        assert_eq!(app.world().resource::<AudioPreferences>().music, 40);
        assert_eq!(app.world().resource::<AudioPreferences>().effects, 70);
        app.world_mut().despawn(music);
        let mute = app
            .world_mut()
            .spawn((Control::Mute, Interaction::Pressed))
            .id();
        app.update();
        assert!(app.world().resource::<AudioPreferences>().muted);
        assert_eq!(app.world().resource::<AudioPreferences>().music, 40);
        app.world_mut().despawn(mute);
        let effects = app
            .world_mut()
            .spawn((Control::Effects(50), Interaction::Pressed))
            .id();
        app.update();
        assert_eq!(app.world().resource::<AudioPreferences>().effects, 100);
        app.world_mut().despawn(effects);
        app.world_mut().spawn((Control::Mute, Interaction::Pressed));
        app.update();
        assert!(!app.world().resource::<AudioPreferences>().muted);
        assert_eq!(app.world().resource::<AudioPreferences>().music, 40);
    }

    #[test]
    fn closed_settings_reject_stale_controls() {
        let mut app = App::new();
        app.init_resource::<SettingsState>()
            .insert_resource(AudioPreferences {
                music: 0,
                effects: 0,
                muted: false,
            })
            .add_systems(PreUpdate, handle);
        app.world_mut()
            .spawn((Control::Music(10), Interaction::Pressed));
        app.world_mut()
            .spawn((Control::Effects(10), Interaction::Pressed));
        app.world_mut().spawn((Control::Mute, Interaction::Pressed));
        app.update();
        let audio = app.world().resource::<AudioPreferences>();
        assert_eq!((audio.music, audio.effects, audio.muted), (0, 0, false));
    }
}
