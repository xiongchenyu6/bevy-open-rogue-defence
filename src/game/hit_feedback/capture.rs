//! Deterministic contact inputs; all reactions, audio, FX and camera are production systems.
//! No gameplay damage is simulated here: this isolates the presentation contract.
//! Optional authored-hit mode changes fixture HP directly to exercise the production
//! health observer. That is a presentation diagnostic, not a real damage-resolver test.
use super::*;
use crate::game::{ActiveSkill, AttackClock, GameAssets, PlayerBuild, SpawnDirector};

#[derive(Resource)]
struct Probe {
    frame: u32,
    target: Entity,
    base: Vec3,
    speed: f32,
    recovered: bool,
    peaked: bool,
    last_elapsed: f32,
    authored_hits: bool,
    expect_hit_restart: bool,
    max_hit_elapsed: f32,
    authored_samples: u32,
    volley: bool,
    aftermath: bool,
}

/// Debug/native capture only; no shipped gameplay entry point.
pub fn install(app: &mut bevy::app::SubApp, speed: f32) {
    assert!(speed == 1.0 || speed == 8.0);
    let world = app.world_mut();
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<PlayerBuild>().skills.fill(0);
    world.resource_mut::<ActiveSkill>().remaining = 10000.0;
    world.resource_mut::<AttackClock>().0.pause();
    world
        .resource_mut::<Time<Virtual>>()
        .set_relative_speed(speed);
    let scene = world.resource::<GameAssets>().enemy_scenes[4].clone();
    let base = Vec3::splat(1.5);
    let target = world
        .spawn((
            Name::new("Impact rhythm survivor"),
            WorldAssetRoot(scene),
            Transform::from_xyz(1.0, 0.0, 0.0).with_scale(base),
            Enemy {
                hp: 100000.0,
                max_hp: 100000.0,
                speed: 0.0,
                damage: 0.0,
                xp: 0,
                wood: 0,
                gold: 0,
                kill_credit: 0,
                elite: false,
            },
        ))
        .id();
    for mut transform in world
        .query_filtered::<&mut Transform, With<Hero>>()
        .iter_mut(world)
    {
        transform.translation = Vec3::new(-3.0, 0.0, 0.0);
    }
    // This diagnostic deliberately owns a fixed 9m close-up. Responsive
    // production framing must not overwrite it on the next Update.
    let production_cameras: Vec<_> = world
        .query_filtered::<Entity, With<crate::game::BattleCamera>>()
        .iter(world)
        .collect();
    for camera in production_cameras {
        world
            .entity_mut(camera)
            .remove::<crate::game::BattleCamera>();
    }
    let center = Vec3::new(0.0, 0.7, 0.0);
    for (mut transform, mut projection) in world
        .query_filtered::<(&mut Transform, &mut Projection), With<Camera3d>>()
        .iter_mut(world)
    {
        *transform = Transform::from_translation(center + Vec3::new(0.0, 10.0, 10.0))
            .looking_at(center, Vec3::Y);
        if let Projection::Orthographic(projection) = &mut *projection {
            projection.scaling_mode = bevy::camera::ScalingMode::FixedVertical {
                viewport_height: 9.0,
            };
        }
    }
    world.insert_resource(Probe {
        frame: 0,
        target,
        base,
        speed,
        recovered: false,
        peaked: false,
        last_elapsed: 0.0,
        authored_hits: std::env::var_os("ROGUE_CAPTURE_AUTHORED_HITS").is_some(),
        expect_hit_restart: std::env::var_os("ROGUE_CAPTURE_EXPECT_HIT_RESTART").is_some(),
        max_hit_elapsed: 0.0,
        authored_samples: 0,
        volley: std::env::var_os("ROGUE_CAPTURE_SHAKE_VOLLEY").is_some(),
        aftermath: std::env::var_os("ROGUE_CAPTURE_DEATH_AFTERMATH").is_some(),
    });
    app.add_systems(
        Update,
        feed.after(CombatSet)
            .after(SurvivalSet)
            .before(HitFeedbackSet),
    );
    app.add_systems(PostUpdate, observe);
    app.add_systems(Last, observe_authored_hit);
}

