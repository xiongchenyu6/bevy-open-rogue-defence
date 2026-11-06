//! Windup audio follows the real pending attack, before its contact resolver.
use super::AudioThrottle;
use crate::game::basic_attacks::{BasicWeapon, PendingSwing, weapon};
use crate::game::hero_fusion::FusionAttack;
use crate::game::{Autoplay, GameSession, Hero, HeroVitals};
use bevy::prelude::*;

// Exclusive only to keep private audio resources private. No event queue, and no
// Added query: a short windup can be created and resolved in the same update.
pub(crate) fn play_melee_swings(world: &mut World) {
    let Some(time) = world.get_resource::<Time>() else {
        return;
    };
    let delta = time.delta_secs();
    let speed = world
        .get_resource::<Time<Virtual>>()
        .map_or(1.0, |t| t.relative_speed());
    let real_delta = world
        .get_resource::<Time<Real>>()
        .map_or(0.0, Time::delta_secs);
    let Some(session) = world.get_resource::<GameSession>() else {
        return;
    };
    let paused = session.paused;
    let finished = session.finished;
    let Some(vitals) = world.get_resource::<HeroVitals>() else {
        return;
    };
    let can_act = vitals.can_act();
    let current_kind = vitals.kind;
    let available = world
        .get_resource::<bevy::audio::AudioOutputStatus>()
        .is_none_or(|s| s.is_available());
    let suppressed = paused || finished || world.contains_resource::<Autoplay>() || !available;
    let mut eligible = [false; 3];
    let mut consumed = Vec::new();
    let mut swings = world.query::<(Entity, &PendingSwing, Option<&FusionAttack>)>();
    for (entity, swing, fusion) in swings.iter(world) {
        if swing.whoosh_consumed {
            continue;
        }
        let valid = !finished
            && can_act
            && (swing.kind == current_kind || fusion.is_some())
            && world.get::<Hero>(swing.hero).is_some();
        if !valid {
            consumed.push(entity);
            continue;
        }
        let (slot, lead) = match weapon(swing.kind) {
            BasicWeapon::Sword => (0, 0.050),
            BasicWeapon::Axe => (1, 0.060),
            BasicWeapon::Daggers => (2, 0.020),
            _ => {
                consumed.push(entity);
                continue;
            }
        };
        // remaining is decremented immediately after this system. Anticipate that
        // update, aligning the recorded PCM RMS peak with contact at 1x. Device
        // buffering/large frames/32x cannot guarantee a physical audible lead.
        if swing.remaining - if paused { 0.0 } else { delta } > lead * speed {
            continue;
        }
        consumed.push(entity);
        if !suppressed {
            eligible[slot] = true;
        }
    }
    for entity in consumed {
        world
            .get_mut::<PendingSwing>(entity)
            .unwrap()
            .whoosh_consumed = true;
    }
    let Some(mut throttle) = world.get_resource_mut::<AudioThrottle>() else {
        return;
    };
    throttle.melee = (throttle.melee - real_delta).max(0.0);
    if !eligible.iter().any(|v| *v) || throttle.melee > 0.0 {
        return;
    }
    throttle.melee = 0.085;
    for (slot, emit) in eligible.into_iter().enumerate() {
        throttle.melee_pending[slot] |= emit;
    }
}
