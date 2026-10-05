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
    let width = (size.x - 24.0).min(880.0);
    let height = (size.y - 24.0).min(560.0);
    let icon = if compact { 52.0 } else { 70.0 };
    commands.spawn((Name::new("Equipment panel"), Node { position_type: PositionType::Absolute,
        width: percent(100), height: percent(100), align_items: AlignItems::Center,
        justify_content: JustifyContent::Center, ..default() },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.82)), GlobalZIndex(45)))
    .with_children(|overlay| {
        overlay.spawn((Node { width: px(width), height: px(height), flex_direction: FlexDirection::Column,
            padding: UiRect::all(px(12)), row_gap: px(8), border: UiRect::all(px(1)), ..default()
        }, BackgroundColor(PANEL), BorderColor::all(GOLD))).with_children(|body| {
            body.spawn(Node { width: percent(100), flex_shrink: 0.0, align_items: AlignItems::Center, ..default() })
            .with_children(|header| {
                header.spawn((Text::new(format!("{} · {}", language.pick("EQUIPMENT", "装备"), owned.len() + crafted.len())),
                    Node { flex_grow: 1.0, ..default() }, TextFont { font_size: FontSize::Px(20.0), ..default() }, TextColor(IVORY)));
                for (tab, title, name) in [(false, language.pick("Gear", "装备"), "Equipment inventory tab"), (true, language.pick("Forge", "合成"), "Equipment crafting tab")] {
                    header.spawn((Button, Tab(tab), Name::new(name), Node {
                        min_width: px(54), height: px(44), align_items: AlignItems::Center, justify_content: JustifyContent::Center,
                        margin: UiRect::right(px(4)), border: UiRect::all(px(1)), ..default()
                    }, BackgroundColor(if crafting_tab == tab { Color::srgb(0.22, 0.19, 0.10) } else { PANEL }), BorderColor::all(GOLD)))
                    .with_child((Text::new(title), TextFont { font_size: FontSize::Px(13.0), ..default() }, TextColor(IVORY), Pickable::IGNORE));
                }
                header.spawn((Button, Close, Name::new("Equipment close"), Node {
                    min_width: px(60), height: px(44), align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center, border: UiRect::all(px(1)), ..default()
                }, BackgroundColor(PANEL), BorderColor::all(GOLD))).with_child((Text::new(language.pick("Close", "关闭")),
                    TextFont { font_size: FontSize::Px(14.0), ..default() }, TextColor(IVORY), Pickable::IGNORE));
            });
            body.spawn((Grid, Name::new("Equipment grid scroll"), ScrollPosition::default(),
                Node { width: percent(100), min_height: px(0), flex_grow: 1.0, overflow: Overflow::scroll_y(), ..default() }))
            .with_children(|scroll| {
                scroll.spawn(Node { width: percent(100), flex_wrap: FlexWrap::Wrap,
                    column_gap: px(8), row_gap: px(8), align_content: AlignContent::FlexStart, ..default() })
                .with_children(|grid| {
                    if crafting_tab {
                        for recipe in RecipeId::ALL {
                            grid.spawn((Button, Recipe(recipe), Name::new(format!("Crafting recipe {}", recipe.index())),
                                Node { width: px(icon + 8.0), height: px(icon + 8.0), padding: UiRect::all(px(3)), border: UiRect::all(px(1)), ..default() },
                                BackgroundColor(PANEL), BorderColor::all(crate::game::rarity::rarity_color(recipe.rarity()))))
                            .with_children(|card| {
                                card.spawn((ImageNode::new(assets.load(recipe.icon())), Node { width: percent(100), height: percent(100), ..default() }, Pickable::IGNORE));
                                card.spawn((Text::new(recipe.rarity().label()), Node { position_type: PositionType::Absolute, left: px(2), top: px(2), ..default() },
                                    TextFont { font_size: FontSize::Px(10.0), ..default() }, TextColor(GOLD), Pickable::IGNORE));
                            });
                        }
                    } else {
                    if owned.is_empty() && crafted.is_empty() {
                        grid.spawn((Text::new(language.pick("No equipment yet.\nVisit the Black Market (B) to buy equipment with kill tokens.", "暂无装备。\n前往黑市（B），用战功购买装备。")),
                            Node { width: percent(100), padding: UiRect::all(px(12)), ..default() },
                            TextFont { font_size: FontSize::Px(16.0), ..default() }, TextColor(IVORY)));
                    }
                    for (index, item) in owned.iter().copied().enumerate() {
                        grid.spawn((Button, Item(item), Name::new(format!("Equipment item {index}")),
                            Node { width: px(icon + 8.0), height: px(icon + 8.0),
                                padding: UiRect::all(px(3)), border: UiRect::all(px(1)), ..default() },
                            BackgroundColor(Color::srgb(0.07, 0.09, 0.105)),
                            BorderColor::all(crate::game::rarity::rarity_color(item.build_rarity()))))
                        .with_children(|card| {
                            card.spawn((ImageNode::new(assets.load(item.icon_path())), Node {
                                width: percent(100), height: percent(100), ..default()
                            }, Pickable::IGNORE));
                            card.spawn((Text::new(item.build_rarity().label()), Node { position_type: PositionType::Absolute,
                                left: px(3), top: px(2), ..default() }, TextFont { font_size: FontSize::Px(10.0), ..default() },
                                TextColor(crate::game::rarity::rarity_color(item.build_rarity())), Pickable::IGNORE));
                        });
                    }
                    for (index, recipe) in crafted.iter().copied().enumerate() {
                        grid.spawn((Button, Crafted(recipe), Name::new(format!("Equipment crafted item {index}")),
                            Node { width: px(icon + 8.0), height: px(icon + 8.0), padding: UiRect::all(px(3)), border: UiRect::all(px(2)), ..default() },
                            BackgroundColor(PANEL), BorderColor::all(crate::game::rarity::rarity_color(recipe.rarity()))))
                        .with_child((ImageNode::new(assets.load(recipe.icon())), Node { width: percent(100), height: percent(100), ..default() }, Pickable::IGNORE));
                    }
                    }
                });
            });
            body.spawn((DetailScroll, Name::new("Equipment detail scroll"), ScrollPosition::default(), Interaction::default(), Node { width: percent(100), flex_shrink: 0.0, height: px(if compact { 100.0 } else { 152.0 }), overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column, row_gap: px(6), padding: UiRect::all(px(10)),
                border: UiRect::top(px(1)), ..default() }, BorderColor::all(GOLD)))
            .with_children(|detail| {
                detail.spawn((DetailName, Name::new("Equipment detail name"), Text::new(""),
                    TextFont { font_size: FontSize::Px(18.0), ..default() }, TextColor(GOLD)));
                if crafting_tab {
                    detail.spawn((MaterialStatus, Name::new("Crafting material status"), Text::new(""),
                        TextFont { font_size: FontSize::Px(12.0), ..default() }, TextColor(IVORY), TextLayout::linebreak(LineBreak::WordOrCharacter)));
                }
                detail.spawn((DetailEffect, Name::new("Equipment detail effect"), Text::new(""),
                    TextFont { font_size: FontSize::Px(14.0), ..default() }, TextColor(IVORY), TextLayout::linebreak(LineBreak::WordOrCharacter)));
            });
                if crafting_tab {
                    body.spawn((Button, ConfirmCraft, Name::new("Crafting confirm"), Node {
                        width: percent(100), min_height: px(44), align_items: AlignItems::Center, justify_content: JustifyContent::Center,
                        ..default() }, BackgroundColor(PANEL)))
                    .with_child((CraftAction, Name::new("Crafting result"), Text::new(""),
                        TextFont { font_size: FontSize::Px(14.0), ..default() }, TextColor(GOLD), Pickable::IGNORE));
                }
            body.spawn((Text::new(language.pick("All owned equipment stays active. Tap an icon to inspect.\nNo slot limit. Forge consumes materials and inherits all their effects.", "所有已持有装备同时生效，点击图标查看。\n无槽限制。合成消耗材料并继承其全部效果。")),
                Node { flex_shrink: 0.0, ..default() }, TextFont { font_size: FontSize::Px(12.0), ..default() }, TextColor(IVORY)));
        });
    }).id()
}
