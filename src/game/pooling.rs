use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

use bevy::{camera::visibility::RenderLayers, prelude::*};

use super::archive::ArchiveEcho;
use super::bonds::{DamageElement, DamageProfile};
use super::boss_vfx::BossPhaseBurstOwner;
use super::bosses::BossSummon;
use super::combat::{
    CommanderAuraState, EliteAffix, EnemyArmor, EnemyBreachDamage, ProjectileFirstFrameTravel,
    ProjectileHitMemory,
};
use super::hero_abilities::{
    AbilityGenerated, PassiveSpirit, SpiritOrbit, TimedSpeedEffect, VoidPull, WetMark,
};
use super::hit_feedback::HitPunch;
use super::skill_lighting::GroundLightProfile;
use super::skills::infection::Infection;
use super::skills::{
    AncestorColossus, BoltTurret, BoneSentry, FrostWyrm, OathBanner, SkillSummon, SpiritHound,
    TideElemental,
};
use super::trial::ChallengeFighter;
use super::{
    BossEnemy, Enemy, EnemyArchetype, EnemyHeroThreat, ImpactFx, PlayerSummon, Projectile,
    TrialEnemy,
};

pub(crate) const PROJECTILE_POOL_CAPACITY: usize = 256;
pub(crate) const IMPACT_FX_POOL_CAPACITY: usize = 128;
pub(crate) const DAMAGE_NUMBER_POOL_CAPACITY: usize = 64;
pub(crate) const DAMAGE_NUMBER_COALESCE_SECONDS: f32 = 0.12;
pub(crate) const DAMAGE_NUMBER_LIFETIME_SECONDS: f32 = 0.7;
pub(crate) const DAMAGE_NUMBER_RENDER_LAYER: usize = 7;
pub(crate) const PLAYER_SUMMON_HARD_CAP: usize = 20;
/// Ordinary waves stop at 130 separately; this leaves thirty physical slots
/// for Q/W challenges, checkpoint bosses, archive echoes, and boss summons.
pub(crate) const ENEMY_POOL_ACTIVE_CAP: usize = 160;
/// Bounds retained scene roots across every role/model key over a long run.
pub(crate) const ENEMY_POOL_CREATED_CAP: usize = 256;
/// Ordinary roles may retain at most 226 roots, leaving thirty guaranteed
/// creation slots for the thirty-slot special-active reserve.
pub(crate) const ENEMY_POOL_SPECIAL_RESERVE: usize = 30;
pub(crate) const REGULAR_ENEMY_CREATED_CAP: usize =
    ENEMY_POOL_CREATED_CAP - ENEMY_POOL_SPECIAL_RESERVE;
pub(crate) const ENEMY_REUSE_QUARANTINE_SECS: f32 = 2.5;
const SUMMON_VISUAL_COUNT: usize = 8;

/// A scene shell never changes this key. Role is part of the key even when two
/// roles happen to use the same GLB, so reuse cannot leak challenge/boss state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum EnemySceneKey {
    Regular(u8),
    Boss(u8),
    BossSummon(u8),
    Adventure(u8),
    Tower,
    Gold,
    Archive,
}

