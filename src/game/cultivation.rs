//! Cultivation card procs. Damage is resolved by the existing typed strike pipeline.
use super::bonds::{BondRuntime, DamageClass, DamageElement, DamageProfile};
use super::combat::CombatSet;
use super::{GameSession, Hero, HeroVitals, PlayerBuild, RunRng};
use bevy::prelude::*;

pub(crate) struct CultivationPlugin;
impl Plugin for CultivationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, (suppress, animate).chain().after(CombatSet));
    }
}
#[derive(Resource)]
struct Art {
    bowl: Handle<Mesh>,
    rim: Handle<Mesh>,
    leg: Handle<Mesh>,
    bronze: Handle<StandardMaterial>,
    jade: Handle<StandardMaterial>,
}
fn setup(
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };
    commands.insert_resource(Art {
        bowl: meshes.add(Sphere::new(0.65)),
        rim: meshes.add(Torus::new(0.53, 0.69)),
        leg: meshes.add(Cuboid::new(0.18, 0.7, 0.18)),
        bronze: materials.add(StandardMaterial {
            base_color: Color::srgb(0.35, 0.24, 0.06),
            metallic: 0.8,
            perceptual_roughness: 0.3,
            ..default()
        }),
        jade: materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 1.0, 0.55),
            emissive: LinearRgba::rgb(1.0, 7.0, 3.0),
            ..default()
        }),
    });
}
#[derive(Component)]
struct Seal {
    remaining: f32,
    origin: Vec3,
}
fn adaptive_profile(vitals: &HeroVitals) -> DamageProfile {
    DamageProfile::new(
        if vitals.intelligence > vitals.strength.max(vitals.agility) {
            DamageElement::Arcane
        } else {
            DamageElement::Physical
        },
        DamageClass::Reaction,
    )
    .as_card_proc()
}
fn proc_count(hits: u32, chance: f32, rng: &mut RunRng) -> u32 {
    // No recursion: a received damage event is consumed once, independent of the
    // resulting damage. Large frames may combine procs into one identical AoE.
    if chance <= 0.0 {
        return 0;
    }
    (0..hits).filter(|_| rng.unit() < chance).count() as u32
}
fn suppress(
    mut commands: Commands,
    session: Res<GameSession>,
    build: Res<PlayerBuild>,
    vitals: Res<HeroVitals>,
    hero: Query<&Transform, With<Hero>>,
    mut bonds: ResMut<BondRuntime>,
    mut rng: ResMut<RunRng>,
    art: Option<Res<Art>>,
) {
    if session.paused {
        return;
    }
    let hits = bonds.take_received_hits();
    if session.finished || !vitals.alive || hits == 0 {
        return;
    }
    let Ok(hero) = hero.single() else { return };
    let count = proc_count(hits, bonds.suppression_chance(), &mut rng);
    if count == 0 {
        return;
    }
    let origin = hero.translation.with_y(0.0);
    super::skills::schedule_typed_fixed_strike(
        &mut commands,
        origin,
        build.attack_damage * bonds.suppression_multiplier() * count as f32,
        0.0,
        4.0,
        true,
        4.0,
        adaptive_profile(&vitals),
    );
    let Some(art) = art else { return };
    commands
        .spawn((
            Name::new("Cultivation cauldron suppression"),
            Seal {
                remaining: 0.7,
                origin,
            },
            Transform::from_translation(origin + Vec3::Y * 2.3),
            Visibility::default(),
        ))
        .with_children(|p| {
            p.spawn((
                Mesh3d(art.bowl.clone()),
                MeshMaterial3d(art.bronze.clone()),
                Transform::default().with_scale(Vec3::new(1.0, 0.7, 1.0)),
            ));
            p.spawn((
                Mesh3d(art.rim.clone()),
                MeshMaterial3d(art.jade.clone()),
                Transform::from_xyz(0.0, 0.3, 0.0),
            ));
            for n in 0..3 {
                let angle = n as f32 * std::f32::consts::TAU / 3.0;
                p.spawn((
                    Mesh3d(art.leg.clone()),
                    MeshMaterial3d(art.bronze.clone()),
                    Transform::from_xyz(angle.cos() * 0.45, -0.45, angle.sin() * 0.45),
                ));
            }
            p.spawn((
                Mesh3d(art.rim.clone()),
                MeshMaterial3d(art.jade.clone()),
                Transform::from_xyz(0.0, -2.25, 0.0).with_scale(Vec3::splat(5.6)),
            ));
        });
}
fn animate(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    mut seals: Query<(Entity, &mut Seal, &mut Transform)>,
) {
    if session.paused {
        return;
    }
    for (entity, mut seal, mut transform) in &mut seals {
        seal.remaining -= time.delta_secs();
        if session.finished || seal.remaining <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        transform.translation = seal.origin + Vec3::Y * (2.1 + seal.remaining * 0.3);
        transform.scale = Vec3::splat(1.0 + (0.7 - seal.remaining) * 0.4);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn suppression_rolls_once_per_hit_and_never_without_card() {
        let mut rng = RunRng::from_seed(42);
        assert_eq!(proc_count(50, 0.0, &mut rng), 0);
        assert_eq!(proc_count(0, 1.0, &mut rng), 0);
        assert_eq!(proc_count(7, 1.0, &mut rng), 7);
        let count = proc_count(10000, 0.12, &mut rng);
        assert!((1000..1400).contains(&count));
    }
    #[test]
    fn adaptive_damage_tracks_dominant_attribute_and_cannot_retrigger_spell_procs() {
        let mut vitals = HeroVitals {
            strength: 50.0,
            agility: 30.0,
            intelligence: 20.0,
            ..default()
        };
        assert_eq!(adaptive_profile(&vitals).element, DamageElement::Physical);
        vitals.intelligence = 100.0;
        let profile = adaptive_profile(&vitals);
        assert_eq!(profile.element, DamageElement::Arcane);
        assert!(profile.card_proc);
        assert_eq!(BondRuntime::default().spell_procs(profile, 0.0), (false, 0));
    }
}
