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
        **text = if preference.0 {
            language.pick("Detailed · switch to simple", "详细 · 切换简洁")
        } else {
            language.pick("Simple · switch to detailed", "简洁 · 切换详细")
        }
        .into();
    }
}
