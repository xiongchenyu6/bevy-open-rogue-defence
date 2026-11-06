//! Moment-to-moment impact feel: enemy hit squash, kill bursts, screen shake,
//! a short hit-stop on heavy kills and a red edge flash when the hero is hurt.
//!
//! `projectile_hits` is the single damage entry point, so it records one
//! [`HitEvent`] per landed hit. Every consumer reads the same frame of events
//! and the queue is cleared in `Last`.

use std::collections::HashMap;

use bevy::prelude::*;

mod enemy_flash;

#[cfg(any(debug_assertions, feature = "dev-tools"))]
#[path = "hit_feedback/capture.rs"]
pub mod capture;

use super::bonds::DamageElement;
use super::combat::{CombatSet, SurvivalSet};
use super::gate_feedback::GateImpact;
use super::pooling::{CombatPools, IMPACT_FX_POOL_CAPACITY, damage_element_color};
use super::{Autoplay, Enemy, GameSession, Hero, HeroVisual, HeroVitals, ImpactFx};

const MAX_EVENTS_PER_FRAME: usize = 256;
const PUNCH_SECONDS: f32 = 0.12;
const MAX_SHAKE_OFFSET: f32 = 0.38;
const SHAKE_DECAY_PER_SECOND: f32 = 2.4;
/// Hit-stop is a 1x-2x feel tool; fast-forwarded battles skip it.
const HIT_STOP_MAX_BATTLE_SPEED: f32 = 2.0;
const HIT_STOP_TIME_SCALE: f32 = 0.06;
const HIT_STOP_COOLDOWN_SECONDS: f32 = 0.5;
const HURT_THRESHOLD_FRACTION: f32 = 0.01;
const HURT_FLASH_SECONDS: f32 = 0.38;
const FLINCH_SECONDS: f32 = 0.34;
const FLINCH_PEAK_AT: f32 = 0.16;
const FLINCH_PUSH: f32 = 0.45;
const FLINCH_TILT: f32 = 0.42;
/// Enemies this close are candidates for "who hit the hero".
const ATTACKER_SEARCH_RADIUS: f32 = 9.0;
/// Cosmetic bursts never take the last impact shells from skill effects.
const BURST_POOL_HEADROOM: usize = IMPACT_FX_POOL_CAPACITY / 2;

pub(crate) struct HitFeedbackPlugin;

/// Runs after combat resolves; audio reads the same frame's events after it.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct HitFeedbackSet;

impl Plugin for HitFeedbackPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HitFeedback>()
            .init_resource::<enemy_flash::EnemyFlash>()
            .init_resource::<ScreenShake>()
            .init_resource::<HitStop>()
            .configure_sets(Update, HitFeedbackSet.after(CombatSet).after(SurvivalSet))
            .add_systems(Startup, (setup_burst_art, spawn_hurt_flash))
            .add_systems(
                Update,
                (
                    detect_hero_hurt,
                    react_to_hits,
                    enemy_flash::collect,
                    start_hero_flinch,
                    animate_hero_flinch,
                    animate_hit_punch,
                    apply_hit_stop,
                    apply_screen_shake,
                    animate_hurt_flash,
                )
                    .chain()
                    .in_set(HitFeedbackSet),
            )
            .add_systems(PostUpdate, enemy_flash::animate)
            .add_systems(Last, clear_frame_events);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HitWeight {
    Normal,
    Elite,
    Boss,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct HitEvent {
    pub(crate) target: Entity,
    pub(crate) position: Vec3,
    pub(crate) element: DamageElement,
    pub(crate) weight: HitWeight,
    pub(crate) critical: bool,
    pub(crate) killed: bool,
}

/// Landed hits and hero damage observed this frame.
#[derive(Resource, Default, Debug)]
pub(crate) struct HitFeedback {
    events: Vec<HitEvent>,
    hero_hurt: bool,
    /// Horizontal direction the hero is knocked towards (away from the attacker).
    hero_push: Vec3,
    previous_hero_health: Option<f32>,
}

impl HitFeedback {
    pub(crate) fn push(&mut self, event: HitEvent) {
        if self.events.len() < MAX_EVENTS_PER_FRAME {
            self.events.push(event);
        }
    }

    pub(crate) fn events(&self) -> &[HitEvent] {
        &self.events
    }

    pub(crate) const fn hero_hurt(&self) -> bool {
        self.hero_hurt
    }
}

#[derive(Resource, Default, Debug)]
struct ScreenShake {
    trauma: f32,
    applied: Vec3,
}

impl ScreenShake {
    fn add(&mut self, amount: f32) {
        self.trauma = (self.trauma + amount).min(1.0);
    }
}

#[derive(Resource, Default, Debug)]
struct HitStop {
    remaining: f32,
    cooldown: f32,
    restore_speed: f32,
    applied_speed: f32,
    requested: f32,
}

#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct HitPunch {
    elapsed: f32,
    presented: bool,
    base_scale: Vec3,
    strength: f32,
}

