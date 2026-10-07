//! Keep Settings reachable beside compact commander recruitment.
use super::{PrimaryWindow, SettingsOpen, UiSystems};
use bevy::prelude::*;

pub(super) fn install(app: &mut App) {
    app.add_systems(
        PostUpdate,
        position_picker_settings.before(UiSystems::Layout),
    );
}

// Keep the commander's heading and the always-available Settings entry disjoint.
fn position_picker_settings(
    windows: Query<&Window, With<PrimaryWindow>>,
    picker: Query<Entity, With<crate::game::hero::HeroSelectorRoot>>,
    mut nodes: Query<
        (
            &mut Node,
            Has<SettingsOpen>,
            Has<crate::game::hero::draft::DraftHeader>,
        ),
        Or<(
            With<SettingsOpen>,
            With<crate::game::hero::draft::DraftHeader>,
        )>,
    >,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let compact_picker = window.height() < 540.0 && !picker.is_empty();
    for (mut node, launcher, header) in &mut nodes {
        if launcher {
            node.reborrow()
                .map_unchanged(|n| &mut n.left)
                .set_if_neq(if compact_picker {
                    Val::Auto
                } else {
                    percent(50)
                });
            node.reborrow()
                .map_unchanged(|n| &mut n.right)
                .set_if_neq(if compact_picker { px(24) } else { Val::Auto });
            node.reborrow()
                .map_unchanged(|n| &mut n.margin)
                .set_if_neq(if compact_picker {
                    UiRect::ZERO
                } else {
                    UiRect::left(px(-40))
                });
        }
        if header {
            node.reborrow()
                .map_unchanged(|n| &mut n.padding.right)
                .set_if_neq(if compact_picker { px(104) } else { px(0) });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn picker_settings_repositions_on_resize_without_changing_portrait_space() {
        let mut app = App::new();
        app.add_systems(PostUpdate, position_picker_settings);
        let window = app
            .world_mut()
            .spawn((
                Window {
                    resolution: (667, 375).into(),
                    ..default()
                },
                PrimaryWindow,
            ))
            .id();
        let launcher = app
            .world_mut()
            .spawn((
                SettingsOpen,
                Node {
                    left: percent(50),
                    margin: UiRect::left(px(-40)),
                    ..default()
                },
            ))
            .id();
        let picker = app
            .world_mut()
            .spawn(crate::game::hero::HeroSelectorRoot)
            .id();
        let header = app
            .world_mut()
            .spawn((
                crate::game::hero::draft::DraftHeader,
                Node {
                    height: px(52),
                    ..default()
                },
            ))
            .id();
        for (width, height) in [(667, 375), (480, 360), (1280, 720), (390, 844)] {
            app.world_mut()
                .get_mut::<Window>(window)
                .unwrap()
                .resolution
                .set(width as f32, height as f32);
            app.update();
            let entry = app.world().get::<Node>(launcher).unwrap();
            let heading = app.world().get::<Node>(header).unwrap();
            assert_eq!(heading.height, px(52));
            if height < 540 {
                assert_eq!(
                    (entry.left, entry.right, entry.margin),
                    (Val::Auto, px(24), UiRect::ZERO)
                );
                assert_eq!(heading.padding.right, px(104));
            } else {
                assert_eq!((entry.left, entry.right), (percent(50), Val::Auto));
                assert_eq!(heading.padding.right, px(0));
            }
        }
        app.world_mut().despawn(picker);
        app.world_mut()
            .get_mut::<Window>(window)
            .unwrap()
            .resolution
            .set(480., 360.);
        app.update();
        assert_eq!(app.world().get::<Node>(launcher).unwrap().left, percent(50));
    }
}
