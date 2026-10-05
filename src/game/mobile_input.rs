use bevy::{
    prelude::*,
    window::{PrimaryWindow, WindowFocused},
};

use super::combat::{CombatSet, SurvivalSet};
use super::market::MarketEffects;
use super::mobile_layout::{ControlScheme, HudMetrics, hud_metrics, update_control_scheme};
use super::{
    Autoplay, DODGE_COOLDOWN_SECONDS, DODGE_DISTANCE, DODGE_INVULNERABILITY, DodgeButton,
    DodgeState, GameSession, Hero, HeroVitals, MoveIntent, PlayerBuild,
};

pub const HERO_MIN_X: f32 = -10.0;
pub const HERO_MAX_X: f32 = 10.0;
pub const HERO_MIN_Z: f32 = -12.0;
pub const HERO_MAX_Z: f32 = 12.0;

pub struct MobileInputPlugin;

impl Plugin for MobileInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ControlScheme>()
            .add_systems(
                PreUpdate,
                update_control_scheme.after(bevy::input::InputSystems),
            )
            .init_resource::<TouchJoystickState>()
            .add_systems(
                Update,
                (read_move_input, move_and_dodge_hero)
                    .chain()
                    .after(SurvivalSet)
                    .before(CombatSet),
            );
    }
}

#[derive(Resource, Debug, Default)]
struct TouchJoystickState {
    touch_id: Option<u64>,
    origin: Vec2,
}

impl TouchJoystickState {
    fn begin(&mut self, id: u64, origin: Vec2) -> bool {
        if self.touch_id.is_some() {
            return false;
        }
        self.touch_id = Some(id);
        self.origin = origin;
        true
    }

    fn clear(&mut self) {
        self.touch_id = None;
        self.origin = Vec2::ZERO;
    }
}

#[allow(clippy::too_many_arguments)]
fn read_move_input(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    touches: Res<Touches>,
    window_query: Query<(Entity, &Window), With<PrimaryWindow>>,
    mut focus_events: MessageReader<WindowFocused>,
    session: Res<GameSession>,
    vitals: Res<HeroVitals>,
    mut intent: ResMut<MoveIntent>,
    mut joystick: ResMut<TouchJoystickState>,
    mut dodge: ResMut<DodgeState>,
    dodge_buttons: Query<
        (
            Ref<Interaction>,
            &ComputedNode,
            &UiGlobalTransform,
            &InheritedVisibility,
        ),
        With<DodgeButton>,
    >,
    autoplay: Option<Res<Autoplay>>,
) {
    let primary_window = window_query.single().ok();
    let lost_focus = focus_events.read().any(|event| {
        !event.focused
            && primary_window.is_some_and(|(window_entity, _)| event.window == window_entity)
    });
    if lost_focus {
        clear_transient_input(&mut intent, &mut joystick, &mut dodge);
        return;
    }

    if session.paused || session.finished || !vitals.can_act() {
        clear_transient_input(&mut intent, &mut joystick, &mut dodge);
        return;
    }

    if autoplay.is_some() {
        // `session.elapsed` deliberately freezes during Archive. Use Bevy's
        // deterministic simulation clock so the capture bot keeps circling
        // instead of walking into one arena edge for the whole epilogue.
        intent.0 = autoplay_movement(time.elapsed_secs());
        if dodge.ready() && session.elapsed > 1.0 {
            dodge.request();
        }
        return;
    }

    if joystick.touch_id.is_some_and(|id| {
        touch_tracking_ended(
            touches.get_pressed(id).is_some(),
            touches.just_released(id),
            touches.just_canceled(id),
        )
    }) {
        joystick.clear();
    }

    let mut movement = Vec2::ZERO;
    if keyboard.pressed(KeyCode::ArrowLeft) {
        movement.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::ArrowRight) {
        movement.x += 1.0;
    }
    if keyboard.pressed(KeyCode::ArrowUp) {
        movement.y -= 1.0;
    }
    if keyboard.pressed(KeyCode::ArrowDown) {
        movement.y += 1.0;
    }
    if let Some((_, window)) = primary_window {
        // Bevy's legacy `Interaction` focus path uses only the first active touch.
        // Hit-test every newly pressed touch as well so a right-hand action still
        // fires while the left hand keeps the movement stick held down.
        let pressed_touch_positions = touches
            .iter_just_pressed()
            .map(|touch| touch.position() * window.scale_factor())
            .collect::<Vec<_>>();
        let dodge_pressed =
            dodge_buttons
                .iter()
                .any(|(interaction, node, transform, visibility)| {
                    (*interaction == Interaction::Pressed && interaction.is_changed())
                        || (visibility.get()
                            && pressed_touch_positions
                                .iter()
                                .any(|position| node.contains_point(*transform, *position)))
                });
        if keyboard.just_pressed(KeyCode::ShiftLeft)
            || keyboard.just_pressed(KeyCode::ShiftRight)
            || keyboard.just_pressed(KeyCode::ControlLeft)
            || keyboard.just_pressed(KeyCode::ControlRight)
            || dodge_pressed
        {
            dodge.request();
        }

        let metrics = hud_metrics(window.width(), window.height());
        if joystick.touch_id.is_none() {
            let candidate = touches
                .iter_just_pressed()
                .filter(|touch| joystick_contains(touch.start_position(), metrics))
                .min_by(|left, right| {
                    let left_distance = left
                        .start_position()
                        .distance_squared(metrics.joystick_center);
                    let right_distance = right
                        .start_position()
                        .distance_squared(metrics.joystick_center);
                    left_distance
                        .total_cmp(&right_distance)
                        .then_with(|| left.id().cmp(&right.id()))
                });
            if let Some(touch) = candidate {
                joystick.begin(touch.id(), touch.start_position());
            }
        }
        if let Some(touch) = joystick
            .touch_id
            .and_then(|touch_id| touches.get_pressed(touch_id))
        {
            movement = joystick_intent(
                joystick.origin,
                touch.position(),
                metrics.drag_radius,
                metrics.deadzone,
            );
        }
    }

    intent.0 = movement.clamp_length_max(1.0);
}

