//! A single cavern spans every invasion lane. Its art never owns collision or spawning.
use super::combat::CombatSet;
use super::{ArenaEntity, Enemy, GameSession};
use bevy::prelude::*;

pub(super) struct DemonDenPlugin;
impl Plugin for DemonDenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, tone_cavern_rock)
            .add_systems(Update, (emergence, animate).chain().after(CombatSet));
    }
}
#[derive(Resource)]
struct DenArt {
    puff: Handle<Mesh>,
    glow: Handle<StandardMaterial>,
}
#[derive(Component)]
struct Emergence {
    remaining: f32,
    initial: f32,
}
const MAX_EMERGENCE: usize = 32;
const CAVERN_MODEL: &str = "models/environment_demon_cavern.glb";
/// The baked stone is a warm tan that reads as a bright slab under the sun and
/// washes out the HUD drawn over it. A cool, darker multiply keeps the texture
/// but lets the cavern recede behind the lane.
const CAVERN_ROCK_TINT: Color = Color::srgb(0.46, 0.45, 0.56);

fn tone_cavern_rock(
    server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut toned: Local<bool>,
) {
    if *toned {
        return;
    }
    // The subasset path can register after the material itself; retry until seen.
    let rock: Vec<_> = materials
        .iter()
        .filter_map(|(id, _)| {
            (server.get_path(id)?.path().to_string_lossy() == CAVERN_MODEL).then_some(id)
        })
        .collect();
    for id in rock {
        if let Some(mut material) = materials.get_mut(id) {
            material.base_color = CAVERN_ROCK_TINT;
            *toned = true;
        }
    }
}

fn setup(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let puff = meshes.add(Sphere::new(1.0).mesh().ico(2).unwrap());
    let glow = materials.add(StandardMaterial {
        base_color: Color::srgba(0.50, 0.06, 0.82, 0.30),
        emissive: LinearRgba::new(1.15, 0.025, 2.6, 1.0),
        alpha_mode: AlphaMode::Add,
        unlit: true,
        ..default()
    });
    commands.insert_resource(DenArt { puff, glow });
    commands.spawn((
        Name::new("Demon den"),
        WorldAssetRoot(server.load("models/environment_demon_cavern.glb#Scene0")),
        Transform::from_xyz(15.65, 0.0, 0.0).with_rotation(Quat::from_rotation_y(-0.55)),
        Visibility::default(),
        ArenaEntity,
    ));
    // A violet glow in the cavern mouth marks where the invasion emerges and
    // separates the dark rock from the lane's far end.
    commands.spawn((
        Name::new("Demon den mouth glow"),
        PointLight {
            color: Color::srgb(0.62, 0.25, 1.0),
            intensity: 900_000.0,
            range: 11.0,
            radius: 0.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(16.0, 2.2, 0.0),
        ArenaEntity,
    ));
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
            Transform::from_translation(enemy.translation.with_y(0.5))
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
    mut pulses: Query<(Entity, &mut Emergence, &mut Transform)>,
) {
    if session.finished {
        for (entity, _, _) in &mut pulses {
            commands.entity(entity).despawn();
        }
        return;
    }
    if session.paused {
        return;
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
    #[test]
    fn cavern_rock_is_toned_once_and_other_materials_are_left_alone() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<StandardMaterial>()
            .add_systems(Update, tone_cavern_rock);
        let rock: Handle<StandardMaterial> = app
            .world()
            .resource::<AssetServer>()
            .load(format!("{CAVERN_MODEL}#Material0"));
        let other = {
            let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
            materials
                .insert(rock.id(), StandardMaterial::default())
                .unwrap();
            materials.add(StandardMaterial::default())
        };
        app.update();
        let materials = app.world().resource::<Assets<StandardMaterial>>();
        assert_eq!(materials.get(&rock).unwrap().base_color, CAVERN_ROCK_TINT);
        assert_eq!(materials.get(&other).unwrap().base_color, Color::WHITE);
    }

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<GameSession>()
            .init_resource::<Time>()
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
    fn pause_freezes_spawn_pulse_at_actual_birth_then_resumes_cleanup() {
        let mut app = app();
        let birth = Vec3::new(16.0, 0.0, 4.0);
        app.world_mut()
            .spawn((enemy(), Transform::from_translation(birth)));
        app.update();
        let (pulse, before, remaining) = app
            .world_mut()
            .query::<(Entity, &Transform, &Emergence)>()
            .single(app.world())
            .map(|(e, t, p)| (e, *t, p.remaining))
            .unwrap();
        assert_eq!(before.translation, birth.with_y(0.5));
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(10));
        app.update();
        assert_eq!(*app.world().get::<Transform>(pulse).unwrap(), before);
        assert_eq!(
            app.world().get::<Emergence>(pulse).unwrap().remaining,
            remaining
        );
        app.world_mut().resource_mut::<GameSession>().paused = false;
        app.update();
        assert!(app.world().get_entity(pulse).is_err());
    }

    #[test]
    fn finished_battle_retires_emergence_without_creating_more() {
        let mut app = app();
        app.world_mut()
            .spawn((enemy(), Transform::from_xyz(16.0, 0.0, -3.0)));
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&Emergence>()
                .iter(app.world())
                .count(),
            1
        );
        app.world_mut().resource_mut::<GameSession>().finished = true;
        app.world_mut()
            .spawn((enemy(), Transform::from_xyz(16.0, 0.0, 2.0)));
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
