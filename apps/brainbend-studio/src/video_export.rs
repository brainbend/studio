//! Publishing the lesson as a video.
//!
//! Like the handouts, the app is the only thing that can draw its slides, so the video is made in steps. While a
//! "Preparing" page covers the window, each stretch of each slide is painted large in turn and the window is captured
//! (egui's screenshot); the slide's part of it is kept as a picture. When every picture is taken, ffmpeg joins them,
//! with the slides' sound, into an MP4 in the background (see `brainbend-video`).
//! Objects appear and disappear on their timing; effects and motion paths are not played yet. A slide's transition is
//! blended in by ffmpeg between the pictures on either side of it.
use super::*;
use brainbend_video::encode;
use brainbend_video::plan::{self, MediaInfo, Plan, Settings};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;
use std::time::Duration;

/// Space around the slide on the Preparing page, in points; the top holds the heading.
const MARGIN: f32 = 24.0;
const HEADING: f32 = 64.0;
/// Frames to show a stretch before capturing it, so the pictures and fonts it needs have been loaded.
const SETTLE_FRAMES: u8 = 3;

/// What the author chose on the Video page of the Publish dialog.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Options {
    /// Pixels high: 720 or 1080.
    pub height: u32,
    pub fps: u32,
    /// Seconds a slide with no timeline and no sound shows.
    pub hold_secs: u32,
    pub audio: bool,
}

impl Default for Options {
    fn default() -> Self { Self { height: 720, fps: 30, hold_secs: 5, audio: true } }
}

impl Options {
    pub const HEIGHTS: [u32; 2] = [720, 1080];
    pub const RATES: [u32; 2] = [24, 30];

    fn settings(self) -> Settings { Settings { height: self.height, fps: self.fps, hold_ms: self.hold_secs.max(1) * 1000, audio: self.audio } }
}

enum Phase {
    /// Painting and capturing the pictures.
    Capturing,
    /// ffmpeg is joining them, on another thread.
    Encoding { done: Arc<AtomicU64>, cancel: Arc<AtomicBool>, rx: Receiver<Result<(), String>> },
}

/// A video being made.
pub struct VideoRun {
    phase: Phase,
    plan: Plan,
    settings: Settings,
    /// A folder for the pictures and sound files, removed at the end.
    dir: PathBuf,
    output: PathBuf,
    /// The lesson's name, written into the video for players to show.
    title: String,
    size: (u32, u32),
    sounds: Vec<(PathBuf, brainbend_video::plan::Clip)>,
    /// The pictures taken so far, with how long each shows.
    pictures: Vec<(PathBuf, u32)>,
    /// How many of the plan's pictures are done.
    done: usize,
    /// Frames the current picture has been on show.
    shown: u8,
    /// A screenshot has been asked for and has not come back yet.
    waiting: bool,
    /// Where the slide was painted on the last frame, in points.
    rect: Option<egui::Rect>,
}

#[cfg(test)]
impl VideoRun {
    pub(super) fn waiting(&self) -> bool { matches!(self.phase, Phase::Capturing) && self.waiting }
    pub(super) fn encoding(&self) -> bool { matches!(self.phase, Phase::Encoding { .. }) }
    pub(super) fn pictures_taken(&self) -> usize { self.pictures.len() }
    pub(super) fn plan(&self) -> &Plan { &self.plan }
    pub(super) fn work_dir(&self) -> &std::path::Path { &self.dir }
    pub(super) fn title(&self) -> &str { &self.title }
}

/// A file name for a folder of working files that no other run uses.
fn work_dir() -> PathBuf {
    static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("brainbend-video-{}-{n}", std::process::id()))
}

/// The video's size: the slides' shape at the chosen height, in even numbers of pixels as H.264 needs.
fn video_size(slide_w: f32, slide_h: f32, height: u32) -> (u32, u32) {
    let aspect = if slide_h > 0.0 { slide_w / slide_h } else { 16.0 / 9.0 };
    let even = |v: f32| ((v / 2.0).round() as u32 * 2).max(2);
    (even(height as f32 * aspect), even(height as f32))
}

impl App {
    /// Ask where to save the video, then make it.
    pub(super) fn export_video(&mut self) {
        self.end_state_swap();
        self.leave_master_view();
        let Some(p) = &self.project else { return };
        if p.is_web() { return; }
        if brainbend_video::ffmpeg().is_none() {
            self.status = if self.video_support.request("make the video", true) {
                "Studio needs video support to make a video. Download it, then publish again.".into()
            } else {
                "Video is not available on this kind of computer.".into()
            };
            return;
        }
        let slides = self.handout_slides();
        if slides.is_empty() { self.status = "There are no slides to put in the video.".into(); return; }
        let Some(path) = rfd::FileDialog::new().add_filter("MP4 video", &["mp4"]).set_file_name(format!("{}.mp4", file_safe_name(&p.name))).save_file() else { return };
        self.begin_video_export(slides, path);
    }

