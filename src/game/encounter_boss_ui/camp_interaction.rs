//! Scene-anchored hit areas, never a duplicate dock. Activate on release, not drag.
use crate::game::{
    AdventureBossState, AdventureCampRequest, GameLanguage, GameSession,
    controller_input::ControllerFocused,
    encounter_bosses::{ADVENTURE_BOSSES, ADVENTURE_OFFER_TIMES, ADVENTURE_SITES},
    mobile_layout::hud_metrics,
};
use bevy::{prelude::*, window::PrimaryWindow};

#[derive(Component)]
pub(crate) struct SceneInputBlocker;

#[derive(Component)]
pub(super) struct CampEntry(u8);
#[derive(Component)]
pub(super) struct Marker(u8);
#[derive(Component)]
pub(super) struct Caption(u8);
#[derive(Clone, Copy)]
struct Press {
    index: u8,
    origin: Vec2,
    touch: Option<u64>,
    moved: bool,
}
impl Press {
    fn track(&mut self, point: Vec2) {
        self.moved |= self.origin.distance(point) > 12.0;
    }
    fn released(mut self, point: Vec2, same_target: bool) -> bool {
        self.track(point);
        !self.moved && same_target
    }
}
#[derive(Resource, Default)]
pub(super) struct CampInteraction {
    press: Option<Press>,
    hint: Option<u8>,
    remaining: f32,
    known: u8,
}

pub(super) fn spawn(mut commands: Commands, assets: Option<Res<AssetServer>>) {
    let Some(assets) = assets else {
        return;
    };
    for index in 0..3 {
        commands
            .spawn((
                Name::new(format!("Adventure camp entry {index}")),
                CampEntry(index),
                Button,
                Node {
                    position_type: PositionType::Absolute,
                    display: Display::None,
                    ..default()
                },
                BackgroundColor(Color::NONE),
                GlobalZIndex(5),
            ))
            .with_children(|entry| {
                entry.spawn((
                    Marker(index),
                    Pickable::IGNORE,
                    ImageNode::new(assets.load("ui/icons/execution_brand.png")),
                    Node {
                        position_type: PositionType::Absolute,
                        width: px(26),
                        height: px(26),
                        right: px(4),
                        top: px(4),
                        ..default()
                    },
                ));
                entry.spawn((
                    Caption(index),
                    Pickable::IGNORE,
                    Text::new(""),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.98, 0.89, 0.68)),
                    BackgroundColor(Color::srgba(0.025, 0.04, 0.05, 0.94)),
                    Node {
                        position_type: PositionType::Absolute,
                        bottom: px(0),
                        left: px(0),
                        max_width: px(160),
                        padding: UiRect::all(px(5)),
                        ..default()
                    },
                ));
            });
    }
}

