//! Controlled transitional-state proof using both production countdown presenters.
use bevy::prelude::*;
use rogue_commander::game::{CaptureTarget, GameLanguage, GameSession, SpawnDirector};
#[derive(Resource, Default)]
struct Probe {
    frame: u32,
    paused_remaining: f32,
    restored_frame: Option<u32>,
}
pub fn install(app: &mut bevy::app::SubApp) {
    app.init_resource::<Probe>()
        .add_systems(First, |mut p: ResMut<Probe>| p.frame += 1)
        .add_systems(PostUpdate, inject.before(bevy::camera::CameraUpdateSystems))
        .add_systems(Last, audit);
}
fn inject(mut p: ResMut<Probe>, mut s: ResMut<GameSession>, mut director: ResMut<SpawnDirector>) {
    director.enabled = false;
    match p.frame {
        10 => {
            s.core_hp = 0.;
            s.gate_revive_remaining = 0.;
            s.paused = true;
            println!(
                "COUNTDOWN_FIXTURE injected corehp0/remaining0 transitional state after gameplay update"
            );
        }
        30 => s.gate_revive_remaining = 3.0,
        40 | 80 => s.paused = false,
        60 => {
            assert!(s.gate_revive_remaining > 0. && s.gate_revive_remaining < 3.0);
            s.paused = true;
            p.paused_remaining = s.gate_revive_remaining;
        }
        _ => {}
    }
}
fn audit(world: &mut World) {
    let frame = world.resource::<Probe>().frame;
    let restored_now = frame > 80
        && world.resource::<GameSession>().core_hp > 0.
        && world.resource::<Probe>().restored_frame.is_none();
    if frame == 180 {
        assert!(
            world.resource::<Probe>().restored_frame.is_some() || restored_now,
            "normal countdown must have restored by frame180"
        );
    }
    if restored_now {
        world.resource_mut::<Probe>().restored_frame = Some(frame);
    }
    if ![20, 35, 70].contains(&frame) && !restored_now {
        return;
    }
    let s = world.resource::<GameSession>();
    let language = *world.resource::<GameLanguage>();
    let down = s.core_hp <= 0.;
    if frame == 70 {
        assert_eq!(
            s.gate_revive_remaining,
            world.resource::<Probe>().paused_remaining
        );
    }
    if restored_now {
        assert!(
            !down && s.gate_revive_remaining == 0. && !s.paused,
            "normal gameplay countdown restores gate"
        );
    }
    let labels: Vec<_> = world
        .query::<(&Text, Option<&Name>)>()
        .iter(world)
        .map(|(t, n)| (t.0.clone(), n.map(|n| n.as_str().to_owned())))
        .collect();
    let digit = &labels
        .iter()
        .find(|(_, n)| n.as_deref() == Some("Gate rebuild countdown digit"))
        .unwrap()
        .0;
    let hud = labels
        .iter()
        .find(|(t, _)| {
            t.starts_with(language.pick("GATE DOWN", "城门重建"))
                || t.starts_with(language.pick("GATE RESTORED", "城门已重建"))
        })
        .expect("production gate HUD state");
    if down {
        assert_eq!(
            hud.0,
            format!(
                "{} {digit}{}",
                language.pick("GATE DOWN", "城门重建"),
                language.pick("s", "秒")
            )
        );
    }
    let badge = world
        .query::<(&Name, &Node)>()
        .iter(world)
        .find(|(n, _)| n.as_str() == "Gate rebuild countdown")
        .unwrap()
        .1
        .display;
    assert_eq!(badge, if down { Display::Flex } else { Display::None });
    if frame == 20 {
        assert_eq!(digit, "4");
    }
    let label = match frame {
        20 => "transition",
        35 => "progress",
        70 => "paused",
        _ => "restored",
    };
    println!(
        "COUNTDOWN_LABELS_CHECK {label} PASS hud={} central={digit}",
        hud.0
    );
    let target = world.resource::<CaptureTarget>().0.clone();
    let path =
        std::path::PathBuf::from(std::env::args().nth(2).unwrap()).join(format!("{label}.png"));
    super::queue_screenshot(world, target, path);
    if restored_now {
        println!("COUNTDOWN_LABELS_PASS transition/progress/pause/resume/restore");
    }
}
