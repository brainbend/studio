//! Updates: asks GitHub for the newest published release, and when it is newer than this build downloads it,
//! checks its signature, and swaps it in for the running app.
//!
//! Releases are signed in CI with a private key kept as a GitHub secret. The matching public key (`update-key.pub`) is
//! built into the app, which refuses any download whose signature does not verify. That is what keeps a hijacked
//! GitHub account from pushing a malicious update: the checksum or the file alone would prove nothing.
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;
use std::time::Duration;

use eframe::egui;

const LATEST_URL: &str = "https://api.github.com/repos/brainbend/studio/releases/latest";
const DOWNLOAD_PREFIX: &str = "https://github.com/brainbend/studio/releases/download/";
const PUBLIC_KEY: &str = include_str!("../update-key.pub");
const MAX_DOWNLOAD: u64 = 400 * 1024 * 1024;

/// One file attached to a release.
#[derive(Clone, Debug, PartialEq)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
}

/// A published release.
#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    pub tag: String,
    pub version: semver::Version,
    pub notes: String,
    pub url: String,
    pub assets: Vec<Asset>,
}

/// Reads GitHub's "latest release" reply. The tag is `v<version>`.
pub fn parse_release(json: &str) -> Result<Release, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("unreadable reply: {e}"))?;
    let tag = v["tag_name"].as_str().ok_or("the reply has no tag")?.to_string();
    let version = semver::Version::parse(tag.trim_start_matches('v')).map_err(|e| format!("odd release tag {tag}: {e}"))?;
    let url = v["html_url"].as_str().unwrap_or("https://github.com/brainbend/studio/releases").to_string();
    let notes = v["body"].as_str().unwrap_or("").to_string();
    let assets = v["assets"]
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(|a| Some(Asset { name: a["name"].as_str()?.to_string(), url: a["browser_download_url"].as_str()?.to_string(), size: a["size"].as_u64().unwrap_or(0) }))
                .collect()
        })
        .unwrap_or_default();
    Ok(Release { tag, version, notes, url, assets })
}

/// True when `latest` is a newer version than `current` (a release candidate counts as older than its release).
pub fn is_newer(current: &str, latest: &semver::Version) -> bool {
    semver::Version::parse(current).map(|c| latest > &c).unwrap_or(false)
}

/// GitHub's release notes as plain text for the update window: no HTML comments, markdown markers or the
/// "Full Changelog" link line, headings as plain lines, and bullets as dots.
pub fn clean_notes(raw: &str) -> String {
    let mut text = String::new();
    let mut rest = raw;
    while let Some(start) = rest.find("<!--") {
        text.push_str(&rest[..start]);
        match rest[start..].find("-->") {
            Some(end) => rest = &rest[start + end + 3..],
            None => { rest = ""; break; }
        }
    }
    text.push_str(rest);
    let mut lines: Vec<String> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.to_lowercase().replace('*', "").trim_start().starts_with("full changelog") { continue; }
        let line = line.trim_start_matches('#').trim_start();
        let line = match line.strip_prefix("* ").or_else(|| line.strip_prefix("- ")) { Some(item) => format!("\u{2022} {item}"), None => line.to_string() };
        let line = line.replace("**", "");
        if line.is_empty() && lines.last().is_none_or(|l| l.is_empty()) { continue; }
        lines.push(line);
    }
    while lines.last().is_some_and(|l| l.is_empty()) { lines.pop(); }
    lines.join("\n")
}

/// How this copy of the app was put on the computer, which decides what file updates it and how that file is installed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// The macOS app, or an unpacked zip or tar.gz: the program replaces itself.
    Portable,
    /// The Windows installer: the new .msi is run.
    Msi,
    /// A Debian or Ubuntu package: the new .deb is installed with the system's own permission prompt.
    Deb,
    /// A Fedora or Red Hat package: the new .rpm is installed the same way.
    Rpm,
    /// An AppImage: the new AppImage takes its place.
    AppImage,
}

impl Kind {
    /// What the update window says about how this kind installs.
    pub fn note(self) -> Option<&'static str> {
        match self {
            Kind::Portable => None,
            Kind::Msi => Some("Windows will ask for permission to run the installer."),
            Kind::Deb | Kind::Rpm => Some("Your system will ask for permission to install the package."),
            Kind::AppImage => Some("The new AppImage takes the place of this one."),
        }
    }
}

/// What kind of install the running program is. `appimage` is the APPIMAGE variable an AppImage sets, `program_files` the
/// folders the Windows installer uses, and `packages` says which of dpkg and rpm this Linux system has.
pub fn detect_kind(exe: &Path, appimage: Option<&std::ffi::OsStr>, program_files: &[PathBuf], packages: (bool, bool)) -> Kind {
    if cfg!(target_os = "linux") {
        if appimage.is_some_and(|a| !a.is_empty()) { return Kind::AppImage; }
        if exe.starts_with("/usr/bin") {
            if packages.0 { return Kind::Deb; }
            if packages.1 { return Kind::Rpm; }
        }
    }
    if cfg!(target_os = "windows") && program_files.iter().any(|d| !d.as_os_str().is_empty() && exe.starts_with(d)) { return Kind::Msi; }
    Kind::Portable
}

