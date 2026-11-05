//! Real shop rendering and input safety, using production offer and purchase handlers.
use bevy::{
    input::gamepad::{
        GamepadConnection, GamepadConnectionEvent, RawGamepadButtonChangedEvent, RawGamepadEvent,
    },
    prelude::*,
    ui::UiSystems,
};
use rogue_commander::game::{ControlScheme, ControllerFocus, GameSession};
#[derive(Resource, Default)]
struct Probe {
    frame: u32,
    before: u32,
    price: u32,
    pad: Option<Entity>,
    opened_elapsed: f32,
}
pub fn configure(world: &mut World) {
    super::market_wallet_capture::configure(world);
}
pub fn install(app: &mut bevy::app::SubApp) {
    super::market_wallet_capture::install_fixture(app);
    let world = app.world_mut();
    let pad = world.spawn(Name::new("Market controller fixture")).id();
    let connection = GamepadConnectionEvent::new(
        pad,
        GamepadConnection::Connected {
            name: "Market capture pad".into(),
            vendor_id: None,
            product_id: None,
        },
    );
    world.write_message(connection.clone());
    world.write_message(RawGamepadEvent::Connection(connection));
    app.insert_resource(Probe {
        pad: Some(pad),
        ..default()
    })
    .add_systems(
        PreUpdate,
        exercise
            .after(UiSystems::Focus)
            .before(rogue_commander::game::SettingsInputSet),
    );
}
fn interaction(world: &mut World, name: &str, value: Interaction) {
    let e = world
        .query::<(Entity, &Name)>()
        .iter(world)
        .find(|(_, n)| n.as_str() == name)
        .expect("real market control")
        .0;
    *world.get_mut::<Interaction>(e).unwrap() = value;
}
fn exercise(world: &mut World) {
    world.resource_mut::<Probe>().frame += 1;
    let frame = world.resource::<Probe>().frame;
    world.resource_mut::<ButtonInput<KeyCode>>().clear();
    world
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::KeyB);
    for mut i in world.query::<&mut Interaction>().iter_mut(world) {
        *i = Interaction::None;
    }
    if (40..60).contains(&frame) {
        interaction(world, "Market offer 1", Interaction::Hovered);
    }
    if (61..80).contains(&frame) {
        interaction(world, "Market auto 1", Interaction::Hovered);
    }
    match frame {
        20 => world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyB),
        21 => {
            assert!(world.resource::<GameSession>().paused);
            world.resource_mut::<Probe>().opened_elapsed = world.resource::<GameSession>().elapsed;
        }
        35 => {
            super::market_wallet_capture::visible_controls(world);
            println!("[capture] PASS all five item tiles and controls fit");
        }
        61 => {
            world.resource_mut::<GameSession>().kills = 0;
            interaction(world, "Market auto 1", Interaction::Pressed);
        }
        70 => {
            assert_eq!(world.resource::<GameSession>().kills, 0);
            assert!(world.query::<&Text>().iter(world).any(|t| t.0 == "✓"));
            println!("[capture] PASS auto-buy marked without accidental unaffordable purchase");
        }
        81 => {
            // Turn the auto-buy back off before changing the fixture wallet.
            interaction(world, "Market auto 1", Interaction::Pressed);
            world.resource_mut::<GameSession>().kills = 100_000;
        }
        85 => {
            world.insert_resource(ControlScheme::Touch);
            let before = world.resource::<GameSession>().kills;
            let price = world
                .query::<(&Name, &Text)>()
                .iter(world)
                .find(|(n, _)| n.as_str() == "Market price 2")
                .unwrap()
                .1
                .0
                .parse()
                .unwrap();
            world.resource_mut::<Probe>().before = before;
            world.resource_mut::<Probe>().price = price;
            interaction(world, "Market offer 2", Interaction::Pressed);
        }
        90 => {
            assert_eq!(
                world.resource::<GameSession>().kills,
                world.resource::<Probe>().before,
                "touch inspect must not buy"
            );
            println!("[capture] PASS touch tile activation previews without spending");
            interaction(world, "Market buy selected", Interaction::Pressed);
        }
        95 => {
            assert_eq!(
                world.resource::<GameSession>().kills,
                world.resource::<Probe>().before - world.resource::<Probe>().price,
                "explicit purchase exact visible price"
            );
            println!("[capture] PASS explicit purchase charges the displayed item price");
        }
        115 => {
            world.resource_mut::<GameSession>().kills = 0;
            interaction(world, "Market offer 3", Interaction::Pressed);
        }
        120 => interaction(world, "Market buy selected", Interaction::Pressed),
        125 => {
            assert_eq!(
                world.resource::<GameSession>().kills,
                0,
                "unaffordable purchase must not charge"
            );
            let buy = world
                .query::<(Entity, &Name)>()
                .iter(world)
                .find(|(_, n)| n.as_str() == "Market buy selected")
                .unwrap()
                .0;
            assert!(
                world
                    .entity(buy)
                    .contains::<rogue_commander::game::ControllerDisabled>()
            );
            println!("[capture] PASS unaffordable selection remains inspectable and cannot spend");
        }
        130 | 132 => {
            let pad = world.resource::<Probe>().pad.unwrap();
            world.write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                pad,
                GamepadButton::DPadRight,
                if frame == 130 { 1.0 } else { 0.0 },
            )));
        }
        135 | 137 => {
            let pad = world.resource::<Probe>().pad.unwrap();
            world.write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                pad,
                GamepadButton::DPadDown,
                if frame == 135 { 1.0 } else { 0.0 },
            )));
        }
        145 => {
            let focused = world
                .resource::<ControllerFocus>()
                .entity
                .expect("controller focus");
            assert_eq!(
                world.get::<Name>(focused).unwrap().as_str(),
                "Market offer 4"
            );
            assert!(
                !world
                    .entity(focused)
                    .contains::<rogue_commander::game::ControllerDisabled>()
            );
            assert_eq!(world.resource::<GameSession>().kills, 0);
            assert_eq!(
                world.resource::<GameSession>().elapsed,
                world.resource::<Probe>().opened_elapsed,
                "shop holds the battle clock"
            );
            super::market_wallet_capture::visible_controls(world);
            println!("[capture] PASS controller inspection remains in the shop scope");
        }
        151 => interaction(world, "Market close", Interaction::Pressed),
        155 => {
            assert!(!world.resource::<GameSession>().paused);
            assert!(
                world.resource::<GameSession>().elapsed > world.resource::<Probe>().opened_elapsed,
                "Close restarts the real battle clock"
            );
            assert!(
                world
                    .query::<(&Name, &Node)>()
                    .iter(world)
                    .any(|(n, node)| n.as_str() == "Market panel" && node.display == Display::None)
            );
            println!("[capture] PASS shop Close resumes the battle");
        }
        _ => {}
    }
}
