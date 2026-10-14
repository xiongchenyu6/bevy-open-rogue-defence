//! Controller sampling and navigation of the same public controls used by pointers.
use super::{GameLanguage, GameSession};
use bevy::{prelude::*, window::PrimaryWindow};
#[path = "controller_navigation.rs"]
mod navigation;

#[derive(Component)]
pub(super) struct ControllerDetailScroll;
#[derive(Component, Clone, Copy)]
pub struct ControllerScope(pub i32);
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ControllerInputSet;
#[derive(Resource, Default, Debug)]
pub struct ControllerInput {
    pub movement: Vec2,
    pub dodge: bool,
    pub pause: bool,
    pub speed: i8,
    active: Option<Entity>,
    navigation: bool,
    direction: Vec2,
    previous_sticks: Vec<(Entity, Vec2, Vec2)>,
    repeat_at: f32,
}
#[derive(Resource, Default, Debug)]
pub struct ControllerFocus {
    pub entity: Option<Entity>,
    pub key: String,
    pub scope: i32,
    can_back: bool,
    owned: Vec<Entity>,
}
pub(super) struct ControllerInputPlugin;
impl Plugin for ControllerInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ControllerInput>()
            .init_resource::<ControllerFocus>()
            .add_systems(Startup, setup_hint)
            .add_systems(Update, update_hint)
            .add_systems(
                PreUpdate,
                controller
                    .in_set(ControllerInputSet)
                    .after(bevy::input::InputSystems)
                    .after(bevy::ui::UiSystems::Focus)
                    .after(super::mobile_layout::update_control_scheme)
                    .before(super::AssetReadinessInputSet)
                    .before(super::SettingsInputSet)
                    .before(super::SettingsDepartureInputSet),
            );
    }
}

