use std::cmp::Ordering;

use bevy::prelude::*;

#[cfg(test)]
use super::bonds::DamageElement;
use super::bonds::{DamageClass, DamageProfile, summon_cap};
use super::combat::CombatSet;
use super::market::MarketEffects;
use super::pooling::{CombatPools, SummonVisual};
use super::{
    ActiveSkill, BondId, BossEnemy, Core, Enemy, GameAssets, GameSession, Hero, HeroIdentity,
    HeroKind, HeroVitals, ImpactFx, MoveIntent, PlayerBuild, PlayerSummon, Projectile,
    SelectedHero, SkillButtonLabel,
};

pub struct HeroAbilitiesPlugin;

impl Plugin for HeroAbilitiesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HeroAbilityRuntime>()
            .add_systems(PostStartup, attach_default_identity)
            .add_systems(PreUpdate, configure_ultimate)
            .add_systems(
                Update,
                (
                    synchronize_identity,
                    tick_speed_effects,
                    tick_wet_marks,
                    tick_ultimate_sequence,
                    cleanup_spirit_summons,
                    update_spirit_companions,
                )
                    .chain()
                    .after(CombatSet),
            )
            .add_systems(
                PostUpdate,
                (
                    observe_projectile_passives,
                    maintain_passive_state,
                    tick_tide_passive,
                    observe_ultimate_cast,
                    update_active_skill_label,
                )
                    .chain(),
            );
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct UltimateProfile {
    name: &'static str,
    cooldown_secs: f32,
}

const fn ultimate_profile(kind: HeroKind) -> UltimateProfile {
    match kind {
        HeroKind::StormSavant => UltimateProfile {
            name: "THUNDER DOMAIN",
            cooldown_secs: 50.0,
        },
        HeroKind::EmberRanger => UltimateProfile {
            name: "SCARLET VOLLEY",
            cooldown_secs: 45.0,
        },
        HeroKind::StoneWarden => UltimateProfile {
            name: "MOUNTAIN ANCHOR",
            cooldown_secs: 50.0,
        },
        HeroKind::SpiritKeeper => UltimateProfile {
            name: "SPIRIT MARCH",
            cooldown_secs: 55.0,
        },
        HeroKind::TideWalker => UltimateProfile {
            name: "STILL SEA",
            cooldown_secs: 52.0,
        },
        HeroKind::StarWagerer => UltimateProfile {
            name: "LOADED FATE",
            cooldown_secs: 48.0,
        },
    }
}

#[derive(Debug, Clone, Copy)]
struct UltimateSequence {
    kind: HeroKind,
    pulses_left: u8,
    pulse_timer: f32,
    pulse_interval: f32,
}

#[derive(Resource)]
struct HeroAbilityRuntime {
    observed_kind: HeroKind,
    previous_ultimate_remaining: f32,
    sequence: Option<UltimateSequence>,
    storm_spell_signals: u8,
    ember_stacks: u8,
    ember_decay_timer: f32,
    ember_multiplier: f32,
    passive_timer: f32,
    star_kill_progress: u32,
    last_total_kills: u32,
    last_core_hp: f32,
}

impl Default for HeroAbilityRuntime {
    fn default() -> Self {
        Self {
            observed_kind: HeroKind::StormSavant,
            previous_ultimate_remaining: 0.0,
            sequence: None,
            storm_spell_signals: 0,
            ember_stacks: 0,
            ember_decay_timer: 0.0,
            ember_multiplier: 1.0,
            passive_timer: 0.0,
            star_kill_progress: 0,
            last_total_kills: 0,
            last_core_hp: GameSession::default().core_hp,
        }
    }
}

#[derive(Component)]
pub(crate) struct AbilityGenerated;

#[derive(Component)]
pub(crate) struct TimedSpeedEffect {
    pub(crate) original_speed: f32,
    pub(crate) remaining: f32,
}

#[derive(Component)]
pub(crate) struct WetMark {
    pub(crate) remaining: f32,
}

#[derive(Component)]
pub(crate) struct SpiritOrbit {
    pub(crate) angle: f32,
    pub(crate) radius: f32,
    pub(crate) remaining: Option<f32>,
    pub(crate) shot_timer: f32,
    pub(crate) damage_multiplier: f32,
}

#[derive(Component)]
pub(crate) struct PassiveSpirit;

fn attach_default_identity(
    mut commands: Commands,
    selected: Res<SelectedHero>,
    heroes: Query<Entity, (With<Hero>, Without<HeroIdentity>)>,
) {
    for entity in &heroes {
        commands.entity(entity).insert(HeroIdentity(selected.kind));
    }
}

