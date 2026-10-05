use bevy::{audio::Volume, prelude::*};

use super::bonds::DamageElement;
use super::gate_feedback::GateImpact;
use super::hit_feedback::{HitFeedback, HitFeedbackSet, HitWeight};
use super::ultimate_vfx::UltimateCues;
use super::{Autoplay, BossEnemy, DraftRoot, Projectile};

pub struct GameAudioPlugin;

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameAudio>()
            .init_resource::<AudioThrottle>()
            .init_resource::<HitAudioThrottle>()
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
    previous_ultimate_serial: u64,
    previous_crafting_revision: u32,
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
        PlaybackSettings::LOOP.with_volume(Volume::Linear(0.36)),
    ));
}

fn play_feedback(
    mut commands: Commands,
    time: Res<Time>,
    audio: Res<GameAudio>,
    cues: Res<UltimateCues>,
    crafting: Option<Res<super::crafting::CraftingState>>,
    autoplay: Option<Res<Autoplay>>,
    bosses: Query<(), Added<BossEnemy>>,
    projectiles: Query<(), Added<Projectile>>,
    drafts: Query<(), Added<DraftRoot>>,
    mut throttle: ResMut<AudioThrottle>,
) {
    if autoplay.is_some() {
        return;
    }
    throttle.shot = (throttle.shot - time.delta_secs()).max(0.0);

    if !bosses.is_empty() {
        play_once(&mut commands, &audio.boss_spawn, 0.72);
    }
    let crafting_revision = crafting.as_ref().map_or(0, |state| state.revision);
    if crafting_revision != throttle.previous_crafting_revision {
        play_once(&mut commands, &audio.draft, 0.65);
    }
    throttle.previous_crafting_revision = crafting_revision;
    if !drafts.is_empty() {
        play_once(&mut commands, &audio.draft, 0.50);
    }
    if !projectiles.is_empty() && throttle.shot <= 0.0 {
        play_once(&mut commands, &audio.shot, 0.22);
        throttle.shot = 0.085;
    }
    if cues.serial != throttle.previous_ultimate_serial {
        play_once(&mut commands, &audio.ultimate, 0.58);
    }
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
) {
    if autoplay.is_some() {
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
