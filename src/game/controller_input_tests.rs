use super::*;
use bevy::input::gamepad::GamepadAxis;
fn world() -> World {
    let mut world = World::new();
    world.init_resource::<ControllerInput>();
    world.init_resource::<ControllerFocus>();
    world.init_resource::<Time<Real>>();
    world.init_resource::<super::super::ControlScheme>();
    world.init_resource::<GameSession>();
    world
}
fn control(world: &mut World, parent: Entity, name: &str, x: f32) -> Entity {
    world
        .spawn((
            Button,
            Name::new(name.to_owned()),
            ChildOf(parent),
            ComputedNode {
                size: Vec2::splat(40.),
                ..default()
            },
            UiGlobalTransform::from_xy(x, 100.),
        ))
        .id()
}
fn pad(world: &mut World) -> Entity {
    world.spawn(Gamepad::default()).id()
}
fn press(world: &mut World, pad: Entity, button: GamepadButton) {
    world
        .get_mut::<Gamepad>(pad)
        .unwrap()
        .digital_mut()
        .press(button);
}
fn clear(world: &mut World, pad: Entity) {
    world.get_mut::<Gamepad>(pad).unwrap().digital_mut().clear();
}
#[test]
fn held_confirm_cannot_purchase_twice_or_select_rebuilt_next_card() {
    let mut world = world();
    let root = world.spawn((ControllerScope(150), Name::new("Cards"))).id();
    let first = control(&mut world, root, "Choice 0", 100.);
    let pad = pad(&mut world);
    press(&mut world, pad, GamepadButton::South);
    controller(&mut world);
    assert_eq!(
        *world.get::<Interaction>(first).unwrap(),
        Interaction::Pressed
    );
    clear(&mut world, pad);
    world.entity_mut(root).despawn();
    let root = world.spawn((ControllerScope(150), Name::new("Cards"))).id();
    let next = control(&mut world, root, "Choice 0", 100.);
    controller(&mut world);
    assert_ne!(
        *world.get::<Interaction>(next).unwrap(),
        Interaction::Pressed
    );
    assert!(world.get::<ControllerFocused>(next).is_some());
    assert!(!world.resource::<ControllerInput>().dodge);
}
#[test]
fn topmost_confirmation_owns_back_without_battle_pause() {
    let mut world = world();
    let settings = world
        .spawn((ControllerScope(500), Name::new("Settings")))
        .id();
    let close = control(&mut world, settings, "Settings close", 100.);
    let prompt = world
        .spawn((ControllerScope(530), Name::new("Confirmation")))
        .id();
    let cancel = control(&mut world, prompt, "Settings departure cancel", 100.);
    let pad = pad(&mut world);
    press(&mut world, pad, GamepadButton::East);
    controller(&mut world);
    assert_eq!(
        *world.get::<Interaction>(cancel).unwrap(),
        Interaction::Pressed
    );
    assert_eq!(*world.get::<Interaction>(close).unwrap(), Interaction::None);
    assert!(!world.resource::<ControllerInput>().pause);
    clear(&mut world, pad);
    world.entity_mut(prompt).despawn();
    controller(&mut world);
    assert_ne!(
        *world.get::<Interaction>(close).unwrap(),
        Interaction::Pressed
    );
}
#[test]
fn loading_scope_with_no_buttons_blocks_underlying_chapter_activation() {
    let mut world = world();
    let chapter = world.spawn(ControllerScope(400)).id();
    let start = control(&mut world, chapter, "Start", 100.);
    world.spawn(ControllerScope(1000));
    let pad = pad(&mut world);
    press(&mut world, pad, GamepadButton::South);
    controller(&mut world);
    assert_eq!(*world.get::<Interaction>(start).unwrap(), Interaction::None);
    assert!(world.resource::<ControllerFocus>().entity.is_none());
}
#[test]
fn focus_survives_rebuild_and_excludes_hidden_and_disabled_buttons() {
    let mut world = world();
    let root = world
        .spawn((ControllerScope(400), Name::new("Chapters")))
        .id();
    let hidden = control(&mut world, root, "Hidden", 0.);
    world.entity_mut(hidden).insert(Node {
        display: Display::None,
        ..default()
    });
    let disabled = control(&mut world, root, "Locked", 30.);
    world.entity_mut(disabled).insert(ControllerDisabled);
    let play = control(&mut world, root, "Playable", 60.);
    let pad = pad(&mut world);
    press(&mut world, pad, GamepadButton::North);
    controller(&mut world);
    assert_eq!(world.resource::<ControllerFocus>().entity, Some(play));
    let key = world.resource::<ControllerFocus>().key.clone();
    clear(&mut world, pad);
    world.entity_mut(root).despawn();
    let root = world
        .spawn((ControllerScope(400), Name::new("Chapters")))
        .id();
    let play = control(&mut world, root, "Playable", 60.);
    controller(&mut world);
    assert_eq!(world.resource::<ControllerFocus>().key, key);
    assert_eq!(world.resource::<ControllerFocus>().entity, Some(play));
}
#[test]
fn pointer_takeover_keeps_the_real_click_after_ui_focus() {
    let mut world = world();
    let root = world
        .spawn((ControllerScope(400), Name::new("Chapters")))
        .id();
    let play = control(&mut world, root, "Playable", 60.);
    let pad = pad(&mut world);
    press(&mut world, pad, GamepadButton::North);
    controller(&mut world);
    clear(&mut world, pad);
    *world.resource_mut::<super::super::ControlScheme>() = super::super::ControlScheme::Keyboard;
    *world.get_mut::<Interaction>(play).unwrap() = Interaction::Pressed;
    controller(&mut world);
    assert_eq!(
        *world.get::<Interaction>(play).unwrap(),
        Interaction::Pressed
    );
    assert_eq!(world.get::<Outline>(play).unwrap().color, Color::NONE);
    assert!(world.resource::<ControllerFocus>().entity.is_none());
}
#[test]
fn a_new_controller_button_takes_over_without_old_held_stick_stealing_it() {
    let mut world = world();
    let first = pad(&mut world);
    let second = pad(&mut world);
    world
        .get_mut::<Gamepad>(first)
        .unwrap()
        .analog_mut()
        .set(GamepadAxis::LeftStickX, 0.8);
    controller(&mut world);
    assert_eq!(world.resource::<ControllerInput>().active, Some(first));
    press(&mut world, second, GamepadButton::North);
    controller(&mut world);
    clear(&mut world, second);
    controller(&mut world);
    assert_eq!(world.resource::<ControllerInput>().active, Some(second));
    assert_eq!(world.resource::<ControllerInput>().movement, Vec2::ZERO);
}
#[test]
fn analog_movement_dodge_and_disconnect_clear_transient_input() {
    let mut world = world();
    let pad = pad(&mut world);
    world
        .get_mut::<Gamepad>(pad)
        .unwrap()
        .analog_mut()
        .set(GamepadAxis::LeftStickY, 0.6);
    press(&mut world, pad, GamepadButton::South);
    controller(&mut world);
    assert!((world.resource::<ControllerInput>().movement.y + 0.5).abs() < 0.001);
    assert!(world.resource::<ControllerInput>().dodge);
    clear(&mut world, pad);
    controller(&mut world);
    assert!(!world.resource::<ControllerInput>().dodge);
    world.entity_mut(pad).despawn();
    controller(&mut world);
    assert_eq!(world.resource::<ControllerInput>().movement, Vec2::ZERO);
    assert!(world.resource::<ControllerInput>().pause);
}

