use super::audio_mix::{AudioChannel, AudioVoice};
use bevy::{audio::Volume, prelude::*};

use super::bonds::DamageElement;
use super::gate_feedback::GateImpact;
use super::hit_feedback::{HitFeedback, HitFeedbackSet, HitWeight};
use super::ultimate_vfx::UltimateCues;
use super::{Autoplay, BossEnemy, DraftRoot, Projectile};

#[path = "audio_output_health.rs"]
mod output_health;

pub struct GameAudioPlugin;

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameAudio>()
            .init_resource::<AudioThrottle>()
            .init_resource::<HitAudioThrottle>()
            .add_systems(First, output_health::discard_unavailable_effects)
            .add_systems(Startup, start_ambient)
            .add_systems(
                Update,
                (play_feedback, play_hit_feedback.after(HitFeedbackSet)),
            );
    }
}

#[derive(Resource)]
struct GameAudio {
    ambient: Handle<AudioSource>,
    shot: Handle<AudioSource>,
    impact: Handle<AudioSource>,
    draft: Handle<AudioSource>,
    ultimate: Handle<AudioSource>,
    boss_spawn: Handle<AudioSource>,
    hit_physical: Handle<AudioSource>,
    hit_magic: Handle<AudioSource>,
    crit: Handle<AudioSource>,
    kill: Handle<AudioSource>,
    kill_heavy: Handle<AudioSource>,
    hero_hurt: Handle<AudioSource>,
    gate_hit: Handle<AudioSource>,
}

impl FromWorld for GameAudio {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            ambient: assets.load("audio/arena_loop.ogg"),
            shot: assets.load("audio/shot.wav"),
            impact: assets.load("audio/impact.wav"),
            draft: assets.load("audio/draft.wav"),
            ultimate: assets.load("audio/ultimate.wav"),
            boss_spawn: assets.load("audio/boss_spawn.wav"),
            hit_physical: assets.load("audio/hit_physical.wav"),
            hit_magic: assets.load("audio/hit_magic.wav"),
            crit: assets.load("audio/crit.wav"),
            kill: assets.load("audio/kill.wav"),
            kill_heavy: assets.load("audio/kill_heavy.wav"),
            hero_hurt: assets.load("audio/hero_hurt.wav"),
            gate_hit: assets.load("audio/gate_hit.wav"),
        }
    }
}

#[derive(Resource, Default)]
struct AudioThrottle {
    shot: f32,
    ultimate: f32,
    boss: f32,
    previous_ultimate_serial: u64,
    previous_crafting_revision: u32,
}

impl AudioThrottle {
    fn tick(&mut self, delta: f32) {
        for timer in [&mut self.shot, &mut self.ultimate, &mut self.boss] {
            *timer = (*timer - delta).max(0.0);
        }
    }
}

/// Real-time cooldowns so fast-forwarded battles do not stack hundreds of voices.
#[derive(Resource, Default)]
struct HitAudioThrottle {
    hit: f32,
    heavy_hit: f32,
    crit: f32,
    kill: f32,
    kill_heavy: f32,
    hero_hurt: f32,
    variation: u32,
}

impl HitAudioThrottle {
    fn tick(&mut self, delta: f32) {
        for timer in [
            &mut self.hit,
            &mut self.heavy_hit,
            &mut self.crit,
            &mut self.kill,
            &mut self.kill_heavy,
            &mut self.hero_hurt,
        ] {
            *timer = (*timer - delta).max(0.0);
        }
    }

    /// Cheap deterministic pitch spread in `[1 - spread, 1 + spread]`.
    fn pitch(&mut self, spread: f32) -> f32 {
        self.variation = self
            .variation
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        let unit = (self.variation >> 8) as f32 / (1u32 << 24) as f32;
        1.0 + (unit * 2.0 - 1.0) * spread
    }
}

fn ready(timer: &mut f32, cooldown: f32) -> bool {
    if *timer > 0.0 {
        return false;
    }
    *timer = cooldown;
    true
}

/// Many simultaneous hits (area spells) sound bigger, but stay bounded.
fn crowd_volume(base: f32, count: usize) -> f32 {
    base * (1.0 + 0.12 * (count.max(1) as f32).ln()).min(1.45)
}

