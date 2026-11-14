//! Physical collapse and reconstruction. These never move the Core collider.
use super::*;
use bevy::light::{NotShadowCaster, NotShadowReceiver};

const REBUILD_RISE: f32 = 0.85;
#[derive(Component)]
pub(super) struct GateDust {
    remaining: f32,
    velocity: Vec3,
    size: f32,
}

pub(super) fn rebuild_offset(remaining: f32) -> f32 {
    let progress = 1.0 - (remaining / REBUILD_RISE).clamp(0.0, 1.0);
    // A quick heavy lift followed by a soft settle, not a springy toy bounce.
    -2.4 * (1.0 - progress).powi(3)
}

pub(super) fn gate_transition(
    mut commands: Commands,
    session: Res<GameSession>,
    autoplay: Option<Res<Autoplay>>,
    art: Option<Res<GateDebrisArt>>,
    gates: Query<&GlobalTransform, With<GateArt>>,
    mut impact: ResMut<GateImpact>,
) {
    if session.paused || session.save_active || session.finished {
        return;
    }
    let down = session.core_hp <= 0.0 || session.gate_revive_remaining > 0.0;
    let Some(previous_down) = impact.previous_down.replace(down) else {
        return;
    };
    if down == previous_down {
        return;
    }
    impact.shudder = 0.0;
    impact.rebuild_remaining = if down { 0.0 } else { REBUILD_RISE };
    if autoplay.is_some() {
        return;
    }
    let (Some(art), Ok(gate)) = (art, gates.single()) else {
        return;
    };
    // Independent world-space particles remain visible after the wall hides.
    for index in 0..18 {
        let z = -5.4 + index as f32 * 0.64;
        let origin = gate.transform_point(Vec3::new(0.5, -0.8, z)).with_y(0.18);
        let size = 0.38 + impact.unit() * 0.35;
        commands.spawn((
            Name::new(if down {
                "Gate collapse dust"
            } else {
                "Gate reconstruction dust"
            }),
            GateDust {
                remaining: 1.2,
                velocity: Vec3::new(
                    if down { 1.6 } else { 0.7 },
                    0.45,
                    (impact.unit() - 0.5) * 0.7,
                ),
                size,
            },
            Mesh3d(art.dust.clone()),
            MeshMaterial3d(art.dust_fades[0].clone()),
            Transform::from_translation(origin).with_scale(Vec3::new(size, size * 0.4, size)),
            NotShadowCaster,
            NotShadowReceiver,
        ));
        if down {
            commands.spawn((
                Name::new("Gate collapse masonry"),
                GateDebris {
                    velocity: Vec3::new(
                        1.4 + impact.unit() * 2.5,
                        1.8 + impact.unit() * 2.0,
                        (impact.unit() - 0.5) * 2.0,
                    ),
                    spin: Vec3::new(2.0, 4.0, 3.0),
                    size: size * 0.48,
                    remaining: 1.2,
                },
                Mesh3d(art.chip.clone()),
                MeshMaterial3d(art.stone.clone()),
                Transform::from_translation(origin + Vec3::Y * 0.7)
                    .with_scale(Vec3::splat(size * 0.48)),
            ));
        }
    }
}

pub(super) fn animate_dust(
    mut commands: Commands,
    time: Res<Time<Real>>,
    session: Res<GameSession>,
    art: Option<Res<GateDebrisArt>>,
    mut dust: Query<(
        Entity,
        &mut GateDust,
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
) {
    if session.paused || session.save_active {
        return;
    }
    let Some(art) = art else {
        return;
    };
    for (entity, mut dust, mut transform, mut material) in &mut dust {
        dust.remaining -= time.delta_secs().min(0.05);
        if dust.remaining <= 0.0 || session.finished {
            commands.entity(entity).despawn();
            continue;
        }
        let age = 1.0 - dust.remaining / 1.2;
        transform.translation += dust.velocity * time.delta_secs().min(0.05);
        let size = dust.size * (1.0 + age * 1.8);
        transform.scale = Vec3::new(size, size * 0.45, size);
        material.0 = art.dust_fades[(age * 8.0).floor().clamp(0.0, 7.0) as usize].clone();
    }
}
pub(super) fn reset_run(world: &mut World) {
    let entities: Vec<_> = world
        .query_filtered::<Entity, With<GateDust>>()
        .iter(world)
        .collect();
    for entity in entities {
        world.despawn(entity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn collapse_and_rebuild_are_single_edges_and_pause_freezes_lift() {
        let mut app = super::super::tests::gate_app(1000.0);
        app.add_systems(Update, gate_transition.before(shudder_gate));
        app.update();
        app.world_mut()
            .resource_mut::<GameSession>()
            .gate_revive_remaining = 3.0;
        app.update();
        assert_eq!(
            app.world().resource::<GateImpact>().previous_down,
            Some(true)
        );
        app.world_mut()
            .resource_mut::<GameSession>()
            .gate_revive_remaining = 0.0;
        app.update();
        assert!(app.world().resource::<GateImpact>().rebuild_remaining > 0.7);
        let remaining = app.world().resource::<GateImpact>().rebuild_remaining;
        app.world_mut().resource_mut::<GameSession>().paused = true;
        super::super::tests::advance(&mut app, 500);
        assert_eq!(
            app.world().resource::<GateImpact>().rebuild_remaining,
            remaining
        );
        app.world_mut().resource_mut::<GameSession>().paused = false;
        for _ in 0..20 {
            super::super::tests::advance(&mut app, 50);
        }
        assert_eq!(app.world().resource::<GateImpact>().rebuild_remaining, 0.0);
        assert_eq!(rebuild_offset(0.0), 0.0);
        assert_eq!(app.world().resource::<GameSession>().core_hp, 1000.0);
    }
}