#[test]
fn navigation_reveals_focus_and_right_stick_scrolls_the_list() {
    let mut world = world();
    let root = world
        .spawn((ControllerScope(280), Name::new("Equipment")))
        .id();
    let list = world
        .spawn((
            ChildOf(root),
            Name::new("List"),
            ScrollPosition::default(),
            ComputedNode {
                size: Vec2::new(200., 100.),
                ..default()
            },
            UiGlobalTransform::from_xy(100., 50.),
        ))
        .id();
    let visible = control(&mut world, list, "Item 0", 100.);
    let lower = control(&mut world, list, "Item 1", 100.);
    world
        .entity_mut(visible)
        .insert(UiGlobalTransform::from_xy(100., 30.));
    world
        .entity_mut(lower)
        .insert(UiGlobalTransform::from_xy(100., 160.));
    let pad = pad(&mut world);
    press(&mut world, pad, GamepadButton::North);
    controller(&mut world);
    clear(&mut world, pad);
    world
        .get_mut::<Gamepad>(pad)
        .unwrap()
        .analog_mut()
        .set(GamepadButton::DPadDown, 1.);
    controller(&mut world);
    assert_eq!(world.resource::<ControllerFocus>().entity, Some(lower));
    assert!((world.get::<ScrollPosition>(list).unwrap().y - 80.).abs() < 0.001);
    world
        .get_mut::<Gamepad>(pad)
        .unwrap()
        .analog_mut()
        .set(GamepadButton::DPadDown, 0.);
    world
        .get_mut::<Gamepad>(pad)
        .unwrap()
        .analog_mut()
        .set(GamepadAxis::RightStickY, -1.);
    world
        .resource_mut::<Time<Real>>()
        .advance_by(std::time::Duration::from_secs_f32(0.1));
    controller(&mut world);
    assert!((world.get::<ScrollPosition>(list).unwrap().y - 130.).abs() < 0.001);
}

