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

// Versioned, tiny audio preference; malformed saves never partly overwrite defaults.
fn parse_audio(value: &str) -> Option<crate::game::AudioPreferences> {
    let mut fields = value.split_whitespace();
    if fields.next()? != "1" {
        return None;
    }
    let music = fields.next()?.parse::<u8>().ok()?;
    let effects = fields.next()?.parse::<u8>().ok()?;
    let muted = match fields.next()? {
        "0" => false,
        "1" => true,
        _ => return None,
    };
    if music > 100 || effects > 100 || fields.next().is_some() {
        return None;
    }
    Some(crate::game::AudioPreferences {
        music,
        effects,
        muted,
    })
}
fn encode_audio(value: &crate::game::AudioPreferences) -> Result<String, String> {
    if value.music > 100 || value.effects > 100 {
        return Err("Audio volume must be 0–100".into());
    }
    Ok(format!(
        "1 {} {} {}\n",
        value.music,
        value.effects,
        u8::from(value.muted)
    ))
}
#[cfg(not(target_arch = "wasm32"))]
fn audio_path() -> Option<std::path::PathBuf> {
    std::env::var_os("ROGUE_AUDIO_SAVE")
        .filter(|v| !v.is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| preference_path("audio"))
}
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn load_audio() -> Option<crate::game::AudioPreferences> {
    read_audio(&audio_path()?)
}
#[cfg(not(target_arch = "wasm32"))]
fn read_audio(path: &std::path::Path) -> Option<crate::game::AudioPreferences> {
    parse_audio(&std::fs::read_to_string(path).ok()?)
}
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn save_audio(value: &crate::game::AudioPreferences) -> Result<(), String> {
    write_audio(
        &audio_path().ok_or("No user configuration directory")?,
        value,
    )
}
#[cfg(not(target_arch = "wasm32"))]
fn write_audio(
    path: &std::path::Path,
    value: &crate::game::AudioPreferences,
) -> Result<(), String> {
    let data = encode_audio(value)?;
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let temporary = path.with_extension("audio.tmp");
    std::fs::write(&temporary, data).map_err(|e| e.to_string())?;
    if let Err(error) = std::fs::rename(&temporary, path) {
        let _ = std::fs::remove_file(temporary);
        return Err(error.to_string());
    }
    Ok(())
}
#[cfg(target_arch = "wasm32")]
pub(crate) fn load_audio() -> Option<crate::game::AudioPreferences> {
    let storage = web_sys::window()?.local_storage().ok()??;
    parse_audio(&storage.get_item("rogue-commander-audio").ok()??)
}
#[cfg(target_arch = "wasm32")]
pub(crate) fn save_audio(value: &crate::game::AudioPreferences) -> Result<(), String> {
    let data = encode_audio(value)?;
    let storage = web_sys::window()
        .ok_or("Browser window unavailable")?
        .local_storage()
        .map_err(|_| "Browser storage unavailable")?
        .ok_or("Browser storage unavailable")?;
    storage
        .set_item("rogue-commander-audio", &data)
        .map_err(|_| "Could not save audio preferences".into())
}
#[cfg(all(test, not(target_arch = "wasm32")))]
mod audio_tests {
    use super::*;
    #[test]
    fn audio_persistence_roundtrip_rejects_corruption_and_reports_write_failure() {
        let dir = std::env::temp_dir().join(format!("rogue-audio-test-{}", std::process::id()));
        let path = dir.join("nested/audio");
        let value = crate::game::AudioPreferences {
            music: 23,
            effects: 67,
            muted: true,
        };
        assert_eq!(read_audio(&path), None);
        write_audio(&path, &value).unwrap();
        assert_eq!(read_audio(&path), Some(value));
        for bad in [
            "",
            "1 101 50 0",
            "1 50 50 yes",
            "2 50 50 0",
            "1 50 50 0 extra",
        ] {
            std::fs::write(&path, bad).unwrap();
            assert_eq!(read_audio(&path), None);
        }
        assert!(write_audio(&dir, &value).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn tutorial_path() -> Option<std::path::PathBuf> {
    std::env::var_os("ROGUE_TUTORIAL_SAVE")
        .filter(|v| !v.is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| preference_path("tutorial-dismissed"))
}
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn load_tutorial_dismissed() -> bool {
    tutorial_path().is_some_and(|path| read_tutorial_dismissed(&path))
}
#[cfg(not(target_arch = "wasm32"))]
fn read_tutorial_dismissed(path: &std::path::Path) -> bool {
    std::fs::read_to_string(path).is_ok_and(|value| value.trim() == "1")
}
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn save_tutorial_dismissed() -> Result<(), String> {
    let path = tutorial_path().ok_or("No user configuration directory")?;
    write_tutorial_dismissed(&path).map_err(|error| error.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
fn write_tutorial_dismissed(path: &std::path::Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, "1")?;
    if let Err(error) = std::fs::rename(&temporary, path) {
        let _ = std::fs::remove_file(temporary);
        return Err(error);
    }
    Ok(())
}
#[cfg(target_arch = "wasm32")]
pub(crate) fn load_tutorial_dismissed() -> bool {
    web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| {
            storage
                .get_item("rogue-commander-tutorial-dismissed")
                .ok()
                .flatten()
        })
        .is_some_and(|value| value == "1")
}
#[cfg(target_arch = "wasm32")]
pub(crate) fn save_tutorial_dismissed() -> Result<(), String> {
    let storage = web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .ok_or("Browser storage unavailable")?;
    storage
        .set_item("rogue-commander-tutorial-dismissed", "1")
        .map_err(|_| "Could not save tutorial preference".into())
}
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tutorial_preference_tests {
    use super::*;
    #[test]
    fn dismissal_survives_disk_reload_and_invalid_values_repeat_the_guide() {
        let directory = std::env::temp_dir().join(format!(
            "rogue-tutorial-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = directory.join("nested/tutorial-dismissed");
        assert!(!read_tutorial_dismissed(&path));
        write_tutorial_dismissed(&path).unwrap();
        assert!(read_tutorial_dismissed(&path));
        assert!(!path.with_extension("tmp").exists());
        std::fs::write(&path, "invalid").unwrap();
        assert!(!read_tutorial_dismissed(&path));
        assert!(write_tutorial_dismissed(&directory).is_err());
        std::fs::remove_dir_all(directory).unwrap();
    }
}
