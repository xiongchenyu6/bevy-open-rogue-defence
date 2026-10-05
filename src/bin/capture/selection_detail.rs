//! Real skill draw, pending hand preview and reversible detailed/simple UI toggle.
use bevy::{prelude::*, ui::UiSystems};
use rogue_commander::game::{
    ActiveSkill, AttackClock, Autoplay, DraftChoice, DraftState, GameSession, HeroKind,
    PlayerBuild, SelectedHero, SelectionDetail, SkillDrawButton, SkillId, SpawnDirector,
};
#[derive(Resource, Default)]
struct DetailCapture(u32);
pub fn configure(world: &mut World) {
    world.remove_resource::<Autoplay>();
    world
        .resource_mut::<super::CaptureControls>()
        .suppress_drafts = false;
    world.resource_mut::<SelectedHero>().kind = HeroKind::StoneWarden;
    world.resource_mut::<SelectedHero>().locked = true;
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<GameSession>().gold = 50_000;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
    world.resource_mut::<PlayerBuild>().skills.fill(0);
    world.resource_mut::<PlayerBuild>().skills[0] = 2;
    world.resource_mut::<PlayerBuild>().skills[3] = 1;
    world.resource_mut::<PlayerBuild>().skills[13] = 1;
    world.resource_mut::<SelectionDetail>().0 = true;
}
pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    world.resource_mut::<PlayerBuild>().skills.fill(0);
    world.resource_mut::<PlayerBuild>().skills[0] = 2;
    world.resource_mut::<PlayerBuild>().skills[3] = 1;
    world.resource_mut::<PlayerBuild>().skills[13] = 1;
    world.resource_mut::<SelectionDetail>().0 = true;
    world.resource_mut::<ActiveSkill>().remaining = 10_000.;
    world.resource_mut::<AttackClock>().0.pause();
    app.init_resource::<DetailCapture>()
        .add_systems(PreUpdate, exercise.after(UiSystems::Focus));
}
fn press(world: &mut World, target: &str) {
    let mut found = false;
    for (name, skill, mut interaction) in world
        .query::<(Option<&Name>, Option<&SkillDrawButton>, &mut Interaction)>()
        .iter_mut(world)
    {
        if (target == "draw" && skill.is_some()) || name.is_some_and(|name| name.as_str() == target)
        {
            *interaction = Interaction::Pressed;
            found = true;
        } else {
            *interaction = Interaction::None;
        }
    }
    assert!(found, "real UI control must exist: {target}");
}
fn exercise(world: &mut World) {
    world.resource_mut::<DetailCapture>().0 += 1;
    let frame = world.resource::<DetailCapture>().0;
    match frame {
        10 | 22 => press(world, "draw"),
        20 => {
            let root = world
                .resource::<DraftState>()
                .root
                .expect("real paid skill draft opened");
            world.despawn(root);
            let mut draft = world.resource_mut::<DraftState>();
            draft.root = None;
            draft.choices = vec![
                DraftChoice::Skill(SkillId(0)),
                DraftChoice::Skill(SkillId(3)),
                DraftChoice::Skill(SkillId(13)),
            ];
            world.resource_mut::<GameSession>().paused = false;
        }
        130 => {
            for (name, mut interaction) in
                world.query::<(&Name, &mut Interaction)>().iter_mut(world)
            {
                *interaction = if name.as_str() == "Build card 1" {
                    Interaction::Hovered
                } else {
                    Interaction::None
                };
            }
        }
        135 => {
            let text = world
                .query::<(&Name, &Text)>()
                .iter(world)
                .find(|(name, _)| name.as_str() == "Selection details")
                .unwrap()
                .1
                .0
                .clone();
            assert!(
                text.contains("范围")
                    || text.to_lowercase().contains("range")
                    || text.contains("宽")
                    || text.to_lowercase().contains("width"),
                "tidal actual area dimensions: {text}"
            );
            println!("[capture] PASS selection area details: {text}");
        }
        60 | 100 => press(world, "Selection detail toggle"),
        40 | 80 | 120 => {
            let detailed = world.resource::<SelectionDetail>().0;
            assert_eq!(detailed, frame != 80);
            let text = world
                .query::<(&Name, &Text)>()
                .iter(world)
                .find(|(name, _)| name.as_str() == "Selection details")
                .expect("actual selection detail panel exists")
                .1
                .0
                .clone();
            if detailed {
                assert!(
                    text.contains("冷却") || text.to_lowercase().contains("cooldown"),
                    "{text}"
                );
                assert!(
                    text.contains("伤害") || text.to_lowercase().contains("damage"),
                    "{text}"
                );
                assert!(
                    text.contains('→'),
                    "current and next rank comparison: {text}"
                );
            } else {
                assert!(
                    !text.contains("公式")
                        && !text.to_lowercase().contains("formula")
                        && !text
                            .lines()
                            .any(|line| line.starts_with("Damage (") || line.starts_with("伤害 (")),
                    "simple mode hides formula and numeric damage lines: {text}"
                );
            }
            if !detailed {
                assert!(text.starts_with("等级") || text.starts_with("Rank"));
                assert!(
                    text.contains('·'),
                    "simple mode retains one short effect description"
                );
                assert_eq!(text.lines().count(), 1);
            }
            assert_eq!(
                world.resource::<PlayerBuild>().skills[0],
                2,
                "preview/toggle cannot apply upgrade"
            );
            println!("[capture] selection detail frame={frame}, detailed={detailed}: {text}");
            if frame == 120 {
                println!(
                    "[capture] PASS selection-details: real paid draw, detailed cooldown/damage/current-next, simple toggle, unchanged learned rank"
                );
            }
        }
        _ => {}
    }
}
