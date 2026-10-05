//! One persisted player preference, shared by the launcher and in-game settings.
use crate::game::GameLanguage;

#[cfg(not(target_arch = "wasm32"))]
fn preference_path(name: &str) -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    let nonempty = |key| {
        std::env::var_os(key)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    };
    let base = nonempty("XDG_CONFIG_HOME").or_else(|| {
        if cfg!(target_os = "windows") {
            nonempty("APPDATA")
        } else if cfg!(target_os = "macos") {
            nonempty("HOME").map(|home| home.join("Library/Application Support"))
        } else {
            nonempty("HOME").map(|home| home.join(".config"))
        }
    })?;
    Some(base.join("rogue-commander").join(name))
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn load_language() -> Option<GameLanguage> {
    read_language(&preference_path("language")?)
}

#[cfg(not(target_arch = "wasm32"))]
fn read_language(path: &std::path::Path) -> Option<GameLanguage> {
    GameLanguage::parse(std::fs::read_to_string(path).ok()?.trim())
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn save_language(language: GameLanguage) -> Result<(), String> {
    let path = preference_path("language").ok_or("No user configuration directory")?;
    write_language(&path, language).map_err(|error| error.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn write_language(path: &std::path::Path, language: GameLanguage) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // The preference is tiny; write then rename so interruption preserves the old value.
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, language.query_code())?;
    if let Err(error) = std::fs::rename(&temporary, path) {
        let _ = std::fs::remove_file(temporary);
        return Err(error);
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn load_language() -> Option<GameLanguage> {
    let storage = web_sys::window()?.local_storage().ok()??;
    GameLanguage::parse(&storage.get_item("rogue-commander-language").ok()??)
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn save_language(language: GameLanguage) -> Result<(), String> {
    use js_sys::wasm_bindgen::{JsCast, JsValue};
    let window = web_sys::window().ok_or("Browser window unavailable")?;
    // Keep the shared URL's other parameters (especially its run seed) intact.
    if let Ok(url) = window
        .location()
        .href()
        .and_then(|href| web_sys::Url::new(&href))
    {
        url.search_params().set("lang", language.query_code());
        if let Ok(history) = window.history() {
            let _ = history.replace_state_with_url(&JsValue::NULL, "", Some(&url.href()));
        }
    }
    // The web shell owns its loading/rotation/accessibility copy; update it without reload.
    if let Ok(callback) = js_sys::Reflect::get(window.as_ref(), &"rogueCommanderLanguage".into())
        && let Some(callback) = callback.dyn_ref::<js_sys::Function>()
    {
        let _ = callback.call1(window.as_ref(), &language.query_code().into());
    }
    let storage = window
        .local_storage()
        .map_err(|_| "Browser storage unavailable")?
        .ok_or("Browser storage unavailable")?;
    storage
        .set_item("rogue-commander-language", language.query_code())
        .map_err(|_| "Could not save language preference".into())
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    #[test]
    fn preferences_survive_reload_and_reject_invalid_values() {
        let dir =
            std::env::temp_dir().join(format!("rogue-preference-test-{}", std::process::id()));
        let path = dir.join("nested/language");
        assert_eq!(read_language(&path), None);
        write_language(&path, GameLanguage::SimplifiedChinese).unwrap();
        assert_eq!(read_language(&path), Some(GameLanguage::SimplifiedChinese));
        write_language(&path, GameLanguage::English).unwrap();
        assert_eq!(read_language(&path), Some(GameLanguage::English));
        std::fs::write(&path, "invalid").unwrap();
        assert_eq!(read_language(&path), None);
        assert!(write_language(&dir, GameLanguage::English).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn selection_detail_path() -> Option<std::path::PathBuf> {
    std::env::var_os("ROGUE_DETAIL_SAVE")
        .filter(|value| !value.is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| preference_path("selection-detail"))
}
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn load_selection_detail() -> Option<bool> {
    parse_detail(
        std::fs::read_to_string(selection_detail_path()?)
            .ok()?
            .trim(),
    )
}
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn save_selection_detail(detailed: bool) -> Result<(), String> {
    write_detail(
        &selection_detail_path().ok_or("No user configuration directory")?,
        detailed,
    )
    .map_err(|e| e.to_string())
}
fn parse_detail(value: &str) -> Option<bool> {
    match value {
        "detailed" => Some(true),
        "simple" => Some(false),
        _ => None,
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn write_detail(path: &std::path::Path, detailed: bool) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, if detailed { "detailed" } else { "simple" })?;
    if let Err(error) = std::fs::rename(&temporary, path) {
        let _ = std::fs::remove_file(temporary);
        return Err(error);
    }
    Ok(())
}
#[cfg(target_arch = "wasm32")]
pub(crate) fn load_selection_detail() -> Option<bool> {
    let storage = web_sys::window()?.local_storage().ok()??;
    parse_detail(
        &storage
            .get_item("rogue-commander-selection-detail")
            .ok()??,
    )
}
#[cfg(target_arch = "wasm32")]
pub(crate) fn save_selection_detail(detailed: bool) -> Result<(), String> {
    let storage = web_sys::window()
        .ok_or("Browser window unavailable")?
        .local_storage()
        .map_err(|_| "Browser storage unavailable")?
        .ok_or("Browser storage unavailable")?;
    storage
        .set_item(
            "rogue-commander-selection-detail",
            if detailed { "detailed" } else { "simple" },
        )
        .map_err(|_| "Could not save selection detail preference".into())
}
#[cfg(all(test, not(target_arch = "wasm32")))]
mod detail_tests {
    use super::*;
    #[test]
    fn selection_mode_survives_reload_and_invalid_value_defaults_safely() {
        let dir = std::env::temp_dir().join(format!("rogue-detail-test-{}", std::process::id()));
        let path = dir.join("selection-detail");
        write_detail(&path, false).unwrap();
        assert_eq!(
            parse_detail(std::fs::read_to_string(&path).unwrap().trim()),
            Some(false)
        );
        write_detail(&path, true).unwrap();
        assert_eq!(
            parse_detail(std::fs::read_to_string(&path).unwrap().trim()),
            Some(true)
        );
        assert_eq!(parse_detail("garbage"), None);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
