//! Icon-led build choices; payment and rewards remain in progression.rs.
use crate::game::encounters::EncounterId;
use crate::game::localization::{bond_card_name, bond_family_detail, bond_name, skill_name};
use crate::game::rarity::rarity_color;
use crate::game::{
    CaptureTarget, DraftAction, DraftActionButton, DraftButton, DraftChoice, DraftKind, DraftRoot,
    DraftState, GameLanguage, GameSession, PlayerBuild,
};
use bevy::{prelude::*, window::PrimaryWindow};

const GOLD: Color = Color::srgb(0.91, 0.73, 0.42);
const IVORY: Color = Color::srgb(0.96, 0.93, 0.84);
const MUTED: Color = Color::srgb(0.57, 0.66, 0.67);
const PANEL: Color = Color::srgb(0.045, 0.075, 0.085);
const EDGE: Color = Color::srgb(0.23, 0.32, 0.34);

#[derive(Resource, Default)]
pub(super) struct BuildPreview {
    pub root: Option<Entity>,
    pub index: Option<usize>,
}
#[derive(Component)]
pub(super) struct ArtPath(String);
#[derive(Component)]
pub(super) struct BuildContent;
#[derive(Component)]
pub(super) struct BuildHeader;
#[derive(Component)]
pub(super) struct BuildHeading;
#[derive(Component)]
pub(super) struct BuildGallery;
#[derive(Component)]
pub(super) struct BuildFooter;
#[derive(Component)]
pub(super) struct BuildArt;
#[derive(Component)]
pub(super) struct BuildTitle;
/// Encounter cards print their benefit and cost on the card itself.
#[derive(Component)]
pub(super) struct BuildTradeLine;
#[derive(Component)]
pub(super) struct BuildDetailTitle;
#[derive(Component)]
pub(super) struct BuildDetail;
#[derive(Component)]
pub(super) struct BuildDetailScroll;
#[derive(Component)]
pub(super) struct BuildActionHint;
#[derive(Component)]
pub(super) struct BuildPip {
    choice: DraftChoice,
    point: u8,
}

pub(super) fn sync_preview(draft: Res<DraftState>, mut preview: ResMut<BuildPreview>) {
    if preview.root != draft.root {
        preview.root = draft.root;
        preview.index = (!draft.choices.is_empty()).then_some(0);
    }
}

pub(super) fn load_images(
    server: Option<Res<AssetServer>>,
    mut images: Query<(&ArtPath, &mut ImageNode), Added<ArtPath>>,
) {
    let Some(server) = server else {
        return;
    };
    for (path, mut image) in &mut images {
        image.image = server.load(path.0.clone());
    }
}

fn label(text: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
    )
}
fn art(id: &str, size: f32) -> impl Bundle {
    (
        ArtPath(format!("ui/icons/{id}.png")),
        ImageNode::default(),
        Node {
            width: px(size),
            height: px(size),
            flex_shrink: 0.0,
            ..default()
        },
    )
}
fn name(language: GameLanguage, choice: DraftChoice) -> String {
    match choice {
        DraftChoice::Skill(skill) => skill_name(language, skill),
        DraftChoice::Evolution { skill, tier } => format!(
            "{} · {} {tier}",
            skill_name(language, skill),
            language.pick("Ascension", "升华")
        ),
        DraftChoice::EvolveUr { skill, path } => {
            crate::game::ur_evolution::name(language, skill, path).to_owned()
        }
        DraftChoice::ReplaceSkill { old, new } => format!(
            "{} → {}",
            skill_name(language, old),
            skill_name(language, new)
        ),
        DraftChoice::Bond(card) => bond_card_name(language, card),
        DraftChoice::ReplaceBond(family) => bond_name(language, family),
        DraftChoice::Encounter(event) => event.name(language).to_owned(),
    }
}
fn short_name(language: GameLanguage, choice: DraftChoice) -> String {
    name(language, choice)
}
fn progress(choice: DraftChoice, build: &PlayerBuild) -> (u8, u8, u8) {
    match choice {
        DraftChoice::Skill(skill) => (build.skill_rank(skill), 1, skill.definition().max_rank),
        DraftChoice::Evolution { tier, .. } => (tier.saturating_sub(1), 1, 3),
        DraftChoice::EvolveUr { .. } => (0, 1, 1),
        DraftChoice::ReplaceSkill { new, .. } => (0, 1, new.definition().max_rank),
        DraftChoice::Bond(card) => {
            let family = card.definition().family;
            (build.bond(family), 1, family.required())
        }
        DraftChoice::ReplaceBond(family) => (build.bond(family), 0, family.required()),
        DraftChoice::Encounter(_) => (0, 0, 0),
    }
}

