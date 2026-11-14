//! Render real gate damage/rebuild and the scheduled third-minute raid warning.
use bevy::prelude::*;
use rogue_commander::game::{
    ActiveSkill, AttackClock, Autoplay, CampaignMenu, GameSession, HeroVitals, PlayerBuild,
    RaidWarning, SelectedHero, SpawnDirector,
};
#[derive(Resource, Default)]
struct WorldFeedbackProbe {
    frame: u32,
    saw_warning: bool,
    saw_debris: bool,
    saw_collapse: bool,
    saw_rebuild: bool,
    saw_rise: bool,
    camp_position: Option<Vec3>,
    camp_moved: bool,
}
pub fn configure(world: &mut World) {
    world.remove_resource::<Autoplay>();
    world.resource_mut::<CampaignMenu>().0 = false;
    world.resource_mut::<SelectedHero>().locked = true;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
    world
        .resource_mut::<super::CaptureControls>()
        .suppress_drafts = true;
}
pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    world.remove_resource::<Autoplay>();
    world.resource_mut::<PlayerBuild>().skills.fill(0);
    world.resource_mut::<AttackClock>().0.pause();
    world.resource_mut::<ActiveSkill>().remaining = 10000.0;
    world
        .resource_mut::<Time<Virtual>>()
        .set_relative_speed(1.0);
    world.resource_mut::<SpawnDirector>().enabled = true;
    world.resource_mut::<HeroVitals>().max_health = 100000.0;
    world.resource_mut::<HeroVitals>().health = 100000.0;
    let mut session = world.resource_mut::<GameSession>();
    session.elapsed = 135.0;
    session.wave = 3;
    session.core_hp = session.core_max_hp;
    app.init_resource::<WorldFeedbackProbe>()
        .add_systems(Last, exercise);
    println!("[capture] world-feedback: 360 frames, real minute-3 raid and 4s gate reconstruction");
}
fn exercise(world: &mut World) {
    world.resource_mut::<WorldFeedbackProbe>().frame += 1;
    let frame = world.resource::<WorldFeedbackProbe>().frame;
    let names: Vec<_> = world
        .query::<(&Name, &Transform)>()
        .iter(world)
        .map(|(name, transform)| (name.as_str().to_string(), transform.translation))
        .collect();
    let warning = world.resource::<RaidWarning>().remaining_secs > 0.0;
    {
        let mut probe = world.resource_mut::<WorldFeedbackProbe>();
        probe.saw_warning |= warning;
        if let Some((_, position)) = names.iter().find(|(name, _)| name == "Camp atmosphere 0/0") {
            if let Some(previous) = probe.camp_position {
                probe.camp_moved |= previous.distance(*position) > 0.02;
            } else {
                probe.camp_position = Some(*position);
            }
        }
        probe.saw_debris |= names
            .iter()
            .any(|(name, _)| name == "Raid warning falling cavern stone");
        probe.saw_collapse |= names.iter().any(|(name, _)| name == "Gate collapse dust");
        probe.saw_rebuild |= names
            .iter()
            .any(|(name, _)| name == "Gate reconstruction dust");
        probe.saw_rise |= names
            .iter()
            .any(|(name, position)| name == "Gate art" && position.y < -0.2);
    }
    let health_fraction = match frame {
        30 => Some(0.70),
        60 => Some(0.45),
        90 => Some(0.20),
        120 => Some(0.0),
        _ => None,
    };
    if let Some(fraction) = health_fraction {
        let mut session = world.resource_mut::<GameSession>();
        session.core_hp = session.core_max_hp * fraction;
        println!("[capture] controlled gate hit at frame {frame}, hp fraction {fraction}");
    }
    if frame == 320 {
        let probe = world.resource::<WorldFeedbackProbe>();
        assert!(
            probe.saw_warning && probe.saw_debris,
            "real raid must disturb its entrance"
        );
        assert!(
            probe.saw_collapse && probe.saw_rebuild && probe.saw_rise,
            "collapse and reconstruction must both be visible"
        );
        assert!(world.resource::<GameSession>().core_hp > 0.0);
        assert!(probe.camp_moved, "real camp atmosphere must animate");
        assert_eq!(
            names
                .iter()
                .filter(|(name, _)| name.contains("road verge") || name == "Worn dusty road center")
                .count(),
            3,
            "road transition materials must exist"
        );
        println!(
            "[capture] PASS world-feedback: real raid warning, falling stone, collapse dust, rebuilt gate rise"
        );
    }
}
