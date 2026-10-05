//! Exercise real speed controls and the live hero sheet at desktop/mobile sizes.
use super::CaptureControls;
use bevy::prelude::*;
use rogue_commander::game::{
    Autoplay, GameSession, HeroStatsSnapshot, HeroVitals, PlayerBuild, SelectedHero, SpawnDirector,
};

#[derive(Resource, Default)]
struct AttributesCapture {
    frame: u32,
    paused_elapsed: Option<f32>,
}

pub fn configure(world: &mut World) {
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SelectedHero>().locked = true;
    world.resource_mut::<CaptureControls>().suppress_drafts = true;
    world.resource_mut::<SpawnDirector>().enabled = false;
    let mut session = world.resource_mut::<GameSession>();
    session.elapsed = 20.0;
    session.auto_bond_draws = false;
    println!(
        "[capture] hero-attributes: actual +/-/reset controls, live stats sheet, pause/close, no ultimate HUD"
    );
}

pub fn install(app: &mut bevy::app::SubApp) {
    app.world_mut().remove_resource::<Autoplay>();
    app.init_resource::<AttributesCapture>().add_systems(
        PreUpdate,
        exercise
            .after(bevy::ui::UiSystems::Focus)
            .before(rogue_commander::game::SettingsInputSet),
    );
}

#[allow(clippy::too_many_arguments)]
fn exercise(
    mut state: ResMut<AttributesCapture>,
    mut buttons: Query<(&Name, &mut Interaction), With<Button>>,
    mut keyboard: ResMut<ButtonInput<KeyCode>>,
    clock: Res<Time<Virtual>>,
    session: Res<GameSession>,
    stats: Res<HeroStatsSnapshot>,
    vitals: Res<HeroVitals>,
    build: Res<PlayerBuild>,
    names: Query<&Name>,
    texts: Query<(&Name, &Text)>,
    layout: Query<(&Name, &ComputedNode, &UiGlobalTransform)>,
    windows: Query<&Window>,
) {
    state.frame += 1;
    let action = match state.frame {
        5 | 12 | 19 | 26 | 33 | 40 => Some("Speed increase"),
        55 | 75 => Some("Speed decrease"),
        65 => Some("Speed reset"),
        95 | 195 => Some("Hero attributes open"),
        180 => Some("Hero attributes close"),
        _ => None,
    };
    for (_, mut interaction) in &mut buttons {
        *interaction = Interaction::None;
    }
    if let Some(action) = action {
        let (_, mut interaction) = buttons
            .iter_mut()
            .find(|(name, _)| name.as_str() == action)
            .unwrap_or_else(|| panic!("missing actual control: {action}"));
        *interaction = Interaction::Pressed;
    }
    keyboard.release(KeyCode::Escape);
    keyboard.clear();
    if state.frame == 205 {
        keyboard.press(KeyCode::Escape);
    }
    let expected = match state.frame {
        8 => Some(2.0),
        15 => Some(4.0),
        22 => Some(8.0),
        29 => Some(16.0),
        36 | 45 => Some(32.0),
        58 => Some(16.0),
        68 | 78 => Some(1.0),
        _ => None,
    };
    if let Some(expected) = expected {
        assert_eq!(
            clock.relative_speed(),
            expected,
            "real virtual time multiplier"
        );
        println!("[capture] actual speed control -> {expected}x");
    }
    if matches!(state.frame, 45 | 100 | 140 | 175 | 210) {
        assert!(
            !names
                .iter()
                .any(|name| name.as_str() == "Automatic ultimate status"),
            "removed ultimate HUD must not exist"
        );
        let window = windows.single().unwrap();
        let viewport = Vec2::new(
            window.physical_width() as f32,
            window.physical_height() as f32,
        );
        for name in [
            "Speed decrease",
            "Speed reset",
            "Speed increase",
            "Hero attributes open",
        ] {
            assert_in_viewport(name, viewport, &layout);
        }
        if matches!(state.frame, 100 | 140 | 175) {
            assert_in_viewport("Hero attributes panel", viewport, &layout);
            assert_in_viewport("Hero attributes close", viewport, &layout);
            let mut label_count = 0;
            for (name, text) in &texts {
                if !name.as_str().starts_with("Hero stat label ") {
                    continue;
                }
                assert!(!text.0.trim().is_empty(), "attribute label text missing");
                // Portrait sheets scroll vertically; landscape shows every row.
                if viewport.x >= 600.0 {
                    assert_in_viewport(name.as_str(), viewport, &layout);
                }
                let (_, node, _) = layout
                    .iter()
                    .find(|(label, _, _)| label.as_str() == name.as_str())
                    .unwrap();
                assert!(
                    node.size().x >= 40.0 && node.size().y >= 10.0,
                    "attribute label collapsed: {} {:?}",
                    name.as_str(),
                    node.size()
                );
                label_count += 1;
            }
            assert_eq!(
                label_count, 22,
                "all attribute field labels must be visible"
            );
            assert!(session.paused, "sheet freezes the battle");
            if let Some(elapsed) = state.paused_elapsed {
                assert_eq!(
                    session.elapsed, elapsed,
                    "open sheet does not advance combat"
                );
            } else {
                state.paused_elapsed = Some(session.elapsed);
            }
            assert_eq!(stats.strength, vitals.strength);
            assert_eq!(stats.agility, vitals.agility);
            assert_eq!(stats.intelligence, vitals.intelligence);
            assert_eq!(stats.health, vitals.health);
            assert_eq!(stats.attack_damage, build.attack_damage);
            assert_stat_text("strength", stats.strength, &texts);
            assert_stat_text("attacks_per_second", stats.attacks_per_second, &texts);
            assert_stat_text("life_leech", stats.life_leech * 100.0, &texts);
            assert_stat_text("physical_penetration", stats.physical_penetration, &texts);
            assert_stat_text("spell_penetration", stats.spell_penetration * 100.0, &texts);
            println!(
                "[capture] live sheet values agree with authoritative snapshot and hero; paused; panel fits viewport"
            );
        }
    }
    if state.frame == 185 {
        assert!(!session.paused);
        assert!(
            !names
                .iter()
                .any(|name| name.as_str() == "Hero attributes panel")
        );
        assert!(session.elapsed > state.paused_elapsed.unwrap());
        println!("[capture] real close button restores running battle");
    }
    if state.frame == 200 {
        assert!(session.paused);
    }
    if state.frame == 210 {
        assert!(
            !session.paused,
            "Esc closes sheet without toggling manual pause"
        );
        assert!(
            !names
                .iter()
                .any(|name| name.as_str() == "Hero attributes panel")
        );
        println!(
            "[capture] PASS: 1→2→4→8→16→32, upper/lower clamps, halve/reset; live sheet open/close/Esc; no ultimate HUD"
        );
    }
}

