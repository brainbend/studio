//! Video support: asking before the video program (ffmpeg) is downloaded, the first time a feature needs it.
//!
//! The program is not part of Studio's own download. The picture of a video and, later, Publish as video need it, so
//! when one of them finds there is none, a window explains what it is and how big it is, and fetches it only if the
//! author says yes (see `brainbend-video` for how the download is checked). Saying "Not now" is remembered until Studio
//! is closed, so opening several files does not ask again and again; the buttons that need it ask again when pressed.
use std::sync::atomic::Ordering;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;
use std::time::Duration;

use brainbend_doc::{Kind, Project};
use eframe::egui;

use crate::update::Progress;

pub enum Phase {
    Idle,
    /// The window asks whether to download.
    Asking,
    Downloading { progress: Arc<Progress>, rx: Receiver<Result<(), String>> },
    Failed(String),
}

pub struct VideoSupport {
    pub phase: Phase,
    /// What the program is wanted for, finishing "Studio needs it to …".
    pub why: String,
    /// "Not now" was chosen for this run.
    pub declined: bool,
}

impl Default for VideoSupport {
    fn default() -> Self { Self { phase: Phase::Idle, why: String::new(), declined: false } }
}

impl VideoSupport {
    /// Opens the window if the program is missing and can be downloaded. `force` asks even after "Not now" (for a button
    /// the author pressed). Returns whether it asked.
    pub fn request(&mut self, why: &str, force: bool) -> bool {
        if brainbend_video::needs_download().is_none() { return false; }
        self.ask(why, force)
    }

    fn ask(&mut self, why: &str, force: bool) -> bool {
        if (self.declined && !force) || !matches!(self.phase, Phase::Idle | Phase::Failed(_)) { return false; }
        self.why = why.to_string();
        self.phase = Phase::Asking;
        true
    }

    /// Starts the download in the background.
    pub fn start(&mut self, ctx: &egui::Context) {
        if !matches!(self.phase, Phase::Asking | Phase::Failed(_)) { return; }
        let progress = Arc::new(Progress::default());
        if let Some(c) = brainbend_video::component() { progress.total.store(c.size, Ordering::Relaxed); }
        let (tx, rx) = channel();
        let (p, ctx) = (progress.clone(), ctx.clone());
        std::thread::spawn(move || {
            let result = brainbend_video::download(&mut |done, total| {
                p.done.store(done, Ordering::Relaxed);
                p.total.store(total, Ordering::Relaxed);
                ctx.request_repaint();
            });
            let _ = tx.send(result.map(|_| ()));
            ctx.request_repaint();
        });
        self.phase = Phase::Downloading { progress, rx };
    }

    /// Picks up a finished download. Call once a frame; returns true on the frame the program became ready.
    pub fn poll(&mut self) -> bool {
        let Phase::Downloading { rx, .. } = &self.phase else { return false };
        match rx.try_recv() {
            Ok(Ok(())) => { self.phase = Phase::Idle; true }
            Ok(Err(e)) => { self.phase = Phase::Failed(e); false }
            Err(TryRecvError::Empty) => false,
            Err(TryRecvError::Disconnected) => { self.phase = Phase::Failed("the download stopped unexpectedly".into()); false }
        }
    }

    pub fn window_open(&self) -> bool { !matches!(self.phase, Phase::Idle) }
}

/// Whether the project has a video with no picture to show before it plays.
pub fn has_video_without_picture(p: &Project) -> bool {
    p.slides.iter().flat_map(|s| s.layers.iter()).flat_map(|l| l.objects.iter()).any(|o| matches!(o.kind, Kind::Video) && o.poster.is_none() && o.asset.is_some())
}

fn megabytes(bytes: u64) -> String { format!("{:.0} MB", bytes as f64 / 1_000_000.0) }

impl super::App {
    /// Offers the download when the open project has videos that cannot get a picture without it.
    pub(super) fn offer_video_support_for_open_project(&mut self) {
        if self.project.as_ref().is_some_and(has_video_without_picture) {
            self.video_support.request("show a picture of each video before it plays", false);
        }
    }

