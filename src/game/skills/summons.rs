use super::*;
use crate::game::summon_movement::{advance, follow};

pub(super) fn update_skill_summons(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    build: Res<PlayerBuild>,
    mut sentries: Query<
        (Entity, &mut Transform, &mut BoneSentry),
        (With<SkillSummon>, Without<SpiritHound>, Without<Enemy>),
    >,
    mut hounds: Query<
        (Entity, &mut Transform, &mut SpiritHound),
        (With<SkillSummon>, Without<BoneSentry>, Without<Enemy>),
    >,
    enemies: Query<(Entity, &Transform, &Enemy), (Without<SkillSummon>, Without<OrbitBlade>)>,
    mut pools: ResMut<CombatPools>,
    market_effects: Res<MarketEffects>,
) {
    if session.paused || session.finished {
        return;
    }
    let delta = time.delta_secs();
    let attack_delta = summon_attack_delta(delta, market_effects.summon_attack_speed_multiplier());
    let targets: Vec<(Entity, Vec3)> = enemies
        .iter()
        .filter(|(_, _, enemy)| enemy.hp > 0.0)
        .map(|(entity, transform, _)| (entity, transform.translation))
        .collect();
    for (entity, mut transform, mut sentry) in &mut sentries {
        sentry.attack_remaining = (sentry.attack_remaining - attack_delta).max(0.0);
        let Some((target_entity, target)) = nearest_position(transform.translation, &targets)
        else {
            follow(&mut transform, entity.to_bits(), 3.8, delta);
            continue;
        };
        if !advance(
            &mut transform,
            target,
            3.8,
            1.1 * rank_size(build.skills[4]),
            delta,
        ) || sentry.attack_remaining > 0.0
        {
            continue;
        }
        sentry.attack_remaining = 0.9;
        schedule_typed_strike(
            &mut commands,
            target_entity,
            sentry.damage,
            0.0,
            0.05,
            false,
            0.72,
            DamageProfile::for_skill(4),
        );
    }

    for (entity, mut transform, mut hound) in &mut hounds {
        hound.remaining -= delta;
        if hound.remaining <= 0.0 {
            pools.retire_summon(&mut commands, entity);
            continue;
        }
        hound.attack_remaining = (hound.attack_remaining - attack_delta).max(0.0);
        let Some((target_entity, target)) = nearest_position(transform.translation, &targets)
        else {
            follow(&mut transform, entity.to_bits(), 5.2, delta);
            continue;
        };
        if advance(
            &mut transform,
            target,
            5.2,
            1.1 * rank_size(build.skills[5]),
            delta,
        ) && hound.attack_remaining <= 0.0
        {
            hound.attack_remaining += 0.7;
            schedule_typed_strike(
                &mut commands,
                target_entity,
                hound.damage,
                0.0,
                0.05,
                false,
                0.48,
                DamageProfile::for_skill(5),
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn update_rare_summons(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    build: Res<PlayerBuild>,
    assets: Option<Res<GameAssets>>,
    mut runtime: ResMut<SkillRuntime>,
    mut tide_elementals: Query<
        (Entity, &mut Transform, &mut TideElemental),
        (With<SkillSummon>, Without<BoltTurret>, Without<Enemy>),
    >,
    mut bolt_turrets: Query<
        (Entity, &Transform, &mut BoltTurret, Option<&mut Visibility>),
        (With<SkillSummon>, Without<TideElemental>, Without<Enemy>),
    >,
    enemies: Query<(Entity, &Transform, &Enemy), (Without<SkillSummon>, Without<Hero>)>,
    mut pools: ResMut<CombatPools>,
    market_effects: Res<MarketEffects>,
) {
    if session.paused || session.finished {
        return;
    }
    let Some(assets) = assets else {
        return;
    };
    let delta = time.delta_secs();
    let attack_delta = summon_attack_delta(delta, market_effects.summon_attack_speed_multiplier());
    let targets: Vec<(Entity, Vec3)> = enemies
        .iter()
        .filter(|(_, _, enemy)| enemy.hp > 0.0)
        .map(|(entity, transform, _)| (entity, transform.translation))
        .collect();
    for (entity, mut transform, mut elemental) in &mut tide_elementals {
        elemental.remaining -= delta;
        if elemental.remaining <= 0.0 {
            pools.retire_summon(&mut commands, entity);
            continue;
        }
        elemental.attack_remaining = (elemental.attack_remaining - attack_delta).max(0.0);
        let Some((target_entity, target)) = nearest_position(transform.translation, &targets)
        else {
            follow(&mut transform, entity.to_bits(), 3.5, delta);
            continue;
        };
        if !advance(
            &mut transform,
            target,
            3.5,
            6.0 * rank_size(build.skills[16]),
            delta,
        ) {
            continue;
        }
        if elemental.attack_remaining <= 0.0 {
            elemental.attack_remaining += 1.0;
            let direction = (target - transform.translation).normalize_or_zero();
            if spawn_typed_projectile(
                &mut commands,
                &assets,
                &mut pools,
                transform.translation + Vec3::Y * 0.65,
                direction,
                15.0,
                elemental.damage,
                0.46,
                0,
                1.8,
                0.76,
                DamageProfile::for_skill(16),
            ) {
                runtime.wet.insert(target_entity, 3.0);
            }
        }
    }

    for (entity, transform, mut turret, visibility) in &mut bolt_turrets {
        let gate_down = session.core_hp <= 0.0 || session.gate_revive_remaining > 0.0;
        if let Some(mut visibility) = visibility {
            *visibility = if gate_down {
                Visibility::Hidden
            } else {
                Visibility::Visible
            };
        }
        turret.remaining -= delta;
        if turret.remaining <= 0.0 {
            pools.retire_summon(&mut commands, entity);
            continue;
        }
        if gate_down {
            continue;
        }
        turret.attack_remaining = (turret.attack_remaining - attack_delta).max(0.0);
        if turret.attack_remaining > 0.0 {
            continue;
        }
        let Some((_, target)) = targets
            .iter()
            .copied()
            .filter(|(_, position)| {
                (*position - transform.translation)
                    .with_y(0.0)
                    .length_squared()
                    <= (8.0 * rank_size(build.skills[17])).powi(2)
            })
            .min_by(|left, right| left.1.x.total_cmp(&right.1.x))
        else {
            continue;
        };
        turret.attack_remaining += 0.72;
        spawn_typed_projectile(
            &mut commands,
            &assets,
            &mut pools,
            transform.translation + Vec3::Y * 0.75,
            (target - transform.translation).normalize_or_zero(),
            21.0,
            turret.damage,
            0.43,
            1,
            1.7,
            0.72,
            DamageProfile::for_skill(17),
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn update_epic_summons(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    build: Res<PlayerBuild>,
    assets: Option<Res<GameAssets>>,
    mut runtime: ResMut<SkillRuntime>,
    mut wyrms: Query<
        (Entity, &mut Transform, &mut FrostWyrm),
        (With<SkillSummon>, Without<AncestorColossus>, Without<Enemy>),
    >,
    mut colossi: Query<
        (Entity, &mut Transform, &mut AncestorColossus),
        (With<SkillSummon>, Without<FrostWyrm>, Without<Enemy>),
    >,
    mut enemies: Query<
        (
            Entity,
            &mut Transform,
            &Enemy,
            Option<&BossEnemy>,
            Option<&TrialEnemy>,
            Option<&ArchiveEcho>,
        ),
        (Without<SkillSummon>, Without<Hero>),
    >,
    mut pools: ResMut<CombatPools>,
    market_effects: Res<MarketEffects>,
) {
    if session.paused || session.finished {
        return;
    }
    let Some(assets) = assets else {
        return;
    };
    let delta = time.delta_secs();
    let attack_delta = summon_attack_delta(delta, market_effects.summon_attack_speed_multiplier());
    let snapshots: Vec<TargetInfo> = enemies
        .iter()
        .filter(|(_, _, enemy, _, _, _)| enemy.hp > 0.0)
        .map(|(entity, transform, enemy, boss, trial, archive)| {
            target_info(entity, transform, enemy, boss, trial, archive)
        })
        .collect();

    for (entity, mut transform, mut wyrm) in &mut wyrms {
        wyrm.remaining -= delta;
        if wyrm.remaining <= 0.0 {
            pools.retire_summon(&mut commands, entity);
            continue;
        }
        wyrm.attack_remaining = (wyrm.attack_remaining - attack_delta).max(0.0);
        let Some(target) = snapshots.iter().copied().min_by(|left, right| {
            transform
                .translation
                .distance_squared(left.position)
                .total_cmp(&transform.translation.distance_squared(right.position))
        }) else {
            follow(&mut transform, entity.to_bits(), 4.2, delta);
            continue;
        };
        if !advance(
            &mut transform,
            target.position,
            4.2,
            7.0 * rank_size(build.skills[26]),
            delta,
        ) || wyrm.attack_remaining > 0.0
        {
            continue;
        }
        wyrm.attack_remaining += 1.15;
        let direction = (target.position - transform.translation).normalize_or_zero();
        if spawn_typed_projectile(
            &mut commands,
            &assets,
            &mut pools,
            transform.translation + Vec3::Y * 0.7,
            direction,
            17.0,
            wyrm.damage,
            0.62,
            3,
            2.0,
            1.05,
            DamageProfile::for_skill(26),
        ) {
            for hit in line_targets(
                transform.translation,
                direction,
                &snapshots,
                14.0 * rank_size(build.skills[26]),
                0.9 * rank_size(build.skills[26]),
                4,
            ) {
                let count = wyrm.frost_hits.entry(hit.entity).or_default();
                *count += 1;
                if *count >= 3 {
                    *count = 0;
                    apply_control(&mut runtime, hit, 1.8, 1.2, 0.0);
                }
            }
        }
        wyrm.frost_hits
            .retain(|target, _| enemies.contains(*target));
    }

    for (entity, mut transform, mut colossus) in &mut colossi {
        colossus.remaining -= delta;
        colossus.slam_remaining = (colossus.slam_remaining - attack_delta).max(0.0);
        if colossus.remaining <= 0.0 {
            pools.retire_summon(&mut commands, entity);
            continue;
        }
        let Some(target) = snapshots.iter().min_by(|a, b| {
            transform
                .translation
                .distance_squared(a.position)
                .total_cmp(&transform.translation.distance_squared(b.position))
        }) else {
            follow(&mut transform, entity.to_bits(), 3.0, delta);
            continue;
        };
        let in_reach = advance(
            &mut transform,
            target.position,
            3.0,
            2.6 * rank_size(build.skills[27]),
            delta,
        );
        let center = transform.translation;
        for (enemy_entity, mut enemy_transform, enemy, boss, trial, archive) in &mut enemies {
            if enemy.hp <= 0.0 {
                continue;
            }
            let distance = center.distance(enemy_transform.translation);
            if distance > skill_effect_radius(27, build.skills[27]).unwrap() {
                continue;
            }
            let target = target_info(enemy_entity, &enemy_transform, enemy, boss, trial, archive);
            let pull_scale = forced_movement_scale(enemy.elite, boss.is_some());
            if distance > 1.55 {
                let to_center = center - enemy_transform.translation;
                enemy_transform.translation += to_center.normalize_or_zero()
                    * (2.4 * pull_scale * delta).min((distance - 1.55).max(0.0));
            } else {
                apply_control(&mut runtime, target, 0.0, 0.0, 0.16);
            }
        }
        while in_reach && colossus.slam_remaining <= 0.0 && colossus.remaining >= -0.001 {
            colossus.slam_remaining += 3.0;
            schedule_typed_fixed_strike(
                &mut commands,
                center,
                colossus.damage,
                0.0,
                3.1,
                true,
                3.25 * rank_size(build.skills[27]),
                DamageProfile::for_skill(27),
            );
        }
        if colossus.remaining <= 0.0 {
            pools.retire_summon(&mut commands, entity);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<GameSession>()
            .init_resource::<CombatPools>()
            .init_resource::<MarketEffects>()
            .init_resource::<SkillRuntime>()
            .init_resource::<PlayerBuild>();
        app.insert_resource(GameAssets {
            hero_scenes: default(),
            enemy_scenes: default(),
            boss_scenes: default(),
            summon_scenes: default(),
            projectile_mesh: default(),
            fx_mesh: default(),
            enemy_material: default(),
            enemy_materials: default(),
            elite_material: default(),
            boss_materials: default(),
            hero_materials: default(),
            projectile_material: default(),
            storm_material: default(),
        });
        app.world_mut().spawn((Hero, Transform::default()));
        app
    }
    fn enemy(app: &mut App, x: f32) -> Entity {
        app.world_mut()
            .spawn((
                Enemy {
                    hp: 1000.0,
                    max_hp: 1000.0,
                    speed: 0.0,
                    damage: 0.0,
                    xp: 0,
                    wood: 0,
                    gold: 0,
                    kill_credit: 0,
                    elite: false,
                },
                Transform::from_xyz(x, 0.0, 0.0),
            ))
            .id()
    }
    fn tick(app: &mut App, delta: f32) {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(delta));
    }
    #[test]
    fn wall_turret_stops_firing_while_gate_is_down_and_resumes_after_rebuild() {
        let mut app = app();
        enemy(&mut app, 3.0);
        let tower = app
            .world_mut()
            .spawn((
                SkillSummon,
                Visibility::Visible,
                BoltTurret {
                    remaining: 20.0,
                    attack_remaining: 0.0,
                    damage: 20.0,
                },
                Transform::default(),
            ))
            .id();
        app.world_mut().resource_mut::<GameSession>().core_hp = 0.0;
        app.world_mut()
            .resource_mut::<GameSession>()
            .gate_revive_remaining = 4.0;
        tick(&mut app, 1.0);
        app.world_mut()
            .run_system_once(update_rare_summons)
            .unwrap();
        assert_eq!(
            app.world_mut()
                .query::<&Projectile>()
                .iter(app.world())
                .count(),
            0
        );
        assert_eq!(
            app.world()
                .get::<BoltTurret>(tower)
                .unwrap()
                .attack_remaining,
            0.0
        );
        assert_eq!(
            app.world().get::<Visibility>(tower),
            Some(&Visibility::Hidden)
        );
        {
            let mut session = app.world_mut().resource_mut::<GameSession>();
            session.core_hp = session.core_max_hp;
            session.gate_revive_remaining = 0.0;
        }
        tick(&mut app, 0.1);
        app.world_mut()
            .run_system_once(update_rare_summons)
            .unwrap();
        assert_eq!(
            app.world_mut()
                .query::<&Projectile>()
                .iter(app.world())
                .count(),
            1
        );
        assert_eq!(
            app.world().get::<Visibility>(tower),
            Some(&Visibility::Visible)
        );
    }
    #[test]
    fn skeleton_chases_before_emitting_real_contact_and_pause_freezes_it() {
        let mut app = app();
        let target = enemy(&mut app, 8.0);
        let skeleton = app
            .world_mut()
            .spawn((
                SkillSummon,
                BoneSentry {
                    attack_remaining: 0.0,
                    damage: 50.0,
                },
                Transform::default(),
            ))
            .id();
        tick(&mut app, 0.25);
        app.world_mut()
            .run_system_once(update_skill_summons)
            .unwrap();
        let position = app.world().get::<Transform>(skeleton).unwrap().translation;
        assert!(position.x > 0.9 && position.x < 1.0);
        assert_eq!(
            app.world_mut()
                .query::<&SkillStrike>()
                .iter(app.world())
                .count(),
            0
        );
        app.world_mut().resource_mut::<GameSession>().paused = true;
        tick(&mut app, 1.0);
        app.world_mut()
            .run_system_once(update_skill_summons)
            .unwrap();
        assert_eq!(
            app.world().get::<Transform>(skeleton).unwrap().translation,
            position
        );
        app.world_mut().resource_mut::<GameSession>().paused = false;
        tick(&mut app, 32.0);
        app.world_mut()
            .run_system_once(update_skill_summons)
            .unwrap();
        assert!(
            (app.world()
                .get::<Transform>(skeleton)
                .unwrap()
                .translation
                .x
                - 6.9)
                .abs()
                < 0.001
        );
        let strikes: Vec<_> = app
            .world_mut()
            .query::<&SkillStrike>()
            .iter(app.world())
            .collect();
        assert_eq!(strikes.len(), 1);
        assert_eq!(strikes[0].target, target);
        assert_eq!(strikes[0].damage, 50.0);
        assert_eq!(strikes[0].profile.skill_index, Some(4));
    }
    #[test]
    fn hound_never_overshoots_at_high_speed_and_returns_when_enemy_dies() {
        let mut app = app();
        let target = enemy(&mut app, 6.0);
        let hound = app
            .world_mut()
            .spawn((
                SkillSummon,
                SpiritHound {
                    source_skill: 5,
                    remaining: 100.0,
                    attack_remaining: 0.0,
                    damage: 20.0,
                },
                Transform::default(),
            ))
            .id();
        tick(&mut app, 10.0);
        app.world_mut()
            .run_system_once(update_skill_summons)
            .unwrap();
        let before = app.world().get::<Transform>(hound).unwrap().translation;
        assert!((before.x - 4.9).abs() < 0.001);
        app.world_mut().get_mut::<Enemy>(target).unwrap().hp = 0.0;
        tick(&mut app, 1.0);
        app.world_mut()
            .run_system_once(update_skill_summons)
            .unwrap();
        assert!(
            app.world()
                .get::<Transform>(hound)
                .unwrap()
                .translation
                .length()
                < before.length()
        );
    }
    #[test]
    fn water_elemental_approaches_range_while_stationary_turret_cannot_shoot_across_arena() {
        let mut app = app();
        enemy(&mut app, 15.0);
        let water = app
            .world_mut()
            .spawn((
                SkillSummon,
                TideElemental {
                    remaining: 50.0,
                    attack_remaining: 0.0,
                    damage: 20.0,
                },
                Transform::default(),
            ))
            .id();
        let turret = app
            .world_mut()
            .spawn((
                SkillSummon,
                BoltTurret {
                    remaining: 50.0,
                    attack_remaining: 0.0,
                    damage: 20.0,
                },
                Transform::default(),
            ))
            .id();
        tick(&mut app, 1.0);
        app.world_mut()
            .run_system_once(update_rare_summons)
            .unwrap();
        assert_eq!(
            app.world().get::<Transform>(water).unwrap().translation.x,
            3.5
        );
        assert_eq!(
            app.world().get::<Transform>(turret).unwrap().translation,
            Vec3::ZERO
        );
        assert_eq!(
            app.world_mut()
                .query::<&Projectile>()
                .iter(app.world())
                .count(),
            0
        );
        tick(&mut app, 2.0);
        app.world_mut()
            .run_system_once(update_rare_summons)
            .unwrap();
        assert!((app.world().get::<Transform>(water).unwrap().translation.x - 9.0).abs() < 0.001);
        assert_eq!(
            app.world_mut()
                .query::<&Projectile>()
                .iter(app.world())
                .count(),
            1
        );
    }
}
