//! Device failure detection and real-time recovery, outside the audio callback.
use crate::{AudioSink, SpatialAudioSink, audio_output::AudioOutput};
use alloc::sync::Arc;
use bevy_ecs::prelude::*;
use bevy_ecs::system::NonSendMarker;
use bevy_time::{Real, Time};
use core::sync::atomic::{AtomicBool, Ordering};
use rodio::{
    DeviceSinkBuilder, MixerDeviceSink,
    cpal::traits::{DeviceTrait, HostTrait},
};
use tracing::warn;

/// Whether the current output device is accepting audio.
/// Failed output becomes unavailable immediately, even before the next frame.
#[derive(Resource, Clone, Default)]
pub struct AudioOutputStatus {
    available: Arc<AtomicBool>,
    fault: Arc<AtomicBool>,
}
impl AudioOutputStatus {
    /// Returns false while the device is absent, failed, or awaiting recovery.
    pub fn is_available(&self) -> bool {
        self.available.load(Ordering::Acquire) && !self.fault.load(Ordering::Acquire)
    }
}

pub(crate) struct OutputRecovery<T> {
    pub(crate) stream: Option<T>,
    pub(crate) status: AudioOutputStatus,
    next_retry: f64,
}
impl<T> OutputRecovery<T> {
    pub(crate) fn new(mut open: impl FnMut(Arc<AtomicBool>) -> Option<T>) -> Self {
        let status = AudioOutputStatus::default();
        let stream = open(status.fault.clone());
        status.available.store(stream.is_some(), Ordering::Release);
        Self {
            stream,
            status,
            next_retry: 5.0,
        }
    }
    pub(crate) fn is_available(&self) -> bool {
        self.status.is_available()
    }

    // Returns true once per broken stream so its old sink components are removed.
    fn tick(&mut self, now: f64, mut open: impl FnMut(Arc<AtomicBool>) -> Option<T>) -> bool {
        let disconnected = self.status.fault.load(Ordering::Acquire) && self.stream.is_some();
        if disconnected {
            self.status.available.store(false, Ordering::Release);
            // Drop on the caller's thread, never in the CPAL error callback.
            drop(self.stream.take());
            self.next_retry = now + 5.0;
        }
        if self.stream.is_none() && now >= self.next_retry {
            self.status.fault.store(false, Ordering::Release);
            self.stream = open(self.status.fault.clone());
            self.status
                .available
                .store(self.stream.is_some(), Ordering::Release);
            self.next_retry = now + 5.0;
        }
        disconnected
    }
}

fn error_callback(
    fault: Arc<AtomicBool>,
) -> impl FnMut(rodio::cpal::StreamError) + Clone + Send + 'static {
    move |error: rodio::cpal::StreamError| {
        if !fault.swap(true, Ordering::AcqRel) {
            warn!("Audio device failed; playback stopped until recovery: {error}");
        }
    }
}

pub(crate) fn open_output(fault: Arc<AtomicBool>) -> Option<MixerDeviceSink> {
    let callback = error_callback(fault);
    let result = DeviceSinkBuilder::from_default_device()
        .and_then(|builder| {
            builder
                .with_error_callback(callback.clone())
                .open_sink_or_fallback()
        })
        .or_else(|original_error| {
            let Ok(devices) = rodio::cpal::default_host().output_devices() else {
                return Err(original_error);
            };
            devices
                .filter(|device| {
                    device
                        .description()
                        .map(|description| {
                            description.driver().is_some_and(|driver| driver != "null")
                        })
                        .unwrap_or(false)
                })
                .find_map(|device| {
                    DeviceSinkBuilder::from_device(device)
                        .and_then(|builder| {
                            builder
                                .with_error_callback(callback.clone())
                                .open_sink_or_fallback()
                        })
                        .ok()
                })
                .ok_or(original_error)
        });
    match result {
        Ok(mut stream) => {
            stream.log_on_drop(false);
            Some(stream)
        }
        Err(error) => {
            warn!("Audio device unavailable; retrying in 5 seconds: {error}");
            None
        }
    }
}

pub(crate) fn maintain_audio_output(
    _main_thread: NonSendMarker,
    time: Res<Time<Real>>,
    mut output: ResMut<AudioOutput>,
    sinks: Query<Entity, Or<(With<AudioSink>, With<SpatialAudioSink>)>>,
    commands: Commands,
) {
    let was_available = output.recovery.is_available();
    if output.recovery.tick(time.elapsed_secs_f64(), open_output) {
        detach_failed_sinks(sinks, commands);
    }
    if !was_available && output.recovery.is_available() {
        tracing::info!("Audio output restored.");
    }
}