fn clear_transient_input(
    intent: &mut MoveIntent,
    joystick: &mut TouchJoystickState,
    dodge: &mut DodgeState,
) {
    intent.0 = Vec2::ZERO;
    joystick.clear();
    dodge.requested = false;
}

fn joystick_contains(position: Vec2, metrics: HudMetrics) -> bool {
    position.distance_squared(metrics.joystick_center) <= (metrics.joystick_size * 0.5).powi(2)
}

fn touch_tracking_ended(is_pressed: bool, just_released: bool, just_canceled: bool) -> bool {
    !is_pressed || just_released || just_canceled
}

fn joystick_intent(origin: Vec2, position: Vec2, drag_radius: f32, deadzone: f32) -> Vec2 {
    let drag = position - origin;
    let distance = drag.length();
    if distance <= deadzone || drag_radius <= deadzone {
        return Vec2::ZERO;
    }
    let strength = ((distance - deadzone) / (drag_radius - deadzone)).clamp(0.0, 1.0);
    drag.normalize_or_zero() * strength
}

fn autoplay_movement(elapsed: f32) -> Vec2 {
    let phase = elapsed * 0.58;
    Vec2::new(phase.cos(), phase.sin() * 0.72).clamp_length_max(1.0)
}

#[allow(clippy::too_many_arguments)]
fn move_and_dodge_hero(
    time: Res<Time>,
    intent: Res<MoveIntent>,
    build: Res<PlayerBuild>,
    session: Res<GameSession>,
    mut market_effects: ResMut<MarketEffects>,
    mut vitals: ResMut<HeroVitals>,
    mut dodge: ResMut<DodgeState>,
    mut hero_query: Query<&mut Transform, With<Hero>>,
) {
    if session.paused || session.finished {
        dodge.requested = false;
        return;
    }
    dodge.cooldown_remaining = (dodge.cooldown_remaining - time.delta_secs()).max(0.0);
    if !vitals.can_act() {
        dodge.requested = false;
        return;
    }
    let Ok(mut transform) = hero_query.single_mut() else {
        dodge.requested = false;
        return;
    };

    let movement = intent.0.clamp_length_max(1.0);
    if movement.length_squared() > 0.01 {
        dodge.last_direction = movement.normalize();
    }
    if dodge.requested && dodge.ready() {
        transform.translation = dodge_destination(transform.translation, dodge.last_direction);
        dodge.cooldown_remaining =
            dodge_cooldown_duration(market_effects.dodge_cooldown_multiplier());
        vitals.invulnerable_remaining = vitals.invulnerable_remaining.max(DODGE_INVULNERABILITY);
        market_effects.begin_dodge_control_guard();
    }
    dodge.requested = false;

    let direction = Vec3::new(movement.x, 0.0, movement.y);
    transform.translation += direction * build.move_speed * time.delta_secs();
    clamp_hero_position(&mut transform.translation);
    if direction.length_squared() > 0.01 {
        let yaw = direction.x.atan2(direction.z);
        transform.rotation = Quat::from_rotation_y(yaw);
    }
}

