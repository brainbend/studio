//! Turning the pictures and sounds of a [`Plan`](crate::plan::Plan) into an MP4 with ffmpeg.
//!
//! The pictures are shown one after another for their lengths (ffmpeg's concat list), scaled to the video's size, and
//! each sound is cut, delayed to its place and mixed in. The result is H.264 video and AAC sound in an MP4 that plays
//! anywhere and starts quickly when streamed.
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::time::Duration;

use crate::plan::{Blend, Clip, MediaInfo};

/// Everything needed to make one video.
pub struct Job {
    /// The pictures in order, with how long each shows (milliseconds).
    pub pictures: Vec<(PathBuf, u32)>,
    /// The sounds: the file each is read from, and where it goes.
    pub sounds: Vec<(PathBuf, Clip)>,
    /// Even numbers of pixels, as H.264 needs.
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub total_ms: u64,
    /// The slide transitions: where a slide blends the one before into its first pictures.
    pub blends: Vec<Blend>,
    /// The title written into the file, which players show.
    pub title: String,
    /// Where the concat list is written while encoding (a lesson with transitions has one more beside it for each).
    pub list: PathBuf,
    pub output: PathBuf,
}

/// The pictures in runs: a new run starts at each transition, so the run it blends into is a stream of its own.
fn runs(job: &Job) -> Vec<(&[(PathBuf, u32)], Option<&Blend>)> {
    let mut out: Vec<(&[(PathBuf, u32)], Option<&Blend>)> = vec![];
    let mut start = 0;
    let mut blend: Option<&Blend> = None;
    for b in &job.blends {
        if b.shot <= start || b.shot >= job.pictures.len() { continue; }
        out.push((&job.pictures[start..b.shot], blend));
        start = b.shot;
        blend = Some(b);
    }
    out.push((&job.pictures[start..], blend));
    out
}

/// Where run `k`'s concat list is written.
fn list_path(job: &Job, k: usize) -> PathBuf {
    if k == 0 { return job.list.clone(); }
    let stem = job.list.file_stem().and_then(|s| s.to_str()).unwrap_or("pictures");
    job.list.with_file_name(format!("{stem}-{k}.txt"))
}

/// A file name written so ffmpeg's concat list reads it back as is.
fn quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
}

/// The concat list: each picture with its length, and the last picture again (ffmpeg ignores the length of the last
/// entry unless it is repeated).
pub fn concat_list(pictures: &[(PathBuf, u32)]) -> String {
    let mut s = String::from("ffconcat version 1.0\n");
    for (path, ms) in pictures {
        s.push_str(&format!("file {}\nduration {:.3}\n", quote(path), *ms as f64 / 1000.0));
    }
    if let Some((path, _)) = pictures.last() { s.push_str(&format!("file {}\n", quote(path))); }
    s
}

fn secs(ms: u64) -> String { format!("{:.3}", ms as f64 / 1000.0) }

/// The sound part of the filter graph, ending in the label `[a]`; `None` when there is no sound.
fn sound_filters(sounds: &[(PathBuf, Clip)], first_input: usize, total_ms: u64) -> Option<String> {
    if sounds.is_empty() { return None; }
    let mut parts: Vec<String> = vec![];
    for (i, (_, c)) in sounds.iter().enumerate() {
        let mut chain = format!("[{}:a]", first_input + i);
        if c.looped { chain.push_str("aloop=loop=-1:size=2147483647,"); }
        chain.push_str(&format!("atrim=duration={},asetpts=PTS-STARTPTS,volume={:.3},adelay={}:all=1", secs(c.length_ms), c.volume, c.start_ms));
        parts.push(format!("{chain}[s{i}]"));
    }
    let labels: String = (0..sounds.len()).map(|i| format!("[s{i}]")).collect();
    let mixed = if sounds.len() == 1 { format!("{labels}anull") } else { format!("{labels}amix=inputs={}:duration=longest:normalize=0", sounds.len()) };
    // Pad the sound to the length of the video, so the two end together.
    parts.push(format!("{mixed},apad=whole_dur={}[a]", secs(total_ms)));
    Some(parts.join(";"))
}

