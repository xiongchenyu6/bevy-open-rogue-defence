//! Small, consistent tactile feedback without rearranging layout or changing rules.
use super::{GameSession, ui::HudValue};
use bevy::prelude::*;

pub(crate) struct UiResponsePlugin;
impl Plugin for UiResponsePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (prepare, animate)
                .chain()
                .before(bevy::ui::UiSystems::Layout),
        );
    }
}
#[derive(Component)]
struct ButtonResponse {
    scale: Vec2,
}
#[derive(Component)]
struct CounterResponse {
    previous: u32,
    remaining: f32,
    color: Color,
}
fn amount(value: HudValue, session: &GameSession) -> Option<u32> {
    match value {
        HudValue::Gold => Some(session.gold),
        HudValue::Wood => Some(session.wood),
        HudValue::Kills => Some(session.kills),
        _ => None,
    }
}
fn prepare(
    mut commands: Commands,
    session: Res<GameSession>,
    buttons: Query<
        (Entity, &UiTransform),
        (With<Button>, Without<ButtonResponse>, Without<HudValue>),
    >,
    counters: Query<(Entity, &HudValue, &TextColor), Without<CounterResponse>>,
) {
    for (entity, transform) in &buttons {
        commands.entity(entity).insert(ButtonResponse {
            scale: transform.scale,
        });
    }
    for (entity, value, color) in &counters {
        if let Some(previous) = amount(*value, &session) {
            commands.entity(entity).insert(CounterResponse {
                previous,
                remaining: 0.0,
                color: color.0,
            });
        }
    }
}
fn animate(
    time: Res<Time<Real>>,
    session: Res<GameSession>,
    mut buttons: Query<
        (&Interaction, &ButtonResponse, &mut UiTransform),
        (With<Button>, Without<HudValue>),
    >,
    mut counters: Query<(
        &HudValue,
        &mut CounterResponse,
        &mut UiTransform,
        &mut TextColor,
    )>,
) {
    let dt = time.delta_secs().min(0.05);
    for (interaction, response, mut transform) in &mut buttons {
        let target = response.scale
            * match interaction {
                Interaction::Pressed => 0.965,
                Interaction::Hovered => 1.012,
                Interaction::None => 1.0,
            };
        transform.scale = transform.scale.lerp(target, 1.0 - (-26.0 * dt).exp());
    }
    for (value, mut response, mut transform, mut color) in &mut counters {
        let current = amount(*value, &session).unwrap_or(response.previous);
        if current > response.previous {
            response.remaining = 0.32;
        }
        response.previous = current;
        response.remaining = (response.remaining - dt).max(0.0);
        let pulse = response.remaining / 0.32;
        transform.scale = Vec2::splat(1.0 + pulse * 0.065);
        color.0 = response
            .color
            .mix(&Color::srgb(1.0, 0.98, 0.8), pulse * 0.8);
    }
}
