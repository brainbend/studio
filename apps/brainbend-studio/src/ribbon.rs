//! The ribbon: tabbed, icon-led commands above the canvas (Home / Insert / Slides / Design / Transitions / Animations / View).
//!
//! Object property controls edit a *copy* of the primary selected object; whichever fields the user
//! changed are then written to every selected object, so formatting works on multi-selections too.
use super::*;

// Phosphor icon codepoints (regular weight).
pub const UNDO: char = '\u{e024}';
pub const REDO: char = '\u{e026}';
pub const COPY: char = '\u{e1ca}';
pub const TABLE: char = '\u{e476}';
pub const TRASH: char = '\u{e4a6}';
pub const BOLD: char = '\u{e5be}';
pub const ITALIC: char = '\u{e5c0}';
pub const UNDERLINE: char = '\u{e5c4}';
pub const LINK: char = '\u{e2e2}';
pub const SIGMA: char = '\u{eab8}';
pub const SEARCH: char = '\u{e30c}';
pub const PAINTER: char = '\u{e590}';
pub const CLEAR_FMT: char = '\u{e488}';
pub const JUSTIFY: char = '\u{e482}';
pub const FUNCTION: char = '\u{ebe4}';
pub const PIN: char = '\u{e316}';
pub const BOLT: char = '\u{e2de}';
pub const GAUGE: char = '\u{e628}';
pub const HASH: char = '\u{e2a2}';
pub const STAR: char = '\u{e46a}';
pub const CHAT: char = '\u{e15c}';
pub const GLOBE: char = '\u{e288}';
pub const PANORAMA: char = '\u{eaa2}';
pub const STRIKE: char = '\u{e5c2}';
pub const HIGHLIGHT: char = '\u{e3b2}';
pub const LINE_SPACING: char = '\u{e098}';
pub const ROTATE: char = '\u{e036}';
pub const SPACING: char = '\u{e0a0}';
pub const INDENT_OUT: char = '\u{e062}';
pub const INDENT_IN: char = '\u{e064}';
pub const SPELL: char = '\u{e182}';
pub const ALIGN_L: char = '\u{e484}';
pub const ALIGN_C: char = '\u{e480}';
pub const ALIGN_R: char = '\u{e486}';
pub const TEXT_T: char = '\u{e48a}';
pub const RECT: char = '\u{e3f0}';
pub const CIRCLE: char = '\u{e18a}';
pub const LINE: char = '\u{e6d2}';
pub const CLICK: char = '\u{e7c8}';
pub const IMAGE: char = '\u{e2ca}';
pub const CROP: char = '\u{e1d4}';
pub const NODES: char = '\u{e6ce}';
pub const POLYLINE: char = '\u{e6d4}';
pub const ARROW_COUNTER: char = '\u{e038}';
pub const VIDEO: char = '\u{e4da}';
pub const SPEAKER: char = '\u{e44a}';
pub const STACK: char = '\u{e466}';
pub const BUCKET: char = '\u{e392}';
pub const DROP: char = '\u{e210}';
pub const PLAY: char = '\u{e3d0}';
pub const EXPORT: char = '\u{eaf0}';
pub const EYE_OFF: char = '\u{e224}';
pub const TIMER: char = '\u{e492}';
pub const CHECK: char = '\u{e186}';
pub const X: char = '\u{e4f6}';
pub const QUESTION: char = '\u{e3e8}';
pub const FRONT: char = '\u{e462}';
pub const BACK: char = '\u{e468}';
pub const AL_LEFT: char = '\u{e50e}';
pub const AL_CENTER_H: char = '\u{e50a}';
pub const AL_RIGHT: char = '\u{e510}';
pub const AL_TOP: char = '\u{e512}';
pub const AL_CENTER_V: char = '\u{e50c}';
pub const AL_BOTTOM: char = '\u{e506}';
pub const DIST_H: char = '\u{e534}';
pub const DIST_V: char = '\u{e536}';
pub const SQUARE_BORDER: char = '\u{e3f0}';

pub const FILE_PLUS: char = '\u{e236}';
pub const SUN: char = '\u{e472}';
pub const SAVE: char = '\u{e248}';
pub const FOLDER: char = '\u{e256}';
pub const CARET: char = '\u{e136}';
pub const PASTE: char = '\u{e196}';
pub const CUT: char = '\u{eae0}';
pub const MAGNIFY: char = '\u{e30c}';
pub const ACCESS: char = '\u{e4e8}';
pub const MOON: char = '\u{e330}';
pub const BOOK_OPEN: char = '\u{e0e6}';
pub const BUG: char = '\u{e5f4}';
pub const DISCORD: char = '\u{e61a}';
pub const REFRESH: char = '\u{e094}';
pub const WARNING_CIRCLE: char = '\u{e4e2}';
pub const ARROW_UP: char = '\u{e08e}';
pub const ARROW_DOWN: char = '\u{e03e}';
pub const SLIDERS: char = '\u{e434}';
pub const LIST_NUM: char = '\u{e2f6}';
pub const LIST_BULLETS: char = '\u{e2f2}';
pub const RADIO: char = '\u{eb08}';
pub const TEXTBOX: char = '\u{eb0a}';
pub const SHAPES: char = '\u{e69c}';
pub const FILM: char = '\u{e792}';
pub const FRAME: char = '\u{e626}';
pub const PEN: char = '\u{e3ac}';
pub const TEXT_AA: char = '\u{e6ee}';

pub fn icons() -> egui::FontFamily {
    egui::FontFamily::Name("icons".into())
}

fn icon_text(c: char, size: f32) -> egui::RichText {
    egui::RichText::new(c.to_string()).family(icons()).size(size)
}

const PALETTE: [(u8, u8, u8); 18] = [
    (255, 255, 255), (242, 242, 242), (191, 191, 191), (127, 127, 127), (64, 64, 64), (0, 0, 0),
    (3, 83, 158), (0, 120, 215), (0, 130, 120), (60, 160, 70), (240, 180, 0), (240, 120, 30),
    (200, 40, 40), (200, 60, 140), (110, 70, 170), (20, 30, 60), (180, 225, 230), (250, 230, 180),
];

/// Corner radius used throughout the ribbon (card, buttons, fields).
pub const RIBBON_RADIUS: f32 = 4.0;

/// Make ordinary buttons flat: no background or outline until hovered, like the Office ribbon.
pub(super) fn flat(ui: &mut egui::Ui) {
    let r = egui::CornerRadius::same(RIBBON_RADIUS as u8);
    let w = &mut ui.style_mut().visuals.widgets;
    for st in [&mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] { st.corner_radius = r; }
    w.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
    w.inactive.bg_stroke = egui::Stroke::NONE;
    w.noninteractive.bg_stroke = egui::Stroke::NONE;
}

/// Run `add` with the normal (framed) widget look, for inputs that should read as fields.
pub(super) fn framed(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    ui.scope(|ui| {
        let fill = ui.visuals().widgets.inactive.bg_fill;
        let edge = ui.visuals().widgets.hovered.bg_stroke.color.gamma_multiply(0.6);
        let w = &mut ui.style_mut().visuals.widgets;
        w.inactive.weak_bg_fill = fill;
        w.inactive.bg_stroke = egui::Stroke::new(1.0, edge);
        add(ui);
    });
}

fn text_col(ui: &egui::Ui) -> egui::Color32 {
    if ui.is_enabled() { ui.visuals().text_color() } else { ui.visuals().weak_text_color() }
}

/// The logo's yellow and purple. Blue marks what is selected; these mark the main actions and what is hot.
pub const YELLOW: egui::Color32 = egui::Color32::from_rgb(255, 205, 0);
pub const PURPLE: egui::Color32 = egui::Color32::from_rgb(110, 70, 200);

/// The main-action color: yellow on the dark theme, purple on the light one, where yellow would not stand out.
pub fn primary(ui: &egui::Ui) -> egui::Color32 { if ui.visuals().dark_mode { YELLOW } else { PURPLE } }

/// Text on `primary`: near-black on the yellow, white on the purple.
pub fn on_primary(ui: &egui::Ui) -> egui::Color32 { if ui.visuals().dark_mode { egui::Color32::from_rgb(28, 28, 30) } else { egui::Color32::WHITE } }

/// The one blue of the app: selected tabs and ribbon items, buttons, the selected slide and layer, selection handles,
/// and the File menu's list. White text on it reads clearly.
pub const ACCENT: egui::Color32 = egui::Color32::from_rgb(10, 110, 235);
/// The accent under the pointer.
pub const ACCENT_HOVER: egui::Color32 = egui::Color32::from_rgb(52, 140, 250);

/// Icon colors give the ribbon its pop; each icon keeps one hue across the app.
pub fn tint(icon: char) -> egui::Color32 {
    let c = |r, g, b| egui::Color32::from_rgb(r, g, b);
    match icon {
        PASTE | FILE_PLUS | FRAME | MAGNIFY | ACCESS | FOLDER | SAVE | CLICK => ACCENT,
        TEXTBOX | TEXT_T => c(40, 60, 120),
        IMAGE | PLAY | CHECK => c(48, 160, 100),
        VIDEO | TRASH => c(214, 74, 84),
        SPEAKER | STACK | FILM | SHAPES => c(135, 92, 205),
        RECT | CIRCLE | EXPORT | TIMER | QUESTION | SLIDERS => c(236, 112, 40),
        LINE | RADIO | LIST_NUM | COPY | TABLE => c(0, 150, 160),
        MOON | SUN => c(240, 170, 20),
        _ => ACCENT,
    }
}

/// Every large ribbon button is the same portrait box, so rows of them line up exactly.
/// A label wider than this wraps onto two lines; the button itself is only as wide as its text needs.
const BIG_W: f32 = 58.0;
const BIG_H: f32 = 58.0;

/// Break a label into at most two centred lines that fit the button (explicit `\n` wins).
fn label_lines(ui: &egui::Ui, label: &str) -> Vec<String> {
    if label.contains('\n') { return label.split('\n').map(String::from).collect(); }
    let width = |t: &str| ui.fonts_mut(|f| f.layout_no_wrap(t.to_string(), egui::FontId::proportional(11.5), egui::Color32::WHITE).size().x);
    if width(label) <= BIG_W - 6.0 { return vec![label.to_string()]; }
    // Split at the space that balances the two lines best.
    let mut best: Option<(f32, usize)> = None;
    for (i, _) in label.match_indices(' ') {
        let score = (width(&label[..i]) - width(&label[i + 1..])).abs();
        if best.is_none_or(|b| score < b.0) { best = Some((score, i)); }
    }
    match best {
        Some((_, i)) => vec![label[..i].to_string(), label[i + 1..].to_string()],
        None => vec![label.to_string()],
    }
}

/// Large flat button: colored icon on top, up to two lines of label under it, optional dropdown caret.
/// Icon and text sit at fixed positions so neighbouring buttons align regardless of label length.
fn big_widget(ui: &mut egui::Ui, icon: char, label: &str, caret: bool, selected: bool) -> egui::Response {
    big_widget_min(ui, icon, label, caret, selected, 0.0)
}

/// `big_widget`, at least `min_w` wide.
fn big_widget_min(ui: &mut egui::Ui, icon: char, label: &str, caret: bool, selected: bool, min_w: f32) -> egui::Response {
    let lines = label_lines(ui, label);
    let font = egui::FontId::proportional(11.5);
    let widest = lines.iter().enumerate().map(|(i, l)| {
        let w = ui.fonts_mut(|f| f.layout_no_wrap(l.clone(), font.clone(), egui::Color32::WHITE).size().x);
        if caret && i + 1 == lines.len() { w + 14.0 } else { w }
    }).fold(0.0f32, f32::max);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2((widest + 6.0).max(36.0).max(min_w), BIG_H), egui::Sense::click());
    let plain = label.replace('\n', " ");
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), plain.clone()));
    if ui.is_rect_visible(rect) {
        let enabled = ui.is_enabled();
        let vis = ui.style().interact_selectable(&resp, selected);
        if resp.hovered() || resp.has_focus() || selected {
            ui.painter().rect_filled(rect, RIBBON_RADIUS, if selected { vis.bg_fill } else { vis.weak_bg_fill });
        }
        let col = if selected { vis.fg_stroke.color } else { text_col(ui) };
        let icon_col = if !selected && enabled && ui.visuals().override_text_color.is_none() { tint(icon) } else { col };
        ui.painter().text(egui::pos2(rect.center().x, rect.top() + 4.0), egui::Align2::CENTER_TOP, icon, egui::FontId::new(22.0, icons()), icon_col);
        let n = lines.len();
        for (i, line) in lines.iter().enumerate() {
            let y = rect.top() + 31.0 + i as f32 * 12.0;
            let font = font.clone();
            if caret && i + 1 == n {
                // Centre text and caret together.
                let tw = ui.fonts_mut(|f| f.layout_no_wrap(line.clone(), font.clone(), col).size().x);
                let start = rect.center().x - (tw + 11.0) / 2.0;
                ui.painter().text(egui::pos2(start, y), egui::Align2::LEFT_TOP, line, font, col);
                chevron(ui.painter(), egui::pos2(start + tw + 8.0, y + 7.0), 2.8, col);
            } else {
                ui.painter().text(egui::pos2(rect.center().x, y), egui::Align2::CENTER_TOP, line, font, col);
            }
        }
    }
    resp
}

/// A big ribbon button whose picture is drawn by `draw` (given the painter, the 30 x 20 frame it may draw in, the colour of the
/// new slide and the colour of the old one), for a gallery of things no icon font has a glyph for.
fn big_drawn_button(ui: &mut egui::Ui, label: &str, tip: &str, selected: bool, draw: impl FnOnce(&egui::Painter, egui::Rect, egui::Color32, egui::Color32)) -> egui::Response {
    let w = big_width(ui, label, false);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, BIG_H), egui::Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label));
    if ui.is_rect_visible(rect) {
        let vis = ui.style().interact_selectable(&resp, selected);
        if resp.hovered() || resp.has_focus() || selected {
            ui.painter().rect_filled(rect, RIBBON_RADIUS, if selected { vis.bg_fill } else { vis.weak_bg_fill });
        }
        let col = if selected { vis.fg_stroke.color } else { text_col(ui) };
        let frame = egui::Rect::from_center_size(egui::pos2(rect.center().x, rect.top() + 18.0), egui::vec2(30.0, 20.0));
        let old = if selected { col.gamma_multiply(0.45) } else { text_col(ui).gamma_multiply(0.35) };
        let new = if selected { col } else { ACCENT };
        draw(&ui.painter().with_clip_rect(frame.expand(1.5)), frame, new, old);
        ui.painter().rect_stroke(frame, 2.0, egui::Stroke::new(1.2, col), egui::StrokeKind::Outside);
        ui.painter().text(egui::pos2(rect.center().x, rect.top() + 31.0), egui::Align2::CENTER_TOP, label, egui::FontId::proportional(11.5), col);
    }
    resp.on_hover_text(tip)
}

/// What each kind of slide transition looks like in the ribbon: the old slide in grey, the new one in colour, part way through.
fn draw_transition(kind: brainbend_doc::TransitionKind, p: &egui::Painter, f: egui::Rect, new: egui::Color32, old: egui::Color32) {
    use brainbend_doc::TransitionKind::*;
    let part = |x0: f32, x1: f32, y0: f32, y1: f32| egui::Rect::from_min_max(egui::pos2(f.left() + f.width() * x0, f.top() + f.height() * y0), egui::pos2(f.left() + f.width() * x1, f.top() + f.height() * y1));
    match kind {
        Fade => {
            const N: usize = 10;
            for i in 0..N {
                let t = i as f32 / (N - 1) as f32;
                let c = egui::Color32::from_rgba_unmultiplied(new.r(), new.g(), new.b(), (255.0 * t) as u8);
                p.rect_filled(part(i as f32 / N as f32, (i + 1) as f32 / N as f32, 0.0, 1.0), 0.0, old);
                p.rect_filled(part(i as f32 / N as f32, (i + 1) as f32 / N as f32, 0.0, 1.0), 0.0, c);
            }
        }
        Push => {
            p.rect_filled(part(-0.45, 0.5, 0.0, 1.0), 0.0, old);
            p.rect_filled(part(0.5, 1.45, 0.0, 1.0), 0.0, new);
            p.line_segment([part(0.5, 0.5, 0.0, 0.0).min, part(0.5, 0.5, 1.0, 1.0).min + egui::vec2(0.0, 0.0)], egui::Stroke::new(1.0, egui::Color32::from_black_alpha(90)));
        }
        Cover => {
            p.rect_filled(f, 0.0, old);
            p.rect_filled(part(0.4, 1.0, 0.0, 1.0), 0.0, new);
            p.line_segment([part(0.4, 0.4, 0.0, 0.0).min, part(0.4, 0.4, 1.0, 1.0).min], egui::Stroke::new(1.5, egui::Color32::from_black_alpha(110)));
        }
        Uncover => {
            p.rect_filled(f, 0.0, new);
            p.rect_filled(part(-0.4, 0.55, 0.0, 1.0), 0.0, old);
            p.line_segment([part(0.55, 0.55, 0.0, 0.0).min, part(0.55, 0.55, 1.0, 1.0).min], egui::Stroke::new(1.5, egui::Color32::from_black_alpha(110)));
        }
        Wipe => {
            p.rect_filled(f, 0.0, old);
            p.rect_filled(part(0.0, 0.58, 0.0, 1.0), 0.0, new);
            for k in 0..4 { let y = k as f32 / 4.0; p.rect_filled(part(0.58, 0.75, y + 0.06, y + 0.19), 0.0, new.gamma_multiply(0.55)); }
        }
        Split => {
            p.rect_filled(f, 0.0, old);
            p.rect_filled(part(0.3, 0.7, 0.0, 1.0), 0.0, new);
            let s = egui::Stroke::new(1.3, egui::Color32::WHITE);
            p.line_segment([part(0.3, 0.3, 0.5, 0.5).min, part(0.12, 0.12, 0.5, 0.5).min], s);
            p.line_segment([part(0.7, 0.7, 0.5, 0.5).min, part(0.88, 0.88, 0.5, 0.5).min], s);
        }
        Zoom => {
            p.rect_filled(f, 0.0, old);
            p.rect_filled(part(0.28, 0.72, 0.18, 0.82), 1.0, new);
            let s = egui::Stroke::new(1.0, new);
            p.rect_stroke(part(0.12, 0.88, 0.08, 0.92), 1.0, s, egui::StrokeKind::Inside);
        }
        Dissolve => {
            p.rect_filled(f, 0.0, old);
            // Scattered squares of the new slide, in a fixed pattern.
            for i in 0..6 { for j in 0..4 {
                if (i * 7 + j * 5 + i * j) % 3 != 0 { p.rect_filled(part(i as f32 / 6.0, (i + 1) as f32 / 6.0, j as f32 / 4.0, (j + 1) as f32 / 4.0), 0.0, new); }
            } }
        }
    }
}

/// `c` mixed towards black by `t` (0 to 1), still fully opaque.
fn shade(c: egui::Color32, t: f32) -> egui::Color32 {
    let k = 1.0 - t;
    egui::Color32::from_rgb((c.r() as f32 * k).round() as u8, (c.g() as f32 * k).round() as u8, (c.b() as f32 * k).round() as u8)
}

/// The width `big_widget` gives a button, so a group of them can be sized before it is drawn.
fn big_width(ui: &egui::Ui, label: &str, caret: bool) -> f32 {
    let lines = label_lines(ui, label);
    let font = egui::FontId::proportional(11.5);
    let widest = lines.iter().enumerate().map(|(i, l)| {
        let w = ui.fonts_mut(|f| f.layout_no_wrap(l.clone(), font.clone(), egui::Color32::WHITE).size().x);
        if caret && i + 1 == lines.len() { w + 14.0 } else { w }
    }).fold(0.0f32, f32::max);
    (widest + 6.0).max(36.0)
}

/// A caption under a row of controls, like `group` but without the divider after it.
fn pinned_group(ui: &mut egui::Ui, caption: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(3.0, 2.0);
        let body = ui.vertical(|ui| {
            ui.set_min_height(BIG_H);
            add(ui);
        });
        let w = body.response.rect.width().max(30.0);
        ui.allocate_ui_with_layout(egui::vec2(w, CAP_H), egui::Layout::top_down(egui::Align::Center), |ui| {
            ui.add(egui::Label::new(egui::RichText::new(caption).size(10.5).color(ui.visuals().text_color().gamma_multiply(0.55))).wrap_mode(egui::TextWrapMode::Extend));
        });
    });
}

/// Large flat button (icon above a short label).
fn big_button(ui: &mut egui::Ui, icon: char, label: &str, tip: &str) -> egui::Response {
    big_widget(ui, icon, label, false, false).on_hover_text(tip)
}

/// Large flat toggle button, drawn pressed when `selected`.
fn big_toggle(ui: &mut egui::Ui, icon: char, label: &str, tip: &str, selected: bool) -> egui::Response {
    big_widget(ui, icon, label, false, selected).on_hover_text(tip)
}

/// Large flat button that opens a dropdown of choices.
/// A big menu button whose popup stays open until a click outside it (or the content closes it).
fn big_menu_open(ui: &mut egui::Ui, icon: char, label: &str, tip: &str, content: impl FnOnce(&mut egui::Ui)) -> egui::Response {
    let resp = big_widget(ui, icon, label, true, false).on_hover_text(tip);
    egui::Popup::from_toggle_button_response(&resp).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| { content(ui); });
    resp
}

fn big_menu(ui: &mut egui::Ui, icon: char, label: &str, tip: &str, content: impl FnOnce(&mut egui::Ui)) -> egui::Response {
    let resp = big_widget(ui, icon, label, true, false).on_hover_text(tip);
    egui::Popup::from_toggle_button_response(&resp).close_behavior(egui::PopupCloseBehavior::CloseOnClick).show(|ui| {
        ui.set_min_width(190.0);
        content(ui);
    });
    resp
}

/// Text followed by a small dropdown caret (the caret glyph lives in the icon font).
fn caret_job(ui: &egui::Ui, text: &str, icon: Option<char>) -> egui::text::LayoutJob {
    let col = text_col(ui);
    let mut job = egui::text::LayoutJob::default();
    if let Some(i) = icon {
        job.append(&i.to_string(), 0.0, egui::text::TextFormat { font_id: egui::FontId::new(16.0, icons()), color: col, ..Default::default() });
        job.append("  ", 0.0, egui::text::TextFormat { font_id: egui::FontId::proportional(12.5), color: col, ..Default::default() });
    }
    job.append(text, 0.0, egui::text::TextFormat { font_id: egui::FontId::proportional(12.5), color: col, ..Default::default() });
    job.append("  ", 0.0, egui::text::TextFormat { font_id: egui::FontId::proportional(12.5), color: col, ..Default::default() });
    job.append(&CARET.to_string(), 0.0, egui::text::TextFormat { font_id: egui::FontId::new(9.0, icons()), color: col, ..Default::default() });
    job
}

/// An entrance or exit as one button; its panel holds the effects, how it starts and how long it takes.
/// There is no delay to set: when the object is shown is the timeline's job.
fn effect_menu(ui: &mut egui::Ui, icon: char, title: &str, tip: &str, effect: &mut Option<Entrance>, step: &mut u32, with_delay: bool) -> bool {
    let mut changed = false;
    let tip = match effect { Some(e) => format!("{title}: {}", effect_label(e.kind)), None => tip.to_string() };
    let btn = big_widget(ui, icon, title, true, effect.is_some()).on_hover_text(tip);
    egui::Popup::from_toggle_button_response(&btn).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
        ui.set_min_width(210.0);
        if ui.selectable_label(effect.is_none(), "none").clicked() { *effect = None; changed = true; }
        for k in EFFECT_KINDS {
            if ui.selectable_label(effect.map(|e| e.kind) == Some(k), effect_label(k)).clicked() {
                let prev = *effect;
                *effect = Some(Entrance { kind: k, delay_ms: prev.map(|e| e.delay_ms).unwrap_or(0), duration_ms: prev.map(|e| e.duration_ms).unwrap_or(500) });
                changed = true;
            }
        }
        if let Some(en) = effect.as_mut() {
            ui.separator();
            changed |= click_row(ui, &format!("ck-{title}"), "Starts", step);
            ui.horizontal(|ui| {
                ui.label("takes");
                changed |= ui.add(egui::DragValue::new(&mut en.duration_ms).range(50..=10000).speed(10.0).suffix(" ms")).changed();
            });
            if with_delay { changed |= delay_leftover(ui, &mut en.delay_ms); }
            else { ui.weak("Leaves when its bar ends on the timeline."); }
        }
    });
    changed
}

/// A delay saved by an earlier version is still honored, so it is shown (and can be cleared); nothing offers to set a new one.
fn delay_leftover(ui: &mut egui::Ui, delay: &mut u32) -> bool {
    if *delay == 0 { return false; }
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.weak(format!("Delayed {} ms", *delay));
        if ui.small_button("Clear").on_hover_text("Remove the delay. Set when it shows by dragging its bar on the timeline").clicked() { *delay = 0; changed = true; }
    });
    changed
}

