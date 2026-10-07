//! Recruitment presentation. Offers and commitment remain in the hero gameplay module.
use bevy::{prelude::*, window::PrimaryWindow};

use super::{HeroChoiceButton, HeroOffer, HeroSelectorRoot, HeroState};
use crate::game::content::{Attribute, HEROES, SKILLS};
use crate::game::localization::{hero_name, skill_name};
use crate::game::rarity::rarity_color;
use crate::game::{CaptureTarget, GameLanguage, HeroKind, SkillId};

const GOLD: Color = Color::srgb(0.89, 0.71, 0.40);
const IVORY: Color = Color::srgb(0.96, 0.92, 0.82);
const MUTED: Color = Color::srgb(0.59, 0.66, 0.66);
const EDGE: Color = Color::srgb(0.25, 0.30, 0.31);
const PANEL: Color = Color::srgb(0.055, 0.078, 0.086);

#[derive(Resource, Default)]
pub(in crate::game) struct DraftArt {
    pub(in crate::game) portraits: [Handle<Image>; HeroKind::COUNT],
    pub(in crate::game) backdrop: Handle<Image>,
}

pub(super) fn load_art(mut commands: Commands, server: Res<AssetServer>) {
    let names = [
        "storm_savant",
        "ember_ranger",
        "stone_warden",
        "spirit_keeper",
        "tide_walker",
        "star_wagerer",
        "frost_oracle",
        "iron_berserker",
        "rift_dancer",
        "grove_artificer",
        "solar_vanguard",
        "void_reaper",
    ];
    commands.insert_resource(DraftArt {
        portraits: names.map(|name| server.load(format!("ui/portraits/hero_{name}.png"))),
        backdrop: server.load("ui/draft_backdrop.png"),
    });
}

#[derive(Component)]
pub(super) struct DraftContent;
#[derive(Component)]
pub(in crate::game) struct DraftHeader;
#[derive(Component)]
pub(super) struct DraftGallery;
#[derive(Component)]
pub(super) struct DraftFooter;
#[derive(Component)]
pub(super) struct DraftHeading;
#[derive(Component)]
pub(super) struct DraftEyebrow;
#[derive(Component)]
pub(super) struct DraftCardName;
#[derive(Component)]
pub(super) struct DraftPortrait;
#[derive(Component)]
pub(super) struct DraftCardCaption;
#[derive(Component)]
pub(super) struct DraftDetailTitle;
#[derive(Component)]
pub(super) struct FusionPortrait(bool);
#[derive(Component)]
pub(super) struct FusionArrow;
#[derive(Component)]
pub(super) struct DraftDetailSkill;
#[derive(Component)]
pub(super) struct DraftDetailRole;
#[derive(Component)]
pub(super) struct DraftSelectedMark(HeroKind);

#[derive(Clone, Copy)]
struct Metrics {
    width: f32,
    height: f32,
    card_width: f32,
    card_height: f32,
    header: f32,
    footer: f32,
    gap: f32,
    compact: bool,
    portrait: bool,
}

