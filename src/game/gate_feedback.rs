//! Gate impact feel: the wall shudders when struck, throws stone chips, and
//! shows progressively heavier cracks and rubble as its health falls.
//!
//! Gameplay keeps reading the `Core` transform, so every visual reaction moves
//! the [`GateArt`] child instead.

use bevy::prelude::*;

use super::combat::{CombatSet, SurvivalSet};
use super::hit_feedback::HitFeedbackSet;
use super::{Autoplay, GameSession};

/// Damage since the last shudder that triggers a new one, as a max-HP fraction.
const IMPULSE_DAMAGE_FRACTION: f32 = 0.004;
const MIN_IMPULSE_SECONDS: f32 = 0.16;
const SHUDDER_DECAY_PER_SECOND: f32 = 3.2;
const SHUDDER_PUSH: f32 = 0.16;
const SHUDDER_TILT: f32 = 0.035;
const DEBRIS_GRAVITY: f32 = 14.0;
const DEBRIS_LIFETIME: f32 = 0.9;
/// Health fractions below which crack stages 1, 2 and 3 appear.
const STAGE_THRESHOLDS: [f32; 3] = [0.75, 0.5, 0.25];

pub(crate) struct GateFeedbackPlugin;

impl Plugin for GateFeedbackPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GateImpact>().add_systems(
            Update,
            (
                attach_damage_art,
                detect_gate_damage,
                shudder_gate,
                show_damage_stages,
                animate_debris,
            )
                .chain()
                .after(CombatSet)
                .after(SurvivalSet)
                .before(HitFeedbackSet),
        );
    }
}

/// Visual-only wrapper for the wall, doors and towers under `Core`.
#[derive(Component)]
pub(crate) struct GateArt;

#[derive(Resource, Debug, Default)]
pub(crate) struct GateImpact {
    previous_hp: Option<f32>,
    pending_damage: f32,
    since_impulse: f32,
    shudder: f32,
    shudder_clock: f32,
    variation: u32,
    /// Set for one frame when a shudder starts; audio and camera read it.
    struck: Option<f32>,
}

impl GateImpact {
    /// Strength in `0..=1` of the shudder that started this frame, if any.
    pub(crate) const fn struck(&self) -> Option<f32> {
        self.struck
    }

    fn unit(&mut self) -> f32 {
        self.variation = self
            .variation
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        (self.variation >> 8) as f32 / (1u32 << 24) as f32
    }
}

#[derive(Component)]
struct GateDamageStage(u8);

#[derive(Component)]
struct GateDamageArtAttached;

#[derive(Component)]
struct GateDebris {
    velocity: Vec3,
    spin: Vec3,
    size: f32,
    remaining: f32,
}

#[derive(Resource)]
struct GateDebrisArt {
    chip: Handle<Mesh>,
    stone: Handle<StandardMaterial>,
}

pub(crate) fn damage_stage(hp: f32, max_hp: f32) -> u8 {
    let fraction = hp / max_hp.max(1.0);
    STAGE_THRESHOLDS
        .iter()
        .filter(|threshold| hp > 0.0 && fraction < **threshold)
        .count() as u8
}

fn impulse_strength(damage: f32, max_hp: f32) -> f32 {
    (0.3 + damage / max_hp.max(1.0) * 18.0).min(1.0)
}

