#[cfg(target_arch = "wasm32")]
use bevy::asset::{AssetMetaCheck, AssetPlugin};
use bevy::prelude::*;

pub mod game;

pub fn build_app() -> App {
    let primary_window = Window {
        title: "Rogue Commander".into(),
        resolution: (1280, 720).into(),
        resizable: true,
        ..default()
    };

    #[cfg(target_arch = "wasm32")]
    let primary_window = Window {
        canvas: Some("#bevy-canvas".into()),
        fit_canvas_to_parent: true,
        prevent_default_event_handling: true,
        ..primary_window
    };

    let default_plugins = DefaultPlugins.set(WindowPlugin {
        primary_window: Some(primary_window),
        ..default()
    });

    #[cfg(target_arch = "wasm32")]
    let default_plugins = default_plugins.set(AssetPlugin {
        // HTTP readers cannot cheaply test for optional `.meta` sidecars.
        // This project does not use them, so skip the otherwise noisy 404s.
        meta_check: AssetMetaCheck::Never,
        ..default()
    });

    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb(0.015, 0.025, 0.045)))
        .add_plugins(default_plugins)
        .add_plugins(game::GamePlugin);

    if let Ok(value) = std::env::var("ROGUE_COMMANDER_SEED") {
        if let Some(seed) = game::parse_run_seed(&value) {
            app.insert_resource(game::RunSeed(seed));
            app.insert_resource(game::RunRng::from_seed(seed));
        } else {
            warn!("Ignoring invalid ROGUE_COMMANDER_SEED={value:?}");
        }
    }

    #[cfg(target_os = "android")]
    app.insert_resource(bevy::winit::WinitSettings::mobile());

    app
}

pub fn run() {
    build_app().run();
}

/// Exports the `android_main` symbol expected by Android GameActivity while
/// keeping the regular desktop binary entry point in `src/main.rs`.
#[bevy_main]
pub fn main() {
    run();
}