fn synchronize_identity(
    mut commands: Commands,
    selected: Res<SelectedHero>,
    mut heroes: Query<(Entity, Option<&mut HeroIdentity>), With<Hero>>,
) {
    for (entity, identity) in &mut heroes {
        if let Some(mut identity) = identity {
            identity.0 = selected.kind;
        } else {
            commands.entity(entity).insert(HeroIdentity(selected.kind));
        }
    }
}

fn configure_ultimate(selected: Res<SelectedHero>, mut active: ResMut<ActiveSkill>) {
    active.cooldown = ultimate_profile(selected.kind).cooldown_secs;
}

fn tick_speed_effects(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    mut enemies: Query<(Entity, &mut Enemy, &mut TimedSpeedEffect)>,
) {
    if session.paused || session.finished {
        return;
    }
    for (entity, mut enemy, mut effect) in &mut enemies {
        effect.remaining -= time.delta_secs();
        if effect.remaining <= 0.0 {
            enemy.speed = effect.original_speed;
            commands.entity(entity).remove::<TimedSpeedEffect>();
        }
    }
}

fn tick_wet_marks(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    mut marks: Query<(Entity, &mut WetMark)>,
) {
    if session.paused || session.finished {
        return;
    }
    for (entity, mut mark) in &mut marks {
        mark.remaining -= time.delta_secs();
        if mark.remaining <= 0.0 {
            commands.entity(entity).remove::<WetMark>();
        }
    }
}

fn observe_projectile_passives(
    mut commands: Commands,
    selected: Res<SelectedHero>,
    vitals: Res<HeroVitals>,
    assets: Option<Res<GameAssets>>,
    mut build: ResMut<PlayerBuild>,
    hero: Query<&Transform, With<Hero>>,
    enemies: Query<(&Transform, Option<&BossEnemy>), With<Enemy>>,
    projectiles: Query<
        (&Projectile, Option<&DamageProfile>),
        (Added<Projectile>, Without<AbilityGenerated>),
    >,
    mut runtime: ResMut<HeroAbilityRuntime>,
    mut pools: ResMut<CombatPools>,
) {
    if !vitals.can_act() {
        return;
    }
    let mut external_shots = 0_u8;
    let mut spell_signals = 0_u8;
    for (_, profile) in &projectiles {
        external_shots = external_shots.saturating_add(1);
        if is_storm_spell_signal(profile) {
            spell_signals = spell_signals.saturating_add(1);
        }
    }
    if external_shots == 0 {
        return;
    }

    match selected.kind {
        HeroKind::StormSavant => {
            if spell_signals == 0 {
                return;
            }
            runtime.storm_spell_signals = runtime.storm_spell_signals.saturating_add(spell_signals);
            if runtime.storm_spell_signals < 6 {
                return;
            }
            runtime.storm_spell_signals -= 6;
            let (Some(assets), Ok(hero), Some(target)) = (
                assets.as_deref(),
                hero.single(),
                preferred_target(hero.single().ok(), &enemies),
            ) else {
                return;
            };
            spawn_projectile_fan(
                &mut commands,
                &mut pools,
                assets,
                HeroKind::StormSavant,
                hero.translation,
                target,
                1,
                build.attack_damage * 0.4,
                &build,
            );
            spawn_ability_fx(
                &mut commands,
                &mut pools,
                assets,
                HeroKind::StormSavant,
                target,
                1.4,
                0.26,
            );
        }
        HeroKind::EmberRanger => {
            runtime.ember_stacks = runtime.ember_stacks.saturating_add(1).min(12);
            runtime.ember_decay_timer = 0.0;
            let next_multiplier = ember_attack_speed_multiplier(runtime.ember_stacks);
            build.attacks_per_second *= next_multiplier / runtime.ember_multiplier.max(0.01);
            runtime.ember_multiplier = next_multiplier;
            if let (Some(assets), Ok(hero)) = (assets.as_deref(), hero.single()) {
                spawn_ability_fx(
                    &mut commands,
                    &mut pools,
                    assets,
                    HeroKind::EmberRanger,
                    hero.translation,
                    0.7 + runtime.ember_stacks as f32 * 0.04,
                    0.18,
                );
            }
        }
        _ => {}
    }
}

