//! Exercise the production readiness plugin with controlled, genuine asset I/O.
use bevy::{
    app::SubApp,
    asset::{
        AssetApp, LoadState, RecursiveDependencyLoadState,
        io::{
            AssetReader, AssetReaderError, AssetSourceBuilder, AssetSourceId, PathStream, Reader,
            file::FileAssetReader,
        },
    },
    camera::RenderTarget,
    input::{ButtonState, mouse::MouseButtonInput},
    prelude::*,
    window::{CursorMoved, WindowRef},
};
use rogue_commander::game::{
    AssetReadiness, AssetReadinessInputSet, AssetReadinessPlugin, Autoplay, CampaignMenu,
    GameAssets, GameSession, HeroKind, SelectedHero,
};
use std::{
    fs,
    future::poll_fn,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Poll, Waker},
    time::{SystemTime, UNIX_EPOCH},
};
const BACKDROP: &str = "ui/draft_backdrop.png";
#[derive(Default)]
struct ReadGate {
    open: AtomicBool,
    reads: AtomicUsize,
    waiters: Mutex<Vec<Waker>>,
}
impl ReadGate {
    async fn wait(&self) {
        poll_fn(|context| {
            let mut waiters = self.waiters.lock().unwrap();
            if self.open.load(Ordering::SeqCst) {
                Poll::Ready(())
            } else {
                waiters.push(context.waker().clone());
                Poll::Pending
            }
        })
        .await;
    }
    fn release(&self) {
        self.open.store(true, Ordering::SeqCst);
        for waker in self.waiters.lock().unwrap().drain(..) {
            waker.wake();
        }
    }
}
struct ControlledReader {
    file: FileAssetReader,
    gate: Arc<ReadGate>,
}
impl AssetReader for ControlledReader {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        if path == Path::new(BACKDROP) {
            self.gate.reads.fetch_add(1, Ordering::SeqCst);
            self.gate.wait().await;
        }
        self.file.read(path).await
    }
    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        self.file.read_meta(path).await
    }
    async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        self.file.read_directory(path).await
    }
    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
        self.file.is_directory(path).await
    }
}
#[derive(Resource)]
struct Probe {
    frame: u32,
    root: PathBuf,
    gate: Arc<ReadGate>,
    retry_position: Option<Vec2>,
    pointer_targets: Vec<(Entity, RenderTarget)>,
    unoffered: Option<Handle<WorldAsset>>,
}
fn copy_assets(source: &Path, target: &Path, relative: &Path) {
    fs::create_dir_all(target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let child = relative.join(entry.file_name());
        if child == Path::new(BACKDROP) {
            continue;
        }
        if entry.path().is_dir() {
            copy_assets(&entry.path(), &target.join(entry.file_name()), &child);
        } else {
            fs::copy(entry.path(), target.join(entry.file_name())).unwrap();
        }
    }
}
/// Register before AssetPlugin; no production asset files or readiness flags are changed.
pub fn prepare(app: &mut App) {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("rogue-readiness-{}-{nonce}", std::process::id()));
    copy_assets(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("assets"),
        &root,
        Path::new(""),
    );
    let gate = Arc::new(ReadGate::default());
    let reader_gate = gate.clone();
    let reader_root = root.clone();
    app.register_asset_source(
        AssetSourceId::Default,
        AssetSourceBuilder::new(move || {
            Box::new(ControlledReader {
                file: FileAssetReader::new(&reader_root),
                gate: reader_gate.clone(),
            })
        }),
    );
    app.insert_resource(Probe {
        frame: 0,
        root,
        gate,
        retry_position: None,
        pointer_targets: Vec::new(),
        unoffered: None,
    });
}
pub fn configure(world: &mut World) {
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
fn press(world: &mut World, name: &str) {
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
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join(BACKDROP),
            p.root.join(BACKDROP),
        )
        .unwrap();
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
    let window = world
        .query::<(Entity, &Window)>()
        .iter(world)
        .next()
        .unwrap()
        .0;
    let position = world
        .resource::<Probe>()
        .retry_position
        .expect("original retry button coordinates");
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
        state: if frame == 66 {
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
    world.resource_mut::<Probe>().pointer_targets = targets;
    if frame == 66 {
        println!("ASSET_READINESS_REAL_REPEAT_POINTER position={position:?}");
    }
}
fn audit(world: &mut World) {
    let frame = world.resource::<Probe>().frame;
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
        .get_path_id(BACKDROP)
        .expect("real backdrop request must exist");
    let state = world.resource::<AssetServer>().get_load_state(critical);
    let reads = probe.gate.reads.load(Ordering::SeqCst);
    let gate = world.resource::<AssetReadiness>();
    println!(
        "ASSET_READINESS_STAGE frame={frame} ready={} loaded={}/{} failed={} backdrop={state:?} real_reads={reads}",
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
