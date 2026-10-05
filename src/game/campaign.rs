//! Six full expeditions sharing the complete wave/boss ruleset.
use super::{Enemy, GameSession, TrialEnemy};
use bevy::prelude::*;
pub const CHAPTER_COUNT: usize = 6;
#[derive(Clone, Copy, Debug)]
pub struct Chapter {
    pub duration: f32,
    pub health_scale: f32,
    pub damage_scale: f32,
    pub reward_scale: f32,
}
pub fn chapter(stage: usize) -> Chapter {
    let n = stage.min(CHAPTER_COUNT - 1) as f32;
    Chapter {
        duration: 1500.0,
        health_scale: 1.0 + n * 0.65 + n * n * 0.18,
        damage_scale: 1.0 + n * 0.25,
        reward_scale: 1.0 + n * 0.2,
    }
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
/// Tower and gold challenge enemies keep their fixed tables in every chapter:
/// they are the DPS exam. Waves, bosses and the archive imprint scale.
pub(super) fn scale_enemies(
    run: Res<CampaignRun>,
    mut enemies: Query<
        (&mut Enemy, Option<&mut super::combat::EnemyBreachDamage>),
        (Added<Enemy>, Without<TrialEnemy>),
    >,
) {
    let difficulty = chapter(run.stage);
    for (mut enemy, breach) in &mut enemies {
        if let Some(mut breach) = breach {
            breach.0 *= difficulty.damage_scale;
        }
        enemy.hp *= difficulty.health_scale;
        enemy.max_hp *= difficulty.health_scale;
        enemy.damage *= difficulty.damage_scale;
        enemy.xp = (enemy.xp as f32 * difficulty.reward_scale).round() as u32;
        enemy.gold = (enemy.gold as f32 * difficulty.reward_scale).round() as u32;
    }
}
#[cfg(test)]
fn checkpoint_seconds(stage: usize, index: usize) -> f32 {
    chapter(stage).duration * (index.min(4) + 1) as f32 / 5.0
}
pub(crate) fn fresh_session(stage: usize) -> GameSession {
    GameSession {
        duration: chapter(stage).duration,
        ..default()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn six_chapters_keep_all_five_bosses_and_increase_difficulty_and_rewards() {
        for stage in 0..CHAPTER_COUNT {
            assert_eq!(checkpoint_seconds(stage, 4), chapter(stage).duration);
            if stage > 0 {
                assert!(chapter(stage).health_scale > chapter(stage - 1).health_scale);
                assert!(chapter(stage).damage_scale > chapter(stage - 1).damage_scale);
                assert!(chapter(stage).reward_scale > chapter(stage - 1).reward_scale);
            }
        }
    }
    #[test]
    fn chapter_scaling_changes_real_enemy_health_damage_breach_and_rewards_once() {
        let mut app = App::new();
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
        assert_eq!(enemy.xp, 14);
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
