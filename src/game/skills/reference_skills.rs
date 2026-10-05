//! Public-reference skill identities with explicitly adapted, bounded combat rules.
use super::*;

const RAY: usize = 30;
const MINE: usize = 31;
const MAX_MINES: usize = 6;

pub(super) fn install(app: &mut App) {
    app.add_systems(Startup, setup_art).add_systems(
        Update,
        (cast, tick, falling_ice).chain().after(cast_learned_skills),
    );
}

#[derive(Resource)]
struct Art {
    beam: Handle<Mesh>,
    mine: Handle<Mesh>,
    ice: Handle<Mesh>,
    fire: Handle<StandardMaterial>,
    metal: Handle<StandardMaterial>,
    frost: Handle<StandardMaterial>,
}
fn setup_art(
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };
    commands.insert_resource(Art {
        beam: meshes.add(Cylinder::new(1.0, 1.0)),
        mine: meshes.add(Cylinder::new(0.42, 0.20)),
        ice: meshes.add(Cone::new(0.34, 1.9)),
        fire: materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.35, 0.04),
            emissive: LinearRgba::rgb(10.0, 2.0, 0.1),
            ..default()
        }),
        metal: materials.add(StandardMaterial {
            base_color: Color::srgb(0.28, 0.16, 0.04),
            emissive: LinearRgba::rgb(1.2, 0.25, 0.01),
            metallic: 0.8,
            ..default()
        }),
        frost: materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.85, 1.0),
            emissive: LinearRgba::rgb(0.6, 3.0, 5.0),
            ..default()
        }),
    });
}
#[derive(Component)]
struct Ray {
    origin: Vec3,
    direction: Vec3,
    remaining: f32,
    pulse: f32,
    width: f32,
    damage: f32,
}
#[derive(Component)]
struct Mine {
    remaining: f32,
    armed: f32,
    radius: f32,
    damage: f32,
}
#[derive(Component)]
struct Ice {
    remaining: f32,
    destination: Vec3,
    damage: f32,
}

fn beam_transform(origin: Vec3, end: Vec3, width: f32) -> Transform {
    let delta = end - origin;
    Transform::from_translation((origin + end) * 0.5 + Vec3::Y * 0.65)
        .with_rotation(Quat::from_rotation_arc(Vec3::Y, delta.normalize_or_zero()))
        .with_scale(Vec3::new(width, delta.length().max(0.01), width))
}
fn in_beam(point: Vec3, origin: Vec3, direction: Vec3, length: f32, width: f32) -> bool {
    let offset = (point - origin).with_y(0.0);
    let along = offset.dot(direction);
    along >= 0.0
        && along <= length
        && (offset - direction * along).length_squared() <= width * width
}

