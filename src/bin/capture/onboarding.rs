//! First-expedition onboarding and persisted dismissal through real UI actions.
use bevy::{app::SubApp, prelude::*};
use rogue_commander::game::{
    AssetReadinessPlugin, Autoplay, CampaignMenu, CampaignProgress, CampaignReward, CampaignRun,
    GameLanguage, GameSession, OnboardingInputSet, OnboardingPlugin, OnboardingState, RunReward,
    SelectedHero, SettingsDepartureInputSet,
};
use std::{
    fs,
    path::{Component, PathBuf},
};
#[derive(Resource)]
struct Probe {
    frame: u32,
    skip: bool,
    reload: bool,
    path: PathBuf,
    menu_probe: bool,
    result_probe: bool,
    settlement: Option<(u32, Option<RunReward>)>,
    failure: bool,
}
pub fn plugins(app: &mut App) {
    app.add_plugins((AssetReadinessPlugin, OnboardingPlugin));
}
pub fn configure(world: &mut World, scenario: &str) {
    let path = PathBuf::from(
        std::env::var_os("ROGUE_TUTORIAL_SAVE")
            .expect("onboarding captures require isolated ROGUE_TUTORIAL_SAVE"),
    );
    assert!(path.is_absolute() && !path.components().any(|c| matches!(c, Component::ParentDir)));
    assert!(
        path.starts_with(std::env::temp_dir())
            || path
                .starts_with(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/verification")),
        "never use real player preferences"
    );
    let failure = scenario == "onboarding-save-failure";
    if failure {
        assert!(
            !path.parent().unwrap().exists(),
            "save-failure scenario needs a fresh dedicated tutorial parent directory"
        );
    }
    let menu_probe = scenario == "onboarding-menu";
    let result_probe = scenario == "onboarding-result";
    let reload = scenario == "onboarding-reload" || menu_probe || result_probe;
    if reload {
        assert_eq!(fs::read_to_string(&path).unwrap().trim(), "1");
    } else {
        assert!(
            !path.exists(),
            "first-process onboarding capture needs fresh isolated preference"
        );
    }
    world.remove_resource::<Autoplay>();
    world.resource_mut::<CampaignMenu>().0 = true;
    world.resource_mut::<GameSession>().paused = true;
    if result_probe {
        // Terminal-state UI fixture; this does not claim a completed gameplay run.
        world.insert_resource(CampaignProgress::default());
        world.resource_mut::<CampaignMenu>().0 = false;
        let mut session = world.resource_mut::<GameSession>();
        session.finished = true;
        session.victory = true;
    }
    world.insert_resource(Probe {
        frame: 0,
        skip: scenario == "onboarding-skip" || failure,
        reload,
        path,
        menu_probe,
        result_probe,
        settlement: None,
        failure,
    });
}
pub fn install(app: &mut SubApp) {
    app.add_systems(
        PreUpdate,
        input
            .after(bevy::ui::UiSystems::Focus)
            .before(SettingsDepartureInputSet)
            .before(OnboardingInputSet),
    );
    app.add_systems(Last, audit);
}
fn press(world: &mut World, name: &str) {
    let e = world
        .query::<(Entity, &Name, &Interaction)>()
        .iter(world)
        .find_map(|(e, n, _)| (n.as_str() == name).then_some(e))
        .unwrap_or_else(|| panic!("missing actual onboarding control {name}"));
    *world.get_mut::<Interaction>(e).unwrap() = Interaction::Pressed;
}
fn input(world: &mut World) {
    let (frame, skip, reload) = {
        let mut p = world.resource_mut::<Probe>();
        p.frame += 1;
        (p.frame, p.skip, p.reload)
    };
    let pressed: Vec<Entity> = world
        .query::<(Entity, &Interaction)>()
        .iter(world)
        .filter_map(|(e, i)| (*i == Interaction::Pressed).then_some(e))
        .collect();
    for e in pressed {
        *world.get_mut::<Interaction>(e).unwrap() = Interaction::None;
    }
    if world.resource::<Probe>().menu_probe || world.resource::<Probe>().result_probe {
        let action = match frame {
            5 => Some("Settings open"),
            15 => Some("Settings tutorial"),
            25 => Some(if world.resource::<Probe>().menu_probe {
                "Campaign chapter 0"
            } else {
                "Campaign retry"
            }),
            40 => Some("Tutorial skip"),
            _ => None,
        };
        if frame == 25 {
            world
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::Digit1);
        }
        if frame == 26 {
            world
                .resource_mut::<ButtonInput<KeyCode>>()
                .release(KeyCode::Digit1);
        }
        if let Some(action) = action {
            press(world, action);
            println!("ONBOARDING_MENU_RESULT_ACTION frame={frame} {action}");
        }
        return;
    }
    if world.resource::<Probe>().failure {
        let path = world.resource::<Probe>().path.clone();
        let parent = path.parent().unwrap();
        let backup = parent.with_extension("capture-backup");
        if frame == 29 {
            assert!(
                !parent.exists() && !backup.exists(),
                "obstruct only a newly-owned private preference parent"
            );
            fs::create_dir_all(parent).unwrap();
            fs::rename(parent, &backup).unwrap();
            fs::write(parent, "capture-owned obstruction").unwrap();
        }
        if frame == 56 {
            fs::remove_file(parent).unwrap();
            fs::rename(&backup, parent).unwrap();
            println!("ONBOARDING_OWNED_SAVE_PATH_REPAIRED");
        }
    }
    if !skip && !reload {
        if frame == 90 {
            world
                .resource_mut::<ButtonInput<MouseButton>>()
                .press(MouseButton::Left);
        }
        if frame == 91 {
            press(world, "Commander card 0");
            world
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::Digit1);
            println!("ONBOARDING_HELD_CLOSE_UNDERLYING_INPUT_ATTEMPT");
        }
        if frame == 92 {
            world
                .resource_mut::<ButtonInput<MouseButton>>()
                .release(MouseButton::Left);
        }
    }
    let replay_block = if reload {
        65
    } else if skip {
        70
    } else {
        130
    };
    if frame == replay_block {
        press(world, "Commander card 0");
    }
    let key_attempt = if reload {
        frame == 65
    } else if skip {
        [20, 70].contains(&frame)
    } else {
        [20, 40, 60, 80, 130].contains(&frame)
    };
    if key_attempt || (!skip && !reload && frame == 91) {
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Digit1);
    } else {
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::Digit1);
    }
    let action = if frame == 10 {
        Some("Campaign chapter 0")
    } else if reload {
        match frame {
            40 => Some("Settings open"),
            55 => Some("Settings tutorial"),
            75 => Some("Tutorial skip"),
            85 => Some("Commander card 0"),
            _ => None,
        }
    } else if skip {
        match frame {
            30 => Some("Tutorial skip"),
            50 => Some("Settings open"),
            60 => Some("Settings tutorial"),
            80 => Some("Tutorial skip"),
            85 if world.resource::<Probe>().failure => Some("Settings open"),
            95 if world.resource::<Probe>().failure => Some("Settings close"),
            100 => Some("Commander card 0"),
            _ => None,
        }
    } else {
        match frame {
            30 | 50 | 70 | 90 => Some("Tutorial next"),
            110 => Some("Settings open"),
            120 => Some("Settings tutorial"),
            140 => Some("Tutorial skip"),
            160 => Some("Commander card 0"),
            _ => None,
        }
    };
    if let Some(action) = action {
        press(world, action);
        println!("ONBOARDING_ACTION frame={frame} {action}");
    }
}
fn audit(world: &mut World) {
    let (frame, skip, reload) = {
        let p = world.resource::<Probe>();
        (p.frame, p.skip, p.reload)
    };
    if world.resource::<Probe>().menu_probe || world.resource::<Probe>().result_probe {
        audit_menu_result(world, frame);
        return;
    }
    if !skip && !reload && frame == 91 {
        assert!(
            world.resource::<OnboardingState>().input_blocked,
            "held closing click must keep release latch active"
        );
        assert!(world.resource::<OnboardingState>().page.is_none());
        assert!(world.resource::<GameSession>().paused);
        assert!(!world.resource::<SelectedHero>().locked);
        println!("ONBOARDING_RELEASE_LATCH_HELD_PASS");
    }
    if !skip && !reload && frame == 92 {
        assert!(
            !world.resource::<OnboardingState>().input_blocked,
            "actual input release clears latch"
        );
        assert!(!world.resource::<SelectedHero>().locked);
        assert!(world.resource::<GameSession>().paused);
        println!("ONBOARDING_RELEASE_LATCH_RELEASED_PASS");
    }
    let settings_frame = if reload {
        50
    } else if skip {
        55
    } else {
        115
    };
    if frame == settings_frame {
        check_settings(world);
    }
    let final_frame = if reload {
        100
    } else if skip {
        120
    } else {
        180
    };
    let replay_frame = if reload {
        70
    } else if skip {
        75
    } else {
        135
    };
    let picker_frame = if reload {
        30
    } else if skip {
        40
    } else {
        100
    };
    let after_close = if reload {
        80
    } else if skip {
        90
    } else {
        150
    };
    let page = if !reload && !skip {
        match frame {
            25 => Some(0),
            45 => Some(1),
            65 => Some(2),
            85 => Some(3),
            _ => None,
        }
    } else if !reload && frame == 25 {
        Some(0)
    } else {
        None
    };
    if let Some(page) = page {
        check_page(world, page);
    }
    if frame == replay_frame {
        check_page(world, 0);
    }
    if frame <= after_close {
        assert!(
            !world.resource::<SelectedHero>().locked,
            "guide/key attempts must not choose a hero"
        );
        assert_eq!(
            world.resource::<GameSession>().elapsed,
            0.0,
            "gameplay clock stays frozen before an actual fresh choice"
        );
    }
    if [picker_frame, after_close].contains(&frame) {
        assert!(world.resource::<OnboardingState>().page.is_none());
        assert!(
            world.resource::<GameSession>().paused,
            "real picker retains its own pause after tutorial closes"
        );
        assert!(
            world
                .query::<&Name>()
                .iter(world)
                .any(|n| n.as_str() == "Commander card 0")
        );
        if (frame == after_close || !reload) && !(world.resource::<Probe>().failure && frame == 40)
        {
            assert_eq!(
                fs::read_to_string(&world.resource::<Probe>().path)
                    .unwrap()
                    .trim(),
                "1"
            );
        }
        println!("ONBOARDING_PICKER_PAUSE_PRESERVED frame={frame} reload={reload}");
    }
    if world.resource::<Probe>().failure && [40, 55, 90].contains(&frame) {
        let p = world.resource::<Probe>();
        let state = world.resource::<OnboardingState>();
        if frame < 80 {
            assert!(state.dismissed && state.save_failed && state.page.is_none());
            assert!(
                !p.path.exists(),
                "failed real preference write must not create dismissal file"
            );
        } else {
            assert!(state.dismissed && !state.save_failed);
            assert_eq!(fs::read_to_string(&p.path).unwrap().trim(), "1");
        }
        if frame == 55 || frame == 90 {
            let warning = world.query::<&Text>().iter(world).any(|t| {
                t.0.contains("could not be saved on this device")
                    || t.0.contains("无法在此设备上保存偏好")
            });
            assert_eq!(
                warning,
                frame == 55,
                "real Settings warning reflects failed/repaired dismissal persistence"
            );
        }
        println!("ONBOARDING_SAVE_FAILURE_FLOW frame={frame}");
    }
    if frame == final_frame {
        assert!(world.resource::<SelectedHero>().locked);
        assert!(world.resource::<GameSession>().elapsed > 0.0);
        assert_eq!(
            world.resource::<CampaignRun>().run_id,
            1,
            "one genuine first-expedition GUI start"
        );
        let state = world.resource::<OnboardingState>();
        assert!(state.dismissed && !state.save_failed && state.page.is_none());
        assert_eq!(
            fs::read_to_string(&world.resource::<Probe>().path)
                .unwrap()
                .trim(),
            "1"
        );
        println!(
            "ONBOARDING_PASS reload={reload} skip={skip} actual first run, persistence, replay, blocked inputs and fresh choice"
        );
    }
}
fn check_page(world: &mut World, page: usize) {
    let state = world.resource::<OnboardingState>();
    assert_eq!(state.page, Some(page));
    assert!(state.input_blocked);
    assert!(world.resource::<GameSession>().paused);
    let chinese = world.resource::<GameLanguage>().is_chinese();
    let titles = if chinese {
        ["移动与闪避", "三次生命", "持续成长", "永久成长"]
    } else {
        ["MOVE & DODGE", "THREE LIVES", "KEEP GROWING", "YOUR LEGACY"]
    };
    assert!(
        world
            .query::<&Text>()
            .iter(world)
            .any(|t| t.0.contains(titles[page])),
        "actual localized page title"
    );
    let image = world
        .query::<(&Name, &ImageNode)>()
        .iter(world)
        .find_map(|(n, i)| (n.as_str() == "Tutorial illustration").then_some(i.image.clone()))
        .expect("actual illustrated page");
    assert!(
        world
            .resource::<AssetServer>()
            .is_loaded_with_dependencies(image.id()),
        "illustration must genuinely load"
    );
    let viewport = world
        .query::<&Window>()
        .iter(world)
        .next()
        .map(|w| Vec2::new(w.physical_width() as f32, w.physical_height() as f32))
        .unwrap();
    for (name, node, t) in world
        .query_filtered::<(&Name, &ComputedNode, &UiGlobalTransform), With<Button>>()
        .iter(world)
    {
        if !name.as_str().starts_with("Tutorial ") {
            continue;
        }
        assert!(node.size().min_element() >= 43.5);
        let half = node.size() * 0.5;
        assert!(
            (t.translation - half).cmpge(Vec2::splat(-1.0)).all()
                && (t.translation + half).cmple(viewport + Vec2::ONE).all(),
            "{name} must stay inside compact viewport"
        );
    }
    println!(
        "ONBOARDING_ILLUSTRATED_PAGE page={page} locale={}",
        if chinese { "zh" } else { "en" }
    );
}