/// `detect_kind` for the running app.
pub fn kind() -> Kind {
    let Ok(exe) = std::env::current_exe() else { return Kind::Portable };
    let folders: Vec<PathBuf> = ["ProgramFiles", "ProgramFiles(Arm)", "ProgramW6432"].iter().filter_map(std::env::var_os).map(PathBuf::from).collect();
    detect_kind(&exe, std::env::var_os("APPIMAGE").as_deref(), &folders, (Path::new("/usr/bin/dpkg").exists(), Path::new("/usr/bin/rpm").exists()))
}

/// The ending of the file name that holds this kind of install's update.
pub fn suffix_for(kind: Kind) -> Option<&'static str> {
    let arm = cfg!(target_arch = "aarch64");
    if cfg!(target_os = "macos") {
        return Some("-macos-universal.zip");
    }
    if cfg!(target_os = "windows") {
        return Some(match (kind, arm) {
            (Kind::Msi, false) => "-windows.msi",
            (Kind::Msi, true) => "-windows-arm64.msi",
            (_, false) => "-windows.zip",
            (_, true) => "-windows-arm64.zip",
        });
    }
    if cfg!(target_os = "linux") {
        return Some(match (kind, arm) {
            (Kind::Deb, false) => "_amd64.deb",
            (Kind::Deb, true) => "_arm64.deb",
            (Kind::Rpm, false) => ".x86_64.rpm",
            (Kind::Rpm, true) => ".aarch64.rpm",
            (Kind::AppImage, false) => "-x86_64.AppImage",
            (Kind::AppImage, true) => "-aarch64.AppImage",
            (_, false) => "-linux.tar.gz",
            (_, true) => "-linux-aarch64.tar.gz",
        });
    }
    None
}

/// The ending of the file name that holds this system's update.
pub fn platform_suffix() -> Option<&'static str> { suffix_for(kind()) }

/// The download for a system and the signature that goes with it, if the release has both.
pub fn pick_assets<'a>(release: &'a Release, suffix: &str) -> Option<(&'a Asset, &'a Asset)> {
    let file = release.assets.iter().find(|a| a.name.ends_with(suffix))?;
    let sig_name = format!("{}.minisig", file.name);
    let sig = release.assets.iter().find(|a| a.name == sig_name)?;
    Some((file, sig))
}

/// Can this release be installed from inside the app (rather than only downloaded by hand)?
pub fn installable(release: &Release) -> bool {
    platform_suffix().is_some_and(|s| pick_assets(release, s).is_some())
}

// Debug builds can point at a local test server and a test key; release builds always use GitHub and the built-in key.
#[cfg(debug_assertions)]
fn test_override(name: &str) -> Option<String> { std::env::var(name).ok().filter(|v| !v.is_empty()) }
#[cfg(not(debug_assertions))]
fn test_override(_name: &str) -> Option<String> { None }

fn latest_url() -> String { test_override("BRAINBEND_UPDATE_URL").unwrap_or_else(|| LATEST_URL.to_string()) }

fn public_key() -> Result<minisign_verify::PublicKey, String> {
    match test_override("BRAINBEND_UPDATE_PUBKEY") {
        Some(b64) => minisign_verify::PublicKey::from_base64(b64.trim()),
        None => minisign_verify::PublicKey::decode(PUBLIC_KEY),
    }
    .map_err(|e| format!("the app's update key is unreadable ({e})"))
}

/// Only this project's own release files are downloaded (a debug build pointed at a test server may fetch from it).
fn download_allowed(url: &str, allow_any: bool) -> bool { allow_any || url.starts_with(DOWNLOAD_PREFIX) }

/// Checks `data` against its signature file.
pub fn verify(data: &[u8], signature: &str, key: &minisign_verify::PublicKey) -> Result<(), String> {
    let sig = minisign_verify::Signature::decode(signature).map_err(|e| format!("the signature file is unreadable ({e})"))?;
    key.verify(data, &sig, false).map_err(|e| format!("the download's signature does not match, so it was not installed ({e})"))
}

fn agent(secs: u64) -> ureq::Agent {
    ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(secs))).build().into()
}

/// Asks GitHub; `Ok(None)` means this build is current. No release published yet also counts as current.
pub fn check(current: &str) -> Result<Option<Release>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .http_status_as_error(false)
        .build()
        .into();
    let mut reply = agent
        .get(&latest_url())
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

/// How far a download has got.
#[derive(Default)]
pub struct Progress {
    pub done: AtomicU64,
    pub total: AtomicU64,
}

fn fetch(url: &str, max: u64, progress: Option<&Progress>, allow_any: bool) -> Result<Vec<u8>, String> {
    if !download_allowed(url, allow_any) { return Err("the download address is not one of this project's release files".into()); }
    let mut reply = agent(900)
        .get(url)
        .header("User-Agent", concat!("BrainBend-Studio/", env!("CARGO_PKG_VERSION")))
        .call()
        .map_err(|e| format!("download failed: {e}"))?;
    let mut reader = reply.body_mut().with_config().limit(max).reader();
    let mut out = Vec::new();
    let mut chunk = [0u8; 64 * 1024];
    loop {
        let n = reader.read(&mut chunk).map_err(|e| format!("download failed: {e}"))?;
        if n == 0 { break; }
        out.extend_from_slice(&chunk[..n]);
        if let Some(p) = progress { p.done.store(out.len() as u64, Ordering::Relaxed); }
    }
    Ok(out)
}

fn home_dir() -> Option<PathBuf> { std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from) }

fn settings_path() -> Option<PathBuf> { Some(home_dir()?.join(".brainbend-studio").join("settings.json")) }

