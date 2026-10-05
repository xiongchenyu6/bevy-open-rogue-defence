//! Hostile silhouettes and impacts driven only by actual resolved combat actions.
use super::{EnemyAttackCue, EnemyAttackKind, GameSession};
use bevy::prelude::*;
const MAX_PIECES: usize = 240;
pub(super) struct EnemyVfxPlugin;
impl Plugin for EnemyVfxPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<EnemyAttackCue>()
            .add_systems(Startup, setup)
            .add_systems(PostUpdate, animate.before(TransformSystems::Propagate));
    }
}
#[derive(Resource)]
struct Art {
    streak: Handle<Mesh>,
    shard: Handle<Mesh>,
    ring: Handle<Mesh>,
    acid: Handle<StandardMaterial>,
    red: Handle<StandardMaterial>,
    gold: Handle<StandardMaterial>,
    green: Handle<StandardMaterial>,
}
#[derive(Component)]
struct Piece {
    age: f32,
    duration: f32,
    from: Vec3,
    to: Vec3,
    arc: f32,
    scale: Vec3,
    expand: bool,
    flight: bool,
    kind: EnemyAttackKind,
}
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut color = |r, g, b| {
        materials.add(StandardMaterial {
            base_color: Color::srgb(r, g, b),
            emissive: LinearRgba::new(r * 5., g * 5., b * 5., 1.),
            unlit: true,
            ..default()
        })
    };
    commands.insert_resource(Art {
        streak: meshes.add(Cuboid::new(0.12, 0.12, 1.0)),
        shard: meshes.add(ConicalFrustum {
            radius_top: 0.,
            radius_bottom: 0.24,
            height: 0.65,
        }),
        ring: meshes.add(Torus::new(0.78, 0.88)),
        acid: color(0.4, 1., 0.05),
        red: color(1., 0.08, 0.04),
        gold: color(1., 0.52, 0.05),
        green: color(0.05, 1., 0.65),
    });
}
fn material(art: &Art, kind: EnemyAttackKind) -> Handle<StandardMaterial> {
    match kind {
        EnemyAttackKind::Shot => art.acid.clone(),
        EnemyAttackKind::Heal => art.green.clone(),
        EnemyAttackKind::Slam => art.gold.clone(),
        _ => art.red.clone(),
    }
}
#[allow(clippy::too_many_arguments)]
fn spawn(
    commands: &mut Commands,
    art: &Art,
    kind: EnemyAttackKind,
    name: &str,
    mesh: Handle<Mesh>,
    from: Vec3,
    to: Vec3,
    scale: Vec3,
    duration: f32,
    arc: f32,
    expand: bool,
    flight: bool,
    budget: &mut usize,
) {
    if *budget == 0 {
        return;
    }
    *budget -= 1;
    let mut transform = Transform::from_translation(from).with_scale(scale);
    if flight {
        transform = transform.looking_at(to, Vec3::Y);
    }
    commands.spawn((
        Name::new(name.to_string()),
        Mesh3d(mesh),
        MeshMaterial3d(material(art, kind)),
        transform,
        Piece {
            age: 0.,
            duration,
            from,
            to,
            arc,
            scale,
            expand,
            flight,
            kind,
        },
    ));
}
fn impact(commands: &mut Commands, art: &Art, kind: EnemyAttackKind, at: Vec3, budget: &mut usize) {
    spawn(
        commands,
        art,
        kind,
        "Enemy attack impact",
        art.ring.clone(),
        at,
        at,
        Vec3::splat(0.3),
        0.32,
        0.,
        true,
        false,
        budget,
    );
    for i in 0..8 {
        let angle = i as f32 * std::f32::consts::TAU / 8.;
        let end = at + Vec3::new(angle.cos() * 0.8, 0.4, angle.sin() * 0.8);
        spawn(
            commands,
            art,
            kind,
            "Enemy attack spark",
            art.streak.clone(),
            at,
            end,
            Vec3::new(1.5, 1.5, 0.25),
            0.24,
            0.,
            false,
            false,
            budget,
        );
    }
}
fn animate(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    art: Res<Art>,
    mut cues: MessageReader<EnemyAttackCue>,
    mut pieces: Query<(Entity, &mut Piece, &mut Transform)>,
) {
    if session.finished {
        cues.clear();
        for (entity, _, _) in &mut pieces {
            commands.entity(entity).despawn();
        }
        return;
    }
    if session.paused {
        cues.clear();
        return;
    }
    let mut budget = MAX_PIECES.saturating_sub(pieces.iter().len());
    for (entity, mut piece, mut transform) in &mut pieces {
        piece.age += time.delta_secs();
        if piece.age >= piece.duration {
            if piece.flight {
                impact(&mut commands, &art, piece.kind, piece.to, &mut budget);
            }
            commands.entity(entity).despawn();
            continue;
        }
        let t = piece.age / piece.duration;
        transform.translation =
            piece.from.lerp(piece.to, t) + Vec3::Y * (4. * piece.arc * t * (1. - t));
        if piece.flight {
            let direction = (piece.to - piece.from) + Vec3::Y * (4. * piece.arc * (1. - 2. * t));
            transform.look_to(direction, Vec3::Y);
        }
        transform.scale = piece.scale
            * if piece.expand {
                1. + t * 5.
            } else {
                1. - t * 0.5
            };
    }
    for cue in cues.read().take(24) {
        let from = cue.origin + Vec3::Y * 0.85;
        let to = cue.target + Vec3::Y * 0.55;
        match cue.kind {
            EnemyAttackKind::Shot => {
                spawn(
                    &mut commands,
                    &art,
                    cue.kind,
                    "Enemy attack flight",
                    art.streak.clone(),
                    from,
                    to,
                    Vec3::new(2., 2., 1.5),
                    0.3,
                    0.35,
                    false,
                    true,
                    &mut budget,
                );
                // Broad fletching distinguishes hostile darts from point bullets.
                for side in [-1., 1.] {
                    spawn(
                        &mut commands,
                        &art,
                        cue.kind,
                        "Enemy attack flight fin",
                        art.shard.clone(),
                        from + Vec3::X * side * 0.12,
                        to + Vec3::X * side * 0.12,
                        Vec3::splat(0.65),
                        0.3,
                        0.35,
                        false,
                        true,
                        &mut budget,
                    );
                }
            }
            EnemyAttackKind::Slash => {
                let direction = (to - from).with_y(0.).normalize_or(Vec3::X);
                let angle = direction.z.atan2(direction.x);
                for i in 0..10 {
                    let a = angle - 0.9 + i as f32 * 0.18;
                    let p = from + Vec3::new(a.cos(), 0., a.sin()) * 0.85;
                    spawn(
                        &mut commands,
                        &art,
                        cue.kind,
                        "Enemy attack slash",
                        art.streak.clone(),
                        p,
                        p,
                        Vec3::new(1.8, 1.8, 0.25),
                        0.22,
                        0.,
                        false,
                        false,
                        &mut budget,
                    );
                }
                impact(&mut commands, &art, cue.kind, to, &mut budget);
            }
            EnemyAttackKind::Beam => {
                for i in 0..12 {
                    let p = from.lerp(to, i as f32 / 11.);
                    spawn(
                        &mut commands,
                        &art,
                        cue.kind,
                        "Enemy attack beam",
                        art.streak.clone(),
                        p,
                        p,
                        Vec3::new(2., 2., 0.55),
                        0.24,
                        0.,
                        false,
                        false,
                        &mut budget,
                    );
                }
                impact(&mut commands, &art, cue.kind, to, &mut budget);
            }
            EnemyAttackKind::Slam | EnemyAttackKind::Burst | EnemyAttackKind::Heal => {
                impact(&mut commands, &art, cue.kind, to.with_y(0.18), &mut budget)
            }
        }
    }
}
