//! Real rank progression with basic attacks disabled: damage must come from orbit blades.
use super::CaptureControls;
use bevy::prelude::*;
use rogue_commander::game::{
    ActiveSkill, AttackClock, Autoplay, DraftRoot, Enemy, GameAssets, GameSession, Hero, HeroKind,
    PlayerBuild, SelectedHero, SpawnDirector,
};
#[derive(Resource, Default)]
struct OrbitCapture {
    frame: u32,
    saw_wake: bool,
    saw_damage: bool,
    original: Vec<Entity>,
}
#[derive(Component)]
struct OrbitTarget;
const CENTER: Vec3 = Vec3::new(-4.0, 0.0, 0.0);
pub fn configure(world: &mut World) {
    world.resource_mut::<SelectedHero>().kind = HeroKind::StoneWarden;
    world.resource_mut::<SelectedHero>().locked = true;
    world.resource_mut::<CaptureControls>().suppress_drafts = true;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
    world.resource_mut::<SpawnDirector>().enabled = false;
}
pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    world.remove_resource::<Autoplay>();
    world.resource_mut::<PlayerBuild>().skills.fill(0);
    world.resource_mut::<PlayerBuild>().skills[6] = 1;
    world.resource_mut::<ActiveSkill>().remaining = 10_000.0;
    world.resource_mut::<AttackClock>().0.reset();
    world.resource_mut::<AttackClock>().0.pause();
    for mut hero in world
        .query_filtered::<&mut Transform, With<Hero>>()
        .iter_mut(world)
    {
        hero.translation = CENTER;
    }
    for (mut camera, mut projection) in world
        .query_filtered::<(&mut Transform, &mut Projection), With<Camera3d>>()
        .iter_mut(world)
    {
        *camera = Transform::from_translation(CENTER + Vec3::new(0.0, 10.0, 8.5))
            .looking_at(CENTER, Vec3::Y);
        if let Projection::Orthographic(projection) = &mut *projection {
            projection.scaling_mode = bevy::camera::ScalingMode::FixedVertical {
                viewport_height: 9.0,
            };
        }
    }
    let scene = world.resource::<GameAssets>().enemy_scenes[0].clone();
    for i in 0..4 {
        let angle = i as f32 * std::f32::consts::FRAC_PI_2;
        world.spawn((
            Name::new("Orbit blade damage target"),
            OrbitTarget,
            WorldAssetRoot(scene.clone()),
            Transform::from_translation(
                CENTER + Vec3::new(angle.cos() * 2.05, 0.0, angle.sin() * 2.05),
            )
            .with_scale(Vec3::splat(0.7)),
            Enemy {
                hp: 100_000.0,
                max_hp: 100_000.0,
                speed: 0.0,
                damage: 0.0,
                xp: 0,
                wood: 0,
                gold: 0,
                kill_credit: 0,
                elite: false,
            },
        ));
    }
    app.init_resource::<OrbitCapture>()
        .add_systems(PreUpdate, upgrade)
        .add_systems(Last, verify);
    println!(
        "[capture] orbit-blade real passive; all other skills, basic attacks and ultimate suppressed"
    );
}
fn upgrade(mut capture: ResMut<OrbitCapture>, mut build: ResMut<PlayerBuild>) {
    capture.frame += 1;
    match capture.frame {
        100 => {
            build.skills[6] = 3;
            println!("[capture] orbit blade 1 -> 3");
        }
        200 => {
            build.skills[6] = 5;
            println!("[capture] orbit blade 3 -> 5");
        }
        _ => {}
    }
}
#[allow(clippy::too_many_arguments)]
fn verify(
    mut capture: ResMut<OrbitCapture>,
    build: Res<PlayerBuild>,
    assets: Res<GameAssets>,
    active: Res<ActiveSkill>,
    clock: Res<AttackClock>,
    models: Query<(Entity, &Name, &Mesh3d, &Transform)>,
    names: Query<&Name>,
    targets: Query<&Enemy, With<OrbitTarget>>,
    drafts: Query<(), With<DraftRoot>>,
) {
    capture.saw_wake |= names.iter().any(|name| name.as_str() == "Orbit blade wake");
    capture.saw_damage |= targets.iter().any(|enemy| enemy.hp < enemy.max_hp);
    if matches!(capture.frame, 80 | 180 | 280) {
        let rank = build.skills[6];
        let expected = 2 + usize::from(rank.saturating_sub(1)) / 2;
        let blades: Vec<_> = models
            .iter()
            .filter(|(_, name, _, _)| name.as_str() == "Orbit blade")
            .collect();
        assert_eq!(blades.len(), expected);
        assert_eq!(
            models
                .iter()
                .filter(|(_, name, _, _)| name.as_str() == "Orbit blade luminous edge")
                .count(),
            expected
        );
        for (_, _, mesh, transform) in &blades {
            assert_ne!(
                mesh.0, assets.projectile_mesh,
                "orbit blade must not reuse dot mesh"
            );
            assert!((transform.scale.x - (1.0 + f32::from(rank - 1) * 0.13)).abs() < 0.001);
            assert!(((transform.translation - CENTER).with_y(0.0).length() - 2.05).abs() < 0.001);
        }
        if capture.frame == 80 {
            capture.original = blades.iter().map(|(entity, ..)| *entity).collect();
        } else {
            assert!(
                capture
                    .original
                    .iter()
                    .all(|id| blades.iter().any(|(entity, ..)| entity == id)),
                "upgrades should retain existing blades"
            );
        }
        assert!(clock.0.is_paused());
        assert!(active.remaining > 9000.0);
        assert_eq!(drafts.iter().count(), 0);
        println!(
            "[capture] PASS orbit blade rank {rank}: {expected} authored metal/edge pairs, radius 2.05, scale {:.2}",
            1.0 + f32::from(rank - 1) * 0.13
        );
    }
    if capture.frame == 295 {
        assert!(
            capture.saw_wake,
            "orbit movement must leave segmented blade silhouettes"
        );
        assert!(
            capture.saw_damage,
            "passive alone must damage targets on its orbit"
        );
        assert!(
            build
                .skills
                .iter()
                .enumerate()
                .all(|(index, rank)| index == 6 || *rank == 0)
        );
        println!("[capture] PASS orbit-blade real damage, segmented trails, only skill 6 enabled");
    }
}
