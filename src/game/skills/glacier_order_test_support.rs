//! Controlled real casting/pulse bridge for Glacier ordering regressions.
use super::*;

pub(crate) fn test_glacier_order_pulse(
    world: &mut World,
    primary_entity: Entity,
    cast: bool,
) -> (usize, f32, f32, u64, f32) {
    use bevy::ecs::system::RunSystemOnce;
    if cast {
        world
            .run_system_once(
                move |mut commands: Commands,
                      assets: Res<GameAssets>,
                      mut pools: ResMut<CombatPools>,
                      build: Res<PlayerBuild>,
                      vitals: Res<HeroVitals>,
                      mut runtime: ResMut<SkillRuntime>,
                      enemies: Query<(Entity, &Transform, &Enemy)>| {
                    let targets: Vec<_> = enemies
                        .iter()
                        .map(|(entity, transform, enemy)| {
                            target_info(entity, transform, enemy, None, None, None)
                        })
                        .collect();
                    let primary = *targets
                        .iter()
                        .find(|target| target.entity == primary_entity)
                        .unwrap();
                    // Controlled fixture: a learned skill with elapsed initial cooldown and
                    // real resolved-basic-attack charge, before calling the actual caster.
                    runtime.prepare_cadence(&build.skills);
                    runtime.cooldowns[14] = 0.0;
                    let charge = attack_charge_requirement(14, build.skills[14]);
                    apply_resolved_attack_volleys(&mut runtime, charge as u32, |_| {});
                    assert!(runtime.cast_ready(14));
                    let mut budget = SummonBudget::from_existing(0, 0);
                    cast_rare_skills(
                        &mut commands,
                        &assets,
                        &mut pools,
                        &build,
                        &vitals,
                        Vec3::ZERO,
                        primary,
                        &targets,
                        &mut budget,
                        &mut runtime,
                    );
                },
            )
            .unwrap();
    }
    assert_eq!(world.query::<&SkillZone>().iter(world).count(), 1);
    let zone = world.query::<&SkillZone>().single(world).unwrap();
    assert_eq!(zone.kind, ZoneKind::GlacierCore);
    if cast {
        assert_eq!(zone.remaining, 5.0);
        assert_eq!(zone.pulse_remaining, 0.8);
    }
    world
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs_f32(0.8));
    world.run_system_once(update_spell_zones).unwrap();
    let strikes: Vec<_> = world.query::<&SkillStrike>().iter(world).collect();
    assert!(
        strikes
            .iter()
            .all(|s| s.profile == DamageProfile::for_skill(14) && !s.area)
    );
    let count = strikes.len();
    let damage = strikes[0].damage;
    assert!(strikes.iter().all(|s| s.damage == damage));
    let zone = world.query::<&SkillZone>().single(world).unwrap();
    let result = (
        count,
        zone.pulse_remaining,
        zone.remaining,
        world.resource::<SkillRuntime>().spell_cast_count,
        damage,
    );
    world.run_system_once(resolve_skill_strikes).unwrap();
    result
}
