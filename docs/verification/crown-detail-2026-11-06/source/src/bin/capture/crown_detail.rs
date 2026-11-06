//! Controlled prerequisite rank, real market purchase and real G-draft text.
//! This is UI reachability evidence, not naturally earned equipment progression.
use bevy::{prelude::*, ui::UiSystems};
use rogue_commander::game::{
    ActiveSkill, AttackClock, Autoplay, ControlScheme, DraftChoice, DraftState, GameLanguage,
    GameSession, PlayerBuild, SelectedHero, SelectionDetail, SkillId, SpawnDirector,
    prepare_crafting_market,
};

const SKILL: usize = 26;

#[derive(Resource)]
struct Probe {
    frame: u32,
    expected_fixed: bool,
    wallet: u32,
    gold: u32,
    price: u32,
    purchased: bool,
}

pub fn configure(world: &mut World) {
    let expected_fixed = match std::env::var("EXPECT_CROWN_DETAIL_FIXED")
        .expect("crown-detail requires EXPECT_CROWN_DETAIL_FIXED=0 or 1")
        .as_str()
    {
        "0" => false,
        "1" => true,
        _ => panic!("EXPECT_CROWN_DETAIL_FIXED must be 0 or 1"),
    };
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SelectedHero>().locked = true;
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
    // The actual G draw must survive the capture harness's draft suppression.
    world
        .resource_mut::<super::CaptureControls>()
        .suppress_drafts = false;
    world.insert_resource(Probe {
        frame: 0,
        expected_fixed,
        wallet: 0,
        gold: 0,
        price: 0,
        purchased: false,
    });
}

pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<AttackClock>().0.pause();
    world.resource_mut::<ActiveSkill>().remaining = 10_000.0;
    *world.resource_mut::<ControlScheme>() = ControlScheme::Keyboard;
    world.resource_mut::<SelectionDetail>().0 = true;
    let mut build = world.resource_mut::<PlayerBuild>();
    build.skills.fill(0);
    build.skills[SKILL] = 5;
    let mut session = world.resource_mut::<GameSession>();
    session.elapsed = 181.0;
    session.gold = 50_000;
    session.kills = 50_000;
    session.paused = true;
    app.add_systems(
        PreUpdate,
        exercise
            .after(UiSystems::Focus)
            .before(rogue_commander::game::SettingsInputSet),
    );
}

fn interact(world: &mut World, wanted: &str, value: Interaction) {
    let matches: Vec<_> = world
        .query::<(Entity, &Name, &Interaction)>()
        .iter(world)
        .filter(|(_, name, _)| name.as_str() == wanted)
        .map(|(entity, _, _)| entity)
        .collect();
    assert_eq!(matches.len(), 1, "one actual control: {wanted}");
    *world.get_mut::<Interaction>(matches[0]).unwrap() = value;
}

fn text(world: &mut World, wanted: &str) -> String {
    let matches: Vec<_> = world
        .query::<(&Name, &Text)>()
        .iter(world)
        .filter(|(name, _)| name.as_str() == wanted)
        .map(|(_, text)| text.0.clone())
        .collect();
    assert_eq!(matches.len(), 1, "one actual text: {wanted}");
    matches[0].clone()
}

fn assert_branches(world: &World) {
    let draft = world.resource::<DraftState>();
    assert!(draft.root.is_some(), "real G draw must create selection UI");
    assert_eq!(draft.choices.len(), 3);
    let paths: Vec<_> = draft
        .choices
        .iter()
        .map(|choice| match choice {
            DraftChoice::EvolveUr { skill, path } => {
                assert_eq!(*skill, SkillId(SKILL as u8));
                path.index()
            }
            _ => panic!("rank6 must reach real UR evolution choices: {choice:?}"),
        })
        .collect();
    assert_eq!(paths, [0, 1, 2], "three actual distinct evolution paths");
    assert_eq!(world.resource::<PlayerBuild>().skills[SKILL], 6);
}

