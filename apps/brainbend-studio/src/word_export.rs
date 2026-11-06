//! Putting pictures of the slides in a Word or PDF handout.
//!
//! Word cannot draw our slides, and the app is the only thing that can: it paints them with egui. So the handout is made
//! in steps (the PDF handout works the same way). While a "Preparing" page covers the window, each slide is painted large in turn, the window is captured (egui's
//! screenshot) and the slide's part of it is kept as a PNG. When every slide has one, the .docx is written.
//! Question and Results slides are drawn the way the player shows them.
use super::*;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Space around the slide on the Preparing page, in points; the top holds the heading.
const MARGIN: f32 = 24.0;
const HEADING: f32 = 64.0;
/// Frames to show a slide before capturing it, so the pictures and fonts it needs have been loaded.
const SETTLE_FRAMES: u8 = 3;

/// What the pictures are for.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Handout { Word, Pdf }

impl Handout {
    fn name(self) -> &'static str { match self { Handout::Word => "Word", Handout::Pdf => "PDF" } }
}

/// A handout being made: the slides still to draw and the pictures taken so far.
pub struct Capture {
    format: Handout,
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
            // The window is opaque, so the color needs no un-premultiplying.
            rgba.extend_from_slice(&[c.r(), c.g(), c.b(), 255]);
        }
    }
    let img = image::RgbaImage::from_raw(w as u32, h as u32, rgba)?;
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png).ok()?;
    Some(out.into_inner())
}

/// `png` with a grey frame drawn just inside its edge, the same on all four sides. The frame is part of the picture because
/// Word draws a picture's outline unevenly (the top comes out thin and pale).
pub fn frame_png(png: &[u8]) -> Option<Vec<u8>> {
    let mut img = image::load_from_memory(png).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    // About a point and a half on the page, whatever size the picture was taken at.
    let t = (w / 320).max(2).min(w / 2).min(h / 2);
    let c = image::Rgba([102, 102, 102, 255]);
    for y in 0..h {
        for x in 0..w {
            if x < t || y < t || x >= w - t || y >= h - t { img.put_pixel(x, y, c); }
        }
    }
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png).ok()?;
    Some(out.into_inner())
}

impl Capture {
    /// Whether a screenshot has been asked for and not come back yet.
    #[cfg(test)]
    pub(super) fn waiting(&self) -> bool { self.waiting }
    #[cfg(test)]
    pub(super) fn todo_is_empty(&self) -> bool { self.todo.is_empty() }
}

impl App {
    /// Ask where to save a Word handout, then make it. Slides that can be drawn are drawn first.
    pub(super) fn export_word(&mut self) { self.export_handout(Handout::Word); }

    /// Ask where to save a PDF handout, then make it. A Web Lesson has no slides to draw, so it uses the browser's print view
    /// (the Print destination).
    pub(super) fn export_pdf(&mut self) {
        if self.project.as_ref().is_some_and(|p| p.is_web()) { self.print(); return; }
        self.export_handout(Handout::Pdf);
    }

    fn export_handout(&mut self, format: Handout) {
        self.end_state_swap();
        self.leave_master_view();
        let Some(p) = &self.project else { return };
        let slides = if p.is_web() { vec![] } else { self.handout_slides() };
        let (label, ext) = match format { Handout::Word => ("Word document", "docx"), Handout::Pdf => ("PDF document", "pdf") };
        let Some(path) = rfd::FileDialog::new().add_filter(label, &[ext]).set_file_name(format!("{}.{ext}", file_safe_name(&p.name))).save_file() else { return };
        self.begin_word_export(format, slides, path);
    }

