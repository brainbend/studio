//! Windows: the app draws its own caption bar, as Office and Storyline do. The window has no frame of the system's, so the top row holds the
//! app's icon with Save, Undo, Redo and Find beside it, the name of the app and the file centered, and the minimize, maximize and close
//! buttons at the right, with the theme toggle, Try It, Preview and Publish hanging from the top edge to their left, as on a Mac. The menus are
//! the row below (see `window_bar`).
//!
//! Without a frame the window is dragged by this row (a double-click maximizes it) and resized from a thin border all round, which is done here
//! too. Setting `BRAINBEND_NATIVE_FRAME` brings the system's frame back, with the one bar under it.
use eframe::egui;
use egui::viewport::ResizeDirection as Dir;

use crate::App;

/// The height of the caption bar and the width of each of its buttons.
pub(crate) const CAPTION_H: f32 = 40.0;
const BUTTON_W: f32 = 46.0;
/// How far in from the window's edge a press starts a resize.
const EDGE: f32 = 6.0;

/// Whether the app draws its own caption bar (Windows, unless the system's frame is asked for).
pub(crate) fn wanted() -> bool { cfg!(windows) && std::env::var_os("BRAINBEND_NATIVE_FRAME").is_none() }

impl App {
    pub(crate) fn caption_bar(&mut self, root: &mut egui::Ui, ctx: &egui::Context) {
        let bg = root.visuals().panel_fill;
        let ink = root.visuals().text_color();
        let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        let icon = self.app_icon(ctx);
        egui::Panel::top("caption").frame(egui::Frame::new().fill(bg).inner_margin(egui::Margin { left: 10, right: 0, top: 0, bottom: 0 })).show_separator_line(false).show(root, |ui| {
            let bar = ui.max_rect();
            // Drag the window by what is empty; double-click maximizes it.
            let drag = ui.interact(egui::Rect::from_min_size(bar.min, egui::vec2(bar.width(), CAPTION_H)), egui::Id::new("caption-drag"), egui::Sense::click_and_drag());
            if drag.drag_started() { ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag); }
            if drag.double_clicked() { ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized)); }
            let (mut left_end, mut right_start) = (bar.left(), bar.right());
            ui.horizontal(|ui| {
                ui.set_min_height(CAPTION_H);
                ui.spacing_mut().item_spacing.x = 4.0;
                if let Some(tex) = &icon { ui.add(egui::Image::new((tex.id(), egui::vec2(20.0, 20.0)))); ui.add_space(6.0); }
                if self.project.is_some() { ui.scope(|ui| { crate::ribbon::flat(ui); self.quick_icons(ui, true); }); }
                left_end = ui.min_rect().right();
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    if caption_button(ui, Glyph::Close, "Close").clicked() { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
                    if caption_button(ui, if maximized { Glyph::Restore } else { Glyph::Maximize }, if maximized { "Restore" } else { "Maximize" }).clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                    }
                    if caption_button(ui, Glyph::Minimize, "Minimize").clicked() { ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true)); }
                    // The theme toggle, Try It, Preview and Publish hang from the top edge, to the left of the window's buttons.
                    right_start = self.bar_actions(ui, ctx, true, 8.0).left();
                });
            });
            // The name of the app and the file is centered on the window, and gives way (with an ellipsis) when the buttons come close.
            let span = (right_start - 12.0) - (left_end + 12.0);
            if span > 40.0 {
                let mut job = egui::text::LayoutJob::simple(self.window_title(), egui::FontId::proportional(13.0), ink, span);
                job.wrap.max_rows = 1;
                job.wrap.break_anywhere = true;
                job.wrap.overflow_character = Some('…');
                let galley = ui.fonts_mut(|f| f.layout_job(job));
                let w = galley.size().x;
                let x = (bar.center().x - w / 2.0).clamp(left_end + 12.0, (right_start - 12.0 - w).max(left_end + 12.0));
                ui.painter().galley(egui::pos2(x, bar.top() + CAPTION_H / 2.0 - galley.size().y / 2.0), galley, ink);
            }
        });
    }

    /// The app's icon as a small picture, made the first time it is asked for.
    fn app_icon(&mut self, ctx: &egui::Context) -> Option<egui::TextureHandle> {
        if self.app_icon.is_none() {
            let img = image::load_from_memory(include_bytes!("../../../assets/brainbend-icon-256.png")).ok()?;
            let small = image::imageops::resize(&img.to_rgba8(), 64, 64, image::imageops::FilterType::Lanczos3);
            let color = egui::ColorImage::from_rgba_unmultiplied([64, 64], small.as_raw());
            self.app_icon = Some(ctx.load_texture("app-icon", color, egui::TextureOptions::LINEAR));
        }
        self.app_icon.clone()
    }

    /// A frameless window has no edge of its own to take hold of: a press near its border starts a resize, and the pointer shows which way.
    pub(crate) fn resize_edges(&self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().maximized.unwrap_or(false) || i.viewport().fullscreen.unwrap_or(false)) { return; }
        let rect = ctx.content_rect();
        // A thin line round the window, so its edge shows against the desktop.
        ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("window-edge")))
            .rect_stroke(rect, 0.0, egui::Stroke::new(1.0, egui::Color32::from_gray(110)), egui::StrokeKind::Inside);
        let Some(p) = ctx.input(|i| i.pointer.hover_pos()) else { return };
        let (l, r, t, b) = (p.x - rect.left() < EDGE, rect.right() - p.x < EDGE, p.y - rect.top() < EDGE, rect.bottom() - p.y < EDGE);
        let (dir, cursor) = match (l, r, t, b) {
            (true, _, true, _) => (Dir::NorthWest, egui::CursorIcon::ResizeNorthWest),
            (_, true, true, _) => (Dir::NorthEast, egui::CursorIcon::ResizeNorthEast),
            (true, _, _, true) => (Dir::SouthWest, egui::CursorIcon::ResizeSouthWest),
            (_, true, _, true) => (Dir::SouthEast, egui::CursorIcon::ResizeSouthEast),
            (true, ..) => (Dir::West, egui::CursorIcon::ResizeWest),
            (_, true, ..) => (Dir::East, egui::CursorIcon::ResizeEast),
            (_, _, true, _) => (Dir::North, egui::CursorIcon::ResizeNorth),
            (_, _, _, true) => (Dir::South, egui::CursorIcon::ResizeSouth),
            _ => return,
        };
        ctx.set_cursor_icon(cursor);
        if ctx.input(|i| i.pointer.primary_pressed()) { ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(dir)); }
    }
}

