use super::*;
use bevy::prelude::*;

#[derive(Default)]
struct PreviewModifiers {
    bonds: BondRuntime,
    market: MarketEffects,
    encounters: EncounterState,
}

impl PreviewModifiers {
    fn context(&self) -> DamagePreviewContext<'_> {
        DamagePreviewContext {
            bonds: &self.bonds,
            market: &self.market,
            encounters: &self.encounters,
        }
    }
}

#[test]
fn crown_rank_six_evolution_preview_never_offers_a_downgrade() {
    let modifiers = PreviewModifiers::default();
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
            let full = detailed(
                language,
                skill,
                6,
                &build,
                &vitals,
                &runtime,
                modifiers.context(),
            );
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
    let modifiers = PreviewModifiers::default();
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
                    modifiers.context(),
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
    let modifiers = PreviewModifiers::default();
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
        modifiers.context(),
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
    let modifiers = PreviewModifiers::default();
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
        modifiers.context(),
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
        modifiers.context(),
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
        modifiers.context(),
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
    let modifiers = PreviewModifiers::default();
    let build = PlayerBuild::default();
    let vitals = HeroVitals::default();
    let runtime = SkillRuntime::default();
    for index in 0..SkillId::COUNT {
        for language in [GameLanguage::English, GameLanguage::SimplifiedChinese] {
            let text = detailed(
                language,
                SkillId(index as u8),
                5,
                &build,
                &vitals,
                &runtime,
                modifiers.context(),
            );
            assert!(text.contains("5 → 5/5"));
            assert!(!text.contains("NaN") && !text.contains("infinity"));
            assert!(!text.is_empty());
        }
    }
}

#[test]
fn adjusted_damage_uses_actual_crown_without_changing_the_base_formula() {
    use crate::game::{GameSession, market::MarketItem};
    let mut modifiers = PreviewModifiers::default();
    let mut build = PlayerBuild::default();
    build.skills.fill(0);
    build.skills[26] = 5;
    let mut vitals = HeroVitals::default();
    MarketItem::AscendantCrown.apply(
        &mut GameSession::default(),
        &mut build,
        &mut vitals,
        &mut modifiers.market,
    );
    assert_eq!(build.skills[26], 6);
    let runtime = SkillRuntime::default();
    let raw = skills::skill_preview_damage(26, 6, &build, &vitals, &runtime).unwrap();
    let adjusted = modifiers
        .context()
        .adjusted(SkillId(26), 6, raw, &build, &vitals)
        .unwrap();
    assert!((adjusted - raw * 1.30).abs() < 0.001);
    for language in [GameLanguage::English, GameLanguage::SimplifiedChinese] {
        let text = detailed(
            language,
            SkillId(26),
            6,
            &build,
            &vitals,
            &runtime,
            modifiers.context(),
        );
        assert!(text.contains(&format!("{raw:.1} → {raw:.1}")));
        assert!(text.contains(&format!("{adjusted:.1} → {adjusted:.1}")));
        assert!(text.contains(language.pick(
            "With current bonuses (non-critical)",
            "计入当前增伤（非暴击）"
        )));
        assert!(text.contains(language.pick("before target type, distance", "未计目标类型、距离")));
        assert!(!text.contains("Values include active power bonuses"));
        assert!(!text.contains("数值包含当前强度增幅"));
    }
    assert_eq!(
        skills::skill_preview_damage(26, 6, &build, &vitals, &runtime),
        Some(raw)
    );
    assert_eq!(build.skills[26], 6);
}