fn dodge_cooldown_duration(multiplier: f32) -> f32 {
    DODGE_COOLDOWN_SECONDS * multiplier
}

pub fn dodge_destination(origin: Vec3, direction: Vec2) -> Vec3 {
    let direction = direction.normalize_or_zero();
    let mut destination = origin + Vec3::new(direction.x, 0.0, direction.y) * DODGE_DISTANCE;
    clamp_hero_position(&mut destination);
    destination
}

fn clamp_hero_position(position: &mut Vec3) {
    position.x = position.x.clamp(HERO_MIN_X, HERO_MAX_X);
    position.z = position.z.clamp(HERO_MIN_Z, HERO_MAX_Z);
}

pub(crate) fn reset_run(world: &mut World) {
    world.insert_resource(TouchJoystickState::default());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phone_joystick_only_starts_inside_the_visible_lower_left_pad() {
        for (width, height) in [(667.0, 375.0), (844.0, 390.0)] {
            let metrics = hud_metrics(width, height);
            assert!(joystick_contains(metrics.joystick_center, metrics));
            assert!(joystick_contains(
                metrics.joystick_center + Vec2::X * metrics.joystick_size * 0.49,
                metrics
            ));
            assert!(!joystick_contains(Vec2::new(metrics.safe_x, 40.0), metrics));
            assert!(!joystick_contains(
                Vec2::new(width - 40.0, height - 40.0),
                metrics
            ));
        }
    }

    #[test]
    fn joystick_deadzone_scales_then_clamps_drag_strength() {
        let metrics = hud_metrics(844.0, 390.0);
        let origin = metrics.joystick_center;

        assert_eq!(
            joystick_intent(
                origin,
                origin + Vec2::X * (metrics.deadzone - 0.1),
                metrics.drag_radius,
                metrics.deadzone,
            ),
            Vec2::ZERO
        );
        let partial = joystick_intent(
            origin,
            origin + Vec2::new(metrics.drag_radius * 0.5, 0.0),
            metrics.drag_radius,
            metrics.deadzone,
        );
        assert!(partial.x > 0.0 && partial.x < 1.0);
        assert_eq!(
            joystick_intent(
                origin,
                origin + Vec2::Y * metrics.drag_radius * 2.0,
                metrics.drag_radius,
                metrics.deadzone,
            ),
            Vec2::Y
        );
    }

    #[test]
    fn joystick_keeps_the_first_touch_until_it_is_cleared() {
        let mut joystick = TouchJoystickState::default();
        assert!(joystick.begin(7, Vec2::new(80.0, 320.0)));
        assert!(!joystick.begin(11, Vec2::new(90.0, 315.0)));
        assert_eq!(joystick.touch_id, Some(7));

        joystick.clear();
        assert!(joystick.begin(11, Vec2::new(90.0, 315.0)));
        assert_eq!(joystick.touch_id, Some(11));
    }

    #[test]
    fn release_cancel_or_missing_touch_ends_joystick_tracking() {
        assert!(!touch_tracking_ended(true, false, false));
        assert!(touch_tracking_ended(false, false, false));
        assert!(touch_tracking_ended(false, true, false));
        assert!(touch_tracking_ended(false, false, true));
    }

    #[test]
    fn dodge_moves_exact_distance_until_it_reaches_the_arena_edge() {
        let open = dodge_destination(Vec3::ZERO, Vec2::X);
        assert!((open.x - 3.2).abs() < 0.0001);
        assert_eq!(open.z, 0.0);

        let clamped = dodge_destination(Vec3::new(9.0, 0.0, 11.5), Vec2::ONE);
        assert_eq!(clamped.x, HERO_MAX_X);
        assert_eq!(clamped.z, HERO_MAX_Z);
    }

    #[test]
    fn arena_bounds_match_the_gameplay_spec() {
        assert_eq!((HERO_MIN_X, HERO_MAX_X), (-10.0, 10.0));
        assert_eq!((HERO_MIN_Z, HERO_MAX_Z), (-12.0, 12.0));
    }

    #[test]
    fn wind_boots_reduce_dodge_cooldown_by_fifteen_percent() {
        let expected = DODGE_COOLDOWN_SECONDS * 0.85;
        assert!((dodge_cooldown_duration(0.85) - expected).abs() < f32::EPSILON);
    }

    #[test]
    fn autoplay_direction_keeps_changing_when_run_elapsed_is_frozen() {
        let before = autoplay_movement(1_500.0);
        let during_archive = autoplay_movement(1_501.0);
        assert_ne!(before, during_archive);
        assert!(before.length() <= 1.0);
        assert!(during_archive.length() <= 1.0);
    }
}
