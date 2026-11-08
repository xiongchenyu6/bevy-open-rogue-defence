// These checks exercise the registered production Update schedule, including InputPlugin.
fn decision_phase_key(app: &mut App, code: KeyCode, text: &str, state: bevy::input::ButtonState) {
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<Window>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .write_message(bevy::input::keyboard::KeyboardInput {
            key_code: code,
            logical_key: bevy::input::keyboard::Key::Character(text.into()),
            state,
            text: Some(text.into()),
            repeat: false,
            window,
        });
}

fn decision_phase_app() -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: format!("{}/assets", env!("CARGO_MANIFEST_DIR")),
                ..default()
            })
            .set(bevy::window::WindowPlugin {
                primary_window: Some(Window::default()),
                exit_condition: bevy::window::ExitCondition::DontExit,
                ..default()
            })
            .disable::<bevy::log::LogPlugin>()
            .disable::<bevy::app::TerminalCtrlCHandlerPlugin>()
            .disable::<bevy::render::RenderPlugin>()
            .disable::<bevy::winit::WinitPlugin>()
            .disable::<bevy::audio::AudioPlugin>()
            .disable::<bevy::core_pipeline::CorePipelinePlugin>()
            .disable::<bevy::post_process::PostProcessPlugin>()
            .disable::<bevy::anti_alias::AntiAliasPlugin>()
            .disable::<bevy::sprite_render::SpriteRenderPlugin>()
            .disable::<bevy::ui_render::UiRenderPlugin>()
            .disable::<bevy::pbr::PbrPlugin>()
            .disable::<bevy::gizmos_render::GizmoRenderPlugin>()
            .disable::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>(),
    )
    .init_asset::<StandardMaterial>()
    // RenderPlugin normally registers this loader in TexturePlugin::finish.
    .insert_resource(bevy::image::CompressedImageFormatSupport(
        bevy::image::CompressedImageFormats::NONE,
    ))
    .register_asset_loader(bevy::image::ImageLoader::new(
        bevy::image::CompressedImageFormats::NONE,
    ))
    .init_asset::<bevy::audio::AudioSource>()
    .init_asset_loader::<bevy::audio::AudioLoader>()
    .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ))
    .insert_resource(super::super::RunSeed(0xA11CE))
    .add_plugins((
        super::super::GamePlugin,
        super::super::AssetReadinessPlugin,
        super::super::OnboardingPlugin,
    ));
    app.world_mut()
        .resource_mut::<super::super::CampaignMenu>()
        .0 = false;
    app.world_mut()
        .resource_mut::<super::super::OnboardingState>()
        .dismissed = true;
    app.finish();
    app.cleanup();

    let started = std::time::Instant::now();
    loop {
        app.update();
        if app.world().resource::<super::super::AssetReadiness>().ready {
            break;
        }
        let gate = app.world().resource::<super::super::AssetReadiness>();
        assert!(
            started.elapsed().as_secs() < 20,
            "real assets did not become ready: loaded={} total={} failed={}",
            gate.loaded,
            gate.total,
            gate.failed
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    // Production-schedule tests settle artificial runs in memory.
    app.insert_resource(super::super::CampaignProgress::default());
    app.insert_resource(super::super::CampaignReward::default());
    decision_phase_key(
        &mut app,
        KeyCode::Digit1,
        "1",
        bevy::input::ButtonState::Pressed,
    );
    app.update();
    assert!(app.world().resource::<super::super::SelectedHero>().locked);
    decision_phase_key(
        &mut app,
        KeyCode::Digit1,
        "1",
        bevy::input::ButtonState::Released,
    );
    app.world_mut().resource_mut::<SpawnDirector>().enabled = false;
    app.world_mut().resource_mut::<AttackClock>().0.pause();
    app.world_mut().resource_mut::<ActiveSkill>().remaining = 10_000.;
    app.world_mut()
        .run_system_once(|mut commands: Commands, mut pools: ResMut<CombatPools>| {
            pools.retire_all_projectiles(&mut commands);
        })
        .unwrap();
    app
}

fn prepare_decision_phase(app: &mut App, level: u32) {
    {
        let mut session = app.world_mut().resource_mut::<GameSession>();
        session.level = level;
        session.xp = 0;
        session.xp_next = 60 + 18 * (level - 1);
        session.elapsed = 100.;
        session.gold = 80;
        session.skill_points = 0;
        session.paused = false;
        session.finished = false;
    }
    {
        let mut economy = app.world_mut().resource_mut::<super::super::SkillEconomy>();
        economy.first_free_granted = true;
        let claimed = [5, 10, 15, 20, 25]
            .into_iter()
            .filter(|required| *required <= level)
            .count();
        economy.level_grants = (1_u8 << claimed) - 1;
    }
    {
        let claimed = [10, 20, 40]
            .into_iter()
            .filter(|required| *required <= level)
            .count();
        let mut recruitment = app
            .world_mut()
            .resource_mut::<super::super::RecruitmentProgress>();
        recruitment.claimed_milestones = (1_u8 << claimed) - 1;
        recruitment.pending = None;
    }
    {
        let mut build = app.world_mut().resource_mut::<PlayerBuild>();
        build.skills.fill(0);
        build.skills[..6].fill(4);
    }
    // Synchronize the controlled level through the real growth and bond systems.
    app.update();
    app.insert_resource(RunRng::from_seed(0xA11CE));
    assert!(!app.world().resource::<GameSession>().paused);
}

#[test]
fn production_decision_phase_g_pauses_before_actual_contact() {
    let mut app = decision_phase_app();
    prepare_decision_phase(&mut app, 8);
    let position = Vec3::new(10., 0., 0.);
    let mut enemy = test_enemy(1000.);
    enemy.elite = false;
    let target = app
        .world_mut()
        .spawn((enemy, Transform::from_translation(position)))
        .id();
    app.world_mut()
        .run_system_once(
            move |mut commands: Commands, mut pools: ResMut<CombatPools>| {
                let projectile = pools
                    .rent_projectile(
                        &mut commands,
                        Handle::default(),
                        Handle::default(),
                        Transform::from_translation(position),
                        Projectile {
                            velocity: Vec3::ZERO,
                            damage: 25.,
                            radius: 0.1,
                            remaining_hits: 0,
                            lifetime: 2.,
                        },
                    )
                    .unwrap();
                commands
                    .entity(projectile)
                    .insert(DamageProfile::PHYSICAL_ATTACK);
            },
        )
        .unwrap();
    decision_phase_key(
        &mut app,
        KeyCode::KeyG,
        "g",
        bevy::input::ButtonState::Pressed,
    );
    app.update();
    let health = app.world().get::<Enemy>(target).unwrap().hp;
    let session = app.world().resource::<GameSession>();
    let draft = app.world().resource::<super::super::DraftState>();
    println!(
        "PRODUCTION_DECISION_CONTACT hp={health} paused={} gold={} choices={:?}",
        session.paused, session.gold, draft.choices
    );
    assert!(draft.root.is_some(), "actual G must open a paid hand");
    assert!(session.paused);
    assert_eq!(session.gold, 0);
    assert_eq!(
        app.world()
            .resource::<super::super::SkillEconomy>()
            .paid_draws,
        1
    );
    assert_eq!(
        health, 1000.,
        "opening the hand must precede battle contact"
    );
    assert_eq!(
        app.world_mut()
            .query::<&Projectile>()
            .iter(app.world())
            .count(),
        1
    );
}

#[test]
fn production_decision_phase_milestones_precede_g_including_kill_healing() {
    for (level, needs_healing) in [(9, false), (19, false), (39, false), (9, true)] {
        let mut app = decision_phase_app();
        prepare_decision_phase(&mut app, level);
        if needs_healing {
            use super::super::BondCardId;
            {
                let mut build = app.world_mut().resource_mut::<PlayerBuild>();
                for card in [
                    BondCardId::WarpathCleave,
                    BondCardId::WarpathRecovery,
                    BondCardId::WarpathFury,
                ] {
                    build.owned_bond_cards[card.index()] = true;
                }
            }
            // Acquire the effects before earning kills, as the production rule requires.
            app.update();
            app.world_mut().resource_mut::<HeroVitals>().health = 1.;
        }
        let before_vitals = app.world().resource::<HeroVitals>();
        let health_before = before_vitals.health;
        let max_health_before = before_vitals.max_health;
        {
            let mut session = app.world_mut().resource_mut::<GameSession>();
            session.xp = session.xp_next;
            if needs_healing {
                session.kills = 3;
                session.total_kills = 3;
            }
        }
        decision_phase_key(
            &mut app,
            KeyCode::KeyG,
            "g",
            bevy::input::ButtonState::Pressed,
        );
        app.update();
        let session = app.world().resource::<GameSession>();
        let pending = app
            .world()
            .resource::<super::super::RecruitmentProgress>()
            .pending;
        println!(
            "PRODUCTION_DECISION_MILESTONE old_level={level} healing={needs_healing} new_level={} pending={pending:?} gold={} health={}",
            session.level,
            session.gold,
            app.world().resource::<HeroVitals>().health
        );
        assert_eq!(session.level, level + 1);
        assert_eq!(
            pending,
            Some(super::super::RecruitmentSource::Milestone(level + 1))
        );
        assert!(session.paused);
        assert_eq!(session.gold, 80);
        assert!(
            app.world()
                .resource::<super::super::DraftState>()
                .root
                .is_none()
        );
        assert_eq!(
            app.world()
                .resource::<super::super::SkillEconomy>()
                .paid_draws,
            0
        );
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<super::super::hero::HeroChoiceButton>>()
                .iter(app.world())
                .count(),
            6
        );
        if needs_healing {
            let vitals = app.world().resource::<HeroVitals>();
            assert!(
                vitals.health > health_before + vitals.max_health - max_health_before,
                "kill healing must recover damage in addition to new growth capacity"
            );
            assert_eq!(
                app.world_mut()
                    .resource_mut::<BondRuntime>()
                    .take_kill_healing(),
                0.
            );
        }
    }
}

