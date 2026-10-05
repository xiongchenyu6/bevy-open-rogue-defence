//! A single cavern spans every invasion lane. Its art never owns collision or spawning.
use super::combat::CombatSet;
use super::{ArenaEntity, Enemy, GameSession};
use bevy::prelude::*;

pub(super) struct DemonDenPlugin;
impl Plugin for DemonDenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, (emergence, animate).chain().after(CombatSet));
    }
}
#[derive(Resource)]
struct DenArt {
    puff: Handle<Mesh>,
    glow: Handle<StandardMaterial>,
}
#[derive(Component)]
struct Mist {
    origin: Vec3,
    phase: f32,
    scale: Vec3,
}
#[derive(Component)]
struct Emergence {
    remaining: f32,
    initial: f32,
}
#[derive(Resource, Default)]
struct DenClock(f32);
const MAX_EMERGENCE: usize = 32;

fn setup(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let rock = server.load("models/environment_rubble_large.glb#Scene0");
    let boulder = meshes.add(Sphere::new(1.0).mesh().ico(1).unwrap());
    let mist_mesh = meshes.add(Sphere::new(1.0).mesh().ico(2).unwrap());
    let tooth = meshes.add(ConicalFrustum {
        radius_top: 0.025,
        radius_bottom: 0.28,
        height: 1.0,
    });
    let black = materials.add(StandardMaterial {
        base_color: Color::srgb(0.004, 0.002, 0.008),
        unlit: true,
        ..default()
    });
    let ivory = materials.add(StandardMaterial {
        base_color: Color::srgb(0.69, 0.58, 0.78),
        perceptual_roughness: 0.7,
        emissive: LinearRgba::new(0.06, 0.035, 0.1, 1.0),
        ..default()
    });
    let glow = materials.add(StandardMaterial {
        base_color: Color::srgba(0.50, 0.06, 0.82, 0.30),
        emissive: LinearRgba::new(1.15, 0.025, 2.6, 1.0),
        alpha_mode: AlphaMode::Add,
        unlit: true,
        ..default()
    });
    let fog = materials.add(StandardMaterial {
        base_color: Color::srgba(0.26, 0.045, 0.42, 0.10),
        emissive: LinearRgba::new(0.12, 0.01, 0.22, 1.0),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    commands.insert_resource(DenArt {
        puff: mist_mesh.clone(),
        glow: glow.clone(),
    });
    commands.init_resource::<DenClock>();
    commands
        .spawn((
            Name::new("Demon den"),
            Transform::from_xyz(15.65, 0.0, 0.0).with_rotation(Quat::from_rotation_y(-0.55)),
            Visibility::default(),
            ArenaEntity,
        ))
        .with_children(|den| {
            // A solid black tunnel volume recedes behind the open rock arch. Spawned
            // enemies become visible as they leave this cavity towards decreasing X.
            den.spawn((
                Name::new("Demon den mouth"),
                Mesh3d(meshes.add(Cylinder::new(1.0, 1.0))),
                MeshMaterial3d(black),
                Transform::from_xyz(-0.50, 2.55, 0.0)
                    .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2))
                    .with_scale(Vec3::new(2.65, 0.30, 5.30)),
            ));
            // The authored rock arch provides depth without opaque background
            // spheres obscuring the mouth from the production camera.
            for index in 0..15 {
                let angle = index as f32 / 14.0 * std::f32::consts::PI;
                let position = Vec3::new(0.0, 0.15 + angle.sin() * 5.35, angle.cos() * 5.65);
                den.spawn((
                    Name::new("Demon den rock arch"),
                    WorldAssetRoot(rock.clone()),
                    Transform::from_translation(position - Vec3::Y * 0.65)
                        .with_rotation(Quat::from_euler(
                            EulerRot::YXZ,
                            index as f32 * 0.71,
                            0.13,
                            -angle * 0.10,
                        ))
                        .with_scale(Vec3::new(0.85, 1.35, 1.10)),
                ));
                if index % 2 == 1 {
                    den.spawn((
                        Name::new("Demon den violet fissure"),
                        Mesh3d(boulder.clone()),
                        MeshMaterial3d(glow.clone()),
                        Transform::from_translation(position + Vec3::new(-0.8, -0.25, 0.0))
                            .with_rotation(Quat::from_rotation_x(angle))
                            .with_scale(Vec3::new(0.12, 0.65, 0.14)),
                    ));
                }
            }
            for (z, y, length, angle) in [
                (-3.9, 2.8, 1.10, -0.35),
                (-2.9, 4.0, 1.35, -0.18),
                (-1.2, 4.8, 1.45, -0.1),
                (1.2, 4.8, 1.45, 0.1),
                (2.9, 4.0, 1.35, 0.18),
                (3.9, 2.8, 1.10, 0.35),
            ] {
                den.spawn((
                    Name::new("Demon den fang"),
                    Mesh3d(tooth.clone()),
                    MeshMaterial3d(ivory.clone()),
                    Transform::from_xyz(0.60, y, z)
                        .with_rotation(Quat::from_euler(
                            EulerRot::XYZ,
                            angle,
                            0.0,
                            std::f32::consts::PI,
                        ))
                        .with_scale(Vec3::new(1.0, length, 1.0)),
                ));
            }
            for index in 0..12 {
                let z = -4.9 + index as f32 * 0.89;
                let origin = Vec3::new(0.50, 0.34, z);
                let scale = Vec3::new(0.8, 0.38, 0.7);
                den.spawn((
                    Name::new("Demon den breathing mist"),
                    Mesh3d(mist_mesh.clone()),
                    MeshMaterial3d(fog.clone()),
                    Transform::from_translation(origin).with_scale(scale),
                    Mist {
                        origin,
                        phase: index as f32 * 0.67,
                        scale,
                    },
                ));
            }
            den.spawn((
                Name::new("Demon den interior purple light"),
                PointLight {
                    color: Color::srgb(0.55, 0.14, 1.0),
                    intensity: 60_000.0,
                    range: 14.0,
                    shadow_maps_enabled: false,
                    ..default()
                },
                Transform::from_xyz(2.5, 3.6, 0.0),
            ));
        });
}