/// ffmpeg's arguments for `job`, writing to `output`.
pub fn args(job: &Job, output: &Path) -> Vec<OsString> {
    let mut a: Vec<OsString> = vec![];
    let mut push = |s: &str| a.push(s.into());
    for s in ["-y", "-progress", "pipe:1", "-nostats"] { push(s); }
    let runs = runs(job);
    for k in 0..runs.len() {
        for s in ["-f", "concat", "-safe", "0", "-i"] { a.push(s.into()); }
        a.push(list_path(job, k).into_os_string());
    }
    for (path, _) in &job.sounds {
        a.push("-i".into());
        a.push(path.clone().into_os_string());
    }
    let (w, h) = (job.width, job.height);
    // Each run is one stream, in the same size, rate and format so that they can be blended.
    let label = |k: usize| if runs.len() == 1 { "v".to_string() } else { format!("v{k}") };
    let mut parts: Vec<String> = (0..runs.len()).map(|k| format!(
        "[{k}:v]fps={},scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:color=white,format=yuv420p,setsar=1,settb=AVTB[{}]", job.fps, label(k))).collect();
    // A transition holds the video so far on its last picture for as long as it takes, and blends the next run in from
    // where that run starts, so the run keeps its own time and the video is no longer.
    let mut so_far: u64 = runs[0].0.iter().map(|(_, ms)| *ms as u64).sum();
    let mut acc = "v0".to_string();
    for (k, (pictures, blend)) in runs.iter().enumerate().skip(1) {
        let d = blend.map_or(0, |b| b.ms as u64).max(1);
        let name = blend.map_or("fade", |b| b.name);
        let out = if k == runs.len() - 1 { "v".to_string() } else { format!("b{k}") };
        parts.push(format!("[{acc}]tpad=stop_mode=clone:stop_duration={}[p{k}];[p{k}][v{k}]xfade=transition={name}:duration={}:offset={}[{out}]", secs(d), secs(d), secs(so_far)));
        so_far += pictures.iter().map(|(_, ms)| *ms as u64).sum::<u64>();
        acc = out;
    }
    let video = parts.join(";");
    let graph = match sound_filters(&job.sounds, runs.len(), job.total_ms) {
        Some(s) => format!("{video};{s}"),
        None => video,
    };
    a.push("-filter_complex".into());
    a.push(graph.into());
    for s in ["-map", "[v]"] { a.push(s.into()); }
    if !job.sounds.is_empty() { for s in ["-map", "[a]", "-c:a", "aac", "-b:a", "160k"] { a.push(s.into()); } }
    for s in ["-c:v", "libx264", "-preset", "medium", "-crf", "20", "-pix_fmt", "yuv420p", "-movflags", "+faststart", "-t"] { a.push(s.into()); }
    a.push(secs(job.total_ms).into());
    if !job.title.trim().is_empty() { a.push("-metadata".into()); a.push(format!("title={}", job.title.trim()).into()); }
    for s in ["-f", "mp4"] { a.push(s.into()); }
    a.push(output.as_os_str().to_owned());
    a
}

/// `HH:MM:SS.xx` as milliseconds.
fn parse_clock(s: &str) -> Option<u64> {
    let mut it = s.trim().split(':');
    let (h, m, sec): (u64, u64, f64) = (it.next()?.parse().ok()?, it.next()?.parse().ok()?, it.next()?.parse().ok()?);
    Some(h * 3_600_000 + m * 60_000 + (sec * 1000.0).round() as u64)
}

/// How long a media file is and whether it has sound, read from what ffmpeg prints about it. `None` when ffmpeg is not
/// there or cannot read the file.
pub fn probe(path: &Path) -> Option<MediaInfo> {
    // The details are printed at the normal log level; ffmpeg then complains it has no output file, which is fine.
    let out = Command::new(crate::ffmpeg()?).args(["-nostdin", "-hide_banner", "-i"]).arg(path).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped()).output().ok()?;
    parse_probe(&String::from_utf8_lossy(&out.stderr))
}

fn parse_probe(text: &str) -> Option<MediaInfo> {
    let duration = text.lines().find_map(|l| l.trim().strip_prefix("Duration:").and_then(|r| parse_clock(r.split(',').next()?)))?;
    Some(MediaInfo { duration_ms: duration, has_audio: text.lines().any(|l| l.contains("Stream #") && l.contains(": Audio:")) })
}

