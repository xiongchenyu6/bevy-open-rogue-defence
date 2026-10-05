//! Post-mastery attacks use authoritative typed strikes; visuals cannot deal damage.
use super::super::{bonds::DamageElement, skill_evolution::SkillEvolution};
use super::*;
#[derive(Resource)]
struct EvolutionClock([f32; SkillId::COUNT]);
impl Default for EvolutionClock {
    fn default() -> Self {
        Self([0.0; SkillId::COUNT])
    }
}
#[derive(Resource)]
struct Art {
    mesh: Handle<Mesh>,
    materials: Vec<Handle<StandardMaterial>>,
}
#[derive(Component)]
struct Beam(f32);
pub(super) fn install(app: &mut App) {
    app.init_resource::<EvolutionClock>()
        .add_systems(Startup, setup)
        .add_systems(Update, activate.after(cast_learned_skills))
        .add_systems(PostUpdate, fade.before(TransformSystems::Propagate));
}
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let colors = [
        (1.0, 0.16, 0.025),
        (0.1, 0.8, 1.0),
        (0.55, 0.3, 1.0),
        (0.0, 1.0, 0.6),
        (1.0, 0.85, 0.3),
        (1.0, 0.25, 0.85),
    ];
    commands.insert_resource(Art {
        mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        materials: colors
            .into_iter()
            .map(|(r, g, b)| {
                materials.add(StandardMaterial {
                    base_color: Color::srgb(r, g, b),
                    emissive: LinearRgba::new(r * 7.0, g * 7.0, b * 7.0, 1.0),
                    ..default()
                })
            })
            .collect(),
    });
}
fn draw(
    commands: &mut Commands,
    art: &Art,
    from: Vec3,
    to: Vec3,
    width: f32,
    color: usize,
    capacity: &mut usize,
) {
    if *capacity == 0 {
        return;
    }
    *capacity -= 1;
    let delta = to - from;
    commands.spawn((
        Name::new("Skill ascension beam"),
        Mesh3d(art.mesh.clone()),
        MeshMaterial3d(art.materials[color].clone()),
        Transform::from_translation((from + to) * 0.5 + Vec3::Y * 0.5)
            .with_rotation(Quat::from_rotation_arc(
                Vec3::Z,
                delta.normalize_or(Vec3::Z),
            ))
            .with_scale(Vec3::new(width, width, delta.length().max(0.05))),
        Beam(0.32),
    ));
}
fn activate(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    build: Res<PlayerBuild>,
    vitals: Res<HeroVitals>,
    evolution: Res<SkillEvolution>,
    mut clock: ResMut<EvolutionClock>,
    mut runtime: ResMut<SkillRuntime>,
    mut pools: ResMut<CombatPools>,
    art: Option<Res<Art>>,
    hero: Query<&Transform, With<Hero>>,
    mut enemies: Query<(Entity, &Enemy, &mut Transform), Without<Hero>>,
    bones: Query<(Entity, &Transform), (With<BoneSentry>, Without<Enemy>, Without<Hero>)>,
    waters: Query<&Transform, (With<TideElemental>, Without<Enemy>, Without<Hero>)>,
    beams: Query<(), With<Beam>>,
) {
    if session.paused || session.finished || !vitals.can_act() {
        return;
    }
    let Ok(hero) = hero.single() else {
        return;
    };
    let Some(art) = art else {
        return;
    };
    if evolution.tiers[4] > 0 {
        for (entity, _) in bones.iter().skip(1) {
            pools.retire_summon(&mut commands, entity);
        }
    }
    let mut capacity = 128usize.saturating_sub(beams.iter().count());
    for index in 0..SkillId::COUNT {
        let tier = evolution.tiers[index];
        if tier == 0 || build.skills[index] == 0 || index == 6 || (26..=29).contains(&index) {
            continue;
        }
        clock.0[index] -= time.delta_secs();
        if clock.0[index] > 0.0 {
            continue;
        }
        let origin = if index == 4 {
            bones.iter().next().map(|(_, t)| t.translation)
        } else if index == 16 {
            waters.iter().next().map(|t| t.translation)
        } else {
            Some(hero.translation)
        };
        let Some(origin) = origin else {
            continue;
        };
        let mut candidates: Vec<_> = enemies
            .iter()
            .filter(|(_, e, _)| e.hp > 0.0)
            .map(|(entity, enemy, t)| (entity, t.translation, enemy.speed))
            .collect();
        candidates.sort_by(|a, b| {
            a.1.distance_squared(origin)
                .total_cmp(&b.1.distance_squared(origin))
        });
        let Some(primary) = candidates.first().copied() else {
            continue;
        };
        if primary.1.distance_squared(origin) > 18.0 * 18.0 {
            continue;
        }
        clock.0[index] = (4.0 - f32::from(tier) * 0.5).max(2.0);
        let profile = DamageProfile::for_skill(index);
        let damage = scaled_skill_damage(index, build.skills[index], &build, &vitals, &runtime)
            .max(vitals.attack_damage() * 0.45 + vitals.spell_power() * 0.2)
            * 0.65;
        let direction = (primary.1 - origin).with_y(0.0).normalize_or(Vec3::X);
        let mut from = origin;
        let mut hits = 0;
        for (entity, position, speed) in candidates {
            let offset = (position - origin).with_y(0.0);
            let along = offset.dot(direction);
            let sideways = (offset - direction * along).length();
            let hit = match profile.element {
                DamageElement::Flame => {
                    along >= 0.0
                        && along <= 9.0 + f32::from(tier) * 2.0
                        && sideways <= 0.55 + f32::from(tier) * 0.2
                }
                DamageElement::Tide => {
                    along >= 0.0
                        && along <= 8.0 + f32::from(tier) * 2.0
                        && sideways <= 1.0 + along * 0.22
                }
                DamageElement::Frost => {
                    position.distance_squared(primary.1) <= (2.0 + f32::from(tier) * 0.6).powi(2)
                }
                DamageElement::Physical => {
                    offset.length()
                        <= if index == 4 {
                            3.0 + f32::from(tier) * 0.4
                        } else {
                            5.0 + f32::from(tier)
                        }
                }
                _ => hits < 2 + usize::from(tier),
            };
            if !hit {
                continue;
            }
            hits += 1;
            schedule_typed_strike(
                &mut commands,
                entity,
                damage,
                0.0,
                0.05,
                false,
                0.7,
                profile,
            );
            match profile.element {
                DamageElement::Frost | DamageElement::Storm => {
                    let control = runtime.controls.entry(entity).or_insert(ControlState {
                        base_speed: speed,
                        slow_remaining: 0.0,
                        frozen_remaining: 0.0,
                        stunned_remaining: 0.0,
                    });
                    if profile.element == DamageElement::Frost {
                        control.frozen_remaining =
                            control.frozen_remaining.max(0.6 + f32::from(tier) * 0.2);
                    } else {
                        control.stunned_remaining = control.stunned_remaining.max(0.2);
                    }
                    let color = if profile.element == DamageElement::Frost {
                        1
                    } else {
                        2
                    };
                    draw(
                        &mut commands,
                        &art,
                        from,
                        position,
                        0.09 + f32::from(tier) * 0.025,
                        color,
                        &mut capacity,
                    );
                    from = position;
                }
                DamageElement::Tide => {
                    runtime.wet.insert(entity, 2.0);
                    if let Ok((_, _, mut transform)) = enemies.get_mut(entity) {
                        transform.translation += direction * (0.3 + f32::from(tier) * 0.15);
                    }
                    draw(
                        &mut commands,
                        &art,
                        origin,
                        position,
                        0.15,
                        3,
                        &mut capacity,
                    );
                }
                DamageElement::Physical => {
                    draw(
                        &mut commands,
                        &art,
                        position - direction + Vec3::Y,
                        position + direction,
                        0.16,
                        4,
                        &mut capacity,
                    );
                }
                DamageElement::Flame => {}
                _ => {
                    draw(
                        &mut commands,
                        &art,
                        origin,
                        position,
                        0.12,
                        5,
                        &mut capacity,
                    );
                }
            }
        }
        if profile.element == DamageElement::Flame && hits > 0 {
            draw(
                &mut commands,
                &art,
                origin,
                origin + direction * (9.0 + f32::from(tier) * 2.0),
                0.25 + f32::from(tier) * 0.1,
                0,
                &mut capacity,
            );
        }
    }
}
fn fade(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    mut beams: Query<(Entity, &mut Beam, &mut Transform)>,
) {
    if session.paused && !session.finished {
        return;
    }
    for (entity, mut beam, mut transform) in &mut beams {
        beam.0 -= time.delta_secs();
        if session.finished || beam.0 <= 0.0 {
            commands.entity(entity).despawn();
        } else {
            transform.scale.x *= 0.9;
            transform.scale.y *= 0.9;
        }
    }
}

