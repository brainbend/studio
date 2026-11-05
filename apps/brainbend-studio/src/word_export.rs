//! Putting pictures of the slides in a Word handout.
//!
//! Word cannot draw our slides, and the app is the only thing that can: it paints them with egui. So the handout is made
//! in steps. While a "Preparing" page covers the window, each slide is painted large in turn, the window is captured (egui's
//! screenshot) and the slide's part of it is kept as a PNG. When every slide has one, the .docx is written.
//! Question and Results slides are listed as text in the document, so they are not drawn.
use super::*;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Space around the slide on the Preparing page, in points; the top holds the heading.
const MARGIN: f32 = 24.0;
const HEADING: f32 = 64.0;
/// Frames to show a slide before capturing it, so the pictures and fonts it needs have been loaded.
const SETTLE_FRAMES: u8 = 3;

/// A handout being made: the slides still to draw and the pictures taken so far.
pub struct Capture {
    /// The slides to put in the document (indices into the project's slides).
    slides: Vec<usize>,
    /// The ones to draw, in order: the slides that are Normal.
    todo: Vec<usize>,
    /// How many of `todo` have been drawn and kept.
    done: usize,
    pictures: BTreeMap<usize, Vec<u8>>,
    path: PathBuf,
    /// Frames the current slide has been on show.
    shown: u8,
    /// A screenshot has been asked for and has not come back yet.
    waiting: bool,
    /// Where the slide was painted on the last frame, in points.
    rect: Option<egui::Rect>,
}

/// The part of `image` inside `rect` (in points), as a PNG.
pub fn crop_png(image: &egui::ColorImage, rect: egui::Rect, pixels_per_point: f32) -> Option<Vec<u8>> {
    let [iw, ih] = image.size;
    let x0 = ((rect.min.x * pixels_per_point).round().max(0.0) as usize).min(iw);
    let y0 = ((rect.min.y * pixels_per_point).round().max(0.0) as usize).min(ih);
    let x1 = ((rect.max.x * pixels_per_point).round().max(0.0) as usize).min(iw);
    let y1 = ((rect.max.y * pixels_per_point).round().max(0.0) as usize).min(ih);
    if x1 <= x0 || y1 <= y0 { return None; }
    let (w, h) = (x1 - x0, y1 - y0);
    let mut rgba = Vec::with_capacity(w * h * 4);
    for y in y0..y1 {
        for c in &image.pixels[y * iw + x0..y * iw + x1] {
            // The window is opaque, so the colour needs no un-premultiplying.
            rgba.extend_from_slice(&[c.r(), c.g(), c.b(), 255]);
        }
    }
    let img = image::RgbaImage::from_raw(w as u32, h as u32, rgba)?;
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png).ok()?;
    Some(out.into_inner())
}

impl Capture {
    /// Whether a screenshot has been asked for and not come back yet.
    #[cfg(test)]
    pub(super) fn waiting(&self) -> bool { self.waiting }
}

impl App {
    /// Ask where to save a Word handout, then make it. Slides that can be drawn are drawn first.
    pub(super) fn export_word(&mut self) {
        self.end_state_swap();
        self.leave_master_view();
        let Some(p) = &self.project else { return };
        let slides = if p.is_web() { vec![] } else { self.handout_slides() };
        let Some(path) = rfd::FileDialog::new().add_filter("Word document", &["docx"]).set_file_name(format!("{}.docx", file_safe_name(&p.name))).save_file() else { return };
        self.begin_word_export(slides, path);
    }

    /// Start making the handout for `slides`, to be saved at `path`.
    pub(super) fn begin_word_export(&mut self, slides: Vec<usize>, path: PathBuf) {
        let Some(p) = &self.project else { return };
        let todo: Vec<usize> = slides.iter().copied().filter(|&i| p.slides.get(i).is_some_and(|s| matches!(s.kind, SlideKind::Normal))).collect();
        self.word_capture = Some(Capture { slides, todo, done: 0, pictures: BTreeMap::new(), path, shown: 0, waiting: false, rect: None });
        if self.word_capture.as_ref().is_some_and(|c| c.todo.is_empty()) { self.finish_word_export(); }
    }

