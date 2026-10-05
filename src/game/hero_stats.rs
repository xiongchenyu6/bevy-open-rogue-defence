//! Read-only hero sheet, derived from the same values used by combat.
use super::basic_attacks::{class_basic_stats, weapon};
use super::bonds::{BondRuntime, DamageProfile, summon_cap};
use super::combat::{effective_basic_attack_rate, market_outgoing_damage_multiplier};
use super::hero_abilities::HeroAttackSpeedModifier;
use super::market::MarketEffects;
use super::{HeroVitals, PlayerBuild};
use bevy::prelude::*;

/// Damage values are per native attack before target armor, boss resistances and
/// critical hits. Min/max additionally account for the Chaos Die's random roll.
/// Penetration comes from equipped black-market items and matches the damage resolver.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct HeroStatsSnapshot {
    pub level: u32,
    pub strength: f32,
    pub agility: f32,
    pub intelligence: f32,
    pub health: f32,
    pub max_health: f32,
    pub energy: f32,
    pub max_energy: f32,
    pub attack_damage: f32,
    pub basic_hit_damage: f32,
    pub basic_hit_min: f32,
    pub basic_hit_max: f32,
    pub attacks_per_second: f32,
    pub attack_range: f32,
    pub cleave_target_bonus: usize,
    pub move_speed: f32,
    pub armor: f32,
    pub physical_damage_reduction: f32,
    pub magic_resistance: f32,
    pub spell_power: f32,
    pub summon_power: f32,
    pub crit_chance: f32,
    pub crit_multiplier: f32,
    pub life_leech: f32,
    pub physical_penetration: f32,
    pub spell_penetration: f32,
    pub evasion: f32,
    pub spell_echo_chance: f32,
    pub summon_cap: usize,
}

impl HeroStatsSnapshot {
    pub(crate) fn capture(
        vitals: &HeroVitals,
        build: &PlayerBuild,
        bonds: &BondRuntime,
        market: &MarketEffects,
        speed: &HeroAttackSpeedModifier,
    ) -> Self {
        let weapon = weapon(vitals.kind);
        let profile = if weapon.melee() {
            DamageProfile::PHYSICAL_ATTACK.as_melee()
        } else {
            DamageProfile::PHYSICAL_ATTACK
        };
        let basic_hit_damage = build.attack_damage
            * bonds.damage_multiplier(profile)
            * market_outgoing_damage_multiplier(market, profile, build, vitals);
        let (random_min, random_max) = market.chaos_damage_bounds();
        let spell_power = market.effective_spell_power(vitals.spell_power());
        Self {
            level: vitals.level,
            strength: vitals.strength,
            agility: vitals.agility,
            intelligence: vitals.intelligence,
            health: vitals.health,
            max_health: vitals.max_health,
            energy: vitals.energy,
            max_energy: vitals.max_energy,
            attack_damage: build.attack_damage,
            basic_hit_damage,
            basic_hit_min: basic_hit_damage * random_min,
            basic_hit_max: basic_hit_damage * random_max,
            attacks_per_second: effective_basic_attack_rate(build.attacks_per_second, speed.0),
            attack_range: class_basic_stats(vitals.kind, vitals.level, build).range,
            cleave_target_bonus: bonds.melee_cleave(),
            move_speed: build.move_speed,
            armor: vitals.armor,
            physical_damage_reduction: 1.0 - 100.0 / (100.0 + 5.0 * vitals.armor.max(0.0)),
            magic_resistance: vitals.magic_resistance.clamp(0.0, 0.95),
            spell_power,
            summon_power: market
                .effective_summon_power(spell_power * 0.7 + vitals.attack_damage() * 0.5),
            crit_chance: bonds.critical_chance(),
            crit_multiplier: bonds.critical_damage(),
            life_leech: market.life_leech() + bonds.life_leech(),
            physical_penetration: market.physical_penetration(),
            spell_penetration: market.spell_penetration(),
            evasion: bonds.evasion_chance(),
            spell_echo_chance: market.spell_echo_chance(),
            summon_cap: summon_cap(market.summon_cap_bonus()),
        }
    }
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<HeroStatsSnapshot>()
        .add_systems(Last, update)
        .add_systems(Update, update_model_scale.after(super::bonds::BondSet));
}

