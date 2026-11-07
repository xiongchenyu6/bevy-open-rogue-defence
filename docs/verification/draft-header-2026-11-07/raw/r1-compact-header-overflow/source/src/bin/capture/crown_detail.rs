//! Controlled prerequisite rank, real market purchase and real G-draft text.
//! This is UI reachability evidence, not naturally earned equipment progression.
use bevy::{prelude::*, ui::UiSystems};
use rogue_commander::game::{
    ActiveSkill, AttackClock, Autoplay, ControlScheme, DraftChoice, DraftState, Enemy, GameAssets,
    GameLanguage, GameSession, Hero, HeroKind, PlayerBuild, PlayerSummon, SelectedHero,
    SelectionDetail, SkillId, SpawnDirector, prepare_crafting_market,
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
    damage_case: bool,
    expect_modifiers: bool,
    raw_preview: Option<f32>,
    adjusted_preview: Option<f32>,
    target: Option<Entity>,
    target_position: Vec3,
    previous_hp: f32,
    saw_imported_wyrm: bool,
    hp_intervals: u32,
    matches_display: bool,
}

pub fn configure(world: &mut World, scenario: &str) {
    let damage_case = scenario == "crown-damage-preview";
    let expect_modifiers = if damage_case {
        match std::env::var("EXPECT_SKILL_DAMAGE_MODIFIERS")
            .expect("crown-damage-preview requires EXPECT_SKILL_DAMAGE_MODIFIERS=0 or 1")
            .as_str()
        {
            "0" => false,
            "1" => true,
            _ => panic!("EXPECT_SKILL_DAMAGE_MODIFIERS must be 0 or 1"),
        }
    } else {
        false
    };
    if damage_case {
        world.resource_mut::<SelectedHero>().kind = HeroKind::StormSavant;
    }
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
        damage_case,
        expect_modifiers,
        raw_preview: None,
        adjusted_preview: None,
        target: None,
        target_position: Vec3::ZERO,
        previous_hp: 50_000.0,
        saw_imported_wyrm: false,
        hp_intervals: 0,
        matches_display: false,
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
    if std::env::var_os("ROGUE_CAPTURE_DRAFT_LAYOUT").is_some() {
        super::draft_layout::audit(world, frame);
    }
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
    if world.resource::<Probe>().damage_case {
        let expect_modifiers = world.resource::<Probe>().expect_modifiers;
        assert_eq!(
            format!("{current:.1}"),
            "650.8",
            "controlled StormSavant/rank6 base"
        );
        let adjusted = detail.lines().find(|line| {
            line.starts_with("With current bonuses (non-critical):")
                || line.starts_with("计入当前增伤（非暴击）:")
        });
        let displayed = if expect_modifiers {
            let line = adjusted.expect("actual adjusted damage UI line");
            let (_, values) = line.split_once(':').unwrap();
            let (a, b) = values.trim().split_once(" → ").unwrap();
            let a: f32 = a.parse().expect("adjusted current damage");
            let b: f32 = b.parse().expect("adjusted next damage");
            assert!(a.is_finite() && b.is_finite());
            assert_eq!(a, b);
            assert_eq!(
                format!("{a:.1}"),
                "846.0",
                "controlled Crown-only outgoing display"
            );
            Some(a)
        } else {
            assert!(
                adjusted.is_none(),
                "old baseline must not invent an adjusted line"
            );
            None
        };
        let mut probe = world.resource_mut::<Probe>();
        probe.raw_preview = Some(current);
        probe.adjusted_preview = displayed;
        println!(
            "CROWN_DAMAGE_CARD frame={frame} base_display={current} adjusted_display={displayed:?} modifiers_expected={expect_modifiers}"
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
    let damage_case = world.resource::<Probe>().damage_case;
    if frame >= 80 && (!damage_case || frame < 190) {
        interact(world, "Build card 0", Interaction::Hovered);
    }
    if damage_case {
        match frame {
            190 => interact(world, "Build later", Interaction::Pressed),
            195 => {
                assert!(world.resource::<DraftState>().root.is_none());
                assert!(!world.resource::<GameSession>().paused);
            }
            200 => spawn_damage_target(world),
            _ => {}
        }
        if frame > 200 {
            observe_damage(world, frame);
        }
    }
}

fn spawn_damage_target(world: &mut World) {
    assert!(world.resource::<DraftState>().root.is_none());
    assert!(!world.resource::<GameSession>().paused);
    let position = world
        .query_filtered::<&Transform, With<Hero>>()
        .single(world)
        .unwrap()
        .translation
        + Vec3::new(5.5, 0.0, 1.0);
    let scene = world.resource::<GameAssets>().enemy_scenes[0].clone();
    let target = world
        .spawn((
            Name::new("Controlled Crown frost damage target"),
            WorldAssetRoot(scene),
            Transform::from_translation(position).with_scale(Vec3::splat(0.8)),
            Enemy {
                hp: 50_000.0,
                max_hp: 50_000.0,
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
    let mut probe = world.resource_mut::<Probe>();
    assert!(
        probe.target.is_none(),
        "only one stationary target is created"
    );
    probe.target = Some(target);
    probe.target_position = position;
    println!(
        "CROWN_DAMAGE_TARGET frame=200 actor={target:?} position={position:?} initial_hp=50000 armor_component=false"
    );
}

fn imported_mesh(world: &World, root: Entity) -> bool {
    let mut pending = vec![root];
    while let Some(entity) = pending.pop() {
        if world.get::<Mesh3d>(entity).is_some() {
            return true;
        }
        if let Some(children) = world.get::<Children>(entity) {
            pending.extend(children.iter());
        }
    }
    false
}

fn observe_damage(world: &mut World, frame: u32) {
    assert!(!world.resource::<GameSession>().paused);
    assert!(
        world.resource::<DraftState>().root.is_none(),
        "Later must keep the pending draft closed"
    );
    assert!(
        world.resource::<AttackClock>().0.is_paused(),
        "ordinary attacks remain disabled"
    );
    assert!(world.resource::<ActiveSkill>().remaining > 9_000.0);
    assert!(
        world
            .resource::<PlayerBuild>()
            .skills
            .iter()
            .enumerate()
            .all(|(index, rank)| { *rank == if index == SKILL { 6 } else { 0 } })
    );
    let target = world
        .resource::<Probe>()
        .target
        .expect("actual static target");
    assert_eq!(
        world.query::<&Enemy>().iter(world).count(),
        1,
        "only the controlled target is present"
    );
    let position = world.get::<Transform>(target).unwrap().translation;
    assert_eq!(
        position,
        world.resource::<Probe>().target_position,
        "target must never move into the projectile"
    );
    let scene = world.resource::<GameAssets>().summon_scenes[4].id();
    let wyrms: Vec<_> = world
        .query_filtered::<(Entity, &WorldAssetRoot, &InheritedVisibility), With<PlayerSummon>>()
        .iter(world)
        .filter(|(_, root, visible)| root.0.id() == scene && visible.get())
        .map(|(entity, _, _)| entity)
        .collect();
    if wyrms.iter().any(|entity| imported_mesh(world, *entity)) {
        if !world.resource::<Probe>().saw_imported_wyrm {
            println!(
                "CROWN_DAMAGE_WYRM frame={frame} public_player_summon=true actual_scene4=true imported_mesh=true"
            );
        }
        world.resource_mut::<Probe>().saw_imported_wyrm = true;
    }
    let hp = world.get::<Enemy>(target).unwrap().hp;
    let previous = world.resource::<Probe>().previous_hp;
    if hp < previous {
        let damage = previous - hp;
        let probe = world.resource::<Probe>();
        let raw = probe
            .raw_preview
            .expect("actual UI raw number was recorded");
        let display = probe.adjusted_preview.unwrap_or(raw * 1.30);
        let matches = format!("{damage:.1}") == format!("{display:.1}");
        println!(
            "CROWN_DAMAGE_HP frame={frame} actor={target:?} hp_before={previous} hp_after={hp} before_bits={} after_bits={} damage={damage} raw_display={raw} adjusted_display={:?} display_rounding_match={matches}",
            previous.to_bits(),
            hp.to_bits(),
            probe.adjusted_preview
        );
        let mut probe = world.resource_mut::<Probe>();
        probe.hp_intervals += 1;
        probe.matches_display |= matches;
    }
    world.resource_mut::<Probe>().previous_hp = hp;
    if frame == 450 {
        let probe = world.resource::<Probe>();
        assert!(
            probe.saw_imported_wyrm,
            "actual producer must create a visible imported FrostWyrm"
        );
        assert!(
            probe.hp_intervals > 0 && hp > 0.0,
            "real damage must hit the surviving target"
        );
        assert!(
            probe.matches_display,
            "at least one actual HP interval must match the controlled outgoing display rounding"
        );
        // DamageProfile is private to the library. The native runner must also
        // validate the real projectile_hit trace; do not forge that observation.
        println!(
            "[capture] PASS crown-damage-preview: actual purchase/G/Later, imported scene4 PlayerSummon, stationary target and real HP loss; modifier_ui={}; resolver_profile_and_noncritical_require_external_trace=true",
            probe.expect_modifiers
        );
    }
}