impl App {
    /// Text Styles: Normal Text and Heading 2 to 4. The button names the style of the selected object.
    fn text_styles_menu(&mut self, ui: &mut egui::Ui) {
        let theme = self.project.as_ref().map(|p| p.theme.clone()).unwrap_or_default();
        let current = self.selected_object().map(|o| o.role).unwrap_or_default();
        let mut pick = None;
        let btn = big_menu(ui, TEXT_AA, current.label(), "Text styles: set whether the text is a heading (published as <h1> to <h4>) or normal text", |ui| {
            ui.set_min_width(250.0);
            for role in TextRole::CHOICES {
                let (size, heading) = role.look();
                let font = if heading { &theme.heading_font } else { &theme.body_font };
                let mut job = egui::text::LayoutJob::default();
                job.append(role.label(), 0.0, egui::text::TextFormat {
                    font_id: egui::FontId::new((size * 0.42).clamp(13.0, 28.0), font_family(font)),
                    color: egui::Color32::from_gray(20),
                    coords: egui::epaint::text::VariationCoords::new([(b"wght", 400.0)]),
                    ..Default::default()
                });
                let galley = ui.fonts_mut(|f| f.layout_job(job));
                let (rect, resp) = ui.allocate_exact_size(egui::vec2(250.0, galley.size().y + 12.0), egui::Sense::click());
                // Each style on white, so the black text reads in a dark menu too.
                ui.painter().rect_filled(rect, 4.0, egui::Color32::WHITE);
                if role == current { ui.painter().text(egui::pos2(rect.left() + 8.0, rect.center().y), egui::Align2::LEFT_CENTER, CHECK.to_string(), egui::FontId::new(14.0, icons()), tint(CHECK)); }
                ui.painter().galley(egui::pos2(rect.left() + 30.0, rect.center().y - galley.size().y / 2.0), galley, egui::Color32::from_gray(20));
                if resp.hovered() { ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(2.0, primary(ui)), egui::StrokeKind::Inside); }
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, role.label()));
                let tip = match role.level() { Some(n) => format!("{} (<h{n}>, {}, {:.0})", role.label(), font, size), None => format!("{} (<p>, {}, {:.0})", role.label(), font, size) };
                if resp.on_hover_text(tip).clicked() { pick = Some(role); }
            }
        });
        // A heading says so on the button: a small H1 to H4 at its corner.
        if let Some(n) = current.level() {
            let font = egui::FontId::proportional(8.5);
            let text = format!("H{n}");
            let w = ui.fonts_mut(|f| f.layout_no_wrap(text.clone(), font.clone(), egui::Color32::WHITE).size().x);
            // At the icon's lower right corner (the icon is centred near the top of the large button).
            let r = egui::Rect::from_min_size(egui::pos2(btn.rect.center().x + 13.0 - (w + 3.0), btn.rect.top() + 4.0 + 20.0 - 10.0), egui::vec2(w + 3.0, 10.0));
            ui.painter().rect_filled(r, 2.0, primary(ui));
            ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, text, font, on_primary(ui));
        }
        if let Some(role) = pick { self.apply_text_style(role); }
    }

    /// Give the selected objects a text style: the element they are published as, and its starting look (size, the
    /// theme's font and text color). Character formatting is cleared (links stay). One undo step.
    pub(super) fn apply_text_style(&mut self, role: TextRole) {
        let theme = self.project.as_ref().map(|p| p.theme.clone()).unwrap_or_default();
        let (size, heading) = role.look();
        let family = if heading { theme.heading_font.clone() } else { theme.body_font.clone() };
        let col = theme.colors.first().copied().unwrap_or(Rgba(34, 34, 34, 255));
        self.edit_text_range(move |o, _, _, _| {
            let n = o.text.chars().count();
            o.format_range(0, n, &Attr::Clear);
            o.role = role;
            o.style.font_family = family.clone();
            o.style.size = size;
            o.style.bold = false;
            o.style.italic = false;
            o.style.underline = false;
            o.style.color = col;
            let style = o.style.clone();
            for s in o.states.iter_mut() { s.style = style.clone(); }
        });
    }
}

/// The common font sizes Grow and Shrink step through, as in Office.
const SIZE_STEPS: [f32; 19] = [8.0, 9.0, 10.0, 11.0, 12.0, 14.0, 16.0, 18.0, 20.0, 24.0, 28.0, 32.0, 36.0, 40.0, 48.0, 60.0, 72.0, 96.0, 120.0];

/// The next size up or down from `cur` (stays put at either end).
pub(super) fn step_size(cur: f32, up: bool) -> f32 {
    if up { SIZE_STEPS.iter().copied().find(|s| *s > cur + 0.01).unwrap_or(cur) }
    else { SIZE_STEPS.iter().rev().copied().find(|s| *s < cur - 0.01).unwrap_or(cur) }
}

/// A large "A" with a small arrow: Grow font (up) or Shrink font (down).
fn grow_job(ui: &egui::Ui, up: bool) -> egui::text::LayoutJob {
    let col = text_col(ui);
    let mut job = egui::text::LayoutJob::default();
    job.append("A", 0.0, egui::text::TextFormat { font_id: egui::FontId::proportional(if up { 14.0 } else { 11.0 }), color: col, ..Default::default() });
    job.append(&(if up { ARROW_UP } else { ARROW_DOWN }).to_string(), 0.0, egui::text::TextFormat { font_id: egui::FontId::new(8.0, icons()), color: col, ..Default::default() });
    job
}

/// An icon followed by a short label, for small buttons that have both.
fn caret_job_plain(ui: &egui::Ui, icon: char, text: &str) -> egui::text::LayoutJob {
    let col = text_col(ui);
    let mut job = egui::text::LayoutJob::default();
    job.append(&icon.to_string(), 0.0, egui::text::TextFormat { font_id: egui::FontId::new(16.0, icons()), color: tint(icon), ..Default::default() });
    job.append("  ", 0.0, egui::text::TextFormat { font_id: egui::FontId::proportional(12.5), color: col, ..Default::default() });
    job.append(text, 0.0, egui::text::TextFormat { font_id: egui::FontId::proportional(12.5), color: col, ..Default::default() });
    job
}

/// One row in a dropdown: small icon, then text.
fn menu_item(ui: &mut egui::Ui, icon: char, text: &str) -> egui::Response {
    let col = text_col(ui);
    let mut job = egui::text::LayoutJob::default();
    job.append(&icon.to_string(), 0.0, egui::text::TextFormat { font_id: egui::FontId::new(16.0, icons()), color: col, ..Default::default() });
    job.append(&format!("   {text}"), 0.0, egui::text::TextFormat { font_id: egui::FontId::proportional(13.0), color: col, ..Default::default() });
    ui.add(egui::Button::new(job).frame(false).min_size(egui::vec2(ui.available_width().min(300.0), 24.0)))
}

/// Small square icon button; `on` draws it pressed (for toggles).
/// A compact icon button, three of which stack in the height of one large button.
fn tiny_button(ui: &mut egui::Ui, icon: char, on: bool, tip: &str) -> egui::Response {
    ui.scope(|ui| {
        ui.spacing_mut().button_padding = egui::vec2(1.0, 1.0);
        ui.spacing_mut().interact_size.y = 20.0;
        ui.add(egui::Button::new(icon_text(icon, 14.0)).selected(on).min_size(egui::vec2(22.0, 20.0)))
    }).inner.on_hover_text(tip)
}

fn icon_button(ui: &mut egui::Ui, icon: char, on: bool, tip: &str) -> egui::Response {
    // Small and tight: a 22px square holding a 15px glyph, with no padding of its own.
    let resp = ui.scope(|ui| {
        ui.spacing_mut().button_padding = egui::vec2(1.0, 1.0);
        ui.add(egui::Button::new(icon_text(icon, 15.0)).selected(on).min_size(egui::vec2(22.0, 22.0)))
    }).inner;
    // An icon has no words of its own: the tip names it for screen readers and for tests.
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tip));
    resp.on_hover_text(tip)
}

/// A small icon button that opens a dropdown, like `icon_button` with a popup attached.
fn icon_popup(ui: &mut egui::Ui, icon: char, tip: &str, close: egui::PopupCloseBehavior, content: impl FnOnce(&mut egui::Ui)) -> egui::Response {
    let resp = icon_button(ui, icon, false, tip);
    egui::Popup::from_toggle_button_response(&resp).close_behavior(close).show(content);
    resp
}

/// A small text button as compact as the icon ones (22px square minimum).
fn compact_text_button(ui: &mut egui::Ui, job: egui::text::LayoutJob, tip: &str) -> egui::Response {
    ui.scope(|ui| {
        ui.spacing_mut().button_padding = egui::vec2(1.0, 1.0);
        ui.add(egui::Button::new(job).min_size(egui::vec2(22.0, 22.0)))
    }).inner.on_hover_text(tip)
}

/// A down chevron: two strokes, drawn rather than typeset so it stays crisp and the same at every size.
pub(super) fn chevron(painter: &egui::Painter, center: egui::Pos2, half_w: f32, color: egui::Color32) {
    let h = half_w * 0.55;
    painter.add(egui::Shape::line(
        vec![center + egui::vec2(-half_w, -h), center + egui::vec2(0.0, h), center + egui::vec2(half_w, -h)],
        egui::Stroke::new(1.5, color),
    ));
}

/// A drop-down box with a chevron instead of egui's solid triangle.
pub(super) fn combo(salt: impl std::hash::Hash + std::fmt::Debug) -> egui::ComboBox {
    egui::ComboBox::from_id_salt(salt).icon(|ui: &egui::Ui, rect: egui::Rect, visuals: &egui::style::WidgetVisuals, _open: bool| {
        chevron(ui.painter(), rect.center(), 3.5, visuals.fg_stroke.color);
    })
}

/// A text button with a chevron after the label, for a menu opened from a small button.
fn caret_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let font = egui::FontId::proportional(12.5);
    let col = text_col(ui);
    let galley = ui.fonts_mut(|f| f.layout_no_wrap(text.to_string(), font, col));
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(galley.size().x + 32.0, 24.0), egui::Sense::click());
    if resp.hovered() || resp.has_focus() { ui.painter().rect_filled(rect, RIBBON_RADIUS, ui.visuals().widgets.hovered.weak_bg_fill); }
    let left = rect.left() + 10.0;
    ui.painter().galley(egui::pos2(left, rect.center().y - galley.size().y / 2.0), galley.clone(), col);
    chevron(ui.painter(), egui::pos2(left + galley.size().x + 10.0, rect.center().y + 0.5), 3.0, col);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, text));
    resp
}

/// The tabs, left to right, in the order an author works through them. Layout Builder shows only inside its own mode.
pub const TABS: [&str; 9] = ["File", "Home", "Theme", "Slides", "Objects", "Animations", "Layout Builder", "Review", "Help"];
pub const HOME_TAB: usize = 1;
pub const THEME_TAB: usize = 2;
pub const SLIDES_TAB: usize = 3;
pub const OBJECTS_TAB: usize = 4;
pub const ANIMATIONS_TAB: usize = 5;
pub const MASTER_TAB: usize = 6;
pub const REVIEW_TAB: usize = 7;
pub const HELP_TAB: usize = 8;
const ISSUES_URL: &str = "https://github.com/brainbend/studio/issues/new/choose";
/// The community link goes through a redirect page on the project's own site, so a Discord invite that expires or
/// changes is fixed there and not by shipping a new build.
pub const COMMUNITY_URL: &str = "https://brainbend.org/discord/";

/// Flat icon button for the title bar.
/// A title-bar button drawn by hand so it never changes size on hover: an optional icon, a label and an optional
/// drop-down caret, with a fixed 1px outline drawn inside its frame. `lift` stretches the frame up to the window's top edge.
struct Pill<'a> {
    icon: Option<(char, egui::Color32)>,
    label: &'a str,
    caret: bool,
    fill: Option<egui::Color32>,
    ink: egui::Color32,
    outline: bool,
    bold: bool,
    radius: egui::CornerRadius,
    lift: f32,
    min_w: f32,
}

fn pill(ui: &mut egui::Ui, id: &str, p: Pill) -> egui::Response {
    let font = egui::FontId::proportional(12.5);
    let mut job = egui::text::LayoutJob::default();
    if let Some((c, col)) = p.icon {
        job.append(&c.to_string(), 0.0, egui::text::TextFormat { font_id: egui::FontId::new(15.0, icons()), color: col, ..Default::default() });
        if !p.label.is_empty() { job.append("  ", 0.0, egui::text::TextFormat { font_id: font.clone(), color: p.ink, ..Default::default() }); }
    }
    job.append(p.label, 0.0, egui::text::TextFormat { font_id: font.clone(), color: p.ink, ..Default::default() });
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    // The chevron is drawn after the label, not typeset in it.
    let caret_w = if p.caret { 16.0 } else { 0.0 };
    let size = egui::vec2((galley.size().x + caret_w + 22.0).max(p.min_w), 24.0);
    let (slot, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    // A lifted button reaches the window's top edge, wherever the row happens to sit under it.
    let top = if p.lift > 0.0 { ui.ctx().content_rect().top() } else { slot.top() };
    let frame = egui::Rect::from_min_max(egui::pos2(slot.left(), top.min(slot.top())), slot.max);
    let resp = ui.interact(frame, ui.id().with(id), egui::Sense::click());
    let hot = resp.hovered() || resp.is_pointer_button_down_on();
    let base = p.fill.unwrap_or(egui::Color32::TRANSPARENT);
    let fill = if hot { if p.fill.is_some() { base.gamma_multiply(0.88) } else { p.ink.gamma_multiply(0.12) } } else { base };
    let painter = ui.painter();
    painter.rect_filled(frame, p.radius, fill);
    if p.outline {
        // Sides and bottom only: the top edge is the window's own edge, so a line there would just look doubled.
        let below_top = egui::Rect::from_min_max(frame.min + egui::vec2(0.0, if p.lift > 0.0 { 1.0 } else { 0.0 }), frame.max);
        painter.with_clip_rect(below_top).rect_stroke(frame, p.radius, egui::Stroke::new(1.0, p.ink.gamma_multiply(0.45)), egui::StrokeKind::Inside);
    }
    let _ = p.bold;
    let left = slot.center().x - (galley.size().x + caret_w) / 2.0;
    let gw = galley.size().x;
    painter.galley(egui::pos2(left, slot.center().y - galley.size().y / 2.0), galley, p.ink);
    if p.caret { chevron(painter, egui::pos2(left + gw + 10.0, slot.center().y + 0.5), 3.0, p.ink); }
    resp
}

fn title_icon(ui: &mut egui::Ui, icon: char, tip: &str) -> egui::Response {
    ui.add(egui::Button::new(icon_text(icon, 17.0).color(tint(icon))).min_size(egui::vec2(24.0, 24.0))).on_hover_text(tip)
}

/// Space small-control rows so two (or one) of them sit centred in the ribbon height.
fn rows(ui: &mut egui::Ui) {
    ui.spacing_mut().item_spacing.y = 6.0;
    ui.add_space(3.0);
}

/// Room under a group for its caption.
const CAP_H: f32 = 14.0;

/// A ribbon group: a fixed-height column of controls, its caption centred underneath, and a thin divider line.
fn group(ui: &mut egui::Ui, caption: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
        let body = ui.vertical(|ui| {
            ui.set_min_height(BIG_H);
            add(ui);
        });
        let w = body.response.rect.width().max(30.0);
        ui.allocate_ui_with_layout(egui::vec2(w, CAP_H), egui::Layout::top_down(egui::Align::Center), |ui| {
            ui.add(egui::Label::new(egui::RichText::new(caption).size(10.5).color(ui.visuals().text_color().gamma_multiply(0.55))).wrap_mode(egui::TextWrapMode::Extend));
        });
    });
    ui.add_space(1.0);
    let (r, _) = ui.allocate_exact_size(egui::vec2(1.0, BIG_H + CAP_H + 2.0), egui::Sense::hover());
    let line = ui.visuals().text_color().gamma_multiply(if ui.visuals().dark_mode { 0.30 } else { 0.25 });
    ui.painter().line_segment([r.center_top() + egui::vec2(0.0, 6.0), r.center_bottom() - egui::vec2(0.0, 6.0)], egui::Stroke::new(1.0, line));
    ui.add_space(1.0);
}

/// Extra width an outlined button gets, so its label does not touch the border.
const OUTLINE_PAD: f32 = 16.0;

/// For a main-action button drawn as an outline: its text and icon take the main-action color, and the button's
/// own hover fill is left off (the outline shape paints the states).
fn outlined_look(ui: &mut egui::Ui) {
    let p = primary(ui);
    let v = ui.visuals_mut();
    v.override_text_color = Some(p);
    for w in [&mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] { w.weak_bg_fill = egui::Color32::TRANSPARENT; w.bg_fill = egui::Color32::TRANSPARENT; }
}

/// The outline and inside of a main-action button: a border in the main-action color, with an opaque tint inside
/// and a darker border when the pointer is over it (`hot`), and more of both while pressed (`down`).
fn outline_shape(ui: &egui::Ui, rect: egui::Rect, hot: bool, down: bool) -> egui::Shape {
    let p = primary(ui);
    let card = if ui.visuals().dark_mode { egui::Color32::from_gray(46) } else { egui::Color32::WHITE };
    let mix = |t: f32| { let m = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8; egui::Color32::from_rgb(m(card.r(), p.r()), m(card.g(), p.g()), m(card.b(), p.b())) };
    let (fill, line) = if down { (mix(0.30), shade(p, 0.24)) } else if hot { (mix(0.15), shade(p, 0.12)) } else { (egui::Color32::TRANSPARENT, p) };
    egui::Shape::Vec(vec![
        egui::Shape::rect_filled(rect, RIBBON_RADIUS, fill),
        egui::Shape::rect_stroke(rect, RIBBON_RADIUS, egui::Stroke::new(2.0, line), egui::StrokeKind::Inside),
    ])
}

/// How wide the leading "way out" button of a mode is, divider included.
fn lead_width(ui: &egui::Ui, label: &str, caption: &str) -> f32 { big_width(ui, label, false).max(caption_width(ui, caption)) + 12.0 }

/// How wide a group's small caption is.
fn caption_width(ui: &egui::Ui, caption: &str) -> f32 {
    ui.fonts_mut(|f| f.layout_no_wrap(caption.to_string(), egui::FontId::proportional(10.5), egui::Color32::WHITE).size().x)
}

/// The button that leaves a mode, at the very start of the ribbon in the main-action color (so it is never
/// scrolled away or missed), with a divider after it. True when clicked.
fn lead_button(ui: &mut egui::Ui, at: egui::Rect, icon: char, label: &str, caption: &str, tip: &str) -> bool {
    let mut clicked = false;
    ui.scope_builder(egui::UiBuilder::new().max_rect(at).layout(egui::Layout::left_to_right(egui::Align::Min)), |ui| {
        let cw = caption_width(ui, caption).max(big_width(ui, label, false));
        group(ui, caption, |ui| {
            // The group is exactly as wide as its caption or button, whichever is wider, with the button centred over it.
            ui.allocate_ui_with_layout(egui::vec2(cw, BIG_H), egui::Layout::top_down(egui::Align::Center), |ui| {
            let backing = ui.painter().add(egui::Shape::Noop);
            let resp = ui.scope(|ui| { outlined_look(ui); big_widget_min(ui, icon, label, false, false, cw).on_hover_text(tip) }).inner;
            ui.painter().set(backing, outline_shape(ui, resp.rect, resp.hovered(), resp.is_pointer_button_down_on()));
            clicked = resp.clicked();
            });
        });
    });
    clicked
}

fn rgba(c: egui::Color32) -> Rgba {
    Rgba(c.r(), c.g(), c.b(), c.a())
}

/// A small icon button with a color bar under it; opens a palette + picker. Returns true when changed.
/// "Starts: automatically / on click #n" for one animation. Effects with the same click number play together.
fn click_row(ui: &mut egui::Ui, id: &str, label: &str, step: &mut u32) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        combo(id).width(110.0).selected_text(if *step == 0 { "automatically" } else { "on click" }).show_ui(ui, |ui| {
            if ui.selectable_label(*step == 0, "automatically").clicked() && *step != 0 { *step = 0; changed = true; }
            if ui.selectable_label(*step > 0, "on click").on_hover_text("Waits for the learner to click the slide, press Next, or press the space bar").clicked() && *step == 0 { *step = 1; changed = true; }
        });
        if *step > 0 {
            ui.label("click");
            changed |= ui.add(egui::DragValue::new(step).range(1..=99)).changed();
        }
    });
    changed
}

pub(super) fn color_menu(ui: &mut egui::Ui, icon: char, tip: &str, cur: &mut Option<Rgba>, none_label: Option<&str>) -> bool {
    let mut changed = false;
    let resp = icon_button(ui, icon, false, tip);
    let r = resp.rect;
    let bar = egui::Rect::from_min_max(egui::pos2(r.left() + 5.0, r.bottom() - 5.0), egui::pos2(r.right() - 5.0, r.bottom() - 2.5));
    match cur {
        Some(c) => { ui.painter().rect_filled(bar, 1.0, color(*c)); }
        None => { ui.painter().rect_stroke(bar, 1.0, egui::Stroke::new(1.0, ui.visuals().weak_text_color()), egui::StrokeKind::Inside); }
    }
    egui::Popup::from_toggle_button_response(&resp).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
        changed = color_popup(ui, cur, none_label);
    });
    changed
}

/// A color square that opens the same choices as the ribbon's color buttons: the project's colors first. For settings
/// outside the ribbon (a slider's colors, the player's accent).
pub(super) fn color_swatch(ui: &mut egui::Ui, c: &mut Rgba) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(28.0, 20.0), egui::Sense::click());
    ui.painter().rect_filled(rect, 3.0, color(*c));
    ui.painter().rect_stroke(rect, 3.0, egui::Stroke::new(1.0, ui.visuals().weak_text_color()), egui::StrokeKind::Inside);
    let mut changed = false;
    egui::Popup::from_toggle_button_response(&resp).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
        let mut cur = Some(*c);
        if color_popup(ui, &mut cur, None) { if let Some(n) = cur { *c = n; changed = true; } }
    });
    changed
}

/// A row of color squares; returns the one clicked.
fn swatch_row(ui: &mut egui::Ui, colors: &[Rgba], cur: Option<Rgba>) -> Option<Rgba> {
    let mut picked = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(3.0, 3.0);
        for c in colors {
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::click());
            ui.painter().rect_filled(rect, 3.0, color(*c));
            let on = cur.is_some_and(|k| (k.0, k.1, k.2) == (c.0, c.1, c.2));
            ui.painter().rect_stroke(rect, 3.0, egui::Stroke::new(if on { 2.0 } else { 1.0 }, if on { ACCENT } else { egui::Color32::from_gray(110) }), egui::StrokeKind::Inside);
            if resp.on_hover_text(c.hex()).clicked() { picked = Some(*c); }
        }
    });
    picked
}

