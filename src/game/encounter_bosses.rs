//! Optional battles use the real boss AI without advancing campaign checkpoints.
use super::bosses::{BossSummon, BossTelegraph};
use super::combat::{CombatSet, EnemyArmor, EnemyBreachDamage};
use super::content::BOSSES;
use super::pooling::{CombatPools, EnemySceneKey};
use super::{BossEnemy, DraftState, Enemy, GameAssets, GameSession, RecruitmentProgress};
use bevy::prelude::*;

pub const ADVENTURE_OFFER_TIMES: [f32; 3] = [180.0, 480.0, 780.0];
pub const ADVENTURE_SITES: [Vec3; 3] = [
    Vec3::new(-3.0, 0.0, -9.0),
    Vec3::new(5.0, 0.0, 9.0),
    Vec3::new(8.0, 0.0, -9.0),
];
pub const ADVENTURE_LEADER_VISUALS: [usize; 3] = [1, 4, 10];
pub const ADVENTURE_LANDING_SITES: [Vec3; 3] = [
    Vec3::new(-3.0, 0.0, -3.5),
    Vec3::new(5.0, 0.0, 3.5),
    Vec3::new(8.0, 0.0, -3.5),
];
pub const CAMP_ARRIVAL_SECONDS: f32 = 1.0;
pub const ADVENTURE_GUARD_VISUALS: [usize; 3] = [0, 3, 9];
pub const ADVENTURE_BATTLE_SECONDS: f32 = 90.0;
#[derive(Clone, Copy)]
pub struct AdventureAbility {
    pub name_en: &'static str,
    pub name_zh: &'static str,
    pub description_en: &'static str,
    pub description_zh: &'static str,
}
#[derive(Clone, Copy)]
pub struct AdventureBossDef {
    pub name_en: &'static str,
    pub name_zh: &'static str,
    pub icon_path: &'static str,
    pub boss_index: u8,
    pub hp: f32,
    pub damage: f32,
    pub reward_xp: u32,
    pub reward_gold: u32,
    pub reward_wood: u32,
    pub small_gold: u32,
    pub small_wood: u32,
    pub abilities: [AdventureAbility; 3],
}
const PHASE: AdventureAbility = AdventureAbility {
    name_en: "Desperate fury",
    name_zh: "半血狂暴",
    description_en: "Below 50% HP: casts 30% faster, areas 25% wider, hazard damage rises from 120% to 160% of attack.",
    description_zh: "生命低于50%：施法频率提高30%，范围扩大25%，危险区域伤害由攻击力120%提高至160%。",
};
pub const ADVENTURE_BOSSES: [AdventureBossDef; 3] = [
    AdventureBossDef {
        name_en: "Embermaw Alpha",
        name_zh: "炎牙兽王",
        icon_path: "icons/cinder_aura.png",
        boss_index: 0,
        hp: 5000.0,
        damage: 18.0,
        reward_xp: 400,
        reward_gold: 600,
        reward_wood: 100,
        small_gold: 90,
        small_wood: 15,
        abilities: [
            AdventureAbility {
                name_en: "Cinder charge",
                name_zh: "炽烬冲锋",
                description_en: "Alternates every 8s; 0.9s warning. Physical lane: 7m long, 2.5m wide; 120% attack. A standing gate blocks the charge.",
                description_zh: "每8秒交替施放，预警0.9秒。物理冲锋长7米、宽2.5米，造成攻击力120%伤害。完整城门能挡住冲锋。",
            },
            AdventureAbility {
                name_en: "Lava scar",
                name_zh: "熔岩裂痕",
                description_en: "Alternating cast: 2.6m radius lava for 3s; magical 120% attack every 0.5s.",
                description_zh: "交替施放：半径2.6米熔岩持续3秒，每0.5秒造成攻击力120%法术伤害。",
            },
            PHASE,
        ],
    },
    AdventureBossDef {
        name_en: "Tidehorn Matriarch",
        name_zh: "潮角巨兽",
        icon_path: "icons/tide_elemental.png",
        boss_index: 1,
        hp: 18000.0,
        damage: 30.0,
        reward_xp: 700,
        reward_gold: 1200,
        reward_wood: 180,
        small_gold: 180,
        small_wood: 25,
        abilities: [
            AdventureAbility {
                name_en: "Tidal crush",
                name_zh: "怒潮冲击",
                description_en: "Alternates every 10s; 1.1s warning. Magical wave: 16m long, 4.4m wide; 120% attack. Heals 8% maximum HP per cast.",
                description_zh: "每10秒交替施放，预警1.1秒。水波长16米、宽4.4米，造成攻击力120%法术伤害；每次施法恢复8%最大生命。",
            },
            AdventureAbility {
                name_en: "Undertow",
                name_zh: "深海暗流",
                description_en: "Alternating cast: 3.4m radius for 2s; magical 120% attack every 0.5s, stun 0.45s on damage.",
                description_zh: "交替施放：半径3.4米暗流持续2秒，每0.5秒造成攻击力120%法术伤害；造成伤害时眩晕0.45秒。",
            },
            PHASE,
        ],
    },
    AdventureBossDef {
        name_en: "Plaguejaw Elder",
        name_zh: "疫颚长老",
        icon_path: "icons/forbidden_arts.png",
        boss_index: 4,
        hp: 45000.0,
        damage: 45.0,
        reward_xp: 1000,
        reward_gold: 2000,
        reward_wood: 300,
        small_gold: 300,
        small_wood: 40,
        abilities: [
            AdventureAbility {
                name_en: "Thorn host",
                name_zh: "荆棘军团",
                description_en: "Every 7s; 1s warning. Summons 5 attackers, 10 in phase II; at most 12 living minions. Minions grant no loot.",
                description_zh: "每7秒施放，预警1秒。召唤5只进攻仆从，二阶段10只；最多12只存活。仆从没有掉落。",
            },
            AdventureAbility {
                name_en: "Venom garden",
                name_zh: "剧毒花园",
                description_en: "Every cast leaves a 2.8m radius poison field for 4s; magical 120% attack every 0.5s.",
                description_zh: "每次施法留下半径2.8米毒地，持续4秒，每0.5秒造成攻击力120%法术伤害。",
            },
            PHASE,
        ],
    },
];
#[derive(Component, Debug, Clone, Copy)]
pub struct EncounterBoss {
    pub index: u8,
}
#[derive(Component, Debug, Clone, Copy)]
pub struct EncounterGuard {
    pub owner: Entity,
}
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct CampCombatant {
    pub center: Vec3,
}
/// No Enemy component exists in flight, so all real damage/targeting ignores arrivals.
#[derive(Component)]
pub struct CampArrival {
    pub start: Vec3,
    pub end: Vec3,
    pub remaining_secs: f32,
    enemy: Option<Enemy>,
    boss_index: Option<u8>,
}
fn arrival_position(start: Vec3, end: Vec3, progress: f32) -> Vec3 {
    let progress = progress.clamp(0.0, 1.0);
    start.lerp(end, progress) + Vec3::Y * (4.0 * progress * (1.0 - progress) * 2.8)
}
fn advance_arrivals(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    mut arrivals: Query<(Entity, &mut Transform, &mut CampArrival)>,
) {
    if session.paused || session.finished || session.save_active {
        return;
    }
    for (entity, mut transform, mut arrival) in &mut arrivals {
        arrival.remaining_secs = (arrival.remaining_secs - time.delta_secs()).max(0.0);
        transform.translation = arrival_position(
            arrival.start,
            arrival.end,
            1.0 - arrival.remaining_secs / CAMP_ARRIVAL_SECONDS,
        );
        if arrival.remaining_secs <= 0.0 {
            if let Some(enemy) = arrival.enemy.take() {
                commands.entity(entity).insert(enemy);
            }
            if let Some(index) = arrival.boss_index {
                commands.entity(entity).insert(BossEnemy { index });
            }
            commands.entity(entity).remove::<CampArrival>();
        }
    }
}
#[derive(Message, Debug, Clone, Copy)]
pub struct AdventureCampRequest(pub u8);
#[derive(Resource, Default)]
pub struct AdventureBossState {
    pub pending: Option<u8>,
    pub active: Option<Entity>,
    pub remaining_secs: f32,
    pub revision: u64,
    pub completed: u8,
    pub unlocked: u8,
    pub resolved: u8,
    boss_defeated: bool,
    pub active_index: u8,
}
#[derive(Message, Debug, Clone, Copy)]
pub enum AdventureBossChoice {
    Fight,
    TakeReward,
}
pub struct EncounterBossesPlugin;
impl Plugin for EncounterBossesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AdventureBossState>()
            .add_message::<AdventureBossChoice>()
            .add_message::<AdventureCampRequest>()
            .add_systems(Update, advance_arrivals.before(CombatSet))
            .add_systems(
                Update,
                (tick_adventure, choose_adventure).chain().after(CombatSet),
            );
    }
}
fn can_offer(session: &GameSession) -> bool {
    !session.paused && !session.finished && !session.save_active
}
fn tick_adventure(
    mut commands: Commands,
    time: Res<Time>,
    mut session: ResMut<GameSession>,
    mut state: ResMut<AdventureBossState>,
    draft: Option<Res<DraftState>>,
    recruitment: Option<Res<RecruitmentProgress>>,
    bosses: Query<&Enemy, With<BossEnemy>>,
    dangers: Query<(Entity, &BossTelegraph)>,
    summons: Query<(Entity, &BossSummon)>,
    guards: Query<(Entity, &EncounterGuard, Option<&Enemy>)>,
    arrivals: Query<(), With<CampArrival>>,
    mut requests: MessageReader<AdventureCampRequest>,
    mut pools: ResMut<CombatPools>,
) {
    let incoming: Vec<u8> = requests.read().map(|request| request.0).collect();
    for (index, due) in ADVENTURE_OFFER_TIMES.iter().enumerate() {
        if session.elapsed >= *due {
            state.unlocked |= 1 << index;
        }
    }
    if let Some(active) = state.active {
        if state.boss_defeated
            && !guards.iter().any(|(_, guard, enemy)| {
                guard.owner == active && enemy.is_none_or(|enemy| enemy.hp > 0.0)
            })
            && !session.finished
        {
            cleanup_battle(&mut commands, &mut pools, active, &dangers, &summons);
            commands.queue(move |world: &mut World| award_victory(world, active));
        } else if session.finished
            || (!state.boss_defeated && bosses.get(active).is_err() && !arrivals.contains(active))
        {
            pools.retire_enemy(&mut commands, active);
            cleanup_battle(&mut commands, &mut pools, active, &dangers, &summons);
            for (entity, guard, _) in &guards {
                if guard.owner == active {
                    pools.retire_enemy(&mut commands, entity);
                }
            }
            state.active = None;
            state.remaining_secs = 0.0;
        } else if !session.paused && !session.save_active {
            state.remaining_secs = (state.remaining_secs - time.delta_secs()).max(0.0);
            if state.remaining_secs <= 0.0 {
                cleanup_battle(&mut commands, &mut pools, active, &dangers, &summons);
                for (entity, guard, _) in &guards {
                    if guard.owner == active {
                        pools.retire_enemy(&mut commands, entity);
                    }
                }
                pools.retire_enemy(&mut commands, active);
                state.active = None;
                state.revision += 1;
            }
        }
        return;
    }
    if state.pending.is_some()
        || !can_offer(&session)
        || draft.as_deref().is_some_and(|draft| draft.kind.is_some())
        || recruitment
            .as_deref()
            .is_some_and(|recruitment| recruitment.pending.is_some())
    {
        return;
    }
    for index in incoming {
        if index < 3 && state.unlocked & (1 << index) != 0 && state.resolved & (1 << index) == 0 {
            state.pending = Some(index);
            session.paused = true;
            break;
        }
    }
}
fn choose_adventure(
    mut commands: Commands,
    mut choices: MessageReader<AdventureBossChoice>,
    mut state: ResMut<AdventureBossState>,
    mut session: ResMut<GameSession>,
    assets: Option<Res<GameAssets>>,
    mut pools: ResMut<CombatPools>,
) {
    for choice in choices.read() {
        let Some(index) = state.pending else {
            continue;
        };
        if session.finished {
            continue;
        }
        let def = &ADVENTURE_BOSSES[usize::from(index)];
        match choice {
            AdventureBossChoice::TakeReward => {
                session.gold = session.gold.saturating_add(def.small_gold);
                session.wood = session.wood.saturating_add(def.small_wood);
            }
            AdventureBossChoice::Fight => {
                let Some(assets) = assets.as_deref() else {
                    continue;
                };
                let boss = &BOSSES[usize::from(def.boss_index)];
                let site = ADVENTURE_SITES[usize::from(index)];
                let landing = ADVENTURE_LANDING_SITES[usize::from(index)];
                // Rent all four roots together: exhausted pools leave the offer intact.
                let transforms = [
                    Transform::from_translation(site).with_scale(Vec3::splat(1.15)),
                    Transform::from_translation(site + Vec3::new(1.8, 0.0, -1.3))
                        .with_scale(Vec3::splat(0.55)),
                    Transform::from_translation(site + Vec3::new(1.8, 0.0, 0.0))
                        .with_scale(Vec3::splat(0.55)),
                    Transform::from_translation(site + Vec3::new(1.8, 0.0, 1.3))
                        .with_scale(Vec3::splat(0.55)),
                ];
                let actors: Vec<_> = transforms
                    .iter()
                    .enumerate()
                    .map(|(unit, transform)| {
                        if unit == 0 {
                            (
                                EnemySceneKey::BossSummon(
                                    ADVENTURE_LEADER_VISUALS[usize::from(index)] as u8,
                                ),
                                assets.enemy_scenes[ADVENTURE_LEADER_VISUALS[usize::from(index)]]
                                    .clone(),
                                *transform,
                            )
                        } else {
                            (
                                EnemySceneKey::BossSummon(ADVENTURE_GUARD_VISUALS[unit - 1] as u8),
                                assets.enemy_scenes[ADVENTURE_GUARD_VISUALS[unit - 1]].clone(),
                                transform.with_scale(Vec3::splat(0.9 + (unit - 1) as f32 * 0.05)),
                            )
                        }
                    })
                    .collect();
                let Some(entities) = pools.rent_enemy_scene_batch(&mut commands, &actors) else {
                    continue;
                };
                let entity = entities[0];
                for (unit, guard) in entities.iter().enumerate().skip(1) {
                    commands.entity(*guard).insert((
                        CampArrival {
                            start: transforms[unit].translation,
                            end: landing + transforms[unit].translation - site,
                            remaining_secs: CAMP_ARRIVAL_SECONDS,
                            boss_index: None,
                            enemy: Some(Enemy {
                                hp: def.hp * 0.06,
                                max_hp: def.hp * 0.06,
                                speed: 1.6,
                                damage: def.damage * 0.35,
                                xp: 0,
                                gold: 0,
                                wood: 0,
                                kill_credit: 0,
                                elite: false,
                            }),
                        },
                        EncounterGuard { owner: entity },
                        CampCombatant { center: landing },
                        EnemyArmor(0.0),
                        Name::new("Adventure camp guard"),
                    ));
                }
                commands.entity(entity).insert((
                    CampArrival {
                        start: site,
                        end: landing,
                        remaining_secs: CAMP_ARRIVAL_SECONDS,
                        boss_index: Some(def.boss_index),
                        enemy: Some(Enemy {
                            hp: def.hp / boss.modifiers.max_hp_multiplier,
                            max_hp: def.hp / boss.modifiers.max_hp_multiplier,
                            speed: boss.speed,
                            damage: def.damage,
                            xp: 0,
                            wood: 0,
                            gold: 0,
                            kill_credit: 0,
                            elite: true,
                        }),
                    },
                    EncounterBoss { index },
                    CampCombatant { center: landing },
                    EnemyArmor(0.0),
                    EnemyBreachDamage(def.damage),
                    Name::new(def.name_en),
                ));
                state.active = Some(entity);
                state.active_index = index;
                state.boss_defeated = false;
                state.remaining_secs = ADVENTURE_BATTLE_SECONDS;
            }
        }
        state.resolved |= 1 << index;
        state.pending = None;
        state.revision += 1;
        session.paused = false;
    }
}
fn cleanup_battle(
    commands: &mut Commands,
    pools: &mut CombatPools,
    boss: Entity,
    dangers: &Query<(Entity, &BossTelegraph)>,
    summons: &Query<(Entity, &BossSummon)>,
) {
    for (entity, danger) in dangers {
        if danger.boss == boss {
            commands.entity(entity).try_despawn();
        }
    }
    for (entity, summon) in summons {
        if summon.0 == boss {
            pools.retire_enemy(commands, entity);
        }
    }
}
/// Called only by the real enemy defeat resolver; stale or duplicate requests pay nothing.
pub(crate) fn successful_defeat(world: &mut World, entity: Entity, index: u8) {
    let Some(mut state) = world.get_resource_mut::<AdventureBossState>() else {
        return;
    };
    if state.active == Some(entity) && state.completed & (1 << index) == 0 {
        state.boss_defeated = true;
    }
}
pub(crate) fn award_victory(world: &mut World, entity: Entity) {
    let Some(state) = world.get_resource::<AdventureBossState>() else {
        return;
    };
    if state.active != Some(entity) || !state.boss_defeated {
        return;
    }
    let index = state.active_index;
    let mut state = world.resource_mut::<AdventureBossState>();
    state.active = None;
    state.boss_defeated = false;
    state.remaining_secs = 0.0;
    state.completed |= 1 << index;
    state.revision += 1;
    let def = &ADVENTURE_BOSSES[usize::from(index)];
    let mut session = world.resource_mut::<GameSession>();
    if session.finished {
        return;
    }
    session.xp = session
        .xp
        .saturating_add((def.reward_xp as f32 * session.xp_multiplier) as u32);
    session.gold = session.gold.saturating_add(def.reward_gold);
    session.wood = session.wood.saturating_add(def.reward_wood);
    session.total_kills += 1;
    if let Some(mut relics) = world.get_resource_mut::<super::BasicRelics>() {
        relics.grant(index);
    }
    if let Some(mut recruitment) = world.get_resource_mut::<RecruitmentProgress>() {
        recruitment.fruit_count = recruitment.fruit_count.saturating_add(1);
    }
}
pub(crate) fn reset_run(world: &mut World) {
    world.insert_resource(AdventureBossState::default());
    if let Some(mut messages) = world.get_resource_mut::<Messages<AdventureBossChoice>>() {
        messages.clear();
    }
    if let Some(mut messages) = world.get_resource_mut::<Messages<AdventureCampRequest>>() {
        messages.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<GameSession>()
            .init_resource::<RecruitmentProgress>()
            .init_resource::<super::super::BasicRelics>()
            .init_resource::<CombatPools>()
            .init_resource::<AdventureBossState>()
            .add_message::<AdventureCampRequest>()
            .add_message::<AdventureBossChoice>();
        app
    }
    #[test]
    fn camp_offer_requires_unlocked_request_and_allows_existing_campaign_boss() {
        let mut app = test_app();
        app.add_systems(Update, tick_adventure);
        app.world_mut().resource_mut::<GameSession>().elapsed = 180.0;
        app.update();
        assert_eq!(app.world().resource::<AdventureBossState>().unlocked, 1);
        assert!(
            app.world()
                .resource::<AdventureBossState>()
                .pending
                .is_none()
        );
        app.world_mut().spawn((
            BossEnemy { index: 0 },
            Enemy {
                hp: 10.0,
                max_hp: 10.0,
                speed: 1.0,
                damage: 1.0,
                xp: 0,
                gold: 0,
                wood: 0,
                kill_credit: 0,
                elite: true,
            },
        ));
        app.world_mut().resource_mut::<GameSession>().elapsed = 300.0;
        app.world_mut().write_message(AdventureCampRequest(0));
        app.update();
        assert_eq!(
            app.world().resource::<AdventureBossState>().pending,
            Some(0)
        );
        assert!(app.world().resource::<GameSession>().paused);
        assert!(can_offer(&GameSession {
            elapsed: 299.0,
            ..default()
        }));
        assert!(!can_offer(&GameSession {
            paused: true,
            ..default()
        }));
    }
    #[test]
    fn failed_fight_rental_keeps_choice_and_small_reward_pays_once() {
        let mut app = test_app();
        app.add_systems(Update, choose_adventure);
        app.world_mut().resource_mut::<AdventureBossState>().pending = Some(0);
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.world_mut().write_message(AdventureBossChoice::Fight);
        app.update();
        assert_eq!(
            app.world().resource::<AdventureBossState>().pending,
            Some(0)
        );
        let before = app.world().resource::<GameSession>().gold;
        app.world_mut()
            .write_message(AdventureBossChoice::TakeReward);
        app.world_mut()
            .write_message(AdventureBossChoice::TakeReward);
        app.update();
        assert_eq!(
            app.world().resource::<GameSession>().gold,
            before + ADVENTURE_BOSSES[0].small_gold
        );
        assert_eq!(app.world().resource::<RecruitmentProgress>().fruit_count, 0);
        assert!(!app.world().resource::<super::super::BasicRelics>().owns(0));
        assert!(!app.world().resource::<GameSession>().paused);
    }
    #[test]
    fn real_boss_defeat_waits_for_guard_then_awards_once_without_campaign_credit() {
        let mut app = test_app();
        app.add_systems(Update, tick_adventure);
        let boss = app.world_mut().spawn(EncounterBoss { index: 1 }).id();
        let guard = app
            .world_mut()
            .spawn((
                EncounterGuard { owner: boss },
                Enemy {
                    hp: 20.0,
                    max_hp: 20.0,
                    speed: 1.0,
                    damage: 1.0,
                    xp: 0,
                    gold: 0,
                    wood: 0,
                    kill_credit: 0,
                    elite: false,
                },
            ))
            .id();
        {
            let mut state = app.world_mut().resource_mut::<AdventureBossState>();
            state.active = Some(boss);
            state.active_index = 1;
            state.remaining_secs = 90.0;
        }
        successful_defeat(app.world_mut(), boss, 1);
        let gold = app.world().resource::<GameSession>().gold;
        app.update();
        assert_eq!(app.world().resource::<GameSession>().gold, gold);
        app.world_mut().entity_mut(guard).despawn();
        app.update();
        assert_eq!(
            app.world().resource::<GameSession>().gold,
            gold + ADVENTURE_BOSSES[1].reward_gold
        );
        assert_eq!(app.world().resource::<RecruitmentProgress>().fruit_count, 1);
        assert!(app.world().resource::<super::super::BasicRelics>().owns(1));
        assert_eq!(app.world().resource::<GameSession>().bosses_defeated, 0);
        assert!(!app.world().resource::<GameSession>().save_active);
        award_victory(app.world_mut(), boss);
        assert_eq!(app.world().resource::<RecruitmentProgress>().fruit_count, 1);
        reset_run(app.world_mut());
        assert_eq!(app.world().resource::<AdventureBossState>().unlocked, 0);
    }
    #[test]
    fn wild_arrival_is_airborne_untargetable_and_paused_until_real_landing() {
        use bevy::ecs::system::RunSystemOnce;
        use std::time::Duration;
        let mut app = test_app();
        let start = ADVENTURE_SITES[0];
        let end = ADVENTURE_LANDING_SITES[0];
        let actor = app
            .world_mut()
            .spawn((
                Transform::from_translation(start),
                CampArrival {
                    start,
                    end,
                    remaining_secs: CAMP_ARRIVAL_SECONDS,
                    boss_index: Some(0),
                    enemy: Some(Enemy {
                        hp: 100.0,
                        max_hp: 100.0,
                        speed: 1.0,
                        damage: 10.0,
                        xp: 0,
                        gold: 0,
                        wood: 0,
                        kill_credit: 0,
                        elite: true,
                    }),
                },
            ))
            .id();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(0.5));
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.world_mut().run_system_once(advance_arrivals).unwrap();
        assert_eq!(
            app.world().get::<Transform>(actor).unwrap().translation,
            start
        );
        assert!(app.world().get::<Enemy>(actor).is_none());
        app.world_mut().resource_mut::<GameSession>().paused = false;
        app.world_mut().run_system_once(advance_arrivals).unwrap();
        let midpoint = app.world().get::<Transform>(actor).unwrap().translation;
        assert!((midpoint.y - 2.8).abs() < 0.001);
        assert!((midpoint.z - (start.z + end.z) * 0.5).abs() < 0.001);
        assert!(app.world().get::<Enemy>(actor).is_none());
        assert!(app.world().get::<BossEnemy>(actor).is_none());
        app.world_mut().run_system_once(advance_arrivals).unwrap();
        assert_eq!(
            app.world().get::<Transform>(actor).unwrap().translation,
            end
        );
        assert!(app.world().get::<CampArrival>(actor).is_none());
        assert_eq!(app.world().get::<Enemy>(actor).unwrap().hp, 100.0);
        assert_eq!(app.world().get::<BossEnemy>(actor).unwrap().index, 0);
    }
    #[test]
    fn all_camps_have_three_different_wild_guards_and_a_separate_leader_model() {
        for leader in ADVENTURE_LEADER_VISUALS {
            let mut models = std::collections::BTreeSet::from(ADVENTURE_GUARD_VISUALS);
            models.insert(leader);
            assert_eq!(models.len(), 4);
        }
        for (site, landing) in ADVENTURE_SITES.iter().zip(ADVENTURE_LANDING_SITES) {
            assert!(site.z.abs() > 5.65);
            assert!(landing.z.abs() < 5.65);
            assert_eq!(site.x, landing.x);
        }
    }
}
