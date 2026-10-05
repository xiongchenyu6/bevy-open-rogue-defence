use std::collections::HashSet;

use super::bond_catalog::{BOND_CARDS, BOND_FAMILIES};
use super::content::{Attribute, BOSSES, ENEMIES, HEROES, Rarity, SKILLS};
use super::progression::bond_cost;
use super::{BondId, EncounterState, GameSession, HeroKind, PlayerBuild, SkillId};

#[test]
fn content_catalog_has_expected_unique_counts() {
    assert_eq!(HEROES.len(), HeroKind::COUNT);
    assert_eq!(SKILLS.len(), 32);
    assert_eq!(BOND_CARDS.len(), 101);
    assert_eq!(BOND_FAMILIES.len(), 26);
    assert_eq!(ENEMIES.len(), 12);
    assert_eq!(BOSSES.len(), 5);

    let mut ids = HashSet::new();
    for id in HEROES
        .iter()
        .map(|item| item.id)
        .chain(SKILLS.iter().map(|item| item.id))
        .chain(BOND_CARDS.iter().map(|item| item.id))
        .chain(BOND_FAMILIES.iter().map(|item| item.id))
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
fn starting_skill_pools_stay_within_one_rarity_tier_above_the_hero() {
    for hero in HEROES {
        for starting_skill in hero.starting_skill_pool {
            let skill = SKILLS
                .iter()
                .find(|skill| skill.id == starting_skill)
                .expect("starting skill exists");
            assert!(
                skill.rarity.index() <= hero.rarity.index() + 1,
                "{} ({}) cannot start with {} ({})",
                hero.id,
                hero.rarity.label(),
                skill.id,
                skill.rarity.label()
            );
        }
    }
}

#[test]
fn boss_table_keeps_the_five_minute_schedule_and_combat_numbers() {
    let rows: Vec<_> = BOSSES
        .iter()
        .map(|boss| {
            (
                boss.name_en,
                boss.spawn_seconds,
                boss.hp,
                boss.speed,
                boss.damage_per_second,
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            ("CINDER COLOSSUS", 300.0, 8_000.0, 0.85, 14.0),
            ("ABYSSAL LEVIATHAN", 600.0, 28_000.0, 0.62, 20.0),
            ("DREAD VEIL", 900.0, 70_000.0, 0.92, 28.0),
            ("SUN CROWN MAGUS", 1_200.0, 160_000.0, 1.18, 38.0),
            ("THE GREEN REGENT", 1_500.0, 420_000.0, 0.78, 55.0),
        ]
    );
}

#[test]
fn core_and_family_member_counts_match_reference_subset() {
    let session = GameSession::default();
    assert_eq!(session.core_hp, 1_000.0);
    assert_eq!(session.core_max_hp, 1_000.0);
    assert_eq!(
        BondId::ALL.map(|bond| bond.required()),
        [
            3, 2, 4, 2, 5, 2, 2, 4, 3, 3, 8, 12, 4, 4, 4, 4, 4, 4, 2, 3, 3, 4, 4, 4, 3, 4
        ]
    );
}

#[test]
fn bond_price_rises_five_wood_per_paid_draw_without_cap() {
    let mut session = GameSession::default();
    let encounters = EncounterState::default();
    let prices: Vec<u32> = (0..5)
        .map(|draws| {
            session.bond_draws = draws;
            bond_cost(&session, &encounters)
        })
        .collect();
    assert_eq!(prices, [10, 15, 20, 25, 30]);
    session.bond_draws = 30;
    assert_eq!(bond_cost(&session, &encounters), 160);
}

#[test]
fn completed_bond_releases_one_of_seven_slots() {
    let mut build = PlayerBuild::default();
    let family = BondId::Journey;
    let members = family.definition().members;
    build.owned_bond_cards[members[0].index()] = true;
    assert_eq!(build.active_bond_count(), 1);
    for card in members {
        build.owned_bond_cards[card.index()] = true;
    }
    assert_eq!(build.active_bond_count(), 0);
    assert!(build.bond_complete(family));
    assert_eq!(build.bond(family), 3);
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
    assert_eq!(counts, [12, 10, 6, 4]);
}

#[test]
fn bond_cards_have_valid_membership_and_unique_icons_exist() {
    for card in super::BondCardId::ALL {
        assert!(card.family().definition().members.contains(&card));
        let icon = card.definition().icon;
        assert!(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("assets/ui/icons/{icon}.png"))
                .is_file()
        );
    }
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
    }

    assert_eq!(Rarity::R.color_name(), "WHITE");
    assert_eq!(Rarity::Sr.color_name(), "BLUE");
    assert_eq!(Rarity::Ssr.color_name(), "ORANGE");
    assert_eq!(Rarity::Ur.color_name(), "RED");
}