/// Pick a color freely: a square for shade, a bar for hue, the color itself, and a code to type (hex, or red, green and blue)
/// and how opaque it is. Drawn by hand so it is as small and plain as the rest of the menu.
pub(super) fn color_entry(ui: &mut egui::Ui, cur: &mut Rgba) -> bool {
    use egui::ecolor::Hsva;
    // Menus use the usual width; a window can ask for more.
    let w_id = egui::Id::new("color-entry-width");
    let W: f32 = ui.ctx().data(|d| d.get_temp(w_id)).unwrap_or(232.0);
    let mut changed = false;
    // Hue and shade are kept between frames: a grey or black has no hue of its own to come back to.
    let state_id = ui.id().with("hsv");
    let mut hsv: Hsva = match ui.data(|d| d.get_temp::<(f32, f32, f32, [u8; 3])>(state_id)) {
        Some((h, s, v, rgb)) if rgb == [cur.0, cur.1, cur.2] => Hsva::new(h, s, v, 1.0),
        _ => Hsva::from_srgb([cur.0, cur.1, cur.2]),
    };
    let mut moved = false;

    // The shade square: white to the hue across, to black down.
    let (sq, sq_resp) = ui.allocate_exact_size(egui::vec2(W, 132.0), egui::Sense::click_and_drag());
    {
        let hue = Hsva::new(hsv.h, 1.0, 1.0, 1.0);
        let hue = egui::Color32::from_rgb(hue.to_srgb()[0], hue.to_srgb()[1], hue.to_srgb()[2]);
        let mut mesh = egui::epaint::Mesh::default();
        let clear = egui::Color32::from_rgba_unmultiplied(0, 0, 0, 0);
        for (pos, c) in [(sq.left_top(), egui::Color32::WHITE), (sq.right_top(), hue), (sq.right_bottom(), hue), (sq.left_bottom(), egui::Color32::WHITE)] { mesh.colored_vertex(pos, c); }
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        ui.painter().add(egui::Shape::mesh(mesh));
        let mut shade = egui::epaint::Mesh::default();
        for (pos, c) in [(sq.left_top(), clear), (sq.right_top(), clear), (sq.right_bottom(), egui::Color32::BLACK), (sq.left_bottom(), egui::Color32::BLACK)] { shade.colored_vertex(pos, c); }
        shade.add_triangle(0, 1, 2);
        shade.add_triangle(0, 2, 3);
        ui.painter().add(egui::Shape::mesh(shade));
        ui.painter().rect_stroke(sq, 2.0, egui::Stroke::new(1.0, egui::Color32::from_gray(110)), egui::StrokeKind::Inside);
        let at = egui::pos2(sq.left() + hsv.s * sq.width(), sq.top() + (1.0 - hsv.v) * sq.height());
        ui.painter().circle_stroke(at, 6.0, egui::Stroke::new(2.0, egui::Color32::BLACK));
        ui.painter().circle_stroke(at, 5.0, egui::Stroke::new(1.5, egui::Color32::WHITE));
    }
    if let (true, Some(p)) = (sq_resp.dragged() || sq_resp.clicked(), sq_resp.interact_pointer_pos()) {
        hsv.s = ((p.x - sq.left()) / sq.width()).clamp(0.0, 1.0);
        hsv.v = 1.0 - ((p.y - sq.top()) / sq.height()).clamp(0.0, 1.0);
        moved = true;
    }

    // The hue bar.
    ui.add_space(4.0);
    let (bar, bar_resp) = ui.allocate_exact_size(egui::vec2(W, 14.0), egui::Sense::click_and_drag());
    {
        let mut mesh = egui::epaint::Mesh::default();
        for i in 0..=6u32 {
            let x = bar.left() + bar.width() * i as f32 / 6.0;
            let c = Hsva::new(i as f32 / 6.0, 1.0, 1.0, 1.0).to_srgb();
            let c = egui::Color32::from_rgb(c[0], c[1], c[2]);
            mesh.colored_vertex(egui::pos2(x, bar.top()), c);
            mesh.colored_vertex(egui::pos2(x, bar.bottom()), c);
            if i > 0 {
                let k = i * 2;
                mesh.add_triangle(k - 2, k - 1, k);
                mesh.add_triangle(k - 1, k + 1, k);
            }
        }
        ui.painter().add(egui::Shape::mesh(mesh));
        ui.painter().rect_stroke(bar, 2.0, egui::Stroke::new(1.0, egui::Color32::from_gray(110)), egui::StrokeKind::Inside);
        let x = bar.left() + hsv.h * bar.width();
        let knob = egui::Rect::from_center_size(egui::pos2(x, bar.center().y), egui::vec2(6.0, bar.height() + 4.0));
        ui.painter().rect(knob, 2.0, egui::Color32::TRANSPARENT, egui::Stroke::new(2.0, egui::Color32::WHITE), egui::StrokeKind::Middle);
        ui.painter().rect_stroke(knob.expand(1.0), 2.0, egui::Stroke::new(1.0, egui::Color32::BLACK), egui::StrokeKind::Outside);
    }
    if let (true, Some(p)) = (bar_resp.dragged() || bar_resp.clicked(), bar_resp.interact_pointer_pos()) {
        hsv.h = ((p.x - bar.left()) / bar.width()).clamp(0.0, 1.0);
        moved = true;
    }
    if moved {
        let c = hsv.to_srgb();
        *cur = Rgba(c[0], c[1], c[2], cur.3);
        changed = true;
    }
    ui.data_mut(|d| d.insert_temp(state_id, (hsv.h, hsv.s, hsv.v, [cur.0, cur.1, cur.2])));

    // The color, and how to say it: a hex code, or red, green and blue.
    ui.add_space(6.0);
    let rgb_id = ui.id().with("rgb-mode");
    let mut rgb_mode: bool = ui.data(|d| d.get_temp(rgb_id)).unwrap_or(false);
    ui.horizontal(|ui| {
        let (sw, _) = ui.allocate_exact_size(egui::vec2(34.0, 24.0), egui::Sense::hover());
        ui.painter().rect_filled(sw, 3.0, color(Rgba(cur.0, cur.1, cur.2, 255)));
        ui.painter().rect_stroke(sw, 3.0, egui::Stroke::new(1.0, egui::Color32::from_gray(110)), egui::StrokeKind::Inside);
        combo(ui.id().with("mode")).width(58.0).selected_text(if rgb_mode { "RGB" } else { "Hex" }).show_ui(ui, |ui| {
            if ui.selectable_label(!rgb_mode, "Hex").clicked() { rgb_mode = false; }
            if ui.selectable_label(rgb_mode, "RGB").clicked() { rgb_mode = true; }
        });
        if rgb_mode {
            ui.spacing_mut().item_spacing.x = 3.0;
            for v in [&mut cur.0, &mut cur.1, &mut cur.2] { changed |= ui.add(egui::DragValue::new(v).range(0..=255).speed(0.5)).changed(); }
        } else {
            // A typed code takes effect when Enter is pressed or the box is left, so half a code never sets a color.
            let id = ui.id().with("typed-color");
            let mut text: String = ui.data_mut(|d| d.get_temp(id)).unwrap_or_else(|| cur.hex());
            let r = ui.add(egui::TextEdit::singleline(&mut text).desired_width(86.0).hint_text("#4F81BD"));
            if r.has_focus() { ui.data_mut(|d| d.insert_temp(id, text.clone())); } else { ui.data_mut(|d| d.remove_temp::<String>(id)); }
            if r.lost_focus() {
                if let Some(c) = Rgba::parse(&text) { *cur = Rgba(c.0, c.1, c.2, cur.3); changed = true; }
                ui.data_mut(|d| d.remove_temp::<String>(id));
            }
        }
    });
    ui.data_mut(|d| d.insert_temp(rgb_id, rgb_mode));
    ui.horizontal(|ui| {
        ui.label("Opacity");
        let mut pct = (cur.3 as f32 / 255.0 * 100.0).round();
        ui.spacing_mut().slider_width = 110.0;
        if ui.add(egui::Slider::new(&mut pct, 0.0..=100.0).suffix("%").integer()).changed() { cur.3 = (pct / 100.0 * 255.0).round() as u8; changed = true; }
    });
    changed
}

/// The choices in a color menu: "none", the author's palette (when they have made one), recently used colors, and a free
/// picker with a way to type a code, a + that adds the color to the palette and a pencil that edits the palette.
fn color_popup(ui: &mut egui::Ui, cur: &mut Option<Rgba>, none_label: Option<&str>) -> bool {
    let mut changed = false;
    ui.set_min_width(232.0);
    if let Some(label) = none_label {
        if ui.selectable_label(cur.is_none(), label).clicked() { *cur = None; changed = true; }
    }
    let palette: Vec<Rgba> = ui.ctx().data(|d| d.get_temp(egui::Id::new("palette-colors"))).unwrap_or_default();
    let recent: Vec<Rgba> = ui.ctx().data(|d| d.get_temp(egui::Id::new("recent-colors"))).unwrap_or_default();
    if !palette.is_empty() {
        ui.add_space(4.0);
        ui.label(egui::RichText::new("Palette").weak().size(12.0));
        if let Some(c) = swatch_row(ui, &palette, *cur) { *cur = Some(c); changed = true; }
    }
    if !recent.is_empty() {
        ui.add_space(4.0);
        ui.label(egui::RichText::new("Recent").weak().size(12.0));
        if let Some(c) = swatch_row(ui, &recent, *cur) { *cur = Some(c); changed = true; }
    }
    ui.add_space(6.0);
    let mut c = cur.unwrap_or(Rgba(255, 255, 255, 255));
    if color_entry(ui, &mut c) { *cur = Some(c); changed = true; }
    ui.horizontal(|ui| {
        let plus = crate::panos::icon_char("plus").unwrap_or('+');
        if icon_button(ui, plus, false, "Add this color to the palette").clicked() {
            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("palette-add"), c));
        }
        if icon_button(ui, PEN, false, "Edit the palette").clicked() {
            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("open-project-colors"), true));
            ui.close();
        }
    });
    // What was chosen is remembered as a recent color once the pointer is released.
    if changed { if let Some(c) = *cur { ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("picked-color"), c)); } }
    changed
}

/// Build a linear gradient: a bar to see it, markers to select, move, add and remove its colors, the angle, presets
/// and the selected color's picker. All inline. True when anything changed.
fn gradient_editor(ui: &mut egui::Ui, angle: &mut f32, stops: &mut Vec<(f32, Rgba)>, kind: &mut brainbend_doc::GradientKind, geom: &mut brainbend_doc::GradientGeom, slide_size: (f32, f32)) -> bool {
    let mut changed = false;
    let w: f32 = ui.ctx().data(|d| d.get_temp(egui::Id::new("color-entry-width"))).unwrap_or(300.0);
    let sel_id = ui.id().with("stop");
    let mut sel: usize = ui.data(|d| d.get_temp(sel_id)).unwrap_or(0).min(stops.len().saturating_sub(1));
    // The color being edited comes first: the project's colors, recent ones and a free picker.
    if let Some(st) = stops.get_mut(sel) {
        let mut c = Some(st.1);
        if color_popup(ui, &mut c, None) { if let Some(n) = c { st.1 = n; changed = true; } }
    }
    ui.add_space(8.0);
    // The shape of the gradient.
    ui.columns(4, |cols| {
        for (i, k) in brainbend_doc::GradientKind::ALL.into_iter().enumerate() {
            if cols[i].add_sized([cols[i].available_width(), 22.0], egui::Button::selectable(*kind == k, k.label())).clicked() && *kind != k { *kind = k; changed = true; }
        }
    });
    ui.add_space(8.0);
    // The bar, with a marker under it for each color. Click the bar to add a color; drag a marker to move it.
    let (bar_full, bar_resp) = ui.allocate_exact_size(egui::vec2(w, 26.0), egui::Sense::click());
    let bar = bar_full.shrink2(egui::vec2(8.0, 0.0));
    paint_gradient(&ui.painter_at(bar), bar, 90.0, stops);
    ui.painter().rect_stroke(bar, 3.0, egui::Stroke::new(1.0, ui.visuals().weak_text_color()), egui::StrokeKind::Inside);
    let (strip_full, _) = ui.allocate_exact_size(egui::vec2(w, 18.0), egui::Sense::hover());
    let strip = strip_full.shrink2(egui::vec2(8.0, 0.0));
    let mut moved: Option<(usize, f32)> = None;
    for i in 0..stops.len() {
        let x = strip.left() + stops[i].0.clamp(0.0, 1.0) * strip.width();
        let m = egui::Rect::from_center_size(egui::pos2(x, strip.center().y), egui::vec2(16.0, 16.0));
        let r = ui.interact(m, ui.id().with(("stop-marker", i)), egui::Sense::click_and_drag());
        if r.clicked() || r.drag_started() { sel = i; }
        if let (true, Some(p)) = (r.dragged(), r.interact_pointer_pos()) { moved = Some((i, ((p.x - strip.left()) / strip.width()).clamp(0.0, 1.0))); }
        ui.painter().rect_filled(m, 3.0, color(stops[i].1));
        ui.painter().rect_stroke(m, 3.0, egui::Stroke::new(if sel == i { 2.5 } else { 1.0 }, if sel == i { ACCENT } else { egui::Color32::from_gray(120) }), egui::StrokeKind::Outside);
    }
    if let Some((i, t)) = moved { stops[i].0 = t; changed = true; }
    if bar_resp.clicked() && stops.len() < 6 {
        if let Some(p) = bar_resp.interact_pointer_pos() {
            let t = ((p.x - bar.left()) / bar.width()).clamp(0.0, 1.0);
            let c = gradient_at(stops, t);
            stops.push((t, Rgba(c.r(), c.g(), c.b(), 255)));
            sel = stops.len() - 1;
            changed = true;
        }
    }
    // Keep the colors in order along the bar, following the selected one.
    let selected = stops.get(sel).copied();
    stops.sort_by(|a, b| a.0.total_cmp(&b.0));
    if let Some(s) = selected { sel = stops.iter().position(|x| *x == s).unwrap_or(0); }
    ui.add_space(2.0);
    ui.weak("Click the bar to add a color. Drag a marker to move it.");
    ui.weak(match *kind {
        brainbend_doc::GradientKind::Radial => "On the slide, drag the middle dot to move the centre and the other dot to change the size.",
        brainbend_doc::GradientKind::Angular => "On the slide, drag the middle dot to move the centre and the other dot to change where it starts.",
        _ => "On the slide, drag the two dots to set where the gradient starts and ends.",
    });
    let home = {
        let mut g = brainbend_doc::GradientGeom::default();
        if matches!(*kind, brainbend_doc::GradientKind::Linear | brainbend_doc::GradientKind::Reflected) { g.line = Some(brainbend_doc::GradientGeom::fitted_line(*angle, slide_size.0, slide_size.1)); }
        g
    };
    if *geom != home && ui.small_button("Reset position").on_hover_text("Back to filling the slide from its middle").clicked() { *geom = home; changed = true; }
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(format!("Color {} of {}", sel + 1, stops.len()));
        if let Some(st) = stops.get_mut(sel) {
            let mut pct = (st.0 * 100.0).round();
            if ui.add(egui::DragValue::new(&mut pct).range(0.0..=100.0).suffix("%")).changed() { st.0 = pct / 100.0; changed = true; }
        }
        if ui.add_enabled(stops.len() > 2, egui::Button::new("Remove")).on_hover_text("Remove this color (a gradient needs two)").clicked() { stops.remove(sel); sel = 0; changed = true; }
    });
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add_enabled_ui(kind.uses_angle(), |ui| {
        ui.label(if *kind == brainbend_doc::GradientKind::Angular { "Start" } else { "Angle" });
        ui.spacing_mut().slider_width = (w - 170.0).max(80.0);
        // The slider rounds what it shows, so work on a copy and only take it when the author moves the slider: a handle
        // dragged on the slide leaves the angle between whole degrees.
        let mut shown = *angle;
        let r = ui.add(egui::Slider::new(&mut shown, 0.0..=360.0).suffix("°").fixed_decimals(0));
        if r.changed() && (r.dragged() || r.has_focus() || r.is_pointer_button_down_on() || r.lost_focus()) {
            *angle = shown;
            if geom.line.is_none() { geom.line = Some(brainbend_doc::GradientGeom::fitted_line(*angle, slide_size.0, slide_size.1)); }
            geom.turn_line(*angle, slide_size.0, slide_size.1);
            changed = true;
        }
        });
        if ui.button("Flip").on_hover_text("Reverse the colors").clicked() { stops.reverse(); for st in stops.iter_mut() { st.0 = 1.0 - st.0; } sel = stops.len() - 1 - sel; changed = true; }
    });
    ui.data_mut(|d| d.insert_temp(sel_id, sel));
    changed
}

/// A grid of squares to pick a table size: hover to preview "rows x columns", click to insert.
fn table_picker(ui: &mut egui::Ui) -> Option<(u16, u16)> {
    const R: usize = 8;
    const C: usize = 8;
    let cell = 20.0;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(C as f32 * (cell + 2.0), R as f32 * (cell + 2.0)), egui::Sense::click());
    let hover = resp.hover_pos().map(|p| (((p.y - rect.top()) / (cell + 2.0)) as usize, ((p.x - rect.left()) / (cell + 2.0)) as usize));
    for r in 0..R {
        for c in 0..C {
            let on = hover.is_some_and(|(hr, hc)| r <= hr && c <= hc);
            let sq = egui::Rect::from_min_size(rect.min + egui::vec2(c as f32 * (cell + 2.0), r as f32 * (cell + 2.0)), egui::vec2(cell, cell));
            ui.painter().rect_filled(sq, 2.0, if on { ACCENT } else { ui.visuals().text_color().gamma_multiply(0.12) });
        }
    }
    match hover { Some((r, c)) => ui.label(format!("{} x {} table", r + 1, c + 1)), None => ui.weak("Choose a size") };
    match (hover, resp.clicked()) { (Some((r, c)), true) => Some((r as u16 + 1, c as u16 + 1)), _ => None }
}

/// A grid of the preset shapes, each drawn small. Returns the one clicked.
fn shape_gallery(ui: &mut egui::Ui) -> Option<&'static str> {
    use brainbend_doc::shapes::{preset, Preset, GALLERY};
    let mut picked = None;
    ui.set_min_width(190.0);
    egui::Grid::new("shape-gallery").spacing([4.0, 4.0]).show(ui, |ui| {
        for (i, (name, label)) in GALLERY.iter().enumerate() {
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(40.0, 36.0), egui::Sense::click());
            if resp.hovered() { ui.painter().rect_filled(rect, 4.0, ui.visuals().text_color().gamma_multiply(0.1)); }
            let inner = rect.shrink2(egui::vec2(8.0, 7.0));
            let col = ui.visuals().text_color().gamma_multiply(0.85);
            let stroke = egui::Stroke::new(1.6, col);
            match preset(name, inner.width(), inner.height(), &|_, d| d) {
                Preset::Rect | Preset::Unknown => { ui.painter().rect_stroke(inner, 0.0, stroke, egui::StrokeKind::Inside); }
                Preset::Round(r) => { ui.painter().rect_stroke(inner, r * inner.height(), stroke, egui::StrokeKind::Inside); }
                Preset::Ellipse => { ui.painter().add(egui::epaint::EllipseShape::stroke(inner.center(), inner.size() / 2.0, stroke)); }
                Preset::Line => { ui.painter().line_segment([inner.left_bottom(), inner.right_top()], stroke); }
                Preset::Path(cmds) => {
                    for poly in path_polygons(&cmds, inner) { ui.painter().add(egui::Shape::closed_line(poly, stroke)); }
                }
            }
            if resp.on_hover_text(*label).clicked() { picked = Some(*name); }
            if (i + 1) % 4 == 0 { ui.end_row(); }
        }
    });
    picked
}

/// Copy the fields that differ between `a` (before) and `b` (after) onto `t`.
fn apply_diff(a: &Object, b: &Object, t: &mut Object) {
    macro_rules! field { ($($f:ident),*) => { $( if a.$f != b.$f { t.$f = b.$f.clone(); } )* } }
    field!(fill, stroke, dash, shadow, glow, soft_edge, reflection, radius, hidden, entrance, exit, emphasis, motion, click_steps, timing, alt, gradient, rotation, line_spacing, x, y, w, h, media, captions, text_rotation);
    // Picking a flat color replaces a gradient.
    if a.fill != b.fill && a.gradient == b.gradient { t.gradient = None; }
    macro_rules! sfield { ($($f:ident),*) => { $( if a.style.$f != b.style.$f { t.style.$f = b.style.$f.clone(); } )* } }
    sfield!(font_family, size, bold, italic, underline, color, align, valign);
}

/// Fill, outline and effects (shadow, corners) for the object being edited. Shared by the selection groups on Home.
/// A color mixed towards white: `t` of the color, the rest white.
fn tint_toward_white(c: Rgba, t: f32) -> Rgba {
    let m = |v: u8| (v as f32 * t + 255.0 * (1.0 - t)).round() as u8;
    Rgba(m(c.0), m(c.1), m(c.2), 255)
}

/// What a Quick Styles swatch gives a shape: fill, outline, shadow and the text color that reads on it.
pub(super) struct QuickLook { pub(super) fill: Rgba, pub(super) stroke: Option<Stroke>, pub(super) shadow: Option<Shadow>, pub(super) text: Rgba }

/// The Quick Styles grid: four looks (outline, light fill, solid fill, intense) in each of the theme's six accent colors.
pub(super) fn quick_look(row: usize, accent: Rgba, theme: &[Rgba]) -> QuickLook {
    use brainbend_doc::check::contrast_ratio;
    let paper = theme.get(1).copied().unwrap_or(Rgba(255, 255, 255, 255));
    let ink = theme.first().copied().unwrap_or(Rgba(34, 34, 34, 255));
    let on = |fill: Rgba| if contrast_ratio(Rgba(255, 255, 255, 255), fill) >= contrast_ratio(ink, fill) { Rgba(255, 255, 255, 255) } else { ink };
    let line = |c: Rgba, w: f32| Some(Stroke { color: c, width: w });
    match row {
        0 => QuickLook { fill: paper, stroke: line(accent, 3.0), shadow: None, text: ink },
        1 => { let f = tint_toward_white(accent, 0.18); QuickLook { fill: f, stroke: line(accent, 2.0), shadow: None, text: on(f) } }
        2 => QuickLook { fill: accent, stroke: None, shadow: None, text: on(accent) },
        _ => QuickLook { fill: accent, stroke: line(Rgba(accent.0 / 2, accent.1 / 2, accent.2 / 2, 255), 3.0), shadow: Some(Shadow { dx: 4.0, dy: 4.0, blur: 8.0, spread: 0.0, color: Rgba(0, 0, 0, 90) }), text: on(accent) },
    }
}

/// Quick Styles: a gallery of ready-made fill, outline and shadow looks in the theme's colors.
fn quick_styles(ui: &mut egui::Ui, obj: &mut Object, changed: &mut bool) {
    let theme: Vec<Rgba> = ui.ctx().data(|d| d.get_temp(egui::Id::new("theme-colors"))).unwrap_or_default();
    let accents: Vec<Rgba> = if theme.len() >= 10 { theme[4..10].to_vec() } else { vec![Rgba(3, 83, 158, 255), Rgba(232, 119, 34, 255), Rgba(46, 139, 87, 255), Rgba(142, 68, 173, 255), Rgba(200, 50, 70, 255), Rgba(90, 100, 115, 255)] };
    big_menu(ui, SHAPES, "Quick Styles", "Give the shape a ready-made fill, outline and shadow in the theme's colors", |ui| {
        ui.set_min_width(0.0);
        let labels = ["Outline", "Light fill", "Solid fill", "Intense"];
        egui::Grid::new("quick-styles").spacing([5.0, 5.0]).show(ui, |ui| {
            for (row, name) in labels.iter().enumerate() {
                for (col, accent) in accents.iter().enumerate() {
                    let look = quick_look(row, *accent, &theme);
                    let (rect, resp) = ui.allocate_exact_size(egui::vec2(46.0, 32.0), egui::Sense::click());
                    let r = rect.shrink(4.0);
                    if let Some(sh) = look.shadow { ui.painter().rect_filled(r.translate(egui::vec2(2.5, 2.5)), 4.0, egui::Color32::from_black_alpha(sh.color.3)); }
                    ui.painter().rect_filled(r, 4.0, color(look.fill));
                    if let Some(st) = look.stroke { ui.painter().rect_stroke(r, 4.0, egui::Stroke::new((st.width * 0.6).max(1.0), color(st.color)), egui::StrokeKind::Inside); }
                    ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, "Abc", egui::FontId::proportional(12.0), color(look.text));
                    if resp.hovered() { ui.painter().rect_stroke(rect, 5.0, egui::Stroke::new(2.0, primary(ui)), egui::StrokeKind::Inside); }
                    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &format!("{name}, {}", brainbend_doc::master::THEME_COLOR_NAMES[col + 4])));
                    if resp.on_hover_text(format!("{name}, {}", brainbend_doc::master::THEME_COLOR_NAMES[col + 4])).clicked() {
                        obj.fill = Some(look.fill);
                        obj.stroke = look.stroke;
                        obj.shadow = look.shadow;
                        obj.style.color = look.text;
                        *changed = true;
                        ui.close();
                    }
                }
                ui.end_row();
            }
        });
    });
}

fn drawing_group(ui: &mut egui::Ui, obj: &mut Object, enabled: bool, changed: &mut bool) {
    let is_media = matches!(obj.kind, Kind::Video | Kind::Audio | Kind::Web(_) | Kind::Pano(_) | Kind::Unsupported(_));
    let is_pic = matches!(obj.kind, Kind::Image);
    let is_line = matches!(obj.kind, Kind::Line | Kind::LineUp);
    {
            let on = enabled && !is_media;
            group(ui, "Drawing", |ui| { ui.add_enabled_ui(on, |ui| { ui.horizontal(|ui| {
            if !is_line && !is_pic { quick_styles(ui, obj, changed); }
            ui.vertical(|ui| {
            rows(ui);
                ui.horizontal(|ui| {
                    if !is_line && !is_pic {
                        *changed |= color_menu(ui, BUCKET, "Shape fill", &mut obj.fill, Some("No fill"));
                    }
                    let mut stroke_color = obj.stroke.map(|s| s.color);
                    if color_menu(ui, PEN, if is_line { "Line color" } else { "Outline color" }, &mut stroke_color, Some("No outline")) {
                        obj.stroke = stroke_color.map(|c| Stroke { color: c, width: obj.stroke.map(|s| s.width).unwrap_or(2.0) });
                        *changed = true;
                    }
                    if let Some(st) = obj.stroke.as_mut() {
                        framed(ui, |ui| { *changed |= ui.add(egui::DragValue::new(&mut st.width).range(0.5..=40.0).speed(0.1).suffix(" px")).on_hover_text("Thickness").changed(); });
                        icon_popup(ui, LINE, "Outline pattern: solid, dotted or dashed", egui::PopupCloseBehavior::CloseOnClick, |ui| {
                            for (d, label) in [(LineDash::Solid, "Solid"), (LineDash::Dotted, "Dotted"), (LineDash::Dashed, "Dashed")] {
                                if ui.selectable_label(obj.dash == d, label).clicked() { obj.dash = d; *changed = true; }
                            }
                        });
                    }
                });
            }); }); }); });
        }
}

impl App {
    /// Object currently selected (primary) and whether it can carry text/shape formatting.
    fn ribbon_target(&self) -> Option<Object> {
        self.selected_object().cloned()
    }

    /// Write the fields `before → after` changed onto every selected object, as one undo step per edit burst.
    pub(super) fn apply_object_edit(&mut self, ctx: &egui::Context, before: &Object, after: &Object) {
        let sel = self.format_targets();
        if sel.is_empty() { return; }
        let slides_before = self.project.as_ref().unwrap().slides.clone();
        let layers = &mut self.project.as_mut().unwrap().slides[self.slide].layers;
        for (li, oi) in sel {
            if let Some(t) = layers.get_mut(li).and_then(|l| l.objects.get_mut(oi)) {
                apply_diff(before, after, t);
                // Choosing an outline for a table cell restyles all four of its sides.
                if before.stroke != after.stroke { t.table_borders = None; }
                // Objects with states (buttons) draw from their initial state: keep it in step.
                if !t.states.is_empty() {
                    let (f, st, sh, sk, init) = (t.fill, t.style.clone(), t.shadow, t.stroke, t.initial_state.clone());
                    for s in t.states.iter_mut() {
                        if s.name == init { s.fill = f; s.shadow = sh; s.stroke = sk; s.gradient = t.gradient.clone(); }
                        s.style = st.clone();
                    }
                }
            }
        }
        let editing = ctx.input(|i| i.pointer.any_down()) || ctx.memory(|m| m.focused().is_some());
        if !(self.edit_open && editing) { self.push_undo(slides_before); }
        self.edit_open = true;
        self.dirty = true;
    }