fn check_settings(world: &mut World) {
    assert!(
        world
            .query::<&Name>()
            .iter(world)
            .any(|n| n.as_str() == "Settings page")
    );
    let viewport = world
        .query::<&Window>()
        .iter(world)
        .next()
        .map(|w| Vec2::new(w.physical_width() as f32, w.physical_height() as f32))
        .unwrap();
    let mut count = 0;
    for (name, node, t) in world
        .query_filtered::<(&Name, &ComputedNode, &UiGlobalTransform), With<Button>>()
        .iter(world)
    {
        if !name.as_str().starts_with("Settings ") {
            continue;
        }
        count += 1;
        assert!(
            node.size().min_element() >= 41.5,
            "settings replay row and footer remain usable"
        );
        let half = node.size() * 0.5;
        assert!(
            (t.translation - half).cmpge(Vec2::splat(-1.0)).all()
                && (t.translation + half).cmple(viewport + Vec2::ONE).all(),
            "{name} clipped after tutorial entry: {:?}",
            t.translation
        );
    }
    assert!(count >= 5);
    println!("ONBOARDING_SETTINGS_LAYOUT_PASS controls={count}");
}
fn audit_menu_result(world: &mut World, frame: u32) {
    if frame == 1 {
        let snapshot = (
            world.resource::<CampaignProgress>().currency,
            world.resource::<CampaignReward>().last,
        );
        world.resource_mut::<Probe>().settlement = Some(snapshot);
    }
    if frame == 10 {
        check_settings(world);
    }
    if frame == 20 || frame == 30 {
        check_page(world, 0);
    }
    if ![10, 20, 30, 50, 60].contains(&frame) {
        return;
    }
    let p = world.resource::<Probe>();
    assert_eq!(
        world.resource::<CampaignMenu>().0,
        p.menu_probe,
        "replay preserves chapters/results context"
    );
    assert_eq!(world.resource::<GameSession>().finished, p.result_probe);
    assert!(world.resource::<GameSession>().paused);
    assert_eq!(
        world.resource::<CampaignRun>().run_id,
        0,
        "underlying chapter/retry action must remain blocked"
    );
    assert_eq!(
        (
            world.resource::<CampaignProgress>().currency,
            world.resource::<CampaignReward>().last
        ),
        p.settlement.unwrap(),
        "guide replay cannot duplicate result rewards"
    );
    if frame >= 50 {
        assert!(world.resource::<OnboardingState>().page.is_none());
        assert_eq!(fs::read_to_string(&p.path).unwrap().trim(), "1");
    }
    if frame == 60 {
        println!(
            "ONBOARDING_MENU_RESULT_PASS menu={} result={} pause/context/rewards preserved",
            p.menu_probe, p.result_probe
        );
    }
}
