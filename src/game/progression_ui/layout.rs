use super::*;

#[derive(Clone, Copy)]
struct Layout {
    width: f32,
    height: f32,
    header: f32,
    footer: f32,
    gap: f32,
    card_width: f32,
    card_height: f32,
    gallery_width: f32,
    gallery_height: f32,
    phone: bool,
    icon: f32,
    compact: bool,
    stacked: bool,
    count: usize,
}
fn layout(width: f32, height: f32, count: usize) -> Layout {
    let phone = width < 560.0 && height > width;
    let viewport_width = width;
    let compact = height < 540.0;
    let width =
        (width - if compact { 40.0 } else { 80.0 }).min(if count > 3 { 1420.0 } else { 1060.0 });
    let stacked = width < 900.0;
    let height = (height
        - if compact {
            24.0
        } else if stacked {
            128.0
        } else {
            90.0
        })
    .min(720.0);
    let header = if compact {
        if !stacked {
            48.0
        } else if width < 420.0 {
            160.0
        } else {
            88.0
        }
    } else if width < 420.0 {
        208.0
    } else if stacked {
        128.0
    } else {
        86.0
    };
    let mut footer: f32 = if compact && stacked {
        96.0
    } else if compact {
        126.0
    } else {
        210.0
    };
    let gap = if compact || phone { 8.0 } else { 18.0 };
    if phone {
        footer = footer.min((height - header - 2.0 * gap - 224.0).max(96.0));
    }
    let count = count.max(1);
    let gallery_width = if phone { viewport_width - 16.0 } else { width };
    let columns = if phone { count.min(3) } else { count };
    let card_width = (gallery_width - gap * (columns - 1) as f32 - 1.0) / columns as f32;
    let gallery_height = height - header - footer - gap * 2.0;
    let card_height = if phone { 224.0 } else { gallery_height };
    let icon = if phone {
        40.0
    } else {
        (card_width * 0.8)
            .min(card_height - if compact { 74.0 } else { 112.0 })
            .clamp(32.0, 290.0)
    };
    Layout {
        width,
        height,
        header,
        footer,
        gap,
        card_width,
        card_height,
        gallery_width,
        gallery_height,
        phone,
        icon,
        compact,
        stacked,
        count,
    }
}