/// Where downloads are unpacked: inside the user's own folder, so no other account can plant files there.
fn update_dir() -> Option<PathBuf> { Some(home_dir()?.join(".brainbend-studio").join("update")) }

/// Unpacks a downloaded archive with the system's own tools.
fn extract(archive: &Path, out: &Path) -> Result<(), String> {
    let mut cmd = if cfg!(target_os = "macos") {
        let mut c = std::process::Command::new("ditto");
        c.arg("-x").arg("-k").arg(archive).arg(out);
        c
    } else if cfg!(target_os = "windows") {
        let mut c = std::process::Command::new("tar");
        c.arg("-xf").arg(archive).arg("-C").arg(out);
        c
    } else {
        let mut c = std::process::Command::new("tar");
        c.arg("-xzf").arg(archive).arg("-C").arg(out);
        c
    };
    let status = cmd.status().map_err(|e| format!("could not unpack the update: {e}"))?;
    if status.success() { Ok(()) } else { Err("could not unpack the update".into()) }
}

/// The app inside an unpacked update: the `.app` on a Mac, the program elsewhere.
fn find_payload(out: &Path) -> Result<PathBuf, String> {
    if cfg!(target_os = "macos") {
        let entries = std::fs::read_dir(out).map_err(|e| format!("could not read the update: {e}"))?;
        entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .find(|p| p.extension().is_some_and(|x| x == "app"))
            .ok_or_else(|| "the update does not contain the app".into())
    } else {
        let name = if cfg!(target_os = "windows") { "brainbend-studio.exe" } else { "brainbend-studio" };
        let p = out.join(name);
        if p.exists() { Ok(p) } else { Err("the update does not contain the app".into()) }
    }
}

/// Downloads this release for this system, checks its signature, and unpacks it. Returns what to install.
pub fn stage(release: &Release, progress: &Progress) -> Result<PathBuf, String> {
    let dir = update_dir().ok_or("could not find your home folder")?;
    stage_into(release, progress, &public_key()?, test_override("BRAINBEND_UPDATE_URL").is_some(), &dir)
}

fn stage_into(release: &Release, progress: &Progress, key: &minisign_verify::PublicKey, allow_any: bool, dir: &Path) -> Result<PathBuf, String> {
    let suffix = platform_suffix().ok_or("updates are not available on this system")?;
    let (file, sig) = pick_assets(release, suffix).ok_or("this release has no signed download for this system")?;
    progress.total.store(file.size, Ordering::Relaxed);
    let sig_text = String::from_utf8(fetch(&sig.url, 64 * 1024, None, allow_any)?).map_err(|_| "the signature file is unreadable".to_string())?;
    let data = fetch(&file.url, MAX_DOWNLOAD, Some(progress), allow_any)?;
    // Nothing touches the disk until the signature has checked out.
    verify(&data, &sig_text, key)?;
    let _ = std::fs::remove_dir_all(dir);
    let out = dir.join("out");
    std::fs::create_dir_all(&out).map_err(|e| format!("could not prepare the update: {e}"))?;
    let archive = dir.join(&file.name);
    std::fs::write(&archive, &data).map_err(|e| format!("could not save the update: {e}"))?;
    if kind() != Kind::Portable {
        // An installer, a package or an AppImage is used as it is.
        let kept = out.join(&file.name);
        std::fs::rename(&archive, &kept).map_err(|e| format!("could not save the update: {e}"))?;
        return Ok(kept);
    }
    extract(&archive, &out)?;
    let payload = find_payload(&out)?;
    let _ = std::fs::remove_file(&archive);
    Ok(payload)
}

/// The `.app` this process is running from.
#[cfg(target_os = "macos")]
fn running_app() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| format!("could not find the running app: {e}"))?;
    let app = exe
        .ancestors()
        .find(|p| p.extension().is_some_and(|x| x == "app"))
        .ok_or("BrainBend Studio is not running from an installed app, so it cannot replace itself")?
        .to_path_buf();
    if app.starts_with("/Volumes") {
        return Err("BrainBend Studio is running from a disk image. Drag it into Applications and open it from there first".into());
    }
    let dir = app.parent().ok_or("could not find where the app is installed")?;
    let probe = dir.join(".brainbend-write-test");
    std::fs::write(&probe, b"").map_err(|_| format!("no permission to replace the app in {}", dir.display()))?;
    let _ = std::fs::remove_file(probe);
    Ok(app)
}

/// The helper that swaps the app on a Mac. Arguments: the pid to wait for, the installed app, the new app, and the
/// program that opens an app (a parameter so the tests do not launch anything). If anything fails the old app stays.
#[cfg(target_os = "macos")]
const SWAP_SCRIPT: &str = r#"pid=$1; app=$2; new=$3; opener=$4
while kill -0 "$pid" 2>/dev/null; do sleep 0.2; done
stage="$app.updating"; old="$app.old"
rm -rf "$stage" "$old"
ditto "$new" "$stage" || { rm -rf "$stage"; "$opener" "$app"; exit 1; }
mv "$app" "$old" || { rm -rf "$stage"; "$opener" "$app"; exit 1; }
if mv "$stage" "$app"; then rm -rf "$old"; else mv "$old" "$app"; fi
"$opener" "$app"
"#;

/// Swaps the unpacked update in for the running app and starts it once this process has exited.
#[cfg(target_os = "macos")]
pub fn install(staged: &Path) -> Result<(), String> {
    let app = running_app()?;
    // A helper outlives this process: it waits for us to quit, swaps the bundles, and opens the new one.
    std::process::Command::new("/bin/sh")
        .arg("-c").arg(SWAP_SCRIPT).arg("sh")
        .arg(std::process::id().to_string()).arg(&app).arg(staged).arg("open")
        .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("could not start the update: {e}"))?;
    Ok(())
}