impl EnemySceneKey {
    const fn is_regular(self) -> bool {
        matches!(self, Self::Regular(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SummonVisual {
    BoneSentry,
    SpiritHound,
    TideElemental,
    BoltTurret,
    FrostWyrm,
    AncestorColossus,
    OathBanner,
    SpiritOrbit,
}

impl SummonVisual {
    const fn index(self) -> usize {
        self as usize
    }

    const fn uses_world_asset(self) -> bool {
        matches!(
            self,
            Self::BoneSentry
                | Self::SpiritHound
                | Self::TideElemental
                | Self::BoltTurret
                | Self::FrostWyrm
                | Self::AncestorColossus
        )
    }
}

#[derive(Debug, Clone, Copy)]
struct EntityLease {
    entity: Entity,
    newly_created: bool,
}

/// Monotonic counters for one pool. `active` is the current leased count;
/// the other counters cover the lifetime of the resource.
#[allow(dead_code)] // Exposed for opt-in debug overlays; unit tests are the current consumer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PoolMetrics {
    created: usize,
    reused: usize,
    active: usize,
    peak: usize,
    rejected: usize,
}

#[allow(dead_code)]
impl PoolMetrics {
    pub const fn created(self) -> usize {
        self.created
    }

    pub const fn reused(self) -> usize {
        self.reused
    }

    pub const fn active(self) -> usize {
        self.active
    }

    pub const fn peak(self) -> usize {
        self.peak
    }

    pub const fn rejected(self) -> usize {
        self.rejected
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CombatPoolSnapshot {
    /// Total logical projectiles. This includes the 256 visible shells and
    /// any elastic mesh-free overflow shells, so `rejected` is data loss.
    pub projectiles: PoolMetrics,
    /// Only the projectile entities which retain a mesh/material shell.
    pub projectile_visible: PoolMetrics,
    /// Lifetime count of logical projectiles which intentionally omitted art.
    pub projectile_visual_fallbacks: usize,
    pub impact_fx: PoolMetrics,
    pub summons: PoolMetrics,
    pub enemies: PoolMetrics,
    pub damage_numbers: DamageNumberPoolMetrics,
}

/// Lifetime counters for the fixed 64-entry damage-number visual pool.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DamageNumberPoolMetrics {
    created: usize,
    reused: usize,
    active: usize,
    peak: usize,
    coalesced: usize,
    overwritten: usize,
}

#[allow(dead_code)]
impl DamageNumberPoolMetrics {
    pub const fn created(self) -> usize {
        self.created
    }

    pub const fn reused(self) -> usize {
        self.reused
    }

    pub const fn active(self) -> usize {
        self.active
    }

    pub const fn peak(self) -> usize {
        self.peak
    }

    pub const fn coalesced(self) -> usize {
        self.coalesced
    }

    pub const fn overwritten(self) -> usize {
        self.overwritten
    }
}

#[derive(Debug)]
struct EntityPool {
    capacity: usize,
    created: usize,
    free: Vec<Entity>,
    active: BTreeSet<Entity>,
    pending: Vec<Entity>,
    reused: usize,
    peak: usize,
    rejected: usize,
}

impl EntityPool {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            created: 0,
            free: Vec::with_capacity(capacity),
            active: BTreeSet::new(),
            pending: Vec::with_capacity(capacity),
            reused: 0,
            peak: 0,
            rejected: 0,
        }
    }

    fn rent(&mut self, commands: &mut Commands) -> Option<Entity> {
        self.rent_with_status(commands).map(|lease| lease.entity)
    }

    fn rent_with_status(&mut self, commands: &mut Commands) -> Option<EntityLease> {
        let (entity, newly_created) = if let Some(entity) = self.free.pop() {
            self.reused = self.reused.saturating_add(1);
            (entity, false)
        } else if self.created < self.capacity {
            self.created += 1;
            (commands.spawn_empty().id(), true)
        } else {
            self.rejected = self.rejected.saturating_add(1);
            return None;
        };

        let newly_active = self.active.insert(entity);
        debug_assert!(newly_active, "a free pooled entity was already active");
        self.peak = self.peak.max(self.active.len());
        Some(EntityLease {
            entity,
            newly_created,
        })
    }

    fn retire(&mut self, entity: Entity) -> bool {
        if !self.active.remove(&entity) {
            return false;
        }
        self.pending.push(entity);
        true
    }

    fn active_entities(&self) -> Vec<Entity> {
        self.active.iter().copied().collect()
    }

    fn finalize_pending(&mut self) {
        self.free.append(&mut self.pending);
    }

    #[allow(dead_code)]
    fn metrics(&self) -> PoolMetrics {
        PoolMetrics {
            created: self.created,
            reused: self.reused,
            active: self.active.len(),
            peak: self.peak,
            rejected: self.rejected,
        }
    }
}

/// An entity pool that grows only to the observed concurrent peak. It is used
/// for gameplay-only projectile overflow: unlike `EntityPool`, it has no
/// artificial capacity and therefore never drops combat data.
#[derive(Debug, Default)]
struct ElasticEntityPool {
    created: usize,
    free: Vec<Entity>,
    active: BTreeSet<Entity>,
    pending: Vec<Entity>,
    reused: usize,
    peak: usize,
}

impl ElasticEntityPool {
    fn rent(&mut self, commands: &mut Commands) -> EntityLease {
        let (entity, newly_created) = if let Some(entity) = self.free.pop() {
            self.reused = self.reused.saturating_add(1);
            (entity, false)
        } else {
            self.created = self.created.saturating_add(1);
            (commands.spawn_empty().id(), true)
        };
        let newly_active = self.active.insert(entity);
        debug_assert!(newly_active, "a free elastic entity was already active");
        self.peak = self.peak.max(self.active.len());
        EntityLease {
            entity,
            newly_created,
        }
    }

    fn retire(&mut self, entity: Entity) -> bool {
        if !self.active.remove(&entity) {
            return false;
        }
        self.pending.push(entity);
        true
    }

    fn active_entities(&self) -> Vec<Entity> {
        self.active.iter().copied().collect()
    }

    fn finalize_pending(&mut self) {
        self.free.append(&mut self.pending);
    }

    fn metrics(&self) -> PoolMetrics {
        PoolMetrics {
            created: self.created,
            reused: self.reused,
            active: self.active.len(),
            peak: self.peak,
            rejected: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct QuarantinedEnemy {
    entity: Entity,
    key: EnemySceneKey,
    release_at: f32,
}

#[derive(Debug)]
struct EnemyScenePool {
    created: usize,
    regular_created: usize,
    free: HashMap<EnemySceneKey, Vec<Entity>>,
    active: BTreeMap<Entity, EnemySceneKey>,
    quarantine: Vec<QuarantinedEnemy>,
    decorations: HashMap<Entity, Vec<Entity>>,
    game_clock: f32,
    reused: usize,
    peak: usize,
    rejected: usize,
}

impl Default for EnemyScenePool {
    fn default() -> Self {
        Self {
            created: 0,
            regular_created: 0,
            free: HashMap::new(),
            active: BTreeMap::new(),
            quarantine: Vec::new(),
            decorations: HashMap::new(),
            game_clock: 0.0,
            reused: 0,
            peak: 0,
            rejected: 0,
        }
    }
}

impl EnemyScenePool {
    fn can_rent(&self, key: EnemySceneKey, count: usize) -> bool {
        if self.active.len().saturating_add(count) > ENEMY_POOL_ACTIVE_CAP {
            return false;
        }
        let ready = self.free.get(&key).map_or(0, Vec::len);
        let global_creatable = ENEMY_POOL_CREATED_CAP.saturating_sub(self.created);
        let creatable = if key.is_regular() {
            global_creatable.min(REGULAR_ENEMY_CREATED_CAP.saturating_sub(self.regular_created))
        } else {
            global_creatable
        };
        ready.saturating_add(creatable) >= count
    }

    fn rent_group(
        &mut self,
        commands: &mut Commands,
        key: EnemySceneKey,
        count: usize,
    ) -> Option<Vec<EntityLease>> {
        if !self.can_rent(key, count) {
            self.rejected = self.rejected.saturating_add(count.max(1));
            return None;
        }

        let mut leases = Vec::with_capacity(count);
        for _ in 0..count {
            let (entity, newly_created) =
                if let Some(entity) = self.free.get_mut(&key).and_then(Vec::pop) {
                    self.reused = self.reused.saturating_add(1);
                    (entity, false)
                } else {
                    self.created = self.created.saturating_add(1);
                    if key.is_regular() {
                        self.regular_created = self.regular_created.saturating_add(1);
                    }
                    (commands.spawn_empty().id(), true)
                };
            let previous = self.active.insert(entity, key);
            debug_assert!(previous.is_none(), "a free enemy shell was already active");
            leases.push(EntityLease {
                entity,
                newly_created,
            });
        }
        self.peak = self.peak.max(self.active.len());
        Some(leases)
    }

    fn retire(&mut self, entity: Entity) -> Option<EnemySceneKey> {
        let key = self.active.remove(&entity)?;
        self.quarantine.push(QuarantinedEnemy {
            entity,
            key,
            release_at: self.game_clock + ENEMY_REUSE_QUARANTINE_SECS,
        });
        Some(key)
    }

    fn advance(&mut self, delta: f32) {
        self.game_clock += delta.max(0.0);
        let mut waiting = Vec::with_capacity(self.quarantine.len());
        let mut released = Vec::new();
        for entry in self.quarantine.drain(..) {
            if entry.release_at <= self.game_clock {
                released.push(entry);
            } else {
                waiting.push(entry);
            }
        }
        self.quarantine = waiting;
        for entry in released {
            self.free.entry(entry.key).or_default().push(entry.entity);
        }
    }

    fn regular_active_count(&self) -> usize {
        self.active.values().filter(|key| key.is_regular()).count()
    }

    fn metrics(&self) -> PoolMetrics {
        PoolMetrics {
            created: self.created,
            reused: self.reused,
            active: self.active.len(),
            peak: self.peak,
            rejected: self.rejected,
        }
    }
}

#[derive(Component)]
struct PooledProjectile;

#[derive(Component)]
struct ActivePooledProjectile;

/// Marks projectile overflow that participates in every gameplay query but
/// deliberately has no `Mesh3d` or material component.
#[derive(Component)]
struct LogicOnlyPooledProjectile;

#[derive(Component)]
struct PooledImpactFx;

#[derive(Component)]
struct ActivePooledImpactFx;

#[derive(Component)]
struct PooledDamageNumber;

#[allow(dead_code)] // Runtime owns animation state in `DamageNumberPool`; tests audit the mirror.
#[derive(Component, Debug, Clone, Copy)]
struct ActivePooledDamageNumber {
    target: Entity,
    amount: f32,
    element: DamageElement,
    age: f32,
}

#[derive(Debug, Clone, Copy)]
struct DamageNumberEntry {
    target: Entity,
    amount: f32,
    element: DamageElement,
    critical: bool,
    age: f32,
    origin: Vec3,
    serial: u64,
}

#[derive(Debug)]
struct DamageNumberPool {
    shells: EntityPool,
    entries: BTreeMap<Entity, DamageNumberEntry>,
    serial: u64,
    coalesced: usize,
    overwritten: usize,
}

impl Default for DamageNumberPool {
    fn default() -> Self {
        Self {
            shells: EntityPool::new(DAMAGE_NUMBER_POOL_CAPACITY),
            entries: BTreeMap::new(),
            serial: 0,
            coalesced: 0,
            overwritten: 0,
        }
    }
}

impl DamageNumberPool {
    fn show(
        &mut self,
        commands: &mut Commands,
        target: Entity,
        position: Vec3,
        amount: f32,
        element: DamageElement,
        critical: bool,
    ) -> Option<Entity> {
        if !amount.is_finite() || amount <= 0.0 {
            return None;
        }

        self.serial = self.serial.saturating_add(1);
        if let Some((entity, entry)) = self.entries.iter_mut().find(|(_, entry)| {
            entry.target == target && entry.age <= DAMAGE_NUMBER_COALESCE_SECONDS
        }) {
            entry.amount += amount;
            entry.element = element;
            entry.critical |= critical;
            entry.age = 0.0;
            entry.origin = position;
            entry.serial = self.serial;
            self.coalesced = self.coalesced.saturating_add(1);
            let entity = *entity;
            write_damage_number(commands, entity, *entry, None);
            return Some(entity);
        }

        let lease = if self.shells.active.len() < DAMAGE_NUMBER_POOL_CAPACITY {
            self.shells
                .rent_with_status(commands)
                .expect("damage-number capacity was checked before rent")
        } else {
            let entity = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.serial)
                .map(|(entity, _)| *entity)
                .expect("a full damage-number pool has an oldest entry");
            self.entries.remove(&entity);
            self.overwritten = self.overwritten.saturating_add(1);
            EntityLease {
                entity,
                newly_created: false,
            }
        };

        let entry = DamageNumberEntry {
            target,
            amount,
            element,
            critical,
            age: 0.0,
            origin: position,
            serial: self.serial,
        };
        self.entries.insert(lease.entity, entry);
        if lease.newly_created {
            commands.entity(lease.entity).insert((
                PooledDamageNumber,
                TextLayout::justify(Justify::Center),
                RenderLayers::layer(DAMAGE_NUMBER_RENDER_LAYER),
            ));
        }
        write_damage_number(commands, lease.entity, entry, None);
        Some(lease.entity)
    }

    fn tick(
        &mut self,
        commands: &mut Commands,
        delta: f32,
        camera: Option<(&Camera, &GlobalTransform)>,
    ) {
        let mut ordered: Vec<(Entity, Entity, u64)> = self
            .entries
            .iter()
            .map(|(entity, entry)| (*entity, entry.target, entry.serial))
            .collect();
        ordered.sort_by(|left, right| {
            left.1
                .to_bits()
                .cmp(&right.1.to_bits())
                .then_with(|| right.2.cmp(&left.2))
                .then_with(|| left.0.to_bits().cmp(&right.0.to_bits()))
        });
        let mut stack_indices = HashMap::with_capacity(ordered.len());
        let mut previous_target = None;
        let mut stack_index = 0usize;
        for (entity, target, _) in ordered {
            if previous_target == Some(target) {
                stack_index = stack_index.saturating_add(1);
            } else {
                previous_target = Some(target);
                stack_index = 0;
            }
            stack_indices.insert(entity, stack_index);
        }

        let mut expired = Vec::new();
        for (entity, entry) in &mut self.entries {
            entry.age += delta.max(0.0);
            if entry.age >= DAMAGE_NUMBER_LIFETIME_SECONDS {
                expired.push(*entity);
            } else {
                let screen_position = camera.and_then(|(camera, camera_transform)| {
                    damage_number_screen_position(
                        camera,
                        camera_transform,
                        *entry,
                        stack_indices.get(entity).copied().unwrap_or(0),
                    )
                });
                write_damage_number(commands, *entity, *entry, screen_position);
            }
        }
        for entity in expired {
            self.retire(commands, entity);
        }
    }

    fn retire(&mut self, commands: &mut Commands, entity: Entity) -> bool {
        if !self.shells.retire(entity) {
            return false;
        }
        self.entries.remove(&entity);
        commands
            .entity(entity)
            .remove::<ActivePooledDamageNumber>()
            .insert((
                Text2d::new(""),
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.0)),
                Transform::IDENTITY,
                Visibility::Hidden,
            ));
        true
    }

    fn retire_all(&mut self, commands: &mut Commands) {
        for entity in self.shells.active_entities() {
            self.retire(commands, entity);
        }
    }

    fn finalize_pending(&mut self) {
        self.shells.finalize_pending();
    }

    fn metrics(&self) -> DamageNumberPoolMetrics {
        DamageNumberPoolMetrics {
            created: self.shells.created,
            reused: self.shells.reused,
            active: self.shells.active.len(),
            peak: self.shells.peak,
            coalesced: self.coalesced,
            overwritten: self.overwritten,
        }
    }
}

fn write_damage_number(
    commands: &mut Commands,
    entity: Entity,
    entry: DamageNumberEntry,
    screen_position: Option<Vec2>,
) {
    let remaining = DAMAGE_NUMBER_LIFETIME_SECONDS - entry.age;
    let alpha = (remaining / 0.24).clamp(0.0, 1.0);
    commands.entity(entity).insert((
        ActivePooledDamageNumber {
            target: entry.target,
            amount: entry.amount,
            element: entry.element,
            age: entry.age,
        },
        Text2d::new(format_damage_number(entry.amount, entry.critical)),
        TextFont {
            font_size: FontSize::Px(if entry.critical { 48.0 } else { 34.0 }),
            ..default()
        },
        TextColor(damage_number_color(entry).with_alpha(alpha)),
        Transform::from_translation(screen_position.unwrap_or_default().extend(0.0))
            .with_scale(Vec3::splat(damage_number_pop_scale(entry))),
        if screen_position.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        },
    ));
}

fn damage_number_screen_position(
    camera: &Camera,
    camera_transform: &GlobalTransform,
    entry: DamageNumberEntry,
    stack_index: usize,
) -> Option<Vec2> {
    // `world_to_viewport` returns logical coordinates; use the matching
    // logical extent so HiDPI phones do not shift numbers toward a corner.
    let size = camera.logical_viewport_size()?;
    let world_position = entry.origin + Vec3::Y * 1.1;
    let mut viewport = camera
        .world_to_viewport(camera_transform, world_position)
        .ok()?;
    // Reserve the left stats, right challenge panel, top timer, and bottom
    // mobile controls. Off-lane numbers clamp beside those panels instead of
    // drawing over them.
    let left_margin = (size.x * 0.2).clamp(180.0, 260.0);
    let right_margin = (size.x * 0.29).clamp(230.0, 380.0);
    let top_margin = (size.y * 0.15).max(80.0);
    let bottom_margin = (size.y * 0.23).max(130.0);
    viewport.x = viewport.x.clamp(
        left_margin.min(size.x * 0.5),
        (size.x - right_margin).max(size.x * 0.5),
    );
    viewport.y = viewport.y.clamp(
        top_margin.min(size.y * 0.5),
        (size.y - bottom_margin).max(size.y * 0.5),
    );
    let stack_slot = stack_index.min(5);
    let x_offset = [-10.0, 12.0, -20.0, 20.0, -6.0, 8.0][stack_slot];
    let y_offset = stack_slot as f32 * 38.0;
    Some(Vec2::new(
        viewport.x - size.x * 0.5 + x_offset,
        size.y * 0.5 - viewport.y + entry.age * 30.0 + y_offset,
    ))
}

fn format_damage_number(amount: f32, critical: bool) -> String {
    if critical {
        format!("{:.0}!", amount.max(0.0))
    } else {
        format!("{:.0}", amount.max(0.0))
    }
}

/// Fresh numbers overshoot then settle; crits punch harder so they read at a glance.
fn damage_number_pop_scale(entry: DamageNumberEntry) -> f32 {
    let (overshoot, duration) = if entry.critical {
        (0.75, 0.16)
    } else {
        (0.28, 0.10)
    };
    1.0 + overshoot * (1.0 - entry.age / duration).clamp(0.0, 1.0)
}

fn damage_number_color(entry: DamageNumberEntry) -> Color {
    if entry.critical {
        Color::srgb(1.0, 0.82, 0.18)
    } else {
        damage_element_color(entry.element)
    }
}

pub(crate) const fn damage_element_color(element: DamageElement) -> Color {
    match element {
        DamageElement::Physical => Color::srgb(1.0, 0.95, 0.82),
        DamageElement::Storm => Color::srgb(0.38, 0.86, 1.0),
        DamageElement::Flame => Color::srgb(1.0, 0.38, 0.14),
        DamageElement::Frost => Color::srgb(0.58, 0.88, 1.0),
        DamageElement::Tide => Color::srgb(0.28, 1.0, 0.76),
        DamageElement::Arcane => Color::srgb(0.78, 0.5, 1.0),
        DamageElement::Pure => Color::srgb(1.0, 0.88, 0.28),
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
struct PooledSummon {
    visual: SummonVisual,
}

#[derive(Component)]
struct ActivePooledSummon;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
struct PooledEnemy {
    key: EnemySceneKey,
}

#[derive(Component)]
struct ActivePooledEnemy;

/// Lazily creates combat render entities and keeps retired entities unavailable
/// until the `Last` schedule. This prevents an entity from being retired and
/// leased again before its deferred component removals have been applied.
#[derive(Resource, Debug)]
pub struct CombatPools {
    /// The fixed set of mesh-bearing projectile shells.
    projectiles: EntityPool,
    /// Elastic, mesh-free shells used only when all visible shells are active.
    projectile_overflow: ElasticEntityPool,
    projectile_peak: usize,
    projectile_visual_fallbacks: usize,
    impact_fx: EntityPool,
    damage_numbers: DamageNumberPool,
    summons: [EntityPool; SUMMON_VISUAL_COUNT],
    enemies: EnemyScenePool,
    /// Logical x3 splitling groups that could not atomically acquire enemy
    /// shells. Kept alongside the enemy pool so defeat resolution needs no
    /// extra Bevy system parameter and cannot silently lose pressure.
    pending_splitlings: VecDeque<(Vec3, usize)>,
    next_summon_slot: u64,
    summon_peak: usize,
    summon_rejected: usize,
}

impl Default for CombatPools {
    fn default() -> Self {
        Self {
            projectiles: EntityPool::new(PROJECTILE_POOL_CAPACITY),
            projectile_overflow: ElasticEntityPool::default(),
            projectile_peak: 0,
            projectile_visual_fallbacks: 0,
            impact_fx: EntityPool::new(IMPACT_FX_POOL_CAPACITY),
            damage_numbers: DamageNumberPool::default(),
            summons: std::array::from_fn(|_| EntityPool::new(PLAYER_SUMMON_HARD_CAP)),
            enemies: EnemyScenePool::default(),
            pending_splitlings: VecDeque::new(),
            next_summon_slot: 0,
            summon_peak: 0,
            summon_rejected: 0,
        }
    }
}

impl CombatPools {
    pub(crate) fn enqueue_splitlings(&mut self, position: Vec3, logical_count: usize) {
        if logical_count > 0 {
            self.pending_splitlings.push_back((position, logical_count));
        }
    }

    pub(crate) fn pop_pending_splitlings(&mut self) -> Option<(Vec3, usize)> {
        self.pending_splitlings.pop_front()
    }

    pub(crate) fn push_front_pending_splitlings(&mut self, position: Vec3, logical_count: usize) {
        self.pending_splitlings
            .push_front((position, logical_count));
    }

    #[allow(dead_code)]
    pub(crate) fn pending_splitling_groups(&self) -> usize {
        self.pending_splitlings.len()
    }

    #[allow(dead_code)]
    pub(crate) fn enemy_metrics(&self) -> PoolMetrics {
        self.enemies.metrics()
    }

    pub(crate) fn regular_enemy_active_count(&self) -> usize {
        self.enemies.regular_active_count()
    }

    pub(crate) fn enemy_active_capacity_remaining(&self) -> usize {
        ENEMY_POOL_ACTIVE_CAP.saturating_sub(self.enemies.active.len())
    }

    pub(crate) fn rent_enemy_scene(
        &mut self,
        commands: &mut Commands,
        key: EnemySceneKey,
        scene: Handle<WorldAsset>,
        transform: Transform,
    ) -> Option<Entity> {
        self.rent_enemy_scene_group(commands, key, scene, &[transform])
            .and_then(|entities| entities.into_iter().next())
    }

    /// Preflight all model groups together before renting any adventure actors.
    pub(crate) fn rent_enemy_scene_batch(
        &mut self,
        commands: &mut Commands,
        actors: &[(EnemySceneKey, Handle<WorldAsset>, Transform)],
    ) -> Option<Vec<Entity>> {
        let mut groups = BTreeMap::<EnemySceneKey, usize>::new();
        for (key, _, _) in actors {
            *groups.entry(*key).or_default() += 1;
        }
        let mut needed = 0usize;
        let mut regular_needed = 0usize;
        for (key, count) in groups {
            let free = self.enemies.free.get(&key).map_or(0, Vec::len);
            let create = count.saturating_sub(free);
            needed += create;
            if key.is_regular() {
                regular_needed += create;
            }
        }
        if self.enemies.active.len().saturating_add(actors.len()) > ENEMY_POOL_ACTIVE_CAP
            || self.enemies.created.saturating_add(needed) > ENEMY_POOL_CREATED_CAP
            || self.enemies.regular_created.saturating_add(regular_needed)
                > REGULAR_ENEMY_CREATED_CAP
        {
            self.enemies.rejected = self.enemies.rejected.saturating_add(actors.len());
            return None;
        }
        Some(
            actors
                .iter()
                .map(|(key, scene, transform)| {
                    self.rent_enemy_scene(commands, *key, scene.clone(), *transform)
                        .expect("all actor keys and capacities were preflighted")
                })
                .collect(),
        )
    }

    /// Atomically rents every shell or none. This is used by W and boss summon
    /// groups so gameplay state can only advance after the whole group exists.
    pub(crate) fn rent_enemy_scene_group(
        &mut self,
        commands: &mut Commands,
        key: EnemySceneKey,
        scene: Handle<WorldAsset>,
        transforms: &[Transform],
    ) -> Option<Vec<Entity>> {
        let leases = self.enemies.rent_group(commands, key, transforms.len())?;
        let mut entities = Vec::with_capacity(leases.len());
        for (lease, transform) in leases.into_iter().zip(transforms.iter()) {
            let mut entity = commands.entity(lease.entity);
            clear_enemy_gameplay_components(&mut entity);
            if lease.newly_created {
                entity.insert((PooledEnemy { key }, WorldAssetRoot(scene.clone())));
            }
            entity.insert((ActivePooledEnemy, *transform, Visibility::Visible));
            entities.push(lease.entity);
        }
        Some(entities)
    }

    pub(crate) fn track_enemy_decoration(&mut self, enemy: Entity, decoration: Entity) {
        debug_assert!(self.enemies.active.contains_key(&enemy));
        self.enemies
            .decorations
            .entry(enemy)
            .or_default()
            .push(decoration);
    }

    pub(crate) fn retire_enemy(&mut self, commands: &mut Commands, entity: Entity) -> bool {
        if self.enemies.retire(entity).is_none() {
            return false;
        }
        if let Some(decorations) = self.enemies.decorations.remove(&entity) {
            for decoration in decorations {
                commands.entity(decoration).try_despawn();
            }
        }
        let mut entity_commands = commands.entity(entity);
        clear_enemy_gameplay_components(&mut entity_commands);
        entity_commands
            .remove::<ActivePooledEnemy>()
            .insert(Visibility::Hidden);
        true
    }

    pub(crate) fn retire_all_enemies(&mut self, commands: &mut Commands) {
        let active: Vec<_> = self.enemies.active.keys().copied().collect();
        for entity in active {
            self.retire_enemy(commands, entity);
        }
        self.pending_splitlings.clear();
    }

    fn advance_enemy_reuse_clock(&mut self, delta: f32) {
        self.enemies.advance(delta);
    }

    #[allow(dead_code)]
    pub(crate) fn projectile_metrics(&self) -> PoolMetrics {
        let visible = self.projectiles.metrics();
        let overflow = self.projectile_overflow.metrics();
        PoolMetrics {
            created: visible.created.saturating_add(overflow.created),
            reused: visible.reused.saturating_add(overflow.reused),
            active: visible.active.saturating_add(overflow.active),
            peak: self.projectile_peak,
            // Visible exhaustion is an intentional presentation fallback, not
            // a rejected logical shot. Elastic ECS allocation has no normal
            // rejection path.
            rejected: 0,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn projectile_visible_metrics(&self) -> PoolMetrics {
        self.projectiles.metrics()
    }

    #[allow(dead_code)]
    pub(crate) const fn projectile_visual_fallbacks(&self) -> usize {
        self.projectile_visual_fallbacks
    }

    pub(crate) fn impact_fx_metrics(&self) -> PoolMetrics {
        self.impact_fx.metrics()
    }

    #[allow(dead_code)]
    pub(crate) fn damage_number_metrics(&self) -> DamageNumberPoolMetrics {
        self.damage_numbers.metrics()
    }

    #[allow(dead_code)]
    pub(crate) fn summon_metrics(&self) -> PoolMetrics {
        PoolMetrics {
            created: self.summons.iter().map(|pool| pool.created).sum(),
            reused: self.summons.iter().map(|pool| pool.reused).sum(),
            active: self.summon_active_count(),
            peak: self.summon_peak,
            rejected: self.summon_rejected,
        }
    }

    #[allow(dead_code)]
    pub fn snapshot(&self) -> CombatPoolSnapshot {
        CombatPoolSnapshot {
            projectiles: self.projectile_metrics(),
            projectile_visible: self.projectile_visible_metrics(),
            projectile_visual_fallbacks: self.projectile_visual_fallbacks,
            impact_fx: self.impact_fx_metrics(),
            summons: self.summon_metrics(),
            enemies: self.enemy_metrics(),
            damage_numbers: self.damage_number_metrics(),
        }
    }

    pub(crate) fn summon_active_count(&self) -> usize {
        self.summons.iter().map(|pool| pool.active.len()).sum()
    }

    fn rent_summon_shell(
        &mut self,
        commands: &mut Commands,
        visual: SummonVisual,
    ) -> Option<EntityLease> {
        if self.summon_active_count() >= PLAYER_SUMMON_HARD_CAP {
            self.summon_rejected = self.summon_rejected.saturating_add(1);
            return None;
        }
        let Some(lease) = self.summons[visual.index()].rent_with_status(commands) else {
            self.summon_rejected = self.summon_rejected.saturating_add(1);
            return None;
        };
        let mut entity = commands.entity(lease.entity);
        clear_summon_gameplay_components(&mut entity);
        entity.insert(super::summon_movement::FormationSlot(self.next_summon_slot));
        self.next_summon_slot = self.next_summon_slot.saturating_add(1);
        self.summon_peak = self.summon_peak.max(self.summon_active_count());
        Some(lease)
    }

    pub(crate) fn rent_scene_summon(
        &mut self,
        commands: &mut Commands,
        visual: SummonVisual,
        scene: Handle<WorldAsset>,
        transform: Transform,
    ) -> Option<Entity> {
        debug_assert!(visual.uses_world_asset());
        let lease = self.rent_summon_shell(commands, visual)?;
        let mut entity = commands.entity(lease.entity);
        if lease.newly_created {
            entity.insert((PooledSummon { visual }, WorldAssetRoot(scene)));
        }
        entity.insert((
            ActivePooledSummon,
            PlayerSummon,
            transform,
            Visibility::Visible,
        ));
        Some(lease.entity)
    }

    pub(crate) fn rent_mesh_summon(
        &mut self,
        commands: &mut Commands,
        visual: SummonVisual,
        mesh: Handle<Mesh>,
        material: Handle<StandardMaterial>,
        transform: Transform,
    ) -> Option<Entity> {
        debug_assert!(!visual.uses_world_asset());
        let lease = self.rent_summon_shell(commands, visual)?;
        let mut entity = commands.entity(lease.entity);
        if lease.newly_created {
            entity.insert((
                PooledSummon { visual },
                Mesh3d(mesh),
                MeshMaterial3d(material),
            ));
        }
        entity.insert((
            ActivePooledSummon,
            PlayerSummon,
            transform,
            Visibility::Visible,
        ));
        Some(lease.entity)
    }

    pub(crate) fn rent_projectile(
        &mut self,
        commands: &mut Commands,
        mesh: Handle<Mesh>,
        material: Handle<StandardMaterial>,
        transform: Transform,
        projectile: Projectile,
    ) -> Option<Entity> {
        let (lease, visible) = if let Some(lease) = self.projectiles.rent_with_status(commands) {
            (lease, true)
        } else {
            self.projectile_visual_fallbacks = self.projectile_visual_fallbacks.saturating_add(1);
            (self.projectile_overflow.rent(commands), false)
        };
        let entity = lease.entity;
        let mut entity_commands = commands.entity(entity);
        entity_commands
            .remove::<(
                ProjectileHitMemory,
                super::basic_attacks::Ricochet,
                ProjectileFirstFrameTravel,
                super::attack_vfx::WeaponFlight,
                super::hero_fusion::FusionAttack,
                Name,
                AbilityGenerated,
                DamageProfile,
            )>()
            .insert((
                PooledProjectile,
                ActivePooledProjectile,
                transform,
                projectile,
            ));
        if visible {
            entity_commands
                .remove::<LogicOnlyPooledProjectile>()
                .insert((Mesh3d(mesh), MeshMaterial3d(material), Visibility::Visible));
        } else {
            entity_commands
                .remove::<(Mesh3d, MeshMaterial3d<StandardMaterial>)>()
                .insert((LogicOnlyPooledProjectile, Visibility::Hidden));
        }
        self.projectile_peak = self
            .projectile_peak
            .max(self.projectiles.active.len() + self.projectile_overflow.active.len());
        Some(entity)
    }

    pub(crate) fn show_damage_number(
        &mut self,
        commands: &mut Commands,
        target: Entity,
        position: Vec3,
        amount: f32,
        element: DamageElement,
        critical: bool,
    ) -> Option<Entity> {
        self.damage_numbers
            .show(commands, target, position, amount, element, critical)
    }

    pub(crate) fn tick_damage_numbers(
        &mut self,
        commands: &mut Commands,
        delta: f32,
        camera: Option<(&Camera, &GlobalTransform)>,
    ) {
        self.damage_numbers.tick(commands, delta, camera);
    }

    pub(crate) fn retire_all_damage_numbers(&mut self, commands: &mut Commands) {
        self.damage_numbers.retire_all(commands);
    }

    pub(crate) fn rent_impact_fx(
        &mut self,
        commands: &mut Commands,
        mesh: Handle<Mesh>,
        material: Handle<StandardMaterial>,
        transform: Transform,
        impact_fx: ImpactFx,
    ) -> Option<Entity> {
        self.rent_named_impact_fx(commands, mesh, material, transform, impact_fx, None)
    }

    pub(crate) fn rent_named_impact_fx(
        &mut self,
        commands: &mut Commands,
        mesh: Handle<Mesh>,
        material: Handle<StandardMaterial>,
        transform: Transform,
        impact_fx: ImpactFx,
        name: Option<Name>,
    ) -> Option<Entity> {
        let entity = self.impact_fx.rent(commands)?;
        let mut entity_commands = commands.entity(entity);
        entity_commands
            .remove::<(Name, GroundLightProfile, BossPhaseBurstOwner)>()
            .insert((
                PooledImpactFx,
                ActivePooledImpactFx,
                Mesh3d(mesh),
                MeshMaterial3d(material),
                transform,
                Visibility::Visible,
                impact_fx,
            ));
        if let Some(name) = name {
            entity_commands.insert(name);
        }
        Some(entity)
    }

    pub(crate) fn retire_projectile(&mut self, commands: &mut Commands, entity: Entity) {
        if !self.projectiles.retire(entity) && !self.projectile_overflow.retire(entity) {
            return;
        }
        commands
            .entity(entity)
            .remove::<(
                Projectile,
                ActivePooledProjectile,
                ProjectileHitMemory,
                super::basic_attacks::Ricochet,
                ProjectileFirstFrameTravel,
                super::attack_vfx::WeaponFlight,
                super::hero_fusion::FusionAttack,
                Name,
                AbilityGenerated,
                DamageProfile,
            )>()
            .insert(Visibility::Hidden);
    }

    pub(crate) fn retire_impact_fx(&mut self, commands: &mut Commands, entity: Entity) {
        if !self.impact_fx.retire(entity) {
            return;
        }
        commands
            .entity(entity)
            .remove::<(
                ImpactFx,
                ActivePooledImpactFx,
                Name,
                GroundLightProfile,
                BossPhaseBurstOwner,
            )>()
            .insert(Visibility::Hidden);
    }

    pub(crate) fn retire_summon(&mut self, commands: &mut Commands, entity: Entity) -> bool {
        let Some(index) = self
            .summons
            .iter()
            .position(|pool| pool.active.contains(&entity))
        else {
            return false;
        };
        if !self.summons[index].retire(entity) {
            return false;
        }
        let mut entity_commands = commands.entity(entity);
        clear_summon_gameplay_components(&mut entity_commands);
        entity_commands
            .remove::<(ActivePooledSummon, PlayerSummon)>()
            .insert(Visibility::Hidden);
        true
    }

    pub(crate) fn retire_all_projectiles(&mut self, commands: &mut Commands) {
        for entity in self.projectiles.active_entities() {
            self.retire_projectile(commands, entity);
        }
        for entity in self.projectile_overflow.active_entities() {
            self.retire_projectile(commands, entity);
        }
    }

    pub(crate) fn retire_all_effects(&mut self, commands: &mut Commands) {
        for entity in self.impact_fx.active_entities() {
            self.retire_impact_fx(commands, entity);
        }
    }

    fn finalize_pending(&mut self) {
        self.projectiles.finalize_pending();
        self.projectile_overflow.finalize_pending();
        self.impact_fx.finalize_pending();
        self.damage_numbers.finalize_pending();
        for pool in &mut self.summons {
            pool.finalize_pending();
        }
    }
}

fn clear_summon_gameplay_components(entity: &mut EntityCommands<'_>) {
    entity
        .remove::<(
            SkillSummon,
            super::summon_movement::FormationSlot,
            BoneSentry,
            SpiritHound,
            TideElemental,
            BoltTurret,
            FrostWyrm,
            AncestorColossus,
            OathBanner,
            SpiritOrbit,
            PassiveSpirit,
        )>()
        .remove::<(Name, AbilityGenerated, DamageProfile, ProjectileHitMemory)>();
}

fn clear_enemy_gameplay_components(entity: &mut EntityCommands<'_>) {
    entity
        .remove::<(
            super::encounter_bosses::EncounterBoss,
            super::encounter_bosses::EncounterGuard,
            super::encounter_bosses::CampCombatant,
            super::encounter_bosses::CampArrival,
            super::encounter_bosses::AdventureSkillClock,
        )>()
        .remove::<(
            Enemy,
            EnemyArchetype,
            EnemyHeroThreat,
            BossEnemy,
            BossSummon,
            TrialEnemy,
            ArchiveEcho,
            EnemyArmor,
            EnemyBreachDamage,
            EliteAffix,
            CommanderAuraState,
            ChallengeFighter,
        )>()
        .remove::<(
            TimedSpeedEffect,
            WetMark,
            VoidPull,
            Infection,
            Name,
            DamageProfile,
            HitPunch,
        )>();
}

pub(crate) struct PoolPlugin;

impl Plugin for PoolPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CombatPools>()
            .add_systems(Last, (finalize_recycles, advance_enemy_reuse_quarantine));
    }
}

pub(crate) fn finalize_recycles(mut pools: ResMut<CombatPools>) {
    pools.finalize_pending();
}

fn advance_enemy_reuse_quarantine(
    time: Res<Time>,
    session: Option<Res<super::GameSession>>,
    mut pools: ResMut<CombatPools>,
) {
    if session.is_some_and(|session| session.paused) {
        return;
    }
    pools.advance_enemy_reuse_clock(time.delta_secs());
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::super::combat::MAX_REGULAR_ENEMIES;
    use super::*;

    fn projectile(seed: f32) -> Projectile {
        Projectile {
            velocity: Vec3::new(seed, 0.0, -seed),
            damage: 10.0 + seed,
            radius: 0.4,
            remaining_hits: 1,
            lifetime: 1.5,
        }
    }

    fn impact_fx(seed: f32) -> ImpactFx {
        ImpactFx {
            lifetime: 0.2 + seed,
            start_scale: 0.1,
            end_scale: 0.8 + seed,
        }
    }

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_plugins(PoolPlugin);
        app
    }

    fn rent_one_projectile(world: &mut World, seed: f32) -> Entity {
        world
            .run_system_once(
                move |mut commands: Commands, mut pools: ResMut<CombatPools>| {
                    pools.rent_projectile(
                        &mut commands,
                        Handle::default(),
                        Handle::default(),
                        Transform::from_xyz(seed, 0.25, -seed),
                        projectile(seed),
                    )
                },
            )
            .expect("rent system should run")
            .expect("projectile capacity should be available")
    }

    fn retire_one_projectile(world: &mut World, entity: Entity) {
        world
            .run_system_once(
                move |mut commands: Commands, mut pools: ResMut<CombatPools>| {
                    pools.retire_projectile(&mut commands, entity);
                },
            )
            .expect("retire system should run");
    }

    fn show_damage_number(
        world: &mut World,
        target: Entity,
        amount: f32,
        element: DamageElement,
    ) -> Option<Entity> {
        world
            .run_system_once(
                move |mut commands: Commands, mut pools: ResMut<CombatPools>| {
                    pools.show_damage_number(
                        &mut commands,
                        target,
                        Vec3::new(2.0, 0.0, -1.0),
                        amount,
                        element,
                        false,
                    )
                },
            )
            .expect("damage-number show system should run")
    }

    fn tick_damage_numbers(world: &mut World, delta: f32, _camera_rotation: Quat) {
        world
            .run_system_once(
                move |mut commands: Commands, mut pools: ResMut<CombatPools>| {
                    pools.tick_damage_numbers(&mut commands, delta, None);
                },
            )
            .expect("damage-number tick system should run");
    }

    fn retire_one_damage_number(world: &mut World, entity: Entity) {
        world
            .run_system_once(
                move |mut commands: Commands, mut pools: ResMut<CombatPools>| {
                    assert!(pools.damage_numbers.retire(&mut commands, entity));
                },
            )
            .expect("damage-number retire system should run");
    }

    fn rent_one_scene_summon(world: &mut World, visual: SummonVisual, seed: f32) -> Entity {
        world
            .run_system_once(
                move |mut commands: Commands, mut pools: ResMut<CombatPools>| {
                    pools.rent_scene_summon(
                        &mut commands,
                        visual,
                        Handle::default(),
                        Transform::from_xyz(seed, 0.0, -seed),
                    )
                },
            )
            .expect("summon rent system should run")
            .expect("summon capacity should be available")
    }

    fn rent_one_mesh_summon(world: &mut World, visual: SummonVisual, seed: f32) -> Entity {
        world
            .run_system_once(
                move |mut commands: Commands, mut pools: ResMut<CombatPools>| {
                    pools.rent_mesh_summon(
                        &mut commands,
                        visual,
                        Handle::default(),
                        Handle::default(),
                        Transform::from_xyz(seed, 0.0, -seed),
                    )
                },
            )
            .expect("mesh summon rent system should run")
            .expect("mesh summon capacity should be available")
    }

    fn retire_one_summon(world: &mut World, entity: Entity) {
        world
            .run_system_once(
                move |mut commands: Commands, mut pools: ResMut<CombatPools>| {
                    pools.retire_summon(&mut commands, entity);
                },
            )
            .expect("summon retire system should run");
    }

    fn try_rent_one_enemy(
        world: &mut World,
        key: EnemySceneKey,
        scene: Handle<WorldAsset>,
        seed: f32,
    ) -> Option<Entity> {
        world
            .run_system_once(
                move |mut commands: Commands, mut pools: ResMut<CombatPools>| {
                    pools.rent_enemy_scene(
                        &mut commands,
                        key,
                        scene.clone(),
                        Transform::from_xyz(seed, 0.0, -seed),
                    )
                },
            )
            .expect("enemy rent system should run")
    }

    fn rent_one_enemy(
        world: &mut World,
        key: EnemySceneKey,
        scene: Handle<WorldAsset>,
        seed: f32,
    ) -> Entity {
        try_rent_one_enemy(world, key, scene, seed)
            .expect("enemy scene capacity should be available")
    }

    fn retire_one_enemy(world: &mut World, entity: Entity) {
        world
            .run_system_once(
                move |mut commands: Commands, mut pools: ResMut<CombatPools>| {
                    assert!(pools.retire_enemy(&mut commands, entity));
                },
            )
            .expect("enemy retire system should run");
    }

    fn advance_enemy_clock(world: &mut World, delta: f32) {
        world
            .resource_mut::<CombatPools>()
            .advance_enemy_reuse_clock(delta);
    }

    fn finalize(world: &mut World) {
        world
            .run_system_once(finalize_recycles)
            .expect("finalizer should run");
    }

    #[test]
    fn enemy_pool_rejects_n_plus_one_and_special_groups_atomically() {
        let mut app = test_app();
        let (leased, group, extra) = app
            .world_mut()
            .run_system_once(|mut commands: Commands, mut pools: ResMut<CombatPools>| {
                let transforms = vec![Transform::default(); ENEMY_POOL_ACTIVE_CAP];
                let leased = pools
                    .rent_enemy_scene_group(
                        &mut commands,
                        EnemySceneKey::Boss(0),
                        Handle::default(),
                        &transforms,
                    )
                    .expect("the global active capacity should be rentable");
                let group = pools.rent_enemy_scene_group(
                    &mut commands,
                    EnemySceneKey::Gold,
                    Handle::default(),
                    &vec![Transform::default(); 10],
                );
                let extra = pools.rent_enemy_scene(
                    &mut commands,
                    EnemySceneKey::Archive,
                    Handle::default(),
                    Transform::default(),
                );
                (leased, group, extra)
            })
            .expect("enemy capacity system should run");

        assert_eq!(leased.len(), ENEMY_POOL_ACTIVE_CAP);
        assert!(group.is_none(), "a ten-target W group must be all-or-none");
        assert!(extra.is_none());
        let metrics = app.world().resource::<CombatPools>().enemy_metrics();
        assert_eq!(metrics.created(), ENEMY_POOL_ACTIVE_CAP);
        assert_eq!(metrics.active(), ENEMY_POOL_ACTIVE_CAP);
        assert_eq!(metrics.peak(), ENEMY_POOL_ACTIVE_CAP);
        assert_eq!(metrics.rejected(), 11);
    }

    #[test]
    fn regular_retention_cannot_consume_the_thirty_special_creation_slots() {
        let mut app = test_app();
        let (regular_active, denied_regular, boss) = app
            .world_mut()
            .run_system_once(|mut commands: Commands, mut pools: ResMut<CombatPools>| {
                let quarantined = pools
                    .rent_enemy_scene_group(
                        &mut commands,
                        EnemySceneKey::Regular(0),
                        Handle::default(),
                        &vec![Transform::default(); 96],
                    )
                    .expect("first regular generation should fit");
                for entity in quarantined {
                    assert!(pools.retire_enemy(&mut commands, entity));
                }
                let regular_active = pools
                    .rent_enemy_scene_group(
                        &mut commands,
                        EnemySceneKey::Regular(0),
                        Handle::default(),
                        &vec![Transform::default(); MAX_REGULAR_ENEMIES],
                    )
                    .expect("130 active regular shells should still fit");
                let denied_regular = pools.rent_enemy_scene(
                    &mut commands,
                    EnemySceneKey::Regular(1),
                    Handle::default(),
                    Transform::default(),
                );
                let boss = pools.rent_enemy_scene(
                    &mut commands,
                    EnemySceneKey::Boss(0),
                    Handle::default(),
                    Transform::default(),
                );
                (regular_active, denied_regular, boss)
            })
            .expect("special reservation system should run");

        assert_eq!(regular_active.len(), MAX_REGULAR_ENEMIES);
        assert!(denied_regular.is_none());
        assert!(
            boss.is_some(),
            "a due boss retains a guaranteed creation slot"
        );
        let pools = app.world().resource::<CombatPools>();
        assert_eq!(pools.enemies.regular_created, REGULAR_ENEMY_CREATED_CAP);
        assert_eq!(
            pools.enemy_metrics().created(),
            REGULAR_ENEMY_CREATED_CAP + 1
        );
        assert_eq!(pools.regular_enemy_active_count(), MAX_REGULAR_ENEMIES);
    }

    #[test]
    fn enemy_scene_shell_waits_two_point_five_seconds_and_keeps_only_render_children() {
        let mut app = test_app();
        let scene: Handle<WorldAsset> =
            bevy::asset::uuid_handle!("00000000-0000-0000-0000-000000000001");
        let first = rent_one_enemy(
            app.world_mut(),
            EnemySceneKey::Regular(3),
            scene.clone(),
            1.0,
        );
        let root = app
            .world()
            .get::<WorldAssetRoot>(first)
            .expect("new enemy shell has a world root")
            .clone();
        let render_child = app
            .world_mut()
            .spawn(Name::new("PERSISTENT WORLD-ASSET CHILD"))
            .id();
        let decoration = app
            .world_mut()
            .spawn(Name::new("TRANSIENT AFFIX CHILD"))
            .id();
        app.world_mut().entity_mut(first).add_child(render_child);
        app.world_mut().entity_mut(first).add_child(decoration);
        app.world_mut()
            .resource_mut::<CombatPools>()
            .track_enemy_decoration(first, decoration);
        app.world_mut().entity_mut(first).insert((
            Name::new("DIRTY ENEMY"),
            Enemy {
                hp: 1.0,
                max_hp: 2.0,
                speed: 3.0,
                damage: 4.0,
                xp: 5,
                wood: 6,
                gold: 7,
                kill_credit: 8,
                elite: true,
            },
            BossEnemy { index: 4 },
            EnemyArmor(9.0),
            TimedSpeedEffect {
                multiplier: 0.3,
                remaining: 7.0,
            },
            WetMark { remaining: 6.0 },
            Infection::new(3, 1, 50.0),
            VoidPull {
                center: Vec3::X,
                remaining: 0.5,
                speed: 3.0,
            },
        ));

        retire_one_enemy(app.world_mut(), first);
        assert_eq!(
            app.world().get::<Visibility>(first),
            Some(&Visibility::Hidden)
        );
        assert!(app.world().get::<Enemy>(first).is_none());
        assert!(app.world().get::<BossEnemy>(first).is_none());
        assert!(app.world().get::<EnemyArmor>(first).is_none());
        assert!(app.world().get::<TimedSpeedEffect>(first).is_none());
        assert!(app.world().get::<WetMark>(first).is_none());
        assert!(app.world().get::<VoidPull>(first).is_none());
        assert!(app.world().get::<Infection>(first).is_none());
        assert!(app.world().get::<Name>(first).is_none());
        assert!(app.world().entities().contains(render_child));
        assert!(!app.world().entities().contains(decoration));

        let before_quarantine = rent_one_enemy(
            app.world_mut(),
            EnemySceneKey::Regular(3),
            scene.clone(),
            2.0,
        );
        assert_ne!(before_quarantine, first);
        advance_enemy_clock(app.world_mut(), ENEMY_REUSE_QUARANTINE_SECS - 0.001);
        assert!(
            app.world()
                .resource::<CombatPools>()
                .enemies
                .free
                .get(&EnemySceneKey::Regular(3))
                .is_none_or(Vec::is_empty)
        );
        advance_enemy_clock(app.world_mut(), 0.001);
        app.world_mut().clear_trackers();
        let reused = rent_one_enemy(app.world_mut(), EnemySceneKey::Regular(3), scene, 3.0);
        assert_eq!(reused, first);
        assert_eq!(app.world().get::<WorldAssetRoot>(reused), Some(&root));
        assert!(
            !app.world()
                .entity(reused)
                .get_ref::<WorldAssetRoot>()
                .expect("reused enemy keeps its root")
                .is_changed()
        );
        assert!(
            app.world()
                .get::<Children>(reused)
                .is_some_and(|children| children.contains(&render_child))
        );
        assert_eq!(
            app.world().get::<PooledEnemy>(reused).map(|pool| pool.key),
            Some(EnemySceneKey::Regular(3))
        );
    }

    #[test]
    fn enemy_keys_never_exchange_entities_or_scene_handles() {
        let mut app = test_app();
        let first_scene: Handle<WorldAsset> =
            bevy::asset::uuid_handle!("00000000-0000-0000-0000-000000000011");
        let second_scene: Handle<WorldAsset> =
            bevy::asset::uuid_handle!("00000000-0000-0000-0000-000000000012");
        let first = rent_one_enemy(
            app.world_mut(),
            EnemySceneKey::Regular(0),
            first_scene.clone(),
            0.0,
        );
        retire_one_enemy(app.world_mut(), first);
        advance_enemy_clock(app.world_mut(), ENEMY_REUSE_QUARANTINE_SECS);

        let other = rent_one_enemy(
            app.world_mut(),
            EnemySceneKey::Regular(1),
            second_scene.clone(),
            1.0,
        );
        assert_ne!(other, first);
        assert_eq!(
            app.world().get::<WorldAssetRoot>(other),
            Some(&WorldAssetRoot(second_scene))
        );
        let reused = rent_one_enemy(
            app.world_mut(),
            EnemySceneKey::Regular(0),
            first_scene.clone(),
            2.0,
        );
        assert_eq!(reused, first);
        assert_eq!(
            app.world().get::<WorldAssetRoot>(reused),
            Some(&WorldAssetRoot(first_scene))
        );
    }

    #[test]
    fn paused_game_time_does_not_advance_enemy_quarantine() {
        let mut app = test_app();
        app.insert_resource(super::super::GameSession {
            paused: true,
            ..default()
        });
        let enemy = rent_one_enemy(
            app.world_mut(),
            EnemySceneKey::Archive,
            Handle::default(),
            0.0,
        );
        retire_one_enemy(app.world_mut(), enemy);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(3));
        app.world_mut()
            .run_system_once(advance_enemy_reuse_quarantine)
            .expect("paused quarantine system should run");
        assert!(
            app.world()
                .resource::<CombatPools>()
                .enemies
                .free
                .get(&EnemySceneKey::Archive)
                .is_none_or(Vec::is_empty)
        );

        app.world_mut()
            .resource_mut::<super::super::GameSession>()
            .paused = false;
        app.world_mut()
            .run_system_once(advance_enemy_reuse_quarantine)
            .expect("running quarantine system should run");
        assert_eq!(
            app.world()
                .resource::<CombatPools>()
                .enemies
                .free
                .get(&EnemySceneKey::Archive)
                .map(Vec::len),
            Some(1)
        );
    }

    #[test]
    fn unified_snapshot_keeps_release_rejection_counters() {
        let mut app = test_app();
        let _ = rent_one_enemy(
            app.world_mut(),
            EnemySceneKey::Archive,
            Handle::default(),
            0.0,
        );
        let snapshot = app.world().resource::<CombatPools>().snapshot();
        assert_eq!(snapshot.enemies.created(), 1);
        assert_eq!(snapshot.enemies.active(), 1);
        assert_eq!(snapshot.projectiles.rejected(), 0);
    }

    #[test]
    fn projectile_n_plus_one_keeps_logic_and_omits_only_the_overflow_mesh() {
        let mut app = test_app();
        assert_eq!(
            app.world()
                .resource::<CombatPools>()
                .projectile_metrics()
                .created(),
            0
        );

        let leases = app
            .world_mut()
            .run_system_once(
                |mut commands: Commands, mut pools: ResMut<CombatPools>| -> Vec<Option<Entity>> {
                    (0..=PROJECTILE_POOL_CAPACITY)
                        .map(|index| {
                            pools.rent_projectile(
                                &mut commands,
                                Handle::default(),
                                Handle::default(),
                                Transform::from_xyz(index as f32, 0.0, 0.0),
                                projectile(index as f32),
                            )
                        })
                        .collect()
                },
            )
            .expect("batch rent system should run");

        assert!(
            leases[..PROJECTILE_POOL_CAPACITY]
                .iter()
                .all(Option::is_some)
        );
        assert!(leases.iter().all(Option::is_some));
        let overflow = leases[PROJECTILE_POOL_CAPACITY].expect("257th shot stays logical");
        assert!(app.world().get::<Projectile>(overflow).is_some());
        assert!(app.world().get::<Transform>(overflow).is_some());
        assert!(app.world().get::<Mesh3d>(overflow).is_none());
        assert!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(overflow)
                .is_none()
        );
        assert!(
            app.world()
                .get::<LogicOnlyPooledProjectile>(overflow)
                .is_some()
        );
        let pools = app.world().resource::<CombatPools>();
        let metrics = pools.projectile_metrics();
        assert_eq!(metrics.created(), PROJECTILE_POOL_CAPACITY + 1);
        assert_eq!(metrics.reused(), 0);
        assert_eq!(metrics.active(), PROJECTILE_POOL_CAPACITY + 1);
        assert_eq!(metrics.peak(), PROJECTILE_POOL_CAPACITY + 1);
        assert_eq!(metrics.rejected(), 0, "logical damage was not rejected");
        let visible = pools.projectile_visible_metrics();
        assert_eq!(visible.created(), PROJECTILE_POOL_CAPACITY);
        assert_eq!(visible.active(), PROJECTILE_POOL_CAPACITY);
        assert_eq!(visible.peak(), PROJECTILE_POOL_CAPACITY);
        assert_eq!(visible.rejected(), 1);
        assert_eq!(pools.projectile_visual_fallbacks(), 1);
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, (With<Projectile>, With<Mesh3d>)>()
                .iter(app.world())
                .count(),
            PROJECTILE_POOL_CAPACITY
        );
    }

    #[test]
    fn logic_only_projectile_preserves_metadata_and_reuses_after_last() {
        let mut app = test_app();
        let leases: Vec<Entity> = (0..PROJECTILE_POOL_CAPACITY)
            .map(|index| rent_one_projectile(app.world_mut(), index as f32))
            .collect();
        assert_eq!(leases.len(), PROJECTILE_POOL_CAPACITY);
        let overflow = rent_one_projectile(app.world_mut(), 900.0);
        app.world_mut().entity_mut(overflow).insert((
            ProjectileHitMemory::default(),
            AbilityGenerated,
            DamageProfile::new(
                DamageElement::Storm,
                super::super::bonds::DamageClass::Spell,
            ),
        ));

        let logical = app
            .world()
            .get::<Projectile>(overflow)
            .expect("overflow participates in projectile gameplay queries");
        assert_eq!(logical.damage, 910.0);
        assert!(app.world().get::<ProjectileHitMemory>(overflow).is_some());
        assert_eq!(
            app.world()
                .get::<DamageProfile>(overflow)
                .map(|p| p.element),
            Some(DamageElement::Storm)
        );
        assert!(app.world().get::<Mesh3d>(overflow).is_none());

        retire_one_projectile(app.world_mut(), overflow);
        assert!(app.world().get::<Projectile>(overflow).is_none());
        assert!(app.world().get::<ProjectileHitMemory>(overflow).is_none());
        assert!(app.world().get::<DamageProfile>(overflow).is_none());
        assert!(app.world().get::<Mesh3d>(overflow).is_none());
        finalize(app.world_mut());

        let reused = rent_one_projectile(app.world_mut(), 901.0);
        assert_eq!(reused, overflow);
        assert_eq!(
            app.world()
                .get::<Projectile>(reused)
                .map(|shot| shot.damage),
            Some(911.0)
        );
        assert!(app.world().get::<ProjectileHitMemory>(reused).is_none());
        assert!(app.world().get::<DamageProfile>(reused).is_none());
        assert!(app.world().get::<Mesh3d>(reused).is_none());
        let pools = app.world().resource::<CombatPools>();
        assert_eq!(pools.projectile_overflow.metrics().created(), 1);
        assert_eq!(pools.projectile_overflow.metrics().reused(), 1);
        assert_eq!(pools.projectile_metrics().rejected(), 0);
        assert_eq!(pools.projectile_visual_fallbacks(), 2);
    }

    #[test]
    fn damage_number_pool_overwrites_oldest_at_sixty_four_without_growing() {
        let mut app = test_app();
        let mut leases = Vec::new();
        for index in 0..=DAMAGE_NUMBER_POOL_CAPACITY {
            leases.push(
                show_damage_number(
                    app.world_mut(),
                    Entity::from_bits(10_000 + index as u64),
                    index as f32 + 1.0,
                    DamageElement::Physical,
                )
                .expect("positive HP damage always gets a visual slot"),
            );
        }

        assert_eq!(leases[0], leases[DAMAGE_NUMBER_POOL_CAPACITY]);
        let metrics = app
            .world()
            .resource::<CombatPools>()
            .damage_number_metrics();
        assert_eq!(metrics.created(), DAMAGE_NUMBER_POOL_CAPACITY);
        assert_eq!(metrics.reused(), 0);
        assert_eq!(metrics.active(), DAMAGE_NUMBER_POOL_CAPACITY);
        assert_eq!(metrics.peak(), DAMAGE_NUMBER_POOL_CAPACITY);
        assert_eq!(metrics.coalesced(), 0);
        assert_eq!(metrics.overwritten(), 1);
        assert_eq!(
            app.world()
                .get::<ActivePooledDamageNumber>(leases[0])
                .map(|number| number.target),
            Some(Entity::from_bits(
                10_000 + DAMAGE_NUMBER_POOL_CAPACITY as u64
            ))
        );
        assert_eq!(
            app.world()
                .get::<Text2d>(leases[0])
                .map(|text| text.0.as_str()),
            Some("65")
        );
        let mut query = app
            .world_mut()
            .query_filtered::<Entity, With<ActivePooledDamageNumber>>();
        assert_eq!(query.iter(app.world()).count(), DAMAGE_NUMBER_POOL_CAPACITY);
    }

    #[test]
    fn damage_numbers_coalesce_same_target_and_use_latest_element_color() {
        let mut app = test_app();
        let target = Entity::from_bits(42);
        let first = show_damage_number(app.world_mut(), target, 10.4, DamageElement::Physical)
            .expect("first number should show");
        tick_damage_numbers(app.world_mut(), 0.1, Quat::from_rotation_y(0.25));
        let coalesced = show_damage_number(app.world_mut(), target, 4.6, DamageElement::Storm)
            .expect("coalesced number should show");

        assert_eq!(coalesced, first);
        assert_eq!(
            app.world().get::<Text2d>(first).map(|text| text.0.as_str()),
            Some("15")
        );
        assert_eq!(
            app.world().get::<TextColor>(first),
            Some(&TextColor(damage_element_color(DamageElement::Storm)))
        );
        let state = app
            .world()
            .get::<ActivePooledDamageNumber>(first)
            .expect("coalesced lease remains active");
        assert_eq!(state.target, target);
        assert_eq!(state.element, DamageElement::Storm);
        assert_eq!(state.age, 0.0);
        assert!((state.amount - 15.0).abs() < 0.001);
        let metrics = app
            .world()
            .resource::<CombatPools>()
            .damage_number_metrics();
        assert_eq!(metrics.created(), 1);
        assert_eq!(metrics.active(), 1);
        assert_eq!(metrics.coalesced(), 1);
    }

    #[test]
    fn damage_number_colors_are_distinct_and_zero_hp_damage_spawns_nothing() {
        let mut app = test_app();
        assert!(
            show_damage_number(
                app.world_mut(),
                Entity::from_bits(1),
                0.0,
                DamageElement::Pure,
            )
            .is_none()
        );
        assert!(
            show_damage_number(
                app.world_mut(),
                Entity::from_bits(2),
                -1.0,
                DamageElement::Flame,
            )
            .is_none()
        );
        for (index, element) in [
            DamageElement::Physical,
            DamageElement::Storm,
            DamageElement::Flame,
            DamageElement::Frost,
            DamageElement::Tide,
            DamageElement::Arcane,
            DamageElement::Pure,
        ]
        .into_iter()
        .enumerate()
        {
            let entity = show_damage_number(
                app.world_mut(),
                Entity::from_bits(100 + index as u64),
                1.0,
                element,
            )
            .expect("positive HP loss gets a number");
            assert_eq!(
                app.world().get::<TextColor>(entity),
                Some(&TextColor(damage_element_color(element)))
            );
        }
        let metrics = app
            .world()
            .resource::<CombatPools>()
            .damage_number_metrics();
        assert_eq!(metrics.created(), 7);
        assert_eq!(metrics.active(), 7);
    }

    #[test]
    fn damage_number_retirement_clears_visual_state_and_reuses_only_after_last() {
        let mut app = test_app();
        let first = show_damage_number(
            app.world_mut(),
            Entity::from_bits(1),
            12.0,
            DamageElement::Flame,
        )
        .expect("first number should show");
        retire_one_damage_number(app.world_mut(), first);
        assert!(app.world().get::<ActivePooledDamageNumber>(first).is_none());
        assert_eq!(
            app.world().get::<Text2d>(first).map(|text| text.0.as_str()),
            Some("")
        );
        assert_eq!(
            app.world().get::<TextColor>(first),
            Some(&TextColor(Color::srgba(1.0, 1.0, 1.0, 0.0)))
        );
        assert_eq!(
            app.world().get::<Transform>(first),
            Some(&Transform::IDENTITY)
        );
        assert_eq!(
            app.world().get::<Visibility>(first),
            Some(&Visibility::Hidden)
        );

        let before_last = show_damage_number(
            app.world_mut(),
            Entity::from_bits(2),
            20.0,
            DamageElement::Frost,
        )
        .expect("pending shell cannot block a new number");
        assert_ne!(before_last, first);
        finalize(app.world_mut());
        app.world_mut().clear_trackers();
        let reused = show_damage_number(
            app.world_mut(),
            Entity::from_bits(3),
            33.0,
            DamageElement::Pure,
        )
        .expect("retired shell should return after Last");
        assert_eq!(reused, first);
        assert_eq!(
            app.world()
                .get::<Text2d>(reused)
                .map(|text| text.0.as_str()),
            Some("33")
        );
        assert_eq!(
            app.world().get::<Visibility>(reused),
            Some(&Visibility::Hidden),
            "a fresh number stays hidden until the 3D camera projects it"
        );
        assert!(
            app.world()
                .entity(reused)
                .get_ref::<ActivePooledDamageNumber>()
                .expect("active state is restored")
                .is_added()
        );
        let metrics = app
            .world()
            .resource::<CombatPools>()
            .damage_number_metrics();
        assert_eq!(metrics.created(), 2);
        assert_eq!(metrics.reused(), 1);
        assert_eq!(metrics.active(), 2);
    }

    #[test]
    fn damage_number_lifetime_expires_into_pending_then_last_free_list() {
        let mut app = test_app();
        let entity = show_damage_number(
            app.world_mut(),
            Entity::from_bits(9),
            99.0,
            DamageElement::Arcane,
        )
        .expect("number should show");
        tick_damage_numbers(
            app.world_mut(),
            DAMAGE_NUMBER_LIFETIME_SECONDS - 0.001,
            Quat::IDENTITY,
        );
        assert!(
            app.world()
                .get::<ActivePooledDamageNumber>(entity)
                .is_some()
        );
        tick_damage_numbers(app.world_mut(), 0.001, Quat::IDENTITY);
        assert!(
            app.world()
                .get::<ActivePooledDamageNumber>(entity)
                .is_none()
        );
        assert!(
            app.world()
                .resource::<CombatPools>()
                .damage_numbers
                .shells
                .free
                .is_empty()
        );
        finalize(app.world_mut());
        assert_eq!(
            app.world()
                .resource::<CombatPools>()
                .damage_numbers
                .shells
                .free,
            vec![entity]
        );
    }

    #[test]
    fn shared_summon_pool_rejects_n_plus_one_across_skill_and_spirit_shells() {
        let mut app = test_app();
        let leases = app
            .world_mut()
            .run_system_once(
                |mut commands: Commands, mut pools: ResMut<CombatPools>| -> Vec<Option<Entity>> {
                    let mut leases = Vec::new();
                    for index in 0..18 {
                        let lease = pools.rent_scene_summon(
                            &mut commands,
                            SummonVisual::SpiritHound,
                            Handle::default(),
                            Transform::from_xyz(index as f32, 0.0, 0.0),
                        );
                        if let Some(entity) = lease {
                            commands.entity(entity).insert(SkillSummon);
                        }
                        leases.push(lease);
                    }
                    for index in 0..2 {
                        let lease = pools.rent_mesh_summon(
                            &mut commands,
                            SummonVisual::SpiritOrbit,
                            Handle::default(),
                            Handle::default(),
                            Transform::from_xyz(0.0, 0.0, index as f32),
                        );
                        if let Some(entity) = lease {
                            commands.entity(entity).insert(SpiritOrbit {
                                angle: 0.0,
                                radius: 1.0,
                                remaining: None,
                                shot_timer: 0.0,
                                damage_multiplier: 1.0,
                            });
                        }
                        leases.push(lease);
                    }
                    leases.push(pools.rent_scene_summon(
                        &mut commands,
                        SummonVisual::BoneSentry,
                        Handle::default(),
                        Transform::default(),
                    ));
                    leases
                },
            )
            .expect("mixed summon rent system should run");

        assert!(leases[..PLAYER_SUMMON_HARD_CAP].iter().all(Option::is_some));
        assert!(leases[PLAYER_SUMMON_HARD_CAP].is_none());
        let world = app.world_mut();
        assert_eq!(
            world
                .query_filtered::<Entity, With<PlayerSummon>>()
                .iter(world)
                .count(),
            PLAYER_SUMMON_HARD_CAP
        );
        assert_eq!(
            world
                .query_filtered::<Entity, With<SkillSummon>>()
                .iter(world)
                .count(),
            18
        );
        assert_eq!(
            world
                .query_filtered::<Entity, With<SpiritOrbit>>()
                .iter(world)
                .count(),
            2
        );
        let metrics = world.resource::<CombatPools>().summon_metrics();
        assert_eq!(metrics.active(), PLAYER_SUMMON_HARD_CAP);
        assert_eq!(metrics.peak(), PLAYER_SUMMON_HARD_CAP);
        assert_eq!(metrics.rejected(), 1);
    }

    #[test]
    fn scene_summon_reuses_only_after_last_and_preserves_root_children_and_added_marker() {
        let mut app = test_app();
        let first = rent_one_scene_summon(app.world_mut(), SummonVisual::SpiritHound, 1.0);
        let root = app
            .world()
            .get::<WorldAssetRoot>(first)
            .expect("scene root is installed only on first creation")
            .clone();
        let child = app
            .world_mut()
            .spawn(Name::new("PERSISTENT GLB CHILD"))
            .id();
        app.world_mut().entity_mut(first).add_child(child);
        app.world_mut().entity_mut(first).insert((
            SkillSummon,
            Name::new("DIRTY SUMMON"),
            SpiritOrbit {
                angle: 4.0,
                radius: 9.0,
                remaining: Some(1.0),
                shot_timer: 8.0,
                damage_multiplier: 7.0,
            },
        ));

        retire_one_summon(app.world_mut(), first);
        assert_eq!(
            app.world().get::<Visibility>(first),
            Some(&Visibility::Hidden)
        );
        assert!(app.world().get::<Name>(first).is_none());
        assert!(app.world().get::<SkillSummon>(first).is_none());
        assert!(app.world().get::<SpiritOrbit>(first).is_none());
        assert!(app.world().get::<PlayerSummon>(first).is_none());
        assert!(app.world().get::<WorldAssetRoot>(first).is_some());
        assert!(app.world().entities().contains(child));

        let before_last = rent_one_scene_summon(app.world_mut(), SummonVisual::SpiritHound, 2.0);
        assert_ne!(before_last, first, "pending shell must remain unavailable");
        finalize(app.world_mut());
        app.world_mut().clear_trackers();
        let reused = rent_one_scene_summon(app.world_mut(), SummonVisual::SpiritHound, 3.0);
        assert_eq!(reused, first);
        assert_eq!(app.world().get::<WorldAssetRoot>(reused), Some(&root));
        assert!(
            !app.world()
                .entity(reused)
                .get_ref::<WorldAssetRoot>()
                .expect("reused shell retains its original world root")
                .is_changed(),
            "re-renting must not trigger Bevy's world-instance rebuild path"
        );
        assert!(
            app.world()
                .get::<Children>(reused)
                .is_some_and(|children| children.contains(&child))
        );
        assert!(
            app.world()
                .entity(reused)
                .get_ref::<PlayerSummon>()
                .expect("active lease restores shared marker")
                .is_added()
        );
        assert_eq!(
            app.world().get::<PooledSummon>(reused),
            Some(&PooledSummon {
                visual: SummonVisual::SpiritHound,
            })
        );
    }

    #[test]
    fn all_six_world_asset_visuals_reuse_only_their_own_scene_shell() {
        let mut app = test_app();
        let visuals = [
            SummonVisual::BoneSentry,
            SummonVisual::SpiritHound,
            SummonVisual::TideElemental,
            SummonVisual::BoltTurret,
            SummonVisual::FrostWyrm,
            SummonVisual::AncestorColossus,
        ];
        let first: Vec<Entity> = visuals
            .into_iter()
            .enumerate()
            .map(|(index, visual)| rent_one_scene_summon(app.world_mut(), visual, index as f32))
            .collect();
        for entity in &first {
            retire_one_summon(app.world_mut(), *entity);
        }
        finalize(app.world_mut());

        for (index, visual) in visuals.into_iter().enumerate() {
            let reused = rent_one_scene_summon(app.world_mut(), visual, 10.0 + index as f32);
            assert_eq!(reused, first[index]);
            assert!(app.world().get::<WorldAssetRoot>(reused).is_some());
            assert_eq!(
                app.world().get::<PooledSummon>(reused),
                Some(&PooledSummon { visual })
            );
        }
    }

    #[test]
    fn spirit_orbit_reuses_a_fixed_mesh_shell_without_reinserting_render_handles() {
        let mut app = test_app();
        let first = rent_one_mesh_summon(app.world_mut(), SummonVisual::SpiritOrbit, 1.0);
        retire_one_summon(app.world_mut(), first);
        finalize(app.world_mut());
        app.world_mut().clear_trackers();

        let reused = rent_one_mesh_summon(app.world_mut(), SummonVisual::SpiritOrbit, 2.0);
        assert_eq!(reused, first);
        assert!(
            !app.world()
                .entity(reused)
                .get_ref::<Mesh3d>()
                .expect("mesh shell is retained")
                .is_changed()
        );
        assert!(
            !app.world()
                .entity(reused)
                .get_ref::<MeshMaterial3d<StandardMaterial>>()
                .expect("material shell is retained")
                .is_changed()
        );
    }

    #[test]
    fn retired_entity_is_reused_only_after_last_finalization() {
        let mut app = test_app();
        let first = rent_one_projectile(app.world_mut(), 1.0);
        retire_one_projectile(app.world_mut(), first);

        let before_last = rent_one_projectile(app.world_mut(), 2.0);
        assert_ne!(before_last, first, "pending entities cannot be reused");
        retire_one_projectile(app.world_mut(), before_last);
        finalize(app.world_mut());

        let reused = rent_one_projectile(app.world_mut(), 3.0);
        assert_eq!(reused, before_last);
        let metrics = app.world().resource::<CombatPools>().projectile_metrics();
        assert_eq!(metrics.created(), 2);
        assert_eq!(metrics.reused(), 1);
        assert_eq!(metrics.active(), 1);
        assert_eq!(metrics.peak(), 1);
    }

    #[test]
    fn retirement_clears_projectile_metadata_but_retains_render_shell() {
        let mut app = test_app();
        let entity = rent_one_projectile(app.world_mut(), 4.0);
        app.world_mut().entity_mut(entity).insert((
            ProjectileHitMemory::default(),
            ProjectileFirstFrameTravel(0.25),
            AbilityGenerated,
            DamageProfile::PHYSICAL_ATTACK,
        ));

        retire_one_projectile(app.world_mut(), entity);

        let world = app.world();
        assert!(world.get::<Projectile>(entity).is_none());
        assert!(world.get::<ActivePooledProjectile>(entity).is_none());
        assert!(world.get::<ProjectileHitMemory>(entity).is_none());
        assert!(world.get::<ProjectileFirstFrameTravel>(entity).is_none());
        assert!(world.get::<AbilityGenerated>(entity).is_none());
        assert!(world.get::<DamageProfile>(entity).is_none());
        assert!(world.get::<Mesh3d>(entity).is_some());
        assert!(
            world
                .get::<MeshMaterial3d<StandardMaterial>>(entity)
                .is_some()
        );
        assert!(world.get::<Transform>(entity).is_some());
        assert_eq!(world.get::<Visibility>(entity), Some(&Visibility::Hidden));
        let pools = world.resource::<CombatPools>();
        assert!(pools.projectiles.free.is_empty());
        assert_eq!(pools.projectiles.pending, vec![entity]);
    }

    #[test]
    fn projectile_component_is_added_again_on_reuse() {
        let mut app = test_app();
        let entity = rent_one_projectile(app.world_mut(), 5.0);
        assert!(
            app.world()
                .entity(entity)
                .get_ref::<Projectile>()
                .expect("first lease has a projectile")
                .is_added()
        );
        app.world_mut().clear_trackers();

        retire_one_projectile(app.world_mut(), entity);
        finalize(app.world_mut());
        app.world_mut().clear_trackers();
        let reused = rent_one_projectile(app.world_mut(), 6.0);

        assert_eq!(reused, entity);
        assert!(
            app.world()
                .entity(entity)
                .get_ref::<Projectile>()
                .expect("second lease has a projectile")
                .is_added()
        );
        assert_eq!(
            app.world()
                .get::<Transform>(entity)
                .expect("render transform remains")
                .translation,
            Vec3::new(6.0, 0.25, -6.0)
        );
        assert_eq!(
            app.world().get::<Visibility>(entity),
            Some(&Visibility::Visible)
        );
    }

    #[test]
    fn projectile_first_frame_marker_cannot_leak_across_last_finalized_reuse() {
        let mut app = test_app();
        let entity = rent_one_projectile(app.world_mut(), 7.0);
        app.world_mut()
            .entity_mut(entity)
            .insert(ProjectileFirstFrameTravel(0.37));

        retire_one_projectile(app.world_mut(), entity);
        assert!(
            app.world()
                .get::<ProjectileFirstFrameTravel>(entity)
                .is_none(),
            "retirement clears the transient travel allowance"
        );
        app.update();

        // Simulate stale external metadata on the free shell so the rent-side
        // invariant is exercised independently of retire-side cleanup.
        app.world_mut()
            .entity_mut(entity)
            .insert(ProjectileFirstFrameTravel(0.91));
        let reused = rent_one_projectile(app.world_mut(), 8.0);
        assert_eq!(reused, entity);
        assert!(
            app.world()
                .get::<ProjectileFirstFrameTravel>(reused)
                .is_none(),
            "a new lease must never inherit first-frame travel metadata"
        );
    }

    #[test]
    fn impact_fx_pool_rejects_over_capacity_and_removes_names_on_retire() {
        let mut app = test_app();
        let leases = app
            .world_mut()
            .run_system_once(
                |mut commands: Commands, mut pools: ResMut<CombatPools>| -> Vec<Option<Entity>> {
                    (0..=IMPACT_FX_POOL_CAPACITY)
                        .map(|index| {
                            pools.rent_named_impact_fx(
                                &mut commands,
                                Handle::default(),
                                Handle::default(),
                                Transform::from_xyz(0.0, index as f32, 0.0),
                                impact_fx(index as f32 * 0.01),
                                (index == 0).then(|| Name::new("NAMED IMPACT")),
                            )
                        })
                        .collect()
                },
            )
            .expect("batch rent system should run");

        let first = leases[0].expect("first FX lease succeeds");
        assert!(
            leases[..IMPACT_FX_POOL_CAPACITY]
                .iter()
                .all(Option::is_some)
        );
        assert!(leases[IMPACT_FX_POOL_CAPACITY].is_none());
        app.world_mut()
            .run_system_once(
                move |mut commands: Commands, mut pools: ResMut<CombatPools>| {
                    pools.retire_impact_fx(&mut commands, first);
                },
            )
            .expect("FX retire system should run");

        let world = app.world();
        assert!(world.get::<ImpactFx>(first).is_none());
        assert!(world.get::<Name>(first).is_none());
        assert!(world.get::<Mesh3d>(first).is_some());
        assert_eq!(world.get::<Visibility>(first), Some(&Visibility::Hidden));
        let metrics = world.resource::<CombatPools>().impact_fx_metrics();
        assert_eq!(metrics.created(), IMPACT_FX_POOL_CAPACITY);
        assert_eq!(metrics.rejected(), 1);
        assert_eq!(metrics.active(), IMPACT_FX_POOL_CAPACITY - 1);
        assert_eq!(metrics.peak(), IMPACT_FX_POOL_CAPACITY);
    }

    #[test]
    fn exhausted_fx_pool_does_not_reduce_projectile_capacity() {
        let mut app = test_app();
        let projectile = app
            .world_mut()
            .run_system_once(
                |mut commands: Commands, mut pools: ResMut<CombatPools>| -> Option<Entity> {
                    for index in 0..IMPACT_FX_POOL_CAPACITY {
                        assert!(
                            pools
                                .rent_impact_fx(
                                    &mut commands,
                                    Handle::default(),
                                    Handle::default(),
                                    Transform::from_xyz(0.0, 0.0, index as f32),
                                    impact_fx(0.0),
                                )
                                .is_some()
                        );
                    }
                    assert!(
                        pools
                            .rent_impact_fx(
                                &mut commands,
                                Handle::default(),
                                Handle::default(),
                                Transform::default(),
                                impact_fx(0.0),
                            )
                            .is_none()
                    );
                    pools.rent_projectile(
                        &mut commands,
                        Handle::default(),
                        Handle::default(),
                        Transform::default(),
                        projectile(1.0),
                    )
                },
            )
            .expect("independent pool test system should run");

        assert!(projectile.is_some());
        let pools = app.world().resource::<CombatPools>();
        assert_eq!(pools.impact_fx_metrics().rejected(), 1);
        assert_eq!(pools.projectile_metrics().created(), 1);
        assert_eq!(pools.projectile_metrics().rejected(), 0);
    }

    #[test]
    fn bulk_retirement_uses_the_same_pending_path() {
        let mut app = test_app();
        let first = rent_one_projectile(app.world_mut(), 7.0);
        let second = rent_one_projectile(app.world_mut(), 8.0);
        app.world_mut()
            .run_system_once(|mut commands: Commands, mut pools: ResMut<CombatPools>| {
                pools.retire_all_projectiles(&mut commands);
            })
            .expect("bulk retire system should run");

        let pools = app.world().resource::<CombatPools>();
        assert_eq!(pools.projectile_metrics().active(), 0);
        assert!(pools.projectiles.free.is_empty());
        let mut expected = vec![first, second];
        expected.sort();
        assert_eq!(pools.projectiles.pending, expected);
        finalize(app.world_mut());
        assert_eq!(
            app.world().resource::<CombatPools>().projectiles.free.len(),
            2
        );
    }

    #[test]
    fn combat_sources_construct_pooled_components_only_inside_rent_calls() {
        const SOURCES: [(&str, &str); 8] = [
            ("combat.rs", include_str!("combat.rs")),
            ("skills.rs", include_str!("skills.rs")),
            ("skills/summons.rs", include_str!("skills/summons.rs")),
            ("bonds.rs", include_str!("bonds.rs")),
            ("hero_abilities.rs", include_str!("hero_abilities.rs")),
            ("hero_fusion.rs", include_str!("hero_fusion.rs")),
            ("bosses.rs", include_str!("bosses.rs")),
            ("archive.rs", include_str!("archive.rs")),
        ];

        fn assert_rented(
            file: &str,
            source: &str,
            component: &str,
            accepted_rent_calls: &[&str],
        ) -> usize {
            let mut count = 0;
            for (offset, _) in source.match_indices(component) {
                count += 1;
                let prefix = &source[..offset];
                let nearest_rent = accepted_rent_calls
                    .iter()
                    .filter_map(|call| prefix.rfind(call))
                    .max();
                let nearest_spawn = prefix.rfind(".spawn(");
                let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
                assert!(
                    nearest_rent.is_some_and(|rent| nearest_spawn.is_none_or(|spawn| rent > spawn)),
                    "{file}:{line} constructs {component} outside an accepted pool rent call"
                );
            }
            count
        }

        fn assert_enemy_rented(file: &str, source: &str) -> usize {
            let mut count = 0;
            for (offset, _) in source.match_indices("Enemy {") {
                let suffix = &source[offset..source.len().min(offset + 96)];
                if !suffix.contains("hp:") {
                    continue;
                }
                count += 1;
                let prefix = &source[..offset];
                let nearest_rent = [".rent_enemy_scene(", ".rent_enemy_scene_group("]
                    .into_iter()
                    .filter_map(|call| prefix.rfind(call))
                    .max();
                let nearest_spawn = prefix.rfind(".spawn(");
                let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
                assert!(
                    nearest_rent.is_some_and(|rent| nearest_spawn.is_none_or(|spawn| rent > spawn)),
                    "{file}:{line} constructs Enemy outside an accepted pool rent call"
                );
            }
            count
        }

        let mut projectiles = 0;
        let mut effects = 0;
        for (file, full_source) in SOURCES {
            let source = full_source
                .split("#[cfg(test)]")
                .next()
                .expect("production source precedes tests");
            projectiles += assert_rented(file, source, "Projectile {", &[".rent_projectile("]);
            effects += assert_rented(
                file,
                source,
                "ImpactFx {",
                &[".rent_impact_fx(", ".rent_named_impact_fx("],
            );
        }
        assert!(
            projectiles > 0,
            "audit must inspect projectile constructors"
        );
        assert!(effects > 0, "audit must inspect FX constructors");

        const ENEMY_SOURCES: [(&str, &str); 4] = [
            ("combat.rs", include_str!("combat.rs")),
            ("bosses.rs", include_str!("bosses.rs")),
            ("trial.rs", include_str!("trial.rs")),
            ("archive.rs", include_str!("archive.rs")),
        ];
        let mut enemy_builders = 0;
        for (file, full_source) in ENEMY_SOURCES {
            let source = full_source
                .split("#[cfg(test)]")
                .next()
                .expect("production source precedes tests");
            assert!(
                !source.contains("WorldAssetRoot("),
                "{file} must acquire enemy WorldAssetRoot shells through CombatPools"
            );
            assert!(
                !source.contains(".despawn()"),
                "{file} must retire enemy roots instead of despawning them"
            );
            enemy_builders += assert_enemy_rented(file, source);
        }
        assert!(
            enemy_builders >= 6,
            "audit must cover wave, boss, splitling, boss-add, trial, and archive builders"
        );

        let skills = include_str!("skills.rs");
        let hero_abilities = include_str!("hero_abilities.rs");
        assert!(
            !skills.contains("WorldAssetRoot(assets.summon_scenes"),
            "skill summons must not construct disposable world roots"
        );
        assert!(
            skills.contains("summons: Query<(), With<PlayerSummon>>"),
            "skill casting must count the shared player-summon marker"
        );
        assert!(
            hero_abilities.contains("player_summons: Query<(), With<PlayerSummon>>"),
            "SpiritKeeper spawning must count the shared player-summon marker"
        );
        let skill_summons = assert_rented(
            "skills.rs",
            skills,
            "SkillSummon,",
            &[".rent_scene_summon(", ".rent_mesh_summon("],
        );
        let spirit_summons = assert_rented(
            "hero_abilities.rs",
            hero_abilities,
            ".insert(SpiritOrbit {",
            &[".rent_mesh_summon("],
        );
        assert!(
            skill_summons >= 8,
            "audit must cover every skill summon path"
        );
        assert_eq!(spirit_summons, 1, "SpiritOrbit has one pooled constructor");

        fn function_body<'a>(source: &'a str, name: &str) -> &'a str {
            let marker = format!("fn {name}(");
            let start = source.find(&marker).expect("audited function exists");
            let rest = &source[start + marker.len()..];
            let end = ["\nfn ", "\npub(super) fn ", "\n#[cfg(test)]"]
                .into_iter()
                .filter_map(|marker| rest.find(marker))
                .min()
                .unwrap_or(rest.len());
            &rest[..end]
        }

        for (source, function) in [
            (include_str!("skills/summons.rs"), "update_skill_summons"),
            (include_str!("skills/summons.rs"), "update_rare_summons"),
            (include_str!("skills/summons.rs"), "update_epic_summons"),
            (skills, "update_oath_banners"),
        ] {
            assert!(
                !function_body(source, function).contains(".despawn()"),
                "{function} must retire player summons through CombatPools"
            );
        }
        for function in ["cleanup_spirit_summons", "update_spirit_companions"] {
            assert!(
                !function_body(hero_abilities, function).contains(".despawn()"),
                "{function} must retire SpiritOrbit through CombatPools"
            );
        }

        let combat_production = include_str!("combat.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("production combat precedes tests");
        assert_eq!(
            combat_production.matches(".show_damage_number(").count(),
            1,
            "the authoritative damage resolver is the only number producer"
        );
        let number_call = combat_production
            .find(".show_damage_number(")
            .expect("damage-number producer exists");
        let hp_guard = combat_production[..number_call]
            .rfind("if resolution.hp_damage > 0.0 {")
            .expect("number production is guarded by actual HP loss");
        assert!(
            number_call - hp_guard < 320,
            "zero or shield-only damage must not create a number"
        );

        let pooling_production = include_str!("pooling.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("production pooling precedes tests");
        let projectile_rent = function_body(pooling_production, "rent_projectile");
        assert!(projectile_rent.contains("projectile_overflow.rent(commands)"));
        assert!(projectile_rent.contains("LogicOnlyPooledProjectile"));
        assert!(projectile_rent.contains("remove::<(Mesh3d, MeshMaterial3d<StandardMaterial>)>"));
    }
    #[test]
    fn mixed_adventure_group_preflights_all_keys_without_partial_rental() {
        let mut app = test_app();
        // Only three new shells remain, while the four-actor camp needs four.
        app.world_mut()
            .resource_mut::<CombatPools>()
            .enemies
            .created = ENEMY_POOL_CREATED_CAP - 3;
        let result = app
            .world_mut()
            .run_system_once(|mut commands: Commands, mut pools: ResMut<CombatPools>| {
                let actors = [
                    (
                        EnemySceneKey::Boss(0),
                        Handle::default(),
                        Transform::default(),
                    ),
                    (
                        EnemySceneKey::BossSummon(9),
                        Handle::default(),
                        Transform::default(),
                    ),
                    (
                        EnemySceneKey::BossSummon(9),
                        Handle::default(),
                        Transform::default(),
                    ),
                    (
                        EnemySceneKey::BossSummon(9),
                        Handle::default(),
                        Transform::default(),
                    ),
                ];
                pools.rent_enemy_scene_batch(&mut commands, &actors)
            })
            .unwrap();
        assert!(result.is_none());
        let pools = app.world().resource::<CombatPools>();
        assert!(pools.enemies.active.is_empty());
        assert_eq!(pools.enemies.created, ENEMY_POOL_CREATED_CAP - 3);
        app.world_mut()
            .resource_mut::<CombatPools>()
            .enemies
            .created = ENEMY_POOL_CREATED_CAP - 4;
        let result = app
            .world_mut()
            .run_system_once(|mut commands: Commands, mut pools: ResMut<CombatPools>| {
                let actors = [
                    (
                        EnemySceneKey::Boss(0),
                        Handle::default(),
                        Transform::default(),
                    ),
                    (
                        EnemySceneKey::BossSummon(9),
                        Handle::default(),
                        Transform::default(),
                    ),
                    (
                        EnemySceneKey::BossSummon(9),
                        Handle::default(),
                        Transform::default(),
                    ),
                    (
                        EnemySceneKey::BossSummon(9),
                        Handle::default(),
                        Transform::default(),
                    ),
                ];
                pools.rent_enemy_scene_batch(&mut commands, &actors)
            })
            .unwrap()
            .unwrap();
        assert_eq!(result.len(), 4);
        assert_eq!(
            app.world().resource::<CombatPools>().enemies.active.len(),
            4
        );
    }
}
