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

pub fn get_str(key: &str) -> Option<String> { read().get(key).and_then(|v| v.as_str()).map(str::to_string) }

/// Saves a text preference and keeps the others as they are.
pub fn set_str(key: &str, value: &str) {
    let Some(path) = path() else { return };
    let mut all = read();
    all.insert(key.into(), value.into());
    if let Some(dir) = path.parent() { let _ = std::fs::create_dir_all(dir); }
    let _ = std::fs::write(path, serde_json::Value::Object(all).to_string());
}

/// Saves one preference and keeps the others as they are.
pub fn set_bool(key: &str, value: bool) {
    let Some(path) = path() else { return };
    let mut all = read();
    all.insert(key.into(), value.into());
    if let Some(dir) = path.parent() { let _ = std::fs::create_dir_all(dir); }
    let _ = std::fs::write(path, serde_json::Value::Object(all).to_string());
}

/// Which panels of the editor show. Each person decides (it follows them, not the project): someone who never uses animations
/// hides the timeline, someone who never uses AI hides its tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Panels { pub properties: bool, pub timeline: bool, pub notes: bool, pub states: bool, pub triggers: bool, pub ai: bool, pub layers: bool }

impl Panels {
    pub fn load() -> Panels {
        let on = |k: &str| get_bool(k).unwrap_or(true);
        Panels { properties: on("show_properties"), timeline: on("show_timeline"), notes: on("show_notes"), states: on("show_states"), triggers: on("show_triggers"), ai: on("show_ai"), layers: on("show_layers") }
    }
    /// The first of the bottom panel's tabs (0 Timeline, 1 Slide Notes, 2 States) that shows, or `None` when all are hidden.
    pub fn bottom_tabs(&self) -> Vec<usize> { [(0, self.timeline), (1, self.notes), (2, self.states)].into_iter().filter(|t| t.1).map(|t| t.0).collect() }
    /// The side panel's tabs (0 Properties, 1 Triggers, 2 AI) that show.
    pub fn side_tabs(&self) -> Vec<usize> { [(0, self.properties), (1, self.triggers), (2, self.ai)].into_iter().filter(|t| t.1).map(|t| t.0).collect() }
}