fn metrics(width: f32, height: f32, count: usize) -> Metrics {
    let compact = height < 540.0;
    let portrait = width < 620.0 && height > width;
    let margin = if compact {
        24.0
    } else if portrait {
        16.0
    } else {
        40.0
    };
    let width = (width - margin * 2.0).min(1480.0);
    let height = (height - if compact { 24.0 } else { 64.0 }).min(850.0);
    let header = if compact {
        52.0
    } else if portrait {
        90.0
    } else {
        100.0
    };
    let footer = if compact {
        118.0
    } else if portrait {
        156.0
    } else {
        132.0
    };
    let gap = if compact || portrait { 10.0 } else { 18.0 };
    let columns = if portrait {
        count.clamp(1, 3) as f32
    } else {
        count.max(1) as f32
    };
    let rows = (count.max(1) as f32 / columns).ceil();
    Metrics {
        width,
        height,
        // Reserve a pixel so fractional layout rounding cannot wrap the last card.
        card_width: (width - gap * (columns - 1.0) - 1.0) / columns,
        card_height: (height - header - footer - gap * (rows + 1.0)) / rows,
        header,
        footer,
        gap,
        compact,
        portrait,
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

pub(super) fn spawn_hero_selector(
    commands: &mut Commands,
    offers: &[HeroOffer],
    art: &DraftArt,
    language: GameLanguage,
) -> Entity {
    commands
        .spawn((
            HeroSelectorRoot,
            Name::new("Commander recruitment"),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.018, 0.029, 0.038)),
            GlobalZIndex(100),
        ))
        .with_children(|root| {
            root.spawn((
                ImageNode::new(art.backdrop.clone()).with_color(Color::srgb(0.56, 0.63, 0.66)),
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
            ));
            root.spawn((
                DraftContent,
                Node {
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
            ))
            .with_children(|content| {
                content
                    .spawn((
                        DraftHeader,
                        Node {
                            width: percent(100),
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::Center,
                            row_gap: px(6),
                            flex_shrink: 0.0,
                            ..default()
                        },
                    ))
                    .with_children(|titles| {
                        titles.spawn((
                            DraftEyebrow,
                            DraftCopy::Eyebrow,
                            label(
                                language.pick("THE LAST BASTION", "最后的堡垒 · 指挥官征募"),
                                12.0,
                                GOLD,
                            ),
                        ));
                        titles.spawn((
                            DraftHeading,
                            Name::new("Commander selection heading"),
                            DraftCopy::Heading,
                            label(
                                language.pick("Choose your commander", "选择你的指挥官"),
                                36.0,
                                IVORY,
                            ),
                        ));
                    });
                content
                    .spawn((
                        DraftGallery,
                        Node {
                            width: percent(100),
                            flex_wrap: FlexWrap::Wrap,
                            flex_shrink: 0.0,
                            ..default()
                        },
                    ))
                    .with_children(|gallery| {
                        for (index, offer) in offers.iter().copied().enumerate() {
                            let hero = HEROES[offer.kind.index()];
                            gallery
                                .spawn((
                                    Button,
                                    HeroChoiceButton(offer),
                                    super::HeroChoiceOrdinal(index),
                                    Name::new(format!("Commander card {index}")),
                                    Node {
                                        flex_direction: FlexDirection::Column,
                                        flex_shrink: 0.0,
                                        border: UiRect::all(px(2)),
                                        overflow: Overflow::clip(),
                                        ..default()
                                    },
                                    BackgroundColor(PANEL),
                                    BorderColor::all(EDGE),
                                ))
                                .with_children(|card| {
                                    card.spawn((
                                        Node {
                                            flex_grow: 1.0,
                                            width: percent(100),
                                            min_height: px(0),
                                            align_items: AlignItems::Center,
                                            justify_content: JustifyContent::Center,
                                            overflow: Overflow::clip(),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgb(0.075, 0.11, 0.12)),
                                    ))
                                    .with_children(
                                        |art_area| {
                                            art_area.spawn((
                                                DraftPortrait,
                                                ImageNode::new(
                                                    art.portraits[offer.kind.index()].clone(),
                                                ),
                                                Node { ..default() },
                                            ));
                                        },
                                    );
                                    card.spawn((
                                        DraftCardCaption,
                                        Node {
                                            width: percent(100),
                                            flex_shrink: 0.0,
                                            flex_direction: FlexDirection::Column,
                                            align_items: AlignItems::Center,
                                            justify_content: JustifyContent::Center,
                                            row_gap: px(5),
                                            padding: UiRect::axes(px(3), px(7)),
                                            ..default()
                                        },
                                    ))
                                    .with_children(
                                        |caption| {
                                            caption.spawn((
                                                DraftCardName,
                                                DraftCopy::Name(offer.kind),
                                                label(hero_name(language, offer.kind), 17.0, IVORY),
                                                TextLayout::justify(Justify::Center)
                                                    .with_linebreak(LineBreak::WordOrCharacter),
                                            ));
                                            caption.spawn((
                                                DraftCopy::Role(hero.primary),
                                                label(
                                                    role_name(language, hero.primary),
                                                    11.0,
                                                    MUTED,
                                                ),
                                            ));
                                        },
                                    );
                                    card.spawn((
                                        Node {
                                            position_type: PositionType::Absolute,
                                            top: px(9),
                                            left: px(9),
                                            padding: UiRect::axes(px(7), px(4)),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgba(0.02, 0.04, 0.045, 0.9)),
                                    ))
                                    .with_children(|badge| {
                                        badge.spawn(label(
                                            hero.rarity.label(),
                                            10.0,
                                            rarity_color(hero.rarity),
                                        ));
                                    });
                                    card.spawn((
                                        DraftSelectedMark(offer.kind),
                                        Node {
                                            position_type: PositionType::Absolute,
                                            top: px(9),
                                            right: px(9),
                                            width: px(22),
                                            height: px(22),
                                            align_items: AlignItems::Center,
                                            justify_content: JustifyContent::Center,
                                            ..default()
                                        },
                                        BackgroundColor(GOLD),
                                        Visibility::Hidden,
                                    ))
                                    .with_children(|mark| {
                                        mark.spawn(label("+", 17.0, Color::srgb(0.06, 0.08, 0.08)));
                                    });
                                });
                        }
                    });
                content
                    .spawn((
                        DraftFooter,
                        Name::new("Commander details footer"),
                        Node {
                            width: percent(100),
                            flex_shrink: 0.0,
                            padding: UiRect::axes(px(22), px(16)),
                            border: UiRect::top(px(1)),
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            column_gap: px(20),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.025, 0.045, 0.05, 0.96)),
                        BorderColor::all(GOLD),
                    ))
                    .with_children(|footer| {
                        footer
                            .spawn(Node {
                                flex_direction: FlexDirection::Column,
                                flex_grow: 1.0,
                                flex_shrink: 1.0,
                                min_width: px(0),
                                row_gap: px(6),
                                ..default()
                            })
                            .with_children(|details| {
                                details
                                    .spawn(Node {
                                        align_items: AlignItems::Center,
                                        column_gap: px(6),
                                        ..default()
                                    })
                                    .with_children(|line| {
                                        line.spawn((
                                            FusionPortrait(false),
                                            ImageNode::default(),
                                            Node {
                                                width: px(28),
                                                height: px(28),
                                                display: Display::None,
                                                flex_shrink: 0.0,
                                                ..default()
                                            },
                                            Pickable::IGNORE,
                                        ));
                                        line.spawn((
                                            FusionArrow,
                                            label("+", 18.0, GOLD),
                                            Node {
                                                display: Display::None,
                                                ..default()
                                            },
                                        ));
                                        line.spawn((
                                            FusionPortrait(true),
                                            ImageNode::default(),
                                            Node {
                                                width: px(28),
                                                height: px(28),
                                                display: Display::None,
                                                flex_shrink: 0.0,
                                                ..default()
                                            },
                                            Pickable::IGNORE,
                                        ));
                                        line.spawn((DraftDetailTitle, label("", 24.0, IVORY)));
                                    });
                                details.spawn((DraftDetailSkill, label("", 13.0, GOLD)));
                                details.spawn((DraftDetailRole, label("", 12.0, MUTED)));
                            });
                    });
            });
        })
        .id()
}

