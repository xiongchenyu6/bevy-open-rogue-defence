//! Execution marks carry a finite infection: typed damage, two spread generations.
use super::*;

const MAX_INFECTED: usize = 32;
const MAX_GENERATION: u8 = 2;
const DURATION: f32 = 6.0;

#[derive(Component, Clone, Copy)]
pub(crate) struct Infection {
    rank: u8,
    generation: u8,
    remaining: f32,
    tick_remaining: f32,
    spread_remaining: f32,
    spread: bool,
    damage: f32,
}

impl Infection {
    pub(crate) fn rank(&self) -> u8 {
        self.rank
    }
    pub(crate) fn generation(&self) -> u8 {
        self.generation
    }
    pub(crate) fn remaining(&self) -> f32 {
        self.remaining
    }
    pub(crate) fn new(rank: u8, generation: u8, damage: f32) -> Self {
        Self {
            rank: rank.clamp(1, 5),
            generation,
            remaining: DURATION,
            tick_remaining: 1.0,
            spread_remaining: 2.0,
            spread: false,
            damage,
        }
    }
}

pub(super) fn damage_per_tick(rank: u8, attack_damage: f32) -> f32 {
    let rank = rank.clamp(1, 5) as f32;
    8.0 * rank + attack_damage * (0.14 + rank * 0.04)
}

pub(super) fn update(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    brands: Query<&ExecutionBrand, Added<ExecutionBrand>>,
    enemies: Query<(Entity, &Enemy, &Transform)>,
    mut infected: Query<(Entity, &mut Infection)>,
) {
    if session.finished {
        for (entity, _) in &infected {
            commands.entity(entity).remove::<Infection>();
        }
        return;
    }
    let mut occupied: HashSet<Entity> = infected.iter().map(|(e, _)| e).collect();
    for brand in &brands {
        if occupied.len() >= MAX_INFECTED {
            break;
        }
        if enemies
            .get(brand.target)
            .is_ok_and(|(_, enemy, _)| enemy.hp > 0.0)
            && occupied.insert(brand.target)
        {
            commands.entity(brand.target).insert(Infection::new(
                brand.infection_rank,
                0,
                brand.infection_damage,
            ));
        }
    }
    if session.paused {
        return;
    }
    let delta = time.delta_secs();
    let mut spreads = Vec::new();
    for (entity, mut infection) in &mut infected {
        let Ok((_, enemy, transform)) = enemies.get(entity) else {
            commands.entity(entity).remove::<Infection>();
            continue;
        };
        if enemy.hp <= 0.0 {
            commands.entity(entity).remove::<Infection>();
            continue;
        }
        let active_delta = delta.min(infection.remaining);
        infection.remaining -= active_delta;
        infection.tick_remaining -= active_delta;
        infection.spread_remaining -= active_delta;
        while infection.tick_remaining <= 0.0 {
            schedule_typed_strike(
                &mut commands,
                entity,
                infection.damage,
                0.0,
                0.05,
                false,
                0.55,
                DamageProfile::for_skill(18),
            );
            infection.tick_remaining += 1.0;
        }
        if !infection.spread
            && infection.spread_remaining <= 0.0
            && infection.generation < MAX_GENERATION
        {
            infection.spread = true;
            spreads.push((entity, transform.translation, *infection));
        }
        if infection.remaining <= 0.0 {
            commands.entity(entity).remove::<Infection>();
        }
    }
    spreads.sort_by_key(|(entity, _, _)| entity.to_bits());
    for (_, origin, infection) in spreads {
        let radius = 3.0 + 0.25 * infection.rank as f32;
        let mut candidates: Vec<_> = enemies
            .iter()
            .filter_map(|(entity, enemy, transform)| {
                let distance = origin.distance_squared(transform.translation);
                (enemy.hp > 0.0 && !occupied.contains(&entity) && distance <= radius * radius)
                    .then_some((entity, distance))
            })
            .collect();
        candidates.sort_by(|a, b| {
            a.1.total_cmp(&b.1)
                .then_with(|| a.0.to_bits().cmp(&b.0.to_bits()))
        });
        for (entity, _) in candidates.into_iter().take(2) {
            if occupied.len() >= MAX_INFECTED {
                break;
            }
            occupied.insert(entity);
            commands.entity(entity).insert(Infection::new(
                infection.rank,
                infection.generation + 1,
                infection.damage * 0.8,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(GameSession::default())
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
                0.25,
            )))
            .add_systems(Update, update);
        app
    }
    fn enemy(app: &mut App, x: f32) -> Entity {
        app.world_mut()
            .spawn((
                Enemy {
                    hp: 1000.0,
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
    fn infection_ticks_use_skill_accounting_and_pause_freezes_time() {
        let mut app = app();
        let target = enemy(&mut app, 0.0);
        app.world_mut()
            .entity_mut(target)
            .insert(Infection::new(3, 0, damage_per_tick(3, 100.0)));
        for _ in 0..5 {
            app.update();
        }
        let strikes: Vec<_> = app
            .world_mut()
            .query::<&SkillStrike>()
            .iter(app.world())
            .collect();
        assert_eq!(strikes.len(), 1);
        assert_eq!(strikes[0].target, target);
        assert_eq!(strikes[0].profile.skill_index, Some(18));
        assert!((strikes[0].damage - 50.0).abs() < 0.001);
        let remaining = app.world().get::<Infection>(target).unwrap().remaining;
        app.world_mut().resource_mut::<GameSession>().paused = true;
        for _ in 0..8 {
            app.update();
        }
        assert_eq!(
            app.world().get::<Infection>(target).unwrap().remaining,
            remaining
        );
        assert!(damage_per_tick(5, 100.0) > damage_per_tick(1, 100.0));
    }
    #[test]
    fn infection_spreads_once_to_two_nearby_living_targets_and_stops_at_generation_two() {
        let mut app = app();
        let source = enemy(&mut app, 0.0);
        app.world_mut()
            .entity_mut(source)
            .insert(Infection::new(1, 0, 20.0));
        for i in 1..40 {
            enemy(&mut app, i as f32 * 0.05);
        }
        let dead = enemy(&mut app, 0.01);
        app.world_mut().get_mut::<Enemy>(dead).unwrap().hp = 0.0;
        for _ in 0..10 {
            app.update();
        }
        assert_eq!(
            app.world_mut()
                .query::<&Infection>()
                .iter(app.world())
                .count(),
            3
        );
        assert!(app.world().get::<Infection>(dead).is_none());
        for _ in 0..12 {
            app.update();
        }
        let infections: Vec<_> = app
            .world_mut()
            .query::<&Infection>()
            .iter(app.world())
            .collect();
        assert_eq!(infections.len(), 7);
        assert!(infections.iter().all(|i| i.generation <= 2));
        for _ in 0..12 {
            app.update();
        }
        assert!(
            app.world_mut()
                .query::<&Infection>()
                .iter(app.world())
                .count()
                <= 7
        );
    }
    #[test]
    fn infection_cap_and_expiration_bound_many_simultaneous_marks() {
        let mut app = app();
        for i in 0..40 {
            let target = enemy(&mut app, i as f32 * 0.1);
            app.world_mut().spawn(ExecutionBrand {
                target,
                remaining: 8.0,
                threshold: 0.12,
                infection_rank: 5,
                infection_damage: 50.0,
            });
        }
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&Infection>()
                .iter(app.world())
                .count(),
            MAX_INFECTED
        );
        for _ in 0..45 {
            app.update();
        }
        assert_eq!(
            app.world_mut()
                .query::<&Infection>()
                .iter(app.world())
                .count(),
            0
        );
    }
}
