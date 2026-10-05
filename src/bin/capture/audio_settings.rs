//! Exercises actual settings controls, persistent preferences and music playback gain.
use bevy::{audio::AudioSinkPlayback, prelude::*};
use rogue_commander::game::{
    AudioChannel, AudioPreferences, AudioSaveStatus, AudioVoice, Autoplay, GameSession,
    SettingsInputSet,
};

#[derive(Resource, Default)]
struct State {
    frame: u32,
    original_pause: bool,
    music_checked: bool,
    reload: bool,
    live_sink_checked: bool,
    failure: bool,
}

pub fn install(app: &mut bevy::app::SubApp, reload: bool, failure: bool) {
    app.world_mut().remove_resource::<Autoplay>();
    if !reload {
        app.insert_resource(AudioPreferences {
            music: 50,
            effects: 70,
            muted: false,
        });
    }
    app.insert_resource(State {
        reload,
        failure,
        ..default()
    })
    .add_systems(
        PreUpdate,
        exercise
            .after(bevy::ui::UiSystems::Focus)
            .before(SettingsInputSet),
    );
}

#[allow(clippy::too_many_arguments)]
fn exercise(
    mut state: ResMut<State>,
    audio: Res<AudioPreferences>,
    save: Res<AudioSaveStatus>,
    session: Res<GameSession>,
    mut buttons: Query<(&Name, &mut Interaction), With<Button>>,
    layout: Query<(&Name, &ComputedNode, &UiGlobalTransform), With<Button>>,
    windows: Query<&Window>,
    texts: Query<(&Text, &ComputedNode)>,
    voices: Query<(&AudioVoice, &PlaybackSettings, Option<&AudioSink>)>,
) {
    state.frame += 1;
    if state.frame == 9 {
        state.original_pause = session.paused;
    }
    if state.reload {
        if state.frame == 10 {
            for (name, mut interaction) in &mut buttons {
                if name.as_str() == "Settings open" {
                    *interaction = Interaction::Pressed;
                }
            }
        }
        if state.frame == 41 {
            assert!(session.paused);
            assert_eq!((audio.music, audio.effects, audio.muted), (40, 80, false));
            let mut music = false;
            for (voice, settings, sink) in &voices {
                if voice.channel == AudioChannel::Music {
                    assert!(
                        (settings.volume.to_linear() - voice.base_volume * 0.4).abs() < 0.00001
                    );
                    if let Some(sink) = sink {
                        assert!(
                            (sink.volume().to_linear() - voice.base_volume * 0.4).abs() < 0.00001
                        );
                        state.live_sink_checked = true;
                    }
                    music = true;
                }
            }
            assert!(music);
            println!(
                "AUDIO_RELOAD_PASS fresh_process=true music=40 effects=80 muted=false actual_music_gain=true"
            );
        }
        return;
    }
    let action = match state.frame {
        10 | 65 => Some("Settings open"),
        20 => Some("Settings music down"),
        25 => Some("Settings effects up"),
        30 | 75 => Some("Settings mute"),
        50 | 90 => Some("Settings close"),
        _ => None,
    };
    if let Some(target) = action {
        let mut found = false;
        for (name, mut interaction) in &mut buttons {
            if name.as_str() == target {
                *interaction = Interaction::Pressed;
                found = true;
            }
        }
        assert!(found, "missing audio control {target}");
    }
    if [41, 70].contains(&state.frame) {
        assert!(session.paused);
        assert_eq!((audio.music, audio.effects, audio.muted), (40, 80, true));
        assert_eq!(
            save.failed, state.failure,
            "audio save status must reflect actual storage"
        );
        if state.failure {
            assert!(
                texts.iter().any(|(text, node)| node.size().y > 0.0
                    && (text.0.contains("could not be saved")
                        || text.0.contains("无法在此设备上保存偏好"))),
                "visible save failure notice required"
            );
        }
        let window = windows.single().unwrap();
        let viewport = Vec2::new(
            window.physical_width() as f32,
            window.physical_height() as f32,
        );
        let mut controls = 0;
        for (name, node, transform) in &layout {
            if !name.as_str().starts_with("Settings ") || node.size().min_element() == 0.0 {
                continue;
            }
            let half = node.size() * 0.5;
            assert!(
                (transform.translation - half)
                    .cmpge(Vec2::splat(-1.0))
                    .all()
                    && (transform.translation + half)
                        .cmple(viewport + Vec2::ONE)
                        .all(),
                "{name} clipped"
            );
            assert!(
                node.size().min_element() >= 44.0,
                "{name} touch target too small"
            );
            controls += 1;
        }
        assert!(
            controls >= 8,
            "language, audio and back controls must all fit"
        );
        for (voice, settings, sink) in &voices {
            if voice.channel == AudioChannel::Music {
                assert_eq!(settings.volume.to_linear(), 0.0);
                if let Some(sink) = sink {
                    assert_eq!(sink.volume().to_linear(), 0.0);
                    state.live_sink_checked = true;
                }
                state.music_checked = true;
            }
        }
    }
    if state.frame == 100 {
        assert_eq!(session.paused, state.original_pause);
        assert_eq!((audio.music, audio.effects, audio.muted), (40, 80, false));
        assert_eq!(save.failed, state.failure);
        assert!(state.music_checked, "actual ambient music was muted");
        for (voice, settings, sink) in &voices {
            if voice.channel == AudioChannel::Music {
                assert!((settings.volume.to_linear() - voice.base_volume * 0.4).abs() < 0.00001);
                if let Some(sink) = sink {
                    assert!((sink.volume().to_linear() - voice.base_volume * 0.4).abs() < 0.00001);
                }
            }
        }
        let path =
            std::env::var("ROGUE_AUDIO_SAVE").expect("isolated audio preference path required");
        if state.failure {
            assert!(std::path::Path::new(&path).is_dir());
            println!(
                "AUDIO_SAVE_FAILURE_PASS changes_apply=true failure_notice_visible=true pause_restored=true"
            );
        } else {
            let stored = std::fs::read_to_string(path).expect("saved audio preferences exist");
            assert_eq!(stored.trim(), "1 40 80 0");
        }
        println!(
            "AUDIO_DEVICE_SCOPE live_music_sink={}",
            state.live_sink_checked
        );
        println!(
            "AUDIO_SETTINGS_PASS music=40 effects=80 mute_restore=true actual_music_gain=true controls_fit=true pause_restored=true save_status_verified=true"
        );
    }
}
