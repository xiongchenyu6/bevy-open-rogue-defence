use bevy::prelude::*;

use super::combat::CombatSet;
use super::pooling::{CombatPools, EnemySceneKey};
use super::{
    BossEnemy, ChallengeKind, CommandGrowth, DamageMeter, Enemy, GameAssets, GameSession,
    GoldChallengeState, Hero, HeroVitals, PlayerBuild, TrialAction, TrialButton, TrialEnemy,
    TrialState,
};

pub struct TrialPlugin;

impl Plugin for TrialPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                challenge_input,
                update_challenges,
                move_challenge_enemies,
                animate_challenge_auras,
            )
                .chain()
                .before(CombatSet),
        );
    }
}

const TOWER_HP: [f32; 24] = [
    1_200.0,
    1_900.0,
    2_800.0,
    4_400.0,
    8_400.0,
    9_400.0,
    13_000.0,
    18_000.0,
    25_000.0,
    43_000.0,
    48_000.0,
    64_000.0,
    86_000.0,
    120_000.0,
    216_000.0,
    300_000.0,
    420_000.0,
    580_000.0,
    800_000.0,
    1_100_000.0,
    1_500_000.0,
    2_000_000.0,
    2_800_000.0,
    3_800_000.0,
];
const TOWER_RECOMMENDED_DPS: [f32; 24] = [
    60.0, 95.0, 140.0, 220.0, 350.0, 470.0, 650.0, 900.0, 1_250.0, 1_800.0, 2_400.0, 3_200.0,
    4_300.0, 6_000.0, 9_000.0, 12_500.0, 17_500.0, 24_200.0, 33_300.0, 45_800.0, 62_500.0,
    83_300.0, 116_700.0, 158_300.0,
];
pub(crate) const TOWER_UNLOCK_AT: [f32; 24] = [18.0; 24];
const TOWER_REWARD: [u32; 24] = [
    50, 50, 55, 60, 65, 70, 75, 80, 85, 90, 100, 105, 110, 120, 125, 130, 135, 140, 145, 150, 150,
    150, 150, 150,
];
pub(crate) const GOLD_LEVEL_COUNT: usize = 15;
// A current N1 run visibly spawns ten gold targets and resets the button to
// 59 seconds. Start the cooldown with the challenge so the cadence is sixty
// seconds start-to-start, rather than 20 seconds plus another 60 seconds.
pub(crate) const GOLD_TARGET_COUNT: u8 = 10;
pub(crate) const GOLD_UNLOCK_AT: f32 = 60.0;
const TOWER_DURATION: f32 = 25.0;
const GOLD_DURATION: f32 = 20.0;
const TOWER_COOLDOWN: f32 = 8.0;
const GOLD_COOLDOWN: f32 = 60.0;
const TOWER_AUTO_MULTIPLIER: f32 = 1.15;
const GOLD_AUTO_MULTIPLIER: f32 = 1.20;
const BOSS_TIMES: [f32; 5] = [300.0, 600.0, 900.0, 1_200.0, 1_500.0];
const CHALLENGE_ATTACK_RANGE: f32 = 1.35;
const CHALLENGE_ATTACK_INTERVAL: f32 = 1.0;
const COMMAND_ATTACK_MULTIPLIER: f32 = 1.08;
const COMMAND_HEALTH_MULTIPLIER: f32 = 1.10;
const COMMAND_XP_BONUS: f32 = 0.10;

#[derive(Component)]
pub(crate) struct ChallengeFighter {
    attack_remaining: f32,
}

#[derive(Component)]
struct ChallengeAura {
    phase: f32,
    base_scale: f32,
}

