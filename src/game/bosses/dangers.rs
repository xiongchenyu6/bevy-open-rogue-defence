use crate::game::{BossEnemy, Core, Enemy, GameSession, Hero, HeroVitals};
use bevy::prelude::*;

#[derive(Debug, Clone, Copy)]
pub enum BossDangerShape {
    Circle,
    Lane { half_width: f32, length: f32 },
}

/// Exact warning and damage geometry. Visuals must use this component rather
/// than a separately estimated decorative radius.
#[derive(Component, Debug, Clone, Copy)]
pub struct BossTelegraph {
    pub boss: Entity,
    pub index: u8,
    pub kind: u8,
    pub origin: Vec3,
    pub target: Vec3,
    pub remaining: f32,
    pub duration: f32,
    pub radius: f32,
    pub enraged: bool,
    pub shape: BossDangerShape,
    pub active: bool,
    damage: f32,
    active_secs: f32,
    tick_remaining: f32,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct BossPhaseCue {
    pub boss: Entity,
    pub index: u8,
    pub origin: Vec3,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_dangers(
    commands: &mut Commands,
    boss: Entity,
    index: usize,
    origin: Vec3,
    hero: Option<Vec3>,
    gate: Option<Vec3>,
    session: &GameSession,
    variant: u8,
    enraged: bool,
    damage: f32,
    warning: f32,
) {
    let mut target = hero.unwrap_or(origin - Vec3::X * 6.0);
    // Target-lock attacks never teleport their damaging center through a live
    // gate; the wall absorbs the threat while the player remains behind it.
    if let Some(gate) = gate
        && session.core_hp > 0.0
        && session.gate_revive_remaining <= 0.0
        && origin.x >= gate.x
        && origin.z.abs() <= 5.65
    {
        target.x = target.x.max(gate.x + 1.9);
    }
    target.y = 0.0;
    let secondary = !variant.is_multiple_of(2);
    let growth = if enraged { 1.25 } else { 1.0 };
    let (shape, radius, active_secs, center) = match (index, secondary) {
        (0, false) => (
            BossDangerShape::Lane {
                half_width: 1.25 * growth,
                length: 7.0,
            },
            1.25 * growth,
            0.1,
            target,
        ),
        (0, true) => (BossDangerShape::Circle, 2.6 * growth, 3.0, target),
        (1, false) => (
            BossDangerShape::Lane {
                half_width: 2.2 * growth,
                length: 16.0,
            },
            2.2 * growth,
            0.1,
            target,
        ),
        (1, true) => (BossDangerShape::Circle, 3.4 * growth, 2.0, target),
        (2, _) => (BossDangerShape::Circle, 1.7 * growth, 0.1, target),
        (3, _) => (
            BossDangerShape::Lane {
                half_width: 0.8 * growth,
                length: 20.0,
            },
            0.8 * growth,
            0.1,
            target,
        ),
        (4, _) => (BossDangerShape::Circle, 2.8 * growth, 4.0, target),
        _ => unreachable!(),
    };
    let count = if index == 2 && secondary {
        if enraged { 5 } else { 3 }
    } else if index == 3 && secondary {
        if enraged { 3 } else { 2 }
    } else {
        1
    };
    for mark in 0..count {
        let offset = (mark as f32 - (count - 1) as f32 * 0.5) * 2.4;
        let center = center + Vec3::Z * offset;
        commands.spawn((
            Name::new("Boss danger"),
            Transform::default(),
            Visibility::default(),
            BossTelegraph {
                boss,
                index: index as u8,
                kind: u8::from(secondary),
                origin,
                target: center,
                remaining: warning,
                duration: warning,
                radius,
                enraged,
                shape,
                active: false,
                damage: damage * if enraged { 1.6 } else { 1.2 },
                active_secs,
                tick_remaining: 0.0,
            },
        ));
    }
}

fn contains(danger: &BossTelegraph, point: Vec3) -> bool {
    let point = point.xz();
    match danger.shape {
        BossDangerShape::Circle => point.distance(danger.target.xz()) <= danger.radius,
        BossDangerShape::Lane { half_width, length } => {
            let direction = (danger.target.xz() - danger.origin.xz()).normalize_or(Vec2::NEG_X);
            let relative = point - danger.origin.xz();
            let along = relative.dot(direction);
            along >= 0.0 && along <= length && relative.perp_dot(direction).abs() <= half_width
        }
    }
}

fn apply_danger_damage(danger: &BossTelegraph, vitals: &mut HeroVitals) -> f32 {
    if (danger.index == 0 && danger.kind == 0) || danger.index == 2 {
        vitals.take_physical_damage(danger.damage)
    } else {
        vitals.take_magical_damage(danger.damage)
    }
}

fn gate_collision_point(danger: &BossTelegraph, gate: Vec3) -> Vec3 {
    let z = match danger.shape {
        BossDangerShape::Circle => danger.target.z,
        BossDangerShape::Lane { .. } => {
            let direction = danger.target - danger.origin;
            if direction.x.abs() > 0.001 {
                danger.origin.z + direction.z * (gate.x - danger.origin.x) / direction.x
            } else {
                danger.origin.z
            }
        }
    };
    Vec3::new(gate.x, gate.y, z.clamp(-5.65, 5.65))
}

pub(super) fn tick_dangers(
    mut commands: Commands,
    time: Res<Time>,
    controls: Res<crate::game::skills::SkillRuntime>,
    mut session: ResMut<GameSession>,
    mut vitals: ResMut<HeroVitals>,
    bosses: Query<&Enemy, With<BossEnemy>>,
    hero: Query<&Transform, With<Hero>>,
    gate: Query<&Transform, With<Core>>,
    mut dangers: Query<(Entity, &mut BossTelegraph)>,
) {
    for (entity, mut danger) in &mut dangers {
        if session.finished || !bosses.get(danger.boss).is_ok_and(|enemy| enemy.hp > 0.0) {
            commands.entity(entity).despawn();
            continue;
        }
        if session.paused || session.save_active {
            continue;
        }
        // Inactive telegraphs belong to the boss-held pending windup.
        // Pause their warning with pending; released active hazards keep running.
        if !danger.active && controls.actions_blocked(danger.boss) {
            continue;
        }
        danger.remaining -= time.delta_secs();
        if danger.remaining > 0.0 {
            continue;
        }
        if !danger.active {
            danger.active = true;
            danger.remaining = danger.active_secs;
        }
        danger.tick_remaining -= time.delta_secs();
        if danger.tick_remaining <= 0.0 {
            danger.tick_remaining = 0.5;
            let gate_position = gate.single().ok().map(|transform| transform.translation);
            let gate_alive = session.core_hp > 0.0 && session.gate_revive_remaining <= 0.0;
            if let Some(gate) = gate_position
                && gate_alive
                && contains(&danger, gate_collision_point(&danger, gate))
            {
                session.core_hp = (session.core_hp - danger.damage).max(0.0);
            }
            if let Ok(hero) = hero.single() {
                let protected = gate_alive
                    && gate_position.is_some_and(|gate| {
                        danger.origin.z.abs() <= 5.65
                            && hero.translation.z.abs() <= 5.65
                            && danger.origin.x >= gate.x
                            && hero.translation.x < gate.x
                    });
                if !protected && contains(&danger, hero.translation) {
                    let dealt = apply_danger_damage(&danger, &mut vitals);
                    if dealt > 0.0 && danger.index == 1 {
                        vitals.stunned_remaining = vitals.stunned_remaining.max(0.45);
                    }
                }
            }
        }
        if danger.remaining <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn danger(shape: BossDangerShape) -> BossTelegraph {
        BossTelegraph {
            boss: Entity::PLACEHOLDER,
            index: 0,
            kind: 0,
            origin: Vec3::ZERO,
            target: Vec3::X * 10.0,
            remaining: 1.0,
            duration: 1.0,
            radius: 2.0,
            enraged: false,
            shape,
            active: false,
            damage: 100.0,
            active_secs: 0.1,
            tick_remaining: 0.0,
        }
    }
    #[test]
    fn beam_warning_matches_true_forward_strip_and_can_be_dodged() {
        let beam = danger(BossDangerShape::Lane {
            half_width: 0.8,
            length: 20.0,
        });
        assert!(contains(&beam, Vec3::new(15.0, 0.0, 0.7)));
        assert!(!contains(&beam, Vec3::new(15.0, 0.0, 0.9)));
        assert!(!contains(&beam, Vec3::new(-1.0, 0.0, 0.0)));
        assert!(!contains(&beam, Vec3::new(21.0, 0.0, 0.0)));
    }
    #[test]
    fn locked_shadow_mark_does_not_follow_a_dodging_hero() {
        let mark = danger(BossDangerShape::Circle);
        assert!(contains(&mark, Vec3::X * 10.0));
        assert!(!contains(&mark, Vec3::new(10.0, 0.0, 2.1)));
    }
    fn app_with_danger() -> (App, Entity, Entity) {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<GameSession>()
            .init_resource::<HeroVitals>()
            .init_resource::<crate::game::skills::SkillRuntime>()
            .add_systems(Update, tick_dangers);
        let boss = app
            .world_mut()
            .spawn((
                BossEnemy { index: 0 },
                Enemy {
                    hp: 100.0,
                    max_hp: 100.0,
                    speed: 1.0,
                    damage: 20.0,
                    xp: 0,
                    wood: 0,
                    gold: 0,
                    kill_credit: 0,
                    elite: false,
                },
            ))
            .id();
        app.world_mut()
            .spawn((Hero, Transform::from_xyz(10.0, 0.0, 0.0)));
        let mut hazard = danger(BossDangerShape::Circle);
        hazard.boss = boss;
        let hazard = app.world_mut().spawn(hazard).id();
        (app, boss, hazard)
    }
    fn step(app: &mut App, seconds: f32) {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(seconds));
        app.update();
    }
    #[test]
    fn frozen_owner_holds_warning_but_released_hazard_and_death_cleanup_continue() {
        let (mut app, boss, hazard) = app_with_danger();
        app.init_resource::<crate::game::skills::SkillRuntime>();
        app.world_mut()
            .resource_mut::<crate::game::skills::SkillRuntime>()
            .freeze_relic(boss, 1.0, false, false, 10.0);
        step(&mut app, 0.5);
        assert_eq!(
            app.world().get::<BossTelegraph>(hazard).unwrap().remaining,
            1.0
        );
        app.world_mut()
            .get_mut::<BossTelegraph>(hazard)
            .unwrap()
            .active = true;
        step(&mut app, 0.25);
        assert_eq!(
            app.world().get::<BossTelegraph>(hazard).unwrap().remaining,
            0.75
        );
        app.world_mut().get_mut::<Enemy>(boss).unwrap().hp = 0.0;
        step(&mut app, 0.0);
        assert!(app.world().get_entity(hazard).is_err());
    }

    #[test]
    fn warning_pause_resistance_dodge_and_boss_death_are_real_runtime_rules() {
        let (mut app, boss, hazard) = app_with_danger();
        app.world_mut()
            .get_mut::<BossTelegraph>(hazard)
            .unwrap()
            .index = 1;
        let hp = app.world().resource::<HeroVitals>().health;
        step(&mut app, 0.5);
        assert_eq!(app.world().resource::<HeroVitals>().health, hp);
        app.world_mut().resource_mut::<GameSession>().paused = true;
        step(&mut app, 10.0);
        assert_eq!(
            app.world().get::<BossTelegraph>(hazard).unwrap().remaining,
            0.5
        );
        app.world_mut().resource_mut::<GameSession>().paused = false;
        step(&mut app, 0.5);
        assert!((app.world().resource::<HeroVitals>().health - (hp - 90.0)).abs() < 0.01);
        app.world_mut().get_mut::<Enemy>(boss).unwrap().hp = 0.0;
        step(&mut app, 0.1);
        assert!(app.world().get_entity(hazard).is_err());

        let (mut app, _, _) = app_with_danger();
        let mut hero = app
            .world_mut()
            .query_filtered::<&mut Transform, With<Hero>>();
        hero.single_mut(app.world_mut()).unwrap().translation.z = 3.0;
        let hp = app.world().resource::<HeroVitals>().health;
        step(&mut app, 1.0);
        assert_eq!(app.world().resource::<HeroVitals>().health, hp);
    }
    #[test]
    fn live_gate_absorbs_ray_and_does_not_allow_backline_damage() {
        let (mut app, _, hazard) = app_with_danger();
        app.world_mut()
            .spawn((Core, Transform::from_xyz(8.0, 0.0, 0.0)));
        let mut hero = app
            .world_mut()
            .query_filtered::<&mut Transform, With<Hero>>();
        hero.single_mut(app.world_mut()).unwrap().translation.x = 5.0;
        {
            let mut hazard = app.world_mut().get_mut::<BossTelegraph>(hazard).unwrap();
            hazard.origin = Vec3::X * 15.0;
            hazard.target = Vec3::ZERO;
            hazard.shape = BossDangerShape::Lane {
                half_width: 1.0,
                length: 20.0,
            };
        }
        let hp = app.world().resource::<HeroVitals>().health;
        let gate_hp = app.world().resource::<GameSession>().core_hp;
        step(&mut app, 1.0);
        assert_eq!(app.world().resource::<HeroVitals>().health, hp);
        assert_eq!(
            app.world().resource::<GameSession>().core_hp,
            gate_hp - 100.0
        );
    }
    #[test]
    fn invulnerable_hero_cannot_be_stunned_by_undertow() {
        let (mut app, _, hazard) = app_with_danger();
        app.world_mut()
            .get_mut::<BossTelegraph>(hazard)
            .unwrap()
            .index = 1;
        app.world_mut()
            .resource_mut::<HeroVitals>()
            .invulnerable_remaining = 2.0;
        let hp = app.world().resource::<HeroVitals>().health;
        step(&mut app, 1.0);
        let vitals = app.world().resource::<HeroVitals>();
        assert_eq!(vitals.health, hp);
        assert_eq!(vitals.stunned_remaining, 0.0);
    }

    #[test]
    fn run_reset_removes_dangers_even_when_boss_entity_is_reused() {
        let (mut app, boss, hazard) = app_with_danger();
        let child = app.world_mut().spawn(ChildOf(hazard)).id();
        super::super::reset_run(app.world_mut());
        assert!(app.world().get_entity(hazard).is_err());
        assert!(app.world().get_entity(child).is_err());
        assert!(app.world().get_entity(boss).is_ok());
        step(&mut app, 10.0);
        assert_eq!(app.world().resource::<HeroVitals>().stunned_remaining, 0.0);
        let mut query = app
            .world_mut()
            .query_filtered::<Entity, With<BossTelegraph>>();
        assert!(query.iter(app.world()).next().is_none());
    }
    #[test]
    fn charge_and_shadow_use_armor_while_elemental_hazards_use_magic_resistance() {
        let mut vitals = HeroVitals {
            armor: 20.0,
            magic_resistance: 0.25,
            ..default()
        };
        let mut hazard = danger(BossDangerShape::Circle);
        assert_eq!(apply_danger_damage(&hazard, &mut vitals), 50.0);
        hazard.kind = 1;
        assert_eq!(apply_danger_damage(&hazard, &mut vitals), 75.0);
        hazard.index = 2;
        assert_eq!(apply_danger_damage(&hazard, &mut vitals), 50.0);
        for index in [1, 3, 4] {
            hazard.index = index;
            assert_eq!(apply_danger_damage(&hazard, &mut vitals), 75.0);
        }
    }
    #[test]
    fn zero_target_ray_defaults_left_instead_of_hitting_the_entire_map() {
        let mut ray = danger(BossDangerShape::Lane {
            half_width: 0.8,
            length: 20.0,
        });
        ray.target = ray.origin;
        assert!(contains(&ray, Vec3::NEG_X * 10.0));
        assert!(!contains(&ray, Vec3::X * 10.0));
        assert!(!contains(&ray, Vec3::Z * 10.0));
    }
}
