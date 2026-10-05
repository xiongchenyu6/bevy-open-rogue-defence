//! Real settings departure actions preserve a run until explicitly confirmed.
use bevy::{app::SubApp, prelude::*};
use rogue_commander::game::{
    Autoplay, CampaignMenu, CampaignProgress, CampaignReward, CampaignRun, Enemy, GameSession,
    RunSeed, SelectedHero, SettingsDepartureInputSet, SpawnDirector,
};

#[derive(Resource)]
struct Navigation {
    frame: u32,
    seed: u64,
    gold: u32,
    selected: SelectedHero,
    run_id: u64,
    progress: CampaignProgress,
    enemy: Entity,
}

pub fn install(app: &mut SubApp) {
    let world = app.world_mut();
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SelectedHero>().locked = true;
    world.resource_mut::<SpawnDirector>().enabled = false;
    {
        let mut progress = world.resource_mut::<CampaignProgress>();
        progress.currency = 37;
        progress.unlocked = 2;
        progress.upgrades[0] = 1;
        progress.cleared[0] = true;
    }
    {
        let mut session = world.resource_mut::<GameSession>();
        session.paused = false;
        session.auto_bond_draws = false;
        session.gold = 321;
        session.gold_income_accum = 0.0;
    }
    let enemy = world
        .spawn((
            Enemy {
                hp: 100_000.0,
                max_hp: 100_000.0,
                speed: 0.0,
                damage: 0.0,
                xp: 0,
                wood: 0,
                gold: 0,
                kill_credit: 0,
                elite: false,
            },
            Transform::from_xyz(12.0, 0.0, 0.0),
            Name::new("Settings departure cleanup probe"),
        ))
        .id();
    let selected = world.resource::<SelectedHero>();
    let selected = SelectedHero {
        kind: selected.kind,
        locked: selected.locked,
    };
    let capture = Navigation {
        frame: 0,
        seed: world.resource::<RunSeed>().0,
        gold: 321,
        selected,
        run_id: world.resource::<CampaignRun>().run_id,
        progress: world.resource::<CampaignProgress>().clone(),
        enemy,
    };
    assert!(
        world.resource::<CampaignReward>().last.is_none(),
        "unfinished fixture must not already have a reward"
    );
    world.insert_resource(capture);
    app.add_systems(
        PreUpdate,
        input
            .after(bevy::ui::UiSystems::Focus)
            .before(SettingsDepartureInputSet),
    );
    app.add_systems(Last, audit);
}
fn input(world: &mut World) {
    let frame = {
        let mut c = world.resource_mut::<Navigation>();
        c.frame += 1;
        c.frame
    };
    let pressed: Vec<Entity> = world
        .query::<(Entity, &Interaction)>()
        .iter(world)
        .filter_map(|(e, i)| (*i == Interaction::Pressed).then_some(e))
        .collect();
    for e in pressed {
        *world.get_mut::<Interaction>(e).unwrap() = Interaction::None;
    }
    if frame == 45 {
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
    }
    if frame == 46 {
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::Escape);
    }
    let label = match frame {
        5 | 80 => "Settings open",
        15 | 55 => "Settings chapters",
        25 => "Settings departure cancel",
        35 => "Settings quit",
        65 => "Settings departure confirm",
        _ => return,
    };
    let entity = world
        .query::<(Entity, &Name, &Interaction)>()
        .iter(world)
        .find_map(|(e, n, _)| (n.as_str() == label).then_some(e))
        .unwrap_or_else(|| panic!("missing real action {label}"));
    *world.get_mut::<Interaction>(entity).unwrap() = Interaction::Pressed;
    println!("SETTINGS_NAVIGATION_ACTION {frame} {label}");
}
fn named(world: &mut World, name: &str) -> bool {
    world
        .query::<&Name>()
        .iter(world)
        .any(|n| n.as_str() == name)
}
fn audit(world: &mut World) {
    let frame = world.resource::<Navigation>().frame;
    if ![10, 20, 30, 40, 50, 60, 70, 90].contains(&frame) {
        return;
    }
    if frame < 65 {
        let c = world.resource::<Navigation>();
        let s = world.resource::<GameSession>();
        let selected = world.resource::<SelectedHero>();
        assert!(
            s.paused,
            "settings and cancellation must preserve settings pause"
        );
        assert_eq!(s.gold, c.gold);
        assert_eq!(world.resource::<RunSeed>().0, c.seed);
        assert_eq!(world.resource::<CampaignRun>().run_id, c.run_id);
        assert_eq!(selected.kind, c.selected.kind);
        assert_eq!(selected.locked, c.selected.locked);
        assert!(
            world.get::<Enemy>(c.enemy).is_some(),
            "cancel must preserve live enemy"
        );
        assert_eq!(
            named(world, "Settings departure confirmation"),
            [20, 40, 60].contains(&frame)
        );
    } else {
        let c = world.resource::<Navigation>();
        assert!(world.resource::<CampaignMenu>().0);
        assert!(world.resource::<GameSession>().paused);
        assert_eq!(world.resource::<CampaignRun>().run_id, c.run_id + 1);
        assert!(world.get::<Enemy>(c.enemy).is_none());
        assert_eq!(world.query::<&Enemy>().iter(world).count(), 0);
        assert!(!named(world, "Settings departure confirmation"));
        assert!(named(world, "Campaign chapter 0"));
    }
    let c = world.resource::<Navigation>();
    let progress = world.resource::<CampaignProgress>();
    assert_eq!(progress.unlocked, c.progress.unlocked);
    assert_eq!(progress.currency, c.progress.currency);
    assert_eq!(progress.upgrades, c.progress.upgrades);
    assert_eq!(progress.cleared, c.progress.cleared);
    assert!(
        world.resource::<CampaignReward>().last.is_none(),
        "abandoning a run grants no victory reward"
    );
    println!("SETTINGS_NAVIGATION_STAGE {frame}");
    if frame == 90 {
        let roots: Vec<(String, i32)> = world
            .query::<(&Name, &GlobalZIndex)>()
            .iter(world)
            .map(|(name, z)| (name.as_str().to_owned(), z.0))
            .collect();
        let settings_z = roots
            .iter()
            .find(|(name, _)| name == "Settings page")
            .expect("settings reopened from chapters")
            .1;
        let campaign_z = roots
            .iter()
            .find(|(name, _)| name == "Campaign overlay")
            .expect("chapter menu remains behind settings")
            .1;
        assert!(
            settings_z > campaign_z,
            "settings must draw above the real campaign overlay"
        );
        let window = world
            .query::<&Window>()
            .iter(world)
            .next()
            .expect("capture window");
        let viewport = Vec2::new(
            window.physical_width() as f32,
            window.physical_height() as f32,
        );
        let mut count = 0;
        for (name, node, transform) in world
            .query_filtered::<(&Name, &ComputedNode, &UiGlobalTransform), With<Button>>()
            .iter(world)
        {
            if !name.as_str().starts_with("Settings ") {
                continue;
            }
            count += 1;
            assert!(
                node.size().min_element() >= 43.5,
                "{name} needs a 44px hit target, got {:?}",
                node.size()
            );
            let half = node.size() * 0.5;
            let min = transform.translation - half;
            let max = transform.translation + half;
            assert!(
                min.cmpge(Vec2::splat(-1.0)).all() && max.cmple(viewport + Vec2::ONE).all(),
                "{name} lies outside viewport: {min:?}..{max:?}"
            );
        }
        assert!(count >= 5, "real settings controls must be present");
        println!("SETTINGS_NAVIGATION_CHAPTER_SETTINGS_VISIBLE {count}");
        println!("SETTINGS_NAVIGATION_PASS");
    }
}