#[allow(clippy::too_many_arguments)]
fn challenge_input(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    session: Res<GameSession>,
    damage_meter: Res<DamageMeter>,
    assets: Option<Res<GameAssets>>,
    bosses: Query<&Enemy, With<BossEnemy>>,
    challenges: Query<(Entity, &TrialEnemy)>,
    interactions: Query<(&Interaction, &TrialButton), Changed<Interaction>>,
    mut pools: ResMut<CombatPools>,
    mut tower: ResMut<TrialState>,
    mut gold: ResMut<GoldChallengeState>,
) {
    if session.finished || session.paused || session.save_active {
        return;
    }

    let cancel_pressed = keyboard.just_pressed(KeyCode::F4)
        || interactions.iter().any(|(interaction, button)| {
            *interaction == Interaction::Pressed && button.0 == TrialAction::CancelAll
        });
    if cancel_pressed {
        cancel_all(
            &mut commands,
            &mut pools,
            &challenges,
            &mut tower,
            &mut gold,
        );
    }
    for (interaction, button) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match button.0 {
            TrialAction::ToggleTowerAuto => tower.auto = !tower.auto,
            TrialAction::ToggleGoldAuto => gold.auto = !gold.auto,
            _ => {}
        }
    }

    let Some(assets) = assets.as_deref() else {
        return;
    };
    // Headless capture uses the same unlock, Boss-safety and DPS checks as a
    // player run; Autoplay supplies input but is not a balance cheat.
    let bypass = false;
    let no_living_boss = bosses.iter().all(|boss| boss.hp <= 0.0);
    let recent_dps = damage_meter.recent_dps();

    if keyboard.just_pressed(KeyCode::KeyQ) {
        start_tower(
            &mut commands,
            &mut pools,
            &session,
            assets,
            &mut tower,
            bypass,
        );
    }
    if keyboard.just_pressed(KeyCode::KeyW) {
        start_gold(
            &mut commands,
            &mut pools,
            &session,
            assets,
            &mut gold,
            bypass,
        );
    }
    for (interaction, button) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match button.0 {
            TrialAction::StartTower => start_tower(
                &mut commands,
                &mut pools,
                &session,
                assets,
                &mut tower,
                bypass,
            ),
            TrialAction::StartGold => start_gold(
                &mut commands,
                &mut pools,
                &session,
                assets,
                &mut gold,
                bypass,
            ),
            _ => {}
        }
    }

    let tower_recommended = TOWER_RECOMMENDED_DPS
        .get(tower.level as usize)
        .copied()
        .unwrap_or(f32::INFINITY);
    if tower.auto
        && !tower.active
        && tower_auto_allowed(
            recent_dps,
            tower_recommended,
            session.elapsed,
            no_living_boss,
            bypass,
        )
    {
        start_tower(
            &mut commands,
            &mut pools,
            &session,
            assets,
            &mut tower,
            bypass,
        );
    }
    if gold.auto
        && !gold.active
        && gold_auto_allowed(
            recent_dps,
            gold_recommended_dps(gold.level),
            no_living_boss,
            bypass,
        )
    {
        start_gold(
            &mut commands,
            &mut pools,
            &session,
            assets,
            &mut gold,
            bypass,
        );
    }
}

fn start_tower(
    commands: &mut Commands,
    pools: &mut CombatPools,
    session: &GameSession,
    assets: &GameAssets,
    state: &mut TrialState,
    bypass_unlock: bool,
) {
    if !can_start_tower(session, state, bypass_unlock) {
        return;
    }
    let index = state.level as usize;
    let Some(entity) = spawn_challenge(
        commands,
        pools,
        assets,
        ChallengeKind::Tower,
        state.level + 1,
        TOWER_HP[index],
        TOWER_REWARD[index],
        Vec3::new(15.0, 0.0, -4.6),
        0,
    ) else {
        return;
    };
    state.entity = Some(entity);
    state.active = true;
    state.remaining = TOWER_DURATION;
}

fn start_gold(
    commands: &mut Commands,
    pools: &mut CombatPools,
    session: &GameSession,
    assets: &GameAssets,
    state: &mut GoldChallengeState,
    bypass_unlock: bool,
) {
    if !can_start_gold(session, state, bypass_unlock) {
        return;
    }
    let total_hp = gold_hp(state.level);
    let hp_per_target = total_hp / f32::from(GOLD_TARGET_COUNT);
    let specs: Vec<_> = (0..GOLD_TARGET_COUNT)
        .map(|slot| {
            let row = slot / 4;
            let column = slot % 4;
            (
                Vec3::new(
                    14.6 + f32::from(row) * 0.55,
                    0.0,
                    2.65 + f32::from(column) * 0.72,
                ),
                slot,
            )
        })
        .collect();
    let Some(entities) = spawn_challenge_group(
        commands,
        pools,
        assets,
        ChallengeKind::Gold,
        state.level + 1,
        hp_per_target,
        gold_total_reward(state.level),
        &specs,
    ) else {
        return;
    };
    state.entity = entities.first().copied();
    state.active = true;
    state.remaining = GOLD_DURATION;
    state.cooldown = GOLD_COOLDOWN;
    state.remaining_targets = GOLD_TARGET_COUNT;
}

