use super::*;

fn full_bar(seed: u64, capped: bool) -> App {
    let mut app = tests::test_app();
    app.insert_resource(RunRng::from_seed(seed));
    {
        let mut build = app.world_mut().resource_mut::<PlayerBuild>();
        build.skills.fill(0);
        for index in 0..SKILL_SLOT_LIMIT {
            build.skills[index] = if capped {
                SkillId(index as u8).definition().max_rank
            } else {
                1
            };
        }
    }
    {
        let mut session = app.world_mut().resource_mut::<GameSession>();
        session.level = 8;
        session.elapsed = 100.0;
        session.gold = 80;
        session.skill_points = 0;
    }
    {
        let mut economy = app.world_mut().resource_mut::<SkillEconomy>();
        economy.first_free_granted = true;
        economy.level_grants = 1;
    }
    app
}

#[test]
fn full_slot_normal_g_can_offer_unlearned_unlocked_skill() {
    let mut replacements = 0;
    for seed in 0..128 {
        let mut app = full_bar(seed, false);
        tests::tap_g(&mut app);
        let draft = app.world().resource::<DraftState>();
        assert_eq!(draft.kind, Some(DraftKind::Skill));
        let count = draft
            .choices
            .iter()
            .filter(|choice| matches!(choice, DraftChoice::ReplaceSkill { .. }))
            .count();
        assert!(count <= 3);
        let new_ids: Vec<_> = draft
            .choices
            .iter()
            .filter_map(|c| {
                if let DraftChoice::ReplaceSkill { new, .. } = c {
                    Some(*new)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(
            new_ids
                .iter()
                .map(|id| id.0)
                .collect::<std::collections::HashSet<_>>()
                .len(),
            new_ids.len()
        );
        for choice in &draft.choices {
            if let DraftChoice::ReplaceSkill { old, new } = choice {
                assert!(app.world().resource::<PlayerBuild>().skill_rank(*old) > 0);
                assert_eq!(app.world().resource::<PlayerBuild>().skill_rank(*new), 0);
                assert!(matches!(new.definition().rarity, Rarity::Sr | Rarity::Ssr));
            }
        }
        replacements += count;
    }
    println!("FULL_SLOT_NORMAL_G seeds=128 replacements={replacements}");
    assert!(
        replacements > 0,
        "full slots must retain an optional new-skill path before UR unlock"
    );
}

#[test]
fn full_slot_normal_g_with_capped_owned_skills_still_opens_distinct_swaps() {
    let mut app = full_bar(0xA11CE, true);
    tests::tap_g(&mut app);
    let draft = app.world().resource::<DraftState>();
    println!("FULL_SLOT_CAPPED choices={:?}", draft.choices);
    assert_eq!(draft.kind, Some(DraftKind::Skill));
    assert_eq!(draft.choices.len(), 3);
    assert!(
        draft
            .choices
            .iter()
            .all(|c| matches!(c, DraftChoice::ReplaceSkill { .. }))
    );
    assert!(matches!(draft.choices[0], DraftChoice::ReplaceSkill { .. }));
}

fn swap_hand() -> (App, usize, SkillId, SkillId) {
    swap_hand_with_training(false)
}

fn swap_hand_with_training(earned: bool) -> (App, usize, SkillId, SkillId) {
    for seed in 0..128 {
        let mut app = full_bar(seed, false);
        if earned {
            app.world_mut().resource_mut::<PlayerBuild>().skills.fill(0);
            app.world_mut()
                .resource_mut::<SkillEvolution>()
                .rewarded_ranks
                .fill(0);
            app.world_mut()
                .resource_scope(|world, mut session: Mut<GameSession>| {
                    world.resource_scope(|world, mut build: Mut<PlayerBuild>| {
                        let mut evolution = world.resource_mut::<SkillEvolution>();
                        for id in 0..6 {
                            for _ in 0..4 {
                                grant_skill_rank(
                                    SkillId(id),
                                    &mut session,
                                    &mut build,
                                    &mut evolution,
                                );
                            }
                        }
                    });
                });
        }
        tests::tap_g(&mut app);
        let found = app
            .world()
            .resource::<DraftState>()
            .choices
            .iter()
            .enumerate()
            .find_map(|(i, c)| {
                if let DraftChoice::ReplaceSkill { old, new } = c {
                    Some((i, *old, *new))
                } else {
                    None
                }
            });
        if let Some((i, old, new)) = found {
            return (app, i, old, new);
        }
    }
    panic!("no normal-G replacement across128seeds");
}
fn stats(app: &App) -> (f32, f32, f32, u8, u8, f32) {
    let b = app.world().resource::<PlayerBuild>();
    (
        b.attack_damage,
        b.attacks_per_second,
        b.projectile_speed,
        b.multishot,
        b.pierce,
        app.world().resource::<GameSession>().core_max_hp,
    )
}
#[test]
fn full_slot_paid_ui_swap_resets_rank_preserves_training_and_prevents_retraining() {
    let (mut app, i, old, new) = swap_hand_with_training(true);
    assert_eq!(app.world().resource::<GameSession>().gold, 0);
    assert_eq!(app.world().resource::<GameSession>().skill_points, 0);
    assert_eq!(app.world().resource::<SkillEconomy>().paid_draws, 1);
    assert_eq!(
        app.world().resource::<SkillEconomy>().pending_payment,
        Some(SkillDrawPayment::Gold(80))
    );
    let before = stats(&app);
    assert!(before.0 > PlayerBuild::default().attack_damage);
    println!(
        "FULL_SLOT_EARNED_TRAINING before={before:?} old={old:?} rank={}",
        app.world().resource::<PlayerBuild>().skill_rank(old)
    );
    let old_rank = app.world().resource::<PlayerBuild>().skill_rank(old);
    app.world_mut().resource_mut::<SkillEvolution>().tiers[usize::from(old.0)] = 2;
    tests::press_choice(&mut app, i);
    assert_eq!(app.world().resource::<PlayerBuild>().skill_rank(old), 0);
    assert_eq!(app.world().resource::<PlayerBuild>().skill_rank(new), 1);
    assert_eq!(active_skill_slots(app.world().resource::<PlayerBuild>()), 6);
    assert_eq!(stats(&app), before);
    let evolution = app.world().resource::<SkillEvolution>();
    assert_eq!(evolution.tiers[usize::from(old.0)], 0);
    assert!(evolution.paths[usize::from(old.0)].is_none());
    assert_eq!(evolution.rewarded_ranks[usize::from(old.0)], old_rank);
    assert!(evolution.rewarded_ranks[usize::from(new.0)] >= 1);
    assert!(
        app.world()
            .resource::<SkillEconomy>()
            .pending_payment
            .is_none()
    );
    assert!(app.world().resource::<DraftState>().root.is_none());
    // Production rank-grant entry cannot pay earned rank training a second time.
    app.world_mut()
        .resource_scope(|world, mut session: Mut<GameSession>| {
            world.resource_scope(|world, mut build: Mut<PlayerBuild>| {
                let mut evolution = world.resource_mut::<SkillEvolution>();
                for _ in 0..old_rank {
                    grant_skill_rank(old, &mut session, &mut build, &mut evolution);
                }
            });
        });
    assert_eq!(stats(&app), before);
    println!("FULL_SLOT_EARNED_TRAINING after_relearn={:?}", stats(&app));
}
#[test]
fn full_slot_swap_later_preserves_the_same_paid_hand() {
    let (mut app, _, _, _) = swap_hand();
    let choices = app.world().resource::<DraftState>().choices.clone();
    let entity = app
        .world_mut()
        .query::<(Entity, &DraftActionButton)>()
        .iter(app.world())
        .find(|(_, a)| a.0 == DraftAction::Later)
        .unwrap()
        .0;
    app.world_mut()
        .entity_mut(entity)
        .insert(Interaction::Pressed);
    app.update();
    assert!(app.world().resource::<DraftState>().root.is_none());
    tests::tap_g(&mut app);
    assert_eq!(app.world().resource::<DraftState>().choices, choices);
    assert_eq!(app.world().resource::<GameSession>().gold, 0);
    assert_eq!(app.world().resource::<SkillEconomy>().paid_draws, 1);
}
#[test]
fn full_slot_swap_commit_rechecks_unlock_full_bar_and_new_ownership() {
    for invalid in 0..3 {
        let (mut app, i, old, new) = swap_hand();
        match invalid {
            0 => app.world_mut().resource_mut::<GameSession>().level = 2,
            1 => app.world_mut().resource_mut::<PlayerBuild>().skills[usize::from(old.0)] = 0,
            _ => app.world_mut().resource_mut::<PlayerBuild>().skills[usize::from(new.0)] = 1,
        }
        let ranks = app.world().resource::<PlayerBuild>().skills;
        tests::press_choice(&mut app, i);
        assert_eq!(app.world().resource::<PlayerBuild>().skills, ranks);
        assert!(app.world().resource::<DraftState>().root.is_some());
        assert_eq!(
            app.world().resource::<SkillEconomy>().pending_payment,
            Some(SkillDrawPayment::Gold(80))
        );
    }
}
#[test]
fn full_slot_sampler_seed_replay_stage_gates_and_last_upgrades_are_optional() {
    for (level, elapsed) in [
        (2, 100.0),
        (3, 100.0),
        (7, 719.9),
        (8, 719.9),
        (8, 720.0),
        (30, 1500.0),
    ] {
        for remaining in [0, 1, 2] {
            let app = full_bar(0, true);
            let mut build = app.world().resource::<PlayerBuild>().clone();
            for rank in &mut build.skills[..remaining] {
                *rank = 1;
            }
            let mut missing = 0;
            let mut swaps = 0;
            let mut ur = 0;
            for seed in 0..256 {
                let mut rng = RunRng::from_seed(seed);
                let mut again = RunRng::from_seed(seed);
                let a = pick_skill_choices(
                    &mut rng,
                    level,
                    elapsed,
                    &build,
                    &SelectedHero::default(),
                    &mut DraftAntiDrought::default(),
                    6,
                );
                let b = pick_skill_choices(
                    &mut again,
                    level,
                    elapsed,
                    &build,
                    &SelectedHero::default(),
                    &mut DraftAntiDrought::default(),
                    6,
                );
                assert_eq!(a, b);
                assert_eq!(rng.0, again.0);
                if (0..remaining).any(|i| !a.contains(&DraftChoice::Skill(SkillId(i as u8)))) {
                    missing += 1;
                }
                let mut ids = std::collections::HashSet::new();
                let mut hand_ur = 0;
                for choice in &a {
                    if let DraftChoice::ReplaceSkill { old, new } = choice {
                        assert!(build.skill_rank(*old) > 0);
                        assert!(ids.insert(new.0));
                        assert!(replacement_skill_available(*new, level, elapsed, &build));
                        swaps += 1;
                        if new.definition().rarity == Rarity::Ur {
                            hand_ur += 1;
                            ur += 1;
                        }
                    }
                }
                assert!(hand_ur <= 1);
                println!(
                    "FULL_SLOT_SAMPLE level={level} elapsed={elapsed} remaining={remaining} seed={seed} choices={a:?} rng={}",
                    rng.0
                );
            }
            if level >= 3 {
                assert!(swaps > 0);
                if remaining > 0 {
                    assert!(missing > 0, "last upgrades remain guaranteed");
                }
            }
            if elapsed < EPIC_SKILL_UNLOCK_AT {
                assert_eq!(ur, 0);
            }
            println!(
                "FULL_SLOT_DISTRIBUTION level={level} elapsed={elapsed} remaining={remaining} seeds=256 swaps={swaps} missing_owned={missing} ur={ur}"
            );
        }
    }
}

#[test]
fn full_slot_native_seed_catalog_uses_actual_sampler() {
    let mut build = PlayerBuild::default();
    build.skills.fill(0);
    build.skills[..6].fill(4);
    for rarity in [Rarity::Sr, Rarity::Ssr] {
        let mut found = false;
        for seed in 0..256 {
            let choices = pick_skill_choices(
                &mut RunRng::from_seed(seed),
                8,
                100.,
                &build,
                &SelectedHero::default(),
                &mut DraftAntiDrought::default(),
                6,
            );
            if choices.iter().any(|c|matches!(c,DraftChoice::ReplaceSkill{new,..} if new.definition().rarity==rarity)) {
                println!("FULL_SLOT_NATIVE_SEED rarity={rarity:?} seed={seed}");found=true;break;
            }
        }
        assert!(found);
    }
}