    /// Record `before` as an undo step for a project-level edit, merging a drag/typing burst into one step.
    pub(super) fn commit_project_edit(&mut self, ctx: &egui::Context, before: Snap) {
        let editing = ctx.input(|i| i.pointer.any_down()) || ctx.memory(|m| m.focused().is_some());
        if !(self.edit_open && editing) {
            self.undo.push(before);
            self.redo.clear();
        }
        self.edit_open = true;
        self.dirty = true;
    }

    /// Everything above the canvas: title bar, tab strip and the ribbon card.
    pub(super) fn chrome(&mut self, root: &mut egui::Ui, ctx: &egui::Context) {
        // The harness draws a first frame before fonts exist; the icon font is needed from here on.
        if !ctx.fonts(|f| f.families().contains(&icons())) { return; }
        self.sync_color_events(ctx);
        self.title_bar(root, ctx);
        self.run_format_painter(ctx);
        // The Quick Styles are built from the theme's colors.
        if let Some(p) = &self.project { ctx.data_mut(|d| d.insert_temp(egui::Id::new("theme-colors"), p.theme.colors.clone())); }
        if self.project.is_none() || self.file_view { return; }
        if self.project.as_ref().is_some_and(|p| p.is_web()) { self.lesson_toolbar(root, ctx); return; }
        if self.pano.edit.is_some() { self.pano_chrome(root, ctx); return; }
        let bg = root.visuals().panel_fill;
        let card = if root.visuals().dark_mode { egui::Color32::from_gray(46) } else { egui::Color32::WHITE };
        egui::Panel::top("tabs").frame(egui::Frame::new().fill(bg).inner_margin(egui::Margin::symmetric(10, 1))).show_separator_line(false).show(root, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                let master = self.in_master_view();
                // The Layout Builder has its own short tab list, with its own tab first; that tab exists only there.
                let order: Vec<usize> = if master { vec![0, MASTER_TAB, HOME_TAB, OBJECTS_TAB, ANIMATIONS_TAB, HELP_TAB] } else { (0..TABS.len()).filter(|&i| i != MASTER_TAB).collect() };
                for i in order {
                    let name = TABS[i];
                    let sel = self.ribbon_tab == i && i != 0;
                    let font = egui::FontId::proportional(14.0);
                    let tw = ui.fonts_mut(|f| f.layout_no_wrap(name.to_string(), font.clone(), egui::Color32::WHITE).size().x);
                    let (rect, resp) = ui.allocate_exact_size(egui::vec2(tw + 22.0, 26.0), egui::Sense::click());
                    if sel {
                        // Cut from the ribbon: the card's own color, reaching down to meet the card, with a blue line under the label.
                        let blue = ui.visuals().selection.bg_fill;
                        let over = ui.painter().with_clip_rect(ui.clip_rect().expand2(egui::vec2(0.0, 4.0)));
                        let joined = egui::Rect::from_min_max(rect.min, egui::pos2(rect.right(), rect.bottom() + 2.0));
                        over.rect_filled(joined, egui::CornerRadius { nw: 4, ne: 4, sw: 0, se: 0 }, card);
                        let label = over.text(egui::pos2(rect.center().x, rect.center().y - 1.0), egui::Align2::CENTER_CENTER, name, font, ui.visuals().strong_text_color());
                        // The line glides from the tab it was under to this one.
                        let ctx = ui.ctx().clone();
                        let left = ctx.animate_value_with_time(egui::Id::new("tab-line-left"), label.left(), 0.18);
                        let right = ctx.animate_value_with_time(egui::Id::new("tab-line-right"), label.right(), 0.18);
                        over.rect_filled(egui::Rect::from_min_max(egui::pos2(left, label.bottom() + 2.0), egui::pos2(right, label.bottom() + 4.5)), 1.0, blue);
                    } else {
                        if resp.hovered() { ui.painter().rect_filled(rect, egui::CornerRadius { nw: 4, ne: 4, sw: 0, se: 0 }, ui.visuals().text_color().gamma_multiply(0.10)); }
                        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, name, font, ui.visuals().text_color());
                    }
                    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name));
                    if resp.clicked() {
                        if i == 0 { self.leave_master_view(); self.file_view = true; self.file_page = backstage::FilePage::Info; } else { self.ribbon_tab = i; }
                    }
                }
                // What the groups on Home act on, as a tinted tag after the tabs.
                if let Some(title) = self.format_title().filter(|_| self.ribbon_tab == HOME_TAB) {
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(title).size(10.5).color(egui::Color32::from_rgb(70, 150, 230)).background_color(egui::Color32::from_rgba_unmultiplied(10, 110, 235, 38)));
                }
            });
        });
        egui::Panel::top("ribbon").frame(egui::Frame::new().fill(bg).inner_margin(egui::Margin { left: 8, right: 8, top: 0, bottom: 6 })).show_separator_line(false).show(root, |ui| {
            egui::Frame::new().fill(card).corner_radius(RIBBON_RADIUS as u8).inner_margin(egui::Margin::symmetric(6, 4)).show(ui, |ui| {
                flat(ui);
                ui.spacing_mut().scroll = egui::style::ScrollStyle::floating();
                let normal = self.project.is_some();
                let primary = self.ribbon_target();
                let mut edited = primary.clone();
                let mut edit_changed = false;
                // The controls scroll on the left; Try It, Preview and Publish stay pinned at the right.
                // Try It, Preview and Publish are for the finished lesson, so the Layout Builder does not show them.
                let pin_w = if self.project.is_some() && !self.in_master_view() { self.pinned_width(ui) } else { 0.0 };
                let full = ui.available_rect_before_wrap();
                let h = BIG_H + CAP_H + 8.0;
                // In the Layout Builder the way out comes first, in the main-action color, and does not scroll away.
                let lead_w = if self.in_master_view() { lead_width(ui, "Close", "Layout Builder") } else { 0.0 };
                if lead_w > 0.0 && lead_button(ui, egui::Rect::from_min_size(full.min, egui::vec2(lead_w, h)), X, "Close", "Layout Builder", "Return to the slides. Your changes are kept") { self.leave_master_view(); }
                let left = egui::Rect::from_min_size(full.min + egui::vec2(lead_w, 0.0), egui::vec2((full.width() - pin_w - 8.0 - lead_w).max(80.0), h));
                let pin = egui::Rect::from_min_size(egui::pos2((full.right() - pin_w).max(left.right()), full.top()), egui::vec2(pin_w, h));
                ui.scope_builder(egui::UiBuilder::new().max_rect(left), |ui| {
                    egui::ScrollArea::horizontal().id_salt(("ribbon-scroll", self.ribbon_tab)).auto_shrink([false, true]).show(ui, |ui| {
                        ui.horizontal_top(|ui| {
                            ui.set_min_height(BIG_H);
                            ui.spacing_mut().item_spacing.x = 3.0;
                            ui.add_enabled_ui(!self.try_mode, |ui| {
                            match self.ribbon_tab {
                                HOME_TAB => self.home_tab(ui, normal, edited.as_mut(), &mut edit_changed),
                                OBJECTS_TAB => self.insert_tab(ui, normal),
                                SLIDES_TAB => self.slides_tab(ui, ctx),
                                THEME_TAB => self.design_tab(ui, ctx),
                                ANIMATIONS_TAB => {
                                    // A layout has no transitions of its own: only the shapes on it animate.
                                    if !self.in_master_view() { self.transition_groups(ui, ctx); }
                                    self.animations_tab(ui, edited.as_mut(), &mut edit_changed);
                                }
                                MASTER_TAB => self.master_tab(ui, ctx),
                                REVIEW_TAB => self.review_tab(ui, ctx),
                                HELP_TAB => self.help_tab(ui),
                                _ => self.design_tab(ui, ctx),
                            }
                            });
                        });
                    });
                });
                if pin_w > 0.0 {
                    ui.scope_builder(egui::UiBuilder::new().max_rect(pin).layout(egui::Layout::left_to_right(egui::Align::Min)), |ui| self.pinned_actions(ui));
                }
                if let (true, Some(before), Some(after)) = (edit_changed, primary.as_ref(), edited.as_ref()) {
                    self.apply_object_edit(ctx, before, after);
                } else if !ctx.input(|i| i.pointer.any_down()) && ctx.memory(|m| m.focused().is_none()) {
                    self.edit_open = false;
                }
            });
        });
        self.doc_tabs(root);
    }

    /// The ribbon of the 360° editing mode: one tab with everything for markers and hotspots.
    fn pano_chrome(&mut self, root: &mut egui::Ui, _ctx: &egui::Context) {
        let bg = root.visuals().panel_fill;
        let mut leave = false;
        egui::Panel::top("tabs").frame(egui::Frame::new().fill(bg).inner_margin(egui::Margin::symmetric(10, 1))).show_separator_line(false).show(root, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                if ui.add(egui::Label::new(egui::RichText::new("FILE").size(14.0)).sense(egui::Sense::click())).clicked() { leave = true; self.file_view = true; self.file_page = backstage::FilePage::Info; }
                let resp = ui.add(egui::Label::new(egui::RichText::new("360° IMAGE").size(14.0).strong()).sense(egui::Sense::click()));
                let r = resp.rect;
                ui.painter().line_segment([egui::pos2(r.left(), r.bottom() + 3.0), egui::pos2(r.right(), r.bottom() + 3.0)], egui::Stroke::new(2.5, primary(ui)));
            });
            ui.add_space(2.0);
        });
        egui::Panel::top("ribbon").frame(egui::Frame::new().fill(bg).inner_margin(egui::Margin { left: 8, right: 8, top: 2, bottom: 6 })).show_separator_line(false).show(root, |ui| {
            let card = if ui.visuals().dark_mode { egui::Color32::from_gray(46) } else { egui::Color32::WHITE };
            egui::Frame::new().fill(card).corner_radius(RIBBON_RADIUS as u8).inner_margin(egui::Margin::symmetric(6, 4)).show(ui, |ui| {
                flat(ui);
                // Close leads the ribbon and stays put; the groups scroll beside it.
                let full = ui.available_rect_before_wrap();
                let h = BIG_H + CAP_H + 8.0;
                let lead_w = lead_width(ui, "Close", "360° Image");
                if lead_button(ui, egui::Rect::from_min_size(full.min, egui::vec2(lead_w, h)), X, "Close", "360° Image", "Go back to the slide. Your changes are kept") { self.pano_cmd(panos::Cmd::Close); }
                let rest = egui::Rect::from_min_size(full.min + egui::vec2(lead_w, 0.0), egui::vec2((full.width() - lead_w).max(80.0), h));
                ui.scope_builder(egui::UiBuilder::new().max_rect(rest), |ui| {
                    egui::ScrollArea::horizontal().id_salt("pano-ribbon-scroll").auto_shrink([false, true]).show(ui, |ui| {
                        ui.horizontal_top(|ui| { ui.set_min_height(BIG_H); self.pano_ribbon(ui); });
                    });
                });
                ui.allocate_rect(egui::Rect::from_min_size(full.min, egui::vec2(full.width(), h)), egui::Sense::hover());
            });
        });
        self.pano_tabs(root);
        if leave { self.close_pano_edit(); }
    }

    fn pano_ribbon(&mut self, ui: &mut egui::Ui) {
        use panos::Cmd;
        let Some(p) = self.edited_pano() else { return };
        let sel = self.pano.edit.as_ref().and_then(|e| e.selected).and_then(|i| p.pins.get(i).cloned());
        let drawing = self.pano.edit.as_ref().is_some_and(|e| matches!(e.tool, panos::Tool::Draw(_)));
        let mut cmd: Option<Cmd> = None;
        let mut set = p.clone();
        group(ui, "Insert", |ui| {
            ui.horizontal(|ui| {
                if big_button(ui, PIN, "Marker", "A round button on the picture").clicked() { cmd = Some(Cmd::AddMarker); }
                if big_toggle(ui, NODES, "Hotspot", "Draw a shaded area on the picture, corner by corner", drawing).clicked() { cmd = Some(Cmd::StartHotspot); }
            });
        });
        group(ui, "Selected", |ui| {
            ui.horizontal(|ui| {
                let is_marker = sel.as_ref().is_some_and(|s| s.kind == panos::PinKind::Marker);
                ui.add_enabled_ui(is_marker, |ui| {
                    big_menu(ui, SHAPES, "Icon", "The picture on the marker", |ui| {
                        ui.set_min_width(250.0);
                        egui::Grid::new("pano-icons").spacing([4.0, 4.0]).show(ui, |ui| {
                            for (k, name) in panos::MARKER_ICONS.iter().enumerate() {
                                let Some(icon) = crate::icons::all().iter().find(|i| i.names.contains(name)) else { continue };
                                let on = sel.as_ref().is_some_and(|s| s.icon == *name);
                                let (rect, resp) = ui.allocate_exact_size(egui::vec2(36.0, 36.0), egui::Sense::click());
                                if on || resp.hovered() { ui.painter().rect_filled(rect, 4.0, ui.visuals().selection.bg_fill.gamma_multiply(if on { 0.5 } else { 0.25 })); }
                                ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, icon.ch.to_string(), egui::FontId::new(22.0, icons()), text_col(ui));
                                if resp.on_hover_text(icon.title()).clicked() { cmd = Some(Cmd::SetIcon(Some(icon))); }
                                if (k + 1) % 6 == 0 { ui.end_row(); }
                            }
                        });
                        ui.separator();
                        if ui.button("No icon").clicked() { cmd = Some(Cmd::SetIcon(None)); }
                    });
                });
                ui.add_enabled_ui(sel.is_some(), |ui| {
                    ui.vertical(|ui| {
                        rows(ui);
                        let mut c = sel.as_ref().map(|s| s.color());
                        if color_menu(ui, BUCKET, "The color of the selected marker or hotspot", &mut c, None) { cmd = Some(Cmd::SetColor(c)); }
                        ui.label(egui::RichText::new("Color").size(11.5));
                    });
                    if big_button(ui, BOLT, "Trigger", "Run something when the learner presses the selected marker or hotspot").clicked() { cmd = Some(Cmd::AddTrigger); }
                    if big_button(ui, ARROW_COUNTER, "Switch\nType", "Turn a marker into a hotspot, or the other way round").clicked() { cmd = Some(Cmd::SwitchType); }
                    if big_button(ui, TRASH, "Delete", "Remove the selected marker or hotspot").clicked() { cmd = Some(Cmd::Delete); }
                });
            });
        });
        group(ui, "Cards", |ui| {
            ui.horizontal(|ui| {
                big_menu(ui, if set.dark { MOON } else { SUN }, "Theme", "Dark or light cards", |ui| {
                    if ui.selectable_label(!set.dark, "Light").clicked() { set.dark = false; }
                    if ui.selectable_label(set.dark, "Dark").clicked() { set.dark = true; }
                });
                big_menu(ui, SQUARE_BORDER, "Corners", "Square or rounded cards", |ui| {
                    if ui.selectable_label(!set.rounded, "Square").clicked() { set.rounded = false; }
                    if ui.selectable_label(set.rounded, "Rounded").clicked() { set.rounded = true; }
                    ui.separator();
                    let mut hide = !set.connector;
                    if ui.checkbox(&mut hide, "Hide connector").clicked() { set.connector = !hide; }
                });
            });
        });
        group(ui, "Effects", |ui| {
            ui.vertical(|ui| {
                rows(ui);
                ui.checkbox(&mut set.pulse, "Pulse items if unvisited");
                ui.checkbox(&mut set.reveal, "Reveal hotspots").on_hover_text("Hotspots are shaded all the time. Off: they show only under the pointer, and a guided tour gives a hint");
            });
        });
        group(ui, "View", |ui| {
            if big_button(ui, FRAME, "Initial\nView", "The learner starts by looking at what is on the canvas now").clicked() { cmd = Some(Cmd::SetInitialView); }
        });
        if set != p { self.pano_set(set); }
        if let Some(c) = cmd { self.pano_cmd(c); }
    }

    /// Take what the color menus left for the app: a color to add to the palette, a color that was just used, a request to edit
    /// the palette. (The menus have no access to the project, so they leave notes.)
    pub(super) fn sync_color_events(&mut self, ctx: &egui::Context) {
        let Some(proj) = self.project.as_mut() else { return };
        if let Some(c) = ctx.data_mut(|d| d.remove_temp::<Rgba>(egui::Id::new("palette-add"))) {
            let before = Snap::of(proj);
            let n = proj.palette.len();
            proj.add_to_palette(c);
            if proj.palette.len() != n {
                self.undo.push(before);
                self.redo.clear();
                self.dirty = true;
                self.status = format!("Added {} to the palette", Rgba(c.0, c.1, c.2, 255).hex());
            }
        }
        if let Some(c) = ctx.data_mut(|d| d.remove_temp::<Rgba>(egui::Id::new("picked-color"))) { self.pending_recent = Some(c); }
        if let (Some(c), false) = (self.pending_recent, ctx.input(|i| i.pointer.any_down())) {
            proj.note_color(c);
            self.pending_recent = None;
        }
        // What the menus show.
        ctx.data_mut(|d| {
            d.insert_temp(egui::Id::new("palette-colors"), proj.palette.clone());
            d.insert_temp(egui::Id::new("recent-colors"), proj.recent_colors.clone());
        });
        if ctx.data_mut(|d| d.remove_temp::<bool>(egui::Id::new("open-project-colors"))).unwrap_or(false) { self.theme_colors_open = true; }
    }

    /// The background of the current slide (or layout): none, a color, a gradient you build, or a picture. A window rather
    /// than a menu, because a picker opened inside a menu closes the menu.
    pub(super) fn background_window(&mut self, ctx: &egui::Context) {
        if !self.background_open { return; }
        let cur = self.slide;
        let Some(key) = self.project.as_ref().filter(|p| cur < p.slides.len()).map(|p| match &p.slides[cur].background { Some(Background::Image { asset }) => asset.clone(), _ => String::new() }) else { self.background_open = false; return };
        let Some(proj) = self.project.as_mut() else { return };
        let picture_name = proj.assets.get(&key).map(|a| a.name.clone());
        let before = Snap::of(proj);
        let theme = proj.theme.clone();
        let inherits = cur > 0;
        let mut open = true;
        let mut changed = false;
        let mut want_picture = false;
        let slide_size = (proj.slides[cur].width, proj.slides[cur].height);
        let bg = &mut proj.slides[cur].background;
        crate::tool_window_at(ctx, "background", "Background", [352.0, 420.0], true, &mut open, |ui| {
            // The picker takes the width the window has, so there is no empty strip beside it.
            let w = ui.available_width().clamp(240.0, 720.0);
            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("color-entry-width"), w));
            // What kind of background. A slide with none uses its layout's (or plain white on the master).
            let kind = match &*bg { None => 1, Some(Background::Solid(_)) => 1, Some(Background::Gradient { .. }) => 2, Some(Background::Image { .. }) => 3 };
            let mut pick = kind;
            ui.columns(3, |cols| {
                for (i, label) in ["Color", "Gradient", "Picture"].into_iter().enumerate() {
                    if cols[i].add_sized([cols[i].available_width(), 24.0], egui::Button::selectable(kind == i + 1, label)).clicked() { pick = i + 1; }
                }
            });
            if pick != kind {
                match pick {
                    1 => { *bg = Some(Background::Solid(bg.as_ref().map(|b| b.base_color()).unwrap_or(Rgba(255, 255, 255, 255)))); changed = true; }
                    2 => {
                        let mut g = Background::gradient(135.0, theme.color(4), theme.color(5));
                        if let Background::Gradient { geom, .. } = &mut g { geom.line = Some(brainbend_doc::GradientGeom::fitted_line(135.0, slide_size.0, slide_size.1)); }
                        *bg = Some(g);
                        changed = true;
                    }
                    _ => want_picture = true,
                }
            }
            ui.add_space(8.0);
            match bg {
                None => {
                    // Nothing set yet: offer the color picker on white. The slide changes once a color is chosen.
                    let mut cur_c = Some(Rgba(255, 255, 255, 255));
                    if color_popup(ui, &mut cur_c, None) { if let Some(n) = cur_c { *bg = Some(Background::Solid(n)); changed = true; } }
                }
                Some(Background::Solid(c)) => {
                    let mut cur_c = Some(*c);
                    if color_popup(ui, &mut cur_c, None) { if let Some(n) = cur_c { *c = n; changed = true; } }
                }
                Some(Background::Gradient { angle, stops, kind, geom }) => changed |= gradient_editor(ui, angle, stops, kind, geom, slide_size),
                Some(Background::Image { .. }) => {
                    ui.label(picture_name.as_deref().unwrap_or("Picture"));
                    ui.weak("The picture fills the slide.");
                    ui.add_space(4.0);
                    if ui.button("Choose another picture…").clicked() { want_picture = true; }
                }
            }
            if inherits && bg.is_some() {
                ui.add_space(8.0);
                if ui.button("Use the layout's background").clicked() { *bg = None; changed = true; }
            }
        });
        ctx.data_mut(|d| d.remove_temp::<f32>(egui::Id::new("color-entry-width")));
        if changed { self.commit_project_edit(ctx, before); }
        if want_picture {
            if let Some((key, ..)) = self.load_image_asset_titled("Background Picture") {
                let before = Snap::of(self.project.as_ref().unwrap());
                self.project.as_mut().unwrap().slides[cur].background = Some(Background::Image { asset: key });
                self.commit_project_edit(ctx, before);
            }
        }
        if !open { self.background_open = false; }
    }

    /// While the Background window shows a gradient, dots on the slide set where it starts, ends and centres: drag them.
    pub(super) fn background_handles(&mut self, ui: &mut egui::Ui, painter: &egui::Painter, origin: egui::Pos2, scale: f32, slide_idx: usize) {
        if !self.background_open || slide_idx != self.slide { return; }
        let Some(proj) = self.project.as_ref() else { return };
        let Some(sl) = proj.slides.get(slide_idx) else { return };
        let (w, h) = (sl.width, sl.height);
        let Some(Background::Gradient { angle, stops, kind, geom }) = sl.background.clone() else { return };
        let (Some(first), Some(last)) = (stops.first().map(|s| s.1), stops.last().map(|s| s.1)) else { return };
        use brainbend_doc::GradientKind::*;
        let at = |p: (f32, f32)| origin + egui::vec2(p.0, p.1) * scale;
        let ink = egui::Stroke::new(3.0, egui::Color32::from_black_alpha(160));
        let line = egui::Stroke::new(1.5, egui::Color32::WHITE);
        // (name, where it is on screen, its fill)
        let mut handles: Vec<(&str, egui::Pos2, egui::Color32)> = Vec::new();
        let centre = (geom.center.0 * w, geom.center.1 * h);
        match kind {
            Linear | Reflected => {
                // A gradient that has not been moved yet runs corner to corner, whose ends are off the slide: show a line
                // that is on it, and make that the gradient's line as soon as a dot is dragged.
                let [x0, y0, x1, y1] = geom.line.unwrap_or_else(|| brainbend_doc::GradientGeom::fitted_line(angle, w, h));
                let (a, b) = ((x0 * w, y0 * h), (x1 * w, y1 * h));
                painter.line_segment([at(a), at(b)], ink);
                painter.line_segment([at(a), at(b)], line);
                handles.push(("start", at(a), color(first)));
                handles.push(("end", at(b), color(last)));
            }
            Radial => {
                let r = geom.radius;
                let ring: Vec<egui::Pos2> = (0..=64).map(|i| { let t = i as f32 / 64.0 * std::f32::consts::TAU; at((centre.0 + r * w * t.cos(), centre.1 + r * h * t.sin())) }).collect();
                painter.add(egui::Shape::line(ring.clone(), ink));
                painter.add(egui::Shape::line(ring, line));
                handles.push(("centre", at(centre), color(first)));
                handles.push(("size", at((centre.0 + r * w, centre.1)), color(last)));
            }
            Angular => {
                let a = angle.to_radians();
                let tip = (centre.0 + a.sin() * w.min(h) * 0.25, centre.1 - a.cos() * w.min(h) * 0.25);
                painter.line_segment([at(centre), at(tip)], ink);
                painter.line_segment([at(centre), at(tip)], line);
                handles.push(("centre", at(centre), color(first)));
                handles.push(("angle", at(tip), color(last)));
            }
        }
        for (name, pos, fill) in handles {
            let r = ui.interact(egui::Rect::from_center_size(pos, egui::vec2(22.0, 22.0)), egui::Id::new(("background-handle", name)), egui::Sense::drag());
            r.clone().on_hover_cursor(egui::CursorIcon::Grab);
            painter.circle(pos, 8.0, fill, egui::Stroke::new(3.0, egui::Color32::from_black_alpha(160)));
            painter.circle_stroke(pos, 8.0, egui::Stroke::new(1.5, egui::Color32::WHITE));
            if r.drag_started() { self.snapshot(); }
            let Some(ptr) = r.interact_pointer_pos().filter(|_| r.dragged()) else { continue };
            let p = ((ptr.x - origin.x) / scale, (ptr.y - origin.y) / scale);
            let Some(Background::Gradient { angle, geom, .. }) = self.project.as_mut().and_then(|p| p.slides.get_mut(slide_idx)).and_then(|s| s.background.as_mut()) else { continue };
            if geom.line.is_none() && matches!(kind, Linear | Reflected) { geom.line = Some(brainbend_doc::GradientGeom::fitted_line(*angle, w, h)); }
            match name {
                "start" => { let (_, b) = geom.line_px(*angle, w, h); *angle = geom.set_line_px(p, b, w, h); }
                "end" => { let (a, _) = geom.line_px(*angle, w, h); *angle = geom.set_line_px(a, p, w, h); }
                "centre" => geom.center = ((p.0 / w).clamp(0.0, 1.0), (p.1 / h).clamp(0.0, 1.0)),
                "size" => geom.radius = ((p.0 - geom.center.0 * w).abs() / w).clamp(0.02, 3.0),
                _ => *angle = (p.0 - centre.0).atan2(-(p.1 - centre.1)).to_degrees().rem_euclid(360.0),
            }
            self.dirty = true;
        }
    }

    /// The author's brand colors: pick one to change it with the picker, add more, remove one, and drag them into order.
    /// Every color menu offers them first.
    pub(super) fn theme_colors_window(&mut self, ctx: &egui::Context) {
        if !self.theme_colors_open { return; }
        let Some(proj) = self.project.as_mut() else { self.theme_colors_open = false; return };
        let mut open = true;
        let before = Snap::of(proj);
        let mut changed = false;
        let sel_id = egui::Id::new("brand-colors-selected");
        let draft_id = egui::Id::new("brand-colors-draft");
        let mut sel: usize = ctx.data(|d| d.get_temp(sel_id)).unwrap_or(0);
        let mut draft: Rgba = ctx.data(|d| d.get_temp(draft_id)).unwrap_or(Rgba(79, 129, 189, 255));
        let mut mv: Option<(usize, usize)> = None;
        let mut remove = false;
        let mut add = false;
        crate::tool_window_at(ctx, "brand-colors", "Brand Colors", [360.0, 260.0], true, &mut open, |ui| {
            // The picker takes the width the window has, so there is no empty strip beside it.
            let w = ui.available_width().clamp(240.0, 720.0);
            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("color-entry-width"), w));
            ui.weak("Your project's colors. They show first in every color menu. Colors already on slides do not change.");
            ui.add_space(6.0);
            let n = proj.palette.len();
            sel = sel.min(n.saturating_sub(1));
            if n == 0 {
                ui.label("No colors yet. Choose one below and add it.");
            } else {
                // The palette: click one to change it; drag one to a new place to reorder.
                let mut rects: Vec<egui::Rect> = Vec::new();
                let mut dragged: Option<usize> = None;
                let mut dropped = false;
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                    for i in 0..n {
                        let (rect, resp) = ui.allocate_exact_size(egui::vec2(34.0, 34.0), egui::Sense::click_and_drag());
                        rects.push(rect);
                        ui.painter().rect_filled(rect, 4.0, color(proj.palette[i]));
                        let on = i == sel;
                        ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(if on { 3.0 } else { 1.0 }, if on { ACCENT } else { ui.visuals().weak_text_color() }), egui::StrokeKind::Inside);
                        let name = format!("Brand color {}", proj.palette[i].hex());
                        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &name));
                        if resp.clicked() { sel = i; }
                        if resp.dragged() { dragged = Some(i); sel = i; ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing); }
                        if resp.drag_stopped() { dragged = Some(i); dropped = true; }
                        resp.on_hover_text(format!("{}\nDrag to reorder", proj.palette[i].hex()));
                    }
                });
                if let (Some(from), Some(p)) = (dragged, ui.input(|i| i.pointer.latest_pos())) {
                    if let Some(to) = rects.iter().position(|r| r.expand(3.0).contains(p)).filter(|&t| t != from) {
                        if dropped { mv = Some((from, to)); }
                        else { ui.painter().rect_stroke(rects[to].expand(2.0), 5.0, egui::Stroke::new(2.0, egui::Color32::WHITE), egui::StrokeKind::Outside); }
                    }
                }
            }
            ui.add_space(8.0);
            // The picker: for the chosen color, or, with an empty palette, for the color about to be added.
            if n > 0 {
                if color_entry(ui, &mut proj.palette[sel]) { changed = true; }
            } else {
                color_entry(ui, &mut draft);
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if n == 0 {
                    if ui.button("Add to palette").clicked() { add = true; }
                } else {
                    if ui.add_enabled(n < Project::PALETTE_MAX, egui::Button::new("+ Add color")).on_hover_text("Add a copy of this color, then change it").clicked() { add = true; }
                    if ui.button("Remove").on_hover_text("Remove this color from the palette").clicked() { remove = true; }
                }
            });
        });
        if add {
            let c = if proj.palette.is_empty() { draft } else { proj.palette[sel.min(proj.palette.len() - 1)] };
            proj.palette.push(c);
            sel = proj.palette.len() - 1;
            changed = true;
        }
        if remove && !proj.palette.is_empty() {
            proj.palette.remove(sel.min(proj.palette.len() - 1));
            sel = sel.saturating_sub(1);
            changed = true;
        }
        if let Some((from, to)) = mv {
            let c = proj.palette.remove(from);
            proj.palette.insert(to, c);
            sel = to;
            changed = true;
        }
        ctx.data_mut(|d| { d.insert_temp(sel_id, sel); d.insert_temp(draft_id, draft); });
        if changed { self.commit_project_edit(ctx, before); }
        if !open { self.theme_colors_open = false; }
    }

    /// Custom title bar: window buttons stay native on macOS, then Save / Undo / Redo / Preview,
    /// the centred title, and the theme toggle plus Publish at the right.
    fn title_bar(&mut self, root: &mut egui::Ui, ctx: &egui::Context) {
        let bg = root.visuals().panel_fill;
        egui::Panel::top("titlebar").frame(egui::Frame::new().fill(bg).inner_margin(egui::Margin::symmetric(8, 3))).show_separator_line(false).show(root, |ui| {
            flat(ui);
            let bar = ui.max_rect();
            // Drag the window by its empty area; double-click maximizes.
            let drag = ui.interact(bar, egui::Id::new("titlebar-drag"), egui::Sense::click_and_drag());
            if drag.drag_started() { ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag); }
            if drag.double_clicked() {
                let max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
            }
            let name = self.project.as_ref().map(|p| p.name.clone()).unwrap_or_default();
            let title = if name.is_empty() { "BrainBend Studio".to_string() } else { format!("{name}{}", if self.dirty { "  •" } else { "" }) };
            ui.painter().text(egui::pos2(bar.center().x, bar.center().y), egui::Align2::CENTER_CENTER, title, egui::FontId::proportional(14.0), ui.visuals().text_color());
            let has = self.project.is_some();
            ui.horizontal(|ui| {
                ui.set_min_height(24.0);
                ui.spacing_mut().item_spacing.x = 2.0;
                ui.add_space(if cfg!(target_os = "macos") { 76.0 } else { 4.0 });
                if has { ui.scope(|ui| {
                    if title_icon(ui, SAVE, "Save (⌘S)").clicked() { self.save(false); }
                    if ui.add_enabled(!self.undo.is_empty(), egui::Button::new(icon_text(UNDO, 17.0)).min_size(egui::vec2(24.0, 24.0))).on_hover_text("Undo (⌘Z)").clicked() { self.undo(); }
                    if ui.add_enabled(!self.redo.is_empty(), egui::Button::new(icon_text(REDO, 17.0)).min_size(egui::vec2(24.0, 24.0))).on_hover_text("Redo (⇧⌘Z)").clicked() { self.redo(); }
                    if title_icon(ui, SEARCH, "Find and replace text in the whole project (⌘F)").clicked() { self.find.get_or_insert_with(Default::default).focus = true; }
                }); }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(10.0);
                    let dark = ctx.theme() == egui::Theme::Dark;
                    // The moon is black so it shows on the light theme; the sun keeps its yellow.
                    let ink = text_col(ui);
                    let toggle = pill(ui, "theme", Pill { icon: Some((if dark { SUN } else { MOON }, if dark { tint(SUN) } else { egui::Color32::BLACK })), label: "", caret: false, fill: Some(ink.gamma_multiply(0.14)), ink, outline: true, bold: false, radius: egui::CornerRadius { nw: 0, ne: 0, sw: 6, se: 6 }, lift: 3.0, min_w: 34.0 })
                        .on_hover_text(if dark { "Switch to light mode" } else { "Switch to dark mode" });
                    if toggle.clicked() {
                        ctx.set_theme(if dark { egui::ThemePreference::Light } else { egui::ThemePreference::Dark });
                    }
                    // A newer release is out: a red button to the left of the theme toggle opens the update window.
                    if let Some(r) = self.updater.available().cloned() {
                        ui.add_space(4.0);
                        let white = egui::Color32::WHITE;
                        let (label, fill) = match &self.updater.state {
                            update::State::Downloading { .. } => ("Updating…", egui::Color32::from_rgb(200, 36, 36)),
                            update::State::Ready { .. } => ("Restart to update", egui::Color32::from_rgb(30, 140, 70)),
                            _ => ("Update available", egui::Color32::from_rgb(200, 36, 36)),
                        };
                        let alert = pill(ui, "update", Pill { icon: Some((WARNING_CIRCLE, white)), label, caret: false, fill: Some(fill), ink: white, outline: false, bold: true, radius: egui::CornerRadius { nw: 0, ne: 0, sw: 6, se: 6 }, lift: 3.0, min_w: 60.0 })
                            .on_hover_text(format!("Version {} is available. Click to update.", r.version));
                        alert.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
                        if alert.clicked() { self.updater.dialog_open = true; }
                    }
                });
            });
        });
    }

    /// Help: documentation, reporting problems, the community, and checking for a newer version.
    fn help_tab(&mut self, ui: &mut egui::Ui) {
        group(ui, "Help", |ui| {
            ui.horizontal(|ui| {
                ui.add_enabled_ui(false, |ui| { big_button(ui, BOOK_OPEN, "Docs", "Documentation is coming soon"); });
                if big_button(ui, BUG, "Submit Issue", "Report a problem or suggest a feature on GitHub").clicked() { update::open_in_browser(ISSUES_URL); }
            });
        });
        group(ui, "Community", |ui| {
            if big_button(ui, DISCORD, "BrainBend Community", "Join the BrainBend community on Discord").clicked() { update::open_in_browser(COMMUNITY_URL); }
        });
        group(ui, "AI", |ui| {
            if big_button(ui, BOLT, "Set up AI", "Choose how to use AI to build your lesson: your own API key, or a tool that uses your subscription").clicked() { self.right_tab = crate::AI_TAB; self.assistant.setup_open = true; }
        });
        group(ui, "Windows", |ui| {
            let mut inside = self.windows_inside;
            if ui.checkbox(&mut inside, "Open tool windows inside the app").on_hover_text("Settings, spelling, search and the other tool windows open as panels in the app instead of as windows of their own. Best with a tiling window manager.").changed() {
                self.windows_inside = inside;
                crate::prefs::set_bool("tool_windows_inside", inside);
            }
        });
        group(ui, "Updates", |ui| {
            ui.horizontal(|ui| {
                let checking = matches!(self.updater.state, update::State::Checking(_));
                let check = ui.add_enabled_ui(!checking, |ui| big_button(ui, REFRESH, "Check for Updates", "Look for a newer version of BrainBend Studio")).inner;
                if check.clicked() { self.updater.check_now(ui.ctx()); }
            });
        });
    }

    fn home_tab(&mut self, ui: &mut egui::Ui, normal: bool, obj: Option<&mut Object>, changed: &mut bool) {
        let has_sel = self.selected.is_some();
        let n_sel = self.selection().len();
        let mut list_click: Option<ListStyle> = None;
        group(ui, "Clipboard", |ui| {
            ui.horizontal(|ui| {
                ui.add_enabled_ui(normal && self.clipboard.is_some(), |ui| {
                    if big_button(ui, PASTE, "Paste", "Paste (⌘V)").clicked() { self.paste(); }
                });
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.add_enabled_ui(has_sel, |ui| {
                        if tiny_button(ui, CUT, false, "Cut").clicked() { self.copy_selected(); self.delete_selected(); }
                        if tiny_button(ui, COPY, false, "Copy").clicked() { self.copy_selected(); }
                    });
                    let on = self.painter.is_some();
                    ui.add_enabled_ui(has_sel || on, |ui| {
                        if tiny_button(ui, PAINTER, on, "Format Painter: copy this object's look, then click another object to give it the same look").clicked() {
                            self.painter = if on { None } else { self.selected_object().map(|o| (o.id.clone(), Box::new(o.clone()))) };
                            self.status = if self.painter.is_some() { "Format Painter: click the object that should get this look. Esc cancels.".into() } else { String::new() };
                        }
                    });
                });
            });
        });
        // Every control is always shown; with nothing to apply to, a stand-in object is edited and the
        // controls are disabled, so the ribbon keeps its shape as the selection changes.
        let mut stand_in = Object::new("none", "none", Kind::Rect, 0.0, 0.0, 0.0, 0.0);
        let enabled = obj.is_some() && normal;
        let obj = obj.unwrap_or(&mut stand_in);
        let is_media = matches!(obj.kind, Kind::Image | Kind::Video | Kind::Audio | Kind::Web(_) | Kind::Pano(_) | Kind::Unsupported(_));
        let is_line = matches!(obj.kind, Kind::Line | Kind::LineUp);
        let has_text = !is_media && !is_line && !matches!(obj.kind, Kind::Path(_) | Kind::Hotspot);

        {
            let on = enabled && has_text;
            group(ui, "Font", |ui| { ui.add_enabled_ui(on, |ui| { ui.horizontal(|ui| {
            self.text_styles_menu(ui);
            ui.vertical(|ui| {
            rows(ui);
                ui.horizontal(|ui| {
                    framed(ui, |ui| {
                        *changed |= crate::font_store::font_menu(ui, "rb-font", &mut obj.style.font_family, 92.0);
                        *changed |= ui.add(egui::DragValue::new(&mut obj.style.size).range(4.0..=400.0).suffix(" pt")).on_hover_text("Font size").changed();
                        let sizes = {
                            let (r, resp) = ui.allocate_exact_size(egui::vec2(14.0, 20.0), egui::Sense::click());
                            chevron(ui.painter(), r.center() + egui::vec2(0.0, 0.5), 3.0, if resp.hovered() { ui.visuals().strong_text_color() } else { text_col(ui) });
                            resp.on_hover_text("Common sizes")
                        };
                        egui::Popup::from_toggle_button_response(&sizes).close_behavior(egui::PopupCloseBehavior::CloseOnClick).show(|ui| {
                            for z in [18.0, 24.0, 28.0, 32.0, 36.0, 40.0, 48.0, 60.0, 72.0, 96.0, 120.0] {
                                if ui.selectable_label(obj.style.size == z, format!("{z:.0}")).clicked() { obj.style.size = z; *changed = true; }
                            }
                        });
                    });
                    let part = self.text_sel;
                    let cur = part.and_then(|(a, _)| obj.char_fmts().get(a).and_then(|(_, f)| f.size)).unwrap_or(obj.style.size);
                    for (up, tip) in [(true, "Grow font"), (false, "Shrink font")] {
                        if compact_text_button(ui, grow_job(ui, up), tip).clicked() {
                            let next = step_size(cur, up);
                            self.edit_text_range(move |o, a, b, whole| { if whole { o.style.size = next; } o.format_range(a, b, &Attr::Size(next)); });
                        }
                    }
                    if icon_button(ui, CLEAR_FMT, false, "Clear formatting: back to the object's plain text style").clicked() {
                        self.edit_text_range(|o, a, b, whole| {
                            o.format_range(a, b, &Attr::Clear);
                            if whole { o.style.bold = false; o.style.italic = false; o.style.underline = false; }
                        });
                    }
                });
                ui.horizontal(|ui| {
                    // While text is selected on the slide these format just the selection.
                    let part = self.text_sel;
                    if icon_button(ui, BOLD, obj.style.bold, "Bold (⌘B)").clicked() {
                        if part.is_some() { self.format_selection(Attr::ToggleBold); } else { obj.style.bold = !obj.style.bold; *changed = true; }
                    }
                    if icon_button(ui, ITALIC, obj.style.italic, "Italic (⌘I)").clicked() {
                        if part.is_some() { self.format_selection(Attr::ToggleItalic); } else { obj.style.italic = !obj.style.italic; *changed = true; }
                    }
                    if icon_button(ui, UNDERLINE, obj.style.underline, "Underline (⌘U)").clicked() {
                        if part.is_some() { self.format_selection(Attr::ToggleUnderline); } else { obj.style.underline = !obj.style.underline; *changed = true; }
                    }
                    // The text the next buttons describe: the chosen part, or all of it.
                    let (ra, rb) = part.unwrap_or((0, usize::MAX));
                    let span: Vec<Fmt> = obj.char_fmts().into_iter().enumerate().filter(|(i, _)| *i >= ra && *i < rb).map(|(_, (_, f))| f).collect();
                    let all_strike = !span.is_empty() && span.iter().all(|f| f.strike.unwrap_or(false));
                    if icon_button(ui, STRIKE, all_strike, "Strikethrough").clicked() { self.edit_text_range(|o, a, b, _| o.format_range(a, b, &Attr::ToggleStrike)); }
                    icon_popup(ui, SPACING, "Character spacing", egui::PopupCloseBehavior::CloseOnClick, |ui| {
                        let cur = span.first().and_then(|f| f.spacing).unwrap_or(0.0);
                        for (v, l) in [(-2.0, "Very tight"), (-1.0, "Tight"), (0.0, "Normal"), (2.0, "Loose"), (4.0, "Very loose")] {
                            if ui.selectable_label((cur - v).abs() < 0.01, l).clicked() { self.edit_text_range(move |o, a, b, _| o.format_range(a, b, &Attr::Spacing(v))); }
                        }
                    });
                    let mut aa = egui::text::LayoutJob::default();
                    aa.append("Aa", 0.0, egui::text::TextFormat { font_id: egui::FontId::proportional(13.0), color: text_col(ui), ..Default::default() });
                    let case = compact_text_button(ui, aa, "Change case");
                    egui::Popup::from_toggle_button_response(&case).close_behavior(egui::PopupCloseBehavior::CloseOnClick).show(|ui| {
                        for (c, l) in [(brainbend_doc::richtext::Case::Sentence, "Sentence case."), (brainbend_doc::richtext::Case::Lower, "lowercase"), (brainbend_doc::richtext::Case::Upper, "UPPERCASE"), (brainbend_doc::richtext::Case::Capitalize, "Capitalize Each Word"), (brainbend_doc::richtext::Case::Toggle, "tOGGLE cASE")] {
                            if ui.button(l).clicked() { self.edit_text_range(move |o, a, b, _| o.change_case(a, b, c)); ui.close(); }
                        }
                    });
                    let mut hl = span.first().and_then(|f| f.highlight);
                    if color_menu(ui, HIGHLIGHT, "Text highlight color", &mut hl, Some("No highlight")) { self.edit_text_range(move |o, a, b, _| o.format_range(a, b, &Attr::Highlight(hl))); }
                    let mut c = Some(obj.style.color);
                    if color_menu(ui, TEXT_AA, "Text color", &mut c, None) { if let Some(c) = c { obj.style.color = c; *changed = true; } }
                });
            }); }); }); });
            group(ui, "Paragraph", |ui| { ui.add_enabled_ui(on, |ui| {
            rows(ui);
                ui.horizontal(|ui| {
                    for (a, icon, tip) in [(Align::Left, ALIGN_L, "Align left"), (Align::Center, ALIGN_C, "Center"), (Align::Right, ALIGN_R, "Align right"), (Align::Justify, JUSTIFY, "Justify")] {
                        if icon_button(ui, icon, obj.style.align == a, tip).clicked() { obj.style.align = a; *changed = true; }
                    }
                    ui.add_space(4.0);
                    let ls = obj.list_style();
                    if icon_button(ui, LIST_BULLETS, ls == ListStyle::Bullet, "Bullets").clicked() { list_click = Some(if ls == ListStyle::Bullet { ListStyle::None } else { ListStyle::Bullet }); }
                    if icon_button(ui, LIST_NUM, ls == ListStyle::Number, "Numbering").clicked() { list_click = Some(if ls == ListStyle::Number { ListStyle::None } else { ListStyle::Number }); }
                    if icon_button(ui, INDENT_OUT, false, "Decrease indent").clicked() { self.edit_text_range(|o, a, b, _| o.format_range(a, b, &Attr::Indent(-1))); }
                    if icon_button(ui, INDENT_IN, false, "Increase indent").clicked() { self.edit_text_range(|o, a, b, _| o.format_range(a, b, &Attr::Indent(1))); }
                });
                ui.horizontal(|ui| {
                    icon_popup(ui, LINE_SPACING, "Line spacing", egui::PopupCloseBehavior::CloseOnClick, |ui| {
                        for (v, l) in [(0.0, "Default"), (1.2, "1.0"), (1.38, "1.15"), (1.8, "1.5"), (2.4, "2.0"), (3.0, "2.5")] {
                            if ui.selectable_label((obj.line_spacing - v).abs() < 0.01, l).clicked() { obj.line_spacing = v; *changed = true; }
                        }
                    });
                    let v_icon = match obj.style.valign { VAlign::Top => AL_TOP, VAlign::Middle => AL_CENTER_V, VAlign::Bottom => AL_BOTTOM };
                    icon_popup(ui, v_icon, "Vertical position of the text in its box", egui::PopupCloseBehavior::CloseOnClick, |ui| {
                        for (v, icon, l) in [(VAlign::Top, AL_TOP, "Top"), (VAlign::Middle, AL_CENTER_V, "Middle"), (VAlign::Bottom, AL_BOTTOM, "Bottom")] {
                            if menu_item(ui, icon, l).clicked() { obj.style.valign = v; *changed = true; }
                        }
                    });
                    icon_popup(ui, ROTATE, "Direction of the text: across, or turned a quarter", egui::PopupCloseBehavior::CloseOnClick, |ui| {
                        for (v, l) in [(0i16, "Horizontal"), (90, "Rotate all text 90°"), (270, "Rotate all text 270°")] {
                            if ui.selectable_label(obj.text_rotation == v, l).clicked() { obj.text_rotation = v; *changed = true; }
                        }
                    });
                });
            }); });
        }
        // The selected object's own groups follow; the tab strip names the object they act on.
        if enabled && self.format_title().is_some() {
            self.format_groups(ui, obj, changed);
        } else {
            self.arrange_group(ui, has_sel, n_sel);
            drawing_group(ui, obj, enabled, changed);
            effects_group(ui, obj, enabled, changed);
        }
        if let Some(st) = list_click { self.set_list_selected(st); }
    }

    /// What Home calls the selected object in its caption for the selected object, or `None` when it has no such tab yet.
    pub(super) fn format_title(&self) -> Option<&'static str> {
        if self.in_master_view() || self.try_mode { return None; }
        let o = self.selected_object()?;
        if o.table.is_some() { return Some("TABLE"); }
        match &o.kind {
            Kind::Image => Some("PICTURE"),
            Kind::Video => Some("VIDEO"),
            Kind::Audio => Some("AUDIO"),
            Kind::Hotspot => Some("HOTSPOT"),
            Kind::Rect | Kind::Ellipse | Kind::Path(_) | Kind::Line | Kind::LineUp | Kind::Text => Some(if o.has_button_states() { "BUTTON" } else if matches!(o.kind, Kind::Text) { "TEXT BOX" } else { "SHAPE" }),
            _ => None,
        }
    }

    /// What Home adds for a table: styles, rows and columns, merging, and borders. They act on the chosen cells.
    fn table_groups(&mut self, ui: &mut egui::Ui) {
        use brainbend_doc::tables as t;
        let block = self.cell_block();
        let can_merge = block.as_ref().is_some_and(|(_, (r0, c0, r1, c1))| (r0, c0) != (r1, c1));
        let can_split = self.selected_object().is_some_and(|o| o.table_span.is_some());
        let theme = self.project.as_ref().map(|p| p.theme.clone()).unwrap_or_default();
        // Table styles: five swatches and the three options.
        let mut pick = None;
        let mut opts = self.table_opts;
        let mut opts_changed = false;
        group(ui, "Table Styles", |ui| {
            ui.horizontal(|ui| {
                for st in t::TableStyle::ALL {
                    let look = t::look(st, &theme);
                    let (rect, resp) = ui.allocate_exact_size(egui::vec2(40.0, 34.0), egui::Sense::click());
                    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, st.label()));
                    let (cw, rh) = (rect.width() / 3.0, rect.height() / 3.0);
                    for r in 0..3 {
                        for c in 0..3 {
                            let cell = egui::Rect::from_min_size(rect.min + egui::vec2(c as f32 * cw, r as f32 * rh), egui::vec2(cw, rh));
                            let fill = if r == 0 { look.header } else if r % 2 == 1 { look.band.or(look.body) } else { look.body };
                            ui.painter().rect_filled(cell, 0.0, fill.map(|c| egui::Color32::from_rgb(c.0, c.1, c.2)).unwrap_or(egui::Color32::WHITE));
                            if let Some(sk) = look.stroke { ui.painter().rect_stroke(cell, 0.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(sk.color.0, sk.color.1, sk.color.2)), egui::StrokeKind::Inside); }
                        }
                    }
                    let on = self.table_style == st;
                    ui.painter().rect_stroke(rect, 2.0, egui::Stroke::new(if on || resp.hovered() { 2.0 } else { 1.0 }, if on { ACCENT } else { egui::Color32::GRAY }), egui::StrokeKind::Outside);
                    if resp.on_hover_text(st.label()).clicked() { pick = Some(st); }
                }
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    opts_changed |= ui.checkbox(&mut opts.header_row, "Header row").changed();
                    opts_changed |= ui.checkbox(&mut opts.banded_rows, "Banded rows").changed();
                    opts_changed |= ui.checkbox(&mut opts.first_column, "First column bold").changed();
                });
            });
        });
        // Rows and columns, merging and choosing cells: two menus rather than a row of word buttons.
        let mut op = None;
        let dims = self.selected_object().and_then(|o| o.group.clone()).and_then(|g| self.project.as_ref().map(|p| t::dims(&p.slides[self.slide].layers[self.selected.map(|s| s.0).unwrap_or(0)].objects, &g))).unwrap_or((0, 0));
        let mut merge = false;
        let mut split = false;
        let mut choose: Option<((u16, u16), (u16, u16))> = None;
        group(ui, "Rows & Columns", |ui| {
            ui.horizontal(|ui| {
                big_menu(ui, TABLE, "Add", "Add a row or column next to the chosen cell", |ui| {
                    for (icon, label, o) in [(ARROW_UP, "Row above", TableOp::RowAbove), (ARROW_DOWN, "Row below", TableOp::RowBelow), (AL_LEFT, "Column left", TableOp::ColLeft), (AL_RIGHT, "Column right", TableOp::ColRight)] {
                        if menu_item(ui, icon, label).clicked() { op = Some(o); }
                    }
                });
                big_menu(ui, TRASH, "Delete", "Delete the chosen cell's row or column", |ui| {
                    for (label, o) in [("Delete row", TableOp::DeleteRow), ("Delete column", TableOp::DeleteCol)] {
                        if menu_item(ui, TRASH, label).clicked() { op = Some(o); }
                    }
                });
                big_menu(ui, SQUARE_BORDER, "Cells", "Merge, split or choose cells. Click a cell, then shift-click another to choose a block", |ui| {
                    ui.add_enabled_ui(can_merge, |ui| { if menu_item(ui, SQUARE_BORDER, "Merge cells").clicked() { merge = true; } });
                    ui.add_enabled_ui(can_split, |ui| { if menu_item(ui, SQUARE_BORDER, "Split cell").clicked() { split = true; } });
                    ui.separator();
                    ui.add_enabled_ui(block.is_some(), |ui| {
                        let (r0, c0, r1, c1) = block.as_ref().map(|b| b.1).unwrap_or((0, 0, 0, 0));
                        if menu_item(ui, AL_CENTER_H, "Choose row").clicked() { choose = Some(((r0, 0), (r1, dims.1.saturating_sub(1)))); }
                        if menu_item(ui, AL_CENTER_V, "Choose column").clicked() { choose = Some(((0, c0), (dims.0.saturating_sub(1), c1))); }
                        if menu_item(ui, TABLE, "Choose all cells").clicked() { choose = Some(((0, 0), (dims.0.saturating_sub(1), dims.1.saturating_sub(1)))); }
                    });
                });
            });
        });
        // Borders for the chosen cells.
        let mut borders: Option<(t::BorderSet, Option<Stroke>)> = None;
        group(ui, "Borders", |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let mut c = Some(self.border_color);
                    if color_menu(ui, PEN, "Border color", &mut c, None) { if let Some(c) = c { self.border_color = c; } }
                    framed(ui, |ui| { ui.add(egui::DragValue::new(&mut self.border_width).range(0.5..=24.0).speed(0.25).suffix(" px")); });
                });
                let line = Some(Stroke { color: self.border_color, width: self.border_width });
                let menu = caret_button(ui, "Borders");
                egui::Popup::from_toggle_button_response(&menu).close_behavior(egui::PopupCloseBehavior::CloseOnClick).show(|ui| {
                    use t::BorderSet::*;
                    for (label, set, ln) in [("All", All, line), ("Outside", Outside, line), ("Inside", Inside, line), ("Top", Top, line), ("Bottom", Bottom, line), ("Left", Left, line), ("Right", Right, line), ("No borders", All, None)] {
                        if ui.button(label).clicked() { borders = Some((set, ln)); }
                    }
                });
            });
        });
        if let Some(st) = pick { self.restyle_table(st, opts); } else if opts_changed { self.restyle_table(self.table_style, opts); }
        if let Some(o) = op { self.table_edit(o); }
        if merge { self.merge_cells(); }
        if split { self.split_cell(); }
        if let Some((a, b)) = choose { self.select_cells(a, b); }
        if let Some((set, ln)) = borders { self.set_cell_borders(set, ln); }
    }

    /// The groups of Home that belong to the selected object: styles, fill and outline, and arrangement.
    fn format_groups(&mut self, ui: &mut egui::Ui, obj: &mut Object, changed: &mut bool) {
        let has_sel = self.selected.is_some();
        let n_sel = self.selection().len();
        let mut style_pick: Option<crate::button_styles::StyleDef> = None;
        let mut pick_captions = false;
        if obj.has_button_states() {
            group(ui, "Button Styles", |ui| {
                let corners = &mut self.button_corners;
                big_menu_open(ui, SHAPES, "Styles", "Ready-made button looks: nine colors in four looks, with square, rounded or pill corners", |ui| {
                    if let Some(def) = crate::button_styles::gallery(ui, corners) { style_pick = Some(def); ui.close(); }
                });
            });
        }
        if matches!(obj.kind, Kind::Image) {
            let crop_info = (n_sel == 1 && obj.asset.is_some()).then(|| (obj.id.clone(), obj.crop.is_some()));
            self.picture_group(ui, &crop_info);
        }
        if obj.table.is_some() { self.table_groups(ui); }
        let is_media = matches!(obj.kind, Kind::Video | Kind::Audio);
        if is_media {
            group(ui, "Playback", |ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.spacing_mut().interact_size.y = 20.0;
                    let m = &mut obj.media;
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 14.0;
                        *changed |= ui.checkbox(&mut m.autoplay, "Start automatically").on_hover_text("Plays when the object's turn on the timeline comes; otherwise the learner presses play").changed();
                    });
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 14.0;
                        *changed |= ui.checkbox(&mut m.looped, "Loop").changed();
                        *changed |= ui.checkbox(&mut m.muted, "Muted").changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Volume");
                        let mut pct = (m.volume * 100.0).round();
                        if ui.add(egui::Slider::new(&mut pct, 0.0..=100.0).suffix("%").show_value(true)).changed() { m.volume = pct / 100.0; *changed = true; }
                    });
                });
            });
            if matches!(obj.kind, Kind::Video) {
                group(ui, "Captions", |ui| {
                    ui.vertical(|ui| {
                        ui.label(if obj.captions.is_some() { "Captions: attached" } else { "Captions: none" });
                        ui.horizontal(|ui| {
                            if ui.button("Choose .vtt…").clicked() { pick_captions = true; }
                            if obj.captions.is_some() && ui.button("Remove").clicked() { obj.captions = None; *changed = true; }
                        });
                    });
                });
            }
        }
        if matches!(obj.kind, Kind::Hotspot) { self.hotspot_group(ui, obj, true, n_sel); }
        if !is_media && !matches!(obj.kind, Kind::Hotspot) { drawing_group(ui, obj, true, changed); effects_group(ui, obj, true, changed); }
        self.arrange_group(ui, has_sel, n_sel);
        obj.w = obj.w.max(1.0);
        obj.h = obj.h.max(1.0);
        if let Some(def) = style_pick { self.apply_button_style(&def); }
        if pick_captions {
            if let Some((key, _)) = self.load_media(&["vtt"], "WebVTT captions") {
                self.snapshot();
                let id = obj.id.clone();
                if let Some(o) = self.project.as_mut().and_then(|p| p.slides.get_mut(self.slide)).and_then(|s| s.layers.iter_mut().flat_map(|l| l.objects.iter_mut()).find(|o| o.id == id)) { o.captions = Some(key); }
                self.dirty = true;
            }
        }
    }

    /// Crop, shape and ratio for a single picture. Shared by the selection groups on Home.
    fn picture_group(&mut self, ui: &mut egui::Ui, crop_info: &Option<(String, bool)>) {
        group(ui, "Picture", |ui| {
            ui.horizontal(|ui| {
                ui.add_enabled_ui(crop_info.is_some(), |ui| {
                    let on = crop_info.as_ref().is_some_and(|(id, _)| self.crop_mode.as_deref() == Some(id.as_str()));
                    if big_toggle(ui, CROP, "Crop", "Crop the picture: drag its edges, or drag inside it to move the picture (Esc or Enter to finish)", on).clicked() {
                        self.crop_mode = if on { None } else { crop_info.as_ref().map(|(id, _)| id.clone()) };
                    }
                    ui.add_enabled_ui(crop_info.as_ref().is_some_and(|c| c.1), |ui| {
                        if big_button(ui, ARROW_COUNTER, "Reset", "Remove the crop and show the whole picture").clicked() { self.reset_crop(); }
                    });
                    // Crop to a shape: only the part of the picture inside the shape shows.
                    ui.add_enabled_ui(crop_info.is_some(), |ui| {
                        let cur = self.selected_object().and_then(|o| o.mask);
                        big_menu(ui, CIRCLE, "Shape", "Crop the picture to a shape: oval, rounded, diamond, star ...", |ui| {
                            if ui.selectable_label(cur.is_none(), "None").clicked() { self.set_picture_shape(None); ui.close(); }
                            for shape in brainbend_doc::PictureShape::ALL {
                                if ui.selectable_label(cur == Some(shape), shape.label()).clicked() { self.set_picture_shape(Some(shape)); ui.close(); }
                            }
                        });
                    });
                    // The shape of the crop frame: pick one and the frame closes in to it and keeps it while dragged.
                    ui.add_enabled_ui(crop_info.is_some() && self.crop_mode.is_some(), |ui| {
                        big_menu(ui, FRAME, "Ratio", "Hold the crop to a shape of frame: 1:1, 4:3, 16:9 ...", |ui| {
                            let native = crop_info.as_ref().and_then(|(id, _)| self.picture_ratio(id));
                            let cur = self.crop_aspect;
                            let pick = |ui: &mut egui::Ui, app: &mut Self, label: &str, r: Option<f32>| {
                                let on = match (cur, r) { (None, None) => true, (Some(a), Some(b)) => (a - b).abs() < 0.001, _ => false };
                                if ui.selectable_label(on, label).clicked() { app.set_crop_aspect(r); ui.close(); }
                            };
                            pick(ui, self, "Free", None);
                            if let Some(n) = native { pick(ui, self, "Original", Some(n)); }
                            for (label, w, h) in [("1:1  square", 1.0, 1.0), ("4:3", 4.0, 3.0), ("3:2", 3.0, 2.0), ("16:9", 16.0, 9.0), ("21:9", 21.0, 9.0), ("3:4  portrait", 3.0, 4.0), ("2:3  portrait", 2.0, 3.0), ("9:16  portrait", 9.0, 16.0)] {
                                pick(ui, self, label, Some(w / h));
                            }
                        });
                    });
                });
            });
        });
    }

    /// Outline editing for a single hotspot. Shared by the selection groups on Home.
    fn hotspot_group(&mut self, ui: &mut egui::Ui, obj: &Object, enabled: bool, n_sel: usize) {
        group(ui, "Hotspot", |ui| {
            ui.horizontal(|ui| {
                // Only a single hotspot has an outline to edit: (its id, whether it has an outline that is not just its box).
                let spot = (enabled && n_sel == 1 && matches!(obj.kind, Kind::Hotspot)).then(|| (obj.id.clone(), obj.hit_area.as_ref().is_some_and(|a| !outline::is_box(a)), obj.hit_area.is_some()));
                ui.add_enabled_ui(spot.is_some(), |ui| {
                    let on = spot.as_ref().is_some_and(|(id, _, _)| self.outline_mode.as_deref() == Some(id.as_str()));
                    if big_toggle(ui, NODES, "Outline", "Edit the hotspot's outline: drag a corner, click an edge to add one, Delete removes the chosen corner (Esc or Enter to finish)", on).clicked() {
                        if on { self.end_outline(); } else { self.begin_outline(); }
                    }
                    ui.add_enabled_ui(spot.as_ref().is_some_and(|s| s.1), |ui| {
                        if big_button(ui, FRAME, "Fit", "Shrink the hotspot's frame to its outline").clicked() { self.fit_outline(); }
                    });
                    ui.add_enabled_ui(spot.as_ref().is_some_and(|s| s.2), |ui| {
                        if big_button(ui, ARROW_COUNTER, "Box", "Remove the outline: the whole frame is clickable again").clicked() { self.reset_outline(); }
                    });
                });
            });
        });
    }

    /// Order, group and align the selection. Shared by the selection groups on Home.
    fn arrange_group(&mut self, ui: &mut egui::Ui, has_sel: bool, n_sel: usize) {
        group(ui, "Arrange", |ui| {
            ui.horizontal(|ui| {
                ui.add_enabled_ui(has_sel, |ui| {
                    big_menu(ui, STACK, "Arrange", "Order and align", |ui| {
                        if menu_item(ui, FRONT, "Bring to front").clicked() { self.reorder_selected(true); }
                        if menu_item(ui, BACK, "Send to back").clicked() { self.reorder_selected(false); }
                        ui.separator();
                        ui.add_enabled_ui(n_sel > 1, |ui| { if menu_item(ui, STACK, "Group").clicked() { self.group_selected(); } });
                        if menu_item(ui, STACK, "Ungroup").clicked() { self.ungroup_selected(); }
                        ui.separator();
                        let to_slide = n_sel < 2;
                        ui.weak(if to_slide { "Align to the slide" } else { "Align to each other" });
                        for (icon, axis, mode, label) in [(AL_LEFT, 0u8, 0u8, "Left"), (AL_CENTER_H, 0, 1, "Center"), (AL_RIGHT, 0, 2, "Right"), (AL_TOP, 1, 0, "Top"), (AL_CENTER_V, 1, 1, "Middle"), (AL_BOTTOM, 1, 2, "Bottom")] {
                            if menu_item(ui, icon, label).clicked() { self.align_selected_or_slide(axis, mode); }
                        }
                        ui.separator();
                        ui.add_enabled_ui(n_sel > 2, |ui| {
                            if menu_item(ui, DIST_H, "Distribute horizontally").clicked() { self.align_selected(0, 3); }
                            if menu_item(ui, DIST_V, "Distribute vertically").clicked() { self.align_selected(1, 3); }
                        });
                    });
                });
            });
        });
    }

    fn insert_tab(&mut self, ui: &mut egui::Ui, normal: bool) {
        ui.add_enabled_ui(normal, |ui| {
            group(ui, "Text", |ui| {
                ui.horizontal(|ui| {
                    if big_button(ui, TEXTBOX, "Text Box", "Insert a text box").clicked() { self.insert_text(); }
                    big_menu(ui, TABLE, "Table", "Insert a table", |ui| {
                        ui.set_min_width(0.0);
                        if let Some((r, c)) = table_picker(ui) { self.insert_table(r, c); ui.close(); }
                    });
                    big_menu(ui, HASH, "Slide Number", "Insert a number that follows the slide", |ui| {
                        if menu_item(ui, HASH, "Slide number").clicked() { self.insert_slide_number(false); ui.close(); }
                        if menu_item(ui, HASH, "Slide number of total").clicked() { self.insert_slide_number(true); ui.close(); }
                    });
                    self.symbol_button(ui);
                    // Hyperlink links the chosen text (or all of the object's text). With no text to link it makes a text box
                    // holding the link instead.
                    {
                        let has_text = self.selected_object().is_some_and(|o| !o.text.is_empty());
                        let can = self.selected_object().is_none_or(|o| !matches!(o.kind, Kind::Image | Kind::Video | Kind::Audio | Kind::Hotspot | Kind::Web(_) | Kind::Pano(_) | Kind::Input(_) | Kind::Slider(_) | Kind::Unsupported(_)));
                        let link_btn = ui.add_enabled_ui(can, |ui| big_widget(ui, LINK, "Hyperlink", false, false).on_hover_text("Hyperlink: link the selected text (or all of the object's text) to a web address, or make a text box holding a link")).inner
                            .on_disabled_hover_text("Hyperlink: this object has no text; select a text box or shape, or click an empty part of the slide");
                        let link_id = egui::Id::new("text_link_url");
                        let shown_id = egui::Id::new("text_link_shown");
                        if link_btn.clicked() {
                            let cur = self.selected_object().and_then(|o| {
                                let (a, b) = self.text_sel.unwrap_or((0, usize::MAX));
                                o.char_fmts().into_iter().enumerate().find(|(i, (_, f))| *i >= a && *i < b && f.link.is_some()).and_then(|(_, (_, f))| f.link)
                            }).unwrap_or_default();
                            ui.data_mut(|d| { d.insert_temp(link_id, cur); d.insert_temp(shown_id, String::new()); });
                        }
                        egui::Popup::from_toggle_button_response(&link_btn).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
                            ui.set_min_width(260.0);
                            let mut url: String = ui.data_mut(|d| d.get_temp(link_id)).unwrap_or_default();
                            let mut shown: String = ui.data_mut(|d| d.get_temp(shown_id)).unwrap_or_default();
                            if !has_text {
                                ui.label("Text to show");
                                ui.add(egui::TextEdit::singleline(&mut shown).desired_width(240.0).hint_text("Same as the address"));
                            }
                            ui.label("Web address (https:// or mailto:)");
                            ui.add(egui::TextEdit::singleline(&mut url).desired_width(240.0).hint_text("https://example.com"));
                            ui.data_mut(|d| { d.insert_temp(link_id, url.clone()); d.insert_temp(shown_id, shown.clone()); });
                            let ok = url.starts_with("https://") || url.starts_with("http://") || url.starts_with("mailto:");
                            ui.horizontal(|ui| {
                                let apply = ui.add_enabled(ok, egui::Button::new(if has_text { "Apply" } else { "Insert" })).clicked();
                                let remove = has_text && ui.button("Remove link").clicked();
                                if apply || remove {
                                    if has_text { self.link_selected(apply.then(|| url.trim().to_string())); }
                                    else { self.insert_link_text(shown.trim(), url.trim()); }
                                    ui.close();
                                }
                            });
                        });
                    }
                });
            });
            group(ui, "Media", |ui| {
                ui.horizontal(|ui| {
                    if big_button(ui, IMAGE, "Picture", "Insert a picture").clicked() { self.insert_image(); }
                    big_menu(ui, SHAPES, "Shape", "Insert a shape", |ui| {
                        if let Some(name) = shape_gallery(ui) { self.insert_shape(name); ui.close(); }
                    });
                    if big_button(ui, VIDEO, "Video", "Insert a video").clicked() { self.insert_video(); }
                    if big_button(ui, SPEAKER, "Audio", "Insert audio").clicked() { self.insert_audio(); }
                    big_menu(ui, CHAT, "Caption", "Insert a speech or thought bubble with text", |ui| self.caption_menu(ui));
                    if big_button(ui, PANORAMA, "360° Image", "Pick a panoramic picture the learner can look around, with annotations").clicked() { self.insert_pano(); }
                    if big_button(ui, FUNCTION, "Equation", "Type an equation in LaTeX; it is typeset as a shape you can resize and recolor").clicked() { self.open_equation_dialog(None); }
                    big_menu(ui, STAR, "Icons", "Insert an icon as a vector shape you can recolor and resize", |ui| self.icon_gallery(ui));
                    if big_button(ui, GLOBE, "Web Object", "Show a web page inside the slide").clicked() { self.insert_web_object(); }
                });
            });
            group(ui, "Interactive Objects", |ui| {
                ui.horizontal(|ui| {
                    if big_button(ui, CLICK, "Button", "Insert a button with hover and click looks").clicked() { self.insert_button(); }
                    if big_button(ui, SLIDERS, "Slider", "Insert a slider the learner drags to set a number").clicked() { self.insert_slider(); }
                    big_menu(ui, CLICK, "Hotspot", "Insert an invisible area the learner clicks", |ui| {
                        if menu_item(ui, RECT, "Rectangle").clicked() { self.insert_hotspot(false); ui.close(); }
                        if menu_item(ui, CIRCLE, "Oval").clicked() { self.insert_hotspot(true); ui.close(); }
                        if menu_item(ui, POLYLINE, "Freeform (click its corners)").clicked() {
                            self.draw_outline = Some(vec![]);
                            self.draw_slide = self.slide;
                            self.selected = None;
                            self.multi.clear();
                            self.status = "Click each corner of the hotspot. Enter, double-click or the first corner finishes.".into();
                            ui.close();
                        }
                    });
                    big_menu(ui, GAUGE, "Dial", "Insert a dial the learner turns to set a number", |ui| self.dial_menu(ui));
                    big_menu(ui, CHECK, "Input", "Insert something the learner ticks, picks or types", |ui| self.input_menu(ui));
                    big_menu(ui, PIN, "Marker", "Insert a round button with an icon or a number in it", |ui| self.marker_gallery(ui));
                });
            });
        });
        if !normal {
            ui.add_space(8.0);
            ui.label(egui::RichText::new("Question and results slides are laid out by the player.").weak());
        }
    }

    /// Format Painter: once an object has been copied, the next different object that is selected gets its look.
    fn run_format_painter(&mut self, ctx: &egui::Context) {
        let Some((src, proto)) = self.painter.clone() else { return };
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.painter = None;
            self.status = "Format Painter cancelled.".into();
            return;
        }
        let Some(before) = self.selected_object().cloned() else { return };
        if before.id == src || ctx.input(|i| i.pointer.any_down()) { return; }
        let mut after = before.clone();
        after.style = proto.style.clone();
        after.fill = proto.fill;
        after.gradient = proto.gradient.clone();
        after.stroke = proto.stroke;
        after.dash = proto.dash;
        after.shadow = proto.shadow;
        after.glow = proto.glow;
        after.soft_edge = proto.soft_edge;
        after.reflection = proto.reflection;
        after.radius = proto.radius;
        after.line_spacing = proto.line_spacing;
        self.apply_object_edit(ctx, &before, &after);
        self.painter = None;
        self.status = "Format Painter: applied.".into();
    }

    /// The Symbol button of the Text group: a grid of common symbols; clicking one puts it into the selected text.
    fn symbol_button(&mut self, ui: &mut egui::Ui) {
        const GROUPS: [(&str, &str); 6] = [
            ("Greek", "αβγδεζηθικλμνξοπρστυφχψωΓΔΘΛΞΠΣΦΨΩ"),
            ("Math", "±×÷≠≈≤≥∞√∑∏∫∂∆∇∈∉⊂⊃⊆⊇∪∩∧∨¬∀∃∅≡∝∠∴∵−·°′″²³½¼¾‰"),
            ("Arrows", "←→↑↓↔⇐⇒⇔↦↵"),
            ("Currency", "€£¥¢₹₩¤"),
            ("Writing", "©®™§¶†‡•…–—«»‘’“”‹›"),
            ("Small numbers", "⁰¹²³⁴⁵⁶⁷⁸⁹⁺ⁿ₀₁₂₃₄₅₆₇₈₉₊"),
        ];
        // With nothing selected a symbol starts a new text box; only a picture-like object has no text to put it in.
        let can = self.selected_object().is_none_or(|o| !matches!(o.kind, Kind::Image | Kind::Video | Kind::Audio | Kind::Hotspot | Kind::Web(_) | Kind::Pano(_) | Kind::Input(_) | Kind::Slider(_) | Kind::Unsupported(_)));
        let resp = ui.add_enabled_ui(can, |ui| big_widget(ui, SIGMA, "Symbol", true, false).on_hover_text("Symbol: put a symbol into the selected text, or into a new text box")).inner
            .on_disabled_hover_text("Symbol: this object has no text; select a text box or shape, or click an empty part of the slide");
        let mut pick: Option<String> = None;
        egui::Popup::from_toggle_button_response(&resp).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
            ui.set_width(8.0 * 36.0 + 12.0);
            for (name, chars) in GROUPS {
                ui.label(egui::RichText::new(name).weak().size(12.0));
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
                    for c in chars.chars() {
                        let b = ui.add(egui::Button::new(egui::RichText::new(c.to_string()).family(font_family("Open Sans")).size(18.0)).min_size(egui::vec2(32.0, 30.0)).frame(false));
                        let code = format!("U+{:04X}", c as u32);
                        b.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &code));
                        if b.on_hover_text(code).clicked() { pick = Some(c.to_string()); }
                    }
                });
            }
        });
        if let Some(s) = pick { self.insert_symbol(&s); }
    }

    /// The Caption menu: the eight bubbles two to a row, each drawn with "Abc" in it.
    fn caption_menu(&mut self, ui: &mut egui::Ui) {
        use brainbend_doc::shapes::{caption, CAPTIONS, Preset};
        ui.set_min_width(0.0);
        let accent = self.project.as_ref().map(|p| p.theme.color(4)).unwrap_or(Rgba(3, 83, 158, 255));
        let mut pick = None;
        egui::Grid::new("caption-menu").spacing([8.0, 8.0]).show(ui, |ui| {
            for i in 0..CAPTIONS.len() {
                let (rect, resp) = ui.allocate_exact_size(egui::vec2(132.0, 84.0), egui::Sense::click());
                if resp.hovered() { ui.painter().rect_filled(rect, 5.0, egui::Color32::from_rgb(210, 226, 245)); }
                let r = rect.shrink2(egui::vec2(10.0, 6.0));
                let (preset, _) = caption(i, r.width(), r.height());
                let edge = egui::Stroke::new(1.2, color(Rgba(accent.0 / 2, accent.1 / 2, accent.2 / 2, 255)));
                match preset {
                    Preset::Path(cmds) => {
                        let figs = path_polygons(&cmds, r);
                        fill_figures(ui.painter(), &figs, &|p| p, &|_| color(accent));
                        for f in figs { ui.painter().add(egui::Shape::closed_line(f, edge)); }
                    }
                    Preset::Round(k) => { ui.painter().rect(r, k * r.height().min(r.width()), color(accent), edge, egui::StrokeKind::Inside); }
                    Preset::Ellipse => { ui.painter().add(egui::epaint::EllipseShape { center: r.center(), radius: r.size() / 2.0, fill: color(accent), stroke: edge, angle: 0.0 }); }
                    _ => { ui.painter().rect(r, 0.0, color(accent), edge, egui::StrokeKind::Inside); }
                }
                let body = if CAPTIONS[i].1 { 0.74 } else { 1.0 };
                ui.painter().text(egui::pos2(r.center().x, r.top() + r.height() * body / 2.0), egui::Align2::CENTER_CENTER, "Abc", egui::FontId::proportional(14.0), egui::Color32::WHITE);
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, CAPTIONS[i].0));
                if resp.on_hover_text(CAPTIONS[i].0).clicked() { pick = Some(i); }
                if i % 2 == 1 { ui.end_row(); }
            }
        });
        if let Some(i) = pick { self.insert_caption(i); ui.close(); }
    }

    /// The Input menu: check boxes and radio buttons in four looks each, and the two data-entry fields, drawn small.
    fn input_menu(&mut self, ui: &mut egui::Ui) {
        ui.set_min_width(0.0);
        let (mut check, mut radio, mut entry): (Option<usize>, Option<usize>, Option<bool>) = (None, None, None);
        let cell = egui::vec2(66.0, 56.0);
        let hover = |ui: &egui::Ui, rect: egui::Rect, resp: &egui::Response| { if resp.hovered() { ui.painter().rect_filled(rect, 5.0, egui::Color32::from_rgb(210, 226, 245)); } };
        ui.label(egui::RichText::new("Check Boxes").weak().size(12.0));
        ui.horizontal(|ui| {
            for n in 0..4 {
                let (rect, resp) = ui.allocate_exact_size(cell, egui::Sense::click());
                hover(ui, rect, &resp);
                let ck = Check::look(n, None);
                let side = (ck.size * 0.55).clamp(18.0, 40.0);
                let b = egui::Rect::from_center_size(rect.center(), egui::vec2(side, side));
                ui.painter().rect(b, 3.0, color(ck.box_fill), egui::Stroke::new(1.5, color(ck.box_line.unwrap_or(ck.tick))), egui::StrokeKind::Inside);
                let at = |x: f32, y: f32| b.min + egui::vec2(x, y) * side / 20.0;
                ui.painter().add(egui::Shape::line(vec![at(4.5, 10.5), at(8.0, 14.0), at(15.5, 6.0)], egui::Stroke::new((2.2 * side / 20.0).max(1.5), color(ck.tick))));
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &format!("Check box, {}", Check::LOOKS[n])));
                if resp.on_hover_text(format!("Check box, {}", Check::LOOKS[n])).clicked() { check = Some(n); }
            }
        });
        ui.label(egui::RichText::new("Radio Buttons").weak().size(12.0));
        ui.horizontal(|ui| {
            for n in 0..4 {
                let (rect, resp) = ui.allocate_exact_size(cell, egui::Sense::click());
                hover(ui, rect, &resp);
                let ck = Check::look(n, Some(String::new()));
                let side = (ck.size * 0.55).clamp(18.0, 40.0);
                ui.painter().circle(rect.center(), side / 2.0, color(ck.box_fill), egui::Stroke::new(1.5, color(ck.box_line.unwrap_or(ck.tick))));
                ui.painter().circle_filled(rect.center(), side / 4.0, color(ck.tick));
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &format!("Radio button, {}", Check::LOOKS[n])));
                if resp.on_hover_text(format!("Radio button, {}", Check::LOOKS[n])).clicked() { radio = Some(n); }
            }
        });
        ui.label(egui::RichText::new("Data Entry").weak().size(12.0));
        ui.horizontal(|ui| {
            for (numeric, tip) in [(false, "Text entry"), (true, "Number entry")] {
                let (rect, resp) = ui.allocate_exact_size(cell, egui::Sense::click());
                hover(ui, rect, &resp);
                let b = egui::Rect::from_center_size(rect.center(), egui::vec2(52.0, 28.0));
                ui.painter().rect(b, 2.0, egui::Color32::WHITE, egui::Stroke::new(1.5, egui::Color32::from_gray(120)), egui::StrokeKind::Inside);
                ui.painter().text(b.left_center() + egui::vec2(5.0, 0.0), egui::Align2::LEFT_CENTER, if numeric { "1" } else { "ab" }, egui::FontId::proportional(16.0), egui::Color32::from_gray(50));
                if numeric {
                    let x = b.right() - 9.0;
                    ui.painter().line_segment([egui::pos2(b.right() - 18.0, b.top()), egui::pos2(b.right() - 18.0, b.bottom())], egui::Stroke::new(1.0, egui::Color32::from_gray(150)));
                    ui.painter().add(egui::Shape::convex_polygon(vec![egui::pos2(x - 4.0, b.center().y - 3.0), egui::pos2(x + 4.0, b.center().y - 3.0), egui::pos2(x, b.center().y - 8.0)], egui::Color32::from_gray(90), egui::Stroke::NONE));
                    ui.painter().add(egui::Shape::convex_polygon(vec![egui::pos2(x - 4.0, b.center().y + 3.0), egui::pos2(x + 4.0, b.center().y + 3.0), egui::pos2(x, b.center().y + 8.0)], egui::Color32::from_gray(90), egui::Stroke::NONE));
                } else {
                    ui.painter().line_segment([egui::pos2(b.left() + 24.0, b.top() + 5.0), egui::pos2(b.left() + 24.0, b.bottom() - 5.0)], egui::Stroke::new(1.5, egui::Color32::from_gray(50)));
                }
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tip));
                if resp.on_hover_text(tip).clicked() { entry = Some(numeric); }
            }
        });
        if let Some(n) = check { self.insert_check_look(n); ui.close(); }
        if let Some(n) = radio { self.insert_radio_look(n); ui.close(); }
        if let Some(numeric) = entry { self.insert_input(numeric); ui.close(); }
    }

    /// The Dial menu: the three looks drawn small, and Convert to Dial for the selected object.
    fn dial_menu(&mut self, ui: &mut egui::Ui) {
        ui.set_min_width(0.0);
        let accent = self.project.as_ref().and_then(|p| p.theme.colors.get(4).copied()).unwrap_or(Rgba(3, 83, 158, 255));
        let mut pick = None;
        ui.horizontal(|ui| {
            for (style, tip) in [(DialStyle::Teardrop, "Knob with a pointer"), (DialStyle::Dot, "Knob with a dot"), (DialStyle::Needle, "Needle")] {
                let (rect, resp) = ui.allocate_exact_size(egui::vec2(84.0, 84.0), egui::Sense::click());
                if resp.hovered() { ui.painter().rect_filled(rect, 4.0, egui::Color32::from_rgb(210, 226, 245)); }
                let face = rect.shrink(6.0);
                if style == DialStyle::Needle {
                    ui.painter().circle(face.center(), 9.0, egui::Color32::from_gray(205), egui::Stroke::new(1.5, egui::Color32::from_gray(150)));
                } else {
                    ui.painter().circle(face.center(), face.width() / 2.0, egui::Color32::from_gray(222), egui::Stroke::new(2.0, egui::Color32::from_gray(165)));
                }
                // The pointer, turned a little so it reads as a dial.
                let turn = 0.6_f32;
                let figs = path_polygons(&dial_pointer(style), face);
                let c = face.center();
                let rot = |p: egui::Pos2| { let d = p - c; c + egui::vec2(d.x * turn.cos() - d.y * turn.sin(), d.x * turn.sin() + d.y * turn.cos()) };
                fill_figures(ui.painter(), &figs, &rot, &|_| color(accent));
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tip));
                if resp.on_hover_text(tip).clicked() { pick = Some(style); }
            }
        });
        ui.separator();
        let can = self.selected_object().is_some_and(|o| o.dial.is_none());
        ui.add_enabled_ui(can, |ui| {
            if menu_item(ui, GAUGE, "Convert to Dial").on_hover_text("Make the selected object turn when the learner drags it").on_disabled_hover_text("Select an object first").clicked() { self.convert_to_dial(); ui.close(); }
        });
        if let Some(style) = pick { self.insert_dial(style); ui.close(); }
    }

    /// The Icons gallery: a search box and a scrolling grid of Phosphor icons; clicking one inserts it.
    fn icon_gallery(&mut self, ui: &mut egui::Ui) {
        const COLS: usize = 8;
        const CELL: f32 = 38.0;
        ui.set_width(COLS as f32 * (CELL + 4.0) + 8.0);
        let id = egui::Id::new("icon-search");
        let mut query: String = ui.data_mut(|d| d.get_temp(id)).unwrap_or_default();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(SEARCH.to_string()).family(icons()));
            ui.add(egui::TextEdit::singleline(&mut query).hint_text("Search icons: arrow, home, check…").desired_width(f32::INFINITY));
        });
        ui.data_mut(|d| d.insert_temp(id, query.clone()));
        let found = crate::icons::search(&query);
        ui.weak(format!("{} icons", found.len()));
        let rows = found.len().div_ceil(COLS);
        let mut pick: Option<&'static crate::icons::Icon> = None;
        egui::ScrollArea::vertical().max_height(300.0).auto_shrink([false, true]).show_rows(ui, CELL + 4.0, rows, |ui, range| {
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
            for r in range {
                ui.horizontal(|ui| {
                    for icon in found.iter().skip(r * COLS).take(COLS) {
                        let (rect, resp) = ui.allocate_exact_size(egui::vec2(CELL, CELL), egui::Sense::click());
                        if resp.hovered() { ui.painter().rect_filled(rect, 4.0, ui.visuals().text_color().gamma_multiply(0.12)); }
                        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, icon.ch.to_string(), egui::FontId::new(24.0, icons()), text_col(ui));
                        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, icon.names[0]));
                        if resp.on_hover_text(icon.names.join(", ").replace('-', " ")).clicked() { pick = Some(icon); }
                    }
                });
            }
        });
        if let Some(icon) = pick { self.insert_icon(icon); ui.close(); }
    }

    /// The Publish choices, shared by the title bar and the Home tab.
    /// Whether Try It applies: slide-based projects only.
    fn can_try(&self) -> bool { self.project.as_ref().is_some_and(|p| !p.is_web()) }

    /// How wide the pinned group is, so the ribbon can leave room for it at the right.
    fn pinned_width(&self, ui: &egui::Ui) -> f32 {
        let mut widths = vec![big_width(ui, "Preview", true), big_width(ui, "Publish", false) + OUTLINE_PAD];
        if self.can_try() { widths.push(big_width(ui, "Try It", false)); }
        widths.iter().sum::<f32>() + 3.0 * (widths.len() as f32 - 1.0) + 14.0
    }

    /// Try It, Preview and Publish: pinned at the right end of the ribbon, always in view.
    fn pinned_actions(&mut self, ui: &mut egui::Ui) {
        // A thin divider separates the group from the tabs' controls.
        let (line, _) = ui.allocate_exact_size(egui::vec2(1.0, BIG_H + CAP_H), egui::Sense::hover());
        let col = ui.visuals().text_color().gamma_multiply(if ui.visuals().dark_mode { 0.30 } else { 0.25 });
        ui.painter().line_segment([line.center_top() + egui::vec2(0.0, 6.0), line.center_bottom() - egui::vec2(0.0, 6.0)], egui::Stroke::new(1.0, col));
        ui.add_space(5.0);
        let can_try = self.can_try();
        pinned_group(ui, "", |ui| {
            ui.horizontal(|ui| {
                if can_try {
                    let tip = "Press buttons on the slide to see their Hover, Down, Visited and Selected looks and the layers they show. Nothing is changed or saved. Esc stops. GIF animation, video, audio, JavaScript and variables do not run here: use Preview for those.";
                    if big_toggle(ui, CLICK, "Try It", tip, self.try_mode).clicked() { self.try_mode = !self.try_mode; }
                }
                big_menu(ui, PLAY, "Preview", "Play the lesson in your browser", |ui| self.preview_items(ui));
                // Publish is the main action, so it is outlined in the main-action color; the outline darkens and the
                // inside tints while the pointer is over it, and more while pressed or while the Publish window is open.
                let backing = ui.painter().add(egui::Shape::Noop);
                let publishing = self.publish_open;
                let resp = ui.scope(|ui| { outlined_look(ui); big_widget_min(ui, EXPORT, "Publish", false, false, big_width(ui, "Publish", false) + OUTLINE_PAD).on_hover_text("Publish the project: to the web, an LMS, PDF and more") }).inner;
                if resp.clicked() { self.publish_open = true; }
                ui.painter().set(backing, outline_shape(ui, resp.rect, resp.hovered(), publishing || resp.is_pointer_button_down_on()));
            });
        });
    }


    /// The Preview choices, shared by the title bar and the Home tab.
    pub(super) fn preview_items(&mut self, ui: &mut egui::Ui) {
        ui.set_min_width(200.0);
        if menu_item(ui, PLAY, "Entire lesson").clicked() { self.preview(PreviewScope::All); }
        if self.project.as_ref().is_some_and(|p| p.is_web()) { return; }
        if menu_item(ui, PLAY, "This scene").clicked() { self.preview(PreviewScope::Scene); }
        if menu_item(ui, PLAY, "This slide").clicked() { self.preview(PreviewScope::Slide); }
        if menu_item(ui, PLAY, "From this slide").clicked() { self.preview(PreviewScope::FromHere); }
        let n = self.slide_selection().len();
        ui.add_enabled_ui(n > 1, |ui| {
            if menu_item(ui, PLAY, &if n > 1 { format!("Selected slides ({n})") } else { "Selected slides".to_string() }).on_disabled_hover_text("Select several slides with ⌘-click or ⇧-click in the slide list").clicked() { self.preview(PreviewScope::Selected); }
        });
    }

    /// Link the selected text (or all of the selected object's text) to a web address; `None` removes the link.
    fn link_selected(&mut self, url: Option<String>) {
        let Some((li, oi)) = self.selected else { return };
        let (a, b) = self.text_sel.unwrap_or((0, usize::MAX));
        self.snapshot();
        if let Some(o) = self.project.as_mut().and_then(|p| p.slides[self.slide].layers.get_mut(li)).and_then(|l| l.objects.get_mut(oi)) {
            let n = o.text.chars().count();
            o.format_range(a.min(n), b.min(n), &Attr::Link(url));
            let (t, r) = (o.text.clone(), o.rich.clone());
            for st in o.states.iter_mut() { st.text = t.clone(); st.rich = r.clone(); }
        }
    }

    /// The New Slide choices, shared by Home and Insert.
    fn new_slide_menu(&mut self, ui: &mut egui::Ui) {
        if menu_item(ui, FILE_PLUS, "Same layout").clicked() { self.add_slide(SlideKind::Normal); ui.close(); }
        ui.label(egui::RichText::new("Layouts").weak().size(12.0));
        let layouts: Vec<(String, String)> = self.project.as_ref().map(|p| p.layouts.iter().map(|l| (l.id.clone(), l.name.clone())).collect()).unwrap_or_default();
        for (id, name) in layouts {
            if menu_item(ui, FRAME, &name).clicked() { self.add_slide_with_layout(SlideKind::Normal, Some(id)); ui.close(); }
        }
        ui.separator();
        if menu_item(ui, FILE_PLUS, "No layout").clicked() { self.add_slide_with_layout(SlideKind::Normal, None); ui.close(); }
        if menu_item(ui, CHECK, "Results slide").clicked() { self.add_slide(SlideKind::Results); }
    }

    fn slides_tab(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        group(ui, "Project", |ui| {
            ui.horizontal(|ui| {
                if big_button(ui, FILE_PLUS, "New Scene", "Add a scene after the current one").clicked() { self.add_scene(); }
                big_menu(ui, FILE_PLUS, "New Slide", "Add a slide after this one", |ui| self.new_slide_menu(ui));
            });
        });
        fn multiple_response() -> Question {
            let mut q = Question::new_single();
            q.multiple = true;
            q.choices[1].correct = true;
            q
        }
        // Every question type is a button of its own, laid out three to a column so they fit the ribbon.
        let graded: [(char, &str, &str, fn() -> Question); 15] = [
            (RADIO, "Multiple choice", "One correct answer from several", Question::new_single),
            (CHECK, "Multiple response", "Several correct answers", multiple_response),
            (SLIDERS, "True / False", "True or false", Question::new_true_false),
            (TEXT_T, "Fill in the blank", "Type the answer", Question::new_text_entry),
            (SHAPES, "Matching", "Match each item to its partner", Question::new_matching),
            (LIST_NUM, "Sequencing", "Put the items in order", Question::new_sequence),
            (TEXTBOX, "Fill in the blanks", "Type the missing words in a passage", Question::new_fill_blanks),
            (CLICK, "Drag the words", "Drag words into the gaps of a passage", Question::new_drag_words),
            (PEN, "Mark the words", "Mark the right words in a passage", Question::new_mark_words),
            (TEXT_AA, "Number", "Type a number", Question::new_numeric),
            (STACK, "Sort into groups", "Drag each item into its group", Question::new_categorize),
            (IMAGE, "Find the hotspot", "Click the right place on a picture", Question::new_hotspot),
            (FRAME, "Label the picture", "Put each label on its place in a picture", Question::new_label_picture),
            (RADIO, "Choose from lists", "Pick the right word from drop-down lists in a passage", Question::new_dropdowns),
            (SPEAKER, "Dictation", "Type what is heard", Question::new_dictation),
        ];
        let survey: [(char, &str, &str, fn() -> Question); 2] = [
            (SLIDERS, "Rating scale", "A survey question answered on a scale; not scored", Question::new_likert),
            (TEXTBOX, "Free text", "A survey question answered in the learner's own words; not scored", Question::new_reflection),
        ];
        fn grid(ui: &mut egui::Ui, items: &[(char, &str, &str, fn() -> Question)]) -> Option<fn() -> Question> {
            let mut picked = None;
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 0.0);
                for column in items.chunks(3) {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        ui.spacing_mut().button_padding.y = 1.0;
                        ui.spacing_mut().interact_size.y = 20.0;
                        let w = column.iter().map(|c| c.1.len() as f32 * 7.0 + 36.0).fold(0.0, f32::max);
                        for (icon, label, tip, make) in column {
                            if ui.add(egui::Button::new(caret_job_plain(ui, *icon, label)).frame(false).min_size(egui::vec2(w, 20.0))).on_hover_text(*tip).clicked() { picked = Some(*make); }
                        }
                    });
                }
            });
            picked
        }
        group(ui, "Graded Questions", |ui| {
            if let Some(make) = grid(ui, &graded) { self.add_slide(SlideKind::Question(make())); }
        });
        group(ui, "Survey Questions", |ui| {
            if let Some(make) = grid(ui, &survey) { self.add_slide(SlideKind::Question(make())); }
        });
        group(ui, "Results", |ui| {
            if big_button(ui, CHECK, "Results", "Add a quiz results slide").clicked() { self.add_slide(SlideKind::Results); }
        });
        group(ui, "Course", |ui| {
            ui.horizontal(|ui| {
                if big_button(ui, SLIDERS, "Variables", "The project's variables, which triggers read and change").clicked() { self.variables_open = true; }
                ui.vertical(|ui| {
                    rows(ui);
                    ui.label("Pass mark");
                    if let Some(proj) = self.project.as_mut() {
                        let before = Snap::of(proj);
                        let mut changed = false;
                        framed(ui, |ui| { changed = ui.add(egui::DragValue::new(&mut proj.pass_mark).range(0..=100).suffix(" %")).changed(); });
                        if changed { self.commit_project_edit(_ctx, before); }
                    }
                });
            });
        });
    }

    fn master_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let cur = self.slide;
        let n = self.project.as_ref().map(|p| p.slides.len()).unwrap_or(0);
        let is_layout = cur > 0 && cur < n;
        group(ui, "Edit", |ui| {
            ui.horizontal(|ui| {
                if big_button(ui, FILE_PLUS, "Insert Layout", "Add a new layout").clicked() { self.insert_layout(None); }
                ui.add_enabled_ui(is_layout, |ui| {
                    if big_button(ui, COPY, "Duplicate", "Copy this layout").clicked() { self.insert_layout(Some(cur)); }
                    if big_button(ui, TRASH, "Delete", "Delete this layout").clicked() { self.delete_layout(cur); }
                });
            });
        });
        // Delete and Insert change the slide list and the current slide: look again before using the index.
        let n = self.project.as_ref().map(|p| p.slides.len()).unwrap_or(0);
        let cur = self.slide.min(n.saturating_sub(1));
        let is_layout = cur > 0 && cur < n;
        group(ui, "Placeholders", |ui| {
            big_menu(ui, TEXTBOX, "Placeholder", "Add a heading or text placeholder", |ui| {
                for k in PlaceholderKind::CHOICES {
                    if menu_item(ui, TEXT_T, k.label()).clicked() { self.insert_placeholder(k); ui.close(); }
                }
            });
        });
        group(ui, "Layout", |ui| {
            rows(ui);
            let Some(proj) = self.project.as_mut() else { return };
            let before = Snap::of(proj);
            let mut changed = false;
            if is_layout {
                let id = proj.slides[cur].layout.clone().unwrap_or_default();
                if let Some(l) = proj.layouts.iter_mut().find(|l| l.id == id) {
                    changed |= ui.checkbox(&mut l.show_master, "Show master shapes").changed();
                }
            } else {
                ui.label(egui::RichText::new("Shapes and background here appear on every layout").weak().size(12.0));
            }
            if changed { self.commit_project_edit(ctx, before); }
        });
        group(ui, "Background", |ui| {
            if big_button(ui, BUCKET, "Background", "Background of this layout or the master").clicked() { self.background_open = !self.background_open; }
        });
    }

    /// What an author checks once the project is built: the player's settings, the accessibility report, spelling and focus order.
    fn review_tab(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        if self.project.is_none() { return; }
        group(ui, "Player", |ui| {
            if big_button(ui, PLAY, "Player\nSettings", "Colors, font, sidebar, title, logo and controls of the published player").clicked() { self.player_open = true; }
        });
                group(ui, "Check", |ui| {
            ui.horizontal(|ui| {
                if big_button(ui, ACCESS, "Accessibility\nChecker", "Accessibility and consistency report").clicked() { self.check_results = self.project.as_ref().map(|p| p.check()); }
                if big_button(ui, SPELL, "Spelling", "Check the spelling of the whole project (F7)").clicked() { self.open_spelling(); }
                if big_button(ui, LIST_NUM, "Focus Order", "The order the Tab key and screen readers move through this slide's objects").clicked() { self.focus_open = !self.focus_open; }
            });
        });
    }

    fn design_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let slide_idx = self.slide;
        let Some(proj) = self.project.as_mut() else { return };
        let before = Snap::of(proj);
        let mut changed = false;
        let mut open_master = false;
        let mut layout_pick: Option<Option<String>> = None;
        let mut open_theme_colors = false;
        let mut toggle_background = false;
        group(ui, "Master", |ui| {
            ui.horizontal(|ui| {
                if big_button(ui, STACK, "Layout Builder", "Build your own slide layouts").clicked() { open_master = true; }
                big_menu(ui, FRAME, "Layout", "Change this slide's layout", |ui| {
                    let current = proj.slides[slide_idx].layout.clone();
                    for l in proj.layouts.clone() {
                        if ui.selectable_label(current.as_deref() == Some(l.id.as_str()), l.name.as_str()).clicked() { layout_pick = Some(Some(l.id.clone())); ui.close(); }
                    }
                    ui.separator();
                    if ui.selectable_label(current.is_none(), "No layout").clicked() { layout_pick = Some(None); ui.close(); }
                });
                let cur = (proj.slides[0].width as i32, proj.slides[0].height as i32);
                big_menu(ui, FRAME, "Slide Size", &format!("Now {} x {}", cur.0, cur.1), |ui| {
                    for (w, h, label) in [(1920, 1080, "1920 x 1080 (16:9)"), (1280, 720, "1280 x 720 (16:9)"), (1440, 1080, "1440 x 1080 (4:3)"), (1024, 768, "1024 x 768 (4:3)")] {
                        if ui.selectable_label(cur == (w, h), label).clicked() {
                            for sl in proj.slides.iter_mut() { sl.width = w as f32; sl.height = h as f32; }
                            changed = true;
                        }
                    }
                });
            });
        });
        group(ui, "Brand", |ui| {
            ui.horizontal(|ui| {
                if big_button(ui, BUCKET, "Background", "Color, gradient or picture behind this slide").clicked() { toggle_background = true; }
                if big_button(ui, DROP, "Colors", "Your project's colors, offered first in every color menu").clicked() { open_theme_colors = true; }
                // The theme's fonts, stacked: headings on top, body text below.
                ui.vertical(|ui| {
                    let mut next = proj.theme.clone();
                    let mut moved = false;
                    ui.spacing_mut().item_spacing.y = 4.0;
                    for (label, tip, id) in [("Headings", "The font of headings, in the master, layouts and slides that follow them", "theme-heading-font"), ("Body", "The font of body text, in the master, layouts and slides that follow them", "theme-body-font")] {
                        ui.horizontal(|ui| {
                            ui.add_sized([54.0, 22.0], egui::Label::new(egui::RichText::new(label).weak().size(12.0))).on_hover_text(tip);
                            let font = if id == "theme-heading-font" { &mut next.heading_font } else { &mut next.body_font };
                            moved |= crate::font_store::font_menu(ui, id, font, 130.0);
                        });
                    }
                    if moved { proj.set_theme(next.named()); changed = true; }
                });
            });
        });
        if changed { self.commit_project_edit(ctx, before); }
        if open_master { self.enter_master_view(); }
        if toggle_background { self.background_open = !self.background_open; }
        if open_theme_colors { self.theme_colors_open = true; }
        if let Some(l) = layout_pick { self.set_slide_layout(slide_idx, l); }
    }

    /// The slide's transition: a gallery to which more can be added (each is a name, an icon and what the player does).
    fn transition_groups(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let slide_idx = self.slide;
        let Some(proj) = self.project.as_mut() else { return };
        let before = Snap::of(proj);
        let mut changed = false;
        let mut apply_all = false;
        group(ui, "Slide Transition", |ui| {
            use brainbend_doc::{Transition, TransitionKind};
            ui.horizontal(|ui| {
                let fade = proj.slides[slide_idx].fade_ms;
                let cur = proj.slides[slide_idx].transition;
                if big_toggle(ui, SQUARE_BORDER, "None", "No transition: the next slide just appears", fade == 0).clicked() { proj.slides[slide_idx].fade_ms = 0; changed = true; }
                for k in TransitionKind::ALL {
                    let tip = match k {
                        TransitionKind::Fade => "The slide fades in",
                        TransitionKind::Push => "The new slide pushes the old one out",
                        TransitionKind::Wipe => "The new slide is wiped in",
                        TransitionKind::Split => "The new slide opens from the middle, or closes in from the sides",
                        TransitionKind::Cover => "The new slide slides over the old one",
                        TransitionKind::Uncover => "The old slide slides away to show the new one",
                        TransitionKind::Zoom => "The new slide zooms in, or the old one zooms away",
                        TransitionKind::Dissolve => "The new slide appears in scattered pieces",
                    };
                    if big_drawn_button(ui, k.label(), tip, fade > 0 && cur.kind == k, |p, f, new, old| draw_transition(k, p, f, new, old)).clicked() {
                        proj.slides[slide_idx].transition = Transition::of(k);
                        if fade == 0 { proj.slides[slide_idx].fade_ms = 500; }
                        changed = true;
                    }
                }
                // The choices a kind has: where it comes from, or whether it goes in or out.
                let variants = Transition::variants(cur.kind);
                ui.add_enabled_ui(fade > 0 && variants.len() > 1, |ui| {
                    big_menu(ui, SLIDERS, "Effect\nOptions", "Direction, or in and out, for this transition", |ui| {
                        for v in variants {
                            if ui.selectable_label(v.variant_label() == cur.variant_label(), v.variant_label()).clicked() { proj.slides[slide_idx].transition = v; changed = true; }
                        }
                    });
                });
            });
        });
        group(ui, "Transition Timing", |ui| {
            rows(ui);
            ui.horizontal(|ui| {
                ui.label(icon_text(TIMER, 17.0));
                ui.label("Duration");
                if proj.slides[slide_idx].fade_ms > 0 {
                    framed(ui, |ui| { changed |= ui.add(egui::DragValue::new(&mut proj.slides[slide_idx].fade_ms).range(50..=5000).speed(10.0).suffix(" ms")).changed(); });
                } else {
                    ui.weak("none");
                }
            });
            if ui.button("Apply to all slides").clicked() { apply_all = true; }
        });
        if apply_all {
            let (ms, tr) = (proj.slides[slide_idx].fade_ms, proj.slides[slide_idx].transition);
            for sl in proj.slides.iter_mut() { sl.fade_ms = ms; sl.transition = tr; }
            changed = true;
        }
        if changed { self.commit_project_edit(ctx, before); }
    }

    fn animations_tab(&mut self, ui: &mut egui::Ui, obj: Option<&mut Object>, changed: &mut bool) {
        // Shown even with nothing selected, but disabled.
        let mut stand_in = Object::new("none", "none", Kind::Rect, 0.0, 0.0, 0.0, 0.0);
        let enabled = obj.is_some();
        let obj = obj.unwrap_or(&mut stand_in);
        ui.add_enabled_ui(enabled, |ui| {
        group(ui, "Entrance and Exit", |ui| {
            ui.horizontal(|ui| {
                *changed |= effect_menu(ui, ARROW_DOWN, "Entrance", "How the object appears", &mut obj.entrance, &mut obj.click_steps.entrance, true);
                let had_exit = obj.exit.is_some();
                let exit_changed = effect_menu(ui, ARROW_UP, "Exit", "How the object leaves", &mut obj.exit, &mut obj.click_steps.exit, false);
                // An exit plays when the object leaves, so an object that stays to the end of the timeline is given an end to leave at.
                if exit_changed && !had_exit && obj.exit.is_some() && obj.click_steps.exit == 0 && obj.timing.map_or(true, |t| t.dur_ms.is_none()) {
                    let start = obj.timing.map_or(0, |t| t.start_ms);
                    obj.timing = Some(Timing { start_ms: start, dur_ms: Some(2000) });
                }
                *changed |= exit_changed;
            });
        });
        group(ui, "Emphasis", |ui| {
            ui.horizontal(|ui| {
                let on = obj.emphasis.is_some();
                let tip = obj.emphasis.map(|e| format!("Emphasis: {}", e.kind.label())).unwrap_or_else(|| "Draw attention to the object while it is showing".into());
                let btn = big_widget(ui, STAR, "Emphasis", true, on).on_hover_text(tip);
                egui::Popup::from_toggle_button_response(&btn).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
                    ui.set_min_width(210.0);
                    if ui.selectable_label(obj.emphasis.is_none(), "none").clicked() { obj.emphasis = None; *changed = true; }
                    for k in EmphasisKind::ALL {
                        let r = ui.selectable_label(obj.emphasis.map(|e| e.kind) == Some(k), k.label());
                        if r.clicked() {
                            let old = obj.emphasis;
                            obj.emphasis = Some(Emphasis { kind: k, delay_ms: old.map(|e| e.delay_ms).unwrap_or(0), duration_ms: old.map(|e| e.duration_ms).unwrap_or(800), repeat: old.map(|e| e.repeat).unwrap_or(1) });
                            *changed = true;
                        }
                    }
                    if let Some(e) = obj.emphasis.as_mut() {
                        ui.separator();
                        *changed |= click_row(ui, "ck-emphasis", "Starts", &mut obj.click_steps.emphasis);
                        ui.horizontal(|ui| {
                            ui.label("lasts");
                            *changed |= ui.add(egui::DragValue::new(&mut e.duration_ms).range(100..=60_000).speed(50.0).suffix(" ms")).changed();
                            ui.label("plays");
                            *changed |= ui.add(egui::DragValue::new(&mut e.repeat).range(0..=99).suffix("×")).on_hover_text("Times it plays; 0 = until the slide ends").changed();
                        });
                        *changed |= delay_leftover(ui, &mut e.delay_ms);
                    }
                });
            });
        });
        group(ui, "Motion path", |ui| {
            ui.horizontal(|ui| {
                let on = obj.motion.is_some();
                let btn = big_widget(ui, POLYLINE, "Motion", true, on).on_hover_text("Move the object along a path. Drag the points on the slide to shape it");
                egui::Popup::from_toggle_button_response(&btn).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
                    ui.set_min_width(210.0);
                    if ui.selectable_label(obj.motion.is_none(), "none").clicked() { obj.motion = None; *changed = true; }
                    for k in PathPreset::ALL {
                        if ui.selectable_label(false, k.label()).clicked() {
                            let (delay, dur) = obj.motion.as_ref().map(|m| (m.delay_ms, m.duration_ms)).unwrap_or((0, 2000));
                            let mut m = MotionPath::preset(k);
                            m.delay_ms = delay;
                            m.duration_ms = dur;
                            obj.motion = Some(m);
                            *changed = true;
                        }
                    }
                    if let Some(m) = obj.motion.as_mut() {
                        ui.separator();
                        *changed |= click_row(ui, "ck-motion", "Starts", &mut obj.click_steps.motion);
                        ui.horizontal(|ui| {
                            ui.label("takes");
                            *changed |= ui.add(egui::DragValue::new(&mut m.duration_ms).range(100..=120_000).speed(50.0).suffix(" ms")).changed();
                            *changed |= ui.checkbox(&mut m.smooth, "Smooth").on_hover_text("Smooth curve through the points").changed();
                        });
                        ui.horizontal(|ui| {
                            if ui.small_button("+ Point").on_hover_text("Add a point after the last one").clicked() {
                                let (lx, ly) = m.points.last().copied().unwrap_or((0.0, 0.0));
                                m.points.push((lx + 120.0, ly));
                                *changed = true;
                            }
                            if m.points.len() > 2 && ui.small_button("- Point").clicked() { m.points.pop(); *changed = true; }
                        });
                        *changed |= delay_leftover(ui, &mut m.delay_ms);
                    }
                });
            });
        });
        group(ui, "Visibility", |ui| {
            rows(ui);
            ui.horizontal(|ui| {
                ui.label(icon_text(EYE_OFF, 17.0));
                *changed |= ui.checkbox(&mut obj.hidden, "Hidden when the slide starts").changed();
            });
        });
        });
    }

    /// Format the characters selected in the in-place editor (set by the canvas while editing).
    pub(super) fn format_selection(&mut self, attr: Attr) {
        let (Some((a, b)), Some((li, oi))) = (self.text_sel, self.selected) else { return };
        self.snapshot();
        if let Some(o) = self.project.as_mut().and_then(|p| p.slides[self.slide].layers.get_mut(li)).and_then(|l| l.objects.get_mut(oi)) {
            o.format_range(a, b, &attr);
            let (t, r) = (o.text.clone(), o.rich.clone());
            for st in o.states.iter_mut() { st.text = t.clone(); st.rich = r.clone(); }
        }
    }

    /// Run a text edit on the selected text, or on all the text of every selected object when none is chosen.
    /// `f` gets the object, the character range and whether it is the whole text. One undo step.
    pub(super) fn edit_text_range(&mut self, f: impl Fn(&mut Object, usize, usize, bool)) {
        let targets = self.format_targets();
        if targets.is_empty() { return; }
        let (part, primary) = (self.text_sel, self.selected);
        self.snapshot();
        let slide = self.slide;
        let Some(p) = self.project.as_mut() else { return };
        for (li, oi) in targets {
            let Some(o) = p.slides[slide].layers.get_mut(li).and_then(|l| l.objects.get_mut(oi)) else { continue };
            let n = o.text.chars().count();
            match part {
                Some((a, b)) if primary == Some((li, oi)) => f(o, a.min(n), b.min(n), false),
                _ => f(o, 0, n, true),
            }
            let (t, r) = (o.text.clone(), o.rich.clone());
            for st in o.states.iter_mut() { st.text = t.clone(); st.rich = r.clone(); }
        }
        self.dirty = true;
    }

    /// With several objects selected, line them up with each other; with one, line it up with the slide.
    pub(super) fn align_selected_or_slide(&mut self, axis: u8, mode: u8) {
        if self.selection().len() > 1 {
            self.align_selected(axis, mode);
            return;
        }
        let Some((li, oi)) = self.selected else { return };
        self.snapshot();
        let slide = &mut self.project.as_mut().unwrap().slides[self.slide];
        let (sw, sh) = (slide.width, slide.height);
        if let Some(o) = slide.layers.get_mut(li).and_then(|l| l.objects.get_mut(oi)) {
            let (pos, len, span) = if axis == 0 { (&mut o.x, o.w, sw) } else { (&mut o.y, o.h, sh) };
            *pos = match mode { 0 => 0.0, 1 => ((span - len) / 2.0).round(), _ => span - len };
        }
    }
}