#[test]
fn adjusted_next_rank_uses_candidate_crown_owner_without_mutating_live_ranks() {
    use crate::game::{GameSession, market::MarketItem};
    let mut modifiers = PreviewModifiers::default();
    let mut build = PlayerBuild::default();
    build.skills.fill(0);
    build.skills[27] = 5;
    let mut vitals = HeroVitals::default();
    MarketItem::AscendantCrown.apply(
        &mut GameSession::default(),
        &mut build,
        &mut vitals,
        &mut modifiers.market,
    );
    build.skills[26] = 4;
    assert_eq!(build.skills[27], 6);
    assert_eq!(
        modifiers
            .context()
            .adjusted(SkillId(26), 5, 100.0, &build, &vitals),
        Some(100.0)
    );
    // A separately acquired Crown can leave27 at5 before26 catches up. The
    // actual stable lower-index tie break then changes the next-rank owner.
    let mut modifiers = PreviewModifiers::default();
    let mut build = PlayerBuild::default();
    build.skills.fill(0);
    build.skills[27] = 4;
    MarketItem::AscendantCrown.apply(
        &mut GameSession::default(),
        &mut build,
        &mut vitals,
        &mut modifiers.market,
    );
    build.skills[26] = 4;
    let original = build.skills;
    let context = modifiers.context();
    assert_eq!(
        context.adjusted(SkillId(26), 4, 100.0, &build, &vitals),
        Some(100.0)
    );
    assert!(
        (context
            .adjusted(SkillId(26), 5, 100.0, &build, &vitals)
            .unwrap()
            - 130.0)
            .abs()
            < 0.001
    );
    assert_eq!(build.skills, original);
}

#[test]
fn adjusted_damage_reads_the_actual_summon_bond_runtime_and_not_spell_or_attack_bonuses() {
    use crate::game::{BondCardId, GameSession, bonds::BondsPlugin};
    let mut app = App::new();
    app.init_resource::<Time>()
        .init_resource::<GameSession>()
        .init_resource::<PlayerBuild>()
        .init_resource::<HeroVitals>()
        .add_plugins(BondsPlugin);
    app.world_mut()
        .resource_mut::<PlayerBuild>()
        .owned_bond_cards[BondCardId::SpiritVessel.index()] = true;
    app.update();
    let market = MarketEffects::default();
    let encounters = EncounterState::default();
    let context = DamagePreviewContext {
        bonds: app.world().resource::<BondRuntime>(),
        market: &market,
        encounters: &encounters,
    };
    let build = app.world().resource::<PlayerBuild>();
    let vitals = app.world().resource::<HeroVitals>();
    assert!(
        (context
            .adjusted(SkillId(26), 5, 100.0, build, vitals)
            .unwrap()
            - 120.0)
            .abs()
            < 0.001
    );
    assert_eq!(
        context.adjusted(SkillId(0), 1, 100.0, build, vitals),
        Some(100.0)
    );
    assert_eq!(
        context.adjusted(SkillId(7), 1, 100.0, build, vitals),
        Some(100.0)
    );
}

#[test]
fn adjusted_damage_uses_current_encounter_and_health_but_never_rolls_random_damage() {
    use crate::game::{GameSession, RunRng, encounters::EncounterId, market::MarketItem};
    let mut modifiers = PreviewModifiers::default();
    let mut build = PlayerBuild::default();
    build.skills.fill(0);
    build.skills[26] = 5;
    let mut vitals = HeroVitals::default();
    let mut session = GameSession::default();
    let mut rng = RunRng::from_seed(0xC0126);
    assert!(modifiers.encounters.bank_for_boss(0));
    assert!(modifiers.encounters.open_hand(&mut rng, &build).is_some());
    assert!(modifiers.encounters.accept(
        EncounterId::BloodOath,
        &mut session,
        &mut build,
        &mut vitals,
        &mut rng
    ));
    MarketItem::ChaosDie.apply(&mut session, &mut build, &mut vitals, &mut modifiers.market);
    let before = rng.state();
    assert_eq!(modifiers.market.chaos_damage_bounds(), (0.7, 1.8));
    let adjusted = modifiers
        .context()
        .adjusted(SkillId(26), 5, 100.0, &build, &vitals)
        .unwrap();
    assert!(
        (adjusted - 112.0).abs() < 0.001,
        "random result is not a fixed card multiplier"
    );
    let runtime = SkillRuntime::default();
    let first = detailed(
        GameLanguage::English,
        SkillId(26),
        5,
        &build,
        &vitals,
        &runtime,
        modifiers.context(),
    );
    assert_eq!(
        first,
        detailed(
            GameLanguage::English,
            SkillId(26),
            5,
            &build,
            &vitals,
            &runtime,
            modifiers.context()
        )
    );
    assert_eq!(rng.state(), before);
    MarketItem::BloodContract.apply(&mut session, &mut build, &mut vitals, &mut modifiers.market);
    vitals.health = vitals.max_health * 0.4;
    let low = modifiers
        .context()
        .adjusted(SkillId(26), 5, 100.0, &build, &vitals)
        .unwrap();
    assert!((low - 112.0 * 1.4).abs() < 0.001);
    vitals.health = vitals.max_health;
    assert!(
        (modifiers
            .context()
            .adjusted(SkillId(26), 5, 100.0, &build, &vitals)
            .unwrap()
            - 112.0)
            .abs()
            < 0.001
    );
}