#[test]
fn menu_repeat_and_scroll_ignore_virtual_speed_and_pause() {
    fn run(speed: f32, paused: bool) -> (String, f32) {
        let mut world = world();
        world.init_resource::<Time>();
        world.init_resource::<Time<Virtual>>();
        world
            .resource_mut::<Time<Virtual>>()
            .set_relative_speed(speed);
        if paused {
            world.resource_mut::<Time<Virtual>>().pause();
        }
        let root = world
            .spawn((ControllerScope(280), Name::new("Equipment")))
            .id();
        let list = world
            .spawn((
                ChildOf(root),
                Name::new("List"),
                ScrollPosition::default(),
                ComputedNode {
                    size: Vec2::new(400., 200.),
                    ..default()
                },
                UiGlobalTransform::from_xy(200., 100.),
            ))
            .id();
        for (index, x) in [100., 160., 220.].into_iter().enumerate() {
            control(&mut world, list, &format!("Item {index}"), x);
        }
        let pad = pad(&mut world);
        press(&mut world, pad, GamepadButton::North);
        controller(&mut world);
        clear(&mut world, pad);
        world
            .get_mut::<Gamepad>(pad)
            .unwrap()
            .analog_mut()
            .set(GamepadButton::DPadRight, 1.);
        world
            .get_mut::<Gamepad>(pad)
            .unwrap()
            .analog_mut()
            .set(GamepadAxis::RightStickY, -1.);
        controller(&mut world);
        assert!(world.resource::<ControllerFocus>().key.ends_with("Item 1"));
        for (delta, expected) in [(0.2, "Item 1"), (0.16, "Item 2")] {
            world
                .resource_mut::<Time<Real>>()
                .advance_by(std::time::Duration::from_secs_f32(delta));
            let real = *world.resource::<Time<Real>>();
            let mut current = world.remove_resource::<Time>().unwrap();
            let mut virtual_time = world.remove_resource::<Time<Virtual>>().unwrap();
            bevy::time::update_virtual_time(&mut current, &mut virtual_time, &real);
            world.insert_resource(current);
            world.insert_resource(virtual_time);
            controller(&mut world);
            assert!(
                world.resource::<ControllerFocus>().key.ends_with(expected),
                "repeat uses real time at {speed}×, paused={paused}"
            );
        }
        let virtual_elapsed = world.resource::<Time<Virtual>>().elapsed_secs();
        assert!((virtual_elapsed - if paused { 0. } else { 0.36 * speed }).abs() < 0.001);
        (
            world.resource::<ControllerFocus>().key.clone(),
            world.get::<ScrollPosition>(list).unwrap().y,
        )
    }
    let normal = run(1., false);
    assert_eq!(normal, run(32., false));
    assert_eq!(normal, run(32., true));
    assert!((normal.1 - 180.).abs() < 0.001);
}

#[test]
fn modal_starts_on_its_own_first_choice_before_global_settings_entry() {
    let mut world = world();
    let root = world
        .spawn((ControllerScope(100), Name::new("Recruitment")))
        .id();
    let choice = control(&mut world, root, "Commander card 0", 100.);
    world.spawn((
        Button,
        Name::new("Settings open"),
        ComputedNode {
            size: Vec2::splat(40.),
            ..default()
        },
        UiGlobalTransform::from_xy(100., 0.),
    ));
    let pad = pad(&mut world);
    press(&mut world, pad, GamepadButton::North);
    controller(&mut world);
    assert_eq!(world.resource::<ControllerFocus>().entity, Some(choice));
}
