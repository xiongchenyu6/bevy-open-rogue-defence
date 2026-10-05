//! A wide advancing water front. Damage follows its swept geometry, not target timers.
use super::*;
const SPEED: f32 = 14.0;
const DISTANCE: f32 = 24.0;
const HALF_DEPTH: f32 = 0.6;
const MAX_WAVES: usize = 8;
#[derive(Component)]
pub struct TidalWave {
    pub origin: Vec3,
    pub direction: Vec3,
    pub travelled: f32,
    pub half_width: f32,
    pub rank: u8,
    pub evolution: u8,
    pub remaining: f32,
    damage: f32,
    hit: HashSet<Entity>,
}
impl TidalWave {
    fn new(origin: Vec3, direction: Vec3, rank: u8, damage: f32) -> Self {
        let rank = rank.clamp(1, 5);
        Self {
            origin: origin.with_y(0.0),
            direction: direction.with_y(0.0).normalize_or(Vec3::X),
            travelled: 0.0,
            half_width: 6.0 + 0.5 * (rank - 1) as f32,
            rank,
            evolution: 0,
            remaining: DISTANCE / SPEED,
            damage,
            hit: HashSet::new(),
        }
    }
}
pub(super) fn install(app: &mut App) {
    app.add_systems(Update, tick.after(cast_learned_skills));
}
pub(super) fn spawn(
    commands: &mut Commands,
    origin: Vec3,
    direction: Vec3,
    rank: u8,
    damage: f32,
    evolution: u8,
) {
    let mut wave = TidalWave::new(origin, direction, rank, damage);
    wave.evolution = evolution.min(3);
    wave.half_width += wave.evolution as f32 * 0.75;
    let transform = Transform::from_translation(wave.origin)
        .with_rotation(Quat::from_rotation_arc(Vec3::NEG_Z, wave.direction));
    commands.spawn((
        Name::new("Tidal wave front"),
        wave,
        transform,
        Visibility::default(),
    ));
}
fn swept_contact(
    origin: Vec3,
    direction: Vec3,
    point: Vec3,
    from: f32,
    to: f32,
    width: f32,
) -> bool {
    let offset = (point - origin).with_y(0.0);
    let along = offset.dot(direction);
    let side = offset.dot(Vec3::new(-direction.z, 0.0, direction.x)).abs();
    // Never reach behind the caster; the final front cannot exceed travel range.
    along >= 0.0
        && along >= from - HALF_DEPTH
        && along <= to + HALF_DEPTH
        && along <= DISTANCE
        && side <= width
}
#[allow(clippy::too_many_arguments)]
fn tick(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    build: Res<PlayerBuild>,
    vitals: Res<HeroVitals>,
    mut runtime: ResMut<SkillRuntime>,
    mut waves: Query<(Entity, &mut TidalWave, &mut Transform), Without<Enemy>>,
    enemies: Query<
        (
            Entity,
            &Transform,
            &Enemy,
            Option<&BossEnemy>,
            Option<&TrialEnemy>,
            Option<&ArchiveEcho>,
        ),
        Without<TidalWave>,
    >,
) {
    if session.finished || build.skills[3] == 0 {
        for (e, _, _) in &waves {
            commands.entity(e).despawn();
        }
        return;
    }
    if session.paused {
        return;
    }
    let mut ids: Vec<_> = waves.iter().map(|(e, _, _)| e).collect();
    ids.sort_by_key(|e| e.to_bits());
    for (index, entity) in ids.into_iter().enumerate() {
        if index >= MAX_WAVES {
            commands.entity(entity).despawn();
            continue;
        }
        let Ok((_, mut wave, mut transform)) = waves.get_mut(entity) else {
            continue;
        };
        let from = wave.travelled;
        let delta = time.delta_secs().max(0.0).min(wave.remaining);
        wave.remaining = (wave.remaining - delta).max(0.0);
        let to = (from + SPEED * delta).min(DISTANCE);
        wave.travelled = to;
        transform.translation = wave.origin + wave.direction * to;
        let mut marked = Vec::new();
        for (target, t, enemy, boss, trial, archive) in &enemies {
            if enemy.hp <= 0.0
                || wave.hit.contains(&target)
                || !swept_contact(
                    wave.origin,
                    wave.direction,
                    t.translation,
                    from,
                    to,
                    wave.half_width,
                )
            {
                continue;
            }
            wave.hit.insert(target);
            let info = target_info(target, t, enemy, boss, trial, archive);
            schedule_typed_strike(
                &mut commands,
                target,
                wave.damage,
                0.0,
                0.05,
                false,
                0.8,
                DamageProfile::for_skill(3),
            );
            runtime.wet.insert(target, 3.0);
            apply_control(&mut runtime, info, 1.5, 0.0, 0.0);
            marked.push(info);
        }
        apply_static_seals(&mut commands, &mut runtime, marked, &build, &vitals, 0.0);
        if wave.remaining <= 0.0001 || to >= DISTANCE {
            commands.entity(entity).despawn();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    #[test]
    fn slab_sweeps_forward_without_missing_large_steps_or_hitting_behind() {
        for x in [0.0, 1.0, 8.0, 23.9] {
            assert!(swept_contact(
                Vec3::ZERO,
                Vec3::X,
                Vec3::new(x, 0.0, 5.9),
                0.0,
                24.0,
                6.0
            ));
        }
        assert!(!swept_contact(
            Vec3::ZERO,
            Vec3::X,
            Vec3::new(-0.1, 0.0, 0.0),
            0.0,
            24.0,
            6.0
        ));
        assert!(!swept_contact(
            Vec3::ZERO,
            Vec3::X,
            Vec3::new(24.1, 0.0, 0.0),
            0.0,
            24.0,
            6.0
        ));
        assert!(!swept_contact(
            Vec3::ZERO,
            Vec3::X,
            Vec3::new(8.0, 0.0, 6.1),
            0.0,
            24.0,
            6.0
        ));
        assert!(swept_contact(
            Vec3::ZERO,
            Vec3::X,
            Vec3::new(8.0, 0.0, 7.9),
            0.0,
            24.0,
            TidalWave::new(Vec3::ZERO, Vec3::X, 5, 1.0).half_width
        ));
    }
    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<GameSession>()
            .init_resource::<PlayerBuild>()
            .init_resource::<HeroVitals>()
            .init_resource::<SkillRuntime>();
        app.world_mut().resource_mut::<PlayerBuild>().skills[3] = 1;
        app
    }
    fn enemy(app: &mut App, x: f32, z: f32) -> Entity {
        app.world_mut()
            .spawn((
                Enemy {
                    hp: 1000.0,
                    max_hp: 1000.0,
                    speed: 2.0,
                    damage: 0.0,
                    xp: 0,
                    wood: 0,
                    gold: 0,
                    kill_credit: 0,
                    elite: false,
                },
                Transform::from_xyz(x, 0.0, z),
            ))
            .id()
    }
    #[test]
    fn real_wave_waits_for_front_hits_once_marks_wet_and_freezes_during_pause() {
        let mut app = app();
        let target = enemy(&mut app, 8.0, 5.0);
        enemy(&mut app, 8.0, 8.0);
        let wave = app
            .world_mut()
            .spawn((
                TidalWave::new(Vec3::ZERO, Vec3::X, 1, 80.0),
                Transform::default(),
            ))
            .id();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(100));
        app.world_mut().run_system_once(tick).unwrap();
        assert_eq!(
            app.world_mut()
                .query::<&SkillStrike>()
                .iter(app.world())
                .count(),
            0
        );
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(1));
        app.world_mut().run_system_once(tick).unwrap();
        assert!((app.world().get::<TidalWave>(wave).unwrap().travelled - 1.4).abs() < 0.001);
        app.world_mut().resource_mut::<GameSession>().paused = false;
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(600));
        app.world_mut().run_system_once(tick).unwrap();
        let strikes: Vec<_> = app
            .world_mut()
            .query::<&SkillStrike>()
            .iter(app.world())
            .collect();
        assert_eq!(strikes.len(), 1);
        assert_eq!(strikes[0].target, target);
        assert_eq!(strikes[0].damage, 80.0);
        assert_eq!(strikes[0].profile.skill_index, Some(3));
        assert!(
            app.world()
                .resource::<SkillRuntime>()
                .wet
                .contains_key(&target)
        );
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(10));
        app.world_mut().run_system_once(tick).unwrap();
        assert!(app.world().get::<TidalWave>(wave).is_none());
        assert_eq!(
            app.world_mut()
                .query::<&SkillStrike>()
                .iter(app.world())
                .count(),
            1
        );
    }
    #[test]
    fn concurrent_fronts_are_bounded_and_unlearning_removes_them() {
        let mut app = app();
        for _ in 0..20 {
            app.world_mut().spawn((
                TidalWave::new(Vec3::ZERO, Vec3::X, 1, 1.0),
                Transform::default(),
            ));
        }
        app.world_mut().run_system_once(tick).unwrap();
        assert_eq!(
            app.world_mut()
                .query::<&TidalWave>()
                .iter(app.world())
                .count(),
            8
        );
        app.world_mut().resource_mut::<PlayerBuild>().skills[3] = 0;
        app.world_mut().run_system_once(tick).unwrap();
        assert_eq!(
            app.world_mut()
                .query::<&TidalWave>()
                .iter(app.world())
                .count(),
            0
        );
    }
}