#[derive(Resource)]
struct BurstArt {
    ring: Handle<Mesh>,
    spark: Handle<Mesh>,
    element: [Handle<StandardMaterial>; 7],
    critical: Handle<StandardMaterial>,
    hurt: Handle<StandardMaterial>,
}

/// Visual-only knock-back on the hero model; the `Hero` transform is gameplay.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct HeroFlinch {
    elapsed: f32,
    push: Vec3,
}

#[derive(Component)]
struct HurtFlash {
    remaining: f32,
}

const fn element_index(element: DamageElement) -> usize {
    match element {
        DamageElement::Physical => 0,
        DamageElement::Storm => 1,
        DamageElement::Flame => 2,
        DamageElement::Frost => 3,
        DamageElement::Tide => 4,
        DamageElement::Arcane => 5,
        DamageElement::Pure => 6,
    }
}

const ELEMENTS: [DamageElement; 7] = [
    DamageElement::Physical,
    DamageElement::Storm,
    DamageElement::Flame,
    DamageElement::Frost,
    DamageElement::Tide,
    DamageElement::Arcane,
    DamageElement::Pure,
];

fn glow_material(color: Color, strength: f32) -> StandardMaterial {
    let linear = color.to_linear();
    StandardMaterial {
        base_color: color.with_alpha(0.7),
        emissive: LinearRgba::rgb(
            linear.red * strength,
            linear.green * strength,
            linear.blue * strength,
        ),
        unlit: true,
        alpha_mode: AlphaMode::Add,
        ..default()
    }
}

fn setup_burst_art(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let element =
        ELEMENTS.map(|element| materials.add(glow_material(damage_element_color(element), 3.2)));
    commands.insert_resource(BurstArt {
        ring: meshes.add(Torus::new(0.9, 1.0)),
        spark: meshes.add(
            Sphere::new(0.5)
                .mesh()
                .ico(1)
                .expect("ico subdivisions are valid"),
        ),
        element,
        critical: materials.add(glow_material(Color::srgb(1.0, 0.82, 0.18), 5.0)),
        hurt: materials.add(glow_material(Color::srgb(1.0, 0.16, 0.1), 4.0)),
    });
}

fn spawn_hurt_flash(mut commands: Commands) {
    commands.spawn((
        Name::new("HERO HURT FLASH"),
        HurtFlash { remaining: 0.0 },
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            border: UiRect::all(px(26)),
            ..default()
        },
        BorderColor::all(Color::srgba(0.9, 0.05, 0.04, 0.0)),
        GlobalZIndex(80),
        Pickable::IGNORE,
    ));
}

fn detect_hero_hurt(
    vitals: Option<Res<HeroVitals>>,
    heroes: Query<&GlobalTransform, With<Hero>>,
    enemies: Query<&GlobalTransform, With<Enemy>>,
    mut feedback: ResMut<HitFeedback>,
) {
    let Some(vitals) = vitals else {
        return;
    };
    let health = vitals.health;
    if let Some(previous) = feedback.previous_hero_health {
        let threshold = vitals.max_health.max(1.0) * HURT_THRESHOLD_FRACTION;
        if vitals.alive && previous - health > threshold {
            feedback.hero_hurt = true;
            if let Some(hero) = heroes.iter().next() {
                let attackers = enemies.iter().map(GlobalTransform::translation);
                feedback.hero_push = knockback_direction(hero.translation(), attackers);
            }
        }
    }
    feedback.previous_hero_health = Some(health);
}

/// Away from the nearest enemy; invaders come from +X, so default to -X.
fn knockback_direction(hero: Vec3, attackers: impl Iterator<Item = Vec3>) -> Vec3 {
    attackers
        .map(|enemy| Vec3::new(hero.x - enemy.x, 0.0, hero.z - enemy.z))
        .filter(|away| away.length_squared() <= ATTACKER_SEARCH_RADIUS * ATTACKER_SEARCH_RADIUS)
        .min_by(|left, right| left.length_squared().total_cmp(&right.length_squared()))
        .and_then(Vec3::try_normalize)
        .unwrap_or(Vec3::NEG_X)
}