fn heading_copy(language: GameLanguage, kind: DraftKind) -> &'static str {
    match kind {
        DraftKind::Skill => language.pick("Shape your arsenal", "选择你的技能"),
        DraftKind::Bond => language.pick("Forge a new bond", "缔结新的羁绊"),
        DraftKind::BondReplacement => language.pick("Choose a bond to replace", "选择要替换的羁绊"),
        DraftKind::Encounter => language.pick("Choose an encounter", "选择奇遇"),
    }
}

fn trade_line(language: GameLanguage, event: EncounterId) -> String {
    format!(
        "{}\n{} {}",
        event.effect(language),
        language.pick("Cost:", "代价："),
        event.cost(language)
    )
}

pub(super) fn spawn_draft_ui(
    commands: &mut Commands,
    draft: &mut DraftState,
    kind: DraftKind,
    _rerolls: u32,
    language: GameLanguage,
) {
    // Encounters cannot be refreshed or abandoned; replacements cannot be refreshed.
    let refreshable = !matches!(kind, DraftKind::BondReplacement | DraftKind::Encounter);
    let abandonable = kind != DraftKind::Encounter;
    let heading = if let Some(DraftChoice::EvolveUr { skill, .. }) = draft.choices.first() {
        format!(
            "{} · {}",
            skill_name(language, *skill),
            language.pick("Choose an evolution", "选择进化方向")
        )
    } else {
        heading_copy(language, kind).to_owned()
    };
    let root = commands
        .spawn((
            DraftRoot,
            Name::new("Build selection"),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.014, 0.024, 0.032)),
            GlobalZIndex(150),
        ))
        .with_children(|root| {
            root.spawn((
                ArtPath("ui/draft_backdrop.png".into()),
                ImageNode {
                    color: Color::srgb(0.32, 0.42, 0.47),
                    ..default()
                },
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
            ));
            root.spawn((
                BuildContent,
                Node {
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
            ))
            .with_children(|content| {
                content
                    .spawn((
                        BuildHeader,
                        Node {
                            width: percent(100),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::SpaceBetween,
                            flex_shrink: 0.0,
                            ..default()
                        },
                    ))
                    .with_children(|header| {
                        header.spawn((
                            BuildHeading,
                            BuildCopy::Heading(kind),
                            label(heading, 32.0, IVORY),
                        ));
                        header
                            .spawn(Node {
                                column_gap: px(8),
                                flex_shrink: 0.0,
                                ..default()
                            })
                            .with_children(|actions| {
                                crate::game::selection_detail::spawn_toggle(actions);
                                if refreshable {
                                    action(actions, DraftAction::Refresh, "refresh");
                                }
                                if abandonable {
                                    action(actions, DraftAction::Abandon, "abandon");
                                }
                                action(actions, DraftAction::Later, "later");
                                actions.spawn((
                                    BuildActionHint,
                                    Node {
                                        position_type: PositionType::Absolute,
                                        top: percent(100),
                                        right: px(0),
                                        max_width: px(300),
                                        padding: UiRect::all(px(8)),
                                        display: Display::None,
                                        ..default()
                                    },
                                    GlobalZIndex(160),
                                    BackgroundColor(PANEL),
                                    label("", 12.0, IVORY),
                                ));
                            });
                    });
                content
                    .spawn((
                        BuildGallery,
                        Node {
                            width: percent(100),
                            justify_content: JustifyContent::Center,
                            flex_wrap: FlexWrap::NoWrap,
                            flex_shrink: 0.0,
                            ..default()
                        },
                    ))
                    .with_children(|gallery| {
                        for (index, choice) in draft.choices.iter().copied().enumerate() {
                            let (id, cap) = match choice {
                                DraftChoice::Skill(skill) => {
                                    (skill.definition().id, skill.definition().max_rank)
                                }
                                DraftChoice::Evolution { skill, .. } => (skill.definition().id, 3),
                                DraftChoice::EvolveUr { skill, .. } => (skill.definition().id, 1),
                                DraftChoice::ReplaceSkill { new, .. } => {
                                    (new.definition().id, new.definition().max_rank)
                                }
                                DraftChoice::Bond(card) => {
                                    (card.definition().icon, card.definition().family.required())
                                }
                                DraftChoice::ReplaceBond(family) => {
                                    (family.definition().icon, family.required())
                                }
                                DraftChoice::Encounter(event) => (event.icon(), 0),
                            };
                            gallery
                                .spawn((
                                    Button,
                                    DraftButton(index),
                                    Name::new(format!("Build card {index}")),
                                    Node {
                                        flex_direction: FlexDirection::Column,
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        row_gap: px(12),
                                        border: UiRect::all(px(2)),
                                        flex_shrink: 0.0,
                                        overflow: Overflow::clip(),
                                        ..default()
                                    },
                                    BackgroundColor(PANEL),
                                    BorderColor::all(EDGE),
                                ))
                                .with_children(|card| {
                                    card.spawn((BuildArt, art(id, 200.0)));
                                    card.spawn((
                                        BuildTitle,
                                        BuildCopy::Card(choice),
                                        label(short_name(language, choice), 22.0, IVORY),
                                        Node {
                                            width: percent(95),
                                            ..default()
                                        },
                                        TextLayout::justify(Justify::Center)
                                            .with_linebreak(LineBreak::WordOrCharacter),
                                    ));
                                    if let DraftChoice::Encounter(event) = choice {
                                        card.spawn((
                                            BuildTradeLine,
                                            BuildCopy::Trade(event),
                                            label(trade_line(language, event), 13.0, MUTED),
                                            Node {
                                                width: percent(92),
                                                ..default()
                                            },
                                            TextLayout::justify(Justify::Center)
                                                .with_linebreak(LineBreak::WordOrCharacter),
                                        ));
                                    }
                                    card.spawn(Node {
                                        column_gap: px(4),
                                        height: px(10),
                                        align_items: AlignItems::Center,
                                        ..default()
                                    })
                                    .with_children(|pips| {
                                        for point in 0..cap {
                                            pips.spawn((
                                                BuildPip { choice, point },
                                                Node {
                                                    width: px(7),
                                                    height: px(7),
                                                    border: UiRect::all(px(1)),
                                                    ..default()
                                                },
                                                BackgroundColor(EDGE),
                                                BorderColor::all(MUTED),
                                            ));
                                        }
                                    });
                                    card.spawn((
                                        Node {
                                            position_type: PositionType::Absolute,
                                            left: px(9),
                                            top: px(8),
                                            ..default()
                                        },
                                        label(
                                            if matches!(
                                                choice,
                                                DraftChoice::ReplaceBond(_)
                                                    | DraftChoice::Encounter(_)
                                            ) {
                                                ""
                                            } else {
                                                choice.rarity().label()
                                            },
                                            10.0,
                                            rarity_color(choice.rarity()),
                                        ),
                                    ));
                                    card.spawn((
                                        Node {
                                            position_type: PositionType::Absolute,
                                            right: px(9),
                                            top: px(8),
                                            ..default()
                                        },
                                        label(format!("{}", index + 1), 10.0, MUTED),
                                    ));
                                });
                        }
                    });
                content
                    .spawn((
                        BuildFooter,
                        Node {
                            width: percent(100),
                            padding: UiRect::axes(px(20), px(12)),
                            border: UiRect::top(px(1)),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::SpaceBetween,
                            column_gap: px(16),
                            flex_shrink: 0.0,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.025, 0.05, 0.058, 0.97)),
                        BorderColor::all(GOLD),
                    ))
                    .with_children(|footer| {
                        footer
                            .spawn((
                                BuildDetailScroll,
                                ScrollPosition::default(),
                                Node {
                                    flex_direction: FlexDirection::Column,
                                    row_gap: px(4),
                                    flex_grow: 1.0,
                                    flex_shrink: 1.0,
                                    min_width: px(0),
                                    height: percent(100),
                                    min_height: px(0),
                                    overflow: Overflow::scroll_y(),
                                    ..default()
                                },
                            ))
                            .with_children(|details| {
                                details.spawn((BuildDetailTitle, label("", 22.0, IVORY)));
                                details.spawn((
                                    BuildDetail,
                                    Name::new("Selection details"),
                                    label("", 13.0, MUTED),
                                ));
                            });
                    });
            });
        })
        .id();
    draft.root = Some(root);
}