fn start_ambient(mut commands: Commands, audio: Res<GameAudio>, autoplay: Option<Res<Autoplay>>) {
    if autoplay.is_some() {
        return;
    }
    commands.spawn((
        AudioPlayer::new(audio.ambient.clone()),
        AudioVoice::new(AudioChannel::Music, 0.36),
        PlaybackSettings::LOOP.with_volume(Volume::Linear(0.36)),
    ));
}

fn play_feedback(
    mut commands: Commands,
    time: Res<Time<Real>>,
    audio: Res<GameAudio>,
    cues: Res<UltimateCues>,
    crafting: Option<Res<super::crafting::CraftingState>>,
    autoplay: Option<Res<Autoplay>>,
    bosses: Query<(), Added<BossEnemy>>,
    projectiles: Query<(), Added<Projectile>>,
    drafts: Query<(), Added<DraftRoot>>,
    mut throttle: ResMut<AudioThrottle>,
    output: Option<Res<bevy::audio::AudioOutputStatus>>,
) {
    let crafting_revision = crafting.as_ref().map_or(0, |state| state.revision);
    if autoplay.is_some() || output.as_ref().is_some_and(|status| !status.is_available()) {
        // Consume cues while disconnected so recovery cannot replay old ultimates.
        throttle.previous_ultimate_serial = cues.serial;
        throttle.previous_crafting_revision = crafting_revision;
        return;
    }
    let delta = time.delta_secs();
    throttle.tick(delta);

    if !bosses.is_empty() && ready(&mut throttle.boss, 0.35) {
        play_once(&mut commands, &audio.boss_spawn, 0.72);
    }
    if crafting_revision != throttle.previous_crafting_revision {
        play_once(&mut commands, &audio.draft, 0.65);
    }
    throttle.previous_crafting_revision = crafting_revision;
    if !drafts.is_empty() {
        play_once(&mut commands, &audio.draft, 0.50);
    }
    if !projectiles.is_empty() && ready(&mut throttle.shot, 0.085) {
        play_once(&mut commands, &audio.shot, 0.22);
    }
    if cues.serial != throttle.previous_ultimate_serial && ready(&mut throttle.ultimate, 0.5) {
        play_once(&mut commands, &audio.ultimate, 0.58);
    }
    // Consume suppressed cues now: they must not replay after the battle pauses.
    throttle.previous_ultimate_serial = cues.serial;
}

fn play_hit_feedback(
    mut commands: Commands,
    time: Res<Time<Real>>,
    audio: Res<GameAudio>,
    feedback: Res<HitFeedback>,
    gate: Option<Res<GateImpact>>,
    autoplay: Option<Res<Autoplay>>,
    mut throttle: ResMut<HitAudioThrottle>,
    output: Option<Res<bevy::audio::AudioOutputStatus>>,
) {
    if autoplay.is_some() || output.as_ref().is_some_and(|status| !status.is_available()) {
        return;
    }
    throttle.tick(time.delta_secs());
    let events = feedback.events();
    let kills = events.iter().filter(|event| event.killed).count();
    let heavy_kill = events
        .iter()
        .any(|event| event.killed && event.weight != HitWeight::Normal);
    let crits = events.iter().filter(|event| event.critical).count();
    let heavy_hit = events
        .iter()
        .any(|event| !event.killed && event.weight != HitWeight::Normal);

    if heavy_kill && ready(&mut throttle.kill_heavy, 0.22) {
        let pitch = throttle.pitch(0.05);
        play_pitched(&mut commands, &audio.kill_heavy, 0.85, pitch);
    }
    if kills > 0 && ready(&mut throttle.kill, 0.055) {
        let pitch = throttle.pitch(0.12);
        play_pitched(&mut commands, &audio.kill, crowd_volume(0.5, kills), pitch);
    }
    if crits > 0 && ready(&mut throttle.crit, 0.075) {
        let pitch = throttle.pitch(0.06);
        play_pitched(&mut commands, &audio.crit, crowd_volume(0.55, crits), pitch);
    }
    if heavy_hit && ready(&mut throttle.heavy_hit, 0.16) {
        let pitch = throttle.pitch(0.08);
        play_pitched(&mut commands, &audio.impact, 0.42, pitch);
    }
    if let Some(event) = events.iter().find(|event| !event.critical)
        && ready(&mut throttle.hit, 0.045)
    {
        let source = if event.element == DamageElement::Physical {
            &audio.hit_physical
        } else {
            &audio.hit_magic
        };
        let pitch = throttle.pitch(0.14);
        play_pitched(
            &mut commands,
            source,
            crowd_volume(0.34, events.len()),
            pitch,
        );
    }
    if let Some(strength) = gate.as_deref().and_then(GateImpact::struck) {
        let pitch = throttle.pitch(0.1);
        play_pitched(
            &mut commands,
            &audio.gate_hit,
            0.35 + 0.45 * strength,
            pitch,
        );
    }
    if feedback.hero_hurt() && ready(&mut throttle.hero_hurt, 0.3) {
        let pitch = throttle.pitch(0.06);
        play_pitched(&mut commands, &audio.hero_hurt, 0.7, pitch);
    }
}