fn maintain_passive_state(
    mut commands: Commands,
    time: Res<Time>,
    selected: Res<SelectedHero>,
    vitals: Res<HeroVitals>,
    assets: Option<Res<GameAssets>>,
    intent: Res<MoveIntent>,
    mut session: ResMut<GameSession>,
    mut build: ResMut<PlayerBuild>,
    hero: Query<&Transform, With<Hero>>,
    core: Query<&Transform, (With<Core>, Without<Hero>)>,
    passive_spirits: Query<Entity, With<PassiveSpirit>>,
    player_summons: Query<(), With<PlayerSummon>>,
    market_effects: Res<MarketEffects>,
    mut runtime: ResMut<HeroAbilityRuntime>,
    mut pools: ResMut<CombatPools>,
) {
    if session.paused || session.finished || !vitals.can_act() {
        runtime.last_core_hp = session.core_hp;
        runtime.last_total_kills = session.total_kills;
        return;
    }

    if runtime.observed_kind != selected.kind {
        if runtime.ember_multiplier > 0.01 {
            build.attacks_per_second /= runtime.ember_multiplier;
        }
        runtime.observed_kind = selected.kind;
        runtime.sequence = None;
        runtime.storm_spell_signals = 0;
        runtime.ember_stacks = 0;
        runtime.ember_decay_timer = 0.0;
        runtime.ember_multiplier = 1.0;
        runtime.passive_timer = 0.0;
        runtime.star_kill_progress = 0;
        runtime.last_total_kills = session.total_kills;
        runtime.last_core_hp = session.core_hp;
    }

    match selected.kind {
        HeroKind::EmberRanger => {
            if intent.0.length_squared() > 0.04 && runtime.ember_stacks > 0 {
                runtime.ember_decay_timer += time.delta_secs();
                if runtime.ember_decay_timer >= 0.55 {
                    runtime.ember_decay_timer -= 0.55;
                    runtime.ember_stacks -= 1;
                    let next_multiplier = ember_attack_speed_multiplier(runtime.ember_stacks);
                    build.attacks_per_second *=
                        next_multiplier / runtime.ember_multiplier.max(0.01);
                    runtime.ember_multiplier = next_multiplier;
                }
            }
        }
        HeroKind::StoneWarden => {
            runtime.passive_timer -= time.delta_secs();
            let loss = (runtime.last_core_hp - session.core_hp).max(0.0);
            let in_guard_range = hero
                .single()
                .ok()
                .zip(core.single().ok())
                .is_some_and(|(hero, core)| hero.translation.distance(core.translation) <= 8.0);
            if in_guard_range && loss > 0.0 {
                session.core_hp =
                    (session.core_hp + stone_guard_refund(loss)).min(session.core_max_hp);
            }
            if in_guard_range && runtime.passive_timer <= 0.0 {
                if let (Some(assets), Ok(core)) = (assets.as_deref(), core.single()) {
                    spawn_ability_fx(
                        &mut commands,
                        &mut pools,
                        assets,
                        HeroKind::StoneWarden,
                        core.translation,
                        2.3,
                        0.38,
                    );
                }
                runtime.passive_timer = 2.4;
            }
        }
        HeroKind::SpiritKeeper => {
            if passive_spirits.is_empty()
                && player_summon_slots(
                    player_summons
                        .iter()
                        .count()
                        .max(pools.summon_active_count()),
                    build.bond(BondId::Spirits),
                    market_effects.summon_cap_bonus(),
                ) > 0
                && let (Some(assets), Ok(hero)) = (assets.as_deref(), hero.single())
            {
                spawn_spirit(
                    &mut commands,
                    &mut pools,
                    assets,
                    hero.translation,
                    0.0,
                    1.35,
                    None,
                    1.10,
                    true,
                );
            }
        }
        HeroKind::StarWagerer => {
            let gained = session.total_kills.saturating_sub(runtime.last_total_kills);
            runtime.star_kill_progress = runtime.star_kill_progress.saturating_add(gained);
            while runtime.star_kill_progress >= 12 {
                runtime.star_kill_progress -= 12;
                session.gold = session.gold.saturating_add(18);
                session.wood = session.wood.saturating_add(1);
                if let (Some(assets), Ok(hero)) = (assets.as_deref(), hero.single()) {
                    spawn_ability_fx(
                        &mut commands,
                        &mut pools,
                        assets,
                        HeroKind::StarWagerer,
                        hero.translation,
                        1.5,
                        0.3,
                    );
                }
            }
        }
        HeroKind::StormSavant | HeroKind::TideWalker => {}
    }

    runtime.last_core_hp = session.core_hp;
    runtime.last_total_kills = session.total_kills;
}

