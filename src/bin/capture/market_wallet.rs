//! Paused market purchase presentation; fixture economy, real stock and handlers.
use bevy::{prelude::*, ui::UiSystems};
use rogue_commander::game::{
    AttackClock, Autoplay, GameSession, PlayerBuild, SelectedHero, SpawnDirector,
};
#[derive(Resource, Default)]
struct WalletCapture {
    frame: u32,
    before: u32,
    after: u32,
    price: u32,
    elapsed: f32,
}
pub fn configure(world: &mut World) {
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SelectedHero>().locked = true;
    world
        .resource_mut::<super::CaptureControls>()
        .suppress_drafts = true;
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
}
pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<PlayerBuild>().skills.fill(0);
    world.resource_mut::<AttackClock>().0.pause();
    world.resource_mut::<GameSession>().elapsed = 181.0;
    world.resource_mut::<GameSession>().kills = 100_000;
    app.init_resource::<WalletCapture>().add_systems(
        PreUpdate,
        exercise
            .after(UiSystems::Focus)
            .before(rogue_commander::game::SettingsInputSet),
    );
}
fn wallet(world: &mut World) -> String {
    world
        .query::<(&Name, &Text)>()
        .iter(world)
        .find(|(name, _)| name.as_str() == "Market wallet")
        .expect("real market wallet text")
        .1
        .0
        .clone()
}
fn press(world: &mut World, wanted: &str) {
    let mut found = false;
    for (name, mut interaction) in world.query::<(&Name, &mut Interaction)>().iter_mut(world) {
        *interaction = if name.as_str() == wanted {
            found = true;
            Interaction::Pressed
        } else {
            Interaction::None
        };
    }
    assert!(found, "actual market purchase exists");
}
fn layer_and_owner(world: &World, mut entity: Entity, panel: Entity) -> (i32, bool) {
    let mut layer = None;
    let mut belongs = false;
    loop {
        belongs |= entity == panel;
        if layer.is_none() {
            layer = world.get::<GlobalZIndex>(entity).map(|z| z.0);
        }
        let Some(parent) = world.get::<ChildOf>(entity) else {
            break;
        };
        entity = parent.parent();
    }
    (layer.unwrap_or(0), belongs)
}
fn overlap(a: (Vec2, Vec2), b: (Vec2, Vec2)) -> bool {
    a.0.x < b.1.x && a.1.x > b.0.x && a.0.y < b.1.y && a.1.y > b.0.y
}
fn visible_controls(world: &mut World) {
    let window = world.query::<&Window>().single(world).unwrap();
    let viewport = Vec2::new(
        window.physical_width() as f32,
        window.physical_height() as f32,
    );
    let mut offers = 0;
    let mut autos = 0;
    let mut refresh = false;
    let mut wallet_bottom = 0.0_f32;
    let mut first_offer_top = f32::INFINITY;
    let mut content = Vec::new();
    let mut toggle = None;
    let mut panel = None;
    for (entity, name, node, transform) in world
        .query::<(Entity, &Name, &ComputedNode, &UiGlobalTransform)>()
        .iter(world)
    {
        let rect = (
            transform.translation - node.size() * 0.5,
            transform.translation + node.size() * 0.5,
        );
        if name.as_str() == "Market panel" {
            panel = Some((entity, rect));
        }
        if name.as_str() == "Market toggle" {
            toggle = Some(rect);
        }
        let is_offer = name.as_str().starts_with("Market offer ");
        let is_auto = name.as_str().starts_with("Market auto ");
        let is_refresh = name.as_str() == "Market refresh";
        let is_wallet = name.as_str() == "Market wallet";
        if !is_offer && !is_auto && !is_refresh && !is_wallet {
            continue;
        }
        assert!(node.size().min_element() > 0.0);
        assert!(
            rect.0.x >= -0.5
                && rect.0.y >= -0.5
                && rect.1.x <= viewport.x + 0.5
                && rect.1.y <= viewport.y + 0.5,
            "{} clipped: {rect:?}, viewport={viewport:?}",
            name.as_str()
        );
        if is_offer {
            offers += 1;
            first_offer_top = first_offer_top.min(rect.0.y);
        }
        if is_auto {
            autos += 1;
        }
        if is_refresh {
            refresh = true;
            assert!(node.size().y >= 44.0);
        }
        if is_wallet {
            wallet_bottom = rect.1.y;
        }
        content.push((name.as_str().to_owned(), rect));
    }
    let (panel, panel_rect) = panel.expect("actual panel rectangle");
    let panel_z = world.get::<GlobalZIndex>(panel).unwrap().0;
    assert!(
        !overlap(panel_rect, toggle.expect("actual close toggle")),
        "close toggle must remain accessible outside the panel"
    );
    for (entity, node, computed, transform, background, text, image, name) in world
        .query::<(
            Entity,
            &Node,
            &ComputedNode,
            &UiGlobalTransform,
            Option<&BackgroundColor>,
            Option<&Text>,
            Option<&ImageNode>,
            Option<&Name>,
        )>()
        .iter(world)
    {
        if node.display == Display::None || computed.size().min_element() <= 0.0 {
            continue;
        }
        let painted = background.is_some_and(|color| color.0.to_srgba().alpha > 0.0)
            || text.is_some_and(|text| !text.0.is_empty())
            || image.is_some_and(|image| image.color.to_srgba().alpha > 0.0);
        if !painted {
            continue;
        }
        let (layer, own) = layer_and_owner(world, entity, panel);
        if own {
            continue;
        }
        let rect = (
            transform.translation - computed.size() * 0.5,
            transform.translation + computed.size() * 0.5,
        );
        for (control, control_rect) in &content {
            if overlap(rect, *control_rect) {
                assert!(
                    panel_z > layer,
                    "foreign painted node covers {control}: {name:?}, inherited layer={layer}, panel={panel_z}"
                );
            }
        }
    }
    assert_eq!(offers, 5);
    assert_eq!(autos, 5);
    assert!(refresh);
    assert!(
        wallet_bottom <= first_offer_top,
        "feedback must not overlap offers"
    );
}