fn can_start_tower(session: &GameSession, state: &TrialState, bypass_unlock: bool) -> bool {
    let index = state.level as usize;
    !state.active
        && state.cooldown <= 0.0
        && index < TOWER_HP.len()
        && (bypass_unlock || session.elapsed >= TOWER_UNLOCK_AT[index])
}

fn can_start_gold(session: &GameSession, state: &GoldChallengeState, bypass_unlock: bool) -> bool {
    !state.active
        && state.cooldown <= 0.0
        && (state.level as usize) < GOLD_LEVEL_COUNT
        && (bypass_unlock || session.elapsed >= GOLD_UNLOCK_AT)
}

fn gold_hp(level: u8) -> f32 {
    1_200.0 * 1.55_f32.powi(level as i32)
}

fn gold_reward(level: u8) -> u32 {
    // Interpolate the observed Lv1 baseline to the Lv15 sample (630 gold per
    // target) while keeping every intermediate level deterministic.
    100 + 530 * u32::from(level) / 14
}

fn gold_total_reward(level: u8) -> u32 {
    gold_reward(level).saturating_mul(u32::from(GOLD_TARGET_COUNT))
}

fn gold_recommended_dps(level: u8) -> f32 {
    gold_hp(level) / 18.0
}

fn tower_auto_allowed(
    recent_dps: f32,
    recommended_dps: f32,
    elapsed: f32,
    no_living_boss: bool,
    bypass: bool,
) -> bool {
    bypass
        || (no_living_boss
            && recent_dps > recommended_dps * TOWER_AUTO_MULTIPLIER
            && next_boss_is_more_than_twenty_seconds_away(elapsed))
}

fn gold_auto_allowed(
    recent_dps: f32,
    recommended_dps: f32,
    no_living_boss: bool,
    bypass: bool,
) -> bool {
    bypass || (no_living_boss && recent_dps > recommended_dps * GOLD_AUTO_MULTIPLIER)
}

fn next_boss_is_more_than_twenty_seconds_away(elapsed: f32) -> bool {
    BOSS_TIMES
        .into_iter()
        .find(|boss_time| elapsed <= *boss_time)
        .is_some_and(|boss_time| boss_time - elapsed > 20.0)
}

fn spawn_challenge(
    commands: &mut Commands,
    pools: &mut CombatPools,
    assets: &GameAssets,
    kind: ChallengeKind,
    level: u8,
    hp: f32,
    reward: u32,
    position: Vec3,
    slot: u8,
) -> Option<Entity> {
    spawn_challenge_group(
        commands,
        pools,
        assets,
        kind,
        level,
        hp,
        reward,
        &[(position, slot)],
    )
    .and_then(|entities| entities.into_iter().next())
}