fn tick_tide_passive(
    mut commands: Commands,
    time: Res<Time>,
    selected: Res<SelectedHero>,
    vitals: Res<HeroVitals>,
    session: Res<GameSession>,
    build: Res<PlayerBuild>,
    assets: Option<Res<GameAssets>>,
    hero: Query<&Transform, With<Hero>>,
    mut enemies: ParamSet<(
        Query<(Entity, &Transform), With<Enemy>>,
        Query<
            (
                &mut Enemy,
                &Transform,
                Option<&BossEnemy>,
                Option<&WetMark>,
                Option<&TimedSpeedEffect>,
            ),
            With<Enemy>,
        >,
    )>,
    mut runtime: ResMut<HeroAbilityRuntime>,
    mut pools: ResMut<CombatPools>,
) {
    if selected.kind != HeroKind::TideWalker
        || session.paused
        || session.finished
        || !vitals.can_act()
    {
        return;
    }
    runtime.passive_timer -= time.delta_secs();
    if runtime.passive_timer > 0.0 {
        return;
    }
    runtime.passive_timer = 3.2;

    let Ok(hero) = hero.single() else {
        return;
    };
    let target = enemies
        .p0()
        .iter()
        .filter_map(|(entity, transform)| {
            let distance = hero.translation.distance_squared(transform.translation);
            (distance <= 144.0).then_some((entity, distance))
        })
        .min_by(|left, right| left.1.partial_cmp(&right.1).unwrap_or(Ordering::Equal))
        .map(|(entity, _)| entity);
    let Some(target) = target else {
        return;
    };

    let mut mutable_enemies = enemies.p1();
    let Ok((mut enemy, transform, boss, wet, speed_effect)) = mutable_enemies.get_mut(target)
    else {
        return;
    };
    let position = transform.translation;
    if wet.is_some() {
        let original_speed = speed_effect
            .map(|effect| effect.original_speed)
            .unwrap_or(enemy.speed);
        let multiplier = if boss.is_some() { 0.4 } else { 0.0 };
        enemy.speed = original_speed * multiplier;
        commands.entity(target).insert(TimedSpeedEffect {
            original_speed,
            remaining: if boss.is_some() { 1.0 } else { 1.2 },
        });
        commands.entity(target).remove::<WetMark>();
        if let Some(assets) = assets.as_deref() {
            spawn_damage_zone(
                &mut commands,
                &mut pools,
                assets,
                HeroKind::TideWalker,
                position,
                build.attack_damage * 0.55,
                2.1,
            );
        }
    } else {
        commands.entity(target).insert(WetMark { remaining: 6.0 });
        if let Some(assets) = assets.as_deref() {
            spawn_ability_fx(
                &mut commands,
                &mut pools,
                assets,
                HeroKind::TideWalker,
                position,
                1.4,
                0.3,
            );
        }
    }
}

