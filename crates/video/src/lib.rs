//! Finding, and when needed downloading, the ffmpeg program that BrainBend Studio uses.
//!
//! ffmpeg is a separate program (a GPL build, run as its own process), never linked into the app. It makes the still
//! picture of a video (#11) and encodes a lesson as a video (#27). It is not part of the app's own download: the first
//! time a feature needs it, the app asks, then fetches the build for this machine from
//! <https://github.com/brainbend/studio-components>, checks it against the SHA-256 built into the app (`ffmpeg-manifest.json`),
//! and keeps it in `~/.brainbend-studio/components/`. The checksums come with the app, not from the download site, so a
//! changed file there cannot be installed. Moving to a newer ffmpeg means a new app release.
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;

use sha2::{Digest, Sha256};

/// The builds this app trusts: one zip per platform with the SHA-256 it must have.
const MANIFEST: &str = include_str!("../ffmpeg-manifest.json");

/// What a download holds besides the program, kept beside it for the license.
const NOTICES: [&str; 2] = ["COPYING.GPLv3", "SOURCE.txt"];

/// The build of ffmpeg for one platform.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Component {
    pub version: String,
    /// The program's file name inside the zip and on disk.
    pub program: String,
    pub url: String,
    pub sha256: String,
    /// Bytes to download.
    pub size: u64,
}

/// The platform this build of the app runs on, as the manifest names it. An Apple silicon Mac uses the Intel build,
/// which runs under Rosetta.
pub fn platform() -> Option<&'static str> {
    if cfg!(all(target_os = "windows", target_arch = "x86_64")) { Some("windows-x86_64") }
    else if cfg!(all(target_os = "linux", target_arch = "x86_64")) { Some("linux-x86_64") }
    else if cfg!(target_os = "macos") { Some("macos-x86_64") }
    else { None }
}

/// The trusted build for `platform`, read from the manifest.
fn component_for(manifest: &str, platform: &str) -> Result<Component, String> {
    let v: serde_json::Value = serde_json::from_str(manifest).map_err(|e| format!("unreadable manifest: {e}"))?;
    let version = v["version"].as_str().ok_or("the manifest has no version")?;
    let base = v["base"].as_str().ok_or("the manifest has no address")?;
    let f = &v["files"][platform];
    let field = |k: &str| f[k].as_str().map(str::to_string).ok_or_else(|| format!("no {k} for {platform}"));
    Ok(Component {
        version: version.to_string(),
        program: field("program")?,
        url: format!("{base}{}", field("file")?),
        sha256: field("sha256")?,
        size: f["size"].as_u64().ok_or_else(|| format!("no size for {platform}"))?,
    })
}

/// The build to download on this machine; `None` on a platform we have no build for.
pub fn component() -> Option<Component> { component_for(MANIFEST, platform()?).ok() }

fn home_dir() -> Option<PathBuf> { std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from) }

/// Where a downloaded ffmpeg of this version lives.
fn install_dir(c: &Component) -> Option<PathBuf> {
    Some(home_dir()?.join(".brainbend-studio").join("components").join(format!("ffmpeg-{}", c.version)))
}

/// Whether `program` starts and says its version.
fn runs(program: &Path) -> bool {
    Command::new(program).arg("-version").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|s| s.success())
}

/// Where else ffmpeg might live: on the PATH, or where Homebrew and the common installers put it (an app started from
/// the Finder does not inherit the shell's PATH).
fn installed() -> Vec<PathBuf> {
    ["ffmpeg", "/opt/homebrew/bin/ffmpeg", "/usr/local/bin/ffmpeg", "/usr/bin/ffmpeg"].iter().map(PathBuf::from).collect()
}

/// The ffmpeg to use: the one named by `BRAINBEND_FFMPEG`, else the one the app downloaded, else one installed on the
/// machine. `None` when there is none; then videos have no still until [`download`] has run.
pub fn ffmpeg() -> Option<PathBuf> {
    static FOUND: Mutex<Option<PathBuf>> = Mutex::new(None);
    if let Some(p) = FOUND.lock().unwrap_or_else(|e| e.into_inner()).clone() { return Some(p); }
    let mut candidates: Vec<PathBuf> = vec![];
    if let Some(p) = std::env::var_os("BRAINBEND_FFMPEG") { candidates.push(PathBuf::from(p)); }
    if let Some(c) = component() { if let Some(dir) = install_dir(&c) { candidates.push(dir.join(&c.program)); } }
    candidates.extend(installed());
    let found = candidates.into_iter().find(|p| runs(p))?;
    *FOUND.lock().unwrap_or_else(|e| e.into_inner()) = Some(found.clone());
    Some(found)
}

