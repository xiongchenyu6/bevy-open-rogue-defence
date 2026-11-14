//! Cosmetic receipts for already-settled rewards. These never grant currency.
use super::{GameSession, Hero, combat::CombatSet};
use bevy::{light::NotShadowCaster, prelude::*};

pub(crate) struct RewardFeedbackPlugin;
impl Plugin for RewardFeedbackPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, (emit, animate).chain().after(CombatSet));
    }
}

#[derive(Component)]
pub(crate) struct RewardReceipt {
    pub position: Vec3,
    pub gold: u32,
    pub xp: u32,
    pub important: bool,
}
#[derive(Component)]
struct RewardFlight {
    origin: Vec3,
    age: f32,
    delay: f32,
    size: f32,
    side: f32,
}
#[derive(Resource)]
struct RewardArt {
    coin: Handle<Mesh>,
    rim: Handle<Mesh>,
    rim_material: Handle<StandardMaterial>,
    mote: Handle<Mesh>,
    gold: Handle<StandardMaterial>,
    xp: Handle<StandardMaterial>,
}
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(RewardArt {
        coin: meshes.add(Cylinder::new(0.28, 0.07)),
        rim: meshes.add(Torus::new(0.21, 0.25)),
        rim_material: materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.9, 0.47),
            unlit: true,
            ..default()
        }),
        mote: meshes.add(Sphere::new(0.28).mesh().uv(4, 2)),
        gold: materials.add(StandardMaterial {
            base_color: Color::srgb(0.95, 0.64, 0.18),
            emissive: LinearRgba::new(0.7, 0.38, 0.04, 1.0),
            unlit: true,
            metallic: 0.55,
            perceptual_roughness: 0.48,
            ..default()
        }),
        xp: materials.add(StandardMaterial {
            base_color: Color::srgb(0.35, 0.83, 0.96),
            emissive: LinearRgba::new(0.12, 0.5, 0.7, 1.0),
            unlit: true,
            perceptual_roughness: 0.7,
            ..default()
        }),
    });
}