fn decision_phase_stock_draw(separate_stock_update: bool) -> (Vec<super::super::DraftChoice>, u64) {
    let mut app = decision_phase_app();
    prepare_decision_phase(&mut app, 8);
    app.world_mut().resource_mut::<GameSession>().elapsed = 361.;
    if separate_stock_update {
        app.world_mut().resource_mut::<GameSession>().gold = 0;
        app.update();
        assert_eq!(super::super::market::test_stock_generation(app.world()), 1);
        app.world_mut().resource_mut::<GameSession>().gold = 80;
    }
    decision_phase_key(
        &mut app,
        KeyCode::KeyG,
        "g",
        bevy::input::ButtonState::Pressed,
    );
    app.update();
    assert_eq!(super::super::market::test_stock_generation(app.world()), 1);
    assert_eq!(app.world().resource::<GameSession>().gold, 0);
    assert_eq!(
        app.world()
            .resource::<super::super::SkillEconomy>()
            .paid_draws,
        1
    );
    assert!(app.world().resource::<GameSession>().paused);
    assert!(
        app.world()
            .resource::<super::super::DraftState>()
            .root
            .is_some()
    );
    let result = (
        app.world()
            .resource::<super::super::DraftState>()
            .choices
            .clone(),
        app.world().resource::<RunRng>().state(),
    );
    println!("PRODUCTION_DECISION_STOCK separate_update={separate_stock_update} result={result:?}");
    result
}

