//! Real recruitment layouts and Settings placement across viewport changes.
use bevy::{app::SubApp, prelude::*, render::render_resource::Extent3d};
use rogue_commander::game::{
    Autoplay, CampaignMenu, CaptureTarget, GameSession, RecruitmentProgress, SelectedHero,
    SettingsDepartureInputSet,
};
#[derive(Resource)]
struct Probe {
    frame: u32,
    original: (u32, u32),
    clicked: u32,
}
pub fn configure(world: &mut World) {
    world.remove_resource::<Autoplay>();
    world.resource_mut::<CampaignMenu>().0 = false;
    world.resource_mut::<SelectedHero>().locked = false;
    let mut session = world.resource_mut::<GameSession>();
    session.paused = false;
    session.level = 1;
    session.skill_points = 0;
    session.auto_bond_draws = false;
    let original = world
        .query::<&Window>()
        .iter(world)
        .next()
        .map(|w| (w.physical_width(), w.physical_height()))
        .unwrap();
    world.insert_resource(Probe {
        frame: 0,
        original,
        clicked: 0,
    });
}
pub fn install(app: &mut SubApp) {
    app.add_systems(
        PreUpdate,
        input
            .after(bevy::ui::UiSystems::Focus)
            .before(SettingsDepartureInputSet),
    );
    app.add_systems(Last, audit);
}
fn resize(world: &mut World, width: u32, height: u32) {
    for mut window in world.query::<&mut Window>().iter_mut(world) {
        window.resolution.set_physical_resolution(width, height);
    }
    let target = world.resource::<CaptureTarget>().0.clone();
    world
        .resource_mut::<Assets<Image>>()
        .get_mut(&target)
        .unwrap()
        .resize(Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        });
    println!("PICKER_SETTINGS_RESIZE {width}x{height}");
}
fn input(world: &mut World) {
    let frame = {
        let mut p = world.resource_mut::<Probe>();
        p.frame += 1;
        p.frame
    };
    let pressed: Vec<Entity> = world
        .query::<(Entity, &Interaction)>()
        .iter(world)
        .filter_map(|(e, i)| (*i == Interaction::Pressed).then_some(e))
        .collect();
    for e in pressed {
        *world.get_mut::<Interaction>(e).unwrap() = Interaction::None;
    }
    if frame == 20 {
        let original = world.resource::<Probe>().original;
        let size = if original.1 < 540 {
            (1280, 720)
        } else {
            (640, 360)
        };
        resize(world, size.0, size.1);
    }
    if frame == 45 {
        let size = world.resource::<Probe>().original;
        resize(world, size.0, size.1);
    }
    if [85, 125, 165].contains(&frame) {
        world.resource_mut::<GameSession>().level = match frame {
            85 => 10,
            125 => 20,
            _ => 40,
        };
    }
    if frame == 205 {
        world.resource_mut::<RecruitmentProgress>().fruit_count = 1;
    }
    let name = match frame {
        70 => "Commander card 0",
        115 => "Commander card 1",
        155 => "Commander card 2",
        195 => "Commander card 3",
        240 => "Commander card 5",
        210 => "Fruit recruit",
        255 => "Settings open",
        265 => "Settings close",
        _ => return,
    };
    let e = world
        .query::<(Entity, &Name, &Interaction)>()
        .iter(world)
        .find_map(|(e, n, _)| (n.as_str() == name).then_some(e))
        .unwrap_or_else(|| panic!("missing actual picker control {name}"));
    *world.get_mut::<Interaction>(e).unwrap() = Interaction::Pressed;
    if name.starts_with("Commander card ") {
        world.resource_mut::<Probe>().clicked += 1;
    }
    println!("PICKER_SETTINGS_REAL_CLICK frame={frame} {name}");
}
fn rect(world: &World, e: Entity) -> (Vec2, Vec2) {
    let node = world
        .get::<ComputedNode>(e)
        .expect("actual computed UI node");
    let position = world
        .get::<UiGlobalTransform>(e)
        .expect("actual UI transform")
        .translation;
    (position - node.size() * 0.5, position + node.size() * 0.5)
}
fn named(world: &mut World, name: &str) -> Entity {
    world
        .query::<(Entity, &Name)>()
        .iter(world)
        .find_map(|(e, n)| (n.as_str() == name).then_some(e))
        .unwrap_or_else(|| panic!("missing actual UI node {name}"))
}
fn inside((min, max): (Vec2, Vec2), viewport: Vec2, label: &str) {
    assert!(
        min.cmpge(Vec2::splat(-1.0)).all() && max.cmple(viewport + Vec2::ONE).all(),
        "{label} outside viewport: {min:?}..{max:?}"
    );
}
fn audit(world: &mut World) {
    let frame = world.resource::<Probe>().frame;
    let modal = [15, 35, 60, 100, 140, 180, 225].contains(&frame);
    let normal = [80, 120, 160, 200, 250, 270].contains(&frame);
    if !modal && !normal && frame != 260 {
        return;
    }
    let viewport = world
        .query::<&Window>()
        .iter(world)
        .next()
        .map(|w| Vec2::new(w.physical_width() as f32, w.physical_height() as f32))
        .unwrap();
    if modal {
        let heading = named(world, "Commander selection heading");
        let settings = named(world, "Settings open");
        let h = rect(world, heading);
        let s = rect(world, settings);
        inside(h, viewport, "heading");
        inside(s, viewport, "Settings launcher");
        let overlap = h.0.x < s.1.x && s.0.x < h.1.x && h.0.y < s.1.y && s.0.y < h.1.y;
        assert!(
            !overlap,
            "actual heading overlaps Settings: heading={h:?}, settings={s:?}"
        );
        let footer = named(world, "Commander details footer");
        inside(rect(world, footer), viewport, "recruitment details footer");
        let expected = if frame < 70 { 3 } else { 6 };
        let mut cards = 0;
        for (e, name, node) in world
            .query_filtered::<(Entity, &Name, &ComputedNode), With<Button>>()
            .iter(world)
        {
            if !name.as_str().starts_with("Commander card ") {
                continue;
            }
            cards += 1;
            assert!(
                node.size().min_element() >= 43.5,
                "real choice remains clickable"
            );
            inside(rect(world, e), viewport, name.as_str());
            let art_area = *world
                .get::<Children>(e)
                .expect("actual card children")
                .first()
                .expect("actual art area");
            let portrait = *world
                .get::<Children>(art_area)
                .expect("actual art area children")
                .first()
                .expect("actual portrait image");
            assert!(
                world.get::<ImageNode>(portrait).is_some(),
                "first art child must be the real image"
            );
            let area = rect(world, art_area);
            let image = rect(world, portrait);
            assert!(
                image.0.cmpge(area.0 - Vec2::ONE).all() && image.1.cmple(area.1 + Vec2::ONE).all(),
                "{name} portrait clipped: image={image:?}, art_area={area:?}"
            );
            let size = world.get::<ComputedNode>(portrait).unwrap().size();
            assert!(
                size.y > 0.0 && (size.x / size.y - 0.8).abs() <= 0.02,
                "{name} real portrait aspect ratio must remain .8, got {size:?}"
            );
        }
        assert_eq!(cards, expected, "actual initial/late offer counts");
        println!("PICKER_SETTINGS_PORTRAIT_FIT_PASS frame={frame} actual_images={cards}");
        assert!(world.resource::<GameSession>().paused);
        let center = (s.0 + s.1) * 0.5;
        let expected_x = if viewport.y < 540.0 {
            viewport.x - 64.0
        } else {
            viewport.x * 0.5
        };
        assert!(
            (center.x - expected_x).abs() < 1.0,
            "launcher follows modal/resize policy"
        );
        println!(
            "PICKER_SETTINGS_MODAL_PASS frame={frame} cards={cards} viewport={viewport:?} heading={h:?} settings={s:?}"
        );
    }
    if normal {
        let settings = named(world, "Settings open");
        let s = rect(world, settings);
        let center = (s.0 + s.1) * 0.5;
        assert!(
            (center.x - viewport.x * 0.5).abs() < 1.0,
            "Settings returns to ordinary placement after real selection"
        );
        assert!(world.resource::<SelectedHero>().locked);
        assert!(world.resource::<RecruitmentProgress>().pending.is_none());
        println!("PICKER_SETTINGS_NORMAL_PLACEMENT_PASS frame={frame}");
    }
    if frame == 260 {
        let _ = named(world, "Settings page");
        println!("PICKER_SETTINGS_ACTUAL_OPEN_PASS");
    }
    if frame == 270 {
        assert_eq!(world.resource::<Probe>().clicked, 5);
        assert_eq!(
            world.resource::<RecruitmentProgress>().claimed_milestones & 7,
            7
        );
        assert_eq!(world.resource::<RecruitmentProgress>().fruit_count, 0);
        println!(
            "PICKER_SETTINGS_LAYOUT_PASS actual initial/milestone/fruit choices, resize, footer and Settings restore"
        );
    }
}