fn action(parent: &mut ChildSpawnerCommands, action: DraftAction, id: &str) {
    parent
        .spawn((
            Button,
            DraftActionButton(action),
            Name::new(format!("Build {id}")),
            Node {
                width: px(48),
                height: px(48),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                ..default()
            },
            BackgroundColor(PANEL),
            BorderColor::all(EDGE),
        ))
        .with_children(|button| {
            button.spawn(art(id, 30.0));
        });
}

#[derive(Clone, Copy)]
struct Layout {
    width: f32,
    height: f32,
    header: f32,
    footer: f32,
    gap: f32,
    card_width: f32,
    card_height: f32,
    icon: f32,
    compact: bool,
    count: usize,
}
fn layout(width: f32, height: f32, count: usize) -> Layout {
    let compact = height < 540.0;
    let width =
        (width - if compact { 40.0 } else { 80.0 }).min(if count > 3 { 1420.0 } else { 1060.0 });
    let height = (height - if compact { 24.0 } else { 90.0 }).min(720.0);
    let header = if compact { 48.0 } else { 86.0 };
    let footer = if compact { 126.0 } else { 210.0 };
    let gap = if compact { 8.0 } else { 18.0 };
    let count = count.max(1);
    let card_width = (width - gap * (count - 1) as f32 - 1.0) / count as f32;
    let card_height = height - header - footer - gap * 2.0;
    let icon = (card_width * 0.8)
        .min(card_height - if compact { 74.0 } else { 112.0 })
        .clamp(32.0, 290.0);
    Layout {
        width,
        height,
        header,
        footer,
        gap,
        card_width,
        card_height,
        icon,
        compact,
        count,
    }
}