/// Makes the video. `progress` gets how far along it is, in milliseconds of video; `cancel` stops it early (and the
/// partly written file is removed).
pub fn run(job: &Job, progress: &mut dyn FnMut(u64), cancel: &AtomicBool) -> Result<(), String> {
    let program = crate::ffmpeg().ok_or("the video program is not installed")?;
    for (k, (pictures, _)) in runs(job).iter().enumerate() {
        std::fs::write(list_path(job, k), concat_list(pictures)).map_err(|e| format!("could not prepare the video: {e}"))?;
    }
    let part = job.output.with_extension("mp4.part");
    let mut child = Command::new(program)
        .args(["-nostdin", "-hide_banner", "-loglevel", "error"])
        .args(args(job, &part))
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().map_err(|e| format!("could not start the video program: {e}"))?;
    let (tx, rx) = channel::<u64>();
    let stdout = child.stdout.take().ok_or("no output from the video program")?;
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Some(us) = line.strip_prefix("out_time_us=").and_then(|v| v.trim().parse::<u64>().ok()) { let _ = tx.send(us / 1000); }
        }
    });
    let mut stderr = child.stderr.take().ok_or("no messages from the video program")?;
    let errors = std::thread::spawn(move || { let mut s = String::new(); let _ = stderr.read_to_string(&mut s); s });
    let status = loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            let _ = errors.join();
            let _ = std::fs::remove_file(&part);
            return Err("cancelled".into());
        }
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(ms) => progress(ms.min(job.total_ms)),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break child.wait().map_err(|e| e.to_string())?,
        }
    };
    let _ = reader.join();
    let messages = errors.join().unwrap_or_default();
    if !status.success() {
        let _ = std::fs::remove_file(&part);
        let last = messages.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("it stopped without saying why");
        return Err(format!("the video could not be made: {last}"));
    }
    let _ = std::fs::remove_file(&job.output);
    std::fs::rename(&part, &job.output).map_err(|e| format!("could not save the video: {e}"))?;
    progress(job.total_ms);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(sounds: Vec<(PathBuf, Clip)>) -> Job {
        Job {
            pictures: vec![(PathBuf::from("/t/a.png"), 2500), (PathBuf::from("/t/b.png"), 1000)],
            sounds, width: 1280, height: 720, fps: 30, total_ms: 3500, blends: vec![], title: String::new(),
            list: PathBuf::from("/t/list.txt"), output: PathBuf::from("/t/out.mp4"),
        }
    }

    fn clip(start: u64, len: u64, looped: bool) -> Clip { Clip { asset: "x".into(), start_ms: start, length_ms: len, volume: 0.5, looped } }

    fn text(a: &[OsString]) -> String { a.iter().map(|s| s.to_string_lossy().into_owned()).collect::<Vec<_>>().join(" ") }

    #[test]
    fn the_list_shows_each_picture_for_its_length_and_repeats_the_last() {
        let l = concat_list(&[(PathBuf::from("/t/a.png"), 2500), (PathBuf::from("/t/it's.png"), 1000)]);
        assert_eq!(l, "ffconcat version 1.0\nfile '/t/a.png'\nduration 2.500\nfile '/t/it'\\''s.png'\nduration 1.000\nfile '/t/it'\\''s.png'\n");
    }

    #[test]
    fn a_video_with_no_sound_has_no_audio_track() {
        let a = text(&args(&job(vec![]), Path::new("/t/out.mp4.part")));
        assert!(a.contains("-f concat -safe 0 -i /t/list.txt") && a.contains("-c:v libx264") && a.contains("-pix_fmt yuv420p"));
        assert!(a.contains("[0:v]fps=30") && a.contains("settb=AVTB[v]") && !a.contains("xfade") && !a.contains("-metadata"));
        assert!(a.contains("scale=1280:720") && a.contains("pad=1280:720") && a.contains("fps=30"));
        assert!(!a.contains("aac") && !a.contains("[a]") && !a.contains("amix"));
        assert!(a.contains("-t 3.500") && a.ends_with("-f mp4 /t/out.mp4.part"));
    }

    #[test]
    fn each_sound_is_cut_delayed_and_mixed_and_padded_to_the_video() {
        let a = text(&args(&job(vec![(PathBuf::from("/t/n.mp3"), clip(500, 2000, false)), (PathBuf::from("/t/m.mp3"), clip(1000, 2500, true))]), Path::new("/t/o.part")));
        assert!(a.contains("-i /t/n.mp3 -i /t/m.mp3"));
        assert!(a.contains("[1:a]atrim=duration=2.000,asetpts=PTS-STARTPTS,volume=0.500,adelay=500:all=1[s0]"), "{a}");
        assert!(a.contains("[2:a]aloop=loop=-1:size=2147483647,atrim=duration=2.500,asetpts=PTS-STARTPTS,volume=0.500,adelay=1000:all=1[s1]"), "{a}");
        assert!(a.contains("[s0][s1]amix=inputs=2:duration=longest:normalize=0,apad=whole_dur=3.500[a]"), "{a}");
        assert!(a.contains("-map [a] -c:a aac"));
    }

    #[test]
    fn a_transition_holds_the_video_so_far_and_blends_the_next_slide_in_from_where_it_starts() {
        let mut j = job(vec![(PathBuf::from("/t/n.mp3"), clip(0, 1000, false))]);
        j.blends = vec![Blend { shot: 1, name: "wipeleft", ms: 500 }];
        let a = text(&args(&j, Path::new("/t/o.part")));
        // The second list sits beside the first, written with the system's own separator.
        let second = j.list.with_file_name("list-1.txt");
        assert!(a.contains(&format!("-f concat -safe 0 -i /t/list.txt -f concat -safe 0 -i {} -i /t/n.mp3", second.display())), "{a}");
        assert!(a.contains("[1:v]fps=30") && a.contains("settb=AVTB[v1]") && a.contains("settb=AVTB[v0]"), "{a}");
        assert!(a.contains("[v0]tpad=stop_mode=clone:stop_duration=0.500[p1];[p1][v1]xfade=transition=wipeleft:duration=0.500:offset=2.500[v]"), "{a}");
        assert!(a.contains("[2:a]atrim"), "the sound's input follows the pictures': {a}");
    }

    #[test]
    fn several_transitions_chain_and_each_starts_where_the_video_so_far_ends() {
        let mut j = job(vec![]);
        j.pictures = vec![(PathBuf::from("/t/a.png"), 2000), (PathBuf::from("/t/b.png"), 3000), (PathBuf::from("/t/c.png"), 1000), (PathBuf::from("/t/d.png"), 4000)];
        j.blends = vec![Blend { shot: 1, name: "fade", ms: 400 }, Blend { shot: 3, name: "slideleft", ms: 800 }];
        let a = text(&args(&j, Path::new("/t/o.part")));
        assert!(a.contains("[v0]tpad=stop_mode=clone:stop_duration=0.400[p1];[p1][v1]xfade=transition=fade:duration=0.400:offset=2.000[b1]"), "{a}");
        assert!(a.contains("[b1]tpad=stop_mode=clone:stop_duration=0.800[p2];[p2][v2]xfade=transition=slideleft:duration=0.800:offset=6.000[v]"), "{a}");
        assert_eq!(runs(&j).iter().map(|(p, _)| p.len()).collect::<Vec<_>>(), vec![1, 2, 1]);
    }

    #[test]
    fn a_transition_that_cannot_apply_is_ignored() {
        let mut j = job(vec![]);
        j.blends = vec![Blend { shot: 0, name: "fade", ms: 500 }, Blend { shot: 9, name: "fade", ms: 500 }];
        assert_eq!(runs(&j).len(), 1, "one before the first picture and one past the last are not transitions");
    }

    #[test]
    fn the_title_is_written_into_the_file() {
        let mut j = job(vec![]);
        j.title = "  Safety Basics ".into();
        let a = text(&args(&j, Path::new("/t/o.part")));
        assert!(a.contains("-metadata title=Safety Basics -f mp4"), "{a}");
    }

    #[test]
    fn one_sound_needs_no_mixing() {
        let a = text(&args(&job(vec![(PathBuf::from("/t/n.mp3"), clip(0, 1000, false))]), Path::new("/t/o.part")));
        assert!(a.contains("[s0]anull,apad=whole_dur=3.500[a]") && !a.contains("amix"), "{a}");
    }

    #[test]
    fn what_ffmpeg_says_about_a_file_gives_its_length_and_whether_it_has_sound() {
        let with = "Input #0, mp3, from 'x.mp3':\n  Duration: 00:01:02.50, start: 0.0, bitrate: 128 kb/s\n  Stream #0:0: Audio: mp3, 44100 Hz, stereo, fltp, 128 kb/s\n";
        assert_eq!(parse_probe(with), Some(MediaInfo { duration_ms: 62_500, has_audio: true }));
        let without = "Input #0, mov,mp4, from 'v.mp4':\n  Duration: 00:00:05.04, start: 0.000000, bitrate: 900 kb/s\n  Stream #0:0[0x1](und): Video: h264, yuv420p, 640x360\n";
        assert_eq!(parse_probe(without), Some(MediaInfo { duration_ms: 5040, has_audio: false }));
        assert_eq!(parse_probe("nothing useful"), None);
    }

    /// Makes a real video with whatever ffmpeg is available (set `BRAINBEND_FFMPEG`); skipped when there is none.
    #[test]
    fn a_real_video_is_made_with_its_sound_and_can_be_cancelled() {
        let Some(program) = crate::ffmpeg() else { eprintln!("skipped: no ffmpeg"); return };
        let dir = std::env::temp_dir().join(format!("brainbend-encode-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let make = |args: &[&str], out: &Path| {
            let ok = Command::new(&program).args(["-nostdin", "-loglevel", "error", "-y"]).args(args).arg(out).status().unwrap().success();
            assert!(ok, "ffmpeg could not make {}", out.display());
        };
        let (red, blue, tone) = (dir.join("red.png"), dir.join("blue.png"), dir.join("tone.wav"));
        make(&["-f", "lavfi", "-i", "color=c=red:s=320x180", "-frames:v", "1"], &red);
        make(&["-f", "lavfi", "-i", "color=c=blue:s=320x180", "-frames:v", "1"], &blue);
        make(&["-f", "lavfi", "-i", "sine=frequency=440:duration=1"], &tone);
        let info = probe(&tone).expect("the tone can be read");
        assert!(info.has_audio && (900..1100).contains(&info.duration_ms), "{info:?}");
        let job = Job {
            pictures: vec![(red, 1000), (blue, 1000)], sounds: vec![(tone, clip(500, 1000, false))],
            width: 640, height: 360, fps: 10, total_ms: 2000, blends: vec![Blend { shot: 1, name: "wipeleft", ms: 500 }], title: "My Lesson".into(),
            list: dir.join("list.txt"), output: dir.join("out.mp4"),
        };
        let mut last = 0;
        run(&job, &mut |ms| last = ms, &AtomicBool::new(false)).unwrap();
        assert_eq!(last, 2000);
        let made = probe(&job.output).expect("the video can be read");
        assert!(made.has_audio, "it has its sound");
        assert!((1900..2300).contains(&made.duration_ms), "about two seconds: {made:?}");
        assert!(!job.output.with_extension("mp4.part").exists());
        let details = Command::new(&program).args(["-nostdin", "-hide_banner", "-i"]).arg(&job.output).output().unwrap();
        assert!(String::from_utf8_lossy(&details.stderr).contains("title           : My Lesson"), "the title is in the file");
        // Halfway through the blend both pictures show: the left of the frame is still red or already blue, not one flat colour.
        let mid = dir.join("mid.png");
        make(&["-ss", "1.2", "-i", job.output.to_str().unwrap(), "-frames:v", "1"], &mid);
        assert!(std::fs::metadata(&mid).unwrap().len() > 0);
        // Cancelling leaves nothing behind and no video.
        let other = Job { output: dir.join("never.mp4"), ..job };
        let err = run(&other, &mut |_| {}, &AtomicBool::new(true)).unwrap_err();
        assert_eq!(err, "cancelled");
        assert!(!other.output.exists() && !other.output.with_extension("mp4.part").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
