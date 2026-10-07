//! Bounded ground projections: solid companions cast shadows, energy casts light.
/// Floating souls do not leave a solid footprint or a giant bullet ground glow.
#[derive(bevy::prelude::Component)]
pub(crate) struct NoGroundProjection;
use bevy::{
    light::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
};

use super::bonds::{DamageElement, DamageProfile};
use super::skills::{BloodWheel, HomingSword, OrbitBlade, SkillZone, tidal_wave::TidalWave};
use super::{ArenaEntity, GameSession, ImpactFx, PlayerSummon, Projectile};

const PROJECTION_CAP: usize = 80;

pub(crate) struct SkillLightingPlugin;
impl Plugin for SkillLightingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup).add_systems(
            PostUpdate,
            (color_impacts, exclude_energy_shadows, project)
                .chain()
                .after(bevy::transform::TransformSystems::Propagate)
                .before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate),
        );
    }
}

#[derive(Component)]
struct GroundProjection;

#[derive(Component)]
struct ProjectionSurface;

/// Marks the actual sustained flame ray rather than its disposable particles.
#[derive(Component)]
pub(crate) struct GroundBeam;

/// Cosmetic metadata only: an impact has no damage behavior of its own.
#[derive(Component, Clone, Copy)]
pub(crate) struct GroundLightProfile(pub(crate) DamageProfile);

