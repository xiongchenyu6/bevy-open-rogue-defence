//! Exercise the production readiness plugin with controlled, genuine asset I/O.
use super::asset_io_capture::{self, ReadGate};
use bevy::{
    app::SubApp,
    asset::{LoadState, RecursiveDependencyLoadState},
    camera::RenderTarget,
    input::{ButtonState, mouse::MouseButtonInput},
    prelude::*,
    text::FontSource,
    window::{CursorMoved, WindowRef},
};
use rogue_commander::game::{
    AssetReadiness, AssetReadinessInputSet, AssetReadinessPlugin, Autoplay, CampaignMenu,
    GameAssets, GameLanguage, GameSession, HeroKind, SelectedHero,
};
use std::{
    path::PathBuf,
    sync::{Arc, atomic::Ordering},
};
const BACKDROP: &str = "ui/draft_backdrop.png";
const UI_FONT: &str = "fonts/rogue_commander_ui.ttf";
#[derive(Resource)]
struct Probe {
    target: &'static str,
    frame: u32,
    root: PathBuf,
    gate: Arc<ReadGate>,
    retry_position: Option<Vec2>,
    pointer_targets: Vec<(Entity, RenderTarget)>,
    unoffered: Option<Handle<WorldAsset>>,
}
/// Register before AssetPlugin; no production asset files or readiness flags are changed.
pub fn prepare(app: &mut App, font_scenario: bool) {
    let target = if font_scenario { UI_FONT } else { BACKDROP };
    let (root, gate) = asset_io_capture::prepare(app, &[target], false);
    app.insert_resource(Probe {
        target,
        frame: 0,
        root,
        gate,
        retry_position: None,
        pointer_targets: Vec::new(),
        unoffered: None,
    });
}
pub fn configure(world: &mut World) {
    if world.resource::<Probe>().target == UI_FONT {
        world.insert_resource(GameLanguage::SimplifiedChinese);
    }
    world.remove_resource::<Autoplay>();
    world.resource_mut::<CampaignMenu>().0 = false;
    world.resource_mut::<GameSession>().paused = false;
    world.resource_mut::<SelectedHero>().locked = false;
}
pub fn plugin(app: &mut App) {
    app.add_plugins(AssetReadinessPlugin);
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
pub(super) fn press(world: &mut World, name: &str) {
    let e = world
        .query::<(Entity, &Name, &Interaction)>()
        .iter(world)
        .find_map(|(e, n, _)| (n.as_str() == name).then_some(e))
        .unwrap_or_else(|| panic!("missing real UI control {name}"));
    *world.get_mut::<Interaction>(e).unwrap() = Interaction::Pressed;
}
fn input(world: &mut World) {
    let frame = {
        let mut probe = world.resource_mut::<Probe>();
        probe.frame += 1;
        probe.frame
    };
    if frame == 2 {
        let missing = world
            .resource::<AssetServer>()
            .load::<WorldAsset>("models/readiness-unoffered-missing.glb#Scene0");
        world.resource_mut::<GameAssets>().hero_scenes[HeroKind::SolarVanguard.index()] =
            missing.clone();
        world.resource_mut::<Probe>().unoffered = Some(missing);
    }
    let pressed: Vec<Entity> = world
        .query::<(Entity, &Interaction)>()
        .iter(world)
        .filter_map(|(e, i)| (*i == Interaction::Pressed).then_some(e))
        .collect();
    if frame != 66 {
        for e in pressed {
            *world.get_mut::<Interaction>(e).unwrap() = Interaction::None;
        }
    }
    for (entity, target) in std::mem::take(&mut world.resource_mut::<Probe>().pointer_targets) {
        if let Some(mut current) = world.get_mut::<RenderTarget>(entity) {
            *current = target;
        }
    }
    if [12, 42, 72].contains(&frame) {
        press(world, "Commander card 0");
    }
    if [13, 43, 73].contains(&frame) {
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Digit1);
    }
    if [14, 44, 74].contains(&frame) {
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::Digit1);
    }
    if frame == 20 {
        world.resource::<Probe>().gate.release();
    }
    if frame == 50 {
        let p = world.resource::<Probe>();
        p.gate.open.store(false, Ordering::SeqCst);
        asset_io_capture::restore(&p.root, &[p.target]);
    }
    if frame == 60 {
        press(world, "Retry loading");
        println!("ASSET_READINESS_REAL_RETRY_CLICK");
    }
    if frame == 85 {
        world.resource::<Probe>().gate.release();
    }
    if frame == 120 {
        press(world, "Commander card 0");
    }
}
fn repeat_click(world: &mut World) {
    let frame = world.resource::<Probe>().frame + 1;
    if frame != 66 && frame != 67 {
        return;
    }
    let position = world
        .resource::<Probe>()
        .retry_position
        .expect("original retry button coordinates");
    let targets = pointer_click(world, position, frame == 66);
    world.resource_mut::<Probe>().pointer_targets = targets;
    if frame == 66 {
        println!("ASSET_READINESS_REAL_REPEAT_POINTER position={position:?}");
    }
}
/// Inject real cursor and pointer events against the real button coordinates.
pub(super) fn pointer_click(
    world: &mut World,
    position: Vec2,
    pressed: bool,
) -> Vec<(Entity, RenderTarget)> {
    let window = world
        .query::<(Entity, &Window)>()
        .iter(world)
        .next()
        .unwrap()
        .0;
    let logical = position / world.get::<Window>(window).unwrap().scale_factor();
    world
        .get_mut::<Window>(window)
        .unwrap()
        .set_cursor_position(Some(logical));
    world.write_message(CursorMoved {
        window,
        position: logical,
        delta: None,
    });
    world.write_message(MouseButtonInput {
        button: MouseButton::Left,
        state: if pressed {
            ButtonState::Pressed
        } else {
            ButtonState::Released
        },
        window,
    });
    let targets: Vec<_> = world
        .query_filtered::<(Entity, &RenderTarget), With<Camera>>()
        .iter(world)
        .map(|(e, t)| (e, t.clone()))
        .collect();
    for (e, _) in &targets {
        *world.get_mut::<RenderTarget>(*e).unwrap() =
            RenderTarget::Window(WindowRef::Entity(window));
    }
    targets
}

