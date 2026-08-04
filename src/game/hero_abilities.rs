use std::cmp::Ordering;

use bevy::prelude::*;

use super::bonds::{DamageClass, DamageElement, DamageProfile, summon_cap};
use super::combat::CombatSet;
use super::content::HEROES;
use super::market::MarketEffects;
use super::pooling::{CombatPools, SummonVisual};
use super::{
    ActiveSkill, BondId, BossEnemy, Core, Enemy, GameAssets, GameSession, Hero, HeroIdentity,
    HeroKind, HeroVitals, ImpactFx, MoveIntent, PlayerBuild, PlayerSummon, Projectile,
    SelectedHero,
};

pub struct HeroAbilitiesPlugin;

impl Plugin for HeroAbilitiesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HeroAbilityRuntime>()
            .init_resource::<HeroAttackSpeedModifier>()
            .init_resource::<BasicAttackCounter>()
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
                    observe_basic_attack_passives,
                    observe_projectile_passives,
                    maintain_passive_state,
                    tick_tide_passive,
                    observe_ultimate_cast,
                )
                    .chain(),
            );
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct UltimateProfile {
    pub(crate) name: &'static str,
    pub(crate) cooldown_secs: f32,
}

pub(crate) const fn ultimate_profile(kind: HeroKind) -> UltimateProfile {
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
        HeroKind::FrostOracle => UltimateProfile {
            name: "ABSOLUTE ZERO",
            cooldown_secs: 54.0,
        },
        HeroKind::IronBerserker => UltimateProfile {
            name: "IRON RAMPAGE",
            cooldown_secs: 48.0,
        },
        HeroKind::RiftDancer => UltimateProfile {
            name: "PHASE TEMPEST",
            cooldown_secs: 46.0,
        },
        HeroKind::GroveArtificer => UltimateProfile {
            name: "OVERDRIVE CITADEL",
            cooldown_secs: 58.0,
        },
        HeroKind::SolarVanguard => UltimateProfile {
            name: "DAWN BASTION",
            cooldown_secs: 52.0,
        },
        HeroKind::VoidReaper => UltimateProfile {
            name: "EVENT HORIZON",
            cooldown_secs: 50.0,
        },
    }
}

pub(crate) fn effective_ultimate_cooldown(kind: HeroKind) -> f32 {
    ultimate_profile(kind).cooldown_secs * HEROES[kind.index()].rarity.hero_cooldown_multiplier()
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
    frost_spell_signals: u8,
    void_attack_signals: u8,
    ember_stacks: u8,
    ember_decay_timer: f32,
    passive_timer: f32,
    star_kill_progress: u32,
    hero_kill_progress: u32,
    last_total_kills: u32,
    last_core_hp: f32,
    last_basic_attacks: u64,
}

impl Default for HeroAbilityRuntime {
    fn default() -> Self {
        Self {
            observed_kind: HeroKind::StormSavant,
            previous_ultimate_remaining: 0.0,
            sequence: None,
            storm_spell_signals: 0,
            frost_spell_signals: 0,
            void_attack_signals: 0,
            ember_stacks: 0,
            ember_decay_timer: 0.0,
            passive_timer: 0.0,
            star_kill_progress: 0,
            hero_kill_progress: 0,
            last_total_kills: 0,
            last_core_hp: GameSession::default().core_hp,
            last_basic_attacks: 0,
        }
    }
}

#[derive(Component)]
pub(crate) struct AbilityGenerated;

/// Counts completed basic-attack volleys directly at their source. This is
/// independent of projectile lifetime, so point-blank hits cannot lose a
/// passive trigger when the projectile is recycled in the same frame.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub(crate) struct BasicAttackCounter(pub(crate) u64);

#[derive(Resource, Debug, Clone, Copy)]
pub(crate) struct HeroAttackSpeedModifier(pub(crate) f32);

impl Default for HeroAttackSpeedModifier {
    fn default() -> Self {
        Self(1.0)
    }
}

#[derive(Component)]
pub(crate) struct TimedSpeedEffect {
    pub(crate) multiplier: f32,
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
    active.cooldown = effective_ultimate_cooldown(selected.kind);
}