fn observe_ultimate_cast(
    mut commands: Commands,
    selected: Res<SelectedHero>,
    vitals: Res<HeroVitals>,
    active: Res<ActiveSkill>,
    assets: Option<Res<GameAssets>>,
    mut session: ResMut<GameSession>,
    build: Res<PlayerBuild>,
    market_effects: Res<MarketEffects>,
    hero: Query<&Transform, With<Hero>>,
    player_summons: Query<(), With<PlayerSummon>>,
    mut enemies: Query<
        (
            Entity,
            &mut Enemy,
            &Transform,
            Option<&BossEnemy>,
            Option<&TimedSpeedEffect>,
        ),
        With<Enemy>,
    >,
    mut runtime: ResMut<HeroAbilityRuntime>,
    mut pools: ResMut<CombatPools>,
) {
    let cast_started = did_ultimate_start(runtime.previous_ultimate_remaining, active.remaining);
    runtime.previous_ultimate_remaining = active.remaining;
    if !cast_started || session.paused || session.finished || !vitals.can_act() {
        return;
    }
    let (Some(assets), Ok(hero)) = (assets.as_deref(), hero.single()) else {
        return;
    };

    spawn_ability_fx(
        &mut commands,
        &mut pools,
        assets,
        selected.kind,
        hero.translation,
        4.5,
        0.55,
    );

    match selected.kind {
        HeroKind::StormSavant => {
            runtime.sequence = Some(UltimateSequence {
                kind: selected.kind,
                pulses_left: 10,
                pulse_timer: 0.0,
                pulse_interval: 0.6,
            });
        }
        HeroKind::EmberRanger => {
            runtime.sequence = Some(UltimateSequence {
                kind: selected.kind,
                pulses_left: 5,
                pulse_timer: 0.0,
                pulse_interval: 0.34,
            });
        }
        HeroKind::StoneWarden => {
            for (entity, mut enemy, transform, _, speed_effect) in &mut enemies {
                if hero.translation.distance(transform.translation) > 7.5 {
                    continue;
                }
                let original_speed = speed_effect
                    .map(|effect| effect.original_speed)
                    .unwrap_or(enemy.speed);
                enemy.speed = original_speed * 0.28;
                commands.entity(entity).insert(TimedSpeedEffect {
                    original_speed,
                    remaining: 6.0,
                });
            }
            session.core_hp = (session.core_hp + 90.0).min(session.core_max_hp);
            runtime.sequence = Some(UltimateSequence {
                kind: selected.kind,
                pulses_left: 6,
                pulse_timer: 0.0,
                pulse_interval: 1.0,
            });
        }
        HeroKind::SpiritKeeper => {
            let available = player_summon_slots(
                player_summons
                    .iter()
                    .count()
                    .max(pools.summon_active_count()),
                build.bond(BondId::Spirits),
                market_effects.summon_cap_bonus(),
            );
            for spirit in 0..6.min(available) {
                let angle = spirit as f32 / 6.0 * std::f32::consts::TAU;
                spawn_spirit(
                    &mut commands,
                    &mut pools,
                    assets,
                    hero.translation,
                    angle,
                    2.0,
                    Some(12.0),
                    0.72,
                    false,
                );
            }
        }
        HeroKind::TideWalker => {
            for (entity, mut enemy, _, boss, speed_effect) in &mut enemies {
                let original_speed = speed_effect
                    .map(|effect| effect.original_speed)
                    .unwrap_or(enemy.speed);
                let (multiplier, remaining) = if boss.is_some() {
                    (0.4, 5.0)
                } else {
                    (0.0, 3.0)
                };
                enemy.speed = original_speed * multiplier;
                commands.entity(entity).insert(TimedSpeedEffect {
                    original_speed,
                    remaining,
                });
            }
        }
        HeroKind::StarWagerer => {
            let enemy_count = enemies.iter().count();
            let core_ratio = session.core_hp / session.core_max_hp.max(1.0);
            match choose_fate_outcome(core_ratio, enemy_count) {
                FateOutcome::Damage => {
                    let target = enemies
                        .iter()
                        .map(|(_, _, transform, boss, _)| {
                            (
                                transform.translation,
                                boss.is_some(),
                                hero.translation.distance_squared(transform.translation),
                            )
                        })
                        .min_by(|left, right| {
                            right.1.cmp(&left.1).then_with(|| {
                                left.2.partial_cmp(&right.2).unwrap_or(Ordering::Equal)
                            })
                        })
                        .map(|(position, _, _)| position)
                        .unwrap_or(hero.translation);
                    spawn_damage_zone(
                        &mut commands,
                        &mut pools,
                        assets,
                        HeroKind::StarWagerer,
                        target,
                        build.attack_damage * 6.0 + session.level as f32 * 20.0,
                        7.0,
                    );
                }
                FateOutcome::Healing => {
                    session.core_hp = (session.core_hp + 240.0 + session.level as f32 * 10.0)
                        .min(session.core_max_hp);
                    spawn_ability_fx(
                        &mut commands,
                        &mut pools,
                        assets,
                        HeroKind::StarWagerer,
                        hero.translation,
                        5.5,
                        0.8,
                    );
                }
                FateOutcome::Resources => {
                    session.gold = session.gold.saturating_add(180);
                    session.wood = session.wood.saturating_add(15);
                    session.kills = session.kills.saturating_add(30);
                    spawn_ability_fx(
                        &mut commands,
                        &mut pools,
                        assets,
                        HeroKind::StarWagerer,
                        hero.translation,
                        3.4,
                        0.8,
                    );
                }
            }
        }
    }
}

