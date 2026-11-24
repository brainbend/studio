//! The ribbon: tabbed, icon-led commands above the canvas (Home / Insert / Slides / Design / Transitions / Animations / View).
//!
//! Object property controls edit a *copy* of the primary selected object; whichever fields the user
//! changed are then written to every selected object, so formatting works on multi-selections too.
use super::*;

pub(crate) mod format_panel;

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
pub const JUSTIFY: char = '\u{e482}';
pub const FUNCTION: char = '\u{ebe4}';
pub const PIN: char = '\u{e316}';
pub const THUMBTACK: char = '\u{e3e2}';
pub const GEAR: char = '\u{e270}';
pub const GAUGE: char = '\u{e628}';
pub const HASH: char = '\u{e2a2}';
pub const STAR: char = '\u{e46a}';
pub const CHAT: char = '\u{e15c}';
pub const GLOBE: char = '\u{e288}';
pub const PANORAMA: char = '\u{eaa2}';
pub const STRIKE: char = '\u{e5c2}';
pub const INDENT_OUT: char = '\u{e062}';
pub const INDENT_IN: char = '\u{e064}';
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
pub const LAYOUT: char = '\u{e6d6}';
pub const BUCKET: char = '\u{e392}';
pub const DROP: char = '\u{e210}';
pub const PLAY: char = '\u{e3d0}';
pub const EXPORT: char = '\u{eaf0}';
pub const EYE_OFF: char = '\u{e224}';
pub const TIMER: char = '\u{e492}';
pub const CHECK: char = '\u{e186}';
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
pub const MAGNIFY: char = '\u{e30c}';
pub const ACCESS: char = '\u{e4e8}';
pub const MOON: char = '\u{e330}';
pub const BOOK_OPEN: char = '\u{e0e6}';
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


/// Corner radius used throughout the ribbon (card, buttons, fields).
pub const RIBBON_RADIUS: f32 = 4.0;

/// The height of the title bar: the 24-point row and the margin above and below it (see `title_bar`).
pub const TITLE_BAR_HEIGHT: f32 = 24.0 + 2.0 * 8.0;

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
    if panel_compact(ui) { return compact_widget(ui, icon, label, caret, selected); }
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

/// In the Properties panel the big ribbon buttons are one row tall: the icon, the name and a caret beside each other.
fn panel_compact(ui: &egui::Ui) -> bool { ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new("panel-compact"))).unwrap_or(false) }

/// The next compact button in the Properties panel is this wide (a row's whole control, or half of it).
fn fill_next(ui: &egui::Ui, w: f32) { ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("compact-fill"), w)); }

/// A control in the Properties panel gets a row: its name on the left and the control filling the rest. On the ribbon
/// it is just the control. `f` is given the width of the control.
fn pr(ui: &mut egui::Ui, label: &str, f: impl FnOnce(&mut egui::Ui, f32)) {
    if panel_mode(ui) { format_panel::row(ui, label, |ui, w| { fill_next(ui, w); f(ui, w) }); } else { f(ui, 0.0); }
}

/// The controls of a ribbon group sit side by side; in the Properties panel they stack as rows.
fn hwrap(ui: &mut egui::Ui, f: impl FnOnce(&mut egui::Ui)) { if panel_mode(ui) { ui.vertical(f); } else { ui.horizontal(f); } }

fn compact_widget(ui: &mut egui::Ui, icon: char, label: &str, caret: bool, selected: bool) -> egui::Response {
    compact_widget_in(ui, icon, label, caret, selected, true)
}

/// `compact_widget`; with `filled` off the caller draws what is behind it (Publish's tinted outline).
fn compact_widget_in(ui: &mut egui::Ui, icon: char, label: &str, caret: bool, selected: bool, filled: bool) -> egui::Response {
    let plain = label.replace('\n', " ");
    let font = egui::FontId::proportional(12.0);
    let tw = ui.fonts_mut(|f| f.layout_no_wrap(plain.clone(), font.clone(), egui::Color32::WHITE).size().x);
    let natural = 8.0 + 18.0 + tw + if caret { 16.0 } else { 6.0 };
    let w = ui.ctx().data_mut(|d| d.remove_temp::<f32>(egui::Id::new("compact-fill"))).map(|f| f.max(natural.min(60.0))).unwrap_or(natural);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, 22.0), egui::Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), plain.clone()));
    if ui.is_rect_visible(rect) {
        let enabled = ui.is_enabled();
        let vis = ui.style().interact_selectable(&resp, selected);
        if filled { ui.painter().rect_filled(rect, 3.0, if selected { vis.bg_fill } else if resp.hovered() { vis.weak_bg_fill } else { ui.visuals().widgets.inactive.weak_bg_fill }); }
        let col = if selected { vis.fg_stroke.color } else { text_col(ui) };
        let icon_col = if !selected && enabled && ui.visuals().override_text_color.is_none() { tint(icon) } else { col };
        ui.painter().text(egui::pos2(rect.left() + 6.0, rect.center().y), egui::Align2::LEFT_CENTER, icon, egui::FontId::new(14.0, icons()), icon_col);
        ui.painter().text(egui::pos2(rect.left() + 26.0, rect.center().y), egui::Align2::LEFT_CENTER, &plain, font, col);
        if caret { chevron(ui.painter(), egui::pos2(rect.right() - 9.0, rect.center().y), 2.8, col); }
    }
    resp
}

