//! Six full expeditions sharing the complete wave/boss ruleset.
use super::{Enemy, GameSession, TrialEnemy};
use bevy::prelude::*;
mod config;
pub use config::ChapterCatalog;
pub(crate) use config::install as install_config;

pub const CHAPTER_COUNT: usize = 6;
#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Chapter {
    pub duration: f32,
    pub health_scale: f32,
    pub damage_scale: f32,
    pub gold_scale: f32,
}
#[derive(Resource, Default, Debug)]
pub struct CampaignRun {
    pub stage: usize,
    pub run_id: u64,
}
#[derive(Resource, Default)]
pub struct RunRequest(pub Option<usize>);
pub(crate) struct CampaignPlugin;
impl Plugin for CampaignPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CampaignRun>()
            .init_resource::<RunRequest>()
            .add_plugins(super::run_reset::RunResetPlugin)
            .add_systems(
                Update,
                scale_enemies
                    .after(super::combat::CombatSet)
                    .before(super::bosses::register_bosses),
            );
    }
}
/// Chapter difficulty rewards more gold, not faster in-run stat growth.
/// Tower and gold challenge enemies keep their fixed tables in every chapter:
/// they are the DPS exam. Waves, bosses and the archive imprint scale.
pub(super) fn scale_enemies(
    run: Res<CampaignRun>,
    catalog: Option<Res<ChapterCatalog>>,
    mut enemies: Query<
        (&mut Enemy, Option<&mut super::combat::EnemyBreachDamage>),
        (Added<Enemy>, Without<TrialEnemy>),
    >,
) {
    let Some(catalog) = catalog else {
        return;
    };
    let difficulty = catalog.chapter(run.stage);
    for (mut enemy, breach) in &mut enemies {
        if let Some(mut breach) = breach {
            breach.0 *= difficulty.damage_scale;
        }
        enemy.hp *= difficulty.health_scale;
        enemy.max_hp *= difficulty.health_scale;
        enemy.damage *= difficulty.damage_scale;
        enemy.gold = (enemy.gold as f32 * difficulty.gold_scale).round() as u32;
    }
}
/// Wave schedules retain their 25-minute authoring scale; only expedition
/// progression is rescaled. Attacks, abilities and cooldowns use real run time.
pub(crate) fn timeline_seconds(elapsed: f32, duration: f32) -> f32 {
    elapsed * (1500.0 / duration.max(0.001))
}
pub(crate) fn fresh_session(rules: Chapter) -> GameSession {
    GameSession {
        duration: rules.duration,
        ..default()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shipped_chapters_keep_duration_and_increase_difficulty_and_rewards() {
        let catalog = ChapterCatalog::read_native().unwrap();
        for stage in 0..CHAPTER_COUNT {
            assert_eq!(catalog.chapter(stage).duration, 1500.0);
            if stage > 0 {
                assert!(
                    catalog.chapter(stage).health_scale > catalog.chapter(stage - 1).health_scale
                );
                assert!(
                    catalog.chapter(stage).damage_scale > catalog.chapter(stage - 1).damage_scale
                );
                assert!(catalog.chapter(stage).gold_scale > catalog.chapter(stage - 1).gold_scale);
            }
        }
    }
    #[test]
    fn short_chapter_keeps_all_five_boss_checkpoints_within_duration() {
        let duration = 600.;
        for (index, boss) in super::super::content::BOSSES.iter().enumerate() {
            let checkpoint = duration * (index + 1) as f32 / 5.;
            assert!(timeline_seconds(checkpoint - 0.01, duration) < boss.spawn_seconds);
            assert!(timeline_seconds(checkpoint, duration) >= boss.spawn_seconds);
            assert!(checkpoint <= duration);
        }
        assert_eq!(timeline_seconds(duration, duration), 1500.);
        assert_eq!(timeline_seconds(60., 1500.), 60.);
    }
    #[test]
    fn chapter_scaling_changes_real_enemy_health_damage_breach_and_rewards_once() {
        let mut app = App::new();
        app.insert_resource(ChapterCatalog::read_native().unwrap());
        app.insert_resource(CampaignRun {
            stage: 2,
            run_id: 0,
        });
        let entity = app
            .world_mut()
            .spawn((
                Enemy {
                    hp: 100.0,
                    max_hp: 100.0,
                    speed: 1.0,
                    damage: 10.0,
                    xp: 10,
                    wood: 2,
                    gold: 10,
                    kill_credit: 1,
                    elite: false,
                },
                super::super::combat::EnemyBreachDamage(10.0),
            ))
            .id();
        let system = app.world_mut().register_system(scale_enemies);
        app.world_mut().run_system(system).unwrap();
        let enemy = app.world().get::<Enemy>(entity).unwrap();
        assert!((enemy.hp - 302.0).abs() < 0.01);
        assert_eq!(enemy.damage, 15.0);
        assert_eq!(enemy.xp, 10);
        assert_eq!(enemy.gold, 14);
        assert_eq!(
            app.world()
                .get::<super::super::combat::EnemyBreachDamage>(entity)
                .unwrap()
                .0,
            15.0
        );
        app.world_mut().run_system(system).unwrap();
        assert!((app.world().get::<Enemy>(entity).unwrap().hp - 302.0).abs() < 0.01);
    }
}
