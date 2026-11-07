//! Craft real market ingredients through recipe UI, including a complete advanced chain.
use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    input::touch::TouchPhase,
    prelude::*,
    ui::UiSystems,
};
use rogue_commander::game::{
    ActiveSkill, AttackClock, Autoplay, CraftingState, GameSession, HeroStatsSnapshot, PlayerBuild,
    RecipeId, SelectedHero, SpawnDirector, prepare_crafting_market,
};
#[derive(Resource)]
struct CraftCapture {
    frame: u32,
    phase: u8,
    wallet: u32,
    gold: u32,
    before: Option<HeroStatsSnapshot>,
}
pub fn configure(world: &mut World, scenario: &str) {
    let phase = scenario
        .strip_prefix("crafting-")
        .unwrap()
        .parse::<u8>()
        .unwrap();
    assert!(phase < 2);
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SelectedHero>().locked = true;
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
    world
        .resource_mut::<super::CaptureControls>()
        .suppress_drafts = true;
    world.insert_resource(CraftCapture {
        frame: 0,
        phase,
        wallet: 0,
        gold: 0,
        before: None,
    });
}
pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<PlayerBuild>().skills.fill(0);
    world.resource_mut::<ActiveSkill>().remaining = 10000.;
    world.resource_mut::<AttackClock>().0.pause();
    world.resource_mut::<GameSession>().elapsed = 181.;
    world.resource_mut::<GameSession>().kills = 100000;
    app.add_systems(
        PreUpdate,
        exercise
            .after(UiSystems::Focus)
            .before(rogue_commander::game::SettingsInputSet),
    );
}
fn press(world: &mut World, wanted: &str) {
    let mut found = false;
    for (name, mut interaction) in world.query::<(&Name, &mut Interaction)>().iter_mut(world) {
        *interaction = if name.as_str() == wanted {
            found = true;
            Interaction::Pressed
        } else {
            Interaction::None
        };
    }
    assert!(found, "actual crafting control exists: {wanted}");
}
fn text(world: &mut World, wanted: &str) -> String {
    world
        .query::<(&Name, &Text)>()
        .iter(world)
        .find(|(name, _)| name.as_str() == wanted)
        .unwrap_or_else(|| panic!("actual crafting text exists: {wanted}"))
        .1
        .0
        .clone()
}
fn material_status(world: &mut World) -> String {
    let visible: Vec<_> = world
        .query::<(&Name, &Node)>()
        .iter(world)
        .filter(|(_, node)| node.display != Display::None)
        .filter_map(|(name, _)| {
            name.as_str()
                .strip_prefix("Crafting material ")?
                .parse::<usize>()
                .ok()
        })
        .collect();
    world
        .query::<(&Name, &Text)>()
        .iter(world)
        .filter(|(name, _)| {
            name.as_str()
                .strip_prefix("Crafting material status ")
                .and_then(|index| index.parse::<usize>().ok())
                .is_some_and(|index| visible.contains(&index))
        })
        .map(|(_, text)| text.0.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}
fn audit_material_art(world: &mut World, expected: usize) {
    let detail_height = world
        .query::<(&Name, &ComputedNode)>()
        .iter(world)
        .find(|(name, _)| name.as_str() == "Equipment detail scroll")
        .unwrap()
        .1
        .size()
        .y;
    assert!(
        detail_height >= 40.0,
        "forge details remain readable at every viewport: {detail_height}"
    );
    assert!(
        world
            .query::<(&Name, &Node)>()
            .iter(world)
            .any(|(name, node)| name.as_str() == "Settings open" && node.display == Display::None),
        "settings launcher must not cover forge recipes"
    );
    let roots: Vec<_> = world
        .query::<(&Name, &Node, &ComputedNode, &UiGlobalTransform)>()
        .iter(world)
        .filter(|(name, node, _, _)| {
            name.as_str()
                .strip_prefix("Crafting material ")
                .is_some_and(|index| index.parse::<usize>().is_ok())
                && node.display != Display::None
        })
        .map(|(name, _, node, transform)| {
            (name.as_str().to_owned(), node.size(), transform.translation)
        })
        .collect();
    assert_eq!(
        roots.len(),
        expected,
        "selected recipe shows precisely its ingredients"
    );
    for index in 0..expected {
        let (_, size, center) = roots
            .iter()
            .find(|(name, _, _)| *name == format!("Crafting material {index}"))
            .unwrap();
        assert!(size.x > 40. && size.y > 40.);
        let (image, art, transform) = world
            .query::<(&Name, &ImageNode, &ComputedNode, &UiGlobalTransform)>()
            .iter(world)
            .find(|(name, _, _, _)| name.as_str() == format!("Crafting material art {index}"))
            .map(|(_, image, node, transform)| (image.clone(), *node, *transform))
            .unwrap();
        assert!(image.rect.is_some(), "material uses an authored atlas crop");
        assert!(
            world
                .resource::<AssetServer>()
                .is_loaded_with_dependencies(image.image.id())
        );
        assert!(
            ((transform.translation - *center).abs() + art.size() * 0.5)
                .cmple(*size * 0.5 + Vec2::ONE)
                .all(),
            "material portrait remains inside its slot"
        );
    }
    println!("[capture] MATERIAL_ART count={expected} atlas_loaded=true bounded=true");
}
fn exercise(world: &mut World) {
    world.resource_mut::<CraftCapture>().frame += 1;
    let state = world.resource::<CraftCapture>();
    let (frame, phase) = (state.frame, state.phase);
    let (first, second, advanced) = if phase == 0 {
        (
            RecipeId::Bloodreaver,
            RecipeId::StormBastion,
            RecipeId::NightCrown,
        )
    } else {
        (
            RecipeId::AstralPrism,
            RecipeId::Suncoil,
            RecipeId::WorldEngine,
        )
    };
    world.resource_mut::<ButtonInput<KeyCode>>().clear();
    world
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::KeyB);
    for mut interaction in world.query::<&mut Interaction>().iter_mut(world) {
        *interaction = Interaction::None;
    }
    match frame {
        20 | 195 => press(world, "Equipment open"),
        25 | 205 => press(world, "Equipment crafting tab"),
        30 | 210 => press(world, &format!("Crafting recipe {}", first.index())),
        35 => {
            assert!(
                material_status(world).contains('○'),
                "missing materials actually shown"
            );
            assert!(!world.resource::<CraftingState>().acquired(first));
        }
        40 => press(world, "Crafting confirm"),
        45 => assert!(
            !world.resource::<CraftingState>().acquired(first),
            "missing-material click cannot craft"
        ),
        50 => press(world, "Equipment close"),
        60 => {
            prepare_crafting_market(world, phase);
            world.resource_mut::<GameSession>().paused = true;
        }
        80 | 100 | 120 | 140 | 160 => {
            let session = world.resource::<GameSession>();
            let (wallet, gold) = (session.kills, session.gold);
            let mut state = world.resource_mut::<CraftCapture>();
            state.wallet = wallet;
            state.gold = gold;
            press(world, &format!("Market offer {}", (frame - 80) / 20));
        }
        90 | 110 | 130 | 150 | 170 => {
            let session = world.resource::<GameSession>();
            let state = world.resource::<CraftCapture>();
            assert!(
                session.kills < state.wallet,
                "genuine ingredient purchase charges kill currency"
            );
            assert_eq!(session.gold, state.gold);
        }
        180 => world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyB),
        200 => assert_eq!(text(world, "Equipment count"), "5"),
        215 | 237 | 257 => {
            audit_material_art(world, if frame == 257 { 3 } else { 2 });
            let status = material_status(world);
            assert!(
                status.contains('✓') && !status.contains('○'),
                "all genuine ingredient rows ready: {status}"
            );
            let before = *world.resource::<HeroStatsSnapshot>();
            world.resource_mut::<CraftCapture>().before = Some(before);
        }
        220 | 240 | 260 => press(world, "Crafting confirm"),
        230 | 250 | 270 => {
            let recipe = match frame {
                230 => first,
                250 => second,
                _ => advanced,
            };
            let crafting = world.resource::<CraftingState>();
            assert!(crafting.owns(recipe) && crafting.acquired(recipe));
            let stats = *world.resource::<HeroStatsSnapshot>();
            let before = world.resource::<CraftCapture>().before.unwrap();
            match recipe {
                RecipeId::Bloodreaver => {
                    assert!(stats.physical_penetration >= before.physical_penetration + 2.99);
                    assert!(stats.life_leech >= before.life_leech + 0.049);
                }
                RecipeId::AstralPrism => {
                    assert!(stats.spell_penetration >= before.spell_penetration + 0.099);
                    assert!(stats.spell_power > before.spell_power);
                }
                RecipeId::StormBastion => {
                    assert!(stats.armor >= before.armor + 3.99);
                    assert!(stats.evasion >= before.evasion + 0.049);
                }
                RecipeId::Suncoil => {
                    assert!(stats.crit_chance >= before.crit_chance + 0.049);
                    assert!(stats.spell_power > before.spell_power);
                }
                RecipeId::NightCrown => {
                    assert!(stats.physical_penetration >= before.physical_penetration + 3.99);
                    assert!(stats.life_leech >= before.life_leech + 0.049);
                    assert!(stats.armor >= before.armor + 3.99);
                }
                RecipeId::WorldEngine => {
                    assert!(stats.spell_penetration >= before.spell_penetration + 0.099);
                    assert!(stats.spell_echo_chance >= before.spell_echo_chance + 0.099);
                }
            }
            let count = text(world, "Equipment count");
            assert_eq!(
                count,
                match frame {
                    230 => "4",
                    250 => "3",
                    _ => "1",
                }
            );
            if frame == 270 {
                let crafting = world.resource::<CraftingState>();
                assert!(!crafting.owns(first) && !crafting.owns(second));
            }
            println!(
                "[capture] actual recipe {recipe:?}: inventory={count}; authoritative effects={stats:?}"
            );
        }
        235 => press(world, &format!("Crafting recipe {}", second.index())),
        251 | 255 => press(world, &format!("Crafting recipe {}", advanced.index())),
        253 => press(world, &format!("Crafting recipe {}", first.index())),
        254 => {
            audit_material_art(world, 2);
            let status = material_status(world);
            assert_eq!(
                status.matches('✓').count() + status.matches('○').count(),
                2,
                "hidden third ingredient must not enter the visible status audit"
            );
        }
        275 => press(world, "Crafting confirm"),
        271 => {
            for (name, mut interaction) in
                world.query::<(&Name, &mut Interaction)>().iter_mut(world)
            {
                if name.as_str() == "Equipment detail scroll" {
                    *interaction = Interaction::Hovered;
                }
            }
            let window = world
                .query_filtered::<Entity, With<Window>>()
                .single(world)
                .unwrap();
            world
                .resource_mut::<Messages<MouseWheel>>()
                .write(MouseWheel {
                    unit: MouseScrollUnit::Line,
                    x: 0.,
                    y: -100.,
                    window,
                    phase: TouchPhase::Moved,
                });
        }
        274 => {
            if world.query::<&Window>().single(world).unwrap().height() < 500. {
                assert!(
                    world.query::<(&Name, &ScrollPosition)>().iter(world).any(
                        |(name, position)| name.as_str() == "Equipment detail scroll"
                            && position.0.y > 0.
                    ),
                    "real mobile detail wheel reveals inherited and extra bonuses"
                );
            }
        }
        280 => {
            assert_eq!(
                text(world, "Equipment count"),
                "1",
                "already-crafted confirm cannot duplicate output"
            );
            assert!(world.resource::<CraftingState>().owns(advanced));
            press(world, "Equipment inventory tab");
        }
        283 => {
            let names: Vec<_> = world
                .query::<&Name>()
                .iter(world)
                .map(|name| name.as_str().to_owned())
                .collect();
            assert!(names.iter().any(|name| name == "Equipment crafted item 0"));
            assert!(
                !names
                    .iter()
                    .any(|name| name == "Equipment item 0" || name == "Equipment crafted item 1"),
                "consumed materials replaced by exactly one actual advanced inventory output"
            );
        }
        285 => press(world, "Equipment close"),
        295 => {
            assert!(!world.resource::<GameSession>().paused);
            println!(
                "[capture] PASS crafting-{phase}: five real ingredient purchases, missing materials blocked, three genuine recipes, ingredients consumed, advanced output retained, extra combat bonuses applied, duplicate prevented, close resumes"
            );
        }
        _ => {}
    }
}