    /// Write the document with the pictures taken, and clear the Preparing page.
    fn finish_word_export(&mut self) {
        let Some(c) = self.word_capture.take() else { return };
        let Some(p) = &self.project else { return };
        self.status = match brainbend_export_docx::export(p, &c.slides, &self.handout, &c.pictures) {
            Ok(bytes) => match std::fs::write(&c.path, bytes) {
                Ok(()) => format!("Exported Word handout to {}", c.path.display()),
                Err(e) => format!("Could not write {}: {e}", c.path.display()),
            },
            Err(e) => format!("Word export failed: {e}"),
        };
    }

    /// Run one frame of making the handout: show the next slide big, and take its picture when the window has been captured.
    pub(super) fn word_export_frame(&mut self, ctx: &egui::Context) {
        let Some(c) = self.word_capture.as_mut() else { return };
        // The picture of the slide that was on show comes back as an event.
        let shot = ctx.input(|i| i.events.iter().find_map(|e| match e { egui::Event::Screenshot { image, .. } => Some(image.clone()), _ => None }));
        if let (Some(image), true) = (shot, c.waiting) {
            let index = c.todo[c.done];
            if let Some(png) = c.rect.and_then(|r| crop_png(&image, r, ctx.pixels_per_point())) { c.pictures.insert(index, png); }
            c.done += 1;
            c.shown = 0;
            c.waiting = false;
            c.rect = None;
            if c.done >= c.todo.len() { self.finish_word_export(); return; }
        }
        let Some(c) = self.word_capture.as_ref() else { return };
        let index = c.todo[c.done];
        let (done, total) = (c.done, c.todo.len());
        let Some(slide) = self.project.as_ref().and_then(|p| p.slides.get(index)).cloned() else { self.word_capture = None; return };
        let screen = ctx.content_rect();
        let avail = egui::vec2(screen.width() - 2.0 * MARGIN, screen.height() - HEADING - MARGIN);
        let scale = (avail.x / slide.width).min(avail.y / slide.height).max(0.01);
        let size = egui::vec2(slide.width, slide.height) * scale;
        let rect = egui::Rect::from_min_size(egui::pos2(screen.center().x - size.x / 2.0, screen.top() + HEADING), size);
        let mut cancel = false;
        egui::Area::new(egui::Id::new("word-export-page")).order(egui::Order::Tooltip).fixed_pos(screen.min).show(ctx, |ui| {
            let fill = ui.visuals().panel_fill;
            ui.painter().rect_filled(screen, 0.0, fill);
            ui.allocate_rect(screen, egui::Sense::click_and_drag());
            ui.painter().text(egui::pos2(screen.left() + MARGIN, screen.top() + 20.0), egui::Align2::LEFT_TOP, format!("Preparing the Word handout: slide {} of {}", done + 1, total), egui::FontId::proportional(18.0), ui.visuals().strong_text_color());
            let button = egui::Rect::from_min_size(egui::pos2(screen.right() - MARGIN - 90.0, screen.top() + 14.0), egui::vec2(90.0, 30.0));
            if ui.put(button, egui::Button::new("Cancel")).clicked() { cancel = true; }
            let painter = ui.painter().with_clip_rect(rect);
            self.paint_backdrop(ctx, &painter, rect.min, scale, &slide);
            let layers = start_layers(&slide);
            self.paint_slide(ctx, &painter, rect.min, scale, &slide, &layers, None);
        });
        if cancel { self.word_capture = None; self.status = "Word export cancelled".into(); return; }
        let Some(c) = self.word_capture.as_mut() else { return };
        c.rect = Some(rect);
        if !c.waiting {
            c.shown = c.shown.saturating_add(1);
            if c.shown >= SETTLE_FRAMES {
                c.waiting = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            }
        }
        ctx.request_repaint();
    }

    /// Whether the Preparing page is up (a handout is being made).
    #[allow(dead_code)]
    pub(super) fn word_export_running(&self) -> bool { self.word_capture.is_some() }
}

/// Used by tests: a window-sized image of one colour, standing in for the captured window.
#[cfg(test)]
pub fn plain_image(size: [usize; 2], c: egui::Color32) -> std::sync::Arc<egui::ColorImage> { std::sync::Arc::new(egui::ColorImage::filled(size, c)) }
