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
    icon: f32,
    compact: bool,
    stacked: bool,
    count: usize,
}
fn layout(width: f32, height: f32, count: usize) -> Layout {
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
    let footer = if compact && stacked {
        96.0
    } else if compact {
        126.0
    } else {
        210.0
    };
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
        stacked,
        count,
    }
}

pub(crate) fn update_layout(
    draft: Res<DraftState>,
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
    for (mut node, content, header, heading, actions, gallery, card, footer, art) in &mut nodes {
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
                .set_if_neq(px(m.card_height));
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
                .set_if_neq(px(if m.compact { 7 } else { 14 }));
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
                .set_if_neq(FontSize::Px(if m.compact && m.count > 3 {
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
