//! Real damage/proc verification for completed cultivation recipes.
use super::CaptureControls;
use bevy::prelude::*;
use rogue_commander::game::{
    ActiveSkill, AttackClock, Autoplay, BondId, Enemy, EnemyArchetype, EnemyHeroThreat, GameAssets,
    GameSession, Hero, HeroKind, HeroVitals, PlayerBuild, Projectile, RunRng, SelectedHero,
    SpawnDirector,
};
#[derive(Component)]
struct Target;
#[derive(Resource, Default)]
struct State {
    frame: u32,
    saw_cauldron: bool,
    initial_health: f32,
    initial_enemy_health: f32,
    scheduled: u32,
    native_projectiles: u32,
}
pub fn configure(world: &mut World) {
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SelectedHero>().kind = HeroKind::EmberRanger;
    world.resource_mut::<SelectedHero>().locked = true;
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<CaptureControls>().suppress_drafts = true;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
    world.insert_resource(RunRng::from_seed(0xC0171A710));
    let mut build = world.resource_mut::<PlayerBuild>();
    build.skills.fill(0);
    for family in [BondId::Journey, BondId::Ascension, BondId::Cultivation] {
        for card in family.definition().members {
            build.owned_bond_cards[card.index()] = true;
        }
    }
}
pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    world.remove_resource::<Autoplay>();
    world.resource_mut::<ActiveSkill>().remaining = 10000.0;
    world.resource_mut::<PlayerBuild>().attacks_per_second = 4.0;
    world.resource_mut::<AttackClock>().0.reset();
    for mut t in world
        .query_filtered::<&mut Transform, With<Hero>>()
        .iter_mut(world)
    {
        t.translation = Vec3::new(-5.0, 0.0, 0.0);
    }
    {
        let mut vitals = world.resource_mut::<HeroVitals>();
        vitals.max_health = 10000.0;
        vitals.health = 10000.0;
    }
    let scene = world.resource::<GameAssets>().enemy_scenes[3].clone();
    world.spawn((
        Name::new("Cultivation capture attacker"),
        Target,
        EnemyArchetype(3),
        EnemyHeroThreat::ranged(),
        WorldAssetRoot(scene),
        Transform::from_xyz(-2.0, 0.0, 0.0),
        Enemy {
            hp: 100000.0,
            max_hp: 100000.0,
            speed: 0.0,
            damage: 20.0,
            xp: 0,
            wood: 0,
            gold: 0,
            kill_credit: 0,
            elite: false,
        },
    ));
    for (mut t, mut projection) in world
        .query_filtered::<(&mut Transform, &mut Projection), With<Camera3d>>()
        .iter_mut(world)
    {
        *t = Transform::from_xyz(-4.0, 12.0, 10.0).looking_at(Vec3::new(-4.0, 0.0, 0.0), Vec3::Y);
        if let Projection::Orthographic(p) = &mut *projection {
            p.scaling_mode = bevy::camera::ScalingMode::FixedVertical {
                viewport_height: 10.0,
            };
        }
    }
    world.insert_resource(State::default());
    app.add_systems(PreUpdate, drive)
        .add_systems(PostUpdate, verify);
}
fn drive(mut state: ResMut<State>, mut threats: Query<&mut EnemyHeroThreat, With<Target>>) {
    state.frame += 1;
    // Accelerate only the fixture's threat clock; production move_enemies still
    // decides range, applies actual damage and emits the real attack feedback.
    if state.frame.is_multiple_of(6) {
        for mut threat in &mut threats {
            threat.attack_remaining = 0.0;
        }
    }
}
fn verify(
    mut state: ResMut<State>,
    vitals: Res<HeroVitals>,
    clock: Res<AttackClock>,
    names: Query<&Name>,
    enemies: Query<&Enemy, With<Target>>,
    projectiles: Query<&Projectile, Added<Projectile>>,
) {
    state.saw_cauldron |= names
        .iter()
        .any(|n| n.as_str() == "Cultivation cauldron suppression");
    state.scheduled += clock.0.times_finished_this_tick();
    state.native_projectiles += projectiles
        .iter()
        .filter(|p| p.velocity.length_squared() > 1.0)
        .count() as u32;
    if state.frame == 5 {
        state.initial_health = vitals.health;
        state.initial_enemy_health = enemies.single().unwrap().hp;
    }
    if state.frame == 295 {
        assert!(
            vitals.alive && vitals.health < state.initial_health,
            "real enemy attacks must damage the surviving hero"
        );
        assert!(
            enemies.single().unwrap().hp < state.initial_enemy_health,
            "real player attacks must damage the durable target"
        );
        assert!(
            state.saw_cauldron,
            "received hits must proc the visible cultivation cauldron"
        );
        assert!(
            state.native_projectiles > state.scheduled,
            "bonus weapon volleys must exceed scheduled clock volleys: native={} scheduled={}",
            state.native_projectiles,
            state.scheduled
        );
        println!(
            "[capture] PASS cultivation-combat: hero {:.1}->{:.1}, enemy {:.1}->{:.1}, cauldron observed, native attacks {} > scheduled {}",
            state.initial_health,
            vitals.health,
            state.initial_enemy_health,
            enemies.single().unwrap().hp,
            state.native_projectiles,
            state.scheduled
        );
    }
}