fn emergence(
    mut commands: Commands,
    session: Res<GameSession>,
    art: Option<Res<DenArt>>,
    enemies: Query<&Transform, Added<Enemy>>,
    existing: Query<(), With<Emergence>>,
) {
    if session.paused || session.finished {
        return;
    }
    let Some(art) = art else {
        return;
    };
    let mut slots = MAX_EMERGENCE.saturating_sub(existing.iter().count());
    for enemy in &enemies {
        if enemy.translation.x < 10.0 || slots == 0 {
            continue;
        }
        slots -= 1;
        commands.spawn((
            Name::new("Demon den emergence pulse"),
            Mesh3d(art.puff.clone()),
            MeshMaterial3d(art.glow.clone()),
            Transform::from_xyz(
                16.2 - enemy.translation.z.clamp(-5.1, 5.1) * 0.55_f32.tan(),
                0.5,
                enemy.translation.z.clamp(-5.1, 5.1),
            )
            .with_scale(Vec3::new(0.5, 0.18, 0.5)),
            Emergence {
                remaining: 0.75,
                initial: 0.75,
            },
        ));
    }
}
fn animate(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    mut clock: Option<ResMut<DenClock>>,
    mut mist: Query<(&Mist, &mut Transform), Without<Emergence>>,
    mut pulses: Query<(Entity, &mut Emergence, &mut Transform), Without<Mist>>,
) {
    if session.paused || session.finished {
        return;
    }
    let Some(ref mut clock) = clock else {
        return;
    };
    clock.0 += time.delta_secs();
    for (mist, mut transform) in &mut mist {
        let phase = clock.0 * 1.4 + mist.phase;
        transform.translation =
            mist.origin + Vec3::new(-phase.sin() * 0.35, phase.cos() * 0.12, phase.sin() * 0.1);
        transform.scale = mist.scale * (1.0 + 0.16 * phase.sin());
    }
    for (entity, mut pulse, mut transform) in &mut pulses {
        pulse.remaining -= time.delta_secs();
        if pulse.remaining <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        let progress = 1.0 - pulse.remaining / pulse.initial;
        transform.scale = Vec3::new(
            0.5 + progress * 1.5,
            0.18 * (1.0 - progress),
            0.5 + progress * 1.5,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn enemy() -> Enemy {
        Enemy {
            hp: 100.0,
            max_hp: 100.0,
            speed: 1.0,
            damage: 1.0,
            xp: 1,
            wood: 0,
            gold: 1,
            kill_credit: 1,
            elite: false,
        }
    }
    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<GameSession>()
            .init_resource::<Time>()
            .init_resource::<DenClock>()
            .insert_resource(DenArt {
                puff: Handle::default(),
                glow: Handle::default(),
            })
            .add_systems(Update, (emergence, animate).chain());
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.1));
        app
    }
    #[test]
    fn true_spawn_feedback_is_bounded_and_pool_reuse_emits_again() {
        let mut app = app();
        let actor = app
            .world_mut()
            .spawn((enemy(), Transform::from_xyz(16.0, 0.0, 1.0)))
            .id();
        for _ in 0..40 {
            app.world_mut()
                .spawn((enemy(), Transform::from_xyz(16.0, 0.0, -2.0)));
        }
        app.world_mut()
            .spawn((enemy(), Transform::from_xyz(-8.0, 0.0, 0.0)));
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&Emergence>()
                .iter(app.world())
                .count(),
            MAX_EMERGENCE
        );
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&Emergence>()
                .iter(app.world())
                .count(),
            MAX_EMERGENCE
        );
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(1.0));
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&Emergence>()
                .iter(app.world())
                .count(),
            0
        );
        app.world_mut().entity_mut(actor).remove::<Enemy>();
        app.update();
        app.world_mut().entity_mut(actor).insert(enemy());
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.1));
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&Emergence>()
                .iter(app.world())
                .count(),
            1
        );
        assert_eq!(app.world().get::<Enemy>(actor).unwrap().hp, 100.0);
    }
    #[test]
    fn pause_freezes_mist_and_emergence_then_resumes_cleanup() {
        let mut app = app();
        let mist = app
            .world_mut()
            .spawn((
                Mist {
                    origin: Vec3::new(14.0, 0.3, 0.0),
                    phase: 0.5,
                    scale: Vec3::ONE,
                },
                Transform::default(),
            ))
            .id();
        app.world_mut()
            .spawn((enemy(), Transform::from_xyz(16.0, 0.0, 0.0)));
        app.update();
        let before = *app.world().get::<Transform>(mist).unwrap();
        let phase = app.world().resource::<DenClock>().0;
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(10));
        app.update();
        assert_eq!(app.world().resource::<DenClock>().0, phase);
        assert_eq!(
            app.world().get::<Transform>(mist).unwrap().translation,
            before.translation
        );
        assert_eq!(
            app.world_mut()
                .query::<&Emergence>()
                .iter(app.world())
                .count(),
            1
        );
        app.world_mut().resource_mut::<GameSession>().paused = false;
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&Emergence>()
                .iter(app.world())
                .count(),
            0
        );
    }
}
