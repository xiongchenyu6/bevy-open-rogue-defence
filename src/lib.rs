#[cfg(target_arch = "wasm32")]
use bevy::asset::{AssetMetaCheck, AssetPlugin};
use bevy::prelude::*;

pub mod game;
mod preferences;

pub fn build_app() -> App {
    let run_seed = initial_run_seed();
    let language = initial_game_language();
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
        .insert_resource(game::RunSeed::new(run_seed))
        .insert_resource(game::RunRng::from_seed(run_seed))
        .insert_resource(language)
        .add_plugins(default_plugins)
        .add_plugins(game::GamePlugin);

    #[cfg(target_os = "android")]
    app.insert_resource(bevy::winit::WinitSettings::mobile());

    app
}

#[cfg(not(target_arch = "wasm32"))]
fn initial_game_language() -> game::GameLanguage {
    if let Ok(value) = std::env::var("ROGUE_COMMANDER_LANG") {
        if let Some(language) = game::GameLanguage::parse(&value) {
            return language;
        }
        warn!("Ignoring invalid ROGUE_COMMANDER_LANG={value:?}");
    }

    if let Some(language) = preferences::load_language() {
        return language;
    }

    #[cfg(target_os = "android")]
    return game::GameLanguage::SimplifiedChinese;

    #[cfg(not(target_os = "android"))]
    game::GameLanguage::English
}

#[cfg(target_arch = "wasm32")]
fn initial_game_language() -> game::GameLanguage {
    let search = web_sys::window()
        .and_then(|window| window.location().search().ok())
        .unwrap_or_default();
    language_from_search(&search)
        .or_else(preferences::load_language)
        .unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
fn initial_run_seed() -> u64 {
    if let Ok(value) = std::env::var("ROGUE_COMMANDER_SEED") {
        if let Some(seed) = game::parse_run_seed(&value) {
            return seed;
        }
        warn!("Ignoring invalid ROGUE_COMMANDER_SEED={value:?}");
    }

    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(game::DEFAULT_RUN_SEED);
    mix_seed(timestamp ^ u64::from(std::process::id()).rotate_left(19))
}

#[cfg(target_arch = "wasm32")]
fn initial_run_seed() -> u64 {
    let search = web_sys::window()
        .and_then(|window| window.location().search().ok())
        .unwrap_or_default();
    if let Some(seed) = seed_from_search(&search) {
        return seed;
    }

    let timestamp = js_sys::Date::now().to_bits();
    let entropy = js_sys::Math::random().to_bits().rotate_left(23);
    mix_seed(timestamp ^ entropy)
}

fn mix_seed(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    game::RunSeed::new(value ^ (value >> 31)).0
}

#[cfg(any(target_arch = "wasm32", test))]
fn seed_from_search(search: &str) -> Option<u64> {
    search
        .trim_start_matches('?')
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find_map(|(key, value)| {
            (key == "seed")
                .then(|| game::parse_run_seed(value))
                .flatten()
        })
}

#[cfg(any(target_arch = "wasm32", test))]
fn language_from_search(search: &str) -> Option<game::GameLanguage> {
    search
        .trim_start_matches('?')
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find_map(|(key, value)| {
            (key == "lang")
                .then(|| game::GameLanguage::parse(value))
                .flatten()
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_query_is_shareable_and_ignores_unrelated_parameters() {
        assert_eq!(seed_from_search("?seed=0xA11CE"), Some(0xA11CE));
        assert_eq!(seed_from_search("?foo=1&seed=659918&bar=2"), Some(659_918));
        assert_eq!(seed_from_search("?seed=invalid"), None);
        assert_eq!(seed_from_search("?foo=1"), None);
    }

    #[test]
    fn entropy_mixer_is_stable_and_never_returns_zero() {
        assert_eq!(mix_seed(42), mix_seed(42));
        assert_ne!(mix_seed(42), mix_seed(43));
        assert_ne!(mix_seed(0), 0);
    }

    #[test]
    fn language_query_accepts_supported_values_and_ignores_other_parameters() {
        assert_eq!(
            language_from_search("?seed=7&lang=zh-CN"),
            Some(game::GameLanguage::SimplifiedChinese)
        );
        assert_eq!(
            language_from_search("?lang=en&seed=7"),
            Some(game::GameLanguage::English)
        );
        assert_eq!(language_from_search("?lang=fr"), None);
    }
}