#[test]
fn adjusted_damage_never_fabricates_outgoing_values_for_utilities_or_fate_outcomes() {
    let modifiers = PreviewModifiers::default();
    let build = PlayerBuild::default();
    let vitals = HeroVitals::default();
    let runtime = SkillRuntime::default();
    for index in [20, 21, 28, 29] {
        assert_eq!(
            modifiers
                .context()
                .adjusted(SkillId(index), 5, 100.0, &build, &vitals),
            None
        );
        for language in [GameLanguage::English, GameLanguage::SimplifiedChinese] {
            let text = detailed(
                language,
                SkillId(index),
                5,
                &build,
                &vitals,
                &runtime,
                modifiers.context(),
            );
            assert!(!text.contains(language.pick("With current bonuses", "计入当前增伤")));
            if index == 29 {
                assert!(text.contains(language.pick("base formula only", "为基础公式")));
            }
        }
    }
}

#[test]
fn blood_wheel_previews_physical_outbound_and_flame_return_separately() {
    use crate::game::{GameSession, market::MarketItem};
    let mut modifiers = PreviewModifiers::default();
    let mut build = PlayerBuild::default();
    build.skills.fill(0);
    build.skills[23] = 4;
    build.attack_damage = 100.0;
    let mut vitals = HeroVitals::default();
    MarketItem::AscendantCrown.apply(
        &mut GameSession::default(),
        &mut build,
        &mut vitals,
        &mut modifiers.market,
    );
    let runtime = SkillRuntime::default();
    let raw = 100.0 * 0.9 * SkillId(23).definition().rarity.skill_power_multiplier();
    for rank in 1..=6 {
        assert_eq!(
            skills::skill_preview_damage(23, rank, &build, &vitals, &runtime),
            Some(raw)
        );
    }
    let context = modifiers.context();
    assert_eq!(
        context.adjusted(SkillId(23), 5, raw, &build, &vitals),
        Some(raw)
    );
    let return_damage = context.adjusted_profile(
        SkillId(23),
        5,
        raw,
        DamageProfile::for_skill(23).with_burn(),
        &build,
        &vitals,
    );
    assert!((return_damage - raw * 1.3).abs() < 0.001);
    for language in [GameLanguage::English, GameLanguage::SimplifiedChinese] {
        let text = detailed(language, SkillId(23), 5, &build, &vitals, &runtime, context);
        assert!(text.contains(language.pick("physical outbound hit", "物理出程每击")));
        assert!(text.contains(&format!("{return_damage:.1} → {return_damage:.1}")));
        assert!(text.contains(language.pick("Flame return hit", "火焰返程每击")));
    }
}

#[test]
fn rebirth_preview_uses_restored_health_before_its_actual_burst() {
    use crate::game::{GameSession, market::MarketItem};
    let mut modifiers = PreviewModifiers::default();
    let mut build = PlayerBuild::default();
    let mut vitals = HeroVitals::default();
    MarketItem::BloodContract.apply(
        &mut GameSession::default(),
        &mut build,
        &mut vitals,
        &mut modifiers.market,
    );
    vitals.health = vitals.max_health * 0.1;
    let original_health = vitals.health;
    let context = modifiers.context();
    assert!(
        (context
            .adjusted(SkillId(26), 1, 100.0, &build, &vitals)
            .unwrap()
            - 140.0)
            .abs()
            < 0.001
    );
    assert_eq!(
        context.adjusted(SkillId(25), 1, 100.0, &build, &vitals),
        Some(100.0)
    );
    assert!(skills::ember_rebirth_health(vitals.max_health) > vitals.max_health * 0.5);
    for language in [GameLanguage::English, GameLanguage::SimplifiedChinese] {
        let text = detailed(
            language,
            SkillId(25),
            1,
            &build,
            &vitals,
            &SkillRuntime::default(),
            context,
        );
        assert!(text.contains(language.pick("After rebirth bonuses", "返生后增伤")));
        assert!(text.contains("50.8%"));
    }
    assert_eq!(vitals.health, original_health);
}
