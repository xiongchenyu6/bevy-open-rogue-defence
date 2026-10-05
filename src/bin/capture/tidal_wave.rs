//! Real learned-skill wave casts: travelling collision, broad flanks and rank growth.
use super::CaptureControls;
use bevy::prelude::*;
use rogue_commander::game::{
    ActiveSkill, AttackClock, Autoplay, Enemy, GameAssets, GameSession, Hero, HeroKind, HeroVitals,
    PlayerBuild, SelectedHero, SpawnDirector, TidalWave,
};
const ORIGIN: Vec3 = Vec3::new(-8.0, 0.0, 0.0);
#[derive(Component)]
struct WaveTarget {
    outer: bool,
    inner: bool,
}
#[derive(Resource, Default)]
struct WaveCapture {
    frame: u32,
    first_width: Option<f32>,
    upgraded_width: Option<f32>,
    saw_undamaged_ahead: bool,
    natural_cleanup: bool,
    frozen: Vec<(Entity, Vec3, f32, f32)>,
    frozen_health: Vec<(Entity, f32)>,
}
pub fn configure(world: &mut World) {
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SelectedHero>().kind = HeroKind::StoneWarden;
    world.resource_mut::<SelectedHero>().locked = true;
    *world.resource_mut::<HeroVitals>() = HeroVitals::for_hero(HeroKind::StoneWarden, 1);
    world.resource_mut::<CaptureControls>().suppress_drafts = true;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
    world.resource_mut::<SpawnDirector>().enabled = false;
}
pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    world.remove_resource::<Autoplay>();
    world.resource_mut::<PlayerBuild>().skills.fill(0);
    world.resource_mut::<PlayerBuild>().skills[3] = 1;
    world.resource_mut::<ActiveSkill>().remaining = 10_000.0;
    world.resource_mut::<AttackClock>().0.reset();
    world.resource_mut::<AttackClock>().0.pause();
    for mut hero in world
        .query_filtered::<&mut Transform, With<Hero>>()
        .iter_mut(world)
    {
        hero.translation = ORIGIN;
    }
    for (mut camera, mut projection) in world
        .query_filtered::<(&mut Transform, &mut Projection), With<Camera3d>>()
        .iter_mut(world)
    {
        *camera =
            Transform::from_xyz(3.0, 23.0, 19.0).looking_at(Vec3::new(3.0, 0.0, 0.0), Vec3::Y);
        if let Projection::Orthographic(projection) = &mut *projection {
            projection.scaling_mode = bevy::camera::ScalingMode::FixedVertical {
                viewport_height: 23.0,
            };
        }
    }
    let scene = world.resource::<GameAssets>().enemy_scenes[0].clone();
    for (x, z) in [
        (-2., 0.),
        (3., 0.),
        (8., 0.),
        (12., 0.),
        (3., -4.5),
        (3., 4.5),
        (8., -4.5),
        (8., 4.5),
        (3., -7.4),
        (3., 7.4),
        (8., -7.4),
        (8., 7.4),
    ] {
        world.spawn((
            Name::new(format!("Tidal capture target {x}/{z}")),
            WaveTarget {
                outer: f32::abs(z) > 7.,
                inner: f32::abs(z) > 4. && f32::abs(z) < 5.,
            },
            WorldAssetRoot(scene.clone()),
            Transform::from_xyz(x, 0., z).with_scale(Vec3::splat(0.85)),
            Enemy {
                hp: 100_000.,
                max_hp: 100_000.,
                speed: 0.,
                damage: 0.,
                xp: 0,
                wood: 0,
                gold: 0,
                kill_credit: 0,
                elite: false,
            },
        ));
    }
    app.init_resource::<WaveCapture>()
        .add_systems(PreUpdate, advance)
        .add_systems(Last, verify);
    println!(
        "[capture] tidal-wave: real rank1 -> rank5 casts, wide flanks, pause170..194; camera covers 24m wave travel"
    );
}
fn advance(
    mut capture: ResMut<WaveCapture>,
    mut session: ResMut<GameSession>,
    mut build: ResMut<PlayerBuild>,
) {
    capture.frame += 1;
    match capture.frame {
        140 => {
            build.skills[3] = 5;
            println!("[capture] tidal-wave upgraded to rank5");
        }
        170 => session.paused = true,
        195 => session.paused = false,
        300 => {
            build.skills[3] = 0;
            println!("[capture] tidal-wave casts disabled for lifetime cleanup audit");
        }
        _ => {}
    }
}
fn verify(
    mut capture: ResMut<WaveCapture>,
    waves: Query<(Entity, &TidalWave, &Transform)>,
    targets: Query<(Entity, &Enemy, &WaveTarget)>,
    active: Res<ActiveSkill>,
    clock: Res<AttackClock>,
    autoplay: Option<Res<Autoplay>>,
) {
    for (_, wave, _) in &waves {
        if wave.rank == 1 {
            capture.first_width = Some(wave.half_width);
        }
        if wave.rank == 5 {
            capture.upgraded_width = Some(wave.half_width);
        }
        if wave.rank == 1 && wave.travelled < 2. {
            capture.saw_undamaged_ahead = true;
            assert!(
                targets.iter().all(|(_, enemy, _)| enemy.hp == enemy.max_hp),
                "wave must travel to targets before applying damage"
            );
        }
    }
    if capture.frame == 100 {
        assert!(capture.first_width.is_some(), "rank1 must genuinely cast");
        assert!(
            waves.is_empty(),
            "first wave must expire naturally after crossing the arena"
        );
        capture.natural_cleanup = true;
        assert!(
            targets
                .iter()
                .filter(|(_, _, target)| target.inner)
                .all(|(_, enemy, _)| enemy.hp < enemy.max_hp),
            "rank1 must hit both inner flanks"
        );
        assert!(
            targets
                .iter()
                .filter(|(_, _, target)| target.outer)
                .all(|(_, enemy, _)| enemy.hp == enemy.max_hp),
            "rank1 must not reach the outer rank5-only flank"
        );
        println!("[capture] tidal-wave rank1 travelled damage and flank bounds PASS");
    }
    if capture.frame == 169 {
        capture.frozen = waves
            .iter()
            .map(|(entity, wave, transform)| {
                (
                    entity,
                    transform.translation,
                    wave.travelled,
                    wave.remaining,
                )
            })
            .collect();
        capture.frozen_health = targets
            .iter()
            .map(|(entity, enemy, _)| (entity, enemy.hp))
            .collect();
        assert!(
            !capture.frozen.is_empty(),
            "pause must occur while a real upgraded wave is travelling"
        );
    }
    if (170..195).contains(&capture.frame) {
        assert_eq!(waves.iter().count(), capture.frozen.len());
        for (entity, position, travelled, remaining) in &capture.frozen {
            let (_, wave, transform) = waves.get(*entity).expect("paused wave must persist");
            assert_eq!(transform.translation, *position);
            assert_eq!(wave.travelled, *travelled);
            assert_eq!(wave.remaining, *remaining);
        }
        for (entity, hp) in &capture.frozen_health {
            assert_eq!(
                targets.get(*entity).unwrap().1.hp,
                *hp,
                "paused wave cannot deal damage"
            );
        }
    }
    if capture.frame == 270 {
        assert!(
            capture.upgraded_width.unwrap() > capture.first_width.unwrap(),
            "rank5 wave must be wider"
        );
        assert!(
            targets
                .iter()
                .filter(|(_, _, target)| target.outer)
                .all(|(_, enemy, _)| enemy.hp < enemy.max_hp),
            "rank5 must hit the additional outer flanks"
        );
        println!("[capture] tidal-wave rank5 wider collision + pause/resume PASS");
    }
    if capture.frame == 395 {
        assert!(capture.saw_undamaged_ahead && capture.natural_cleanup);
        assert!(
            waves.is_empty(),
            "wave entities must be cleaned after cast lifetime"
        );
        assert!(clock.0.is_paused() && active.remaining > 9000. && autoplay.is_none());
        println!(
            "[capture] tidal-wave COMPLETE: delayed HP hits, both flank widths, rank growth, pause and cleanup PASS"
        );
    }
}
