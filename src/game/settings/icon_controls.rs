//! Authored control glyphs and touch scrolling for the settings sheet.
use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
};

#[derive(Component)]
pub(super) struct ControlIcon(pub &'static str);
#[derive(Component)]
pub(super) struct SettingsScroll;

pub(super) fn icon(parent: &mut ChildSpawnerCommands, name: &'static str, size: f32) {
    parent.spawn((
        ControlIcon(name),
        Node {
            width: px(size),
            height: px(size),
            flex_shrink: 0.,
            ..default()
        },
        Pickable::IGNORE,
    ));
}
pub(super) fn install(app: &mut App) {
    app.add_systems(Update, (load_icons, scroll));
}
fn load_icons(
    mut commands: Commands,
    assets: Res<AssetServer>,
    icons: Query<(Entity, &ControlIcon), Without<ImageNode>>,
) {
    for (entity, icon) in &icons {
        commands.entity(entity).insert(ImageNode::new(
            assets.load(format!("ui/controls/{}.png", icon.0)),
        ));
    }
}
fn max_scroll(node: &ComputedNode) -> f32 {
    ((node.content_size().y - node.size().y + node.scrollbar_size.y).max(0.))
        * node.inverse_scale_factor()
}

fn bounded_scroll(current: f32, delta: f32, node: &ComputedNode) -> f32 {
    (current + delta).clamp(0., max_scroll(node))
}

fn scroll(
    mut wheel: MessageReader<MouseWheel>,
    touches: Res<Touches>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut bodies: Query<
        (&ComputedNode, &UiGlobalTransform, &mut ScrollPosition),
        With<SettingsScroll>,
    >,
) {
    let wheel_delta = wheel
        .read()
        .map(|event| {
            -event.y
                * if event.unit == MouseScrollUnit::Line {
                    28.
                } else {
                    1.
                }
        })
        .sum::<f32>();
    let cursor = windows.single().ok().and_then(Window::cursor_position);
    for (node, transform, mut position) in &mut bodies {
        // UI geometry is physical pixels; touch/cursor positions and ScrollPosition are logical.
        let scale = node.inverse_scale_factor();
        let bounds = Rect::from_center_size(transform.translation * scale, node.size() * scale);
        let delta = touches
            .iter()
            .filter(|touch| bounds.contains(touch.start_position()))
            .map(|touch| -touch.delta().y)
            .sum::<f32>()
            + if cursor.is_some_and(|cursor| bounds.contains(cursor)) {
                wheel_delta
            } else {
                0.
            };
        // Also clamp an old offset after resize/content changes without another input event.
        let next = Vec2::new(0., bounded_scroll(position.0.y, delta, node));
        position
            .reborrow()
            .map_unchanged(|position| &mut position.0)
            .set_if_neq(next);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scroll_clamps_both_edges_and_tracks_logical_content_extent() {
        let mut node = ComputedNode {
            size: Vec2::new(600., 400.),
            content_size: Vec2::new(600., 1000.),
            inverse_scale_factor: 0.5,
            ..default()
        };
        assert_eq!(bounded_scroll(0., 10000., &node), 300.);
        assert_eq!(bounded_scroll(300., -10000., &node), 0.);
        node.content_size.y = 200.;
        assert_eq!(
            bounded_scroll(300., 0., &node),
            0.,
            "shrinking content cannot leave an empty overscrolled sheet"
        );
    }
}
