//! Production-camera proof of the demon entrance, quiet and during real waves.
use bevy::prelude::*;
use rogue_commander::game::{
    ActiveSkill, AttackClock, Autoplay, Enemy, GameSession, PlayerBuild, SelectedHero,
    SpawnDirector,
};
#[derive(Resource, Default)]
struct CaveCapture {
    frame: u32,
    saw_enemy: bool,
    saw_pulse: bool,
}
pub fn configure(world: &mut World) {
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SelectedHero>().locked = true;
    world
        .resource_mut::<super::CaptureControls>()
        .suppress_drafts = true;
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
}
pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<PlayerBuild>().skills.fill(0);
    world.resource_mut::<ActiveSkill>().remaining = 10_000.;
    world.resource_mut::<AttackClock>().0.pause();
    app.init_resource::<CaveCapture>()
        .add_systems(PreUpdate, exercise);
}
fn exercise(
    mut state: ResMut<CaveCapture>,
    mut director: ResMut<SpawnDirector>,
    enemies: Query<(&Enemy, &Transform)>,
    names: Query<&Name>,
) {
    state.frame += 1;
    let frame = state.frame;
    state.saw_enemy |= enemies
        .iter()
        .any(|(enemy, transform)| enemy.hp > 0. && transform.translation.x > 10.);
    state.saw_pulse |= names
        .iter()
        .any(|name| name.as_str() == "Demon den emergence pulse");
    if frame == 30 {
        assert!(
            enemies.is_empty(),
            "quiet entrance scene has no hand-spawned or real enemies"
        );
        let count = |label: &str| names.iter().filter(|name| name.as_str() == label).count();
        assert_eq!(
            count("Demon den mouth"),
            1,
            "one giant dark mouth, rather than scattered portal circles"
        );
        assert!(count("Demon den rock arch") >= 10);
        assert!(count("Demon den fang") >= 6);
        assert!(count("Demon den breathing mist") >= 6);
        println!(
            "[capture] PASS demon cave quiet: one giant mouth, stone arch, solid fangs and breathing mist, production camera"
        );
    }
    if frame == 60 {
        director.enabled = true;
    }
    if frame == 295 {
        assert!(
            state.saw_enemy,
            "actual SpawnDirector wave must emerge at the den"
        );
        assert!(
            state.saw_pulse,
            "actual enemy addition must trigger emergence pulse"
        );
        println!(
            "[capture] PASS demon-cave: quiet cave, real wave enemies and actual emergence pulse; production camera unchanged"
        );
    }
}