/// One effect in the Effects popup: a header with its on/off switch, and its controls underneath while it is on.
/// Returns true when the switch or anything under it changed.
fn effect_section(ui: &mut egui::Ui, title: &str, on: &mut bool, body: impl FnOnce(&mut egui::Ui) -> bool) -> bool {
    let mut changed = ui.checkbox(on, egui::RichText::new(title).strong()).changed();
    if *on {
        ui.indent(title, |ui| { changed |= body(ui); });
    }
    ui.add_space(4.0);
    changed
}

/// A label and a slider with its value, in one tidy row of a grid.
fn effect_slider(ui: &mut egui::Ui, label: &str, value: &mut f32, range: std::ops::RangeInclusive<f32>, suffix: &str) -> bool {
    effect_slider_if(ui, true, label, value, range, suffix)
}

/// The same, greyed out and untouchable when `enabled` is false (the row stays in the grid either way).
fn effect_slider_if(ui: &mut egui::Ui, enabled: bool, label: &str, value: &mut f32, range: std::ops::RangeInclusive<f32>, suffix: &str) -> bool {
    ui.label(label);
    let changed = ui.add_enabled(enabled, egui::Slider::new(value, range).suffix(suffix.to_string()).max_decimals(0)).changed();
    ui.end_row();
    changed
}