#[derive(Clone, Copy)]
enum Glyph { Minimize, Maximize, Restore, Close }

/// One of the three buttons at the right of the bar, drawn the way Windows draws them: a thin glyph on a square that lights up under the pointer
/// (the close button in red).
fn caption_button(ui: &mut egui::Ui, glyph: Glyph, label: &str) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(BUTTON_W, CAPTION_H), egui::Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    let hot = resp.hovered();
    let red = matches!(glyph, Glyph::Close);
    if hot { ui.painter().rect_filled(rect, 0.0, if red { egui::Color32::from_rgb(196, 43, 28) } else { ui.visuals().text_color().gamma_multiply(0.12) }); }
    let ink = if hot && red { egui::Color32::WHITE } else { ui.visuals().text_color() };
    let stroke = egui::Stroke::new(1.0, ink);
    let c = rect.center();
    let p = ui.painter();
    match glyph {
        Glyph::Minimize => { p.line_segment([c + egui::vec2(-5.0, 0.0), c + egui::vec2(5.0, 0.0)], stroke); }
        Glyph::Maximize => { p.rect_stroke(egui::Rect::from_center_size(c, egui::vec2(10.0, 10.0)), 0.0, stroke, egui::StrokeKind::Middle); }
        Glyph::Restore => {
            p.rect_stroke(egui::Rect::from_center_size(c + egui::vec2(-1.0, 1.0), egui::vec2(8.0, 8.0)), 0.0, stroke, egui::StrokeKind::Middle);
            p.line_segment([c + egui::vec2(-1.0, -4.0), c + egui::vec2(5.0, -4.0)], stroke);
            p.line_segment([c + egui::vec2(5.0, -4.0), c + egui::vec2(5.0, 2.0)], stroke);
        }
        Glyph::Close => {
            p.line_segment([c + egui::vec2(-5.0, -5.0), c + egui::vec2(5.0, 5.0)], stroke);
            p.line_segment([c + egui::vec2(-5.0, 5.0), c + egui::vec2(5.0, -5.0)], stroke);
        }
    }
    resp
}
