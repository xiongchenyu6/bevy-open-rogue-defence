//! Independent music/effect preferences applied to both queued and playing voices.
use bevy::{
    audio::{AudioSinkPlayback, Volume},
    prelude::*,
};
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioPreferences {
    pub music: u8,
    pub effects: u8,
    pub muted: bool,
}
impl Default for AudioPreferences {
    fn default() -> Self {
        crate::preferences::load_audio().unwrap_or(Self {
            music: 100,
            effects: 100,
            muted: false,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioChannel {
    Music,
    Effects,
}
#[derive(Component, Clone, Copy, Debug)]
pub struct AudioVoice {
    pub channel: AudioChannel,
    pub base_volume: f32,
}
impl AudioVoice {
    pub fn new(channel: AudioChannel, base_volume: f32) -> Self {
        Self {
            channel,
            base_volume,
        }
    }
}
#[derive(Resource, Default, Debug)]
pub struct AudioSaveStatus {
    pub failed: bool,
}
#[derive(Resource)]
struct Saved(AudioPreferences);
impl FromWorld for Saved {
    fn from_world(world: &mut World) -> Self {
        Self(*world.resource::<AudioPreferences>())
    }
}
pub struct AudioMixPlugin;
impl Plugin for AudioMixPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AudioPreferences>()
            .init_resource::<AudioSaveStatus>()
            .init_resource::<Saved>()
            .add_systems(
                PostUpdate,
                (mix, persist).before(TransformSystems::Propagate),
            );
    }
}
fn gain(preferences: &AudioPreferences, voice: &AudioVoice) -> f32 {
    if preferences.muted {
        return 0.0;
    }
    let percent = match voice.channel {
        AudioChannel::Music => preferences.music,
        AudioChannel::Effects => preferences.effects,
    }
    .min(100);
    let base = if voice.base_volume.is_finite() {
        voice.base_volume.max(0.0)
    } else {
        0.0
    };
    base * f32::from(percent) / 100.0
}
fn mix(
    preferences: Res<AudioPreferences>,
    mut voices: Query<(&AudioVoice, &mut PlaybackSettings, Option<&mut AudioSink>)>,
) {
    for (voice, mut settings, sink) in &mut voices {
        let volume = Volume::Linear(gain(&preferences, voice));
        if settings.volume.to_linear() != volume.to_linear() {
            settings.volume = volume;
        }
        if let Some(mut sink) = sink
            && sink.volume().to_linear() != volume.to_linear()
        {
            sink.set_volume(volume);
        }
    }
}
fn persist(
    preferences: Res<AudioPreferences>,
    mut saved: ResMut<Saved>,
    mut status: ResMut<AudioSaveStatus>,
) {
    if *preferences == saved.0 {
        return;
    }
    status.failed = crate::preferences::save_audio(&preferences).is_err();
    saved.0 = *preferences;
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unchanged_preferences_do_not_write_and_invalid_save_reports_failure() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        let initial = AudioPreferences {
            music: 100,
            effects: 100,
            muted: false,
        };
        world.insert_resource(initial);
        world.insert_resource(Saved(initial));
        world.init_resource::<AudioSaveStatus>();
        world.run_system_once(persist).unwrap();
        assert!(!world.resource::<AudioSaveStatus>().failed);
        world.resource_mut::<AudioPreferences>().effects = 255;
        world.run_system_once(persist).unwrap();
        assert!(world.resource::<AudioSaveStatus>().failed);
        assert_eq!(world.resource::<Saved>().0.effects, 255);
    }
    #[test]
    fn actual_playback_settings_mix_independently_without_cumulative_scaling() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        world.insert_resource(AudioPreferences {
            music: 25,
            effects: 50,
            muted: false,
        });
        let music = world
            .spawn((
                AudioVoice::new(AudioChannel::Music, 0.36),
                PlaybackSettings::LOOP.with_volume(Volume::Linear(0.36)),
            ))
            .id();
        let effect = world
            .spawn((
                AudioVoice::new(AudioChannel::Effects, 0.55),
                PlaybackSettings::DESPAWN.with_volume(Volume::Linear(0.55)),
            ))
            .id();
        for _ in 0..10 {
            world.run_system_once(mix).unwrap();
        }
        assert!(
            (world
                .get::<PlaybackSettings>(music)
                .unwrap()
                .volume
                .to_linear()
                - 0.09)
                .abs()
                < 0.00001
        );
        assert!(
            (world
                .get::<PlaybackSettings>(effect)
                .unwrap()
                .volume
                .to_linear()
                - 0.275)
                .abs()
                < 0.00001
        );
        world.resource_mut::<AudioPreferences>().muted = true;
        world.run_system_once(mix).unwrap();
        assert_eq!(
            world
                .get::<PlaybackSettings>(music)
                .unwrap()
                .volume
                .to_linear(),
            0.0
        );
        world.resource_mut::<AudioPreferences>().muted = false;
        world.resource_mut::<AudioPreferences>().effects = 100;
        world.run_system_once(mix).unwrap();
        assert_eq!(
            world
                .get::<PlaybackSettings>(effect)
                .unwrap()
                .volume
                .to_linear(),
            0.55
        );
    }
}
