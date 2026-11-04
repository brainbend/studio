//! The projects opened or saved lately, for File ▸ Open. Kept in `~/.brainbend-studio/recent.json`, newest first.
//!
//! Only saved `.brainbend` files are listed: an imported `.story` or `.pptx` has no file of its own until it is saved.
use std::path::{Path, PathBuf};

/// How many projects the list keeps.
pub const MAX_RECENT: usize = 10;

#[derive(Clone, Debug, PartialEq)]
pub struct RecentProject {
    pub path: PathBuf,
    /// When it was last opened or saved, in seconds since 1970.
    pub opened: u64,
}

/// Where the list lives, or `None` when there is no home folder.
pub fn list_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join(".brainbend-studio").join("recent.json"))
}

pub fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// Reads the list; a missing or damaged file is an empty list.
pub fn load_from(file: &Path) -> Vec<RecentProject> {
    let Ok(text) = std::fs::read_to_string(file) else { return vec![] };
    let Ok(serde_json::Value::Array(items)) = serde_json::from_str::<serde_json::Value>(&text) else { return vec![] };
    let mut out: Vec<RecentProject> = items.iter().filter_map(|v| {
        let path = v.get("path")?.as_str()?;
        Some(RecentProject { path: PathBuf::from(path), opened: v.get("opened").and_then(|o| o.as_u64()).unwrap_or(0) })
    }).collect();
    out.truncate(MAX_RECENT);
    out
}

/// Writes the list. A failure is ignored: losing the list is a nuisance, not a reason to interrupt saving a project.
pub fn save_to(file: &Path, list: &[RecentProject]) {
    let items: Vec<serde_json::Value> = list.iter().map(|r| serde_json::json!({ "path": r.path.to_string_lossy(), "opened": r.opened })).collect();
    if let Some(dir) = file.parent() { let _ = std::fs::create_dir_all(dir); }
    if let Ok(text) = serde_json::to_string_pretty(&items) { let _ = std::fs::write(file, text); }
}

/// Puts `path` at the top of the list (once), stamped `now`, and trims the list to `MAX_RECENT`.
pub fn touch(list: &mut Vec<RecentProject>, path: &Path, now: u64) {
    list.retain(|r| r.path != path);
    list.insert(0, RecentProject { path: path.to_path_buf(), opened: now });
    list.truncate(MAX_RECENT);
}

/// Takes `path` off the list.
pub fn forget(list: &mut Vec<RecentProject>, path: &Path) {
    list.retain(|r| r.path != path);
}

/// How long ago, in words: "Just now", "5 minutes ago", "Yesterday", "3 weeks ago".
pub fn ago(now: u64, then: u64) -> String {
    if then == 0 { return String::new(); }
    let s = now.saturating_sub(then);
    let plural = |n: u64, unit: &str| if n == 1 { format!("1 {unit} ago") } else { format!("{n} {unit}s ago") };
    match s {
        0..=59 => "Just now".into(),
        60..=3599 => plural(s / 60, "minute"),
        3600..=86_399 => plural(s / 3600, "hour"),
        86_400..=172_799 => "Yesterday".into(),
        172_800..=1_209_599 => plural(s / 86_400, "day"),
        1_209_600..=5_183_999 => plural(s / 604_800, "week"),
        _ => plural(s / 2_592_000, "month"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf { PathBuf::from(s) }

    #[test]
    fn the_newest_project_comes_first_and_appears_once() {
        let mut l = vec![];
        touch(&mut l, &p("/a.brainbend"), 10);
        touch(&mut l, &p("/b.brainbend"), 20);
        touch(&mut l, &p("/a.brainbend"), 30);
        assert_eq!(l, vec![RecentProject { path: p("/a.brainbend"), opened: 30 }, RecentProject { path: p("/b.brainbend"), opened: 20 }]);
    }

    #[test]
    fn the_list_keeps_only_the_newest_ten() {
        let mut l = vec![];
        for i in 0..15u64 { touch(&mut l, &p(&format!("/{i}.brainbend")), i); }
        assert_eq!(l.len(), MAX_RECENT);
        assert_eq!(l[0].path, p("/14.brainbend"));
        assert_eq!(l[MAX_RECENT - 1].path, p("/5.brainbend"));
    }

    #[test]
    fn forgetting_a_project_takes_it_off() {
        let mut l = vec![];
        touch(&mut l, &p("/a.brainbend"), 1);
        touch(&mut l, &p("/b.brainbend"), 2);
        forget(&mut l, &p("/a.brainbend"));
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].path, p("/b.brainbend"));
    }

    #[test]
    fn the_list_survives_a_round_trip_and_a_damaged_file_is_empty() {
        let dir = std::env::temp_dir().join(format!("bb-recent-{}", std::process::id()));
        let file = dir.join("sub").join("recent.json");
        let mut l = vec![];
        touch(&mut l, &p("/My Course.brainbend"), 5);
        touch(&mut l, &p("/Other.brainbend"), 9);
        save_to(&file, &l);
        assert_eq!(load_from(&file), l);
        std::fs::write(&file, "not json {").unwrap();
        assert!(load_from(&file).is_empty());
        assert!(load_from(&dir.join("missing.json")).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ago_reads_in_plain_words() {
        let now = 10_000_000;
        assert_eq!(ago(now, now - 5), "Just now");
        assert_eq!(ago(now, now - 60), "1 minute ago");
        assert_eq!(ago(now, now - 7200), "2 hours ago");
        assert_eq!(ago(now, now - 90_000), "Yesterday");
        assert_eq!(ago(now, now - 3 * 86_400), "3 days ago");
        assert_eq!(ago(now, now - 15 * 86_400), "2 weeks ago");
        assert_eq!(ago(now, now - 90 * 86_400), "3 months ago");
        assert_eq!(ago(now, 0), "");
    }
}
