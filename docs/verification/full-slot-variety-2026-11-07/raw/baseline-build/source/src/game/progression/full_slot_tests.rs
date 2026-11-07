use super::*;

fn full_bar(seed: u64, capped: bool) -> App {
    let mut app = tests::test_app();
    app.insert_resource(RunRng::from_seed(seed));
    {
        let mut build = app.world_mut().resource_mut::<PlayerBuild>();
        build.skills.fill(0);
        for index in 0..SKILL_SLOT_LIMIT {
            build.skills[index] = if capped { SkillId(index as u8).definition().max_rank } else { 1 };
        }
    }
    {
        let mut session = app.world_mut().resource_mut::<GameSession>();
        session.level = 8;
        session.elapsed = 100.0;
        session.gold = 500;
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
        let count = draft.choices.iter().filter(|choice| matches!(choice, DraftChoice::ReplaceSkill { .. })).count();
        assert!(count <= 1);
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
    assert!(replacements > 0, "full slots must retain an optional new-skill path before UR unlock");
}

#[test]
fn full_slot_normal_g_with_capped_owned_skills_still_opens_one_swap() {
    let mut app = full_bar(0xA11CE, true);
    tests::tap_g(&mut app);
    let draft = app.world().resource::<DraftState>();
    println!("FULL_SLOT_CAPPED choices={:?}", draft.choices);
    assert_eq!(draft.kind, Some(DraftKind::Skill));
    assert_eq!(draft.choices.len(), 1);
    assert!(matches!(draft.choices[0], DraftChoice::ReplaceSkill { .. }));
}
