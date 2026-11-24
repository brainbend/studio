//! The list panels: the Slide List of a media lesson and the Outline of a web lesson.
//!
//! Both show their items three ways, and the choice is remembered for each panel:
//! - **Large**: a thumbnail with the name and a line of detail beside it,
//! - **Icons**: a small icon and the name,
//! - **Text**: the name alone.
//!
//! This file holds what the two panels share: the modes, the switch for them, and one row that draws in any mode. What the rows are (slides,
//! sections, blocks) and what clicking them does stays with each panel.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ListMode { Large, Icon, Text }

impl ListMode {
    pub const ALL: [ListMode; 3] = [ListMode::Large, ListMode::Icon, ListMode::Text];

    fn key(self) -> &'static str {
        match self { ListMode::Large => "large", ListMode::Icon => "icons", ListMode::Text => "text" }
    }

    pub fn label(self) -> &'static str {
        match self { ListMode::Large => "Large", ListMode::Icon => "Icons", ListMode::Text => "Text" }
    }

    fn tip(self) -> &'static str {
        match self {
            ListMode::Large => "Large thumbnails with the name and a line of detail",
            ListMode::Icon => "A small icon and the name",
            ListMode::Text => "The names only",
        }
    }

    /// The glyph of the button for this mode.
    fn glyph(self) -> char {
        match self { ListMode::Large => ribbon::FRAME, ListMode::Icon => ribbon::LIST_BULLETS, ListMode::Text => ribbon::ALIGN_L }
    }

    pub fn row_height(self) -> f32 {
        match self { ListMode::Large => 68.0, ListMode::Icon => 30.0, ListMode::Text => 24.0 }
    }

    /// The mode a panel was left in (Large the first time).
    pub fn load(panel: &str) -> ListMode {
        if cfg!(test) { return ListMode::Large; }
        let saved = prefs::get_str(&format!("list_mode_{panel}"));
        ListMode::ALL.into_iter().find(|m| Some(m.key()) == saved.as_deref()).unwrap_or(ListMode::Large)
    }

    pub fn save(self, panel: &str) { if cfg!(test) { return; } prefs::set_str(&format!("list_mode_{panel}"), self.key()); }
}

/// The switch between the three modes, at the top of a list panel. True when it changed.
pub(crate) fn mode_bar(ui: &mut egui::Ui, mode: &mut ListMode, panel: &str) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for m in ListMode::ALL {
            let on = *mode == m;
            let button = egui::Button::new(egui::RichText::new(m.glyph().to_string()).family(ribbon::icons()).size(15.0)).selected(on).min_size(egui::vec2(28.0, 24.0));
            let r = ui.add(button).on_hover_text(m.tip());
            r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, m.label()));
            if r.clicked() && !on { *mode = m; m.save(panel); changed = true; }
        }
    });
    changed
}

/// What one row shows.
pub(crate) struct RowSpec<'a> {
    pub icon: char,
    pub title: &'a str,
    /// A line under the title in Large mode (the first words of a block, say).
    pub detail: &'a str,
    /// A number written before the title ("1.2").
    pub number: Option<String>,
    /// How far in the row sits (blocks under their section).
    pub depth: usize,
    pub selected: bool,
    pub bold: bool,
    /// A heading that opens and closes: whether it is open.
    pub caret: Option<bool>,
}

/// What happened to a row.
pub(crate) struct RowOut {
    pub resp: egui::Response,
    /// The little arrow of a heading was pressed.
    pub caret_clicked: bool,
}

