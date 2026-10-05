//! Native window preference, separate from simulation and language saves.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn path() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    let read = |key| {
        std::env::var_os(key)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    if let Some(path) = read("ROGUE_DISPLAY_SAVE") {
        return Some(path);
    }
    let base = read("XDG_CONFIG_HOME").or_else(|| {
        if cfg!(target_os = "windows") {
            read("APPDATA")
        } else if cfg!(target_os = "macos") {
            read("HOME").map(|p| p.join("Library/Application Support"))
        } else {
            read("HOME").map(|p| p.join(".config"))
        }
    })?;
    Some(base.join("rogue-commander/display"))
}
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn load() -> Option<bool> {
    parse(&std::fs::read_to_string(path()?).ok()?)
}
#[cfg(target_arch = "wasm32")]
pub(super) fn load() -> Option<bool> {
    None
}
fn parse(value: &str) -> Option<bool> {
    match value.trim() {
        "windowed" => Some(false),
        "fullscreen" => Some(true),
        _ => None,
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn save_at(path: &std::path::Path, fullscreen: bool) -> Result<(), String> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let temp = path.with_extension("display.tmp");
    std::fs::write(
        &temp,
        if fullscreen {
            "fullscreen\n"
        } else {
            "windowed\n"
        },
    )
    .map_err(|e| e.to_string())?;
    if let Err(e) = std::fs::rename(&temp, path) {
        let _ = std::fs::remove_file(temp);
        return Err(e.to_string());
    }
    Ok(())
}
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    #[test]
    fn native_display_save_roundtrips_and_rejects_corruption() {
        let dir = std::env::temp_dir().join(format!("rogue-display-pref-{}", std::process::id()));
        let path = dir.join("settings");
        save_at(&path, true).unwrap();
        assert_eq!(parse(&std::fs::read_to_string(&path).unwrap()), Some(true));
        save_at(&path, false).unwrap();
        assert_eq!(parse(&std::fs::read_to_string(&path).unwrap()), Some(false));
        assert_eq!(parse("invalid"), None);
        assert!(save_at(&dir, true).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
