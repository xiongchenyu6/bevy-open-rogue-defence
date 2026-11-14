//! Quiet ordinary deaths, heavier debris on elite/boss kills. Cosmetic only.
use super::{HitFeedback, HitWeight};
use crate::game::GameSession;
use bevy::{light::NotShadowCaster, prelude::*};
use std::collections::HashSet;

const CAPACITY: usize = 128;

#[derive(Resource)]
pub(super) struct Art {
    chip: Handle<Mesh>,
    dust: Handle<Mesh>,
    stone: Handle<StandardMaterial>,
    ash: Handle<StandardMaterial>,
}

#[derive(Component)]
pub(super) struct Debris {
    age: f32,
    duration: f32,
    origin: Vec3,
    velocity: Vec3,
    scale: Vec3,
    turn: f32,
    dust: bool,
}

pub(super) fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(Art {
        chip: meshes.add(Tetrahedron::default()),
        dust: meshes.add(Sphere::new(0.5).mesh().ico(1).unwrap()),
        stone: materials.add(StandardMaterial {
            base_color: Color::srgb(0.36, 0.29, 0.22),
            perceptual_roughness: 1.0,
            reflectance: 0.08,
            ..default()
        }),
        ash: materials.add(StandardMaterial {
            base_color: Color::srgba(0.40, 0.34, 0.27, 0.12),
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 1.0,
            reflectance: 0.0,
            ..default()
        }),
    });
}

fn count(weight: HitWeight) -> usize {
    match weight {
        HitWeight::Normal => 4,
        HitWeight::Elite => 8,
        HitWeight::Boss => 14,
    }
}

fn pose(piece: &Debris) -> Transform {
    let t = (piece.age / piece.duration).clamp(0.0, 1.0);
    let mut position = piece.origin + piece.velocity * piece.age;
    position.y = (position.y - 5.0 * piece.age * piece.age).max(0.06);
    // Flecks settle and shrink; dust spreads low to the ground then dissipates.
    let size = if piece.dust {
        (0.5 + t * 2.0) * (1.0 - t).sqrt()
    } else {
        (1.0 - ((t - 0.45) / 0.55).max(0.0)).max(0.0)
    };
    Transform::from_translation(position)
        .with_scale(piece.scale * size)
        .with_rotation(Quat::from_euler(EulerRot::XYZ, t * piece.turn, t * 2.0, t))
}

pub(super) fn present(
    mut commands: Commands,
    time: Res<Time<Real>>,
    session: Res<GameSession>,
    feedback: Res<HitFeedback>,
    art: Res<Art>,
    mut pieces: Query<(Entity, &mut Debris, &mut Transform)>,
) {
    if session.finished {
        for (entity, _, _) in &pieces {
            commands.entity(entity).despawn();
        }
        return;
    }
    if session.paused {
        return;
    }
    let mut budget = CAPACITY.saturating_sub(pieces.iter().len());
    for (entity, mut piece, mut transform) in &mut pieces {
        piece.age += time.delta_secs().min(0.05);
        if piece.age >= piece.duration {
            commands.entity(entity).despawn();
        } else {
            *transform = pose(&piece);
        }
    }
    let mut seen = HashSet::new();
    for event in feedback.events().iter().filter(|event| event.killed) {
        if !seen.insert(event.target) {
            continue;
        }
        let direction = event.direction.with_y(0.0).normalize_or(Vec3::X);
        let side = direction.cross(Vec3::Y);
        let size = match event.weight {
            HitWeight::Normal => 1.0,
            HitWeight::Elite => 1.4,
            HitWeight::Boss => 2.1,
        };
        for index in 0..count(event.weight).min(budget) {
            budget -= 1;
            let phase = index as f32 * 2.399_963;
            let dust = index % 4 == 0;
            let piece = Debris {
                age: 0.0,
                duration: if dust {
                    0.5
                } else {
                    0.38 + index as f32 * 0.018
                },
                origin: event.position.with_y(if dust { 0.12 } else { 0.5 }),
                velocity: (direction * (1.0 + phase.cos() * 0.6)
                    + side * phase.sin() * 1.6
                    + Vec3::Y * if dust { 0.1 } else { 1.5 + phase.cos().abs() })
                    * size,
                scale: if dust {
                    Vec3::new(0.95, 0.15, 0.7)
                } else {
                    Vec3::new(0.12, 0.18, 0.09)
                } * size,
                turn: 4.0 + phase,
                dust,
            };
            let transform = pose(&piece);
            commands.spawn((
                Name::new(if dust {
                    "Death settling dust"
                } else {
                    "Death directional chip"
                }),
                Mesh3d(if dust {
                    art.dust.clone()
                } else {
                    art.chip.clone()
                }),
                MeshMaterial3d(if dust {
                    art.ash.clone()
                } else {
                    art.stone.clone()
                }),
                NotShadowCaster,
                transform,
                piece,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::bonds::DamageElement;
    use crate::game::hit_feedback::HitEvent;
    use std::time::Duration;

    #[test]
    fn death_debris_is_deduplicated_bounded_and_freezes_with_pause() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Time<Real>>()
            .init_resource::<GameSession>()
            .init_resource::<HitFeedback>()
            .add_systems(Update, present);
        app.world_mut().run_system_cached(setup).unwrap();
        let target = app.world_mut().spawn_empty().id();
        let hit = HitEvent {
            target,
            position: Vec3::ZERO,
            direction: Vec3::X,
            element: DamageElement::Physical,
            weight: HitWeight::Normal,
            killed: true,
            critical: false,
        };
        for _ in 0..30 {
            app.world_mut().resource_mut::<HitFeedback>().push(hit);
        }
        app.update();
        assert_eq!(
            app.world_mut().query::<&Debris>().iter(app.world()).count(),
            4
        );
        app.world_mut().resource_mut::<HitFeedback>().events.clear();
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(Duration::from_secs(1));
        app.update();
        assert!(
            app.world_mut()
                .query::<&Debris>()
                .iter(app.world())
                .all(|p| p.age == 0.0)
        );
        app.world_mut().resource_mut::<GameSession>().paused = false;
        for _ in 0..256 {
            let target = app.world_mut().spawn_empty().id();
            app.world_mut()
                .resource_mut::<HitFeedback>()
                .push(HitEvent {
                    target,
                    weight: HitWeight::Boss,
                    ..hit
                });
        }
        app.update();
        assert_eq!(
            app.world_mut().query::<&Debris>().iter(app.world()).count(),
            CAPACITY
        );
        app.world_mut().resource_mut::<HitFeedback>().events.clear();
        for _ in 0..25 {
            app.update();
        }
        assert_eq!(
            app.world_mut().query::<&Debris>().iter(app.world()).count(),
            0
        );
    }
}