fn role_name(language: GameLanguage, primary: Attribute) -> &'static str {
    match primary {
        Attribute::Strength => language.pick("VANGUARD", "先锋"),
        Attribute::Agility => language.pick("SKIRMISHER", "游击"),
        Attribute::Intelligence => language.pick("SPELLCASTER", "施法者"),
    }
}

pub(super) fn update_layout(
    choices: Query<&HeroChoiceButton>,
    windows: Query<&Window, With<PrimaryWindow>>,
    target: Option<Res<CaptureTarget>>,
    images: Res<Assets<Image>>,
    mut nodes: Query<(
        &mut Node,
        Has<DraftContent>,
        Has<DraftHeader>,
        Has<DraftGallery>,
        Has<HeroChoiceButton>,
        Has<DraftFooter>,
        Has<DraftPortrait>,
        Has<DraftCardCaption>,
        Has<DraftEyebrow>,
        Has<DraftDetailRole>,
    )>,
    mut fonts: Query<(
        &mut TextFont,
        Has<DraftHeading>,
        Has<DraftCardName>,
        Has<DraftDetailTitle>,
        Has<DraftDetailSkill>,
    )>,
) {
    let (width, height) = windows
        .single()
        .map(|window| (window.width(), window.height()))
        .unwrap_or_else(|_| {
            target
                .as_ref()
                .and_then(|target| images.get(&target.0))
                .map(|image| (image.width() as f32, image.height() as f32))
                .unwrap_or((1280.0, 720.0))
        });
    let m = metrics(width, height, choices.iter().count());
    for (
        mut node,
        content,
        header,
        gallery,
        card,
        footer,
        portrait,
        caption,
        eyebrow,
        detail_role,
    ) in &mut nodes
    {
        if content {
            node.reborrow()
                .map_unchanged(|value| &mut value.width)
                .set_if_neq(px(m.width));
            node.reborrow()
                .map_unchanged(|value| &mut value.height)
                .set_if_neq(px(m.height));
            node.reborrow()
                .map_unchanged(|value| &mut value.row_gap)
                .set_if_neq(px(m.gap));
        }
        if header {
            node.reborrow()
                .map_unchanged(|value| &mut value.height)
                .set_if_neq(px(m.header));
        }
        if gallery {
            node.reborrow()
                .map_unchanged(|value| &mut value.flex_wrap)
                .set_if_neq(if m.portrait {
                    FlexWrap::Wrap
                } else {
                    FlexWrap::NoWrap
                });
            node.reborrow()
                .map_unchanged(|value| &mut value.height)
                .set_if_neq(px(m.height - m.header - m.footer - m.gap * 2.0));
            node.reborrow()
                .map_unchanged(|value| &mut value.column_gap)
                .set_if_neq(px(m.gap));
            node.reborrow()
                .map_unchanged(|value| &mut value.row_gap)
                .set_if_neq(px(m.gap));
        }
        if card {
            node.reborrow()
                .map_unchanged(|value| &mut value.width)
                .set_if_neq(px(m.card_width));
            node.reborrow()
                .map_unchanged(|value| &mut value.height)
                .set_if_neq(px(m.card_height));
        }
        if portrait {
            let art_height = m.card_height - if m.compact || m.portrait { 60.0 } else { 74.0 };
            let h = (m.card_width * 1.65).min(art_height + 12.0);
            node.reborrow()
                .map_unchanged(|value| &mut value.width)
                .set_if_neq(px(h * 0.8));
            node.reborrow()
                .map_unchanged(|value| &mut value.height)
                .set_if_neq(px(h));
            node.reborrow()
                .map_unchanged(|value| &mut value.flex_shrink)
                .set_if_neq(0.0);
        }
        if caption {
            node.reborrow()
                .map_unchanged(|value| &mut value.height)
                .set_if_neq(px(if m.compact || m.portrait { 60.0 } else { 74.0 }));
        }
        if footer {
            node.reborrow()
                .map_unchanged(|value| &mut value.height)
                .set_if_neq(px(m.footer));
            node.reborrow()
                .map_unchanged(|value| &mut value.padding)
                .set_if_neq(UiRect::axes(
                    px(if m.compact { 12 } else { 22 }),
                    px(if m.compact { 8 } else { 14 }),
                ));
            node.reborrow()
                .map_unchanged(|value| &mut value.flex_direction)
                .set_if_neq(if m.portrait {
                    FlexDirection::Column
                } else {
                    FlexDirection::Row
                });
            node.reborrow()
                .map_unchanged(|value| &mut value.align_items)
                .set_if_neq(if m.portrait {
                    AlignItems::Stretch
                } else {
                    AlignItems::Center
                });
            node.reborrow()
                .map_unchanged(|value| &mut value.row_gap)
                .set_if_neq(px(10));
        }
        if eyebrow {
            node.reborrow()
                .map_unchanged(|value| &mut value.display)
                .set_if_neq(if m.compact {
                    Display::None
                } else {
                    Display::Flex
                });
        }
        if detail_role {
            node.reborrow()
                .map_unchanged(|value| &mut value.display)
                .set_if_neq(Display::Flex);
        }
    }
    for (mut font, heading, card, title, skill) in &mut fonts {
        if heading {
            font.reborrow()
                .map_unchanged(|value| &mut value.font_size)
                .set_if_neq(FontSize::Px(if m.compact || m.portrait {
                    22.0
                } else {
                    36.0
                }));
        }
        if card {
            font.reborrow()
                .map_unchanged(|value| &mut value.font_size)
                .set_if_neq(FontSize::Px(if m.compact || m.portrait {
                    12.0
                } else {
                    17.0
                }));
        }
        if title {
            font.reborrow()
                .map_unchanged(|value| &mut value.font_size)
                .set_if_neq(FontSize::Px(if m.compact || m.portrait {
                    18.0
                } else {
                    24.0
                }));
        }
        if skill {
            font.reborrow()
                .map_unchanged(|value| &mut value.font_size)
                .set_if_neq(FontSize::Px(if m.compact { 11.0 } else { 13.0 }));
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn update_feedback(
    state: Res<HeroState>,
    selected: Res<crate::game::SelectedHero>,
    art: Res<DraftArt>,
    mut fusion_portraits: Query<(&FusionPortrait, &mut ImageNode, &mut Node)>,
    mut fusion_arrows: Query<&mut Node, (With<FusionArrow>, Without<FusionPortrait>)>,
    recruitment: Option<Res<crate::game::RecruitmentProgress>>,
    language: Res<GameLanguage>,
    build: Res<crate::game::PlayerBuild>,
    vitals: Res<crate::game::HeroVitals>,
    bonds: Option<Res<crate::game::bonds::BondRuntime>>,
    mut cards: Query<(
        &HeroChoiceButton,
        &Interaction,
        &mut BorderColor,
        &mut BackgroundColor,
    )>,
    mut marks: Query<(&DraftSelectedMark, &mut Visibility)>,
    mut labels: Query<(
        &mut Text,
        Has<DraftDetailTitle>,
        Has<DraftDetailSkill>,
        Has<DraftDetailRole>,
    )>,
) {
    let preview = state.preview;
    let fusing = recruitment.as_ref().is_some_and(|r| r.pending.is_some());
    for (portrait, mut image, mut node) in &mut fusion_portraits {
        node.reborrow()
            .map_unchanged(|value| &mut value.display)
            .set_if_neq(if fusing { Display::Flex } else { Display::None });
        let kind = if portrait.0 {
            preview.map_or(selected.kind, |offer| offer.kind)
        } else {
            selected.kind
        };
        image.image = art.portraits[kind.index()].clone();
    }
    for mut node in &mut fusion_arrows {
        node.reborrow()
            .map_unchanged(|value| &mut value.display)
            .set_if_neq(if fusing { Display::Flex } else { Display::None });
    }
    for (choice, interaction, mut border, mut background) in &mut cards {
        let selected = preview == Some(choice.0);
        let hovered = *interaction != Interaction::None;
        border.set_all(if selected {
            GOLD
        } else if hovered {
            IVORY
        } else {
            EDGE
        });
        background.0 = if selected {
            Color::srgb(0.13, 0.15, 0.13)
        } else if hovered {
            Color::srgb(0.10, 0.14, 0.15)
        } else {
            PANEL
        };
    }
    for (mark, mut visible) in &mut marks {
        *visible = if preview.is_some_and(|offer| offer.kind == mark.0) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    if let Some(offer) = preview {
        let hero = HEROES[offer.kind.index()];
        let skill = SkillId(
            SKILLS
                .iter()
                .position(|skill| skill.id == offer.starting_skill)
                .expect("validated starting skill") as u8,
        );
        for (mut text, title, starting, role) in &mut labels {
            if title {
                text.reborrow()
                    .map_unchanged(|value| &mut value.0)
                    .set_if_neq(hero_name(*language, offer.kind));
            }
            if starting {
                text.reborrow()
                    .map_unchanged(|value| &mut value.0)
                    .set_if_neq(
                        if recruitment.as_ref().is_some_and(|r| r.pending.is_some()) {
                            format!(
                                "{}  {} · {} {}",
                                language.pick("Inherited skill", "继承技能"),
                                skill_name(*language, skill),
                                language.pick("Rank", "等级"),
                                hero.rarity.starting_skill_rank()
                            )
                        } else if language.is_chinese() {
                            format!(
                                "初始技能  {} · 等级 {}",
                                skill_name(*language, skill),
                                hero.rarity.starting_skill_rank()
                            )
                        } else {
                            format!(
                                "Starting skill  {} · Rank {}",
                                skill_name(*language, skill),
                                hero.rarity.starting_skill_rank()
                            )
                        },
                    );
            }
            if role {
                text.reborrow()
                    .map_unchanged(|value| &mut value.0)
                    .set_if_neq(crate::game::basic_attack_details::summary(
                        *language,
                        offer.kind,
                        if fusing { vitals.level } else { 1 },
                        &build,
                        bonds.as_ref().map_or(0, |bonds| bonds.melee_cleave()),
                    ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opening_and_later_portraits_fit_supported_viewports() {
        for (width, height) in [
            (1280.0, 720.0),
            (1760.0, 1350.0),
            (844.0, 390.0),
            (390.0, 844.0),
            (667.0, 375.0),
        ] {
            for count in [3, 6] {
                let m = metrics(width, height, count);
                let cols = if m.portrait { 3.0 } else { count as f32 };
                assert!(m.card_width >= 90.0);
                assert!(m.card_height >= 160.0);
                assert!(m.card_width * cols + m.gap * (cols - 1.0) <= m.width + 0.01);
                assert!(m.height <= height);
            }
        }
    }
}

#[derive(Component)]
pub(super) enum DraftCopy {
    Eyebrow,
    Heading,
    Name(HeroKind),
    Role(Attribute),
}

pub(super) fn refresh_language(
    language: Res<GameLanguage>,
    recruitment: Option<Res<crate::game::RecruitmentProgress>>,
    mut labels: Query<(&DraftCopy, &mut Text)>,
) {
    let context = recruitment.as_ref().and_then(|r| r.pending);
    for (copy, mut text) in &mut labels {
        text.reborrow()
            .map_unchanged(|value| &mut value.0)
            .set_if_neq(match copy {
                DraftCopy::Eyebrow => language
                    .pick("THE LAST BASTION", "最后的堡垒 · 指挥官征募")
                    .into(),
                DraftCopy::Heading => match context {
                    Some(crate::game::RecruitmentSource::Milestone(level)) => format!(
                        "{} {level} · {}",
                        language.pick("Level", "等级"),
                        language.pick("Recruit a hero", "英雄征募")
                    ),
                    Some(crate::game::RecruitmentSource::Fruit) => language
                        .pick("Bond fruit · Recruit", "羁绊果实 · 英雄征募")
                        .into(),
                    None => language
                        .pick("Click a basic commander to begin", "点击基础指挥官开始远征")
                        .into(),
                },
                DraftCopy::Name(kind) => hero_name(*language, *kind),
                DraftCopy::Role(primary) => role_name(*language, *primary).into(),
            });
    }
}
