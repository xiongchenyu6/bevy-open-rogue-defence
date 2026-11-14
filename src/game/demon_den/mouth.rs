//! The invasion entrance reacts to the actual scheduled raid, not a second timer.
use super::*;
use crate::game::RaidWarning;

#[derive(Component)]
pub(super) struct MouthLight;
#[derive(Component)]
pub(super) struct FallingStone {
    remaining: f32,
    velocity: Vec3,
}
#[derive(Resource)]
pub(super) struct MouthArt {
    stone: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    warning: bool,
    clock: f32,
    last_elapsed: f32,
}

pub(super) fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(MouthArt {
        stone: meshes.add(Sphere::new(1.0).mesh().ico(1).unwrap()),
        material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.27, 0.23, 0.29),
            perceptual_roughness: 1.0,
            reflectance: 0.12,
            ..default()
        }),
        warning: false,
        clock: 0.0,
        last_elapsed: 0.0,
    });
}

fn mouth_intensity(clock: f32, remaining: f32) -> f32 {
    if remaining > 0.0 {
        let urgency = 1.0 - (remaining / 3.0).clamp(0.0, 1.0);
        // Warning starts at a distinctly stronger value than idle. The increasing
        // pulse frequency anticipates the actual group release at remaining=0.
        1_150_000.0 + (0.5 + 0.5 * (clock * (7.0 + urgency * 11.0)).sin()) * 700_000.0
    } else {
        780_000.0 + (clock * 0.8).sin() * 90_000.0
    }
}

pub(super) fn animate(
    mut commands: Commands,
    time: Res<Time<Real>>,
    session: Res<GameSession>,
    raid: Option<Res<RaidWarning>>,
    art: Option<ResMut<MouthArt>>,
    mut lights: Query<&mut PointLight, With<MouthLight>>,
    mut stones: Query<(Entity, &mut FallingStone, &mut Transform)>,
) {
    let Some(mut art) = art else {
        return;
    };
    if session.elapsed < art.last_elapsed || session.finished {
        for (entity, _, _) in &mut stones {
            commands.entity(entity).despawn();
        }
        art.warning = false;
        art.clock = 0.0;
    }
    art.last_elapsed = session.elapsed;
    if session.paused || session.save_active {
        return;
    }
    let dt = time.delta_secs().min(0.05);
    art.clock += dt;
    let remaining = raid.as_ref().map_or(0.0, |raid| raid.remaining_secs);
    for mut light in &mut lights {
        light.intensity =
            mouth_intensity(art.clock, if session.finished { 0.0 } else { remaining });
        light.color = if remaining > 0.0 && !session.finished {
            Color::srgb(0.96, 0.30, 0.20)
        } else {
            Color::srgb(0.62, 0.25, 1.0)
        };
    }
    let warning = remaining > 0.0 && !session.finished;
    if warning && !art.warning {
        for index in 0..14 {
            let z = -4.6 + index as f32 * 0.70;
            commands.spawn((
                Name::new("Raid warning falling cavern stone"),
                FallingStone {
                    remaining: 1.8,
                    velocity: Vec3::new(-0.5 - index as f32 * 0.035, 0.0, z * 0.025),
                },
                Mesh3d(art.stone.clone()),
                MeshMaterial3d(art.material.clone()),
                Transform::from_xyz(
                    15.7 + (index as f32 * 1.4).sin() * 0.25,
                    2.8 + (index % 4) as f32 * 0.4,
                    z,
                )
                .with_scale(Vec3::new(0.10, 0.16, 0.12)),
            ));
        }
    }
    art.warning = warning;
    for (entity, mut stone, mut transform) in &mut stones {
        stone.remaining -= dt;
        if stone.remaining <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        stone.velocity.y -= dt * 9.8;
        transform.translation += stone.velocity * dt;
        transform.rotate_local_z(dt * 3.0);
        if transform.translation.y < 0.08 {
            transform.translation.y = 0.08;
            stone.velocity.y = stone.velocity.y.abs() * 0.22;
            stone.velocity.x *= 0.85;
        }
        if stone.remaining < 0.4 {
            transform.scale *= (1.0 - dt * 5.0).max(0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_warning_changes_light_and_pause_preserves_its_phase() {
        let mut app = App::new();
        app.init_resource::<GameSession>()
            .init_resource::<Time<Real>>()
            .init_resource::<RaidWarning>()
            .insert_resource(MouthArt {
                stone: Handle::default(),
                material: Handle::default(),
                warning: false,
                clock: 0.0,
                last_elapsed: 0.0,
            })
            .add_systems(Update, animate);
        let light = app
            .world_mut()
            .spawn((MouthLight, PointLight::default()))
            .id();
        app.update();
        let idle = app.world().get::<PointLight>(light).unwrap().intensity;
        app.world_mut().resource_mut::<RaidWarning>().remaining_secs = 3.0;
        app.update();
        assert!(app.world().get::<PointLight>(light).unwrap().intensity > idle);
        assert_eq!(
            app.world_mut()
                .query::<&FallingStone>()
                .iter(app.world())
                .count(),
            14
        );
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&FallingStone>()
                .iter(app.world())
                .count(),
            14,
            "one debris burst per real warning"
        );
        app.world_mut().resource_mut::<GameSession>().paused = true;
        let clock = app.world().resource::<MouthArt>().clock;
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(std::time::Duration::from_secs(1));
        app.update();
        assert_eq!(app.world().resource::<MouthArt>().clock, clock);
        app.world_mut().resource_mut::<GameSession>().paused = false;
        app.world_mut().resource_mut::<RaidWarning>().remaining_secs = 0.0;
        app.update();
        assert!(app.world().get::<PointLight>(light).unwrap().intensity < 900_000.0);
    }
}