#[test]
fn production_decision_phase_stock_due_and_g_matches_stock_then_g() {
    let stock_then_g = decision_phase_stock_draw(true);
    let same_update = decision_phase_stock_draw(false);
    assert_eq!(same_update, stock_then_g);
}

#[test]
fn production_decision_phase_recruitment_refreshes_capacity_before_continued_g() {
    let mut app = decision_phase_app();
    prepare_decision_phase(&mut app, 9);
    app.world_mut().resource_mut::<GameSession>().skill_points = 2;
    decision_phase_key(
        &mut app,
        KeyCode::KeyG,
        "g",
        bevy::input::ButtonState::Pressed,
    );
    app.update();
    assert!(
        app.world()
            .resource::<super::super::DraftState>()
            .root
            .is_some()
    );
    decision_phase_key(
        &mut app,
        KeyCode::Digit1,
        "1",
        bevy::input::ButtonState::Pressed,
    );
    app.update();
    assert!(
        app.world()
            .resource::<super::super::DraftState>()
            .root
            .is_none()
    );
    decision_phase_key(
        &mut app,
        KeyCode::KeyG,
        "g",
        bevy::input::ButtonState::Released,
    );
    decision_phase_key(
        &mut app,
        KeyCode::Digit1,
        "1",
        bevy::input::ButtonState::Released,
    );
    {
        let mut session = app.world_mut().resource_mut::<GameSession>();
        session.xp = session.xp_next;
    }
    app.update();
    assert_eq!(
        app.world()
            .resource::<super::super::RecruitmentProgress>()
            .pending,
        Some(super::super::RecruitmentSource::Milestone(10))
    );
    let started = std::time::Instant::now();
    loop {
        app.update();
        let gate = app.world().resource::<super::super::AssetReadiness>();
        if gate.ready && gate.heroes.len() == 6 {
            break;
        }
        assert!(
            started.elapsed().as_secs() < 20,
            "recruitment assets did not become ready"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let incoming = app
        .world_mut()
        .query::<(&super::super::hero::HeroChoiceButton, &Name)>()
        .iter(app.world())
        .find(|(_, name)| name.as_str() == "Commander card 0")
        .unwrap()
        .0
        .kind();
    // Existing learned skills can include every possible starting skill of the new hero.
    let mut owned: Vec<usize> = super::super::content::HEROES[incoming.index()]
        .starting_skill_pool
        .iter()
        .map(|id| {
            super::super::content::SKILLS
                .iter()
                .position(|skill| skill.id == *id)
                .unwrap()
        })
        .collect();
    for index in 0..super::super::SkillId::COUNT {
        if owned.len() == 6 {
            break;
        }
        if !owned.contains(&index) {
            owned.push(index);
        }
    }
    {
        let mut build = app.world_mut().resource_mut::<PlayerBuild>();
        build.skills.fill(0);
        for index in owned {
            build.skills[index] = 1;
        }
    }
    decision_phase_key(
        &mut app,
        KeyCode::Digit1,
        "1",
        bevy::input::ButtonState::Pressed,
    );
    app.update();
    let capacity = app
        .world()
        .resource::<super::super::skill_evolution::SkillEvolution>()
        .skill_capacity();
    let fusion_capacity = app
        .world()
        .resource::<super::super::HeroFusion>()
        .skill_capacity();
    let slots = app
        .world()
        .resource::<PlayerBuild>()
        .skills
        .iter()
        .filter(|rank| **rank > 0)
        .count();
    let draft = app.world().resource::<super::super::DraftState>();
    println!(
        "PRODUCTION_DECISION_RECRUIT_RESUME slots={slots} capacity={capacity} fusion_capacity={fusion_capacity} choices={:?}",
        draft.choices
    );
    assert!(
        app.world()
            .resource::<super::super::RecruitmentProgress>()
            .pending
            .is_none()
    );
    assert!(
        draft.root.is_some(),
        "the queued skill hand must resume after real recruitment"
    );
    assert_eq!(fusion_capacity, 7);
    assert_eq!(slots, 6);
    assert_eq!(
        capacity, fusion_capacity,
        "the same-update draw must see the new skill slot"
    );
    assert!(
        draft
            .choices
            .iter()
            .all(|choice| matches!(choice, super::super::DraftChoice::Skill(_)))
    );
}

fn fatal_boss_regeneration_outcome(
    regeneration: bool,
    prior_deaths: u32,
    revival_seed: bool,
) -> (u32, bool, bool) {
    let mut app = decision_phase_app();
    prepare_decision_phase(&mut app, 8);
    if regeneration {
        app.world_mut()
            .resource_mut::<PlayerBuild>()
            .owned_bond_cards[super::super::BondCardId::VitalSpring.index()] = true;
    }
    app.update();
    {
        let mut session = app.world_mut().resource_mut::<GameSession>();
        session.core_hp = 0.;
        session.gate_revive_remaining = 100.;
    }
    {
        let mut vitals = app.world_mut().resource_mut::<HeroVitals>();
        vitals.health = 10.;
        vitals.deaths = prior_deaths;
        vitals.invulnerable_remaining = 0.;
    }
    let hero_position = app
        .world_mut()
        .query_filtered::<&Transform, With<Hero>>()
        .single(app.world())
        .unwrap()
        .translation;
    let mut enemy = test_enemy(1_000_000.);
    enemy.speed = 0.;
    enemy.damage = 0.;
    let boss = app
        .world_mut()
        .spawn((
            enemy,
            super::super::BossEnemy { index: 2 },
            Transform::from_xyz(30., 0., 0.),
        ))
        .id();
    if revival_seed {
        app.insert_resource(MarketEffects::with_revival_seed_for_test());
    }
    super::super::bosses::test_release_fatal_circle(app.world_mut(), boss, hero_position);
    app.update();
    let hit = app.world().resource::<HeroVitals>();
    assert_eq!(
        hit.health, 0.,
        "real boss circle must inflict lethal damage"
    );
    assert!(hit.alive, "boss damage precedes the next survival phase");
    assert_eq!(hit.deaths, prior_deaths);
    app.update();
    let vitals = app.world().resource::<HeroVitals>();
    let session = app.world().resource::<GameSession>();
    if revival_seed {
        assert!(vitals.health > vitals.max_health * 0.3);
        assert!(vitals.invulnerable_remaining > 0.0);
    }
    println!(
        "FATAL_REGEN regeneration={regeneration} prior_deaths={prior_deaths} revival_seed={revival_seed} health={} alive={} deaths={} finished={} victory={}",
        vitals.health, vitals.alive, vitals.deaths, session.finished, session.victory
    );
    assert!(!session.victory);
    (vitals.deaths, vitals.alive, session.finished)
}

#[test]
fn production_fatal_boss_damage_consumes_life_despite_ordinary_regeneration() {
    let outcomes = [(false, 0), (false, 2), (true, 0), (true, 2)]
        .map(|(regeneration, deaths)| fatal_boss_regeneration_outcome(regeneration, deaths, false));
    assert_eq!(
        outcomes,
        [
            (1, false, false),
            (3, false, true),
            (1, false, false),
            (3, false, true)
        ]
    );
}

#[test]
fn production_fatal_boss_damage_preserves_explicit_revival_seed() {
    assert_eq!(
        fatal_boss_regeneration_outcome(true, 2, true),
        (2, true, false)
    );
}