fn attach_damage_art(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    gates: Query<Entity, (With<GateArt>, Without<GateDamageArtAttached>)>,
) {
    let Some(gate) = gates.iter().next() else {
        return;
    };
    let sliver = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let crack = materials.add(StandardMaterial {
        base_color: Color::srgb(0.04, 0.035, 0.03),
        perceptual_roughness: 1.0,
        ..default()
    });
    let molten = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.06, 0.02),
        emissive: LinearRgba::rgb(5.0, 1.3, 0.25),
        ..default()
    });
    let rubble = materials.add(StandardMaterial {
        base_color: Color::srgb(0.33, 0.37, 0.38),
        perceptual_roughness: 0.95,
        ..default()
    });
    commands.insert_resource(GateDebrisArt {
        chip: sliver.clone(),
        stone: rubble.clone(),
    });
    commands.entity(gate).insert(GateDamageArtAttached);
    commands.entity(gate).with_children(|parent| {
        // Zig-zag cracks across the invader-facing masonry and doors (+X face).
        let cracks: [(u8, f32, f32, f32, f32); 12] = [
            (1, -3.4, 0.9, 0.7, 0.5),
            (1, 2.7, 0.3, 0.6, -0.6),
            (1, 0.4, 1.0, 0.8, 0.25),
            (1, -1.0, -0.2, 0.5, -0.9),
            (2, 4.6, 0.6, 0.9, 0.4),
            (2, -4.9, 0.0, 0.8, -0.35),
            (2, -0.5, 0.2, 1.1, 1.1),
            (2, 3.4, -0.4, 0.7, 1.3),
            (2, 1.2, -0.3, 0.9, -1.2),
            (3, -2.6, 0.2, 1.3, 0.8),
            (3, 2.1, 0.8, 1.2, -0.7),
            (3, 0.0, -0.1, 1.4, 0.05),
        ];
        for (index, (stage, z, y, length, angle)) in cracks.into_iter().enumerate() {
            let material = if stage == 3 { &molten } else { &crack };
            for (segment, bend) in [(0.0, 0.0), (1.0, 0.7)] {
                let offset = Vec3::new(0.0, length * 0.42 * segment, length * 0.18 * segment);
                parent.spawn((
                    Name::new(format!("Gate crack {index}")),
                    GateDamageStage(stage),
                    Mesh3d(sliver.clone()),
                    MeshMaterial3d(material.clone()),
                    Transform::from_translation(Vec3::new(0.74, y, z) + offset)
                        .with_rotation(Quat::from_rotation_x(angle + bend))
                        .with_scale(Vec3::new(0.08, length * 0.55, 0.13)),
                    Visibility::Hidden,
                ));
            }
        }
        // Fallen stone piles in front of the wall.
        let piles: [(u8, f32, f32); 10] = [
            (1, -3.0, 0.32),
            (1, 3.2, 0.26),
            (2, -4.4, 0.4),
            (2, 1.5, 0.34),
            (2, 4.7, 0.3),
            (2, -1.6, 0.28),
            (3, -2.2, 0.48),
            (3, 2.6, 0.52),
            (3, 0.6, 0.42),
            (3, -5.0, 0.44),
        ];
        for (index, (stage, z, size)) in piles.into_iter().enumerate() {
            for chunk in 0..3 {
                let spread = chunk as f32 - 1.0;
                parent.spawn((
                    Name::new(format!("Gate rubble {index}")),
                    GateDamageStage(stage),
                    Mesh3d(sliver.clone()),
                    MeshMaterial3d(rubble.clone()),
                    Transform::from_xyz(
                        1.05 + 0.22 * spread.abs(),
                        -0.95 + size * 0.35,
                        z + spread * size * 0.8,
                    )
                    .with_rotation(Quat::from_euler(
                        EulerRot::XYZ,
                        0.4 * spread,
                        0.7 + index as f32,
                        0.3,
                    ))
                    .with_scale(Vec3::splat(size * (1.0 - 0.25 * spread.abs()))),
                    Visibility::Hidden,
                ));
            }
        }
    });
}

fn detect_gate_damage(
    time: Res<Time<Real>>,
    session: Res<GameSession>,
    autoplay: Option<Res<Autoplay>>,
    debris_art: Option<Res<GateDebrisArt>>,
    gates: Query<&GlobalTransform, With<GateArt>>,
    mut commands: Commands,
    mut impact: ResMut<GateImpact>,
) {
    impact.struck = None;
    let hp = session.core_hp;
    let previous = impact.previous_hp.replace(hp).unwrap_or(hp);
    if session.paused || hp <= 0.0 || session.gate_revive_remaining > 0.0 {
        impact.pending_damage = 0.0;
        return;
    }
    impact.pending_damage += (previous - hp).max(0.0);
    impact.since_impulse += time.delta_secs();
    let threshold = session.core_max_hp.max(1.0) * IMPULSE_DAMAGE_FRACTION;
    let stage_crossed =
        damage_stage(hp, session.core_max_hp) > damage_stage(previous, session.core_max_hp);
    if impact.pending_damage <= 0.0
        || (impact.pending_damage < threshold && !stage_crossed)
        || (impact.since_impulse < MIN_IMPULSE_SECONDS && !stage_crossed)
    {
        return;
    }
    let mut strength = impulse_strength(impact.pending_damage, session.core_max_hp);
    if stage_crossed {
        strength = 1.0;
    }
    impact.pending_damage = 0.0;
    impact.since_impulse = 0.0;
    impact.shudder = (impact.shudder + strength).min(1.0);
    impact.struck = Some(strength);
    if autoplay.is_some() {
        return;
    }
    let (Some(art), Ok(gate)) = (debris_art, gates.single()) else {
        return;
    };
    let chips = if stage_crossed {
        10
    } else {
        2 + (strength * 4.0) as usize
    };
    for _ in 0..chips {
        let z = (impact.unit() - 0.5) * 9.0;
        let y = 0.2 + impact.unit() * 1.8;
        let origin = gate.transform_point(Vec3::new(0.8, y, z));
        let velocity = Vec3::new(
            1.5 + impact.unit() * 3.0,
            2.5 + impact.unit() * 3.5,
            (impact.unit() - 0.5) * 3.0,
        );
        let spin = Vec3::new(impact.unit(), impact.unit(), impact.unit()) * 12.0;
        let size = 0.08 + impact.unit() * 0.14;
        commands.spawn((
            Name::new("Gate stone chip"),
            GateDebris {
                velocity,
                spin,
                size,
                remaining: DEBRIS_LIFETIME,
            },
            Mesh3d(art.chip.clone()),
            MeshMaterial3d(art.stone.clone()),
            Transform::from_translation(origin).with_scale(Vec3::splat(size)),
        ));
    }
}

