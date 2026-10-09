//! Authored cast clips use Idle, Walk, Attack, Hit (indices 0–3), Y up and -Z forward.
use std::collections::HashMap;

use bevy::{app::AnimationSystems, prelude::*};

use super::basic_attacks::BasicAttackAim;
use super::combat::AttackVolleyRuntime;
use super::skills::{
    AncestorColossus, BoltTurret, BoneSentry, FrostWyrm, SpiritHound, TideElemental,
};
use super::{
    AttackClock, Core, Enemy, EnemyHeroThreat, GameSession, Hero, HeroVitals, PlayerSummon,
};

#[path = "model_animation/enemy_attacks.rs"]
mod enemy_attacks;

pub(super) struct ModelAnimationPlugin;

impl Plugin for ModelAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CastGraphs>().add_systems(
            PostUpdate,
            (attach_cast_players, enemy_attacks::collect, animate_cast)
                .chain()
                .before(AnimationSystems),
        );
    }
}

#[derive(Clone)]
struct CastGraph {
    handle: Handle<AnimationGraph>,
    clips: Vec<AnimationNodeIndex>,
    source_clips: Vec<Handle<AnimationClip>>,
}

#[derive(Resource, Default)]
struct CastGraphs(HashMap<AssetId<WorldAsset>, CastGraph>);

const ATTACK_SECONDS: f32 = 0.45;
const HIT_SECONDS: f32 = 0.24;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
enum Pose {
    Idle,
    Walk,
    Attack,
    Hit,
}

#[derive(Component)]
struct CastPlayer {
    actor: Entity,
    clips: Vec<AnimationNodeIndex>,
    source_clips: Vec<Handle<AnimationClip>>,
    pose: Option<Pose>,
    previous_position: Option<Vec3>,
    previous_health: Option<f32>,
    previous_cooldown: Option<f32>,
    reaction_remaining: f32,
    enemy_attack_target: Option<Vec3>,
    enemy_attack_pending: bool,
}

impl CastPlayer {
    fn reset(&mut self) {
        self.pose = None;
        self.previous_position = None;
        self.previous_health = None;
        self.previous_cooldown = None;
        self.reaction_remaining = 0.0;
        self.enemy_attack_target = None;
        self.enemy_attack_pending = false;
    }
}

fn attach_cast_players(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut cache: ResMut<CastGraphs>,
    players: Query<Entity, (With<AnimationPlayer>, Without<CastPlayer>)>,
    parents: Query<&ChildOf>,
    scenes: Query<&WorldAssetRoot>,
    actors: Query<
        (),
        Or<(
            With<Enemy>,
            With<Hero>,
            With<PlayerSummon>,
            With<super::environment::CampPreview>,
        )>,
    >,
) {
    for player in &players {
        let mut cursor = player;
        let mut scene = None;
        loop {
            if let Ok(root) = scenes.get(cursor) {
                scene = Some(root.0.clone());
            }
            if actors.contains(cursor) {
                if let Some(scene) = scene {
                    let graph = cache.0.entry(scene.id()).or_insert_with(|| {
                        let path = scene
                            .path()
                            .expect("cast scenes have asset paths")
                            .path()
                            .to_string_lossy()
                            .into_owned();
                        let source_clips: Vec<_> = (0..4)
                            .map(|index| {
                                server
                                    .load(GltfAssetLabel::Animation(index).from_asset(path.clone()))
                            })
                            .collect();
                        let (graph, clips) =
                            AnimationGraph::from_clips(source_clips.iter().cloned());
                        CastGraph {
                            handle: graphs.add(graph),
                            clips,
                            source_clips,
                        }
                    });
                    commands.entity(player).insert((
                        AnimationGraphHandle(graph.handle.clone()),
                        CastPlayer {
                            actor: cursor,
                            clips: graph.clips.clone(),
                            source_clips: graph.source_clips.clone(),
                            pose: None,
                            previous_position: None,
                            previous_health: None,
                            previous_cooldown: None,
                            reaction_remaining: 0.0,
                            enemy_attack_target: None,
                            enemy_attack_pending: false,
                        },
                    ));
                }
                break;
            }
            let Ok(parent) = parents.get(cursor) else {
                break;
            };
            cursor = parent.parent();
        }
    }
}

