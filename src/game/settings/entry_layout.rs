//! Keep the Settings entry clear of recruitment and battle controls.
use super::{
    GameLanguage, GameSession, PrimaryWindow, SettingsLauncherLabel, SettingsOpen, UiSystems,
};
use bevy::prelude::*;

pub(super) fn install(app: &mut App) {
    app.add_systems(
        PostUpdate,
        position_picker_settings.before(UiSystems::Layout),
    );
}

#[derive(Debug, PartialEq)]
struct EntryLayout {
    left: Val,
    right: Val,
    top: Val,
    bottom: Val,
    margin: UiRect,
    width: Val,
    short_label: bool,
}

fn entry_layout(
    width: f32,
    height: f32,
    menu: bool,
    finished: bool,
    picker: bool,
    draft: bool,
) -> EntryLayout {
    let compact_picker = height < 540.0 && picker;
    let compact_draft = height < 540.0 && draft;
    let portrait = crate::game::mobile_layout::portrait_toolbar(width, height);
    let portrait_battle = portrait.is_some() && !menu && !finished && !picker && !draft;
    let right = if compact_draft {
        px(32)
    } else if compact_picker {
        px(24)
    } else if portrait_battle {
        px(portrait.unwrap().left_right)
    } else {
        Val::Auto
    };
    let anchored_right = right != Val::Auto;
    EntryLayout {
        left: if anchored_right {
            Val::Auto
        } else {
            percent(50)
        },
        right,
        top: if compact_draft {
            Val::Auto
        } else if portrait_battle {
            px(72)
        } else {
            px(8)
        },
        bottom: if compact_draft { px(24) } else { Val::Auto },
        margin: if anchored_right {
            UiRect::ZERO
        } else {
            UiRect::left(px(-40))
        },
        width: if portrait_battle { px(44) } else { px(80) },
        short_label: portrait_battle,
    }
}

