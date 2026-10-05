//! Permanent, unique hero legacies; one rotating weapon echo per four native attacks.
use super::attack_vfx::{WeaponCues, WeaponFlight};
use super::basic_attacks::{self, BasicAttackArt, PendingSwing};
use super::bonds::{DamageClass, DamageElement, DamageProfile};
use super::combat::CombatSet;
use super::pooling::CombatPools;
use super::{Enemy, GameAssets, GameSession, Hero, HeroKind, HeroVitals};
use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct HeroFusion {
    absorbed: [bool; HeroKind::COUNT],
    starting_skills: [Option<super::SkillId>; HeroKind::COUNT],
    attacks: u8,
    cursor: usize,
    /// Actual legacy attacks emitted, useful for HUD feedback and capture verification.
    pub echoes_emitted: u64,
    pub attacks_emitted: [u64; HeroKind::COUNT],
}
impl HeroFusion {
    pub fn record_absorbed(&mut self, kind: HeroKind) -> bool {
        let fresh = !self.absorbed[kind.index()];
        self.absorbed[kind.index()] = true;
        fresh
    }
    pub fn register_starting_skill(&mut self, kind: HeroKind, skill: super::SkillId) {
        self.starting_skills[kind.index()].get_or_insert(skill);
    }
    pub fn skill_capacity(&self) -> usize {
        6 + self
            .starting_skills
            .iter()
            .filter(|skill| skill.is_some())
            .count()
            .saturating_sub(1)
    }
    pub fn starting_skill(&self, kind: HeroKind) -> Option<super::SkillId> {
        self.starting_skills[kind.index()]
    }
    pub fn has(&self, kind: HeroKind) -> bool {
        self.absorbed[kind.index()]
    }
    pub fn count(&self) -> usize {
        self.absorbed.iter().filter(|owned| **owned).count()
    }
    pub fn kinds(&self, current: HeroKind) -> impl Iterator<Item = HeroKind> + '_ {
        HeroKind::ALL
            .into_iter()
            .filter(move |kind| *kind == current || self.has(*kind))
    }
    pub(crate) fn on_attack(&mut self, _current: HeroKind) -> Option<HeroKind> {
        if self.count() == 0 {
            self.attacks = 0;
            return None;
        }
        self.attacks += 1;
        if self.attacks < 4 {
            return None;
        }
        self.attacks = 0;
        for step in 0..HeroKind::COUNT {
            let index = (self.cursor + step) % HeroKind::COUNT;
            let kind = HeroKind::ALL[index];
            if self.has(kind) {
                self.cursor = (index + 1) % HeroKind::COUNT;
                return Some(kind);
            }
        }
        None
    }
}
#[derive(Component, Clone, Copy)]
pub(crate) struct FusionAttack {
    pub(crate) kind: HeroKind,
    pub(crate) origin: Vec3,
}
#[derive(Component)]
pub(crate) struct FusionRequest {
    pub(crate) hero: Entity,
    pub(crate) kind: HeroKind,
    pub(crate) target: Entity,
    pub(crate) damage: f32,
    pub(crate) speed: f32,
}
pub(crate) struct HeroFusionPlugin;
impl Plugin for HeroFusionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HeroFusion>()
            .add_systems(Update, emit.after(CombatSet));
    }
}
pub(crate) fn profile(kind: HeroKind) -> DamageProfile {
    let element = match kind {
        HeroKind::StormSavant => DamageElement::Storm,
        HeroKind::EmberRanger | HeroKind::SolarVanguard => DamageElement::Flame,
        HeroKind::TideWalker => DamageElement::Tide,
        HeroKind::FrostOracle => DamageElement::Frost,
        HeroKind::SpiritKeeper
        | HeroKind::StarWagerer
        | HeroKind::VoidReaper
        | HeroKind::RiftDancer => DamageElement::Arcane,
        _ => DamageElement::Physical,
    };
    let mut result = DamageProfile::new(element, DamageClass::Attack).as_card_proc();
    if basic_attacks::weapon(kind).melee() {
        result = result.as_melee();
    }
    if element == DamageElement::Flame {
        result = result.with_burn();
    }
    result
}
#[allow(clippy::too_many_arguments)]
fn emit(
    mut commands: Commands,
    session: Res<GameSession>,
    vitals: Res<HeroVitals>,
    build: Option<Res<super::PlayerBuild>>,
    bonds: Option<Res<super::bonds::BondRuntime>>,
    mut fusion: ResMut<HeroFusion>,
    requests: Query<(Entity, &FusionRequest)>,
    heroes: Query<&Transform, With<Hero>>,
    enemies: Query<(&Enemy, &Transform, Option<&super::combat::EliteAffix>), Without<Hero>>,
    assets: Option<Res<GameAssets>>,
    art: Option<Res<BasicAttackArt>>,
    mut pools: ResMut<CombatPools>,
    mut cues: ResMut<WeaponCues>,
) {
    if session.paused {
        return;
    }
    for (entity, request) in &requests {
        commands.entity(entity).despawn();
        if session.finished || !vitals.can_act() {
            continue;
        }
        let (Ok(hero), Ok((enemy, target, affix))) =
            (heroes.get(request.hero), enemies.get(request.target))
        else {
            continue;
        };
        if enemy.hp <= 0.0 || affix.is_some_and(|affix| affix.is_volatile_pending()) {
            continue;
        }
        let direction = (target.translation - hero.translation)
            .with_y(0.0)
            .normalize_or(Vec3::X);
        let weapon = basic_attacks::weapon(request.kind);
        let fallback = super::PlayerBuild::default();
        let class_stats = basic_attacks::class_basic_stats(
            request.kind,
            vitals.level,
            build.as_deref().unwrap_or(&fallback),
        );
        if weapon.melee() {
            // A projected weapon apparition strikes at the victim, so a ranged
            // current hero can still wield an absorbed melee legacy.
            let origin = target.translation.with_y(0.0) - direction * 0.6;
            commands.spawn((
                Name::new("Fusion legacy melee"),
                FusionAttack {
                    kind: request.kind,
                    origin,
                },
                PendingSwing {
                    hero: request.hero,
                    target: request.target,
                    kind: request.kind,
                    direction,
                    damage: request.damage * 0.45,
                    hit_limit: class_stats.cleave_targets
                        + bonds.as_ref().map_or(0, |bonds| bonds.melee_cleave()),
                    remaining: 0.12,
                },
            ));
        } else {
            let Some(assets) = &assets else { continue };
            let (mesh, material) = art
                .as_deref()
                .map(|art| art.projectile_at_level(request.kind, vitals.level))
                .unwrap_or_else(|| {
                    (
                        assets.projectile_mesh.clone(),
                        assets.projectile_material.clone(),
                    )
                });
            let mut targets: Vec<_> = enemies
                .iter()
                .filter(|(enemy, transform, affix)| {
                    enemy.hp > 0.0
                        && !affix.is_some_and(|affix| affix.is_volatile_pending())
                        && transform.translation.distance_squared(hero.translation)
                            <= class_stats.range * class_stats.range
                })
                .map(|(_, transform, _)| transform.translation)
                .collect();
            targets.sort_by(|a, b| {
                a.distance_squared(target.translation)
                    .total_cmp(&b.distance_squared(target.translation))
            });
            let shots = if weapon == basic_attacks::BasicWeapon::Crossbow {
                class_stats
                    .shots
                    .min(targets.len().min(usize::from(u8::MAX)) as u8)
            } else {
                class_stats.shots
            };
            for shot in 0..shots {
                let direction = if weapon == basic_attacks::BasicWeapon::Crossbow {
                    targets
                        .get(usize::from(shot))
                        .map_or(direction, |position| {
                            (*position - hero.translation).normalize_or(Vec3::X)
                        })
                } else {
                    Quat::from_rotation_y(
                        (f32::from(shot) - (f32::from(class_stats.shots) - 1.0) * 0.5) * 0.14,
                    ) * direction
                };
                let Some(projectile) = pools.rent_projectile(
                    &mut commands,
                    mesh.clone(),
                    material.clone(),
                    Transform::from_translation(hero.translation + Vec3::Y * 0.25)
                        .looking_to(direction, Vec3::Y)
                        .with_scale(Vec3::splat(
                            basic_attacks::basic_attack_visual_tier(vitals.level).projectile_scale,
                        )),
                    super::Projectile {
                        velocity: direction * request.speed,
                        damage: request.damage * 0.45,
                        radius: 0.48,
                        remaining_hits: match request.kind {
                            HeroKind::StormSavant | HeroKind::StarWagerer => 2_u8,
                            HeroKind::GroveArtificer | HeroKind::FrostOracle => 1,
                            _ => 0,
                        }
                        .saturating_add(build.as_ref().map_or(0, |build| build.pierce)),
                        lifetime: 1.4,
                    },
                ) else {
                    continue;
                };
                commands.entity(projectile).insert((
                    Name::new("Fusion legacy projectile"),
                    FusionAttack {
                        kind: request.kind,
                        origin: hero.translation,
                    },
                    WeaponFlight::new(request.kind),
                    profile(request.kind),
                ));
                cues.push(request.kind, hero.translation, direction, false);
                if class_stats.bounces > 0 {
                    commands
                        .entity(projectile)
                        .insert(basic_attacks::Ricochet::new(class_stats.bounces));
                }
            }
        }
        fusion.echoes_emitted = fusion.echoes_emitted.saturating_add(1);
        fusion.attacks_emitted[request.kind.index()] += 1;
    }
}
pub(crate) fn apply_contact(
    commands: &mut Commands,
    target: Entity,
    kind: HeroKind,
    boss: bool,
    existing: Option<&super::hero_abilities::TimedSpeedEffect>,
) {
    let control = match kind {
        HeroKind::StormSavant => Some((if boss { 0.7 } else { 0.0 }, 0.18)),
        HeroKind::FrostOracle => Some((if boss { 0.5 } else { 0.0 }, 0.45)),
        HeroKind::TideWalker => Some((0.5, 1.5)),
        HeroKind::VoidReaper => Some((0.3, 0.8)),
        _ => None,
    };
    if let Some((multiplier, remaining)) = control {
        commands
            .entity(target)
            .insert(super::hero_abilities::TimedSpeedEffect {
                multiplier: existing.map_or(multiplier, |effect| effect.multiplier.min(multiplier)),
                remaining: existing.map_or(remaining, |effect| effect.remaining.max(remaining)),
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn absorbed_heroes_are_unique_and_current_is_never_duplicated() {
        let mut fusion = HeroFusion::default();
        assert!(fusion.record_absorbed(HeroKind::StoneWarden));
        assert!(!fusion.record_absorbed(HeroKind::StoneWarden));
        assert_eq!(fusion.count(), 1);
        assert_eq!(fusion.kinds(HeroKind::StoneWarden).count(), 1);
        assert_eq!(fusion.kinds(HeroKind::StormSavant).count(), 2);
        fusion.register_starting_skill(HeroKind::StoneWarden, super::super::SkillId(4));
        fusion.register_starting_skill(HeroKind::StoneWarden, super::super::SkillId(5));
        assert_eq!(
            fusion.starting_skill(HeroKind::StoneWarden),
            Some(super::super::SkillId(4))
        );
    }
    #[test]
    fn echoes_rotate_at_fixed_cadence_without_multiplying_with_collection_size() {
        let mut fusion = HeroFusion::default();
        for kind in HeroKind::ALL {
            fusion.record_absorbed(kind);
        }
        let echoes: Vec<_> = (0..48)
            .filter_map(|_| fusion.on_attack(HeroKind::StoneWarden))
            .collect();
        assert_eq!(echoes.len(), 12);
        assert!(echoes.contains(&HeroKind::StoneWarden));
        assert_eq!(
            echoes
                .iter()
                .map(|k| k.index())
                .collect::<std::collections::HashSet<_>>()
                .len(),
            12
        );
    }
    #[test]
    fn inherited_weapons_preserve_damage_identity_and_cannot_chain_card_procs() {
        assert_eq!(profile(HeroKind::StormSavant).element, DamageElement::Storm);
        assert_eq!(profile(HeroKind::FrostOracle).element, DamageElement::Frost);
        assert!(profile(HeroKind::EmberRanger).applies_burn);
        assert!(profile(HeroKind::IronBerserker).melee);
        assert!(
            HeroKind::ALL
                .into_iter()
                .all(|kind| profile(kind).card_proc)
        );
    }
    #[test]
    fn capacity_grows_only_for_newly_registered_heroes() {
        let mut fusion = HeroFusion::default();
        assert_eq!(fusion.skill_capacity(), 6);
        fusion.register_starting_skill(HeroKind::StormSavant, super::super::SkillId(0));
        assert_eq!(fusion.skill_capacity(), 6);
        fusion.record_absorbed(HeroKind::StormSavant);
        fusion.register_starting_skill(HeroKind::GroveArtificer, super::super::SkillId(17));
        assert_eq!(fusion.skill_capacity(), 7);
        fusion.record_absorbed(HeroKind::GroveArtificer);
        fusion.register_starting_skill(HeroKind::StormSavant, super::super::SkillId(12));
        assert_eq!(fusion.skill_capacity(), 7);
        assert_eq!(
            fusion.starting_skill(HeroKind::StormSavant),
            Some(super::super::SkillId(0))
        );
        fusion.register_starting_skill(HeroKind::FrostOracle, super::super::SkillId(2));
        assert_eq!(fusion.skill_capacity(), 8);
    }
}
