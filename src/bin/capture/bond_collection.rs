//! Exercise the real collection controls, all families, and pause ownership.
use super::CaptureControls;
use bevy::{
    input::{
        mouse::{MouseScrollUnit, MouseWheel},
        touch::TouchPhase,
    },
    prelude::*,
    ui::RelativeCursorPosition,
};
use rogue_commander::game::{BondId, GameSession, PlayerBuild, SelectedHero, SpawnDirector};
#[derive(Resource, Default)]
struct State {
    frame: u32,
    paused_at: f32,
}
pub fn configure(world: &mut World) {
    world.remove_resource::<rogue_commander::game::Autoplay>();
    world.resource_mut::<SelectedHero>().locked = true;
    world.resource_mut::<CaptureControls>().suppress_drafts = true;
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
    for card in BondId::Journey.definition().members {
        world.resource_mut::<PlayerBuild>().owned_bond_cards[card.index()] = true;
    }
}
pub fn install(app: &mut bevy::app::SubApp) {
    app.world_mut().insert_resource(State::default());
    app.add_systems(
        PreUpdate,
        drive
            .after(bevy::ui::UiSystems::Focus)
            .before(rogue_commander::game::SettingsInputSet),
    )
    .add_systems(Last, verify);
}
fn drive(
    mut state: ResMut<State>,
    mut buttons: Query<(&Name, &mut Interaction)>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut cursors: Query<(&Name, &mut RelativeCursorPosition)>,
    mut wheel: MessageWriter<MouseWheel>,
    windows: Query<Entity, With<Window>>,
) {
    state.frame += 1;
    let name = match state.frame {
        20 => Some("Bond collection open"),
        45 => Some("Bond collection family cultivation"),
        80 | 165 => Some("Bond collection family spirit_energy"),
        _ => None,
    };
    for (label, mut interaction) in &mut buttons {
        if name == Some(label.as_str()) {
            *interaction = Interaction::Pressed;
        } else if *interaction == Interaction::Pressed {
            *interaction = Interaction::None;
        }
    }
    if matches!(state.frame, 145 | 185 | 195) {
        keys.press(KeyCode::KeyH);
    }
    if matches!(state.frame, 146 | 186 | 196) {
        keys.release(KeyCode::KeyH);
    }
    if matches!(state.frame, 115 | 170) {
        for (name, mut cursor) in &mut cursors {
            cursor.cursor_over = name.as_str() == "Bond collection details";
            cursor.normalized = Some(Vec2::splat(0.5));
        }
        wheel.write(MouseWheel {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y: -200.0,
            window: windows.single().unwrap(),
            phase: TouchPhase::Moved,
        });
    }
    if state.frame == 130 {
        keys.press(KeyCode::Escape);
    }
    if state.frame == 131 {
        keys.release(KeyCode::Escape);
    }
}
fn verify(
    mut state: ResMut<State>,
    session: Res<GameSession>,
    nodes: Query<(&Name, &ComputedNode)>,
    layout: Query<(&Name, &ComputedNode, &UiGlobalTransform)>,
    scrolls: Query<(&Name, &ScrollPosition)>,
) {
    if state.frame == 25 {
        assert!(session.paused, "collection must pause active combat");
        state.paused_at = session.elapsed;
        assert_eq!(
            nodes
                .iter()
                .filter(|(n, _)| n.as_str().starts_with("Bond collection family "))
                .count(),
            BondId::COUNT
        );
    }
    if state.frame == 60 {
        assert!(
            nodes
                .iter()
                .any(|(n, _)| n.as_str() == "Bond collection card virtual_heaven_cauldron")
        );
        println!("[capture] PASS collection shows Mortal Cultivation card data");
    }
    if state.frame == 110 {
        assert!(session.paused);
        assert_eq!(
            session.elapsed, state.paused_at,
            "collection reading freezes simulation"
        );
        let cards = nodes
            .iter()
            .filter(|(n, _)| n.as_str().starts_with("Bond collection card "))
            .count();
        assert_eq!(
            cards, 12,
            "Spirit Energy has all twelve adapted cards in the collection"
        );
        let (_, panel) = nodes
            .iter()
            .find(|(n, _)| n.as_str() == "Bond collection panel")
            .unwrap();
        assert!(panel.size().x > 0.0 && panel.size().y > 0.0);
        println!(
            "[capture] PASS collection: 26 families, 12 Spirit Energy cards, paused, measured {:?}",
            panel.size()
        );
    }
    if matches!(state.frame, 120 | 175) {
        let (_, card, transform) = layout
            .iter()
            .find(|(n, _, _)| n.as_str() == "Bond collection card spirit_unity")
            .expect("last Spirit Energy card");
        let (_, view, view_transform) = layout
            .iter()
            .find(|(n, _, _)| n.as_str() == "Bond collection details")
            .unwrap();
        let card_min = transform.translation - card.size() * 0.5;
        let card_max = transform.translation + card.size() * 0.5;
        let view_min = view_transform.translation - view.size() * 0.5;
        let view_max = view_transform.translation + view.size() * 0.5;
        assert!(
            card_min.y >= view_min.y - 1.0 && card_max.y <= view_max.y + 1.0,
            "last card must be fully visible after real wheel scrolling: {card_min:?}..{card_max:?} inside {view_min:?}..{view_max:?}"
        );
        let (_, offset) = scrolls
            .iter()
            .find(|(n, _)| n.as_str() == "Bond collection details")
            .unwrap();
        assert!(offset.0.y > 0.0);
        println!(
            "[capture] PASS collection wheel reaches final spirit_unity card; offset {:.1}",
            offset.0.y
        );
    }
    if matches!(state.frame, 148 | 198) {
        assert!(
            session.paused
                && nodes
                    .iter()
                    .any(|(n, _)| n.as_str() == "Bond collection overlay")
        );
        println!("[capture] PASS H opens collection and pauses combat");
    }
    if state.frame == 188 {
        assert!(
            !session.paused
                && !nodes
                    .iter()
                    .any(|(n, _)| n.as_str() == "Bond collection overlay")
        );
        println!("[capture] PASS H closes collection and restores combat");
    }
    if state.frame == 132 {
        println!("[capture] checking Escape restoration");
        assert!(
            !session.paused,
            "Escape closes collection and resumes its pause"
        );
        assert!(
            !nodes
                .iter()
                .any(|(n, _)| n.as_str() == "Bond collection overlay")
        );
        println!("[capture] PASS Escape closes collection and restores combat");
    }
}
