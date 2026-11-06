//! Art-first shop presentation; transactions remain in the market's public handler.
use super::*;
#[derive(Resource, Default)]
pub(super) struct ShopPreview {
    pub index: usize,
}
#[derive(Component)]
pub(super) struct OfferArt(usize);
#[derive(Component)]
pub(super) struct ArtBox;
#[derive(Component)]
pub(super) struct Price(usize);
#[derive(Component)]
pub(super) struct RarityBadge(usize);
#[derive(Component)]
pub(super) struct Preview;
#[derive(Component)]
pub(super) struct PreviewArt;
#[derive(Component)]
pub(super) struct PreviewText;
#[derive(Component)]
pub(super) struct PreviewBuy;
#[derive(Component)]
pub(super) struct BuyText;
#[derive(Component)]
pub(super) struct Help;
#[derive(Component)]
pub(super) struct HelpText;
#[derive(Component)]
pub(super) struct ShopClose;

fn art(item: MarketItem, assets: &AssetServer) -> ImageNode {
    item.image_node(assets)
}
fn image(
    parent: &mut ChildSpawnerCommands,
    assets: Option<&AssetServer>,
    path: &'static str,
    size: f32,
) {
    if let Some(assets) = assets {
        parent.spawn((
            ImageNode::new(assets.load(path)),
            Node {
                width: px(size),
                height: px(size),
                flex_shrink: 0.,
                ..default()
            },
            Pickable::IGNORE,
        ));
    }
}
fn text(parent: &mut ChildSpawnerCommands, copy: &str, size: f32, color: Color) {
    parent.spawn((
        Text::new(copy),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
        Pickable::IGNORE,
    ));
}
pub(super) fn spawn_shop(
    commands: &mut Commands,
    assets: Option<&AssetServer>,
    language: GameLanguage,
) {
    commands
        .spawn((
            MarketPanel,
            Name::new("Market panel"),
            Node {
                position_type: PositionType::Absolute,
                right: px(18),
                bottom: px(70),
                width: px(660),
                display: Display::None,
                padding: UiRect::all(px(12)),
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(8)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.025, 0.032, 0.038)),
            BorderColor::all(Color::srgb(0.52, 0.39, 0.20)),
            GlobalZIndex(280),
            crate::game::controller_input::ControllerScope(280),
        ))
        .with_children(|panel| {
            panel
                .spawn((
                    MarketHeader,
                    Node {
                        width: percent(100),
                        height: px(36),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceBetween,
                        ..default()
                    },
                ))
                .with_children(|header| {
                    header.spawn((
                        MarketTitle,
                        Text::new(language.pick("BLACK MARKET", "黑市")),
                        TextFont {
                            font_size: FontSize::Px(20.),
                            ..default()
                        },
                        TextColor(Color::srgb(0.96, 0.78, 0.43)),
                    ));
                    header.spawn((
                        MarketWalletLabel,
                        Name::new("Market wallet"),
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(11.),
                            ..default()
                        },
                        TextColor(Color::srgb(0.79, 0.85, 0.84)),
                        TextLayout::justify(Justify::Right),
                        Node {
                            position_type: PositionType::Absolute,
                            right: px(40),
                            top: px(0),
                            width: px(160),
                            ..default()
                        },
                    ));
                    header
                        .spawn((
                            ShopClose,
                            Button,
                            Name::new("Market close"),
                            Node {
                                width: px(32),
                                height: px(32),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.10, 0.13, 0.14)),
                        ))
                        .with_children(|p| text(p, "×", 20., Color::WHITE));
                });
            panel
                .spawn((
                    MarketOfferGrid,
                    Node {
                        width: percent(100),
                        flex_direction: FlexDirection::Row,
                        column_gap: px(6),
                        ..default()
                    },
                ))
                .with_children(|grid| {
                    for index in 0..OFFER_COUNT {
                        grid.spawn((
                            MarketOfferRow,
                            Node {
                                width: percent(19.2),
                                height: px(190),
                                flex_direction: FlexDirection::Column,
                                row_gap: px(3),
                                ..default()
                            },
                        ))
                        .with_children(|tile| {
                            tile.spawn((
                                Button,
                                MarketOfferButton(index),
                                Name::new(format!("Market offer {index}")),
                                Node {
                                    width: percent(100),
                                    flex_grow: 1.,
                                    padding: UiRect::all(px(6)),
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::SpaceBetween,
                                    row_gap: px(3),
                                    border: UiRect::all(px(1)),
                                    border_radius: BorderRadius::all(px(4)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.07, 0.09, 0.11)),
                                BorderColor::all(Color::srgb(0.35, 0.37, 0.38)),
                            ))
                            .with_children(|button| {
                                button.spawn((
                                    RarityBadge(index),
                                    Text::new(""),
                                    TextFont {
                                        font_size: FontSize::Px(10.),
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                    Node {
                                        position_type: PositionType::Absolute,
                                        top: px(5),
                                        left: px(5),
                                        ..default()
                                    },
                                    Pickable::IGNORE,
                                ));
                                if let Some(assets) = assets {
                                    button
                                        .spawn((
                                            ArtBox,
                                            Node {
                                                width: px(96),
                                                height: px(96),
                                                margin: UiRect::top(px(4)),
                                                align_items: AlignItems::Center,
                                                justify_content: JustifyContent::Center,
                                                flex_shrink: 0.,
                                                ..default()
                                            },
                                            Pickable::IGNORE,
                                        ))
                                        .with_child((
                                            OfferArt(index),
                                            art(MarketItem::ArcLens, assets),
                                            Node {
                                                width: px(96),
                                                height: px(96),
                                                ..default()
                                            },
                                            Pickable::IGNORE,
                                        ));
                                }
                                button.spawn((
                                    MarketOfferLabel(index),
                                    Text::new(""),
                                    TextFont {
                                        font_size: FontSize::Px(12.),
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                    TextLayout::justify(Justify::Center),
                                    Node {
                                        height: px(26),
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    Pickable::IGNORE,
                                ));
                                button
                                    .spawn(Node {
                                        align_items: AlignItems::Center,
                                        column_gap: px(4),
                                        ..default()
                                    })
                                    .with_children(|price| {
                                        image(price, assets, "ui/icons/skull.png", 15.);
                                        price.spawn((
                                            Price(index),
                                            Name::new(format!("Market price {index}")),
                                            Text::new(""),
                                            TextFont {
                                                font_size: FontSize::Px(14.),
                                                ..default()
                                            },
                                            TextColor(Color::srgb(1., 0.80, 0.42)),
                                            Pickable::IGNORE,
                                        ));
                                    });
                            });
                            tile.spawn((
                                Button,
                                MarketMarkButton(index),
                                Name::new(format!("Market auto {index}")),
                                Node {
                                    width: percent(100),
                                    height: px(32),
                                    min_height: px(32),
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    column_gap: px(5),
                                    border_radius: BorderRadius::all(px(4)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.08, 0.14, 0.16)),
                            ))
                            .with_children(|button| {
                                image(button, assets, "ui/icons/refresh.png", 18.);
                                button.spawn((
                                    MarketMarkLabel(index),
                                    Text::new(""),
                                    TextFont {
                                        font_size: FontSize::Px(12.),
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.5, 1., 0.7)),
                                    Pickable::IGNORE,
                                ));
                            });
                        });
                    }
                });
            panel
                .spawn((
                    Preview,
                    Name::new("Market preview"),
                    Node {
                        width: percent(100),
                        min_height: px(104),
                        padding: UiRect::all(px(10)),
                        align_items: AlignItems::Center,
                        column_gap: px(12),
                        border: UiRect::top(px(1)),
                        ..default()
                    },
                    BorderColor::all(Color::srgb(0.37, 0.31, 0.19)),
                    BackgroundColor(Color::srgb(0.035, 0.052, 0.058)),
                ))
                .with_children(|preview| {
                    if let Some(assets) = assets {
                        preview
                            .spawn((
                                Node {
                                    width: px(62),
                                    height: px(62),
                                    flex_shrink: 0.,
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    ..default()
                                },
                                Pickable::IGNORE,
                            ))
                            .with_child((
                                PreviewArt,
                                art(MarketItem::ArcLens, assets),
                                Node {
                                    width: px(62),
                                    height: px(62),
                                    ..default()
                                },
                                Pickable::IGNORE,
                            ));
                    }
                    preview.spawn((
                        PreviewText,
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(13.),
                            ..default()
                        },
                        TextColor(Color::srgb(0.86, 0.89, 0.85)),
                        Node {
                            flex_grow: 1.,
                            flex_basis: px(0),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ));
                    preview
                        .spawn((
                            Button,
                            PreviewBuy,
                            Name::new("Market buy selected"),
                            Node {
                                width: px(110),
                                min_height: px(48),
                                padding: UiRect::all(px(6)),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.50, 0.33, 0.12)),
                        ))
                        .with_children(|button| {
                            button.spawn((
                                BuyText,
                                Text::new(""),
                                TextFont {
                                    font_size: FontSize::Px(14.),
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                                Pickable::IGNORE,
                            ));
                        });
                });
            panel
                .spawn((
                    Help,
                    Node {
                        width: percent(100),
                        height: px(20),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                ))
                .with_children(|p| {
                    p.spawn((
                        HelpText,
                        Name::new("Market feedback"),
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(11.),
                            ..default()
                        },
                        TextColor(Color::srgb(0.60, 0.74, 0.73)),
                        Pickable::IGNORE,
                    ));
                });
            panel
                .spawn((
                    Button,
                    MarketRefreshButton,
                    Name::new("Market refresh"),
                    Node {
                        width: percent(100),
                        height: px(40),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        column_gap: px(7),
                        border: UiRect::all(px(1)),
                        ..default()
                    },
                    BorderColor::all(Color::srgb(0.35, 0.30, 0.20)),
                    BackgroundColor(Color::srgb(0.13, 0.11, 0.065)),
                ))
                .with_children(|p| {
                    image(p, assets, "ui/icons/refresh.png", 20.);
                    image(p, assets, "ui/icons/skull.png", 16.);
                    p.spawn((
                        MarketRefreshLabel,
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(13.),
                            ..default()
                        },
                        TextColor(Color::srgb(0.95, 0.80, 0.49)),
                        Pickable::IGNORE,
                    ));
                });
        });
}