/// A percentage slider over a 0 to 1 value.
fn effect_percent(ui: &mut egui::Ui, label: &str, value: &mut f32, min: f32) -> bool {
    ui.label(label);
    let changed = ui.add(egui::Slider::new(value, min..=1.0).custom_formatter(|v, _| format!("{:.0}%", v * 100.0)).custom_parser(|t| t.trim().trim_end_matches('%').parse::<f64>().ok().map(|v| v / 100.0))).changed();
    ui.end_row();
    changed
}

/// A color button with its label, as one row of a grid.
fn effect_color(ui: &mut egui::Ui, label: &str, c: &mut Rgba) -> bool {
    ui.label(label);
    let mut cur = Some(*c);
    let mut changed = false;
    ui.horizontal(|ui| { if color_menu(ui, DROP, label, &mut cur, None) { if let Some(n) = cur { *c = n; changed = true; } } });
    ui.end_row();
    changed
}

/// A round pad for a shadow's direction and distance: drag the dot away from the middle. The sliders beside it do the
/// same with exact numbers (and by keyboard).
fn shadow_dial(ui: &mut egui::Ui, sh: &mut Shadow) -> bool {
    const R: f32 = 40.0;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(R * 2.0 + 8.0, R * 2.0 + 8.0), egui::Sense::click_and_drag());
    let c = rect.center();
    let v = ui.visuals();
    let (ring, ink, accent) = (v.widgets.noninteractive.fg_stroke.color.gamma_multiply(0.5), v.widgets.noninteractive.fg_stroke.color, v.selection.bg_fill);
    let p = ui.painter();
    p.circle_filled(c, R, v.extreme_bg_color);
    p.circle_stroke(c, R, egui::Stroke::new(1.0, ring));
    p.circle_stroke(c, R / 2.0, egui::Stroke::new(1.0, ring.gamma_multiply(0.5)));
    p.line_segment([c - egui::vec2(R, 0.0), c + egui::vec2(R, 0.0)], egui::Stroke::new(1.0, ring.gamma_multiply(0.4)));
    p.line_segment([c - egui::vec2(0.0, R), c + egui::vec2(0.0, R)], egui::Stroke::new(1.0, ring.gamma_multiply(0.4)));
    let mut changed = false;
    if resp.dragged() || resp.clicked() {
        if let Some(pos) = resp.interact_pointer_pos() {
            let mut d = pos - c;
            if d.length() > R { d = d.normalized() * R; }
            if d.length() < 3.0 { d = egui::Vec2::ZERO; }   // the middle means "straight behind"
            let (dx, dy) = (d.x.round(), d.y.round());
            if (dx, dy) != (sh.dx, sh.dy) { sh.dx = dx; sh.dy = dy; changed = true; }
        }
    }
    let off = egui::vec2(sh.dx, sh.dy);
    let dot = c + if off.length() > R { off.normalized() * R } else { off };
    ui.painter().line_segment([c, dot], egui::Stroke::new(2.0, accent));
    ui.painter().circle_filled(dot, 5.5, accent);
    ui.painter().circle_stroke(dot, 5.5, egui::Stroke::new(1.0, ink));
    resp.on_hover_text("Drag to set which way the shadow falls and how far");
    changed
}