/// The build to offer for download: there is no ffmpeg yet and this platform has one. `None` when ffmpeg is already
/// available, or there is nothing to download.
pub fn needs_download() -> Option<Component> {
    if ffmpeg().is_some() { None } else { component() }
}

/// A command that runs ffmpeg quietly with no input, or `None` without an ffmpeg.
pub fn command() -> Option<Command> {
    let mut c = Command::new(ffmpeg()?);
    c.args(["-nostdin", "-hide_banner", "-loglevel", "error"]).stdin(Stdio::null());
    Some(c)
}

/// Fetches the zip, telling `progress` how many bytes of how many have arrived.
fn fetch(c: &Component, progress: &mut dyn FnMut(u64, u64)) -> Result<Vec<u8>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(900))).build().into();
    let mut reply = agent
        .get(&c.url)
        .header("User-Agent", concat!("BrainBend-Studio/", env!("CARGO_PKG_VERSION")))
        .call()
        .map_err(|e| format!("could not download ffmpeg: {e}"))?;
    // The size is in the manifest, so a longer reply is not the file we expect.
    let mut reader = reply.body_mut().with_config().limit(c.size + 1024 * 1024).reader();
    let mut out = Vec::with_capacity(c.size as usize);
    let mut chunk = [0u8; 64 * 1024];
    loop {
        let n = reader.read(&mut chunk).map_err(|e| format!("could not download ffmpeg: {e}"))?;
        if n == 0 { break; }
        out.extend_from_slice(&chunk[..n]);
        progress(out.len() as u64, c.size);
    }
    Ok(out)
}

/// Checks the zip against the manifest and unpacks it into `dir`: the program, with the license notices beside it.
/// Only those three names are read from the zip, and `dir` is replaced whole, so a half-done install is never left.
fn install(bytes: &[u8], c: &Component, dir: &Path) -> Result<PathBuf, String> {
    let got = Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect::<String>();
    if got != c.sha256 { return Err("the download is not the file this version of Studio expects, so it was not installed".into()); }
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| format!("the download is not a zip: {e}"))?;
    let parent = dir.parent().ok_or("no place to install")?;
    std::fs::create_dir_all(parent).map_err(|e| format!("could not make {}: {e}", parent.display()))?;
    let staging = parent.join(format!(".ffmpeg-{}-{}", c.version, std::process::id()));
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(|e| format!("could not make {}: {e}", staging.display()))?;
    let result = (|| {
        for name in std::iter::once(c.program.as_str()).chain(NOTICES) {
            let mut entry = zip.by_name(name).map_err(|_| format!("the download has no {name}"))?;
            let mut data = Vec::new();
            entry.read_to_end(&mut data).map_err(|e| format!("could not read {name}: {e}"))?;
            let path = staging.join(name);
            let mut f = std::fs::File::create(&path).map_err(|e| format!("could not write {}: {e}", path.display()))?;
            f.write_all(&data).map_err(|e| format!("could not write {}: {e}", path.display()))?;
            #[cfg(unix)]
            if name == c.program {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).map_err(|e| format!("could not make {name} runnable: {e}"))?;
            }
        }
        let _ = std::fs::remove_dir_all(dir);
        std::fs::rename(&staging, dir).map_err(|e| format!("could not install ffmpeg: {e}"))?;
        Ok(dir.join(&c.program))
    })();
    if result.is_err() { let _ = std::fs::remove_dir_all(&staging); }
    result
}

