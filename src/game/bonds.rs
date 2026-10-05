use super::bond_catalog::{BOND_FAMILIES, BondCardId, BondEffect};
use super::content::{BossModifiers, SKILLS, Tag};
use super::{GameSession, HeroVitals, PlayerBuild};
use bevy::prelude::*;

pub(crate) struct BondsPlugin;
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct BondSet;
impl Plugin for BondsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BondRuntime>().add_systems(
            Update,
            tick_cards
                .in_set(BondSet)
                .before(super::hero::synchronize_hero_growth),
        );
    }
}
/// Element used by the final damage resolver. `Pure` bypasses armor and boss
/// resistances; elemental secondary effects remain owned by skills and equipment.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum DamageElement {
    Physical,
    Storm,
    Flame,
    Frost,
    Tide,
    Arcane,
    Pure,
}

impl DamageElement {
    pub(crate) const fn is_elemental(self) -> bool {
        matches!(
            self,
            Self::Storm | Self::Flame | Self::Frost | Self::Tide | Self::Arcane
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum DamageClass {
    Attack,
    Spell,
    Summon,
    Reaction,
}

/// Metadata carried by every hero-owned projectile. It deliberately contains
/// no damage number, so combat remains the single source of truth for damage,
/// death rewards, and the DPS meter.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DamageProfile {
    pub(crate) element: DamageElement,
    pub(crate) class: DamageClass,
    pub(crate) applies_burn: bool,
    pub(crate) burn_tick: bool,
    pub(crate) skill_index: Option<u8>,
    /// Secondary equipment projectiles still use the authoritative damage
    /// resolver, but cannot recursively trigger another equipment proc.
    pub(crate) market_proc: bool,
    pub(crate) card_proc: bool,
    pub(crate) melee: bool,
    pub(crate) arrow: bool,
}

impl DamageProfile {
    pub(crate) const PHYSICAL_ATTACK: Self =
        Self::new(DamageElement::Physical, DamageClass::Attack);

    pub(crate) const fn new(element: DamageElement, class: DamageClass) -> Self {
        Self {
            element,
            class,
            applies_burn: false,
            burn_tick: false,
            skill_index: None,
            market_proc: false,
            card_proc: false,
            melee: false,
            arrow: false,
        }
    }

    pub(crate) const fn as_card_proc(mut self) -> Self {
        self.card_proc = true;
        self
    }
    pub(crate) const fn as_melee(mut self) -> Self {
        self.melee = true;
        self
    }
    pub(crate) const fn as_arrow(mut self) -> Self {
        self.arrow = true;
        self
    }
    pub(crate) const fn with_burn(mut self) -> Self {
        self.applies_burn = true;
        self
    }

    pub(crate) const fn as_burn_tick(mut self) -> Self {
        self.burn_tick = true;
        self
    }

    pub(crate) const fn as_market_proc(mut self) -> Self {
        self.market_proc = true;
        self
    }

    pub(crate) fn for_skill(index: usize) -> Self {
        let tags = SKILLS[index].tags;
        let element = if tags.contains(&Tag::Storm) {
            DamageElement::Storm
        } else if tags.contains(&Tag::Frost) {
            DamageElement::Frost
        } else if tags.contains(&Tag::Flame) {
            DamageElement::Flame
        } else if tags.contains(&Tag::Tide) {
            DamageElement::Tide
        } else if tags.contains(&Tag::Arcane) {
            DamageElement::Arcane
        } else {
            DamageElement::Physical
        };
        let class = if tags.contains(&Tag::Summon) {
            DamageClass::Summon
        } else if tags.contains(&Tag::Spell) {
            DamageClass::Spell
        } else {
            DamageClass::Attack
        };
        let mut profile = Self::new(element, class);
        profile.skill_index = Some(index as u8);
        profile
    }
}

/// Source values use Warcraft range units; 100 units map to one arena metre.
const REFERENCE_RANGE_SCALE: f32 = 0.01;
#[derive(Default, Debug, Clone, Copy)]
struct CardModifiers {
    suppression_chance: f32,
    suppression_multiplier: f32,
    extra_attack: f32,
    armor: f32,
    movement: f32,
    regeneration: f32,
    regeneration_amp: f32,
    life_leech: f32,
    crit_chance: f32,
    crit_damage: f32,
    crit_damage_per300: f32,
    summon_damage: f32,
    summon_crit_inheritance: f32,
    element_damage: [f32; 5],
    elemental_damage: f32,
    elemental_more: f32,
    outgoing_more: f32,
    incoming_more: f32,
    distance_damage: f32,
    normal_damage: f32,
    trial_damage: f32,
    boss_damage: f32,
    model_scale: f32,
    snapshot_attributes: f32,
    strength_amp: f32,
    intelligence_amp: f32,
    strength_kill: f32,
    strength_second: f32,
    intelligence_kill: f32,
    intelligence_second: f32,
    quiver_capacity: u16,
    arrow_damage: f32,
    recharge_reduction: f32,
    agility_arrow: f32,
    agility_amp_arrow: f32,
    melee_cleave: u8,
    heal_kill: f32,
    melee_damage: f32,
    spell_echo: f32,
    spell_chain: u8,

    strength: f32,
    agility: f32,
    intelligence: f32,
    attack: f32,
    health: f32,
    agility_amp: f32,
    health_amp: f32,
    attack_amp: f32,
    damage_amp: f32,
    skill_damage: f32,
    attack_speed: f32,
    range: f32,
    evasion: f32,
    arrows: u8,
    pierce: u8,
    ricochet: u8,
    strength_level: f32,
    agility_level: f32,
    intelligence_level: f32,
    attack_level: f32,
    health_level: f32,
    health_amp_level: f32,
    skill_damage_level: f32,
    attack_amp_level: f32,
    damage_amp_level: f32,
    agility_kill: f32,
    agility_second: f32,
    revives: u8,
    immunity_seconds: f32,
    immunity_cooldown: f32,
}
impl CardModifiers {
    fn add(&mut self, effect: BondEffect) {
        use BondEffect::*;
        match effect {
            SuppressionOnHit {
                chance,
                attack_multiplier,
            } => {
                self.suppression_chance += chance;
                self.suppression_multiplier = self.suppression_multiplier.max(attack_multiplier);
            }
            ExtraAttackChance(x) => self.extra_attack += x,
            Armor(x) => self.armor += x,
            MoveSpeed(x) => self.movement += x,
            HealthRegeneration(x) => self.regeneration += x,
            RegenerationAmplification(x) => self.regeneration_amp += x,
            LifeLeech(x) => self.life_leech += x,
            CriticalChance(x) => self.crit_chance += x,
            CriticalDamage(x) => self.crit_damage += x,
            CriticalDamagePer300Kills(x) => self.crit_damage_per300 += x,
            SummonDamage(x) => self.summon_damage += x,
            SummonCritInheritance(x) => self.summon_crit_inheritance += x,
            ElementDamage(tag, x) => {
                if let Some(index) = element_tag_index(tag) {
                    self.element_damage[index] += x;
                }
            }
            ElementalDamage(x) => self.elemental_damage += x,
            ElementalMore(x) => self.elemental_more = (1. + self.elemental_more) * (1. + x) - 1.,
            OutgoingMore(x) => self.outgoing_more = (1. + self.outgoing_more) * (1. + x) - 1.,
            IncomingMore(x) => self.incoming_more = (1. + self.incoming_more) * (1. + x) - 1.,
            DistanceDamage(x) => self.distance_damage += x,
            NormalEnemyDamage(x) => self.normal_damage = (1. + self.normal_damage) * (1. + x) - 1.,
            TrialDamage(x) => self.trial_damage = (1. + self.trial_damage) * (1. + x) - 1.,
            BossDamage(x) => self.boss_damage = (1. + self.boss_damage) * (1. + x) - 1.,
            ModelScale(x) => self.model_scale += x,
            SnapshotAttributes(x) => self.snapshot_attributes += x,
            SeriesDamagePerCard { .. }
            | SkillDamagePerLearnedSkill(_)
            | IntelligenceAmplificationPerLearnedSkill(_) => {
                unreachable!("dynamic effects require the owning build")
            }
            StrengthAmplification(x) => self.strength_amp += x,
            IntelligenceAmplification(x) => self.intelligence_amp += x,
            StrengthPerKill(x) => self.strength_kill += x,
            StrengthPerSecond(x) => self.strength_second += x,
            IntelligencePerKill(x) => self.intelligence_kill += x,
            IntelligencePerSecond(x) => self.intelligence_second += x,
            QuiverCapacity(x) => self.quiver_capacity += x,
            ArrowDamage(x) => self.arrow_damage += x,
            ArrowRechargeReduction(x) => self.recharge_reduction += x,
            AgilityPerArrow(x) => self.agility_arrow += x,
            AgilityAmplificationPerArrow(x) => self.agility_amp_arrow += x,
            MeleeCleave(x) => self.melee_cleave += x,
            HealOnKill(x) => self.heal_kill += x,
            MeleeDamage(x) => self.melee_damage += x,
            SpellEchoChance(x) => self.spell_echo += x,
            SpellChainTargets(x) => self.spell_chain += x,
            Strength(x) => self.strength += x,
            Agility(x) => self.agility += x,
            Intelligence(x) => self.intelligence += x,
            Attack(x) => self.attack += x,
            MaxHealth(x) => self.health += x,
            AgilityAmplification(x) => self.agility_amp += x,
            HealthAmplification(x) => self.health_amp += x,
            AttackAmplification(x) => self.attack_amp += x,
            DamageAmplification(x) => self.damage_amp += x,
            SkillDamage(x) => self.skill_damage += x,
            AttackSpeed(x) => self.attack_speed += x,
            AttackRange(x) => self.range += x * REFERENCE_RANGE_SCALE,
            Evasion(x) => self.evasion += x * 0.01,
            ArrowCount(x) => self.arrows += x,
            ArrowPiercing(x) => self.pierce += x,
            ArrowRicochet(x) => self.ricochet += x,
            StrengthPerLevel(x) => self.strength_level += x,
            AgilityPerLevel(x) => self.agility_level += x,
            IntelligencePerLevel(x) => self.intelligence_level += x,
            AttackPerLevel(x) => self.attack_level += x,
            HealthPerLevel(x) => self.health_level += x,
            HealthAmplificationPerLevel(x) => self.health_amp_level += x,
            SkillDamagePerLevel(x) => self.skill_damage_level += x,
            AttackAmplificationPerLevel(x) => self.attack_amp_level += x,
            DamageAmplificationPerLevel(x) => self.damage_amp_level += x,
            AgilityPerKill(x) => self.agility_kill += x,
            AgilityPerSecond(x) => self.agility_second += x,
            Revives(x) => self.revives += x,
            LethalImmunity { seconds, cooldown } => {
                self.immunity_seconds = seconds;
                self.immunity_cooldown = cooldown;
            }
            Wood(_) | Gold(_) | JobFruit(_) => {} // Settled once by card acquisition.
        }
    }
    fn add_for_build(&mut self, effect: BondEffect, build: &PlayerBuild) {
        // The reference explicitly counts the supporting arrow skill. In this
        // adaptation that volley is unlocked by the first Archery card.
        let main_skills = build.skills.iter().filter(|rank| **rank > 0).count()
            + usize::from(build.bond(super::BondId::Archery) > 0);
        match effect {
            BondEffect::SeriesDamagePerCard { family, amount } => {
                self.damage_amp += f32::from(build.bond(family)) * amount
            }
            BondEffect::SkillDamagePerLearnedSkill(amount) => {
                self.skill_damage += main_skills as f32 * amount
            }
            BondEffect::IntelligenceAmplificationPerLearnedSkill(amount) => {
                self.intelligence_amp += main_skills as f32 * amount
            }
            other => self.add(other),
        }
    }
    fn for_build(build: &PlayerBuild) -> Self {
        let mut result = Self::default();
        for card in BondCardId::ALL {
            if build.owns_bond_card(card) {
                for effect in card.definition().effects {
                    result.add_for_build(*effect, build);
                }
            }
        }
        for family in &BOND_FAMILIES {
            let count = family
                .members
                .iter()
                .filter(|c| build.owns_bond_card(**c))
                .count();
            for bonus in family.bonuses {
                if count >= usize::from(bonus.required) {
                    for effect in bonus.effects {
                        result.add_for_build(*effect, build);
                    }
                }
            }
        }
        result
    }
}
fn element_tag_index(tag: Tag) -> Option<usize> {
    match tag {
        Tag::Storm => Some(0),
        Tag::Flame => Some(1),
        Tag::Frost => Some(2),
        Tag::Tide => Some(3),
        Tag::Arcane => Some(4),
        _ => None,
    }
}
#[derive(Default, Debug, Clone, Copy)]
struct AppliedStats {
    armor: f32,
    strength: f32,
    agility: f32,
    intelligence: f32,
    health: f32,
    energy: f32,
    attack: f32,
    speed: f32,
    movement: f32,
    projectile: f32,
    range: f32,
    arrows: u8,
    pierce: u8,
    ricochet: u8,
}
#[derive(Resource, Default)]
pub(crate) struct BondRuntime {
    modifiers: CardModifiers,
    permanent: CardModifiers,
    applied: AppliedStats,
    last_level: Option<u32>,
    last_kills: u32,
    immunity_remaining: f32,
    revives_used: u8,
    archery_active: bool,
    pub(crate) arrows_remaining: u16,
    pub(crate) recharge_remaining: f32,
    spell_proc_remaining: f32,
    pending_heal: f32,
    pending_hits: u32,
    crit_kill_remainder: u32,
    snapshot_attributes_taken: f32,
    equipment_evasion: f32,
    equipment_critical_chance: f32,
}
impl BondRuntime {
    fn advance(&mut self, build: &PlayerBuild, level: u32, kills: u32, seconds: f32) {
        // Attribute growth begins after acquisition, never retroactively.
        let levels = self.last_level.map_or(0, |last| level.saturating_sub(last)) as f32;
        let kills_gained = kills.saturating_sub(self.last_kills) as f32;
        let previous = self.modifiers;
        if previous.crit_damage_per300 > 0. {
            self.crit_kill_remainder = self
                .crit_kill_remainder
                .saturating_add(kills.saturating_sub(self.last_kills));
            self.permanent.crit_damage +=
                (self.crit_kill_remainder / 300) as f32 * previous.crit_damage_per300;
            self.crit_kill_remainder %= 300;
        }

        self.permanent.strength +=
            previous.strength_level * levels + previous.strength_kill * kills_gained;
        self.permanent.agility +=
            previous.agility_level * levels + previous.agility_kill * kills_gained;
        self.permanent.intelligence +=
            previous.intelligence_level * levels + previous.intelligence_kill * kills_gained;
        self.pending_heal += previous.heal_kill * kills_gained;
        self.permanent.attack += previous.attack_level * levels;
        self.permanent.health += previous.health_level * levels;
        self.permanent.health_amp += previous.health_amp_level * levels;
        self.permanent.skill_damage += previous.skill_damage_level * levels;
        self.permanent.attack_amp += previous.attack_amp_level * levels;
        self.permanent.damage_amp += previous.damage_amp_level * levels;
        self.modifiers = CardModifiers::for_build(build);
        self.permanent.agility += self.modifiers.agility_second * seconds;
        self.permanent.strength += self.modifiers.strength_second * seconds;
        self.permanent.intelligence += self.modifiers.intelligence_second * seconds;
        self.spell_proc_remaining = (self.spell_proc_remaining - seconds).max(0.);
        self.recharge_remaining = (self.recharge_remaining
            - (self.modifiers.recharge_reduction - previous.recharge_reduction))
            .max(0.);
        let archery = [
            BondCardId::ArcheryPreparation,
            BondCardId::ArcheryPiercing,
            BondCardId::ArcheryVolley,
            BondCardId::ArcheryHaste,
        ]
        .into_iter()
        .any(|c| build.owns_bond_card(c));
        let capacity = 30 + self.modifiers.quiver_capacity;
        if archery && !self.archery_active {
            self.arrows_remaining = capacity;
            self.recharge_remaining = 0.;
        }
        self.archery_active = archery;
        if archery {
            let old_capacity = 30 + previous.quiver_capacity;
            self.arrows_remaining =
                (self.arrows_remaining + capacity.saturating_sub(old_capacity)).min(capacity);
            if self.arrows_remaining == 0 {
                self.recharge_remaining = (self.recharge_remaining - seconds).max(0.);
                if self.recharge_remaining <= 0. {
                    self.arrows_remaining = capacity;
                }
            }
        } else {
            self.arrows_remaining = 0;
            self.recharge_remaining = 0.;
        }

        self.last_level = Some(level);
        self.last_kills = kills;
        self.immunity_remaining = (self.immunity_remaining - seconds).max(0.0);
    }
    pub(crate) fn remove_applied(&mut self, build: &mut PlayerBuild, vitals: &mut HeroVitals) {
        let a = std::mem::take(&mut self.applied);
        vitals.armor -= a.armor;
        vitals.strength -= a.strength;
        vitals.agility -= a.agility;
        vitals.intelligence -= a.intelligence;
        vitals.max_health -= a.health;
        vitals.health -= a.health;
        vitals.max_energy -= a.energy;
        vitals.energy -= a.energy;
        build.attack_damage -= a.attack;
        build.attacks_per_second -= a.speed;
        build.move_speed -= a.movement;
        build.projectile_speed -= a.projectile;
        build.attack_range -= a.range;
        build.multishot = build.multishot.saturating_sub(a.arrows);
        build.pierce = build.pierce.saturating_sub(a.pierce);
        build.ricochet = build.ricochet.saturating_sub(a.ricochet);
    }
    pub(crate) fn apply_stats(&mut self, build: &mut PlayerBuild, vitals: &mut HeroVitals) {
        let m = self.modifiers;
        let snapshot_fraction = (m.snapshot_attributes - self.snapshot_attributes_taken).max(0.);
        self.permanent.strength += vitals.strength * snapshot_fraction;
        self.permanent.agility += vitals.agility * snapshot_fraction;
        self.permanent.intelligence += vitals.intelligence * snapshot_fraction;
        self.snapshot_attributes_taken = self.snapshot_attributes_taken.max(m.snapshot_attributes);
        let p = self.permanent;
        let primary_before = vitals.primary_attribute();
        // remove_applied already removed our previous arrow bonus. Count it once.
        // Arrow count means volley size here, never the separate quiver reserve.
        let arrow_count = build.multishot.max(1).saturating_add(m.arrows) as f32;
        let strength =
            (vitals.strength + m.strength + p.strength) * (1. + m.strength_amp) - vitals.strength;
        let agility = (vitals.agility + m.agility + p.agility + arrow_count * m.agility_arrow)
            * (1.0 + m.agility_amp + arrow_count * m.agility_amp_arrow)
            - vitals.agility;
        let intelligence = (vitals.intelligence + m.intelligence + p.intelligence)
            * (1. + m.intelligence_amp)
            - vitals.intelligence;
        vitals.strength += strength;
        vitals.agility += agility;
        vitals.intelligence += intelligence;
        let health = (vitals.max_health
            + strength * 14.0 * vitals.command_health_multiplier
            + m.health
            + p.health)
            * (1.0 + m.health_amp + p.health_amp)
            - vitals.max_health;
        let energy = intelligence * 8.0;
        let attack = vitals.primary_attribute() - primary_before + m.attack + p.attack;
        let speed = (build.attacks_per_second + agility * 0.028) * (1.0 + m.attack_speed)
            - build.attacks_per_second;
        self.applied = AppliedStats {
            armor: m.armor,
            strength,
            agility,
            intelligence,
            health,
            energy,
            attack,
            speed,
            movement: agility * 0.035 + m.movement,
            projectile: intelligence * 0.08,
            range: intelligence * 0.025 + m.range,
            arrows: m.arrows,
            pierce: m.pierce,
            ricochet: m.ricochet,
        };
        let a = self.applied;
        vitals.armor += m.armor;
        vitals.max_health += health;
        vitals.max_energy += energy;
        if vitals.alive {
            vitals.health = (vitals.health + health).clamp(0.0, vitals.max_health);
            vitals.energy = (vitals.energy + energy).clamp(0.0, vitals.max_energy);
        } else {
            vitals.health = 0.0;
        }
        build.attack_damage += attack;
        build.attacks_per_second += speed;
        build.move_speed += a.movement;
        build.projectile_speed += a.projectile;
        build.attack_range += a.range;
        build.multishot = build.multishot.saturating_add(a.arrows);
        build.pierce = build.pierce.saturating_add(a.pierce);
        build.ricochet = build.ricochet.saturating_add(a.ricochet);
    }
    pub(crate) fn damage_multiplier(&self, profile: DamageProfile) -> f32 {
        let mut result = 1.0 + self.modifiers.damage_amp + self.permanent.damage_amp;
        result *= 1. + self.modifiers.outgoing_more;
        if profile.class == DamageClass::Summon {
            result *= 1. + self.modifiers.summon_damage;
        }
        if profile.element.is_elemental() {
            result *= (1. + self.modifiers.elemental_damage) * (1. + self.modifiers.elemental_more);
            let index = match profile.element {
                DamageElement::Storm => Some(0),
                DamageElement::Flame => Some(1),
                DamageElement::Frost => Some(2),
                DamageElement::Tide => Some(3),
                DamageElement::Arcane => Some(4),
                _ => None,
            };
            if let Some(index) = index {
                result *= 1. + self.modifiers.element_damage[index];
            }
        }
        if profile.class == DamageClass::Attack {
            result *= 1.0 + self.modifiers.attack_amp + self.permanent.attack_amp;
        }
        if profile.skill_index.is_some() || profile.class == DamageClass::Spell {
            result *= 1.0 + self.modifiers.skill_damage + self.permanent.skill_damage;
        }
        if profile.melee {
            result *= 1. + self.modifiers.melee_damage;
        }
        if profile.arrow {
            result *= 1. + self.modifiers.arrow_damage;
        }
        result
    }
    pub(crate) fn suppression_chance(&self) -> f32 {
        self.modifiers.suppression_chance.clamp(0., 0.95)
    }
    pub(crate) fn suppression_multiplier(&self) -> f32 {
        self.modifiers.suppression_multiplier
    }
    pub(crate) fn extra_attack_chance(&self) -> f32 {
        self.modifiers.extra_attack.clamp(0., 0.95)
    }
    pub(crate) fn take_received_hits(&mut self) -> u32 {
        std::mem::take(&mut self.pending_hits)
    }
    pub(crate) fn critical_chance(&self) -> f32 {
        (BASE_CRIT_CHANCE + self.modifiers.crit_chance + self.equipment_critical_chance)
            .clamp(0., 1.)
    }
    pub(crate) fn critical_damage(&self) -> f32 {
        BASE_CRIT_MULTIPLIER + self.modifiers.crit_damage + self.permanent.crit_damage
    }
    pub(crate) fn summon_crit_inheritance(&self) -> f32 {
        self.modifiers.summon_crit_inheritance
    }
    pub(crate) fn life_leech(&self) -> f32 {
        self.modifiers.life_leech
    }
    pub(crate) fn model_scale(&self) -> f32 {
        1. + self.modifiers.model_scale
    }
    pub(crate) fn target_damage_multiplier(&self, boss: bool, trial: bool) -> f32 {
        if boss {
            1. + self.modifiers.boss_damage
        } else if trial {
            1. + self.modifiers.trial_damage
        } else {
            1. + self.modifiers.normal_damage
        }
    }
    pub(crate) fn distance_damage_multiplier(&self, distance: f32) -> f32 {
        1. + self.modifiers.distance_damage * (distance / 12.).clamp(0., 1.)
    }
    pub(crate) fn quiver_capacity(&self) -> u16 {
        30 + self.modifiers.quiver_capacity
    }
    pub(crate) fn archery_active(&self) -> bool {
        self.archery_active
    }
    pub(crate) fn consume_arrow(&mut self) {
        if self.archery_active && self.arrows_remaining > 0 {
            self.arrows_remaining -= 1;
            if self.arrows_remaining == 0 {
                self.recharge_remaining = (12. - self.modifiers.recharge_reduction).max(1.);
            }
        }
    }
    pub(crate) fn melee_cleave(&self) -> usize {
        self.modifiers.melee_cleave as usize
    }
    pub(crate) fn take_kill_healing(&mut self) -> f32 {
        std::mem::take(&mut self.pending_heal)
    }
    /// One shared cooldown and an explicit secondary flag prevent hit-chain recursion.
    pub(crate) fn spell_procs(&mut self, profile: DamageProfile, roll: f32) -> (bool, u8) {
        if profile.class != DamageClass::Spell
            || profile.card_proc
            || profile.market_proc
            || self.spell_proc_remaining > 0.
        {
            return (false, 0);
        }
        let echo = roll < self.modifiers.spell_echo;
        let chains = self.modifiers.spell_chain;
        if echo || chains > 0 {
            self.spell_proc_remaining = 1.;
        }
        (echo, chains)
    }
    pub(crate) fn evasion_chance(&self) -> f32 {
        (self.modifiers.evasion + self.equipment_evasion).clamp(0.0, 0.95)
    }
    pub(crate) fn evade(&self, roll: f32) -> bool {
        roll < self.evasion_chance()
    }
    pub(crate) fn take_physical_damage(&mut self, vitals: &mut HeroVitals, damage: f32) {
        let before = vitals.health;
        vitals.take_physical_damage(damage * (1. + self.modifiers.incoming_more));
        if vitals.alive
            && vitals.health <= 0.0
            && self.modifiers.immunity_seconds > 0.0
            && self.immunity_remaining <= 0.0
        {
            self.intercept_fatal(vitals);
            // Judge's Brush negates the fatal hit rather than healing a fixed amount.
            vitals.health = before.max(1.0);
        }
        if before > vitals.health {
            self.pending_hits = self.pending_hits.saturating_add(1).min(64);
        }
    }
    pub(crate) fn intercept_fatal(&mut self, vitals: &mut HeroVitals) -> bool {
        if !vitals.alive || vitals.health > 0.0 {
            return false;
        }
        if self.modifiers.immunity_seconds > 0.0 && self.immunity_remaining <= 0.0 {
            vitals.health = 1.0;
            vitals.invulnerable_remaining = vitals
                .invulnerable_remaining
                .max(self.modifiers.immunity_seconds);
            self.immunity_remaining = self.modifiers.immunity_cooldown;
            return true;
        }
        if self.revives_used < self.modifiers.revives {
            self.revives_used += 1;
            // Full health is this arena's revival adaptation; original refill amount is unverified.
            vitals.health = vitals.max_health;
            vitals.stunned_remaining = 0.0;
            return true;
        }
        false
    }
}
fn tick_cards(
    market: Option<Res<super::market::MarketEffects>>,
    time: Res<Time>,
    session: Res<GameSession>,
    build: Res<PlayerBuild>,
    mut runtime: ResMut<BondRuntime>,
    mut vitals: ResMut<HeroVitals>,
) {
    let seconds = if session.paused || session.finished {
        0.0
    } else {
        time.delta_secs()
    };
    runtime.equipment_evasion = market.as_ref().map_or(0.0, |equipment| equipment.evasion());
    runtime.equipment_critical_chance = market
        .as_ref()
        .map_or(0.0, |equipment| equipment.critical_chance());
    runtime.advance(&build, session.level, session.total_kills, seconds);
    let healing =
        runtime.modifiers.regeneration * (1. + runtime.modifiers.regeneration_amp) * seconds;
    vitals.heal(healing);
}
pub(crate) const fn summon_cap(equipment_bonus: u8) -> usize {
    let cap = 18 + equipment_bonus as usize;
    if cap > 20 { 20 } else { cap }
}
pub(crate) const BASE_CRIT_CHANCE: f32 = 0.05;
pub(crate) const BASE_CRIT_MULTIPLIER: f32 = 1.75;
pub(crate) fn critical_multiplier(
    bonds: &BondRuntime,
    equipment_burns_can_crit: bool,
    profile: DamageProfile,
    roll: f32,
) -> f32 {
    let chance = if profile.class == DamageClass::Summon {
        (BASE_CRIT_CHANCE + bonds.critical_chance() * bonds.summon_crit_inheritance()).min(1.)
    } else {
        bonds.critical_chance()
    };
    if (!profile.burn_tick || equipment_burns_can_crit) && roll < chance {
        bonds.critical_damage()
    } else {
        1.0
    }
}
#[cfg(test)]
pub(crate) fn boss_damage_multiplier(profile: DamageProfile, modifiers: BossModifiers) -> f32 {
    boss_damage_multiplier_with_penetration(profile, modifiers, 0.0)
}
pub(crate) fn boss_damage_multiplier_with_penetration(
    profile: DamageProfile,
    modifiers: BossModifiers,
    penetration: f32,
) -> f32 {
    if profile.element == DamageElement::Pure {
        return 1.0;
    }
    let mut multiplier = 1.0 - modifiers.damage_reduction.clamp(0.0, 0.95);
    if profile.element.is_elemental() {
        multiplier *=
            1.0 - (modifiers.elemental_resistance - penetration.clamp(0.0, 0.95)).clamp(0.0, 0.95);
    }
    if profile.element == DamageElement::Flame {
        multiplier *=
            1.0 - (modifiers.flame_resistance - penetration.clamp(0.0, 0.95)).clamp(0.0, 0.95);
    }
    if profile.element == DamageElement::Frost {
        multiplier *= modifiers.frost_damage_taken_multiplier.max(0.0);
    }
    multiplier
}

#[cfg(test)]
mod tests {
    use super::*;
    fn grant(build: &mut PlayerBuild, cards: &[BondCardId]) {
        for card in cards {
            build.owned_bond_cards[card.index()] = true;
        }
    }
    fn close(left: f32, right: f32) {
        assert!((left - right).abs() < 0.005, "{left} != {right}");
    }

    #[test]
    fn archmage_counts_learned_skills_and_supporting_archery_once() {
        let mut build = PlayerBuild::default();
        build.skills.fill(0);
        build.skills[0] = 5;
        grant(
            &mut build,
            &[BondCardId::ElementResonance, BondCardId::ElementFocus],
        );
        let before = CardModifiers::for_build(&build);
        grant(&mut build, &[BondCardId::ArcheryPreparation]);
        let after = CardModifiers::for_build(&build);
        close(after.skill_damage - before.skill_damage, 0.3);
        close(after.intelligence_amp - before.intelligence_amp, 0.15);
        grant(&mut build, &[BondCardId::ArcheryPiercing]);
        close(
            CardModifiers::for_build(&build).skill_damage,
            after.skill_damage,
        );
    }
    #[test]
    fn critical_cards_and_summon_inheritance_change_resolver_thresholds() {
        let mut build = PlayerBuild::default();
        grant(&mut build, &[BondCardId::DemonBow, BondCardId::RequiemFate]);
        let mut runtime = BondRuntime::default();
        runtime.advance(&build, 1, 0, 0.);
        close(
            critical_multiplier(&runtime, false, DamageProfile::PHYSICAL_ATTACK, 0.07),
            1.85,
        );
        close(
            critical_multiplier(
                &runtime,
                false,
                DamageProfile::PHYSICAL_ATTACK,
                runtime.critical_chance(),
            ),
            1.,
        );
        let summon = DamageProfile::new(DamageElement::Physical, DamageClass::Summon);
        close(critical_multiplier(&runtime, false, summon, 0.085), 1.85);
        close(critical_multiplier(&runtime, false, summon, 0.1), 1.);
        let burn = DamageProfile {
            burn_tick: true,
            ..DamageProfile::new(DamageElement::Flame, DamageClass::Spell)
        };
        close(critical_multiplier(&runtime, false, burn, 0.), 1.);
        close(critical_multiplier(&runtime, true, burn, 0.), 1.85);
    }
    #[test]
    fn cultivation_unique_cards_drive_real_damage_and_proc_chances() {
        let mut build = PlayerBuild::default();
        grant(&mut build, &[BondCardId::MortalCultivation]);
        let mut runtime = BondRuntime::default();
        runtime.advance(&build, 1, 0, 0.);
        close(
            runtime.damage_multiplier(DamageProfile::PHYSICAL_ATTACK),
            1.05,
        );
        grant(
            &mut build,
            &[
                BondCardId::BambooCloudSword,
                BondCardId::VirtualHeavenCauldron,
            ],
        );
        runtime.advance(&build, 1, 0, 0.);
        close(
            runtime.damage_multiplier(DamageProfile::PHYSICAL_ATTACK),
            1.15,
        );
        close(runtime.extra_attack_chance(), 0.15);
        close(runtime.suppression_chance(), 0.12);
        close(runtime.suppression_multiplier(), 3.);
        grant(&mut build, &[BondCardId::MortalCultivation]);
        runtime.advance(&build, 1, 0, 0.);
        close(
            runtime.damage_multiplier(DamageProfile::PHYSICAL_ATTACK),
            1.15,
        );
    }
    #[test]
    fn suppression_only_counts_real_health_loss_and_drains_once() {
        let mut runtime = BondRuntime::default();
        let mut vitals = HeroVitals {
            invulnerable_remaining: 2.,
            ..default()
        };
        runtime.take_physical_damage(&mut vitals, 10.);
        assert_eq!(runtime.take_received_hits(), 0);
        vitals.invulnerable_remaining = 0.;
        runtime.take_physical_damage(&mut vitals, 10.);
        assert_eq!(runtime.take_received_hits(), 1);
        assert_eq!(runtime.take_received_hits(), 0);
    }
    #[test]
    fn elemental_cards_do_not_amplify_unrelated_elements() {
        let mut build = PlayerBuild::default();
        grant(&mut build, &[BondCardId::LightningWill]);
        let mut runtime = BondRuntime::default();
        runtime.advance(&build, 1, 0, 0.);
        close(
            runtime.damage_multiplier(DamageProfile::new(DamageElement::Storm, DamageClass::Spell)),
            1.3,
        );
        close(
            runtime.damage_multiplier(DamageProfile::new(DamageElement::Flame, DamageClass::Spell)),
            1.,
        );
        close(
            runtime.damage_multiplier(DamageProfile::PHYSICAL_ATTACK),
            1.,
        );
    }
    #[test]
    fn science_snapshot_is_granted_once_across_stat_rebuilds() {
        let mut build = PlayerBuild::default();
        grant(
            &mut build,
            &[
                BondCardId::AgilityPotion,
                BondCardId::StrengthSerum,
                BondCardId::InsightElixir,
                BondCardId::UnstableCatalyst,
            ],
        );
        let mut runtime = BondRuntime::default();
        let mut vitals = HeroVitals::default();
        runtime.advance(&build, 1, 0, 0.);
        runtime.apply_stats(&mut build, &mut vitals);
        let first = vitals.strength;
        let permanent = runtime.permanent.strength;
        for _ in 0..10 {
            runtime.remove_applied(&mut build, &mut vitals);
            runtime.advance(&build, 1, 0, 0.);
            runtime.apply_stats(&mut build, &mut vitals);
        }
        close(vitals.strength, first);
        close(runtime.permanent.strength, permanent);
        close(runtime.model_scale(), 1.4);
    }
    #[test]
    fn slaughter_only_counts_future_kills_in_complete_batches() {
        let mut build = PlayerBuild::default();
        let mut runtime = BondRuntime::default();
        runtime.advance(&build, 1, 900, 0.);
        grant(&mut build, &[BondCardId::BloodSlaughter]);
        runtime.advance(&build, 1, 900, 0.);
        runtime.advance(&build, 1, 1199, 0.);
        close(runtime.critical_damage(), BASE_CRIT_MULTIPLIER);
        runtime.advance(&build, 1, 1500, 0.);
        close(runtime.critical_damage(), BASE_CRIT_MULTIPLIER + 0.02);
        runtime.advance(&build, 1, 1500, 0.);
        close(runtime.critical_damage(), BASE_CRIT_MULTIPLIER + 0.02);
    }
    #[test]
    fn strength_and_intelligence_cards_grow_and_amplify_their_actual_attributes() {
        let mut build = PlayerBuild::default();
        grant(
            &mut build,
            &[
                BondCardId::StrengthTraining,
                BondCardId::StrengthGrowth,
                BondCardId::IntelligenceTraining,
                BondCardId::IntelligenceGrowth,
            ],
        );
        let mut runtime = BondRuntime::default();
        runtime.advance(&build, 1, 100, 0.);
        runtime.advance(&build, 1, 110, 5.);
        close(runtime.permanent.strength, 2.);
        close(runtime.permanent.intelligence, 2.);
        let mut vitals = HeroVitals::default();
        let strength = vitals.strength;
        let intelligence = vitals.intelligence;
        runtime.apply_stats(&mut build, &mut vitals);
        close(vitals.strength, (strength + 52.) * 1.1);
        close(vitals.intelligence, (intelligence + 52.) * 1.1);
    }
    #[test]
    fn archery_ammo_is_opt_in_dynamic_and_recharges_only_with_elapsed_time() {
        let mut build = PlayerBuild::default();
        let mut runtime = BondRuntime::default();
        runtime.advance(&build, 1, 0, 20.);
        assert!(!runtime.archery_active());
        assert_eq!(runtime.arrows_remaining, 0);
        grant(
            &mut build,
            &[
                BondCardId::ArcheryPreparation,
                BondCardId::ArcheryPiercing,
                BondCardId::ArcheryVolley,
                BondCardId::ArcheryHaste,
            ],
        );
        runtime.advance(&build, 1, 0, 0.);
        assert_eq!(runtime.arrows_remaining, 40);
        let mut vitals = HeroVitals::default();
        let agility = vitals.agility;
        runtime.apply_stats(&mut build, &mut vitals);
        close(vitals.agility, (agility + 2. * 40.) * (1. + 2. * 0.05));
        runtime.remove_applied(&mut build, &mut vitals);
        for _ in 0..40 {
            runtime.consume_arrow();
        }
        assert_eq!(runtime.arrows_remaining, 0);
        close(runtime.recharge_remaining, 7.);
        runtime.advance(&build, 1, 0, 0.);
        assert_eq!(runtime.arrows_remaining, 0);
        runtime.apply_stats(&mut build, &mut vitals);
        close(vitals.agility, (agility + 2. * 40.) * (1. + 2. * 0.05));
        // Empty reserve must not remove volley-based attributes or double their own bonus.
        runtime.remove_applied(&mut build, &mut vitals);
        runtime.apply_stats(&mut build, &mut vitals);
        close(vitals.agility, (agility + 2. * 40.) * (1. + 2. * 0.05));
        runtime.advance(&build, 1, 0, 6.);
        assert_eq!(runtime.arrows_remaining, 0);
        runtime.advance(&build, 1, 0, 1.);
        assert_eq!(runtime.arrows_remaining, 40);
    }
    #[test]
    fn arcane_echo_and_chain_share_cooldown_and_never_recurse() {
        let mut build = PlayerBuild::default();
        grant(
            &mut build,
            &[BondCardId::ArcanumEcho, BondCardId::ArcanumChain],
        );
        let mut runtime = BondRuntime::default();
        runtime.advance(&build, 1, 0, 0.);
        let spell = DamageProfile::new(DamageElement::Arcane, DamageClass::Spell);
        assert_eq!(runtime.spell_procs(spell, 0.), (true, 1));
        assert_eq!(runtime.spell_procs(spell, 0.), (false, 0));
        runtime.advance(&build, 1, 0, 0.);
        assert_eq!(runtime.spell_procs(spell, 0.), (false, 0));
        runtime.advance(&build, 1, 0, 1.);
        assert_eq!(runtime.spell_procs(spell.as_card_proc(), 0.), (false, 0));
        assert_eq!(runtime.spell_procs(spell.as_market_proc(), 0.), (false, 0));
        assert_eq!(runtime.spell_procs(spell, 0.9), (false, 1));
    }
    #[test]
    fn warpath_changes_only_melee_damage_and_heals_each_kill_once() {
        let mut build = PlayerBuild::default();
        grant(
            &mut build,
            &[
                BondCardId::WarpathCleave,
                BondCardId::WarpathRecovery,
                BondCardId::WarpathFury,
            ],
        );
        let mut runtime = BondRuntime::default();
        runtime.advance(&build, 1, 100, 0.);
        runtime.advance(&build, 1, 103, 0.);
        assert_eq!(runtime.melee_cleave(), 1);
        close(runtime.take_kill_healing(), 6.);
        close(runtime.take_kill_healing(), 0.);
        close(
            runtime.damage_multiplier(DamageProfile::PHYSICAL_ATTACK),
            1.,
        );
        close(
            runtime.damage_multiplier(DamageProfile::PHYSICAL_ATTACK.as_melee()),
            1.25,
        );
    }
    #[test]
    fn arrow_ricochet_card_applies_once_and_removes_without_erasing_other_sources() {
        let mut build = PlayerBuild {
            ricochet: 2,
            ..default()
        };
        let mut vitals = HeroVitals::default();
        let mut runtime = BondRuntime::default();
        grant(&mut build, &[BondCardId::ArcheryHaste]);
        for _ in 0..3 {
            runtime.advance(&build, 1, 0, 0.);
            runtime.apply_stats(&mut build, &mut vitals);
            assert_eq!(build.ricochet, 3);
            runtime.remove_applied(&mut build, &mut vitals);
            assert_eq!(build.ricochet, 2);
        }
        build.owned_bond_cards.fill(false);
        runtime.advance(&build, 1, 0, 0.);
        runtime.apply_stats(&mut build, &mut vitals);
        assert_eq!(build.ricochet, 2);
    }

    #[test]
    fn ranger_tiers_are_cumulative_and_removal_restores_baseline() {
        let mut build = PlayerBuild::default();
        let baseline_shots = build.multishot;
        grant(
            &mut build,
            &[BondCardId::RangerRapidFire, BondCardId::RangerDeadeye],
        );
        let mut runtime = BondRuntime::default();
        let mut vitals = HeroVitals::default();
        let baseline_agility = vitals.agility;
        runtime.advance(&build, 1, 0, 0.);
        runtime.apply_stats(&mut build, &mut vitals);
        assert_eq!(build.multishot, baseline_shots + 1);
        assert_eq!(build.pierce, 1);
        runtime.remove_applied(&mut build, &mut vitals);
        grant(
            &mut build,
            &[BondCardId::RangerSkyshot, BondCardId::RangerWindrunner],
        );
        runtime.advance(&build, 1, 0, 0.);
        runtime.apply_stats(&mut build, &mut vitals);
        assert_eq!(build.multishot, baseline_shots + 2);
        assert_eq!(build.pierce, 2);
        close(vitals.agility, baseline_agility + 120.);
        runtime.remove_applied(&mut build, &mut vitals);
        build.owned_bond_cards.fill(false);
        runtime.advance(&build, 1, 0, 0.);
        runtime.apply_stats(&mut build, &mut vitals);
        assert_eq!(build.multishot, baseline_shots);
        assert_eq!(build.pierce, 0);
        close(vitals.agility, baseline_agility);
    }
    #[test]
    fn growth_begins_after_acquisition_and_survives_family_removal() {
        let mut build = PlayerBuild::default();
        let mut runtime = BondRuntime::default();
        runtime.advance(&build, 10, 100, 0.);
        grant(
            &mut build,
            &[BondCardId::DivineStrength, BondCardId::DivineMight],
        );
        runtime.advance(&build, 10, 100, 0.);
        close(runtime.permanent.strength, 0.);
        runtime.advance(&build, 12, 100, 0.);
        close(runtime.permanent.strength, 10.);
        close(runtime.permanent.attack, 20.);
        close(runtime.permanent.health_amp, 0.02);
        build.owned_bond_cards.fill(false);
        runtime.advance(&build, 12, 100, 0.);
        runtime.advance(&build, 13, 100, 0.);
        close(runtime.permanent.strength, 10.);
    }
    #[test]
    fn agility_progression_counts_time_and_monotonic_kills_once() {
        let mut build = PlayerBuild::default();
        grant(
            &mut build,
            &[BondCardId::AgilityTraining, BondCardId::AgilityGrowth],
        );
        let mut runtime = BondRuntime::default();
        runtime.advance(&build, 1, 100, 0.);
        runtime.advance(&build, 1, 110, 5.);
        close(runtime.permanent.agility, 2.);
        runtime.advance(&build, 1, 110, 0.);
        close(runtime.permanent.agility, 2.);
    }
    #[test]
    fn judge_brush_blocks_fatal_hit_then_two_revives_are_consumed_once() {
        let mut build = PlayerBuild::default();
        grant(
            &mut build,
            &[BondCardId::JudgeBrush, BondCardId::BookOfLife],
        );
        let mut runtime = BondRuntime::default();
        runtime.advance(&build, 1, 0, 0.);
        let mut vitals = HeroVitals {
            health: 37.,
            ..Default::default()
        };
        runtime.take_physical_damage(&mut vitals, 10000.);
        close(vitals.health, 37.);
        close(vitals.invulnerable_remaining, 2.);
        close(runtime.immunity_remaining, 120.);
        assert_eq!(runtime.revives_used, 0);
        vitals.invulnerable_remaining = 0.;
        for used in 1..=2 {
            vitals.health = 0.;
            assert!(runtime.intercept_fatal(&mut vitals));
            assert_eq!(runtime.revives_used, used);
            close(vitals.health, vitals.max_health);
        }
        vitals.health = 0.;
        assert!(!runtime.intercept_fatal(&mut vitals));
        runtime.advance(&build, 1, 0, 120.);
        assert!(runtime.intercept_fatal(&mut vitals));
        assert_eq!(runtime.revives_used, 2);
    }
    #[test]
    fn complete_ascension_grows_each_unlocked_damage_category() {
        let mut build = PlayerBuild::default();
        grant(
            &mut build,
            &[
                BondCardId::DivineMight,
                BondCardId::DivineStrength,
                BondCardId::DivineIntelligence,
                BondCardId::DivineAgility,
                BondCardId::DivineBody,
            ],
        );
        let mut runtime = BondRuntime::default();
        runtime.advance(&build, 8, 0, 0.);
        runtime.advance(&build, 9, 0, 0.);
        close(runtime.permanent.health_amp, 0.01);
        close(runtime.permanent.skill_damage, 0.01);
        close(
            runtime.damage_multiplier(DamageProfile::PHYSICAL_ATTACK),
            1.01 * 1.01,
        );
        close(
            runtime.damage_multiplier(DamageProfile::new(
                DamageElement::Arcane,
                DamageClass::Spell,
            )),
            1.01 * 1.01,
        );
    }
}