fn audit_detail(world: &mut World, frame: u32) {
    assert_branches(world);
    assert!(world.resource::<Probe>().purchased);
    let fixed = world.resource::<Probe>().expected_fixed;
    let language = *world.resource::<GameLanguage>();
    let detail = text(world, "Selection details");
    let wanted = if fixed { "6 → 6/6" } else { "6 → 5/5" };
    let rejected = if fixed { "6 → 5/5" } else { "6 → 6/6" };
    assert!(
        detail.contains(wanted),
        "actual detail must contain {wanted}: {detail}"
    );
    assert!(!detail.contains(rejected));
    assert!(detail.contains(language.pick("Base skill", "基础技能")));
    let damage = detail
        .lines()
        .find(|line| line.starts_with("Damage (") || line.starts_with("伤害 ("))
        .expect("actual summon damage comparison line");
    let (_, values) = damage.split_once(':').expect("damage label separator");
    let (current, next) = values.trim().split_once(" → ").expect("damage comparison");
    let current: f32 = current.parse().expect("numeric displayed current damage");
    let next: f32 = next.parse().expect("numeric displayed next damage");
    assert!(current.is_finite() && next.is_finite() && current > 0.0 && next > 0.0);
    if fixed {
        assert_eq!(
            current, next,
            "rank6 base damage must not preview a downgrade"
        );
    } else {
        assert!(
            current > next,
            "baseline must expose real rank6-to5 damage downgrade"
        );
    }
    println!("CROWN_DETAIL_TEXT frame={frame} language={language:?} fixed={fixed} text={detail:?}");
    println!(
        "CROWN_DETAIL_DAMAGE frame={frame} current={current} next={next} actual_rank=6 branches=3"
    );
    if frame == 180 {
        println!(
            "[capture] PASS crown-detail: actual Crown purchase, exact kill debit, rank5-to6, real G UR branches and actual localized detail; fixed={fixed}"
        );
    }
}

fn exercise(world: &mut World) {
    world.resource_mut::<Probe>().frame += 1;
    let frame = world.resource::<Probe>().frame;
    {
        let mut keyboard = world.resource_mut::<ButtonInput<KeyCode>>();
        keyboard.release(KeyCode::KeyG);
        keyboard.clear();
    }
    for mut interaction in world.query::<&mut Interaction>().iter_mut(world) {
        *interaction = Interaction::None;
    }
    match frame {
        20 => prepare_crafting_market(world, 0),
        40 => {
            assert_eq!(world.resource::<PlayerBuild>().skills[SKILL], 5);
            let price: u32 = text(world, "Market price 4")
                .parse()
                .expect("actual Crown offer price");
            assert!(price > 0);
            let session = world.resource::<GameSession>();
            let (wallet, gold) = (session.kills, session.gold);
            let mut probe = world.resource_mut::<Probe>();
            probe.wallet = wallet;
            probe.gold = gold;
            probe.price = price;
            interact(world, "Market offer 4", Interaction::Pressed);
        }
        50 => {
            let session = world.resource::<GameSession>();
            let probe = world.resource::<Probe>();
            assert_eq!(
                session.kills,
                probe.wallet - probe.price,
                "real purchase exact kill debit"
            );
            assert_eq!(
                session.gold, probe.gold,
                "market must not charge skill gold"
            );
            assert_eq!(world.resource::<PlayerBuild>().skills[SKILL], 6);
            let language = *world.resource::<GameLanguage>();
            let feedback = text(world, "Market feedback");
            assert!(
                feedback.contains(language.pick("Bought Ascendant Crown", "已购买 登临之冠")),
                "actual Crown purchase feedback: {feedback}"
            );
            world.resource_mut::<Probe>().purchased = true;
            println!("CROWN_DETAIL_PURCHASE frame={frame} actual_rank=6 feedback={feedback:?}");
        }
        60 => interact(world, "Market close", Interaction::Pressed),
        70 => {
            assert!(
                !world.resource::<GameSession>().paused,
                "real market close resumes battle"
            );
            world
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::KeyG);
        }
        80 => assert_branches(world),
        100 | 180 => audit_detail(world, frame),
        _ => {}
    }
    if frame >= 80 {
        interact(world, "Build card 0", Interaction::Hovered);
    }
}
