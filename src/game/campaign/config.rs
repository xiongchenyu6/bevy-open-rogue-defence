//! External balance data shared by menu previews, run duration and enemy scaling.
use super::{CHAPTER_COUNT, Chapter};
use bevy::{
    asset::{AssetLoader, LoadContext, LoadState, io::Reader},
    prelude::*,
};
use std::io;

pub const PATH: &str = "data/campaign.json";
#[derive(Asset, Resource, TypePath, Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChapterCatalog {
    chapters: [Chapter; CHAPTER_COUNT],
}
impl ChapterCatalog {
    pub fn chapter(&self, stage: usize) -> Chapter {
        self.chapters[stage.min(CHAPTER_COUNT - 1)]
    }
    fn parse(bytes: &[u8]) -> io::Result<Self> {
        let catalog: Self = serde_json::from_slice(bytes).map_err(io::Error::other)?;
        for (index, chapter) in catalog.chapters.iter().enumerate() {
            for (field, value, max) in [
                ("duration", chapter.duration, 86400.),
                ("health_scale", chapter.health_scale, 10000.),
                ("damage_scale", chapter.damage_scale, 10000.),
                ("gold_scale", chapter.gold_scale, 10000.),
            ] {
                if !value.is_finite() || value <= 0. || value > max {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "Chapter {}: {field} must be greater than zero and at most {max}",
                            index + 1
                        ),
                    ));
                }
            }
        }
        Ok(catalog)
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn read_native() -> io::Result<Self> {
        Self::parse(&std::fs::read("assets/data/campaign.json")?)
    }
}
#[derive(Default, TypePath)]
struct ChapterLoader;
impl AssetLoader for ChapterLoader {
    type Asset = ChapterCatalog;
    type Settings = ();
    type Error = io::Error;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        _: &(),
        _: &mut LoadContext<'_>,
    ) -> io::Result<ChapterCatalog> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        ChapterCatalog::parse(&bytes)
    }
    fn extensions(&self) -> &[&str] {
        &["json"]
    }
}
#[derive(Resource)]
struct Loading(Handle<ChapterCatalog>);
#[derive(Component)]
struct ConfigOverlay;
#[derive(Component)]
struct RetryConfig;