fn feed(world: &mut World) {
    if world.resource::<GameSession>().paused {
        return;
    }
    world.resource_mut::<Probe>().frame += 1;
    let p = world.resource::<Probe>();
    let (frame, target, authored_hits, volley, aftermath) =
        (p.frame, p.target, p.authored_hits, p.volley, p.aftermath);
    if authored_hits && (10..=45).contains(&frame) {
        world.get_mut::<Enemy>(target).unwrap().hp -= 1.0;
    }
    // Record a critical, then ordinary contact every render frame for half a second.
    // This remains denser than the 120ms real-time reaction even at 8x.
    let critical = matches!(frame, 10 | 120);
    let ordinary = (10..=45).contains(&frame) || frame == 90;
    let heavy_kill = frame == 150;
    let position = world.get::<Transform>(target).unwrap().translation;
    let mut feedback = world.resource_mut::<HitFeedback>();
    for (active, is_critical, killed, weight) in [
        (critical, true, false, HitWeight::Normal),
        (ordinary, false, false, HitWeight::Normal),
        (heavy_kill, false, true, HitWeight::Elite),
        (aftermath && frame == 70, false, true, HitWeight::Normal),
    ] {
        if active {
            for _ in 0..if volley && is_critical { 100 } else { 1 } {
                feedback.push(HitEvent {
                    direction: Vec3::X,
                    target,
                    position,
                    element: DamageElement::Physical,
                    weight,
                    critical: is_critical,
                    killed,
                });
            }
        }
    }
}

/// Last runs after production model animation and Bevy's actual clip advancement.
fn observe_authored_hit(world: &mut World) {
    let probe = world.resource::<Probe>();
    if !probe.authored_hits || world.resource::<GameSession>().paused {
        return;
    }
    let (frame, target) = (probe.frame, probe.target);
    if (10..=45).contains(&frame) {
        let mut players = world.query::<(Entity, &AnimationPlayer)>();
        let clips: Vec<_> = players
            .iter(world)
            .filter(|(entity, _)| {
                let mut current = Some(*entity);
                while let Some(entity) = current {
                    if entity == target {
                        return true;
                    }
                    current = world.get::<ChildOf>(entity).map(ChildOf::parent);
                }
                false
            })
            .flat_map(|(_, player)| {
                player
                    .playing_animations()
                    // Cast graphs have their root at zero, then Idle/Walk/Attack/Hit.
                    .filter(|(node, _)| node.index() == 4)
                    .map(|(_, clip)| (clip.elapsed(), clip.seek_time()))
            })
            .collect();
        let probe = &mut *world.resource_mut::<Probe>();
        for (elapsed, seek) in clips {
            probe.authored_samples += 1;
            probe.max_hit_elapsed = probe.max_hit_elapsed.max(elapsed);
            println!("[capture] authored-hit frame={frame} elapsed={elapsed:.9} seek={seek:.9}");
        }
    }
    if frame == 60 {
        let probe = world.resource::<Probe>();
        assert!(
            probe.authored_samples > 0,
            "actual target Hit playback required"
        );
        if probe.expect_hit_restart {
            assert!(
                probe.max_hit_elapsed <= 0.04,
                "baseline must reproduce continuous Hit restarts"
            );
            println!(
                "[capture] PASS authored-hit baseline restarts max_elapsed={:.9} samples={}",
                probe.max_hit_elapsed, probe.authored_samples
            );
        } else {
            assert!(
                probe.max_hit_elapsed >= 0.20,
                "authored Hit must progress through recovery under continuous HP loss"
            );
            println!(
                "[capture] PASS authored-hit recovery max_elapsed={:.9} samples={}",
                probe.max_hit_elapsed, probe.authored_samples
            );
        }
    }
}