    /// Gives the videos that have no picture one, after the program has arrived.
    fn fill_missing_pictures(&mut self) {
        self.status = "Video support is ready.".into();
        if !self.project.as_ref().is_some_and(has_video_without_picture) { return; }
        self.snapshot();
        let made = self.project.as_mut().map_or(0, brainbend_import_legacy::add_stills);
        if made > 0 {
            self.dirty = true;
            self.status = format!("Video support is ready. Made a picture for {made} video{}.", if made == 1 { "" } else { "s" });
        }
    }

    pub(super) fn video_support_window(&mut self, ctx: &egui::Context) {
        if self.video_support.poll() { self.fill_missing_pictures(); }
        if !self.video_support.window_open() { return; }
        #[derive(PartialEq)]
        enum Act { Download, NotNow, Close }
        let mut act: Option<Act> = None;
        let mut open = true;
        let size = brainbend_video::component().map(|c| megabytes(c.size)).unwrap_or_default();
        egui::Window::new("Video support").open(&mut open).collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
            ui.set_width(400.0);
            match &self.video_support.phase {
                Phase::Downloading { progress, .. } => {
                    let (done, total) = (progress.done.load(Ordering::Relaxed), progress.total.load(Ordering::Relaxed));
                    let frac = if total > 0 { (done as f32 / total as f32).min(1.0) } else { 0.0 };
                    ui.label("Downloading the video program…");
                    ui.add(egui::ProgressBar::new(frac).text(format!("{:.1} of {:.1} MB", done as f64 / 1_000_000.0, total as f64 / 1_000_000.0)));
                    ctx.request_repaint_after(Duration::from_millis(100));
                }
                phase => {
                    if let Phase::Failed(error) = phase {
                        ui.colored_label(egui::Color32::from_rgb(220, 70, 70), format!("The download did not finish: {error}"));
                        ui.add_space(6.0);
                    }
                    ui.strong(format!("Studio needs a video program to {}.", self.video_support.why));
                    ui.add_space(4.0);
                    ui.label(format!(
                        "It is ffmpeg, a free program ({size}). Studio downloads it once from github.com/brainbend/studio-components, \
                         checks it, and keeps it on this computer. It is licensed under the GPL and Studio runs it as a separate program."));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let label = if matches!(phase, Phase::Failed(_)) { "Try again" } else { "Download" };
                        if ui.button(label).clicked() { act = Some(Act::Download); }
                        if ui.button("Not now").on_hover_text("You can choose a picture for a video yourself instead.").clicked() { act = Some(Act::NotNow); }
                    });
                }
            }
        });
        if !open && !matches!(self.video_support.phase, Phase::Downloading { .. }) { act = Some(Act::Close); }
        match act {
            Some(Act::Download) => self.video_support.start(ctx),
            Some(Act::NotNow) => { self.video_support.declined = true; self.video_support.phase = Phase::Idle; }
            Some(Act::Close) => self.video_support.phase = Phase::Idle,
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asking_once_and_not_again_after_not_now_unless_a_button_asks() {
        let mut v = VideoSupport::default();
        assert!(v.ask("make a picture", false));
        assert!(matches!(v.phase, Phase::Asking));
        v.declined = true;
        v.phase = Phase::Idle;
        assert!(!v.ask("make a picture", false), "an import does not ask again after Not now");
        assert!(v.ask("make a picture", true), "a button the author pressed does");
        assert!(!v.ask("something else", true), "but not while the window is already up");
        assert_eq!(v.why, "make a picture");
    }

    #[test]
    fn a_finished_download_and_a_failed_one_are_picked_up() {
        let mut v = VideoSupport::default();
        let (tx, rx) = channel();
        v.phase = Phase::Downloading { progress: Arc::new(Progress::default()), rx };
        assert!(!v.poll(), "nothing yet");
        tx.send(Ok(())).unwrap();
        assert!(v.poll() && !v.window_open(), "ready, and the window closes");
        let (tx, rx) = channel();
        v.phase = Phase::Downloading { progress: Arc::new(Progress::default()), rx };
        tx.send(Err("no connection".into())).unwrap();
        assert!(!v.poll());
        assert!(matches!(&v.phase, Phase::Failed(e) if e == "no connection"));
        assert!(v.ask("make a picture", false), "it can be asked again after a failure");
    }
}
