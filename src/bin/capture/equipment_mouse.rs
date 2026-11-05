//! Controlled mouse-message integration, not OS input or earned progression.
use bevy::{
    camera::RenderTarget,
    input::{ButtonState, InputSystems, mouse::MouseButtonInput},
    prelude::*,
    ui::UiSystems,
    window::{CursorMoved, WindowRef},
};
use rogue_commander::game::{GameLanguage, GameSession};

#[derive(Resource, Default)]
struct Probe {
    frame: u32,
    targets: Vec<(Entity, RenderTarget)>,
}
pub fn install(app: &mut bevy::app::SubApp) {
    super::equipment_capture::install_mouse_fixture(app);
    app.init_resource::<Probe>()
        .add_systems(PreUpdate, drive.before(InputSystems))
        .add_systems(
            PreUpdate,
            restore
                .after(UiSystems::Focus)
                .before(rogue_commander::game::SettingsInputSet),
        )
        .add_systems(Last, audit);
}
fn named(world: &mut World, wanted: &str) -> Entity {
    world
        .query::<(Entity, &Name)>()
        .iter(world)
        .find(|(_, name)| name.as_str() == wanted)
        .unwrap_or_else(|| panic!("actual UI entity exists: {wanted}"))
        .0
}
fn cursor(world: &mut World, target: &str) {
    let entity = named(world, target);
    let node = world.get::<ComputedNode>(entity).unwrap();
    assert!(node.size().min_element() > 0.);
    let center = world.get::<UiGlobalTransform>(entity).unwrap().translation;
    let window = world
        .query_filtered::<Entity, With<Window>>()
        .single(world)
        .unwrap();
    let logical = center / world.get::<Window>(window).unwrap().scale_factor();
    world
        .get_mut::<Window>(window)
        .unwrap()
        .set_cursor_position(Some(logical));
    world.write_message(CursorMoved {
        window,
        position: logical,
        delta: None,
    });
    println!("EQUIPMENT_MOUSE_CURSOR {target} center={logical:?}");
}
fn button(world: &mut World, pressed: bool) {
    let window = world
        .query_filtered::<Entity, With<Window>>()
        .single(world)
        .unwrap();
    world.write_message(MouseButtonInput {
        window,
        button: MouseButton::Left,
        state: if pressed {
            ButtonState::Pressed
        } else {
            ButtonState::Released
        },
    });
}
fn drive(world: &mut World) {
    world.resource_mut::<Probe>().frame += 1;
    let frame = world.resource::<Probe>().frame;
    if frame < 200 {
        return;
    }
    // UiFocus only accepts Window targets. Route the actual offscreen camera
    // to its logical Window for focus, then restore its image before rendering.
    let window = world
        .query_filtered::<Entity, With<Window>>()
        .single(world)
        .unwrap();
    let targets: Vec<_> = world
        .query_filtered::<(Entity, &RenderTarget), With<Camera>>()
        .iter(world)
        .map(|(e, t)| (e, t.clone()))
        .collect();
    for (e, _) in &targets {
        *world.get_mut::<RenderTarget>(*e).unwrap() =
            RenderTarget::Window(WindowRef::Entity(window));
    }
    world.resource_mut::<Probe>().targets = targets;
    if (220..270).contains(&frame) && (frame - 220).is_multiple_of(10) {
        let index = (frame - 220) / 10;
        cursor(world, &format!("Equipment item {index}"));
        button(world, true);
        button(world, false);
        println!("EQUIPMENT_MOUSE_INPUT same-tick item={index} frame={frame}");
    }
    if (280..330).contains(&frame) && (frame - 280).is_multiple_of(10) {
        let index = (frame - 280) / 10;
        cursor(world, &format!("Equipment item {index}"));
        button(world, true);
        println!("EQUIPMENT_MOUSE_INPUT held item={index} frame={frame}");
    }
    if (283..333).contains(&frame) && (frame - 283).is_multiple_of(10) {
        button(world, false);
    }
    if frame == 345 {
        cursor(world, "Equipment close");
        button(world, true);
    }
    if frame == 348 {
        button(world, false);
    }
}
fn restore(world: &mut World) {
    let targets = std::mem::take(&mut world.resource_mut::<Probe>().targets);
    for (entity, target) in targets {
        *world.get_mut::<RenderTarget>(entity).unwrap() = target;
    }
}
fn audit(world: &mut World) {
    let frame = world.resource::<Probe>().frame;
    let index = if (222..272).contains(&frame) && (frame - 222).is_multiple_of(10) {
        Some(((frame - 222) / 10, "same-tick"))
    } else if (282..332).contains(&frame) && (frame - 282).is_multiple_of(10) {
        Some(((frame - 282) / 10, "held"))
    } else {
        None
    };
    if let Some((index, mode)) = index {
        let expected = [
            ("Serrated Disc", "锯齿圆盘"),
            ("Arc Lens", "奥术透镜"),
            ("Hardened Plate", "淬硬甲片"),
            ("Wind Boots", "疾风靴"),
            ("Blood Contract", "血契"),
        ][index as usize];
        let language = *world.resource::<GameLanguage>();
        let expected = language.pick(expected.0, expected.1);
        let text = named(world, "Equipment detail name");
        assert!(
            world.get::<Text>(text).unwrap().0.ends_with(expected),
            "actual Item selection {mode}/{index}: {:?}",
            world.get::<Text>(text).unwrap().0
        );
        let card = named(world, &format!("Equipment item {index}"));
        let art = world
            .get::<Children>(card)
            .unwrap()
            .iter()
            .find_map(|child| world.get::<ImageNode>(child))
            .unwrap()
            .clone();
        let selected = named(world, "Equipment selected art");
        let selected = world.get::<ImageNode>(selected).unwrap();
        assert_eq!(art.image, selected.image, "selected art atlas identity");
        assert_eq!(art.rect, selected.rect, "selected art exact region");
        println!("EQUIPMENT_MOUSE_PASS {mode} item={index} title={expected}");
    }
    if frame == 355 {
        assert!(
            !world
                .query::<&Name>()
                .iter(world)
                .any(|n| n.as_str() == "Equipment panel")
        );
        assert!(!world.resource::<GameSession>().paused);
        println!(
            "EQUIPMENT_MOUSE_PASS all 5 same-tick + 5 held Item selections, exact titles/art, real mouse Close; synthetic native only"
        );
    }
}