fn play_once(commands: &mut Commands, source: &Handle<AudioSource>, volume: f32) {
    play_pitched(commands, source, volume, 1.0);
}

fn play_pitched(commands: &mut Commands, source: &Handle<AudioSource>, volume: f32, speed: f32) {
    commands.spawn((
        AudioPlayer::new(source.clone()),
        AudioVoice::new(AudioChannel::Effects, volume),
        PlaybackSettings::DESPAWN
            .with_volume(Volume::Linear(volume))
            .with_speed(speed),
    ));
}

pub(crate) fn reset_run(world: &mut World) {
    world.insert_resource(AudioThrottle::default());
    world.insert_resource(HitAudioThrottle::default());
}

#[cfg(test)]
mod tests {
    use bevy::audio::Decodable;

    use super::*;

    fn feedback_app() -> App {
        let mut sources = Assets::<AudioSource>::default();
        let mut source = || {
            sources.add(AudioSource {
                bytes: Vec::<u8>::new().into(),
            })
        };
        let audio = GameAudio {
            ambient: source(),
            shot: source(),
            impact: source(),
            draft: source(),
            ultimate: source(),
            boss_spawn: source(),
            hit_physical: source(),
            hit_magic: source(),
            crit: source(),
            kill: source(),
            kill_heavy: source(),
            hero_hurt: source(),
            gate_hit: source(),
        };
        let mut app = App::new();
        app.insert_resource(sources)
            .insert_resource(crate::game::AudioPreferences {
                music: 100,
                effects: 100,
                muted: false,
            })
            .insert_resource(audio)
            .init_resource::<Time>()
            .init_resource::<Time<Real>>()
            .init_resource::<UltimateCues>()
            .init_resource::<AudioThrottle>()
            .add_systems(Update, play_feedback);
        app
    }

    fn advance_feedback(app: &mut App, speed: f32) {
        let real = std::time::Duration::from_secs_f64(1.0 / 60.0);
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(real);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(real.mul_f32(speed));
        app.update();
    }

    fn voice_count(app: &mut App, source: Handle<AudioSource>) -> usize {
        app.world_mut()
            .query::<&AudioPlayer>()
            .iter(app.world())
            .filter(|player| player.0 == source)
            .count()
    }

    #[test]
    fn actual_shot_voices_are_bounded_by_real_time_at_every_battle_speed() {
        for speed in [1.0, 2.0, 4.0, 8.0, 16.0, 32.0] {
            let mut app = feedback_app();
            for _ in 0..600 {
                let projectile = app
                    .world_mut()
                    .spawn(Projectile {
                        velocity: Vec3::X,
                        damage: 1.0,
                        radius: 0.1,
                        remaining_hits: 1,
                        lifetime: 1.0,
                    })
                    .id();
                advance_feedback(&mut app, speed);
                app.world_mut().despawn(projectile);
            }
            let shot = app.world().resource::<GameAudio>().shot.clone();
            assert_eq!(voice_count(&mut app, shot), 100, "speed={speed}");
        }
    }

