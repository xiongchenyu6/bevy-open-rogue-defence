//! Close-up of the real passive and ultimate spirit companions.
use bevy::prelude::*;
use rogue_commander::game::{
    ActiveSkill, AttackClock, Autoplay, Enemy, GameAssets, GameSession, Hero, HeroKind, HeroVitals,
    PlayerBuild, SelectedHero, SpawnDirector,
};

#[derive(Resource, Default)]
struct Gallery(u32);
#[derive(Component)]
struct Target;

pub fn configure(world: &mut World) {
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SelectedHero>().kind = HeroKind::SpiritKeeper;
    world.resource_mut::<SelectedHero>().locked = true;
    *world.resource_mut::<HeroVitals>() = HeroVitals::for_hero(HeroKind::SpiritKeeper, 1);
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
    world
        .resource_mut::<super::CaptureControls>()
        .suppress_drafts = true;
}
pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    world.resource_mut::<PlayerBuild>().skills.fill(0);
    world.resource_mut::<AttackClock>().0.pause();
    world.resource_mut::<ActiveSkill>().remaining = 0.0;
    for mut hero in world
        .query_filtered::<&mut Transform, With<Hero>>()
        .iter_mut(world)
    {
        hero.translation = Vec3::new(-7.0, 0.0, 0.0);
    }
    app.init_resource::<Gallery>()
        .add_systems(PreUpdate, exercise)
        .add_systems(PostUpdate, camera.before(TransformSystems::Propagate));
}
fn camera(mut cameras: Query<(&mut Transform, &mut Projection), With<Camera3d>>) {
    for (mut transform, mut projection) in &mut cameras {
        *transform = Transform::from_xyz(5., 9., 10.).looking_at(Vec3::new(-2.0, 0.8, 0.), Vec3::Y);
        if let Projection::Orthographic(p) = &mut *projection {
            p.scaling_mode = bevy::camera::ScalingMode::FixedVertical {
                viewport_height: 8.5,
            };
        }
    }
}
fn exercise(
    mut commands: Commands,
    mut state: ResMut<Gallery>,
    assets: Res<GameAssets>,
    mut targets: Query<(Entity, &mut Enemy), With<Target>>,
    spirits: Query<(
        &Name,
        &Mesh3d,
        &MeshMaterial3d<StandardMaterial>,
        &Transform,
    )>,
) {
    state.0 += 1;
    if state.0 == 1 {
        commands.spawn((
            Target,
            WorldAssetRoot(assets.enemy_scenes[0].clone()),
            Transform::from_xyz(1.0, 0., 2.0),
            Enemy {
                hp: 1_000_000.,
                max_hp: 1_000_000.,
                speed: 0.,
                damage: 0.,
                xp: 0,
                wood: 0,
                gold: 0,
                kill_credit: 0,
                elite: false,
            },
        ));
    }
    if state.0 == 100 {
        let group: Vec<_> = spirits
            .iter()
            .filter(|(name, _, _, _)| name.as_str() == "Spirit companion")
            .collect();
        assert!(
            group.len() >= 6,
            "passive and ultimate must summon real companions"
        );
        for (_, mesh, material, transform) in group {
            assert_eq!(mesh.0, assets.spirit_mesh);
            assert_ne!(mesh.0, assets.projectile_mesh);
            assert_eq!(material.0, assets.spirit_material);
            assert_eq!(transform.scale, Vec3::splat(0.95));
        }
        assert!(targets.iter().any(|(_, enemy)| enemy.hp < enemy.max_hp));
        println!(
            "[capture] PASS: six or more authored spirits from real passive/ultimate; actual damage"
        );
    }
    if state.0 == 140 {
        let positions: Vec<_> = spirits
            .iter()
            .filter(|(name, _, _, _)| name.as_str() == "Spirit companion")
            .map(|(_, _, _, transform)| transform.translation)
            .collect();
        println!("[capture] spirit combat positions: {positions:?}");
        for (index, left) in positions.iter().enumerate() {
            for right in &positions[index + 1..] {
                assert!(
                    (*left - *right).with_y(0.0).length() > 0.6,
                    "spirit silhouettes overlap: {left:?} vs {right:?}"
                );
            }
        }
    }
    if state.0 == 150 {
        for (entity, _) in &mut targets {
            commands.entity(entity).despawn();
        }
    }
}