/// Downloads and installs the build for this machine, then checks that it runs. Slow: call it from a background
/// thread. `progress` gets the bytes received and the total.
pub fn download(progress: &mut dyn FnMut(u64, u64)) -> Result<PathBuf, String> {
    let c = component().ok_or("there is no ffmpeg download for this kind of computer")?;
    let dir = install_dir(&c).ok_or("could not find your home folder to keep ffmpeg in")?;
    let bytes = fetch(&c, progress)?;
    let program = install(&bytes, &c, &dir)?;
    if !runs(&program) {
        let _ = std::fs::remove_dir_all(&dir);
        let hint = if cfg!(target_os = "macos") { " On a Mac with Apple silicon this needs Rosetta." } else { "" };
        return Err(format!("ffmpeg was downloaded but does not start.{hint}"));
    }
    Ok(program)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sha(bytes: &[u8]) -> String { Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect() }

    /// A zip like the ones in the studio-components release, with a stand-in for the program.
    fn make_zip(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = std::io::Cursor::new(Vec::new());
        let mut z = zip::ZipWriter::new(&mut out);
        for (name, body) in files {
            z.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
            z.write_all(body).unwrap();
        }
        z.finish().unwrap();
        out.into_inner()
    }

    fn component_of(zip: &[u8]) -> Component {
        Component { version: "9.9.9".into(), program: "ffmpeg".into(), url: "https://example.invalid/x.zip".into(), sha256: sha(zip), size: zip.len() as u64 }
    }

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("brainbend-video-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn the_manifest_names_a_build_for_each_platform_with_a_checksum() {
        for p in ["windows-x86_64", "linux-x86_64", "macos-x86_64"] {
            let c = component_for(MANIFEST, p).unwrap();
            assert_eq!(c.sha256.len(), 64, "{p}");
            assert!(c.url.starts_with("https://github.com/brainbend/studio-components/releases/download/ffmpeg-"), "{p}");
            assert!(c.url.ends_with(&format!("ffmpeg-{}-{p}.zip", c.version)), "{p}");
            assert!(c.size > 1_000_000, "{p}");
        }
        assert_eq!(component_for(MANIFEST, "windows-x86_64").unwrap().program, "ffmpeg.exe");
        assert!(component_for(MANIFEST, "plan9-mips").is_err());
    }

    #[test]
    fn a_verified_download_is_unpacked_with_its_notices_and_can_run() {
        let zip = make_zip(&[("ffmpeg", b"#!/bin/sh\necho hi\n"), ("COPYING.GPLv3", b"license"), ("SOURCE.txt", b"source")]);
        let dir = temp("ok").join("ffmpeg-9.9.9");
        let program = install(&zip, &component_of(&zip), &dir).unwrap();
        assert_eq!(program, dir.join("ffmpeg"));
        assert_eq!(std::fs::read(dir.join("COPYING.GPLv3")).unwrap(), b"license");
        assert_eq!(std::fs::read(dir.join("SOURCE.txt")).unwrap(), b"source");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&program).unwrap().permissions().mode() & 0o111, 0o111, "the program can be run");
        }
        let leftovers = std::fs::read_dir(dir.parent().unwrap()).unwrap().count();
        assert_eq!(leftovers, 1, "no staging folder is left behind");
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
    }

    #[test]
    fn a_download_that_does_not_match_its_checksum_is_not_installed() {
        let zip = make_zip(&[("ffmpeg", b"real"), ("COPYING.GPLv3", b"l"), ("SOURCE.txt", b"s")]);
        let mut c = component_of(&zip);
        let tampered = make_zip(&[("ffmpeg", b"evil"), ("COPYING.GPLv3", b"l"), ("SOURCE.txt", b"s")]);
        let dir = temp("bad").join("ffmpeg-9.9.9");
        let err = install(&tampered, &c, &dir).unwrap_err();
        assert!(err.contains("not the file"), "{err}");
        assert!(!dir.exists(), "nothing was written");
        c.sha256 = "0".repeat(64);
        assert!(install(&zip, &c, &dir).is_err());
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
    }

    #[test]
    fn a_zip_missing_a_file_leaves_nothing_behind_and_keeps_an_earlier_install() {
        let good = make_zip(&[("ffmpeg", b"v1"), ("COPYING.GPLv3", b"l"), ("SOURCE.txt", b"s")]);
        let dir = temp("partial").join("ffmpeg-9.9.9");
        install(&good, &component_of(&good), &dir).unwrap();
        let bad = make_zip(&[("ffmpeg", b"v2")]);
        let err = install(&bad, &component_of(&bad), &dir).unwrap_err();
        assert!(err.contains("COPYING.GPLv3"), "{err}");
        assert_eq!(std::fs::read(dir.join("ffmpeg")).unwrap(), b"v1", "the earlier install is untouched");
        assert_eq!(std::fs::read_dir(dir.parent().unwrap()).unwrap().count(), 1, "no staging folder is left behind");
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
    }

    /// Fetches the real build from the studio-components release (about 30-70 MB), so it only runs on request:
    /// `cargo test -p brainbend-video -- --ignored`.
    #[test]
    #[ignore = "downloads the real ffmpeg"]
    fn the_real_download_installs_and_runs() {
        let home = temp("live-home");
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("HOME", &home);
        std::env::set_var("USERPROFILE", &home);
        let mut last = 0;
        let program = download(&mut |done, total| { assert!(done <= total + 1024 * 1024); last = done; }).unwrap();
        assert!(last > 1_000_000, "progress was reported");
        assert!(program.starts_with(&home), "{}", program.display());
        assert!(runs(&program));
        assert_eq!(ffmpeg().as_deref(), Some(program.as_path()), "the downloaded copy is the one used");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_program_that_is_not_there_does_not_run() {
        assert!(!runs(Path::new("/definitely/not/ffmpeg")));
    }
}