    /// Start making the video of `slides`, to be saved at `path`.
    pub(super) fn begin_video_export(&mut self, slides: Vec<usize>, path: PathBuf) {
        let Some(p) = self.project.as_ref() else { return };
        let settings = self.video_options.settings();
        let dir = work_dir();
        if let Err(e) = std::fs::create_dir_all(&dir) { self.status = format!("Could not prepare the video: {e}"); return; }
        // The sound has to be a file on disk for ffmpeg to read; each is looked at once, for its length and whether it has sound.
        let mut files: HashMap<String, (PathBuf, MediaInfo)> = HashMap::new();
        if settings.audio {
            for (n, key) in plan::sound_assets(p, &slides).into_iter().enumerate() {
                let Some(asset) = p.assets.get(&key) else { continue };
                let ext = std::path::Path::new(&asset.name).extension().and_then(|e| e.to_str()).unwrap_or("bin");
                let file = dir.join(format!("sound-{n}.{ext}"));
                if std::fs::write(&file, &asset.bytes).is_err() { continue; }
                if let Some(info) = encode::probe(&file) { files.insert(key, (file, info)); }
            }
        }
        let plan = plan::plan(p, &slides, &settings, &|k| files.get(k).map(|(_, i)| *i));
        let Some(first) = plan.shots.first().and_then(|s| p.slides.get(s.slide)) else {
            let _ = std::fs::remove_dir_all(&dir);
            self.status = "There is nothing to put in the video.".into();
            return;
        };
        let size = video_size(first.width, first.height, settings.height);
        let sounds = plan.clips.iter().filter_map(|c| files.get(&c.asset).map(|(f, _)| (f.clone(), c.clone()))).collect();
        // The video's title is the project's name; a project with none takes the name of the file.
        let title = if p.name.trim().is_empty() { path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string() } else { p.name.trim().to_string() };
        self.video_run = Some(VideoRun { phase: Phase::Capturing, plan, settings, dir, output: path, title, size, sounds, pictures: vec![], done: 0, shown: 0, waiting: false, rect: None });
    }

    fn finish_video(&mut self, result: Result<(), String>) {
        let Some(run) = self.video_run.take() else { return };
        let _ = std::fs::remove_dir_all(&run.dir);
        self.status = match result {
            Ok(()) => format!("Published the video to {}", run.output.display()),
            Err(e) if e == "cancelled" => "Video cancelled".into(),
            Err(e) => format!("Could not make the video: {e}"),
        };
    }

    /// Every picture is taken: ffmpeg joins them, on another thread.
    fn start_video_encoding(&mut self) {
        let Some(run) = self.video_run.as_mut() else { return };
        let job = encode::Job {
            pictures: run.pictures.clone(), sounds: run.sounds.clone(),
            width: run.size.0, height: run.size.1, fps: run.settings.fps, total_ms: run.plan.total_ms,
            blends: run.plan.blends.clone(), title: run.title.clone(),
            list: run.dir.join("pictures.txt"), output: run.output.clone(),
        };
        let (done, cancel) = (Arc::new(AtomicU64::new(0)), Arc::new(AtomicBool::new(false)));
        let (d, c) = (done.clone(), cancel.clone());
        let (tx, rx) = channel();
        std::thread::spawn(move || { let _ = tx.send(encode::run(&job, &mut |ms| d.store(ms, Ordering::Relaxed), &c)); });
        run.phase = Phase::Encoding { done, cancel, rx };
    }

