use bevy::prelude::*;
use rogue_commander::game::{
    Autoplay, DamageMeter, GameSession, GoldChallengeState, PlayerBuild, SelectedHero, TrialAction,
    TrialButton, TrialState,
};

#[derive(Resource)]
struct TrialCapture {
    frame: u32,
    scenario: String,
}

pub fn configure(world: &mut World, scenario: &str) {
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SelectedHero>().locked = true;
    let mut session = world.resource_mut::<GameSession>();
    session.elapsed = 17.;
    session.auto_bond_draws = false;
    world.insert_resource(TrialCapture {
        frame: 0,
        scenario: scenario.into(),
    });
}
pub fn install(app: &mut bevy::app::SubApp) {
    app.add_systems(PreUpdate, exercise.after(bevy::ui::UiSystems::Focus));
}
fn exercise(
    mut capture: ResMut<TrialCapture>,
    mut session: ResMut<GameSession>,
    mut build: ResMut<PlayerBuild>,
    meter: Res<DamageMeter>,
    mut tower: ResMut<TrialState>,
    mut gold: ResMut<GoldChallengeState>,
    mut buttons: Query<(&mut Interaction, Option<&TrialButton>, Option<&Name>)>,
) {
    capture.frame += 1;
    build.skills.fill(0);
    build.attack_damage = 0.;
    if capture.frame == 2 {
        assert_eq!(meter.recent_dps(), 0.);
    }
    for (mut interaction, button, name) in &mut buttons {
        if capture.frame == 3 && name.is_some_and(|name| name.as_str() == "Trial drawer") {
            *interaction = Interaction::Pressed;
        }
        if capture.frame == 5
            && button.is_some_and(|button| {
                matches!(
                    button.0,
                    TrialAction::ToggleTowerAuto | TrialAction::ToggleGoldAuto
                )
            })
        {
            *interaction = Interaction::Pressed;
        }
        if capture.frame == 6 {
            *interaction = Interaction::None;
        }
        if capture.frame == 75
            && capture.scenario == "trials-clear"
            && button.is_some_and(|button| button.0 == TrialAction::CancelAll)
        {
            *interaction = Interaction::Pressed;
        }
    }
    if capture.frame == 8 {
        assert!(tower.auto && gold.auto);
        assert!(!gold.active);
    }
    if capture.frame == 15 {
        session.elapsed = 18.;
    }
    if capture.frame == 25 {
        assert!(tower.active);
        assert!(!gold.active);
    }
    if capture.frame == 35 {
        session.elapsed = 60.;
    }
    if capture.frame == 50 {
        assert!(tower.active && gold.active);
        assert_eq!(gold.remaining_targets, 10);
        println!(
            "[trials capture] actual AUTO buttons started both challenges at unlock, with zero dealt damage"
        );
    }
    if capture.frame == 65 && capture.scenario == "trials-retry" {
        tower.remaining = 0.;
        gold.remaining = 0.;
    }
    if capture.frame == 70 && capture.scenario == "trials-retry" {
        assert!(!tower.active && !gold.active);
        tower.cooldown = 0.;
        gold.cooldown = 0.;
    }
    if capture.frame == 85 {
        if capture.scenario == "trials-clear" {
            assert!(!tower.active && !gold.active && !tower.auto && !gold.auto);
            println!("[trials capture] clear disabled automatic retries");
        } else {
            assert!(tower.active && gold.active);
            println!(
                "[trials capture] automatic challenges active; scenario={}",
                capture.scenario
            );
        }
    }
}
