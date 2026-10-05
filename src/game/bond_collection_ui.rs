//! Browsable family collection, independent of the compact battle progress strip.
use super::super::{
    BondId, GameLanguage, GameSession, PlayerBuild, SelectedHero, SettingsInputSet,
};
use super::HudHint;
use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
    ui::RelativeCursorPosition,
};

#[derive(Resource, Default)]
pub(super) struct CollectionState {
    root: Option<Entity>,
    selected: Option<BondId>,
    size: Vec2,
    language: Option<GameLanguage>,
    list_scroll: Vec2,
}
impl CollectionState {
    pub(super) fn open(&self) -> bool {
        self.root.is_some()
    }
}
#[derive(Component)]
struct Open;
#[derive(Component)]
struct Close;
#[derive(Component)]
struct Select(BondId);
#[derive(Component)]
struct Scroll;

pub(super) fn install(app: &mut App) {
    app.init_resource::<CollectionState>()
        .add_systems(
            PreUpdate,
            handle
                .after(bevy::ui::UiSystems::Focus)
                .after(SettingsInputSet),
        )
        .add_systems(Update, scroll);
}
pub(super) fn spawn_entry(parent: &mut ChildSpawnerCommands, assets: &AssetServer) {
    parent
        .spawn((
            Button,
            Open,
            HudHint::Collection,
            Name::new("Bond collection open"),
            Node {
                width: px(44),
                height: px(44),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.05, 0.08, 0.09)),
            BorderColor::all(Color::srgb(0.75, 0.61, 0.35)),
        ))
        .with_children(|p| {
            p.spawn((
                ImageNode::new(assets.load("ui/icons/skill.png")),
                Node {
                    width: px(30),
                    height: px(30),
                    ..default()
                },
                Pickable::IGNORE,
            ));
        });
}
fn family_name(language: GameLanguage, family: BondId) -> &'static str {
    let def = family.definition();
    language.pick(def.name_en, def.name)
}
fn progress(build: &PlayerBuild, family: BondId) -> String {
    format!(
        "{}/{}",
        build.bond(family),
        family.definition().members.len()
    )
}
fn prerequisite(language: GameLanguage, build: &PlayerBuild, family: BondId) -> String {
    match family.definition().prerequisite {
        None => language.pick("Starting collection", "初始图鉴").into(),
        Some(parent) => format!(
            "{} {} · {}",
            language.pick("Requires", "前置"),
            family_name(language, parent),
            if usize::from(build.bond(parent)) >= parent.definition().members.len() {
                language.pick("Completed", "已完成")
            } else {
                language.pick("Incomplete", "未完成")
            }
        ),
    }
}
#[allow(clippy::too_many_arguments)]
fn handle(
    mut commands: Commands,
    mut state: ResMut<CollectionState>,
    mut session: ResMut<GameSession>,
    selected: Res<SelectedHero>,
    build: Res<PlayerBuild>,
    language: Res<GameLanguage>,
    assets: Res<AssetServer>,
    windows: Query<&Window>,
    mut keyboard: ResMut<ButtonInput<KeyCode>>,
    opens: Query<&Interaction, (With<Open>, Changed<Interaction>)>,
    closes: Query<&Interaction, (With<Close>, Changed<Interaction>)>,
    choices: Query<(&Interaction, &Select), Changed<Interaction>>,
    settings: Res<super::super::settings::SettingsState>,
    scroll_positions: Query<(&Name, &ScrollPosition), With<Scroll>>,
) {
    if settings.open {
        return;
    }
    let size = windows
        .single()
        .map(|w| Vec2::new(w.width(), w.height()))
        .unwrap_or(Vec2::new(1280.0, 720.0));
    let choice = choices.iter().find_map(|(interaction, choice)| {
        (*interaction == Interaction::Pressed).then_some(choice.0)
    });
    if state.open() {
        let closing = keyboard.just_pressed(KeyCode::Escape)
            || keyboard.just_pressed(KeyCode::KeyH)
            || closes.iter().any(|i| *i == Interaction::Pressed)
            || session.finished;
        keyboard.clear();
        if closing {
            commands.entity(state.root.take().unwrap()).despawn();
            session.paused = session.finished;
            return;
        }
        session.paused = true;
        if choice.is_none() && state.size == size && state.language == Some(*language) {
            return;
        }
        for (name, position) in &scroll_positions {
            if name.as_str() == "Bond collection families" {
                state.list_scroll = position.0;
            }
        }
        commands.entity(state.root.take().unwrap()).despawn();
    } else {
        if session.paused
            || session.finished
            || session.save_active
            || !selected.locked
            || !(keyboard.just_pressed(KeyCode::KeyH)
                || opens.iter().any(|i| *i == Interaction::Pressed))
        {
            return;
        }
        session.paused = true;
        keyboard.clear();
    }
    state.selected = choice.or(state.selected).or(Some(BondId::Journey));
    state.size = size;
    state.language = Some(*language);
    state.root = Some(panel(
        &mut commands,
        &assets,
        *language,
        &build,
        state.selected.unwrap(),
        size,
        state.list_scroll,
        &session,
    ));
}
fn label(parent: &mut ChildSpawnerCommands, text: impl Into<String>, size: f32, color: Color) {
    parent.spawn((
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
        Node {
            flex_shrink: 0.0,
            max_width: percent(100),
            ..default()
        },
        Pickable::IGNORE,
    ));
}
fn panel(
    commands: &mut Commands,
    assets: &AssetServer,
    language: GameLanguage,
    build: &PlayerBuild,
    selected: BondId,
    size: Vec2,
    list_scroll: Vec2,
    session: &GameSession,
) -> Entity {
    let compact = size.x < 800.0 || size.y < 550.0;
    let narrow = size.x < 600.0;
    let font = if compact { 12.0 } else { 15.0 };
    let ink = Color::srgb(0.94, 0.89, 0.76);
    let muted = Color::srgb(0.59, 0.69, 0.7);
    let gold = Color::srgb(0.9, 0.7, 0.35);
    commands
        .spawn((
            Name::new("Bond collection overlay"),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.01, 0.02, 0.85)),
            GlobalZIndex(285),
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Name::new("Bond collection panel"),
                    Node {
                        width: px((size.x - 16.0).min(1000.0)),
                        height: px((size.y - 16.0).min(660.0)),
                        padding: UiRect::all(px(if compact { 8 } else { 16 })),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(8),
                        border: UiRect::all(px(1)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.025, 0.05, 0.06)),
                    BorderColor::all(gold),
                ))
                .with_children(|panel| {
                    panel
                        .spawn(Node {
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::SpaceBetween,
                            min_height: px(44),
                            flex_shrink: 0.0,
                            ..default()
                        })
                        .with_children(|header| {
                            label(
                                header,
                                language.pick("BOND COLLECTION", "羁绊图鉴"),
                                if compact { 18.0 } else { 24.0 },
                                ink,
                            );
                            header
                                .spawn((
                                    Button,
                                    Close,
                                    Name::new("Bond collection close"),
                                    Node {
                                        width: px(44),
                                        height: px(44),
                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::Center,
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb(0.15, 0.19, 0.21)),
                                ))
                                .with_children(|p| label(p, "×", 24.0, ink));
                        });
                    panel
                        .spawn(Node {
                            flex_grow: 1.0,
                            min_height: px(0),
                            column_gap: px(10),
                            ..default()
                        })
                        .with_children(|columns| {
                            columns
                                .spawn((
                                    Name::new("Bond collection families"),
                                    Scroll,
                                    ScrollPosition(list_scroll),
                                    RelativeCursorPosition::default(),
                                    Node {
                                        width: percent(if narrow { 34.0 } else { 31.0 }),
                                        flex_shrink: 0.0,
                                        overflow: Overflow::scroll_y(),
                                        flex_direction: FlexDirection::Column,
                                        row_gap: px(4),
                                        ..default()
                                    },
                                ))
                                .with_children(|list| {
                                    for family in BondId::ALL {
                                        let complete = usize::from(build.bond(family))
                                            >= family.definition().members.len();
                                        list.spawn((
                                            Button,
                                            Select(family),
                                            Name::new(format!(
                                                "Bond collection family {}",
                                                family.definition().id
                                            )),
                                            Node {
                                                min_height: px(if compact { 52 } else { 62 }),
                                                padding: UiRect::all(px(5)),
                                                flex_shrink: 0.0,
                                                column_gap: px(7),
                                                align_items: AlignItems::Center,
                                                border: UiRect::all(px(1)),
                                                ..default()
                                            },
                                            BackgroundColor(if family == selected {
                                                Color::srgb(0.13, 0.2, 0.21)
                                            } else {
                                                Color::srgb(0.045, 0.08, 0.09)
                                            }),
                                            BorderColor::all(if family == selected {
                                                gold
                                            } else {
                                                Color::srgb(0.12, 0.2, 0.21)
                                            }),
                                        ))
                                        .with_children(
                                            |row| {
                                                if !narrow {
                                                    row.spawn((
                                                        ImageNode::new(assets.load(format!(
                                                            "ui/icons/{}.png",
                                                            family.definition().icon
                                                        ))),
                                                        Node {
                                                            width: px(32),
                                                            height: px(32),
                                                            flex_shrink: 0.0,
                                                            ..default()
                                                        },
                                                        Pickable::IGNORE,
                                                    ));
                                                }
                                                row.spawn((
                                                    Node {
                                                        flex_direction: FlexDirection::Column,
                                                        min_width: px(0),
                                                        ..default()
                                                    },
                                                    Pickable::IGNORE,
                                                ))
                                                .with_children(|text| {
                                                    label(
                                                        text,
                                                        family_name(language, family),
                                                        font,
                                                        ink,
                                                    );
                                                    label(
                                                        text,
                                                        format!(
                                                            "{}{}",
                                                            progress(build, family),
                                                            if complete { " ✓" } else { "" }
                                                        ),
                                                        font - 1.0,
                                                        if complete { gold } else { muted },
                                                    );
                                                });
                                            },
                                        );
                                    }
                                });
                            columns
                                .spawn((
                                    Name::new("Bond collection details"),
                                    Scroll,
                                    ScrollPosition::default(),
                                    RelativeCursorPosition::default(),
                                    Node {
                                        flex_grow: 1.0,
                                        min_width: px(0),
                                        overflow: Overflow::scroll_y(),
                                        flex_direction: FlexDirection::Column,
                                        row_gap: px(8),
                                        ..default()
                                    },
                                ))
                                .with_children(|detail| {
                                    let family = selected.definition();
                                    label(
                                        detail,
                                        format!(
                                            "{} · {}",
                                            family_name(language, selected),
                                            progress(build, selected)
                                        ),
                                        font + 3.0,
                                        gold,
                                    );
                                    label(
                                        detail,
                                        prerequisite(language, build, selected),
                                        font - 1.0,
                                        muted,
                                    );
                                    if let Some((level, seconds)) =
                                        super::super::progression::bond_stage_requirement(selected)
                                    {
                                        let ready =
                                            session.level >= level || session.elapsed >= seconds;
                                        label(
                                            detail,
                                            format!(
                                                "{} {} {} {} {:02}:{:02} · {}",
                                                language.pick("Stage: level", "阶段：等级"),
                                                level,
                                                language.pick("or", "或"),
                                                language.pick("time", "时间"),
                                                seconds as u32 / 60,
                                                seconds as u32 % 60,
                                                if ready {
                                                    language.pick("Reached", "已达到")
                                                } else {
                                                    language.pick("Not reached", "未达到")
                                                }
                                            ),
                                            font - 1.0,
                                            if ready { gold } else { muted },
                                        );
                                    }

                                    for bonus in family.bonuses {
                                        let active = build.bond(selected) >= bonus.required;
                                        label(
                                            detail,
                                            format!(
                                                "{} {} · {}",
                                                if active { "✓" } else { "○" },
                                                bonus.required,
                                                language
                                                    .pick(bonus.description_en, bonus.description)
                                            ),
                                            font - 1.0,
                                            if active { gold } else { muted },
                                        );
                                    }
                                    for card in family.members {
                                        let def = card.definition();
                                        let owned = build.owned_bond_cards[card.index()];
                                        detail
                                            .spawn((
                                                Name::new(format!(
                                                    "Bond collection card {}",
                                                    def.id
                                                )),
                                                Node {
                                                    flex_direction: FlexDirection::Column,
                                                    padding: UiRect::all(px(8)),
                                                    row_gap: px(5),
                                                    flex_shrink: 0.0,
                                                    border: UiRect::all(px(1)),
                                                    ..default()
                                                },
                                                BackgroundColor(Color::srgb(0.045, 0.08, 0.09)),
                                                BorderColor::all(if owned {
                                                    Color::srgb(0.38, 0.52, 0.3)
                                                } else {
                                                    Color::srgb(0.11, 0.19, 0.2)
                                                }),
                                            ))
                                            .with_children(|card| {
                                                card.spawn(Node {
                                                    align_items: AlignItems::Center,
                                                    column_gap: px(6),
                                                    ..default()
                                                })
                                                .with_children(|title| {
                                                    title.spawn((
                                                        ImageNode::new(assets.load(format!(
                                                            "ui/icons/{}.png",
                                                            def.icon
                                                        ))),
                                                        Node {
                                                            width: px(32),
                                                            height: px(32),
                                                            flex_shrink: 0.0,
                                                            ..default()
                                                        },
                                                        Pickable::IGNORE,
                                                    ));
                                                    label(
                                                        title,
                                                        format!(
                                                            "{} {}",
                                                            if owned { "✓" } else { "○" },
                                                            language.pick(def.name_en, def.name)
                                                        ),
                                                        font,
                                                        if owned { gold } else { ink },
                                                    );
                                                });
                                                label(
                                                    card,
                                                    language
                                                        .pick(def.description_en, def.description),
                                                    font - 1.0,
                                                    muted,
                                                );
                                            });
                                    }
                                });
                        });
                });
        })
        .id()
}
fn scroll(
    mut wheel: MessageReader<MouseWheel>,
    touches: Res<Touches>,
    mut bodies: Query<(&RelativeCursorPosition, &mut ScrollPosition), With<Scroll>>,
) {
    let delta: f32 = wheel
        .read()
        .map(|w| {
            -w.y * if w.unit == MouseScrollUnit::Line {
                26.0
            } else {
                1.0
            }
        })
        .sum();
    let drag = touches.iter().map(|t| -t.delta().y).sum::<f32>();
    for (cursor, mut pos) in &mut bodies {
        if cursor.cursor_over() {
            pos.0.y = (pos.0.y + delta + drag).max(0.0);
        }
    }
}

pub(super) fn reset_run(world: &mut World) {
    world.insert_resource(CollectionState::default());
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn collection_lists_unique_cards_in_every_family() {
        let mut seen = std::collections::HashSet::new();
        for family in BondId::ALL {
            for card in family.definition().members {
                assert!(seen.insert(card.index()));
                assert_eq!(card.definition().family, family);
            }
        }
        assert_eq!(
            seen.len(),
            super::super::super::bond_catalog::BondCardId::COUNT
        );
    }
    #[test]
    fn prerequisite_and_progress_use_real_owned_cards() {
        let mut build = PlayerBuild::default();
        assert_eq!(progress(&build, BondId::Journey), "0/3");
        for card in BondId::Journey.definition().members {
            build.owned_bond_cards[card.index()] = true;
        }
        assert_eq!(progress(&build, BondId::Journey), "3/3");
        assert!(prerequisite(GameLanguage::English, &build, BondId::Agility).contains("Completed"));
    }
}