fn tick_speed_effects(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    mut enemies: Query<(Entity, &mut TimedSpeedEffect)>,
) {
    if session.paused || session.finished {
        return;
    }
    for (entity, mut effect) in &mut enemies {
        effect.remaining -= time.delta_secs();
        if effect.remaining <= 0.0 {
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

#[allow(clippy::too_many_arguments)]
fn observe_basic_attack_passives(
    mut commands: Commands,
    selected: Res<SelectedHero>,
    vitals: Res<HeroVitals>,
    attacks: Res<BasicAttackCounter>,
    assets: Option<Res<GameAssets>>,
    build: Res<PlayerBuild>,
    mut attack_speed: ResMut<HeroAttackSpeedModifier>,
    hero: Query<&Transform, With<Hero>>,
    enemies: Query<(&Transform, Option<&BossEnemy>), With<Enemy>>,
    mut runtime: ResMut<HeroAbilityRuntime>,
    mut pools: ResMut<CombatPools>,
) {
    if runtime.observed_kind != selected.kind {
        runtime.last_basic_attacks = attacks.0;
        return;
    }

    let basic_attacks = attacks.0.wrapping_sub(runtime.last_basic_attacks);
    runtime.last_basic_attacks = attacks.0;
    if basic_attacks == 0 || !vitals.can_act() {
        return;
    }

    match selected.kind {
        HeroKind::EmberRanger => {
            let gained = basic_attacks.min(u8::MAX as u64) as u8;
            runtime.ember_stacks = runtime.ember_stacks.saturating_add(gained).min(12);
            runtime.ember_decay_timer = 0.0;
            attack_speed.0 = ember_attack_speed_multiplier(runtime.ember_stacks);
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
        HeroKind::VoidReaper => {
            let gained = basic_attacks.min(u8::MAX as u64) as u8;
            runtime.void_attack_signals = runtime.void_attack_signals.saturating_add(gained);
            while runtime.void_attack_signals >= 5 {
                runtime.void_attack_signals -= 5;
                let (Some(assets), Ok(hero), Some(target)) = (
                    assets.as_deref(),
                    hero.single(),
                    preferred_target(hero.single().ok(), &enemies),
                ) else {
                    continue;
                };
                spawn_projectile_fan(
                    &mut commands,
                    &mut pools,
                    assets,
                    HeroKind::VoidReaper,
                    hero.translation,
                    target,
                    2,
                    build.attack_damage * 0.45,
                    &build,
                );
            }
        }
        _ => {}
    }
}

fn observe_projectile_passives(
    mut commands: Commands,
    selected: Res<SelectedHero>,
    vitals: Res<HeroVitals>,
    assets: Option<Res<GameAssets>>,
    build: Res<PlayerBuild>,
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
    let mut spell_signals = 0_u8;
    for (_, profile) in &projectiles {
        if is_storm_spell_signal(profile) {
            spell_signals = spell_signals.saturating_add(1);
        }
    }
    if spell_signals == 0 {
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
        HeroKind::FrostOracle => {
            runtime.frost_spell_signals = runtime.frost_spell_signals.saturating_add(spell_signals);
            if runtime.frost_spell_signals < 5 {
                return;
            }
            runtime.frost_spell_signals -= 5;
            let (Some(assets), Some(target)) = (
                assets.as_deref(),
                preferred_target(hero.single().ok(), &enemies),
            ) else {
                return;
            };
            spawn_damage_zone(
                &mut commands,
                &mut pools,
                assets,
                HeroKind::FrostOracle,
                target,
                build.attack_damage * 0.55,
                2.4,
            );
        }
        _ => {}
    }
}

fn maintain_passive_state(
    mut commands: Commands,
    time: Res<Time>,
    selected: Res<SelectedHero>,
    mut vitals: ResMut<HeroVitals>,
    assets: Option<Res<GameAssets>>,
    intent: Res<MoveIntent>,
    mut session: ResMut<GameSession>,
    build: Res<PlayerBuild>,
    mut attack_speed: ResMut<HeroAttackSpeedModifier>,
    hero: Query<&Transform, With<Hero>>,
    core: Query<&Transform, (With<Core>, Without<Hero>)>,
    passive_spirits: Query<Entity, With<PassiveSpirit>>,
    player_summons: Query<(), With<PlayerSummon>>,
    market_effects: Res<MarketEffects>,
    mut runtime: ResMut<HeroAbilityRuntime>,
    mut pools: ResMut<CombatPools>,
) {
    if runtime.observed_kind != selected.kind {
        attack_speed.0 = 1.0;
        runtime.observed_kind = selected.kind;
        runtime.sequence = None;
        runtime.storm_spell_signals = 0;
        runtime.frost_spell_signals = 0;
        runtime.void_attack_signals = 0;
        runtime.ember_stacks = 0;
        runtime.ember_decay_timer = 0.0;
        runtime.passive_timer = 0.0;
        runtime.star_kill_progress = 0;
        runtime.hero_kill_progress = 0;
        runtime.last_total_kills = session.total_kills;
        runtime.last_core_hp = session.core_hp;
    }

    let gained = session.total_kills.saturating_sub(runtime.last_total_kills);
    match selected.kind {
        HeroKind::StarWagerer => {
            runtime.star_kill_progress = runtime.star_kill_progress.saturating_add(gained);
        }
        HeroKind::IronBerserker | HeroKind::SolarVanguard => {
            runtime.hero_kill_progress = runtime.hero_kill_progress.saturating_add(gained);
        }
        _ => {}
    }
    runtime.last_total_kills = session.total_kills;

    if session.paused || session.finished || !vitals.can_act() {
        runtime.last_core_hp = session.core_hp;
        return;
    }

    let ability_scale = selected.rarity().hero_ability_multiplier();

    match selected.kind {
        HeroKind::EmberRanger => {
            if intent.0.length_squared() > 0.04 && runtime.ember_stacks > 0 {
                runtime.ember_decay_timer += time.delta_secs();
                if runtime.ember_decay_timer >= 0.55 {
                    runtime.ember_decay_timer -= 0.55;
                    runtime.ember_stacks -= 1;
                    attack_speed.0 = ember_attack_speed_multiplier(runtime.ember_stacks);
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
                session.core_hp = (session.core_hp + stone_guard_refund(loss) * ability_scale)
                    .min(session.core_max_hp);
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
            while runtime.star_kill_progress >= 12 {
                runtime.star_kill_progress -= 12;
                session.gold = session
                    .gold
                    .saturating_add((18.0 * ability_scale).round() as u32);
                session.wood = session.wood.saturating_add((ability_scale).ceil() as u32);
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
        HeroKind::IronBerserker => {
            while runtime.hero_kill_progress >= 8 {
                runtime.hero_kill_progress -= 8;
                vitals.health = (vitals.health
                    + iron_berserker_kill_heal(vitals.max_health) * ability_scale)
                    .min(vitals.max_health);
                if let (Some(assets), Ok(hero)) = (assets.as_deref(), hero.single()) {
                    spawn_ability_fx(
                        &mut commands,
                        &mut pools,
                        assets,
                        HeroKind::IronBerserker,
                        hero.translation,
                        1.8,
                        0.32,
                    );
                }
            }
        }
        HeroKind::RiftDancer => {
            attack_speed.0 = rift_dancer_attack_speed_multiplier(intent.0.length_squared());
        }
        HeroKind::GroveArtificer => {
            let in_repair_range = hero
                .single()
                .ok()
                .zip(core.single().ok())
                .is_some_and(|(hero, core)| hero.translation.distance(core.translation) <= 9.0);
            if in_repair_range {
                runtime.passive_timer += time.delta_secs();
            } else {
                runtime.passive_timer = 0.0;
            }
            if runtime.passive_timer >= 6.0 {
                session.core_hp = (session.core_hp + 12.0 * ability_scale).min(session.core_max_hp);
                runtime.passive_timer -= 6.0;
                if let (Some(assets), Ok(core)) = (assets.as_deref(), core.single()) {
                    spawn_ability_fx(
                        &mut commands,
                        &mut pools,
                        assets,
                        HeroKind::GroveArtificer,
                        core.translation,
                        2.0,
                        0.38,
                    );
                }
            }
        }
        HeroKind::SolarVanguard => {
            while runtime.hero_kill_progress >= 10 {
                runtime.hero_kill_progress -= 10;
                session.core_hp = (session.core_hp + 24.0 * ability_scale).min(session.core_max_hp);
                if let (Some(assets), Ok(core)) = (assets.as_deref(), core.single()) {
                    spawn_ability_fx(
                        &mut commands,
                        &mut pools,
                        assets,
                        HeroKind::SolarVanguard,
                        core.translation,
                        2.2,
                        0.4,
                    );
                }
            }
        }
        HeroKind::StormSavant
        | HeroKind::TideWalker
        | HeroKind::FrostOracle
        | HeroKind::VoidReaper => {}
    }

    runtime.last_core_hp = session.core_hp;
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
    let Ok((_enemy, transform, boss, wet, speed_effect)) = mutable_enemies.get_mut(target) else {
        return;
    };
    let position = transform.translation;
    if wet.is_some() {
        let multiplier = if boss.is_some() { 0.4 } else { 0.0 };
        commands.entity(target).insert(merged_speed_effect(
            speed_effect,
            multiplier,
            if boss.is_some() { 1.0 } else { 1.2 },
        ));
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
    mut vitals: ResMut<HeroVitals>,
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
    let ability_scale = selected.rarity().hero_ability_multiplier();
    let control_scale = selected.rarity().hero_stat_multiplier();

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
            for (entity, _enemy, transform, _, speed_effect) in &mut enemies {
                if hero.translation.distance(transform.translation) > 7.5 {
                    continue;
                }
                commands.entity(entity).insert(merged_speed_effect(
                    speed_effect,
                    0.28,
                    6.0 * control_scale,
                ));
            }
            session.core_hp = (session.core_hp + 90.0 * ability_scale).min(session.core_max_hp);
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
            for (entity, _enemy, _, boss, speed_effect) in &mut enemies {
                let (multiplier, remaining) = if boss.is_some() {
                    (0.4, 5.0)
                } else {
                    (0.0, 3.0)
                };
                commands.entity(entity).insert(merged_speed_effect(
                    speed_effect,
                    multiplier,
                    remaining * control_scale,
                ));
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
                    session.core_hp = (session.core_hp
                        + (240.0 + session.level as f32 * 10.0) * ability_scale)
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
                    session.gold = session
                        .gold
                        .saturating_add((180.0 * ability_scale).round() as u32);
                    session.wood = session
                        .wood
                        .saturating_add((15.0 * ability_scale).round() as u32);
                    session.kills = session
                        .kills
                        .saturating_add((30.0 * ability_scale).round() as u32);
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
        HeroKind::FrostOracle => {
            for (entity, _enemy, _, boss, speed_effect) in &mut enemies {
                let (multiplier, remaining) = if boss.is_some() {
                    (0.5, 4.0)
                } else {
                    (0.0, 2.5)
                };
                commands.entity(entity).insert(merged_speed_effect(
                    speed_effect,
                    multiplier,
                    remaining * control_scale,
                ));
            }
            runtime.sequence = Some(UltimateSequence {
                kind: selected.kind,
                pulses_left: 4,
                pulse_timer: 0.0,
                pulse_interval: 0.55,
            });
        }
        HeroKind::IronBerserker => {
            let missing_health = (vitals.max_health - vitals.health).max(0.0);
            vitals.health =
                (vitals.health + missing_health * 0.25 * ability_scale).min(vitals.max_health);
            runtime.sequence = Some(UltimateSequence {
                kind: selected.kind,
                pulses_left: 6,
                pulse_timer: 0.0,
                pulse_interval: 0.32,
            });
        }
        HeroKind::RiftDancer => {
            runtime.sequence = Some(UltimateSequence {
                kind: selected.kind,
                pulses_left: 8,
                pulse_timer: 0.0,
                pulse_interval: 0.22,
            });
        }
        HeroKind::GroveArtificer => {
            session.core_hp = (session.core_hp + 160.0 * ability_scale).min(session.core_max_hp);
            runtime.sequence = Some(UltimateSequence {
                kind: selected.kind,
                pulses_left: 6,
                pulse_timer: 0.0,
                pulse_interval: 0.75,
            });
        }
        HeroKind::SolarVanguard => {
            session.core_hp = (session.core_hp + 120.0 * ability_scale).min(session.core_max_hp);
            vitals.health =
                (vitals.health + vitals.max_health * 0.25 * ability_scale).min(vitals.max_health);
            for (entity, _enemy, _, _, speed_effect) in &mut enemies {
                commands.entity(entity).insert(merged_speed_effect(
                    speed_effect,
                    0.75,
                    2.0 * control_scale,
                ));
            }
            runtime.sequence = Some(UltimateSequence {
                kind: selected.kind,
                pulses_left: 5,
                pulse_timer: 0.0,
                pulse_interval: 0.6,
            });
        }
        HeroKind::VoidReaper => {
            for (entity, _enemy, _, _, speed_effect) in &mut enemies {
                commands.entity(entity).insert(merged_speed_effect(
                    speed_effect,
                    0.55,
                    4.0 * control_scale,
                ));
            }
            runtime.sequence = Some(UltimateSequence {
                kind: selected.kind,
                pulses_left: 5,
                pulse_timer: 0.0,
                pulse_interval: 0.5,
            });
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
    core: Query<&Transform, (With<Core>, Without<Hero>)>,
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
        HeroKind::FrostOracle => {
            if let Some(target) = preferred_target(Some(hero), &enemies) {
                spawn_damage_zone(
                    &mut commands,
                    &mut pools,
                    assets,
                    sequence.kind,
                    target,
                    build.attack_damage * 0.6 + session.level as f32 * 5.0,
                    3.0,
                );
            }
        }
        HeroKind::IronBerserker => {
            spawn_damage_zone(
                &mut commands,
                &mut pools,
                assets,
                sequence.kind,
                hero.translation,
                build.attack_damage * 0.75,
                2.8,
            );
        }
        HeroKind::RiftDancer => {
            if let Some(target) = preferred_target(Some(hero), &enemies) {
                spawn_projectile_fan(
                    &mut commands,
                    &mut pools,
                    assets,
                    sequence.kind,
                    hero.translation,
                    target,
                    3,
                    build.attack_damage * 0.55,
                    &build,
                );
            }
        }
        HeroKind::GroveArtificer => {
            let center = core
                .single()
                .map(|core| core.translation)
                .unwrap_or(hero.translation);
            spawn_damage_zone(
                &mut commands,
                &mut pools,
                assets,
                sequence.kind,
                center,
                build.attack_damage * 0.65,
                5.0,
            );
        }
        HeroKind::SolarVanguard => {
            if let Some(target) = preferred_target(Some(hero), &enemies) {
                spawn_damage_zone(
                    &mut commands,
                    &mut pools,
                    assets,
                    sequence.kind,
                    target,
                    build.attack_damage * 0.8,
                    3.2,
                );
            }
        }
        HeroKind::VoidReaper => {
            if let Some(target) = preferred_target(Some(hero), &enemies) {
                spawn_damage_zone(
                    &mut commands,
                    &mut pools,
                    assets,
                    sequence.kind,
                    target,
                    build.attack_damage * 1.15,
                    4.0,
                );
            }
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
    let damage = scaled_hero_ability_damage(kind, damage);
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
        commands
            .entity(entity)
            .insert((AbilityGenerated, ability_damage_profile(kind)));
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
    let damage = scaled_hero_ability_damage(kind, damage);
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
    commands
        .entity(entity)
        .insert((AbilityGenerated, ability_damage_profile(kind)));
    spawn_ability_fx(commands, pools, assets, kind, center, radius, 0.38);
}

fn scaled_hero_ability_damage(kind: HeroKind, damage: f32) -> f32 {
    damage * HEROES[kind.index()].rarity.hero_ability_multiplier()
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

const fn iron_berserker_kill_heal(max_health: f32) -> f32 {
    max_health * 0.06
}

const fn rift_dancer_attack_speed_multiplier(movement_squared: f32) -> f32 {
    if movement_squared > 0.04 { 1.15 } else { 1.0 }
}

const fn ability_damage_profile(kind: HeroKind) -> DamageProfile {
    match kind {
        HeroKind::StormSavant => DamageProfile::new(DamageElement::Storm, DamageClass::Spell),
        HeroKind::EmberRanger => DamageProfile::new(DamageElement::Flame, DamageClass::Attack),
        HeroKind::StoneWarden => DamageProfile::new(DamageElement::Physical, DamageClass::Attack),
        HeroKind::SpiritKeeper => DamageProfile::new(DamageElement::Physical, DamageClass::Summon),
        HeroKind::TideWalker => DamageProfile::new(DamageElement::Frost, DamageClass::Spell),
        HeroKind::StarWagerer => DamageProfile::new(DamageElement::Arcane, DamageClass::Spell),
        HeroKind::FrostOracle => DamageProfile::new(DamageElement::Frost, DamageClass::Spell),
        HeroKind::IronBerserker => DamageProfile::new(DamageElement::Physical, DamageClass::Attack),
        HeroKind::RiftDancer => DamageProfile::new(DamageElement::Arcane, DamageClass::Attack),
        HeroKind::GroveArtificer => DamageProfile::new(DamageElement::Storm, DamageClass::Summon),
        HeroKind::SolarVanguard => DamageProfile::new(DamageElement::Flame, DamageClass::Spell),
        HeroKind::VoidReaper => DamageProfile::new(DamageElement::Arcane, DamageClass::Attack),
    }
}

fn merged_speed_effect(
    existing: Option<&TimedSpeedEffect>,
    multiplier: f32,
    remaining: f32,
) -> TimedSpeedEffect {
    TimedSpeedEffect {
        multiplier: existing.map_or(multiplier, |effect| effect.multiplier.min(multiplier)),
        remaining: existing.map_or(remaining, |effect| effect.remaining.max(remaining)),
    }
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
            "frost_oracle",
            "iron_berserker",
            "rift_dancer",
            "grove_artificer",
            "solar_vanguard",
            "void_reaper",
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
        assert_eq!(
            cooldowns,
            [
                50.0, 45.0, 50.0, 55.0, 52.0, 48.0, 54.0, 48.0, 46.0, 58.0, 52.0, 50.0,
            ]
        );
    }

    #[test]
    fn hero_rarity_scales_the_production_damage_and_ultimate_cooldown_paths() {
        for kind in HeroKind::ALL {
            let rarity = HEROES[kind.index()].rarity;
            let expected_cooldown =
                ultimate_profile(kind).cooldown_secs * rarity.hero_cooldown_multiplier();
            assert!((effective_ultimate_cooldown(kind) - expected_cooldown).abs() < 0.0001);
            assert!(
                (scaled_hero_ability_damage(kind, 100.0)
                    - 100.0 * rarity.hero_ability_multiplier())
                .abs()
                    < 0.0001
            );
        }
        assert!(
            effective_ultimate_cooldown(HeroKind::SpiritKeeper)
                < ultimate_profile(HeroKind::SpiritKeeper).cooldown_secs
        );
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
    fn new_sustain_and_movement_passives_have_exact_boundaries() {
        assert!((iron_berserker_kill_heal(1_000.0) - 60.0).abs() < 0.001);
        assert_eq!(rift_dancer_attack_speed_multiplier(0.04), 1.0);
        assert_eq!(rift_dancer_attack_speed_multiplier(0.041), 1.15);
        assert_eq!(HeroAttackSpeedModifier::default().0, 1.0);
    }

    #[test]
    fn every_hero_ability_carries_its_runtime_damage_semantics() {
        let expected = [
            (DamageElement::Storm, DamageClass::Spell),
            (DamageElement::Flame, DamageClass::Attack),
            (DamageElement::Physical, DamageClass::Attack),
            (DamageElement::Physical, DamageClass::Summon),
            (DamageElement::Frost, DamageClass::Spell),
            (DamageElement::Arcane, DamageClass::Spell),
            (DamageElement::Frost, DamageClass::Spell),
            (DamageElement::Physical, DamageClass::Attack),
            (DamageElement::Arcane, DamageClass::Attack),
            (DamageElement::Storm, DamageClass::Summon),
            (DamageElement::Flame, DamageClass::Spell),
            (DamageElement::Arcane, DamageClass::Attack),
        ];
        for (kind, expected) in HeroKind::ALL.into_iter().zip(expected) {
            let profile = ability_damage_profile(kind);
            assert_eq!((profile.element, profile.class), expected);
        }
    }

    #[test]
    fn overlapping_speed_effects_keep_the_strongest_slow_and_longest_time() {
        let existing = TimedSpeedEffect {
            multiplier: 0.5,
            remaining: 2.0,
        };
        let extended = merged_speed_effect(Some(&existing), 0.75, 4.0);
        assert_eq!(extended.multiplier, 0.5);
        assert_eq!(extended.remaining, 4.0);

        let strengthened = merged_speed_effect(Some(&existing), 0.2, 1.0);
        assert_eq!(strengthened.multiplier, 0.2);
        assert_eq!(strengthened.remaining, 2.0);
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
