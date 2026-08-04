use bevy::prelude::*;

use super::combat::CombatSet;
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
    HeroFelled,
}

impl ArchiveOutcome {
    pub const fn shards(self) -> u8 {
        match self {
            Self::Pending => 0,
            Self::Captured => 3,
            Self::TimedOut | Self::HeroFelled => 1,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Pending => "NOT REACHED",
            Self::Captured => "ECHO CAPTURED",
            Self::TimedOut => "TIME EXPIRED",
            Self::HeroFelled => "HERO FELLED",
        }
    }
}

#[derive(Resource)]
pub struct ArchiveState {
    pub active: bool,
    pub outcome: ArchiveOutcome,
    pub echo: Option<Entity>,
    pub frozen_core_hp: f32,
    attack_timer: f32,
    aura_elapsed: f32,
}

impl Default for ArchiveState {
    fn default() -> Self {
        Self {
            active: false,
            outcome: ArchiveOutcome::Pending,
            echo: None,
            frozen_core_hp: 1.0,
            attack_timer: 0.35,
            aura_elapsed: 0.0,
        }
    }
}

impl ArchiveState {
    pub fn shards(&self) -> u8 {
        self.outcome.shards()
    }

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
    archive.frozen_core_hp = session.core_hp.max(1.0);
    archive.attack_timer = 0.35;
    archive.aura_elapsed = 0.0;
    session.core_hp = archive.frozen_core_hp;
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
    mut commands: Commands,
    time: Res<Time>,
    mut session: ResMut<GameSession>,
    mut archive: ResMut<ArchiveState>,
    mut vitals: ResMut<HeroVitals>,
    mut hero: Query<(&Transform, &mut Visibility), (With<Hero>, Without<ArchiveEcho>)>,
    mut echoes: Query<(Entity, &Enemy, &mut Transform), With<ArchiveEcho>>,
    assets: Option<Res<GameAssets>>,
    mut pools: ResMut<CombatPools>,
) {
    if !archive.active || !session.save_active || session.finished {
        return;
    }

    let delta = time.delta_secs();
    archive.aura_elapsed += delta;
    session.core_hp = archive.frozen_core_hp.max(1.0);
    session.save_remaining = (session.save_remaining - delta).max(0.0);

    if !vitals.alive {
        if let Some(entity) = archive.echo {
            pools.retire_enemy(&mut commands, entity);
        }
        pools.retire_all_projectiles(&mut commands);
        pools.retire_all_effects(&mut commands);
        archive.resolve(&mut session, ArchiveOutcome::HeroFelled);
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

    let Ok((echo_entity, echo, mut echo_transform)) = echoes.single_mut() else {
        return;
    };
    let Ok((hero_transform, mut hero_visibility)) = hero.single_mut() else {
        return;
    };

    let to_hero = hero_transform.translation - echo_transform.translation;
    let distance = to_hero.length();
    if distance > ARCHIVE_ATTACK_RANGE {
        echo_transform.translation += to_hero.normalize_or_zero() * echo.speed * time.delta_secs();
    } else {
        archive.attack_timer -= delta;
        if archive.attack_timer <= 0.0 {
            archive.attack_timer += ARCHIVE_ATTACK_INTERVAL;
            vitals.take_physical_damage(echo.damage);
            if let Some(assets) = assets.as_deref() {
                pools.rent_impact_fx(
                    &mut commands,
                    assets.fx_mesh.clone(),
                    assets.storm_material.clone(),
                    Transform::from_translation(hero_transform.translation + Vec3::Y * 0.18)
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
    echo_transform.rotate_y(delta * 1.25);

    if vitals.health <= 0.0 {
        vitals.alive = false;
        *hero_visibility = Visibility::Hidden;
        pools.retire_enemy(&mut commands, echo_entity);
        pools.retire_all_projectiles(&mut commands);
        pools.retire_all_effects(&mut commands);
        archive.resolve(&mut session, ArchiveOutcome::HeroFelled);
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
    fn archive_rules_match_the_gameplay_spec() {
        assert_eq!(ARCHIVE_DURATION, 60.0);
        assert_eq!(ARCHIVE_ECHO_HP, 600_000.0);
        assert_eq!(ArchiveOutcome::Captured.shards(), 3);
        assert_eq!(ArchiveOutcome::TimedOut.shards(), 1);
        assert_eq!(ArchiveOutcome::HeroFelled.shards(), 1);
    }

    #[test]
    fn every_archive_outcome_completes_the_run() {
        for outcome in [
            ArchiveOutcome::Captured,
            ArchiveOutcome::TimedOut,
            ArchiveOutcome::HeroFelled,
        ] {
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
