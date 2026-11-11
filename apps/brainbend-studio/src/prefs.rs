//! The few preferences kept in `~/.brainbend-studio/settings.json`, shared by the parts of the app that have one.

use std::path::PathBuf;

fn home_dir() -> Option<PathBuf> { std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from) }

/// Tests never read or write the real settings file.
pub fn path() -> Option<PathBuf> { if cfg!(test) { None } else { Some(home_dir()?.join(".brainbend-studio").join("settings.json")) } }

fn read() -> serde_json::Map<String, serde_json::Value> {
    path().and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

pub fn get_bool(key: &str) -> Option<bool> { read().get(key).and_then(|v| v.as_bool()) }

/// Saves one preference and keeps the others as they are.
pub fn set_bool(key: &str, value: bool) {
    let Some(path) = path() else { return };
    let mut all = read();
    all.insert(key.into(), value.into());
    if let Some(dir) = path.parent() { let _ = std::fs::create_dir_all(dir); }
    let _ = std::fs::write(path, serde_json::Value::Object(all).to_string());
}

/// Whether the desktop is a tiling window manager (Hyprland, sway, i3 and the like), which would tile every window the app opens.
pub fn tiling_session() -> bool {
    if !cfg!(target_os = "linux") { return false; }
    ["HYPRLAND_INSTANCE_SIGNATURE", "SWAYSOCK", "I3SOCK", "NIRI_SOCKET"].iter().any(|k| std::env::var_os(k).is_some())
        || ["XDG_CURRENT_DESKTOP", "XDG_SESSION_DESKTOP", "DESKTOP_SESSION"].iter().filter_map(|k| std::env::var(k).ok()).any(|d| is_tiling_name(&d))
}

fn is_tiling_name(desktop: &str) -> bool {
    let d = desktop.to_ascii_lowercase();
    ["hyprland", "sway", "i3", "niri", "river", "bspwm", "dwm", "awesome", "qtile", "xmonad", "herbstluftwm", "leftwm", "wayfire"].iter().any(|n| d.split([':', ' ']).any(|part| part == *n))
}

/// Tool windows (settings, spelling, search...) open inside the app instead of as windows of their own. The choice made
/// in the app wins; with none made, a tiling window manager gets them inside, where it cannot tile them.
pub fn tool_windows_inside() -> bool { get_bool("tool_windows_inside").unwrap_or_else(tiling_session) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tiling_desktops_are_recognised_by_name() {
        assert!(is_tiling_name("Hyprland") && is_tiling_name("sway") && is_tiling_name("ubuntu:i3"));
        assert!(!is_tiling_name("GNOME") && !is_tiling_name("KDE") && !is_tiling_name("X-Cinnamon"));
    }
}
