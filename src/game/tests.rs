use std::collections::HashSet;

use super::content::{Attribute, BONDS, BOSSES, ENEMIES, HEROES, Rarity, SKILLS};
use super::progression::bond_cost;
use super::{BondId, GameSession, HeroKind, PlayerBuild, SkillId};

#[test]
fn content_catalog_has_expected_unique_counts() {
    assert_eq!(HEROES.len(), HeroKind::COUNT);
    assert_eq!(SKILLS.len(), 30);
    assert_eq!(BONDS.len(), 10);
    assert_eq!(ENEMIES.len(), 12);
    assert_eq!(BOSSES.len(), 5);

    let mut ids = HashSet::new();
    for id in HEROES
        .iter()
        .map(|item| item.id)
        .chain(SKILLS.iter().map(|item| item.id))
        .chain(BONDS.iter().map(|item| item.id))
        .chain(ENEMIES.iter().map(|item| item.id))
        .chain(BOSSES.iter().map(|item| item.id))
    {
        assert!(ids.insert(id), "duplicate content id: {id}");
    }
}

#[test]
fn every_hero_has_three_valid_theme_starts_and_attributes_are_balanced() {
    let mut attribute_counts = [0_usize; 3];
    let mut rarity_attribute_counts = [[0_usize; 3]; 4];
    for hero in HEROES {
        let primary_index = match hero.primary {
            Attribute::Strength => 0,
            Attribute::Agility => 1,
            Attribute::Intelligence => 2,
        };
        attribute_counts[primary_index] += 1;
        rarity_attribute_counts[hero.rarity.index()][primary_index] += 1;

        assert_eq!(hero.starting_skill, hero.starting_skill_pool[0]);
        let unique: HashSet<_> = hero.starting_skill_pool.into_iter().collect();
        assert_eq!(unique.len(), hero.starting_skill_pool.len(), "{}", hero.id);
        for starting_skill in hero.starting_skill_pool {
            assert!(
                SKILLS.iter().any(|skill| skill.id == starting_skill),
                "{} has missing starting skill {starting_skill}",
                hero.id
            );
        }
    }
    assert_eq!(attribute_counts, [4, 4, 4]);
    assert_eq!(rarity_attribute_counts, [[1, 1, 1]; 4]);
}

#[test]
fn bosses_follow_five_minute_schedule() {
    let times: Vec<u16> = BOSSES.iter().map(|boss| boss.spawn_seconds).collect();
    assert_eq!(times, [300, 600, 900, 1200, 1500]);
}

#[test]
fn core_and_bond_thresholds_match_the_gameplay_spec() {
    let session = GameSession::default();
    assert_eq!(session.core_hp, 1_000.0);
    assert_eq!(session.core_max_hp, 1_000.0);
    assert!(BondId::ALL.into_iter().all(|bond| bond.required() == 6));
}

#[test]
fn bond_price_steps_every_two_draws_and_caps() {
    let mut session = GameSession::default();
    let prices: Vec<u32> = (0..8)
        .map(|draws| {
            session.bond_draws = draws;
            bond_cost(&session)
        })
        .collect();
    assert_eq!(prices, [10, 10, 12, 12, 14, 14, 16, 16]);
    session.bond_draws = 100;
    assert_eq!(bond_cost(&session), 20);
}

#[test]
fn completed_bond_releases_one_of_seven_slots() {
    let mut build = PlayerBuild::default();
    for id in BondId::ALL.into_iter().take(7) {
        build.bonds[id.index()] = 1;
    }
    assert_eq!(build.active_bond_count(), 7);
    build.completed_bonds[BondId::Thunder.index()] = true;
    assert_eq!(build.active_bond_count(), 6);
}

#[test]
fn every_skill_supports_runtime_ranks() {
    let build = PlayerBuild::default();
    for index in 0..SkillId::COUNT {
        let id = SkillId(index as u8);
        assert_eq!(build.skill_rank(id), 0);
        assert!((1..=5).contains(&id.definition().max_rank));
    }
}

#[test]
fn skill_catalog_uses_all_four_build_rarities() {
    let counts = SKILLS.iter().fold([0_u8; 4], |mut counts, skill| {
        let index = match skill.rarity {
            Rarity::R => 0,
            Rarity::Sr => 1,
            Rarity::Ssr => 2,
            Rarity::Ur => 3,
        };
        counts[index] += 1;
        counts
    });
    assert_eq!(counts, [12, 8, 6, 4]);
}

#[test]
fn bond_catalog_uses_all_four_build_rarities() {
    let mut counts = [0_u8; 4];
    for bond in BONDS {
        counts[bond.rarity.index()] += 1;
    }
    assert_eq!(counts, [3, 3, 2, 2]);
}

#[test]
fn rarity_is_strictly_rarer_and_stronger_at_every_tier() {
    for pair in Rarity::ALL.windows(2) {
        let lower = pair[0];
        let higher = pair[1];
        assert!(higher.draw_weight() < lower.draw_weight());
        assert!(higher.hero_stat_multiplier() > lower.hero_stat_multiplier());
        assert!(higher.hero_ability_multiplier() > lower.hero_ability_multiplier());
        assert!(higher.hero_cooldown_multiplier() < lower.hero_cooldown_multiplier());
        assert!(higher.starting_skill_rank() > lower.starting_skill_rank());
        assert!(higher.skill_power_multiplier() > lower.skill_power_multiplier());
        assert!(higher.skill_cooldown_multiplier() < lower.skill_cooldown_multiplier());
        assert!(higher.bond_points_per_pick() > lower.bond_points_per_pick());
    }

    assert_eq!(Rarity::R.color_name(), "WHITE");
    assert_eq!(Rarity::Sr.color_name(), "BLUE");
    assert_eq!(Rarity::Ssr.color_name(), "ORANGE");
    assert_eq!(Rarity::Ur.color_name(), "RED");
}
