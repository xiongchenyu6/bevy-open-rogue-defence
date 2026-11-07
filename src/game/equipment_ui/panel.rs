use super::*;

pub(super) fn panel(
    commands: &mut Commands,
    assets: &AssetServer,
    language: GameLanguage,
    size: Vec2,
    inventory: (&[MarketItem], &[RecipeId], bool),
) -> Entity {
    let (owned, crafted, crafting_tab) = inventory;
    let compact = crate::game::mobile_layout::hud_metrics(size.x, size.y).compact;
    let portrait = size.x < 560.0;
    let short = size.y < 540.0;
    let width = (size.x - 24.0).min(880.0);
    let height = (size.y - 24.0).min(if portrait { 760.0 } else { 560.0 });
    let padding = if compact { 8.0 } else { 12.0 };
    let grid_width = if compact {
        (width - padding * 2.0) * 0.38
    } else {
        320.0
    };
    let preview_size = if short { 48.0 } else if compact { 88.0 } else { 176.0 };
    let empty = !crafting_tab && owned.is_empty() && crafted.is_empty();
    let preview_box = if empty {
        if compact { Vec2::new(110.0, 73.0) } else { Vec2::new(220.0, 147.0) }
    } else {
        Vec2::splat(preview_size)
    };
    commands.spawn((
        Name::new("Equipment panel"),
        Node {
            position_type: PositionType::Absolute,
            width: percent(100), height: percent(100),
            align_items: AlignItems::Center, justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.82)),
        GlobalZIndex(280), crate::game::controller_input::ControllerScope(280),
    )).with_children(|overlay| {
        overlay.spawn((
            Node {
                width: px(width), height: px(height), flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(padding)), row_gap: px(8),
                border: UiRect::all(px(1)), ..default()
            }, BackgroundColor(PANEL), BorderColor::all(GOLD),
        )).with_children(|body| {
            header(body, language, owned.len() + crafted.len(), crafting_tab, compact);
            body.spawn(Node {
                width: percent(100), min_height: px(0), flex_grow: 1.0,
                flex_direction: if portrait { FlexDirection::Column } else { FlexDirection::Row },
                column_gap: px(12), row_gap: px(8), ..default()
            }).with_children(|content| {
                content.spawn((
                    Grid, Name::new("Equipment grid scroll"), ScrollPosition::default(),
                    Node {
                        width: if portrait { percent(100) } else { px(grid_width) },
                        height: if portrait { px((height * 0.25).clamp(68.0, 132.0)) } else { percent(100) },
                        min_height: px(0), min_width: px(0), flex_shrink: 0.0,
                        overflow: Overflow::scroll_y(), ..default()
                    },
                )).with_children(|scroll| {
                    collection(scroll, assets, language, owned, crafted, crafting_tab, compact);
                });
                content.spawn((
                    Name::new("Equipment selected item"),
                    Node {
                        flex_grow: 1.0, min_width: px(0), min_height: px(0),
                        width: if portrait { percent(100) } else { Val::Auto },
                        flex_direction: FlexDirection::Column, row_gap: px(if short { 6.0 } else { 8.0 }),
                        padding: UiRect::all(px(if short { 4.0 } else if compact { 6.0 } else { 10.0 })),
                        border: UiRect::all(px(1)), ..default()
                    },
                    BackgroundColor(Color::srgb(0.045, 0.07, 0.085)),
                    BorderColor::all(Color::srgba(0.90, 0.72, 0.42, 0.35)),
                )).with_children(|selected| {
                    selected.spawn(Node {
                        width: percent(100), flex_shrink: 0.0, align_items: AlignItems::Center,
                        column_gap: px(if compact { 8.0 } else { 14.0 }), ..default()
                    }).with_children(|showcase| {
                        showcase.spawn(Node {
                            width: px(preview_box.x), height: px(preview_box.y), flex_shrink: 0.0,
                            align_items: AlignItems::Center, justify_content: JustifyContent::Center,
                            ..default()
                        }).with_children(|preview| {
                            if empty {
                                preview.spawn((
                                    EmptyArmory, Name::new("Equipment empty armory"),
                                    ImageNode {
                                        color: Color::srgba(1.0, 1.0, 1.0, 0.85),
                                        ..ImageNode::new(assets.load(EMPTY_ARMORY_IMAGE))
                                    },
                                    Node {
                                        display: Display::None, width: percent(100),
                                        height: percent(100), ..default()
                                    },
                                    Pickable::IGNORE,
                                ));
                            } else {
                                preview.spawn((
                                    SelectedArt { max: preview_size }, Name::new("Equipment selected art"),
                                    ImageNode::default(), Node { display: Display::None, ..default() },
                                    Pickable::IGNORE,
                                ));
                            }
                        });
                        showcase.spawn(Node {
                            flex_grow: 1.0, min_width: px(0), flex_direction: FlexDirection::Column,
                            row_gap: px(8), ..default()
                        }).with_children(|identity| {
                            identity.spawn((
                                DetailName, Name::new("Equipment detail name"), Text::new(""),
                                TextFont { font_size: FontSize::Px(if compact { 16.0 } else { 20.0 }), ..default() },
                                TextColor(GOLD), TextLayout::linebreak(LineBreak::WordOrCharacter),
                            ));
                        });
                    });
                    if crafting_tab {
                        materials::spawn(selected, compact, short);
                    }
                    selected.spawn((
                        DetailScroll, Name::new("Equipment detail scroll"), ScrollPosition::default(),
                        crate::game::controller_input::ControllerDetailScroll, Interaction::default(),
                        Node {
                            width: percent(100), min_height: px(if short { 40.0 } else { 0.0 }), flex_grow: 1.0,
                            overflow: Overflow::scroll_y(), flex_direction: FlexDirection::Column,
                            padding: UiRect::top(px(8)), border: UiRect::top(px(1)), ..default()
                        }, BorderColor::all(Color::srgba(0.90, 0.72, 0.42, 0.35)),
                    )).with_children(|details| {
                        details.spawn((
                            DetailEffect, Name::new("Equipment detail effect"), Text::new(""),
                            TextFont { font_size: FontSize::Px(14.0), ..default() },
                            TextColor(IVORY), TextLayout::linebreak(LineBreak::WordOrCharacter),
                        ));

                    });
                    if crafting_tab {
                        selected.spawn((
                            Button, ConfirmCraft, Name::new("Crafting confirm"),
                            Node {
                                width: percent(100), min_height: px(44), flex_shrink: 0.0,
                                align_items: AlignItems::Center, justify_content: JustifyContent::Center,
                                ..default()
                            }, BackgroundColor(PANEL),
                        )).with_child((
                            CraftAction, Name::new("Crafting result"), Text::new(""),
                            TextFont { font_size: FontSize::Px(14.0), ..default() },
                            TextColor(GOLD), Pickable::IGNORE,
                        ));
                    }
                });
            });
            body.spawn((
                Text::new(language.pick(
                    "All owned equipment stays active. Inspect an icon; scroll details to read more.\nNo slot limit. Forge consumes materials and inherits all their effects.",
                    "所有已持有装备同时生效，查看图标；滚动详情查看全部效果。\n无槽限制。合成消耗材料并继承其全部效果。")),
                Node { flex_shrink: 0.0, ..default() },
                TextFont { font_size: FontSize::Px(12.0), ..default() }, TextColor(IVORY),
            ));
        });
    }).id()
}

