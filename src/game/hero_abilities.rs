use std::cmp::Ordering;

use bevy::prelude::*;

use super::bonds::{DamageClass, DamageElement, DamageProfile, summon_cap};
use super::combat::CombatSet;
use super::combat::EliteAffix;
use super::content::HEROES;
use super::market::MarketEffects;
use super::pooling::{CombatPools, SummonVisual};
use super::ultimate_vfx::{UltimateCues, UltimatePhase};
use super::{
    ActiveSkill, BossEnemy, Core, Enemy, GameAssets, GameSession, Hero, HeroIdentity, HeroKind,
    HeroVitals, ImpactFx, MoveIntent, PlayerBuild, PlayerSummon, Projectile, SelectedHero,
};

#[path = "hero_abilities/ultimates.rs"]
mod ultimates;
#[cfg(test)]
use ultimates::automatic_ultimate_useful;
use ultimates::{observe_ultimate_cast, tick_ultimate_sequence};

pub struct HeroAbilitiesPlugin;

impl Plugin for HeroAbilitiesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HeroAbilityRuntime>()
            .init_resource::<HeroAttackSpeedModifier>()
            .init_resource::<BasicAttackCounter>()
            .add_systems(PostStartup, attach_default_identity)
            .add_systems(PreUpdate, tick_inherited_cooldowns)
            .add_systems(
                Update,
                (
                    synchronize_identity,
                    tick_speed_effects,
                    tick_wet_marks,
                    tick_void_pull,
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
                    configure_ultimate,
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
    target: Vec3,
    pulse_timer: f32,
    pulse_interval: f32,
}

#[derive(Resource)]
struct HeroAbilityRuntime {
    observed_kind: Option<HeroKind>,
    inherited: [bool; HeroKind::COUNT],
    inherited_ultimate_remaining: [f32; HeroKind::COUNT],
    ultimate_gap: f32,
    ultimate_cursor: usize,
    sequence: Option<UltimateSequence>,
    storm_spell_signals: u8,
    frost_spell_signals: u8,
    void_attack_signals: u8,
    ember_stacks: u8,
    ember_decay_timer: f32,
    passive_timer: [f32; HeroKind::COUNT],
    star_kill_progress: u32,
    hero_kill_progress: [u32; HeroKind::COUNT],
    last_total_kills: u32,
    last_core_hp: f32,
    last_basic_attacks: u64,
}

impl Default for HeroAbilityRuntime {
    fn default() -> Self {
        Self {
            observed_kind: None,
            inherited: [false; HeroKind::COUNT],
            inherited_ultimate_remaining: [0.; HeroKind::COUNT],
            ultimate_gap: 0.,
            ultimate_cursor: 0,
            sequence: None,
            storm_spell_signals: 0,
            frost_spell_signals: 0,
            void_attack_signals: 0,
            ember_stacks: 0,
            ember_decay_timer: 0.0,
            passive_timer: [0.; HeroKind::COUNT],
            star_kill_progress: 0,
            hero_kill_progress: [0; HeroKind::COUNT],
            last_total_kills: 0,
            last_core_hp: GameSession::default().core_hp,
            last_basic_attacks: 0,
        }
    }
}

impl HeroAbilityRuntime {
    fn has(&self, current: HeroKind, kind: HeroKind) -> bool {
        current == kind || self.inherited[kind.index()]
    }
    fn kinds(&self, current: HeroKind) -> Vec<HeroKind> {
        HeroKind::ALL
            .into_iter()
            .filter(|kind| self.has(current, *kind))
            .collect()
    }
}

#[derive(Component)]
pub(crate) struct AbilityGenerated;

/// Counts completed basic-attack volleys directly at their source. This is
/// independent of projectile lifetime, so point-blank hits cannot lose a
/// passive trigger when the projectile is recycled in the same frame.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct BasicAttackCounter(pub(crate) u64);

impl BasicAttackCounter {
    pub fn count(&self) -> u64 {
        self.0
    }
}

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
pub(crate) struct VoidPull {
    pub(crate) center: Vec3,
    pub(crate) remaining: f32,
    pub(crate) speed: f32,
}

fn tick_void_pull(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    mut enemies: Query<(Entity, &mut Transform, &mut VoidPull), With<Enemy>>,
) {
    if session.paused || session.finished {
        return;
    }
    for (entity, mut transform, mut pull) in &mut enemies {
        let delta = pull.remaining.min(time.delta_secs());
        let offset = pull.center - transform.translation;
        let movement = offset.normalize_or_zero() * (pull.speed * delta).min(offset.length());
        transform.translation += movement;
        pull.remaining -= delta;
        if pull.remaining <= 0.0 {
            commands.entity(entity).remove::<VoidPull>();
        }
    }
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

fn configure_ultimate(
    selected: Res<SelectedHero>,
    mut active: ResMut<ActiveSkill>,
    fusion: Option<Res<super::HeroFusion>>,
    mut runtime: ResMut<HeroAbilityRuntime>,
) {
    if !selected.locked {
        return;
    }
    if runtime.observed_kind.is_none() {
        active.remaining = active
            .remaining
            .max(effective_ultimate_cooldown(selected.kind));
    }
    if let Some(previous) = runtime
        .observed_kind
        .filter(|previous| *previous != selected.kind)
    {
        runtime.inherited_ultimate_remaining[previous.index()] = active.remaining;
        active.remaining = runtime.inherited_ultimate_remaining[selected.kind.index()].max(3.);
    }
    let previous_kind = runtime.observed_kind;
    runtime.observed_kind = Some(selected.kind);
    active.cooldown = effective_ultimate_cooldown(selected.kind);
    if let Some(fusion) = fusion {
        for kind in HeroKind::ALL {
            if fusion.has(kind) && !runtime.inherited[kind.index()] && previous_kind != Some(kind) {
                // New inheritances enter the rotation after a short readable delay.
                runtime.inherited_ultimate_remaining[kind.index()] = 3.;
            }
            runtime.inherited[kind.index()] = fusion.has(kind);
        }
    }
}

fn tick_inherited_cooldowns(
    time: Res<Time>,
    session: Res<GameSession>,
    vitals: Res<HeroVitals>,
    mut runtime: ResMut<HeroAbilityRuntime>,
) {
    if !session.paused && !session.finished && vitals.can_act() {
        runtime.ultimate_gap = (runtime.ultimate_gap - time.delta_secs()).max(0.);
        for remaining in &mut runtime.inherited_ultimate_remaining {
            *remaining = (*remaining - time.delta_secs()).max(0.);
        }
    }
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
    let basic_attacks = attacks.0.wrapping_sub(runtime.last_basic_attacks);
    runtime.last_basic_attacks = attacks.0;
    if basic_attacks == 0 || !vitals.can_act() {
        return;
    }

    for kind in runtime.kinds(selected.kind) {
        match kind {
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

    for kind in runtime.kinds(selected.kind) {
        match kind {
            HeroKind::StormSavant => {
                if spell_signals == 0 {
                    continue;
                }
                runtime.storm_spell_signals =
                    runtime.storm_spell_signals.saturating_add(spell_signals);
                if runtime.storm_spell_signals < 6 {
                    continue;
                }
                runtime.storm_spell_signals -= 6;
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
                runtime.frost_spell_signals =
                    runtime.frost_spell_signals.saturating_add(spell_signals);
                if runtime.frost_spell_signals < 5 {
                    continue;
                }
                runtime.frost_spell_signals -= 5;
                let (Some(assets), Some(target)) = (
                    assets.as_deref(),
                    preferred_target(hero.single().ok(), &enemies),
                ) else {
                    continue;
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
}

fn maintain_passive_state(
    mut commands: Commands,
    time: Res<Time>,
    selected: Res<SelectedHero>,
    mut vitals: ResMut<HeroVitals>,
    assets: Option<Res<GameAssets>>,
    intent: Res<MoveIntent>,
    mut session: ResMut<GameSession>,
    mut attack_speed: ResMut<HeroAttackSpeedModifier>,
    hero: Query<&Transform, With<Hero>>,
    core: Query<&Transform, (With<Core>, Without<Hero>)>,
    passive_spirits: Query<Entity, With<PassiveSpirit>>,
    player_summons: Query<(), With<PlayerSummon>>,
    market_effects: Res<MarketEffects>,
    mut runtime: ResMut<HeroAbilityRuntime>,
    mut pools: ResMut<CombatPools>,
) {
    let gained = session.total_kills.saturating_sub(runtime.last_total_kills);
    for kind in runtime.kinds(selected.kind) {
        match kind {
            HeroKind::StarWagerer => {
                runtime.star_kill_progress = runtime.star_kill_progress.saturating_add(gained);
            }
            HeroKind::IronBerserker | HeroKind::SolarVanguard => {
                runtime.hero_kill_progress[kind.index()] =
                    runtime.hero_kill_progress[kind.index()].saturating_add(gained);
            }
            _ => {}
        }
    }
    runtime.last_total_kills = session.total_kills;

    if session.paused || session.finished || !vitals.can_act() {
        runtime.last_core_hp = session.core_hp;
        return;
    }

    attack_speed.0 = if runtime.has(selected.kind, HeroKind::EmberRanger) {
        ember_attack_speed_multiplier(runtime.ember_stacks)
    } else {
        1.
    };
    for kind in runtime.kinds(selected.kind) {
        let ability_scale = HEROES[kind.index()].rarity.hero_ability_multiplier();
        match kind {
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
                runtime.passive_timer[kind.index()] -= time.delta_secs();
                let loss = (runtime.last_core_hp - session.core_hp).max(0.0);
                let in_guard_range = hero
                    .single()
                    .ok()
                    .zip(core.single().ok())
                    .is_some_and(|(hero, core)| hero.translation.distance(core.translation) <= 8.0);
                if in_guard_range && loss > 0.0 {
                    session.heal_gate(stone_guard_refund(loss) * ability_scale);
                }
                if in_guard_range && runtime.passive_timer[kind.index()] <= 0.0 {
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
                    runtime.passive_timer[kind.index()] = 2.4;
                }
            }
            HeroKind::SpiritKeeper => {
                if passive_spirits.is_empty()
                    && player_summon_slots(
                        player_summons
                            .iter()
                            .count()
                            .max(pools.summon_active_count()),
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
                while runtime.hero_kill_progress[kind.index()] >= 8 {
                    runtime.hero_kill_progress[kind.index()] -= 8;
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
                attack_speed.0 *= rift_dancer_attack_speed_multiplier(intent.0.length_squared());
            }
            HeroKind::GroveArtificer => {
                let in_repair_range = hero
                    .single()
                    .ok()
                    .zip(core.single().ok())
                    .is_some_and(|(hero, core)| hero.translation.distance(core.translation) <= 9.0);
                if in_repair_range {
                    runtime.passive_timer[kind.index()] += time.delta_secs();
                } else {
                    runtime.passive_timer[kind.index()] = 0.0;
                }
                if runtime.passive_timer[kind.index()] >= 6.0 {
                    session.heal_gate(12.0 * ability_scale);
                    runtime.passive_timer[kind.index()] -= 6.0;
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
                while runtime.hero_kill_progress[kind.index()] >= 10 {
                    runtime.hero_kill_progress[kind.index()] -= 10;
                    session.heal_gate(24.0 * ability_scale);
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
    if !runtime.has(selected.kind, HeroKind::TideWalker)
        || session.paused
        || session.finished
        || !vitals.can_act()
    {
        return;
    }
    runtime.passive_timer[HeroKind::TideWalker.index()] -= time.delta_secs();
    if runtime.passive_timer[HeroKind::TideWalker.index()] > 0.0 {
        return;
    }
    runtime.passive_timer[HeroKind::TideWalker.index()] = 3.2;

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

fn cleanup_spirit_summons(
    mut commands: Commands,
    session: Res<GameSession>,
    spirits: Query<Entity, With<SpiritOrbit>>,
    mut pools: ResMut<CombatPools>,
) {
    if !session.finished {
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
    build: Res<PlayerBuild>,
    assets: Option<Res<GameAssets>>,
    enemies: Query<(&Enemy, &Transform, Option<&EliteAffix>), Without<SpiritOrbit>>,
    mut spirits: Query<
        (
            Entity,
            &mut Transform,
            &mut SpiritOrbit,
            &super::summon_movement::FormationSlot,
        ),
        Without<Hero>,
    >,
    mut pools: ResMut<CombatPools>,
) {
    if session.paused || session.finished {
        return;
    }
    let Some(assets) = assets.as_deref() else {
        return;
    };

    for (entity, mut transform, mut spirit, slot) in &mut spirits {
        if let Some(remaining) = &mut spirit.remaining {
            *remaining -= time.delta_secs();
            if *remaining <= 0.0 {
                pools.retire_summon(&mut commands, entity);
                continue;
            }
        }
        spirit.angle = (spirit.angle + time.delta_secs() * 1.7) % std::f32::consts::TAU;
        // Float vertically, but pursuit owns horizontal movement. The old
        // orbital radius now sets a readable ranged engagement distance.
        transform.translation.y = 0.75 + (spirit.angle * 2.).sin() * 0.16;
        spirit.shot_timer = (spirit.shot_timer - time.delta_secs()).max(0.);
        let target = enemies
            .iter()
            .filter(|(enemy, _, affix)| {
                enemy.hp > 0. && !affix.is_some_and(|affix| affix.is_volatile_pending())
            })
            .min_by(|(_, left, _), (_, right, _)| {
                transform
                    .translation
                    .distance_squared(left.translation)
                    .total_cmp(&transform.translation.distance_squared(right.translation))
            })
            .map(|(_, target, _)| target.translation);
        let Some(target) = target else {
            super::summon_movement::follow(&mut transform, slot.0, 5.2, time.delta_secs());
            continue;
        };
        let ready = super::summon_movement::advance(
            &mut transform,
            target,
            5.2,
            (4. + spirit.radius).clamp(4., 6.),
            time.delta_secs(),
        );
        if !ready || spirit.shot_timer > 0. {
            continue;
        }
        {
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
    spawn_ultimate_zone(commands, pools, assets, kind, center, damage, radius);
    spawn_ability_fx(commands, pools, assets, kind, center, radius, 0.38);
}

fn spawn_ultimate_zone(
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
        Transform::from_translation(
            origin.with_x(origin.x.max(crate::game::resources::GATE_POSITION.x + 2.3))
                + Vec3::Y * 0.75,
        )
        .with_rotation(Quat::from_rotation_arc(Vec3::NEG_Z, Vec3::X))
        .with_scale(Vec3::splat(2.3)),
    )?;
    commands
        .entity(entity)
        .insert(Name::new("Spirit companion"));
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

const fn player_summon_slots(existing: usize, equipment_bonus: u8) -> usize {
    summon_cap(equipment_bonus).saturating_sub(existing)
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

pub(crate) fn reset_run(world: &mut World) {
    world.insert_resource(HeroAbilityRuntime::default());
    world.insert_resource(BasicAttackCounter::default());
    world.insert_resource(HeroAttackSpeedModifier::default());
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::game::content::HEROES;

    fn test_game_assets() -> GameAssets {
        GameAssets {
            hero_scenes: std::array::from_fn(|_| Handle::default()),
            enemy_scenes: std::array::from_fn(|_| Handle::default()),
            boss_scenes: std::array::from_fn(|_| Handle::default()),
            adventure_scenes: default(),
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

    fn auto_test_app(kind: HeroKind) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<HeroAbilityRuntime>()
            .init_resource::<UltimateCues>()
            .init_resource::<HeroVitals>()
            .init_resource::<ActiveSkill>()
            .init_resource::<PlayerBuild>()
            .init_resource::<MarketEffects>()
            .init_resource::<CombatPools>()
            .insert_resource(GameSession {
                paused: false,
                ..default()
            })
            .insert_resource(SelectedHero { kind, locked: true })
            .insert_resource(test_game_assets())
            .add_systems(Update, observe_ultimate_cast);
        app.world_mut().spawn((Hero, Transform::default()));
        app.world_mut().spawn((
            Enemy {
                hp: 1000.0,
                max_hp: 1000.0,
                speed: 0.0,
                damage: 0.0,
                xp: 0,
                wood: 0,
                gold: 0,
                kill_credit: 1,
                elite: false,
            },
            Transform::from_xyz(1.0, 0.0, 0.0),
        ));
        app
    }

    fn companion_test_app(remaining: Option<f32>) -> (App, Entity, Entity) {
        use bevy::ecs::system::RunSystemOnce;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_millis(100),
            ))
            .init_resource::<GameSession>()
            .init_resource::<HeroVitals>()
            .init_resource::<PlayerBuild>()
            .init_resource::<CombatPools>()
            .insert_resource(test_game_assets())
            .add_systems(Update, update_spirit_companions);
        let hero = app.world_mut().spawn((Hero, Transform::default())).id();
        let spirit = app
            .world_mut()
            .run_system_once(
                move |mut commands: Commands,
                      mut pools: ResMut<CombatPools>,
                      assets: Res<GameAssets>| {
                    spawn_spirit(
                        &mut commands,
                        &mut pools,
                        &assets,
                        Vec3::ZERO,
                        0.,
                        1.35,
                        remaining,
                        1.1,
                        true,
                    )
                    .unwrap()
                },
            )
            .unwrap();
        (app, hero, spirit)
    }
    fn companion_enemy(app: &mut App, position: Vec3) -> Entity {
        app.world_mut()
            .spawn((
                Enemy {
                    hp: 1000.,
                    max_hp: 1000.,
                    speed: 0.,
                    damage: 0.,
                    xp: 0,
                    wood: 0,
                    gold: 0,
                    kill_credit: 0,
                    elite: false,
                },
                Transform::from_translation(position),
            ))
            .id()
    }
    #[test]
    fn spirit_companion_pursues_then_fires_and_retargets_after_death() {
        let (mut app, _, spirit) = companion_test_app(None);
        let enemy = companion_enemy(&mut app, Vec3::new(20., 0., 0.));
        for _ in 0..10 {
            app.update();
        }
        let moving = app.world().get::<Transform>(spirit).unwrap().translation;
        assert!(moving.x > 3. && moving.x < 14.);
        assert_eq!(
            app.world_mut()
                .query::<&Projectile>()
                .iter(app.world())
                .count(),
            0,
            "no ranged attacks from outside engagement range"
        );
        for _ in 0..40 {
            app.update();
        }
        let transform = app.world().get::<Transform>(spirit).unwrap();
        assert!((transform.translation.x - 14.65).abs() < 0.01);
        assert!((transform.rotation * Vec3::NEG_Z).dot(Vec3::X) > 0.99);
        assert!(
            app.world_mut()
                .query::<&Projectile>()
                .iter(app.world())
                .count()
                > 0
        );
        app.world_mut().get_mut::<Enemy>(enemy).unwrap().hp = 0.;
        companion_enemy(&mut app, Vec3::new(0., 0., 0.));
        for _ in 0..10 {
            app.update();
        }
        let transform = app.world().get::<Transform>(spirit).unwrap();
        assert!(transform.translation.x < 10.);
        assert!((transform.rotation * Vec3::NEG_Z).dot(Vec3::NEG_X) > 0.99);
    }
    #[test]
    fn spirit_companion_pause_freezes_movement_lifetime_and_fire_then_returns_outside_gate() {
        let (mut app, hero, spirit) = companion_test_app(Some(20.));
        let enemy = companion_enemy(&mut app, Vec3::new(20., 0., 0.));
        for _ in 0..10 {
            app.update();
        }
        app.world_mut().resource_mut::<GameSession>().paused = true;
        let before = *app.world().get::<Transform>(spirit).unwrap();
        let lifetime = app.world().get::<SpiritOrbit>(spirit).unwrap().remaining;
        let timer = app.world().get::<SpiritOrbit>(spirit).unwrap().shot_timer;
        for _ in 0..10 {
            app.update();
        }
        assert_eq!(*app.world().get::<Transform>(spirit).unwrap(), before);
        assert_eq!(
            app.world().get::<SpiritOrbit>(spirit).unwrap().remaining,
            lifetime
        );
        assert_eq!(
            app.world().get::<SpiritOrbit>(spirit).unwrap().shot_timer,
            timer
        );
        app.world_mut().resource_mut::<GameSession>().paused = false;
        app.world_mut().get_mut::<Enemy>(enemy).unwrap().hp = 0.;
        app.world_mut()
            .get_mut::<Transform>(hero)
            .unwrap()
            .translation = Vec3::new(-10., 0., 0.);
        for _ in 0..40 {
            app.update();
        }
        let transform = app.world().get::<Transform>(spirit).unwrap();
        assert!(transform.translation.x >= crate::game::resources::GATE_POSITION.x + 1.8);
        assert!(transform.translation.x < crate::game::resources::GATE_POSITION.x + 5.0);
        assert!((transform.rotation * Vec3::NEG_Z).dot(Vec3::X) > 0.99);
        assert!((transform.translation.z).abs() <= 4.1);
        assert!(app.world().get::<PassiveSpirit>(spirit).is_some());
        app.world_mut()
            .get_mut::<SpiritOrbit>(spirit)
            .unwrap()
            .remaining = Some(0.05);
        app.update();
        assert!(
            app.world().get::<SpiritOrbit>(spirit).is_none(),
            "expired summon returns to its pool"
        );
    }

    #[test]
    fn first_selected_ultimate_waits_full_actual_hero_cooldown_only_once() {
        use bevy::ecs::system::RunSystemOnce;
        for kind in HeroKind::ALL {
            let mut app = auto_test_app(kind);
            app.world_mut().run_system_once(configure_ultimate).unwrap();
            assert_eq!(
                app.world().resource::<ActiveSkill>().remaining,
                effective_ultimate_cooldown(kind)
            );
            app.world_mut().resource_mut::<ActiveSkill>().remaining = 0.0;
            app.world_mut().run_system_once(configure_ultimate).unwrap();
            assert_eq!(app.world().resource::<ActiveSkill>().remaining, 0.0);
        }
    }
    #[test]
    fn unselected_preview_does_not_seed_wrong_initial_ultimate() {
        use bevy::ecs::system::RunSystemOnce;
        let mut app = auto_test_app(HeroKind::StormSavant);
        app.world_mut().resource_mut::<SelectedHero>().locked = false;
        app.world_mut().run_system_once(configure_ultimate).unwrap();
        assert!(
            app.world()
                .resource::<HeroAbilityRuntime>()
                .observed_kind
                .is_none()
        );
        *app.world_mut().resource_mut::<SelectedHero>() = SelectedHero {
            kind: HeroKind::EmberRanger,
            locked: true,
        };
        app.world_mut().run_system_once(configure_ultimate).unwrap();
        assert_eq!(
            app.world().resource::<ActiveSkill>().remaining,
            effective_ultimate_cooldown(HeroKind::EmberRanger)
        );
    }
    #[test]
    fn recruitment_preserves_old_cooldown_and_passive_progress() {
        let mut app = auto_test_app(HeroKind::EmberRanger);
        app.init_resource::<super::super::HeroFusion>().add_systems(
            PreUpdate,
            (tick_inherited_cooldowns, configure_ultimate).chain(),
        );
        app.world_mut().resource_mut::<ActiveSkill>().remaining = 23.;
        app.update();
        {
            let mut runtime = app.world_mut().resource_mut::<HeroAbilityRuntime>();
            runtime.ember_stacks = 9;
            runtime.storm_spell_signals = 4;
            runtime.passive_timer[HeroKind::TideWalker.index()] = 2.;
        }
        app.world_mut()
            .resource_mut::<super::super::HeroFusion>()
            .record_absorbed(HeroKind::EmberRanger);
        app.world_mut().resource_mut::<SelectedHero>().kind = HeroKind::StarWagerer;
        app.update();
        let runtime = app.world().resource::<HeroAbilityRuntime>();
        assert!(runtime.inherited_ultimate_remaining[HeroKind::EmberRanger.index()] > 22.);
        assert_eq!(runtime.ember_stacks, 9);
        assert_eq!(runtime.storm_spell_signals, 4);
        assert_eq!(runtime.passive_timer[HeroKind::TideWalker.index()], 2.);
        assert!(runtime.has(HeroKind::StarWagerer, HeroKind::EmberRanger));
        assert_eq!(app.world().resource::<UltimateCues>().serial, 0);
    }

    #[test]
    fn inherited_ultimates_really_auto_cast_in_separate_bounded_sequences() {
        let mut app = auto_test_app(HeroKind::StoneWarden);
        app.init_resource::<super::super::HeroFusion>()
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_millis(100),
            ))
            .add_systems(
                PreUpdate,
                (tick_inherited_cooldowns, configure_ultimate).chain(),
            )
            .add_systems(Update, tick_ultimate_sequence.before(observe_ultimate_cast));
        app.world_mut().resource_mut::<ActiveSkill>().remaining = 1000.;
        for kind in [HeroKind::StormSavant, HeroKind::FrostOracle] {
            app.world_mut()
                .resource_mut::<super::super::HeroFusion>()
                .record_absorbed(kind);
        }
        let mut casts = Vec::new();
        let mut previous = 0;
        for frame in 0..200 {
            app.update();
            let serial = app.world().resource::<UltimateCues>().serial;
            assert!(
                serial <= previous + 1,
                "at most one ultimate starts per frame"
            );
            if serial > previous {
                let kind = app
                    .world()
                    .resource::<HeroAbilityRuntime>()
                    .sequence
                    .unwrap()
                    .kind;
                casts.push((frame, kind));
            }
            previous = serial;
        }
        assert_eq!(casts.len(), 2, "both inherited heroes must cast: {casts:?}");
        assert_eq!(casts[0].1, HeroKind::StormSavant);
        assert_eq!(casts[1].1, HeroKind::FrostOracle);
        assert!(casts[1].0 - casts[0].0 >= 30);
        assert_eq!(app.world().resource::<ActiveSkill>().remaining, 1000.);
    }

    #[test]
    fn inherited_kill_passives_all_apply_to_the_same_real_kills() {
        let mut app = auto_test_app(HeroKind::StormSavant);
        app.init_resource::<HeroAttackSpeedModifier>()
            .init_resource::<MoveIntent>()
            .add_systems(Update, maintain_passive_state.after(observe_ultimate_cast));
        app.world_mut().resource_mut::<ActiveSkill>().remaining = 1000.;
        {
            let mut runtime = app.world_mut().resource_mut::<HeroAbilityRuntime>();
            for kind in [
                HeroKind::StarWagerer,
                HeroKind::IronBerserker,
                HeroKind::SolarVanguard,
            ] {
                runtime.inherited[kind.index()] = true;
                runtime.inherited_ultimate_remaining[kind.index()] = 1000.;
            }
        }
        app.world_mut().resource_mut::<GameSession>().core_hp = 50.;
        app.world_mut().resource_mut::<HeroVitals>().health = 100.;
        app.world_mut().resource_mut::<GameSession>().total_kills = 24;
        let gold = app.world().resource::<GameSession>().gold;
        app.update();
        assert!(app.world().resource::<GameSession>().gold > gold);
        assert!(app.world().resource::<GameSession>().core_hp > 50.);
        assert!(app.world().resource::<HeroVitals>().health > 100.);
        let after = app.world().resource::<GameSession>().gold;
        app.update();
        assert_eq!(
            app.world().resource::<GameSession>().gold,
            after,
            "same kills cannot reward twice"
        );
    }

    #[test]
    fn all_twelve_heroes_auto_cast_once_without_input() {
        for kind in HeroKind::ALL {
            let mut app = auto_test_app(kind);
            app.update();
            assert!(
                app.world().resource::<ActiveSkill>().remaining > 0.0,
                "{kind:?}"
            );
            let sequences = app
                .world()
                .resource::<HeroAbilityRuntime>()
                .sequence
                .map(|s| s.pulses_left);
            let expected = match kind {
                HeroKind::StormSavant => Some(10),
                HeroKind::EmberRanger => Some(5),
                HeroKind::StoneWarden => Some(6),
                HeroKind::SpiritKeeper => None,
                HeroKind::TideWalker => Some(3),
                HeroKind::StarWagerer => None,
                HeroKind::FrostOracle => Some(4),
                HeroKind::IronBerserker => Some(6),
                HeroKind::RiftDancer => Some(8),
                HeroKind::GroveArtificer => Some(6),
                HeroKind::SolarVanguard => Some(5),
                HeroKind::VoidReaper => Some(5),
            };
            assert_eq!(sequences, expected);
            let gold = app.world().resource::<GameSession>().gold;
            let summon_count = app
                .world_mut()
                .query::<&SpiritOrbit>()
                .iter(app.world())
                .count();
            if kind == HeroKind::SpiritKeeper {
                assert_eq!(summon_count, 6);
            }
            app.update();
            assert_eq!(gold, app.world().resource::<GameSession>().gold);
            assert_eq!(app.world().resource::<UltimateCues>().serial, 1);
            assert_eq!(
                summon_count,
                app.world_mut()
                    .query::<&SpiritOrbit>()
                    .iter(app.world())
                    .count()
            );
        }
    }

    #[test]
    fn automatic_cast_targets_living_archive_echo_but_not_dead_enemies() {
        for living in [false, true] {
            let mut app = auto_test_app(HeroKind::StormSavant);
            let enemy = app
                .world_mut()
                .query_filtered::<Entity, With<Enemy>>()
                .single(app.world())
                .expect("fixture enemy");
            app.world_mut()
                .entity_mut(enemy)
                .insert(crate::game::archive::ArchiveEcho);
            if !living {
                app.world_mut().get_mut::<Enemy>(enemy).unwrap().hp = 0.0;
            }
            app.update();
            assert_eq!(
                app.world().resource::<UltimateCues>().serial,
                u64::from(living)
            );
            assert_eq!(
                app.world().resource::<ActiveSkill>().remaining > 0.0,
                living
            );
        }
    }

    #[test]
    fn automatic_cast_does_not_spend_cooldown_when_blocked() {
        for blocker in 0..6 {
            let mut app = auto_test_app(HeroKind::StormSavant);
            match blocker {
                0 => app.world_mut().resource_mut::<GameSession>().paused = true,
                1 => app.world_mut().resource_mut::<GameSession>().finished = true,
                2 => app.world_mut().resource_mut::<HeroVitals>().alive = false,
                3 => app.world_mut().resource_mut::<SelectedHero>().locked = false,
                4 => {
                    app.world_mut().remove_resource::<GameAssets>();
                }
                _ => {
                    let entities: Vec<_> = app
                        .world_mut()
                        .query_filtered::<Entity, With<Enemy>>()
                        .iter(app.world())
                        .collect();
                    for entity in entities {
                        app.world_mut().despawn(entity);
                    }
                }
            }
            app.update();
            assert_eq!(
                app.world().resource::<ActiveSkill>().remaining,
                0.0,
                "blocker {blocker}"
            );
        }
    }
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
    fn automatic_casts_wait_for_useful_targets() {
        for kind in HeroKind::ALL {
            assert!(!automatic_ultimate_useful(
                kind,
                Vec3::ZERO,
                None,
                false,
                18
            ));
            assert!(automatic_ultimate_useful(
                kind,
                Vec3::ZERO,
                Some(Vec3::X),
                false,
                18
            ));
        }
        assert!(!automatic_ultimate_useful(
            HeroKind::IronBerserker,
            Vec3::ZERO,
            Some(Vec3::X * 3.0),
            false,
            18
        ));
        assert!(!automatic_ultimate_useful(
            HeroKind::SpiritKeeper,
            Vec3::ZERO,
            Some(Vec3::X),
            false,
            0
        ));
        assert!(automatic_ultimate_useful(
            HeroKind::StoneWarden,
            Vec3::ZERO,
            None,
            true,
            0
        ));
    }

    #[test]
    fn spirit_keeper_passive_and_ultimate_share_the_global_summon_slots() {
        assert_eq!(player_summon_slots(17, 0), 1);
        assert_eq!(player_summon_slots(18, 0), 0);
        assert_eq!(player_summon_slots(18, 2), 2);
        assert_eq!(player_summon_slots(18, 1), 1);
        assert_eq!(player_summon_slots(18, 3), 2);
        assert_eq!(player_summon_slots(20, 3), 0);
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
