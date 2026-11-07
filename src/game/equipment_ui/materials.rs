//! Material portraits and availability use the same inventory checks as forging.
use super::*;

const OWNED: Color = Color::srgb(0.42, 0.86, 0.68);
const MISSING: Color = Color::srgb(0.62, 0.66, 0.68);

#[derive(Component)]
pub(super) struct Slot(usize);
#[derive(Component)]
pub(super) struct Art {
    index: usize,
    max: f32,
}
#[derive(Component)]
pub(super) struct Label {
    index: usize,
    status: bool,
}
#[derive(Component)]
pub(super) struct Badge(RecipeId);

pub(super) fn spawn(parent: &mut ChildSpawnerCommands, compact: bool, short: bool) {
    let icon = if short { 32.0 } else { 44.0 };
    parent
        .spawn((
            Name::new("Crafting material chain"),
            Node {
                width: percent(100),
                flex_shrink: 0.0,
                column_gap: px(6),
                ..default()
            },
        ))
        .with_children(|row| {
            for index in 0..3 {
                row.spawn((
                    Slot(index),
                    Name::new(format!("Crafting material {index}")),
                    Node {
                        flex_grow: 1.0,
                        flex_basis: px(0),
                        min_width: px(0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(px(4)),
                        row_gap: px(3),
                        border: UiRect::all(px(1)),
                        ..default()
                    },
                    BackgroundColor(PANEL),
                    BorderColor::all(MISSING),
                    Pickable::IGNORE,
                ))
                .with_children(|material| {
                    material.spawn((
                        Art { index, max: icon },
                        Name::new(format!("Crafting material art {index}")),
                        ImageNode::default(),
                        Node {
                            width: px(icon),
                            height: px(icon),
                            flex_shrink: 0.0,
                            ..default()
                        },
                        Pickable::IGNORE,
                    ));
                    for status in [false, true] {
                        material.spawn((
                            Label { index, status },
                            Name::new(format!(
                                "Crafting material {} {index}",
                                if status { "status" } else { "name" }
                            )),
                            Text::new(""),
                            TextFont {
                                font_size: FontSize::Px(if compact { 11.0 } else { 12.0 }),
                                ..default()
                            },
                            TextColor(IVORY),
                            TextLayout::justify(Justify::Center)
                                .with_linebreak(LineBreak::WordOrCharacter),
                            Node {
                                width: percent(100),
                                ..default()
                            },
                            Pickable::IGNORE,
                        ));
                    }
                });
            }
        });
}

pub(super) fn badge(parent: &mut ChildSpawnerCommands, assets: &AssetServer, recipe: RecipeId) {
    parent.spawn((
        Badge(recipe),
        Name::new(format!("Crafting availability {}", recipe.index())),
        ImageNode::new(assets.load("ui/icons/confirm.png")),
        Node {
            position_type: PositionType::Absolute,
            right: px(3),
            top: px(3),
            width: px(18),
            height: px(18),
            display: Display::None,
            ..default()
        },
        Pickable::IGNORE,
    ));
}

pub(super) fn update(
    state: Res<EquipmentPanelState>,
    language: Res<GameLanguage>,
    crafting: Res<CraftingState>,
    market: Res<MarketEffects>,
    assets: Res<AssetServer>,
    mut nodes: ParamSet<(
        Query<(&Slot, &mut Node, &mut BorderColor)>,
        Query<(&Art, &mut ImageNode, &mut Node)>,
        Query<(&Badge, &mut Node, &mut ImageNode)>,
    )>,
    mut labels: Query<(&Label, &mut Text, &mut TextColor)>,
) {
    if !state.open() || !state.crafting_tab {
        return;
    }
    let recipe = state.recipe.unwrap_or(RecipeId::Bloodreaver);
    let ingredients = recipe.ingredients();
    for (slot, mut node, mut border) in &mut nodes.p0() {
        let ingredient = ingredients.get(slot.0);
        node.display = if ingredient.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        *border = BorderColor::all(
            if ingredient.is_some_and(|item| crafting.has_ingredient(*item, &market)) {
                OWNED
            } else {
                MISSING
            },
        );
    }
    for (art, mut image, mut node) in &mut nodes.p1() {
        if let Some(ingredient) = ingredients.get(art.index) {
            *image = ingredient.image_node(&assets);
            image.color = if crafting.has_ingredient(*ingredient, &market) {
                Color::WHITE
            } else {
                Color::srgba(1.0, 1.0, 1.0, 0.55)
            };
            let size = match ingredient {
                super::super::crafting::Ingredient::Gear(item) => item.art_size(art.max),
                super::super::crafting::Ingredient::Crafted(recipe) => recipe.art_size(art.max),
            };
            node.width = px(size.x);
            node.height = px(size.y);
        }
    }
    for (label, mut text, mut color) in &mut labels {
        if let Some(ingredient) = ingredients.get(label.index) {
            let owned = crafting.has_ingredient(*ingredient, &market);
            let value = if label.status {
                if owned {
                    language.pick("✓ Owned", "✓ 已持有")
                } else {
                    language.pick("○ Missing", "○ 未持有")
                }
            } else {
                ingredient.name(*language)
            };
            text.reborrow()
                .map_unchanged(|text| &mut text.0)
                .set_if_neq(value.into());
            color.0 = if !label.status {
                IVORY
            } else if owned {
                OWNED
            } else {
                MISSING
            };
        }
    }
    for (badge, mut node, mut image) in &mut nodes.p2() {
        let forged = crafting.acquired(badge.0);
        node.display = if forged || crafting.ready(badge.0, &market) {
            Display::Flex
        } else {
            Display::None
        };
        image.color = if forged { GOLD } else { OWNED };
    }
}