pub(super) fn reset_run(world: &mut World) {
    world.insert_resource(EvolutionClock::default());
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    #[test]
    fn ascension_visuals_freeze_while_paused_and_are_removed_on_finish() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<GameSession>()
            .add_systems(Update, fade);
        let entity = app
            .world_mut()
            .spawn((Beam(0.3), Transform::default()))
            .id();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.2));
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.update();
        assert_eq!(app.world().get::<Beam>(entity).unwrap().0, 0.3);
        assert_eq!(
            app.world().get::<Transform>(entity).unwrap().scale,
            Vec3::ONE
        );
        app.world_mut().resource_mut::<GameSession>().finished = true;
        app.update();
        assert!(app.world().get_entity(entity).is_err());
    }
    #[test]
    fn flame_ascension_pierces_the_line_but_does_not_damage_off_axis_targets_or_run_while_paused() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<GameSession>()
            .init_resource::<PlayerBuild>()
            .init_resource::<HeroVitals>()
            .init_resource::<SkillEvolution>()
            .init_resource::<EvolutionClock>()
            .init_resource::<SkillRuntime>()
            .init_resource::<CombatPools>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>();
        app.world_mut().run_system_once(setup).unwrap();
        app.world_mut().spawn((Hero, Transform::default()));
        let mut ids = Vec::new();
        for point in [
            Vec3::new(4.0, 0.0, 0.0),
            Vec3::new(6.0, 0.0, 0.0),
            Vec3::new(4.0, 0.0, 4.0),
        ] {
            ids.push(
                app.world_mut()
                    .spawn((
                        Enemy {
                            hp: 100.0,
                            max_hp: 100.0,
                            speed: 1.0,
                            damage: 1.0,
                            xp: 0,
                            wood: 0,
                            gold: 0,
                            kill_credit: 1,
                            elite: false,
                        },
                        Transform::from_translation(point),
                    ))
                    .id(),
            );
        }
        app.world_mut().resource_mut::<PlayerBuild>().skills[1] = 5;
        app.world_mut().resource_mut::<SkillEvolution>().tiers[1] = 1;
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.world_mut().run_system_once(activate).unwrap();
        assert_eq!(
            app.world_mut()
                .query::<&SkillStrike>()
                .iter(app.world())
                .count(),
            0
        );
        app.world_mut().resource_mut::<GameSession>().paused = false;
        app.world_mut().run_system_once(activate).unwrap();
        let targets: Vec<_> = app
            .world_mut()
            .query::<&SkillStrike>()
            .iter(app.world())
            .map(|strike| strike.target)
            .collect();
        assert_eq!(targets.len(), 2);
        assert!(targets.contains(&ids[0]) && targets.contains(&ids[1]));
        assert!(!targets.contains(&ids[2]));
        assert_eq!(
            app.world_mut().query::<&Beam>().iter(app.world()).count(),
            1
        );
    }
}
