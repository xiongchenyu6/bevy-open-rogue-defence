//! Controlled full bar, actual paid G entry and actual pointer settlement.
use bevy::{
    camera::RenderTarget,
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
        mouse::MouseButtonInput,
    },
    prelude::*,
    text::TextLayoutInfo,
    window::{CursorMoved, WindowRef},
};
use rogue_commander::game::{
    ActiveSkill, AttackClock, Autoplay, CampaignMenu, DraftChoice, DraftState, GameSession,
    PlayerBuild, RunRng, SelectedHero, SkillEconomy, SkillId, SpawnDirector,
};
#[derive(Resource)]
struct Probe {
    frame: u32,
    expected: bool,
    choice: Option<(usize, SkillId, SkillId)>,
    targets: Vec<(Entity, RenderTarget)>,
    attack: f32,
}
pub fn configure(world: &mut World) {
    let expected = match std::env::var("EXPECT_FULL_SLOT_VARIETY").as_deref() {
        Ok("0") => false,
        Ok("1") => true,
        _ => panic!("EXPECT_FULL_SLOT_VARIETY must be0or1"),
    };
    world.remove_resource::<Autoplay>();
    world.resource_mut::<CampaignMenu>().0 = false;
    world.resource_mut::<SelectedHero>().locked = true;
    world.resource_mut::<SpawnDirector>().enabled = false;
    world
        .resource_mut::<super::CaptureControls>()
        .suppress_drafts = false;
    world.insert_resource(Probe {
        frame: 0,
        expected,
        choice: None,
        targets: Vec::new(),
        attack: 0.,
    });
}
pub fn install(app: &mut bevy::app::SubApp) {
    app.world_mut().resource_mut::<AttackClock>().0.pause();
    app.world_mut().resource_mut::<ActiveSkill>().remaining = 10000.;
    app.add_systems(First, input)
        .add_systems(PreUpdate, restore.after(bevy::ui::UiSystems::Focus))
        .add_systems(Last, audit);
}
fn input(world: &mut World) {
    world.resource_mut::<Probe>().frame += 1;
    let frame = world.resource::<Probe>().frame;
    if frame == 5 {
        let seed = std::env::var("ROGUE_CAPTURE_VARIETY_SEED").unwrap_or_else(|_| "A11CE".into());
        world.insert_resource(RunRng::from_seed(
            u64::from_str_radix(seed.trim_start_matches("0x"), 16).unwrap(),
        ));
        let mut build = world.resource_mut::<PlayerBuild>();
        build.skills.fill(0);
        build.skills[..6].fill(4);
        let mut session = world.resource_mut::<GameSession>();
        session.elapsed = 100.;
        session.level = 8;
        session.gold = 80;
        session.skill_points = 0;
        session.auto_bond_draws = false;
        let mut economy = world.resource_mut::<SkillEconomy>();
        economy.first_free_granted = true;
        economy.level_grants = 1;
    }
    if matches!(frame, 10 | 11) {
        let window = world
            .query_filtered::<Entity, With<Window>>()
            .iter(world)
            .next()
            .unwrap();
        world.write_message(KeyboardInput {
            key_code: KeyCode::KeyG,
            logical_key: Key::Character("g".into()),
            state: if frame == 10 {
                ButtonState::Pressed
            } else {
                ButtonState::Released
            },
            text: if frame == 10 { Some("g".into()) } else { None },
            repeat: false,
            window,
        });
    }
    if frame == 50 && world.resource::<Probe>().expected {
        let choice = world
            .resource::<DraftState>()
            .choices
            .iter()
            .enumerate()
            .find_map(|(i, c)| match c {
                DraftChoice::ReplaceSkill { old, new } => Some((i, *old, *new)),
                _ => None,
            })
            .expect("actual G must offer a replacement");
        world.resource_mut::<Probe>().choice = Some(choice);
    }
    if matches!(frame, 80 | 130 | 134) && world.resource::<Probe>().expected {
        let index = world.resource::<Probe>().choice.unwrap().0;
        let position = world
            .query::<(&Name, &UiGlobalTransform)>()
            .iter(world)
            .find(|(n, _)| n.as_str() == format!("Build card {index}"))
            .unwrap()
            .1
            .translation;
        let window = world
            .query_filtered::<Entity, With<Window>>()
            .iter(world)
            .next()
            .unwrap();
        let scale = world.get::<Window>(window).unwrap().scale_factor();
        let position = position / scale;
        world
            .get_mut::<Window>(window)
            .unwrap()
            .set_cursor_position(Some(position));
        world.write_message(CursorMoved {
            window,
            position,
            delta: None,
        });
        let targets: Vec<_> = world
            .query_filtered::<(Entity, &RenderTarget), With<Camera>>()
            .iter(world)
            .map(|(e, t)| (e, t.clone()))
            .collect();
        for (entity, _) in &targets {
            *world.get_mut::<RenderTarget>(*entity).unwrap() =
                RenderTarget::Window(WindowRef::Entity(window));
        }
        world.resource_mut::<Probe>().targets = targets;
        if frame != 80 {
            world.write_message(MouseButtonInput {
                button: MouseButton::Left,
                state: if frame == 130 {
                    ButtonState::Pressed
                } else {
                    ButtonState::Released
                },
                window,
            });
        }
    }
}
fn restore(world: &mut World) {
    let targets = std::mem::take(&mut world.resource_mut::<Probe>().targets);
    for (e, t) in targets {
        *world.get_mut::<RenderTarget>(e).unwrap() = t;
    }
}
fn audit(world: &mut World) {
    let frame = world.resource::<Probe>().frame;
    if frame == 100 {
        let attack = world.resource::<PlayerBuild>().attack_damage;
        world.resource_mut::<Probe>().attack = attack;
        let draft = world.resource::<DraftState>();
        assert!(draft.root.is_some());
        assert_eq!(draft.choices.len(), 3);
        let replacements = draft
            .choices
            .iter()
            .filter(|c| matches!(c, DraftChoice::ReplaceSkill { .. }))
            .count();
        if world.resource::<Probe>().expected {
            assert!(replacements > 0);
        } else {
            assert_eq!(replacements, 0);
        }
        assert_eq!(world.resource::<GameSession>().gold, 0);
        assert_eq!(world.resource::<GameSession>().skill_points, 0);
        assert_eq!(world.resource::<SkillEconomy>().paid_draws, 1);
        println!(
            "[capture] FULL_SLOT_OFFER choices={:?} gold=0 paid_draws=1",
            draft.choices
        );
        for (name, node, layout, text) in world
            .query::<(&Name, &ComputedNode, &TextLayoutInfo, &Text)>()
            .iter(world)
        {
            if name.as_str().starts_with("Build title ") {
                assert!(
                    layout.size.x <= node.size().x + 1. && layout.size.y <= node.size().y + 1.,
                    "title overflow {name}: {text:?} {:?}/{:?}",
                    layout.size,
                    node.size()
                );
                println!(
                    "[capture] FULL_SLOT_TITLE {name} text={:?} glyph={:?} node={:?}",
                    text.0,
                    layout.size,
                    node.size()
                );
            }
        }
    }
    if frame == 160 {
        let probe = world.resource::<Probe>();
        if let Some((_, old, new)) = probe.choice {
            let build = world.resource::<PlayerBuild>();
            assert_eq!(build.skills[usize::from(old.0)], 0);
            assert_eq!(build.skills[usize::from(new.0)], 1);
            assert_eq!(build.attack_damage, probe.attack);
            assert!(world.resource::<DraftState>().root.is_none());
            println!(
                "[capture] PASS full-slot-variety actual-G paid80 pointer-swap old={} new={} rarity={:?} old_rank=0 new_rank=1 attack_preserved=true",
                old.0,
                new.0,
                new.definition().rarity
            );
        } else {
            assert!(!probe.expected);
            println!("[capture] PASS full-slot-variety before actual-G owned-upgrades-only paid80");
        }
    }
}