pub(super) fn update_layout(
    draft: Res<DraftState>,
    windows: Query<&Window, With<PrimaryWindow>>,
    target: Option<Res<CaptureTarget>>,
    images: Option<Res<Assets<Image>>>,
    mut nodes: Query<(
        &mut Node,
        Has<BuildContent>,
        Has<BuildHeader>,
        Has<BuildGallery>,
        Has<DraftButton>,
        Has<BuildFooter>,
        Has<BuildArt>,
    )>,
    mut fonts: Query<(
        &mut TextFont,
        Has<BuildHeading>,
        Has<BuildTitle>,
        Has<BuildDetailTitle>,
        Has<BuildDetail>,
        Has<BuildTradeLine>,
    )>,
) {
    if draft.root.is_none() {
        return;
    }
    let size = windows
        .single()
        .map(|w| (w.width(), w.height()))
        .unwrap_or_else(|_| {
            target
                .as_ref()
                .and_then(|t| images.as_ref()?.get(&t.0))
                .map(|i| (i.width() as f32, i.height() as f32))
                .unwrap_or((1280.0, 720.0))
        });
    let m = layout(size.0, size.1, draft.choices.len());
    // Encounter cards give the saved icon height to their benefit/cost lines.
    let icon = if draft.kind == Some(DraftKind::Encounter) {
        (m.icon * 0.55).max(40.0)
    } else {
        m.icon
    };
    for (mut node, content, header, gallery, card, footer, art) in &mut nodes {
        if content {
            node.width = px(m.width);
            node.height = px(m.height);
            node.row_gap = px(m.gap);
        }
        if header {
            node.height = px(m.header);
        }
        if gallery {
            node.height = px(m.card_height);
            node.column_gap = px(m.gap);
        }
        if card {
            node.width = px(m.card_width);
            node.height = px(m.card_height);
            node.row_gap = px(if m.compact { 7 } else { 14 });
        }
        if footer {
            node.height = px(m.footer);
            node.padding = UiRect::axes(px(if m.compact { 12 } else { 20 }), px(10));
        }
        if art {
            node.width = px(icon);
            node.height = px(icon);
        }
    }
    for (mut font, heading, title, detail_title, detail, trade) in &mut fonts {
        if heading {
            font.font_size = FontSize::Px(if m.compact { 22.0 } else { 32.0 });
        }
        if title {
            font.font_size = FontSize::Px(if m.compact && m.count > 3 {
                11.0
            } else if m.compact {
                16.0
            } else if m.count > 3 {
                17.0
            } else {
                23.0
            });
        }
        if detail_title {
            font.font_size = FontSize::Px(if m.compact { 16.0 } else { 22.0 });
        }
        if detail {
            font.font_size = FontSize::Px(if m.compact { 11.0 } else { 13.0 });
        }
        if trade {
            font.font_size = FontSize::Px(if m.compact { 10.0 } else { 13.0 });
        }
    }
}

fn summary(language: GameLanguage, choice: DraftChoice, build: &PlayerBuild) -> String {
    let (current, gain, cap) = progress(choice, build);
    let _ = (gain, cap);
    match choice {
        DraftChoice::Evolution {skill,tier} => crate::game::skill_evolution::description(language,skill,tier),
        DraftChoice::EvolveUr {skill,path} => crate::game::ur_evolution::description(language,skill,path),
        DraftChoice::ReplaceSkill {old,new} => format!("{} → {}\n{}",skill_name(language,old),skill_name(language,new),language.pick("Replace this learned skill with the UR skill at level 1. Its ascension is removed; earned permanent training is retained and cannot be earned twice.","以1级传说技能替换此已学技能，移除旧技能升华；已获永久训练保留且不可重复获取。")),
        DraftChoice::Bond(card) => {
            let def = card.definition();
            format!(
                "{}\n{}",
                language.pick(def.description_en, def.description),
                bond_family_detail(language, def.family, build)
            )
        }
        DraftChoice::ReplaceBond(family) => format!(
            "{}\n{}",
            language.pick(
                "Remove this family's active cards and ongoing effects.",
                "移除此羁绊的活动卡及持续效果。"
            ),
            bond_family_detail(language, family, build)
        ),
        DraftChoice::Skill(skill) => crate::game::skill_details::simple(language, skill, current),
        DraftChoice::Encounter(event) => trade_line(language, event),
    }
}