#[allow(clippy::too_many_arguments)]
fn react_to_hits(
    mut commands: Commands,
    feedback: Res<HitFeedback>,
    session: Res<GameSession>,
    autoplay: Option<Res<Autoplay>>,
    art: Option<Res<BurstArt>>,
    mut pools: ResMut<CombatPools>,
    mut shake: ResMut<ScreenShake>,
    mut hit_stop: ResMut<HitStop>,
    gate: Option<Res<GateImpact>>,
    mut enemies: Query<(&Transform, Option<&mut HitPunch>)>,
) {
    if session.finished {
        return;
    }
    // Commands are deferred: collect once per target so a later ordinary hit
    // cannot overwrite a same-frame critical punch before it is inserted.
    let mut punches = HashMap::<Entity, f32>::new();
    for event in feedback.events().iter().filter(|event| !event.killed) {
        punches
            .entry(event.target)
            .and_modify(|strength| *strength = strength.max(punch_strength(*event)))
            .or_insert_with(|| punch_strength(*event));
    }
    for (target, strength) in punches {
        if let Ok((transform, punch)) = enemies.get_mut(target) {
            match punch {
                Some(mut punch) => {
                    // Complete impact -> overshoot -> recovery even under a
                    // continuous barrage. Restarting at zero erases the impact.
                    punch.strength = punch.strength.max(strength);
                }
                None => {
                    commands.entity(target).insert(HitPunch {
                        elapsed: 0.0,
                        presented: false,
                        base_scale: transform.scale,
                        strength,
                    });
                }
            }
        }
    }
    let mut frame_trauma = 0.0_f32;
    for event in feedback.events() {
        if let Some(art) = art.as_deref()
            && pools.impact_fx_metrics().active() < BURST_POOL_HEADROOM
        {
            spawn_burst(&mut commands, &mut pools, art, *event);
        }
        if autoplay.is_none() {
            frame_trauma = frame_trauma.max(shake_trauma(*event));
            hit_stop.requested = hit_stop.requested.max(hit_stop_seconds(*event));
        }
    }
    if autoplay.is_none() && feedback.hero_hurt() {
        frame_trauma = frame_trauma.max(0.32);
    }
    if let Some(strength) = gate.as_deref().and_then(GateImpact::struck)
        && autoplay.is_none()
    {
        frame_trauma = frame_trauma.max(0.16 * strength * strength);
    }
    // A volley has one readable impact, not one camera impulse per projectile.
    // Keep the strongest hero, gate or enemy impact in this frame.
    shake.add(frame_trauma);
}

fn punch_strength(event: HitEvent) -> f32 {
    let base = match event.weight {
        HitWeight::Normal => 1.0,
        HitWeight::Elite => 0.7,
        HitWeight::Boss => 0.35,
    };
    if event.critical { base * 1.6 } else { base }
}

fn shake_trauma(event: HitEvent) -> f32 {
    match (event.killed, event.weight, event.critical) {
        (true, HitWeight::Boss, _) => 1.0,
        (true, HitWeight::Elite, _) => 0.45,
        (false, HitWeight::Boss, true) => 0.16,
        (false, HitWeight::Boss, false) => 0.05,
        (_, _, true) => 0.07,
        _ => 0.0,
    }
}

fn hit_stop_seconds(event: HitEvent) -> f32 {
    match (event.killed, event.weight) {
        (true, HitWeight::Boss) => 0.16,
        (true, HitWeight::Elite) => 0.055,
        _ => 0.0,
    }
}

fn spawn_burst(commands: &mut Commands, pools: &mut CombatPools, art: &BurstArt, event: HitEvent) {
    let ground = Vec3::new(event.position.x, 0.22, event.position.z);
    let body = Vec3::new(event.position.x, 0.9, event.position.z);
    if event.critical {
        pools.rent_impact_fx(
            commands,
            art.spark.clone(),
            art.critical.clone(),
            Transform::from_translation(body).with_scale(Vec3::splat(0.35)),
            ImpactFx {
                lifetime: 0.16,
                start_scale: 0.35,
                end_scale: 1.2,
            },
        );
    }
    if !event.killed {
        return;
    }
    let material = art.element[element_index(event.element)].clone();
    let (lifetime, end_scale) = match event.weight {
        HitWeight::Normal => (0.3, 1.9),
        HitWeight::Elite => (0.42, 3.4),
        HitWeight::Boss => (0.6, 7.5),
    };
    pools.rent_impact_fx(
        commands,
        art.ring.clone(),
        material.clone(),
        Transform::from_translation(ground).with_scale(Vec3::splat(0.4)),
        ImpactFx {
            lifetime,
            start_scale: 0.4,
            end_scale,
        },
    );
    pools.rent_impact_fx(
        commands,
        art.spark.clone(),
        material,
        Transform::from_translation(body).with_scale(Vec3::splat(0.9)),
        ImpactFx {
            lifetime: lifetime * 0.6,
            start_scale: end_scale * 0.45,
            end_scale: 0.05,
        },
    );
}

