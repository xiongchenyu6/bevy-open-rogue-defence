//! Real later recruitment over missing catalog scenes; only level 10 is awarded.
use super::{
    asset_io_capture::{self, ReadGate},
    asset_readiness_capture,
};
use bevy::{
    app::SubApp,
    asset::{LoadState, RecursiveDependencyLoadState},
    camera::RenderTarget,
    prelude::*,
};
use rogue_commander::game::{
    AssetReadiness, AssetReadinessInputSet, AssetReadinessPlugin, Autoplay, BondCardId,
    CampaignMenu, GameAssets, GameSession, HeroFusion, HeroKind, PlayerBuild, RecruitmentProgress,
    RecruitmentSource, SelectedHero, SkillId,
};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, atomic::Ordering},
};
const LATE_MODELS: [&str; 9] = [
    "models/hero_spirit_keeper.glb",
    "models/hero_tide_walker.glb",
    "models/hero_star_wagerer.glb",
    "models/hero_frost_oracle.glb",
    "models/hero_iron_berserker.glb",
    "models/hero_rift_dancer.glb",
    "models/hero_grove_artificer.glb",
    "models/hero_solar_vanguard.glb",
    "models/hero_void_reaper.glb",
];
#[derive(Resource)]
struct Probe {
    frame: u32,
    root: PathBuf,
    gate: Arc<ReadGate>,
    old: Option<HeroKind>,
    elapsed: f32,
    skills: [u8; SkillId::COUNT],
    cards: [bool; BondCardId::COUNT],
    fusion_count: usize,
    offers: Vec<String>,
    retry_reads: Option<BTreeMap<String, usize>>,
    retry_position: Option<Vec2>,
    pointer_targets: Vec<(Entity, RenderTarget)>,
}
pub fn prepare(app: &mut App) {
    let (root, gate) = asset_io_capture::prepare(app, &LATE_MODELS, true);
    app.insert_resource(Probe {
        frame: 0,
        root,
        gate,
        old: None,
        elapsed: 0.,
        skills: [0; SkillId::COUNT],
        cards: [false; BondCardId::COUNT],
        fusion_count: 0,
        offers: Vec::new(),
        retry_reads: None,
        retry_position: None,
        pointer_targets: Vec::new(),
    });
}
pub fn plugin(app: &mut App) {
    app.add_plugins(AssetReadinessPlugin);
}
pub fn configure(world: &mut World) {
    world.remove_resource::<Autoplay>();
    world.resource_mut::<CampaignMenu>().0 = false;
    world.resource_mut::<SelectedHero>().locked = false;
    let mut session = world.resource_mut::<GameSession>();
    session.paused = false;
    session.auto_bond_draws = false;
}
pub fn install(app: &mut SubApp) {
    app.add_systems(First, repeat_click);
    app.add_systems(
        PreUpdate,
        input
            .after(bevy::ui::UiSystems::Focus)
            .before(AssetReadinessInputSet),
    );
    app.add_systems(Last, audit);
}
fn input(world: &mut World) {
    let frame = {
        let mut p = world.resource_mut::<Probe>();
        p.frame += 1;
        p.frame
    };
    let pressed: Vec<_> = world
        .query::<(Entity, &Interaction)>()
        .iter(world)
        .filter_map(|(e, i)| (*i == Interaction::Pressed).then_some(e))
        .collect();
    if frame != 66 {
        for entity in pressed {
            *world.get_mut::<Interaction>(entity).unwrap() = Interaction::None;
        }
    }
    for (entity, target) in std::mem::take(&mut world.resource_mut::<Probe>().pointer_targets) {
        if let Some(mut current) = world.get_mut::<RenderTarget>(entity) {
            *current = target;
        }
    }
    if frame == 35 {
        let old = world.resource::<SelectedHero>().kind;
        let elapsed = world.resource::<GameSession>().elapsed;
        let build = world.resource::<PlayerBuild>();
        let skills = build.skills;
        let cards = build.owned_bond_cards;
        let fusion_count = world.resource::<HeroFusion>().count();
        {
            let mut p = world.resource_mut::<Probe>();
            p.old = Some(old);
            p.elapsed = elapsed;
            p.skills = skills;
            p.cards = cards;
            p.fusion_count = fusion_count;
        }
        world.resource_mut::<GameSession>().level = 10;
        println!("RECRUIT_READINESS_AWARD level=10 old={old:?} clock={elapsed}");
    }
    if [20, 36, 42, 72, 92].contains(&frame) {
        asset_readiness_capture::press(world, "Commander card 0");
    }
    if [36, 43, 73, 93].contains(&frame) {
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Digit1);
    }
    if [37, 44, 74, 94].contains(&frame) {
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::Digit1);
    }
    if frame == 50 {
        let p = world.resource::<Probe>();
        p.gate.open.store(false, Ordering::SeqCst);
        asset_io_capture::restore(&p.root, &LATE_MODELS);
    }
    if frame == 60 {
        asset_readiness_capture::press(world, "Retry loading");
    }
    if frame == 95 {
        world.resource::<Probe>().gate.release();
    }
    if frame == 130 {
        asset_readiness_capture::press(world, "Commander card 0");
    }
}
fn repeat_click(world: &mut World) {
    let frame = world.resource::<Probe>().frame + 1;
    if ![66, 67].contains(&frame) {
        return;
    }
    let position = world
        .resource::<Probe>()
        .retry_position
        .expect("real loading retry button coordinates");
    let targets = asset_readiness_capture::pointer_click(world, position, frame == 66);
    world.resource_mut::<Probe>().pointer_targets = targets;
}
fn offer_paths(world: &mut World) -> Vec<String> {
    let cards: Vec<_> = world
        .query::<(Entity, &Name)>()
        .iter(world)
        .filter_map(|(entity, name)| {
            name.as_str()
                .starts_with("Commander card ")
                .then_some(entity)
        })
        .collect();
    let portraits: Vec<_> = world
        .query::<(Entity, &ImageNode)>()
        .iter(world)
        .map(|(e, image)| (e, image.image.clone()))
        .collect();
    let mut paths = Vec::new();
    for (entity, image) in portraits {
        let mut ancestor = entity;
        while !cards.contains(&ancestor) {
            let Some(parent) = world.get::<ChildOf>(ancestor) else {
                break;
            };
            ancestor = parent.parent();
        }
        if !cards.contains(&ancestor) {
            continue;
        }
        let Some(path) = world.resource::<AssetServer>().get_path(image.id()) else {
            continue;
        };
        if let Some(file) = path
            .path()
            .file_stem()
            .and_then(|name| name.to_str())
            .filter(|name| name.starts_with("hero_"))
        {
            paths.push(format!("models/{file}.glb"));
        }
    }
    paths.sort();
    paths.dedup();
    paths
}
fn audit(world: &mut World) {
    let frame = world.resource::<Probe>().frame;
    if frame == 15 {
        assert!(
            world.resource::<AssetReadiness>().ready,
            "missing unoffered catalog scenes must not block initial R offers"
        );
        assert_eq!(offer_paths(world).len(), 3);
        assert!(!world.resource::<SelectedHero>().locked);
        for handle in &world.resource::<GameAssets>().hero_scenes[3..] {
            let states = world
                .resource::<AssetServer>()
                .get_load_states(handle.id())
                .expect("catalog model requested");
            assert!(
                matches!(states.0, LoadState::Failed(_))
                    || matches!(states.2, RecursiveDependencyLoadState::Failed(_)),
                "actual unoffered GLB must fail"
            );
        }
        assert_eq!(
            world.resource::<Probe>().gate.reads.load(Ordering::SeqCst),
            9
        );
        println!("RECRUIT_READINESS_INITIAL_READY nine genuine unoffered failures ignored");
    }
    if frame == 30 {
        assert!(world.resource::<SelectedHero>().locked);
        assert!(world.resource::<GameSession>().elapsed > 0.);
    }
    if frame == 36 {
        let paths = offer_paths(world);
        assert_eq!(paths.len(), 6);
        assert!(
            paths
                .iter()
                .any(|path| LATE_MODELS.contains(&path.as_str()))
        );
        world.resource_mut::<Probe>().offers = paths;
    }
    if (36..130).contains(&frame) {
        let p = world.resource::<Probe>();
        let selected = world.resource::<SelectedHero>();
        assert!(selected.locked);
        assert_eq!(
            Some(selected.kind),
            p.old,
            "blocked card/key cannot replace commander"
        );
        assert_eq!(
            world.resource::<GameSession>().elapsed,
            p.elapsed,
            "later picker owns a frozen clock"
        );
        assert_eq!(
            world.resource::<RecruitmentProgress>().claimed_milestones,
            0
        );
        assert_eq!(
            world.resource::<RecruitmentProgress>().pending,
            Some(RecruitmentSource::Milestone(10))
        );
        assert_eq!(world.resource::<HeroFusion>().count(), p.fusion_count);
        assert_eq!(world.resource::<PlayerBuild>().skills, p.skills);
        assert_eq!(world.resource::<PlayerBuild>().owned_bond_cards, p.cards);
    }
    if frame == 59 {
        let position = world
            .query::<(&Name, &UiGlobalTransform)>()
            .iter(world)
            .find_map(|(name, t)| (name.as_str() == "Retry loading").then_some(t.translation))
            .expect("real later-recruitment retry layout");
        world.resource_mut::<Probe>().retry_position = Some(position);
    }
    if [36, 45].contains(&frame) {
        let g = world.resource::<AssetReadiness>();
        assert!(!g.ready && g.failed > 0);
    }
    if [70, 75, 94].contains(&frame) {
        let g = world.resource::<AssetReadiness>();
        assert!(!g.ready && g.is_retrying());
        let reads = world
            .resource::<Probe>()
            .gate
            .path_reads
            .lock()
            .unwrap()
            .clone();
        if frame == 70 {
            let p = world.resource::<Probe>();
            for path in LATE_MODELS {
                assert_eq!(
                    reads.get(path),
                    Some(&if p.offers.iter().any(|offer| offer == path) {
                        2
                    } else {
                        1
                    }),
                    "retry reads only real offered failed scenes: path={path}, offers={:?}, reads={reads:?}",
                    p.offers
                );
            }
            world.resource_mut::<Probe>().retry_reads = Some(reads);
        } else {
            assert_eq!(
                Some(&reads),
                world.resource::<Probe>().retry_reads.as_ref(),
                "repeat click cannot issue more reads"
            );
        }
        assert!(!world.resource::<Probe>().gate.open.load(Ordering::SeqCst));
        assert!(
            world
                .query::<&Name>()
                .iter(world)
                .all(|name| name.as_str() != "Retry loading")
        );
        println!(
            "RECRUIT_READINESS_HELD frame={frame} path_reads={:?}",
            world.resource::<Probe>().retry_reads
        );
    }
    if frame == 120 {
        let g = world.resource::<AssetReadiness>();
        assert!(g.ready && g.failed == 0 && g.loaded == g.total);
        for path in &world.resource::<Probe>().offers {
            let id = world
                .resource::<AssetServer>()
                .get_path_id(format!("{path}#Scene0"))
                .expect("actual offered model handle");
            assert!(
                world
                    .resource::<AssetServer>()
                    .is_loaded_with_dependencies(id),
                "actual current offered scene and all dependencies must load"
            );
        }
        println!("RECRUIT_READINESS_RECOVERED current six scene dependencies loaded");
    }
    if frame >= 130 {
        let p = world.resource::<Probe>();
        let old = p.old.unwrap();
        let selected = world.resource::<SelectedHero>();
        assert!(selected.locked && selected.kind != old);
        assert_eq!(
            selected.kind,
            HeroKind::GroveArtificer,
            "fixed seed must select a genuinely reloaded non-R model"
        );
        assert_eq!(
            world.resource::<RecruitmentProgress>().claimed_milestones,
            1
        );
        assert_eq!(world.resource::<RecruitmentProgress>().pending, None);
        assert!(world.resource::<HeroFusion>().has(old));
        assert_eq!(world.resource::<HeroFusion>().count(), p.fusion_count + 1);
        assert_eq!(world.resource::<PlayerBuild>().owned_bond_cards, p.cards);
        for (current, prior) in world.resource::<PlayerBuild>().skills.iter().zip(p.skills) {
            assert!(
                *current >= prior,
                "recruitment must retain prior skill ranks"
            );
        }
    }
    if frame == 150 {
        assert!(world.resource::<GameSession>().elapsed > world.resource::<Probe>().elapsed);
        println!(
            "RECRUIT_READINESS_PASS initial ready, six late offers blocked, genuine held retry >1.1s, one commitment, retained skills/bonds/legacy, resumed clock"
        );
    }
}