/// `c` mixed towards black by `t` (0 to 1), still fully opaque.
fn shade(c: egui::Color32, t: f32) -> egui::Color32 {
    let k = 1.0 - t;
    egui::Color32::from_rgb((c.r() as f32 * k).round() as u8, (c.g() as f32 * k).round() as u8, (c.b() as f32 * k).round() as u8)
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

/// An icon and a word in the menu's own color, for a row of a dropdown.
fn menu_job(ui: &egui::Ui, icon: char, text: &str) -> egui::text::LayoutJob {
    let col = text_col(ui);
    let mut job = egui::text::LayoutJob::default();
    job.append(&icon.to_string(), 0.0, egui::text::TextFormat { font_id: egui::FontId::new(16.0, icons()), color: col, ..Default::default() });
    job.append(&format!("   {text}"), 0.0, egui::text::TextFormat { font_id: egui::FontId::proportional(13.0), color: col, ..Default::default() });
    job
}

/// One row in a dropdown: small icon, then text.
pub(crate) fn menu_item(ui: &mut egui::Ui, icon: char, text: &str) -> egui::Response {
    let job = menu_job(ui, icon, text);
    ui.add(egui::Button::new(job).frame(false).min_size(egui::vec2(ui.available_width().min(300.0), 24.0)))
}

fn icon_button(ui: &mut egui::Ui, icon: char, on: bool, tip: &str) -> egui::Response {
    // Small and tight: a 22px square holding a 15px glyph, with no padding of its own.
    let resp = ui.scope(|ui| {
        ui.spacing_mut().button_padding = egui::vec2(1.0, 1.0);
        if icon == THUMBTACK {
            // The pin is painted by hand, 1px left of the box center, so its drawing looks centered.
            let r = ui.add(egui::Button::new("").selected(on).min_size(egui::vec2(22.0, 22.0)));
            let col = ui.style().interact_selectable(&r, on).fg_stroke.color;
            ui.painter().text(r.rect.center() + egui::vec2(-1.0, 0.0), egui::Align2::CENTER_CENTER, icon, egui::FontId::new(15.0, icons()), col);
            r
        } else {
            ui.add(egui::Button::new(icon_text(icon, 15.0)).selected(on).min_size(egui::vec2(22.0, 22.0)))
        }
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

/// The tabs, left to right. Everything about the chosen slide or
/// object is in the Properties panel; the ribbon is for adding things and for the project as a whole.
pub(crate) const ISSUES_URL: &str = "https://github.com/brainbend/studio/issues/new/choose";
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
    // The tab hangs from the top edge, so what shows is the frame, and the label is centered in that.
    let mid = frame.center().y;
    painter.galley(egui::pos2(left, mid - galley.size().y / 2.0), galley, p.ink);
    if p.caret { chevron(painter, egui::pos2(left + gw + 10.0, mid + 0.5), 3.0, p.ink); }
    resp
}

/// The corners and lift of the theme toggle and the buttons beside it: hanging from the window's top edge, or ordinary rounded buttons.
fn tab_shape(hang: bool) -> (egui::CornerRadius, f32) {
    if hang { (egui::CornerRadius { nw: 0, ne: 0, sw: 6, se: 6 }, 3.0) } else { (egui::CornerRadius::same(6), 0.0) }
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
/// Whether the controls are being drawn into the Properties panel (as bars) rather than the ribbon.
fn panel_mode(ui: &egui::Ui) -> bool { ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new("panel-groups"))).unwrap_or(false) }

fn group(ui: &mut egui::Ui, caption: &str, add: impl FnOnce(&mut egui::Ui)) {
    // In the Properties panel a group is a bar of the accordion instead of a block of the ribbon.
    if panel_mode(ui) {
        format_panel::section(ui, caption, caption, false, |ui| { add(ui); });
        return;
    }
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
            resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, *label));
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

/// Fill, outline and effects (shadow, corners) for the object being edited. Shared by the selection groups of the Properties panel.
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
    if panel_mode(ui) { return; }
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


fn multiple_response() -> Question {
    let mut q = Question::new_single();
    q.multiple = true;
    q.choices[1].correct = true;
    q
}
// Every question type is a button of its own, laid out three to a column so they fit the ribbon.

const GRADED_QUESTIONS: [(char, &str, &str, fn() -> Question); 15] = [
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
const SURVEY_QUESTIONS: [(char, &str, &str, fn() -> Question); 2] = [
    (SLIDERS, "Rating scale", "A survey question answered on a scale; not scored", Question::new_likert),
    (TEXTBOX, "Free text", "A survey question answered in the learner's own words; not scored", Question::new_reflection),
];

/// One thing that can be added to a slide: a button that adds it, or one that opens choices first (a table size, a shape).
type InsertLeaf = (char, &'static str, &'static str, fn(&mut App));
type InsertSub = (char, &'static str, &'static str, fn(&mut App, &mut egui::Ui));
enum InsertEntry { Leaf(InsertLeaf), Sub(InsertSub) }

impl InsertEntry {
    fn label(&self) -> &'static str { match self { InsertEntry::Leaf(l) => l.1, InsertEntry::Sub(s) => s.1 } }
    fn tip(&self) -> &'static str { match self { InsertEntry::Leaf(l) => l.2, InsertEntry::Sub(s) => s.2 } }
    fn icon(&self) -> char { match self { InsertEntry::Leaf(l) => l.0, InsertEntry::Sub(s) => s.0 } }
}

/// Everything that can be added to a slide, in three groups: the gallery and the toolbar are both drawn from this.
/// What cannot be added to a state: things with states or values of their own, and media. A state holds decoration (shapes, icons,
/// pictures, text, equations), because those are what the player can show and hide with the state.
fn blocked_in_state(label: &str) -> bool {
    matches!(label, "Button" | "Slider" | "Hotspot" | "Dial" | "Input" | "Marker" | "Video" | "Audio" | "360° Image" | "Web Object")
}

const BLOCKED_TIP: &str = "A state can hold shapes, icons, pictures, text and equations. Press Done to add this to the slide.";

/// How big a button of a toolbar band is, and the room a divider takes.
pub(crate) const BAR_BTN: f32 = 30.0;
pub(crate) const BAR_H: f32 = 28.0;
const BAR_GAP: f32 = 2.0;
const BAR_DIV: f32 = 13.0;

/// A row of icon buttons that stays one row. `dividers[i]` says item `i` is a divider line, not a button. The items are centered when they all fit; when
/// they do not, the ones that do not go in a "…" menu at the end of the row. `draw(ui, i, in_menu)` draws item `i`: as a button in the row, or as a menu entry.
pub(crate) fn overflow_row(ui: &mut egui::Ui, dividers: &[bool], draw: &mut dyn FnMut(&mut egui::Ui, usize, bool)) {
    let width = |i: usize| (if dividers[i] { BAR_DIV } else { BAR_BTN }) + BAR_GAP;
    let total: f32 = (0..dividers.len()).map(width).sum();
    let avail = ui.available_width();
    let fits = total <= avail;
    // How many items show: all of them, or as many as leave room for the menu button (never ending on a divider).
    let mut shown = dividers.len();
    if !fits {
        let (mut used, room) = (0.0, avail - (BAR_BTN + BAR_GAP));
        shown = 0;
        while shown < dividers.len() && used + width(shown) <= room { used += width(shown); shown += 1; }
        while shown > 0 && dividers[shown - 1] { shown -= 1; }
    }
    let used: f32 = (0..shown).map(width).sum::<f32>() + if fits { 0.0 } else { BAR_BTN + BAR_GAP };
    ui.horizontal(|ui| {
        ui.set_height(BAR_H);
        ui.spacing_mut().item_spacing = egui::vec2(BAR_GAP, 2.0);
        ui.spacing_mut().button_padding = egui::vec2(5.0, 4.0);
        ui.add_space(((avail - used) / 2.0).max(0.0));
        for i in 0..shown {
            if dividers[i] {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(BAR_DIV, BAR_H), egui::Sense::hover());
                let line = ui.visuals().text_color().gamma_multiply(if ui.visuals().dark_mode { 0.30 } else { 0.25 });
                ui.painter().line_segment([rect.center_top() + egui::vec2(0.0, 4.0), rect.center_bottom() - egui::vec2(0.0, 4.0)], egui::Stroke::new(1.0, line));
            } else {
                draw(ui, i, false);
            }
        }
        if !fits {
            let more = egui::Button::new("").frame(false).min_size(egui::vec2(BAR_BTN, BAR_H));
            let (resp, _) = egui::containers::menu::MenuButton::from_button(more).ui(ui, |ui| {
                ui.set_min_width(200.0);
                for i in shown..dividers.len() {
                    if dividers[i] { if i > shown { ui.separator(); } } else { draw(ui, i, true); }
                }
            });
            // (the three dots are drawn: they are not in the icon font under a name to rely on)
            let ink = text_col(ui);
            for dx in [-6.0, 0.0, 6.0] { ui.painter().circle_filled(resp.rect.center() + egui::vec2(dx, 0.0), 1.9, ink); }
            resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "More"));
            resp.on_hover_text("More");
        }
    });
}

