//! Equipped black-market items remain active together; this panel reads ownership.
use super::crafting::{CraftingState, RecipeId};
use super::market::{MarketEffects, MarketItem};
use super::{GameLanguage, GameSession, SelectedHero, SettingsInputSet};
use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
};

#[path = "equipment_ui/panel.rs"]
mod panel;
use panel::panel;

const GOLD: Color = Color::srgb(0.90, 0.72, 0.42);
const PANEL: Color = Color::srgb(0.035, 0.055, 0.065);
const IVORY: Color = Color::srgb(0.95, 0.93, 0.85);

#[derive(Resource, Default)]
pub(crate) struct EquipmentPanelState {
    root: Option<Entity>,
    selected: Option<MarketItem>,
    selected_crafted: Option<RecipeId>,
    recipe: Option<RecipeId>,
    crafting_tab: bool,
    crafting_revision: u32,
    known_crafted: Vec<RecipeId>,
    latest_crafted: Option<RecipeId>,
    known: Vec<MarketItem>,
    latest: Option<MarketItem>,
    notice_remaining: f32,
    prior_paused: bool,
    size: Vec2,
    language: Option<GameLanguage>,
}
impl EquipmentPanelState {
    pub(crate) fn open(&self) -> bool {
        self.root.is_some()
    }
    fn close(&mut self, commands: &mut Commands, session: &mut GameSession) {
        if let Some(root) = self.root.take() {
            commands.entity(root).despawn();
        }
        session.paused = self.prior_paused || session.finished;
    }
}
#[derive(Component)]
struct Open;
#[derive(Component)]
struct Close;
#[derive(Component)]
struct Crafted(RecipeId);
#[derive(Component)]
struct Tab(bool);
#[derive(Component)]
struct Recipe(RecipeId);
#[derive(Component)]
struct ConfirmCraft;
#[derive(Component)]
struct MaterialStatus;
#[derive(Component)]
struct CraftAction;
#[derive(Component)]
struct Count;
#[derive(Component)]
struct EntryHint;
#[derive(Component)]
struct Notice;
#[derive(Component)]
struct Item(MarketItem);
#[derive(Component)]
struct DetailName;
#[derive(Component)]
struct DetailEffect;
#[derive(Component)]
struct Grid;
#[derive(Component)]
struct DetailScroll;

pub(crate) fn install(app: &mut App) {
    app.init_resource::<EquipmentPanelState>()
        .init_resource::<CraftingState>()
        .add_systems(
            PreUpdate,
            input
                .after(bevy::ui::UiSystems::Focus)
                .after(SettingsInputSet),
        )
        .add_systems(Update, (feedback, details, scroll).chain());
}

