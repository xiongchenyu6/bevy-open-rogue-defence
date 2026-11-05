//! Observe a real output-device outage. The external driver owns the audio server.
//! No output status, sinks, voices or failure callbacks are injected here.
use std::{io::Write, time::Instant};

use bevy::{
    audio::{AudioOutputStatus, AudioSinkPlayback},
    prelude::*,
    time::TimeUpdateStrategy,
};
use rogue_commander::game::{
    ActiveSkill, AttackClock, AudioChannel, AudioPreferences, AudioVoice, Autoplay, CampaignMenu,
    GameSession, HeroKind, HeroVitals, PlayerBuild, SelectedHero, SpawnDirector,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Paused,
    Eightfold,
}
impl Mode {
    fn from_env() -> Self {
        match std::env::var("ROGUE_CAPTURE_AUDIO_MODE")
            .as_deref()
            .unwrap_or("paused")
        {
            "paused" => Self::Paused,
            "8x" => Self::Eightfold,
            _ => panic!("ROGUE_CAPTURE_AUDIO_MODE must be paused or 8x"),
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Paused => "paused",
            Self::Eightfold => "8x",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Snapshot {
    available: bool,
    entity: Option<Entity>,
    sink_present: bool,
    sink_added: bool,
    sink_tick: Option<u32>,
    sink_generation: u32,
    gain: Option<f32>,
    settings_gain: Option<f32>,
    effects: usize,
    phase: &'static str,
}

#[derive(Resource)]
struct Probe {
    started: Instant,
    mode: Mode,
    music: Option<Entity>,
    generation: u32,
    last_added_tick: Option<u32>,
    initial_generation: Option<u32>,
    initial_real: f64,
    initial_virtual: f64,
    saw_unavailable: bool,
    saw_sink_absent: bool,
    unavailable_frames: u32,
    saw_clean_unavailable: bool,
    passed: bool,
    previous: Option<Snapshot>,
}

pub fn configure(world: &mut World) {
    // Preference persistence must never touch the user's real configuration.
    assert!(
        std::env::var_os("ROGUE_AUDIO_SAVE").is_some(),
        "audio-backend requires isolated ROGUE_AUDIO_SAVE"
    );
    world.remove_resource::<Autoplay>();
    world.resource_mut::<CampaignMenu>().0 = false;
    *world.resource_mut::<SelectedHero>() = SelectedHero {
        kind: HeroKind::StormSavant,
        locked: true,
    };
    *world.resource_mut::<HeroVitals>() = HeroVitals::for_hero(HeroKind::StormSavant, 1);
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
    world.resource_mut::<GameSession>().paused = Mode::from_env() == Mode::Paused;
    world.insert_resource(AudioPreferences {
        music: 40,
        effects: 80,
        muted: false,
    });
    // Automatic is Bevy 0.19's real renderer/system-instant strategy. Its enum
    // does not have an AutomaticInstant variant.
    world.insert_resource(TimeUpdateStrategy::Automatic);
}

pub fn install(app: &mut bevy::app::SubApp) {
    let mode = Mode::from_env();
    let world = app.world_mut();
    world.resource_mut::<PlayerBuild>().skills.fill(0);
    world.resource_mut::<ActiveSkill>().remaining = 10_000.0;
    world.resource_mut::<AttackClock>().0.pause();
    world.resource_mut::<GameSession>().paused = mode == Mode::Paused;
    {
        let mut time = world.resource_mut::<Time<Virtual>>();
        match mode {
            Mode::Paused => time.pause(),
            Mode::Eightfold => {
                time.unpause();
                time.set_relative_speed(8.0);
            }
        }
    }
    let initial_real = world.resource::<Time<Real>>().elapsed_secs_f64();
    let initial_virtual = world.resource::<Time<Virtual>>().elapsed_secs_f64();
    world.insert_resource(Probe {
        started: Instant::now(),
        mode,
        music: None,
        generation: 0,
        last_added_tick: None,
        initial_generation: None,
        initial_real,
        initial_virtual,
        saw_unavailable: false,
        saw_sink_absent: false,
        unavailable_frames: 0,
        saw_clean_unavailable: false,
        passed: false,
        previous: None,
    });
    app.add_systems(Last, observe);
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn observe(
    mut probe: ResMut<Probe>,
    output: Res<AudioOutputStatus>,
    preferences: Res<AudioPreferences>,
    session: Res<GameSession>,
    real: Res<Time<Real>>,
    virtual_time: Res<Time<Virtual>>,
    voices: Query<(
        Entity,
        &AudioVoice,
        &PlaybackSettings,
        Option<Ref<AudioSink>>,
    )>,
) {
    // Capture-only wall-clock pacing, retained after PASS for predictable native
    // completion. Device maintenance still uses the production Time<Real> path.
    std::thread::sleep(std::time::Duration::from_millis(20));
    let wall = probe.started.elapsed().as_secs_f64();
    assert!(
        probe.passed || wall < 90.0,
        "audio-backend cycle did not finish within 90 real seconds"
    );
    assert_eq!(
        (preferences.music, preferences.effects, preferences.muted),
        (40, 80, false)
    );
    assert_eq!(session.paused, probe.mode == Mode::Paused);
    match probe.mode {
        Mode::Paused => {
            assert!(virtual_time.is_paused());
            assert_eq!(virtual_time.elapsed_secs_f64(), probe.initial_virtual);
        }
        Mode::Eightfold => {
            assert!(!virtual_time.is_paused());
            assert_eq!(virtual_time.relative_speed(), 8.0);
        }
    }
    let available = output.is_available();
    let mut effects = 0;
    let mut music = None;
    for (entity, voice, settings, sink) in &voices {
        if voice.channel == AudioChannel::Effects {
            effects += 1;
            continue;
        }
        assert!(
            music.is_none(),
            "ambient loop must remain a single music entity"
        );
        let settings_gain = settings.volume.to_linear();
        assert!((settings_gain - 0.144).abs() < 0.000_01);
        let sink = sink.map(|sink| {
            (
                sink.volume().to_linear(),
                sink.is_added(),
                sink.last_changed().get(),
            )
        });
        music = Some((entity, settings_gain, sink));
    }
    let (entity, settings_gain, sink) = music.map_or((None, None, None), |(entity, gain, sink)| {
        (Some(entity), Some(gain), sink)
    });
    if let Some(entity) = entity {
        if let Some(original) = probe.music {
            assert_eq!(
                entity, original,
                "device recovery replaced the retained music entity"
            );
        } else {
            probe.music = Some(entity);
        }
    }
    if probe.music.is_some() {
        assert!(entity.is_some(), "ambient music entity disappeared");
    }
    let sink_present = sink.is_some();
    let sink_added = sink.is_some_and(|(_, added, _)| added);
    let sink_tick = sink.map(|(_, _, tick)| tick);
    let gain = sink.map(|(gain, _, _)| gain);
    if let Some((gain, added, tick)) = sink {
        assert!((gain - 0.144).abs() < 0.000_01);
        if probe.last_added_tick.is_none() || (added && probe.last_added_tick != Some(tick)) {
            probe.generation += 1;
            probe.last_added_tick = Some(tick);
        }
    }
    if available && sink_present && probe.initial_generation.is_none() {
        probe.initial_generation = Some(probe.generation);
        println!("AUDIO_BACKEND_READY mode={} gain=0.144", probe.mode.label());
    }
    if probe.initial_generation.is_some() && !available {
        probe.saw_unavailable = true;
        probe.saw_sink_absent |= !sink_present;
        probe.unavailable_frames += 1;
        // The backend callback may fire after this frame's First cleanup. A
        // second processed unavailable frame must retain no transient voices.
        if probe.unavailable_frames >= 2 && !sink_present {
            assert_eq!(
                effects, 0,
                "stale effects survived unavailable-output cleanup"
            );
            probe.saw_clean_unavailable = true;
        }
    }
    if !probe.passed
        && available
        && sink_present
        && probe.saw_unavailable
        && probe.saw_sink_absent
        && probe.saw_clean_unavailable
        && probe
            .initial_generation
            .is_some_and(|initial| probe.generation > initial)
    {
        assert!(
            real.elapsed_secs_f64() - probe.initial_real >= 5.0,
            "recovery must follow real elapsed time rather than a simulated step"
        );
        probe.passed = true;
        println!(
            "AUDIO_BACKEND_PASS mode={} retained_music=true new_sink=true gain=0.144 real_seconds={:.3} wall_seconds={wall:.3}",
            probe.mode.label(),
            real.elapsed_secs_f64() - probe.initial_real
        );
    }
    let phase = if probe.passed {
        "restored"
    } else if probe.saw_unavailable && !available {
        "offline"
    } else if probe.initial_generation.is_some() {
        "initial"
    } else {
        "awaiting"
    };
    let snapshot = Snapshot {
        available,
        entity,
        sink_present,
        sink_added,
        sink_tick,
        sink_generation: probe.generation,
        gain,
        settings_gain,
        effects,
        phase,
    };
    if probe.previous != Some(snapshot) {
        let entity = entity.map_or_else(|| "null".to_owned(), |entity| format!("\"{entity:?}\""));
        let tick = sink_tick.map_or_else(|| "null".to_owned(), |tick| tick.to_string());
        let gain = gain.map_or_else(|| "null".to_owned(), |gain| format!("{gain:.6}"));
        let settings_gain =
            settings_gain.map_or_else(|| "null".to_owned(), |gain| format!("{gain:.6}"));
        println!(
            "AUDIO_BACKEND_STATE {{\"mode\":\"{}\",\"phase\":\"{phase}\",\"available\":{available},\"music_entity\":{entity},\"sink_present\":{sink_present},\"sink_added\":{sink_added},\"sink_added_tick\":{tick},\"sink_generation\":{},\"gain\":{gain},\"settings_gain\":{settings_gain},\"effects\":{effects},\"wall_seconds\":{wall:.3},\"real_seconds\":{:.3},\"virtual_seconds\":{:.3}}}",
            probe.mode.label(),
            probe.generation,
            real.elapsed_secs_f64() - probe.initial_real,
            virtual_time.elapsed_secs_f64() - probe.initial_virtual
        );
        probe.previous = Some(snapshot);
    }
    std::io::stdout()
        .flush()
        .expect("flush backend capture telemetry");
}

pub fn assert_complete(world: &World) {
    assert!(
        world.resource::<Probe>().passed,
        "audio-backend native frames ended before a real outage/recovery cycle"
    );
}
