from pathlib import Path
p=Path('src/game/progression_ui.rs');s=p.read_text();old='''                (DraftChoice::ReplaceSkill { new, .. }, true, Some(vitals), Some(runtime)) => {
                    format!(''';new='''                (DraftChoice::ReplaceSkill { old, new }, true, Some(vitals), Some(runtime)) => {
                    let mut replacement_build = (*build).clone();
                    replacement_build.skills[usize::from(old.0)] = 0;
                    format!(''';assert old in s;s=s.replace(old,new);needle='''                            new,
                            0,
                            &build,''';assert needle in s;s=s.replace(needle,'''                            new,
                            0,
                            &replacement_build,''');i=s.index('    #[test]\n    fn simple_mode_also_shortens');s=s[:i]+'''    #[test]
    fn replacement_details_transfer_crown_bonus_without_mutating_the_live_build() {
        use crate::game::{HeroVitals, market::{MarketEffects, MarketItem}, skills::SkillRuntime};
        let mut build = PlayerBuild::default();
        build.skills.fill(0);
        build.skills[0] = 5;
        for index in 27..=31 { build.skills[index] = 1; }
        let mut session = GameSession::default();
        let mut vitals = HeroVitals::default();
        let mut market = MarketEffects::default();
        MarketItem::AscendantCrown.apply(&mut session, &mut build, &mut vitals, &mut market);
        let original = build.skills;
        let runtime = SkillRuntime::default();
        let raw = crate::game::skills::skill_preview_damage(26, 1, &build, &vitals, &runtime).unwrap();
        let expected = format!("{:.1} → {:.1}", raw * 1.3, raw * 1.3);
        let mut app = App::new();
        app.insert_resource(build).insert_resource(session).insert_resource(vitals)
            .insert_resource(runtime).insert_resource(market)
            .init_resource::<crate::game::bonds::BondRuntime>()
            .init_resource::<crate::game::encounters::EncounterState>()
            .insert_resource(GameLanguage::English)
            .add_systems(Update, update_feedback);
        let root = app.world_mut().spawn_empty().id();
        app.insert_resource(DraftState {
            root: Some(root),
            choices: vec![DraftChoice::ReplaceSkill { old: SkillId(0), new: SkillId(26) }],
            ..default()
        }).insert_resource(BuildPreview { root: Some(root), index: Some(0) });
        let detail = app.world_mut().spawn((BuildDetail, Text::default())).id();
        for language in [GameLanguage::English, GameLanguage::SimplifiedChinese] {
            app.insert_resource(language);
            app.update();
            let text = &app.world().get::<Text>(detail).unwrap().0;
            assert!(text.contains(&expected), "{text}");
            assert_eq!(app.world().resource::<PlayerBuild>().skills, original);
        }
    }

'''+s[i:];p.write_text(s)