fn insert_sections() -> [(&'static str, Vec<InsertEntry>); 3] {
    use InsertEntry::{Leaf as L, Sub as S};
    [
        ("Text", vec![
            L((TEXTBOX, "Text Box", "A box to type in", |a| a.insert_text())),
            S((TABLE, "Table", "Rows and columns", |a, ui| { ui.set_min_width(0.0); if let Some((r, c)) = table_picker(ui) { a.insert_table(r, c); ui.close(); } })),
            S((HASH, "Slide Number", "A number that follows the slide", |a, ui| {
                if menu_item(ui, HASH, "Slide number").clicked() { a.insert_slide_number(false); ui.close(); }
                if menu_item(ui, HASH, "Slide number of total").clicked() { a.insert_slide_number(true); ui.close(); }
            })),
            S((TEXTBOX, "Placeholder", "An empty box that asks for text", |a, ui| { for k in PlaceholderKind::CHOICES { if menu_item(ui, TEXT_T, k.label()).clicked() { a.insert_placeholder(k); ui.close(); } } })),
            S((SIGMA, "Symbol", "Greek letters, math, arrows and more", |a, ui| { if a.can_take_text() { if let Some(s) = App::symbol_grid(ui) { a.insert_symbol(&s); ui.close(); } } else { ui.weak("This object has no text"); } })),
            S((LINK, "Hyperlink", "Link text to a web address", |a, ui| { if a.can_take_text() { a.link_form_menu(ui); } else { ui.weak("This object has no text"); } })),
        ]),
        ("Media", vec![
            L((IMAGE, "Picture", "A picture from a file", |a| a.insert_image())),
            S((SHAPES, "Shape", "Rectangles, arrows, stars and more", |a, ui| { if let Some(name) = shape_gallery(ui) { a.insert_shape(name); ui.close(); } })),
            L((VIDEO, "Video", "A video from a file", |a| a.insert_video())),
            L((SPEAKER, "Audio", "A sound from a file", |a| a.insert_audio())),
            S((CHAT, "Caption", "A speech or thought bubble with text", |a, ui| a.caption_menu(ui))),
            L((PANORAMA, "360° Image", "A panoramic picture to look around, with annotations", |a| a.insert_pano())),
            L((FUNCTION, "Equation", "Type an equation in LaTeX", |a| a.open_equation_dialog(None))),
            S((STAR, "Icons", "A vector icon you can recolor and resize", |a, ui| a.icon_gallery(ui))),
            L((GLOBE, "Web Object", "A web page inside the slide", |a| a.insert_web_object())),
        ]),
        ("Interactive", vec![
            L((CLICK, "Button", "A button with hover and click looks", |a| a.insert_button())),
            L((SLIDERS, "Slider", "A slider the learner drags to set a number", |a| a.insert_slider())),
            S((CLICK, "Hotspot", "An invisible area the learner clicks", |a, ui| {
                if menu_item(ui, RECT, "Rectangle").clicked() { a.insert_hotspot(false); ui.close(); }
                if menu_item(ui, CIRCLE, "Oval").clicked() { a.insert_hotspot(true); ui.close(); }
                if menu_item(ui, POLYLINE, "Freeform (click its corners)").clicked() {
                    a.view.draw_outline = Some(vec![]);
                    a.view.draw_slide = a.view.slide;
                    a.view.selected = None;
                    a.view.multi.clear();
                    a.status = "Click each corner of the hotspot. Enter, double-click or the first corner finishes.".into();
                    ui.close();
                }
            })),
            S((GAUGE, "Dial", "A dial the learner turns to set a number", |a, ui| a.dial_menu(ui))),
            S((CHECK, "Input", "Check boxes, radio buttons and entry fields", |a, ui| a.input_menu(ui))),
            S((PIN, "Marker", "A round button with an icon or a number in it", |a, ui| a.marker_gallery(ui))),
        ]),
    ]
}


/// What the Studio Settings controls change: copies of the settings, applied by `App::apply_settings` after the controls are drawn.
pub(crate) struct SettingsDraft {
    one: bool,
    each_tab: bool,
    preset: Option<crate::dock::Preset>,
    menu_bar: bool,
    insert_bar: bool,
    panels: crate::prefs::Panels,
}

/// The Studio Settings controls, in the window and on the File page.
pub(crate) fn settings_controls(ui: &mut egui::Ui, d: &mut SettingsDraft) {
        ui.label(egui::RichText::new("Windows and panels").strong());
        ui.checkbox(&mut d.one, "Open one section at a time").on_hover_text("In the Properties panel, opening a section closes the one that was open. Turn it off to keep several open at once.");
        ui.checkbox(&mut d.each_tab, "Open every slide in a new tab").on_hover_text("Otherwise a slide chosen in the slide list replaces the one in the tab. Double-click or middle-click a slide, or use its menu, to open it in a tab of its own.");
    ui.checkbox(&mut d.insert_bar, "Show the insert bar").on_hover_text("The row above the slide with + Add and the objects you add most");
        ui.checkbox(&mut d.menu_bar, "Show the menu bar").on_hover_text("File, Edit, View, Window and Help along the top of the window");
        ui.add_space(10.0);
        ui.label(egui::RichText::new("Where things are").strong());
        ui.weak("Drag a tab to move it: onto another group, to the edge of a group for a new row or column, to the left or top edge of the slide for a panel there, or out of the app for a window. Right-click a tab for the same moves from a menu.");
        ui.horizontal_wrapped(|ui| {
            for pr in crate::dock::Preset::ALL {
                if ui.button(pr.name()).on_hover_text(pr.about()).clicked() { d.preset = Some(pr); }
            }
        });
        ui.weak("Standard is how the app started. Tools go back to windows of their own.");
        ui.add_space(10.0);
        ui.label(egui::RichText::new("What shows").strong());
        ui.weak("Hide what you do not use. For example, hide Properties if the AI does most of the designing.");
        ui.add_space(2.0);
        ui.label("Bottom panel");
        ui.horizontal(|ui| {
            ui.checkbox(&mut d.panels.timeline, "Timeline").on_hover_text("When things appear and move");
            ui.checkbox(&mut d.panels.notes, "Slide Notes");
            ui.checkbox(&mut d.panels.states, "States").on_hover_text("The looks of a button or object in each state");
        });
        ui.label("Side panel");
        ui.horizontal(|ui| {
            ui.checkbox(&mut d.panels.properties, "Properties").on_hover_text("The settings of the slide or object you pick");
            ui.checkbox(&mut d.panels.triggers, "Triggers");
            ui.checkbox(&mut d.panels.ai, "AI");
        });
        ui.label("Slide");
        ui.horizontal(|ui| {
            ui.checkbox(&mut d.panels.slides, "Slide List").on_hover_text("The slides of the scene, as thumbnails");
            ui.checkbox(&mut d.panels.layers, "Slide Layers");
        });
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
        let layers = &mut self.project.as_mut().unwrap().slide_mut(self.view.slide).layers;
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

    /// Everything above the canvas: the title bar, the menu bar and the tab strip.
    pub(super) fn chrome(&mut self, root: &mut egui::Ui, ctx: &egui::Context) {
        // The harness draws a first frame before fonts exist; the icon font is needed from here on.
        if !ctx.fonts(|f| f.families().contains(&icons())) { return; }
        self.sync_color_events(ctx);
        if self.caption { self.caption_bar(root, ctx); }
        if self.merged_bar { self.window_bar(root, ctx); } else { self.title_bar(root, ctx); self.menu_bar(root); }
        self.run_format_painter(ctx);
        // The Quick Styles are built from the theme's colors.
        if let Some(p) = &self.project { ctx.data_mut(|d| d.insert_temp(egui::Id::new("theme-colors"), p.theme.colors.clone())); }
        if self.project.is_none() || self.file_view { return; }
        if self.project.as_ref().is_some_and(|p| p.is_web()) { return; }
        if !ctx.input(|i| i.pointer.any_down()) && ctx.memory(|m| m.focused().is_none()) { self.edit_open = false; }
        self.doc_tabs(root);
    }



    fn tab_actions(&mut self, ui: &mut egui::Ui, master: bool, hang: bool) {
        if master { return; }
        ui.spacing_mut().item_spacing.x = 6.0;
        // The same hanging tabs as the theme toggle. The row runs right to left, so Publish comes first and ends up furthest right;
        // it is the main action, so it is the one in the main-action color.
        let ink = text_col(ui);
        let (hanging, lift) = tab_shape(hang);
        let publishing = self.file_view && self.file_page == backstage::FilePage::Publish;
        let (base, on) = (primary(ui), on_primary(ui));
        let fill = if publishing { shade(base, 0.2) } else { base };
        let publish = pill(ui, "publish", Pill { icon: Some((EXPORT, on)), label: "Publish", caret: false, fill: Some(fill), ink: on, outline: false, bold: true, radius: hanging, lift, min_w: 60.0 })
            .on_hover_text("Publish the project: to the web, an LMS, PDF, Word, PowerPoint or video");
        publish.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Publish"));
        if publish.clicked() { self.open_file_page(backstage::FilePage::Publish); }
        let preview = pill(ui, "preview", Pill { icon: Some((PLAY, ink)), label: "Preview", caret: true, fill: Some(ink.gamma_multiply(0.14)), ink, outline: true, bold: false, radius: hanging, lift, min_w: 60.0 })
            .on_hover_text("Play the lesson in your browser");
        preview.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Preview"));
        egui::Popup::from_toggle_button_response(&preview).close_behavior(egui::PopupCloseBehavior::CloseOnClick).show(|ui| { ui.set_min_width(190.0); self.preview_items(ui); });
        if self.can_try() {
            let tip = "Press buttons on the slide to see their Hover, Down, Visited and Selected looks and the layers they show. Nothing is changed or saved. Esc stops.";
            let lit = if self.try_mode { ACCENT } else { ink.gamma_multiply(0.14) };
            let try_it = pill(ui, "try-it", Pill { icon: Some((CLICK, if self.try_mode { egui::Color32::WHITE } else { ink })), label: "Try It", caret: false, fill: Some(lit), ink: if self.try_mode { egui::Color32::WHITE } else { ink }, outline: !self.try_mode, bold: false, radius: hanging, lift, min_w: 60.0 })
                .on_hover_text(tip);
            try_it.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, self.try_mode, "Try It"));
            if try_it.clicked() { self.try_mode = !self.try_mode; }
        }
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

    /// While the Background bar of the Properties panel is open (it says so each time it is drawn), a gradient's dots show on the slide.
    fn background_bar_open(ctx: &egui::Context) -> bool {
        let seen: u64 = ctx.data(|d| d.get_temp(egui::Id::new("background-bar-seen"))).unwrap_or(0);
        seen != 0 && ctx.cumulative_pass_nr().saturating_sub(seen) <= 2
    }

    /// While the Background window shows a gradient, dots on the slide set where it starts, ends and centres: drag them.
    pub(super) fn background_handles(&mut self, ui: &mut egui::Ui, painter: &egui::Painter, origin: egui::Pos2, scale: f32, slide_idx: usize) {
        if !Self::background_bar_open(ui.ctx()) || slide_idx != self.view.slide { return; }
        let Some(proj) = self.project.as_ref() else { return };
        let Some(sl) = proj.slide_get(slide_idx) else { return };
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
            let Some(Background::Gradient { angle, geom, .. }) = self.project.as_mut().and_then(|p| p.slide_get_mut(slide_idx)).and_then(|s| s.background.as_mut()) else { continue };
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
    pub(super) fn theme_colors_window(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
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
        crate::dock_tool(ui, "Brand Colors", &mut open, |ui| {
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
        egui::Panel::top("titlebar").frame(egui::Frame::new().fill(bg).inner_margin(egui::Margin::symmetric(14, 8))).show_separator_line(false).show(root, |ui| {
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
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.add_space(if cfg!(target_os = "macos") { 76.0 } else { 4.0 });
                if has { self.quick_icons(ui, false); }
                self.bar_actions(ui, ctx, true, 10.0);
            });
        });
    }

    /// The row's right end: the theme toggle, then Try It, Preview and Publish at its far right (and the update button to the left of the toggle).
    /// `hang` is for a bar with nothing above it, where they hang from the window's top edge; in a bar below a title bar of the system's they are ordinary buttons.
    /// `gap` is the room left at the right end. Returns the room the buttons took.
    pub(crate) fn bar_actions(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, hang: bool, gap: f32) -> egui::Rect {
        let shape = tab_shape(hang);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(gap);
            // Publish, Preview and Try It live here.
            if self.project.is_some() && !self.file_view {
                let master = self.in_master_view();
                self.tab_actions(ui, master, hang);
                ui.add_space(6.0);
            }
            let dark = ctx.theme() == egui::Theme::Dark;
            // The moon is black so it shows on the light theme; the sun keeps its yellow.
            let ink = text_col(ui);
            let toggle = pill(ui, "theme", Pill { icon: Some((if dark { SUN } else { MOON }, if dark { tint(SUN) } else { egui::Color32::BLACK })), label: "", caret: false, fill: Some(ink.gamma_multiply(0.14)), ink, outline: true, bold: false, radius: shape.0, lift: shape.1, min_w: 34.0 })
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
                let alert = pill(ui, "update", Pill { icon: Some((WARNING_CIRCLE, white)), label, caret: false, fill: Some(fill), ink: white, outline: false, bold: true, radius: shape.0, lift: shape.1, min_w: 60.0 })
                    .on_hover_text(format!("Version {} is available. Click to update.", r.version));
                alert.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
                if alert.clicked() { self.updater.dialog_open = true; }
            }
        }).response.rect
    }

    /// Save, Undo, Redo and Find. `neutral` draws them in the text color, to sit with the menus, instead of in their own colors.
    pub(crate) fn quick_icons(&mut self, ui: &mut egui::Ui, neutral: bool) {
        let icon = |ui: &mut egui::Ui, enabled: bool, c: char, tip: &str| {
            let color = if neutral { text_col(ui) } else { tint(c) };
            ui.add_enabled(enabled, egui::Button::new(icon_text(c, 17.0).color(color)).min_size(egui::vec2(24.0, 24.0))).on_hover_text(tip)
        };
        let (cmd, shift) = if cfg!(target_os = "macos") { ("⌘", "⇧⌘") } else { ("Ctrl+", "Ctrl+Shift+") };
        if icon(ui, true, SAVE, &format!("Save ({cmd}S)")).clicked() { self.save(false); }
        if icon(ui, !self.undo.is_empty(), UNDO, &format!("Undo ({cmd}Z)")).clicked() { self.undo(); }
        if icon(ui, !self.redo.is_empty(), REDO, &format!("Redo ({shift}Z)")).clicked() { self.redo(); }
        if icon(ui, true, SEARCH, &format!("Find and replace text in the whole project ({cmd}F)")).clicked() { self.find.get_or_insert_with(Default::default).focus = true; }
    }


    /// The settings as they are now, for the controls to change; `apply_settings` puts what they changed back.
    pub(crate) fn settings_draft(&self) -> SettingsDraft {
        SettingsDraft { one: self.one_section, each_tab: self.new_tab_each, preset: None, menu_bar: self.show_menu_bar, insert_bar: self.show_insert_bar, panels: self.panels }
    }

    pub(crate) fn apply_settings(&mut self, d: SettingsDraft) {
        let SettingsDraft { one, each_tab, preset, menu_bar, insert_bar, panels } = d;
        if let Some(pr) = preset { self.apply_preset(pr); }
        if menu_bar != self.show_menu_bar { self.run_cmd(crate::commands::Cmd::ShowMenuBar); }
        if insert_bar != self.show_insert_bar { self.run_cmd(crate::commands::Cmd::ShowInsertBar); }
        if one != self.one_section { self.one_section = one; crate::prefs::set_bool("one_section_open", one); }
        if each_tab != self.new_tab_each { self.new_tab_each = each_tab; crate::prefs::set_bool("slides_in_new_tabs", each_tab); }
        for (key, was, now) in [("show_properties", self.panels.properties, panels.properties), ("show_timeline", self.panels.timeline, panels.timeline), ("show_notes", self.panels.notes, panels.notes), ("show_states", self.panels.states, panels.states), ("show_triggers", self.panels.triggers, panels.triggers), ("show_ai", self.panels.ai, panels.ai), ("show_layers", self.panels.layers, panels.layers), ("show_slides", self.panels.slides, panels.slides)] {
            if was != now { crate::prefs::set_bool(key, now); }
        }
        self.panels = panels;
    }



    /// What the Properties panel calls the selected object, or `None` when it has no such tab yet.
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

    /// What the Properties panel adds for a table: styles, rows and columns, merging, and borders. They act on the chosen cells.
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
                if !panel_mode(ui) {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        opts_changed |= ui.checkbox(&mut opts.header_row, "Header row").changed();
                        opts_changed |= ui.checkbox(&mut opts.banded_rows, "Banded rows").changed();
                        opts_changed |= ui.checkbox(&mut opts.first_column, "First column bold").changed();
                    });
                }
            });
            // In the Properties panel the options go under the swatches, where there is room for their words.
            if panel_mode(ui) {
                opts_changed |= ui.checkbox(&mut opts.header_row, "Header row").changed();
                opts_changed |= ui.checkbox(&mut opts.banded_rows, "Banded rows").changed();
                opts_changed |= ui.checkbox(&mut opts.first_column, "First column bold").changed();
            }
        });
        // Rows and columns, merging and choosing cells: two menus rather than a row of word buttons.
        let mut op = None;
        let dims = self.selected_object().and_then(|o| o.group.clone()).and_then(|g| self.project.as_ref().map(|p| t::dims(&p.slide(self.view.slide).layers[self.view.selected.map(|s| s.0).unwrap_or(0)].objects, &g))).unwrap_or((0, 0));
        let mut merge = false;
        let mut split = false;
        let mut choose: Option<((u16, u16), (u16, u16))> = None;
        group(ui, "Rows & Columns", |ui| {
            hwrap(ui, |ui| {
                pr(ui, "Add", |ui, _| { big_menu(ui, TABLE, "Add", "Add a row or column next to the chosen cell", |ui| {
                    for (icon, label, o) in [(ARROW_UP, "Row above", TableOp::RowAbove), (ARROW_DOWN, "Row below", TableOp::RowBelow), (AL_LEFT, "Column left", TableOp::ColLeft), (AL_RIGHT, "Column right", TableOp::ColRight)] {
                        if menu_item(ui, icon, label).clicked() { op = Some(o); }
                    }
                }); });
                pr(ui, "Delete", |ui, _| { big_menu(ui, TRASH, "Delete", "Delete the chosen cell's row or column", |ui| {
                    for (label, o) in [("Delete row", TableOp::DeleteRow), ("Delete column", TableOp::DeleteCol)] {
                        if menu_item(ui, TRASH, label).clicked() { op = Some(o); }
                    }
                }); });
                pr(ui, "Cells", |ui, _| { big_menu(ui, SQUARE_BORDER, "Cells", "Merge, split or choose cells. Click a cell, then shift-click another to choose a block", |ui| {
                    ui.add_enabled_ui(can_merge, |ui| { if menu_item(ui, SQUARE_BORDER, "Merge cells").clicked() { merge = true; } });
                    ui.add_enabled_ui(can_split, |ui| { if menu_item(ui, SQUARE_BORDER, "Split cell").clicked() { split = true; } });
                    ui.separator();
                    ui.add_enabled_ui(block.is_some(), |ui| {
                        let (r0, c0, r1, c1) = block.as_ref().map(|b| b.1).unwrap_or((0, 0, 0, 0));
                        if menu_item(ui, AL_CENTER_H, "Choose row").clicked() { choose = Some(((r0, 0), (r1, dims.1.saturating_sub(1)))); }
                        if menu_item(ui, AL_CENTER_V, "Choose column").clicked() { choose = Some(((0, c0), (dims.0.saturating_sub(1), c1))); }
                        if menu_item(ui, TABLE, "Choose all cells").clicked() { choose = Some(((0, 0), (dims.0.saturating_sub(1), dims.1.saturating_sub(1)))); }
                    });
                }); });
            });
        });
        // Borders for the chosen cells.
        let mut borders: Option<(t::BorderSet, Option<Stroke>)> = None;
        group(ui, "Borders", |ui| {
            ui.vertical(|ui| {
                if panel_mode(ui) {
                    let mut c = Some(self.border_color);
                    format_panel::row(ui, "Color", |ui, w| { if format_panel::color_field(ui, w, &mut c, None) { if let Some(c) = c { self.border_color = c; } } });
                    format_panel::row(ui, "Thickness", |ui, w| { ui.add_sized([w, 22.0], egui::DragValue::new(&mut self.border_width).range(0.5..=24.0).speed(0.25).suffix(" px")); });
                } else {
                ui.horizontal(|ui| {
                    let mut c = Some(self.border_color);
                    if color_menu(ui, PEN, "Border color", &mut c, None) { if let Some(c) = c { self.border_color = c; } }
                    framed(ui, |ui| { ui.add(egui::DragValue::new(&mut self.border_width).range(0.5..=24.0).speed(0.25).suffix(" px")); });
                });
                }
                let line = Some(Stroke { color: self.border_color, width: self.border_width });
                let menu = if panel_mode(ui) { let mut r = None; format_panel::row(ui, "Apply to", |ui, w| { fill_next(ui, w); r = Some(compact_widget(ui, SQUARE_BORDER, "Borders", true, false)); }); r.unwrap() } else { caret_button(ui, "Borders") };
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

    /// The groups of the Properties panel that belong to the selected object: styles, fill and outline, and arrangement.
    fn format_groups(&mut self, ui: &mut egui::Ui, obj: &mut Object, changed: &mut bool) {
        let has_sel = self.view.selected.is_some();
        let n_sel = self.selection().len();
        let mut style_pick: Option<crate::button_styles::StyleDef> = None;
        let mut pick_captions = false;
        if obj.has_button_states() {
            group(ui, "Button Styles", |ui| {
                let corners = &mut self.button_corners;
                pr(ui, "Look", |ui, _| {
                    big_menu_open(ui, SHAPES, "Styles", "Ready-made button looks: nine colors in four looks, with square, rounded or pill corners", |ui| {
                        if let Some(def) = crate::button_styles::gallery(ui, corners) { style_pick = Some(def); ui.close(); }
                    });
                });
                // The gloss of the state being viewed in the States tab (the state the object starts in otherwise).
                if !matches!(obj.kind, Kind::Image) {
                    let name = self.view.state_view.as_ref().filter(|(id, _)| *id == obj.id).map(|(_, n)| n.clone()).unwrap_or_else(|| obj.initial_state.clone());
                    if let Some(st) = obj.states.iter_mut().find(|s| s.name == name) {
                        pr(ui, "Shine", |ui, _| {
                            let mut on = st.shine.is_some();
                            if ui.checkbox(&mut on, "").on_hover_text("A soft white gloss over the fill, bright at the top. Hover is usually stronger and Down reversed").changed() {
                                st.shine = on.then_some(brainbend_doc::Shine { alpha: 0.4, reverse: false });
                                *changed = true;
                            }
                            if let Some(sh) = &mut st.shine {
                                *changed |= ui.add(egui::Slider::new(&mut sh.alpha, 0.0..=1.0).show_value(false)).changed();
                                *changed |= ui.checkbox(&mut sh.reverse, "reversed").changed();
                            }
                        });
                    }
                }
            });
        }
        if matches!(obj.kind, Kind::Image) {
            let crop_info = (n_sel == 1 && obj.asset.is_some()).then(|| (obj.id.clone(), obj.crop.is_some()));
            self.picture_group(ui, &crop_info);
        }
        if obj.table.is_some() { self.table_groups(ui); }
        let is_media = matches!(obj.kind, Kind::Video | Kind::Audio);
        // The Properties panel has Playback and Captions among its Settings.
        if is_media && !panel_mode(ui) {
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
                if let Some(o) = self.project.as_mut().and_then(|p| p.slide_get_mut(self.view.slide)).and_then(|s| s.layers.iter_mut().flat_map(|l| l.objects.iter_mut()).find(|o| o.id == id)) { o.captions = Some(key); }
                self.dirty = true;
            }
        }
    }

    /// Crop, shape and ratio for a single picture. Shared by the selection groups of the Properties panel.
    fn picture_group(&mut self, ui: &mut egui::Ui, crop_info: &Option<(String, bool)>) {
        group(ui, "Picture", |ui| {
            hwrap(ui, |ui| {
                ui.add_enabled_ui(crop_info.is_some(), |ui| {
                    let on = crop_info.as_ref().is_some_and(|(id, _)| self.view.crop_mode.as_deref() == Some(id.as_str()));
                    pr(ui, "Crop", |ui, w| {
                        let half = if w > 0.0 { (w - ui.spacing().item_spacing.x) / 2.0 } else { 0.0 };
                        if half > 0.0 { fill_next(ui, half); }
                        if big_toggle(ui, CROP, "Crop", "Crop the picture: drag its edges, or drag inside it to move the picture (Esc or Enter to finish)", on).clicked() {
                            self.view.crop_mode = if on { None } else { crop_info.as_ref().map(|(id, _)| id.clone()) };
                        }
                        ui.add_enabled_ui(crop_info.as_ref().is_some_and(|c| c.1), |ui| {
                            if half > 0.0 { fill_next(ui, half); }
                            if big_button(ui, ARROW_COUNTER, "Reset", "Remove the crop and show the whole picture").clicked() { self.reset_crop(); }
                        });
                    });
                    // Crop to a shape: only the part of the picture inside the shape shows.
                    pr(ui, "Shape", |ui, _| {
                        let cur = self.selected_object().and_then(|o| o.mask);
                        big_menu(ui, CIRCLE, "Shape", "Crop the picture to a shape: oval, rounded, diamond, star ...", |ui| {
                            if ui.selectable_label(cur.is_none(), "None").clicked() { self.set_picture_shape(None); ui.close(); }
                            for shape in brainbend_doc::PictureShape::ALL {
                                if ui.selectable_label(cur == Some(shape), shape.label()).clicked() { self.set_picture_shape(Some(shape)); ui.close(); }
                            }
                        });
                    });
                    // The shape of the crop frame: pick one and the frame closes in to it and keeps it while dragged.
                    ui.add_enabled_ui(self.view.crop_mode.is_some(), |ui| {
                        pr(ui, "Ratio", |ui, _| {
                            big_menu(ui, FRAME, "Ratio", "Hold the crop to a shape of frame: 1:1, 4:3, 16:9 ...", |ui| {
                                let native = crop_info.as_ref().and_then(|(id, _)| self.picture_ratio(id));
                                let cur = self.view.crop_aspect;
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
        });
    }

    /// Outline editing for a single hotspot. Shared by the selection groups of the Properties panel.
    fn hotspot_group(&mut self, ui: &mut egui::Ui, obj: &Object, enabled: bool, n_sel: usize) {
        group(ui, "Hotspot", |ui| {
            hwrap(ui, |ui| {
                // Only a single hotspot has an outline to edit: (its id, whether it has an outline that is not just its box).
                let spot = (enabled && n_sel == 1 && matches!(obj.kind, Kind::Hotspot)).then(|| (obj.id.clone(), obj.hit_area.as_ref().is_some_and(|a| !outline::is_box(a)), obj.hit_area.is_some()));
                ui.add_enabled_ui(spot.is_some(), |ui| {
                    let on = spot.as_ref().is_some_and(|(id, _, _)| self.view.outline_mode.as_deref() == Some(id.as_str()));
                    pr(ui, "Outline", |ui, _| {
                        if big_toggle(ui, NODES, "Outline", "Edit the hotspot's outline: drag a corner, click an edge to add one, Delete removes the chosen corner (Esc or Enter to finish)", on).clicked() {
                            if on { self.end_outline(); } else { self.begin_outline(); }
                        }
                    });
                    ui.add_enabled_ui(spot.as_ref().is_some_and(|s| s.1), |ui| {
                        pr(ui, "Frame", |ui, _| { if big_button(ui, FRAME, "Fit", "Shrink the hotspot's frame to its outline").clicked() { self.fit_outline(); } });
                    });
                    ui.add_enabled_ui(spot.as_ref().is_some_and(|s| s.2), |ui| {
                        pr(ui, "Shape", |ui, _| { if big_button(ui, ARROW_COUNTER, "Box", "Remove the outline: the whole frame is clickable again").clicked() { self.reset_outline(); } });
                    });
                });
            });
        });
    }

    /// Order, group and align the selection. Shared by the selection groups of the Properties panel.
    fn arrange_group(&mut self, ui: &mut egui::Ui, has_sel: bool, n_sel: usize) {
        if panel_mode(ui) { return; }
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
        let after = styled_like(&before, &proto);
        self.apply_object_edit(ctx, &before, &after);
        self.clear_word_styles();
        self.painter = None;
        self.status = "Format Painter: applied.".into();
    }

    /// After a style is given to the selected objects, their words lose any bold, italic, size or color of their own (a link stays),
    /// so the style shows all the way through.
    fn clear_word_styles(&mut self) {
        let targets = self.format_targets();
        let Some(p) = self.project.as_mut() else { return };
        for (li, oi) in targets {
            if let Some(o) = p.slide_mut(self.view.slide).layers.get_mut(li).and_then(|l| l.objects.get_mut(oi)) {
                if o.rich.is_some() { o.format_range(0, usize::MAX, &brainbend_doc::richtext::Attr::Clear); }
            }
        }
    }

    /// Paste Style: the style copied with Copy Style goes onto whatever is selected (and can be pasted again and again).
    pub(crate) fn paste_style(&mut self, ctx: &egui::Context) {
        let Some(proto) = self.style_clip.clone() else { return };
        let Some(before) = self.selected_object().cloned() else { return };
        let after = styled_like(&before, &proto);
        self.apply_object_edit(ctx, &before, &after);
        self.clear_word_styles();
        self.status = "Paste Style: applied.".into();
    }

    /// The grid of common symbols, in a menu or a popup: the one clicked, if any.
    fn symbol_grid(ui: &mut egui::Ui) -> Option<String> {
        const GROUPS: [(&str, &str); 6] = [
            ("Greek", "αβγδεζηθικλμνξοπρστυφχψωΓΔΘΛΞΠΣΦΨΩ"),
            ("Math", "±×÷≠≈≤≥∞√∑∏∫∂∆∇∈∉⊂⊃⊆⊇∪∩∧∨¬∀∃∅≡∝∠∴∵−·°′″²³½¼¾‰"),
            ("Arrows", "←→↑↓↔⇐⇒⇔↦↵"),
            ("Currency", "€£¥¢₹₩¤"),
            ("Writing", "©®™§¶†‡•…–—«»‘’“”‹›"),
            ("Small numbers", "⁰¹²³⁴⁵⁶⁷⁸⁹⁺ⁿ₀₁₂₃₄₅₆₇₈₉₊"),
        ];
        let mut pick: Option<String> = None;
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
        pick
    }

    /// Whether the selection can hold a symbol or a link: with nothing selected they start a new text box; only a picture-like object has no text.
    fn can_take_text(&self) -> bool {
        self.selected_object().is_none_or(|o| !matches!(o.kind, Kind::Image | Kind::Video | Kind::Audio | Kind::Hotspot | Kind::Web(_) | Kind::Pano(_) | Kind::Input(_) | Kind::Slider(_) | Kind::Unsupported(_)))
    }

    /// The Hyperlink form as a menu shows it: the address (and, with no text to link, the words to show) and Apply or Insert.
    fn link_form_menu(&mut self, ui: &mut egui::Ui) {
        ui.set_min_width(260.0);
        let has_text = self.selected_object().is_some_and(|o| !o.text.is_empty());
        let cur: String = self.selected_object().and_then(|o| {
            let (a, b) = self.view.text_sel.unwrap_or((0, usize::MAX));
            o.char_fmts().into_iter().enumerate().find(|(i, (_, f))| *i >= a && *i < b && f.link.is_some()).and_then(|(_, (_, f))| f.link)
        }).unwrap_or_default();
        let key = egui::Id::new("menu-link-form");
        let (seen, mut url, mut shown): (String, String, String) = ui.data_mut(|d| d.get_temp(key)).unwrap_or((String::from("\u{0}"), String::new(), String::new()));
        if seen != cur { url = cur.clone(); shown.clear(); }
        if !has_text {
            ui.label("Text to show");
            ui.add(egui::TextEdit::singleline(&mut shown).desired_width(240.0).hint_text("Same as the address"));
        }
        ui.label("Web address (https:// or mailto:)");
        ui.add(egui::TextEdit::singleline(&mut url).desired_width(240.0).hint_text("https://example.com"));
        let ok = url.starts_with("https://") || url.starts_with("http://") || url.starts_with("mailto:");
        let (mut apply, mut remove) = (false, false);
        ui.horizontal(|ui| {
            apply = ui.add_enabled(ok, egui::Button::new(if has_text { "Apply" } else { "Insert" })).clicked();
            remove = has_text && ui.button("Remove link").clicked();
        });
        ui.data_mut(|d| d.insert_temp(key, (cur, url.clone(), shown.clone())));
        if apply || remove {
            if has_text { self.link_selected(apply.then(|| url.trim().to_string())); }
            else { self.insert_link_text(shown.trim(), url.trim()); }
            ui.close();
        }
    }

    /// A slim toolbar at the top of the slide area (between the panels, not across the window): a button for each thing that can be added,
    /// grouped as Text, Media and Interactive, each named in a tooltip. A button with choices (a table size, a shape) opens them below it.
    pub(crate) fn insert_bar(&mut self, root: &mut egui::Ui) {
        // The Scene View, the 360° editor and Try It have nothing to add objects to.
        if !self.show_insert_bar || self.scene_view || self.pano.edit.is_some() || self.try_mode { return; }
        let band = if root.visuals().dark_mode { egui::Color32::from_gray(40) } else { egui::Color32::from_gray(246) };
        egui::Panel::top("insert-bar").frame(egui::Frame::new().fill(band).inner_margin(egui::Margin::symmetric(8, 3))).show_separator_line(false).show(root, |ui| {
            self.insert_bar_row(ui);
        });
    }

    /// The same bar inside a window of its own (an editor window), on a band like the main window's.
    pub(crate) fn insert_bar_in(&mut self, ui: &mut egui::Ui) {
        if !self.show_insert_bar || self.try_mode { return; }
        let band = if ui.visuals().dark_mode { egui::Color32::from_gray(40) } else { egui::Color32::from_gray(246) };
        egui::Frame::new().fill(band).inner_margin(egui::Margin::symmetric(8, 3)).show(ui, |ui| self.insert_bar_row(ui));
    }

    /// The buttons of the insert bar, centered in the room there is. What does not fit goes in the menu at its end, so the bar is one row.
    fn insert_bar_row(&mut self, ui: &mut egui::Ui) {
        let mut items: Vec<Option<InsertEntry>> = vec![];
        for (n, (_, entries)) in insert_sections().into_iter().enumerate() {
            if n > 0 { items.push(None); }
            items.extend(entries.into_iter().map(Some));
        }
        let dividers: Vec<bool> = items.iter().map(|e| e.is_none()).collect();
        overflow_row(ui, &dividers, &mut |ui, i, in_menu| {
            let Some(entry) = items[i].as_ref() else { return };
            let (label, tip) = (entry.label(), entry.tip());
            let blocked = self.view.state_editing && self.swap.is_some() && blocked_in_state(label);
            if in_menu {
                if blocked { ui.add_enabled(false, egui::Label::new(egui::RichText::new(label).weak())).on_disabled_hover_text(BLOCKED_TIP); return; }
                match entry {
                    InsertEntry::Leaf((icon, _, _, act)) => { if menu_item(ui, *icon, label).on_hover_text(tip).clicked() { act(self); ui.close(); } }
                    InsertEntry::Sub((icon, _, _, content)) => { ui.menu_button(menu_job(ui, *icon, label), |ui| content(self, ui)).response.on_hover_text(tip); }
                }
                return;
            }
            let button = egui::Button::new(egui::RichText::new(entry.icon().to_string()).font(egui::FontId::new(18.0, icons())).color(text_col(ui))).frame(false);
            let named = |r: egui::Response| { r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label)); r.on_hover_ui(|ui| { ui.strong(label); ui.weak(tip); }) };
            if blocked { ui.add_enabled(false, egui::Button::new(egui::RichText::new(entry.icon().to_string()).font(egui::FontId::new(18.0, icons()))).frame(false).min_size(egui::vec2(BAR_BTN, BAR_H))).on_disabled_hover_text(BLOCKED_TIP); return; }
            match entry {
                InsertEntry::Leaf((_, _, _, act)) => { let r = named(ui.add_sized([BAR_BTN, BAR_H], button)); if r.clicked() { act(self); } }
                InsertEntry::Sub((_, _, _, content)) => { named(egui::containers::menu::MenuButton::from_button(button.min_size(egui::vec2(BAR_BTN, BAR_H))).ui(ui, |ui| content(self, ui)).0); }
            }
        });
    }

    /// The names of the things that are added by one click (the ones with no choices to make), in the order `insert_leaf_act` takes.
    pub(crate) fn insert_leaf_labels() -> Vec<&'static str> {
        insert_sections().into_iter().flat_map(|(_, entries)| entries).filter_map(|e| match e { InsertEntry::Leaf(l) => Some(l.1), InsertEntry::Sub(_) => None }).collect()
    }

    /// Add the thing at place `n` of `insert_leaf_labels`.
    pub(crate) fn insert_leaf_act(&mut self, n: usize) {
        let act = insert_sections().into_iter().flat_map(|(_, entries)| entries).filter_map(|e| match e { InsertEntry::Leaf(l) => Some(l.3), InsertEntry::Sub(_) => None }).nth(n);
        if self.view.state_editing && self.swap.is_some() && Self::insert_leaf_labels().get(n).is_some_and(|l| blocked_in_state(l)) { return; }
        if let Some(act) = act { act(self); }
    }

    /// Everything that can be added to a slide, as a menu with a search box on top: the gallery behind the + button and the Insert menu.
    /// Typing narrows the list to the objects whose names match; an object with choices (a table size, a shape) opens them beside it.
    pub(crate) fn insert_menu(&mut self, ui: &mut egui::Ui) {
        let sections = insert_sections();
        ui.set_min_width(240.0);
        let id = egui::Id::new("insert-search");
        let mut query: String = ui.data_mut(|d| d.get_temp(id)).unwrap_or_default();
        let field = ui.add(egui::TextEdit::singleline(&mut query).hint_text("Search objects").desired_width(240.0));
        // Typing starts the search as soon as the gallery opens.
        if !field.has_focus() && ui.memory(|m| m.focused().is_none()) { field.request_focus(); }
        ui.data_mut(|d| d.insert_temp(id, query.clone()));
        let needle = query.trim().to_lowercase();
        let wanted = |label: &str, tip: &str| needle.is_empty() || label.to_lowercase().contains(&needle) || tip.to_lowercase().contains(&needle);
        let mut found = false;
        let room = (ui.ctx().content_rect().height() - 160.0).max(200.0);
        egui::ScrollArea::vertical().max_height(room).auto_shrink([true, true]).show(ui, |ui| {
        for (heading, entries) in sections {
            let shown: Vec<InsertEntry> = entries.into_iter().filter(|e| wanted(e.label(), e.tip())).collect();
            if shown.is_empty() { continue; }
            found = true;
            if needle.is_empty() { ui.label(egui::RichText::new(heading).weak().size(12.0)); }
            for e in shown {
                if self.view.state_editing && self.swap.is_some() && blocked_in_state(e.label()) {
                    ui.add_enabled(false, egui::Label::new(egui::RichText::new(e.label()).weak())).on_disabled_hover_text(BLOCKED_TIP);
                    continue;
                }
                match e {
                    InsertEntry::Leaf((icon, label, tip, act)) => { if menu_item(ui, icon, label).on_hover_text(tip).clicked() { act(self); ui.close(); } }
                    InsertEntry::Sub((icon, label, tip, content)) => { ui.menu_button(menu_job(ui, icon, label), |ui| content(self, ui)).response.on_hover_text(tip); }
                }
            }
        }
        if !found { ui.weak("Nothing matches"); }
        });
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

    /// The Publish choices, shared by the title bar.
    /// Whether Try It applies: slide-based projects only.
    fn can_try(&self) -> bool { self.project.as_ref().is_some_and(|p| !p.is_web()) }

    /// The Preview choices, shared by the title bar.
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
        let Some((li, oi)) = self.view.selected else { return };
        let (a, b) = self.view.text_sel.unwrap_or((0, usize::MAX));
        self.snapshot();
        if let Some(o) = self.project.as_mut().and_then(|p| p.slide_mut(self.view.slide).layers.get_mut(li)).and_then(|l| l.objects.get_mut(oi)) {
            let n = o.text.chars().count();
            o.format_range(a.min(n), b.min(n), &Attr::Link(url));
            let (t, r) = (o.text.clone(), o.rich.clone());
            for st in o.states.iter_mut() { st.text = t.clone(); st.rich = r.clone(); }
        }
    }


    /// The menu that adds a slide, in the slide list and its right-click menu: the choices of the Add Slides tab, with their icons.
    /// `scene` is the scene the slide goes at the end of (`None`: after the slide that is showing).
    pub(crate) fn add_slide_choices(&mut self, ui: &mut egui::Ui, scene: Option<usize>) {
        enum Pick { Kind(SlideKind), Layout(Option<String>), Scene }
        let mut pick: Option<Pick> = None;
        ui.set_min_width(220.0);
        if menu_item(ui, FILE_PLUS, "Slide").on_hover_text("A slide with the layout of the one before it").clicked() { pick = Some(Pick::Kind(SlideKind::Normal)); ui.close(); }
        let layouts: Vec<(String, String)> = self.project.as_ref().map(|p| p.layouts.iter().map(|l| (l.id.clone(), l.name.clone())).collect()).unwrap_or_default();
        if !layouts.is_empty() {
            ui.menu_button(menu_job(ui, FRAME, "Slide from layout"), |ui| {
                for (id, name) in &layouts {
                    if menu_item(ui, FRAME, name).clicked() { pick = Some(Pick::Layout(Some(id.clone()))); ui.close(); }
                }
            });
        }
        ui.menu_button(menu_job(ui, RADIO, "Graded question"), |ui| {
            for (icon, label, tip, make) in GRADED_QUESTIONS {
                if menu_item(ui, icon, label).on_hover_text(tip).clicked() { pick = Some(Pick::Kind(SlideKind::Question(make()))); ui.close(); }
            }
        });
        ui.menu_button(menu_job(ui, SLIDERS, "Survey question"), |ui| {
            for (icon, label, tip, make) in SURVEY_QUESTIONS {
                if menu_item(ui, icon, label).on_hover_text(tip).clicked() { pick = Some(Pick::Kind(SlideKind::Question(make()))); ui.close(); }
            }
        });
        if menu_item(ui, CHECK, "Results").on_hover_text("A quiz results slide").clicked() { pick = Some(Pick::Kind(SlideKind::Results)); ui.close(); }
        ui.separator();
        if menu_item(ui, STACK, "New scene").on_hover_text("A new scene, with a slide in it").clicked() { pick = Some(Pick::Scene); ui.close(); }
        if let Some(p) = pick {
            if let Some(&last) = scene.and_then(|si| self.project.as_ref().map(|pr| pr.scene_slides(si))).as_ref().and_then(|v| v.last()) { self.view.slide = last; }
            match p {
                Pick::Kind(kind) => self.add_slide(kind),
                Pick::Layout(l) => self.add_slide_with_layout(SlideKind::Normal, l),
                Pick::Scene => self.add_scene(),
            }
            self.scene_view = false;
        }
    }





    /// Format the characters selected in the in-place editor (set by the canvas while editing).
    pub(super) fn format_selection(&mut self, attr: Attr) {
        let (Some((a, b)), Some((li, oi))) = (self.view.text_sel, self.view.selected) else { return };
        self.snapshot();
        if let Some(o) = self.project.as_mut().and_then(|p| p.slide_mut(self.view.slide).layers.get_mut(li)).and_then(|l| l.objects.get_mut(oi)) {
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
        let (part, primary) = (self.view.text_sel, self.view.selected);
        self.snapshot();
        let slide = self.view.slide;
        let Some(p) = self.project.as_mut() else { return };
        for (li, oi) in targets {
            let Some(o) = p.slide_mut(slide).layers.get_mut(li).and_then(|l| l.objects.get_mut(oi)) else { continue };
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
        let Some((li, oi)) = self.view.selected else { return };
        self.snapshot();
        let slide = &mut self.project.as_mut().unwrap().slide_mut(self.view.slide);
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
fn effect_pane(ui: &mut egui::Ui, obj: &mut Object, pane: EffectPane) -> bool { effect_pane_in(ui, obj, pane, false) }

/// `effect_pane`; `inline` is for the Properties panel, where it takes the panel's width and has no preview of its own.
fn effect_pane_in(ui: &mut egui::Ui, obj: &mut Object, pane: EffectPane, inline: bool) -> bool {
    let mut changed = false;
    if !inline {
        ui.set_width(if pane == EffectPane::Shadow { 330.0 } else { 280.0 });
        ui.spacing_mut().slider_width = 96.0;
        effects_preview(ui, obj);
        ui.add_space(6.0);
    } else {
        ui.spacing_mut().slider_width = 90.0;
    }
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
    if panel_mode(ui) { return; }
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

/// The object with the look of `proto` (text style, fill, line, effects, corners, line spacing) and everything else its own.
fn styled_like(before: &Object, proto: &Object) -> Object {
    let mut after = before.clone();
    // Bold, italic and the like may sit on the words rather than on the object: take what the first word has.
    let mut style = proto.style.clone();
    if let Some((_, f)) = proto.char_fmts().first() {
        style.bold = f.bold.unwrap_or(style.bold);
        style.italic = f.italic.unwrap_or(style.italic);
        style.underline = f.underline.unwrap_or(style.underline);
        if let Some(c) = f.color { style.color = c; }
        if let Some(z) = f.size { style.size = z; }
        if let Some(n) = &f.font_family { style.font_family = n.clone(); }
    }
    after.style = style;
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
    after
}

/// The controls of a slide's (or layout's) background: none, a color, a gradient you build, or a picture. They are the body of the
/// Background bar in the Properties panel. Returns whether something changed; `want_picture` asks the caller to choose a picture.
pub(crate) fn background_controls(ui: &mut egui::Ui, bg: &mut Option<Background>, theme: &brainbend_doc::Theme, slide_size: (f32, f32), inherits: bool, picture_name: Option<String>, want_picture: &mut bool) -> bool {
    let mut changed = false;
    // The picker takes the width the panel has, so there is no empty strip beside it.
    let w = ui.available_width().clamp(200.0, 720.0);
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
            _ => *want_picture = true,
        }
    }
    ui.add_space(4.0);
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
            if ui.button("Choose another picture…").clicked() { *want_picture = true; }
        }
    }
    if inherits && bg.is_some() && ui.button("Use the layout's background").clicked() { *bg = None; changed = true; }
    ui.ctx().data_mut(|d| d.remove_temp::<f32>(egui::Id::new("color-entry-width")));
    let pass = ui.ctx().cumulative_pass_nr().max(1);
    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("background-bar-seen"), pass));
    changed
}