fn assert_in_viewport(
    name: &str,
    viewport: Vec2,
    layout: &Query<(&Name, &ComputedNode, &UiGlobalTransform)>,
) {
    let (_, node, transform) = layout
        .iter()
        .find(|(actual, _, _)| actual.as_str() == name)
        .unwrap_or_else(|| panic!("missing UI node: {name}"));
    let min = transform.translation - node.size() * 0.5;
    let max = transform.translation + node.size() * 0.5;
    assert!(
        node.size().min_element() > 0.0,
        "{name}: nonempty UI bounds"
    );
    assert!(
        min.cmpge(Vec2::splat(-1.0)).all() && max.cmple(viewport + Vec2::ONE).all(),
        "{name} outside viewport: {min:?}..{max:?} vs{viewport:?}"
    );
}

fn assert_stat_text(field: &str, expected: f32, texts: &Query<(&Name, &Text)>) {
    let name = format!("Hero stat {field}");
    let (_, text) = texts
        .iter()
        .find(|(actual, _)| actual.as_str() == name)
        .unwrap_or_else(|| panic!("missing stat value node: {name}"));
    let value = text
        .0
        .trim()
        .trim_end_matches('%')
        .replace(',', "")
        .parse::<f32>()
        .unwrap_or_else(|_| panic!("stat value is not numeric: {}", text.0));
    assert!(
        (value - expected).abs() <= 0.11,
        "{field}: UI {value} != actual {expected}"
    );
}
