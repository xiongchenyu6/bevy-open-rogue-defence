//! Presentation-only readiness: installed explicitly by the windowed entry point.
//! Recruitment owns its pause; this gate never changes the session pause flag.
use super::{GameAssets, GameLanguage, HeroKind, HeroVisual, RunSeed, SelectedHero};
use bevy::{
    asset::{LoadState, RecursiveDependencyLoadState, UntypedAssetId},
    prelude::*,
};
use std::collections::HashSet;

#[derive(Resource, Default)]
pub struct AssetReadiness {
    pub ready: bool,
    pub loaded: usize,
    pub total: usize,
    pub failed: usize,
    pub(super) heroes: Vec<HeroKind>,
    required: Vec<UntypedAssetId>,
    failures: Vec<(UntypedAssetId, bevy::asset::AssetPath<'static>)>,
    retrying: bool,
    retry_paths: HashSet<bevy::asset::AssetPath<'static>>,
    root: Option<Entity>,
    drawn: Option<(usize, usize, usize, GameLanguage, bool, bool)>,
}
impl AssetReadiness {
    pub fn is_retrying(&self) -> bool {
        self.retrying
    }
}
#[derive(Component)]
pub(super) struct LoadingFallback;
#[derive(Component)]
struct RetryLoading;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetReadinessInputSet;

pub struct AssetReadinessPlugin;
impl Plugin for AssetReadinessPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AssetReadiness>()
            .add_systems(
                PreUpdate,
                (track_failures, retry, inspect)
                    .chain()
                    .in_set(AssetReadinessInputSet)
                    .after(bevy::ui::UiSystems::Focus),
            )
            .add_systems(PostUpdate, draw)
            .configure_sets(
                Update,
                (super::combat::CombatSet, super::combat::SurvivalSet).run_if(combat_ready),
            );
    }
}
fn combat_ready(gate: Res<AssetReadiness>) -> bool {
    gate.ready
}