fn observe(world: &mut World) {
    if world.resource::<GameSession>().paused {
        return;
    }
    let p = world.resource::<Probe>();
    if p.aftermath && matches!(p.frame, 71 | 88 | 151) {
        let frame = p.frame;
        let debris = world
            .query::<&super::aftermath::Debris>()
            .iter(world)
            .count();
        if frame == 71 {
            assert_eq!(debris, 4, "ordinary kill presents four quiet debris pieces");
        } else if frame == 88 {
            assert_eq!(debris, 0, "ordinary death debris dissipates");
        } else {
            assert_eq!(debris, 8, "elite kill has heavier debris");
        }
        println!("[capture] PASS synthetic death aftermath frame={frame} debris={debris}");
    }
    let p = world.resource::<Probe>();
    let (frame, target, base, previous_elapsed) = (p.frame, p.target, p.base, p.last_elapsed);
    if p.volley && matches!(frame, 10 | 12 | 120 | 150 | 152 | 165) {
        let shake = world.resource::<ScreenShake>();
        println!(
            "[capture] volley-shake frame={frame} trauma={:.9} offset={:?}",
            shake.trauma, shake.applied
        );
        if matches!(frame, 10 | 120) {
            assert!(
                shake.trauma <= 0.07,
                "one volley must not stack camera trauma"
            );
            assert!(
                shake.applied.length() > 0.0001,
                "critical impulse must reach the rendered camera"
            );
            println!("[capture] PASS 100 critical contacts use one camera impulse");
        }
    }
    if matches!(
        frame,
        10 | 11 | 12 | 14 | 18 | 46 | 60 | 90 | 92 | 120 | 121 | 122 | 150 | 152 | 165
    ) {
        let expected = p.speed;
        let speed = world.resource::<Time<Virtual>>().relative_speed();
        let stop = world.resource::<HitStop>();
        println!(
            "[capture] synthetic-hit-time frame={frame} requested_speed={expected} actual_speed={speed:.9} real_delta={:.9} remaining={:.9} cooldown={:.9}",
            world.resource::<Time<Real>>().delta_secs(),
            stop.remaining,
            stop.cooldown
        );
        if matches!(frame, 10 | 120) {
            if expected == 1. {
                assert_eq!(
                    speed, HIT_STOP_TIME_SCALE,
                    "surviving critical requests one short stop"
                );
                assert!(stop.remaining > 0. && stop.remaining < 0.04);
            } else {
                assert_eq!(speed, 8., "fast-forward critical must not slow the battle");
            }
        }
        if matches!(frame, 12 | 14 | 60 | 90 | 92 | 122 | 165) {
            assert_eq!(
                speed, expected,
                "short critical stop recovers; ordinary90 never stops"
            );
        }
    }
    let scale = world.get::<Transform>(target).unwrap().scale;
    let punch = world.get::<HitPunch>(target).copied();
    let p = &mut *world.resource_mut::<Probe>();
    if (10..=60).contains(&frame) {
        p.peaked |= scale.y < base.y * 0.95;
        if punch.is_none() && previous_elapsed > 0.0 {
            assert_eq!(scale, base, "production reaction restores authored scale");
            p.recovered = true;
        }
    }
    p.last_elapsed = punch.map_or(0.0, |p| p.elapsed);
    if matches!(
        frame,
        10 | 12 | 14 | 18 | 46 | 60 | 90 | 92 | 120 | 122 | 150 | 152 | 165
    ) {
        println!("[capture] impact frame={frame} scale={scale:?} punch={punch:?}");
    }
    if frame == 60 {
        assert!(
            p.recovered && p.peaked,
            "dense contacts must visibly deform AND finish recovery"
        );
        println!("[capture] PASS dense-hit impact peak and exact production recovery");
    }
    if frame == 90 {
        assert_eq!(punch.expect("fresh ordinary punch").strength, 1.0);
        println!("[capture] PASS ordinary contact does not inherit critical strength");
    }
    if frame == 165 {
        let speed = p.speed;
        assert_eq!(
            world.resource::<Time<Virtual>>().relative_speed(),
            speed,
            "production hit-stop must restore 1x or leave 8x untouched"
        );
        println!("[capture] PASS heavy-kill sequence leaves requested {speed}x battle speed");
    }
}