fn audit(world: &mut World) {
    let frame = world.resource::<Probe>().frame;
    if world.resource::<Probe>().target == UI_FONT {
        audit_font(world, frame);
    }
    if frame == 59 {
        let position = world
            .query::<(&Name, &UiGlobalTransform)>()
            .iter(world)
            .find_map(|(n, t)| (n.as_str() == "Retry loading").then_some(t.translation))
            .expect("actual retry button layout");
        world.resource_mut::<Probe>().retry_position = Some(position);
    }
    if frame <= 119 {
        assert!(
            !world.resource::<SelectedHero>().locked,
            "loading/retry attempts must not lock a hero"
        );
        assert_eq!(
            world.resource::<GameSession>().elapsed,
            0.0,
            "gameplay clock must stay frozen under the actual picker/readiness gate"
        );
    }
    if ![10, 15, 40, 45, 67, 70, 75, 110, 140].contains(&frame) {
        return;
    }
    let probe = world.resource::<Probe>();
    let critical = world
        .resource::<AssetServer>()
        .get_path_id(probe.target)
        .expect("real controlled asset request must exist");
    let state = world.resource::<AssetServer>().get_load_state(critical);
    let reads = probe.gate.reads.load(Ordering::SeqCst);
    let gate = world.resource::<AssetReadiness>();
    println!(
        "ASSET_READINESS_STAGE frame={frame} ready={} loaded={}/{} failed={} controlled={state:?} real_reads={reads}",
        gate.ready, gate.loaded, gate.total, gate.failed
    );
    if [10, 15].contains(&frame) {
        assert!(!gate.ready && gate.total > 0);
        assert!(
            matches!(state, Some(LoadState::Loading)),
            "controlled FileAssetReader must genuinely remain in Loading"
        );
    }
    if [40, 45].contains(&frame) {
        assert!(!gate.ready && gate.failed > 0);
        assert!(matches!(state, Some(LoadState::Failed(_))));
    }
    if frame == 110 || frame == 140 {
        let unoffered = world
            .resource::<Probe>()
            .unoffered
            .as_ref()
            .expect("real unoffered catalog handle");
        let states = world
            .resource::<AssetServer>()
            .get_load_states(unoffered.id())
            .expect("unoffered request tracked");
        assert!(
            matches!(&states.0, LoadState::Failed(_))
                || matches!(&states.2, RecursiveDependencyLoadState::Failed(_)),
            "unoffered catalog scene must genuinely fail"
        );
        println!("ASSET_READINESS_UNOFFERED_FAILURE_IGNORED SolarVanguard {states:?}");
    }
    if frame == 110 {
        assert!(gate.ready && gate.loaded == gate.total && gate.failed == 0);
    }
    if [67, 70, 75].contains(&frame) {
        assert!(!gate.ready && gate.total > 0 && gate.is_retrying());
        assert_eq!(
            reads, 2,
            "retry must reach the real reader exactly once despite repeat click"
        );
        assert!(
            !probe.gate.open.load(Ordering::SeqCst),
            "second real file read remains held"
        );
        let retry_visible = world
            .query::<&Name>()
            .iter(world)
            .any(|n| n.as_str() == "Retry loading");
        assert!(
            !retry_visible,
            "pending retry must not expose another retry action"
        );
        let preparing = world
            .query::<&Text>()
            .iter(world)
            .any(|t| t.0.contains("Preparing expedition") || t.0.contains("正在准备远征"));
        assert!(
            preparing,
            "real retry overlay must show preparing, not stale failure"
        );
    }
    if frame == 140 {
        assert!(
            world.resource::<SelectedHero>().locked,
            "fresh actual button becomes available only after readiness"
        );
        assert!(world.resource::<GameSession>().elapsed > 0.0);
        assert_eq!(
            reads, 2,
            "one original read and one genuine retry; no repeated reload"
        );
        println!(
            "ASSET_READINESS_PASS genuine pending/error/reload/ready, blocked inputs and clock"
        );
    }
}