fn detach_failed_sinks(
    sinks: Query<Entity, Or<(With<AudioSink>, With<SpatialAudioSink>)>>,
    mut commands: Commands,
) {
    for entity in &sinks {
        // Keep AudioPlayer and PlaybackSettings, including looping music.
        commands
            .entity(entity)
            .remove::<(AudioSink, SpatialAudioSink)>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::AtomicUsize;

    #[test]
    fn device_maintenance_is_bound_to_the_main_thread() {
        use bevy_ecs::system::{IntoSystem, System};
        let mut system = IntoSystem::into_system(maintain_audio_output);
        system.initialize(&mut World::new());
        assert!(!system.is_send());
    }

    #[test]
    fn failed_sink_detachment_preserves_loop_sources_and_playback_preferences() {
        use crate::{AudioPlayer, AudioSource, PlaybackSettings, Volume};
        use bevy_app::{App, Update};
        use bevy_asset::Handle;
        let (mixer, _samples) =
            rodio::mixer::mixer(2.try_into().unwrap(), 44_100.try_into().unwrap());
        let mut app = App::new();
        app.add_systems(Update, detach_failed_sinks);
        let music = app
            .world_mut()
            .spawn((
                AudioPlayer::new(Handle::<AudioSource>::default()),
                PlaybackSettings::LOOP
                    .with_volume(Volume::Linear(0.36))
                    .with_speed(0.8),
                AudioSink::new(rodio::Player::connect_new(&mixer)),
            ))
            .id();
        let spatial = app
            .world_mut()
            .spawn(SpatialAudioSink::new(rodio::SpatialPlayer::connect_new(
                &mixer,
                [0.0; 3],
                [-1.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
            )))
            .id();
        app.update();
        assert!(app.world().get::<AudioSink>(music).is_none());
        assert!(app.world().get::<SpatialAudioSink>(spatial).is_none());
        assert!(app.world().get::<AudioPlayer>(music).is_some());
        let settings = app.world().get::<PlaybackSettings>(music).unwrap();
        assert_eq!(settings.speed, 0.8);
        assert_eq!(settings.volume.to_linear(), 0.36);
        assert!(matches!(settings.mode, crate::PlaybackMode::Loop));
    }
    struct EventCounter(Arc<AtomicUsize>);
    impl tracing::Subscriber for EventCounter {
        fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }
        fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
        fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
        fn event(&self, event: &tracing::Event<'_>) {
            assert_eq!(*event.metadata().level(), tracing::Level::WARN);
            self.0.fetch_add(1, Ordering::Relaxed);
        }
        fn enter(&self, _: &tracing::span::Id) {}
        fn exit(&self, _: &tracing::span::Id) {}
    }
    #[test]
    fn repeated_poll_errors_emit_one_warning_and_mark_output_failed() {
        let fault = Arc::new(AtomicBool::new(false));
        let events = Arc::new(AtomicUsize::new(0));
        let mut callback = error_callback(fault.clone());
        tracing::subscriber::with_default(EventCounter(events.clone()), || {
            for _ in 0..10_000 {
                callback(rodio::cpal::StreamError::BackendSpecific {
                    err: rodio::cpal::BackendSpecificError {
                        description: "`alsa::poll()` returned POLLERR".into(),
                    },
                });
            }
        });
        assert_eq!(events.load(Ordering::Relaxed), 1);
        assert!(fault.load(Ordering::Acquire));
    }
    struct FakeStream(Arc<AtomicUsize>);
    impl Drop for FakeStream {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }
    #[test]
    fn failure_drops_once_and_retries_after_five_real_seconds() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut recovery = OutputRecovery::new(|_| Some(FakeStream(drops.clone())));
        let status = recovery.status.clone();
        recovery.status.fault.store(true, Ordering::Release);
        assert!(!status.is_available());
        assert!(recovery.tick(20.0, |_| panic!("early retry")));
        assert_eq!(drops.load(Ordering::Relaxed), 1);
        assert!(!recovery.tick(24.999, |_| panic!("early retry")));
        assert!(!recovery.tick(25.0, |_| None));
        assert!(!recovery.tick(29.999, |_| panic!("early retry")));
        assert!(!recovery.tick(30.0, |_| Some(FakeStream(drops.clone()))));
        assert!(status.is_available());
        assert_eq!(drops.load(Ordering::Relaxed), 1);
    }
    #[test]
    fn initial_device_absence_retries_without_a_fault_or_game_clock() {
        let mut recovery = OutputRecovery::<()>::new(|_| None);
        assert!(!recovery.status.is_available());
        assert!(!recovery.tick(4.999, |_| panic!("early retry")));
        assert!(!recovery.tick(5.0, |_| Some(())));
        assert!(recovery.status.is_available());
        assert!(!recovery.tick(500.0, |_| panic!("healthy stream reopened")));
    }
}
