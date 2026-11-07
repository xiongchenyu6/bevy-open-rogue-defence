use super::*;
#[test]
fn crown_rank_six_evolution_preview_never_offers_a_downgrade() {
    use crate::game::{
        GameSession,
        market::{MarketEffects, MarketItem},
        skill_evolution::SkillEvolution,
    };
    for index in 0..SkillId::COUNT {
        let skill = SkillId(index as u8);
        let mut build = PlayerBuild::default();
        build.skills.fill(0);
        build.skills[index] = 5;
        let mut vitals = HeroVitals::default();
        MarketItem::AscendantCrown.apply(
            &mut GameSession::default(),
            &mut build,
            &mut vitals,
            &mut MarketEffects::default(),
        );
        assert_eq!(build.skill_rank(skill), 6, "actual Crown effect");
        let evolution = SkillEvolution {
            unlocked_stage: 1,
            ..Default::default()
        };
        assert!(evolution.branch_ready(skill, &build) || evolution.eligible(skill, &build));
        let runtime = SkillRuntime::default();
        let damage = skills::skill_preview_damage(index, 6, &build, &vitals, &runtime);
        for language in [GameLanguage::English, GameLanguage::SimplifiedChinese] {
            let short = simple(language, skill, 6);
            let full = detailed(language, skill, 6, &build, &vitals, &runtime);
            assert!(short.contains("6 → 6/6"), "{index}: {short}");
            assert!(
                full.lines().next().unwrap().contains("6 → 6/6"),
                "{index}: {full}"
            );
            assert!(!full.contains("6 → 5/5"));
            if let Some(value) = damage {
                assert!(
                    full.contains(&format!("{value:.1} → {value:.1}")),
                    "{index}: {full}"
                );
            }
        }
        assert_eq!(build.skill_rank(skill), 6);
    }
}

#[test]
fn base_rank_preview_advances_only_until_mastery_in_both_modes() {
    let build = PlayerBuild::default();
    let vitals = HeroVitals::default();
    let runtime = SkillRuntime::default();
    for index in 0..SkillId::COUNT {
        for language in [GameLanguage::English, GameLanguage::SimplifiedChinese] {
            for (rank, next) in [(0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (5, 5)] {
                let short = simple(language, SkillId(index as u8), rank);
                let full = detailed(
                    language,
                    SkillId(index as u8),
                    rank,
                    &build,
                    &vitals,
                    &runtime,
                );
                let progression = format!("{rank} → {next}/5");
                assert!(short.contains(&progression));
                assert!(full.lines().next().unwrap().contains(&progression));
                // A controlled snapshot for exact before/after text comparison,
                // in addition to the mastery progression assertions above.
                println!(
                    "BASE_RANK_DETAIL index={index} language={} rank={rank} texts=({short:?}, {full:?})",
                    language.pick("en", "zh")
                );
            }
        }
    }
}

#[test]
fn detailed_preview_uses_live_damage_cooldown_and_next_radius() {
    let build = PlayerBuild::default();
    let vitals = HeroVitals::default();
    let runtime = SkillRuntime::default();
    let detail = detailed(
        GameLanguage::English,
        SkillId(13),
        2,
        &build,
        &vitals,
        &runtime,
    );
    let a = skills::skill_preview_damage(13, 2, &build, &vitals, &runtime).unwrap();
    let b = skills::skill_preview_damage(13, 3, &build, &vitals, &runtime).unwrap();
    assert!(detail.contains(&format!("{a:.1} → {b:.1}")));
    assert!(detail.contains(&format!("Cooldown {:.1}s", skills::rank_cooldown(13, 2))));
    assert!(detail.contains("Effect radius 4.00 → 4.80 m"));
    assert!(detail.contains("spell power"));
    assert!(!simple(GameLanguage::English, SkillId(13), 2).contains("Formula:"));
}
#[test]
fn passive_and_infection_previews_do_not_invent_catalog_damage() {
    let build = PlayerBuild::default();
    let vitals = HeroVitals::default();
    let runtime = SkillRuntime::default();
    let prism = detailed(
        GameLanguage::English,
        SkillId(20),
        4,
        &build,
        &vitals,
        &runtime,
    );
    assert!(prism.contains("Copied spell damage 94% → Copied spell damage 100%"));
    assert!(!prism.contains("Damage ("));
    let infection = detailed(
        GameLanguage::SimplifiedChinese,
        SkillId(18),
        2,
        &build,
        &vitals,
        &runtime,
    );
    assert!(infection.contains("物理疫蚀每跳"));
    assert!(infection.contains("0.14 + 0.04"));
    let volley = detailed(
        GameLanguage::English,
        SkillId(7),
        2,
        &build,
        &vitals,
        &runtime,
    );
    assert!(volley.contains(&format!(
        "{:.1} → {:.1}",
        build.attack_damage * 0.6,
        build.attack_damage * 0.6
    )));
    assert!(volley.contains("Formula: 0.60 × attack damage"));
}
#[test]
fn all_skill_descriptions_are_localized_and_bounded_at_max_rank() {
    let build = PlayerBuild::default();
    let vitals = HeroVitals::default();
    let runtime = SkillRuntime::default();
    for index in 0..SkillId::COUNT {
        for language in [GameLanguage::English, GameLanguage::SimplifiedChinese] {
            let text = detailed(language, SkillId(index as u8), 5, &build, &vitals, &runtime);
            assert!(text.contains("5 → 5/5"));
            assert!(!text.contains("NaN") && !text.contains("infinity"));
            assert!(!text.is_empty());
        }
    }
}