fn tick_ultimate_sequence(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    vitals: Res<HeroVitals>,
    build: Res<PlayerBuild>,
    assets: Option<Res<GameAssets>>,
    hero: Query<&Transform, With<Hero>>,
    enemies: Query<(&Transform, Option<&BossEnemy>), With<Enemy>>,
    mut runtime: ResMut<HeroAbilityRuntime>,
    mut pools: ResMut<CombatPools>,
) {
    if session.paused || session.finished {
        return;
    }
    if !vitals.can_act() {
        runtime.sequence = None;
        return;
    }
    let Some(mut sequence) = runtime.sequence else {
        return;
    };
    let (Some(assets), Ok(hero)) = (assets.as_deref(), hero.single()) else {
        return;
    };

    sequence.pulse_timer -= time.delta_secs();
    if sequence.pulse_timer > 0.0 {
        runtime.sequence = Some(sequence);
        return;
    }

    match sequence.kind {
        HeroKind::StormSavant => {
            if let Some(target) = preferred_target(Some(hero), &enemies) {
                spawn_damage_zone(
                    &mut commands,
                    &mut pools,
                    assets,
                    sequence.kind,
                    target,
                    45.0 + session.level as f32 * 5.0 + build.attack_damage * 0.45,
                    2.5,
                );
            }
        }
        HeroKind::EmberRanger => {
            if let Some(target) = preferred_target(Some(hero), &enemies) {
                spawn_projectile_fan(
                    &mut commands,
                    &mut pools,
                    assets,
                    sequence.kind,
                    hero.translation,
                    target,
                    5,
                    build.attack_damage * 0.72,
                    &build,
                );
            }
        }
        HeroKind::StoneWarden => {
            spawn_ability_fx(
                &mut commands,
                &mut pools,
                assets,
                sequence.kind,
                hero.translation,
                4.2,
                0.72,
            );
        }
        HeroKind::SpiritKeeper | HeroKind::TideWalker | HeroKind::StarWagerer => {}
    }

    sequence.pulses_left = sequence.pulses_left.saturating_sub(1);
    sequence.pulse_timer += sequence.pulse_interval;
    runtime.sequence = (sequence.pulses_left > 0).then_some(sequence);
}

fn cleanup_spirit_summons(
    mut commands: Commands,
    session: Res<GameSession>,
    selected: Res<SelectedHero>,
    spirits: Query<Entity, With<SpiritOrbit>>,
    mut pools: ResMut<CombatPools>,
) {
    if !session.finished && !selected.is_changed() {
        return;
    }
    for entity in &spirits {
        pools.retire_summon(&mut commands, entity);
    }
}

fn update_spirit_companions(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    vitals: Res<HeroVitals>,
    build: Res<PlayerBuild>,
    assets: Option<Res<GameAssets>>,
    hero: Query<&Transform, (With<Hero>, Without<SpiritOrbit>)>,
    enemies: Query<&Transform, (With<Enemy>, Without<SpiritOrbit>)>,
    mut spirits: Query<(Entity, &mut Transform, &mut SpiritOrbit), Without<Hero>>,
    mut pools: ResMut<CombatPools>,
) {
    if session.paused || session.finished || !vitals.can_act() {
        return;
    }
    let (Some(assets), Ok(hero)) = (assets.as_deref(), hero.single()) else {
        return;
    };

    for (entity, mut transform, mut spirit) in &mut spirits {
        if let Some(remaining) = &mut spirit.remaining {
            *remaining -= time.delta_secs();
            if *remaining <= 0.0 {
                pools.retire_summon(&mut commands, entity);
                continue;
            }
        }
        spirit.angle = (spirit.angle + time.delta_secs() * 1.7) % std::f32::consts::TAU;
        transform.translation = hero.translation
            + Vec3::new(
                spirit.angle.cos() * spirit.radius,
                0.75 + (spirit.angle * 2.0).sin() * 0.16,
                spirit.angle.sin() * spirit.radius,
            );
        spirit.shot_timer -= time.delta_secs();
        if spirit.shot_timer > 0.0 {
            continue;
        }
        let target = enemies
            .iter()
            .min_by(|left, right| {
                transform
                    .translation
                    .distance_squared(left.translation)
                    .partial_cmp(&transform.translation.distance_squared(right.translation))
                    .unwrap_or(Ordering::Equal)
            })
            .map(|target| target.translation);
        if let Some(target) = target {
            spawn_projectile_fan(
                &mut commands,
                &mut pools,
                assets,
                HeroKind::SpiritKeeper,
                transform.translation,
                target,
                1,
                build.attack_damage * spirit.damage_multiplier,
                &build,
            );
            spirit.shot_timer = 2.2 + spirit.angle.rem_euclid(0.35);
        } else {
            spirit.shot_timer = 0.3;
        }
    }
}