// Cover the rendered leader and ground footprint. Clip to the actual viewport;
// never clamp an offscreen camp to the screen edge and invent a remote button.
fn camp_rect(
    camera: &Camera,
    transform: &GlobalTransform,
    center: Vec3,
    size: Vec2,
) -> Option<Rect> {
    let mut min = Vec2::splat(f32::INFINITY);
    let mut max = Vec2::splat(f32::NEG_INFINITY);
    for x in [-2.4, 2.4] {
        for z in [-2.4, 2.4] {
            for y in [0.0, 3.4] {
                let p = camera
                    .world_to_viewport(transform, center + Vec3::new(x, y, z))
                    .ok()?;
                min = min.min(p);
                max = max.max(p);
            }
        }
    }
    let center = (min + max) * 0.5;
    let half = ((max - min) * 0.5).max(Vec2::splat(24.0));
    let rect = Rect::from_corners(center - half, center + half)
        .intersect(Rect::from_corners(Vec2::ZERO, size));
    (rect.width() >= 24.0 && rect.height() >= 24.0).then_some(rect)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn position(
    state: Res<AdventureBossState>,
    session: Res<GameSession>,
    language: Res<GameLanguage>,
    time: Res<Time<Real>>,
    mut feedback: ResMut<CampInteraction>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut entries: Query<(&CampEntry, &Interaction, Has<ControllerFocused>, &mut Node)>,
    mut markers: Query<(&Marker, &mut Node), Without<CampEntry>>,
    mut captions: Query<(&Caption, &mut Text, &mut Node), (Without<CampEntry>, Without<Marker>)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Ok((camera, transform)) = cameras.single() else {
        return;
    };
    let size = Vec2::new(window.width(), window.height());
    feedback.remaining = (feedback.remaining - time.delta_secs()).max(0.0);
    let newly_ready = state.unlocked & !feedback.known;
    if newly_ready != 0 {
        feedback.hint = Some(newly_ready.trailing_zeros() as u8);
        feedback.remaining = 3.0;
    }
    feedback.known = state.unlocked;
    let mut show_caption = [false; 3];
    for (entry, interaction, focused, mut node) in &mut entries {
        let i = usize::from(entry.0);
        let rect = camp_rect(camera, transform, ADVENTURE_SITES[i], size);
        let show = !session.paused
            && !session.finished
            && !session.save_active
            && state.pending.is_none()
            && state.active.is_none()
            && state.resolved & (1 << i) == 0;
        node.display = if show && rect.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if let Some(rect) = rect {
            node.left = px(rect.min.x);
            node.top = px(rect.min.y);
            node.width = px(rect.width());
            node.height = px(rect.height());
        }
        show_caption[i] = focused
            || *interaction == Interaction::Hovered
            || (feedback.remaining > 0.0 && feedback.hint == Some(entry.0));
    }
    for (marker, mut node) in &mut markers {
        node.display = if state.unlocked & (1 << marker.0) != 0 {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (caption, mut text, mut node) in &mut captions {
        let i = usize::from(caption.0);
        node.display = if show_caption[i] {
            Display::Flex
        } else {
            Display::None
        };
        let def = &ADVENTURE_BOSSES[i];
        let suffix = if state.unlocked & (1 << i) != 0 {
            language.pick("Tap to challenge", "点击挑战").to_string()
        } else {
            let seconds = (ADVENTURE_OFFER_TIMES[i] - session.elapsed).max(0.0).ceil() as u32;
            format!(
                "{} {}:{:02}",
                language.pick("Unlock in", "解锁"),
                seconds / 60,
                seconds % 60
            )
        };
        text.0 = format!("{}\n{suffix}", language.pick(def.name_en, def.name_zh));
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn input(
    state: Res<AdventureBossState>,
    session: Res<GameSession>,
    windows: Query<&Window, With<PrimaryWindow>>,
    touches: Res<Touches>,
    mouse: Res<ButtonInput<MouseButton>>,
    scheme: Res<crate::game::ControlScheme>,
    mut feedback: ResMut<CampInteraction>,
    entries: Query<(
        &CampEntry,
        &Interaction,
        &ComputedNode,
        &UiGlobalTransform,
        &InheritedVisibility,
        Has<ControllerFocused>,
    )>,
    blockers: Query<
        (&ComputedNode, &UiGlobalTransform, &InheritedVisibility),
        (
            Or<(With<Button>, With<SceneInputBlocker>)>,
            Without<CampEntry>,
        ),
    >,
    mut requests: MessageWriter<AdventureCampRequest>,
) {
    if session.paused
        || session.finished
        || session.save_active
        || state.pending.is_some()
        || state.active.is_some()
    {
        feedback.press = None;
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    if !window.focused {
        feedback.press = None;
        return;
    }
    let scale = window.scale_factor();
    let metrics = hud_metrics(window.width(), window.height());
    let pad = |p: Vec2| p.distance(metrics.joystick_center) <= metrics.joystick_size * 0.5;
    // An active movement finger suppresses scene selection by every other finger.
    if touches.iter().any(|t| pad(t.start_position())) {
        feedback.press = None;
        return;
    }
    let hit = |p: Vec2| -> Option<u8> {
        if pad(p) && scheme.touch() {
            return None;
        }
        let physical = p * scale;
        if blockers
            .iter()
            .any(|(n, t, v)| v.get() && n.contains_point(*t, physical))
        {
            return None;
        }
        entries
            .iter()
            .filter(|(e, _, n, t, v, _)| {
                v.get() && state.resolved & (1 << e.0) == 0 && n.contains_point(**t, physical)
            })
            .min_by(|a, b| {
                a.3.translation
                    .distance_squared(physical)
                    .total_cmp(&b.3.translation.distance_squared(physical))
            })
            .map(|(e, ..)| e.0)
    };
    let mut selected = None;
    if scheme.controller() {
        selected = entries
            .iter()
            .find(|(_, i, _, _, v, f)| *f && v.get() && **i == Interaction::Pressed)
            .map(|(e, ..)| e.0);
    }
    if let Some(mut press) = feedback.press.take() {
        match press.touch {
            Some(id) => {
                if let Some(touch) = touches.get_pressed(id) {
                    press.track(touch.position());
                    feedback.press = Some(press);
                } else if let Some(touch) = touches.iter_just_released().find(|t| t.id() == id) {
                    if press.released(touch.position(), hit(touch.position()) == Some(press.index))
                    {
                        selected = Some(press.index);
                    }
                }
                // Cancellation/missing touch intentionally drops the gesture.
            }
            None => {
                if let Some(p) = window.cursor_position() {
                    press.track(p);
                    if mouse.just_released(MouseButton::Left) {
                        if press.released(p, hit(p) == Some(press.index)) {
                            selected = Some(press.index);
                        }
                    } else if mouse.pressed(MouseButton::Left) {
                        feedback.press = Some(press);
                    }
                }
            }
        }
    }
    if feedback.press.is_none() {
        if let Some(touch) = touches
            .iter_just_pressed()
            .find(|t| hit(t.position()).is_some())
        {
            feedback.press = Some(Press {
                index: hit(touch.position()).unwrap(),
                origin: touch.position(),
                touch: Some(touch.id()),
                moved: false,
            });
        } else if !scheme.touch()
            && mouse.just_pressed(MouseButton::Left)
            && let Some(p) = window.cursor_position()
            && let Some(index) = hit(p)
        {
            feedback.press = Some(Press {
                index,
                origin: p,
                touch: None,
                moved: false,
            });
        }
    }
    if let Some(index) = selected {
        if state.unlocked & (1 << index) != 0 {
            requests.write(AdventureCampRequest(index));
        } else {
            feedback.hint = Some(index);
            feedback.remaining = 3.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_same_target_release_without_drag_activates() {
        let p = Press {
            index: 0,
            origin: Vec2::ZERO,
            touch: Some(1),
            moved: false,
        };
        assert!(p.released(Vec2::new(4., 3.), true));
        assert!(!p.released(Vec2::ZERO, false));
        assert!(!p.released(Vec2::new(13., 0.), true));
        let mut drag = p;
        drag.track(Vec2::new(30., 0.));
        assert!(
            !drag.released(Vec2::ZERO, true),
            "returning to origin is still a drag"
        );
    }
}
