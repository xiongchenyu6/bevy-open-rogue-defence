use super::{EliteAffix, EliteAffixKind};
use crate::game::{Enemy, GameSession, Hero, HeroVitals};
use bevy::prelude::*;

pub const YELLOW_SIGN_RADIUS: f32 = 4.5;
pub const BLOODRITE_RADIUS: f32 = 3.5;
const BLOODRITE_HEAL_PER_SECOND: f32 = 0.012;

#[derive(Resource, Default)]
pub struct ElitePressure {
    pub yellow_sign_active: bool,
}
impl ElitePressure {
    pub fn cooldown_recovery_multiplier(&self) -> f32 {
        if self.yellow_sign_active { 0.5 } else { 1.0 }
    }
}

pub(super) fn tick_pressure(
    time: Res<Time>,
    session: Res<GameSession>,
    vitals: Res<HeroVitals>,
    mut pressure: ResMut<ElitePressure>,
    hero: Query<&Transform, With<Hero>>,
    mut enemies: ParamSet<(
        Query<(Entity, &Enemy, &Transform, &EliteAffix), Without<Hero>>,
        Query<(Entity, &mut Enemy, &Transform, Option<&EliteAffix>), Without<Hero>>,
    )>,
) {
    if session.finished {
        pressure.yellow_sign_active = false;
        return;
    }
    if session.paused || session.save_active {
        return;
    }
    let hero = hero.single().ok().map(|transform| transform.translation);
    let mut healers = Vec::new();
    pressure.yellow_sign_active = false;
    for (entity, enemy, transform, affix) in &enemies.p0() {
        if enemy.hp <= 0.0 || affix.is_volatile_pending() {
            continue;
        }
        match affix.kind() {
            EliteAffixKind::YellowSign => {
                if vitals.alive
                    && hero
                        .is_some_and(|hero| within(hero, transform.translation, YELLOW_SIGN_RADIUS))
                {
                    pressure.yellow_sign_active = true;
                }
            }
            EliteAffixKind::Regeneration => healers.push((entity, transform.translation)),
            _ => {}
        }
    }
    if healers.is_empty() {
        return;
    }
    for (entity, mut enemy, transform, affix) in &mut enemies.p1() {
        if enemy.hp <= 0.0 || affix.is_some_and(|affix| affix.is_volatile_pending()) {
            continue;
        }
        if healers
            .iter()
            .any(|(_, healer)| within(*healer, transform.translation, BLOODRITE_RADIUS))
        {
            let healed = if healers.iter().any(|(healer, _)| *healer == entity) {
                bloodrite_healing(enemy.max_hp, time.delta_secs())
            } else {
                18.0 * (1.0 + 0.04 * session.wave.saturating_sub(1).min(24) as f32)
                    * time.delta_secs()
            };
            enemy.hp = (enemy.hp + healed).min(enemy.max_hp);
        }
    }
}
fn within(a: Vec3, b: Vec3, radius: f32) -> bool {
    a.xz().distance_squared(b.xz()) <= radius * radius
}
fn bloodrite_healing(max_hp: f32, seconds: f32) -> f32 {
    max_hp * BLOODRITE_HEAL_PER_SECOND * seconds.max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn yellow_sign_pressure_has_a_real_escape_boundary_and_halves_recovery() {
        assert!(within(Vec3::ZERO, Vec3::X * 4.5, YELLOW_SIGN_RADIUS));
        assert!(!within(Vec3::ZERO, Vec3::X * 4.51, YELLOW_SIGN_RADIUS));
        assert_eq!(ElitePressure::default().cooldown_recovery_multiplier(), 1.0);
        assert_eq!(
            ElitePressure {
                yellow_sign_active: true
            }
            .cooldown_recovery_multiplier(),
            0.5
        );
    }
    #[test]
    fn bloodrite_healing_is_proportional_and_does_not_stack_per_healer() {
        assert_eq!(bloodrite_healing(1_000.0, 0.5), 6.0);
        assert_eq!(bloodrite_healing(1_000.0, -1.0), 0.0);
    }
    fn enemy(max_hp: f32) -> Enemy {
        Enemy {
            hp: max_hp * 0.5,
            max_hp,
            speed: 1.0,
            damage: 10.0,
            xp: 0,
            wood: 0,
            gold: 0,
            kill_credit: 0,
            elite: true,
        }
    }
    #[test]
    fn live_affixes_heal_fixed_amount_without_boss_percent_scaling_and_release_pressure() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<GameSession>()
            .init_resource::<HeroVitals>()
            .init_resource::<ElitePressure>()
            .add_systems(Update, tick_pressure);
        let hero = app
            .world_mut()
            .spawn((Hero, Transform::from_xyz(4.4, 0.0, 0.0)))
            .id();
        let healer = app
            .world_mut()
            .spawn((
                enemy(10_000.0),
                Transform::default(),
                EliteAffix::new(EliteAffixKind::Regeneration),
            ))
            .id();
        app.world_mut().spawn((
            enemy(10_000.0),
            Transform::default(),
            EliteAffix::new(EliteAffixKind::Regeneration),
        ));
        let ally = app
            .world_mut()
            .spawn((enemy(100_000.0), Transform::default()))
            .id();
        let sign = app
            .world_mut()
            .spawn((
                enemy(1_000.0),
                Transform::default(),
                EliteAffix::new(EliteAffixKind::YellowSign),
            ))
            .id();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(1));
        app.update();
        assert_eq!(app.world().get::<Enemy>(healer).unwrap().hp, 5_120.0);
        assert_eq!(app.world().get::<Enemy>(ally).unwrap().hp, 50_018.0);
        assert!(app.world().resource::<ElitePressure>().yellow_sign_active);
        app.world_mut()
            .get_mut::<Transform>(hero)
            .unwrap()
            .translation
            .x = 4.6;
        app.update();
        assert!(!app.world().resource::<ElitePressure>().yellow_sign_active);
        app.world_mut()
            .get_mut::<Transform>(hero)
            .unwrap()
            .translation
            .x = 4.4;
        app.world_mut().get_mut::<Enemy>(sign).unwrap().hp = 0.0;
        app.update();
        assert!(!app.world().resource::<ElitePressure>().yellow_sign_active);
        app.world_mut().resource_mut::<GameSession>().paused = true;
        let hp = app.world().get::<Enemy>(ally).unwrap().hp;
        app.update();
        assert_eq!(app.world().get::<Enemy>(ally).unwrap().hp, hp);
    }
}
