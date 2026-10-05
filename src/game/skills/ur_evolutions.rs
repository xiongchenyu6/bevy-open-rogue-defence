//! Exclusive UR specializations. Every offensive branch uses ordinary typed strikes.
use super::*;
use crate::game::{
    hero_abilities::BasicAttackCounter,
    skill_evolution::{SkillEvolution, UrEvolutionPath},
};
#[derive(Message, Clone, Copy)]
pub struct UrBranchCue {
    pub skill: usize,
    pub path: UrEvolutionPath,
    pub origin: Vec3,
    pub target: Vec3,
}
#[derive(Resource, Default)]
struct BranchClock {
    remaining: [f32; 4],
    attack_seen: u64,
    attack_progress: u8,
}
pub(super) fn install(app: &mut App) {
    app.add_message::<UrBranchCue>()
        .init_resource::<BranchClock>()
        .add_systems(Update, activate.after(cast_learned_skills));
}
pub(super) fn reset_run(world: &mut World) {
    world.insert_resource(BranchClock::default());
    if let Some(mut cues) = world.get_resource_mut::<Messages<UrBranchCue>>() {
        cues.clear();
    }
}
fn ready(clock: &mut f32, delta: f32, period: f32) -> bool {
    *clock = (*clock - delta).max(0.0);
    if *clock > 0.0 {
        false
    } else {
        *clock = period;
        true
    }
}
fn path(evolution: &SkillEvolution, build: &PlayerBuild, index: usize) -> Option<UrEvolutionPath> {
    (build.skills[index] >= 5)
        .then(|| evolution.paths[index])
        .flatten()
}
#[derive(bevy::ecs::system::SystemParam)]
struct State<'w> {
    time: Res<'w, Time>,
    session: ResMut<'w, GameSession>,
    build: Res<'w, PlayerBuild>,
    vitals: ResMut<'w, HeroVitals>,
    evolution: Res<'w, SkillEvolution>,
    clock: ResMut<'w, BranchClock>,
    runtime: ResMut<'w, SkillRuntime>,
    pools: ResMut<'w, CombatPools>,
    assets: Option<Res<'w, GameAssets>>,
    attacks: Res<'w, BasicAttackCounter>,
    market: Res<'w, MarketEffects>,
}
#[allow(clippy::too_many_arguments)]
fn activate(
    mut commands: Commands,
    state: State,
    mut cues: MessageWriter<UrBranchCue>,
    hero: Query<&Transform, (With<Hero>, Without<Enemy>, Without<SkillSummon>)>,
    mut enemies: Query<
        (
            Entity,
            &mut Transform,
            &Enemy,
            Option<&BossEnemy>,
            Option<&TrialEnemy>,
            Option<&ArchiveEcho>,
        ),
        (Without<Hero>, Without<SkillSummon>),
    >,
    summons: Query<&Transform, (With<SkillSummon>, Without<Enemy>, Without<Hero>)>,
    mut wyrms: Query<(Entity, &Transform, &mut FrostWyrm), (Without<Enemy>, Without<Hero>)>,
    colossi: Query<&Transform, (With<AncestorColossus>, Without<Enemy>, Without<Hero>)>,
    hounds: Query<(Entity, &SpiritHound)>,
) {
    let State {
        time,
        mut session,
        build,
        mut vitals,
        evolution,
        mut clock,
        mut runtime,
        mut pools,
        assets,
        attacks,
        market,
    } = state;
    if session.paused || session.finished || !vitals.can_act() {
        return;
    }
    let Ok(hero) = hero.single() else {
        return;
    };
    let mut targets: Vec<_> = enemies
        .iter()
        .filter(|(_, _, enemy, _, _, _)| enemy.hp > 0.0)
        .map(|(entity, t, e, b, trial, a)| target_info(entity, t, e, b, trial, a))
        .collect();
    targets.sort_by(|a, b| {
        hero.translation
            .distance_squared(a.position)
            .total_cmp(&hero.translation.distance_squared(b.position))
    });
    let attack_delta = attacks.0.saturating_sub(clock.attack_seen);
    clock.attack_seen = attacks.0;
    let Some(primary) = targets.first().copied() else {
        return;
    };
    let spell = effective_spell_power(&vitals, &runtime).max(1.0);
    let summon_power = effective_summon_power(&vitals, &runtime).max(1.0);
    let delta = time.delta_secs();
    for index in 26..=29 {
        let Some(branch) = path(&evolution, &build, index) else {
            clock.remaining[index - 26] = 0.0;
            continue;
        };
        let period = match (index, branch) {
            (26, UrEvolutionPath::Control) => 2.0,
            (26, _) => 3.0,
            (27, UrEvolutionPath::Control) => 1.0,
            (27, UrEvolutionPath::Sustain) => 4.0,
            (27, _) => 3.0,
            (28, UrEvolutionPath::Control) => 2.5,
            (28, _) => 3.0,
            (29, UrEvolutionPath::Surge) => 6.0,
            (29, UrEvolutionPath::Control) => 5.0,
            _ => 8.0,
        };
        // Eternal twins remain bounded even when the base skill recasts a third dragon.
        if index == 26 && branch == UrEvolutionPath::Sustain {
            for (n, (entity, _, mut wyrm)) in wyrms.iter_mut().enumerate() {
                if n >= 2 {
                    pools.retire_summon(&mut commands, entity);
                } else {
                    wyrm.remaining = wyrm.remaining.max(21.0);
                }
            }
        }
        if index == 29 && branch == UrEvolutionPath::Surge {
            for (entity, _) in hounds.iter().filter(|(_, h)| h.source_skill == 29).skip(4) {
                pools.retire_summon(&mut commands, entity);
            }
        }
        if index == 28 && branch == UrEvolutionPath::Surge {
            clock.attack_progress = clock
                .attack_progress
                .saturating_add(attack_delta.min(4) as u8);
            if clock.attack_progress < 4 {
                continue;
            }
            clock.attack_progress -= 4;
        } else if !ready(&mut clock.remaining[index - 26], delta, period) {
            continue;
        }
        let mut emit = |origin: Vec3, target: Vec3| {
            cues.write(UrBranchCue {
                skill: index,
                path: branch,
                origin,
                target,
            });
        };
        match (index, branch) {
            (26, UrEvolutionPath::Surge) => {
                let Some((_, dragon, _)) = wyrms.iter().next() else {
                    continue;
                };
                for target in targets
                    .iter()
                    .filter(|t| t.position.distance(dragon.translation) <= 12.0)
                    .take(6)
                {
                    emit(dragon.translation, target.position);
                    strike(&mut commands, *target, summon_power * 0.9, 26, 0.7);
                    apply_control(&mut runtime, *target, 2.0, 0.0, 0.0);
                }
            }
            (26, UrEvolutionPath::Control) => {
                let Some((_, dragon, _)) = wyrms.iter().next() else {
                    continue;
                };
                emit(dragon.translation, primary.position);
                strike(&mut commands, primary, summon_power * 1.4, 26, 1.1);
                apply_control(&mut runtime, primary, 0.0, 1.5, 0.0);
            }
            (26, UrEvolutionPath::Sustain) => {
                if let Some(assets) = assets.as_deref() {
                    let existing = wyrms.iter().count();
                    let mut budget = SummonBudget::from_existing(
                        summons.iter().count(),
                        market.summon_cap_bonus(),
                    );
                    for n in existing..2 {
                        if !budget.take() {
                            break;
                        }
                        if let Some(entity) = pools.rent_scene_summon(
                            &mut commands,
                            SummonVisual::FrostWyrm,
                            assets.summon_scenes[4].clone(),
                            Transform::from_translation(
                                hero.translation.with_x(
                                    hero.translation
                                        .x
                                        .max(super::super::resources::GATE_POSITION.x + 2.3),
                                ) + Vec3::new(0.0, 0.0, n as f32 * 2.0 - 1.0),
                            )
                            .with_scale(growth::summon_scale(26, 5)),
                        ) {
                            emit(
                                hero.translation,
                                hero.translation.with_x(
                                    hero.translation
                                        .x
                                        .max(super::super::resources::GATE_POSITION.x + 2.3),
                                ) + Vec3::new(0.0, 0.0, n as f32 * 2.0 - 1.0),
                            );
                            commands.entity(entity).insert((
                                Name::new("UR eternal frost twin"),
                                SkillSummon,
                                FrostWyrm {
                                    remaining: 21.0,
                                    attack_remaining: 0.2,
                                    damage: scaled_skill_damage(26, 5, &build, &vitals, &runtime),
                                    frost_hits: HashMap::new(),
                                },
                            ));
                        }
                    }
                }
            }
            (27, UrEvolutionPath::Surge) => {
                if let Some(colossus) = colossi.iter().next() {
                    emit(colossus.translation, colossus.translation);
                    schedule_typed_fixed_strike(
                        &mut commands,
                        colossus.translation,
                        summon_power * 1.8,
                        0.0,
                        5.5,
                        true,
                        5.5,
                        DamageProfile::for_skill(27),
                    );
                }
            }
            (27, UrEvolutionPath::Control) => {
                if let Some(colossus) = colossi.iter().next() {
                    for target in targets
                        .iter()
                        .filter(|t| t.position.distance(colossus.translation) <= 6.0)
                        .take(24)
                    {
                        if let Ok((_, mut transform, enemy, boss, _, _)) =
                            enemies.get_mut(target.entity)
                        {
                            let shift = colossus.translation - transform.translation;
                            let amount = (shift.length() - 1.5).clamp(0.0, 2.0)
                                * forced_movement_scale(enemy.elite, boss.is_some());
                            transform.translation += shift.normalize_or_zero() * amount;
                        }
                        emit(colossus.translation, target.position);
                        apply_control(&mut runtime, *target, 1.0, 0.0, 0.35);
                        strike(&mut commands, *target, summon_power * 0.3, 27, 0.5);
                    }
                }
            }
            (27, UrEvolutionPath::Sustain) => {
                if let Some(colossus) = colossi.iter().next() {
                    emit(colossus.translation, hero.translation);
                    let heal = vitals.max_health * 0.02;
                    vitals.heal(heal);
                    let repair = session.core_max_hp * 0.02;
                    session.heal_gate(repair);
                }
            }
            (28, UrEvolutionPath::Surge) => {
                emit(hero.translation, primary.position);
                schedule_typed_strike(
                    &mut commands,
                    primary.entity,
                    build.attack_damage * 0.7,
                    0.0,
                    0.5,
                    false,
                    1.2,
                    DamageProfile::new(DamageElement::Physical, DamageClass::Attack),
                );
            }
            (28, UrEvolutionPath::Control) => {
                for (n, target) in targets.iter().take(4).enumerate() {
                    emit(hero.translation, target.position);
                    schedule_typed_strike(
                        &mut commands,
                        target.entity,
                        spell * 0.7,
                        n as f32 * 0.08,
                        0.4,
                        false,
                        0.8,
                        spell_profile(28),
                    );
                }
            }
            (28, UrEvolutionPath::Sustain) => {
                for position in summons.iter().take(8) {
                    if let Some(target) = targets
                        .iter()
                        .filter(|t| t.position.distance(position.translation) <= 8.0)
                        .min_by(|a, b| {
                            a.position
                                .distance_squared(position.translation)
                                .total_cmp(&b.position.distance_squared(position.translation))
                        })
                    {
                        emit(position.translation, target.position);
                        schedule_typed_strike(
                            &mut commands,
                            target.entity,
                            summon_power * 0.65,
                            0.0,
                            0.5,
                            false,
                            1.0,
                            DamageProfile::new(DamageElement::Arcane, DamageClass::Summon),
                        );
                    }
                }
            }
            (29, UrEvolutionPath::Surge) => {
                let Some(assets) = assets.as_deref() else {
                    continue;
                };
                let existing = hounds
                    .iter()
                    .filter(|(_, hound)| hound.source_skill == 29)
                    .count();
                let mut budget =
                    SummonBudget::from_existing(summons.iter().count(), market.summon_cap_bonus());
                for n in 0..2.min(4usize.saturating_sub(existing)) {
                    if !budget.take() {
                        break;
                    }
                    if let Some(entity) = pools.rent_scene_summon(
                        &mut commands,
                        SummonVisual::SpiritHound,
                        assets.summon_scenes[1].clone(),
                        Transform::from_translation(
                            hero.translation.with_x(
                                hero.translation
                                    .x
                                    .max(super::super::resources::GATE_POSITION.x + 2.3),
                            ) + Vec3::new(0.0, 0.0, n as f32),
                        )
                        .with_scale(growth::summon_scale(29, 5)),
                    ) {
                        emit(
                            hero.translation,
                            hero.translation.with_x(
                                hero.translation
                                    .x
                                    .max(super::super::resources::GATE_POSITION.x + 2.3),
                            ) + Vec3::new(0.0, 0.0, n as f32),
                        );
                        commands.entity(entity).insert((
                            Name::new("UR fate hunting spirit"),
                            SkillSummon,
                            SpiritHound {
                                source_skill: 29,
                                remaining: 12.0,
                                attack_remaining: 0.0,
                                damage: summon_power * 0.9,
                            },
                        ));
                    }
                }
            }
            (29, UrEvolutionPath::Control) => {
                for target in targets.iter().take(6) {
                    emit(hero.translation, target.position);
                    strike(&mut commands, *target, spell * 1.8, 29, 1.2);
                    apply_control(&mut runtime, *target, 0.0, 0.0, 1.0);
                }
            }
            (29, UrEvolutionPath::Sustain) => {
                emit(hero.translation, hero.translation);
                session.gold = session.gold.saturating_add(20);
                session.wood = session.wood.saturating_add(4);
            }
            _ => {}
        }
    }
}
fn spell_profile(index: usize) -> DamageProfile {
    let mut profile = DamageProfile::for_skill(index);
    profile.class = DamageClass::Spell;
    profile
}
fn strike(commands: &mut Commands, target: TargetInfo, damage: f32, index: usize, fx: f32) {
    schedule_typed_strike(
        commands,
        target.entity,
        damage,
        0.0,
        0.4,
        false,
        fx,
        if index == 29 {
            spell_profile(index)
        } else {
            DamageProfile::for_skill(index)
        },
    );
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn specialization_requires_mastery_and_an_explicit_choice() {
        let mut e = SkillEvolution::default();
        let mut b = PlayerBuild::default();
        b.skills[26] = 5;
        assert_eq!(path(&e, &b, 26), None);
        for p in [
            UrEvolutionPath::Surge,
            UrEvolutionPath::Control,
            UrEvolutionPath::Sustain,
        ] {
            e.paths[26] = Some(p);
            assert_eq!(path(&e, &b, 26), Some(p));
            b.skills[26] = 4;
            assert_eq!(path(&e, &b, 26), None);
            b.skills[26] = 5;
        }
    }
    #[test]
    fn periodic_branches_never_accumulate_catch_up_bursts() {
        let mut timer = 0.0;
        assert!(ready(&mut timer, 100.0, 3.0));
        assert_eq!(timer, 3.0);
        assert!(!ready(&mut timer, 0.1, 3.0));
    }
    fn test_app(index: usize, branch: UrEvolutionPath) -> (App, Entity) {
        let mut app = App::new();
        app.add_message::<UrBranchCue>();
        app.init_resource::<Time>()
            .init_resource::<GameSession>()
            .init_resource::<PlayerBuild>()
            .init_resource::<HeroVitals>()
            .init_resource::<SkillEvolution>()
            .init_resource::<BranchClock>()
            .init_resource::<SkillRuntime>()
            .init_resource::<CombatPools>()
            .init_resource::<BasicAttackCounter>()
            .init_resource::<MarketEffects>();
        app.world_mut().resource_mut::<PlayerBuild>().skills[index] = 5;
        app.world_mut().resource_mut::<SkillEvolution>().paths[index] = Some(branch);
        app.world_mut().spawn((Hero, Transform::default()));
        let enemy = app
            .world_mut()
            .spawn((
                Transform::from_xyz(3.0, 0.0, 0.0),
                Enemy {
                    hp: 1000.0,
                    max_hp: 1000.0,
                    speed: 2.0,
                    damage: 1.0,
                    xp: 1,
                    wood: 0,
                    gold: 1,
                    kill_credit: 1,
                    elite: false,
                },
            ))
            .id();
        (app, enemy)
    }
    #[test]
    fn fate_control_emits_real_typed_strike_and_stun_but_freezes_when_paused() {
        use bevy::ecs::system::RunSystemOnce;
        let (mut app, enemy) = test_app(29, UrEvolutionPath::Control);
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.world_mut().run_system_once(activate).unwrap();
        assert_eq!(
            app.world_mut()
                .query::<&SkillStrike>()
                .iter(app.world())
                .count(),
            0
        );
        app.world_mut().resource_mut::<GameSession>().paused = false;
        app.world_mut().run_system_once(activate).unwrap();
        let strikes: Vec<_> = app
            .world_mut()
            .query::<&SkillStrike>()
            .iter(app.world())
            .collect();
        assert_eq!(strikes.len(), 1);
        assert_eq!(strikes[0].target, enemy);
        assert!(strikes[0].damage > 0.0);
        assert!(app.world().resource::<SkillRuntime>().controls[&enemy].stunned_remaining > 0.0);
    }
    #[test]
    fn fortune_pays_only_during_live_combat_and_never_when_unlearned() {
        use bevy::ecs::system::RunSystemOnce;
        let (mut app, enemy) = test_app(29, UrEvolutionPath::Sustain);
        let gold = app.world().resource::<GameSession>().gold;
        app.world_mut().run_system_once(activate).unwrap();
        assert_eq!(app.world().resource::<GameSession>().gold, gold + 20);
        app.world_mut().resource_mut::<BranchClock>().remaining[3] = 0.0;
        app.world_mut().get_mut::<Enemy>(enemy).unwrap().hp = 0.0;
        app.world_mut().run_system_once(activate).unwrap();
        assert_eq!(app.world().resource::<GameSession>().gold, gold + 20);
        app.world_mut().get_mut::<Enemy>(enemy).unwrap().hp = 1000.0;
        app.world_mut().resource_mut::<PlayerBuild>().skills[29] = 0;
        app.world_mut().run_system_once(activate).unwrap();
        assert_eq!(app.world().resource::<GameSession>().gold, gold + 20);
    }
    #[test]
    fn martial_wheel_waits_for_four_actual_attacks_and_cannot_recurse() {
        use bevy::ecs::system::RunSystemOnce;
        let (mut app, _) = test_app(28, UrEvolutionPath::Surge);
        app.world_mut().resource_mut::<BasicAttackCounter>().0 = 3;
        app.world_mut().run_system_once(activate).unwrap();
        assert_eq!(
            app.world_mut()
                .query::<&SkillStrike>()
                .iter(app.world())
                .count(),
            0
        );
        app.world_mut().resource_mut::<BasicAttackCounter>().0 = 4;
        app.world_mut().run_system_once(activate).unwrap();
        assert_eq!(
            app.world_mut()
                .query::<&SkillStrike>()
                .iter(app.world())
                .count(),
            1
        );
        app.world_mut().run_system_once(activate).unwrap();
        assert_eq!(
            app.world_mut()
                .query::<&SkillStrike>()
                .iter(app.world())
                .count(),
            1
        );
    }
    fn summon_entity(app: &mut App, index: usize) -> Entity {
        let entity = app
            .world_mut()
            .spawn((SkillSummon, Transform::default()))
            .id();
        if index == 26 {
            app.world_mut().entity_mut(entity).insert(FrostWyrm {
                remaining: 1.0,
                attack_remaining: 0.0,
                damage: 10.0,
                frost_hits: HashMap::new(),
            });
        } else {
            app.world_mut().entity_mut(entity).insert(AncestorColossus {
                remaining: 20.0,
                slam_remaining: 3.0,
                damage: 10.0,
            });
        }
        entity
    }
    fn add_assets(app: &mut App) {
        app.insert_resource(GameAssets {
            hero_scenes: default(),
            enemy_scenes: default(),
            boss_scenes: default(),
            adventure_scenes: default(),
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
    }
    #[test]
    fn frost_branches_emit_distinct_actual_damage_and_controls() {
        use bevy::ecs::system::RunSystemOnce;
        for branch in [UrEvolutionPath::Surge, UrEvolutionPath::Control] {
            let (mut app, enemy) = test_app(26, branch);
            summon_entity(&mut app, 26);
            app.world_mut().run_system_once(activate).unwrap();
            let strike = app
                .world_mut()
                .query::<&SkillStrike>()
                .single(app.world())
                .unwrap();
            assert_eq!(strike.target, enemy);
            assert!(strike.damage > 0.0);
            assert_eq!(strike.profile.class, DamageClass::Summon);
            let status = &app.world().resource::<SkillRuntime>().controls[&enemy];
            if branch == UrEvolutionPath::Control {
                assert_eq!(status.frozen_remaining, 1.5);
            } else {
                assert_eq!(status.slow_remaining, 2.0);
                assert_eq!(status.frozen_remaining, 0.0);
            }
        }
    }
    #[test]
    fn colossus_branches_produce_large_slam_pull_or_real_repair() {
        use bevy::ecs::system::RunSystemOnce;
        for branch in [
            UrEvolutionPath::Surge,
            UrEvolutionPath::Control,
            UrEvolutionPath::Sustain,
        ] {
            let (mut app, enemy) = test_app(27, branch);
            summon_entity(&mut app, 27);
            app.world_mut().resource_mut::<GameSession>().core_hp = 100.0;
            app.world_mut().resource_mut::<HeroVitals>().health = 100.0;
            app.world_mut().run_system_once(activate).unwrap();
            match branch {
                UrEvolutionPath::Surge => {
                    let strike = app
                        .world_mut()
                        .query::<&SkillStrike>()
                        .single(app.world())
                        .unwrap();
                    assert_eq!(strike.radius, 5.5);
                    assert!(strike.area && strike.damage > 0.0);
                }
                UrEvolutionPath::Control => {
                    assert!(app.world().get::<Transform>(enemy).unwrap().translation.x < 3.0);
                    assert!(
                        app.world().resource::<SkillRuntime>().controls[&enemy].stunned_remaining
                            > 0.0
                    );
                }
                UrEvolutionPath::Sustain => {
                    assert!(app.world().resource::<HeroVitals>().health > 100.0);
                    assert!(app.world().resource::<GameSession>().core_hp > 100.0);
                    assert_eq!(
                        app.world_mut()
                            .query::<&SkillStrike>()
                            .iter(app.world())
                            .count(),
                        0
                    );
                }
            }
        }
    }
    #[test]
    fn triune_caster_and_commander_branches_use_different_damage_classes() {
        use bevy::ecs::system::RunSystemOnce;
        for (branch, class) in [
            (UrEvolutionPath::Control, DamageClass::Spell),
            (UrEvolutionPath::Sustain, DamageClass::Summon),
        ] {
            let (mut app, enemy) = test_app(28, branch);
            summon_entity(&mut app, 27);
            app.world_mut().run_system_once(activate).unwrap();
            let strike = app
                .world_mut()
                .query::<&SkillStrike>()
                .single(app.world())
                .unwrap();
            assert_eq!(strike.target, enemy);
            assert_eq!(strike.profile.class, class);
            assert!(strike.damage > 0.0);
        }
    }
    #[test]
    fn eternal_twins_and_fate_hounds_are_real_bounded_summons() {
        use bevy::ecs::system::RunSystemOnce;
        for (index, branch) in [(26, UrEvolutionPath::Sustain), (29, UrEvolutionPath::Surge)] {
            let (mut app, _) = test_app(index, branch);
            add_assets(&mut app);
            app.world_mut().run_system_once(activate).unwrap();
            assert_eq!(
                app.world_mut()
                    .query::<&SkillSummon>()
                    .iter(app.world())
                    .count(),
                2
            );
            for _ in 0..8 {
                app.world_mut().resource_mut::<BranchClock>().remaining[index - 26] = 0.0;
                app.world_mut().run_system_once(activate).unwrap();
            }
            let count = app
                .world_mut()
                .query::<&SkillSummon>()
                .iter(app.world())
                .count();
            assert_eq!(count, if index == 26 { 2 } else { 4 });
            if index == 26 {
                assert!(
                    app.world_mut()
                        .query::<&FrostWyrm>()
                        .iter(app.world())
                        .all(|wyrm| wyrm.remaining >= 21.0)
                );
            } else {
                assert!(
                    app.world_mut()
                        .query::<&SpiritHound>()
                        .iter(app.world())
                        .all(|hound| hound.source_skill == 29 && hound.damage > 0.0)
                );
            }
        }
    }
}
