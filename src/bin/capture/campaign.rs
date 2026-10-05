//! Real menu actions, archive combat result, training purchases and in-app restarts.
use bevy::prelude::*;
use rogue_commander::game::{
    ActiveSkill, ArchiveState, Autoplay, CampaignMenu, CampaignProgress, CampaignReward,
    CampaignRun, Enemy, GameAssets, GameSession, Hero, HeroFusion, HeroKind, HeroVitals,
    PlayerBuild, SelectedHero, SpawnDirector, chapter, skill_evolution::SkillEvolution,
};
#[derive(Resource, Default)]
struct CampaignCapture {
    frame: u32,
    reload: bool,
    difficulty_target: Option<Entity>,
}
pub fn configure(world: &mut World, reload: bool) {
    world.remove_resource::<Autoplay>();
    world.resource_mut::<CampaignMenu>().0 = true;
    world
        .resource_mut::<super::CaptureControls>()
        .suppress_drafts = true;
    let path = std::env::var_os("ROGUE_CAMPAIGN_SAVE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            assert!(
                !reload,
                "campaign-reload requires the first process's ROGUE_CAMPAIGN_SAVE path"
            );
            std::env::temp_dir().join(format!(
                "rogue-campaign-capture-{}.save",
                std::process::id()
            ))
        });
    let progress = CampaignProgress::load_from_path(&path).expect("isolated campaign capture save");
    if !reload {
        assert_eq!(
            progress.currency, 0,
            "campaign capture needs a fresh isolated save path"
        );
        assert_eq!(progress.upgrades, [0; 6]);
    }
    world.insert_resource(progress);
    world.insert_resource(CampaignCapture {
        reload,
        ..default()
    });
    println!("[capture] campaign persistent path={}", path.display());
}
pub fn install(app: &mut bevy::app::SubApp) {
    app.world_mut().remove_resource::<Autoplay>();
    app.add_systems(
        PreUpdate,
        (isolate_spawn, exercise)
            .chain()
            .after(bevy::ui::UiSystems::Focus),
    );
}
fn isolate_spawn(mut director: ResMut<SpawnDirector>) {
    // Run reset is First: pause the fresh director before the next Update can
    // enqueue a normal wave, not one frame later in the fixture's Last phase.
    director.enabled = false;
}
fn press(world: &mut World, name: &str) {
    let mut found = false;
    for (label, mut interaction) in world.query::<(&Name, &mut Interaction)>().iter_mut(world) {
        if label.as_str() == name {
            *interaction = Interaction::Pressed;
            found = true;
        }
    }
    assert!(found, "missing actual campaign action {name}");
}
fn check_fresh_run(world: &World, stage: usize) {
    assert_eq!(world.resource::<CampaignRun>().stage, stage);
    assert_eq!(
        world.resource::<HeroFusion>().count(),
        0,
        "fusion must not leak between expeditions"
    );
    assert!(
        world
            .resource::<SkillEvolution>()
            .tiers
            .iter()
            .all(|tier| *tier == 0)
    );
    assert_eq!(
        world.resource::<PlayerBuild>().skills[31],
        0,
        "previous learned skill must not survive a new expedition"
    );
    assert_eq!(world.resource::<GameSession>().level, 1);
    assert_eq!(world.resource::<GameSession>().gate_revive_remaining, 0.);
    assert_eq!(
        world.resource::<GameSession>().core_hp,
        world.resource::<GameSession>().core_max_hp
    );
    let vitals = world.resource::<HeroVitals>();
    assert_eq!(vitals.deaths, 0);
    assert_eq!(vitals.lives_remaining(), 3);
    assert!(vitals.alive && vitals.revive_remaining == 0.);
}
fn check_permanent(world: &World) {
    let vitals = world.resource::<HeroVitals>();
    let intrinsic = HeroVitals::for_hero(vitals.kind, 1);
    assert!(
        (vitals.strength - intrinsic.strength - 2.).abs() < 0.01,
        "one permanent Strength rank must apply exactly once"
    );
    assert!(
        (vitals.spell_power() - intrinsic.spell_power() - 4.).abs() < 0.01,
        "permanent spell power must survive retries"
    );
}
fn exercise(world: &mut World) {
    let (frame, reload) = {
        let mut state = world.resource_mut::<CampaignCapture>();
        state.frame += 1;
        (state.frame, state.reload)
    };
    world.resource_mut::<ActiveSkill>().remaining = 10_000.;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
    for mut interaction in world.query::<&mut Interaction>().iter_mut(world) {
        if *interaction == Interaction::Pressed {
            *interaction = Interaction::None;
        }
    }
    if reload {
        if frame == 25 {
            let progress = world.resource::<CampaignProgress>();
            assert_eq!(progress.unlocked, 2);
            assert_eq!(progress.upgrades, [1, 0, 0, 0, 0, 1]);
            assert_eq!(progress.currency, 65);
            assert!(progress.cleared[0] && !progress.cleared[1]);
            println!(
                "[capture] campaign separate-process reload PASS: unlocked2, Strength1, Spell1, shards65"
            );
        }
        if frame == 50 {
            press(world, "Campaign training");
        }
        return;
    }
    match frame {
        25 => press(world, "Campaign chapter 0"),
        60 => press(world, "Commander card 0"),
        85 => {
            assert!(world.resource::<SelectedHero>().locked);
            world
                .resource_mut::<HeroFusion>()
                .record_absorbed(HeroKind::IronBerserker);
            world.resource_mut::<PlayerBuild>().skills[31] = 5;
            world.resource_mut::<SkillEvolution>().tiers[31] = 2;
        }
        100 => {
            let mut session = world.resource_mut::<GameSession>();
            session.elapsed = session.duration;
            session.bosses_defeated = 5;
            session.total_kills = 200;
            session.save_active = true;
            session.paused = false;
            println!(
                "[capture] campaign: prepared final checkpoint; archive echo must fall to actual hero combat"
            );
        }
        105 => {
            let echo = world
                .resource::<ArchiveState>()
                .echo
                .expect("real archive encounter spawned");
            world.get_mut::<Enemy>(echo).unwrap().hp = 1.;
            let position = world.get::<Transform>(echo).unwrap().translation;
            for mut hero in world
                .query_filtered::<&mut Transform, With<Hero>>()
                .iter_mut(world)
            {
                hero.translation = position - Vec3::X * 1.2;
            }
            world.resource_mut::<PlayerBuild>().attack_damage = 1000.;
        }
        140 => {
            assert!(
                world.resource::<GameSession>().finished && world.resource::<GameSession>().victory
            );
            let reward = world
                .resource::<CampaignReward>()
                .last
                .expect("real result settlement");
            assert_eq!(reward.chapter, 0);
            assert_eq!(reward.first_clear_bonus, 40);
            assert_eq!(reward.imprint_bonus, 15);
            let progress = world.resource::<CampaignProgress>();
            assert_eq!(progress.unlocked, 2);
            assert_eq!(progress.currency, 90);
            println!("[capture] campaign first clear and unlock through actual archive kill PASS");
        }
        145 => press(world, "Campaign training"),
        170 => press(world, "Campaign buy 0"),
        180 => press(world, "Campaign buy 5"),
        195 => {
            let progress = world.resource::<CampaignProgress>();
            assert_eq!(progress.currency, 60);
            assert_eq!(progress.upgrades, [1, 0, 0, 0, 0, 1]);
            assert!(progress.last_error.is_none());
        }
        205 => press(world, "Campaign chapters"),
        230 => press(world, "Campaign chapter 1"),
        245 => check_fresh_run(world, 1),
        265 => press(world, "Commander card 0"),
        280 => {
            check_permanent(world);
            let scene = world.resource::<GameAssets>().enemy_scenes[0].clone();
            let target = world
                .spawn((
                    Name::new("Campaign difficulty witness"),
                    WorldAssetRoot(scene),
                    Transform::from_xyz(13., 0., 4.),
                    Enemy {
                        hp: 100.,
                        max_hp: 100.,
                        speed: 0.,
                        damage: 10.,
                        xp: 0,
                        wood: 0,
                        gold: 0,
                        kill_credit: 0,
                        elite: false,
                    },
                ))
                .id();
            world.resource_mut::<CampaignCapture>().difficulty_target = Some(target);
        }
        290 => {
            let target = world
                .resource::<CampaignCapture>()
                .difficulty_target
                .unwrap();
            let enemy = world.get::<Enemy>(target).unwrap();
            assert!((enemy.max_hp - 100. * chapter(1).health_scale).abs() < 0.01);
            assert!((enemy.damage - 10. * chapter(1).damage_scale).abs() < 0.01);
            println!(
                "[capture] campaign second chapter stronger; permanent bonuses applied exactly once; old fusion/skills cleared PASS"
            );
        }
        300 => {
            let mut vitals = world.resource_mut::<HeroVitals>();
            vitals.deaths = 2;
            vitals.invulnerable_remaining = 0.;
            let damage = vitals.max_health * 100.;
            assert!(vitals.take_magical_damage(damage) > 0.);
        }
        320 => {
            assert!(
                world.resource::<GameSession>().finished
                    && !world.resource::<GameSession>().victory
            );
            assert_eq!(world.resource::<CampaignReward>().last.unwrap().reward, 5);
            assert_eq!(world.resource::<CampaignProgress>().currency, 65);
            assert_eq!(world.resource::<CampaignProgress>().unlocked, 2);
        }
        330 => press(world, "Campaign retry"),
        345 => check_fresh_run(world, 1),
        360 => press(world, "Commander card 0"),
        390 => {
            check_permanent(world);
            assert_eq!(world.resource::<CampaignProgress>().currency, 65);
            assert!(
                world.resource::<CampaignReward>().last.is_none(),
                "retry must open a new settlement boundary"
            );
            world
                .resource::<CampaignProgress>()
                .save()
                .expect("persist isolated campaign fixture");
            println!(
                "[capture] campaign COMPLETE: clear/unlock, real purchases, harder chapter, persistent attributes, failure reward and clean retry PASS"
            );
        }
        _ => {}
    }
}
