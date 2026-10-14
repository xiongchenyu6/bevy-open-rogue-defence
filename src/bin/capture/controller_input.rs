//! Synthetic native gamepad proof. All navigation and purchases use raw device events.
use bevy::{
    app::SubApp,
    input::gamepad::{
        GamepadAxis, GamepadConnection, GamepadConnectionEvent, RawGamepadAxisChangedEvent,
        RawGamepadButtonChangedEvent, RawGamepadEvent,
    },
    prelude::*,
};
use rogue_commander::game::{
    AdventureBossState, Autoplay, CampaignMenu, CaptureTarget, ControllerDisabled, ControllerFocus,
    ControllerScope, CraftingState, DraftChoice, DraftState, GameSession, Hero, PlayerBuild,
    RecipeId, SelectedHero, SkillEconomy, SpawnDirector, prepare_crafting_market,
};
use std::collections::{HashMap, VecDeque};
#[derive(Clone, Copy)]
enum Action {
    Target(&'static str),
    Focus(&'static str),
    Button(GamepadButton),
    Move,
    Scroll(f32),
    Funding,
    Market,
    Shot(&'static str),
    Check(&'static str),
}
const ROUTE: &[Action] = &[
    Action::Shot("01-chapters"),
    Action::Target("Campaign training"),
    Action::Shot("01b-training"),
    Action::Button(GamepadButton::East),
    Action::Target("Campaign chapter 0"),
    Action::Shot("02-recruitment"),
    Action::Target("Commander card 0"),
    Action::Funding,
    Action::Move,
    Action::Check("movement"),
    Action::Shot("03-combat"),
    Action::Button(GamepadButton::RightTrigger),
    Action::Button(GamepadButton::Start),
    Action::Shot("04-pause"),
    Action::Target("Settings open"),
    Action::Target("Settings mute"),
    Action::Target("Settings music down"),
    Action::Target("Settings display toggle"),
    Action::Target("Settings display toggle"),
    Action::Target("Settings chapters"),
    Action::Shot("05-confirmation"),
    Action::Button(GamepadButton::East),
    Action::Check("cancel"),
    Action::Shot("06-settings"),
    Action::Target("Settings close"),
    Action::Button(GamepadButton::East),
    Action::Check("resume"),
    Action::Button(GamepadButton::East),
    Action::Focus("Skill draw control"),
    Action::Shot("03b-controller-tooltip"),
    Action::Target("Skill draw control"),
    Action::Shot("07-cards"),
    Action::Target("Build card 0"),
    Action::Check("draft"),
    Action::Target("Build card 0"),
    Action::Check("draft-second"),
    Action::Target("Build later"),
    Action::Check("banked"),
    Action::Target("Adventure camp entry 0"),
    Action::Shot("07b-camp-choice"),
    Action::Button(GamepadButton::East),
    Action::Check("camp-back"),
    Action::Target("Encounter boss safe reward"),
    Action::Check("camp-reward"),
    Action::Target("Skill draw control"),
    Action::Check("banked-reopen"),
    Action::Shot("07c-banked-reopen"),
    Action::Target("Build later"),
    Action::Target("Equipment open"),
    Action::Target("Equipment crafting tab"),
    Action::Target("Crafting recipe 0"),
    Action::Shot("08-forge-missing"),
    Action::Button(GamepadButton::East),
    Action::Market,
    Action::Shot("09-market"),
    Action::Target("Market offer 0"),
    Action::Target("Market offer 1"),
    Action::Target("Market offer 2"),
    Action::Target("Market offer 3"),
    Action::Target("Market offer 4"),
    Action::Button(GamepadButton::East),
    Action::Target("Equipment open"),
    Action::Target("Equipment crafting tab"),
    Action::Target("Crafting recipe 0"),
    Action::Shot("10-forge-ready"),
    Action::Target("Crafting confirm"),
    Action::Check("forged"),
    Action::Shot("11-forged"),
    Action::Button(GamepadButton::East),
    Action::Button(GamepadButton::Start),
    Action::Target("Settings open"),
    Action::Target("Settings chapters"),
    Action::Target("Settings departure confirm"),
    Action::Check("chapters"),
    Action::Shot("12-returned-chapters"),
];
const SCROLL_ROUTE: &[Action] = &[
    Action::Target("Campaign chapter 0"),
    Action::Target("Commander card 0"),
    Action::Button(GamepadButton::East),
    Action::Target("Equipment open"),
    Action::Target("Equipment crafting tab"),
    Action::Focus("Crafting recipe 5"),
    Action::Shot("scroll-before"),
    Action::Scroll(-1.),
    Action::Check("scroll-down"),
    Action::Shot("scroll-after"),
    Action::Scroll(1.),
    Action::Check("scroll-up"),
    Action::Shot("scroll-restored"),
];
#[derive(Resource)]
struct Driver {
    pad: Entity,
    scroll_mode: bool,
    axis_release: Option<u32>,
    scroll_before: f32,
    frame: u32,
    step: usize,
    next: u32,
    release: Option<(u32, GamepadButton)>,
    origin: Option<Vec3>,
    skill_levels: u32,
    banked: Vec<DraftChoice>,
    banked_gold: u32,
    banked_paid_draws: u32,
}
pub fn configure(world: &mut World) {
    world.remove_resource::<Autoplay>();
    world.resource_mut::<CampaignMenu>().0 = true;
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
    world
        .resource_mut::<super::CaptureControls>()
        .suppress_drafts = false;
}
pub fn install(app: &mut SubApp) {
    let world = app.world_mut();
    let pad = world.spawn(Name::new("Synthetic capture controller")).id();
    let connection = GamepadConnectionEvent::new(
        pad,
        GamepadConnection::Connected {
            name: "Native capture probe".into(),
            vendor_id: None,
            product_id: None,
        },
    );
    world
        .resource_mut::<Messages<GamepadConnectionEvent>>()
        .write(connection.clone());
    world
        .resource_mut::<Messages<RawGamepadEvent>>()
        .write(RawGamepadEvent::Connection(connection));
    world.insert_resource(Driver {
        pad,
        scroll_mode: std::env::args()
            .nth(4)
            .is_some_and(|s| s == "controller-scroll"),
        axis_release: None,
        scroll_before: 0.,
        frame: 0,
        step: 0,
        next: 15,
        release: None,
        origin: None,
        skill_levels: 0,
        banked: vec![],
        banked_gold: 0,
        banked_paid_draws: 0,
    });
    app.add_systems(First, drive);
    app.add_systems(Last, keep_isolated);
}
fn keep_isolated(mut director: ResMut<SpawnDirector>) {
    director.enabled = false;
}
fn button(world: &mut World, pad: Entity, button: GamepadButton, value: f32) {
    world
        .resource_mut::<Messages<RawGamepadEvent>>()
        .write(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
            pad, button, value,
        )));
}
fn axis(world: &mut World, pad: Entity, axis: GamepadAxis, value: f32) {
    world
        .resource_mut::<Messages<RawGamepadEvent>>()
        .write(RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
            pad, axis, value,
        )));
}
fn assert_disjoint(world: &mut World, a: &str, b: &str) {
    let mut q = world.query::<(&Name, &ComputedNode, &UiGlobalTransform)>();
    let mut rect = |name: &str| {
        q.iter(world)
            .find(|(n, _, _)| n.as_str() == name)
            .map(|(_, n, t)| Rect::from_center_size(t.translation, n.size()))
            .unwrap()
    };
    let x = rect(a);
    let y = rect(b);
    assert!(
        x.max.x <= y.min.x || y.max.x <= x.min.x || x.max.y <= y.min.y || y.max.y <= x.min.y,
        "{a} overlaps {b}: {x:?} / {y:?}"
    );
}
fn named(world: &mut World, name: &str) -> bool {
    world
        .query::<&Name>()
        .iter(world)
        .any(|n| n.as_str() == name)
}
fn scope(world: &World, entity: Entity) -> Option<i32> {
    let mut current = Some(entity);
    let mut scope = 0;
    while let Some(id) = current {
        if world
            .get::<Node>(id)
            .is_some_and(|n| n.display == Display::None)
        {
            return None;
        }
        if world
            .get::<Visibility>(id)
            .is_some_and(|v| *v == Visibility::Hidden)
        {
            return None;
        }
        if let Some(z) = world.get::<ControllerScope>(id) {
            scope = scope.max(z.0);
        }
        current = world.get::<ChildOf>(id).map(ChildOf::parent);
    }
    Some(scope)
}
fn semantic_key(world: &World, entity: Entity) -> String {
    let mut names = vec![];
    let mut current = Some(entity);
    while let Some(id) = current {
        if let Some(name) = world.get::<Name>(id) {
            names.push(name.as_str().to_owned());
        } else if let Some(parent) = world.get::<ChildOf>(id)
            && let Some(children) = world.get::<Children>(parent.parent())
        {
            names.push(format!(
                "#{}",
                children.iter().position(|child| child == id).unwrap_or(0)
            ));
        }
        current = world.get::<ChildOf>(id).map(ChildOf::parent);
    }
    names.reverse();
    names.join("/")
}
fn direction_to(world: &mut World, target: &str) -> Option<GamepadButton> {
    let focus = world.resource::<ControllerFocus>();
    let current = focus.entity?;
    let active = focus.scope;
    let mut q = world
        .query_filtered::<(Entity, Option<&Name>, &ComputedNode, &UiGlobalTransform), With<Button>>(
        );
    let mut controls: Vec<_> = q
        .iter(world)
        .filter(|(id, name, node, _)| {
            node.size().min_element() > 0.
                && world.get::<ControllerDisabled>(*id).is_none()
                && scope(world, *id).is_some_and(|value| {
                    value == active
                        || name.is_some_and(|n| n.as_str() == "Settings open")
                            && matches!(active, 90 | 100 | 150 | 400)
                })
        })
        .map(|(id, name, _, transform)| {
            (
                id,
                name.map(|n| n.as_str().to_owned()).unwrap_or_default(),
                transform.translation,
            )
        })
        .collect();
    controls.sort_by(|a, b| {
        scope(world, b.0)
            .cmp(&scope(world, a.0))
            .then(a.2.y.total_cmp(&b.2.y))
            .then(a.2.x.total_cmp(&b.2.x))
            .then(semantic_key(world, a.0).cmp(&semantic_key(world, b.0)))
    });
    let goal = controls.iter().find(|(_, name, _)| name == target)?.0;
    if current == goal {
        return Some(GamepadButton::South);
    }
    let directions = [
        (Vec2::X, GamepadButton::DPadRight),
        (-Vec2::X, GamepadButton::DPadLeft),
        (Vec2::Y, GamepadButton::DPadDown),
        (-Vec2::Y, GamepadButton::DPadUp),
    ];
    let mut queue = VecDeque::from([current]);
    let mut path: HashMap<Entity, (Entity, GamepadButton)> = HashMap::new();
    while let Some(from) = queue.pop_front() {
        let origin = controls.iter().find(|(id, _, _)| *id == from)?.2;
        for (direction, button) in directions {
            let next = controls
                .iter()
                .filter(|(id, _, p)| *id != from && (*p - origin).dot(direction) > 1.)
                .min_by(|a, b| {
                    score(a.2 - origin, direction).total_cmp(&score(b.2 - origin, direction))
                })
                .map(|(id, _, _)| *id);
            if let Some(next) = next
                && next != current
                && !path.contains_key(&next)
            {
                path.insert(next, (from, button));
                queue.push_back(next);
            }
        }
    }
    let mut node = goal;
    let mut first = None;
    while node != current {
        let (parent, button) = *path.get(&node)?;
        first = Some(button);
        node = parent;
    }
    first
}
fn score(offset: Vec2, direction: Vec2) -> f32 {
    offset.dot(direction) + offset.perp_dot(direction).abs() * 3.
}
fn drive(world: &mut World) {
    let mut driver = world.remove_resource::<Driver>().unwrap();
    driver.frame += 1;
    let route = if driver.scroll_mode {
        SCROLL_ROUTE
    } else {
        ROUTE
    };
    if driver.axis_release.is_some_and(|due| driver.frame >= due) {
        axis(world, driver.pad, GamepadAxis::RightStickY, 0.);
        driver.axis_release = None;
    }

    if let Some((when, key)) = driver.release
        && driver.frame >= when
    {
        button(world, driver.pad, key, 0.);
        driver.release = None;
    }
    if driver.frame == 3 {
        button(world, driver.pad, GamepadButton::North, 1.);
        driver.release = Some((4, GamepadButton::North));
    }
    if driver.step >= route.len() {
        if driver.frame == driver.next {
            println!(
                "CONTROLLER_ROUTE_PASS all {} steps; synthetic native only",
                route.len()
            );
        }
        world.insert_resource(driver);
        return;
    }
    assert!(
        driver.frame < 1600,
        "controller route stalled at step {}: focus {:?}",
        driver.step,
        world.resource::<ControllerFocus>()
    );
    if driver.frame < driver.next {
        world.insert_resource(driver);
        return;
    }
    let action = route[driver.step];
    let mut advance = true;
    let mut wait = 12;
    match action {
        Action::Target(target) | Action::Focus(target) => match direction_to(world, target) {
            Some(key) => {
                if key != GamepadButton::South || matches!(action, Action::Target(_)) {
                    button(world, driver.pad, key, 1.);
                }
                let hold = if key == GamepadButton::South { 8 } else { 1 };
                driver.release = Some((driver.frame + hold, key));
                advance = key == GamepadButton::South;
                wait = if advance { 20 } else { 4 };
                println!(
                    "CONTROLLER_ROUTE_INPUT step={} frame={} target={} key={:?}",
                    driver.step, driver.frame, target, key
                );
            }
            None => {
                advance = false;
                wait = 4;
            }
        },
        Action::Button(key) => {
            button(world, driver.pad, key, 1.);
            driver.release = Some((driver.frame + 1, key));
        }
        Action::Scroll(value) => {
            let mut q = world.query::<(&Name, &ScrollPosition)>();
            driver.scroll_before = q
                .iter(world)
                .find(|(n, _)| n.as_str() == "Equipment detail scroll")
                .unwrap()
                .1
                .y;
            axis(world, driver.pad, GamepadAxis::RightStickY, value);
            driver.axis_release = Some(driver.frame + 12);
            wait = 20;
        }
        Action::Move => {
            driver.origin = world
                .query_filtered::<&Transform, With<Hero>>()
                .iter(world)
                .next()
                .map(|t| t.translation);
            axis(world, driver.pad, GamepadAxis::LeftStickX, 0.6);
            button(world, driver.pad, GamepadButton::South, 1.);
            driver.release = Some((driver.frame + 1, GamepadButton::South));
            wait = 20;
        }
        Action::Funding => {
            println!("CONTROLLER_FIXTURE injected wallet/time, not earned progression");
            {
                let mut s = world.resource_mut::<GameSession>();
                s.gold = 10_000;
                s.wood = 500;
                s.kills = 10_000;
                s.elapsed = 240.;
                s.auto_bond_draws = false;
            }
            driver.skill_levels = world
                .resource::<PlayerBuild>()
                .skills
                .iter()
                .map(|v| u32::from(*v))
                .sum();
        }
        Action::Market => {
            println!(
                "CONTROLLER_FIXTURE injected deterministic stock; purchases use public controller navigation/button handlers"
            );
            prepare_crafting_market(world, 0);
            world.resource_mut::<GameSession>().paused = true;
        }
        Action::Shot(label) => {
            if label == "07-cards" {
                assert_disjoint(world, "Settings open", "Selection detail toggle");
            }
            if label == "03-combat" {
                assert_disjoint(world, "Controller guidance", "Settings open");
            }
            if label == "03b-controller-tooltip" {
                assert!(
                    world
                        .query::<&Text>()
                        .iter(world)
                        .any(|text| text.0.contains("Learn a skill · A (South)")),
                    "HUD tooltip follows controller input mode"
                );
            }
            let output = std::env::args().nth(2).unwrap();
            let path = std::path::PathBuf::from(output).join(format!("{label}.png"));
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            let target = world.resource::<CaptureTarget>().0.clone();
            super::queue_screenshot(world, target, path);
        }
        Action::Check(label) => {
            match label {
                "scroll-down" | "scroll-up" => {
                    let mut q = world.query::<(&Name, &ScrollPosition)>();
                    let after = q
                        .iter(world)
                        .find(|(n, _)| n.as_str() == "Equipment detail scroll")
                        .unwrap()
                        .1
                        .y;
                    if label == "scroll-down" {
                        assert!(after > driver.scroll_before + 1.);
                    } else {
                        assert!(after < driver.scroll_before - 1.);
                    }
                    let entity = world.resource::<ControllerFocus>().entity.unwrap();
                    let node = world.get::<ComputedNode>(entity).unwrap();
                    let position = world.get::<UiGlobalTransform>(entity).unwrap().translation;
                    let mut ancestor = world.get::<ChildOf>(entity).map(ChildOf::parent);
                    while let Some(id) = ancestor {
                        if world.get::<ScrollPosition>(id).is_some() {
                            let parent = world.get::<ComputedNode>(id).unwrap();
                            let center = world.get::<UiGlobalTransform>(id).unwrap().translation;
                            assert!(
                                position.y - node.size().y / 2.
                                    >= center.y - parent.size().y / 2. - 1.
                                    && position.y + node.size().y / 2.
                                        <= center.y + parent.size().y / 2. + 1.,
                                "focused recipe remains revealed"
                            );
                        }
                        ancestor = world.get::<ChildOf>(id).map(ChildOf::parent);
                    }
                    println!(
                        "CONTROLLER_SCROLL_OFFSET {label} before={} after={after} focus={entity:?}",
                        driver.scroll_before
                    );
                }
                "movement" => {
                    axis(world, driver.pad, GamepadAxis::LeftStickX, 0.);
                    let position = world
                        .query_filtered::<&Transform, With<Hero>>()
                        .iter(world)
                        .next()
                        .unwrap()
                        .translation;
                    assert!(position.distance(driver.origin.unwrap()) > 1.);
                    assert!(world.resource::<SelectedHero>().locked);
                }
                "cancel" => {
                    assert!(!named(world, "Settings departure confirmation"));
                    assert!(named(world, "Settings page"));
                    assert!(world.resource::<GameSession>().paused);
                }
                "resume" => assert!(!world.resource::<GameSession>().paused),
                "draft" | "draft-second" => {
                    let levels: u32 = world
                        .resource::<PlayerBuild>()
                        .skills
                        .iter()
                        .map(|v| u32::from(*v))
                        .sum();
                    assert_eq!(
                        levels,
                        driver.skill_levels + if label == "draft" { 1 } else { 2 },
                        "held confirm selects exactly one card"
                    );
                    assert!(
                        named(world, "Build selection"),
                        "funded next draft remains unselected"
                    );
                }
                "banked" => {
                    let draft = world.resource::<DraftState>();
                    assert!(draft.root.is_none() && draft.kind.is_some());
                    driver.banked = draft.choices.clone();
                    driver.banked_gold = world.resource::<GameSession>().gold;
                    driver.banked_paid_draws = world.resource::<SkillEconomy>().paid_draws;
                }
                "banked-reopen" => {
                    assert_eq!(world.resource::<DraftState>().choices, driver.banked);
                    assert!(world.resource::<DraftState>().root.is_some());
                    assert_eq!(
                        world.resource::<SkillEconomy>().paid_draws,
                        driver.banked_paid_draws,
                        "banked draft must not charge again"
                    );
                    assert!(
                        world.resource::<GameSession>().gold >= driver.banked_gold + 90,
                        "wallet retains camp reward and normal passive income"
                    );
                }
                "camp-back" => {
                    assert_eq!(world.resource::<AdventureBossState>().pending, Some(0));
                    assert!(world.resource::<GameSession>().paused);
                }
                "camp-reward" => {
                    assert!(world.resource::<AdventureBossState>().pending.is_none());
                    assert_eq!(world.resource::<AdventureBossState>().resolved & 1, 1);
                }
                "forged" => assert!(
                    world
                        .resource::<CraftingState>()
                        .acquired(RecipeId::Bloodreaver)
                ),
                "chapters" => assert!(world.resource::<CampaignMenu>().0),
                _ => unreachable!(),
            };
            println!("CONTROLLER_ROUTE_CHECK {label} PASS frame={}", driver.frame);
        }
    }
    if advance {
        driver.step += 1;
    }
    driver.next = driver.frame + wait;
    world.insert_resource(driver);
}
