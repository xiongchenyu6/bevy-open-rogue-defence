//! Focus and activation stay within the current visible public UI scope.
use super::{
    ControllerDetailScroll, ControllerDisabled, ControllerFocus, ControllerFocused,
    ControllerInput, ControllerScope,
};
use bevy::prelude::*;
struct Control {
    entity: Entity,
    key: String,
    scope: i32,
    position: Vec2,
    size: Vec2,
}
fn ancestry(world: &World, entity: Entity) -> (i32, String, bool) {
    let mut current = Some(entity);
    let mut scope = 0;
    let mut key = Vec::new();
    let mut visible = true;
    while let Some(id) = current {
        if let Some(node) = world.get::<Node>(id) {
            visible &= node.display != Display::None;
        }
        if let Some(value) = world.get::<Visibility>(id) {
            visible &= *value != Visibility::Hidden;
        }
        if let Some(value) = world.get::<ControllerScope>(id) {
            scope = scope.max(value.0);
        }
        if let Some(name) = world.get::<Name>(id) {
            key.push(name.as_str().to_owned());
        } else if let Some(parent) = world.get::<ChildOf>(id)
            && let Some(children) = world.get::<Children>(parent.parent())
        {
            key.push(format!(
                "#{}",
                children.iter().position(|child| child == id).unwrap_or(0)
            ));
        }
        current = world.get::<ChildOf>(id).map(ChildOf::parent);
    }
    key.reverse();
    (scope, key.join("/"), visible)
}
pub(super) fn navigate(
    world: &mut World,
    input: &mut ControllerInput,
    focus: &mut ControllerFocus,
    sample: Option<(Vec2, Vec2, Vec2, bool, bool, bool, bool, bool, bool)>,
    now: f32,
    delta: f32,
    focused: bool,
) {
    let previous_entity = focus.entity;
    let previous_key = focus.key.clone();
    focus.entity = None;
    let mut query =
        world.query_filtered::<(Entity, &ComputedNode, &UiGlobalTransform), With<Button>>();
    let mut controls: Vec<_> = query
        .iter(world)
        .filter_map(|(entity, node, transform)| {
            let (scope, key, visible) = ancestry(world, entity);
            (visible
                && node.size().min_element() > 0.0
                && world.get::<ControllerDisabled>(entity).is_none())
            .then_some(Control {
                entity,
                key,
                scope,
                position: transform.translation,
                size: node.size(),
            })
        })
        .collect();
    // Scope roots without buttons still block underlying controls (asset loading).
    let scope = world
        .query::<(Entity, &ControllerScope)>()
        .iter(world)
        .filter(|(id, _)| ancestry(world, *id).2)
        .map(|(_, s)| s.0)
        .max()
        .unwrap_or(0);
    controls.retain(|c| {
        c.scope == scope
            || (matches!(scope, 90 | 100 | 150 | 400) && c.key.ends_with("Settings open"))
    });
    focus.can_back = scope == 0
        || controls.iter().any(|c| back_control(&c.key))
        || controls.iter().any(|c| c.key.contains("Market panel"));
    controls.sort_by(|a, b| {
        b.scope.cmp(&a.scope).then(
            a.position
                .y
                .total_cmp(&b.position.y)
                .then(a.position.x.total_cmp(&b.position.x))
                .then(a.key.cmp(&b.key)),
        )
    });
    if scope != focus.scope {
        focus.key.clear();
        focus.scope = scope;
        input.direction = Vec2::ZERO;
    }
    if !controls.iter().any(|c| c.key == focus.key) {
        focus.key = controls.first().map(|c| c.key.clone()).unwrap_or_default();
    }
    if let Some((stick, right, dpad, confirm, back, start, left_shoulder, right_shoulder, select)) =
        sample.filter(|_| focused)
    {
        if scope == 0 && (back || select) {
            input.navigation = !input.navigation;
        }
        let navigating = scope > 0 || input.navigation;
        if !navigating {
            input.movement = Vec2::new(stick.x, -stick.y);
            input.dodge = confirm;
            input.pause |= start;
            input.speed = if left_shoulder {
                -1
            } else if right_shoulder {
                1
            } else {
                0
            };
        } else {
            let raw = if dpad.length_squared() > 0.0 {
                dpad
            } else {
                stick
            };
            let direction = if raw.length() < 0.5 {
                Vec2::ZERO
            } else if raw.x.abs() > raw.y.abs() {
                Vec2::new(raw.x.signum(), 0.)
            } else {
                Vec2::new(0., -raw.y.signum())
            };
            if direction != Vec2::ZERO && (direction != input.direction || now >= input.repeat_at) {
                let initial = direction != input.direction;
                if let Some(current) = controls.iter().find(|c| c.key == focus.key) {
                    let origin = current.position;
                    if let Some(next) = controls
                        .iter()
                        .filter(|c| c.key != focus.key && (c.position - origin).dot(direction) > 1.)
                        .min_by(|a, b| {
                            navigation_score(a.position - origin, direction)
                                .total_cmp(&navigation_score(b.position - origin, direction))
                        })
                    {
                        focus.key = next.key.clone();
                    }
                }
                input.repeat_at = now + if initial { 0.35 } else { 0.12 };
            }
            input.direction = direction;
            let selected = controls.iter().find(|c| c.key == focus.key);
            let activation = if back || (start && scope > 0) {
                controls
                    .iter()
                    .find(|c| back_control(&c.key))
                    .map(|c| c.entity)
                    .or_else(|| {
                        if scope == 280 && controls.iter().any(|c| c.key.contains("Market panel")) {
                            world
                                .query::<(Entity, &Name)>()
                                .iter(world)
                                .find(|(_, n)| n.as_str() == "Market toggle")
                                .map(|(e, _)| e)
                        } else {
                            None
                        }
                    })
            } else if confirm {
                selected.map(|c| c.entity)
            } else {
                None
            };
            if let Some(entity) = activation {
                if let Some(mut value) = world.get_mut::<Interaction>(entity) {
                    *value = Interaction::Pressed;
                }
                focus.owned.push(entity);
            }
            if scope == 0 && start {
                input.pause = true;
                input.navigation = false;
            }
            if let Some(control) = selected {
                focus.entity = Some(control.entity);
                if world.get::<ControllerFocused>(control.entity).is_none() {
                    world.entity_mut(control.entity).insert(ControllerFocused);
                }
                world.entity_mut(control.entity).insert(Outline {
                    width: px(3),
                    offset: px(2),
                    color: Color::srgb(1., 0.85, 0.3),
                });
                if !focus.owned.contains(&control.entity) {
                    focus.owned.push(control.entity);
                }
                let detail_target = world
                    .query_filtered::<Entity, (With<ControllerDetailScroll>, With<ScrollPosition>)>(
                    )
                    .iter(world)
                    .find(|id| {
                        let (target_scope, _, visible) = ancestry(world, *id);
                        visible
                            && target_scope == scope
                            && world
                                .get::<ComputedNode>(*id)
                                .is_some_and(|n| n.size().min_element() > 0.)
                    });
                if let Some(id) = detail_target
                    && let Some(mut scroll) = world.get_mut::<ScrollPosition>(id)
                {
                    scroll.y = (scroll.y - right.y * delta * 500.).max(0.);
                }
                // Scroll ancestors to reveal focused control, and right stick scrolls details/list.
                let mut ancestor = world.get::<ChildOf>(control.entity).map(ChildOf::parent);
                while let Some(id) = ancestor {
                    if let (Some(node), Some(transform)) = (
                        world.get::<ComputedNode>(id),
                        world.get::<UiGlobalTransform>(id),
                    ) {
                        let inverse_scale = node.inverse_scale_factor();
                        let top = transform.translation.y - node.size().y * 0.5;
                        let bottom = top + node.size().y;
                        let reveal =
                            previous_entity != Some(control.entity) || previous_key != focus.key;
                        let adjust = if !reveal {
                            0.
                        } else if control.position.y - control.size.y * 0.5 < top {
                            control.position.y - control.size.y * 0.5 - top
                        } else if control.position.y + control.size.y * 0.5 > bottom {
                            control.position.y + control.size.y * 0.5 - bottom
                        } else {
                            0.
                        };
                        if let Some(mut scroll) = world.get_mut::<ScrollPosition>(id) {
                            scroll.y = (scroll.y + adjust * inverse_scale
                                - if detail_target.is_none() {
                                    right.y * delta * 500.
                                } else {
                                    0.
                                })
                            .max(0.);
                        }
                    }
                    ancestor = world.get::<ChildOf>(id).map(ChildOf::parent);
                }
            }
        }
    }
    if previous_entity != focus.entity
        && let Some(old) = previous_entity
    {
        if let Some(mut outline) = world.get_mut::<Outline>(old) {
            outline.color = Color::NONE;
        }
        if let Ok(mut entity) = world.get_entity_mut(old) {
            entity.remove::<ControllerFocused>();
        }
    }
}
fn back_control(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    [
        "close",
        "cancel",
        "resume",
        "tutorial back",
        "campaign chapters",
    ]
    .iter()
    .any(|suffix| key.ends_with(suffix))
}
pub(super) fn navigation_score(offset: Vec2, direction: Vec2) -> f32 {
    let forward = offset.dot(direction);
    let side = offset.perp_dot(direction).abs();
    forward + side * 3.
}
