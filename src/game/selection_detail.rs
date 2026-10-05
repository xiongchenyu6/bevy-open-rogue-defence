//! One remembered selection preference, shared by drafts and settings.
use super::GameLanguage;
use bevy::prelude::*;

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionDetail(pub bool);
impl Default for SelectionDetail {
    fn default() -> Self {
        Self(crate::preferences::load_selection_detail().unwrap_or(true))
    }
}
#[derive(Component)]
pub(crate) struct SelectionDetailToggle;
#[derive(Component)]
pub(crate) struct SelectionDetailLabel;

pub(crate) fn spawn_toggle(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Button,
            SelectionDetailToggle,
            Name::new("Selection detail toggle"),
            Node {
                min_height: px(36),
                padding: UiRect::axes(px(10), px(6)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(Color::srgb(0.055, 0.085, 0.095)),
            BorderColor::all(Color::srgb(0.90, 0.72, 0.42)),
        ))
        .with_children(|button| {
            button.spawn((
                SelectionDetailLabel,
                Text::default(),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::srgb(0.96, 0.93, 0.85)),
            ));
        });
}
pub(crate) fn toggle(
    buttons: Query<&Interaction, (With<SelectionDetailToggle>, Changed<Interaction>)>,
    mut preference: ResMut<SelectionDetail>,
) {
    if buttons
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        preference.0 = !preference.0;
        if let Err(error) = crate::preferences::save_selection_detail(preference.0) {
            warn!("Could not save selection detail preference: {error}");
        }
    }
}
pub(crate) fn update_labels(
    preference: Res<SelectionDetail>,
    language: Res<GameLanguage>,
    mut labels: Query<&mut Text, With<SelectionDetailLabel>>,
) {
    for mut text in &mut labels {
        text.set_if_neq(Text(
            if preference.0 {
                language.pick("Detailed · switch to simple", "详细 · 切换简洁")
            } else {
                language.pick("Simple · switch to detailed", "简洁 · 切换详细")
            }
            .into(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detail_mode_labels_only_change_with_their_displayed_content() {
        use crate::game::settings::{TextRefreshProbe, count_text_refreshes};
        let mut app = App::new();
        app.insert_resource(SelectionDetail(true))
            .init_resource::<GameLanguage>()
            .init_resource::<TextRefreshProbe>()
            .add_systems(Update, update_labels)
            .add_systems(Last, count_text_refreshes);
        let label = app
            .world_mut()
            .spawn((SelectionDetailLabel, Text::default()))
            .id();
        app.update();
        app.update();
        assert_eq!(app.world().resource::<TextRefreshProbe>().0, 0);
        app.world_mut().resource_mut::<SelectionDetail>().0 = false;
        app.update();
        assert_eq!(app.world().resource::<TextRefreshProbe>().0, 1);
        assert_eq!(
            app.world().get::<Text>(label).unwrap().0,
            "Simple · switch to detailed"
        );
        app.world_mut()
            .insert_resource(GameLanguage::SimplifiedChinese);
        app.update();
        assert_eq!(app.world().resource::<TextRefreshProbe>().0, 1);
        assert_eq!(app.world().get::<Text>(label).unwrap().0, "简洁 · 切换详细");
        app.update();
        assert_eq!(app.world().resource::<TextRefreshProbe>().0, 0);
    }
}
