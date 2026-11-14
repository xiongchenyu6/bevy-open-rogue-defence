//! Real basic attacks defeat a small group; receipts follow actual settlement.
use bevy::prelude::*;
use rogue_commander::game::*;
#[derive(Resource, Default)]
struct Probe {
    frame: u32,
    gold: u32,
    xp: u32,
    saw_gold: bool,
    saw_xp: bool,
}
pub fn configure(world: &mut World) {
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SelectedHero>().locked = true;
    world.resource_mut::<SelectedHero>().kind = HeroKind::EmberRanger;
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
    world
        .resource_mut::<super::CaptureControls>()
        .suppress_drafts = true;
}
pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    world.remove_resource::<Autoplay>();
    world.resource_mut::<ActiveSkill>().remaining = 10000.0;
    world.resource_mut::<PlayerBuild>().skills.fill(0);
    world.resource_mut::<PlayerBuild>().attack_damage = 100.0;
    world.resource_mut::<PlayerBuild>().attack_range = 12.0;
    world.resource_mut::<AttackClock>().0.unpause();
    for mut transform in world
        .query_filtered::<&mut Transform, With<Hero>>()
        .iter_mut(world)
    {
        transform.translation = Vec3::new(-3.0, 0.0, 0.0);
    }
    capture_reward_targets(world);
    let gold = world.resource::<GameSession>().gold;
    let xp = world.resource::<GameSession>().xp;
    app.insert_resource(Probe {
        gold,
        xp,
        ..default()
    })
    .add_systems(Last, observe);
}
fn observe(world: &mut World) {
    let gold = world
        .query::<&Name>()
        .iter(world)
        .any(|n| n.as_str() == "Collected gold");
    let xp = world
        .query::<&Name>()
        .iter(world)
        .any(|n| n.as_str() == "Collected experience");
    let mut probe = world.resource_mut::<Probe>();
    probe.frame += 1;
    probe.saw_gold |= gold;
    probe.saw_xp |= xp;
    if probe.frame == 250 {
        let before = (probe.gold, probe.xp, probe.saw_gold, probe.saw_xp);
        let session = world.resource::<GameSession>();
        assert!(
            session.total_kills >= 6
                && session.gold >= before.0 + 18
                && session.xp >= before.1 + 12
        );
        assert!(
            before.2 && before.3,
            "actual rewarded kills produce both collection visuals"
        );
        println!(
            "[capture] PASS reward-feedback: 6 real kills, settled gold/XP and collected tokens"
        );
    }
}