fn start_hero_flinch(
    mut commands: Commands,
    feedback: Res<HitFeedback>,
    session: Res<GameSession>,
    art: Option<Res<BurstArt>>,
    mut pools: ResMut<CombatPools>,
    heroes: Query<&GlobalTransform, With<Hero>>,
    mut visuals: Query<(Entity, Option<&mut HeroFlinch>), With<HeroVisual>>,
) {
    if !feedback.hero_hurt() || session.finished {
        return;
    }
    for (entity, flinch) in &mut visuals {
        match flinch {
            Some(mut flinch) => {
                flinch.elapsed = flinch.elapsed.min(FLINCH_PEAK_AT * 0.5);
                flinch.push = feedback.hero_push;
            }
            None => {
                commands.entity(entity).insert(HeroFlinch {
                    elapsed: 0.0,
                    push: feedback.hero_push,
                });
            }
        }
    }
    if let (Some(art), Some(hero)) = (art.as_deref(), heroes.iter().next()) {
        let body = hero.translation() + Vec3::Y * 0.9 - feedback.hero_push * 0.35;
        pools.rent_impact_fx(
            &mut commands,
            art.spark.clone(),
            art.hurt.clone(),
            Transform::from_translation(body).with_scale(Vec3::splat(0.4)),
            ImpactFx {
                lifetime: 0.2,
                start_scale: 0.4,
                end_scale: 1.4,
            },
        );
    }
}

/// Snap back from the blow, then recover with an ease-out.
fn flinch_curve(elapsed: f32) -> f32 {
    if elapsed < FLINCH_PEAK_AT {
        (elapsed / FLINCH_PEAK_AT * std::f32::consts::FRAC_PI_2).sin()
    } else {
        let t = ((elapsed - FLINCH_PEAK_AT) / (FLINCH_SECONDS - FLINCH_PEAK_AT)).clamp(0.0, 1.0);
        (1.0 - t) * (1.0 - t)
    }
}

/// Offset and lean of the hero model in the hero's local space.
fn flinch_pose(push_world: Vec3, hero_rotation: Quat, elapsed: f32) -> (Vec3, Quat) {
    let amount = flinch_curve(elapsed);
    let push = hero_rotation.inverse() * push_world;
    let axis = Vec3::Y.cross(push).try_normalize().unwrap_or(Vec3::Z);
    (
        push * FLINCH_PUSH * amount,
        Quat::from_axis_angle(axis, FLINCH_TILT * amount),
    )
}

fn animate_hero_flinch(
    mut commands: Commands,
    time: Res<Time<Real>>,
    session: Res<GameSession>,
    heroes: Query<&GlobalTransform, With<Hero>>,
    mut visuals: Query<(Entity, &mut Transform, &mut HeroFlinch), With<HeroVisual>>,
) {
    let delta = if session.paused {
        0.0
    } else {
        time.delta_secs()
    };
    let hero_rotation = heroes
        .iter()
        .next()
        .map_or(Quat::IDENTITY, |hero| hero.compute_transform().rotation);
    for (entity, mut transform, mut flinch) in &mut visuals {
        flinch.elapsed += delta;
        if flinch.elapsed >= FLINCH_SECONDS {
            transform.translation = Vec3::ZERO;
            transform.rotation = Quat::IDENTITY;
            commands.entity(entity).remove::<HeroFlinch>();
            continue;
        }
        let (offset, lean) = flinch_pose(flinch.push, hero_rotation, flinch.elapsed);
        transform.translation = offset;
        transform.rotation = lean;
    }
}