#[derive(Component)]
struct ControllerHint;
fn setup_hint(mut commands: Commands) {
    commands.spawn((
        ControllerHint,
        Name::new("Controller guidance"),
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(12.),
            ..default()
        },
        TextColor(Color::srgb(1., 0.88, 0.55)),
        Node {
            position_type: PositionType::Absolute,
            left: px(12),
            top: px(4),
            display: Display::None,
            ..default()
        },
        GlobalZIndex(1100),
        Pickable::IGNORE,
    ));
}
fn update_hint(
    input: Res<ControllerInput>,
    focus: Res<ControllerFocus>,
    mode: Res<super::ControlScheme>,
    language: Res<GameLanguage>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut hints: Query<(&mut Text, &mut Node, &mut TextFont), With<ControllerHint>>,
) {
    let size = windows
        .single()
        .map(|w| Vec2::new(w.width(), w.height()))
        .unwrap_or(Vec2::new(1280., 720.));
    let compact = super::mobile_layout::hud_metrics(size.x, size.y).compact;
    for (mut text, mut node, mut font) in &mut hints {
        let display = if mode.controller() {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
        let navigating = focus.scope > 0 || input.navigation;
        node.reborrow()
            .map_unchanged(|n| &mut n.top)
            .set_if_neq(px(if compact && focus.scope == 0 { 54. } else { 0. }));
        node.reborrow()
            .map_unchanged(|n| &mut n.left)
            .set_if_neq(px(if compact && focus.scope == 0 {
                super::mobile_layout::hud_metrics(size.x, size.y).safe_x + 226.
            } else {
                12.
            }));
        node.reborrow()
            .map_unchanged(|n| &mut n.max_width)
            .set_if_neq(px(if compact && focus.scope == 0 {
                (size.x - 2. * super::mobile_layout::hud_metrics(size.x, size.y).safe_x - 456.)
                    .max(110.)
            } else {
                size.x - 24.
            }));
        font.reborrow()
            .map_unchanged(|f| &mut f.font_size)
            .set_if_neq(FontSize::Px(if compact { 10. } else { 11. }));
        let copy = if compact && focus.scope == 0 {
            if navigating {
                language.pick("A Select · B Close", "A 选择 · B 关闭")
            } else {
                language.pick("A Dodge · B Menu", "A 闪避 · B 菜单")
            }
        } else if compact {
            if focus.can_back {
                language.pick(
                    "D-pad Focus · A Confirm · B Back · R stick Scroll",
                    "方向键移动 · A 确认 · B 返回 · 右摇杆滚动",
                )
            } else {
                language.pick(
                    "D-pad Focus · A Choose · R stick Scroll",
                    "方向键移动 · A 选择 · 右摇杆滚动",
                )
            }
        } else if focus.scope > 0 || input.navigation {
            if focus.can_back {
                language.pick(
                    "D-pad · Navigate   A (South) · Confirm   B (East) · Back   R stick · Scroll",
                    "方向键 · 移动焦点   A（下键）· 确认   B（右键）· 返回   右摇杆 · 滚动",
                )
            } else {
                language.pick(
                    "D-pad · Navigate   A (South) · Choose   R stick · Scroll",
                    "方向键 · 移动焦点   A（下键）· 选择   右摇杆 · 滚动",
                )
            }
        } else {
            language.pick(
                "L stick · Move   A (South) · Dodge   Start · Pause   LB/RB · Speed   B (East) · Menu",
                "左摇杆 · 移动   A（下键）· 闪避   Start · 暂停   左肩/右肩 · 速度   B（右键）· 菜单",
            )
        };
        if text.0 != copy {
            text.0 = copy.to_owned();
        }
    }
}

pub fn deadzone(stick: Vec2) -> Vec2 {
    let length = stick.length();
    if length <= 0.2 {
        Vec2::ZERO
    } else {
        stick / length * ((length - 0.2) / 0.8).min(1.0)
    }
}
fn controller(world: &mut World) {
    let now = world.resource::<Time<Real>>().elapsed_secs();
    let delta = world.resource::<Time<Real>>().delta_secs();
    let mut input = world.remove_resource::<ControllerInput>().unwrap();
    input.movement = Vec2::ZERO;
    input.dodge = false;
    input.pause = false;
    input.speed = 0;
    let mut focus = world.remove_resource::<ControllerFocus>().unwrap();

    let world_controller_mode = world.resource::<super::ControlScheme>().controller();
    for entity in focus.owned.drain(..) {
        if let Some(mut interaction) = world.get_mut::<Interaction>(entity)
            && *interaction == Interaction::Pressed
            && world_controller_mode
        {
            *interaction = Interaction::None;
        }
        if !world_controller_mode {
            if let Some(mut outline) = world.get_mut::<Outline>(entity) {
                outline.color = Color::NONE;
            }
            if let Ok(mut entity) = world.get_entity_mut(entity) {
                entity.remove::<ControllerFocused>();
            }
        }
    }
    let mut pads = world.query::<(Entity, &Gamepad)>();
    let buttons = pads
        .iter(world)
        .filter(|(_, pad)| pad.get_just_pressed().next().is_some())
        .map(|(id, _)| id)
        .min_by_key(|id| id.to_bits());
    let changed_sticks = pads
        .iter(world)
        .filter(|(id, pad)| {
            let left = deadzone(pad.left_stick());
            let right = deadzone(pad.right_stick());
            let previous = input.previous_sticks.iter().find(|(old, _, _)| old == id);
            (left.length_squared() > 0.0 || right.length_squared() > 0.0)
                && previous.is_none_or(|(_, old_left, old_right)| {
                    left.distance(*old_left) > 0.1 || right.distance(*old_right) > 0.1
                })
        })
        .map(|(id, _)| id)
        .min_by_key(|id| id.to_bits());
    let deliberate = buttons.or(changed_sticks);
    input.previous_sticks = pads
        .iter(world)
        .map(|(id, pad)| (id, deadzone(pad.left_stick()), deadzone(pad.right_stick())))
        .collect();
    if let Some(id) = deliberate {
        input.active = Some(id);
        *world.resource_mut::<super::ControlScheme>() = super::ControlScheme::Controller;
    }
    let disconnected = input
        .active
        .is_some_and(|id| world.get::<Gamepad>(id).is_none());
    let focused = world
        .query_filtered::<&Window, With<PrimaryWindow>>()
        .iter(world)
        .next()
        .is_none_or(|w| w.focused);
    if disconnected || (!focused && input.active.is_some()) {
        input.active = None;
        input.navigation = false;
        if let Some(session) = world.get_resource::<GameSession>()
            && world_controller_mode
            && !session.paused
            && !session.finished
        {
            input.pause = true;
        }
    }
    let scheme_controller = world.resource::<super::ControlScheme>().controller();
    let sample = input
        .active
        .and_then(|id| world.get::<Gamepad>(id))
        .map(|pad| {
            (
                deadzone(pad.left_stick()),
                deadzone(pad.right_stick()),
                pad.dpad(),
                pad.just_pressed(GamepadButton::South),
                pad.just_pressed(GamepadButton::East),
                pad.just_pressed(GamepadButton::Start),
                pad.just_pressed(GamepadButton::LeftTrigger),
                pad.just_pressed(GamepadButton::RightTrigger),
                pad.just_pressed(GamepadButton::Select),
            )
        });
    if !scheme_controller {
        focus.entity = None;
        world.insert_resource(focus);
        world.insert_resource(input);
        return;
    }
    navigation::navigate(world, &mut input, &mut focus, sample, now, delta, focused);
    world.insert_resource(focus);
    world.insert_resource(input);
}
pub(super) fn reset_run(world: &mut World) {
    if let Some(mut input) = world.get_resource_mut::<ControllerInput>() {
        input.movement = Vec2::ZERO;
        input.dodge = false;
        input.pause = false;
        input.speed = 0;
        input.navigation = false;
        input.direction = Vec2::ZERO;
    }
    if world.contains_resource::<ControllerFocus>() {
        world.insert_resource(ControllerFocus::default());
    }
    for mut pad in world.query::<&mut Gamepad>().iter_mut(world) {
        pad.digital_mut().clear();
    }
}
#[derive(Component)]
pub struct ControllerDisabled;
#[derive(Component)]
pub(super) struct ControllerFocused;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn radial_deadzone_rescales_and_clamps() {
        assert_eq!(deadzone(Vec2::new(0.1, 0.1)), Vec2::ZERO);
        assert!((deadzone(Vec2::X * 0.6).x - 0.5).abs() < 0.001);
        assert!((deadzone(Vec2::ONE).length() - 1.).abs() < 0.001);
    }
    #[test]
    fn navigation_prefers_same_row() {
        assert!(
            navigation::navigation_score(Vec2::new(100., 0.), Vec2::X)
                < navigation::navigation_score(Vec2::new(20., 100.), Vec2::X)
        );
    }
}

#[cfg(test)]
#[path = "controller_input_tests.rs"]
mod ecs_tests;