    /// Run one frame of making the video: show the next stretch big and take its picture when the window has been
    /// captured; once all are taken, wait for the encoder.
    pub(super) fn video_export_frame(&mut self, ctx: &egui::Context) {
        let Some(run) = self.video_run.as_mut() else { return };
        let screen = ctx.content_rect();
        if let Phase::Encoding { done, cancel, rx } = &run.phase {
            match rx.try_recv() {
                Ok(result) => { self.finish_video(result); return; }
                Err(TryRecvError::Disconnected) => { self.finish_video(Err("the video program stopped unexpectedly".into())); return; }
                Err(TryRecvError::Empty) => {}
            }
            let frac = (done.load(Ordering::Relaxed) as f32 / run.plan.total_ms.max(1) as f32).min(1.0);
            let mut stop = false;
            egui::Area::new(egui::Id::new("video-export-page")).order(egui::Order::Tooltip).fixed_pos(screen.min).show(ctx, |ui| {
                let fill = ui.visuals().panel_fill;
                ui.painter().rect_filled(screen, 0.0, fill);
                ui.allocate_rect(screen, egui::Sense::click_and_drag());
                ui.painter().text(egui::pos2(screen.left() + MARGIN, screen.top() + 20.0), egui::Align2::LEFT_TOP, "Making the video", egui::FontId::proportional(18.0), ui.visuals().strong_text_color());
                let bar = egui::Rect::from_min_size(egui::pos2(screen.left() + MARGIN, screen.top() + 64.0), egui::vec2((screen.width() - 2.0 * MARGIN).min(520.0), 22.0));
                ui.put(bar, egui::ProgressBar::new(frac).text(format!("{:.0}%", frac * 100.0)));
                let button = egui::Rect::from_min_size(egui::pos2(screen.right() - MARGIN - 90.0, screen.top() + 14.0), egui::vec2(90.0, 30.0));
                if ui.put(button, egui::Button::new("Cancel")).clicked() { stop = true; }
            });
            if stop { cancel.store(true, Ordering::Relaxed); }
            ctx.request_repaint_after(Duration::from_millis(100));
            return;
        }
        // The picture of the stretch that was on show comes back as an event.
        let shot = ctx.input(|i| i.events.iter().find_map(|e| match e { egui::Event::Screenshot { image, .. } => Some(image.clone()), _ => None }));
        if let (Some(image), true) = (shot, run.waiting) {
            let length = run.plan.shots[run.done].length_ms;
            let png = run.rect.and_then(|r| word_export::crop_png(&image, r, ctx.pixels_per_point()));
            let file = run.dir.join(format!("picture-{:04}.png", run.done));
            match png.map(|b| std::fs::write(&file, b)) {
                Some(Ok(())) => run.pictures.push((file, length)),
                _ => { self.finish_video(Err("could not take a picture of a slide".into())); return; }
            }
            run.done += 1;
            run.shown = 0;
            run.waiting = false;
            run.rect = None;
            if run.done >= run.plan.shots.len() { self.start_video_encoding(); ctx.request_repaint(); return; }
        }
        let Some(run) = self.video_run.as_ref() else { return };
        let (done, total) = (run.done, run.plan.shots.len());
        let shot = run.plan.shots[done].clone();
        let Some(mut slide) = self.project.as_ref().and_then(|p| p.slides.get(shot.slide)).cloned() else { self.finish_video(Err("a slide went missing".into())); return };
        let avail = egui::vec2(screen.width() - 2.0 * MARGIN, screen.height() - HEADING - MARGIN);
        let scale = (avail.x / slide.width).min(avail.y / slide.height).max(0.01);
        let size = egui::vec2(slide.width, slide.height) * scale;
        let rect = egui::Rect::from_min_size(egui::pos2(screen.center().x - size.x / 2.0, screen.top() + HEADING), size);
        let normal = matches!(slide.kind, SlideKind::Normal);
        let layers = slide.start_layers();
        if normal {
            // What shows at this moment: objects whose turn on the timeline it is, and that do not start hidden.
            for &li in &layers { slide.layers[li].objects.retain(|o| plan::drawn_at(o, shot.at_ms)); }
        }
        let mut cancel = false;
        egui::Area::new(egui::Id::new("video-export-page")).order(egui::Order::Tooltip).fixed_pos(screen.min).show(ctx, |ui| {
            let fill = ui.visuals().panel_fill;
            ui.painter().rect_filled(screen, 0.0, fill);
            ui.allocate_rect(screen, egui::Sense::click_and_drag());
            ui.painter().text(egui::pos2(screen.left() + MARGIN, screen.top() + 20.0), egui::Align2::LEFT_TOP, format!("Preparing the video: picture {} of {}", done + 1, total), egui::FontId::proportional(18.0), ui.visuals().strong_text_color());
            let button = egui::Rect::from_min_size(egui::pos2(screen.right() - MARGIN - 90.0, screen.top() + 14.0), egui::vec2(90.0, 30.0));
            if ui.put(button, egui::Button::new("Cancel")).clicked() { cancel = true; }
            let painter = ui.painter().with_clip_rect(rect);
            if normal {
                self.paint_backdrop(ctx, &painter, rect.min, scale, &slide);
                self.paint_slide(ctx, &painter, rect.min, scale, &slide, &layers, None);
            } else {
                // Question and Results slides look as the player lays them out.
                painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(244, 246, 249));
                self.paint_player_slide(ctx, &painter, rect.min, scale, &slide, false, false);
            }
        });
        if cancel { self.finish_video(Err("cancelled".into())); return; }
        let Some(run) = self.video_run.as_mut() else { return };
        run.rect = Some(rect);
        if !run.waiting {
            run.shown = run.shown.saturating_add(1);
            if run.shown >= SETTLE_FRAMES {
                run.waiting = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            }
        }
        ctx.request_repaint();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_video_is_the_shape_of_the_slides_in_even_pixels() {
        assert_eq!(video_size(1280.0, 720.0, 720), (1280, 720));
        assert_eq!(video_size(1280.0, 720.0, 1080), (1920, 1080));
        assert_eq!(video_size(1024.0, 768.0, 720), (960, 720));
        assert_eq!(video_size(1000.0, 1000.0, 1080), (1080, 1080));
        assert_eq!(video_size(0.0, 0.0, 720), (1280, 720), "a slide with no size gets the usual shape");
    }

    #[test]
    fn the_options_become_planner_settings() {
        let s = Options { height: 1080, fps: 24, hold_secs: 0, audio: false }.settings();
        assert_eq!((s.height, s.fps, s.hold_ms, s.audio), (1080, 24, 1000, false), "a slide always shows at least a second");
    }
}
