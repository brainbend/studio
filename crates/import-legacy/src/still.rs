//! A still picture for a video that has none, so the editor and the player can show something before it plays.
//!
//! The frame comes from `ffmpeg` when it is installed; without it a video simply has no still, as before.
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use brainbend_doc::{Asset, Kind, Project};

/// Where `ffmpeg` might live: on the PATH, or where Homebrew and the common installers put it (an app started
/// from the Finder does not inherit the shell's PATH).
fn ffmpeg() -> Option<PathBuf> {
    let candidates = ["ffmpeg", "/opt/homebrew/bin/ffmpeg", "/usr/local/bin/ffmpeg", "/usr/bin/ffmpeg"];
    candidates.iter().map(PathBuf::from).find(|p| {
        Command::new(p).arg("-version").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|s| s.success())
    })
}

/// One frame of the video as a JPEG, about a second in (the first frame when the clip is shorter), at most 1280 wide.
pub fn video_still(bytes: &[u8]) -> Option<Vec<u8>> {
    if bytes.is_empty() { return None; }
    let ffmpeg = ffmpeg()?;
    static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("brainbend-still-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).ok()?;
    let (input, output) = (dir.join("clip"), dir.join("still.jpg"));
    let result = (|| {
        std::fs::write(&input, bytes).ok()?;
        for seek in ["1", "0"] {
            let _ = std::fs::remove_file(&output);
            let ok = Command::new(&ffmpeg)
                .args(["-nostdin", "-loglevel", "error", "-y", "-ss", seek, "-i"]).arg(&input)
                .args(["-frames:v", "1", "-vf", "scale='min(1280,iw)':-2", "-q:v", "4"]).arg(&output)
                .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
                .status().is_ok_and(|s| s.success());
            if ok { if let Ok(b) = std::fs::read(&output) { if !b.is_empty() { return Some(b); } } }
        }
        None
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}

/// The asset for a video's still, named after the video.
pub fn still_asset(video: &Asset) -> Option<Asset> {
    let bytes = video_still(&video.bytes)?;
    let stem = Path::new(&video.name).file_stem().and_then(|s| s.to_str()).unwrap_or("video");
    Some(Asset { name: format!("{stem} still.jpg"), mime: "image/jpeg".into(), bytes, file: None })
}

/// Give every video that has no still one. Returns how many it made.
pub fn add_stills(project: &mut Project) -> usize {
    let mut made = 0;
    let mut cache: Vec<(String, String)> = vec![]; // video asset key -> still asset key
    let mut new_assets: Vec<(String, Asset)> = vec![];
    for slide in project.slides.iter_mut() {
        for o in slide.layers.iter_mut().flat_map(|l| l.objects.iter_mut()) {
            if !matches!(o.kind, Kind::Video) || o.poster.is_some() { continue; }
            let Some(key) = o.asset.clone() else { continue };
            if let Some((_, still)) = cache.iter().find(|(k, _)| *k == key) { o.poster = Some(still.clone()); continue; }
            let Some(video) = project.assets.get(&key) else { continue };
            let Some(still) = still_asset(video) else { continue };
            let still_key = format!("{key}-still");
            cache.push((key, still_key.clone()));
            new_assets.push((still_key.clone(), still));
            o.poster = Some(still_key);
            made += 1;
        }
    }
    project.assets.extend(new_assets);
    made
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-second test clip, made with ffmpeg itself; the tests are skipped where ffmpeg is not installed.
    fn clip() -> Option<Vec<u8>> {
        let ffmpeg = ffmpeg()?;
        let out = std::env::temp_dir().join(format!("brainbend-testclip-{}.mp4", std::process::id()));
        let ok = Command::new(ffmpeg).args(["-nostdin", "-loglevel", "error", "-y", "-f", "lavfi", "-i", "testsrc=size=320x180:rate=10:duration=1", "-pix_fmt", "yuv420p"]).arg(&out)
            .stdin(Stdio::null()).status().is_ok_and(|s| s.success());
        let bytes = ok.then(|| std::fs::read(&out).ok()).flatten();
        let _ = std::fs::remove_file(&out);
        bytes
    }

    #[test]
    fn a_still_is_a_jpeg_frame_and_nothing_comes_from_nothing() {
        assert!(video_still(&[]).is_none());
        assert!(video_still(b"this is not a video").is_none() || ffmpeg().is_none());
        let Some(bytes) = clip() else { return };
        let still = video_still(&bytes).expect("a frame");
        assert_eq!(&still[..2], &[0xFF, 0xD8], "a JPEG");
        let img = image::load_from_memory(&still).unwrap();
        assert_eq!((img.width(), img.height()), (320, 180));
    }

    #[test]
    fn videos_without_a_picture_get_one_and_share_it_per_video() {
        let Some(bytes) = clip() else { return };
        use brainbend_doc::*;
        let mut p = Project::new("t");
        p.assets.insert("v".into(), Asset { name: "clip.mp4".into(), mime: "video/mp4".into(), bytes, file: None });
        let mut a = Slide::new("a", "A", 1280.0, 720.0);
        for id in ["v1", "v2"] {
            let mut o = Object::new(id, id, Kind::Video, 0.0, 0.0, 100.0, 60.0);
            o.asset = Some("v".into());
            a.layers[0].objects.push(o);
        }
        p.slides = vec![a];
        assert_eq!(add_stills(&mut p), 1, "one still for the one video file");
        let objs = &p.slides[0].layers[0].objects;
        assert!(objs[0].poster.is_some() && objs[0].poster == objs[1].poster);
        assert!(p.assets.contains_key(objs[0].poster.as_ref().unwrap()));
        assert_eq!(add_stills(&mut p), 0, "a video that has one keeps it");
    }
}
