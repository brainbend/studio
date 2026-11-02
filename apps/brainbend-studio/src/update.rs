//! Update check: asks GitHub for the newest published release and tells the author when it is newer than this build.
//! Phase 1 only notifies; the notice links to the release page.
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::time::Duration;

const LATEST_URL: &str = "https://api.github.com/repos/brainbend/studio/releases/latest";

/// A published release.
#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    pub tag: String,
    pub version: semver::Version,
    pub notes: String,
    pub url: String,
}

/// Reads GitHub's "latest release" reply. The tag is `v<version>`.
pub fn parse_release(json: &str) -> Result<Release, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("unreadable reply: {e}"))?;
    let tag = v["tag_name"].as_str().ok_or("the reply has no tag")?.to_string();
    let version = semver::Version::parse(tag.trim_start_matches('v')).map_err(|e| format!("odd release tag {tag}: {e}"))?;
    let url = v["html_url"].as_str().unwrap_or("https://github.com/brainbend/studio/releases").to_string();
    let notes = v["body"].as_str().unwrap_or("").to_string();
    Ok(Release { tag, version, notes, url })
}

/// True when `latest` is a newer version than `current` (a release candidate counts as older than its release).
pub fn is_newer(current: &str, latest: &semver::Version) -> bool {
    semver::Version::parse(current).map(|c| latest > &c).unwrap_or(false)
}

/// Asks GitHub; `Ok(None)` means this build is current. No release published yet also counts as current.
pub fn check(current: &str) -> Result<Option<Release>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .http_status_as_error(false)
        .build()
        .into();
    let mut reply = agent
        .get(LATEST_URL)
        .header("User-Agent", concat!("BrainBend-Studio/", env!("CARGO_PKG_VERSION")))
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("could not reach GitHub: {e}"))?;
    match reply.status().as_u16() {
        404 => return Ok(None),
        200 => {}
        code => return Err(format!("GitHub answered {code}")),
    }
    let body = reply.body_mut().read_to_string().map_err(|e| format!("could not read the reply: {e}"))?;
    let release = parse_release(&body)?;
    Ok(is_newer(current, &release.version).then_some(release))
}

pub enum State {
    Idle,
    Checking(Receiver<Result<Option<Release>, String>>),
    UpToDate,
    Available(Release),
    Failed(String),
}

pub struct Updater {
    pub state: State,
    /// Check once when the app opens.
    pub on_launch: bool,
}

impl Default for Updater {
    fn default() -> Self { Self { state: State::Idle, on_launch: true } }
}

fn settings_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join(".brainbend-studio").join("settings.json"))
}

impl Updater {
    /// The saved preference, with the check on when nothing is saved.
    pub fn load() -> Self {
        let on_launch = settings_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| v["check_for_updates"].as_bool())
            .unwrap_or(true);
        Self { state: State::Idle, on_launch }
    }

    pub fn save(&self) {
        let Some(path) = settings_path() else { return };
        if let Some(dir) = path.parent() { let _ = std::fs::create_dir_all(dir); }
        let _ = std::fs::write(path, serde_json::json!({ "check_for_updates": self.on_launch }).to_string());
    }

    /// Starts a check in the background; does nothing if one is already running.
    pub fn start(&mut self, ctx: &eframe::egui::Context) {
        if matches!(self.state, State::Checking(_)) { return; }
        let (tx, rx) = channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(check(env!("CARGO_PKG_VERSION")));
            ctx.request_repaint();
        });
        self.state = State::Checking(rx);
    }

    /// Picks up the result of a running check. Call once a frame.
    pub fn poll(&mut self) {
        let State::Checking(rx) = &self.state else { return };
        self.state = match rx.try_recv() {
            Ok(Ok(Some(release))) => State::Available(release),
            Ok(Ok(None)) => State::UpToDate,
            Ok(Err(e)) => State::Failed(e),
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => State::Failed("the check stopped unexpectedly".into()),
        };
    }

    pub fn available(&self) -> Option<&Release> {
        match &self.state { State::Available(r) => Some(r), _ => None }
    }
}

/// Opens a web page in the default browser.
pub fn open_in_browser(url: &str) {
    let opener = if cfg!(target_os = "macos") { "open" } else if cfg!(target_os = "windows") { "explorer" } else { "xdg-open" };
    let _ = std::process::Command::new(opener).arg(url).spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPLY: &str = r###"{"tag_name":"v0.2.0","html_url":"https://github.com/brainbend/studio/releases/tag/v0.2.0","body":"## What's new\n- Updates"}"###;

    #[test]
    fn reads_the_latest_release_reply() {
        let r = parse_release(REPLY).unwrap();
        assert_eq!(r.tag, "v0.2.0");
        assert_eq!(r.version, semver::Version::new(0, 2, 0));
        assert!(r.url.ends_with("/tag/v0.2.0"));
        assert!(r.notes.contains("What's new"));
    }

    #[test]
    fn rejects_replies_without_a_usable_tag() {
        assert!(parse_release("{}").is_err());
        assert!(parse_release(r#"{"tag_name":"nightly"}"#).is_err());
        assert!(parse_release("not json").is_err());
    }

    #[test]
    fn compares_versions_by_semver() {
        let v = |s: &str| semver::Version::parse(s).unwrap();
        assert!(is_newer("0.1.0", &v("0.2.0")));
        assert!(is_newer("0.9.0", &v("0.10.0")), "numeric, not alphabetical");
        assert!(!is_newer("0.2.0", &v("0.2.0")));
        assert!(!is_newer("0.3.0", &v("0.2.0")));
        assert!(is_newer("0.2.0-rc.1", &v("0.2.0")), "a release beats its candidate");
        assert!(!is_newer("0.2.0", &v("0.2.0-rc.1")));
        assert!(!is_newer("garbage", &v("0.2.0")));
    }
}
