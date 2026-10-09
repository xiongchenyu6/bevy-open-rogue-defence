//! Optional wild leaders own distinct skills and never advance campaign checkpoints.
use super::combat::{CombatSet, EnemyArmor, EnemyBreachDamage};
use super::pooling::{CombatPools, EnemySceneKey};
use super::{
    Core, DraftState, Enemy, EnemyAttackCue, EnemyAttackKind, GameAssets, GameSession, Hero,
    HeroVitals, RecruitmentProgress,
};
use bevy::prelude::*;

pub const ADVENTURE_OFFER_TIMES: [f32; 3] = [180.0, 480.0, 780.0];
pub const ADVENTURE_SITES: [Vec3; 3] = [
    Vec3::new(-3.0, 0.0, -9.0),
    Vec3::new(5.0, 0.0, 9.0),
    Vec3::new(8.0, 0.0, -9.0),
];
pub const ADVENTURE_LEADER_HEIGHT: f32 = 2.7;
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
    pub move_speed: f32,
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
    description_en: "Below 50% HP: cooldowns ×0.75, areas ×1.20, damage ×1.35.",
    description_zh: "生命低于50%：冷却乘0.75、范围乘1.20、伤害乘1.35。",
};
pub const ADVENTURE_BOSSES: [AdventureBossDef; 3] = [
    AdventureBossDef {
        name_en: "Embermaw Alpha",
        name_zh: "炎牙兽王",
        icon_path: "icons/cinder_aura.png",
        move_speed: 0.9,
        hp: 5000.0,
        damage: 18.0,
        reward_xp: 400,
        reward_gold: 600,
        reward_wood: 100,
        small_gold: 90,
        small_wood: 15,
        abilities: [
            AdventureAbility {
                name_en: "Rending fan",
                name_zh: "裂焰扇爪",
                description_en: "Every 4s: 0.7s warning, 5m forward cone (92 degrees), physical 200% attack.",
                description_zh: "每4秒：预警0.7秒，正面5米、92度扇爪，造成攻击力200%物理伤害。",
            },
            AdventureAbility {
                name_en: "Meteor triad",
                name_zh: "三重陨火",
                description_en: "Every 7s: three locked meteors, 1.1s warning, 1.6m radius each, magical 160% attack.",
                description_zh: "每7秒：三枚锁定陨石，预警1.1秒，每枚半径1.6米，造成攻击力160%法术伤害。",
            },
            PHASE,
        ],
    },
    AdventureBossDef {
        name_en: "Tidehorn Matriarch",
        name_zh: "潮角巨兽",
        icon_path: "icons/tide_elemental.png",
        move_speed: 0.75,
        hp: 18000.0,
        damage: 30.0,
        reward_xp: 700,
        reward_gold: 1200,
        reward_wood: 180,
        small_gold: 180,
        small_wood: 25,
        abilities: [
            AdventureAbility {
                name_en: "Glacial surge",
                name_zh: "冰海奔涌",
                description_en: "Every 6s: 0.9s warning, a 3m wide wave travels 14m at 9m/s, magical 150% attack and 0.6s freeze.",
                description_zh: "每6秒：预警0.9秒，宽3米水波以每秒9米推进14米，造成攻击力150%法术伤害并冻结0.6秒。",
            },
            AdventureAbility {
                name_en: "Frost crown",
                name_zh: "霜冠爆裂",
                description_en: "Every 9s: 1.2s warning, 4m radius nova, magical 180% attack and 1s freeze.",
                description_zh: "每9秒：预警1.2秒，半径4米冰爆，造成攻击力180%法术伤害并冻结1秒。",
            },
            PHASE,
        ],
    },
    AdventureBossDef {
        name_en: "Plaguejaw Elder",
        name_zh: "疫颚长老",
        icon_path: "icons/forbidden_arts.png",
        move_speed: 0.8,
        hp: 45000.0,
        damage: 45.0,
        reward_xp: 1000,
        reward_gold: 2000,
        reward_wood: 300,
        small_gold: 300,
        small_wood: 40,
        abilities: [
            AdventureAbility {
                name_en: "Venom pods",
                name_zh: "疫毒孢囊",
                description_en: "Every 5s: 0.8s warning, three 2m poison pools lasting 5s, magical 35% attack every 0.5s.",
                description_zh: "每5秒：预警0.8秒，三处半径2米毒池持续5秒，每0.5秒造成攻击力35%法术伤害。",
            },
            AdventureAbility {
                name_en: "Plague burst",
                name_zh: "疫颚爆种",
                description_en: "Every 8s: target-locked spore, 1.3s warning, 2.8m radius explosion, magical 220% attack.",
                description_zh: "每8秒：目标锁定爆种，预警1.3秒，半径2.8米爆炸，造成攻击力220%法术伤害。",
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
#[derive(Component)]
pub struct AdventureSkillClock {
    pub enraged: bool,
    cooldowns: [f32; 2],
}
impl Default for AdventureSkillClock {
    fn default() -> Self {
        Self {
            enraged: false,
            cooldowns: [2.0, 4.0],
        }
    }
}
#[derive(Message, Clone, Copy)]
pub struct AdventurePhaseCue {
    pub owner: Entity,
    pub index: u8,
    pub origin: Vec3,
}
#[derive(Message, Clone, Copy)]
pub struct AdventureImpactCue {
    pub owner: Entity,
    pub index: u8,
    pub kind: u8,
    pub origin: Vec3,
    pub target: Vec3,
    pub shape: AdventureDangerShape,
}
#[derive(Clone, Copy, Debug)]
pub enum AdventureDangerShape {
    Circle { radius: f32 },
    Cone { range: f32, half_angle: f32 },
    Wave { half_width: f32, length: f32 },
}
#[derive(Component)]
pub struct AdventureDanger {
    pub owner: Entity,
    pub index: u8,
    pub kind: u8,
    pub origin: Vec3,
    pub target: Vec3,
    pub remaining_secs: f32,
    pub warning_secs: f32,
    pub active: bool,
    pub shape: AdventureDangerShape,
    damage: f32,
    physical: bool,
    freeze_secs: f32,
    active_secs: f32,
    tick_remaining: f32,
    distance: f32,
    wave_origin: Vec3,
    direction: Vec3,
    hit_hero: bool,
    hit_gate: bool,
}
const ADVENTURE_COOLDOWNS: [[f32; 2]; 3] = [[4.0, 7.0], [6.0, 9.0], [5.0, 8.0]];
fn tick_adventure_skills(
    mut commands: Commands,
    time: Res<Time>,
    controls: Res<super::skills::SkillRuntime>,
    session: Res<GameSession>,
    hero: Query<&Transform, With<Hero>>,
    gate: Query<&Transform, With<Core>>,
    mut leaders: Query<(
        Entity,
        &EncounterBoss,
        &Enemy,
        &Transform,
        &mut AdventureSkillClock,
    )>,
    mut phase: MessageWriter<AdventurePhaseCue>,
    mut attack_cues: MessageWriter<EnemyAttackCue>,
) {
    if session.paused || session.finished || session.save_active {
        return;
    }
    let Some(hero) = hero.iter().next() else {
        return;
    };
    for (owner, boss, enemy, transform, mut clock) in &mut leaders {
        if enemy.hp <= 0.0 {
            continue;
        }
        let delta = controls.action_delta(owner, time.delta_secs());
        if controls.actions_blocked(owner) || (delta <= 0.0 && time.delta_secs() > 0.0) {
            continue;
        }
        if !clock.enraged && enemy.hp <= enemy.max_hp * 0.5 {
            clock.enraged = true;
            phase.write(AdventurePhaseCue {
                owner,
                index: boss.index,
                origin: transform.translation,
            });
        }
        let mut target = hero.translation;
        if let Some(gate) = gate.iter().next()
            && session.core_hp > 0.0
            && session.gate_revive_remaining <= 0.0
            && target.x < gate.translation.x
            && transform.translation.x >= gate.translation.x
        {
            target.x = gate.translation.x + 1.9;
        }
        for (kind, interval) in ADVENTURE_COOLDOWNS[usize::from(boss.index)]
            .iter()
            .enumerate()
        {
            clock.cooldowns[kind] -= delta;
            if clock.cooldowns[kind] > 0.0 {
                continue;
            }
            // At most one cast of each type per frame, including 32× speed.
            clock.cooldowns[kind] = *interval * if clock.enraged { 0.75 } else { 1.0 };
            attack_cues.write(EnemyAttackCue {
                source: owner,
                origin: transform.translation,
                target,
                kind: if boss.index == 0 && kind == 0 {
                    EnemyAttackKind::Slash
                } else {
                    EnemyAttackKind::Burst
                },
            });
            spawn_adventure_dangers(
                &mut commands,
                owner,
                boss.index,
                kind as u8,
                transform.translation,
                target,
                enemy.damage,
                clock.enraged,
            );
        }
    }
}
fn spawn_adventure_dangers(
    commands: &mut Commands,
    owner: Entity,
    index: u8,
    kind: u8,
    origin: Vec3,
    target: Vec3,
    attack: f32,
    enraged: bool,
) {
    let size = if enraged { 1.2 } else { 1.0 };
    let power = if enraged { 1.35 } else { 1.0 };
    let (shape, warning, active, ratio, physical, freeze, count) = match (index, kind) {
        (0, 0) => (
            AdventureDangerShape::Cone {
                range: 5.0 * size,
                half_angle: 0.8,
            },
            0.7,
            0.05,
            2.0,
            true,
            0.0,
            1,
        ),
        (0, 1) => (
            AdventureDangerShape::Circle { radius: 1.6 * size },
            1.1,
            0.05,
            1.6,
            false,
            0.0,
            3,
        ),
        (1, 0) => (
            AdventureDangerShape::Wave {
                half_width: 1.5 * size,
                length: 14.0,
            },
            0.9,
            14.0 / 9.0,
            1.5,
            false,
            0.6,
            1,
        ),
        (1, 1) => (
            AdventureDangerShape::Circle { radius: 4.0 * size },
            1.2,
            0.05,
            1.8,
            false,
            1.0,
            1,
        ),
        (2, 0) => (
            AdventureDangerShape::Circle { radius: 2.0 * size },
            0.8,
            5.0,
            0.35,
            false,
            0.0,
            3,
        ),
        (2, 1) => (
            AdventureDangerShape::Circle { radius: 2.8 * size },
            1.3,
            0.05,
            2.2,
            false,
            0.0,
            1,
        ),
        _ => return,
    };
    for n in 0..count {
        let offset = if count == 3 {
            Vec3::new((n as f32 - 1.0) * 2.0, 0.0, (n % 2) as f32 * 1.4)
        } else {
            Vec3::ZERO
        };
        let center = if index == 1 && kind == 1 {
            origin
        } else {
            target + offset
        };
        commands.spawn((
            Name::new("Adventure skill danger"),
            Transform::default(),
            Visibility::default(),
            AdventureDanger {
                owner,
                index,
                kind,
                origin,
                target: center,
                remaining_secs: warning,
                warning_secs: warning,
                active: false,
                shape,
                damage: attack * ratio * power,
                physical,
                freeze_secs: freeze,
                active_secs: active,
                tick_remaining: 0.0,
                distance: 0.0,
                wave_origin: origin,
                direction: (target - origin).with_y(0.0).normalize_or(Vec3::NEG_X),
                hit_hero: false,
                hit_gate: false,
            },
        ));
    }
}
fn adventure_contains(danger: &AdventureDanger, point: Vec3) -> bool {
    match danger.shape {
        AdventureDangerShape::Circle { radius } => {
            point.xz().distance_squared(danger.target.xz()) <= radius * radius
        }
        AdventureDangerShape::Cone { range, half_angle } => {
            let offset = point.xz() - danger.origin.xz();
            offset.length_squared() <= range * range
                && (offset.length_squared() < 0.001
                    || offset
                        .normalize()
                        .dot((danger.target.xz() - danger.origin.xz()).normalize_or(Vec2::NEG_X))
                        >= half_angle.cos())
        }
        AdventureDangerShape::Wave { half_width, length } => {
            let direction = (danger.target.xz() - danger.origin.xz()).normalize_or(Vec2::NEG_X);
            let offset = point.xz() - danger.origin.xz();
            let forward = offset.dot(direction);
            forward >= 0.0 && forward <= length && offset.perp_dot(direction).abs() <= half_width
        }
    }
}
fn tick_adventure_dangers(
    mut commands: Commands,
    time: Res<Time>,
    mut session: ResMut<GameSession>,
    mut vitals: ResMut<HeroVitals>,
    hero: Query<&Transform, With<Hero>>,
    gate: Query<&Transform, With<Core>>,
    leaders: Query<&Enemy, With<EncounterBoss>>,
    mut dangers: Query<(Entity, &mut AdventureDanger)>,
    mut impacts: MessageWriter<AdventureImpactCue>,
) {
    for (entity, mut danger) in &mut dangers {
        if session.finished
            || leaders.get(danger.owner).is_err()
            || leaders.get(danger.owner).is_ok_and(|enemy| enemy.hp <= 0.0)
        {
            commands.entity(entity).try_despawn();
            continue;
        }
        if session.paused || session.save_active {
            continue;
        }
        let delta = time.delta_secs();
        danger.remaining_secs -= delta;
        if !danger.active {
            if danger.remaining_secs > 0.0 {
                continue;
            }
            danger.active = true;
            danger.remaining_secs = danger.active_secs;
            impacts.write(AdventureImpactCue {
                owner: danger.owner,
                index: danger.index,
                kind: danger.kind,
                origin: danger.origin,
                target: danger.target,
                shape: danger.shape,
            });
        }
        if let AdventureDangerShape::Wave { half_width, .. } = danger.shape {
            let direction = danger.direction;
            danger.origin = danger.wave_origin + direction * danger.distance;
            danger.target = danger.origin + direction * 14.0;
            let step = (delta * 9.0).min((14.0 - danger.distance).max(0.0));
            danger.shape = AdventureDangerShape::Wave {
                half_width,
                length: (step + 3.0).min(14.0 - danger.distance),
            };
            danger.distance += step;
            // Bounds cover the entire swept front, so high speed cannot tunnel past the hero.
            danger.tick_remaining = 0.0;
            danger.remaining_secs = if danger.distance >= 14.0 {
                0.0
            } else {
                danger.remaining_secs
            };
            // Apply damage below before moving the visible front for the next tick.
            if !danger.hit_hero
                && let Some(hero) = hero.iter().next()
                && adventure_contains(&danger, hero.translation)
                && !gate_protects_adventure_hero(
                    &session,
                    gate.iter().next(),
                    danger.origin,
                    hero.translation,
                )
            {
                let dealt = vitals.take_magical_damage(danger.damage);
                if dealt > 0.0 {
                    vitals.stunned_remaining = vitals.stunned_remaining.max(danger.freeze_secs);
                    danger.hit_hero = true;
                }
            }
            if !danger.hit_gate
                && let Some(gate) = gate.iter().next()
                && session.core_hp > 0.0
            {
                let distance = (gate.translation.x - danger.origin.x) / direction.x;
                if direction.x.abs() > 0.001 && distance >= 0.0 {
                    let point = danger.origin + direction * distance;
                    if point.z.abs() <= 5.65 && adventure_contains(&danger, point) {
                        session.core_hp = (session.core_hp - danger.damage).max(0.0);
                        danger.hit_gate = true;
                    }
                }
            }
        } else {
            danger.tick_remaining -= delta;
            if danger.tick_remaining <= 0.0 {
                danger.tick_remaining = 0.5;
                if let Some(gate) = gate.iter().next()
                    && session.core_hp > 0.0
                    && !danger.hit_gate
                {
                    let center = match danger.shape {
                        AdventureDangerShape::Circle { .. } => danger.target,
                        _ => danger.origin,
                    };
                    let point = Vec3::new(gate.translation.x, 0.0, center.z.clamp(-5.65, 5.65));
                    if adventure_contains(&danger, point) {
                        session.core_hp = (session.core_hp - danger.damage).max(0.0);
                        if danger.active_secs < 0.5 {
                            danger.hit_gate = true;
                        }
                    }
                }
                if let Some(hero) = hero.iter().next()
                    && vitals.alive
                    && !danger.hit_hero
                    && adventure_contains(&danger, hero.translation)
                    && !gate_protects_adventure_hero(
                        &session,
                        gate.iter().next(),
                        danger.origin,
                        hero.translation,
                    )
                {
                    let dealt = if danger.physical {
                        vitals.take_physical_damage(danger.damage)
                    } else {
                        vitals.take_magical_damage(danger.damage)
                    };
                    if dealt > 0.0 {
                        vitals.stunned_remaining = vitals.stunned_remaining.max(danger.freeze_secs);
                        if danger.active_secs < 0.5 {
                            danger.hit_hero = true;
                        }
                    }
                }
            }
        }
        if danger.active_secs < 0.5 || danger.remaining_secs <= 0.0 {
            commands.entity(entity).try_despawn();
        }
    }
}
fn gate_protects_adventure_hero(
    session: &GameSession,
    gate: Option<&Transform>,
    origin: Vec3,
    hero: Vec3,
) -> bool {
    session.core_hp > 0.0
        && session.gate_revive_remaining <= 0.0
        && gate.is_some_and(|gate| {
            origin.x >= gate.translation.x
                && hero.x < gate.translation.x
                && hero.z.abs() <= 5.65
                && origin.z.abs() <= 5.65
        })
}

/// No Enemy component exists in flight, so all real damage/targeting ignores arrivals.
#[derive(Component)]
pub struct CampArrival {
    pub start: Vec3,
    pub end: Vec3,
    pub remaining_secs: f32,
    enemy: Option<Enemy>,
    leader: bool,
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
            if arrival.leader {
                commands
                    .entity(entity)
                    .insert(AdventureSkillClock::default());
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
            .add_message::<AdventurePhaseCue>()
            .add_message::<AdventureImpactCue>()
            .add_message::<EnemyAttackCue>()
            .add_systems(Update, advance_arrivals.before(CombatSet))
            .add_systems(
                Update,
                (
                    tick_adventure,
                    choose_adventure,
                    tick_adventure_skills,
                    tick_adventure_dangers,
                )
                    .chain()
                    .after(CombatSet),
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
    bosses: Query<&Enemy, With<EncounterBoss>>,
    dangers: Query<(Entity, &AdventureDanger)>,
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
            cleanup_battle(&mut commands, active, &dangers);
            commands.queue(move |world: &mut World| award_victory(world, active));
        } else if session.finished
            || (!state.boss_defeated && bosses.get(active).is_err() && !arrivals.contains(active))
        {
            pools.retire_enemy(&mut commands, active);
            cleanup_battle(&mut commands, active, &dangers);
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
                cleanup_battle(&mut commands, active, &dangers);
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
                let site = ADVENTURE_SITES[usize::from(index)];
                let landing = ADVENTURE_LANDING_SITES[usize::from(index)];
                // Rent all four roots together: exhausted pools leave the offer intact.
                let transforms = [
                    Transform::from_translation(site)
                        .with_scale(Vec3::splat(ADVENTURE_LEADER_HEIGHT)),
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
                                EnemySceneKey::Adventure(index),
                                assets.adventure_scenes[usize::from(index)].clone(),
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
                            leader: false,
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
                        leader: true,
                        enemy: Some(Enemy {
                            hp: def.hp,
                            max_hp: def.hp,
                            speed: def.move_speed,
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
    boss: Entity,
    dangers: &Query<(Entity, &AdventureDanger)>,
) {
    for (entity, danger) in dangers {
        if danger.owner == boss {
            commands.entity(entity).try_despawn();
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
    let mut query = world.query_filtered::<Entity, With<AdventureDanger>>();
    let hazards: Vec<_> = query.iter(world).collect();
    for entity in hazards {
        world.despawn(entity);
    }
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
    use super::super::BossEnemy;
    use super::*;
    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<GameSession>()
            .init_resource::<RecruitmentProgress>()
            .init_resource::<super::super::BasicRelics>()
            .init_resource::<CombatPools>()
            .init_resource::<AdventureBossState>()
            .init_resource::<super::super::skills::SkillRuntime>()
            .add_message::<AdventureCampRequest>()
            .add_message::<AdventureBossChoice>()
            .add_message::<AdventurePhaseCue>()
            .add_message::<AdventureImpactCue>()
            .add_message::<EnemyAttackCue>();
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
                    leader: true,
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
        assert!(app.world().get::<BossEnemy>(actor).is_none());
        assert!(app.world().get::<AdventureSkillClock>(actor).is_some());
    }
    #[test]
    fn all_camps_have_three_different_wild_guards_and_a_separate_leader_model() {
        assert_eq!(
            std::collections::BTreeSet::from(ADVENTURE_GUARD_VISUALS).len(),
            3
        );
        assert_eq!(ADVENTURE_LEADER_HEIGHT, 2.7);
        for (site, landing) in ADVENTURE_SITES.iter().zip(ADVENTURE_LANDING_SITES) {
            assert!(site.z.abs() > 5.65);
            assert!(landing.z.abs() < 5.65);
            assert_eq!(site.x, landing.x);
        }
    }
    fn skill_test_enemy(hp: f32) -> Enemy {
        Enemy {
            hp,
            max_hp: 100.0,
            speed: 1.0,
            damage: 20.0,
            xp: 0,
            gold: 0,
            wood: 0,
            kill_credit: 0,
            elite: true,
        }
    }
    #[test]
    fn independent_leaders_cast_distinct_loadouts_and_phase_once_without_campaign_components() {
        use bevy::ecs::system::RunSystemOnce;
        use std::time::Duration;
        for index in 0..3 {
            let mut app = test_app();
            app.add_message::<AdventurePhaseCue>();
            app.world_mut()
                .spawn((Hero, Transform::from_xyz(-2.0, 0.0, 0.0)));
            let leader = app
                .world_mut()
                .spawn((
                    EncounterBoss { index },
                    skill_test_enemy(49.0),
                    Transform::default(),
                    AdventureSkillClock::default(),
                ))
                .id();
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(Duration::from_secs(2));
            app.world_mut()
                .run_system_once(tick_adventure_skills)
                .unwrap();
            assert!(app.world().get::<BossEnemy>(leader).is_none());
            assert!(
                app.world()
                    .get::<AdventureSkillClock>(leader)
                    .unwrap()
                    .enraged
            );
            let mut query = app.world_mut().query::<&AdventureDanger>();
            let hazards: Vec<_> = query.iter(app.world()).collect();
            assert_eq!(hazards.len(), if index == 2 { 3 } else { 1 });
            match (index, hazards[0].shape) {
                (0, AdventureDangerShape::Cone { range, .. }) => {
                    assert!((range - 6.0).abs() < 0.001)
                }
                (1, AdventureDangerShape::Wave { half_width, .. }) => {
                    assert!((half_width - 1.8).abs() < 0.001)
                }
                (2, AdventureDangerShape::Circle { radius }) => {
                    assert!((radius - 2.4).abs() < 0.001)
                }
                _ => panic!("wrong leader-specific skill geometry"),
            }
            app.world_mut().get_mut::<Enemy>(leader).unwrap().hp = 100.0;
            app.world_mut()
                .run_system_once(tick_adventure_skills)
                .unwrap();
            assert_eq!(
                app.world().resource::<Messages<AdventurePhaseCue>>().len(),
                1
            );
            let mut query = app.world_mut().query::<&AdventureDanger>();
            assert!(query.iter(app.world()).any(|hazard| hazard.kind == 1));
        }
    }
    #[test]
    fn real_wave_sweep_hits_at_high_speed_freezes_after_damage_and_cleans_on_owner_death() {
        use bevy::ecs::system::RunSystemOnce;
        use std::time::Duration;
        let mut app = test_app();
        app.init_resource::<HeroVitals>();
        app.world_mut().resource_mut::<GameSession>().core_hp = 0.0;
        app.world_mut()
            .resource_mut::<HeroVitals>()
            .magic_resistance = 0.0;
        let leader = app
            .world_mut()
            .spawn((EncounterBoss { index: 1 }, skill_test_enemy(100.0)))
            .id();
        app.world_mut()
            .spawn((Hero, Transform::from_xyz(-6.0, 0.0, 0.0)));
        app.world_mut()
            .run_system_once(move |mut commands: Commands| {
                spawn_adventure_dangers(
                    &mut commands,
                    leader,
                    1,
                    0,
                    Vec3::ZERO,
                    Vec3::NEG_X * 12.0,
                    20.0,
                    false,
                );
            })
            .unwrap();
        let health = app.world().resource::<HeroVitals>().health;
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(0.8));
        app.world_mut()
            .run_system_once(tick_adventure_dangers)
            .unwrap();
        assert_eq!(app.world().resource::<HeroVitals>().health, health);
        app.world_mut()
            .run_system_once(tick_adventure_dangers)
            .unwrap();
        assert!((app.world().resource::<HeroVitals>().health - (health - 30.0)).abs() < 0.001);
        assert!((app.world().resource::<HeroVitals>().stunned_remaining - 0.6).abs() < 0.001);
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.world_mut().entity_mut(leader).remove::<Enemy>();
        app.world_mut()
            .run_system_once(tick_adventure_dangers)
            .unwrap();
        assert_eq!(
            app.world_mut()
                .query::<&AdventureDanger>()
                .iter(app.world())
                .count(),
            0
        );
    }
    #[test]
    fn independent_claw_warning_dodge_and_armor_are_real() {
        use bevy::ecs::system::RunSystemOnce;
        use std::time::Duration;
        let mut app = test_app();
        app.init_resource::<HeroVitals>();
        app.world_mut().resource_mut::<GameSession>().core_hp = 0.0;
        app.world_mut().resource_mut::<HeroVitals>().armor = 20.0;
        let leader = app
            .world_mut()
            .spawn((EncounterBoss { index: 0 }, skill_test_enemy(100.0)))
            .id();
        let hero = app
            .world_mut()
            .spawn((Hero, Transform::from_xyz(-2.0, 0.0, 0.0)))
            .id();
        app.world_mut()
            .run_system_once(move |mut commands: Commands| {
                spawn_adventure_dangers(
                    &mut commands,
                    leader,
                    0,
                    0,
                    Vec3::ZERO,
                    Vec3::NEG_X * 2.0,
                    20.0,
                    false,
                )
            })
            .unwrap();
        let health = app.world().resource::<HeroVitals>().health;
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(0.4));
        app.world_mut()
            .run_system_once(tick_adventure_dangers)
            .unwrap();
        assert_eq!(app.world().resource::<HeroVitals>().health, health);
        app.world_mut()
            .get_mut::<Transform>(hero)
            .unwrap()
            .translation = Vec3::Z * 5.0;
        app.world_mut()
            .run_system_once(tick_adventure_dangers)
            .unwrap();
        assert_eq!(app.world().resource::<HeroVitals>().health, health);
        app.world_mut()
            .run_system_once(move |mut commands: Commands| {
                spawn_adventure_dangers(
                    &mut commands,
                    leader,
                    0,
                    0,
                    Vec3::ZERO,
                    Vec3::NEG_X * 2.0,
                    20.0,
                    false,
                )
            })
            .unwrap();
        app.world_mut()
            .get_mut::<Transform>(hero)
            .unwrap()
            .translation = Vec3::NEG_X * 2.0;
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(0.8));
        app.world_mut()
            .run_system_once(tick_adventure_dangers)
            .unwrap();
        assert!((app.world().resource::<HeroVitals>().health - (health - 20.0)).abs() < 0.001);
        reset_run(app.world_mut());
        assert_eq!(
            app.world_mut()
                .query::<&AdventureDanger>()
                .iter(app.world())
                .count(),
            0
        );
    }
}