fn short_summary(language: GameLanguage, choice: DraftChoice, build: &PlayerBuild) -> String {
    match choice {
        DraftChoice::Bond(card) => {
            let def = card.definition();
            language.pick(def.description_en, def.description).into()
        }
        DraftChoice::EvolveUr { skill, path } => {
            crate::game::ur_evolution::description(language, skill, path)
                .lines()
                .nth(1)
                .unwrap_or_default()
                .into()
        }
        DraftChoice::ReplaceBond(_) => language
            .pick(
                "Remove this family's active cards and effects.",
                "移除此羁绊的活动卡及持续效果。",
            )
            .into(),
        DraftChoice::ReplaceSkill { old, new } => format!(
            "{} → {} · {}",
            skill_name(language, old),
            skill_name(language, new),
            crate::game::skill_details::simple(language, new, 0)
        ),
        DraftChoice::Evolution { skill, tier } => {
            crate::game::skill_evolution::description(language, skill, tier)
                .lines()
                .next()
                .unwrap_or_default()
                .into()
        }
        DraftChoice::Skill(_) | DraftChoice::Encounter(_) => summary(language, choice, build),
    }
}

pub(super) fn update_feedback(
    draft: Res<DraftState>,
    preview: Res<BuildPreview>,
    build: Res<PlayerBuild>,
    session: Res<GameSession>,
    language: Res<GameLanguage>,
    detail: Option<Res<crate::game::selection_detail::SelectionDetail>>,
    vitals: Option<Res<crate::game::HeroVitals>>,
    runtime: Option<Res<crate::game::skills::SkillRuntime>>,
    mut cards: Query<(
        &DraftButton,
        &Interaction,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
    mut actions: Query<
        (
            &DraftActionButton,
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        Without<DraftButton>,
    >,
    mut pips: Query<
        (&BuildPip, &mut BackgroundColor, &mut BorderColor),
        (Without<DraftButton>, Without<DraftActionButton>),
    >,
    mut labels: Query<
        (&mut Text, Has<BuildDetailTitle>, Has<BuildDetail>),
        Without<BuildActionHint>,
    >,
    mut hints: Query<(&mut Text, &mut Node), With<BuildActionHint>>,
) {
    if draft.root.is_none() {
        return;
    }
    let mut hovered_index = None;
    for (button, interaction, mut bg, mut border) in &mut cards {
        let selected = preview.root == draft.root && preview.index == Some(button.0);
        let hovered = *interaction != Interaction::None;
        if hovered {
            hovered_index = Some(button.0);
        }
        let locked = draft
            .choices
            .get(button.0)
            .is_some_and(|choice| !choice_available(*choice, &session));
        bg.0 = if locked {
            Color::srgb(0.03, 0.045, 0.05)
        } else if selected {
            Color::srgb(0.09, 0.15, 0.16)
        } else if hovered {
            Color::srgb(0.07, 0.12, 0.14)
        } else {
            PANEL
        };
        border.set_all(if locked {
            Color::srgb(0.16, 0.2, 0.21)
        } else if selected {
            GOLD
        } else if hovered {
            IVORY
        } else {
            EDGE
        });
    }
    let mut help = None;
    for (action, interaction, mut bg, mut border) in &mut actions {
        let hover = *interaction != Interaction::None;
        bg.0 = if hover {
            Color::srgb(0.12, 0.17, 0.18)
        } else {
            PANEL
        };
        border.set_all(if hover { GOLD } else { EDGE });
        if hover {
            help = Some(match action.0 {
                DraftAction::Refresh => {
                    if language.is_chinese() {
                        format!("刷新选项 · 剩余 {} 次", session.skill_rerolls)
                    } else {
                        format!("Refresh these offers · {} remaining", session.skill_rerolls)
                    }
                }
                DraftAction::Abandon => language
                    .pick(
                        "Abandon this draw to gain one refresh.",
                        "放弃本次选择，获得一次刷新。",
                    )
                    .into(),
                DraftAction::Later => language
                    .pick(
                        "Keep these offers and return to battle. No second payment.",
                        "保留选项并返回战场，下次无需再次付费。",
                    )
                    .into(),
            });
        }
    }
    for (pip, mut bg, mut border) in &mut pips {
        let (owned, projected) = match pip.choice {
            DraftChoice::Skill(_)
            | DraftChoice::Evolution { .. }
            | DraftChoice::EvolveUr { .. }
            | DraftChoice::ReplaceSkill { .. } => {
                let (current, gain, cap) = progress(pip.choice, &build);
                (pip.point < current, pip.point < (current + gain).min(cap))
            }
            DraftChoice::Bond(card) => {
                let member = card.definition().family.definition().members[pip.point as usize];
                (build.owned_bond_cards[member.index()], member == card)
            }
            DraftChoice::ReplaceBond(family) => {
                let member = family.definition().members[pip.point as usize];
                (build.owned_bond_cards[member.index()], false)
            }
            DraftChoice::Encounter(_) => (false, false),
        };
        bg.0 = if owned {
            IVORY
        } else if projected {
            GOLD
        } else {
            EDGE
        };
        border.set_all(if projected { GOLD } else { MUTED });
    }
    for (mut text, mut node) in &mut hints {
        node.display = if help.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        **text = help.clone().unwrap_or_default();
    }
    if let Some(choice) = hovered_index
        .or(preview.index)
        .and_then(|index| draft.choices.get(index))
        .copied()
    {
        let title = name(*language, choice);
        let description = if let Some(new_bond) = draft.pending_bond_replacement {
            if language.is_chinese() {
                format!(
                    "移除此羁绊的活动卡及持续效果，加入 {}",
                    bond_card_name(*language, new_bond)
                )
            } else {
                format!(
                    "Remove this family’s active cards and ongoing effects to add {}",
                    bond_card_name(*language, new_bond)
                )
            }
        } else {
            match (
                choice,
                detail.as_ref().is_none_or(|mode| mode.0),
                vitals.as_deref(),
                runtime.as_deref(),
            ) {
                (choice, false, _, _) => short_summary(*language, choice, &build),
                (DraftChoice::ReplaceSkill { new, .. }, true, Some(vitals), Some(runtime)) => {
                    format!(
                        "{}\n\n{}",
                        summary(*language, choice, &build),
                        crate::game::skill_details::detailed(
                            *language, new, 0, &build, vitals, runtime
                        )
                    )
                }
                (
                    DraftChoice::Evolution { skill, .. } | DraftChoice::EvolveUr { skill, .. },
                    true,
                    Some(vitals),
                    Some(runtime),
                ) => format!(
                    "{}\n\n{}\n{}",
                    summary(*language, choice, &build),
                    language.pick("Base skill", "基础技能"),
                    crate::game::skill_details::detailed(
                        *language,
                        skill,
                        build.skill_rank(skill),
                        &build,
                        vitals,
                        runtime
                    )
                ),
                (DraftChoice::Skill(skill), true, Some(vitals), Some(runtime)) => {
                    crate::game::skill_details::detailed(
                        *language,
                        skill,
                        build.skill_rank(skill),
                        &build,
                        vitals,
                        runtime,
                    )
                }
                (DraftChoice::Encounter(event), _, _, _) if !event.affordable(&session) => {
                    format!(
                        "{}\n{}",
                        summary(*language, choice, &build),
                        language.pick("Not enough gold.", "金币不足。")
                    )
                }
                _ => summary(*language, choice, &build),
            }
        };
        for (mut text, is_title, is_detail) in &mut labels {
            if is_title {
                **text = title.clone();
            }
            if is_detail {
                **text = description.clone();
            }
        }
    }
}

/// Only gold-priced encounters can be unavailable; every other choice is always live.
fn choice_available(choice: DraftChoice, session: &GameSession) -> bool {
    match choice {
        DraftChoice::Encounter(event) => event.affordable(session),
        _ => true,
    }
}

pub(super) fn scroll_details(
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    touches: Res<Touches>,
    mut bodies: Query<&mut ScrollPosition, With<BuildDetailScroll>>,
) {
    let delta: f32 = wheel
        .read()
        .map(|w| {
            -w.y * if w.unit == bevy::input::mouse::MouseScrollUnit::Line {
                24.0
            } else {
                1.0
            }
        })
        .sum();
    let drag = touches.iter().map(|touch| -touch.delta().y).sum::<f32>();
    for mut position in &mut bodies {
        position.0.y = (position.0.y + delta + drag).max(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::SkillId;
    #[test]
    fn simple_mode_also_shortens_bond_and_ur_evolution_choices() {
        let build = PlayerBuild::default();
        for language in [GameLanguage::English, GameLanguage::SimplifiedChinese] {
            let card = crate::game::BondCardId::DivineMight;
            let choice = DraftChoice::Bond(card);
            let simple = short_summary(language, choice, &build);
            let full = summary(language, choice, &build);
            assert!(full.starts_with(&simple) && full.len() > simple.len());
            let choice = DraftChoice::EvolveUr {
                skill: SkillId(26),
                path: crate::game::ur_evolution::UrEvolutionPath::Control,
            };
            let simple = short_summary(language, choice, &build);
            let full = summary(language, choice, &build);
            assert!(full.contains(&simple) && full.len() > simple.len());
            assert!(!simple.contains('\n'));
        }
    }
    #[test]
    fn hover_details_follow_each_card_and_action_help_stays_separate() {
        use crate::game::BondCardId;
        let mut app = App::new();
        app.init_resource::<PlayerBuild>()
            .init_resource::<GameSession>()
            .insert_resource(GameLanguage::SimplifiedChinese)
            .add_systems(Update, update_feedback);
        let root = app.world_mut().spawn_empty().id();
        let choices = [
            BondCardId::DivineMight,
            BondCardId::DivineBody,
            BondCardId::DivineAgility,
        ];
        app.insert_resource(DraftState {
            root: Some(root),
            choices: choices.into_iter().map(DraftChoice::Bond).collect(),
            ..default()
        })
        .insert_resource(BuildPreview {
            root: Some(root),
            index: Some(2),
        });
        let title = app
            .world_mut()
            .spawn((BuildDetailTitle, Text::default()))
            .id();
        let detail = app.world_mut().spawn((BuildDetail, Text::default())).id();
        let hint = app
            .world_mut()
            .spawn((BuildActionHint, Text::default(), Node::default()))
            .id();
        let cards: Vec<_> = (0..3)
            .map(|index| {
                app.world_mut()
                    .spawn((
                        DraftButton(index),
                        Interaction::None,
                        BackgroundColor::default(),
                        BorderColor::default(),
                    ))
                    .id()
            })
            .collect();
        for (index, card) in choices.iter().enumerate() {
            app.world_mut()
                .entity_mut(cards[index])
                .insert(Interaction::Hovered);
            app.update();
            assert_eq!(
                app.world().get::<Text>(title).unwrap().0,
                card.definition().name
            );
            assert!(
                app.world()
                    .get::<Text>(detail)
                    .unwrap()
                    .0
                    .contains(card.definition().description)
            );
            assert_eq!(app.world().resource::<BuildPreview>().index, Some(2));
            app.world_mut()
                .entity_mut(cards[index])
                .insert(Interaction::None);
        }
        let action = app
            .world_mut()
            .spawn((
                DraftActionButton(DraftAction::Abandon),
                Interaction::Hovered,
                BackgroundColor::default(),
                BorderColor::default(),
            ))
            .id();
        app.update();
        assert_eq!(app.world().get::<Text>(title).unwrap().0, "神性");
        assert!(
            app.world()
                .get::<Text>(detail)
                .unwrap()
                .0
                .contains(choices[2].definition().description)
        );
        assert_eq!(
            app.world().get::<Text>(hint).unwrap().0,
            "放弃本次选择，获得一次刷新。"
        );
        assert_eq!(
            app.world().get::<Node>(hint).unwrap().display,
            Display::Flex
        );
        app.world_mut().entity_mut(action).insert(Interaction::None);
        app.insert_resource(GameLanguage::English);
        app.update();
        assert_eq!(app.world().get::<Text>(title).unwrap().0, "Divine Grace");
        assert!(
            app.world()
                .get::<Text>(detail)
                .unwrap()
                .0
                .contains(choices[2].definition().description_en)
        );
        assert_eq!(
            app.world().get::<Node>(hint).unwrap().display,
            Display::None
        );
    }
    #[test]
    fn build_cards_and_actions_fit_all_supported_landscape_sizes() {
        for (width, height) in [
            (1280.0, 720.0),
            (1760.0, 1350.0),
            (844.0, 390.0),
            (667.0, 375.0),
        ] {
            for count in [3, 7] {
                let m = layout(width, height, count);
                assert!(m.card_width >= 80.0);
                assert!(m.card_height >= 150.0);
                assert!(m.icon >= 60.0);
                assert!(m.card_width * count as f32 + m.gap * (count - 1) as f32 <= m.width);
                assert!(m.height <= height);
            }
        }
    }
}

#[derive(Component)]
pub(super) enum BuildCopy {
    Heading(DraftKind),
    Card(DraftChoice),
    Trade(EncounterId),
}

pub(super) fn refresh_language(
    language: Res<GameLanguage>,
    draft: Res<DraftState>,
    mut labels: Query<(&BuildCopy, &mut Text)>,
) {
    if !language.is_changed() {
        return;
    }
    for (copy, mut text) in &mut labels {
        **text = match copy {
            BuildCopy::Card(choice) => short_name(*language, *choice),
            BuildCopy::Trade(event) => trade_line(*language, *event),
            BuildCopy::Heading(_)
                if matches!(draft.choices.first(), Some(DraftChoice::EvolveUr { .. })) =>
            {
                let Some(DraftChoice::EvolveUr { skill, .. }) = draft.choices.first() else {
                    unreachable!()
                };
                format!(
                    "{} · {}",
                    skill_name(*language, *skill),
                    language.pick("Choose an evolution", "选择进化方向")
                )
            }
            BuildCopy::Heading(kind) => heading_copy(*language, *kind).into(),
        };
    }
}

#[cfg(test)]
mod live_language_tests {
    use super::*;
    use crate::game::{BondId, RunRng, SkillId};

    #[test]
    fn existing_skill_bond_and_replacement_modals_translate_without_resetting_choices() {
        for kind in [
            DraftKind::Skill,
            DraftKind::Bond,
            DraftKind::BondReplacement,
            DraftKind::Encounter,
        ] {
            let mut app = App::new();
            app.init_resource::<GameLanguage>()
                .init_resource::<PlayerBuild>()
                .init_resource::<GameSession>()
                .init_resource::<RunRng>()
                .add_systems(Update, (refresh_language, update_feedback));
            let choices = if kind == DraftKind::Skill {
                vec![
                    DraftChoice::Skill(SkillId(0)),
                    DraftChoice::Skill(SkillId(1)),
                ]
            } else if kind == DraftKind::BondReplacement {
                vec![
                    DraftChoice::ReplaceBond(BondId::Journey),
                    DraftChoice::ReplaceBond(BondId::Agility),
                ]
            } else if kind == DraftKind::Encounter {
                vec![
                    DraftChoice::Encounter(EncounterId::BloodOath),
                    DraftChoice::Encounter(EncounterId::ArmoryDonation),
                ]
            } else {
                vec![
                    DraftChoice::Bond(BondId::Journey.definition().members[0]),
                    DraftChoice::Bond(BondId::Journey.definition().members[1]),
                ]
            };
            let mut draft = DraftState {
                choices: choices.clone(),
                kind: Some(kind),
                opened_for: 0.7,
                pending_bond_replacement: (kind == DraftKind::BondReplacement)
                    .then_some(BondId::Agility.definition().members[0]),
                ..default()
            };
            {
                let mut commands = app.world_mut().commands();
                spawn_draft_ui(&mut commands, &mut draft, kind, 2, GameLanguage::English);
            }
            app.world_mut().flush();
            let root = draft.root;
            app.insert_resource(BuildPreview {
                root,
                index: Some(1),
            })
            .insert_resource(draft);
            {
                let mut build = app.world_mut().resource_mut::<PlayerBuild>();
                build.owned_bond_cards[BondId::Journey.definition().members[0].index()] = true;
                build.skills[0] = 2;
            }
            {
                let mut session = app.world_mut().resource_mut::<GameSession>();
                session.paused = true;
                session.gold = 123;
                session.wood = 45;
                session.skill_rerolls = 2;
            }
            let rng = app.world().resource::<RunRng>().0;
            for language in [GameLanguage::SimplifiedChinese, GameLanguage::English] {
                *app.world_mut().resource_mut::<GameLanguage>() = language;
                app.update();
                let draft = app.world().resource::<DraftState>();
                assert_eq!(draft.root, root);
                assert_eq!(draft.choices, choices);
                assert_eq!(draft.opened_for, 0.7);
                assert_eq!(draft.kind, Some(kind));
                assert_eq!(
                    draft.pending_bond_replacement,
                    (kind == DraftKind::BondReplacement)
                        .then_some(BondId::Agility.definition().members[0])
                );
                assert_eq!(app.world().resource::<BuildPreview>().index, Some(1));
                assert_eq!(
                    app.world().resource::<PlayerBuild>().bond(BondId::Journey),
                    1
                );
                assert_eq!(app.world().resource::<PlayerBuild>().skills[0], 2);
                let session = app.world().resource::<GameSession>();
                assert!(session.paused);
                assert_eq!(
                    (session.gold, session.wood, session.skill_rerolls),
                    (123, 45, 2)
                );
                assert_eq!(app.world().resource::<RunRng>().0, rng);
                let titles: Vec<_> = app
                    .world_mut()
                    .query_filtered::<&Text, With<BuildTitle>>()
                    .iter(app.world())
                    .map(|text| text.0.clone())
                    .collect();
                assert!(
                    choices
                        .iter()
                        .all(|choice| titles.contains(&short_name(language, *choice)))
                );
                if kind == DraftKind::Encounter {
                    let trades: Vec<_> = app
                        .world_mut()
                        .query_filtered::<&Text, With<BuildTradeLine>>()
                        .iter(app.world())
                        .map(|text| text.0.clone())
                        .collect();
                    assert_eq!(trades.len(), choices.len());
                    assert!(trades.contains(&trade_line(language, EncounterId::BloodOath)));
                    assert!(
                        trades
                            .iter()
                            .all(|line| line.contains(language.pick("Cost:", "代价：")))
                    );
                }
            }
        }
    }
}
