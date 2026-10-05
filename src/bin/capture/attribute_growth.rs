//! Acquire equipment through actual purchase buttons, then compare the live sheet.
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
struct GrowthCapture {
    frame: u32,
    baseline: Option<HeroStatsSnapshot>,
    wallet: u32,
    last_wallet: u32,
    gold: u32,
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
    app.init_resource::<GrowthCapture>().add_systems(
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
fn value(world: &mut World, field: &str) -> f32 {
    let wanted = format!("Hero stat {field}");
    let text = world
        .query::<(&Name, &Text)>()
        .iter(world)
        .find(|(name, _)| name.as_str() == wanted)
        .expect("actual live stat text exists")
        .1
        .0
        .clone();
    text.trim()
        .trim_end_matches('%')
        .replace(',', "")
        .parse()
        .expect("numeric live stat value")
}
fn exercise(world: &mut World) {
    world.resource_mut::<GrowthCapture>().frame += 1;
    let frame = world.resource::<GrowthCapture>().frame;
    world
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::KeyB);
    world.resource_mut::<ButtonInput<KeyCode>>().clear();
    for mut interaction in world.query::<&mut Interaction>().iter_mut(world) {
        *interaction = Interaction::None;
    }
    match frame {
        20 | 195 => press(world, "Hero attributes open"),
        35 => {
            assert!(
                world
                    .query::<&Name>()
                    .iter(world)
                    .any(|name| name.as_str() == "Hero attributes panel"),
                "real baseline open button must create the actual attributes sheet"
            );
            assert!(world.resource::<GameSession>().paused);
            let baseline = *world.resource::<HeroStatsSnapshot>();
            let session = world.resource::<GameSession>();
            let (wallet, gold) = (session.kills, session.gold);
            let mut state = world.resource_mut::<GrowthCapture>();
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
        45 => press(world, "Hero attributes close"),
        60 => {
            prepare_attribute_growth_market(world);
            world.resource_mut::<GameSession>().paused = true;
        }
        85 | 105 | 125 | 145 | 165 => {
            let index = (frame - 85) / 20;
            let session = world.resource::<GameSession>();
            let (wallet, gold) = (session.kills, session.gold);
            let mut state = world.resource_mut::<GrowthCapture>();
            state.last_wallet = wallet;
            state.gold = gold;
            press(world, &format!("Market offer {index}"));
        }
        93 | 113 | 133 | 153 | 173 => {
            let stats = *world.resource::<HeroStatsSnapshot>();
            let state = world.resource::<GrowthCapture>();
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
            world.resource_mut::<GrowthCapture>().last_wallet = wallet;
            println!(
                "[capture] PASS actual market purchase frame={frame}: remaining kill currency={wallet}; stats={stats:?}"
            );
        }
        180 => world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyB),
        230 => {
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
        210 | 250 => {
            let stats = *world.resource::<HeroStatsSnapshot>();
            let base = world.resource::<GrowthCapture>().baseline.unwrap();
            assert!(
                stats.armor > base.armor
                    && stats.evasion > base.evasion
                    && stats.life_leech > base.life_leech
                    && stats.physical_penetration > base.physical_penetration
                    && stats.spell_penetration > base.spell_penetration
            );
            for (field, expected) in [
                ("armor", stats.armor),
                ("evasion", stats.evasion * 100.),
                ("life_leech", stats.life_leech * 100.),
                ("physical_penetration", stats.physical_penetration),
                ("spell_penetration", stats.spell_penetration * 100.),
            ] {
                let shown = value(world, field);
                assert!(
                    (shown - expected).abs() <= 0.11,
                    "{field}: UI={shown} actual={expected}"
                );
            }
            assert!(
                world
                    .query::<&Name>()
                    .iter(world)
                    .any(|name| name.as_str() == "Hero attribute growth sources")
            );
            assert!(
                world.resource::<GameSession>().paused,
                "real sheet pauses combat"
            );
            if frame == 250 {
                let height = world.query::<&Window>().single(world).unwrap().height();
                if height < 500. {
                    assert!(
                        world
                            .query::<&ScrollPosition>()
                            .iter(world)
                            .any(|position| position.0.y > 0.),
                        "actual wheel scroll reveals mobile growth source guidance"
                    );
                }
                let spent = world.resource::<GrowthCapture>().wallet
                    - world.resource::<GameSession>().kills;
                println!(
                    "[capture] PASS attribute-growth: five actual equipment purchases spent {spent} kill currency, armor/evasion/leech/physical and spell penetration increased, UI matches authoritative stats, source hints exist"
                );
            }
        }
        _ => {}
    }
}
