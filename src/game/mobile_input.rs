use bevy::{prelude::*, window::PrimaryWindow};

use super::combat::{CombatSet, SurvivalSet};
use super::market::MarketEffects;
use super::{
    ActiveSkill, Autoplay, DODGE_COOLDOWN_SECONDS, DODGE_DISTANCE, DODGE_INVULNERABILITY,
    DodgeButton, DodgeState, GameSession, Hero, HeroVitals, MoveIntent, PlayerBuild,
    UltimateButton,
};

pub const HERO_MIN_X: f32 = -10.0;
pub const HERO_MAX_X: f32 = 10.0;
pub const HERO_MIN_Z: f32 = -6.0;
pub const HERO_MAX_Z: f32 = 6.0;

pub struct MobileInputPlugin;

impl Plugin for MobileInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (read_move_input, move_and_dodge_hero)
                .chain()
                .after(SurvivalSet)
                .before(CombatSet),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn read_move_input(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    touches: Res<Touches>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    session: Res<GameSession>,
    vitals: Res<HeroVitals>,
    mut intent: ResMut<MoveIntent>,
    mut dodge: ResMut<DodgeState>,
    mut active: ResMut<ActiveSkill>,
    dodge_buttons: Query<&Interaction, (With<DodgeButton>, Changed<Interaction>)>,
    ultimate_buttons: Query<&Interaction, (With<UltimateButton>, Changed<Interaction>)>,
    autoplay: Option<Res<Autoplay>>,
) {
    if session.paused || session.finished || !vitals.can_act() {
        intent.0 = Vec2::ZERO;
        dodge.requested = false;
        active.requested = false;
        return;
    }

    if autoplay.is_some() {
        // `session.elapsed` deliberately freezes during Archive. Use Bevy's
        // deterministic simulation clock so the capture bot keeps circling
        // instead of walking into one arena edge for the whole epilogue.
        intent.0 = autoplay_movement(time.elapsed_secs());
        if active.remaining <= 0.0 && session.elapsed > 2.0 {
            active.requested = true;
        }
        if dodge.ready() && session.elapsed > 1.0 {
            dodge.request();
        }
        return;
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
    if keyboard.just_pressed(KeyCode::Space)
        || ultimate_buttons
            .iter()
            .any(|interaction| *interaction == Interaction::Pressed)
    {
        active.requested = true;
    }
    if keyboard.just_pressed(KeyCode::ShiftLeft)
        || keyboard.just_pressed(KeyCode::ShiftRight)
        || keyboard.just_pressed(KeyCode::ControlLeft)
        || keyboard.just_pressed(KeyCode::ControlRight)
        || dodge_buttons
            .iter()
            .any(|interaction| *interaction == Interaction::Pressed)
    {
        dodge.request();
    }

    if let Ok(window) = window_query.single() {
        for touch in touches.iter() {
            if touch.start_position().x < window.width() * 0.55 {
                let drag = touch.distance() / 72.0;
                movement = Vec2::new(drag.x, drag.y).clamp_length_max(1.0);
                break;
            }
        }
    }

    intent.0 = movement.clamp_length_max(1.0);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dodge_moves_exact_distance_until_it_reaches_the_arena_edge() {
        let open = dodge_destination(Vec3::ZERO, Vec2::X);
        assert!((open.x - 3.2).abs() < 0.0001);
        assert_eq!(open.z, 0.0);

        let clamped = dodge_destination(Vec3::new(9.0, 0.0, 5.5), Vec2::ONE);
        assert_eq!(clamped.x, HERO_MAX_X);
        assert_eq!(clamped.z, HERO_MAX_Z);
    }

    #[test]
    fn arena_bounds_match_the_gameplay_spec() {
        assert_eq!((HERO_MIN_X, HERO_MAX_X), (-10.0, 10.0));
        assert_eq!((HERO_MIN_Z, HERO_MAX_Z), (-6.0, 6.0));
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