/// Runs a helper script that outlives this process: it waits for us to quit, installs, and opens the app again.
#[cfg(all(unix, not(target_os = "macos")))]
fn spawn_helper(script: &str, args: &[&Path]) -> Result<(), String> {
    let mut c = std::process::Command::new("/bin/sh");
    c.arg("-c").arg(script).arg("sh").arg(std::process::id().to_string());
    for a in args { c.arg(a); }
    c.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .spawn().map(|_| ()).map_err(|e| format!("could not start the update: {e}"))
}

/// Installs a package with the system's own permission prompt (pkexec), then opens the app again whether or not it worked.
#[cfg(all(unix, not(target_os = "macos")))]
const PACKAGE_SCRIPT: &str = r#"pid=$1; kind=$2; pkg=$3; app=$4
while kill -0 "$pid" 2>/dev/null; do sleep 0.2; done
if [ "$kind" = deb ]; then pkexec dpkg -i "$pkg"; else pkexec rpm -U --replacepkgs "$pkg"; fi
exec "$app"
"#;

#[cfg(all(unix, not(target_os = "macos")))]
fn replace_file(staged: &Path, target: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let next = target.with_extension("new");
    std::fs::copy(staged, &next).map_err(|e| format!("no permission to replace the app in {} ({e})", target.parent().map(|p| p.display().to_string()).unwrap_or_default()))?;
    std::fs::set_permissions(&next, std::fs::Permissions::from_mode(0o755)).map_err(|e| format!("could not install the update: {e}"))?;
    std::fs::rename(&next, target).map_err(|e| format!("could not install the update: {e}"))
}

/// Swaps the new program in for the running one, or hands the download to the installer that put this copy here.
#[cfg(target_os = "windows")]
pub fn install(staged: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let exe = std::env::current_exe().map_err(|e| format!("could not find the running app: {e}"))?;
    if kind() == Kind::Msi {
        // The installer cannot replace the program while it runs, so a script waits for us to quit, runs it, and opens the app.
        let script = staged.with_extension("cmd");
        let text = format!("@echo off\r\ntimeout /t 2 /nobreak >nul\r\nmsiexec /i \"{}\" /passive /norestart\r\nstart \"\" \"{}\"\r\n", staged.display(), exe.display());
        std::fs::write(&script, text).map_err(|e| format!("could not prepare the update: {e}"))?;
        std::process::Command::new("cmd").arg("/C").arg(&script).creation_flags(0x0800_0000).spawn().map_err(|e| format!("could not start the update: {e}"))?;
        return Ok(());
    }
    let old = exe.with_extension("exe.old");
    let _ = std::fs::remove_file(&old);
    std::fs::rename(&exe, &old).map_err(|e| format!("no permission to replace the app in {} ({e})", exe.parent().map(|p| p.display().to_string()).unwrap_or_default()))?;
    if let Err(e) = std::fs::copy(staged, &exe) {
        let _ = std::fs::rename(&old, &exe);
        return Err(format!("could not install the update: {e}"));
    }
    std::process::Command::new(&exe).spawn().map_err(|e| format!("could not restart: {e}"))?;
    Ok(())
}

/// Swaps the new program in for the running one (the old file is replaced in one step, so it never half-exists), or hands
/// the download to the package system that put this copy here.
#[cfg(all(unix, not(target_os = "macos")))]
pub fn install(staged: &Path) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("could not find the running app: {e}"))?;
    let target = match kind() {
        Kind::Deb | Kind::Rpm => {
            let which = if kind() == Kind::Deb { "deb" } else { "rpm" };
            return spawn_helper(PACKAGE_SCRIPT, &[Path::new(which), staged, &exe]);
        }
        Kind::AppImage => std::env::var_os("APPIMAGE").map(PathBuf::from).ok_or("could not find the AppImage file")?,
        _ => exe,
    };
    replace_file(staged, &target)?;
    std::process::Command::new(&target).spawn().map_err(|e| format!("could not restart: {e}"))?;
    Ok(())
}

#[cfg(not(any(unix, windows)))]
pub fn install(_staged: &Path) -> Result<(), String> { Err("updates are not available on this system".into()) }

/// Tidies up after an update: the unpacked download, and on Windows the old program file.
pub fn clean_up_after_update() {
    if let Some(dir) = update_dir() { let _ = std::fs::remove_dir_all(dir); }
    #[cfg(target_os = "windows")]
    if let Ok(exe) = std::env::current_exe() { let _ = std::fs::remove_file(exe.with_extension("exe.old")); }
}

pub enum State {
    Idle,
    Checking(Receiver<Result<Option<Release>, String>>),
    UpToDate,
    Available(Release),
    Downloading { release: Release, progress: Arc<Progress>, rx: Receiver<Result<PathBuf, String>> },
    Ready { release: Release, staged: PathBuf },
    UpdateFailed { release: Release, error: String },
    Failed(String),
}

pub struct Updater {
    pub state: State,
    /// Check once when the app opens.
    pub on_launch: bool,
    /// The update window is showing.
    pub dialog_open: bool,
    /// The update is installed and the app should close so the new one can open.
    pub restarting: bool,
    /// A check the author asked for is running: when it ends they get a window with the result.
    pub manual: bool,
    /// The window saying there is no update, or that the check failed.
    pub notice_open: bool,
}