// Inspect real production TextFont sources; language strings alone cannot prove fallback.
fn audit_font(world: &mut World, frame: u32) {
    if ![10, 15, 40, 45, 67, 70, 75, 110, 140].contains(&frame) {
        return;
    }
    let font_id = world
        .resource::<AssetServer>()
        .get_path_id(UI_FONT)
        .expect("production UI font request");
    if frame < 85 {
        assert!(
            !world
                .resource::<AssetServer>()
                .is_loaded_with_dependencies(font_id)
        );
        let overlay = world
            .query::<(Entity, &Name)>()
            .iter(world)
            .find_map(|(entity, name)| (name.as_str() == "Expedition loading").then_some(entity))
            .expect("actual fallback loading overlay");
        let rows: Vec<_> = world
            .query::<(Entity, &Text, &TextFont)>()
            .iter(world)
            .map(|(entity, text, font)| (entity, text.0.clone(), font.font.clone()))
            .collect();
        let mut status_seen = false;
        let mut retry_seen = false;
        for (entity, text, font) in rows {
            let mut ancestor = entity;
            let mut in_overlay = ancestor == overlay;
            while !in_overlay {
                let Some(parent) = world.get::<ChildOf>(ancestor) else {
                    break;
                };
                ancestor = parent.parent();
                in_overlay = ancestor == overlay;
            }
            if !in_overlay {
                continue;
            }
            assert!(
                matches!(font,FontSource::Handle(ref handle) if *handle == Handle::<Font>::default()),
                "fallback must use Bevy's embedded font"
            );
            assert!(
                !text.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)),
                "missing font overlay must remain readable English"
            );
            if text.contains("Preparing expedition")
                || text.contains("Some expedition assets could not be loaded")
            {
                status_seen = true;
            }
            if text == "Retry loading" {
                retry_seen = true;
            }
        }
        assert!(status_seen, "actual English fallback status");
        assert_eq!(retry_seen, [40, 45].contains(&frame));
        println!(
            "ASSET_READINESS_FONT_FALLBACK frame={frame} embedded English, retry={retry_seen}"
        );
    } else {
        assert!(
            world
                .resource::<AssetServer>()
                .is_loaded_with_dependencies(font_id)
        );
        assert!(
            world
                .query::<&Name>()
                .iter(world)
                .all(|name| name.as_str() != "Expedition loading")
        );
        let rows: Vec<_> = world
            .query::<(&Text, &TextFont)>()
            .iter(world)
            .map(|(text, font)| (text.0.clone(), font.font.clone()))
            .collect();
        assert!(
            rows.iter().any(|(text, font)| text
                .chars()
                .any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c))
                && matches!(font,FontSource::Handle(handle) if handle.id().untyped()==font_id)),
            "recovered actual Chinese UI must use loaded production font"
        );
        if frame == 110 {
            assert!(
                rows.iter()
                    .any(|(text, _)| text == "点击基础指挥官开始远征")
            );
        }
        println!("ASSET_READINESS_FONT_RECOVERED frame={frame} loaded real Chinese UI font");
    }
}