/// One row, in the mode given. `thumb` paints the thumbnail of Large mode inside the rectangle it is given.
pub(crate) fn row(ui: &mut egui::Ui, mode: ListMode, spec: &RowSpec, thumb: &mut dyn FnMut(&egui::Painter, egui::Rect)) -> RowOut {
    let h = mode.row_height();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), h), egui::Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    let accent = ribbon::ACCENT;
    if spec.selected {
        painter.rect_filled(rect, 4.0, accent);
    } else if resp.hovered() {
        painter.rect_filled(rect, 4.0, ui.visuals().text_color().gamma_multiply(0.08));
    }
    let fg = if spec.selected { egui::Color32::WHITE } else { ui.visuals().text_color() };
    let weak = if spec.selected { egui::Color32::from_white_alpha(200) } else { ui.visuals().weak_text_color() };
    let mut x = rect.left() + 6.0 + spec.depth as f32 * 14.0;
    let mut caret_clicked = false;
    if let Some(open) = spec.caret {
        let c = egui::Rect::from_min_size(egui::pos2(x - 2.0, rect.top()), egui::vec2(16.0, h));
        // (drawn, not written: the fonts here have no triangle)
        let m = c.center();
        let tri = if open { vec![m + egui::vec2(-4.0, -2.5), m + egui::vec2(4.0, -2.5), m + egui::vec2(0.0, 3.0)] } else { vec![m + egui::vec2(-2.5, -4.0), m + egui::vec2(-2.5, 4.0), m + egui::vec2(3.0, 0.0)] };
        painter.add(egui::Shape::convex_polygon(tri, fg, egui::Stroke::NONE));
        if resp.clicked() && resp.interact_pointer_pos().is_some_and(|p| c.contains(p)) { caret_clicked = true; }
        x += 16.0;
    }
    let title = match &spec.number { Some(n) => format!("{n}  {}", spec.title), None => spec.title.to_string() };
    let font = egui::FontId::proportional(13.0);
    // (There is no bold face here: a heading is drawn twice, a hair apart.)
    let heavy = if spec.bold { 0.6 } else { 0.0 };
    match mode {
        ListMode::Text => {
            let clip = egui::Rect::from_min_max(egui::pos2(x, rect.top()), egui::pos2(rect.right() - 4.0, rect.bottom()));
            let p = painter.with_clip_rect(clip);
            p.text(egui::pos2(x, rect.center().y), egui::Align2::LEFT_CENTER, &title, font.clone(), fg);
            if heavy > 0.0 { p.text(egui::pos2(x + heavy, rect.center().y), egui::Align2::LEFT_CENTER, &title, font, fg); }
        }
        ListMode::Icon => {
            painter.text(egui::pos2(x + 9.0, rect.center().y), egui::Align2::CENTER_CENTER, spec.icon.to_string(), egui::FontId::new(16.0, ribbon::icons()), fg);
            let clip = egui::Rect::from_min_max(egui::pos2(x + 26.0, rect.top()), egui::pos2(rect.right() - 4.0, rect.bottom()));
            let p = painter.with_clip_rect(clip);
            p.text(egui::pos2(x + 26.0, rect.center().y), egui::Align2::LEFT_CENTER, &title, font.clone(), fg);
            if heavy > 0.0 { p.text(egui::pos2(x + 26.0 + heavy, rect.center().y), egui::Align2::LEFT_CENTER, &title, font, fg); }
        }
        ListMode::Large => {
            let tw = ((rect.width() - (x - rect.left()) - 8.0) * 0.45).clamp(48.0, 96.0);
            let t = egui::Rect::from_min_size(egui::pos2(x, rect.top() + 6.0), egui::vec2(tw, h - 12.0));
            thumb(&painter.with_clip_rect(t), t);
            painter.rect_stroke(t, 3.0, egui::Stroke::new(1.0, if spec.selected { egui::Color32::from_white_alpha(160) } else { egui::Color32::from_gray(120) }), egui::StrokeKind::Inside);
            let tx = t.right() + 8.0;
            let wrap = (rect.right() - 4.0 - tx).max(20.0);
            let clip = egui::Rect::from_min_max(egui::pos2(tx, rect.top() + 2.0), egui::pos2(rect.right() - 4.0, rect.bottom() - 2.0));
            let p = painter.with_clip_rect(clip);
            let g = p.layout(title, font, fg, wrap);
            let gh = g.size().y;
            p.galley(egui::pos2(tx, rect.top() + 10.0), g, fg);
            if !spec.detail.is_empty() {
                let d = p.layout(spec.detail.to_string(), egui::FontId::proportional(11.0), weak, wrap);
                p.galley(egui::pos2(tx, rect.top() + 12.0 + gh), d, weak);
            }
        }
    }
    RowOut { resp, caret_clicked }
}

/// A line where a dragged item would land.
pub(crate) fn drop_line(ui: &egui::Ui, rect: egui::Rect, below: bool) {
    let y = if below { rect.bottom() } else { rect.top() };
    ui.painter().line_segment([egui::pos2(rect.left() + 4.0, y), egui::pos2(rect.right() - 4.0, y)], egui::Stroke::new(2.0, ribbon::ACCENT));
}