    #[test]
    fn disconnected_output_does_not_queue_shots_or_old_ultimate_cues() {
        let mut app = feedback_app();
        app.insert_resource(bevy::audio::AudioOutputStatus::default());
        app.world_mut().resource_mut::<UltimateCues>().serial = 5;
        for _ in 0..100 {
            app.world_mut().spawn(Projectile {
                velocity: Vec3::X,
                damage: 1.0,
                radius: 0.1,
                remaining_hits: 1,
                lifetime: 1.0,
            });
            advance_feedback(&mut app, 32.0);
        }
        assert_eq!(
            app.world_mut()
                .query::<&AudioVoice>()
                .iter(app.world())
                .count(),
            0
        );
        assert_eq!(
            app.world()
                .resource::<AudioThrottle>()
                .previous_ultimate_serial,
            5
        );
        // After output becomes usable, consumed cues must not replay.
        app.world_mut()
            .remove_resource::<bevy::audio::AudioOutputStatus>();
        advance_feedback(&mut app, 1.0);
        assert_eq!(
            app.world_mut()
                .query::<&AudioVoice>()
                .iter(app.world())
                .count(),
            0
        );
    }

    #[test]
    fn rapid_ultimate_cues_do_not_stack_or_replay_after_activity_stops() {
        let mut app = feedback_app();
        for _ in 0..12 {
            app.world_mut().resource_mut::<UltimateCues>().serial += 1;
            advance_feedback(&mut app, 32.0);
        }
        let ultimate = app.world().resource::<GameAudio>().ultimate.clone();
        assert_eq!(voice_count(&mut app, ultimate.clone()), 1);
        // Game time freezes; elapsed real time must not release suppressed old cues.
        for _ in 0..60 {
            advance_feedback(&mut app, 0.0);
        }
        assert_eq!(voice_count(&mut app, ultimate.clone()), 1);
        app.world_mut().resource_mut::<UltimateCues>().serial += 1;
        advance_feedback(&mut app, 1.0);
        assert_eq!(voice_count(&mut app, ultimate), 2);
    }

    #[test]
    fn accelerated_boss_arrivals_remain_audible_without_stacking_every_frame() {
        let mut app = feedback_app();
        for _ in 0..60 {
            let boss = app.world_mut().spawn(BossEnemy { index: 0 }).id();
            advance_feedback(&mut app, 32.0);
            app.world_mut().despawn(boss);
        }
        let boss = app.world().resource::<GameAudio>().boss_spawn.clone();
        let count = voice_count(&mut app, boss);
        assert!((2..=3).contains(&count), "actual boss voices={count}");
    }

    fn assert_audio_decodes(bytes: &[u8]) {
        let source = AudioSource {
            bytes: bytes.to_vec().into(),
        };
        assert!(source.decoder().next().is_some());
    }

    #[test]
    fn every_shipped_audio_asset_has_an_enabled_decoder() {
        assert_audio_decodes(include_bytes!("../../assets/audio/arena_loop.ogg"));
        assert_audio_decodes(include_bytes!("../../assets/audio/shot.wav"));
        assert_audio_decodes(include_bytes!("../../assets/audio/impact.wav"));
        assert_audio_decodes(include_bytes!("../../assets/audio/draft.wav"));
        assert_audio_decodes(include_bytes!("../../assets/audio/ultimate.wav"));
        assert_audio_decodes(include_bytes!("../../assets/audio/boss_spawn.wav"));
        assert_audio_decodes(include_bytes!("../../assets/audio/hit_physical.wav"));
        assert_audio_decodes(include_bytes!("../../assets/audio/hit_magic.wav"));
        assert_audio_decodes(include_bytes!("../../assets/audio/crit.wav"));
        assert_audio_decodes(include_bytes!("../../assets/audio/kill.wav"));
        assert_audio_decodes(include_bytes!("../../assets/audio/kill_heavy.wav"));
        assert_audio_decodes(include_bytes!("../../assets/audio/hero_hurt.wav"));
        assert_audio_decodes(include_bytes!("../../assets/audio/gate_hit.wav"));
    }

    #[test]
    fn pitch_variation_stays_within_spread_and_changes_between_hits() {
        let mut throttle = HitAudioThrottle::default();
        let pitches: Vec<f32> = (0..32).map(|_| throttle.pitch(0.1)).collect();
        assert!(pitches.iter().all(|pitch| (0.9..=1.1).contains(pitch)));
        assert!(pitches.windows(2).any(|pair| pair[0] != pair[1]));
    }

    #[test]
    fn crowd_volume_grows_but_is_capped() {
        assert_eq!(crowd_volume(0.4, 1), 0.4);
        assert!(crowd_volume(0.4, 8) > 0.4);
        assert!(crowd_volume(0.4, 10_000) <= 0.4 * 1.45 + f32::EPSILON);
    }
}
