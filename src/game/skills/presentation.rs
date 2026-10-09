//! Cosmetic skill/status presentation, sampled from authoritative gameplay state.
use super::super::bonds::DamageElement;
use super::infection::Infection;
use super::*;

#[path = "frost_control_art.rs"]
mod frost_control_art;
#[path = "spectacle.rs"]
mod spectacle;
#[path = "vortex_art.rs"]
mod vortex_art;

const MAX_PARTICLES: usize = 600;
const EMIT_INTERVAL: f32 = 0.085;

pub(super) fn install(app: &mut App) {
    frost_control_art::install(app);
    app.init_resource::<EmissionClock>()
        .add_systems(Startup, setup)
        .add_systems(
            PostUpdate,
            (
                animate,
                authored_bodies,
                finish_vortex_ground,
                branch_feedback,
                present,
            )
                .chain()
                .before(TransformSystems::Propagate),
        );
}
#[derive(Resource, Default)]
struct EmissionClock {
    remaining: f32,
    phase: f32,
    frame: u32,
    cast_cooldowns: [f32; SkillId::COUNT],
}
#[derive(Resource)]
struct Art {
    shard: Handle<Mesh>,
    sphere: Handle<Mesh>,
    bar: Handle<Mesh>,
    ring: Handle<Mesh>,
    helix: Handle<Mesh>,
    vortex: Handle<Mesh>,
    vortex_material: Handle<StandardMaterial>,
    vortex_ground: Handle<StandardMaterial>,
    blade: Handle<Mesh>,
    crescent: Handle<Mesh>,
    colors: Vec<Handle<StandardMaterial>>,
}
#[derive(Component)]
struct Particle {
    remaining: f32,
    duration: f32,
    velocity: Vec3,
    spin: f32,
    scale: Vec3,
    growth: f32,
}
#[derive(Clone, Copy)]
enum Motif {
    Fire,
    Ice,
    Shock,
    Water,
    Virus,
    Buff,
    Arcane,
    Steel,
}
impl Motif {
    fn color(self) -> usize {
        match self {
            Self::Fire => 0,
            Self::Ice => 1,
            Self::Shock => 2,
            Self::Water => 3,
            Self::Virus => 4,
            Self::Buff => 5,
            Self::Arcane => 6,
            Self::Steel => 7,
        }
    }
    fn for_profile(profile: DamageProfile) -> Self {
        if profile.skill_index == Some(18) {
            return Self::Virus;
        }
        match profile.element {
            DamageElement::Flame => Self::Fire,
            DamageElement::Frost => Self::Ice,
            DamageElement::Storm => Self::Shock,
            DamageElement::Tide => Self::Water,
            DamageElement::Arcane => Self::Arcane,
            DamageElement::Pure => Self::Buff,
            DamageElement::Physical => Self::Steel,
        }
    }
}
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let colors = [
        (1.0, 0.16, 0.015),
        (0.18, 0.8, 1.0),
        (0.65, 0.45, 1.0),
        (0.01, 0.95, 0.85),
        (0.12, 0.82, 0.38),
        (1.0, 0.68, 0.12),
        (1.0, 0.13, 0.75),
        (0.75, 0.88, 1.0),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (r, g, b))| {
        let infection = index == Motif::Virus.color();
        let intensity = if infection { 6.0 } else { 10.0 };
        materials.add(StandardMaterial {
            base_color: Color::srgba(r, g, b, if infection { 0.72 } else { 1.0 }),
            emissive: LinearRgba::new(r * intensity, g * intensity, b * intensity, 1.0),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        })
    })
    .collect();
    commands.insert_resource(Art {
        shard: meshes.add(crate::game::energy_art::wisp_mesh()),
        sphere: meshes.add(Sphere::new(1.0).mesh().ico(1).unwrap()),
        bar: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        ring: meshes.add(crate::game::energy_art::halo_mesh()),
        helix: meshes.add(spectacle::helix_mesh()),
        vortex: meshes.add(vortex_art::mesh()),
        vortex_material: materials.add(vortex_art::material()),
        vortex_ground: materials.add(vortex_art::ground_material()),
        blade: meshes.add(spectacle::blade_mesh()),
        crescent: meshes.add(spectacle::crescent_mesh()),
        colors,
    });
}
fn finish_vortex_ground(
    art: Res<Art>,
    mut zones: Query<(&SkillZone, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    for (zone, mut material) in &mut zones {
        if zone.kind == ZoneKind::VortexColumn && material.0 != art.vortex_ground {
            material.0 = art.vortex_ground.clone();
        }
    }
}

struct Painter<'a, 'w, 's> {
    identity: Option<usize>,
    commands: &'a mut Commands<'w, 's>,
    art: &'a Art,
    budget: usize,
}
impl Painter<'_, '_, '_> {
    fn emit(
        &mut self,
        motif: Motif,
        mesh: Handle<Mesh>,
        position: Vec3,
        scale: Vec3,
        rotation: Quat,
        velocity: Vec3,
        duration: f32,
        spin: f32,
        growth: f32,
    ) {
        if self.budget == 0 {
            return;
        }
        self.budget -= 1;
        self.commands.spawn((
            Name::new(self.identity.map_or_else(
                || "Skill spectacle effect".into(),
                |index| format!("Skill identity {index}"),
            )),
            Mesh3d(mesh),
            MeshMaterial3d(
                if self.identity == Some(15) && matches!(motif, Motif::Water) {
                    self.art.vortex_material.clone()
                } else {
                    self.art.colors[motif.color()].clone()
                },
            ),
            Transform::from_translation(position)
                .with_scale(scale)
                .with_rotation(rotation),
            Particle {
                remaining: duration,
                duration,
                velocity,
                spin,
                scale,
                growth,
            },
        ));
    }
    fn beam(&mut self, motif: Motif, a: Vec3, b: Vec3, width: f32, duration: f32) {
        let direction = b - a;
        self.emit(
            motif,
            self.art.bar.clone(),
            (a + b) * 0.5,
            Vec3::new(width, direction.length().max(0.01), width),
            Quat::from_rotation_arc(Vec3::Y, direction.normalize_or(Vec3::Y)),
            Vec3::ZERO,
            duration,
            0.0,
            0.0,
        );
    }
    fn ring(&mut self, motif: Motif, position: Vec3, radius: f32, duration: f32, growth: f32) {
        self.emit(
            motif,
            self.art.ring.clone(),
            position,
            Vec3::splat(radius),
            Quat::IDENTITY,
            Vec3::ZERO,
            duration,
            0.0,
            growth,
        );
    }
    fn burst(&mut self, motif: Motif, position: Vec3, rank: u8, phase: f32, large: bool) {
        let rank = rank.clamp(1, 5);
        let size = 1.0 + f32::from(rank - 1) * 0.16;
        let count = if large {
            4 + usize::from(rank)
        } else {
            2 + usize::from(rank) / 2
        };
        match motif {
            Motif::Fire => {
                for i in 0..count {
                    let angle = phase + i as f32 * std::f32::consts::TAU / count as f32;
                    let offset = radial(angle) * 0.35 * size;
                    self.emit(
                        motif,
                        self.art.shard.clone(),
                        position + offset,
                        Vec3::new(0.14, 0.42, 0.14) * size,
                        Quat::from_rotation_z(angle.sin() * 0.3),
                        Vec3::Y * (1.8 + i as f32 * 0.18) + offset * 0.7,
                        0.48,
                        1.5,
                        0.3,
                    );
                    if large {
                        self.emit(
                            Motif::Buff,
                            self.art.sphere.clone(),
                            position + offset,
                            Vec3::splat(0.09),
                            Quat::IDENTITY,
                            offset * 3.0 + Vec3::Y * 2.5,
                            0.5,
                            0.0,
                            0.0,
                        );
                    }
                }
            }
            Motif::Ice => {
                for i in 0..count {
                    let offset = radial(phase + i as f32 * std::f32::consts::TAU / count as f32)
                        * 0.65
                        * size;
                    self.emit(
                        motif,
                        self.art.shard.clone(),
                        position + offset,
                        Vec3::new(0.13, 0.60, 0.13) * size,
                        Quat::from_rotation_arc(Vec3::Y, (offset * 0.35 + Vec3::Y).normalize()),
                        Vec3::Y * 0.08,
                        0.6,
                        0.0,
                        0.08,
                    );
                    if large {
                        self.beam(motif, position, position + offset * 1.8, 0.05, 0.5);
                    }
                }
            }
            Motif::Shock => {
                for i in 0..count {
                    let offset = radial(phase + i as f32 * 2.4) * size;
                    let middle = position + offset * 0.5 + Vec3::Y * 0.65;
                    self.beam(motif, position + Vec3::Y * 0.25, middle, 0.065, 0.23);
                    self.beam(
                        motif,
                        middle,
                        position + offset + Vec3::Y * 0.1,
                        0.055,
                        0.23,
                    );
                }
            }
            Motif::Virus => {
                // Spores move outside the actor silhouette, never through its face.
                for i in 0..count {
                    let angle = phase + i as f32 * std::f32::consts::TAU / count as f32;
                    let offset = radial(angle) * 0.75 * size;
                    self.emit(
                        motif,
                        self.art.shard.clone(),
                        position + offset + Vec3::Y * 0.18,
                        Vec3::new(0.12, 0.35, 0.12) * size,
                        Quat::from_rotation_z(angle.sin() * 0.4),
                        radial(angle + std::f32::consts::FRAC_PI_2) * 0.35 + Vec3::Y * 0.65,
                        0.42,
                        1.0,
                        -0.3,
                    );
                }
                if large {
                    self.ring(motif, position.with_y(0.12), 0.85 * size, 0.35, 0.5);
                }
            }
            Motif::Buff => {
                for i in 0..count {
                    let angle = phase + i as f32 * std::f32::consts::TAU / count as f32;
                    let pos = position + radial(angle) * 0.9 * size + Vec3::Y * 0.5;
                    self.emit(
                        motif,
                        self.art.shard.clone(),
                        pos,
                        Vec3::new(0.12, 0.23, 0.12) * size,
                        Quat::from_rotation_z(std::f32::consts::FRAC_PI_4),
                        Vec3::Y * 1.0,
                        0.6,
                        2.0,
                        0.0,
                    );
                    self.beam(motif, pos - Vec3::Y * 0.4, pos + Vec3::Y * 0.3, 0.04, 0.32);
                }
                if large {
                    self.ring(motif, position.with_y(0.14), 1.0 * size, 0.5, 0.65);
                }
            }
            Motif::Water => {
                for i in 0..count {
                    let offset = radial(phase + i as f32 * 2.4) * 0.55 * size;
                    self.emit(
                        motif,
                        self.art.sphere.clone(),
                        position + offset,
                        Vec3::new(0.09, 0.19, 0.09) * size,
                        Quat::IDENTITY,
                        offset * 1.2 + Vec3::Y,
                        0.50,
                        0.0,
                        0.0,
                    );
                }
                if large {
                    self.ring(motif, position.with_y(0.14), 0.6 * size, 0.6, 2.0);
                }
            }
            Motif::Arcane | Motif::Steel => {
                for i in 0..count {
                    let direction = radial(phase + i as f32 * std::f32::consts::TAU / count as f32);
                    self.emit(
                        motif,
                        self.art.shard.clone(),
                        position + direction * 0.35,
                        Vec3::new(0.08, 0.4, 0.08) * size,
                        Quat::from_rotation_arc(Vec3::Y, direction),
                        direction * 2.5,
                        0.45,
                        2.0,
                        0.1,
                    );
                }
            }
        }
    }
}
fn radial(angle: f32) -> Vec3 {
    Vec3::new(angle.cos(), 0.0, angle.sin())
}
fn animate(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    mut particles: Query<(Entity, &mut Particle, &mut Transform)>,
) {
    if session.paused && !session.finished {
        return;
    }
    for (entity, mut particle, mut transform) in &mut particles {
        particle.remaining -= time.delta_secs();
        if session.finished || particle.remaining <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        let t = 1.0 - particle.remaining / particle.duration;
        transform.translation += particle.velocity * time.delta_secs();
        transform.rotate_y(particle.spin * time.delta_secs());
        transform.scale = particle.scale * (1.0 + particle.growth * t) * ((1.0 - t) * 3.0).min(1.0);
    }
}
#[allow(clippy::type_complexity)]
fn present(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    build: Res<PlayerBuild>,
    runtime: Res<SkillRuntime>,
    art: Option<Res<Art>>,
    mut clock: ResMut<EmissionClock>,
    particles: Query<(), With<Particle>>,
    enemies: Query<(Entity, &Transform, Option<&Infection>), With<Enemy>>,
    heroes: Query<&Transform, With<Hero>>,
    strikes: Query<&SkillStrike, Added<SkillStrike>>,
    projectiles: Query<(&Transform, &DamageProfile, &Projectile)>,
    zones: Query<(&Transform, &SkillZone)>,
    banners: Query<&Transform, With<OathBanner>>,
    new_summons: Query<
        (
            &Transform,
            Option<&BoneSentry>,
            Option<&SpiritHound>,
            Option<&TideElemental>,
            Option<&BoltTurret>,
            Option<&OathBanner>,
            Option<&FrostWyrm>,
            Option<&AncestorColossus>,
        ),
        Added<SkillSummon>,
    >,
) {
    if session.paused || session.finished {
        return;
    }
    let Some(art) = art else {
        return;
    };
    clock.remaining -= time.delta_secs();
    clock.phase += time.delta_secs() * 4.0;
    let tick = clock.remaining <= 0.0;
    if tick {
        clock.remaining = EMIT_INTERVAL;
        clock.frame = clock.frame.wrapping_add(1);
    }
    let mut painter = Painter {
        identity: None,
        commands: &mut commands,
        art: &art,
        budget: MAX_PARTICLES.saturating_sub(particles.iter().count()),
    };
    for index in [25, 29] {
        if build.skills[index] > 0
            && runtime.cooldowns[index] > clock.cast_cooldowns[index] + 0.05
            && let Some(hero) = heroes.iter().next()
        {
            spectacle::impact(
                &mut painter,
                index,
                hero.translation,
                hero.translation,
                1.3,
                build.skills[index],
                clock.phase,
                None,
            );
        }
        clock.cast_cooldowns[index] = runtime.cooldowns[index];
    }
    // New impact/cast requests are always shown, even between sampled status frames.
    for strike in strikes.iter().take(24) {
        let Some(index) = strike.profile.skill_index else {
            continue;
        };
        let position = strike.fixed_position.or_else(|| {
            enemies
                .get(strike.target)
                .ok()
                .map(|(_, t, _)| t.translation)
        });
        if let Some(position) = position {
            let index = usize::from(index);
            let origin = heroes
                .iter()
                .next()
                .map_or(position, |hero| hero.translation);
            spectacle::impact(
                &mut painter,
                index,
                origin,
                position,
                strike.radius,
                build.skills[index],
                clock.phase,
                None,
            );
            painter.burst(
                Motif::for_profile(strike.profile),
                position.with_y(0.4),
                build.skills[index],
                clock.phase,
                true,
            );
        }
    }
    for (transform, bone, hound, tide, turret, banner, wyrm, colossus) in &new_summons {
        let index = if bone.is_some() {
            4
        } else if let Some(hound) = hound {
            hound.source_skill
        } else if tide.is_some() {
            16
        } else if turret.is_some() {
            17
        } else if banner.is_some() {
            21
        } else if wyrm.is_some() {
            26
        } else if colossus.is_some() {
            27
        } else {
            continue;
        };
        spectacle::impact(
            &mut painter,
            index,
            transform.translation,
            transform.translation,
            1.2,
            build.skills[index],
            clock.phase,
            None,
        );
    }
    if !tick {
        return;
    }
    // Persistent conditions use real remaining timers; visuals never apply damage/status.
    for (entity, transform, infection) in enemies.iter().take(32) {
        let position = transform.translation.with_y(0.35);
        if let Some(burn) = runtime
            .burns
            .get(&entity)
            .filter(|burn| burn.remaining > 0.0)
        {
            painter.burst(
                Motif::Fire,
                position,
                burn.layers.clamp(1, 5),
                clock.phase,
                false,
            );
        }
        if let Some(control) = runtime.controls.get(&entity) {
            if control.stunned_remaining > 0.0 {
                painter.burst(
                    Motif::Shock,
                    position + Vec3::Y * 1.6,
                    1,
                    clock.phase,
                    false,
                );
            } else if control.slow_remaining > 0.0 && clock.frame.is_multiple_of(3) {
                painter.burst(Motif::Water, position, 1, clock.phase, false);
            }
        }
        if runtime
            .wet
            .get(&entity)
            .is_some_and(|remaining| *remaining > 0.0)
            && clock.frame.is_multiple_of(3)
        {
            painter.burst(Motif::Water, position, 1, clock.phase, false);
        }
        if let Some(infection) = infection.filter(|infection| infection.remaining() > 0.0)
            && clock.frame % 3 == 1
        {
            painter.burst(
                Motif::Virus,
                position,
                infection.rank(),
                clock.phase + f32::from(infection.generation()),
                false,
            );
        }
    }
    // Reserve the same global particle budget for every emitter, including large volleys.
    for (transform, profile, projectile) in projectiles
        .iter()
        .filter(|(_, profile, _)| profile.skill_index.is_some())
        .take(16)
    {
        let rank = profile
            .skill_index
            .map_or(1, |index| build.skills[usize::from(index)]);
        spectacle::projectile(
            &mut painter,
            profile.skill_index.unwrap() as usize,
            transform.translation,
            projectile.velocity,
            rank,
            clock.phase,
        );
        painter.burst(
            Motif::for_profile(*profile),
            transform.translation,
            rank,
            clock.phase,
            false,
        );
    }
    if clock.frame.is_multiple_of(3) {
        for (transform, zone) in zones.iter().take(8) {
            let rank = zone
                .profile
                .skill_index
                .map_or(1, |index| build.skills[usize::from(index)]);
            spectacle::zone(&mut painter, zone, transform.translation, rank, clock.phase);
        }
        for transform in &banners {
            spectacle::impact(
                &mut painter,
                21,
                transform.translation,
                transform.translation,
                1.0,
                build.skills[21],
                clock.phase,
                None,
            );
            painter.burst(
                Motif::Buff,
                transform.translation.with_y(0.2),
                build.skills[21],
                clock.phase,
                true,
            );
        }
        if runtime.spell_power_multiplier > 1.01
            || runtime.summon_inherit_multiplier > 1.01
            || build.skills[20] > 0
            || build.skills[28] > 0
        {
            for transform in &heroes {
                for index in [20, 28] {
                    if build.skills[index] > 0 {
                        spectacle::impact(
                            &mut painter,
                            index,
                            transform.translation,
                            transform.translation,
                            1.0,
                            build.skills[index],
                            clock.phase,
                            None,
                        );
                    }
                }
                painter.burst(
                    Motif::Buff,
                    transform.translation.with_y(0.2),
                    build.skills[20].max(build.skills[28]).max(1),
                    clock.phase,
                    true,
                );
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn authored_bodies(
    mut commands: Commands,
    session: Res<GameSession>,
    art: Option<Res<Art>>,
    build: Res<PlayerBuild>,
    mut bodies: Query<
        (
            Entity,
            &mut Mesh3d,
            &mut MeshMaterial3d<StandardMaterial>,
            &mut Transform,
            Option<&HomingSword>,
        ),
        (
            Or<(Added<HomingSword>, Added<BloodWheel>)>,
            Without<Particle>,
        ),
    >,
) {
    if session.paused || session.finished {
        return;
    }
    let Some(art) = art else {
        return;
    };
    for (entity, mut mesh, mut material, mut transform, sword) in &mut bodies {
        commands
            .entity(entity)
            .insert(Name::new(if sword.is_some() {
                "Skill identity 19"
            } else {
                "Skill identity 23"
            }));
        if sword.is_some() {
            mesh.0 = art.blade.clone();
            material.0 = art.colors[Motif::Steel.color()].clone();
            transform.scale = Vec3::splat(0.65 + f32::from(build.skills[19]) * 0.09);
        } else {
            mesh.0 = art.crescent.clone();
            material.0 = art.colors[Motif::Fire.color()].clone();
            transform.scale = Vec3::ONE;
        }
    }
}
fn branch_feedback(
    mut commands: Commands,
    session: Res<GameSession>,
    build: Res<PlayerBuild>,
    art: Option<Res<Art>>,
    particles: Query<(), With<Particle>>,
    mut cues: MessageReader<super::ur_evolutions::UrBranchCue>,
) {
    if session.paused || session.finished {
        cues.clear();
        return;
    }
    let Some(art) = art else {
        cues.clear();
        return;
    };
    let mut painter = Painter {
        identity: None,
        commands: &mut commands,
        art: &art,
        budget: MAX_PARTICLES.saturating_sub(particles.iter().count()),
    };
    for cue in cues.read().take(16) {
        spectacle::impact(
            &mut painter,
            cue.skill,
            cue.origin,
            cue.target,
            1.0,
            build.skills[cue.skill],
            0.0,
            Some(cue.path),
        );
    }
}

pub(super) fn reset_run(world: &mut World) {
    frost_control_art::reset_run(world);
    world.insert_resource(EmissionClock::default());
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn simultaneous_skill_identities_and_ur_branches_share_one_particle_budget() {
        use crate::game::skill_evolution::UrEvolutionPath;
        use bevy::ecs::system::RunSystemOnce;
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>();
        app.world_mut().run_system_once(setup).unwrap();
        let meshes = app.world().resource::<Assets<Mesh>>().len();
        let materials = app.world().resource::<Assets<StandardMaterial>>().len();
        app.world_mut()
            .run_system_once(|mut commands: Commands, art: Res<Art>| {
                let mut painter = Painter {
                    identity: None,
                    commands: &mut commands,
                    art: &art,
                    budget: MAX_PARTICLES,
                };
                for index in 0..SkillId::COUNT {
                    for path in [
                        UrEvolutionPath::Surge,
                        UrEvolutionPath::Control,
                        UrEvolutionPath::Sustain,
                    ] {
                        spectacle::impact(
                            &mut painter,
                            index,
                            Vec3::ZERO,
                            Vec3::X * 4.0,
                            2.0,
                            5,
                            0.0,
                            Some(path),
                        );
                    }
                }
            })
            .unwrap();
        assert_eq!(
            app.world_mut()
                .query::<&Particle>()
                .iter(app.world())
                .count(),
            MAX_PARTICLES
        );
        assert_eq!(app.world().resource::<Assets<Mesh>>().len(), meshes);
        assert_eq!(
            app.world().resource::<Assets<StandardMaterial>>().len(),
            materials
        );
        assert_eq!(
            app.world_mut()
                .query::<&SkillStrike>()
                .iter(app.world())
                .count(),
            0
        );
    }
    #[test]
    fn dense_infection_feedback_is_bounded_pauses_and_cleans_up() {
        let mut app = App::new();
        app.add_message::<super::super::ur_evolutions::UrBranchCue>()
            .init_resource::<Time>()
            .init_resource::<GameSession>()
            .init_resource::<PlayerBuild>()
            .init_resource::<SkillRuntime>()
            .init_resource::<EmissionClock>()
            .insert_resource(Art {
                shard: default(),
                sphere: default(),
                bar: default(),
                ring: default(),
                helix: default(),
                vortex: default(),
                vortex_material: default(),
                vortex_ground: default(),
                blade: default(),
                crescent: default(),
                colors: vec![Handle::default(); 8],
            })
            .add_systems(
                Update,
                (
                    animate,
                    authored_bodies,
                    finish_vortex_ground,
                    branch_feedback,
                    present,
                )
                    .chain(),
            );
        for index in 0..32 {
            app.world_mut().spawn((
                Enemy {
                    hp: 100.0,
                    max_hp: 100.0,
                    speed: 0.0,
                    damage: 0.0,
                    xp: 0,
                    wood: 0,
                    gold: 0,
                    kill_credit: 1,
                    elite: false,
                },
                Transform::from_xyz(index as f32, 0.0, 0.0),
                Infection::new(5, 0, 1.0),
            ));
        }
        for _ in 0..10 {
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_secs_f32(EMIT_INTERVAL));
            app.update();
            let count = app
                .world_mut()
                .query::<&Particle>()
                .iter(app.world())
                .count();
            assert!((1..=MAX_PARTICLES).contains(&count));
        }
        let count = app
            .world_mut()
            .query::<&Particle>()
            .iter(app.world())
            .count();
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&Particle>()
                .iter(app.world())
                .count(),
            count
        );
        app.world_mut().resource_mut::<GameSession>().finished = true;
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&Particle>()
                .iter(app.world())
                .count(),
            0
        );
    }
}