pub(crate) fn spawn_entry(parent: &mut ChildSpawnerCommands, assets: &AssetServer) {
    parent
        .spawn((
            Button,
            Open,
            Name::new("Equipment open"),
            Node {
                width: px(64),
                height: px(44),
                min_width: px(64),
                border: UiRect::all(px(1)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: px(3),
                ..default()
            },
            BackgroundColor(PANEL),
            BorderColor::all(GOLD),
        ))
        .with_children(|entry| {
            entry.spawn((
                ImageNode::new(assets.load("ui/icons/market.png")),
                Node {
                    width: px(30),
                    height: px(30),
                    ..default()
                },
                Pickable::IGNORE,
            ));
            entry.spawn((
                Count,
                Name::new("Equipment count"),
                Text::new("0"),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(IVORY),
                Pickable::IGNORE,
            ));
            entry
                .spawn((
                    EntryHint,
                    Node {
                        position_type: PositionType::Absolute,
                        right: px(0),
                        bottom: percent(110),
                        width: px(160),
                        padding: UiRect::all(px(8)),
                        display: Display::None,
                        ..default()
                    },
                    BackgroundColor(PANEL),
                    Pickable::IGNORE,
                ))
                .with_child((
                    Text::new(""),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(IVORY),
                    Pickable::IGNORE,
                ));
            entry
                .spawn((
                    Notice,
                    Name::new("Equipment acquisition"),
                    Node {
                        position_type: PositionType::Absolute,
                        top: percent(115),
                        left: px(0),
                        width: px(230),
                        padding: UiRect::all(px(8)),
                        display: Display::None,
                        ..default()
                    },
                    BackgroundColor(PANEL),
                    Pickable::IGNORE,
                ))
                .with_child((
                    Text::new(""),
                    TextFont {
                        font_size: FontSize::Px(13.0),
                        ..default()
                    },
                    TextColor(GOLD),
                    Pickable::IGNORE,
                ));
        });
}

#[allow(clippy::too_many_arguments)]
fn input(
    mut commands: Commands,
    mut state: ResMut<EquipmentPanelState>,
    mut session: ResMut<GameSession>,
    selected: Res<SelectedHero>,
    language: Res<GameLanguage>,
    equipment: Res<MarketEffects>,
    crafting: Res<CraftingState>,
    (tabs, recipes, crafted_items, craft_buttons): (
        Query<(&Interaction, &Tab), Changed<Interaction>>,
        Query<(&Interaction, &Recipe), Changed<Interaction>>,
        Query<(&Interaction, &Crafted), Changed<Interaction>>,
        Query<&Interaction, (With<ConfirmCraft>, Changed<Interaction>)>,
    ),
    assets: Res<AssetServer>,
    windows: Query<&Window>,
    mut keyboard: ResMut<ButtonInput<KeyCode>>,
    open: Query<&Interaction, (With<Open>, Changed<Interaction>)>,
    close: Query<&Interaction, (With<Close>, Changed<Interaction>)>,
    items: Query<(&Interaction, &Item), Changed<Interaction>>,
    settings: Res<super::settings::SettingsState>,
    hero: Res<super::ui::hero_stats_ui::HeroStatsPanelState>,
) {
    if settings.open || hero.open() {
        return;
    }
    let size = windows
        .single()
        .map(|w| Vec2::new(w.width(), w.height()))
        .unwrap_or(Vec2::new(1280.0, 720.0));
    if state.open() {
        if keyboard.just_pressed(KeyCode::Escape)
            || keyboard.just_pressed(KeyCode::KeyI)
            || close.iter().any(|i| *i == Interaction::Pressed)
            || session.finished
        {
            keyboard.clear();
            state.close(&mut commands, &mut session);
            return;
        }
        let mut rebuild = false;
        for (interaction, tab) in &tabs {
            if *interaction == Interaction::Pressed && state.crafting_tab != tab.0 {
                state.crafting_tab = tab.0;
                if state.recipe.is_none() {
                    state.recipe = Some(RecipeId::Bloodreaver);
                }
                rebuild = true;
            }
        }
        for (interaction, recipe) in &recipes {
            if matches!(interaction, Interaction::Pressed | Interaction::Hovered) {
                state.recipe = Some(recipe.0);
            }
        }
        for (interaction, item) in &crafted_items {
            if matches!(interaction, Interaction::Pressed | Interaction::Hovered) {
                state.selected_crafted = Some(item.0);
                state.selected = None;
            }
        }
        if craft_buttons
            .iter()
            .any(|interaction| *interaction == Interaction::Pressed)
            && let Some(recipe) = state.recipe
            && crafting.ready(recipe, &equipment)
        {
            commands.queue(move |world: &mut World| {
                super::crafting::craft_recipe(world, recipe);
            });
        }
        for (interaction, item) in &items {
            if matches!(interaction, Interaction::Pressed | Interaction::Hovered) {
                state.selected = Some(item.0);
                state.selected_crafted = None;
            }
        }
        if !rebuild
            && state.size == size
            && state.language == Some(*language)
            && state.known == equipment.equipment()
            && state.crafting_revision == crafting.revision
        {
            return;
        }
        commands.entity(state.root.take().unwrap()).despawn();
    } else {
        let requested = keyboard.just_pressed(KeyCode::KeyI)
            || open
                .iter()
                .any(|interaction| *interaction == Interaction::Pressed);
        if !requested
            || session.paused
            || session.finished
            || !selected.locked
            || session.save_active
        {
            return;
        }
        state.prior_paused = session.paused;
        session.paused = true;
        keyboard.clear();
    }
    state.size = size;
    state.language = Some(*language);
    state.crafting_revision = crafting.revision;
    let owned = equipment.equipment();
    let crafted = crafting.equipment();
    if !state.selected.is_some_and(|item| owned.contains(&item))
        && !state
            .selected_crafted
            .is_some_and(|item| crafted.contains(&item))
    {
        state.selected = owned.first().copied();
        state.selected_crafted = if state.selected.is_none() {
            crafted.first().copied()
        } else {
            None
        };
    }
    state.root = Some(panel(
        &mut commands,
        &assets,
        *language,
        size,
        (&owned, &crafted, state.crafting_tab),
    ));
}

#[allow(clippy::too_many_arguments)]
fn feedback(
    time: Res<Time<Real>>,
    language: Res<GameLanguage>,
    equipment: Res<MarketEffects>,
    crafting: Res<CraftingState>,
    session: Res<GameSession>,
    mut state: ResMut<EquipmentPanelState>,
    mut counts: Query<&mut Text, With<Count>>,
    mut entries: Query<(&Interaction, &mut BorderColor), With<Open>>,
    mut hints: Query<(&mut Node, &Children), (With<EntryHint>, Without<Notice>)>,
    mut notices: Query<(&mut Node, &Children), (With<Notice>, Without<EntryHint>)>,
    mut texts: Query<&mut Text, Without<Count>>,
) {
    let owned = equipment.equipment();
    if let Some(latest) = owned
        .iter()
        .copied()
        .rfind(|item| !state.known.contains(item))
    {
        state.latest = Some(latest);
        state.latest_crafted = None;
        state.notice_remaining = 3.0;
    }
    let crafted = crafting.equipment();
    if let Some(latest) = crafted
        .iter()
        .copied()
        .rfind(|item| !state.known_crafted.contains(item))
    {
        state.latest_crafted = Some(latest);
        state.latest = None;
        state.notice_remaining = 3.0;
    }
    state.known_crafted = crafted;
    state.known = owned;
    if !session.paused {
        state.notice_remaining = (state.notice_remaining - time.delta_secs()).max(0.0);
    }
    for mut count in &mut counts {
        **count = (state.known.len() + state.known_crafted.len()).to_string();
    }
    let hovered = entries
        .iter()
        .any(|(interaction, _)| *interaction == Interaction::Hovered);
    for (_, mut border) in &mut entries {
        *border = BorderColor::all(if state.notice_remaining > 0.0 {
            Color::srgb(1.0, 0.85, 0.35)
        } else {
            GOLD
        });
    }
    for (mut node, children) in &mut hints {
        node.display = if hovered {
            Display::Flex
        } else {
            Display::None
        };
        for child in children.iter() {
            if let Ok(mut text) = texts.get_mut(child) {
                **text = language.pick("Equipment · I", "装备 · I").into();
            }
        }
    }
    for (mut node, children) in &mut notices {
        node.display = if state.notice_remaining > 0.0 && !state.open() && !session.paused {
            Display::Flex
        } else {
            Display::None
        };
        for child in children.iter() {
            if let Ok(mut text) = texts.get_mut(child) {
                **text = state
                    .latest
                    .map(|item| {
                        format!(
                            "{} {}",
                            language.pick("Equipped:", "获得装备："),
                            item.name(*language)
                        )
                    })
                    .or_else(|| {
                        state.latest_crafted.map(|recipe| {
                            format!(
                                "{} {}",
                                language.pick("Forged:", "合成成功："),
                                recipe.name(*language)
                            )
                        })
                    })
                    .unwrap_or_default();
            }
        }
    }
}
fn details(
    state: Res<EquipmentPanelState>,
    language: Res<GameLanguage>,
    crafting: Res<CraftingState>,
    market: Res<MarketEffects>,
    mut texts: Query<(&Name, &mut Text)>,
    mut cards: Query<(
        Option<&Item>,
        Option<&Crafted>,
        Option<&Recipe>,
        Option<&ConfirmCraft>,
        &mut BackgroundColor,
    )>,
) {
    if !state.open() {
        return;
    }
    let recipe = state.recipe.unwrap_or(RecipeId::Bloodreaver);
    let detail_name = if state.crafting_tab {
        format!("[{}] {}", recipe.rarity().label(), recipe.name(*language))
    } else if let Some(item) = state.selected {
        format!("[{}] {}", item.build_rarity().label(), item.name(*language))
    } else if let Some(item) = state.selected_crafted {
        format!("[{}] {}", item.rarity().label(), item.name(*language))
    } else {
        language.pick("No equipment", "暂无装备").into()
    };
    let detail_effect = if state.crafting_tab {
        recipe.full_effect(*language)
    } else if let Some(item) = state.selected {
        item.effect_text(*language).into()
    } else if let Some(item) = state.selected_crafted {
        item.full_effect(*language)
    } else {
        language
            .pick(
                "The Black Market opens after 3 minutes.",
                "黑市在开局3分钟后开放。",
            )
            .into()
    };
    let ready = crafting.ready(recipe, &market);
    let action = if crafting.acquired(recipe) {
        language.pick("Already forged", "已合成")
    } else if ready {
        language.pick("Forge equipment", "合成装备")
    } else {
        language.pick("Missing materials", "缺少材料")
    };
    for (name, mut text) in &mut texts {
        let value = match name.as_str() {
            "Equipment detail name" => detail_name.clone(),
            "Equipment detail effect" => detail_effect.clone(),
            "Crafting material status" => crafting.material_status(recipe, &market, *language),
            "Crafting result" => action.into(),
            _ => continue,
        };
        **text = value;
    }
    for (item, crafted, choice, confirm, mut background) in &mut cards {
        if item.is_none() && crafted.is_none() && choice.is_none() && confirm.is_none() {
            continue;
        }
        let highlighted = item.is_some_and(|item| state.selected == Some(item.0))
            || crafted.is_some_and(|item| state.selected_crafted == Some(item.0))
            || choice.is_some_and(|item| recipe == item.0)
            || (confirm.is_some() && ready);
        *background = BackgroundColor(if highlighted {
            Color::srgb(0.22, 0.19, 0.10)
        } else {
            PANEL
        });
    }
}
fn scroll(
    mut wheel: MessageReader<MouseWheel>,
    touches: Res<Touches>,
    state: Res<EquipmentPanelState>,
    mut bodies: Query<&mut ScrollPosition, (With<Grid>, Without<DetailScroll>)>,
    mut details: Query<(&Interaction, &mut ScrollPosition), (With<DetailScroll>, Without<Grid>)>,
) {
    let delta = wheel
        .read()
        .map(|w| {
            -w.y * if w.unit == MouseScrollUnit::Line {
                28.0
            } else {
                1.0
            }
        })
        .sum::<f32>();
    if !state.open() {
        return;
    }
    let drag = touches.iter().map(|touch| -touch.delta().y).sum::<f32>();
    if details
        .iter()
        .any(|(interaction, _)| matches!(interaction, Interaction::Hovered | Interaction::Pressed))
    {
        for (_, mut position) in &mut details {
            position.0.y = (position.0.y + delta + drag).max(0.0);
        }
    } else {
        for mut position in &mut bodies {
            position.0.y = (position.0.y + delta + drag).max(0.0);
        }
    }
}

/// Scene reset removes the temporary panel and acquisition notice, never market ownership.
pub(crate) fn reset_run(world: &mut World) {
    world.init_resource::<EquipmentPanelState>();
    if let Some(root) = world.resource::<EquipmentPanelState>().root
        && let Ok(entity) = world.get_entity_mut(root)
    {
        entity.despawn();
    }
    *world.resource_mut::<EquipmentPanelState>() = EquipmentPanelState::default();
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    #[test]
    fn close_preserves_the_pause_it_did_not_own_and_run_reset_clears_its_entities() {
        for prior_paused in [false, true] {
            let mut app = App::new();
            app.init_resource::<EquipmentPanelState>()
                .init_resource::<GameSession>();
            let root = app.world_mut().spawn(Name::new("Equipment panel")).id();
            let mut state = app.world_mut().resource_mut::<EquipmentPanelState>();
            state.root = Some(root);
            state.prior_paused = prior_paused;
            app.world_mut().resource_mut::<GameSession>().paused = true;
            app.world_mut()
                .run_system_once(
                    |mut commands: Commands,
                     mut state: ResMut<EquipmentPanelState>,
                     mut session: ResMut<GameSession>| {
                        state.close(&mut commands, &mut session);
                    },
                )
                .unwrap();
            assert!(app.world().get_entity(root).is_err());
            assert_eq!(app.world().resource::<GameSession>().paused, prior_paused);
            assert!(!app.world().resource::<EquipmentPanelState>().open());
            let root = app.world_mut().spawn(Name::new("Equipment panel")).id();
            app.world_mut().resource_mut::<EquipmentPanelState>().root = Some(root);
            reset_run(app.world_mut());
            assert!(app.world().get_entity(root).is_err());
            assert!(!app.world().resource::<EquipmentPanelState>().open());
        }
    }
}