#[derive(Resource)]
struct ProjectionArt {
    shadow: Handle<StandardMaterial>,
    cool: Handle<StandardMaterial>,
    warm: Handle<StandardMaterial>,
    storm: Handle<StandardMaterial>,
    infection: Handle<StandardMaterial>,
    impacts: [Handle<StandardMaterial>; 4],
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let shadow = materials.add(StandardMaterial {
        base_color: Color::srgba(0.025, 0.015, 0.045, 0.23),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    let light = |color: Color| StandardMaterial {
        base_color: color,
        emissive: color.to_linear() * 1.6,
        alpha_mode: AlphaMode::Add,
        unlit: true,
        ..default()
    };
    let cool = materials.add(light(Color::srgba(0.16, 0.55, 0.85, 0.16)));
    let warm = materials.add(light(Color::srgba(0.95, 0.28, 0.08, 0.16)));
    let storm = materials.add(light(Color::srgba(0.65, 0.25, 0.95, 0.16)));
    let infection = materials.add(light(Color::srgba(0.16, 0.85, 0.22, 0.16)));
    let impacts = [
        (0.18, 0.70, 1.0),
        (1.0, 0.28, 0.04),
        (0.65, 0.25, 0.95),
        (0.18, 0.90, 0.12),
    ]
    .map(|(r, g, b)| {
        materials.add(StandardMaterial {
            base_color: Color::srgba(r, g, b, 0.32),
            emissive: LinearRgba::rgb(r * 5.0, g * 5.0, b * 5.0),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })
    });
    let disc = meshes.add(Circle::new(1.0));
    // Layered translucent discs soften the edge without another texture or light.
    for _ in 0..PROJECTION_CAP {
        commands
            .spawn((
                Name::new("Skill ground projection"),
                ArenaEntity,
                GroundProjection,
                Transform::default(),
                Visibility::Hidden,
            ))
            .with_children(|parent| {
                for (index, radius) in [1.0, 0.78, 0.52].into_iter().enumerate() {
                    parent.spawn((
                        Mesh3d(disc.clone()),
                        ProjectionSurface,
                        MeshMaterial3d(shadow.clone()),
                        Transform::from_xyz(0.0, index as f32 * 0.002, 0.0)
                            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
                            .with_scale(Vec3::splat(radius)),
                        NotShadowCaster,
                        NotShadowReceiver,
                    ));
                }
            });
    }
    commands.insert_resource(ProjectionArt {
        shadow,
        cool,
        warm,
        storm,
        infection,
        impacts,
    });
}

fn impact_color_index(profile: &DamageProfile) -> usize {
    if profile.skill_index == Some(18) {
        3
    } else {
        match profile.element {
            DamageElement::Flame => 1,
            DamageElement::Storm | DamageElement::Arcane => 2,
            _ => 0,
        }
    }
}

fn color_impacts(
    session: Res<GameSession>,
    art: Res<ProjectionArt>,
    mut effects: Query<
        (
            &mut MeshMaterial3d<StandardMaterial>,
            Option<&GroundLightProfile>,
            Option<&SkillZone>,
        ),
        Or<(With<GroundLightProfile>, With<SkillZone>)>,
    >,
) {
    if session.paused && !session.finished {
        return;
    }
    for (mut material, ground, zone) in &mut effects {
        let profile = ground
            .map(|p| &p.0)
            .or_else(|| zone.map(SkillZone::lighting_profile));
        if let Some(profile) = profile {
            material.0 = art.impacts[impact_color_index(profile)].clone();
        }
    }
}

impl ProjectionArt {
    fn energy(
        &self,
        profile: Option<&DamageProfile>,
        material: Option<&StandardMaterial>,
    ) -> &Handle<StandardMaterial> {
        if let Some(profile) = profile {
            if profile.skill_index == Some(18) {
                return &self.infection;
            }
            return match profile.element {
                DamageElement::Flame => &self.warm,
                DamageElement::Storm | DamageElement::Arcane => &self.storm,
                _ => &self.cool,
            };
        }
        // Impact shells retain the real emitter material even when they have no
        // damage metadata. Read it instead of inventing a fire-colored ice blast.
        let Some(material) = material else {
            return &self.cool;
        };
        let c = if material
            .emissive
            .red
            .max(material.emissive.green)
            .max(material.emissive.blue)
            > 0.1
        {
            material.emissive
        } else {
            material.base_color.to_linear()
        };
        if c.green > c.red * 1.25 && c.green > c.blue * 1.25 {
            &self.infection
        } else if c.red > c.blue * 1.5 {
            &self.warm
        } else if c.red > c.green * 1.25 && c.blue > c.green * 1.25 {
            &self.storm
        } else {
            &self.cool
        }
    }
}

// Transparent/additive and strongly emissive art must never cast black fire/light.
// Includes asynchronously loaded GLB meshes; material lookup retries while loading.
fn exclude_energy_shadows(
    mut commands: Commands,
    meshes: Query<(Entity, &MeshMaterial3d<StandardMaterial>), Without<NotShadowCaster>>,
    materials: Res<Assets<StandardMaterial>>,
) {
    for (entity, handle) in &meshes {
        let Some(material) = materials.get(&handle.0) else {
            continue;
        };
        let energy = material
            .emissive
            .red
            .max(material.emissive.green)
            .max(material.emissive.blue)
            > 2.0;
        if material.alpha_mode != AlphaMode::Opaque || energy {
            commands.entity(entity).insert(NotShadowCaster);
        }
    }
}

fn footprint(position: Vec3, scale: Vec3, radius: f32) -> Transform {
    let height = position.y.max(0.0);
    let spread = 1.0 + height.min(5.0) * 0.13;
    Transform::from_xyz(
        position.x + height * 0.12,
        0.035,
        position.z + height * 0.08,
    )
    .with_scale(Vec3::new(
        (scale.x.abs() * radius * spread).clamp(0.24, 7.0),
        1.0,
        (scale.z.abs() * radius * spread).clamp(0.24, 8.0),
    ))
}

fn project(
    session: Res<GameSession>,
    art: Res<ProjectionArt>,
    materials: Res<Assets<StandardMaterial>>,
    sources: Query<
        (
            &GlobalTransform,
            Option<&PlayerSummon>,
            Option<&Projectile>,
            Option<&OrbitBlade>,
            Option<&HomingSword>,
            Option<&BloodWheel>,
            Option<&TidalWave>,
            Option<&ImpactFx>,
            Option<&SkillZone>,
            Option<&Visibility>,
            Option<&GroundBeam>,
            Option<&DamageProfile>,
            Option<&MeshMaterial3d<StandardMaterial>>,
            Option<&GroundLightProfile>,
        ),
        (
            Without<GroundProjection>,
            Without<NoGroundProjection>,
            Without<ProjectionSurface>,
            Or<(
                With<PlayerSummon>,
                With<Projectile>,
                With<OrbitBlade>,
                With<HomingSword>,
                With<BloodWheel>,
                With<TidalWave>,
                With<ImpactFx>,
                With<SkillZone>,
                With<GroundBeam>,
            )>,
        ),
    >,
    mut projections: Query<
        (
            &mut Transform,
            &mut GlobalTransform,
            &mut Visibility,
            &Children,
        ),
        (With<GroundProjection>, Without<ProjectionSurface>),
    >,
    mut surfaces: Query<
        (
            &mut MeshMaterial3d<StandardMaterial>,
            &Transform,
            &mut GlobalTransform,
        ),
        (With<ProjectionSurface>, Without<GroundProjection>),
    >,
) {
    if session.paused && !session.finished {
        return;
    }
    let mut slots = projections.iter_mut();
    if !session.finished {
        let mut ordered = sources.iter().collect::<Vec<_>>();
        // Companions and broad signature skills remain grounded during dense
        // projectile storms. The fixed cosmetic budget never starves them.
        ordered.sort_by_key(|source| {
            if source.1.is_some() {
                0
            } else if source.3.is_some() || source.4.is_some() || source.5.is_some() {
                1
            } else if source.6.is_some() || source.8.is_some() || source.10.is_some() {
                2
            } else {
                3
            }
        });
        for (
            global,
            summon,
            projectile,
            blade,
            sword,
            wheel,
            wave,
            impact,
            zone,
            visibility,
            beam,
            profile,
            source_material,
            ground_profile,
        ) in ordered
        {
            if visibility == Some(&Visibility::Hidden) {
                continue;
            }
            let (scale, _, position) = global.to_scale_rotation_translation();
            let solid = summon.is_some() || blade.is_some() || sword.is_some() || wheel.is_some();
            // Preserve a clear battlefield: tiny bullet/impact particles have no ground light.
            if !solid
                && wave.is_none()
                && zone.is_none()
                && beam.is_none()
                && projectile.is_none_or(|p| p.radius < 0.4)
                && impact.is_none_or(|fx| fx.end_scale < 1.8)
            {
                continue;
            }
            let Some((mut transform, mut projected_global, mut visible, children)) = slots.next()
            else {
                break;
            };
            let material = if solid {
                &art.shadow
            } else if beam.is_some() {
                &art.warm
            } else {
                art.energy(
                    ground_profile
                        .map(|p| &p.0)
                        .or(profile)
                        .or_else(|| zone.map(SkillZone::lighting_profile)),
                    source_material.and_then(|h| materials.get(&h.0)),
                )
            };
            let radius = if summon.is_some() {
                0.72
            } else if solid {
                0.65
            } else {
                1.15
            };
            *transform = footprint(position, scale, radius);
            if let Some(wave) = wave {
                transform.scale = Vec3::new(1.5, 1.0, wave.half_width);
                transform.rotation = Quat::from_rotation_arc(Vec3::X, wave.direction);
            }
            if let Some(zone) = zone {
                let radius = zone.lighting_radius();
                transform.scale = Vec3::new(radius, 1.0, radius);
            }
            if beam.is_some() {
                let direction = (global.rotation() * Vec3::Y)
                    .with_y(0.0)
                    .normalize_or(Vec3::X);
                transform.scale = Vec3::new(scale.y.abs() * 0.5, 1.0, scale.x.abs() * 1.8);
                transform.rotation = Quat::from_rotation_arc(Vec3::X, direction);
            }
            // Only these eighty roots and their three surfaces need late propagation.
            // Sources already have this frame's world transforms; do not wait a frame
            // for the normal hierarchy propagation pass to catch the new footprint.
            *projected_global = GlobalTransform::from(*transform);
            *visible = Visibility::Visible;
            for child in children {
                if let Ok((mut surface, local, mut surface_global)) = surfaces.get_mut(*child) {
                    surface.0 = material.clone();
                    *surface_global = projected_global.mul_transform(*local);
                }
            }
        }
    }
    // A retired pool shell loses its active marker immediately: its projection disappears.
    for (_, _, mut visibility, _) in slots {
        *visibility = Visibility::Hidden;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::pooling::{CombatPools, finalize_recycles};
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn pooled_impact_ground_element_overrides_generic_blue_and_clears_on_reuse() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<GameSession>()
            .init_resource::<CombatPools>()
            .add_systems(Startup, setup)
            .add_systems(Update, (color_impacts, project).chain())
            .add_systems(Last, finalize_recycles);
        let mut first = None;
        for index in [13, 2, 18] {
            let shell = app
                .world_mut()
                .run_system_once(
                    move |mut commands: Commands, mut pools: ResMut<CombatPools>| {
                        let shell = pools
                            .rent_impact_fx(
                                &mut commands,
                                Handle::default(),
                                Handle::default(),
                                Transform::default(),
                                ImpactFx {
                                    lifetime: 0.24,
                                    start_scale: 0.12,
                                    end_scale: 3.2,
                                },
                            )
                            .unwrap();
                        commands
                            .entity(shell)
                            .insert(GroundLightProfile(DamageProfile::for_skill(index)));
                        shell
                    },
                )
                .unwrap();
            if let Some(first) = first {
                assert_eq!(shell, first);
            } else {
                first = Some(shell);
            }
            assert!(
                app.world().get::<DamageProfile>(shell).is_none(),
                "cosmetic marker must not become gameplay metadata"
            );
            app.update();
            let art = app.world().resource::<ProjectionArt>();
            let source_color =
                art.impacts[impact_color_index(&DamageProfile::for_skill(index))].clone();
            assert_eq!(
                app.world()
                    .get::<MeshMaterial3d<StandardMaterial>>(shell)
                    .unwrap()
                    .0,
                source_color
            );
            let expected = art
                .energy(Some(&DamageProfile::for_skill(index)), None)
                .clone();
            let children = app
                .world_mut()
                .query_filtered::<(&Visibility, &Children), With<GroundProjection>>()
                .iter(app.world())
                .find(|(v, _)| **v == Visibility::Visible)
                .map(|(_, c)| c.iter().collect::<Vec<_>>())
                .unwrap();
            for child in children {
                assert_eq!(
                    app.world()
                        .get::<MeshMaterial3d<StandardMaterial>>(child)
                        .unwrap()
                        .0,
                    expected
                );
            }
            app.world_mut()
                .run_system_once(
                    move |mut commands: Commands, mut pools: ResMut<CombatPools>| {
                        pools.retire_impact_fx(&mut commands, shell);
                    },
                )
                .unwrap();
            assert!(app.world().get::<GroundLightProfile>(shell).is_none());
            app.update();
        }
        let first = first.unwrap();
        // Defensively prove rent clears stale presentation state too.
        app.world_mut()
            .entity_mut(first)
            .insert(GroundLightProfile(DamageProfile::for_skill(13)));
        let reused = app
            .world_mut()
            .run_system_once(|mut commands: Commands, mut pools: ResMut<CombatPools>| {
                pools
                    .rent_impact_fx(
                        &mut commands,
                        Handle::default(),
                        Handle::default(),
                        Transform::default(),
                        ImpactFx {
                            lifetime: 0.2,
                            start_scale: 0.1,
                            end_scale: 2.0,
                        },
                    )
                    .unwrap()
            })
            .unwrap();
        assert_eq!(reused, first);
        assert!(app.world().get::<GroundLightProfile>(reused).is_none());
    }
    #[test]
    fn active_summon_shadow_follows_growth_freezes_and_retires_on_pool_release() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<GameSession>()
            .add_systems(Startup, setup)
            .add_systems(Update, project);
        let source = app
            .world_mut()
            .spawn((
                PlayerSummon,
                GlobalTransform::from(Transform::from_xyz(2.0, 0.5, 3.0)),
                Visibility::Visible,
            ))
            .id();
        app.update();
        let active = |world: &mut World| {
            world
                .query_filtered::<(&Transform, &Visibility), With<GroundProjection>>()
                .iter(world)
                .filter(|(_, visibility)| **visibility == Visibility::Visible)
                .map(|(transform, _)| *transform)
                .collect::<Vec<_>>()
        };
        let initial = active(app.world_mut());
        assert_eq!(initial.len(), 1);
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.world_mut()
            .entity_mut(source)
            .insert(GlobalTransform::from(
                Transform::from_xyz(5.0, 2.0, 6.0).with_scale(Vec3::splat(2.0)),
            ));
        app.update();
        assert_eq!(active(app.world_mut()), initial);
        app.world_mut().resource_mut::<GameSession>().paused = false;
        app.update();
        let grown = active(app.world_mut());
        assert_eq!(grown.len(), 1);
        assert!(grown[0].scale.x > initial[0].scale.x);
        assert!(grown[0].translation.x > initial[0].translation.x);
        let (root_global, children) = app
            .world_mut()
            .query_filtered::<(&GlobalTransform, &Visibility, &Children), With<GroundProjection>>()
            .iter(app.world())
            .find(|(_, visibility, _)| **visibility == Visibility::Visible)
            .map(|(global, _, children)| (*global, children.iter().collect::<Vec<_>>()))
            .unwrap();
        assert_eq!(root_global.translation(), grown[0].translation);
        for child in children {
            let local = *app.world().get::<Transform>(child).unwrap();
            let world = app.world().get::<GlobalTransform>(child).unwrap();
            assert_eq!(*world, root_global.mul_transform(local));
        }
        app.world_mut().entity_mut(source).remove::<PlayerSummon>();
        app.update();
        assert!(active(app.world_mut()).is_empty());
        app.world_mut().entity_mut(source).insert(PlayerSummon);
        app.update();
        assert_eq!(active(app.world_mut()).len(), 1);
        app.world_mut().resource_mut::<GameSession>().finished = true;
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.update();
        assert!(active(app.world_mut()).is_empty());
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<GroundProjection>>()
                .iter(app.world())
                .count(),
            PROJECTION_CAP
        );
    }
    #[test]
    fn ground_light_uses_real_element_and_real_impact_material() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_systems(Startup, setup);
        app.update();
        let art = app.world().resource::<ProjectionArt>();
        assert_eq!(
            art.energy(Some(&DamageProfile::for_skill(1)), None),
            &art.warm
        );
        assert_eq!(
            art.energy(Some(&DamageProfile::for_skill(2)), None),
            &art.cool
        );
        assert_eq!(
            art.energy(Some(&DamageProfile::for_skill(0)), None),
            &art.storm
        );
        assert_eq!(
            art.energy(Some(&DamageProfile::for_skill(18)), None),
            &art.infection
        );
        let ice = StandardMaterial {
            emissive: LinearRgba::rgb(0.6, 3.0, 5.0),
            ..default()
        };
        assert_eq!(art.energy(None, Some(&ice)), &art.cool);
        let fire = StandardMaterial {
            emissive: LinearRgba::rgb(10.0, 2.0, 0.1),
            ..default()
        };
        assert_eq!(art.energy(None, Some(&fire)), &art.warm);
    }
    #[test]
    fn projection_stays_on_ground_and_grows_with_real_rank_scale_and_height() {
        let low = footprint(Vec3::new(2.0, 0.5, 3.0), Vec3::ONE, 0.72);
        let upgraded = footprint(Vec3::new(2.0, 3.0, 3.0), Vec3::splat(2.0), 0.72);
        assert_eq!(low.translation.y, upgraded.translation.y);
        assert!(upgraded.scale.x > low.scale.x * 2.0);
        assert!(upgraded.translation.x > low.translation.x);
        assert_eq!(footprint(Vec3::ZERO, Vec3::splat(100.0), 1.0).scale.z, 8.0);
    }
}
