use bevy::{audio::Volume, prelude::*};

use super::{ActiveSkill, Autoplay, BossEnemy, DraftRoot, ImpactFx, Projectile};

pub struct GameAudioPlugin;

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameAudio>()
            .init_resource::<AudioThrottle>()
            .add_systems(Startup, start_ambient)
            .add_systems(Update, play_feedback);
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
        }
    }
}

#[derive(Resource, Default)]
struct AudioThrottle {
    shot: f32,
    impact: f32,
    previous_ultimate_remaining: f32,
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
    active: Res<ActiveSkill>,
    autoplay: Option<Res<Autoplay>>,
    bosses: Query<(), Added<BossEnemy>>,
    projectiles: Query<(), Added<Projectile>>,
    impacts: Query<(), Added<ImpactFx>>,
    drafts: Query<(), Added<DraftRoot>>,
    mut throttle: ResMut<AudioThrottle>,
) {
    if autoplay.is_some() {
        return;
    }
    throttle.shot = (throttle.shot - time.delta_secs()).max(0.0);
    throttle.impact = (throttle.impact - time.delta_secs()).max(0.0);

    if !bosses.is_empty() {
        play_once(&mut commands, &audio.boss_spawn, 0.72);
    }
    if !drafts.is_empty() {
        play_once(&mut commands, &audio.draft, 0.50);
    }
    if !projectiles.is_empty() && throttle.shot <= 0.0 {
        play_once(&mut commands, &audio.shot, 0.30);
        throttle.shot = 0.085;
    }
    if !impacts.is_empty() && throttle.impact <= 0.0 {
        play_once(&mut commands, &audio.impact, 0.46);
        throttle.impact = 0.11;
    }
    if active.remaining > throttle.previous_ultimate_remaining + 0.5
        && active.remaining > active.cooldown * 0.75
    {
        play_once(&mut commands, &audio.ultimate, 0.58);
    }
    throttle.previous_ultimate_remaining = active.remaining;
}

fn play_once(commands: &mut Commands, source: &Handle<AudioSource>, volume: f32) {
    commands.spawn((
        AudioPlayer::new(source.clone()),
        PlaybackSettings::DESPAWN.with_volume(Volume::Linear(volume)),
    ));
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
    }
}