impl Default for Updater {
    fn default() -> Self { Self { state: State::Idle, on_launch: true, dialog_open: false, restarting: false, manual: false, notice_open: false } }
}

impl Updater {
    /// The saved preference, with the check on when nothing is saved.
    pub fn load() -> Self {
        let on_launch = settings_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| v["check_for_updates"].as_bool())
            .unwrap_or(true);
        Self { on_launch, ..Self::default() }
    }

    pub fn save(&self) {
        let Some(path) = settings_path() else { return };
        if let Some(dir) = path.parent() { let _ = std::fs::create_dir_all(dir); }
        let _ = std::fs::write(path, serde_json::json!({ "check_for_updates": self.on_launch }).to_string());
    }

    /// "Check for Updates": an update already known opens its window; otherwise a check runs and, when it ends, a window
    /// says what it found.
    pub fn check_now(&mut self, ctx: &egui::Context) {
        if self.available().is_some() { self.dialog_open = true; return; }
        if matches!(self.state, State::Checking(_)) { self.manual = true; return; }
        self.manual = true;
        self.start(ctx);
    }

    /// Starts a check in the background; does nothing while a check or a download is running or an update is known.
    pub fn start(&mut self, ctx: &egui::Context) {
        if !matches!(self.state, State::Idle | State::UpToDate | State::Failed(_)) { return; }
        let (tx, rx) = channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(check(env!("CARGO_PKG_VERSION")));
            ctx.request_repaint();
        });
        self.state = State::Checking(rx);
    }

    /// Starts downloading the known update in the background.
    pub fn download(&mut self, ctx: &egui::Context) {
        let release = match std::mem::replace(&mut self.state, State::Idle) {
            State::Available(r) | State::UpdateFailed { release: r, .. } => r,
            other => { self.state = other; return; }
        };
        let progress = Arc::new(Progress::default());
        let (tx, rx) = channel();
        let (job, p, ctx) = (release.clone(), progress.clone(), ctx.clone());
        std::thread::spawn(move || {
            let _ = tx.send(stage(&job, &p));
            ctx.request_repaint();
        });
        self.state = State::Downloading { release, progress, rx };
    }

    /// Installs the downloaded update. On success the app should close (see `restarting`).
    pub fn install_now(&mut self) -> Result<(), String> {
        let (release, staged) = match std::mem::replace(&mut self.state, State::Idle) {
            State::Ready { release, staged } => (release, staged),
            other => { self.state = other; return Err("there is no update ready to install".into()); }
        };
        match install(&staged) {
            Ok(()) => { self.restarting = true; self.state = State::Ready { release, staged }; Ok(()) }
            Err(error) => { self.state = State::UpdateFailed { release, error: error.clone() }; Err(error) }
        }
    }

    /// Picks up the result of a running check or download. Call once a frame.
    pub fn poll(&mut self) {
        match &self.state {
            State::Checking(rx) => {
                self.state = match rx.try_recv() {
                    Ok(Ok(Some(release))) => State::Available(release),
                    Ok(Ok(None)) => State::UpToDate,
                    Ok(Err(e)) => State::Failed(e),
                    Err(TryRecvError::Empty) => return,
                    Err(TryRecvError::Disconnected) => State::Failed("the check stopped unexpectedly".into()),
                };
                if std::mem::take(&mut self.manual) {
                    match self.state {
                        State::Available(_) => self.dialog_open = true,
                        _ => self.notice_open = true,
                    }
                }
            }
            State::Downloading { release, rx, .. } => {
                let release = release.clone();
                self.state = match rx.try_recv() {
                    Ok(Ok(staged)) => State::Ready { release, staged },
                    Ok(Err(error)) => State::UpdateFailed { release, error },
                    Err(TryRecvError::Empty) => return,
                    Err(TryRecvError::Disconnected) => State::UpdateFailed { release, error: "the download stopped unexpectedly".into() },
                };
            }
            _ => {}
        }
    }

    /// The newer release, whatever stage the update is at.
    pub fn available(&self) -> Option<&Release> {
        match &self.state {
            State::Available(r) | State::Downloading { release: r, .. } | State::Ready { release: r, .. } | State::UpdateFailed { release: r, .. } => Some(r),
            _ => None,
        }
    }
}

/// Opens a web page in the default browser.
pub fn open_in_browser(url: &str) {
    let opener = if cfg!(target_os = "macos") { "open" } else if cfg!(target_os = "windows") { "explorer" } else { "xdg-open" };
    let _ = std::process::Command::new(opener).arg(url).spawn();
}