fn choose_pose(
    moving: bool,
    attacked: bool,
    hurt: bool,
    previous: Option<Pose>,
    reaction: &mut f32,
    delta: f32,
) -> Pose {
    *reaction = (*reaction - delta).max(0.0);
    if hurt {
        *reaction = HIT_SECONDS;
        Pose::Hit
    } else if attacked {
        *reaction = ATTACK_SECONDS;
        Pose::Attack
    } else if *reaction > 0.0 {
        previous.unwrap_or(Pose::Idle)
    } else if moving {
        Pose::Walk
    } else {
        Pose::Idle
    }
}

fn animate_cast(
    time: Res<Time>,
    clips: Res<Assets<AnimationClip>>,
    session: Res<GameSession>,
    controls: Res<super::skills::SkillRuntime>,
    vitals: Res<HeroVitals>,
    volleys: Option<Res<AttackVolleyRuntime>>,
    aim: Option<Res<BasicAttackAim>>,
    attack_clock: Option<Res<AttackClock>>,
    mut actors: Query<
        (
            &mut Transform,
            Option<&Enemy>,
            Option<&Hero>,
            Option<&PlayerSummon>,
            Option<&EnemyHeroThreat>,
            Option<&Visibility>,
        ),
        (
            Without<Core>,
            Or<(
                With<Enemy>,
                With<Hero>,
                With<PlayerSummon>,
                With<WorldAssetRoot>,
            )>,
        ),
    >,
    summons: Query<(
        Option<&BoneSentry>,
        Option<&SpiritHound>,
        Option<&TideElemental>,
        Option<&BoltTurret>,
        Option<&FrostWyrm>,
        Option<&AncestorColossus>,
    )>,
    previews: Query<(), With<super::environment::CampPreview>>,
    mut players: Query<(&mut AnimationPlayer, &mut CastPlayer)>,
) {
    let targets: Vec<Vec3> = actors
        .iter()
        .filter(|(_, enemy, ..)| enemy.is_some_and(|enemy| enemy.hp > 0.0))
        .map(|(transform, ..)| transform.translation)
        .collect();
    for (mut player, mut state) in &mut players {
        let Ok((mut transform, enemy, hero, summon, threat, visibility)) =
            actors.get_mut(state.actor)
        else {
            player.stop_all();
            state.reset();
            continue;
        };
        // Pooled scene hierarchies persist after their gameplay markers are removed.
        if (enemy.is_none()
            && hero.is_none()
            && summon.is_none()
            && !previews.contains(state.actor))
            || visibility == Some(&Visibility::Hidden)
            || (hero.is_some() && !vitals.alive)
        {
            player.stop_all();
            state.reset();
            continue;
        }
        if session.paused || session.finished {
            player.pause_all();
            continue;
        }
        if enemy.is_some() && controls.actions_blocked(state.actor) {
            player.pause_all();
            // Do not replay control-time damage/movement/cues as a new action.
            state.previous_position = Some(transform.translation);
            state.previous_health = enemy.map(|enemy| enemy.hp);
            state.previous_cooldown = threat.map(|threat| threat.attack_remaining);
            state.enemy_attack_pending = false;
            continue;
        }
        player.resume_all();
        let position = transform.translation;
        let displacement = state
            .previous_position
            .map_or(Vec3::ZERO, |previous| (position - previous).with_y(0.0));
        let moving =
            enemy_attacks::visible_movement(displacement, time.delta_secs(), enemy.is_some());
        let health = enemy
            .map(|enemy| enemy.hp)
            .or_else(|| hero.map(|_| vitals.health));
        let hurt = health
            .zip(state.previous_health)
            .is_some_and(|(health, previous)| health < previous);
        let cooldown = threat.map(|threat| threat.attack_remaining).or_else(|| {
            let (sentry, hound, tide, turret, wyrm, colossus) = summons.get(state.actor).ok()?;
            sentry
                .map(|value| value.attack_remaining)
                .or_else(|| hound.map(|value| value.attack_remaining))
                .or_else(|| tide.map(|value| value.attack_remaining))
                .or_else(|| turret.map(|value| value.attack_remaining))
                .or_else(|| wyrm.map(|value| value.attack_remaining))
                .or_else(|| colossus.map(|value| value.slam_remaining))
        });
        let attacked = if hero.is_some() {
            vitals.can_act()
                && volleys
                    .as_ref()
                    .is_some_and(|volleys| volleys.resolved_this_frame > 0)
        } else if enemy.is_some() {
            std::mem::take(&mut state.enemy_attack_pending)
        } else {
            cooldown
                .zip(state.previous_cooldown)
                .is_some_and(|(current, previous)| current > previous)
        };
        let previous = state.pose;
        // Let a real hostile attack finish rather than restarting Hit on every damage tick.
        let react_to_hit = hurt
            && !(enemy.is_some()
                && (attacked
                    || (previous == Some(Pose::Attack)
                        && state.reaction_remaining > time.delta_secs())));
        let pose = choose_pose(
            moving,
            attacked,
            react_to_hit,
            previous,
            &mut state.reaction_remaining,
            time.delta_secs(),
        );
        if pose == Pose::Attack && attacked {
            // Fast attack builds must finish their action before the next volley,
            // rather than continuously restarting the first half of the source clip.
            let next_attack = if hero.is_some() {
                attack_clock
                    .as_ref()
                    .map(|clock| clock.0.duration().as_secs_f32())
            } else {
                cooldown
            };
            state.reaction_remaining = next_attack
                .unwrap_or(ATTACK_SECONDS)
                .clamp(0.03, ATTACK_SECONDS);
        }
        if pose != previous.unwrap_or(Pose::Idle) || previous.is_none() || attacked || react_to_hit
        {
            player.stop_all();
            let animation = player.start(state.clips[pose as usize]);
            if matches!(pose, Pose::Idle | Pose::Walk) {
                animation.repeat();
            } else if let Some(clip) = state
                .source_clips
                .get(pose as usize)
                .and_then(|handle| clips.get(handle))
            {
                let hold = state.reaction_remaining;
                // Consume the entire authored action during its gameplay reaction window.
                // Bevy advances this with virtual Time, so game speed applies only once.
                animation.set_speed(clip.duration() / hold);
            }
            state.pose = Some(pose);
        }
        // Summon AI owns facing during pursuit. Formation movement must not
        // make an idle summon turn its back to the approach lane afterward.
        let facing = if summon.is_some() {
            targets.is_empty().then_some(Vec3::X)
        } else if enemy.is_some() && pose == Pose::Attack {
            state
                .enemy_attack_target
                .map(|target| (target - position).with_y(0.0))
        } else if attacked && hero.is_some() && aim.as_ref().is_some_and(|aim| aim.0.is_some()) {
            aim.as_ref()
                .and_then(|aim| aim.0)
                .map(|target| (target - position).with_y(0.0))
        } else if attacked && enemy.is_none() {
            targets
                .iter()
                .min_by(|a, b| {
                    a.distance_squared(position)
                        .total_cmp(&b.distance_squared(position))
                })
                .map(|target| (*target - position).with_y(0.0))
        } else if moving {
            Some(displacement)
        } else {
            None
        };
        if let Some(direction) = facing.filter(|direction| direction.length_squared() > 0.000_001) {
            transform.look_to(direction, Vec3::Y);
        }
        state.previous_position = Some(position);
        state.previous_health = health;
        state.previous_cooldown = cooldown;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_enemy_pauses_authored_clip_and_facing_without_replaying_attack_on_thaw() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<Assets<AnimationClip>>()
            .init_resource::<GameSession>()
            .init_resource::<HeroVitals>()
            .init_resource::<super::super::skills::SkillRuntime>()
            .add_systems(Update, animate_cast);
        let mut clip = AnimationClip::default();
        clip.set_duration(1.0);
        let clip = app
            .world_mut()
            .resource_mut::<Assets<AnimationClip>>()
            .add(clip);
        let actor = app
            .world_mut()
            .spawn((
                Enemy {
                    hp: 100.0,
                    max_hp: 100.0,
                    speed: 1.0,
                    damage: 10.0,
                    xp: 0,
                    wood: 0,
                    gold: 0,
                    kill_credit: 0,
                    elite: false,
                },
                EnemyHeroThreat::ranged(),
                Transform::default(),
                Visibility::Visible,
            ))
            .id();
        let node = AnimationNodeIndex::new(0);
        let mut animation = AnimationPlayer::default();
        animation.play(node).repeat().set_seek_time(0.37);
        let player = app
            .world_mut()
            .spawn((
                animation,
                CastPlayer {
                    actor,
                    clips: vec![node; 4],
                    source_clips: vec![clip; 4],
                    pose: Some(Pose::Walk),
                    previous_position: Some(Vec3::ZERO),
                    previous_health: Some(100.0),
                    previous_cooldown: Some(1.0),
                    reaction_remaining: 0.25,
                    enemy_attack_target: Some(Vec3::X),
                    enemy_attack_pending: true,
                },
            ))
            .id();
        app.world_mut()
            .resource_mut::<super::super::skills::SkillRuntime>()
            .freeze_relic(actor, 1.0, false, false, 10.0);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.5));
        let transform = *app.world().get::<Transform>(actor).unwrap();
        app.update();
        assert!(
            app.world()
                .get::<AnimationPlayer>(player)
                .unwrap()
                .all_paused()
        );
        assert_eq!(
            app.world()
                .get::<AnimationPlayer>(player)
                .unwrap()
                .animation(node)
                .unwrap()
                .seek_time(),
            0.37
        );
        assert_eq!(*app.world().get::<Transform>(actor).unwrap(), transform);
        assert_eq!(
            app.world()
                .get::<CastPlayer>(player)
                .unwrap()
                .reaction_remaining,
            0.25
        );
        assert!(
            !app.world()
                .get::<CastPlayer>(player)
                .unwrap()
                .enemy_attack_pending
        );
        app.world_mut()
            .insert_resource(super::super::skills::SkillRuntime::default());
        app.update();
        assert!(
            !app.world()
                .get::<AnimationPlayer>(player)
                .unwrap()
                .all_paused()
        );
        assert_ne!(
            app.world().get::<CastPlayer>(player).unwrap().pose,
            Some(Pose::Attack)
        );
        app.world_mut()
            .resource_mut::<super::super::skills::SkillRuntime>()
            .freeze_relic(actor, 1.0, false, false, 10.0);
        app.world_mut()
            .get_mut::<Visibility>(actor)
            .unwrap()
            .set_if_neq(Visibility::Hidden);
        app.update();
        assert!(
            app.world()
                .get::<AnimationPlayer>(player)
                .unwrap()
                .playing_animations()
                .next()
                .is_none()
        );
    }

    #[test]
    fn idle_summon_formation_displacement_does_not_override_forward_facing() {
        use bevy::ecs::system::RunSystemOnce;
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<Assets<AnimationClip>>()
            .init_resource::<GameSession>()
            .init_resource::<HeroVitals>()
            .init_resource::<super::super::skills::SkillRuntime>();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(33));
        let actor = app
            .world_mut()
            .spawn((
                PlayerSummon,
                Transform::from_xyz(1.0, 0.0, 2.0),
                Visibility::Visible,
            ))
            .id();
        app.world_mut().spawn((
            AnimationPlayer::default(),
            CastPlayer {
                actor,
                clips: vec![AnimationNodeIndex::new(0); 4],
                source_clips: vec![],
                pose: Some(Pose::Walk),
                previous_position: Some(Vec3::new(2.0, 0.0, 1.0)),
                previous_health: None,
                previous_cooldown: None,
                reaction_remaining: 0.0,
                enemy_attack_target: None,
                enemy_attack_pending: false,
            },
        ));
        app.world_mut().run_system_once(animate_cast).unwrap();
        let transform = app.world().get::<Transform>(actor).unwrap();
        assert!(transform.forward().dot(Vec3::X) > 0.999);
        assert_eq!(transform.translation, Vec3::new(1.0, 0.0, 2.0));
    }

    #[test]
    fn damage_interrupts_walk_and_reaction_returns_to_locomotion() {
        let mut remaining = 0.0;
        assert_eq!(
            choose_pose(true, false, true, Some(Pose::Walk), &mut remaining, 0.016),
            Pose::Hit
        );
        assert_eq!(
            choose_pose(true, false, false, Some(Pose::Hit), &mut remaining, 0.1),
            Pose::Hit
        );
        assert_eq!(
            choose_pose(true, false, false, Some(Pose::Hit), &mut remaining, 0.2),
            Pose::Walk
        );
    }

    #[test]
    fn attacks_play_before_returning_to_idle() {
        let mut remaining = 0.0;
        assert_eq!(
            choose_pose(false, true, false, Some(Pose::Idle), &mut remaining, 0.016),
            Pose::Attack
        );
        assert_eq!(
            choose_pose(false, false, false, Some(Pose::Attack), &mut remaining, 0.2),
            Pose::Attack
        );
        assert_eq!(
            choose_pose(false, false, false, Some(Pose::Attack), &mut remaining, 0.3),
            Pose::Idle
        );
    }

    #[test]
    fn pooled_players_pause_retire_and_restart_without_teleport_walk() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<GameSession>()
            .init_resource::<HeroVitals>()
            .init_resource::<super::super::skills::SkillRuntime>()
            .init_resource::<Assets<AnimationClip>>()
            .add_systems(Update, animate_cast);
        let actor = app
            .world_mut()
            .spawn((
                PlayerSummon,
                WorldAssetRoot(Handle::default()),
                Transform::default(),
                Visibility::Visible,
            ))
            .id();
        let player = app
            .world_mut()
            .spawn((
                AnimationPlayer::default(),
                CastPlayer {
                    actor,
                    clips: (0..4).map(AnimationNodeIndex::new).collect(),
                    source_clips: Vec::new(),
                    pose: None,
                    previous_position: None,
                    previous_health: None,
                    previous_cooldown: None,
                    reaction_remaining: 0.0,
                    enemy_attack_target: None,
                    enemy_attack_pending: false,
                },
            ))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<CastPlayer>(player).unwrap().pose,
            Some(Pose::Idle)
        );
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.update();
        assert!(
            app.world()
                .get::<AnimationPlayer>(player)
                .unwrap()
                .all_paused()
        );
        app.world_mut().resource_mut::<GameSession>().paused = false;
        app.world_mut()
            .get_mut::<Transform>(actor)
            .unwrap()
            .translation
            .x = 1.0;
        app.update();
        assert!(
            !app.world()
                .get::<AnimationPlayer>(player)
                .unwrap()
                .all_paused()
        );
        assert_eq!(
            app.world().get::<CastPlayer>(player).unwrap().pose,
            Some(Pose::Walk)
        );
        app.world_mut().entity_mut(actor).remove::<PlayerSummon>();
        app.update();
        assert_eq!(
            app.world()
                .get::<AnimationPlayer>(player)
                .unwrap()
                .playing_animations()
                .count(),
            0
        );
        assert!(
            app.world()
                .get::<CastPlayer>(player)
                .unwrap()
                .previous_position
                .is_none()
        );
        app.world_mut()
            .entity_mut(actor)
            .insert((PlayerSummon, Transform::from_xyz(100.0, 0.0, 0.0)));
        app.update();
        assert_eq!(
            app.world().get::<CastPlayer>(player).unwrap().pose,
            Some(Pose::Idle)
        );
        app.world_mut().resource_mut::<GameSession>().finished = true;
        app.update();
        assert!(
            app.world()
                .get::<AnimationPlayer>(player)
                .unwrap()
                .all_paused()
        );
    }

    #[test]
    fn authored_action_durations_fit_the_gameplay_reaction_windows() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<GameSession>()
            .init_resource::<HeroVitals>()
            .init_resource::<super::super::skills::SkillRuntime>()
            .init_resource::<Assets<AnimationClip>>()
            .insert_resource(AttackVolleyRuntime {
                pending: 0,
                resolved_this_frame: 1,
            })
            .add_systems(Update, animate_cast);
        let source_clips: Vec<_> = [1.0, 1.0, 1.35, 0.96]
            .into_iter()
            .map(|duration| {
                let mut clip = AnimationClip::default();
                clip.set_duration(duration);
                app.world_mut()
                    .resource_mut::<Assets<AnimationClip>>()
                    .add(clip)
            })
            .collect();
        let actor = app
            .world_mut()
            .spawn((Hero, Transform::default(), Visibility::Visible))
            .id();
        let player = app
            .world_mut()
            .spawn((
                AnimationPlayer::default(),
                CastPlayer {
                    actor,
                    clips: (0..4).map(AnimationNodeIndex::new).collect(),
                    source_clips,
                    pose: None,
                    previous_position: None,
                    previous_health: None,
                    previous_cooldown: None,
                    reaction_remaining: 0.0,
                    enemy_attack_target: None,
                    enemy_attack_pending: false,
                },
            ))
            .id();
        app.update();
        let attack_speed = app
            .world()
            .get::<AnimationPlayer>(player)
            .unwrap()
            .animation(AnimationNodeIndex::new(Pose::Attack as usize))
            .unwrap()
            .speed();
        assert!((attack_speed - 3.0).abs() < 0.0001);
        app.world_mut()
            .insert_resource(AttackClock(Timer::from_seconds(0.15, TimerMode::Repeating)));
        app.update();
        let fast_attack_speed = app
            .world()
            .get::<AnimationPlayer>(player)
            .unwrap()
            .animation(AnimationNodeIndex::new(Pose::Attack as usize))
            .unwrap()
            .speed();
        assert!((fast_attack_speed - 9.0).abs() < 0.0001);
        app.world_mut().resource_mut::<HeroVitals>().health -= 1.0;
        app.update();
        let hit_speed = app
            .world()
            .get::<AnimationPlayer>(player)
            .unwrap()
            .animation(AnimationNodeIndex::new(Pose::Hit as usize))
            .unwrap()
            .speed();
        assert!((hit_speed - 4.0).abs() < 0.0001);
    }
}