fn exercise(world: &mut World) {
    world.resource_mut::<WalletCapture>().frame += 1;
    let frame = world.resource::<WalletCapture>().frame;
    world
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::KeyB);
    world.resource_mut::<ButtonInput<KeyCode>>().clear();
    for mut interaction in world.query::<&mut Interaction>().iter_mut(world) {
        *interaction = Interaction::None;
    }
    // An actual camp hover reveals its inherited-layer caption beside shop offers.
    // This catches camp80 occlusion that wallet-only bounds previously missed.
    if (50..=145).contains(&frame) {
        for (name, mut interaction) in world.query::<(&Name, &mut Interaction)>().iter_mut(world) {
            if name.as_str() == "Adventure camp entry 0" {
                *interaction = Interaction::Hovered;
            }
        }
    }
    if (175..=190).contains(&frame) {
        for (name, mut interaction) in world.query::<(&Name, &mut Interaction)>().iter_mut(world) {
            if name.as_str() == "Market toggle" {
                *interaction = Interaction::Hovered;
            }
        }
    }
    match frame {
        20 => world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyB),
        35 => {
            assert!(world.resource::<GameSession>().paused);
            let before = world.resource::<GameSession>().kills;
            let elapsed = world.resource::<GameSession>().elapsed;
            let offer = world
                .query::<(&Name, &Children)>()
                .iter(world)
                .find(|(name, _)| name.as_str() == "Market offer 0")
                .unwrap()
                .1[0];
            let label = &world.get::<Text>(offer).unwrap().0;
            let price: u32 = label
                .lines()
                .next()
                .unwrap()
                .split_whitespace()
                .rev()
                .nth(1)
                .unwrap()
                .parse()
                .unwrap();
            let mut state = world.resource_mut::<WalletCapture>();
            state.before = before;
            state.elapsed = elapsed;
            state.price = price;
        }
        45 => press(world, "Market offer 0"),
        65 => {
            let session = world.resource::<GameSession>();
            let after = session.kills;
            let state = world.resource::<WalletCapture>();
            assert_eq!(
                state.before - after,
                state.price,
                "actual purchase charges real visible price"
            );
            assert!(session.paused);
            assert_eq!(session.elapsed, state.elapsed);
            world.resource_mut::<WalletCapture>().after = after;
            let label = wallet(world);
            assert!(
                label.contains(&after.to_string()),
                "remaining wallet survives feedback: {label}"
            );
            assert!(label.contains("RESTOCK") || label.contains("STOCK") || label.contains("补货"));
            assert!(label.contains("Bought") || label.contains("已购买"));
            visible_controls(world);
            println!(
                "[capture] PASS paused purchase wallet and restock retained, all offers/refresh fit: {label}"
            );
        }
        145 => {
            let session = world.resource::<GameSession>();
            let state = world.resource::<WalletCapture>();
            assert!(session.paused);
            assert_eq!(session.elapsed, state.elapsed);
            assert_eq!(session.kills, state.after);
            let after = state.after;
            let label = wallet(world);
            assert!(label.contains(&after.to_string()));
            assert!(label.contains("RESTOCK") || label.contains("STOCK") || label.contains("补货"));
            assert!(!label.contains("Bought") && !label.contains("已购买"));
            visible_controls(world);
            println!(
                "[capture] PASS real-time feedback expired while battle stayed paused: {label}"
            );
        }
        165 => world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyB),
        185 => {
            assert!(
                !world.resource::<GameSession>().paused,
                "actual close toggle resumes battle"
            );
            let hidden = world
                .query::<(&Name, &Node)>()
                .iter(world)
                .any(|(name, node)| {
                    name.as_str() == "Market panel" && node.display == Display::None
                });
            let hint = world
                .query::<(&Name, &Node, &ComputedNode)>()
                .iter(world)
                .any(|(name, node, computed)| {
                    name.as_str() == "Market hint"
                        && node.display == Display::Flex
                        && computed.size().min_element() > 0.0
                });
            assert!(
                hidden && hint,
                "closing the high-layer panel must preserve its hover hint"
            );
            println!(
                "[capture] PASS accessible close toggle, battle resumed, closed-market hover hint visible"
            );
        }
        _ => {}
    }
}