pub(crate) fn install(app: &mut App) {
    if app.world().contains_resource::<AssetServer>() {
        app.init_asset::<ChapterCatalog>()
            .init_asset_loader::<ChapterLoader>()
            .add_systems(Startup, start)
            .add_systems(PreUpdate, (retry, finish).chain())
            .add_systems(Update, loading_screen);
    } else {
        // Headless simulations consume the same checked external file. Never
        // silently substitute a compiled balance table if data is missing.
        #[cfg(not(target_arch = "wasm32"))]
        app.insert_resource(
            ChapterCatalog::read_native().expect("Invalid or missing assets/data/campaign.json"),
        );
    }
}
fn start(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(Loading(server.load(PATH)));
}
fn finish(
    mut commands: Commands,
    loading: Option<Res<Loading>>,
    assets: Res<Assets<ChapterCatalog>>,
    catalog: Option<Res<ChapterCatalog>>,
) {
    if catalog.is_some() {
        return;
    }
    if let Some(loading) = loading
        && let Some(catalog) = assets.get(&loading.0)
    {
        commands.insert_resource(catalog.clone());
    }
}
fn retry(
    server: Res<AssetServer>,
    buttons: Query<&Interaction, (With<RetryConfig>, Changed<Interaction>)>,
) {
    if buttons
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        server.reload(PATH);
    }
}
fn loading_screen(
    mut commands: Commands,
    catalog: Option<Res<ChapterCatalog>>,
    loading: Option<Res<Loading>>,
    server: Res<AssetServer>,
    language: Option<Res<super::super::GameLanguage>>,
    font: Option<Res<super::super::localization::UiFont>>,
    roots: Query<Entity, With<ConfigOverlay>>,
    mut drawn: Local<Option<(bool, bool)>>,
) {
    if catalog.is_some() {
        for root in &roots {
            commands.entity(root).despawn();
        }
        *drawn = None;
        return;
    }
    let failed = loading.is_some_and(|loading| {
        matches!(
            server.get_load_state(loading.0.id()),
            Some(LoadState::Failed(_))
        )
    });
    let font_ready = font
        .as_ref()
        .is_some_and(|font| server.is_loaded_with_dependencies(font.0.id()));
    let chinese = font_ready && language.is_some_and(|language| language.is_chinese());
    if *drawn == Some((failed, chinese)) && !roots.is_empty() {
        return;
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    // The fallback embedded font supports English while the external font loads.
    let title = match (failed, chinese) {
        (true, true) => "远征数据加载失败，请联网后重试。",
        (true, false) => "Expedition data could not be loaded. Reconnect and retry.",
        (false, true) => "正在准备远征…",
        (false, false) => "Preparing expeditions…",
    };
    let text_font = TextFont {
        font: if font_ready {
            font.as_ref().unwrap().0.clone().into()
        } else {
            default()
        },
        font_size: FontSize::Px(20.),
        ..default()
    };
    commands
        .spawn((
            ConfigOverlay,
            Name::new("Expedition data loading"),
            GlobalZIndex(1100),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: px(18),
                ..default()
            },
            BackgroundColor(Color::srgb(0.015, 0.025, 0.045)),
        ))
        .with_children(|panel| {
            panel.spawn((Text::new(title), text_font.clone(), TextColor(Color::WHITE)));
            if failed {
                panel
                    .spawn((
                        Button,
                        RetryConfig,
                        Node {
                            min_height: px(48),
                            padding: UiRect::all(px(16)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.3, 0.23, 0.08)),
                    ))
                    .with_children(|button| {
                        button.spawn((
                            super::super::asset_readiness::LoadingFallback,
                            Text::new(if chinese { "重试" } else { "Retry" }),
                            text_font,
                            TextColor(Color::WHITE),
                        ));
                    });
            }
        });
    *drawn = Some((failed, chinese));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn external_file_controls_duration_and_enemy_balance_without_compiled_defaults() {
        let original = std::fs::read("assets/data/campaign.json").unwrap();
        let mut data: serde_json::Value = serde_json::from_slice(&original).unwrap();
        data["chapters"][0]["duration"] = 720.into();
        data["chapters"][0]["health_scale"] = 7.5.into();
        let loaded = ChapterCatalog::parse(&serde_json::to_vec(&data).unwrap()).unwrap();
        assert_eq!(
            super::super::fresh_session(loaded.chapter(0)).duration,
            720.
        );
        assert_eq!(loaded.chapter(0).health_scale, 7.5);
    }
    #[test]
    fn real_asset_loader_uses_changed_external_balance_for_actual_enemy_and_session() {
        use crate::game::{CampaignRun, Enemy};
        use bevy::asset::{AssetMetaCheck, AssetPlugin};
        let folder = std::env::temp_dir().join(format!("chapter-config-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let mut data: serde_json::Value =
            serde_json::from_slice(&std::fs::read("assets/data/campaign.json").unwrap()).unwrap();
        data["chapters"][0]["duration"] = 600.into();
        data["chapters"][0]["health_scale"] = 2.5.into();
        data["chapters"][0]["damage_scale"] = 3.into();
        data["chapters"][0]["gold_scale"] = 4.into();
        std::fs::write(
            folder.join("campaign.json"),
            serde_json::to_vec(&data).unwrap(),
        )
        .unwrap();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin {
                file_path: folder.to_string_lossy().into_owned(),
                meta_check: AssetMetaCheck::Never,
                ..default()
            })
            .init_asset::<ChapterCatalog>()
            .init_asset_loader::<ChapterLoader>();
        let handle: Handle<ChapterCatalog> =
            app.world().resource::<AssetServer>().load("campaign.json");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !app
            .world()
            .resource::<AssetServer>()
            .is_loaded_with_dependencies(handle.id())
        {
            app.update();
            assert!(
                std::time::Instant::now() < deadline,
                "external configuration never loaded"
            );
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let catalog = app
            .world()
            .resource::<Assets<ChapterCatalog>>()
            .get(&handle)
            .unwrap()
            .clone();
        assert_eq!(
            super::super::fresh_session(catalog.chapter(0)).duration,
            600.
        );
        app.insert_resource(catalog)
            .insert_resource(CampaignRun::default());
        let enemy = app
            .world_mut()
            .spawn(Enemy {
                hp: 100.,
                max_hp: 100.,
                speed: 1.,
                damage: 10.,
                xp: 1,
                wood: 1,
                gold: 10,
                kill_credit: 1,
                elite: false,
            })
            .id();
        let scaling = app.world_mut().register_system(super::super::scale_enemies);
        app.world_mut().run_system(scaling).unwrap();
        let result = app.world().get::<Enemy>(enemy).unwrap();
        assert_eq!((result.max_hp, result.damage, result.gold), (250., 30., 40));
        std::fs::remove_dir_all(folder).unwrap();
    }
    #[test]
    fn malformed_unknown_missing_or_out_of_range_data_is_rejected() {
        let original: serde_json::Value =
            serde_json::from_slice(&std::fs::read("assets/data/campaign.json").unwrap()).unwrap();
        assert!(ChapterCatalog::parse(b"broken").is_err());
        for field in ["duration", "health_scale", "damage_scale", "gold_scale"] {
            for value in [0., -1., 100001.] {
                let mut data = original.clone();
                data["chapters"][0][field] = value.into();
                assert!(ChapterCatalog::parse(&serde_json::to_vec(&data).unwrap()).is_err());
            }
        }
        let mut data = original.clone();
        data["chapters"].as_array_mut().unwrap().pop();
        assert!(ChapterCatalog::parse(&serde_json::to_vec(&data).unwrap()).is_err());
        let mut data = original;
        data["chapters"][0]["health_scalle"] = 1.into();
        assert!(ChapterCatalog::parse(&serde_json::to_vec(&data).unwrap()).is_err());
    }
}