fn track_failures(
    mut events: MessageReader<bevy::asset::UntypedAssetLoadFailedEvent>,
    mut gate: ResMut<AssetReadiness>,
) {
    for event in events.read() {
        // A fresh failure result ends the attempt; the old Failed state may
        // remain visible in AssetServer throughout a genuine asynchronous retry.
        if gate.retrying
            && gate
                .retry_paths
                .iter()
                .any(|path| path.without_label() == event.path.without_label())
        {
            gate.retrying = false;
            gate.drawn = None;
        }
        if !gate.failures.iter().any(|(id, _)| *id == event.id) {
            gate.failures.push((event.id, event.path.clone()));
        }
    }
}
fn retry(
    server: Res<AssetServer>,
    mut gate: ResMut<AssetReadiness>,
    buttons: Query<&Interaction, (With<RetryLoading>, Changed<Interaction>)>,
) {
    if gate.retrying
        || !buttons
            .iter()
            .any(|interaction| *interaction == Interaction::Pressed)
    {
        return;
    }
    let mut paths = HashSet::new();
    // Keep the live requested label: a failed scene may have no parent asset handle.
    // Reloading that label reads its file and rebuilds its nested assets as well.
    for &id in &gate.required {
        if failed(&server, id)
            && let Some(path) = server.get_path(id)
        {
            paths.insert(path.into_owned());
            if let Some((own, _, dependencies)) = server.get_load_states(id) {
                let error = match (own, dependencies) {
                    (LoadState::Failed(error), _)
                    | (_, RecursiveDependencyLoadState::Failed(error)) => Some(error),
                    _ => None,
                };
                if let Some(error) = error {
                    for (dependency, path) in &gate.failures {
                        if let Some(LoadState::Failed(dependency_error)) =
                            server.get_load_state(*dependency)
                            && std::sync::Arc::ptr_eq(&error, &dependency_error)
                        {
                            paths.insert(path.clone());
                        }
                    }
                }
            }
        }
    }
    if paths.is_empty() {
        return;
    }
    for path in &paths {
        server.reload(path.clone());
    }
    gate.retry_paths = paths;
    gate.retrying = true;
    gate.drawn = None;
}
fn failed(server: &AssetServer, id: UntypedAssetId) -> bool {
    server
        .get_load_states(id)
        .is_some_and(|(own, _, dependencies)| {
            matches!(own, LoadState::Failed(_))
                || matches!(dependencies, RecursiveDependencyLoadState::Failed(_))
        })
}
fn inspect(
    server: Res<AssetServer>,
    assets: Option<Res<GameAssets>>,
    art: Option<Res<super::hero::draft::DraftArt>>,
    font: Option<Res<super::localization::UiFont>>,
    seed: Res<RunSeed>,
    selected: Res<SelectedHero>,
    buttons: Query<&super::hero::HeroChoiceButton>,
    environment: Query<(&WorldAssetRoot, Option<&Visibility>), Without<HeroVisual>>,
    mut gate: ResMut<AssetReadiness>,
) {
    let mut heroes: Vec<_> = buttons.iter().map(|button| button.kind()).collect();
    if heroes.is_empty() {
        if selected.locked {
            // The completed recruitment no longer owns an overlay.
            gate.ready = true;
            return;
        }
        heroes = super::hero::initial_offer_kinds(seed.0);
    }
    let (Some(assets), Some(art), Some(font)) = (assets, art, font) else {
        gate.ready = false;
        return;
    };
    let mut ids = HashSet::new();
    for kind in &heroes {
        ids.insert(assets.hero_scenes[kind.index()].id().untyped());
        ids.insert(art.portraits[kind.index()].id().untyped());
    }
    ids.insert(art.backdrop.id().untyped());
    ids.insert(font.0.id().untyped());
    for (root, visibility) in &environment {
        if visibility != Some(&Visibility::Hidden) {
            ids.insert(root.0.id().untyped());
        }
    }
    gate.heroes = heroes;
    gate.required = ids.into_iter().collect();
    gate.total = gate.required.len();
    gate.loaded = gate
        .required
        .iter()
        .filter(|&&id| server.is_loaded_with_dependencies(id))
        .count();
    gate.failed = gate
        .required
        .iter()
        .filter(|&&id| failed(&server, id))
        .count();
    gate.ready = gate.loaded == gate.total;
    if gate.ready {
        gate.retrying = false;
    }
}
fn draw(
    mut commands: Commands,
    mut gate: ResMut<AssetReadiness>,
    server: Res<AssetServer>,
    font: Option<Res<super::localization::UiFont>>,
    language: Res<GameLanguage>,
    menu: Res<super::CampaignMenu>,
    roots: Query<(), With<LoadingRoot>>,
) {
    if gate.root.is_some_and(|root| !roots.contains(root)) {
        gate.root = None;
        gate.drawn = None;
    }
    if gate.ready || menu.0 {
        if let Some(root) = gate.root.take() {
            commands.entity(root).despawn();
        }
        gate.drawn = None;
        return;
    }
    let font_ready = font
        .as_ref()
        .is_some_and(|font| server.is_loaded_with_dependencies(font.0.id()));
    let signature = (
        gate.loaded,
        gate.total,
        gate.failed,
        *language,
        font_ready,
        gate.retrying,
    );
    if gate.drawn == Some(signature) {
        return;
    }
    if let Some(root) = gate.root.take() {
        commands.entity(root).despawn();
    }
    // Bevy's embedded default font supplies English before the external font loads.
    let copy_language = if font_ready {
        *language
    } else {
        GameLanguage::English
    };
    let status = if gate.failed > 0 && !gate.retrying {
        copy_language.pick(
            "Some expedition assets could not be loaded. Restore the game files and retry.",
            "部分远征资源无法加载。请恢复游戏文件后重试。",
        )
    } else {
        copy_language.pick("Preparing expedition", "正在准备远征")
    };
    let root = commands
        .spawn((
            LoadingRoot,
            Name::new("Expedition loading"),
            GlobalZIndex(1000),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: px(18),
                padding: UiRect::all(px(24)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.015, 0.025, 0.045)),
        ))
        .with_children(|panel| {
            loading_text(
                panel,
                format!("{status}\n{} / {}", gate.loaded, gate.total),
                font_ready,
                font.as_deref(),
            );
            if gate.failed > 0 && !gate.retrying {
                panel
                    .spawn((
                        Button,
                        RetryLoading,
                        Name::new("Retry loading"),
                        Node {
                            min_height: px(48),
                            padding: UiRect::all(px(16)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.3, 0.23, 0.08)),
                    ))
                    .with_children(|button| {
                        loading_text(
                            button,
                            copy_language.pick("Retry loading", "重试加载").into(),
                            font_ready,
                            font.as_deref(),
                        )
                    });
            }
        })
        .id();
    gate.root = Some(root);
    gate.drawn = Some(signature);
}
#[derive(Component)]
struct LoadingRoot;
fn loading_text(
    parent: &mut ChildSpawnerCommands,
    copy: String,
    ready: bool,
    font: Option<&super::localization::UiFont>,
) {
    let mut text_font = TextFont {
        font_size: FontSize::Px(20.),
        ..default()
    };
    if ready && let Some(font) = font {
        text_font.font = font.0.clone().into();
    }
    parent.spawn((
        LoadingFallback,
        Text::new(copy),
        text_font,
        TextColor(Color::WHITE),
        Pickable::IGNORE,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::io::{
        AssetReader, AssetReaderError, AssetSourceBuilder, AssetSourceId, PathStream,
        file::FileAssetReader,
    };
    use bevy::asset::{AssetLoader, AssetMetaCheck, AssetPlugin, LoadContext, io::Reader};
    use std::{
        future::poll_fn,
        path::Path,
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, AtomicUsize, Ordering},
        },
        task::{Poll, Waker},
    };
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
            if path == Path::new("held.readyfixture") {
                self.gate.reads.fetch_add(1, Ordering::SeqCst);
                self.gate.wait().await;
            }
            self.file.read(path).await
        }
        async fn read_meta<'a>(
            &'a self,
            path: &'a Path,
        ) -> Result<impl Reader + 'a, AssetReaderError> {
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

    #[derive(Asset, TypePath)]
    struct Fixture(Vec<u8>, #[dependency] Option<Handle<Fixture>>);
    #[derive(Default, TypePath)]
    struct FixtureLoader;
    impl AssetLoader for FixtureLoader {
        type Asset = Fixture;
        type Settings = ();
        type Error = std::io::Error;
        async fn load(
            &self,
            reader: &mut dyn Reader,
            _: &(),
            context: &mut LoadContext<'_>,
        ) -> Result<Fixture, Self::Error> {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let dependency = std::str::from_utf8(&bytes)
                .ok()
                .and_then(|text| text.strip_prefix("dep:"))
                .map(|path| context.load(path.to_owned()));
            Ok(Fixture(bytes, dependency))
        }
        fn extensions(&self) -> &[&str] {
            &["readyfixture"]
        }
    }
    fn wait_for(app: &mut App, id: UntypedAssetId, ready: bool) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            app.update();
            let server = app.world().resource::<AssetServer>();
            if if ready {
                server.is_loaded_with_dependencies(id)
            } else {
                failed(server, id)
            } {
                return;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "asset loader never reached requested state"
            );
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }
    #[test]
    fn real_long_retry_keeps_pending_until_new_failure_then_allows_another_attempt() {
        let dir = std::env::temp_dir().join(format!("readiness-held-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let io_gate = Arc::new(ReadGate::default());
        io_gate.release();
        let reader_gate = io_gate.clone();
        let reader_root = dir.clone();
        let mut app = App::new();
        app.register_asset_source(
            AssetSourceId::Default,
            AssetSourceBuilder::new(move || {
                Box::new(ControlledReader {
                    file: FileAssetReader::new(&reader_root),
                    gate: reader_gate.clone(),
                })
            }),
        );
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin {
                meta_check: AssetMetaCheck::Never,
                ..default()
            })
            .init_asset::<Fixture>()
            .init_asset_loader::<FixtureLoader>()
            .init_resource::<AssetReadiness>()
            .add_systems(PreUpdate, (track_failures, retry).chain());
        let handle: Handle<Fixture> = app
            .world()
            .resource::<AssetServer>()
            .load("held.readyfixture");
        let id = handle.id().untyped();
        wait_for(&mut app, id, false);
        app.update(); // Consume initial failure message before starting the attempt.
        app.world_mut().resource_mut::<AssetReadiness>().required = vec![id];
        io_gate.open.store(false, Ordering::SeqCst);
        let button = app
            .world_mut()
            .spawn((RetryLoading, Interaction::Pressed))
            .id();
        app.update();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while io_gate.reads.load(Ordering::SeqCst) < 2 {
            app.update();
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let held_until = std::time::Instant::now() + std::time::Duration::from_millis(1100);
        while std::time::Instant::now() < held_until {
            app.update();
            assert!(app.world().resource::<AssetReadiness>().is_retrying());
            assert!(failed(app.world().resource::<AssetServer>(), id));
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        app.world_mut().entity_mut(button).insert(Interaction::None);
        app.update();
        app.world_mut()
            .entity_mut(button)
            .insert(Interaction::Pressed);
        app.update();
        assert_eq!(
            io_gate.reads.load(Ordering::SeqCst),
            2,
            "fresh repeated click cannot start another read while pending"
        );
        io_gate.release(); // The still-missing file produces a genuinely new failure.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while app.world().resource::<AssetReadiness>().is_retrying() {
            app.update();
            assert!(
                std::time::Instant::now() < deadline,
                "new failure did not unlock retry"
            );
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        std::fs::write(dir.join("held.readyfixture"), b"recovered").unwrap();
        app.world_mut().entity_mut(button).insert(Interaction::None);
        app.update();
        app.world_mut()
            .entity_mut(button)
            .insert(Interaction::Pressed);
        app.update();
        wait_for(&mut app, id, true);
        assert_eq!(
            app.world()
                .resource::<Assets<Fixture>>()
                .get(&handle)
                .unwrap()
                .0,
            b"recovered"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn pending_gate_stops_combat_from_first_frame_without_taking_pause_ownership() {
        #[derive(Resource, Default)]
        struct Attacks(u32);
        let mut app = App::new();
        app.init_resource::<AssetReadiness>()
            .init_resource::<super::super::GameSession>()
            .init_resource::<Attacks>()
            .configure_sets(Update, super::super::combat::CombatSet.run_if(combat_ready))
            .add_systems(
                Update,
                (|mut session: ResMut<super::super::GameSession>, mut attacks: ResMut<Attacks>| {
                    session.elapsed += 1.;
                    attacks.0 += 1;
                })
                .in_set(super::super::combat::CombatSet),
            );
        app.update();
        assert_eq!(
            app.world().resource::<super::super::GameSession>().elapsed,
            0.
        );
        assert_eq!(app.world().resource::<Attacks>().0, 0);
        assert!(!app.world().resource::<super::super::GameSession>().paused);
        app.world_mut().resource_mut::<AssetReadiness>().ready = true;
        app.update();
        assert_eq!(app.world().resource::<Attacks>().0, 1);
        app.world_mut().resource_mut::<AssetReadiness>().ready = false;
        app.world_mut()
            .resource_mut::<super::super::GameSession>()
            .paused = true;
        app.update();
        assert_eq!(app.world().resource::<Attacks>().0, 1);
        assert!(app.world().resource::<super::super::GameSession>().paused);
    }
    #[test]
    fn real_failed_file_retry_button_reloads_restored_asset_without_changing_pause() {
        let dir = std::env::temp_dir().join(format!("expedition-readiness-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin {
                file_path: dir.to_string_lossy().into(),
                meta_check: AssetMetaCheck::Never,
                ..default()
            })
            .init_asset::<Fixture>()
            .init_asset_loader::<FixtureLoader>()
            .init_resource::<AssetReadiness>()
            .add_systems(PreUpdate, (track_failures, retry).chain());
        std::fs::write(dir.join("parent.readyfixture"), b"dep:child.readyfixture").unwrap();
        let parent: Handle<Fixture> = app
            .world()
            .resource::<AssetServer>()
            .load("parent.readyfixture");
        wait_for(&mut app, parent.id().untyped(), false);
        let handle: Handle<Fixture> = app
            .world()
            .resource::<AssetServer>()
            .load("restored.readyfixture");
        let id = handle.id().untyped();
        wait_for(&mut app, id, false);
        app.world_mut().resource_mut::<AssetReadiness>().required = vec![id, parent.id().untyped()];
        // Unrequired failures must not be retried or included in progress.
        let unused: Handle<Fixture> = app
            .world()
            .resource::<AssetServer>()
            .load("unused.readyfixture");
        wait_for(&mut app, unused.id().untyped(), false);
        app.insert_resource(super::super::GameSession {
            paused: true,
            ..default()
        });
        std::fs::write(dir.join("restored.readyfixture"), b"restored").unwrap();
        std::fs::write(dir.join("child.readyfixture"), b"child restored").unwrap();
        app.world_mut().spawn((RetryLoading, Interaction::Pressed));
        wait_for(&mut app, id, true);
        wait_for(&mut app, parent.id().untyped(), true);
        assert!(app.world().resource::<super::super::GameSession>().paused);
        app.world_mut()
            .resource_mut::<super::super::GameSession>()
            .paused = false;
        app.update();
        assert!(!app.world().resource::<super::super::GameSession>().paused);
        assert_eq!(
            app.world()
                .resource::<Assets<Fixture>>()
                .get(&handle)
                .unwrap()
                .0,
            b"restored"
        );
        assert!(failed(
            app.world().resource::<AssetServer>(),
            unused.id().untyped()
        ));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
