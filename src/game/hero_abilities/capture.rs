//! Debug-only stimuli and observation around the real projectile passive system.
//! No direct passive calls, output spawning or damage changes; unrelated attacks are disabled.
use super::*;
use crate::game::{AttackClock, Autoplay, SpawnDirector};
use std::time::Duration;

#[derive(Component)]
struct InputSpell;

#[derive(Component)]
struct PassiveTarget;

#[derive(Resource)]
struct Probe {
    frame: u32,
    kind: HeroKind,
    batch: bool,
    threshold: usize,
    outputs: usize,
}

fn options(scenario: &str) -> (HeroKind, bool, usize) {
    match scenario {
        "passive-storm-batch" => (HeroKind::StormSavant, true, 6),
        "passive-storm-split" => (HeroKind::StormSavant, false, 6),
        "passive-frost-batch" => (HeroKind::FrostOracle, true, 5),
        "passive-frost-split" => (HeroKind::FrostOracle, false, 5),
        _ => panic!("unknown passive batch scene {scenario}"),
    }
}

pub fn configure(world: &mut World, scenario: &str) {
    let (kind, _, _) = options(scenario);
    world.remove_resource::<Autoplay>();
    *world.resource_mut::<SelectedHero>() = SelectedHero { kind, locked: true };
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
}

pub fn install(app: &mut bevy::app::SubApp, scenario: &str) {
    let (kind, batch, threshold) = options(scenario);
    let world = app.world_mut();
    world.remove_resource::<Autoplay>();
    world.resource_mut::<PlayerBuild>().skills.fill(0);
    world.resource_mut::<ActiveSkill>().remaining = 10_000.0;
    {
        let mut clock = world.resource_mut::<AttackClock>();
        clock.0.reset();
        clock.0.pause();
    }
    world
        .resource_mut::<Time<Virtual>>()
        .set_relative_speed(1.0);
    world.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_secs_f64(if batch { 0.1 } else { 1.0 / 60.0 }),
    ));
    for mut transform in world
        .query_filtered::<&mut Transform, With<Hero>>()
        .iter_mut(world)
    {
        *transform = Transform::from_xyz(-3.0, 0.0, 0.0).looking_to(Vec3::X, Vec3::Y);
    }
    let scene = world.resource::<GameAssets>().enemy_scenes[0].clone();
    world.spawn((
        Name::new("Passive batch target"),
        PassiveTarget,
        WorldAssetRoot(scene),
        Transform::from_xyz(3.0, 0.0, 0.0),
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
    let center = Vec3::new(0.0, 0.5, 0.0);
    for (mut transform, mut projection) in world
        .query_filtered::<(&mut Transform, &mut Projection), With<Camera3d>>()
        .iter_mut(world)
    {
        *transform = Transform::from_translation(center + Vec3::new(0.0, 9.0, 8.0))
            .looking_at(center, Vec3::Y);
        if let Projection::Orthographic(projection) = &mut *projection {
            projection.scaling_mode = bevy::camera::ScalingMode::FixedVertical {
                viewport_height: 8.0,
            };
        }
    }
    world.insert_resource(Probe {
        frame: 0,
        kind,
        batch,
        threshold,
        outputs: 0,
    });
    app.add_systems(PreUpdate, feed).add_systems(Last, observe);
    println!(
        "PASSIVE_BATCH_FIXTURE scenario={scenario} step_hz={} speed=1 synthetic_spell_inputs=true production_observer_and_pools=true",
        if batch { 10 } else { 60 }
    );
}

fn feed(world: &mut World) {
    world.resource_mut::<Probe>().frame += 1;
    let probe = world.resource::<Probe>();
    let count = match probe.frame {
        10 => probe.threshold * if probe.batch { 2 } else { 1 },
        12 if !probe.batch => probe.threshold,
        24 => probe.threshold - 1,
        26 => 1,
        _ => 0,
    };
    for _ in 0..count {
        // Input particles are invisible and far from enemies so the real combat
        // collision pass cannot retire them before the real PostUpdate observer.
        world.spawn((
            InputSpell,
            Projectile {
                velocity: Vec3::ZERO,
                damage: 1.0,
                radius: 0.1,
                remaining_hits: 0,
                lifetime: 100.0,
            },
            DamageProfile::new(DamageElement::Arcane, DamageClass::Spell),
            Transform::from_xyz(1_000.0, 0.0, 1_000.0),
        ));
    }
}

fn observe(world: &mut World) {
    let outputs: Vec<_> = world
        .query_filtered::<(Entity, &Projectile, &DamageProfile), (Added<Projectile>, With<AbilityGenerated>)>()
        .iter(world)
        .map(|(entity, projectile, profile)| (entity, projectile.damage, profile.element))
        .collect();
    let probe = &mut *world.resource_mut::<Probe>();
    for (entity, damage, element) in outputs {
        // Pooled entities may legitimately be reused by the later third group.
        probe.outputs += 1;
        println!(
            "PASSIVE_BATCH_OUTPUT frame={} kind={:?} entity={entity:?} damage={damage:.6} element={element:?} cumulative={}",
            probe.frame, probe.kind, probe.outputs
        );
    }
    let frame = probe.frame;
    let count = probe.outputs;
    if matches!(frame, 20 | 25) {
        assert_eq!(
            count, 2,
            "two full groups emit exactly two production outputs; generated outputs cannot fund the later partial group"
        );
        println!(
            "PASSIVE_BATCH_CHECK frame={frame} outputs=2 full_groups_and_recursion_exclusion=PASS"
        );
    }
    if frame == 35 || frame == 45 {
        assert_eq!(
            count, 3,
            "one final external signal completes exactly one additional group"
        );
        let hp = world
            .query_filtered::<&Enemy, With<PassiveTarget>>()
            .single(world)
            .unwrap()
            .hp;
        assert!(
            hp < 100_000.0,
            "actual passive outputs must damage the real target"
        );
        assert_eq!(world.resource::<BasicAttackCounter>().count(), 0);
        assert!(
            world
                .resource::<PlayerBuild>()
                .skills
                .iter()
                .all(|rank| *rank == 0)
        );
        assert!(world.resource::<ActiveSkill>().remaining > 9_000.0);
        assert!(world.get_resource::<Autoplay>().is_none());
        println!(
            "PASSIVE_BATCH_CHECK frame={frame} outputs=3 exact_remainder_idle_stability=PASS target_hp={hp:.6} unrelated_attacks_suppressed=true"
        );
    }
}
