use super::bonds::BondRuntime;
use super::{EnemyAttackCue, EnemyAttackKind};
use bevy::prelude::*;

use super::combat::CombatSet;
use super::encounters::EncounterState;
use super::hero_abilities::TimedSpeedEffect;
use super::pooling::{CombatPools, EnemySceneKey};
use super::{
    ChallengeKind, CommandGrowth, Enemy, GameAssets, GameSession, GoldChallengeState, Hero,
    HeroVitals, PlayerBuild, TrialAction, TrialButton, TrialEnemy, TrialState,
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

pub(crate) const TOWER_FLOOR_COUNT: usize = 24;
const TOWER_HP: [f32; TOWER_FLOOR_COUNT] = [
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
/// Every floor shares one unlock time; floors gate each other through HP.
pub(crate) const TOWER_UNLOCK_AT: f32 = 18.0;
const TOWER_REWARD: [u32; TOWER_FLOOR_COUNT] = [
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
    assets: Option<Res<GameAssets>>,
    challenges: Query<(Entity, &TrialEnemy)>,
    interactions: Query<(&Interaction, &TrialButton), Changed<Interaction>>,
    mut pools: ResMut<CombatPools>,
    mut tower: ResMut<TrialState>,
    mut gold: ResMut<GoldChallengeState>,
    encounters: Res<EncounterState>,
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
        tower.auto = false;
        gold.auto = false;
        return;
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
    // Automatic starts use the same unlock/cooldown/capacity rules as manual input.
    // A quiet lane or a Boss must not silently disable the player's auto toggle.
    let bypass = false;

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
            &encounters,
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
                &encounters,
                assets,
                &mut gold,
                bypass,
            ),
            _ => {}
        }
    }

    if tower.auto && !tower.active {
        start_tower(
            &mut commands,
            &mut pools,
            &session,
            assets,
            &mut tower,
            bypass,
        );
    }
    if gold.auto && !gold.active {
        start_gold(
            &mut commands,
            &mut pools,
            &session,
            &encounters,
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
    encounters: &EncounterState,
    assets: &GameAssets,
    state: &mut GoldChallengeState,
    bypass_unlock: bool,
) {
    if !can_start_gold(session, state, bypass_unlock) {
        return;
    }
    // Treasure Map: richer targets that take longer to break.
    let total_hp = gold_hp(state.level) * encounters.gold_challenge_health_multiplier();
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
        encounters.gold_challenge_reward(gold_total_reward(state.level)),
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
        && index < TOWER_FLOOR_COUNT
        && (bypass_unlock || session.elapsed >= TOWER_UNLOCK_AT)
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
    mut rng: Option<ResMut<super::RunRng>>,
    time: Res<Time>,
    mut session: ResMut<GameSession>,
    mut vitals: ResMut<HeroVitals>,
    mut bonds: ResMut<BondRuntime>,
    mut cues: MessageWriter<EnemyAttackCue>,
    hero: Query<&Transform, (With<Hero>, Without<TrialEnemy>)>,
    core: Query<
        &Transform,
        (
            With<super::components::Core>,
            Without<Hero>,
            Without<TrialEnemy>,
        ),
    >,
    mut challenges: Query<
        (
            Entity,
            &Enemy,
            &mut ChallengeFighter,
            Option<&TimedSpeedEffect>,
            &mut Transform,
        ),
        (With<TrialEnemy>, Without<Hero>),
    >,
) {
    if session.paused || session.finished || session.save_active || !vitals.can_be_targeted() {
        return;
    }
    let Ok(hero) = hero.single() else {
        return;
    };
    for (entity, enemy, mut fighter, speed_effect, mut transform) in &mut challenges {
        let protecting_gate = core.single().ok().filter(|gate| {
            session.core_hp > 0.0
                && session.gate_revive_remaining <= 0.0
                && hero.translation.x < gate.translation.x
        });
        let target = if let Some(gate) = protecting_gate {
            transform.translation.x = transform.translation.x.max(gate.translation.x + 1.9);
            Vec3::new(gate.translation.x, 0.0, transform.translation.z)
        } else {
            hero.translation
        };
        let to_hero = (target - transform.translation).with_y(0.0);
        let (next, attack_seconds, reached) = super::combat::move_towards_radius(
            transform.translation,
            target,
            enemy.speed * speed_effect.map_or(1.0, |effect| effect.multiplier),
            time.delta_secs(),
            CHALLENGE_ATTACK_RANGE,
        );
        transform.translation = next;
        if reached {
            fighter.attack_remaining -= attack_seconds;
            while fighter.attack_remaining <= 0.0 && vitals.alive {
                fighter.attack_remaining += CHALLENGE_ATTACK_INTERVAL;
                if protecting_gate.is_some() && session.core_hp > 0.0 {
                    session.core_hp = (session.core_hp - enemy.damage).max(0.0);
                } else if protecting_gate.is_none()
                    && !rng
                        .as_deref_mut()
                        .is_some_and(|rng| bonds.evade(rng.unit()))
                {
                    bonds.take_physical_damage(&mut vitals, enemy.damage);
                }
                cues.write(EnemyAttackCue {
                    source: entity,
                    origin: transform.translation,
                    target,
                    kind: EnemyAttackKind::Slam,
                });
            }
        }
        if let Some(direction) = to_hero.try_normalize() {
            transform.look_to(direction, Vec3::Y);
        }
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
    #[test]
    fn real_attack_emits_cue_records_received_hit_and_holds_facing() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<GameSession>()
            .init_resource::<HeroVitals>()
            .init_resource::<BondRuntime>()
            .add_message::<EnemyAttackCue>()
            .add_systems(Update, move_challenge_enemies);

        app.world_mut()
            .spawn((Hero, Transform::from_xyz(-1., 0., 0.), Visibility::Visible));
        let enemy = app
            .world_mut()
            .spawn((
                TrialEnemy {
                    level: 1,
                    reward: 0,
                    kind: ChallengeKind::Tower,
                },
                ChallengeFighter {
                    attack_remaining: 0.,
                },
                Enemy {
                    hp: 1000.,
                    max_hp: 1000.,
                    speed: 0.,
                    damage: 10.,
                    xp: 0,
                    wood: 0,
                    gold: 0,
                    kill_credit: 0,
                    elite: false,
                },
                Transform::default(),
            ))
            .id();
        let before = app.world().resource::<HeroVitals>().health;
        app.update();
        assert!(app.world().resource::<HeroVitals>().health < before);
        assert_eq!(
            app.world_mut()
                .resource_mut::<BondRuntime>()
                .take_received_hits(),
            1
        );
        assert_eq!(app.world().resource::<Messages<EnemyAttackCue>>().len(), 1);
        let rotation = app.world().get::<Transform>(enemy).unwrap().rotation;
        app.update();
        assert!(
            rotation
                .angle_between(app.world().get::<Transform>(enemy).unwrap().rotation)
                .abs()
                < 0.001
        );
        let health = app.world().resource::<HeroVitals>().health;
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.update();
        assert_eq!(app.world().resource::<HeroVitals>().health, health);
    }

    fn test_game_assets() -> GameAssets {
        GameAssets {
            hero_scenes: std::array::from_fn(|_| Handle::default()),
            enemy_scenes: std::array::from_fn(|_| Handle::default()),
            boss_scenes: std::array::from_fn(|_| Handle::default()),
            summon_scenes: std::array::from_fn(|_| Handle::default()),
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

    fn auto_app(elapsed: f32) -> App {
        let mut app = App::new();
        app.insert_resource(GameSession {
            elapsed,
            ..default()
        })
        .insert_resource(test_game_assets())
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<Time>()
        .init_resource::<CombatPools>()
        .init_resource::<TrialState>()
        .init_resource::<GoldChallengeState>()
        .init_resource::<EncounterState>()
        .add_systems(Update, (challenge_input, update_challenges).chain());
        app
    }

    #[test]
    fn auto_buttons_start_at_unlock_without_dps_and_retry_after_timeout() {
        let mut app = auto_app(17.0);
        let tower_button = app
            .world_mut()
            .spawn((
                Interaction::Pressed,
                TrialButton(TrialAction::ToggleTowerAuto),
            ))
            .id();
        let gold_button = app
            .world_mut()
            .spawn((
                Interaction::Pressed,
                TrialButton(TrialAction::ToggleGoldAuto),
            ))
            .id();
        app.update();
        assert!(app.world().resource::<TrialState>().auto);
        assert!(app.world().resource::<GoldChallengeState>().auto);
        assert!(!app.world().resource::<TrialState>().active);
        for entity in [tower_button, gold_button] {
            *app.world_mut().get_mut::<Interaction>(entity).unwrap() = Interaction::None;
        }
        app.world_mut().resource_mut::<GameSession>().elapsed = 18.;
        app.update();
        assert!(app.world().resource::<TrialState>().active);
        assert!(!app.world().resource::<GoldChallengeState>().active);
        app.world_mut().resource_mut::<GameSession>().elapsed = 60.;
        app.update();
        assert!(app.world().resource::<GoldChallengeState>().active);
        assert_eq!(
            app.world()
                .resource::<GoldChallengeState>()
                .remaining_targets,
            10
        );
        assert_eq!(
            app.world()
                .resource::<CombatPools>()
                .enemy_metrics()
                .active(),
            11
        );
        app.update();
        assert_eq!(
            app.world()
                .resource::<CombatPools>()
                .enemy_metrics()
                .active(),
            11,
            "active challenges must not spawn twice"
        );
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(26));
        app.update();
        assert!(!app.world().resource::<TrialState>().active);
        assert!(!app.world().resource::<GoldChallengeState>().active);
        assert_eq!(
            app.world()
                .resource::<CombatPools>()
                .enemy_metrics()
                .active(),
            0
        );
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(34));
        app.update();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::ZERO);
        app.update();
        assert!(app.world().resource::<TrialState>().active);
        assert!(app.world().resource::<GoldChallengeState>().active);
        assert_eq!(
            app.world()
                .resource::<CombatPools>()
                .enemy_metrics()
                .active(),
            11
        );
    }

    #[test]
    fn auto_respects_pause_completion_and_explicit_clear_disables_repeat() {
        let mut app = auto_app(1510.);
        app.world_mut().resource_mut::<TrialState>().auto = true;
        app.world_mut().resource_mut::<GoldChallengeState>().auto = true;
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.update();
        assert!(!app.world().resource::<TrialState>().active);
        app.world_mut().resource_mut::<GameSession>().paused = false;
        app.update();
        assert!(app.world().resource::<TrialState>().active);
        assert!(app.world().resource::<GoldChallengeState>().active);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F4);
        app.update();
        assert!(!app.world().resource::<TrialState>().auto);
        assert!(!app.world().resource::<GoldChallengeState>().auto);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(61));
        app.update();
        app.update();
        assert!(!app.world().resource::<TrialState>().active);
        assert!(!app.world().resource::<GoldChallengeState>().active);
        {
            let mut tower = app.world_mut().resource_mut::<TrialState>();
            tower.auto = true;
            tower.level = 24;
        }
        {
            let mut gold = app.world_mut().resource_mut::<GoldChallengeState>();
            gold.auto = true;
            gold.level = 15;
        }
        app.update();
        assert!(!app.world().resource::<TrialState>().active);
        assert!(!app.world().resource::<GoldChallengeState>().active);
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
                &EncounterState::default(),
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
                &EncounterState::default(),
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
    fn chapter_six_scales_regular_enemies_but_not_tower_or_gold_challengers() {
        use super::super::campaign::{CampaignRun, chapter, scale_enemies};

        let mut app = auto_app(GOLD_UNLOCK_AT);
        app.insert_resource(CampaignRun {
            stage: 5,
            run_id: 0,
        })
        .add_systems(Update, scale_enemies.after(update_challenges));
        app.world_mut().resource_mut::<TrialState>().auto = true;
        app.world_mut().resource_mut::<GoldChallengeState>().auto = true;
        let regular = app
            .world_mut()
            .spawn(Enemy {
                hp: 100.0,
                max_hp: 100.0,
                speed: 1.0,
                damage: 10.0,
                xp: 0,
                wood: 0,
                gold: 0,
                kill_credit: 0,
                elite: false,
            })
            .id();
        app.update();
        app.update();

        let scaled = app.world().get::<Enemy>(regular).unwrap();
        assert_eq!(scaled.max_hp, 100.0 * chapter(5).health_scale);
        assert_eq!(scaled.damage, 10.0 * chapter(5).damage_scale);
        let challengers: Vec<_> = app
            .world_mut()
            .query::<(&Enemy, &TrialEnemy)>()
            .iter(app.world())
            .map(|(enemy, trial)| (trial.kind, enemy.hp, enemy.max_hp, enemy.damage))
            .collect();
        assert_eq!(challengers.len(), 1 + usize::from(GOLD_TARGET_COUNT));
        for (kind, hp, max_hp, damage) in challengers {
            let (expected_hp, expected_damage) = match kind {
                ChallengeKind::Tower => (1_200.0, 12.0),
                ChallengeKind::Gold => (120.0, 3.65),
            };
            assert_eq!(
                (hp, max_hp, damage),
                (expected_hp, expected_hp, expected_damage)
            );
        }
    }

    #[test]
    fn tower_table_matches_specification() {
        assert_eq!(TOWER_UNLOCK_AT, 18.0);
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
    fn every_tower_floor_obeys_the_shared_unlock_time() {
        let mut session = GameSession::default();
        let mut tower = TrialState::default();
        for index in 0..TOWER_FLOOR_COUNT {
            tower.level = index as u8;
            session.elapsed = TOWER_UNLOCK_AT - 0.01;
            assert!(!can_start_tower(&session, &tower, false));
            session.elapsed = TOWER_UNLOCK_AT;
            assert!(can_start_tower(&session, &tower, false));
        }
        tower.level = TOWER_FLOOR_COUNT as u8;
        assert!(!can_start_tower(&session, &tower, false));
        tower.level = 23;
        session.elapsed = 0.0;
        assert!(can_start_tower(&session, &tower, true));
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