/// The shadow's controls: the dial, distance and angle sliders, blur, spread and color.
fn shadow_controls(ui: &mut egui::Ui, sh: &mut Shadow) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        changed |= shadow_dial(ui, sh);
        ui.vertical(|ui| {
            let mut dist = (sh.dx * sh.dx + sh.dy * sh.dy).sqrt();
            let mut angle = if dist > 0.0 { sh.dy.atan2(sh.dx).to_degrees().rem_euclid(360.0) } else { 90.0 };
            egui::Grid::new("shadow-dir").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                let d0 = dist;
                if effect_slider(ui, "Distance", &mut dist, 0.0..=100.0, " px") {
                    let a = angle.to_radians();
                    let was_zero = d0 == 0.0;
                    let a = if was_zero { std::f32::consts::FRAC_PI_2 } else { a };
                    sh.dx = (dist * a.cos()).round(); sh.dy = (dist * a.sin()).round(); changed = true;
                }
                if effect_slider_if(ui, dist > 0.0, "Direction", &mut angle, 0.0..=360.0, "\u{b0}") {
                    let a = angle.to_radians();
                    sh.dx = (dist * a.cos()).round(); sh.dy = (dist * a.sin()).round(); changed = true;
                }
            });
        });
    });
    egui::Grid::new("shadow-look").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
        changed |= effect_slider(ui, "Blur", &mut sh.blur, 0.0..=100.0, " px");
        changed |= effect_slider(ui, "Spread", &mut sh.spread, -50.0..=100.0, " px");
        changed |= effect_color(ui, "Color", &mut sh.color);
    });
    changed
}