fn header(
    body: &mut ChildSpawnerCommands,
    language: GameLanguage,
    count: usize,
    crafting_tab: bool,
    compact: bool,
) {
    body.spawn(Node {
        width: percent(100),
        flex_shrink: 0.0,
        align_items: AlignItems::Center,
        ..default()
    })
    .with_children(|header| {
        header.spawn((
            Text::new(format!("{} · {count}", language.pick("EQUIPMENT", "装备"))),
            Node {
                flex_grow: 1.0,
                min_width: px(0),
                ..default()
            },
            TextFont {
                font_size: FontSize::Px(if compact { 16.0 } else { 20.0 }),
                ..default()
            },
            TextColor(IVORY),
        ));
        for (tab, title, name) in [
            (
                false,
                language.pick("Gear", "装备"),
                "Equipment inventory tab",
            ),
            (
                true,
                language.pick("Forge", "合成"),
                "Equipment crafting tab",
            ),
        ] {
            header
                .spawn((
                    Button,
                    Tab(tab),
                    Name::new(name),
                    Node {
                        min_width: px(54),
                        height: px(44),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        margin: UiRect::right(px(4)),
                        border: UiRect::all(px(1)),
                        ..default()
                    },
                    BackgroundColor(if crafting_tab == tab {
                        Color::srgb(0.22, 0.19, 0.10)
                    } else {
                        PANEL
                    }),
                    BorderColor::all(GOLD),
                ))
                .with_child((
                    Text::new(title),
                    TextFont {
                        font_size: FontSize::Px(13.0),
                        ..default()
                    },
                    TextColor(IVORY),
                    Pickable::IGNORE,
                ));
        }
        header
            .spawn((
                Button,
                Close,
                Name::new("Equipment close"),
                Node {
                    min_width: px(60),
                    height: px(44),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border: UiRect::all(px(1)),
                    ..default()
                },
                BackgroundColor(PANEL),
                BorderColor::all(GOLD),
            ))
            .with_child((
                Text::new(language.pick("Close", "关闭")),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(IVORY),
                Pickable::IGNORE,
            ));
    });
}

