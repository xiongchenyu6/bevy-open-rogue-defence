use std::collections::BTreeMap;

use bevy::prelude::*;

use super::combat::CombatSet;
use super::content::BOSSES;
use super::pooling::{CombatPools, EnemySceneKey};
use super::{BossEnemy, Core, Enemy, EnemyArchetype, GameAssets, GameSession, Hero, ImpactFx};

pub struct BossesPlugin;

impl Plugin for BossesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BossRuntime>().add_systems(
            Update,
            (register_bosses, remove_defeated_bosses, run_boss_mechanics)
                .chain()
                .after(CombatSet),
        );
    }
}

#[derive(Debug, Clone, Copy)]
struct BossProfile {
    name: &'static str,
    first_cast_secs: f32,
    period_secs: f32,
    warning_secs: f32,
}

const BOSS_PROFILES: [BossProfile; 5] = [
    BossProfile {
        name: "CINDER COLOSSUS",
        first_cast_secs: 3.0,
        period_secs: 8.0,
        warning_secs: 0.9,
    },
    BossProfile {
        name: "ABYSSAL LEVIATHAN",
        first_cast_secs: 4.0,
        period_secs: 10.0,
        warning_secs: 1.1,
    },
    BossProfile {
        name: "DREAD VEIL",
        first_cast_secs: 4.0,
        period_secs: 9.0,
        warning_secs: 0.8,
    },
    BossProfile {
        name: "SUN CROWN MAGUS",
        first_cast_secs: 3.0,
        period_secs: 6.0,
        warning_secs: 1.0,
    },
    BossProfile {
        name: "GREEN REGENT",
        first_cast_secs: 3.0,
        period_secs: 7.0,
        warning_secs: 1.0,
    },
];

#[derive(Resource, Default)]
struct BossRuntime {
    // Multiple bosses may overlap when a previous timed boss is still alive.
    // Entity order keeps their state updates deterministic across processes;
    // HashMap iteration would otherwise depend on a randomized hasher.
    states: BTreeMap<Entity, BossState>,
}

#[derive(Debug)]
struct BossState {
    index: u8,
    age_secs: f32,
    casts_started: u32,
    pending: Option<PendingAbility>,
    base_speed: f32,
    base_damage: f32,
    action_speed_multiplier: f32,
    enrage_stacks: u8,
}

#[derive(Debug, Clone, Copy)]
struct PendingAbility {
    remaining_secs: f32,
    target: Vec3,
    variant: u8,
}

/// Marks enemies created by boss mechanics so they do not consume one of the
/// 130 ordinary wave slots.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct BossSummon;

fn register_bosses(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    mut new_bosses: Query<(Entity, &BossEnemy, &mut Enemy, &Transform), Added<BossEnemy>>,
    mut runtime: ResMut<BossRuntime>,
    mut pools: ResMut<CombatPools>,
) {
    for (entity, boss, mut enemy, transform) in &mut new_bosses {
        let index = usize::from(boss.index.min(4));
        let modifiers = BOSSES[index].modifiers;
        let health_ratio = if enemy.max_hp > 0.0 {
            enemy.hp / enemy.max_hp
        } else {
            1.0
        };
        enemy.max_hp *= modifiers.max_hp_multiplier;
        enemy.hp = enemy.max_hp * health_ratio;
        enemy.speed *= modifiers.move_speed_multiplier;
        commands
            .entity(entity)
            .insert(Name::new(BOSS_PROFILES[index].name));
        runtime.states.insert(
            entity,
            BossState {
                index: index as u8,
                age_secs: 0.0,
                casts_started: 0,
                pending: None,
                base_speed: enemy.speed,
                base_damage: enemy.damage,
                action_speed_multiplier: modifiers.attack_speed_multiplier,
                enrage_stacks: 0,
            },
        );
        if let Some(assets) = assets.as_deref() {
            spawn_ring_fx(
                &mut commands,
                &mut pools,
                assets,
                index,
                transform.translation,
                2.5 + index as f32 * 0.22,
                0.7,
            );
        }
    }
}

fn remove_defeated_bosses(living: Query<(), With<BossEnemy>>, mut runtime: ResMut<BossRuntime>) {
    runtime
        .states
        .retain(|entity, _| living.get(*entity).is_ok());
}

