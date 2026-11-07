use super::*;
use bevy::input::{
    InputPlugin,
    touch::{TouchInput, TouchPhase},
};
#[derive(Resource, Default)]
struct Results(Vec<usize>);
fn fixture() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, InputPlugin))
        .init_resource::<Results>();
    let root = app.world_mut().spawn_empty().id();
    app.add_systems(
        Update,
        move |touches: Res<Touches>,
              mut state: Local<DraftTouchChoice>,
              mut results: ResMut<Results>| {
            if let Some(index) = state.release_choice(root, Some(2), &touches) {
                results.0.push(index);
            }
        },
    );
    (app, root)
}
fn event(app: &mut App, window: Entity, phase: TouchPhase, position: Vec2) {
    app.world_mut().write_message(TouchInput {
        phase,
        position,
        window,
        force: None,
        id: 7,
    });
    app.update();
}
#[test]
fn scroll_card_touch_tap_settles_only_on_release() {
    let (mut app, window) = fixture();
    event(&mut app, window, TouchPhase::Started, Vec2::ZERO);
    assert!(app.world().resource::<Results>().0.is_empty());
    event(&mut app, window, TouchPhase::Ended, Vec2::ZERO);
    app.update();
    assert_eq!(app.world().resource::<Results>().0, [2]);
}
#[test]
fn scroll_card_drag_returning_to_start_never_selects() {
    let (mut app, window) = fixture();
    event(&mut app, window, TouchPhase::Started, Vec2::ZERO);
    event(&mut app, window, TouchPhase::Moved, Vec2::Y * 40.);
    event(&mut app, window, TouchPhase::Moved, Vec2::ZERO);
    event(&mut app, window, TouchPhase::Ended, Vec2::ZERO);
    assert!(app.world().resource::<Results>().0.is_empty());
}
#[test]
fn canceled_scroll_card_touch_allows_next_tap() {
    let (mut app, window) = fixture();
    event(&mut app, window, TouchPhase::Started, Vec2::ZERO);
    event(&mut app, window, TouchPhase::Canceled, Vec2::ZERO);
    assert!(app.world().resource::<Results>().0.is_empty());
    event(&mut app, window, TouchPhase::Started, Vec2::ZERO);
    event(&mut app, window, TouchPhase::Ended, Vec2::ZERO);
    assert_eq!(app.world().resource::<Results>().0, [2]);
}