    /// Start making the handout for `slides`, to be saved at `path`.
    pub(super) fn begin_word_export(&mut self, format: Handout, slides: Vec<usize>, path: PathBuf) {
        let Some(p) = &self.project else { return };
        let todo: Vec<usize> = slides.iter().copied().filter(|&i| i < p.slides.len()).collect();
        self.word_capture = Some(Capture { format, slides, todo, done: 0, pictures: BTreeMap::new(), path, shown: 0, waiting: false, rect: None });
        if self.word_capture.as_ref().is_some_and(|c| c.todo.is_empty()) { self.finish_word_export(); }
    }

    /// Write the document with the pictures taken, and clear the Preparing page.
    fn finish_word_export(&mut self) {
        let Some(c) = self.word_capture.take() else { return };
        let Some(p) = &self.project else { return };
        let made = match c.format {
            Handout::Word => {
                let framed: BTreeMap<usize, Vec<u8>> = c.pictures.iter().map(|(i, b)| (*i, frame_png(b).unwrap_or_else(|| b.clone()))).collect();
                brainbend_export_docx::export(p, &c.slides, &self.handout, &framed)
            }
            Handout::Pdf => brainbend_export_pdf::export(p, &c.slides, &self.handout, &c.pictures),
        };
        let name = c.format.name();
        self.status = match made {
            Ok(bytes) => match std::fs::write(&c.path, bytes) {
                Ok(()) => format!("Exported {name} handout to {}", c.path.display()),
                Err(e) => format!("Could not write {}: {e}", c.path.display()),
            },
            Err(e) => format!("{name} export failed: {e}"),
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
        let (done, total, c_name) = (c.done, c.todo.len(), c.format.name());
        let Some(slide) = self.project.as_ref().and_then(|p| p.slides.get(index)).cloned() else { self.word_capture = None; return };
        let screen = ctx.content_rect();
        let avail = egui::vec2(screen.width() - 2.0 * MARGIN, screen.height() - HEADING - MARGIN);
        let scale = (avail.x / slide.width).min(avail.y / slide.height).max(0.01);
        let size = egui::vec2(slide.width, slide.height) * scale;
        let rect = egui::Rect::from_min_size(egui::pos2(screen.center().x - size.x / 2.0, screen.top() + HEADING), size);
        let answers = self.handout.correct_answers;
        let mut cancel = false;
        egui::Area::new(egui::Id::new("word-export-page")).order(egui::Order::Tooltip).fixed_pos(screen.min).show(ctx, |ui| {
            let fill = ui.visuals().panel_fill;
            ui.painter().rect_filled(screen, 0.0, fill);
            ui.allocate_rect(screen, egui::Sense::click_and_drag());
            ui.painter().text(egui::pos2(screen.left() + MARGIN, screen.top() + 20.0), egui::Align2::LEFT_TOP, format!("Preparing the {} handout: slide {} of {}", c_name, done + 1, total), egui::FontId::proportional(18.0), ui.visuals().strong_text_color());
            let button = egui::Rect::from_min_size(egui::pos2(screen.right() - MARGIN - 90.0, screen.top() + 14.0), egui::vec2(90.0, 30.0));
            if ui.put(button, egui::Button::new("Cancel")).clicked() { cancel = true; }
            let painter = ui.painter().with_clip_rect(rect);
            if matches!(slide.kind, SlideKind::Normal) {
                self.paint_backdrop(ctx, &painter, rect.min, scale, &slide);
                let layers = start_layers(&slide);
                self.paint_slide(ctx, &painter, rect.min, scale, &slide, &layers, None);
            } else {
                // Question and Results slides look as the player lays them out; the answers show only on a handout that has them.
                painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(244, 246, 249));
                self.paint_player_slide(ctx, &painter, rect.min, scale, &slide, false, answers);
            }
        });
        if cancel { self.word_capture = None; self.status = "Export cancelled".into(); return; }
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

/// Used by tests: a window-sized image of one color, standing in for the captured window.
#[cfg(test)]
pub fn plain_image(size: [usize; 2], c: egui::Color32) -> std::sync::Arc<egui::ColorImage> { std::sync::Arc::new(egui::ColorImage::filled(size, c)) }