fn run_boss_mechanics(
    mut commands: Commands,
    time: Res<Time>,
    mut session: ResMut<GameSession>,
    assets: Option<Res<GameAssets>>,
    hero: Query<&Transform, (With<Hero>, Without<BossEnemy>)>,
    core: Query<&Transform, (With<Core>, Without<BossEnemy>)>,
    mut bosses: Query<(&BossEnemy, &mut Enemy, &mut Transform)>,
    mut runtime: ResMut<BossRuntime>,
    mut pools: ResMut<CombatPools>,
) {
    if session.paused || session.finished || session.save_active {
        return;
    }
    let Some(assets) = assets.as_deref() else {
        return;
    };
    let hero_position = hero.single().ok().map(|transform| transform.translation);
    let core_position = core.single().ok().map(|transform| transform.translation);
    let delta = time.delta_secs();

    for (entity, state) in &mut runtime.states {
        let Ok((boss, mut enemy, mut transform)) = bosses.get_mut(*entity) else {
            continue;
        };
        let index = usize::from(boss.index.min(4));
        if index != usize::from(state.index) {
            continue;
        }
        let profile = BOSS_PROFILES[index];
        state.age_secs += delta;

        if index == 4 {
            let desired_stacks = green_regent_enrage_stacks(state.age_secs);
            if desired_stacks > state.enrage_stacks {
                state.enrage_stacks = desired_stacks;
                enemy.damage = state.base_damage * green_regent_damage_multiplier(desired_stacks);
                enemy.speed = state.base_speed * (1.0 + desired_stacks as f32 * 0.06);
                spawn_ring_fx(
                    &mut commands,
                    &mut pools,
                    assets,
                    index,
                    transform.translation,
                    4.0 + desired_stacks as f32 * 0.22,
                    0.85,
                );
            }
        }

        if let Some(mut pending) = state.pending.take() {
            pending.remaining_secs -= delta;
            if pending.remaining_secs > 0.0 {
                state.pending = Some(pending);
                continue;
            }
            if !resolve_ability(
                &mut commands,
                &mut pools,
                assets,
                &mut session,
                state,
                &mut enemy,
                &mut transform,
                core_position,
                pending,
            ) {
                pending.remaining_secs = 0.0;
                state.pending = Some(pending);
            }
            continue;
        }

        let expected_casts = ability_cycles_elapsed(
            state.age_secs,
            profile.first_cast_secs,
            profile.period_secs / state.action_speed_multiplier.max(0.01),
        );
        if expected_casts <= state.casts_started {
            continue;
        }

        let variant = state.casts_started as u8;
        state.casts_started = state.casts_started.saturating_add(1);
        let target = warning_target(
            index,
            transform.translation,
            hero_position,
            core_position,
            variant,
        );
        state.pending = Some(PendingAbility {
            remaining_secs: profile.warning_secs,
            target,
            variant,
        });

        if index == 1 {
            enemy.speed = state.base_speed * 0.22;
        }
        let radius = warning_radius(index, variant);
        spawn_ring_fx(
            &mut commands,
            &mut pools,
            assets,
            index,
            target,
            radius,
            profile.warning_secs,
        );
        if index == 0 || index == 3 {
            spawn_ring_fx(
                &mut commands,
                &mut pools,
                assets,
                index,
                transform.translation,
                radius * 0.62,
                profile.warning_secs,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn resolve_ability(
    commands: &mut Commands,
    pools: &mut CombatPools,
    assets: &GameAssets,
    session: &mut GameSession,
    state: &mut BossState,
    enemy: &mut Enemy,
    transform: &mut Transform,
    core_position: Option<Vec3>,
    pending: PendingAbility,
) -> bool {
    let index = usize::from(state.index);
    match index {
        0 => {
            transform.translation = pending.target;
            transform.translation.y = 0.0;
            let hits_gate =
                core_position.is_some_and(|core| core.distance(transform.translation) <= 6.0);
            if hits_gate {
                session.core_hp = (session.core_hp - 32.0).max(0.0);
            }
            spawn_ring_fx(
                commands,
                pools,
                assets,
                index,
                transform.translation,
                3.6,
                0.5,
            );
        }
        1 => {
            enemy.speed = state.base_speed;
            let restored = enemy.max_hp * 0.08;
            enemy.hp = (enemy.hp + restored).min(enemy.max_hp);
            spawn_ring_fx(
                commands,
                pools,
                assets,
                index,
                transform.translation,
                5.0,
                0.75,
            );
        }
        2 => {
            transform.translation = pending.target;
            transform.translation.y = 0.0;
            if !spawn_pressure_wave(
                commands,
                pools,
                assets,
                transform.translation,
                3,
                3,
                session.elapsed,
            ) {
                return false;
            }
            spawn_ring_fx(
                commands,
                pools,
                assets,
                index,
                transform.translation,
                3.8,
                0.55,
            );
        }
        3 => {
            let solar_phase = pending.variant.is_multiple_of(2);
            if solar_phase {
                enemy.speed = state.base_speed * 1.45;
                enemy.damage = state.base_damage * 1.35;
                if core_position.is_some_and(|core| core.distance(transform.translation) <= 11.0) {
                    session.core_hp = (session.core_hp - 14.0).max(0.0);
                }
            } else {
                enemy.speed = state.base_speed * 0.72;
                enemy.damage = state.base_damage * 0.82;
                enemy.hp = (enemy.hp + enemy.max_hp * 0.035).min(enemy.max_hp);
            }
            spawn_ring_fx(
                commands,
                pools,
                assets,
                index,
                transform.translation,
                if solar_phase { 7.0 } else { 3.1 },
                0.7,
            );
        }
        4 => {
            if !spawn_pressure_wave(
                commands,
                pools,
                assets,
                transform.translation,
                5,
                1,
                session.elapsed,
            ) {
                return false;
            }
            spawn_ring_fx(
                commands,
                pools,
                assets,
                index,
                transform.translation,
                5.8,
                0.7,
            );
        }
        _ => unreachable!("boss index is clamped when registered"),
    }
    true
}

fn warning_target(
    index: usize,
    boss: Vec3,
    hero: Option<Vec3>,
    core: Option<Vec3>,
    variant: u8,
) -> Vec3 {
    match index {
        0 => {
            let gate_x = core.map_or(-14.8, |position| position.x) + 2.4;
            Vec3::new(
                (boss.x - 5.6).max(gate_x),
                0.0,
                hero.map_or(0.0, |position| position.z).clamp(-4.0, 4.0),
            )
        }
        2 => {
            let side = if variant.is_multiple_of(2) { 1.0 } else { -1.0 };
            Vec3::new((boss.x + 2.4).min(13.4), 0.0, side * 3.15)
        }
        _ => Vec3::new(boss.x, 0.0, boss.z),
    }
}

const fn warning_radius(index: usize, variant: u8) -> f32 {
    match index {
        0 => 3.6,
        1 => 4.8,
        2 => 3.2,
        3 => {
            if variant.is_multiple_of(2) {
                7.0
            } else {
                3.1
            }
        }
        4 => 5.6,
        _ => 3.0,
    }
}

fn spawn_ring_fx(
    commands: &mut Commands,
    pools: &mut CombatPools,
    assets: &GameAssets,
    boss_index: usize,
    center: Vec3,
    radius: f32,
    lifetime: f32,
) {
    pools.rent_impact_fx(
        commands,
        assets.fx_mesh.clone(),
        assets.boss_materials[boss_index.min(4)].clone(),
        Transform::from_xyz(center.x, 0.2, center.z).with_scale(Vec3::splat(0.18)),
        ImpactFx {
            lifetime,
            start_scale: 0.18,
            end_scale: radius,
        },
    );
}

fn spawn_pressure_wave(
    commands: &mut Commands,
    pools: &mut CombatPools,
    assets: &GameAssets,
    origin: Vec3,
    count: usize,
    scene_index: usize,
    elapsed: f32,
) -> bool {
    let growth = 1.0 + elapsed / 600.0;
    let hp = 115.0 * growth.powf(1.12);
    let scene_index = scene_index.min(3);
    let transforms: Vec<_> = (0..count)
        .map(|unit| {
            let centered = unit as f32 - (count as f32 - 1.0) * 0.5;
            Transform::from_xyz(
                (origin.x + 1.1 + unit as f32 * 0.18).min(15.6),
                0.0,
                (origin.z + centered * 1.35).clamp(-4.3, 4.3),
            )
            .with_scale(Vec3::splat(0.76))
        })
        .collect();
    let Some(entities) = pools.rent_enemy_scene_group(
        commands,
        EnemySceneKey::BossSummon(scene_index as u8),
        assets.enemy_scenes[scene_index].clone(),
        &transforms,
    ) else {
        return false;
    };
    for entity in entities {
        commands.entity(entity).insert((
            Enemy {
                hp,
                max_hp: hp,
                speed: 2.05 + count as f32 * 0.08,
                damage: 10.0 + elapsed / 180.0,
                xp: 0,
                wood: 0,
                gold: 0,
                kill_credit: 0,
                elite: false,
            },
            EnemyArchetype(if scene_index == 1 { 1 } else { 0 }),
            BossSummon,
        ));
    }
    true
}

fn ability_cycles_elapsed(age_secs: f32, first_cast_secs: f32, period_secs: f32) -> u32 {
    if age_secs < first_cast_secs || period_secs <= 0.0 {
        0
    } else {
        ((age_secs - first_cast_secs) / period_secs).floor() as u32 + 1
    }
}

fn green_regent_enrage_stacks(age_secs: f32) -> u8 {
    if age_secs < 90.0 {
        0
    } else {
        (1 + ((age_secs - 90.0) / 10.0).floor() as u32).min(u8::MAX as u32) as u8
    }
}

fn green_regent_damage_multiplier(stacks: u8) -> f32 {
    1.25_f32.powi(i32::from(stacks))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::archive::ArchiveEcho;

    fn test_game_assets() -> GameAssets {
        GameAssets {
            hero_scenes: std::array::from_fn(|_| Handle::default()),
            enemy_scenes: std::array::from_fn(|_| Handle::default()),
            boss_scenes: std::array::from_fn(|_| Handle::default()),
            summon_scenes: std::array::from_fn(|_| Handle::default()),
            enemy_mesh: Handle::default(),
            enemy_meshes: std::array::from_fn(|_| Handle::default()),
            elite_mesh: Handle::default(),
            projectile_mesh: Handle::default(),
            fx_mesh: Handle::default(),
            enemy_material: Handle::default(),
            enemy_materials: std::array::from_fn(|_| Handle::default()),
            elite_material: Handle::default(),
            boss_materials: std::array::from_fn(|_| Handle::default()),
            hero_materials: std::array::from_fn(|_| Handle::default()),
            projectile_material: Handle::default(),
            storm_material: Handle::default(),
        }
    }

    #[test]
    fn boss_pressure_wave_is_all_or_none_at_the_global_boundary() {
        let assets = test_game_assets();
        let mut world = World::new();
        let mut pools = CombatPools::default();
        {
            let mut commands = world.commands();
            pools
                .rent_enemy_scene_group(
                    &mut commands,
                    EnemySceneKey::Tower,
                    Handle::default(),
                    &vec![Transform::default(); 156],
                )
                .expect("pressure fixture should fit");
            assert!(!spawn_pressure_wave(
                &mut commands,
                &mut pools,
                &assets,
                Vec3::ZERO,
                5,
                1,
                1_500.0,
            ));
        }
        world.flush();
        assert_eq!(pools.enemy_metrics().active(), 156);
        assert_eq!(
            world
                .query_filtered::<Entity, With<BossSummon>>()
                .iter(&world)
                .count(),
            0
        );
    }

    #[test]
    fn profiles_are_distinct_and_match_existing_boss_order() {
        let names: Vec<&str> = BOSS_PROFILES.iter().map(|profile| profile.name).collect();
        assert_eq!(
            names,
            [
                "CINDER COLOSSUS",
                "ABYSSAL LEVIATHAN",
                "DREAD VEIL",
                "SUN CROWN MAGUS",
                "GREEN REGENT",
            ]
        );
        let periods: Vec<f32> = BOSS_PROFILES
            .iter()
            .map(|profile| profile.period_secs)
            .collect();
        assert_eq!(periods, [8.0, 10.0, 9.0, 6.0, 7.0]);
    }

    #[test]
    fn ability_cycles_have_exact_boundaries() {
        assert_eq!(ability_cycles_elapsed(2.999, 3.0, 8.0), 0);
        assert_eq!(ability_cycles_elapsed(3.0, 3.0, 8.0), 1);
        assert_eq!(ability_cycles_elapsed(10.999, 3.0, 8.0), 1);
        assert_eq!(ability_cycles_elapsed(11.0, 3.0, 8.0), 2);
        assert_eq!(ability_cycles_elapsed(43.0, 3.0, 8.0), 6);
    }

    #[test]
    fn public_boss_health_movement_and_action_modifiers_are_runtime_inputs() {
        let leviathan = BOSSES[1].modifiers;
        assert_eq!(28_000.0 * leviathan.max_hp_multiplier, 47_600.0);
        assert!((0.62 * leviathan.move_speed_multiplier - 0.372).abs() < 0.0001);

        let sun_crown = BOSSES[3].modifiers;
        let effective_period = BOSS_PROFILES[3].period_secs / sun_crown.attack_speed_multiplier;
        assert_eq!(effective_period, 4.0);
        assert_eq!(ability_cycles_elapsed(7.0, 3.0, effective_period), 2);
    }

    #[test]
    fn green_regent_enrages_at_ninety_then_every_ten_seconds() {
        assert_eq!(green_regent_enrage_stacks(89.999), 0);
        assert_eq!(green_regent_enrage_stacks(90.0), 1);
        assert_eq!(green_regent_enrage_stacks(99.999), 1);
        assert_eq!(green_regent_enrage_stacks(100.0), 2);
        assert_eq!(green_regent_enrage_stacks(130.0), 5);
        assert!((green_regent_damage_multiplier(1) - 1.25).abs() < 0.0001);
        assert!((green_regent_damage_multiplier(2) - 1.5625).abs() < 0.0001);
    }

    #[test]
    fn archive_echo_without_boss_marker_is_not_trackable() {
        let mut world = World::new();
        let echo = world.spawn(ArchiveEcho).id();
        let boss = world.spawn(BossEnemy { index: 4 }).id();
        let mut query = world.query_filtered::<Entity, With<BossEnemy>>();
        let tracked: Vec<Entity> = query.iter(&world).collect();
        assert_eq!(tracked, [boss]);
        assert!(!tracked.contains(&echo));
    }
}