/// Struck walls rock back away from the invaders, then ring down.
fn shudder_gate(
    time: Res<Time<Real>>,
    mut impact: ResMut<GateImpact>,
    mut gates: Query<&mut Transform, With<GateArt>>,
) {
    let delta = time.delta_secs();
    impact.shudder = (impact.shudder - SHUDDER_DECAY_PER_SECOND * delta).max(0.0);
    impact.shudder_clock += delta;
    let amount = impact.shudder * impact.shudder;
    let clock = impact.shudder_clock;
    for mut transform in &mut gates {
        transform.translation = Vec3::new(
            -SHUDDER_PUSH * amount * (0.6 + 0.4 * (clock * 48.0).sin()),
            0.0,
            SHUDDER_PUSH * 0.35 * amount * (clock * 61.0).sin(),
        );
        transform.rotation = Quat::from_rotation_z(SHUDDER_TILT * amount * (clock * 39.0).sin());
    }
}

fn show_damage_stages(
    session: Res<GameSession>,
    mut pieces: Query<(&GateDamageStage, &mut Visibility)>,
) {
    let stage = damage_stage(session.core_hp, session.core_max_hp);
    for (piece, mut visibility) in &mut pieces {
        visibility.set_if_neq(if piece.0 <= stage {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}

fn animate_debris(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut debris: Query<(Entity, &mut GateDebris, &mut Transform)>,
) {
    let delta = time.delta_secs();
    for (entity, mut chip, mut transform) in &mut debris {
        chip.remaining -= delta;
        if chip.remaining <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        chip.velocity.y -= DEBRIS_GRAVITY * delta;
        transform.translation += chip.velocity * delta;
        if transform.translation.y < 0.05 {
            transform.translation.y = 0.05;
            chip.velocity *= Vec3::new(0.5, -0.3, 0.5);
        }
        transform.rotate(Quat::from_scaled_axis(chip.spin * delta));
        // Chips shrink away over their last quarter second.
        transform.scale = Vec3::splat(chip.size * (chip.remaining / 0.25).min(1.0));
    }
}

pub(crate) fn reset_run(world: &mut World) {
    world.insert_resource(GateImpact::default());
    let debris: Vec<Entity> = world
        .query_filtered::<Entity, With<GateDebris>>()
        .iter(world)
        .collect();
    for entity in debris {
        world.despawn(entity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damage_stages_follow_health_quarters_and_clear_when_rebuilt() {
        assert_eq!(damage_stage(1_000.0, 1_000.0), 0);
        assert_eq!(damage_stage(740.0, 1_000.0), 1);
        assert_eq!(damage_stage(499.0, 1_000.0), 2);
        assert_eq!(damage_stage(120.0, 1_000.0), 3);
        // A collapsed gate is hidden; it never shows a leftover stage.
        assert_eq!(damage_stage(0.0, 1_000.0), 0);
    }

    fn gate_app(hp: f32) -> App {
        let mut app = App::new();
        app.init_resource::<Time<Real>>()
            .insert_resource(GameSession {
                core_hp: hp,
                ..default()
            })
            .init_resource::<GateImpact>()
            .add_systems(Update, (detect_gate_damage, shudder_gate).chain());
        app.world_mut().spawn((GateArt, Transform::default()));
        app.update();
        app
    }

    fn advance(app: &mut App, millis: u64) {
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(std::time::Duration::from_millis(millis));
        app.update();
    }

    #[test]
    fn chip_damage_accumulates_until_it_rocks_the_wall_back() {
        let mut app = gate_app(1_000.0);
        app.world_mut().resource_mut::<GameSession>().core_hp = 998.0;
        advance(&mut app, 200);
        assert!(app.world().resource::<GateImpact>().struck().is_none());
        app.world_mut().resource_mut::<GameSession>().core_hp = 995.0;
        advance(&mut app, 10);
        let impact = app.world().resource::<GateImpact>();
        assert!(impact.struck().is_some());
        let mut gates = app
            .world_mut()
            .query_filtered::<&Transform, With<GateArt>>();
        let gate = gates.single(app.world()).unwrap();
        assert!(
            gate.translation.x < 0.0,
            "the wall is pushed away from invaders"
        );
    }

    #[test]
    fn crossing_a_damage_stage_is_always_a_full_strength_hit() {
        let mut app = gate_app(760.0);
        app.world_mut().resource_mut::<GameSession>().core_hp = 749.0;
        advance(&mut app, 10);
        assert_eq!(app.world().resource::<GateImpact>().struck(), Some(1.0));
    }
}