#[allow(clippy::too_many_arguments)]
fn cast(
    mut commands: Commands,
    session: Res<GameSession>,
    build: Res<PlayerBuild>,
    vitals: Res<HeroVitals>,
    hero: Query<&Transform, With<Hero>>,
    enemies: Query<(Entity, &Transform, &Enemy), Without<Hero>>,
    mines: Query<(), With<Mine>>,
    rays: Query<(), With<Ray>>,
    mut runtime: ResMut<SkillRuntime>,
    art: Option<Res<Art>>,
    assets: Option<Res<GameAssets>>,
    mut pools: ResMut<CombatPools>,
) {
    if session.paused || session.finished || !vitals.can_act() {
        return;
    }
    let Ok(hero) = hero.single() else { return };
    let origin = hero.translation.with_y(0.0);
    let Some((target, transform, _)) = enemies
        .iter()
        .filter(|(_, t, e)| e.hp > 0.0 && t.translation.distance_squared(origin) < 144.0)
        .min_by(|a, b| {
            a.1.translation
                .distance_squared(origin)
                .total_cmp(&b.1.translation.distance_squared(origin))
                .then_with(|| a.0.to_bits().cmp(&b.0.to_bits()))
        })
    else {
        return;
    };
    let direction = (transform.translation - origin)
        .with_y(0.0)
        .try_normalize()
        .unwrap_or(Vec3::X);
    for index in [RAY, MINE] {
        let rank = build.skills[index];
        if rank == 0
            || !runtime.cast_ready(index)
            || (index == MINE && mines.iter().count() >= MAX_MINES)
            || (index == RAY && !rays.is_empty())
        {
            continue;
        }
        runtime.begin_cast(index);
        let damage = scaled_skill_damage(index, rank, &build, &vitals, &runtime);
        if index == RAY {
            let width = 0.24 + rank as f32 * 0.06;
            let mut entity = commands.spawn((
                Name::new("Flame ray sweeping beam"),
                Ray {
                    origin,
                    direction,
                    remaining: 2.0,
                    pulse: 0.0,
                    width,
                    damage,
                },
                beam_transform(origin, origin + direction * (12.0 * rank_size(rank)), width),
                super::super::skill_lighting::GroundBeam,
            ));
            if let Some(art) = &art {
                entity.insert((Mesh3d(art.beam.clone()), MeshMaterial3d(art.fire.clone())));
            }
        } else {
            // Place in the approach lane; enemies must actually enter the trigger radius.
            let distance = origin.distance(transform.translation).min(4.0);
            let position = origin + direction * distance;
            let mut entity = commands.spawn((
                Name::new("Goblin mine armed trap"),
                Mine {
                    remaining: 15.0,
                    armed: 0.6,
                    radius: skill_effect_radius(MINE, rank).unwrap(),
                    damage,
                },
                Transform::from_translation(position + Vec3::Y * 0.12)
                    .with_scale(Vec3::splat(rank_size(rank))),
            ));
            if let Some(art) = &art {
                entity.insert((Mesh3d(art.mine.clone()), MeshMaterial3d(art.metal.clone())));
            }
        }
        if let Some(assets) = &assets {
            record_spell_and_echo(
                &mut commands,
                assets,
                &mut pools,
                &mut runtime,
                &build,
                SpellRecord {
                    skill_index: index,
                    target,
                    center: transform.translation,
                    damage,
                },
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn tick(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    vitals: Res<HeroVitals>,
    build: Res<PlayerBuild>,
    enemies: Query<(Entity, &Transform, &Enemy), (Without<Ray>, Without<Mine>, Without<Ice>)>,
    mut rays: Query<(Entity, &mut Ray, &mut Transform), (Without<Mine>, Without<Ice>)>,
    mut mines: Query<(Entity, &mut Mine, &Transform), (Without<Ray>, Without<Ice>)>,
    mut ice: Query<(Entity, &mut Ice, &mut Transform), (Without<Ray>, Without<Mine>)>,
) {
    if session.finished {
        for (e, _, _) in &rays {
            commands.entity(e).despawn();
        }
        for (e, _, _) in &mines {
            commands.entity(e).despawn();
        }
        for (e, _, _) in &ice {
            commands.entity(e).despawn();
        }
        return;
    }
    if session.paused {
        return;
    }
    let dt = time.delta_secs();
    for (entity, mut ray, mut transform) in &mut rays {
        if !vitals.can_act() || build.skills[RAY] == 0 {
            commands.entity(entity).despawn();
            continue;
        }
        let active = dt.min(ray.remaining);
        ray.remaining -= active;
        ray.pulse -= active;
        let sweep = (2.0 - ray.remaining) * 0.32 - 0.32;
        let direction = Quat::from_rotation_y(sweep) * ray.direction;
        *transform = beam_transform(
            ray.origin,
            ray.origin + direction * (12.0 * rank_size(build.skills[RAY])),
            ray.width,
        );
        while ray.pulse <= 0.0 {
            ray.pulse += 0.25;
            for (target, t, enemy) in &enemies {
                if enemy.hp > 0.0
                    && in_beam(
                        t.translation,
                        ray.origin,
                        direction,
                        12.0 * rank_size(build.skills[RAY]),
                        ray.width + 0.35,
                    )
                {
                    schedule_typed_strike(
                        &mut commands,
                        target,
                        ray.damage,
                        0.0,
                        0.0,
                        false,
                        0.3,
                        DamageProfile::for_skill(RAY).with_burn(),
                    );
                }
            }
        }
        if ray.remaining <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
    for (entity, mut mine, transform) in &mut mines {
        if build.skills[MINE] == 0 {
            commands.entity(entity).despawn();
            continue;
        }
        mine.remaining -= dt;
        mine.armed -= dt;
        if mine.remaining <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        let origin = transform.translation;
        if mine.armed <= 0.0
            && enemies.iter().any(|(_, t, e)| {
                e.hp > 0.0
                    && (t.translation - origin).with_y(0.0).length_squared() <= 1.2_f32.powi(2)
            })
        {
            schedule_typed_fixed_strike(
                &mut commands,
                origin,
                mine.damage,
                0.0,
                mine.radius,
                true,
                mine.radius,
                DamageProfile::for_skill(MINE).with_burn(),
            );
            commands.entity(entity).despawn();
        }
    }
    for (entity, mut shard, mut transform) in &mut ice {
        shard.remaining -= dt;
        transform.translation =
            shard.destination + Vec3::Y * (0.5 + shard.remaining.max(0.0) * 9.0);
        if shard.remaining <= 0.0 {
            schedule_typed_fixed_strike(
                &mut commands,
                shard.destination,
                shard.damage,
                0.0,
                1.3 * rank_size(build.skills[14]),
                true,
                1.3 * rank_size(build.skills[14]),
                DamageProfile::for_skill(14),
            );
            commands.entity(entity).despawn();
        }
    }
}

fn falling_ice(
    mut commands: Commands,
    session: Res<GameSession>,
    zones: Query<(&SkillZone, &Transform), Added<SkillZone>>,
    art: Option<Res<Art>>,
) {
    if session.paused || session.finished {
        return;
    }
    for (zone, transform) in &zones {
        if zone.kind != ZoneKind::GlacierCore {
            continue;
        }
        for i in 0..5 {
            let angle = i as f32 * std::f32::consts::TAU / 5.0;
            let destination = transform.translation.with_y(0.0)
                + Vec3::new(angle.cos(), 0.0, angle.sin()) * (zone.radius * 0.46);
            let delay = 0.45 + i as f32 * 0.10;
            let mut entity = commands.spawn((
                Name::new("Falling ice damage shard"),
                Ice {
                    remaining: delay,
                    destination,
                    damage: zone.damage_per_pulse * 0.2,
                },
                Transform::from_translation(destination + Vec3::Y * (0.5 + delay * 9.0))
                    .with_rotation(Quat::from_rotation_z(std::f32::consts::PI)),
            ));
            if let Some(art) = &art {
                entity.insert((Mesh3d(art.ice.clone()), MeshMaterial3d(art.frost.clone())));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ray_hits_only_the_actual_segment_and_width() {
        assert!(in_beam(
            Vec3::new(5.0, 0.0, 0.2),
            Vec3::ZERO,
            Vec3::X,
            12.0,
            0.4
        ));
        assert!(!in_beam(
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::ZERO,
            Vec3::X,
            12.0,
            0.4
        ));
        assert!(!in_beam(
            Vec3::new(13.0, 0.0, 0.0),
            Vec3::ZERO,
            Vec3::X,
            12.0,
            0.4
        ));
        assert!(!in_beam(
            Vec3::new(5.0, 0.0, 1.0),
            Vec3::ZERO,
            Vec3::X,
            12.0,
            0.4
        ));
    }
    #[test]
    fn ray_transform_has_matching_center_and_length() {
        let t = beam_transform(Vec3::ZERO, Vec3::X * 12.0, 0.3);
        assert_eq!(t.translation, Vec3::new(6.0, 0.65, 0.0));
        assert!((t.rotation * Vec3::Y - Vec3::X).length() < 0.001);
        assert_eq!(t.scale.y, 12.0);
    }
    fn test_app() -> App {
        let mut app = App::new();
        let mut build = PlayerBuild::default();
        build.skills[RAY] = 1;
        build.skills[MINE] = 1;
        app.add_plugins(MinimalPlugins)
            .insert_resource(build)
            .insert_resource(GameSession::default())
            .insert_resource(HeroVitals::default())
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_secs_f32(0.25),
            ))
            .add_systems(Update, tick);
        app
    }
    fn enemy(app: &mut App, x: f32, hp: f32) -> Entity {
        app.world_mut()
            .spawn((
                Enemy {
                    hp,
                    max_hp: 1000.0,
                    speed: 0.0,
                    damage: 0.0,
                    xp: 0,
                    wood: 0,
                    gold: 0,
                    kill_credit: 1,
                    elite: false,
                },
                Transform::from_xyz(x, 0.0, 0.0),
            ))
            .id()
    }
    #[test]
    fn mine_waits_for_arming_and_a_living_nearby_target_then_explodes_once() {
        let mut app = test_app();
        let target = enemy(&mut app, 4.0, 100.0);
        let mine = app
            .world_mut()
            .spawn((
                Mine {
                    remaining: 15.0,
                    armed: 0.6,
                    radius: 3.0,
                    damage: 99.0,
                },
                Transform::default(),
            ))
            .id();
        for _ in 0..5 {
            app.update();
        }
        assert!(app.world().get::<Mine>(mine).is_some());
        app.world_mut()
            .get_mut::<Transform>(target)
            .unwrap()
            .translation
            .x = 0.5;
        app.world_mut().get_mut::<Enemy>(target).unwrap().hp = 0.0;
        app.update();
        assert!(app.world().get::<Mine>(mine).is_some());
        app.world_mut().get_mut::<Enemy>(target).unwrap().hp = 100.0;
        app.update();
        app.update();
        assert!(app.world().get::<Mine>(mine).is_none());
        let strikes: Vec<_> = app
            .world_mut()
            .query::<&SkillStrike>()
            .iter(app.world())
            .collect();
        assert_eq!(strikes.len(), 1);
        assert_eq!(strikes[0].profile.skill_index, Some(31));
        assert!(strikes[0].area);
        assert_eq!(strikes[0].damage, 99.0);
    }
    #[test]
    fn reference_effects_freeze_when_paused_and_finish_cleans_traps() {
        let mut app = test_app();
        let mine = app
            .world_mut()
            .spawn((
                Mine {
                    remaining: 15.0,
                    armed: 0.6,
                    radius: 3.0,
                    damage: 99.0,
                },
                Transform::default(),
            ))
            .id();
        app.update();
        let remaining = app.world().get::<Mine>(mine).unwrap().remaining;
        app.world_mut().resource_mut::<GameSession>().paused = true;
        for _ in 0..10 {
            app.update();
        }
        assert_eq!(app.world().get::<Mine>(mine).unwrap().remaining, remaining);
        app.world_mut().resource_mut::<GameSession>().finished = true;
        app.update();
        assert!(app.world().get::<Mine>(mine).is_none());
    }
    #[test]
    fn replacing_ray_and_mine_retires_old_damage_sources() {
        let mut app = test_app();
        enemy(&mut app, 0.5, 100.0);
        let ray = app
            .world_mut()
            .spawn((
                Ray {
                    origin: Vec3::ZERO,
                    direction: Vec3::X,
                    remaining: 2.0,
                    pulse: 0.0,
                    width: 3.0,
                    damage: 17.0,
                },
                Transform::default(),
            ))
            .id();
        let mine = app
            .world_mut()
            .spawn((
                Mine {
                    remaining: 15.0,
                    armed: 0.0,
                    radius: 3.0,
                    damage: 99.0,
                },
                Transform::default(),
            ))
            .id();
        app.world_mut().resource_mut::<PlayerBuild>().skills.fill(0);
        app.update();
        assert!(app.world().get_entity(ray).is_err());
        assert!(app.world().get_entity(mine).is_err());
        assert_eq!(
            app.world_mut()
                .query::<&SkillStrike>()
                .iter(app.world())
                .count(),
            0
        );
    }
    #[test]
    fn ray_damage_uses_real_targets_and_skill_profile() {
        let mut app = test_app();
        let hit = enemy(&mut app, 5.0, 100.0);
        let miss = enemy(&mut app, -5.0, 100.0);
        app.world_mut().spawn((
            Ray {
                origin: Vec3::ZERO,
                direction: Vec3::X,
                remaining: 2.0,
                pulse: 0.0,
                width: 3.0,
                damage: 17.0,
            },
            Transform::default(),
        ));
        app.update();
        let strikes: Vec<_> = app
            .world_mut()
            .query::<&SkillStrike>()
            .iter(app.world())
            .collect();
        assert!(!strikes.is_empty());
        assert!(strikes.iter().all(|s| s.target == hit
            && s.target != miss
            && s.profile.skill_index == Some(30)
            && s.damage == 17.0));
    }
}