fn update_active_skill_label(
    selected: Res<SelectedHero>,
    active: Res<ActiveSkill>,
    mut labels: Query<&mut Text, With<SkillButtonLabel>>,
) {
    let profile = ultimate_profile(selected.kind);
    for mut label in &mut labels {
        **label = if active.remaining <= 0.0 {
            format!("{} READY\nSPACE / TAP", profile.name)
        } else {
            format!("{}\n{:.1}s", profile.name, active.remaining)
        };
    }
}

fn preferred_target(
    hero: Option<&Transform>,
    enemies: &Query<(&Transform, Option<&BossEnemy>), With<Enemy>>,
) -> Option<Vec3> {
    let hero = hero?;
    enemies
        .iter()
        .map(|(transform, boss)| {
            (
                transform.translation,
                boss.is_some(),
                hero.translation.distance_squared(transform.translation),
            )
        })
        .min_by(|left, right| {
            right
                .1
                .cmp(&left.1)
                .then_with(|| left.2.partial_cmp(&right.2).unwrap_or(Ordering::Equal))
        })
        .map(|(position, _, _)| position)
}

#[allow(clippy::too_many_arguments)]
fn spawn_projectile_fan(
    commands: &mut Commands,
    pools: &mut CombatPools,
    assets: &GameAssets,
    kind: HeroKind,
    origin: Vec3,
    target: Vec3,
    count: u8,
    damage: f32,
    build: &PlayerBuild,
) {
    let direction = (target - origin).normalize_or_zero();
    for shot in 0..count.max(1) {
        let offset = shot as f32 - (count.max(1) as f32 - 1.0) * 0.5;
        let shot_direction = Quat::from_rotation_y(offset * 0.115) * direction;
        let Some(entity) = pools.rent_projectile(
            commands,
            assets.projectile_mesh.clone(),
            assets.hero_materials[kind.index()].clone(),
            Transform::from_translation(origin + Vec3::Y * 0.35)
                .with_scale(Vec3::splat(if count > 1 { 0.9 } else { 1.25 })),
            Projectile {
                velocity: shot_direction * build.projectile_speed * 1.05,
                damage,
                radius: 0.55,
                remaining_hits: build.pierce,
                lifetime: 2.0,
            },
        ) else {
            break;
        };
        commands.entity(entity).insert(AbilityGenerated);
    }
}

fn spawn_damage_zone(
    commands: &mut Commands,
    pools: &mut CombatPools,
    assets: &GameAssets,
    kind: HeroKind,
    center: Vec3,
    damage: f32,
    radius: f32,
) {
    let Some(entity) = pools.rent_projectile(
        commands,
        assets.projectile_mesh.clone(),
        assets.hero_materials[kind.index()].clone(),
        Transform::from_translation(center + Vec3::Y * 0.18).with_scale(Vec3::splat(0.35)),
        Projectile {
            velocity: Vec3::ZERO,
            damage,
            radius,
            remaining_hits: u8::MAX,
            lifetime: 0.05,
        },
    ) else {
        return;
    };
    commands.entity(entity).insert(AbilityGenerated);
    spawn_ability_fx(commands, pools, assets, kind, center, radius, 0.38);
}

