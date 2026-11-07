//! Narrow native Ember/Storm launch candidates; no fusion or melee timing changes.
use super::*;

#[derive(Clone, Copy)]
pub(super) struct NativeShot {
    pub kind: HeroKind,
    pub level: u32,
    pub direction: Vec3,
    pub damage: f32,
    pub speed: f32,
    pub pierce: u8,
    pub bounces: u8,
}

pub(super) fn release_phase(kind: HeroKind) -> Option<f32> {
    match kind {
        HeroKind::EmberRanger => Some(0.25),
        HeroKind::StormSavant => Some(0.28),
        _ => None,
    }
}

#[derive(Component)]
pub(super) struct PendingRangedShot {
    hero: Entity,
    shot: NativeShot,
    remaining: f32,
    new: bool,
}
impl PendingRangedShot {
    pub(super) fn new(hero: Entity, shot: NativeShot, remaining: f32) -> Self {
        Self {
            hero,
            shot,
            remaining,
            new: true,
        }
    }
}

/// Single factory for the existing native projectile path (all native profiles).
/// This function never commits an attack, rolls RNG, or performs target selection.
pub(super) fn emit(
    commands: &mut Commands,
    pools: &mut CombatPools,
    assets: &GameAssets,
    art: Option<&BasicAttackArt>,
    origin: Vec3,
    shot: NativeShot,
    first_travel: Option<f32>,
) -> Option<Entity> {
    let (mesh, material) = art
        .map(|art| art.projectile_at_level(shot.kind, shot.level))
        .unwrap_or_else(|| {
            (
                assets.projectile_mesh.clone(),
                assets.projectile_material.clone(),
            )
        });
    let entity = pools.rent_projectile(
        commands,
        mesh,
        material,
        Transform::from_translation(origin + Vec3::Y * 0.25)
            .looking_to(shot.direction, Vec3::Y)
            .with_scale(Vec3::splat(
                basic_attacks::basic_attack_visual_tier(shot.level).projectile_scale,
            )),
        Projectile {
            velocity: shot.direction * shot.speed,
            damage: shot.damage,
            radius: 0.48,
            remaining_hits: shot.pierce,
            lifetime: 1.4,
        },
    )?;
    let profile = if basic_attacks::weapon(shot.kind) == basic_attacks::BasicWeapon::Crossbow {
        DamageProfile::PHYSICAL_ATTACK.as_arrow()
    } else {
        DamageProfile::PHYSICAL_ATTACK
    };
    commands
        .entity(entity)
        .insert((profile, WeaponFlight::new(shot.kind)));
    if basic_attacks::is_starter_ember_arrow(shot.kind, shot.level) {
        commands
            .entity(entity)
            .insert(super::super::skill_lighting::NoGroundProjection);
    }
    if shot.bounces > 0 {
        commands
            .entity(entity)
            .insert(basic_attacks::Ricochet::new(shot.bounces));
    }
    if let Some(seconds) = first_travel {
        commands
            .entity(entity)
            .insert(ProjectileFirstFrameTravel(seconds));
    }
    Some(entity)
}

pub(super) fn release_pending(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    vitals: Res<HeroVitals>,
    controls: Res<SkillRuntime>,
    assets: Option<Res<GameAssets>>,
    art: Option<Res<BasicAttackArt>>,
    mut pools: ResMut<CombatPools>,
    heroes: Query<&Transform, With<Hero>>,
    mut shots: Query<(Entity, &mut PendingRangedShot)>,
) {
    for (entity, mut pending) in &mut shots {
        if session.finished
            || !vitals.can_act()
            || vitals.kind != pending.shot.kind
            || !heroes.contains(pending.hero)
            || controls.actions_blocked(pending.hero)
        {
            commands.entity(entity).despawn();
            continue;
        }
        if session.paused {
            continue;
        }
        // Creation already includes completion age. Do not subtract this frame twice.
        if pending.new {
            pending.new = false;
        } else {
            pending.remaining -= time.delta_secs();
        }
        if pending.remaining > 0.0 {
            continue;
        }
        let Some(assets) = assets.as_deref() else {
            commands.entity(entity).despawn();
            continue;
        };
        let Ok(hero) = heroes.get(pending.hero) else {
            continue;
        };
        // Only the portion after release travels, including a catch-up launch
        // already due within this update.
        let travel = (-pending.remaining).clamp(0.0, time.delta_secs().max(0.0));
        let _ = emit(
            &mut commands,
            &mut pools,
            assets,
            art.as_deref(),
            hero.translation,
            pending.shot,
            Some(travel),
        );
        // Current pool always leases either a visible or logic-only carrier.
        commands.entity(entity).despawn();
    }
}