pub(crate) fn update_layout(
    draft: Res<DraftState>,
    language: Res<GameLanguage>,
    windows: Query<&Window, With<PrimaryWindow>>,
    target: Option<Res<CaptureTarget>>,
    images: Option<Res<Assets<Image>>>,
    mut nodes: Query<(
        &mut Node,
        Has<BuildContent>,
        Has<BuildHeader>,
        Has<BuildHeading>,
        Has<BuildHeaderActions>,
        Has<BuildGallery>,
        Has<DraftButton>,
        Has<BuildFooter>,
        Has<BuildArt>,
        Has<BuildTitle>,
        Has<BuildGalleryHint>,
    )>,
    mut title_layouts: Query<&mut TextLayout, With<BuildTitle>>,
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
    let icon = if draft.kind == Some(DraftKind::Encounter) && !m.phone {
        (m.icon * 0.55).max(40.0)
    } else {
        m.icon
    };
    for (
        mut node,
        content,
        header,
        heading,
        actions,
        gallery,
        card,
        footer,
        art,
        title,
        gallery_hint,
    ) in &mut nodes
    {
        if gallery_hint {
            node.reborrow()
                .map_unchanged(|n| &mut n.display)
                .set_if_neq(if m.phone && m.count > 3 {
                    Display::Flex
                } else {
                    Display::None
                });
        }
        if title {
            node.reborrow()
                .map_unchanged(|n| &mut n.width)
                .set_if_neq(percent(if m.phone { 100 } else { 95 }));
        }
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
                .map_unchanged(|value| &mut value.flex_direction)
                .set_if_neq(if m.stacked {
                    FlexDirection::Column
                } else {
                    FlexDirection::Row
                });
            node.reborrow()
                .map_unchanged(|value| &mut value.align_items)
                .set_if_neq(if m.stacked {
                    AlignItems::Stretch
                } else {
                    AlignItems::Center
                });
            node.reborrow()
                .map_unchanged(|value| &mut value.row_gap)
                .set_if_neq(px(8));
            node.reborrow()
                .map_unchanged(|value| &mut value.height)
                .set_if_neq(px(m.header));
        }
        if heading {
            // Reserve both shaped lines when narrow CJK text measures as one.
            node.reborrow()
                .map_unchanged(|value| &mut value.min_height)
                .set_if_neq(if m.stacked && m.width < 420.0 {
                    px(64)
                } else {
                    Val::Auto
                });
            node.reborrow()
                .map_unchanged(|value| &mut value.width)
                .set_if_neq(if m.stacked { percent(100) } else { Val::Auto });
            node.reborrow()
                .map_unchanged(|value| &mut value.flex_grow)
                .set_if_neq(if m.stacked { 0.0 } else { 1.0 });
            node.reborrow()
                .map_unchanged(|value| &mut value.flex_shrink)
                .set_if_neq(1.0);
        }
        if actions {
            node.reborrow()
                .map_unchanged(|value| &mut value.width)
                .set_if_neq(if m.stacked { percent(100) } else { Val::Auto });
            node.reborrow()
                .map_unchanged(|value| &mut value.flex_wrap)
                .set_if_neq(if m.stacked {
                    FlexWrap::Wrap
                } else {
                    FlexWrap::NoWrap
                });
        }
        if gallery {
            node.reborrow()
                .map_unchanged(|value| &mut value.height)
                .set_if_neq(px(m.gallery_height));
            node.reborrow()
                .map_unchanged(|n| &mut n.width)
                .set_if_neq(px(m.gallery_width));
            node.reborrow()
                .map_unchanged(|n| &mut n.align_self)
                .set_if_neq(AlignSelf::Center);
            node.reborrow()
                .map_unchanged(|n| &mut n.flex_wrap)
                .set_if_neq(if m.phone {
                    FlexWrap::Wrap
                } else {
                    FlexWrap::NoWrap
                });
            node.reborrow()
                .map_unchanged(|n| &mut n.align_content)
                .set_if_neq(AlignContent::FlexStart);
            node.reborrow()
                .map_unchanged(|n| &mut n.row_gap)
                .set_if_neq(px(m.gap));
            node.reborrow()
                .map_unchanged(|n| &mut n.overflow)
                .set_if_neq(if m.phone {
                    Overflow::scroll_y()
                } else {
                    Overflow::DEFAULT
                });
            node.reborrow()
                .map_unchanged(|value| &mut value.column_gap)
                .set_if_neq(px(m.gap));
        }
        if card {
            node.reborrow()
                .map_unchanged(|value| &mut value.width)
                .set_if_neq(px(m.card_width));
            node.reborrow()
                .map_unchanged(|value| &mut value.height)
                .set_if_neq(px(m.card_height));
            node.reborrow()
                .map_unchanged(|value| &mut value.row_gap)
                .set_if_neq(px(if m.phone || m.compact { 7 } else { 14 }));
        }
        if footer {
            node.reborrow()
                .map_unchanged(|value| &mut value.height)
                .set_if_neq(px(m.footer));
            node.reborrow()
                .map_unchanged(|value| &mut value.padding)
                .set_if_neq(UiRect {
                    left: px(if m.compact { 12 } else { 20 }),
                    right: px(if m.compact { 116 } else { 20 }),
                    top: px(10),
                    bottom: px(10),
                });
        }
        if art {
            node.reborrow()
                .map_unchanged(|value| &mut value.width)
                .set_if_neq(px(icon));
            node.reborrow()
                .map_unchanged(|value| &mut value.height)
                .set_if_neq(px(icon));
        }
    }
    for mut text_layout in &mut title_layouts {
        text_layout
            .reborrow()
            .map_unchanged(|l| &mut l.linebreak)
            .set_if_neq(if m.phone && !language.is_chinese() {
                LineBreak::WordBoundary
            } else {
                LineBreak::WordOrCharacter
            });
    }
    for (mut font, heading, title, detail_title, detail, trade) in &mut fonts {
        if heading {
            font.reborrow()
                .map_unchanged(|value| &mut value.font_size)
                .set_if_neq(FontSize::Px(if m.compact {
                    22.0
                } else if m.width < 420.0 {
                    24.0
                } else if m.stacked {
                    28.0
                } else {
                    32.0
                }));
        }
        if title {
            font.reborrow()
                .map_unchanged(|value| &mut value.font_size)
                .set_if_neq(FontSize::Px(if m.phone {
                    (13.0 + (size.0 - 320.0) / 20.0).clamp(13.0, 15.0)
                } else if m.compact && m.count > 3 {
                    11.0
                } else if m.compact {
                    16.0
                } else if m.count > 3 {
                    17.0
                } else {
                    23.0
                }));
        }
        if detail_title {
            font.reborrow()
                .map_unchanged(|value| &mut value.font_size)
                .set_if_neq(FontSize::Px(if m.compact { 16.0 } else { 22.0 }));
        }
        if detail {
            font.reborrow()
                .map_unchanged(|value| &mut value.font_size)
                .set_if_neq(FontSize::Px(if m.compact { 11.0 } else { 13.0 }));
        }
        if trade {
            font.reborrow()
                .map_unchanged(|value| &mut value.font_size)
                .set_if_neq(FontSize::Px(if m.compact { 10.0 } else { 13.0 }));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn phone_gallery_keeps_three_readable_columns_and_scrolls_extra_rows() {
        for (width, height) in [(320.0, 667.0), (360.0, 800.0)] {
            for count in [3, 6] {
                let m = layout(width, height, count);
                assert!(m.phone);
                assert_eq!(m.gallery_width, width - 16.0);
                assert!(m.card_width >= 90.0 && m.card_height == 224.0);
                assert!(m.gallery_height >= 200.0 && m.footer >= 96.0);
                assert!(3.0 * m.card_width + 2.0 * m.gap <= m.gallery_width);
                assert!(m.header + m.gallery_height + m.footer + 2.0 * m.gap <= m.height);
                assert_eq!(
                    m.width,
                    width - 80.0,
                    "header and footer width remain unchanged"
                );
            }
        }
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
