//! An exclusive in-app transition. Persistent scene roots and progress survive;
//! transient gameplay roots (including pool shells) are disposed as one generation.
use super::*;
use bevy::{
    ecs::{observer::Observer, resource::IsResource},
    render::view::screenshot::Screenshot,
    time::Virtual,
};
use std::collections::HashSet;
#[derive(Resource, Default)]
struct PersistentRoots(HashSet<Entity>);
pub(crate) struct RunResetPlugin;
impl Plugin for RunResetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PersistentRoots>()
            .add_systems(PostStartup, remember_scene)
            .add_systems(First, process_request);
    }
}
fn remember_scene(world: &mut World) {
    world.flush();
    let roots = world
        .query_filtered::<Entity, (Without<ChildOf>, Without<IsResource>)>()
        .iter(world)
        .collect();
    world.resource_mut::<PersistentRoots>().0 = roots;
}
fn process_request(world: &mut World) {
    let Some(stage) = world.resource_mut::<RunRequest>().0.take() else {
        return;
    };
    if !world.resource::<CampaignProgress>().can_play(stage) {
        return;
    }
    reset_run(world, stage);
}
pub fn reset_run(world: &mut World, stage: usize) {
    if stage >= campaign::CHAPTER_COUNT || !world.resource::<CampaignProgress>().can_play(stage) {
        return;
    }
    world.flush();
    let roots: Vec<_> = world
        .query_filtered::<Entity, (Without<ChildOf>, Without<IsResource>)>()
        .iter(world)
        .collect();
    let persistent = world.resource::<PersistentRoots>().0.clone();
    for entity in roots {
        if persistent.contains(&entity)
            // Renderer requests and their callbacks can finish across a run transition.
            || world.get::<Screenshot>(entity).is_some()
            || world.get::<Observer>(entity).is_some()
            || world.get::<Camera>(entity).is_some()
            || world.get::<ArenaEntity>(entity).is_some()
        {
            continue;
        }
        let _ = world.despawn(entity);
    }
    // Bevy resources live on IsResource entities. They are excluded from scene
    // disposal and reset explicitly, preserving assets and permanent progress.
    // All transient pool roots have gone; their leases must never outlive a run.
    onboarding::reset_run(world);
    world.insert_resource(CombatPools::default());
    world.insert_resource(campaign::fresh_session(stage));
    world.insert_resource(PlayerBuild::default());
    world.insert_resource(HeroVitals::default());
    world.insert_resource(SelectedHero::default());
    world.insert_resource(HeroFusion::default());
    world.insert_resource(CraftingState::default());
    world.insert_resource(RecruitmentProgress::default());
    world.insert_resource(SpawnDirector::default());
    world.insert_resource(AttackClock::default());
    world.insert_resource(MoveIntent::default());
    world.insert_resource(ActiveSkill::default());
    world.insert_resource(DodgeState::default());
    world.insert_resource(TrialState::default());
    world.insert_resource(GoldChallengeState::default());
    world.insert_resource(DraftState::default());
    world.insert_resource(EncounterState::default());
    encounter_bosses::reset_run(world);
    encounter_boss_ui::reset_run(world);
    basic_relics::reset_run(world);
    world.insert_resource(DamageMeter::default());
    world.insert_resource(ArchiveState::default());
    world.insert_resource(skill_evolution::SkillEvolution::default());
    world.insert_resource(bonds::BondRuntime::default());
    skills::reset_run(world);
    mobile_input::reset_run(world);
    #[cfg(any(debug_assertions, feature = "dev-tools"))]
    debug_menu::reset_run(world);
    world.insert_resource(basic_attacks::BasicAttackAim::default());
    world.insert_resource(attack_vfx::WeaponCues::default());
    world.insert_resource(ultimate_vfx::UltimateCues::default());
    world.insert_resource(bond_feedback::BondCelebrationState::default());
    hero::reset_run(world);
    hero_abilities::reset_run(world);
    combat::reset_run(world);
    bosses::reset_run(world);
    boss_vfx::reset_run(world);
    elite_vfx::reset_run(world);
    market::reset_run(world);
    progression::reset_run(world);
    ui::reset_run(world);
    skill_feedback::reset_run(world);
    audio::reset_run(world);
    hit_feedback::reset_run(world);
    gate_feedback::reset_run(world);
    settings::reset_run(world);
    campaign_progress::reset_run_progress(world);
    if let Some(mut menu) = world.get_resource_mut::<campaign_ui::CampaignMenu>() {
        menu.0 = false;
    }
    let next = world.resource::<CampaignRun>().run_id.wrapping_add(1);
    world.insert_resource(CampaignRun {
        stage,
        run_id: next,
    });
    let seed = world.resource::<RunSeed>().0.wrapping_add(next);
    world.insert_resource(RunSeed(seed));
    world.insert_resource(RunRng::from_seed(seed));
    if let Some(mut time) = world.get_resource_mut::<Time<Virtual>>() {
        time.set_relative_speed(1.0);
        time.unpause();
    }
    if let Some(mut keys) = world.get_resource_mut::<ButtonInput<KeyCode>>() {
        keys.clear();
    }
    if let Some(mut messages) = world.get_resource_mut::<Messages<EnemyAttackCue>>() {
        messages.clear();
    }
    for (mut transform, mut visibility) in world
        .query_filtered::<(&mut Transform, &mut Visibility), With<Hero>>()
        .iter_mut(world)
    {
        *transform = Transform::from_translation(resources::HERO_RESPAWN_POSITION);
        *visibility = Visibility::Visible;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn return_to_chapters_discards_run_without_awarding_or_losing_permanent_progress() {
        let mut app = App::new();
        app.init_resource::<CampaignProgress>()
            .add_plugins((RunResetPlugin, campaign_ui::CampaignUiPlugin));
        let world = app.world_mut();
        world.insert_resource(CampaignRun {
            stage: 0,
            run_id: 9,
        });
        world.insert_resource(RunSeed(88));
        world.resource_mut::<CampaignProgress>().currency = 123;
        world.resource_mut::<CampaignProgress>().upgrades[2] = 2;
        world.insert_resource(GameSession {
            gold: 999,
            kills: 600,
            ..default()
        });
        let scene = world
            .spawn((ArenaEntity, Name::new("Persistent scene")))
            .id();
        remember_scene(world);
        let enemy = world.spawn(Name::new("Transient enemy")).id();
        campaign_ui::open_chapters(world);
        assert!(world.get_entity(enemy).is_err());
        assert!(world.get_entity(scene).is_ok());
        assert!(world.resource::<campaign_ui::CampaignMenu>().0);
        assert!(world.resource::<GameSession>().paused);
        assert_eq!(world.resource::<CampaignRun>().run_id, 10);
        assert_eq!(world.resource::<CampaignProgress>().currency, 123);
        assert_eq!(world.resource::<CampaignProgress>().upgrades[2], 2);
        assert!(!world.resource::<SelectedHero>().locked);
        assert!(!world.resource::<GameSession>().finished);
        assert_eq!(world.resource::<GameSession>().kills, 0);
    }

    #[test]
    fn two_in_app_restarts_preserve_scene_and_progress_but_remove_run_entities_and_bonuses() {
        let mut world = World::new();
        world.insert_resource(PersistentRoots::default());
        world.insert_resource(CampaignProgress::default());
        world.insert_resource(CampaignRun::default());
        world.insert_resource(RunSeed(77));
        world.resource_mut::<CampaignProgress>().currency = 90;
        world.resource_mut::<CampaignProgress>().upgrades[0] = 3;
        world.resource_mut::<CampaignProgress>().unlocked = 2;
        let scene = world
            .spawn((Name::new("Persistent scene"), ArenaEntity))
            .id();
        let hero = world
            .spawn((
                Hero,
                Transform::from_xyz(8.0, -1000.0, 0.0),
                Visibility::Hidden,
                ArenaEntity,
            ))
            .id();
        remember_scene(&mut world);
        #[derive(Resource)]
        struct LatePersistentAsset(u32);
        world.insert_resource(LatePersistentAsset(99));
        #[derive(Resource, Default)]
        struct ReceivedScreenshot(bool);
        world.init_resource::<ReceivedScreenshot>();
        let screenshot = world
            .spawn(Screenshot::primary_window())
            .observe(
                |_event: On<bevy::render::view::screenshot::ScreenshotCaptured>,
                 mut received: ResMut<ReceivedScreenshot>| {
                    received.0 = true;
                },
            )
            .id();

        let transient = world
            .spawn((Name::new("Old ability"), Transform::default()))
            .id();
        let mut build = PlayerBuild::default();
        build.skills[3] = 5;
        world.insert_resource(build);
        let mut fusion = HeroFusion::default();
        fusion.record_absorbed(HeroKind::StoneWarden);
        world.insert_resource(fusion);
        let mut encounters = EncounterState::default();
        assert!(encounters.bank_for_boss(0));
        assert!(encounters.bank_for_boss(1));
        world.insert_resource(encounters);
        reset_run(&mut world, 1);
        assert_eq!(
            *world.resource::<EncounterState>(),
            EncounterState::default(),
            "banked encounters and taken events belong to one run"
        );
        assert!(world.get_entity(scene).is_ok());
        assert_eq!(world.resource::<LatePersistentAsset>().0, 99);
        assert!(world.get::<Screenshot>(screenshot).is_some());

        assert!(world.get_entity(transient).is_err());
        assert!(
            world
                .resource::<PlayerBuild>()
                .skills
                .iter()
                .all(|rank| *rank == 0)
        );
        assert_eq!(world.resource::<HeroFusion>().count(), 0);
        assert_eq!(world.resource::<CampaignProgress>().currency, 90);
        assert_eq!(world.resource::<CampaignProgress>().upgrades[0], 3);
        assert_eq!(world.resource::<CampaignRun>().stage, 1);
        assert_eq!(world.resource::<CampaignRun>().run_id, 1);
        assert_eq!(world.get::<Visibility>(hero), Some(&Visibility::Visible));
        assert_eq!(
            world.get::<Transform>(hero).unwrap().translation,
            resources::HERO_RESPAWN_POSITION
        );
        assert!(!world.resource::<SelectedHero>().locked);
        assert_eq!(world.resource::<GameSession>().elapsed, 0.0);
        assert!(!world.resource::<TrialState>().active);
        let second_transient = world.spawn(Name::new("Second run ability")).id();
        reset_run(&mut world, 0);
        assert!(world.get_entity(second_transient).is_err());
        assert_eq!(world.resource::<LatePersistentAsset>().0, 99);
        assert!(world.get_entity(scene).is_ok());
        assert_eq!(world.resource::<CampaignRun>().run_id, 2);
        assert!(world.get::<Screenshot>(screenshot).is_some());
        world.trigger(bevy::render::view::screenshot::ScreenshotCaptured {
            entity: screenshot,
            image: Image::default(),
        });
        assert!(
            world.resource::<ReceivedScreenshot>().0,
            "pending screenshot callback must remain functional after two resets"
        );

        assert_eq!(world.resource::<CampaignProgress>().currency, 90);
    }
    #[test]
    fn locked_chapter_request_cannot_dispose_current_run() {
        let mut world = World::new();
        world.insert_resource(CampaignProgress::default());
        let entity = world.spawn_empty().id();
        reset_run(&mut world, 5);
        assert!(world.get_entity(entity).is_ok());
    }
}