/// Squash on impact, overshoot, then settle back to the authored scale.
fn punch_scale(base: Vec3, elapsed: f32, strength: f32) -> Vec3 {
    let t = (elapsed / PUNCH_SECONDS).clamp(0.0, 1.0);
    let wave = (t * std::f32::consts::PI * 1.5).sin() * (1.0 - t);
    let squash = 0.22 * strength * wave;
    Vec3::new(
        base.x * (1.0 + squash * 0.8),
        base.y * (1.0 - squash),
        base.z * (1.0 + squash * 0.8),
    )
}

fn animate_hit_punch(
    mut commands: Commands,
    time: Res<Time<Real>>,
    session: Res<GameSession>,
    mut punched: Query<(Entity, &mut Transform, &mut HitPunch)>,
) {
    let delta = if session.paused {
        0.0
    } else {
        time.delta_secs()
    };
    for (entity, mut transform, mut punch) in &mut punched {
        if !session.paused && !punch.presented {
            // A long contact frame must render impact before aging through the
            // whole reaction. Later hits merge strength without rewinding it.
            punch.elapsed = PUNCH_SECONDS * 0.2;
            punch.presented = true;
        } else {
            punch.elapsed += delta;
        }
        if punch.elapsed >= PUNCH_SECONDS {
            transform.scale = punch.base_scale;
            commands.entity(entity).remove::<HitPunch>();
        } else {
            transform.scale = punch_scale(punch.base_scale, punch.elapsed, punch.strength);
        }
    }
}

fn apply_hit_stop(
    time: Res<Time<Real>>,
    session: Res<GameSession>,
    mut virtual_time: ResMut<Time<Virtual>>,
    mut hit_stop: ResMut<HitStop>,
) {
    let delta = time.delta_secs();
    hit_stop.cooldown = (hit_stop.cooldown - delta).max(0.0);
    let requested = std::mem::take(&mut hit_stop.requested);
    if hit_stop.remaining > 0.0 {
        hit_stop.remaining -= delta;
        if hit_stop.remaining <= 0.0 {
            // Battle-speed changes during the freeze win over the saved speed.
            if virtual_time.relative_speed() == hit_stop.applied_speed {
                virtual_time.set_relative_speed(hit_stop.restore_speed);
            }
            hit_stop.cooldown = HIT_STOP_COOLDOWN_SECONDS;
        }
        return;
    }
    let speed = virtual_time.relative_speed();
    if requested <= 0.0
        || hit_stop.cooldown > 0.0
        || session.paused
        || session.finished
        || speed > HIT_STOP_MAX_BATTLE_SPEED
    {
        return;
    }
    hit_stop.remaining = requested;
    hit_stop.restore_speed = speed;
    hit_stop.applied_speed = speed * HIT_STOP_TIME_SCALE;
    virtual_time.set_relative_speed(hit_stop.applied_speed);
}

fn shake_offset(trauma: f32, seconds: f32, right: Vec3, up: Vec3) -> Vec3 {
    let amount = trauma * trauma * MAX_SHAKE_OFFSET;
    let x = (seconds * 61.0).sin() * 0.6 + (seconds * 37.0 + 1.3).sin() * 0.4;
    let y = (seconds * 53.0 + 0.7).sin() * 0.6 + (seconds * 29.0 + 2.1).sin() * 0.4;
    (right * x + up * y) * amount
}

/// Removes last frame's offset before adding the new one so captures and other
/// camera writers keep ownership of the base transform.
fn apply_screen_shake(
    time: Res<Time<Real>>,
    session: Res<GameSession>,
    mut shake: ResMut<ScreenShake>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
) {
    let Some(mut transform) = cameras.iter_mut().next() else {
        return;
    };
    let previous = shake.applied;
    transform.translation -= previous;
    if session.paused {
        shake.applied = Vec3::ZERO;
        return;
    }
    let offset = shake_offset(
        shake.trauma,
        time.elapsed_secs(),
        *transform.right(),
        *transform.up(),
    );
    transform.translation += offset;
    shake.applied = offset;
    // Show the contact impulse before decaying it. At 30 Hz a small critical
    // impulse otherwise disappears entirely before its first visible frame.
    shake.trauma = (shake.trauma - SHAKE_DECAY_PER_SECOND * time.delta_secs()).max(0.0);
}

