//! Production draft distribution and finite-budget UI regressions.
use super::tests::{press_choice, tap_g, test_app};
use super::*;

#[test]
fn skill_hands_do_not_guarantee_the_highest_rank_or_a_fixed_first_slot() {
    let mut build = PlayerBuild::default();
    build.skills[..6].fill(1);
    build.skills[0] = 4;
    let mut missed_highest = 0;
    let mut firsts = std::collections::BTreeSet::new();
    for seed in 1..=512 {
        let choices = pick_skill_choices(
            &mut RunRng(seed),
            30,
            600.0,
            &build,
            &SelectedHero::default(),
            &mut DraftAntiDrought::default(),
            6,
        );
        assert_eq!(choices.len(), 3);
        assert!(choices.iter().all(|choice| matches!(choice,
            DraftChoice::Skill(skill) if build.skill_rank(*skill) > 0)));
        missed_highest += usize::from(!choices.contains(&DraftChoice::Skill(SkillId(0))));
        if let DraftChoice::Skill(skill) = choices[0] {
            firsts.insert(skill.0);
        }
    }
    assert!(
        missed_highest > 50,
        "highest skill was absent only {missed_highest} times"
    );
    assert!(firsts.len() >= 5);
}

#[test]
fn ascensions_sample_all_eligible_skills_and_share_hands_with_normal_upgrades() {
    let mut build = PlayerBuild::default();
    build.skills[..5].fill(5);
    build.skills[5] = 1;
    let evolution = SkillEvolution {
        unlocked_stage: 1,
        ..Default::default()
    };
    let mut offered = std::collections::BTreeSet::new();
    let mut normal_hands = 0;
    for seed in 1..=128 {
        let choices = late_skill_choices(
            vec![DraftChoice::Skill(SkillId(5))],
            &build,
            &evolution,
            600.0,
            &mut RunRng(seed),
        );
        assert_eq!(choices.len(), 3);
        normal_hands += usize::from(choices.contains(&DraftChoice::Skill(SkillId(5))));
        for choice in choices {
            if let DraftChoice::Evolution { skill, tier } = choice {
                assert_eq!(tier, 1);
                offered.insert(skill.0);
            }
        }
    }
    assert_eq!(offered.len(), 5);
    assert!(normal_hands > 20 && normal_hands < 110);
}

#[test]
fn ur_replacements_are_rare_while_training_remains_available() {
    let mut build = PlayerBuild::default();
    build.skills[..6].fill(1);
    let evolution = SkillEvolution::default();
    let mut rng = RunRng(0xBEEF);
    let mut replacement_hands = 0;
    for _ in 0..1_000 {
        let choices = late_skill_choices(
            vec![DraftChoice::Skill(SkillId(0))],
            &build,
            &evolution,
            900.0,
            &mut rng,
        );
        replacement_hands += usize::from(
            choices
                .iter()
                .any(|choice| matches!(choice, DraftChoice::ReplaceSkill { .. })),
        );
    }
    assert!(
        (20..90).contains(&replacement_hands),
        "replacement hands: {replacement_hands}"
    );
    build.skills[..6].fill(5);
    assert!(
        late_skill_choices(Vec::new(), &build, &evolution, 900.0, &mut rng)
            .iter()
            .any(|choice| matches!(choice, DraftChoice::ReplaceSkill { .. }))
    );
}

#[test]
fn final_full_bar_hands_never_combine_owned_ur_upgrades_and_new_ur_replacements() {
    let mut build = PlayerBuild::default();
    build.skills[..5].fill(1);
    let ur = (0..SKILLS.len())
        .find(|index| SKILLS[*index].rarity == Rarity::Ur)
        .unwrap();
    build.skills[ur] = 1;
    let mut rng = RunRng(0xA11CE);
    let evolution = SkillEvolution::default();
    let mut drought = DraftAntiDrought::default();
    let mut upgraded_ur = 0;
    let mut replaced_ur = 0;
    for _ in 0..1_000 {
        let choices = pick_skill_choices(
            &mut rng,
            30,
            900.0,
            &build,
            &SelectedHero::default(),
            &mut drought,
            6,
        );
        let choices = late_skill_choices(choices, &build, &evolution, 900.0, &mut rng);
        assert_eq!(choices.len(), 3);
        let mut ur_cards = 0;
        for choice in choices {
            match choice {
                DraftChoice::Skill(skill) if skill.definition().rarity == Rarity::Ur => {
                    upgraded_ur += 1;
                    ur_cards += 1;
                }
                DraftChoice::ReplaceSkill { .. } => {
                    replaced_ur += 1;
                    ur_cards += 1;
                }
                _ => {}
            }
        }
        assert!(ur_cards <= 1);
    }
    assert!(upgraded_ur > 0 && replaced_ur > 0);
}

#[test]
fn audit_seeded_skill_purchase_budgets_through_real_card_clicks() {
    for budget in [5_000, 10_000] {
        let mut builds = std::collections::BTreeSet::new();
        let mut draws = 0;
        let mut mastered = 0;
        let mut full_mastery = 0;
        for seed in 1..=32 {
            let mut app = test_app();
            app.world_mut().insert_resource(RunRng(seed));
            {
                let mut session = app.world_mut().resource_mut::<GameSession>();
                session.gold = budget;
                session.level = 30;
                session.elapsed = 600.0;
                session.xp = 0;
            }
            tap_g(&mut app);
            for _ in 0..100 {
                if app.world().resource::<DraftState>().root.is_none() {
                    break;
                }
                assert_eq!(
                    app.world().resource::<DraftState>().kind,
                    Some(DraftKind::Skill)
                );
                // Focus the strongest offered skill; do not rely on card ordering.
                let build = app.world().resource::<PlayerBuild>();
                let choice = app
                    .world()
                    .resource::<DraftState>()
                    .choices
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, choice)| match choice {
                        DraftChoice::Skill(skill) => build.skill_rank(*skill),
                        _ => 0,
                    })
                    .unwrap()
                    .0;
                press_choice(&mut app, choice);
                draws += 1;
                app.update();
            }
            assert!(app.world().resource::<DraftState>().root.is_none());
            let ranks = app.world().resource::<PlayerBuild>().skills;
            let count = ranks
                .iter()
                .enumerate()
                .filter(|(index, rank)| **rank >= SKILLS[*index].max_rank)
                .count();
            mastered += count;
            full_mastery += usize::from(count >= 6);
            builds.insert(ranks);
        }
        println!(
            "DRAFT_AUDIT budget={budget} seeds=32 draws={draws} mastered={mastered} full_mastery={full_mastery} distinct_builds={}",
            builds.len()
        );
        assert_eq!(
            full_mastery, 0,
            "finite budget should require build tradeoffs"
        );
        assert!(builds.len() >= 24, "too few distinct seeded builds");
    }
}
