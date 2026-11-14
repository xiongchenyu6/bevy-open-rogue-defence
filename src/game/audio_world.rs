//! One audible cue per real threat/building transition, never per warning frame.
use super::*;
use crate::game::{GameSession, RaidWarning};

#[derive(Resource, Default)]
pub(super) struct WorldAudioState {
    gate_up: Option<bool>,
    raid: bool,
}
impl WorldAudioState {
    fn observe(&mut self, gate_up: bool, raid: bool) -> (Option<bool>, bool) {
        let gate = self
            .gate_up
            .replace(gate_up)
            .filter(|old| *old != gate_up)
            .map(|_| gate_up);
        let warning = raid && !self.raid;
        self.raid = raid;
        (gate, warning)
    }
}
pub(super) fn play_world_feedback(
    mut commands: Commands,
    audio: Res<GameAudio>,
    session: Res<GameSession>,
    raid: Res<RaidWarning>,
    mut state: ResMut<WorldAudioState>,
    autoplay: Option<Res<Autoplay>>,
    output: Option<Res<bevy::audio::AudioOutputStatus>>,
) {
    if session.paused {
        return;
    }
    let (gate, warning) = state.observe(
        session.core_hp > 0.0 && session.gate_revive_remaining <= 0.0,
        raid.remaining_secs > 0.0,
    );
    if session.finished || autoplay.is_some() || output.as_ref().is_some_and(|o| !o.is_available())
    {
        return;
    }
    if let Some(rebuilt) = gate {
        if rebuilt {
            play_pitched(&mut commands, &audio.gate_hit, 0.65, 1.15);
        } else {
            play_pitched(&mut commands, &audio.kill_heavy, 0.75, 0.65);
        }
    }
    if warning {
        play_pitched(&mut commands, &audio.raid_steps, 0.42, 1.0);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn world_cues_only_fire_on_new_transitions() {
        let mut state = WorldAudioState::default();
        assert_eq!(state.observe(true, false), (None, false));
        assert_eq!(state.observe(false, true), (Some(false), true));
        for _ in 0..300 {
            assert_eq!(state.observe(false, true), (None, false));
        }
        assert_eq!(state.observe(true, false), (Some(true), false));
        assert_eq!(state.observe(true, true), (None, true));
    }
}
