//! Empty decorative art, including genuine pending/failed image I/O fallback.
use super::asset_io_capture::{self, ReadGate};
use bevy::{asset::LoadState, prelude::*, ui::UiSystems};
use rogue_commander::game::{ActiveSkill, AttackClock, GameSession};
use std::{path::PathBuf, sync::Arc};
const IMAGE: &str = "ui/armory/empty-rack.png";
#[derive(Resource)]
struct Missing {
    root: PathBuf,
    gate: Arc<ReadGate>,
}
#[derive(Resource, Default)]
struct Probe(u32);
pub fn prepare(app: &mut App) {
    let (root, gate) = asset_io_capture::prepare(app, &[IMAGE], false);
    app.insert_resource(Missing { root, gate });
}
pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    world.resource_mut::<ActiveSkill>().remaining = 10000.;
    world.resource_mut::<AttackClock>().0.pause();
    world.resource_mut::<GameSession>().elapsed = 10.;
    app.init_resource::<Probe>()
        .add_systems(PreUpdate, drive.after(UiSystems::Focus))
        .add_systems(Last, audit);
}
fn drive(world: &mut World) {
    world.resource_mut::<Probe>().0 += 1;
    let frame = world.resource::<Probe>().0;
    for mut interaction in world.query::<&mut Interaction>().iter_mut(world) {
        *interaction = Interaction::None;
    }
    if frame == 20 {
        let mut found = false;
        for (name, mut interaction) in world.query::<(&Name, &mut Interaction)>().iter_mut(world) {
            if name.as_str() == "Equipment open" {
                *interaction = Interaction::Pressed;
                found = true;
            }
        }
        assert!(found);
    }
    if world.contains_resource::<Missing>() {
        if frame == 40 {
            world.resource::<Missing>().gate.release();
        }
        if frame == 75 {
            asset_io_capture::restore(&world.resource::<Missing>().root, &[IMAGE]);
            world.resource::<AssetServer>().reload(IMAGE);
        }
    }
}
fn audit(world: &mut World) {
    let frame = world.resource::<Probe>().0;
    let missing = world.contains_resource::<Missing>();
    if ![35, 65, 125].contains(&frame) {
        return;
    }
    assert!(world.resource::<GameSession>().paused);
    let (handle, display, size) = world
        .query::<(&Name, &ImageNode, &Node, &ComputedNode)>()
        .iter(world)
        .find(|(name, _, _, _)| name.as_str() == "Equipment empty armory")
        .map(|(_, image, node, computed)| (image.image.clone(), node.display, computed.size()))
        .expect("actual empty armory node");
    assert!(world.query::<&Text>().iter(world).any(|text| {
        text.0.starts_with("No equipment yet.") || text.0.starts_with("暂无装备。")
    }));
    let server = world.resource::<AssetServer>();
    if missing && frame < 75 {
        assert_eq!(
            display,
            Display::None,
            "pending/failed art must never be a white quad"
        );
        if frame == 65 {
            assert!(matches!(
                server.get_load_state(handle.id()),
                Some(LoadState::Failed(_))
            ));
            println!(
                "EQUIPMENT_EMPTY_MISSING_PASS actual image load failed; art hidden; original empty copy visible"
            );
        }
    } else {
        assert!(server.is_loaded_with_dependencies(handle.id()));
        assert_eq!(display, Display::Flex);
        assert!(size.min_element() > 0.);
        assert!(size.x <= 221. && size.y <= 148.);
        println!(
            "EQUIPMENT_EMPTY_ART_PASS frame={frame} actual image ready and visible size={size:?}; empty copy retained"
        );
    }
    if frame == 125 {
        println!(
            "EQUIPMENT_EMPTY_PASS original empty art, no owned items, retained copy and controls; missing={missing}"
        );
    }
}