// One context drives both placement and the launcher's only text writer.
#[allow(clippy::too_many_arguments)]
fn position_picker_settings(
    windows: Query<&Window, With<PrimaryWindow>>,
    picker: Query<Entity, With<crate::game::hero::HeroSelectorRoot>>,
    draft: Option<Res<crate::game::DraftState>>,
    menu: Res<crate::game::campaign_ui::CampaignMenu>,
    session: Res<GameSession>,
    language: Res<GameLanguage>,
    mut labels: Query<&mut Text, With<SettingsLauncherLabel>>,
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
    let picker = !picker.is_empty();
    let layout = entry_layout(
        window.width(),
        window.height(),
        menu.0,
        session.finished,
        picker,
        draft.is_some_and(|d| d.root.is_some()),
    );
    for mut label in &mut labels {
        label.set_if_neq(Text::new(if layout.short_label {
            language.pick("Menu", "菜单")
        } else {
            language.pick("Settings", "设置")
        }));
    }
    for (mut node, launcher, header) in &mut nodes {
        if launcher {
            node.reborrow()
                .map_unchanged(|n| &mut n.left)
                .set_if_neq(layout.left);
            node.reborrow()
                .map_unchanged(|n| &mut n.right)
                .set_if_neq(layout.right);
            node.reborrow()
                .map_unchanged(|n| &mut n.top)
                .set_if_neq(layout.top);
            node.reborrow()
                .map_unchanged(|n| &mut n.bottom)
                .set_if_neq(layout.bottom);
            node.reborrow()
                .map_unchanged(|n| &mut n.margin)
                .set_if_neq(layout.margin);
            node.reborrow()
                .map_unchanged(|n| &mut n.width)
                .set_if_neq(layout.width);
            node.reborrow()
                .map_unchanged(|n| &mut n.height)
                .set_if_neq(px(44));
        }
        if header {
            node.reborrow()
                .map_unchanged(|n| &mut n.padding.right)
                .set_if_neq(if window.height() < 540.0 && picker {
                    px(104)
                } else {
                    px(0)
                });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rect(layout: &EntryLayout, window_width: f32) -> Rect {
        let Val::Px(width) = layout.width else {
            panic!("pixel width required")
        };
        let left = match layout.right {
            Val::Px(right) => window_width - right - width,
            Val::Auto => window_width * 0.5 - 40.0,
            _ => panic!("unexpected horizontal anchor"),
        };
        let Val::Px(top) = layout.top else {
            panic!("top anchored context required")
        };
        Rect::from_corners(Vec2::new(left, top), Vec2::new(left + width, top + 44.0))
    }

    #[test]
    fn battle_launcher_rect_clears_pause_and_speed_at_supported_sizes() {
        for (width, height) in [
            (390.0, 844.0),
            (375.0, 667.0),
            (667.0, 375.0),
            (1280.0, 720.0),
        ] {
            let layout = entry_layout(width, height, false, false, false, false);
            let entry = rect(&layout, width);
            let hud = crate::game::mobile_layout::hud_metrics(width, height);
            let portrait = crate::game::mobile_layout::portrait_toolbar(width, height);
            let pause_right = if let Some(toolbar) = portrait {
                toolbar.right_right
            } else if hud.compact {
                hud.safe_x + 104.0
            } else {
                150.0
            };
            let pause = Rect::from_corners(
                Vec2::new(
                    width - pause_right - if portrait.is_some() { 44.0 } else { 52.0 },
                    if portrait.is_some() { 72.0 } else { hud.safe_y },
                ),
                Vec2::new(
                    width - pause_right,
                    if portrait.is_some() {
                        116.0
                    } else {
                        hud.safe_y + 48.0
                    },
                ),
            );
            assert!(
                entry.intersect(pause).is_empty(),
                "overlap at {width}: {entry:?} / {pause:?}"
            );
            if width < 560.0 {
                assert_eq!(entry.min, Vec2::new(width - hud.safe_x - 96.0, 72.0));
                assert_eq!(entry.size(), Vec2::splat(44.0));
                assert_eq!(pause.min.x - entry.max.x, 8.0);
                assert!(entry.max.y < 124.0, "must remain above the speed toolbar");
                assert!(layout.short_label);
            } else {
                assert_eq!(entry.size(), Vec2::new(80.0, 44.0));
                assert!(!layout.short_label);
            }
        }
    }

    #[test]
    fn menu_picker_draft_and_result_keep_full_settings_entry() {
        for (menu, finished, picker, draft) in [
            (true, false, false, false),
            (false, true, false, false),
            (false, false, true, false),
            (false, false, false, true),
        ] {
            let layout = entry_layout(390.0, 844.0, menu, finished, picker, draft);
            assert_eq!(layout.left, percent(50));
            assert_eq!(layout.width, px(80));
            assert_eq!(layout.top, px(8));
            assert!(!layout.short_label);
        }
        let picker = entry_layout(667.0, 375.0, false, false, true, false);
        assert_eq!(
            (picker.right, picker.top, picker.bottom),
            (px(24), px(8), Val::Auto)
        );
        let draft = entry_layout(667.0, 375.0, false, false, false, true);
        assert_eq!(
            (draft.right, draft.top, draft.bottom),
            (px(32), Val::Auto, px(24))
        );
        assert_eq!((picker.width, draft.width), (px(80), px(80)));
    }

    #[derive(Resource, Default)]
    struct Refreshes(usize, usize);

    #[test]
    fn launcher_context_has_one_localized_writer_without_idle_churn() {
        let mut app = App::new();
        app.init_resource::<GameLanguage>()
            .init_resource::<GameSession>()
            .insert_resource(crate::game::campaign_ui::CampaignMenu(false))
            .init_resource::<Refreshes>()
            .add_systems(PostUpdate, position_picker_settings)
            .add_systems(
                Last,
                |nodes: Query<(), (With<SettingsOpen>, Changed<Node>)>,
                 labels: Query<(), (With<SettingsLauncherLabel>, Changed<Text>)>,
                 mut refreshes: ResMut<Refreshes>| {
                    refreshes.0 = nodes.iter().count();
                    refreshes.1 = labels.iter().count();
                },
            );
        app.world_mut().spawn((
            Window {
                resolution: (390, 844).into(),
                ..default()
            },
            PrimaryWindow,
        ));
        app.world_mut().spawn((SettingsOpen, Node::default()));
        let label = app
            .world_mut()
            .spawn((SettingsLauncherLabel, Text::default()))
            .id();
        app.update();
        assert_eq!(app.world().get::<Text>(label).unwrap().0, "Menu");
        app.update();
        let refreshes = app.world().resource::<Refreshes>();
        assert_eq!((refreshes.0, refreshes.1), (0, 0));
        *app.world_mut().resource_mut::<GameLanguage>() = GameLanguage::SimplifiedChinese;
        app.update();
        assert_eq!(app.world().get::<Text>(label).unwrap().0, "菜单");
        app.world_mut()
            .resource_mut::<crate::game::campaign_ui::CampaignMenu>()
            .0 = true;
        app.update();
        assert_eq!(app.world().get::<Text>(label).unwrap().0, "设置");
        app.update();
        let refreshes = app.world().resource::<Refreshes>();
        assert_eq!((refreshes.0, refreshes.1), (0, 0));
    }

    #[test]
    fn picker_settings_repositions_on_resize_without_changing_portrait_space() {
        let mut app = App::new();
        app.init_resource::<GameLanguage>()
            .init_resource::<GameSession>()
            .init_resource::<crate::game::campaign_ui::CampaignMenu>()
            .add_systems(PostUpdate, position_picker_settings);
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
        app.insert_resource(crate::game::DraftState {
            root: Some(header),
            ..default()
        });
        app.update();
        let entry = app.world().get::<Node>(launcher).unwrap();
        assert_eq!(
            (entry.left, entry.right, entry.top, entry.bottom),
            (Val::Auto, px(32), Val::Auto, px(24))
        );
        app.world_mut()
            .resource_mut::<crate::game::DraftState>()
            .root = None;
        app.update();
        assert_eq!(app.world().get::<Node>(launcher).unwrap().top, px(8));
    }
}
