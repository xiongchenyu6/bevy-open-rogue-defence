//! Verify the real inventory and previews after actual market acquisitions.
use bevy::{
    input::{
        mouse::{MouseScrollUnit, MouseWheel},
        touch::TouchPhase,
    },
    prelude::*,
    ui::UiSystems,
};
use rogue_commander::game::{
    ActiveSkill, AttackClock, Autoplay, GameSession, HeroStatsSnapshot, PlayerBuild, SelectedHero,
    SpawnDirector, prepare_attribute_growth_market,
};
#[derive(Resource, Default)]
struct EquipmentCapture {
    frame: u32,
    baseline: Option<HeroStatsSnapshot>,
    wallet: u32,
    last_wallet: u32,
    gold: u32,
    initial_preview: String,
}
pub fn configure(world: &mut World) {
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SelectedHero>().locked = true;
    world
        .resource_mut::<super::CaptureControls>()
        .suppress_drafts = true;
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
}
pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<PlayerBuild>().skills.fill(0);
    world.resource_mut::<ActiveSkill>().remaining = 10_000.;
    world.resource_mut::<AttackClock>().0.pause();
    world.resource_mut::<GameSession>().elapsed = 181.;
    world.resource_mut::<GameSession>().kills = 100_000;
    app.init_resource::<EquipmentCapture>().add_systems(
        PreUpdate,
        exercise
            .after(UiSystems::Focus)
            .before(rogue_commander::game::SettingsInputSet),
    );
}
fn press(world: &mut World, target: &str) {
    let mut found = false;
    for (name, mut interaction) in world.query::<(&Name, &mut Interaction)>().iter_mut(world) {
        *interaction = if name.as_str() == target {
            found = true;
            Interaction::Pressed
        } else {
            Interaction::None
        };
    }
    assert!(found, "real purchase/sheet button exists: {target}");
}
fn named_text(world: &mut World, wanted: &str) -> String {
    world
        .query::<(&Name, &Text)>()
        .iter(world)
        .find(|(name, _)| name.as_str() == wanted)
        .unwrap_or_else(|| panic!("actual text exists: {wanted}"))
        .1
        .0
        .clone()
}
fn exists(world: &mut World, wanted: &str) -> bool {
    world
        .query::<&Name>()
        .iter(world)
        .any(|name| name.as_str() == wanted)
}
fn exercise(world: &mut World) {
    world.resource_mut::<EquipmentCapture>().frame += 1;
    let frame = world.resource::<EquipmentCapture>().frame;
    world
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::KeyB);
    world.resource_mut::<ButtonInput<KeyCode>>().clear();
    for mut interaction in world.query::<&mut Interaction>().iter_mut(world) {
        *interaction = Interaction::None;
    }
    match frame {
        20 | 195 => press(world, "Equipment open"),
        35 => {
            assert!(
                world
                    .query::<&Name>()
                    .iter(world)
                    .any(|name| name.as_str() == "Equipment panel"),
                "real baseline open button must create the actual attributes sheet"
            );
            assert!(world.resource::<GameSession>().paused);
            assert!(named_text(world, "Equipment count").contains('0'));
            assert!(!exists(world, "Equipment item 0"));
            let baseline = *world.resource::<HeroStatsSnapshot>();
            let session = world.resource::<GameSession>();
            let (wallet, gold) = (session.kills, session.gold);
            let mut state = world.resource_mut::<EquipmentCapture>();
            state.baseline = Some(baseline);
            state.wallet = wallet;
            state.last_wallet = wallet;
            state.gold = gold;
            println!(
                "[capture] baseline real hero: armor={} evasion={} leech={} physical_pen={} spell_pen={}",
                baseline.armor,
                baseline.evasion,
                baseline.life_leech,
                baseline.physical_penetration,
                baseline.spell_penetration
            );
        }
        45 => press(world, "Equipment close"),
        60 => {
            prepare_attribute_growth_market(world);
            world.resource_mut::<GameSession>().paused = true;
        }
        85 | 105 | 125 | 145 | 165 => {
            let index = (frame - 85) / 20;
            let session = world.resource::<GameSession>();
            let (wallet, gold) = (session.kills, session.gold);
            let mut state = world.resource_mut::<EquipmentCapture>();
            state.last_wallet = wallet;
            state.gold = gold;
            press(world, &format!("Market offer {index}"));
        }
        93 | 113 | 133 | 153 | 173 => {
            let stats = *world.resource::<HeroStatsSnapshot>();
            let state = world.resource::<EquipmentCapture>();
            let base = state.baseline.unwrap();
            let session = world.resource::<GameSession>();
            assert!(
                session.kills < state.last_wallet,
                "actual equipment purchase spends kill currency"
            );
            assert_eq!(
                session.gold, state.gold,
                "market purchase does not spend gold"
            );
            match frame {
                93 => assert!(stats.physical_penetration >= base.physical_penetration + 2.99),
                113 => assert!(stats.spell_penetration > base.spell_penetration),
                133 => assert!(stats.evasion > base.evasion),
                153 => assert!(stats.armor >= base.armor + 5.99),
                173 => assert!(stats.life_leech >= base.life_leech + 0.049),
                _ => unreachable!(),
            }
            let wallet = session.kills;
            world.resource_mut::<EquipmentCapture>().last_wallet = wallet;
            println!(
                "[capture] PASS actual market purchase frame={frame}: remaining kill currency={wallet}; stats={stats:?}"
            );
        }
        180 => world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyB),
        190 => {
            let visible = world
                .query::<(&Name, &Node)>()
                .iter(world)
                .any(|(name, node)| {
                    name.as_str() == "Equipment acquisition" && node.display == Display::Flex
                });
            assert!(
                visible,
                "actual equipment acquisition feedback appears after leaving market"
            );
        }
        210 => {
            assert!(exists(world, "Equipment panel"));
            assert!(world.resource::<GameSession>().paused);
            assert!(named_text(world, "Equipment count").contains('5'));
            for index in 0..5 {
                assert!(
                    exists(world, &format!("Equipment item {index}")),
                    "real acquired item {index} exists"
                );
            }
            assert!(!exists(world, "Equipment item 5"));
            let preview = named_text(world, "Equipment detail name");
            world.resource_mut::<EquipmentCapture>().initial_preview = preview;
            press(world, "Equipment item 4");
        }
        230 => {
            let name = named_text(world, "Equipment detail name");
            let effect = named_text(world, "Equipment detail effect");
            assert!(!name.trim().is_empty() && !effect.trim().is_empty());
            assert_ne!(
                name,
                world.resource::<EquipmentCapture>().initial_preview,
                "selecting a different owned item changes the real preview"
            );
            println!("[capture] actual selected equipment: {name}; {effect}");
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
        260 => {
            assert!(exists(world, "Equipment grid scroll"));
            let stats = *world.resource::<HeroStatsSnapshot>();
            let base = world.resource::<EquipmentCapture>().baseline.unwrap();
            assert!(
                stats.armor > base.armor
                    && stats.evasion > base.evasion
                    && stats.life_leech > base.life_leech
                    && stats.physical_penetration > base.physical_penetration
                    && stats.spell_penetration > base.spell_penetration
            );
            assert!(world.resource::<GameSession>().paused);
        }
        265 => press(world, "Equipment close"),
        270 => {
            assert!(!exists(world, "Equipment panel"));
            assert!(
                !world.resource::<GameSession>().paused,
                "real equipment close resumes combat"
            );
            press(world, "Equipment open");
        }
        280 => world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape),
        290 => {
            assert!(!exists(world, "Equipment panel"));
            assert!(!world.resource::<GameSession>().paused);
            println!(
                "[capture] PASS equipment: empty inventory, five actual market purchases, HUD count five, all owned item cards, selected effect preview, real wheel scrolling, close and Escape resume combat"
            );
        }
        _ => {}
    }
}
