//! Drop transient voices during device loss; retain music for the recovered mixer.
use super::{AudioChannel, AudioVoice};
use bevy::{audio::AudioOutputStatus, prelude::*};

pub(super) fn discard_unavailable_effects(
    output: Option<Res<AudioOutputStatus>>,
    voices: Query<(Entity, &AudioVoice)>,
    mut commands: Commands,
) {
    if output.as_ref().is_none_or(|status| status.is_available()) {
        return;
    }
    for (entity, voice) in &voices {
        if voice.channel == AudioChannel::Effects {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_absence_clears_queued_effects_but_preserves_music_and_game_entities() {
        let mut app = App::new();
        app.init_resource::<AudioOutputStatus>()
            .add_systems(First, discard_unavailable_effects);
        let music = app
            .world_mut()
            .spawn((
                AudioPlayer::new(Handle::<AudioSource>::default()),
                AudioVoice::new(AudioChannel::Music, 0.36),
                PlaybackSettings::LOOP.with_speed(0.8),
            ))
            .id();
        let game_entity = app.world_mut().spawn(Transform::default()).id();
        for _ in 0..20 {
            app.world_mut().spawn((
                AudioPlayer::new(Handle::<AudioSource>::default()),
                AudioVoice::new(AudioChannel::Effects, 0.5),
                PlaybackSettings::DESPAWN,
            ));
        }
        app.update();
        assert!(app.world().get_entity(game_entity).is_ok());
        assert!(app.world().get::<AudioPlayer>(music).is_some());
        assert_eq!(
            app.world().get::<PlaybackSettings>(music).unwrap().speed,
            0.8
        );
        assert_eq!(
            app.world_mut()
                .query::<&AudioVoice>()
                .iter(app.world())
                .count(),
            1
        );
    }
}