fn update_model_scale(
    bonds: Res<BondRuntime>,
    mut visuals: Query<&mut Transform, With<super::HeroVisual>>,
) {
    for mut visual in &mut visuals {
        visual.scale = Vec3::splat(bonds.model_scale());
    }
}

fn update(
    vitals: Res<HeroVitals>,
    build: Res<PlayerBuild>,
    bonds: Res<BondRuntime>,
    market: Res<MarketEffects>,
    speed: Res<HeroAttackSpeedModifier>,
    mut snapshot: ResMut<HeroStatsSnapshot>,
) {
    *snapshot = HeroStatsSnapshot::capture(&vitals, &build, &bonds, &market, &speed);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{HeroKind, bonds::critical_multiplier};
    #[test]
    fn learned_bonds_update_displayed_stats_and_model_without_scale_accumulation() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<super::super::GameSession>()
            .init_resource::<PlayerBuild>()
            .init_resource::<HeroVitals>()
            .add_plugins(super::super::bonds::BondsPlugin)
            .add_systems(
                Update,
                update_model_scale.after(super::super::bonds::BondSet),
            );
        for card in [
            super::super::BondCardId::AgilityPotion,
            super::super::BondCardId::StrengthSerum,
            super::super::BondCardId::InsightElixir,
            super::super::BondCardId::UnstableCatalyst,
            super::super::BondCardId::DemonBow,
            super::super::BondCardId::BloodFeast,
        ] {
            app.world_mut()
                .resource_mut::<PlayerBuild>()
                .owned_bond_cards[card.index()] = true;
        }
        let visual = app
            .world_mut()
            .spawn((super::super::HeroVisual, Transform::default()))
            .id();
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(
            app.world().get::<Transform>(visual).unwrap().scale,
            Vec3::splat(1.4)
        );
        let snapshot = HeroStatsSnapshot::capture(
            app.world().resource::<HeroVitals>(),
            app.world().resource::<PlayerBuild>(),
            app.world().resource::<BondRuntime>(),
            &MarketEffects::default(),
            &HeroAttackSpeedModifier(1.),
        );
        assert!((snapshot.crit_chance - 0.08).abs() < 0.001);
        assert!((snapshot.crit_multiplier - 1.85).abs() < 0.001);
        assert!((snapshot.life_leech - 0.03).abs() < 0.001);
        app.world_mut()
            .resource_mut::<PlayerBuild>()
            .owned_bond_cards
            .fill(false);
        app.update();
        assert_eq!(
            app.world().get::<Transform>(visual).unwrap().scale,
            Vec3::ONE
        );
    }
    #[test]
    fn snapshot_matches_live_attack_clock_weapon_range_and_hero_mitigation() {
        for kind in HeroKind::ALL {
            let mut vitals = HeroVitals::for_hero(kind, 10);
            vitals.armor = 8.0;
            vitals.magic_resistance = 0.2;
            let build = PlayerBuild {
                attack_damage: 150.0,
                attacks_per_second: 2.0,
                attack_range: 12.0,
                ..default()
            };
            let bonds = BondRuntime::default();
            let snapshot = HeroStatsSnapshot::capture(
                &vitals,
                &build,
                &BondRuntime::default(),
                &MarketEffects::default(),
                &HeroAttackSpeedModifier(1.3),
            );
            assert!((snapshot.attacks_per_second - 2.6).abs() < 0.001);
            assert_eq!(
                snapshot.attack_range,
                class_basic_stats(kind, vitals.level, &build).range
            );
            assert_eq!(snapshot.basic_hit_damage, 150.0);
            assert!(
                (vitals.take_physical_damage(100.0)
                    - 100.0 * (1.0 - snapshot.physical_damage_reduction))
                    .abs()
                    < 0.001
            );
            assert!(
                (vitals.take_magical_damage(100.0) - 100.0 * (1.0 - snapshot.magic_resistance))
                    .abs()
                    < 0.001
            );
            assert_eq!(
                critical_multiplier(
                    &bonds,
                    false,
                    DamageProfile::PHYSICAL_ATTACK,
                    snapshot.crit_chance - 0.001
                ),
                snapshot.crit_multiplier
            );
            assert_eq!(
                critical_multiplier(
                    &bonds,
                    false,
                    DamageProfile::PHYSICAL_ATTACK,
                    snapshot.crit_chance
                ),
                1.0
            );
            assert_eq!(snapshot.physical_penetration, 0.0);
            assert_eq!(snapshot.spell_penetration, 0.0);
        }
    }
}
