//! Native production impact rhythm fixture. Debug builds or --features dev-tools.
use bevy::prelude::*;
use rogue_commander::game::{GameSession, SelectedHero, SpawnDirector};

pub fn configure(world: &mut World) {
    world.resource_mut::<SelectedHero>().locked = true;
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
    world
        .resource_mut::<super::CaptureControls>()
        .suppress_drafts = true;
}

pub fn install(app: &mut bevy::app::SubApp, scenario: &str) {
    rogue_commander::game::hit_feedback_capture::install(
        app,
        if scenario == "hit-feedback-8x" {
            8.0
        } else {
            1.0
        },
    );
}