fn collection(
    scroll: &mut ChildSpawnerCommands,
    assets: &AssetServer,
    language: GameLanguage,
    owned: &[MarketItem],
    crafted: &[RecipeId],
    crafting_tab: bool,
    compact: bool,
) {
    let icon = if compact {
        52.0
    } else if crafting_tab {
        96.0
    } else {
        88.0
    };
    scroll.spawn(Node {
        width: percent(100), flex_wrap: FlexWrap::Wrap, column_gap: px(8), row_gap: px(8),
        align_content: AlignContent::FlexStart, ..default()
    }).with_children(|grid| {
        if crafting_tab {
            for recipe in RecipeId::ALL {
                grid.spawn((
                    Button, Recipe(recipe), Name::new(format!("Crafting recipe {}", recipe.index())),
                    card_node(compact, true), BackgroundColor(PANEL),
                    BorderColor::all(crate::game::rarity::rarity_color(recipe.rarity())),
                )).with_children(|card| {
                    card_content(card, recipe.image_node(assets), recipe.art_size(icon),
                        recipe.rarity(), (!compact).then_some(recipe.name(language)));
                    materials::badge(card, assets, recipe);
                });
            }
            return;
        }
        if owned.is_empty() && crafted.is_empty() {
            grid.spawn((
                Text::new(language.pick(
                    "No equipment yet.\nVisit the Black Market (B) to buy equipment with kill tokens.",
                    "暂无装备。\n前往黑市（B），用战功购买装备。")),
                Node { width: percent(100), padding: UiRect::all(px(12)), ..default() },
                TextFont { font_size: FontSize::Px(16.0), ..default() }, TextColor(IVORY),
            ));
        }
        for (index, item) in owned.iter().copied().enumerate() {
            grid.spawn((
                Button, Item(item), Name::new(format!("Equipment item {index}")),
                card_node(compact, false), BackgroundColor(PANEL),
                BorderColor::all(crate::game::rarity::rarity_color(item.build_rarity())),
            )).with_children(|card| card_content(card, item.image_node(assets),
                item.art_size(icon), item.build_rarity(), None));
        }
        for (index, recipe) in crafted.iter().copied().enumerate() {
            grid.spawn((
                Button, Crafted(recipe), Name::new(format!("Equipment crafted item {index}")),
                card_node(compact, false), BackgroundColor(PANEL),
                BorderColor::all(crate::game::rarity::rarity_color(recipe.rarity())),
            )).with_children(|card| card_content(card, recipe.image_node(assets),
                recipe.art_size(icon), recipe.rarity(), None));
        }
    });
}

fn card_node(compact: bool, recipe: bool) -> Node {
    let (width, height) = if compact {
        (60.0, 60.0)
    } else if recipe {
        (148.0, 124.0)
    } else {
        (96.0, 96.0)
    };
    Node {
        width: px(width),
        height: px(height),
        flex_shrink: 0.0,
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        row_gap: px(2),
        padding: UiRect::all(px(3)),
        border: UiRect::all(px(1)),
        ..default()
    }
}

fn card_content(
    card: &mut ChildSpawnerCommands,
    image: ImageNode,
    size: Vec2,
    rarity: crate::game::content::Rarity,
    caption: Option<&str>,
) {
    card.spawn((
        image,
        Node {
            width: px(size.x),
            height: px(size.y),
            flex_shrink: 0.0,
            ..default()
        },
        Pickable::IGNORE,
    ));
    card.spawn((
        Text::new(rarity.label()),
        Node {
            position_type: PositionType::Absolute,
            left: px(3),
            top: px(2),
            ..default()
        },
        TextFont {
            font_size: FontSize::Px(10.0),
            ..default()
        },
        TextColor(crate::game::rarity::rarity_color(rarity)),
        Pickable::IGNORE,
    ));
    if let Some(caption) = caption {
        card.spawn((
            Text::new(caption),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(IVORY),
            Pickable::IGNORE,
        ));
    }
}
