use super::bonds::BondRuntime;
use super::{EnemyAttackCue, EnemyAttackKind};
use bevy::prelude::*;

use super::combat::CombatSet;
use super::hero_abilities::TimedSpeedEffect;
use super::pooling::{CombatPools, EnemySceneKey};
use super::{
    BossEnemy, Enemy, GameAssets, GameSession, GoldChallengeState, Hero, HeroVitals, ImpactFx,
    TrialState,
};

pub const ARCHIVE_DURATION: f32 = 60.0;
pub const ARCHIVE_ECHO_HP: f32 = 600_000.0;
const ARCHIVE_ECHO_SPEED: f32 = 2.25;
const ARCHIVE_ATTACK_RANGE: f32 = 2.6;
const ARCHIVE_ATTACK_DAMAGE: f32 = 72.0;
const ARCHIVE_ATTACK_INTERVAL: f32 = 1.15;

pub struct ArchivePlugin;

impl Plugin for ArchivePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ArchiveState>().add_systems(
            Update,
            (begin_archive, run_archive, animate_archive_aura)
                .chain()
                .after(CombatSet),
        );
    }
}

#[derive(Component)]
pub struct ArchiveEcho;

#[derive(Component)]
struct ArchiveAura {
    offset: f32,
    base_scale: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ArchiveOutcome {
    #[default]
    Pending,
    Captured,
    TimedOut,
}

#[derive(Resource)]
pub struct ArchiveState {
    pub active: bool,
    pub outcome: ArchiveOutcome,
    pub echo: Option<Entity>,
    attack_timer: f32,
    aura_elapsed: f32,
}

impl Default for ArchiveState {
    fn default() -> Self {
        Self {
            active: false,
            outcome: ArchiveOutcome::Pending,
            echo: None,
            attack_timer: 0.35,
            aura_elapsed: 0.0,
        }
    }
}

impl ArchiveState {
    pub fn resolve(&mut self, session: &mut GameSession, outcome: ArchiveOutcome) {
        self.active = false;
        self.outcome = outcome;
        self.echo = None;
        session.save_active = false;
        session.save_remaining = 0.0;
        session.finished = true;
        session.victory = true;
        session.paused = true;
    }
}

fn begin_archive(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    mut session: ResMut<GameSession>,
    mut archive: ResMut<ArchiveState>,
    mut tower: ResMut<TrialState>,
    mut gold: ResMut<GoldChallengeState>,
    mut pools: ResMut<CombatPools>,
    ordinary_enemies: Query<Entity, (With<Enemy>, Without<BossEnemy>, Without<ArchiveEcho>)>,
) {
    if !session.save_active || archive.active || archive.outcome != ArchiveOutcome::Pending {
        return;
    }
    let Some(assets) = assets else {
        return;
    };

    for entity in &ordinary_enemies {
        pools.retire_enemy(&mut commands, entity);
    }
    pools.retire_all_projectiles(&mut commands);
    pools.retire_all_effects(&mut commands);
    reset_challenge(&mut *tower);
    reset_challenge(&mut *gold);

    let Some(echo) = pools.rent_enemy_scene(
        &mut commands,
        EnemySceneKey::Archive,
        assets.boss_scenes[4].clone(),
        Transform::from_xyz(10.8, 0.0, 0.0).with_scale(Vec3::splat(1.42)),
    ) else {
        return;
    };
    commands.entity(echo).insert((
        Enemy {
            hp: ARCHIVE_ECHO_HP,
            max_hp: ARCHIVE_ECHO_HP,
            speed: ARCHIVE_ECHO_SPEED,
            damage: ARCHIVE_ATTACK_DAMAGE,
            xp: 0,
            wood: 0,
            gold: 0,
            kill_credit: 0,
            elite: true,
        },
        ArchiveEcho,
    ));
    for (offset, scale) in [(0.0, 1.7), (1.8, 2.25)] {
        let decoration = commands
            .spawn((
                Mesh3d(assets.fx_mesh.clone()),
                MeshMaterial3d(assets.storm_material.clone()),
                Transform::from_xyz(0.0, 0.12, 0.0).with_scale(Vec3::splat(scale)),
                ArchiveAura {
                    offset,
                    base_scale: scale,
                },
            ))
            .id();
        commands.entity(echo).add_child(decoration);
        pools.track_enemy_decoration(echo, decoration);
    }

    archive.active = true;
    archive.attack_timer = 0.35;
    archive.aura_elapsed = 0.0;
    session.save_remaining = ARCHIVE_DURATION;

    archive.echo = Some(echo);
}

trait ResettableChallenge {
    fn clear_active(&mut self);
}

impl ResettableChallenge for TrialState {
    fn clear_active(&mut self) {
        self.active = false;
        self.remaining = 0.0;
        self.entity = None;
    }
}

impl ResettableChallenge for GoldChallengeState {
    fn clear_active(&mut self) {
        self.active = false;
        self.remaining = 0.0;
        self.entity = None;
        self.remaining_targets = 0;
    }
}

fn reset_challenge(state: &mut impl ResettableChallenge) {
    state.clear_active();
}

fn run_archive(
    mut rng: Option<ResMut<super::RunRng>>,
    mut commands: Commands,
    time: Res<Time>,
    controls: Res<super::skills::SkillRuntime>,
    mut session: ResMut<GameSession>,
    mut archive: ResMut<ArchiveState>,
    mut vitals: ResMut<HeroVitals>,
    mut bonds: ResMut<BondRuntime>,
    mut cues: MessageWriter<EnemyAttackCue>,
    hero: Query<(&Transform, &Visibility), (With<Hero>, Without<ArchiveEcho>)>,
    core: Query<
        &Transform,
        (
            With<super::components::Core>,
            Without<Hero>,
            Without<ArchiveEcho>,
        ),
    >,
    mut echoes: Query<
        (Entity, &Enemy, Option<&TimedSpeedEffect>, &mut Transform),
        With<ArchiveEcho>,
    >,
    assets: Option<Res<GameAssets>>,
    mut pools: ResMut<CombatPools>,
) {
    if !archive.active || !session.save_active || session.finished || session.paused {
        return;
    }

    let delta = time.delta_secs();
    archive.aura_elapsed += delta;
    session.save_remaining = (session.save_remaining - delta).max(0.0);

    if !vitals.alive || vitals.health <= 0.0 {
        // The survival system owns life consumption and revival in the final echo too.
        return;
    }

    if session.save_remaining <= 0.0 {
        if let Some(entity) = archive.echo {
            pools.retire_enemy(&mut commands, entity);
        }
        pools.retire_all_projectiles(&mut commands);
        pools.retire_all_effects(&mut commands);
        archive.resolve(&mut session, ArchiveOutcome::TimedOut);
        return;
    }

    let Ok((echo_entity, echo, speed_effect, mut echo_transform)) = echoes.single_mut() else {
        return;
    };
    let Ok((hero_transform, _hero_visibility)) = hero.single() else {
        return;
    };

    if controls.actions_blocked(echo_entity) {
        return;
    }
    let protecting_gate = core.single().ok().filter(|gate| {
        session.core_hp > 0.0
            && session.gate_revive_remaining <= 0.0
            && hero_transform.translation.x < gate.translation.x
    });
    let target = if let Some(gate) = protecting_gate {
        echo_transform.translation.x = echo_transform.translation.x.max(gate.translation.x + 1.9);
        Vec3::new(gate.translation.x, 0.0, echo_transform.translation.z)
    } else {
        hero_transform.translation
    };
    let to_hero = target - echo_transform.translation;
    let (next, attack_seconds, reached) = super::combat::move_towards_radius(
        echo_transform.translation,
        target,
        echo.speed * speed_effect.map_or(1.0, |effect| effect.multiplier),
        delta,
        ARCHIVE_ATTACK_RANGE,
    );
    echo_transform.translation = next;
    if reached {
        archive.attack_timer -= attack_seconds;
        if archive.attack_timer <= 0.0 {
            archive.attack_timer += ARCHIVE_ATTACK_INTERVAL;
            if protecting_gate.is_some() {
                session.core_hp = (session.core_hp - echo.damage).max(0.0);
            } else {
                if !rng
                    .as_deref_mut()
                    .is_some_and(|rng| bonds.evade(rng.unit()))
                {
                    bonds.take_physical_damage(&mut vitals, echo.damage);
                }
            }
            cues.write(EnemyAttackCue {
                source: echo_entity,
                origin: echo_transform.translation,
                target,
                kind: EnemyAttackKind::Slam,
            });
            if let Some(assets) = assets.as_deref() {
                pools.rent_impact_fx(
                    &mut commands,
                    assets.fx_mesh.clone(),
                    assets.storm_material.clone(),
                    Transform::from_translation(target + Vec3::Y * 0.18)
                        .with_scale(Vec3::splat(0.2)),
                    ImpactFx {
                        lifetime: 0.42,
                        start_scale: 0.2,
                        end_scale: 2.4,
                    },
                );
            }
        }
    }
    echo_transform.translation.y = (archive.aura_elapsed * 2.2).sin() * 0.14;
    if let Some(direction) = to_hero.with_y(0.0).try_normalize() {
        echo_transform.look_to(direction, Vec3::Y);
    }
}

fn animate_archive_aura(
    time: Res<Time>,
    archive: Res<ArchiveState>,
    mut auras: Query<(&ArchiveAura, &mut Transform)>,
) {
    for (aura, mut transform) in &mut auras {
        let pulse = 1.0 + (archive.aura_elapsed * 3.4 + aura.offset).sin() * 0.12;
        transform.scale = Vec3::splat(aura.base_scale * pulse);
        transform.rotate_y(time.delta_secs() * (1.8 + aura.offset * 0.25));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn controlled_enemy_preserves_attack_clock_and_resumes_after_thaw() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<GameSession>()
            .init_resource::<HeroVitals>()
            .init_resource::<super::super::skills::SkillRuntime>()
            .init_resource::<BondRuntime>()
            .init_resource::<super::super::skills::SkillRuntime>()
            .add_message::<EnemyAttackCue>()
            .add_systems(Update, run_archive);
        app.init_resource::<CombatPools>()
            .insert_resource(ArchiveState {
                active: true,
                attack_timer: 0.,
                ..default()
            });
        {
            let mut session = app.world_mut().resource_mut::<GameSession>();
            session.save_active = true;
            session.save_remaining = 60.;
        }
        app.world_mut()
            .spawn((Hero, Transform::from_xyz(-1., 0., 0.), Visibility::Visible));
        let enemy = app
            .world_mut()
            .spawn((
                ArchiveEcho,
                Enemy {
                    hp: 1000.,
                    max_hp: 1000.,
                    speed: 0.,
                    damage: 10.,
                    xp: 0,
                    wood: 0,
                    gold: 0,
                    kill_credit: 0,
                    elite: false,
                },
                Transform::default(),
            ))
            .id();
        app.world_mut()
            .resource_mut::<super::super::skills::SkillRuntime>()
            .freeze_relic(enemy, 2.0, false, false, 10.0);
        let frozen_health = app.world().resource::<HeroVitals>().health;
        let frozen_transform = *app.world().get::<Transform>(enemy).unwrap();
        app.update();
        assert_eq!(app.world().resource::<HeroVitals>().health, frozen_health);
        assert_eq!(app.world().resource::<Messages<EnemyAttackCue>>().len(), 0);
        assert_eq!(
            *app.world().get::<Transform>(enemy).unwrap(),
            frozen_transform
        );
        assert_eq!(app.world().resource::<ArchiveState>().attack_timer, 0.0);
        app.world_mut()
            .insert_resource(super::super::skills::SkillRuntime::default());
        let before = app.world().resource::<HeroVitals>().health;
        app.update();
        assert!(app.world().resource::<HeroVitals>().health < before);
        assert_eq!(
            app.world_mut()
                .resource_mut::<BondRuntime>()
                .take_received_hits(),
            1
        );
        assert_eq!(app.world().resource::<Messages<EnemyAttackCue>>().len(), 1);
        let rotation = app.world().get::<Transform>(enemy).unwrap().rotation;
        app.update();
        assert!(
            rotation
                .angle_between(app.world().get::<Transform>(enemy).unwrap().rotation)
                .abs()
                < 0.001
        );
        let health = app.world().resource::<HeroVitals>().health;
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.update();
        assert_eq!(app.world().resource::<HeroVitals>().health, health);
    }

    #[test]
    fn real_attack_emits_cue_records_received_hit_and_holds_facing() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<GameSession>()
            .init_resource::<HeroVitals>()
            .init_resource::<super::super::skills::SkillRuntime>()
            .init_resource::<BondRuntime>()
            .add_message::<EnemyAttackCue>()
            .add_systems(Update, run_archive);
        app.init_resource::<CombatPools>()
            .insert_resource(ArchiveState {
                active: true,
                attack_timer: 0.,
                ..default()
            });
        {
            let mut session = app.world_mut().resource_mut::<GameSession>();
            session.save_active = true;
            session.save_remaining = 60.;
        }
        app.world_mut()
            .spawn((Hero, Transform::from_xyz(-1., 0., 0.), Visibility::Visible));
        let enemy = app
            .world_mut()
            .spawn((
                ArchiveEcho,
                Enemy {
                    hp: 1000.,
                    max_hp: 1000.,
                    speed: 0.,
                    damage: 10.,
                    xp: 0,
                    wood: 0,
                    gold: 0,
                    kill_credit: 0,
                    elite: false,
                },
                Transform::default(),
            ))
            .id();
        let before = app.world().resource::<HeroVitals>().health;
        app.update();
        assert!(app.world().resource::<HeroVitals>().health < before);
        assert_eq!(
            app.world_mut()
                .resource_mut::<BondRuntime>()
                .take_received_hits(),
            1
        );
        assert_eq!(app.world().resource::<Messages<EnemyAttackCue>>().len(), 1);
        let rotation = app.world().get::<Transform>(enemy).unwrap().rotation;
        app.update();
        assert!(
            rotation
                .angle_between(app.world().get::<Transform>(enemy).unwrap().rotation)
                .abs()
                < 0.001
        );
        let health = app.world().resource::<HeroVitals>().health;
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.update();
        assert_eq!(app.world().resource::<HeroVitals>().health, health);
    }

    #[test]
    fn archive_rules_match_the_gameplay_spec() {
        assert_eq!(ARCHIVE_DURATION, 60.0);
        assert_eq!(ARCHIVE_ECHO_HP, 600_000.0);
    }

    #[test]
    fn every_archive_outcome_completes_the_run() {
        for outcome in [ArchiveOutcome::Captured, ArchiveOutcome::TimedOut] {
            let mut session = GameSession {
                save_active: true,
                ..default()
            };
            let mut archive = ArchiveState {
                active: true,
                ..default()
            };
            archive.resolve(&mut session, outcome);
            assert!(session.finished);
            assert!(session.victory);
            assert!(session.paused);
            assert!(!session.save_active);
            assert_eq!(archive.outcome, outcome);
        }
    }

    #[test]
    fn archive_echo_is_not_a_q_or_w_challenge() {
        let mut world = World::new();
        let echo = world.spawn(ArchiveEcho).id();
        world.spawn(super::super::TrialEnemy {
            level: 1,
            reward: 1,
            kind: super::super::ChallengeKind::Tower,
        });

        let mut trials = world.query_filtered::<Entity, With<super::super::TrialEnemy>>();
        assert!(!trials.iter(&world).any(|entity| entity == echo));
    }
}