/// A small sample of the object's look with every effect applied, redrawn as the controls move.
fn effects_preview(ui: &mut egui::Ui, obj: &Object) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 86.0), egui::Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 4.0, egui::Color32::from_gray(238));   // light in both themes, so a dark shadow shows
    let sample = egui::Rect::from_center_size(rect.center() - egui::vec2(0.0, 8.0), egui::vec2(110.0, 34.0));
    // The sample is smaller than a slide, so its effects are drawn at half size.
    let mut sample_obj = Object::new("sample", "Sample", Kind::Rect, 0.0, 0.0, sample.width(), sample.height());
    sample_obj.fill = Some(obj.fill.unwrap_or(Rgba(3, 83, 158, 255)));
    sample_obj.radius = 8.0;
    sample_obj.shadow = obj.shadow; sample_obj.glow = obj.glow; sample_obj.reflection = obj.reflection; sample_obj.soft_edge = obj.soft_edge;
    let fill = color(sample_obj.fill.unwrap());
    super::effects_paint::behind(&p, &sample_obj, sample, 0.5, Some(fill), &|q| q);
    if obj.soft_edge > 0.0 { super::effects_paint::soft_fill(&p, &super::effects_paint::figures(&sample_obj, sample, 0.5), sample, fill, obj.soft_edge * 0.5, &|q| q); }
    else { p.rect_filled(sample, 4.0, fill); }
    p.text(sample.center(), egui::Align2::CENTER_CENTER, "Sample", egui::FontId::proportional(13.0), egui::Color32::WHITE);
}

/// The effects buttons: one popup each, so none of them is tall.
#[derive(Clone, Copy, PartialEq)]
enum EffectPane { Presets, Shadow, Glow, Reflection, Soft, Corners }

/// The contents of one Effects popup: a small preview, then that effect's controls. Returns true when anything changed.
fn effect_pane(ui: &mut egui::Ui, obj: &mut Object, pane: EffectPane) -> bool {
    let mut changed = false;
    ui.set_width(if pane == EffectPane::Shadow { 330.0 } else { 280.0 });
    ui.spacing_mut().slider_width = 96.0;
    effects_preview(ui, obj);
    ui.add_space(6.0);
    match pane {
        EffectPane::Presets => {
            for (name, tip) in [("None", "Remove every effect"), ("Soft shadow", "A gentle shadow just below"), ("Warm glow", "A warm halo"), ("Mirror", "A faded reflection below")] {
                if ui.add(egui::Button::new(name).min_size(egui::vec2(ui.available_width(), 22.0))).on_hover_text(tip).clicked() {
                    obj.shadow = None; obj.glow = None; obj.reflection = None; obj.soft_edge = 0.0;
                    match name {
                        "Soft shadow" => obj.shadow = Some(Shadow { dx: 0.0, dy: 4.0, blur: 14.0, spread: 0.0, color: Rgba(0, 0, 0, 80) }),
                        "Warm glow" => obj.glow = Some(Glow { radius: 14.0, color: Rgba(255, 190, 40, 220) }),
                        "Mirror" => obj.reflection = Some(Reflection { size: 0.5, opacity: 0.4, gap: 2.0 }),
                        _ => {}
                    }
                    changed = true;
                }
            }
        }
        EffectPane::Shadow => {
            let mut sh = obj.shadow.unwrap_or(Shadow { dx: 4.0, dy: 4.0, blur: 8.0, spread: 0.0, color: Rgba(0, 0, 0, 90) });
            let mut on = obj.shadow.is_some();
            changed |= effect_section(ui, "Use shadow", &mut on, |ui| shadow_controls(ui, &mut sh));
            obj.shadow = if on { Some(sh) } else { None };
        }
        EffectPane::Glow => {
            let mut gl = obj.glow.unwrap_or(Glow { radius: 12.0, color: Rgba(255, 190, 40, 200) });
            let mut on = obj.glow.is_some();
            changed |= effect_section(ui, "Use glow", &mut on, |ui| {
                let mut c = false;
                egui::Grid::new("glow-look").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                    c |= effect_slider(ui, "Size", &mut gl.radius, 1.0..=100.0, " px");
                    c |= effect_color(ui, "Color", &mut gl.color);
                });
                c
            });
            obj.glow = if on { Some(gl) } else { None };
        }
        EffectPane::Reflection => {
            let mut rf = obj.reflection.unwrap_or(Reflection { size: 0.5, opacity: 0.4, gap: 2.0 });
            let mut on = obj.reflection.is_some();
            changed |= effect_section(ui, "Use reflection", &mut on, |ui| {
                let mut c = false;
                egui::Grid::new("refl-look").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                    c |= effect_percent(ui, "Size", &mut rf.size, 0.1);
                    c |= effect_percent(ui, "Opacity", &mut rf.opacity, 0.05);
                    c |= effect_slider(ui, "Gap", &mut rf.gap, 0.0..=100.0, " px");
                });
                ui.weak("Not every browser shows reflections.");
                c
            });
            obj.reflection = if on { Some(rf) } else { None };
        }
        EffectPane::Soft => {
            let mut soft = obj.soft_edge;
            let was = soft > 0.0;
            let mut on = was;
            changed |= effect_section(ui, "Use soft edges", &mut on, |ui| {
                let mut c = false;
                if !was { soft = 10.0; c = true; }   // just switched on: start with a visible fade
                egui::Grid::new("soft-look").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| { c |= effect_slider(ui, "Fade", &mut soft, 1.0..=100.0, " px"); });
                ui.weak("While on, the shadow and glow are hidden.");
                c
            });
            obj.soft_edge = if on { soft.max(1.0) } else { 0.0 };
        }
        EffectPane::Corners => {
            egui::Grid::new("corner-look").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                changed |= effect_slider(ui, "Radius", &mut obj.radius, 0.0..=300.0, " px");
            });
        }
    }
    changed
}

/// The Effects group of the ribbon: a button for each effect, lit while the object uses it.
fn effects_group(ui: &mut egui::Ui, obj: &mut Object, enabled: bool, changed: &mut bool) {
    let is_media = matches!(obj.kind, Kind::Video | Kind::Audio | Kind::Web(_) | Kind::Pano(_) | Kind::Unsupported(_));
    let is_line = matches!(obj.kind, Kind::Line | Kind::LineUp);
    let is_pic = matches!(obj.kind, Kind::Image);
    if is_line { return; }
    group(ui, "Effects", |ui| { ui.add_enabled_ui(enabled && !is_media, |ui| {
        let panes = [
            (EffectPane::Presets, "Presets", "Ready-made effects", false),
            (EffectPane::Shadow, "Shadow", "A shadow behind the object", obj.shadow.is_some()),
            (EffectPane::Glow, "Glow", "A soft halo of color around the object", obj.glow.is_some()),
            (EffectPane::Reflection, "Reflection", "A faded mirror image below the object", obj.reflection.is_some()),
            (EffectPane::Soft, "Soft edges", "Fade the object's edges", obj.soft_edge > 0.0),
            (EffectPane::Corners, "Corners", "Round the corners of a shape", obj.radius > 0.0),
        ];
        for row in panes.chunks(3) {
            ui.horizontal(|ui| {
                for &(pane, label, tip, on) in row {
                    if pane == EffectPane::Corners && is_pic { continue; }
                    let btn = ui.scope(|ui| {
                        ui.spacing_mut().button_padding = egui::vec2(6.0, 1.0);
                        ui.add(egui::Button::new(egui::RichText::new(label).size(12.0)).selected(on).min_size(egui::vec2(22.0, 22.0)))
                    }).inner.on_hover_text(tip);
                    egui::Popup::from_toggle_button_response(&btn).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
                        *changed |= effect_pane(ui, obj, pane);
                    });
                }
            });
        }
    }); });
}