pub(super) fn select_preview(
    mut preview: ResMut<ShopPreview>,
    mut state: ResMut<MarketState>,
    mut session: ResMut<GameSession>,
    offers: Query<(
        &MarketOfferButton,
        &Interaction,
        Has<crate::game::controller_input::ControllerFocused>,
    )>,
    close: Query<&Interaction, (With<ShopClose>, Changed<Interaction>)>,
) {
    if !state.open {
        return;
    }
    if close.iter().any(|i| *i == Interaction::Pressed) {
        close_market(&mut state, &mut session);
        return;
    }
    if let Some((button, _, _)) = offers
        .iter()
        .find(|(_, i, _)| **i == Interaction::Pressed)
        .or_else(|| offers.iter().find(|(_, _, focused)| *focused))
        .or_else(|| offers.iter().find(|(_, i, _)| **i == Interaction::Hovered))
    {
        if preview.index != button.0 {
            // Feedback belongs to the previous selection. Do not show its
            // missing-cost message below another item's different price.
            state.notice_timer = 0.0;
        }
        preview.index = button.0;
    }
}

#[allow(clippy::type_complexity)]
pub(super) fn layout(
    state: Res<MarketState>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut nodes: Query<
        (
            &mut Node,
            Option<&MarketPanel>,
            Option<&MarketHeader>,
            Option<&MarketOfferGrid>,
            Option<&MarketOfferRow>,
            Option<&MarketOfferButton>,
            Option<&MarketMarkButton>,
            Option<&MarketRefreshButton>,
            Option<&MarketToggleButton>,
            Option<&Preview>,
        ),
        Or<(
            With<MarketPanel>,
            With<MarketHeader>,
            With<MarketOfferGrid>,
            With<MarketOfferRow>,
            With<MarketOfferButton>,
            With<MarketMarkButton>,
            With<MarketRefreshButton>,
            With<MarketToggleButton>,
            With<Preview>,
        )>,
    >,
) {
    let size = windows
        .single()
        .map(|w| Vec2::new(w.width(), w.height()))
        .unwrap_or(Vec2::new(1280., 720.));
    let layout = market_layout(size.x, size.y);
    let compact = layout.compact;
    let columns = if size.x < 600. { 3. } else { 5. };
    for (mut node, panel, header, grid, row, offer, mark, refresh, toggle, preview) in &mut nodes {
        let mut next = node.clone();
        if panel.is_some() {
            next.left = if compact {
                px(layout.safe_x)
            } else {
                Val::Auto
            };
            next.right = px(if compact { layout.safe_x } else { 18. });
            next.top = if compact { px(8) } else { Val::Auto };
            next.bottom = if compact { px(8) } else { px(70) };
            next.width = if compact { Val::Auto } else { px(660) };
            next.height = Val::Auto;
            next.min_height = px(0);
            next.padding = UiRect::all(px(if compact { 8. } else { 12. }));
            next.row_gap = px(if compact { 3. } else { 8. });
        } else if header.is_some() {
            next.height = px(if compact { 32. } else { 36. });
            next.min_height = next.height;
        } else if grid.is_some() {
            next.flex_direction = FlexDirection::Row;
            next.flex_wrap = FlexWrap::Wrap;
            next.column_gap = px(6);
            next.row_gap = px(6);
            next.align_items = AlignItems::Stretch;
        } else if row.is_some() {
            next.width = percent((100. - (columns - 1.) * 1.2) / columns);
            next.height = px(if compact { 150. } else { 198. });
            next.min_height = next.height;
            next.flex_direction = FlexDirection::Column;
            next.flex_shrink = 0.;
        } else if offer.is_some() {
            next.width = percent(100);
            next.height = Val::Auto;
            next.min_height = px(0);
            next.flex_grow = 1.;
        } else if mark.is_some() {
            next.width = percent(100);
            next.height = px(32);
            next.min_height = next.height;
        } else if refresh.is_some() {
            next.height = px(44);
            next.min_height = px(44);
        } else if preview.is_some() {
            next.min_height = px(if compact { 80. } else { 104. });
            next.padding = UiRect::all(px(if compact { 6. } else { 10. }));
        } else if toggle.is_some() {
            next.display = if state.open {
                Display::None
            } else {
                Display::Flex
            };
            next.left = Val::Auto;
            next.right = px(layout.toggle_right);
            next.width = px(layout.toggle_width);
            next.height = px(layout.toggle_height);
            next.min_height = next.height;
            next.top = if layout.portrait {
                px(176)
            } else if compact {
                px(layout.safe_y)
            } else {
                Val::Auto
            };
            next.bottom = if compact {
                Val::Auto
            } else {
                px(layout.safe_y)
            };
        }
        node.set_if_neq(next);
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(super) fn update_visuals(
    mut commands: Commands,
    assets: Option<Res<AssetServer>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    language: Res<GameLanguage>,
    state: Res<MarketState>,
    effects: Res<MarketEffects>,
    session: Res<GameSession>,
    preview: Res<ShopPreview>,
    mut icons: Query<
        (&OfferArt, &mut ImageNode, &mut Node),
        (Without<PreviewArt>, Without<Preview>, Without<ArtBox>),
    >,
    mut preview_icons: Query<
        (&mut ImageNode, &mut Node),
        (With<PreviewArt>, Without<OfferArt>, Without<ArtBox>),
    >,
    mut copy: Query<
        (
            &mut Text,
            Option<&Price>,
            Option<&RarityBadge>,
            Has<PreviewText>,
            Has<BuyText>,
            Has<HelpText>,
        ),
        (Without<MarketOfferLabel>, Without<MarketMarkLabel>),
    >,
    mut frames: Query<(&MarketOfferButton, &mut BorderColor)>,
    interactions: Query<(
        &MarketMarkButton,
        &Interaction,
        Has<crate::game::controller_input::ControllerFocused>,
    )>,
    mut art_boxes: Query<&mut Node, (With<ArtBox>, Without<OfferArt>, Without<PreviewArt>)>,
    controls: Query<
        (
            &Interaction,
            Has<crate::game::controller_input::ControllerFocused>,
            Has<ShopClose>,
            Has<MarketRefreshButton>,
            Has<PreviewBuy>,
        ),
        Or<(With<ShopClose>, With<MarketRefreshButton>, With<PreviewBuy>)>,
    >,
    mut buy_buttons: Query<
        (
            Entity,
            &mut BackgroundColor,
            Has<crate::game::controller_input::ControllerDisabled>,
        ),
        With<PreviewBuy>,
    >,
) {
    let compact = windows
        .single()
        .is_ok_and(|w| hud_metrics(w.width(), w.height()).compact);
    let selected = state.offers[preview.index.min(OFFER_COUNT - 1)].item;
    if let Some(assets) = assets.as_deref() {
        for (index, mut image, mut node) in &mut icons {
            let item = state.offers[index.0].item;
            let new = art(item, assets);
            if image.image != new.image || image.rect != new.rect {
                *image = new;
            }
            let size = item.art_size(if compact { 56. } else { 96. });
            node.reborrow()
                .map_unchanged(|n| &mut n.width)
                .set_if_neq(px(size.x));
            node.reborrow()
                .map_unchanged(|n| &mut n.height)
                .set_if_neq(px(size.y));
        }
        for (mut image, mut node) in &mut preview_icons {
            let size = selected.art_size(62.);
            node.reborrow()
                .map_unchanged(|n| &mut n.width)
                .set_if_neq(px(size.x));
            node.reborrow()
                .map_unchanged(|n| &mut n.height)
                .set_if_neq(px(size.y));
            let new = art(selected, assets);
            if image.image != new.image || image.rect != new.rect {
                *image = new;
            }
        }
    }
    let control_hover = controls
        .iter()
        .find(|(i, focused, _, _, _)| *focused || **i != Interaction::None);
    let auto_hover = interactions
        .iter()
        .find(|(_, i, f)| *f || **i != Interaction::None)
        .map(|(b, _, _)| b.0);
    for (mut text, price, rarity, detail, buy, help) in &mut copy {
        let value = if let Some(p) = price {
            let item = state.offers[p.0].item;
            format!("{}", discounted_cost(item.price(), &effects))
        } else if let Some(r) = rarity {
            {
                let item = state.offers[r.0].item;
                if effects.owns(item) {
                    format!("{} ✓", item.tag())
                } else {
                    item.tag().to_owned()
                }
            }
        } else if detail {
            format!(
                "{} · {}\n{}",
                selected.name(*language),
                language.pick(
                    if selected.is_consumable() {
                        "Consumable"
                    } else {
                        "Equipment"
                    },
                    if selected.is_consumable() {
                        "消耗品"
                    } else {
                        "装备"
                    }
                ),
                selected.effect_text(*language)
            )
        } else if buy {
            let price = discounted_cost(selected.price(), &effects);
            if effects.owns(selected) {
                language.pick("Owned", "已拥有").to_owned()
            } else if session.kills < price {
                language.pick("Need", "还差").to_owned() + &format!(" {}", price - session.kills)
            } else {
                format!("{} {}", language.pick("Buy", "购买"), price)
            }
        } else if help {
            if state.notice_timer > 0. {
                state.notice[usize::from(language.is_chinese())].clone()
            } else if let Some((_, _, close, refresh, _)) = control_hover {
                if close {
                    language
                        .pick("Close market · resume battle", "关闭黑市 · 继续战斗")
                        .to_owned()
                } else if refresh {
                    format!(
                        "{} {} · {}",
                        language.pick("Restock", "刷新"),
                        state.refresh_price(&effects),
                        language.pick("cost rises with each refresh", "每次刷新费用提高")
                    )
                } else {
                    language.pick("Buy selected item with kills · permanent equipment, one-use consumables", "使用击杀购买所选商品 · 装备永久生效，消耗品使用一次").to_owned()
                }
            } else if let Some(i) = auto_hover {
                let item = state.offers[i].item;
                if state.marked[i] == Some(item) {
                    language
                        .pick(
                            "Auto-buy enabled · buys when you can afford it",
                            "已开启自动购买 · 资源足够时购入",
                        )
                        .to_owned()
                } else {
                    language
                        .pick(
                            "Auto-buy · mark this item to buy when affordable",
                            "自动购买 · 标记后资源足够时购入",
                        )
                        .to_owned()
                }
            } else {
                language
                    .pick(
                        "Inspect an item · effects remain active after hero absorption",
                        "查看商品 · 装备效果在英雄吞噬后保留",
                    )
                    .to_owned()
            }
        } else {
            continue;
        };
        text.reborrow()
            .map_unchanged(|t| &mut t.0)
            .set_if_neq(value);
    }
    for (button, mut border) in &mut frames {
        let item = state.offers[button.0].item;
        border.set_if_neq(BorderColor::all(if button.0 == preview.index {
            Color::srgb(1., 0.80, 0.35)
        } else {
            item.color()
        }));
    }
    let can_buy =
        !effects.owns(selected) && session.kills >= discounted_cost(selected.price(), &effects);
    for (entity, mut background, disabled) in &mut buy_buttons {
        if can_buy && disabled {
            commands
                .entity(entity)
                .remove::<crate::game::controller_input::ControllerDisabled>();
        } else if !can_buy && !disabled {
            commands
                .entity(entity)
                .insert(crate::game::controller_input::ControllerDisabled);
        }
        background.set_if_neq(BackgroundColor(if can_buy {
            Color::srgb(0.44, 0.30, 0.10)
        } else {
            Color::srgb(0.13, 0.15, 0.16)
        }));
    }
    for mut node in &mut art_boxes {
        let size = px(if compact { 56. } else { 96. });
        node.reborrow()
            .map_unchanged(|n| &mut n.width)
            .set_if_neq(size);
        node.reborrow()
            .map_unchanged(|n| &mut n.height)
            .set_if_neq(size);
    }
}