fn animate_hurt_flash(
    time: Res<Time<Real>>,
    feedback: Res<HitFeedback>,
    autoplay: Option<Res<Autoplay>>,
    mut flashes: Query<(&mut HurtFlash, &mut BorderColor)>,
) {
    for (mut flash, mut border) in &mut flashes {
        if feedback.hero_hurt() && autoplay.is_none() {
            flash.remaining = HURT_FLASH_SECONDS;
        }
        flash.remaining = (flash.remaining - time.delta_secs()).max(0.0);
        let alpha = 0.55 * flash.remaining / HURT_FLASH_SECONDS;
        *border = BorderColor::all(Color::srgba(0.9, 0.05, 0.04, alpha));
    }
}

fn clear_frame_events(mut feedback: ResMut<HitFeedback>) {
    feedback.events.clear();
    feedback.hero_hurt = false;
}

pub(crate) fn reset_run(world: &mut World) {
    world.insert_resource(HitFeedback::default());
    world.insert_resource(ScreenShake::default());
    world.insert_resource(HitStop::default());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(weight: HitWeight, critical: bool, killed: bool) -> HitEvent {
        HitEvent {
            target: Entity::from_bits(1),
            position: Vec3::ZERO,
            element: DamageElement::Physical,
            weight,
            critical,
            killed,
        }
    }

    #[test]
    fn punch_returns_exactly_to_base_scale() {
        let base = Vec3::splat(1.25);
        assert_eq!(punch_scale(base, PUNCH_SECONDS, 1.6), base);
        let mid = punch_scale(base, PUNCH_SECONDS * 0.2, 1.0);
        assert!(mid.y < base.y && mid.x > base.x);
    }

    #[test]
    fn heavier_kills_shake_and_freeze_harder() {
        let normal = event(HitWeight::Normal, false, true);
        let elite = event(HitWeight::Elite, false, true);
        let boss = event(HitWeight::Boss, false, true);
        assert_eq!(shake_trauma(event(HitWeight::Normal, false, false)), 0.0);
        assert!(shake_trauma(normal) < shake_trauma(elite));
        assert!(shake_trauma(elite) < shake_trauma(boss));
        assert_eq!(hit_stop_seconds(normal), 0.0);
        assert!(hit_stop_seconds(elite) < hit_stop_seconds(boss));
    }

    fn hit_stop_app(battle_speed: f32) -> App {
        let mut app = App::new();
        app.init_resource::<Time<Real>>()
            .init_resource::<Time<Virtual>>()
            .init_resource::<GameSession>()
            .insert_resource(HitStop {
                requested: 0.1,
                ..default()
            })
            .add_systems(Update, apply_hit_stop);
        app.world_mut()
            .resource_mut::<Time<Virtual>>()
            .set_relative_speed(battle_speed);
        app
    }

    #[test]
    fn hit_stop_slows_then_restores_battle_speed() {
        let mut app = hit_stop_app(2.0);
        app.update();
        let slowed = app.world().resource::<Time<Virtual>>().relative_speed();
        assert!((slowed - 2.0 * HIT_STOP_TIME_SCALE).abs() < 1e-6);
        app.world_mut().resource_mut::<HitStop>().remaining = 0.0001;
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(std::time::Duration::from_millis(5));
        app.update();
        assert_eq!(
            app.world().resource::<Time<Virtual>>().relative_speed(),
            2.0
        );
    }

    #[test]
    fn fast_forwarded_battles_skip_hit_stop() {
        let mut app = hit_stop_app(8.0);
        app.update();
        assert_eq!(
            app.world().resource::<Time<Virtual>>().relative_speed(),
            8.0
        );
    }

    #[test]
    fn hits_punch_survivors_and_kills_leave_a_burst() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<GameSession>()
            .init_resource::<CombatPools>()
            .init_resource::<ScreenShake>()
            .init_resource::<HitStop>()
            .init_resource::<HitFeedback>();
        app.world_mut().run_system_cached(setup_burst_art).unwrap();
        let survivor = app
            .world_mut()
            .spawn(Transform::from_scale(Vec3::splat(1.1)))
            .id();
        {
            let mut feedback = app.world_mut().resource_mut::<HitFeedback>();
            feedback.push(HitEvent {
                critical: true,
                ..event(HitWeight::Normal, false, false)
            });
            feedback.events[0].target = survivor;
            feedback.push(event(HitWeight::Boss, false, true));
        }
        app.world_mut().run_system_cached(react_to_hits).unwrap();

        let punch = app
            .world()
            .get::<HitPunch>(survivor)
            .expect("survivor squashes");
        assert_eq!(punch.base_scale, Vec3::splat(1.1));
        // Crit spark + kill ring + kill flash.
        assert_eq!(
            app.world()
                .resource::<CombatPools>()
                .impact_fx_metrics()
                .active(),
            3
        );
        let shake = app.world().resource::<ScreenShake>().trauma;
        assert_eq!(shake, 1.0);
        assert!(app.world().resource::<HitStop>().requested > 0.1);
    }

    fn punch_app() -> (App, Entity) {
        let mut app = App::new();
        app.init_resource::<GameSession>()
            .init_resource::<CombatPools>()
            .init_resource::<ScreenShake>()
            .init_resource::<HitStop>()
            .init_resource::<HitFeedback>()
            .init_resource::<Time<Real>>();
        let target = app
            .world_mut()
            .spawn(Transform::from_scale(Vec3::splat(1.25)))
            .id();
        (app, target)
    }

    fn record_punch(app: &mut App, target: Entity, critical: bool) {
        let mut feedback = app.world_mut().resource_mut::<HitFeedback>();
        feedback.events.clear();
        feedback.push(HitEvent {
            target,
            ..event(HitWeight::Normal, critical, false)
        });
    }

    #[test]
    fn large_real_frame_keeps_new_hit_punch_visible() {
        let (mut app, target) = punch_app();
        let base = app.world().get::<Transform>(target).unwrap().scale;
        // Synthetic event input, actual production reaction systems. The native
        // companion uses a real sword contact and authoritative HP decrease.
        record_punch(&mut app, target, false);
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(std::time::Duration::from_millis(160));
        app.add_systems(Update, (react_to_hits, animate_hit_punch).chain());
        app.update();
        let scale = app.world().get::<Transform>(target).unwrap().scale;
        println!(
            "PUNCH_LARGE_FRAME base={base:?} first_scale={scale:?} active={}",
            app.world().get::<HitPunch>(target).is_some()
        );
        assert!(
            scale.y < base.y && scale.x > base.x,
            "first rendered contact must retain impact squash even when its real frame exceeds the reaction duration"
        );
        assert!(app.world().get::<HitPunch>(target).is_some());
        app.world_mut().resource_mut::<HitFeedback>().events.clear();
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(std::time::Duration::from_millis(160));
        app.update();
        assert_eq!(app.world().get::<Transform>(target).unwrap().scale, base);
        assert!(app.world().get::<HitPunch>(target).is_none());
    }

    #[test]
    fn new_hit_punch_waits_for_unpaused_first_presentation() {
        let (mut app, target) = punch_app();
        let base = app.world().get::<Transform>(target).unwrap().scale;
        record_punch(&mut app, target, false);
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(std::time::Duration::from_millis(160));
        app.add_systems(Update, (react_to_hits, animate_hit_punch).chain());
        app.update();
        assert_eq!(app.world().get::<Transform>(target).unwrap().scale, base);
        let punch = app.world().get::<HitPunch>(target).unwrap();
        assert!(!punch.presented);
        assert_eq!(punch.elapsed, 0.);
        app.world_mut().resource_mut::<HitFeedback>().events.clear();
        app.world_mut().resource_mut::<GameSession>().paused = false;
        app.update();
        assert!(app.world().get::<Transform>(target).unwrap().scale.y < base.y);
        assert!(app.world().get::<HitPunch>(target).unwrap().presented);
    }

    #[test]
    fn dense_hits_complete_recovery_and_next_normal_does_not_keep_critical_strength() {
        let (mut app, target) = punch_app();
        let base = app.world().get::<Transform>(target).unwrap().scale;
        record_punch(&mut app, target, true);
        app.world_mut().run_system_cached(react_to_hits).unwrap();
        // Twelve 10ms hits arrive during one 120ms reaction. None may rewind it.
        for _ in 0..13 {
            record_punch(&mut app, target, false);
            app.world_mut().run_system_cached(react_to_hits).unwrap();
            app.world_mut()
                .resource_mut::<Time<Real>>()
                .advance_by(std::time::Duration::from_millis(10));
            app.world_mut()
                .run_system_cached(animate_hit_punch)
                .unwrap();
            if app.world().get::<HitPunch>(target).is_none() {
                break;
            }
        }
        assert!(
            app.world().get::<HitPunch>(target).is_none(),
            "dense hits must not pin the reaction"
        );
        assert_eq!(app.world().get::<Transform>(target).unwrap().scale, base);
        record_punch(&mut app, target, false);
        app.world_mut().run_system_cached(react_to_hits).unwrap();
        let next = app.world().get::<HitPunch>(target).unwrap();
        assert_eq!(next.strength, 1.0);
        assert_eq!(next.base_scale, base);
    }

    #[test]
    fn simultaneous_critical_volley_does_not_multiply_camera_shake() {
        let (mut app, target) = punch_app();
        for _ in 0..100 {
            app.world_mut()
                .resource_mut::<HitFeedback>()
                .push(HitEvent {
                    target,
                    ..event(HitWeight::Normal, true, false)
                });
        }
        app.world_mut().run_system_cached(react_to_hits).unwrap();
        assert_eq!(app.world().resource::<ScreenShake>().trauma, 0.07);
        app.world_mut()
            .resource_mut::<HitFeedback>()
            .push(HitEvent {
                target,
                ..event(HitWeight::Boss, false, true)
            });
        app.world_mut().run_system_cached(react_to_hits).unwrap();
        assert_eq!(app.world().resource::<ScreenShake>().trauma, 1.0);
    }

    #[test]
    fn simultaneous_punches_use_strongest_hit_in_either_order() {
        for critical_first in [false, true] {
            let (mut app, target) = punch_app();
            record_punch(&mut app, target, critical_first);
            app.world_mut()
                .resource_mut::<HitFeedback>()
                .push(HitEvent {
                    target,
                    ..event(HitWeight::Normal, !critical_first, false)
                });
            app.world_mut().run_system_cached(react_to_hits).unwrap();
            assert_eq!(app.world().get::<HitPunch>(target).unwrap().strength, 1.6);
        }
    }

    #[test]
    fn low_frame_rate_keeps_first_camera_impulse_and_restores_exact_position() {
        let mut app = App::new();
        app.init_resource::<Time<Real>>()
            .init_resource::<GameSession>()
            .insert_resource(ScreenShake {
                trauma: 0.07,
                ..default()
            })
            .add_systems(Update, apply_screen_shake);
        let base = Vec3::new(0.0, 10.0, 10.0);
        let camera = app
            .world_mut()
            .spawn((Camera3d::default(), Transform::from_translation(base)))
            .id();
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(std::time::Duration::from_millis(100));
        app.update();
        assert!(app.world().resource::<ScreenShake>().applied.length() > 0.0001);
        assert_eq!(app.world().resource::<ScreenShake>().trauma, 0.0);
        app.update();
        assert_eq!(
            app.world().get::<Transform>(camera).unwrap().translation,
            base
        );
        assert_eq!(app.world().resource::<ScreenShake>().applied, Vec3::ZERO);
    }

    #[test]
    fn hero_is_knocked_away_from_the_nearest_attacker() {
        let hero = Vec3::new(-8.0, 0.0, 0.0);
        let push = knockback_direction(
            hero,
            [Vec3::new(-6.0, 0.0, 0.0), Vec3::new(-8.0, 0.0, 5.0)].into_iter(),
        );
        assert!((push - Vec3::NEG_X).length() < 1e-5);
        assert_eq!(knockback_direction(hero, std::iter::empty()), Vec3::NEG_X);
    }

    #[test]
    fn flinch_leans_back_away_from_the_blow_and_fully_recovers() {
        let (offset, lean) = flinch_pose(Vec3::NEG_X, Quat::IDENTITY, FLINCH_PEAK_AT);
        assert!(offset.x < -0.3);
        // The model's head tips the same way it is pushed.
        assert!((lean * Vec3::Y).x < -0.3);
        let (offset, lean) = flinch_pose(Vec3::NEG_X, Quat::IDENTITY, FLINCH_SECONDS);
        assert!(offset.length() < 1e-5);
        assert!(lean.angle_between(Quat::IDENTITY) < 1e-4);
    }

    #[test]
    fn hero_hurt_ignores_healing_and_chip_damage() {
        let mut feedback = HitFeedback {
            previous_hero_health: Some(100.0),
            ..default()
        };
        let mut world = World::new();
        world.insert_resource(HeroVitals {
            health: 99.5,
            max_health: 100.0,
            alive: true,
            ..HeroVitals::default()
        });
        std::mem::swap(
            &mut *world.get_resource_or_insert_with(HitFeedback::default),
            &mut feedback,
        );
        world.run_system_cached(detect_hero_hurt).unwrap();
        assert!(!world.resource::<HitFeedback>().hero_hurt());
        world.resource_mut::<HeroVitals>().health = 80.0;
        world.run_system_cached(detect_hero_hurt).unwrap();
        assert!(world.resource::<HitFeedback>().hero_hurt());
    }
}
