//! Same hero and level before/after a genuine market purchase; no fake visual events.
use bevy::{prelude::*, ui::UiSystems};
use rogue_commander::game::{
    Autoplay, GameSession, HeroKind, HeroVisual, HeroVitals, SelectedHero, SpawnDirector,
    prepare_crafting_market,
};
#[derive(Resource, Default)]
struct Probe {
    frame: u32,
    initial: Vec<(Entity, Handle<StandardMaterial>, bool)>,
    wallet: u32,
}

pub fn configure(world: &mut World, scenario: &str) {
    let kind = match scenario {
        "equipment-growth-archer" => HeroKind::EmberRanger,
        "equipment-growth-mage" => HeroKind::StormSavant,
        "equipment-growth-sword" => HeroKind::StoneWarden,
        _ => panic!("equipment scenario"),
    };
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SelectedHero>().kind = kind;
    world.resource_mut::<SelectedHero>().locked = true;
    *world.resource_mut::<HeroVitals>() = HeroVitals::for_hero(kind, 1);
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
    world
        .resource_mut::<super::CaptureControls>()
        .suppress_drafts = true;
}
pub fn install(app: &mut bevy::app::SubApp) {
    super::attack_capture::install(app, "attack-equipment");
    let cameras: Vec<_> = app
        .world_mut()
        .query_filtered::<Entity, With<rogue_commander::game::BattleCamera>>()
        .iter(app.world())
        .collect();
    for entity in cameras {
        app.world_mut()
            .entity_mut(entity)
            .remove::<rogue_commander::game::BattleCamera>();
    }
    app.init_resource::<Probe>().add_systems(
        PreUpdate,
        exercise
            .after(UiSystems::Focus)
            .before(rogue_commander::game::SettingsInputSet),
    );
}
fn meshes(world: &mut World) -> Vec<(Entity, Handle<StandardMaterial>, bool)> {
    let candidates: Vec<_> = world
        .query::<(Entity, &MeshMaterial3d<StandardMaterial>)>()
        .iter(world)
        .map(|(e, m)| (e, m.0.clone()))
        .collect();
    candidates
        .into_iter()
        .filter_map(|(entity, handle)| {
            let mut cursor = Some(entity);
            let mut weapon = false;
            for _ in 0..32 {
                let e = cursor?;
                weapon |= world.get::<Name>(e).is_some_and(|n| {
                    n.as_str().starts_with("1H_") || n.as_str().starts_with("2H_")
                });
                if world.get::<HeroVisual>(e).is_some() {
                    return Some((entity, handle, weapon));
                }
                cursor = world.get::<ChildOf>(e).map(ChildOf::parent);
            }
            None
        })
        .collect()
}
fn press(world: &mut World, name: &str) {
    let entity = world
        .query::<(Entity, &Name)>()
        .iter(world)
        .find(|(_, n)| n.as_str() == name)
        .expect("market control")
        .0;
    *world.get_mut::<Interaction>(entity).unwrap() = Interaction::Pressed;
}
fn exercise(world: &mut World) {
    world.resource_mut::<Probe>().frame += 1;
    let frame = world.resource::<Probe>().frame;
    world.resource_mut::<ButtonInput<KeyCode>>().clear();
    for mut i in world.query::<&mut Interaction>().iter_mut(world) {
        *i = Interaction::None;
    }
    match frame {
        80 => {
            let entries = meshes(world);
            assert!(entries.iter().any(|e| e.2));
            world.resource_mut::<Probe>().initial = entries;
        }
        100 => {
            let mut s = world.resource_mut::<GameSession>();
            s.elapsed = 181.;
            s.kills = 1000;
            s.paused = true;
        }
        101 => {
            world.insert_resource(rogue_commander::game::ControlScheme::Touch);
            prepare_crafting_market(world, 1);
        }
        110 => press(world, "Market offer 0"),
        120 => {
            let wallet = world.resource::<GameSession>().kills;
            world.resource_mut::<Probe>().wallet = wallet;
            press(world, "Market buy selected");
        }
        130 => {
            assert!(world.resource::<GameSession>().kills < world.resource::<Probe>().wallet);
            world
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::KeyB);
        }
        180 => {
            let current = meshes(world);
            let before = &world.resource::<Probe>().initial;
            let changed_weapon = current
                .iter()
                .filter(|(e, h, weapon)| {
                    *weapon
                        && before
                            .iter()
                            .any(|(old, handle, _)| old == e && handle != h)
                })
                .count();
            assert!(
                changed_weapon > 0,
                "purchase must visibly change held weapon materials"
            );
            for (entity, handle, weapon) in &current {
                if !weapon
                    && let Some((_, original, _)) = before.iter().find(|(e, _, _)| e == entity)
                {
                    assert_eq!(handle, original, "equipment must not recolor the body");
                }
            }
            assert_eq!(world.resource::<GameSession>().level, 1);
            assert_eq!(world.resource::<HeroVitals>().level, 1);
            println!(
                "[capture] PASS equipment growth: actual paid ArcLens, same level 1, {changed_weapon} held weapon primitives changed, body untouched"
            );
        }
        _ => {}
    }
}