#[cfg(any(debug_assertions, feature = "dev-tools"))]
pub fn capture_targets(world: &mut World) {
    use super::{CombatPools, Enemy, GameAssets, pooling::EnemySceneKey};
    world
        .run_system_cached(
            |mut commands: Commands, mut pools: ResMut<CombatPools>, assets: Res<GameAssets>| {
                for i in 0..6 {
                    let transform =
                        Transform::from_xyz(0.2 + (i / 3) as f32 * 1.4, 0.0, (i % 3) as f32 - 1.0)
                            .with_scale(Vec3::splat(0.72));
                    let entity = pools
                        .rent_enemy_scene(
                            &mut commands,
                            EnemySceneKey::Regular(0),
                            assets.enemy_scenes[0].clone(),
                            transform,
                        )
                        .expect("six production enemy slots");
                    commands.entity(entity).insert((
                        Name::new("Reward combat target"),
                        Enemy {
                            hp: 1.0,
                            max_hp: 1.0,
                            speed: 0.0,
                            damage: 0.0,
                            xp: 2,
                            wood: 0,
                            gold: 3,
                            kill_credit: 1,
                            elite: false,
                        },
                    ));
                }
            },
        )
        .unwrap();
}
fn emit(
    mut commands: Commands,
    art: Res<RewardArt>,
    session: Res<GameSession>,
    receipts: Query<(Entity, &RewardReceipt)>,
    flights: Query<(), With<RewardFlight>>,
) {
    let mut remaining = 72usize.saturating_sub(flights.iter().count());
    for (entity, receipt) in &receipts {
        commands.entity(entity).despawn();
        if session.finished || session.paused {
            continue;
        }
        // A handful of tokens communicates collection; never one token per gold.
        let categories = [
            (receipt.gold, &art.coin, &art.gold),
            (receipt.xp, &art.mote, &art.xp),
        ];
        for (category, (amount, mesh, material)) in categories.into_iter().enumerate() {
            let count = if amount == 0 {
                0
            } else if receipt.important {
                3
            } else {
                1
            };
            for i in 0..count.min(remaining) {
                remaining -= 1;
                let side = if category == 0 { -1.0 } else { 1.0 };
                let size = if receipt.important { 1.5 } else { 1.0 };
                let mut token = commands.spawn((
                    Name::new(if category == 0 {
                        "Collected gold"
                    } else {
                        "Collected experience"
                    }),
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    NotShadowCaster,
                    Transform::from_translation(receipt.position + Vec3::Y * 0.75)
                        .with_scale(Vec3::splat(size)),
                    RewardFlight {
                        origin: receipt.position + Vec3::Y * 0.75,
                        age: 0.0,
                        delay: i as f32 * 0.07,
                        size,
                        side,
                    },
                ));
                if category == 0 {
                    token.with_children(|parent| {
                        for y in [-0.04, 0.04] {
                            parent.spawn((
                                Mesh3d(art.rim.clone()),
                                MeshMaterial3d(art.rim_material.clone()),
                                Transform::from_xyz(0.0, y, 0.0)
                                    .with_scale(Vec3::new(1.0, 0.25, 1.0)),
                                NotShadowCaster,
                            ));
                        }
                    });
                }
            }
        }
    }
}
fn flight_position(origin: Vec3, target: Vec3, age: f32, side: f32) -> Vec3 {
    if age < 0.22 {
        let t = (age / 0.22).clamp(0.0, 1.0);
        return origin
            + Vec3::new(
                side * 0.32 * t,
                0.65 * (t * std::f32::consts::FRAC_PI_2).sin(),
                0.0,
            );
    }
    let t = ((age - 0.22) / 0.55).clamp(0.0, 1.0);
    let start = origin + Vec3::new(side * 0.32, 0.65, 0.0);
    start.lerp(target, t * t) + Vec3::Y * ((t * std::f32::consts::PI).sin() * 0.35)
}
fn animate(
    mut commands: Commands,
    time: Res<Time<Real>>,
    session: Res<GameSession>,
    hero: Query<&Transform, (With<Hero>, Without<RewardFlight>)>,
    mut flights: Query<(Entity, &mut Transform, &mut RewardFlight)>,
) {
    let target = hero.iter().next().map(|t| t.translation + Vec3::Y * 0.7);
    for (entity, mut transform, mut flight) in &mut flights {
        if session.finished || target.is_none() {
            commands.entity(entity).despawn();
            continue;
        }
        if session.paused {
            continue;
        }
        flight.age += time.delta_secs().min(0.05);
        let age = (flight.age - flight.delay).max(0.0);
        if age >= 0.77 {
            commands.entity(entity).despawn();
            continue;
        }
        transform.translation = flight_position(flight.origin, target.unwrap(), age, flight.side);
        transform.rotation = Quat::from_rotation_z(age * 7.0) * Quat::from_rotation_x(1.1);
        transform.scale = Vec3::splat(flight.size * (1.0 - ((age - 0.62) / 0.15).clamp(0.0, 1.0)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn collection_is_bounded_cosmetic_and_cleans_up() {
        let mut app = App::new();
        app.init_resource::<GameSession>()
            .init_resource::<Time<Real>>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_systems(Update, (emit, animate).chain());
        app.world_mut().run_system_cached(setup).unwrap();
        app.world_mut().spawn((Hero, Transform::default()));
        let before = app.world().resource::<GameSession>().gold;
        for _ in 0..100 {
            app.world_mut().spawn(RewardReceipt {
                position: Vec3::X,
                gold: 100,
                xp: 10,
                important: true,
            });
        }
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&RewardFlight>()
                .iter(app.world())
                .count(),
            72
        );
        assert_eq!(app.world().resource::<GameSession>().gold, before);
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(std::time::Duration::from_secs(1));
        app.update();
        assert!(
            app.world_mut()
                .query::<&RewardFlight>()
                .iter(app.world())
                .all(|f| f.age == 0.0)
        );
        app.world_mut().resource_mut::<GameSession>().paused = false;
        for _ in 0..22 {
            app.update();
        }
        assert_eq!(
            app.world_mut()
                .query::<&RewardFlight>()
                .iter(app.world())
                .count(),
            0
        );
        assert_eq!(app.world().resource::<GameSession>().gold, before);
    }
    #[test]
    fn reward_flight_bursts_then_converges_without_overshoot() {
        let origin = Vec3::ZERO;
        let target = Vec3::new(-8.0, 0.7, 0.0);
        assert_eq!(flight_position(origin, target, 0.0, 1.0), origin);
        assert!(flight_position(origin, target, 0.22, 1.0).y > 0.6);
        assert!(flight_position(origin, target, 0.77, 1.0).distance(target) < 0.0001);
    }
}
