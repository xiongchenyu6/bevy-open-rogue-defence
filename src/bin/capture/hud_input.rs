//! Verify actual touch/key events select controls independently of viewport size.
use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
        touch::{TouchInput, TouchPhase},
    },
    prelude::*,
};
use rogue_commander::game::{ControlScheme, MoveIntent};
#[derive(Resource, Default)]
struct Step(u32);
pub fn install(app: &mut bevy::app::SubApp) {
    app.init_resource::<Step>()
        .add_systems(First, input)
        .add_systems(Last, verify);
}
fn input(
    mut step: ResMut<Step>,
    windows: Query<(Entity, &Window)>,
    mut touch: MessageWriter<TouchInput>,
    mut keyboard: MessageWriter<KeyboardInput>,
) {
    step.0 += 1;
    let (window, size) = windows.single().unwrap();
    if step.0 == 70 || step.0 == 110 {
        touch.write(TouchInput {
            phase: if step.0 == 70 {
                TouchPhase::Started
            } else {
                TouchPhase::Ended
            },
            position: Vec2::new(size.width() * 0.5, size.height() * 0.5),
            window,
            force: None,
            id: 91,
        });
    }
    if step.0 == 140 || step.0 == 160 {
        keyboard.write(KeyboardInput {
            key_code: KeyCode::ArrowRight,
            logical_key: Key::ArrowRight,
            state: if step.0 == 140 {
                ButtonState::Pressed
            } else {
                ButtonState::Released
            },
            text: None,
            repeat: false,
            window,
        });
    }
}
fn verify(
    step: Res<Step>,
    scheme: Res<ControlScheme>,
    movement: Res<MoveIntent>,
    nodes: Query<(&Name, &Node, &ComputedNode, &UiGlobalTransform)>,
    windows: Query<&Window>,
) {
    if ![40, 95, 125, 150, 180].contains(&step.0) {
        return;
    }
    let expected_touch = (70..140).contains(&step.0);
    assert_eq!(scheme.touch, expected_touch);
    let (_, pad, _, _) = nodes
        .iter()
        .find(|(name, _, _, _)| name.as_str() == "Movement pad")
        .expect("named movement pad");
    assert_eq!(
        pad.display != Display::None,
        expected_touch,
        "keyboard mode must never display joystick, even at phone resolution"
    );
    let (_, _, dodge, position) = nodes
        .iter()
        .find(|(name, _, _, _)| name.as_str() == "Dodge control")
        .unwrap();
    assert!(dodge.size().min_element() > 0.);
    if !expected_touch {
        assert!(
            dodge.size().y <= 48.,
            "desktop dodge is a compact key control"
        );
        let (_, _, readout, readout_position) = nodes
            .iter()
            .find(|(name, _, _, _)| name.as_str() == "XP readout")
            .expect("named XP readout");
        assert!(
            (readout_position.translation.y - position.translation.y).abs() <= 2.0,
            "desktop XP and dodge must share a center line"
        );
        assert!(
            readout.size().y <= 30.0,
            "desktop XP readout must not stretch vertically"
        );
        let (_, _, bar, bar_position) = nodes
            .iter()
            .find(|(name, _, _, _)| name.as_str() == "Battle bottom controls")
            .unwrap();
        assert!(
            (bar.size().y - 40.0).abs() <= 1.0,
            "desktop control bar has explicit 40px height"
        );
        let bottom_gap =
            windows.single().unwrap().height() - (bar_position.translation.y + bar.size().y * 0.5);
        assert!(
            (0.0..=28.0).contains(&bottom_gap),
            "desktop bar must hug bottom safe area: gap={bottom_gap}"
        );
    }
    let window = windows.single().unwrap();
    let min = position.translation - dodge.size() * 0.5;
    let max = position.translation + dodge.size() * 0.5;
    assert!(
        min.cmpge(Vec2::splat(-1.)).all()
            && max
                .cmple(Vec2::new(window.width() + 1., window.height() + 1.))
                .all()
    );
    if step.0 == 150 {
        assert!(movement.0.x > 0., "keyboard movement still works");
    }
    println!(
        "[capture] PASS input layout frame {}: touch={}, pad={}, dodge={:?}",
        step.0,
        scheme.touch,
        pad.display != Display::None,
        dodge.size()
    );
}
