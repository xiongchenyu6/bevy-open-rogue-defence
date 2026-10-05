//! Hero-specific ultimate silhouettes. All entities here are cosmetic and bounded.
use super::{GameSession, HeroKind};
use bevy::prelude::*;

const MAX_PIECES: usize = 420;
const MAX_CUES: usize = 24;

#[derive(Clone, Copy)]
pub(crate) enum UltimatePhase {
    Cast,
    Pulse,
}
struct Cue {
    kind: HeroKind,
    origin: Vec3,
    target: Vec3,
    phase: UltimatePhase,
}
#[derive(Resource, Default)]
pub(crate) struct UltimateCues {
    pending: Vec<Cue>,
    pub(crate) serial: u64,
}
impl UltimateCues {
    pub(crate) fn push(
        &mut self,
        kind: HeroKind,
        origin: Vec3,
        target: Vec3,
        phase: UltimatePhase,
    ) {
        if matches!(phase, UltimatePhase::Cast) {
            self.serial = self.serial.wrapping_add(1);
        }
        if self.pending.len() < MAX_CUES {
            self.pending.push(Cue {
                kind,
                origin,
                target,
                phase,
            });
        }
    }
}

pub(crate) struct UltimateVfxPlugin;
impl Plugin for UltimateVfxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UltimateCues>()
            .add_systems(Startup, setup)
            .add_systems(PostUpdate, animate.before(TransformSystems::Propagate));
    }
}
#[derive(Resource)]
struct Art {
    bar: Handle<Mesh>,
    crystal: Handle<Mesh>,
    ring: Handle<Mesh>,
    colors: Vec<Handle<StandardMaterial>>,
    white: Handle<StandardMaterial>,
}
#[derive(Component)]
struct Piece {
    age: f32,
    duration: f32,
    delay: f32,
    scale: Vec3,
    velocity: Vec3,
    spin: f32,
    expand: f32,
}
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let colors = [
        (0.25, 0.70, 1.0),
        (1.0, 0.24, 0.04),
        (0.84, 0.62, 0.30),
        (0.25, 1.0, 0.63),
        (0.1, 0.80, 0.92),
        (1.0, 0.62, 0.94),
        (0.60, 0.9, 1.0),
        (1.0, 0.12, 0.10),
        (0.75, 0.3, 1.0),
        (0.4, 1.0, 0.17),
        (1.0, 0.77, 0.25),
        (0.49, 0.14, 0.94),
    ]
    .into_iter()
    .map(|(r, g, b)| {
        materials.add(StandardMaterial {
            base_color: Color::srgb(r, g, b),
            emissive: LinearRgba::new(r * 12.0, g * 12.0, b * 12.0, 1.0),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        })
    })
    .collect();
    commands.insert_resource(Art {
        bar: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        crystal: meshes.add(crate::game::energy_art::wisp_mesh()),
        ring: meshes.add(crate::game::energy_art::halo_mesh()),
        colors,
        white: materials.add(StandardMaterial {
            base_color: Color::srgb(0.95, 0.97, 1.0),
            emissive: LinearRgba::new(16.0, 16.0, 16.0, 1.0),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        }),
    });
}
struct Painter<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    art: &'a Art,
    available: usize,
    material: Handle<StandardMaterial>,
}
impl Painter<'_, '_, '_> {
    fn piece(
        &mut self,
        mesh: Handle<Mesh>,
        transform: Transform,
        velocity: Vec3,
        duration: f32,
        delay: f32,
        spin: f32,
        expand: f32,
    ) {
        if self.available == 0 {
            return;
        }
        self.available -= 1;
        self.commands.spawn((
            Name::new("Hero ultimate effect"),
            Mesh3d(mesh),
            MeshMaterial3d(self.material.clone()),
            Piece {
                age: 0.0,
                duration,
                delay,
                scale: transform.scale,
                velocity,
                spin,
                expand,
            },
            transform,
            if delay > 0.0 {
                Visibility::Hidden
            } else {
                Visibility::Visible
            },
        ));
    }
    fn beam(&mut self, a: Vec3, b: Vec3, width: f32, duration: f32, delay: f32) {
        let delta = b - a;
        let transform = Transform::from_translation((a + b) * 0.5)
            .with_rotation(Quat::from_rotation_arc(Vec3::Y, delta.normalize_or_zero()))
            .with_scale(Vec3::new(width, delta.length().max(0.01), width));
        self.piece(
            self.art.bar.clone(),
            transform,
            Vec3::ZERO,
            duration,
            delay,
            0.0,
            0.0,
        );
    }
    fn ring(&mut self, center: Vec3, radius: f32, duration: f32, expand: f32, rotation: Quat) {
        self.piece(
            self.art.ring.clone(),
            Transform::from_translation(center)
                .with_scale(Vec3::splat(radius))
                .with_rotation(rotation),
            Vec3::ZERO,
            duration,
            0.0,
            0.4,
            expand,
        );
    }
    fn crystal(&mut self, pos: Vec3, size: Vec3, velocity: Vec3, duration: f32, delay: f32) {
        self.piece(
            self.art.crystal.clone(),
            Transform::from_translation(pos).with_scale(size),
            velocity,
            duration,
            delay,
            0.0,
            0.15,
        );
    }
    fn arc(
        &mut self,
        center: Vec3,
        radius: f32,
        start: f32,
        sweep: f32,
        width: f32,
        duration: f32,
        delay: f32,
    ) {
        for segment in 0..12 {
            let a = start + sweep * segment as f32 / 12.0;
            let b = start + sweep * (segment + 1) as f32 / 12.0;
            self.beam(
                center + radial(a) * radius,
                center + radial(b) * radius,
                width,
                duration,
                delay + segment as f32 * 0.012,
            );
        }
    }
    fn paint(&mut self, cue: Cue) {
        use HeroKind::*;
        let cast = matches!(cue.phase, UltimatePhase::Cast);
        let origin = Vec3::new(cue.origin.x, 0.55, cue.origin.z);
        let target = Vec3::new(cue.target.x, 0.45, cue.target.z);
        let aim = (target - origin).with_y(0.0).normalize_or(Vec3::X);
        let side = Vec3::new(-aim.z, 0.0, aim.x);
        match cue.kind {
            StormSavant => {
                // Forked vertical strikes, with a broken electric crown on cast.
                let count = if cast { 5 } else { 2 };
                for fork in 0..count {
                    let foot = target + radial(fork as f32 * 2.4) * if cast { 1.3 } else { 0.35 };
                    let mut previous = foot + Vec3::Y * 5.0;
                    for segment in 0..5 {
                        let next = foot
                            + Vec3::Y * (4 - segment) as f32
                            + side * if segment % 2 == 0 { 0.4 } else { -0.35 };
                        self.beam(previous, next, 0.08, 0.38, fork as f32 * 0.06);
                        previous = next;
                    }
                }
            }
            EmberRanger => {
                // A broad arrowhead volley rather than a radial sphere.
                for lane in -3i32..=3 {
                    let direction = (aim + side * lane as f32 * 0.19).normalize();
                    let start = origin + direction * 0.7;
                    let rotation = Quat::from_rotation_arc(Vec3::Y, direction);
                    self.piece(
                        self.art.crystal.clone(),
                        Transform::from_translation(start)
                            .with_rotation(rotation)
                            .with_scale(Vec3::new(0.18, 0.85, 0.18)),
                        direction * 14.0,
                        0.65,
                        lane.unsigned_abs() as f32 * 0.035,
                        0.0,
                        0.0,
                    );
                    self.beam(start, start - direction * 1.0, 0.1, 0.18, 0.0);
                }
            }
            StoneWarden => {
                for i in 0..8 {
                    let angle = i as f32 * std::f32::consts::TAU / 8.0;
                    let point = origin + radial(angle) * if cast { 3.5 } else { 2.5 };
                    self.crystal(
                        point.with_y(-0.5),
                        Vec3::new(0.5, 1.6, 0.5),
                        Vec3::Y * 2.3,
                        1.0,
                        i as f32 * 0.035,
                    );
                    self.beam(
                        origin.with_y(0.1),
                        point.with_y(0.1),
                        0.055,
                        0.65,
                        i as f32 * 0.035,
                    );
                }
            }
            SpiritKeeper => {
                // Six rising spectral wisps match the summoned orbiting companions.
                for i in 0..6 {
                    let angle = i as f32 * std::f32::consts::TAU / 6.0;
                    let point = origin + radial(angle) * 2.0;
                    self.crystal(
                        point,
                        Vec3::new(0.30, 0.55, 0.30),
                        Vec3::Y * 0.9,
                        1.5,
                        i as f32 * 0.09,
                    );
                    self.arc(origin, 2.0, angle, 0.65, 0.08, 1.25, i as f32 * 0.06);
                    self.beam(point, point - Vec3::Y * 0.8, 0.06, 1.2, i as f32 * 0.09);
                }
            }
            TideWalker => {
                // Parallel curling wave crests travel forward across the arena.
                for wave in 0..3 {
                    for segment in -6..=6 {
                        let point = origin + side * segment as f32 * 0.65 + aim * wave as f32 * 0.8;
                        self.piece(
                            self.art.bar.clone(),
                            Transform::from_translation(
                                point + Vec3::Y * (0.25 + (segment as f32 * 0.45).cos() * 0.2),
                            )
                            .with_rotation(Quat::from_rotation_arc(Vec3::X, side))
                            .with_scale(Vec3::new(0.62, 0.24, 0.14)),
                            aim * 9.0,
                            1.3,
                            wave as f32 * 0.13,
                            0.0,
                            0.0,
                        );
                    }
                }
            }
            StarWagerer => {
                // Five-point falling constellation, then its connecting pentagram.
                let center = target + Vec3::Y * 0.8;
                for i in 0..5 {
                    let a = radial(i as f32 * std::f32::consts::TAU / 5.0) * 2.5;
                    let b = radial((i + 2) as f32 * std::f32::consts::TAU / 5.0) * 2.5;
                    self.beam(center + a, center + b, 0.075, 1.2, 0.25);
                    self.crystal(
                        center + a + Vec3::Y * 3.5,
                        Vec3::new(0.26, 0.42, 0.26),
                        -Vec3::Y * 4.0,
                        0.9,
                        i as f32 * 0.05,
                    );
                }
            }
            FrostOracle => {
                // A six-armed snowflake fractures into an outward-growing ice field.
                for i in 0..6 {
                    let direction = radial(i as f32 * std::f32::consts::TAU / 6.0);
                    self.beam(target, target + direction * 3.8, 0.06, 1.0, 0.0);
                    for distance in 1..=3 {
                        self.crystal(
                            target + direction * distance as f32 * 1.2,
                            Vec3::new(0.2, 0.6 + distance as f32 * 0.20, 0.2),
                            Vec3::Y * 0.4,
                            0.9,
                            distance as f32 * 0.1,
                        );
                    }
                }
            }
            IronBerserker => {
                self.arc(
                    origin,
                    3.2,
                    0.0,
                    std::f32::consts::TAU * 0.85,
                    0.22,
                    0.38,
                    0.0,
                );
                self.arc(
                    origin + Vec3::Y * 0.3,
                    2.5,
                    std::f32::consts::PI,
                    std::f32::consts::TAU * 0.8,
                    0.12,
                    0.32,
                    0.08,
                );
                for i in 0..6 {
                    let d = radial(i as f32 * 1.05);
                    self.crystal(
                        origin + d * 2.8,
                        Vec3::new(0.2, 0.4, 0.2),
                        d * 4.0,
                        0.38,
                        0.0,
                    );
                }
            }
            RiftDancer => {
                // A teleport trace ends in two staggered crossing blade cuts.
                self.beam(origin, target, 0.055, 0.28, 0.0);
                self.beam(
                    target + Vec3::new(-1.2, 0.0, -1.2),
                    target + Vec3::new(1.2, 1.8, 1.2),
                    0.18,
                    0.30,
                    0.0,
                );
                self.beam(
                    target + Vec3::new(-1.2, 1.8, 1.2),
                    target + Vec3::new(1.2, 0.0, -1.2),
                    0.14,
                    0.30,
                    0.12,
                );
            }
            GroveArtificer => {
                if cast {
                    for i in 0..4 {
                        let point = origin
                            + radial(
                                std::f32::consts::FRAC_PI_4
                                    + i as f32 * std::f32::consts::FRAC_PI_2,
                            ) * 2.2;
                        self.crystal(point, Vec3::new(0.35, 1.1, 0.35), Vec3::Y * 0.3, 1.4, 0.0);
                        self.beam(point, origin + Vec3::Y * 2.0, 0.075, 1.3, 0.0);
                    }
                } else {
                    let apex = (origin + target) * 0.5 + Vec3::Y * 4.0;
                    self.beam(origin + Vec3::Y, apex, 0.13, 0.35, 0.0);
                    self.beam(apex, target, 0.18, 0.35, 0.10);
                    self.arc(target, 1.5, 0.0, std::f32::consts::TAU, 0.07, 0.4, 0.15);
                }
            }
            SolarVanguard => {
                // Upright shield plates make a protective bastion, gold rays rise inside.
                for i in 0..6 {
                    let angle = i as f32 * std::f32::consts::TAU / 6.0;
                    let point = origin + radial(angle) * 2.2;
                    self.piece(
                        self.art.bar.clone(),
                        Transform::from_translation(point + Vec3::Y * 0.7)
                            .with_rotation(Quat::from_rotation_y(-angle))
                            .with_scale(Vec3::new(0.12, 1.8, 0.7)),
                        Vec3::Y * 0.35,
                        if cast { 1.5 } else { 0.55 },
                        0.0,
                        0.0,
                        0.0,
                    );
                    self.beam(point, point + Vec3::Y * 3.4, 0.045, 0.75, i as f32 * 0.06);
                }
            }
            VoidReaper => {
                // Inclined accretion rings and contracting spokes pull inward.
                self.ring(
                    target + Vec3::Y * 0.65,
                    2.7,
                    1.05,
                    -0.72,
                    Quat::from_rotation_x(0.45),
                );
                self.ring(
                    target + Vec3::Y * 0.65,
                    2.2,
                    0.85,
                    -0.8,
                    Quat::from_rotation_z(-0.6),
                );
                for i in 0..10 {
                    let d = radial(i as f32 * std::f32::consts::TAU / 10.0);
                    self.crystal(
                        target + d * 3.7 + Vec3::Y * 0.5,
                        Vec3::new(0.15, 0.30, 0.15),
                        -d * 3.8,
                        0.85,
                        i as f32 * 0.015,
                    );
                }
            }
        }
        // Small white highlight increases contrast without painting a generic explosion.
        if cast {
            let old = self.material.clone();
            self.material = self.art.white.clone();
            self.beam(origin, origin + Vec3::Y * 1.1, 0.065, 0.20, 0.0);
            self.material = old;
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
    art: Option<Res<Art>>,
    mut cues: ResMut<UltimateCues>,
    mut pieces: Query<(Entity, &mut Piece, &mut Transform, &mut Visibility)>,
) {
    if session.finished {
        cues.pending.clear();
        for (entity, ..) in &mut pieces {
            commands.entity(entity).despawn();
        }
        return;
    }
    if session.paused {
        return;
    }
    let dt = time.delta_secs();
    let mut active = 0;
    for (entity, mut piece, mut transform, mut visibility) in &mut pieces {
        piece.age += dt;
        let age = piece.age - piece.delay;
        if age >= piece.duration {
            commands.entity(entity).despawn();
            continue;
        }
        active += 1;
        if age < 0.0 {
            continue;
        }
        *visibility = Visibility::Visible;
        transform.translation += piece.velocity * dt;
        transform.rotate_y(piece.spin * dt);
        let t = (age / piece.duration).clamp(0.0, 1.0);
        let fade = ((1.0 - t) * 5.0).min(1.0);
        transform.scale = piece.scale * (1.0 + piece.expand * t).max(0.05) * fade;
    }
    let Some(art) = art else {
        cues.pending.clear();
        return;
    };
    let mut painter = Painter {
        commands: &mut commands,
        art: &art,
        available: MAX_PIECES.saturating_sub(active),
        material: art.white.clone(),
    };
    for cue in cues.pending.drain(..) {
        painter.material = art.colors[cue.kind.index()].clone();
        painter.paint(cue);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn queued_ultimate_cues_are_bounded() {
        let mut cues = UltimateCues::default();
        for _ in 0..100 {
            cues.push(
                HeroKind::StormSavant,
                Vec3::ZERO,
                Vec3::X,
                UltimatePhase::Pulse,
            );
        }
        assert_eq!(cues.pending.len(), MAX_CUES);
        cues.push(
            HeroKind::StormSavant,
            Vec3::ZERO,
            Vec3::X,
            UltimatePhase::Cast,
        );
        assert_eq!(
            cues.serial, 1,
            "sound signal survives visual queue saturation"
        );
        cues.pending.clear();
        assert_eq!(cues.serial, 1);
    }
    #[test]
    fn ultimate_effects_freeze_on_pause_and_clean_up_at_run_end() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<GameSession>()
            .init_resource::<UltimateCues>()
            .add_systems(Update, animate);
        let entity = app
            .world_mut()
            .spawn((
                Piece {
                    age: 0.2,
                    duration: 1.0,
                    delay: 0.0,
                    scale: Vec3::ONE,
                    velocity: Vec3::X,
                    spin: 1.0,
                    expand: 1.0,
                },
                Transform::default(),
                Visibility::Visible,
            ))
            .id();
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.update();
        assert_eq!(app.world().get::<Piece>(entity).unwrap().age, 0.2);
        app.world_mut().resource_mut::<GameSession>().finished = true;
        app.update();
        assert!(app.world().get_entity(entity).is_err());
    }
}