fn spawn_ability_fx(
    commands: &mut Commands,
    pools: &mut CombatPools,
    assets: &GameAssets,
    kind: HeroKind,
    center: Vec3,
    radius: f32,
    lifetime: f32,
) {
    pools.rent_impact_fx(
        commands,
        assets.fx_mesh.clone(),
        assets.hero_materials[kind.index()].clone(),
        Transform::from_xyz(center.x, 0.2, center.z).with_scale(Vec3::splat(0.18)),
        ImpactFx {
            lifetime,
            start_scale: 0.18,
            end_scale: radius,
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn spawn_spirit(
    commands: &mut Commands,
    pools: &mut CombatPools,
    assets: &GameAssets,
    origin: Vec3,
    angle: f32,
    radius: f32,
    remaining: Option<f32>,
    damage_multiplier: f32,
    passive: bool,
) -> Option<Entity> {
    let entity = pools.rent_mesh_summon(
        commands,
        SummonVisual::SpiritOrbit,
        assets.projectile_mesh.clone(),
        assets.hero_materials[HeroKind::SpiritKeeper.index()].clone(),
        Transform::from_translation(origin + Vec3::Y * 0.75).with_scale(Vec3::splat(2.3)),
    )?;
    commands.entity(entity).insert(SpiritOrbit {
        angle,
        radius,
        remaining,
        shot_timer: angle.rem_euclid(1.1),
        damage_multiplier,
    });
    if passive {
        commands.entity(entity).insert(PassiveSpirit);
    }
    Some(entity)
}

const fn player_summon_slots(existing: usize, spirit_points: u8, equipment_bonus: u8) -> usize {
    summon_cap(spirit_points, equipment_bonus).saturating_sub(existing)
}

const fn ember_attack_speed_multiplier(stacks: u8) -> f32 {
    let capped = if stacks < 12 { stacks } else { 12 };
    1.0 + capped as f32 * 0.025
}

fn is_storm_spell_signal(profile: Option<&DamageProfile>) -> bool {
    profile.is_some_and(|profile| profile.class == DamageClass::Spell)
}

const fn stone_guard_refund(incoming_damage: f32) -> f32 {
    incoming_damage * 0.15
}

fn did_ultimate_start(previous_remaining: f32, current_remaining: f32) -> bool {
    current_remaining > previous_remaining + 0.25
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FateOutcome {
    Damage,
    Healing,
    Resources,
}

const fn choose_fate_outcome(core_ratio: f32, enemy_count: usize) -> FateOutcome {
    if core_ratio < 0.65 {
        FateOutcome::Healing
    } else if enemy_count >= 8 {
        FateOutcome::Damage
    } else {
        FateOutcome::Resources
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::game::content::HEROES;

    #[test]
    fn hero_identity_matches_catalog_order() {
        let expected = [
            "storm_savant",
            "ember_ranger",
            "stone_warden",
            "spirit_keeper",
            "tide_walker",
            "star_wagerer",
        ];
        for (index, kind) in HeroKind::ALL.into_iter().enumerate() {
            assert_eq!(kind.index(), index);
            assert_eq!(HEROES[kind.index()].id, expected[index]);
            assert_eq!(HeroKind::from_index(index), kind);
        }
    }

    #[test]
    fn all_heroes_have_named_ultimate_profiles() {
        let mut names = HashSet::new();
        let cooldowns: Vec<f32> = HeroKind::ALL
            .into_iter()
            .map(|kind| {
                let profile = ultimate_profile(kind);
                assert!(names.insert(profile.name));
                profile.cooldown_secs
            })
            .collect();
        assert_eq!(cooldowns, [50.0, 45.0, 50.0, 55.0, 52.0, 48.0]);
    }

    #[test]
    fn active_cast_edge_is_detected_once() {
        assert!(did_ultimate_start(0.0, 50.0));
        assert!(!did_ultimate_start(50.0, 49.98));
        assert!(!did_ultimate_start(0.0, 0.0));
    }

    #[test]
    fn spirit_keeper_passive_and_ultimate_share_the_global_summon_slots() {
        assert_eq!(player_summon_slots(17, 0, 0), 1);
        assert_eq!(player_summon_slots(18, 0, 0), 0);
        assert_eq!(player_summon_slots(18, 2, 0), 2);
        assert_eq!(player_summon_slots(18, 0, 1), 1);
        assert_eq!(player_summon_slots(18, 2, 1), 2);
        assert_eq!(player_summon_slots(20, 2, 1), 0);
    }

    #[test]
    fn ember_passive_stacks_and_caps_deterministically() {
        assert_eq!(ember_attack_speed_multiplier(0), 1.0);
        assert_eq!(ember_attack_speed_multiplier(4), 1.1);
        assert_eq!(ember_attack_speed_multiplier(12), 1.3);
        assert_eq!(ember_attack_speed_multiplier(200), 1.3);
    }

    #[test]
    fn storm_passive_counts_only_spell_projectiles() {
        let spell = DamageProfile::new(DamageElement::Storm, DamageClass::Spell);
        let attack = DamageProfile::PHYSICAL_ATTACK;
        assert!(is_storm_spell_signal(Some(&spell)));
        assert!(!is_storm_spell_signal(Some(&attack)));
        assert!(!is_storm_spell_signal(None));
    }

    #[test]
    fn stone_guard_refunds_fifteen_percent() {
        assert!((stone_guard_refund(100.0) - 15.0).abs() < 0.0001);
        assert!((stone_guard_refund(26.0) - 3.9).abs() < 0.0001);
    }

    #[test]
    fn loaded_fate_uses_stable_priority_rules() {
        assert_eq!(choose_fate_outcome(0.4, 30), FateOutcome::Healing);
        assert_eq!(choose_fate_outcome(0.8, 8), FateOutcome::Damage);
        assert_eq!(choose_fate_outcome(0.8, 7), FateOutcome::Resources);
    }
}
