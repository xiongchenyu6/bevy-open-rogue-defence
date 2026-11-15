//! Campaign action tiles: recognizable pictograms, generous touch areas and shared hints.
use super::*;

#[derive(Component)]
pub(super) struct ActionTile(bool);

fn icon(action: Action) -> &'static str {
    match action {
        Action::Language => "ui/controls/language.png",
        Action::Training => "ui/controls/training.png",
        Action::Chapters => "ui/controls/chapters.png",
        Action::Start(_) => "ui/icons/skip.png",
        Action::Buy(_) => "ui/controls/training.png",
        Action::RetrySave => "ui/icons/confirm.png",
    }
}

pub(super) fn toolbar_button(
    parent: &mut ChildSpawnerCommands,
    assets: &AssetServer,
    name: &str,
    hint: &str,
    action: Action,
    badge: Option<&str>,
) {
    tile(parent, assets, name, hint, badge, action, false, true);
}

pub(super) fn button(
    parent: &mut ChildSpawnerCommands,
    assets: &AssetServer,
    name: impl Into<String>,
    copy: &str,
    action: Action,
    primary: bool,
) {
    tile(
        parent,
        assets,
        &name.into(),
        copy,
        Some(copy),
        action,
        primary,
        false,
    );
}

#[allow(clippy::too_many_arguments)]
fn tile(
    parent: &mut ChildSpawnerCommands,
    assets: &AssetServer,
    name: &str,
    hint: &str,
    copy: Option<&str>,
    action: Action,
    primary: bool,
    square: bool,
) {
    parent
        .spawn((
            Name::new(name.to_string()),
            Button,
            action,
            ActionTile(primary),
            Hint(hint.to_string()),
            Node {
                width: if square { px(48) } else { Val::Auto },
                min_width: px(48),
                min_height: px(48),
                padding: UiRect::axes(px(if square { 5 } else { 12 }), px(5)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(8)),
                column_gap: px(8),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_shrink: 0.,
                ..default()
            },
            BackgroundColor(if primary {
                Color::srgb(0.23, 0.17, 0.08)
            } else {
                Color::srgb(0.045, 0.075, 0.085)
            }),
            BorderColor::all(if primary {
                GOLD
            } else {
                Color::srgb(0.38, 0.33, 0.22)
            }),
        ))
        .with_children(|button| {
            button.spawn((
                ImageNode::new(assets.load(if name.contains("retry") {
                    "ui/icons/refresh.png"
                } else {
                    icon(action)
                })),
                Node {
                    width: px(30),
                    height: px(30),
                    flex_shrink: 0.,
                    ..default()
                },
                Pickable::IGNORE,
            ));
            if let Some(copy) = copy {
                button.spawn((
                    label(
                        copy,
                        if square { 9. } else { 13. },
                        if primary { GOLD } else { IVORY },
                    ),
                    Node {
                        position_type: if square {
                            PositionType::Absolute
                        } else {
                            PositionType::Relative
                        },
                        bottom: if square { px(2) } else { Val::Auto },
                        ..default()
                    },
                    TextLayout::justify(Justify::Center),
                    Pickable::IGNORE,
                ));
            }
        });
}

pub(super) fn feedback(
    mut tiles: Query<
        (
            &Interaction,
            &ActionTile,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        Changed<Interaction>,
    >,
) {
    for (interaction, tile, mut background, mut border) in &mut tiles {
        background.0 = match interaction {
            Interaction::Pressed => Color::srgb(0.34, 0.24, 0.10),
            Interaction::Hovered => Color::srgb(0.14, 0.19, 0.19),
            Interaction::None if tile.0 => Color::srgb(0.23, 0.17, 0.08),
            Interaction::None => Color::srgb(0.045, 0.075, 0.085),
        };
        *border = BorderColor::all(if tile.0 || *interaction != Interaction::None {
            GOLD
        } else {
            Color::srgb(0.38, 0.33, 0.22)
        });
    }
}
