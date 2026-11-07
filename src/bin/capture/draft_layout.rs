//! Measure the actual long-title draft after native UI layout.
use bevy::{prelude::*, text::TextLayoutInfo};

#[derive(Clone, Copy, Debug)]
struct Bounds {
    min: Vec2,
    max: Vec2,
}
impl Bounds {
    fn intersects(self, other: Self) -> bool {
        self.min.x < other.max.x - 0.5
            && self.max.x > other.min.x + 0.5
            && self.min.y < other.max.y - 0.5
            && self.max.y > other.min.y + 0.5
    }
    fn contains(self, other: Self) -> bool {
        other.min.cmpge(self.min - Vec2::ONE).all() && other.max.cmple(self.max + Vec2::ONE).all()
    }
}

pub fn audit(world: &mut World, frame: u32) {
    let window = world.query::<&Window>().single(world).unwrap();
    let viewport = Bounds {
        min: Vec2::ZERO,
        max: Vec2::new(
            window.physical_width() as f32,
            window.physical_height() as f32,
        ),
    };
    let mut bounds = std::collections::BTreeMap::new();
    for (entity, name, node, transform) in world
        .query::<(Entity, &Name, &ComputedNode, &UiGlobalTransform)>()
        .iter(world)
    {
        if matches!(
            name.as_str(),
            "Draft header"
                | "Settings open"
                | "Draft heading"
                | "Draft actions"
                | "Selection detail toggle"
                | "Build refresh"
                | "Build abandon"
                | "Build later"
        ) || name.as_str().starts_with("Build card ")
        {
            assert!(
                node.size().min_element() > 0.0,
                "{name} must have native layout"
            );
            if name.as_str() == "Draft heading" {
                let text = world
                    .get::<TextLayoutInfo>(entity)
                    .expect("shaped heading text");
                assert!(!text.glyphs.is_empty());
                assert!(
                    text.size.cmple(node.size() + Vec2::ONE).all(),
                    "heading glyphs overflow actual node: {:?} > {:?}",
                    text.size,
                    node.size()
                );
                println!(
                    "DRAFT_TEXT frame={frame} node_size={:?} glyph_size={:?} glyph_count={}",
                    node.size(),
                    text.size,
                    text.glyphs.len()
                );
            }
            let rect = Bounds {
                min: transform.translation - node.size() * 0.5,
                max: transform.translation + node.size() * 0.5,
            };
            assert!(
                viewport.contains(rect),
                "{name} outside {viewport:?}: {rect:?}"
            );
            println!(
                "DRAFT_LAYOUT frame={frame} name={:?} min={:?} max={:?}",
                name.as_str(),
                rect.min,
                rect.max
            );
            assert!(bounds.insert(name.as_str().to_owned(), rect).is_none());
        }
    }
    let detail = world
        .query::<(Entity, &Name)>()
        .iter(world)
        .find(|(_, name)| name.as_str() == "Selection details")
        .map(|(entity, _)| entity)
        .expect("actual detail text");
    let scroll = world.get::<ChildOf>(detail).unwrap().parent();
    let node = world.get::<ComputedNode>(scroll).unwrap();
    let transform = world.get::<UiGlobalTransform>(scroll).unwrap();
    let visible_detail = Bounds {
        min: transform.translation - node.size() * 0.5,
        max: transform.translation + node.size() * 0.5,
    };
    assert!(viewport.contains(visible_detail));
    println!(
        "DRAFT_LAYOUT frame={frame} name=\"Detail viewport\" min={:?} max={:?}",
        visible_detail.min, visible_detail.max
    );
    bounds.insert("Detail viewport".into(), visible_detail);
    let settings = bounds["Settings open"];
    for (name, rect) in &bounds {
        if name == "Draft heading"
            || name == "Detail viewport"
            || name == "Selection detail toggle"
            || name.starts_with("Build ")
        {
            assert!(!settings.intersects(*rect), "Settings open overlaps {name}");
        }
    }
    let header = bounds["Draft header"];
    let heading = bounds["Draft heading"];
    let actions = bounds["Draft actions"];
    assert!(header.contains(heading) && header.contains(actions));
    let buttons = [
        "Selection detail toggle",
        "Build refresh",
        "Build abandon",
        "Build later",
    ];
    for (index, name) in buttons.iter().enumerate() {
        let button = bounds[*name];
        assert!(actions.contains(button));
        assert!(!heading.intersects(button), "heading overlaps {name}");
        assert!(
            (button.max - button.min).min_element() >= 43.5,
            "{name} touch area below44px"
        );
        for other in &buttons[index + 1..] {
            assert!(
                !button.intersects(bounds[*other]),
                "{name} overlaps {other}"
            );
        }
        for (card_name, card) in &bounds {
            if card_name.starts_with("Build card ") {
                assert!(!button.intersects(*card), "{name} overlaps {card_name}");
                assert!(!heading.intersects(*card), "heading overlaps {card_name}");
            }
        }
    }
    assert_eq!(
        bounds
            .keys()
            .filter(|name| name.starts_with("Build card "))
            .count(),
        3
    );
    println!(
        "[capture] PASS draft-layout frame={frame}: actual heading/actions/cards are inside viewport and disjoint; all four controls>=44px"
    );
}