impl super::App {
    /// What a "Check for Updates" found when there is nothing to install: you are up to date, or the check failed.
    fn update_notice_window(&mut self, ctx: &egui::Context) {
        if !self.updater.notice_open { return; }
        let mut open = true;
        let mut close = false;
        egui::Window::new("Check for Updates").open(&mut open).collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
            ui.set_width(320.0);
            match &self.updater.state {
                State::Failed(e) => {
                    ui.strong("Could not check for updates");
                    ui.add_space(2.0);
                    ui.weak(format!("{e}. Check your internet connection and try again."));
                }
                _ => {
                    ui.strong("You have the latest version.");
                    ui.add_space(2.0);
                    ui.weak(format!("BrainBend Studio {}", env!("CARGO_PKG_VERSION")));
                }
            }
            ui.add_space(8.0);
            if ui.button("OK").clicked() { close = true; }
        });
        if !open || close { self.updater.notice_open = false; }
    }

    /// The update window: what is new, then download, then restart.
    pub(super) fn update_window(&mut self, ctx: &egui::Context) {
        self.update_notice_window(ctx);
        if !self.updater.dialog_open { return; }
        let Some(release) = self.updater.available().cloned() else { self.updater.dialog_open = false; return };
        #[derive(PartialEq)]
        enum Act { Download, Restart, Page, Later }
        let mut act: Option<Act> = None;
        let mut open = true;
        let can_install = installable(&release);
        egui::Window::new("Update available").open(&mut open).collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
            ui.set_width(380.0);
            ui.strong(format!("BrainBend Studio {} is available", release.version));
            ui.weak(format!("You have version {}.", env!("CARGO_PKG_VERSION")));
            let notes = clean_notes(&release.notes);
            if !notes.is_empty() {
                ui.add_space(6.0);
                egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                    ui.add(egui::Label::new(notes.chars().take(1500).collect::<String>()).wrap());
                });
            }
            ui.add_space(8.0);
            match &self.updater.state {
                State::Downloading { progress, .. } => {
                    let (done, total) = (progress.done.load(Ordering::Relaxed), progress.total.load(Ordering::Relaxed));
                    let frac = if total > 0 { (done as f32 / total as f32).min(1.0) } else { 0.0 };
                    ui.add(egui::ProgressBar::new(frac).text(format!("Downloading… {:.1} of {:.1} MB", done as f64 / 1_048_576.0, total as f64 / 1_048_576.0)));
                    ctx.request_repaint_after(Duration::from_millis(100));
                }
                State::Ready { .. } => {
                    ui.label("The update is downloaded and checked. BrainBend Studio will close and open again with the new version.");
                    if let Some(note) = kind().note() { ui.weak(note); }
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if ui.button("Restart now").clicked() { act = Some(Act::Restart); }
                        if ui.button("Later").clicked() { act = Some(Act::Later); }
                    });
                }
                state => {
                    if let State::UpdateFailed { error, .. } = state { ui.colored_label(egui::Color32::from_rgb(220, 70, 70), format!("The update did not install: {error}.")); ui.add_space(4.0); }
                    ui.horizontal(|ui| {
                        if can_install && ui.button(if matches!(state, State::UpdateFailed { .. }) { "Try again" } else { "Update now" }).clicked() { act = Some(Act::Download); }
                        if ui.button("Release page").clicked() { act = Some(Act::Page); }
                        if ui.button("Later").clicked() { act = Some(Act::Later); }
                    });
                    if let (true, Some(note)) = (can_install, kind().note()) { ui.add_space(2.0); ui.weak(note); }
                    if !can_install { ui.add_space(2.0); ui.weak("This release has no signed download for your system. Get it from the release page."); }
                }
            }
        });
        if !open { self.updater.dialog_open = false; }
        match act {
            Some(Act::Download) => self.updater.download(ctx),
            Some(Act::Restart) => { self.updater.dialog_open = false; self.guard(super::Pending::Update); }
            Some(Act::Page) => open_in_browser(&release.url),
            Some(Act::Later) => self.updater.dialog_open = false,
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPLY: &str = r###"{"tag_name":"v0.2.0","html_url":"https://github.com/brainbend/studio/releases/tag/v0.2.0","body":"## What's new\n- Updates","assets":[
        {"name":"BrainBend-Studio-v0.2.0-macos-universal.zip","browser_download_url":"https://github.com/brainbend/studio/releases/download/v0.2.0/BrainBend-Studio-v0.2.0-macos-universal.zip","size":1000},
        {"name":"BrainBend-Studio-v0.2.0-macos-universal.zip.minisig","browser_download_url":"https://github.com/brainbend/studio/releases/download/v0.2.0/BrainBend-Studio-v0.2.0-macos-universal.zip.minisig","size":300},
        {"name":"BrainBend-Studio-v0.2.0-linux.tar.gz","browser_download_url":"https://github.com/brainbend/studio/releases/download/v0.2.0/BrainBend-Studio-v0.2.0-linux.tar.gz","size":900}]}"###;

    #[test]
    fn reads_the_latest_release_reply() {
        let r = parse_release(REPLY).unwrap();
        assert_eq!(r.tag, "v0.2.0");
        assert_eq!(r.version, semver::Version::new(0, 2, 0));
        assert!(r.url.ends_with("/tag/v0.2.0"));
        assert!(r.notes.contains("What's new"));
        assert_eq!(r.assets.len(), 3);
        assert_eq!(r.assets[0].size, 1000);
    }

    #[test]
    fn release_notes_lose_their_markup() {
        let raw = "<!-- Release notes generated using configuration in\n.github/release.yml at v0.1.3 -->\n\n## What's Changed\n* Faster saving by @someone in https://github.com/brainbend/studio/pull/4\n- Fixes a crash\n\n**Full Changelog**: https://github.com/brainbend/studio/compare/v0.1.2...v0.1.3";
        assert_eq!(clean_notes(raw), "What's Changed\n\u{2022} Faster saving by @someone in https://github.com/brainbend/studio/pull/4\n\u{2022} Fixes a crash");
        assert_eq!(clean_notes("<!-- only a comment -->\n\n**Full Changelog**: https://example.com"), "", "nothing left to show");
        assert_eq!(clean_notes("Plain text stays."), "Plain text stays.");
        assert_eq!(clean_notes("Unfinished <!-- comment"), "Unfinished");
    }

    #[test]
    fn a_reply_without_assets_still_reads() {
        let r = parse_release(r#"{"tag_name":"v9.9.9","html_url":"https://example.com/r"}"#).unwrap();
        assert!(r.assets.is_empty());
        assert!(!installable(&r));
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

    #[test]
    fn each_kind_of_install_is_told_apart_and_has_its_own_download() {
        use std::ffi::OsStr;
        let none: Vec<PathBuf> = vec![];
        assert_eq!(detect_kind(Path::new("/home/me/Studio/brainbend-studio"), None, &none, (true, true)), Kind::Portable, "an unpacked copy updates itself");
        assert_eq!(detect_kind(Path::new("/home/me/x"), Some(OsStr::new("")), &none, (true, true)), Kind::Portable, "an empty APPIMAGE is not an AppImage");
        if cfg!(target_os = "linux") {
            assert_eq!(detect_kind(Path::new("/tmp/.mount_x/usr/bin/brainbend-studio"), Some(OsStr::new("/home/me/Studio.AppImage")), &none, (true, true)), Kind::AppImage);
            assert_eq!(detect_kind(Path::new("/usr/bin/brainbend-studio"), None, &none, (true, false)), Kind::Deb);
            assert_eq!(detect_kind(Path::new("/usr/bin/brainbend-studio"), None, &none, (false, true)), Kind::Rpm);
            assert_eq!(detect_kind(Path::new("/opt/studio/brainbend-studio"), None, &none, (true, true)), Kind::Portable);
            for (k, end) in [(Kind::Deb, ".deb"), (Kind::Rpm, ".rpm"), (Kind::AppImage, ".AppImage"), (Kind::Portable, ".tar.gz")] { assert!(suffix_for(k).unwrap().ends_with(end)); }
        }
        if cfg!(target_os = "windows") {
            let pf = vec![PathBuf::from(r"C:\Program Files")];
            assert_eq!(detect_kind(Path::new(r"C:\Program Files\BrainBend Studio\brainbend-studio.exe"), None, &pf, (false, false)), Kind::Msi);
            assert_eq!(detect_kind(Path::new(r"C:\Users\me\Studio\brainbend-studio.exe"), None, &pf, (false, false)), Kind::Portable);
            assert!(suffix_for(Kind::Msi).unwrap().ends_with(".msi") && suffix_for(Kind::Portable).unwrap().ends_with(".zip"));
        }
    }

    #[test]
    fn picks_this_systems_download_and_needs_its_signature() {
        let r = parse_release(REPLY).unwrap();
        let (file, sig) = pick_assets(&r, "-macos-universal.zip").unwrap();
        assert!(file.name.ends_with("macos-universal.zip"));
        assert_eq!(sig.name, format!("{}.minisig", file.name));
        assert!(pick_assets(&r, "-linux.tar.gz").is_none(), "a download with no signature is never offered");
        assert!(pick_assets(&r, "-windows.zip").is_none(), "nothing for a system the release does not cover");
    }

    // A real signature made with a throwaway key (minisign -S), for the bytes "hello update".
    const TEST_KEY: &str = "RWQFxaPMMRemK/a2rDhcGuWd4FhjBPLDMnwXQtavuyt3gvL3R+/mBOGz";
    const TEST_SIG: &str = "untrusted comment: signature from minisign secret key\nRUQFxaPMMRemK81wvHhqdALQrheowDH9DX4orxyJL66oOBepFlQEDlz1GyNVLmpcONNCYFQOCK/Ep6WBpFhlBAN/OtgY/K0RmgU=\ntrusted comment: test\nSaylpc5UNWnWSlp4v12+f8HIhvszr+Zk8ebka72LjtpoM9dodLf1ciNpL5avH3VWWk83qWrI6Mz8NSNhwcbACQ==\n";

    #[test]
    fn a_signed_download_verifies_and_a_changed_one_does_not() {
        let key = minisign_verify::PublicKey::from_base64(TEST_KEY).unwrap();
        assert!(verify(b"hello update", TEST_SIG, &key).is_ok());
        assert!(verify(b"hello update!", TEST_SIG, &key).is_err(), "one byte different");
        assert!(verify(b"hello update", "not a signature", &key).is_err());
    }

    #[test]
    fn a_signature_from_another_key_is_refused() {
        let built_in = minisign_verify::PublicKey::decode(PUBLIC_KEY).expect("the app's key file reads");
        assert!(verify(b"hello update", TEST_SIG, &built_in).is_err(), "signed by a different key");
    }

    // ---- the download, check and unpack path, against a local web server and a throwaway key ----

    const FIXTURE_ZIP: &[u8] = include_bytes!("../testdata/update/fixture-macos-universal.zip");
    const FIXTURE_SIG: &[u8] = include_bytes!("../testdata/update/fixture-macos-universal.zip.minisig");

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("brainbend-update-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    /// Serves `files` (path, bytes) on a free local port for the rest of the test run; returns its address.
    fn serve(files: Vec<(String, Vec<u8>)>) -> String {
        use std::io::{BufRead, BufReader, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut request = String::new();
                reader.read_line(&mut request).unwrap_or(0);
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" { break; }
                }
                let path = request.split_whitespace().nth(1).unwrap_or("/").to_string();
                let mut out = stream;
                match files.iter().find(|(p, _)| *p == path) {
                    Some((_, body)) => {
                        let _ = write!(out, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                        let _ = out.write_all(body);
                    }
                    None => { let _ = write!(out, "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"); }
                }
            }
        });
        format!("http://127.0.0.1:{port}")
    }

    /// A release whose download for this system lives at `base`.
    fn release_at(base: &str, zip_len: usize) -> Release {
        let name = format!("fixture{}", platform_suffix().unwrap());
        let asset = |n: &str, size: usize| Asset { name: n.to_string(), url: format!("{base}/{n}"), size: size as u64 };
        Release { tag: "v9.9.9".into(), version: semver::Version::new(9, 9, 9), notes: String::new(), url: String::new(), assets: vec![asset(&name, zip_len), asset(&format!("{name}.minisig"), FIXTURE_SIG.len())] }
    }

    fn served(zip: Vec<u8>) -> (Release, minisign_verify::PublicKey) {
        let name = format!("fixture{}", platform_suffix().unwrap());
        let base = serve(vec![(format!("/{name}"), zip.clone()), (format!("/{name}.minisig"), FIXTURE_SIG.to_vec())]);
        (release_at(&base, zip.len()), minisign_verify::PublicKey::from_base64(TEST_KEY).unwrap())
    }

    #[test]
    fn a_tampered_download_is_refused_and_nothing_is_written() {
        let mut bad = FIXTURE_ZIP.to_vec();
        let mid = bad.len() / 2;
        bad[mid] ^= 0xff;
        let (release, key) = served(bad);
        let dir = scratch("tampered");
        let err = stage_into(&release, &Progress::default(), &key, true, &dir).unwrap_err();
        assert!(err.contains("signature"), "{err}");
        assert!(!dir.exists(), "an unverified download never reaches the disk");
    }

    #[test]
    fn a_download_signed_by_another_key_is_refused() {
        let (release, _) = served(FIXTURE_ZIP.to_vec());
        let built_in = minisign_verify::PublicKey::decode(PUBLIC_KEY).unwrap();
        let dir = scratch("wrong-key");
        assert!(stage_into(&release, &Progress::default(), &built_in, true, &dir).is_err());
        assert!(!dir.exists());
    }

    #[test]
    fn a_download_from_a_foreign_address_is_refused_before_any_request() {
        let release = release_at("https://example.com", 10);
        let key = minisign_verify::PublicKey::from_base64(TEST_KEY).unwrap();
        let err = stage_into(&release, &Progress::default(), &key, false, &scratch("foreign")).unwrap_err();
        assert!(err.contains("not one of this project's release files"), "{err}");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn a_signed_download_is_checked_and_unpacked_to_the_app() {
        let (release, key) = served(FIXTURE_ZIP.to_vec());
        let dir = scratch("unpack");
        let progress = Progress::default();
        let app = stage_into(&release, &progress, &key, true, &dir).unwrap();
        assert!(app.extension().is_some_and(|x| x == "app"), "{app:?}");
        assert_eq!(std::fs::read_to_string(app.join("Contents/Resources/marker.txt")).unwrap(), "new version");
        assert_eq!(progress.done.load(Ordering::Relaxed), FIXTURE_ZIP.len() as u64, "progress reached the end");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[cfg(target_os = "macos")]
    fn run_swap(base: &Path, new_exists: bool) -> (bool, PathBuf) {
        let app = base.join("Installed.app");
        let new = base.join("New.app");
        std::fs::create_dir_all(app.join("Contents")).unwrap();
        std::fs::write(app.join("Contents/marker.txt"), "old").unwrap();
        if new_exists {
            std::fs::create_dir_all(new.join("Contents")).unwrap();
            std::fs::write(new.join("Contents/marker.txt"), "new").unwrap();
        }
        // A process that has already exited, so the helper does not wait.
        let mut gone = std::process::Command::new("true").spawn().unwrap();
        let pid = gone.id();
        gone.wait().unwrap();
        let ok = std::process::Command::new("/bin/sh").arg("-c").arg(SWAP_SCRIPT).arg("sh").arg(pid.to_string()).arg(&app).arg(&new).arg("true").status().unwrap().success();
        (ok, app)
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_swap_puts_the_new_app_in_place_and_leaves_nothing_behind() {
        let base = scratch("swap");
        std::fs::create_dir_all(&base).unwrap();
        let (ok, app) = run_swap(&base, true);
        assert!(ok);
        assert_eq!(std::fs::read_to_string(app.join("Contents/marker.txt")).unwrap(), "new");
        assert!(!base.join("Installed.app.old").exists() && !base.join("Installed.app.updating").exists());
        let _ = std::fs::remove_dir_all(base);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn a_failed_swap_leaves_the_old_app_untouched() {
        let base = scratch("swap-fails");
        std::fs::create_dir_all(&base).unwrap();
        let (ok, app) = run_swap(&base, false);
        assert!(!ok, "the helper reports the failure");
        assert_eq!(std::fs::read_to_string(app.join("Contents/marker.txt")).unwrap(), "old");
        assert!(!base.join("Installed.app.updating").exists());
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn only_this_projects_release_files_are_downloaded() {
        assert!(download_allowed("https://github.com/brainbend/studio/releases/download/v1.0.0/x.zip", false));
        assert!(!download_allowed("https://example.com/x.zip", false));
        assert!(!download_allowed("https://github.com/someone-else/studio/releases/download/v1.0.0/x.zip", false));
        assert!(download_allowed("http://127.0.0.1:1/x.zip", true), "a test server, only when a debug build is told to");
    }
}