#[allow(clippy::too_many_arguments)]
fn spawn_challenge_group(
    commands: &mut Commands,
    pools: &mut CombatPools,
    assets: &GameAssets,
    kind: ChallengeKind,
    level: u8,
    hp: f32,
    reward: u32,
    specs: &[(Vec3, u8)],
) -> Option<Vec<Entity>> {
    let (key, scene, material, scale, speed, damage) = match kind {
        ChallengeKind::Tower => (
            EnemySceneKey::Tower,
            assets.boss_scenes[2].clone(),
            assets.storm_material.clone(),
            0.78,
            2.65,
            10.0 + f32::from(level) * 2.0,
        ),
        ChallengeKind::Gold => (
            EnemySceneKey::Gold,
            assets.enemy_scenes[4].clone(),
            assets.enemy_materials[1].clone(),
            0.56,
            3.05,
            3.0 + f32::from(level) * 0.65,
        ),
    };
    let transforms: Vec<_> = specs
        .iter()
        .map(|(position, _)| Transform::from_translation(*position).with_scale(Vec3::splat(scale)))
        .collect();
    let entities = pools.rent_enemy_scene_group(commands, key, scene, &transforms)?;
    for (entity, (_, slot)) in entities.iter().copied().zip(specs.iter().copied()) {
        commands.entity(entity).insert((
            Name::new(challenge_name(kind, level, slot)),
            Enemy {
                hp,
                max_hp: hp,
                speed,
                damage,
                xp: 0,
                wood: 0,
                gold: 0,
                kill_credit: 0,
                elite: kind == ChallengeKind::Tower,
            },
            TrialEnemy {
                level,
                reward,
                kind,
            },
            ChallengeFighter {
                attack_remaining: 0.45 + f32::from(slot) * 0.04,
            },
        ));
        let decoration = commands
            .spawn((
                Name::new(match kind {
                    ChallengeKind::Tower => "CYAN TOWER OUTLINE",
                    ChallengeKind::Gold => "GOLDEN ECHO OUTLINE",
                }),
                Mesh3d(assets.fx_mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_xyz(0.0, 1.15, 0.0).with_scale(Vec3::splat(0.28)),
                ChallengeAura {
                    phase: f32::from(slot) * 0.55,
                    base_scale: if kind == ChallengeKind::Tower {
                        0.32
                    } else {
                        0.23
                    },
                },
            ))
            .id();
        commands.entity(entity).add_child(decoration);
        pools.track_enemy_decoration(entity, decoration);
    }
    Some(entities)
}

fn challenge_name(kind: ChallengeKind, level: u8, slot: u8) -> String {
    match kind {
        ChallengeKind::Tower => format!("TOWER CHALLENGER · FLOOR {level}"),
        ChallengeKind::Gold => format!("GOLDEN ECHO · {}/{}", slot + 1, GOLD_TARGET_COUNT),
    }
}

fn cancel_all(
    commands: &mut Commands,
    pools: &mut CombatPools,
    challenges: &Query<(Entity, &TrialEnemy)>,
    tower: &mut TrialState,
    gold: &mut GoldChallengeState,
) {
    for (entity, _) in challenges {
        pools.retire_enemy(commands, entity);
    }
    abandon_challenge_states(tower, gold);
}

fn abandon_challenge_states(tower: &mut TrialState, gold: &mut GoldChallengeState) {
    if tower.active {
        tower.cooldown = TOWER_COOLDOWN;
    }
    tower.active = false;
    tower.remaining = 0.0;
    tower.entity = None;
    gold.active = false;
    gold.remaining = 0.0;
    gold.entity = None;
    gold.remaining_targets = 0;
}

fn update_challenges(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    challenges: Query<(Entity, &TrialEnemy)>,
    mut tower: ResMut<TrialState>,
    mut gold: ResMut<GoldChallengeState>,
    mut pools: ResMut<CombatPools>,
) {
    if session.finished {
        cancel_all(
            &mut commands,
            &mut pools,
            &challenges,
            &mut tower,
            &mut gold,
        );
        return;
    }
    if session.paused {
        return;
    }
    tower.cooldown = (tower.cooldown - time.delta_secs()).max(0.0);
    gold.cooldown = (gold.cooldown - time.delta_secs()).max(0.0);

    if tower.active {
        tower.remaining -= time.delta_secs();
        if tower.remaining <= 0.0 {
            retire_challenge_kind(&mut commands, &mut pools, &challenges, ChallengeKind::Tower);
            tower.active = false;
            tower.remaining = 0.0;
            tower.cooldown = TOWER_COOLDOWN;
            tower.entity = None;
        }
    }
    if gold.active {
        gold.remaining -= time.delta_secs();
        if gold.remaining <= 0.0 {
            retire_challenge_kind(&mut commands, &mut pools, &challenges, ChallengeKind::Gold);
            gold.active = false;
            gold.remaining = 0.0;
            gold.entity = None;
            gold.remaining_targets = 0;
        }
    }
}

fn retire_challenge_kind(
    commands: &mut Commands,
    pools: &mut CombatPools,
    challenges: &Query<(Entity, &TrialEnemy)>,
    kind: ChallengeKind,
) {
    for (entity, challenge) in challenges {
        if challenge.kind == kind {
            pools.retire_enemy(commands, entity);
        }
    }
}

fn move_challenge_enemies(
    time: Res<Time>,
    session: Res<GameSession>,
    mut vitals: ResMut<HeroVitals>,
    hero: Query<&Transform, (With<Hero>, Without<TrialEnemy>)>,
    mut challenges: Query<
        (&Enemy, &mut ChallengeFighter, &mut Transform),
        (With<TrialEnemy>, Without<Hero>),
    >,
) {
    if session.paused || session.finished || session.save_active || !vitals.can_be_targeted() {
        return;
    }
    let Ok(hero) = hero.single() else {
        return;
    };
    for (enemy, mut fighter, mut transform) in &mut challenges {
        let to_hero = Vec3::new(
            hero.translation.x - transform.translation.x,
            0.0,
            hero.translation.z - transform.translation.z,
        );
        if to_hero.length_squared() > CHALLENGE_ATTACK_RANGE * CHALLENGE_ATTACK_RANGE {
            transform.translation += to_hero.normalize_or_zero() * enemy.speed * time.delta_secs();
        } else {
            fighter.attack_remaining -= time.delta_secs();
            while fighter.attack_remaining <= 0.0 && vitals.alive {
                fighter.attack_remaining += CHALLENGE_ATTACK_INTERVAL;
                vitals.take_physical_damage(enemy.damage);
            }
        }
        transform.rotate_y(time.delta_secs() * 1.4);
    }
}

fn animate_challenge_auras(time: Res<Time>, mut auras: Query<(&ChallengeAura, &mut Transform)>) {
    let elapsed = time.elapsed_secs();
    for (aura, mut transform) in &mut auras {
        let pulse = 1.0 + (elapsed * 4.0 + aura.phase).sin() * 0.18;
        transform.scale = Vec3::splat(aura.base_scale * pulse);
        transform.rotate_y(time.delta_secs() * 2.8);
    }
}

pub(crate) fn complete_tower_challenge(
    level: u8,
    reward: u32,
    session: &mut GameSession,
    state: &mut TrialState,
    build: &mut PlayerBuild,
    vitals: &mut HeroVitals,
) -> bool {
    if !state.active || level != state.level.saturating_add(1) {
        return false;
    }
    session.wood = session.wood.saturating_add(reward);
    let growth = CommandGrowth::from_point(state.command_points);
    state.command_points = state.command_points.saturating_add(1);
    state.growth_ranks[growth.index()] = state.growth_ranks[growth.index()].saturating_add(1);
    state.last_growth = Some(growth);
    match growth {
        CommandGrowth::Attack => build.attack_damage *= COMMAND_ATTACK_MULTIPLIER,
        CommandGrowth::Vitality => {
            let previous = vitals.max_health;
            vitals.command_health_multiplier *= COMMAND_HEALTH_MULTIPLIER;
            vitals.max_health *= COMMAND_HEALTH_MULTIPLIER;
            if vitals.alive {
                vitals.health =
                    (vitals.health + vitals.max_health - previous).min(vitals.max_health);
            }
        }
        CommandGrowth::Insight => session.xp_multiplier += COMMAND_XP_BONUS,
    }
    state.level = level;
    state.active = false;
    state.remaining = 0.0;
    state.cooldown = TOWER_COOLDOWN;
    state.entity = None;
    true
}

pub(crate) fn complete_gold_target(
    level: u8,
    reward: u32,
    session: &mut GameSession,
    state: &mut GoldChallengeState,
) -> bool {
    if !state.active || level != state.level.saturating_add(1) || state.remaining_targets == 0 {
        return false;
    }
    state.remaining_targets -= 1;
    if state.remaining_targets > 0 {
        return false;
    }
    session.gold = session.gold.saturating_add(reward);
    state.level = level;
    state.active = false;
    state.remaining = 0.0;
    state.entity = None;
    true
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn gold_group_commits_zero_or_all_ten_targets() {
        let assets = test_game_assets();
        let session = GameSession {
            elapsed: GOLD_UNLOCK_AT,
            ..default()
        };

        let mut blocked_world = World::new();
        let mut blocked_pool = CombatPools::default();
        let mut blocked_state = GoldChallengeState::default();
        {
            let mut commands = blocked_world.commands();
            blocked_pool
                .rent_enemy_scene_group(
                    &mut commands,
                    EnemySceneKey::BossSummon(0),
                    Handle::default(),
                    &vec![Transform::default(); 155],
                )
                .expect("pressure fixture should fit");
            start_gold(
                &mut commands,
                &mut blocked_pool,
                &session,
                &assets,
                &mut blocked_state,
                false,
            );
        }
        blocked_world.flush();
        assert!(!blocked_state.active);
        assert_eq!(blocked_state.remaining_targets, 0);
        assert_eq!(blocked_pool.enemy_metrics().active(), 155);
        assert_eq!(
            blocked_world
                .query_filtered::<Entity, With<TrialEnemy>>()
                .iter(&blocked_world)
                .count(),
            0
        );

        let mut exact_world = World::new();
        let mut exact_pool = CombatPools::default();
        let mut exact_state = GoldChallengeState::default();
        {
            let mut commands = exact_world.commands();
            exact_pool
                .rent_enemy_scene_group(
                    &mut commands,
                    EnemySceneKey::BossSummon(0),
                    Handle::default(),
                    &vec![Transform::default(); 150],
                )
                .expect("exact-fit fixture should fit");
            start_gold(
                &mut commands,
                &mut exact_pool,
                &session,
                &assets,
                &mut exact_state,
                false,
            );
        }
        exact_world.flush();
        assert!(exact_state.active);
        assert_eq!(exact_state.remaining_targets, GOLD_TARGET_COUNT);
        assert_eq!(exact_pool.enemy_metrics().active(), 160);
        assert_eq!(
            exact_world
                .query_filtered::<Entity, With<TrialEnemy>>()
                .iter(&exact_world)
                .count(),
            usize::from(GOLD_TARGET_COUNT)
        );
    }

    #[test]
    fn tower_table_matches_specification() {
        assert_eq!(TOWER_UNLOCK_AT, [18.0; 24]);
        assert_eq!(
            TOWER_HP,
            [
                1_200.0,
                1_900.0,
                2_800.0,
                4_400.0,
                8_400.0,
                9_400.0,
                13_000.0,
                18_000.0,
                25_000.0,
                43_000.0,
                48_000.0,
                64_000.0,
                86_000.0,
                120_000.0,
                216_000.0,
                300_000.0,
                420_000.0,
                580_000.0,
                800_000.0,
                1_100_000.0,
                1_500_000.0,
                2_000_000.0,
                2_800_000.0,
                3_800_000.0,
            ]
        );
        assert_eq!(
            TOWER_REWARD,
            [
                50, 50, 55, 60, 65, 70, 75, 80, 85, 90, 100, 105, 110, 120, 125, 130, 135, 140,
                145, 150, 150, 150, 150, 150,
            ]
        );
        assert_eq!(TOWER_DURATION, 25.0);
        assert_eq!(TOWER_COOLDOWN, 8.0);
        assert_eq!(TOWER_REWARD[22], 150);
    }

    #[test]
    fn each_tower_floor_obeys_its_own_unlock_time() {
        let mut session = GameSession::default();
        let mut tower = TrialState::default();
        for (index, unlock) in TOWER_UNLOCK_AT.into_iter().enumerate() {
            tower.level = index as u8;
            session.elapsed = unlock - 0.01;
            assert!(!can_start_tower(&session, &tower, false));
            session.elapsed = unlock;
            assert!(can_start_tower(&session, &tower, false));
        }
        tower.level = TOWER_HP.len() as u8;
        assert!(!can_start_tower(&session, &tower, false));
        tower.level = 23;
        session.elapsed = 0.0;
        assert!(can_start_tower(&session, &tower, true));
    }

    #[test]
    fn auto_thresholds_are_strict_and_boss_rules_differ() {
        let tower_required = TOWER_RECOMMENDED_DPS[0] * 1.15;
        assert!(!tower_auto_allowed(
            tower_required,
            60.0,
            100.0,
            true,
            false
        ));
        assert!(tower_auto_allowed(
            tower_required + 0.01,
            60.0,
            100.0,
            true,
            false
        ));
        assert!(!tower_auto_allowed(999.0, 60.0, 280.0, true, false));
        assert!(!tower_auto_allowed(999.0, 60.0, 100.0, false, false));

        let gold_required = gold_recommended_dps(0) * 1.20;
        assert!(!gold_auto_allowed(
            gold_required,
            gold_recommended_dps(0),
            true,
            false
        ));
        assert!(gold_auto_allowed(
            gold_required + 0.01,
            gold_recommended_dps(0),
            true,
            false
        ));
        assert!(!gold_auto_allowed(999.0, 1.0, false, false));
        assert!(tower_auto_allowed(0.0, f32::INFINITY, 299.0, false, true));
        assert!(gold_auto_allowed(0.0, f32::INFINITY, false, true));
    }

    #[test]
    fn gold_challenge_uses_ten_targets_and_rewards_only_the_last() {
        let mut session = GameSession::default();
        let starting_gold = session.gold;
        let mut gold = GoldChallengeState {
            active: true,
            remaining_targets: GOLD_TARGET_COUNT,
            cooldown: 42.0,
            ..default()
        };
        for remaining_after in (1..GOLD_TARGET_COUNT).rev() {
            assert!(!complete_gold_target(
                1,
                gold_total_reward(0),
                &mut session,
                &mut gold
            ));
            assert_eq!(gold.remaining_targets, remaining_after);
            assert_eq!(session.gold, starting_gold);
        }
        assert!(complete_gold_target(
            1,
            gold_total_reward(0),
            &mut session,
            &mut gold
        ));
        assert_eq!(gold.remaining_targets, 0);
        assert_eq!(gold.level, 1);
        assert_eq!(gold.cooldown, 42.0);
        assert_eq!(session.gold, starting_gold + 1_000);
        assert!(!complete_gold_target(1, 100, &mut session, &mut gold));
        assert_eq!(session.gold, starting_gold + 1_000);
    }

    #[test]
    fn f4_abandons_both_groups_without_touching_rewards() {
        let session = GameSession::default();
        let rewards = (session.gold, session.wood);
        let mut tower = TrialState {
            active: true,
            remaining: 12.0,
            ..default()
        };
        let mut gold = GoldChallengeState {
            active: true,
            remaining: 9.0,
            remaining_targets: 5,
            cooldown: 51.0,
            ..default()
        };
        abandon_challenge_states(&mut tower, &mut gold);
        assert!(!tower.active && !gold.active);
        assert_eq!(tower.remaining, 0.0);
        assert_eq!(gold.remaining_targets, 0);
        assert_eq!(tower.cooldown, TOWER_COOLDOWN);
        assert_eq!(gold.cooldown, 51.0);
        assert_eq!((session.gold, session.wood), rewards);
    }

    #[test]
    fn tower_points_rotate_through_three_consumed_growths() {
        let mut session = GameSession::default();
        let starting_wood = session.wood;
        let mut tower = TrialState::default();
        let mut build = PlayerBuild::default();
        let mut vitals = HeroVitals::default();
        let attack = build.attack_damage;
        let max_health = vitals.max_health;

        for level in 1..=3 {
            tower.active = true;
            assert!(complete_tower_challenge(
                level,
                8,
                &mut session,
                &mut tower,
                &mut build,
                &mut vitals,
            ));
        }
        assert_eq!(tower.command_points, 3);
        assert_eq!(tower.growth_ranks, [1, 1, 1]);
        assert!(build.attack_damage > attack);
        assert!(vitals.max_health > max_health);
        let command_health = vitals.max_health;
        vitals.synchronize_level(vitals.kind, 2);
        assert!(vitals.max_health > command_health);
        assert!(vitals.command_health_multiplier > 1.0);
        assert_eq!(session.xp_multiplier, 1.10);
        assert_eq!(session.wood, starting_wood + 24);
    }

    #[test]
    fn gold_formula_and_timeout_values_match_specification() {
        for level in 0..GOLD_LEVEL_COUNT as u8 {
            let expected_hp = 1_200.0 * 1.55_f32.powi(level as i32);
            assert!((gold_hp(level) - expected_hp).abs() < 0.01);
            assert_eq!(gold_reward(level), 100 + 530 * u32::from(level) / 14);
            assert!((gold_recommended_dps(level) - expected_hp / 18.0).abs() < 0.01);
        }
        assert_eq!(GOLD_TARGET_COUNT, 10);
        assert_eq!(GOLD_DURATION, 20.0);
        assert_eq!(GOLD_COOLDOWN, 60.0);
        assert_eq!(GOLD_COOLDOWN - GOLD_DURATION, 40.0);
        assert_eq!(
            challenge_name(ChallengeKind::Gold, 15, GOLD_TARGET_COUNT - 1),
            "GOLDEN ECHO · 10/10"
        );
        assert_eq!(gold_reward(14), 630);
        assert_eq!(gold_total_reward(14), 6_300);
    }
}
