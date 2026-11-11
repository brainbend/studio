//! The formatting accordion of the Properties tab, laid out like a game-engine inspector: a bar for each group that opens to
//! rows, each with its name on the left and its control on the right. The controls edit the same data as the ribbon's.
use super::*;

const BAR_H: f32 = 24.0;
const ROW_H: f32 = 22.0;
const BLUE: egui::Color32 = egui::Color32::from_rgb(70, 150, 230);

/// A group's bar. Click it to open the rows below; clicking the open one closes it. By default opening one closes the one that
/// was open (the "Open one section at a time" option). Until something has been chosen, the first bar that asks to start open is the open one.
pub(crate) fn section(ui: &mut egui::Ui, id: &str, title: &str, default_open: bool, body: impl FnOnce(&mut egui::Ui)) {
    // Which list of bars this is (the slide's or an object's): each keeps its own open bar.
    let scope: u8 = ui.ctx().data(|d| d.get_temp(egui::Id::new("format-bar-scope"))).unwrap_or(0);
    let key = egui::Id::new(("format-bar-open", scope));
    let mut current: Option<Vec<String>> = ui.ctx().data(|d| d.get_temp(key));
    // Expand all / Collapse all: asked for in one pass, answered by every bar drawn in it or the next.
    let ask: Option<(u64, bool)> = ui.ctx().data(|d| d.get_temp(egui::Id::new("format-bar-all")));
    if let Some((pass, expand)) = ask {
        let now = ui.ctx().cumulative_pass_nr();
        if now >= pass && now <= pass + 1 {
            let mut ids = current.take().unwrap_or_default();
            ids.retain(|o| o != id);
            if expand { ids.push(id.to_string()); }
            ui.ctx().data_mut(|d| d.insert_temp(key, ids.clone()));
            current = Some(ids);
        }
    }
    if current.is_none() && default_open {
        current = Some(vec![id.to_string()]);
        ui.ctx().data_mut(|d| d.insert_temp(key, current.clone().unwrap()));
    }
    let open = current.as_ref().is_some_and(|o| o.iter().any(|x| x == id));
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), BAR_H), egui::Sense::click());
    let dark = ui.visuals().dark_mode;
    let base = if dark { egui::Color32::from_gray(40) } else { egui::Color32::from_gray(228) };
    let fill = if resp.hovered() { if dark { egui::Color32::from_gray(50) } else { egui::Color32::from_gray(218) } } else { base };
    ui.painter().rect_filled(rect, 0.0, fill);
    let line = ui.visuals().text_color().gamma_multiply(0.20);
    ui.painter().line_segment([rect.left_top(), rect.right_top()], egui::Stroke::new(1.0, line));
    ui.painter().line_segment([rect.left_bottom(), rect.right_bottom()], egui::Stroke::new(1.0, line));
    // How far open this bar is, 0 to 1: it eases over, so one bar closes while the next opens.
    let t = ui.ctx().animate_bool_with_time_and_easing(egui::Id::new(("format-bar-anim", id)), open, 0.16, egui::emath::easing::cubic_out);
    // The arrow turns from pointing right to pointing down.
    let c = rect.left_center() + egui::vec2(11.0, 0.0);
    let col = ui.visuals().text_color().gamma_multiply(0.7);
    let turn = egui::emath::Rot2::from_angle(t * std::f32::consts::FRAC_PI_2);
    let pts: Vec<egui::Pos2> = [egui::vec2(-2.5, -4.0), egui::vec2(-2.5, 4.0), egui::vec2(3.0, 0.0)].iter().map(|v| c + turn * *v).collect();
    ui.painter().add(egui::Shape::convex_polygon(pts, col, egui::Stroke::NONE));
    ui.painter().text(egui::pos2(rect.left() + 24.0, rect.center().y), egui::Align2::LEFT_CENTER, title, egui::FontId::proportional(13.0), ui.visuals().strong_text_color());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, title));
    if resp.clicked() {
        let one_at_a_time = ui.ctx().data(|d| d.get_temp(egui::Id::new("format-one-open"))).unwrap_or(true);
        let mut ids = current.unwrap_or_default();
        if open { ids.retain(|o| o != id); } else { if one_at_a_time { ids.clear(); } ids.push(id.to_string()); }
        ui.ctx().data_mut(|d| d.insert_temp(key, ids));
    }
    let height_key = egui::Id::new(("format-bar-height", id));
    let frame = || egui::Frame::new().inner_margin(egui::Margin { left: 10, right: 8, top: 6, bottom: 8 });
    if t >= 1.0 {
        let shown = frame().show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 5.0);
            body(ui);
        });
        ui.ctx().data_mut(|d| d.insert_temp(height_key, shown.response.rect.height()));
    } else if t > 0.0 {
        // Opening or closing: the rows are drawn at their full size, but only the part that has slid into view shows.
        let top = ui.cursor().min;
        let width = ui.available_width();
        let known: f32 = ui.ctx().data(|d| d.get_temp(height_key)).unwrap_or(0.0);
        let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(top, egui::vec2(width, 4000.0))));
        let visible = if known > 0.0 { known * t } else { 400.0 * t };
        inner.set_clip_rect(egui::Rect::from_min_size(top, egui::vec2(width, visible)).intersect(ui.clip_rect()));
        frame().show(&mut inner, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 5.0);
            body(ui);
        });
        let full = inner.min_rect().height();
        ui.ctx().data_mut(|d| d.insert_temp(height_key, full));
        ui.allocate_space(egui::vec2(width, full * t));
    }
}

/// Copy Style and Paste Style for the heading row (in a right-to-left layout, after the expand buttons). Returns (copy, paste).
pub(crate) fn style_buttons(ui: &mut egui::Ui, can_paste: bool) -> (bool, bool) {
    // Right-to-left: Paste is added first, so Copy ends up on its left.
    let paste = ui.add_enabled_ui(can_paste, |ui| icon_button(ui, '\u{e196}', false, "Paste Style")).inner.clicked();
    let copy = icon_button(ui, PAINTER, false, "Copy Style").clicked();
    (copy, paste)
}

/// A small square button holding a solid triangle, pointing down or up.
fn triangle_button(ui: &mut egui::Ui, down: bool, tip: &str) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::click());
    if resp.hovered() { ui.painter().rect_filled(rect, 4.0, ui.visuals().widgets.hovered.weak_bg_fill); }
    let c = rect.center();
    let d = if down { 1.0 } else { -1.0 };
    let pts = vec![c + egui::vec2(-4.5, -2.5 * d), c + egui::vec2(4.5, -2.5 * d), c + egui::vec2(0.0, 3.0 * d)];
    ui.painter().add(egui::Shape::convex_polygon(pts, ui.visuals().text_color().gamma_multiply(0.8), egui::Stroke::NONE));
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tip));
    resp.on_hover_text(tip)
}

/// Two small triangle buttons, Expand all (down) and Collapse all (up), for the right end of a panel's heading row (put them in a
/// right-to-left layout, before anything else, so they stay at the far right).
pub(crate) fn expand_buttons(ui: &mut egui::Ui) {
    let mut ask = None;
    // Right-to-left, so Collapse all is added first and ends up on the right.
    if triangle_button(ui, false, "Collapse all").clicked() { ask = Some(false); }
    if triangle_button(ui, true, "Expand all").clicked() { ask = Some(true); }
    if let Some(expand) = ask {
        let pass = ui.ctx().cumulative_pass_nr();
        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("format-bar-all"), (pass, expand)));
        ui.ctx().request_repaint();
    }
}

/// One setting: its name on the left, its control filling the rest of the row.
pub(crate) fn row(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui, f32)) {
    let total = ui.available_width();
    let label_w = (total * 0.36).clamp(60.0, 104.0);
    let ctrl_w = (total - label_w - ui.spacing().item_spacing.x).max(40.0);
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(egui::vec2(label_w, ROW_H), egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.set_min_size(egui::vec2(label_w, ROW_H));
            ui.add(egui::Label::new(egui::RichText::new(label).color(ui.visuals().text_color().gamma_multiply(0.85))).truncate());
        });
        ui.allocate_ui_with_layout(egui::vec2(ctrl_w, ROW_H), egui::Layout::left_to_right(egui::Align::Center), |ui| { ui.set_min_size(egui::vec2(ctrl_w, ROW_H)); add(ui, ctrl_w) });
    });
}

/// A bar of the color, as wide as the row; it opens the same color choices as the ribbon.
fn color_field(ui: &mut egui::Ui, w: f32, cur: &mut Option<Rgba>, none_label: Option<&str>) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, ROW_H), egui::Sense::click());
    match cur {
        Some(c) => {
            ui.painter().rect_filled(rect, 3.0, color(*c));
            // The color's code, in black or white, whichever reads on it, so even a near-black color can be told from the panel.
            let lum = 0.299 * c.0 as f32 + 0.587 * c.1 as f32 + 0.114 * c.2 as f32;
            let ink = if lum > 140.0 { egui::Color32::from_gray(20) } else { egui::Color32::WHITE };
            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, format!("#{:02X}{:02X}{:02X}", c.0, c.1, c.2), egui::FontId::proportional(11.5), ink);
        }
        None => {
            ui.painter().rect_filled(rect, 3.0, ui.visuals().extreme_bg_color);
            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, none_label.unwrap_or("None"), egui::FontId::proportional(12.0), ui.visuals().weak_text_color());
        }
    }
    ui.painter().rect_stroke(rect, 3.0, egui::Stroke::new(1.0, ui.visuals().text_color().gamma_multiply(0.6)), egui::StrokeKind::Inside);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Color"));
    let mut changed = false;
    egui::Popup::from_toggle_button_response(&resp).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
        changed = color_popup(ui, cur, none_label);
    });
    changed
}

/// A drop-down as wide as the row: returns the choice made, if any.
fn pick<T: PartialEq + Copy>(ui: &mut egui::Ui, id: &str, w: f32, shown: &str, options: &[(T, &str)], cur: Option<T>) -> Option<T> {
    let mut picked = None;
    egui::ComboBox::from_id_salt(id).width(w - 8.0).selected_text(shown).show_ui(ui, |ui| {
        for (v, l) in options {
            if ui.selectable_label(cur == Some(*v), *l).clicked() { picked = Some(*v); }
        }
    });
    picked
}

fn number(ui: &mut egui::Ui, w: f32, v: &mut f32, range: std::ops::RangeInclusive<f32>, suffix: &str) -> bool {
    ui.add_sized([w, ROW_H], egui::DragValue::new(v).range(range).suffix(suffix)).changed()
}

impl App {
    /// The formatting bars for the selected object (the panel's heading and Copy Style button are above them).
    pub(crate) fn format_accordion(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.ctx().data_mut(|d| { d.insert_temp(egui::Id::new("format-bar-scope"), 0u8); d.insert_temp(egui::Id::new("panel-compact"), true); });
        let primary = self.ribbon_target();
        let Some(_) = primary else { ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("panel-compact"), false)); return };
        let mut edited = primary.clone();
        let obj = edited.as_mut().unwrap();
        let mut changed = false;
        let is_media = matches!(obj.kind, Kind::Image | Kind::Video | Kind::Audio | Kind::Web(_) | Kind::Pano(_) | Kind::Unsupported(_));
        let is_line = matches!(obj.kind, Kind::Line | Kind::LineUp);
        let has_text = !is_media && !is_line && !matches!(obj.kind, Kind::Path(_) | Kind::Hotspot);
        let draws = !matches!(obj.kind, Kind::Video | Kind::Audio | Kind::Web(_) | Kind::Pano(_) | Kind::Unsupported(_) | Kind::Hotspot);
        // What belongs to this kind of object comes first (a picture's crop, a table's rows, a button's styles).
        if self.format_title().is_some() {
            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("panel-groups"), true));
            self.format_groups(ui, obj, &mut changed);
            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("panel-groups"), false));
        }
        if has_text {
            self.panel_text(ui, obj, &mut changed);
            self.panel_paragraph(ui, obj, &mut changed);
        }
        if draws { panel_fill_and_line(ui, obj, &mut changed); }
        if draws && !is_line { panel_effects(ui, obj, &mut changed); }
        self.panel_arrange(ui);
        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("panel-compact"), false));
        if let (true, Some(before), Some(after)) = (changed, primary.as_ref(), edited.as_ref()) {
            self.apply_object_edit(ctx, before, after);
        }
    }

    fn panel_text(&mut self, ui: &mut egui::Ui, obj: &mut Object, changed: &mut bool) {
        let open = matches!(obj.kind, Kind::Text);
        section(ui, "text", "Text Formatting", open, |ui| {
            row(ui, "Style", |ui, _| self.text_styles_menu(ui));
            row(ui, "Font", |ui, w| { *changed |= crate::font_store::font_menu(ui, "pf-font", &mut obj.style.font_family, w - 8.0); });
            row(ui, "Size", |ui, w| {
                *changed |= number(ui, (w - 92.0).max(40.0), &mut obj.style.size, 4.0..=400.0, " pt");
                let sizes = ui.add_sized([22.0, ROW_H], egui::Button::new("▾")).on_hover_text("Common sizes");
                egui::Popup::from_toggle_button_response(&sizes).close_behavior(egui::PopupCloseBehavior::CloseOnClick).show(|ui| {
                    for z in [18.0, 24.0, 28.0, 32.0, 36.0, 40.0, 48.0, 60.0, 72.0, 96.0, 120.0] {
                        if ui.selectable_label(obj.style.size == z, format!("{z:.0}")).clicked() { obj.style.size = z; *changed = true; }
                    }
                });
                let cur = self.text_sel.and_then(|(a, _)| obj.char_fmts().get(a).and_then(|(_, f)| f.size)).unwrap_or(obj.style.size);
                for (up, tip) in [(true, "Grow font"), (false, "Shrink font")] {
                    if compact_text_button(ui, grow_job(ui, up), tip).clicked() {
                        let next = step_size(cur, up);
                        self.edit_text_range(move |o, a, b, whole| { if whole { o.style.size = next; } o.format_range(a, b, &Attr::Size(next)); });
                    }
                }
            });
            row(ui, "Color", |ui, w| {
                let mut c = Some(obj.style.color);
                if color_field(ui, w, &mut c, None) { if let Some(c) = c { obj.style.color = c; *changed = true; } }
            });
            // While text is selected on the slide these format just the selection.
            let part = self.text_sel;
            let (ra, rb) = part.unwrap_or((0, usize::MAX));
            let span: Vec<Fmt> = obj.char_fmts().into_iter().enumerate().filter(|(i, _)| *i >= ra && *i < rb).map(|(_, (_, f))| f).collect();
            let all_strike = !span.is_empty() && span.iter().all(|f| f.strike.unwrap_or(false));
            row(ui, "Highlight", |ui, w| {
                let mut hl = span.first().and_then(|f| f.highlight);
                if color_field(ui, w, &mut hl, Some("No highlight")) { self.edit_text_range(move |o, a, b, _| o.format_range(a, b, &Attr::Highlight(hl))); }
            });
            row(ui, "Emphasis", |ui, _| {
                if icon_button(ui, BOLD, obj.style.bold, "Bold (⌘B)").clicked() {
                    if part.is_some() { self.format_selection(Attr::ToggleBold); } else { obj.style.bold = !obj.style.bold; *changed = true; }
                }
                if icon_button(ui, ITALIC, obj.style.italic, "Italic (⌘I)").clicked() {
                    if part.is_some() { self.format_selection(Attr::ToggleItalic); } else { obj.style.italic = !obj.style.italic; *changed = true; }
                }
                if icon_button(ui, UNDERLINE, obj.style.underline, "Underline (⌘U)").clicked() {
                    if part.is_some() { self.format_selection(Attr::ToggleUnderline); } else { obj.style.underline = !obj.style.underline; *changed = true; }
                }
                if icon_button(ui, STRIKE, all_strike, "Strikethrough").clicked() { self.edit_text_range(|o, a, b, _| o.format_range(a, b, &Attr::ToggleStrike)); }
            });
            row(ui, "Link", |ui, w| {
                let can = !obj.text.is_empty() && !matches!(obj.kind, Kind::Input(_) | Kind::Slider(_));
                // The address of the chosen words, or of the first linked words when none are chosen.
                let cur: String = span.iter().find_map(|f| f.link.clone()).unwrap_or_default();
                let key = egui::Id::new(("pf-link", obj.id.clone()));
                let (seen, mut text): (String, String) = ui.ctx().data(|d| d.get_temp(key)).unwrap_or((String::from("\u{0}"), String::new()));
                if seen != cur { text = cur.clone(); }
                let mut apply = false;
                ui.add_enabled_ui(can, |ui| {
                    let clear_w = if cur.is_empty() { 0.0 } else { 26.0 };
                    let r = ui.add_sized([(w - 8.0 - clear_w).max(40.0), ROW_H], egui::TextEdit::singleline(&mut text).hint_text("https://example.com")).on_hover_text("Where the chosen words (or all the text) link to. Press Enter to apply.");
                    if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) { apply = true; }
                    if !cur.is_empty() && icon_button(ui, '\u{e4f6}', false, "Remove link").clicked() { text.clear(); apply = true; }
                }).response.on_disabled_hover_text("This object has no text to link");
                if apply {
                    let t = text.trim().to_string();
                    if t.is_empty() { self.link_selected(None); }
                    else if t.starts_with("https://") || t.starts_with("http://") || t.starts_with("mailto:") { self.link_selected(Some(t)); }
                    else { self.status = "A link starts with https://, http:// or mailto:".into(); }
                }
                ui.ctx().data_mut(|d| d.insert_temp(key, (cur, text)));
            });
            row(ui, "Spacing", |ui, w| {
                let cur = span.first().and_then(|f| f.spacing).unwrap_or(0.0);
                let opts = [(-2.0f32, "Very tight"), (-1.0, "Tight"), (0.0, "Normal"), (2.0, "Loose"), (4.0, "Very loose")];
                let shown = opts.iter().find(|(v, _)| (cur - v).abs() < 0.01).map(|o| o.1).unwrap_or("Custom");
                if let Some(v) = pick(ui, "pf-spacing", w, shown, &opts, opts.iter().find(|(v, _)| (cur - v).abs() < 0.01).map(|o| o.0)) { self.edit_text_range(move |o, a, b, _| o.format_range(a, b, &Attr::Spacing(v))); }
            });
            row(ui, "Case", |ui, w| {
                use brainbend_doc::richtext::Case;
                let opts = [(0u8, "Sentence case."), (1, "lowercase"), (2, "UPPERCASE"), (3, "Capitalize Each Word"), (4, "tOGGLE cASE")];
                if let Some(v) = pick(ui, "pf-case", w, "Change case", &opts, None) {
                    let c = [Case::Sentence, Case::Lower, Case::Upper, Case::Capitalize, Case::Toggle][v as usize];
                    self.edit_text_range(move |o, a, b, _| o.change_case(a, b, c));
                }
            });
            if ui.add_sized([ui.available_width(), ROW_H], egui::Button::new("Clear Formatting")).on_hover_text("Back to the object's plain text style").clicked() {
                self.edit_text_range(|o, a, b, whole| {
                    o.format_range(a, b, &Attr::Clear);
                    if whole { o.style.bold = false; o.style.italic = false; o.style.underline = false; }
                });
            }
        });
    }

    fn panel_paragraph(&mut self, ui: &mut egui::Ui, obj: &mut Object, changed: &mut bool) {
        let mut list_click: Option<ListStyle> = None;
        section(ui, "paragraph", "Paragraph", false, |ui| {
            row(ui, "Align", |ui, _| {
                for (a, icon, tip) in [(Align::Left, ALIGN_L, "Align left"), (Align::Center, ALIGN_C, "Center"), (Align::Right, ALIGN_R, "Align right"), (Align::Justify, JUSTIFY, "Justify")] {
                    if icon_button(ui, icon, obj.style.align == a, tip).clicked() { obj.style.align = a; *changed = true; }
                }
            });
            row(ui, "List", |ui, _| {
                let ls = obj.list_style();
                if icon_button(ui, LIST_BULLETS, ls == ListStyle::Bullet, "Bullets").clicked() { list_click = Some(if ls == ListStyle::Bullet { ListStyle::None } else { ListStyle::Bullet }); }
                if icon_button(ui, LIST_NUM, ls == ListStyle::Number, "Numbering").clicked() { list_click = Some(if ls == ListStyle::Number { ListStyle::None } else { ListStyle::Number }); }
                if icon_button(ui, INDENT_OUT, false, "Decrease indent").clicked() { self.edit_text_range(|o, a, b, _| o.format_range(a, b, &Attr::Indent(-1))); }
                if icon_button(ui, INDENT_IN, false, "Increase indent").clicked() { self.edit_text_range(|o, a, b, _| o.format_range(a, b, &Attr::Indent(1))); }
            });
            row(ui, "Line spacing", |ui, w| {
                let opts = [(0.0f32, "Default"), (1.2, "1.0"), (1.38, "1.15"), (1.8, "1.5"), (2.4, "2.0"), (3.0, "2.5")];
                let hit = opts.iter().find(|(v, _)| (obj.line_spacing - v).abs() < 0.01);
                if let Some(v) = pick(ui, "pf-linespace", w, hit.map(|o| o.1).unwrap_or("Custom"), &opts, hit.map(|o| o.0)) { obj.line_spacing = v; *changed = true; }
            });
            row(ui, "Vertical", |ui, w| {
                let opts = [(VAlign::Top, "Top"), (VAlign::Middle, "Middle"), (VAlign::Bottom, "Bottom")];
                let shown = opts.iter().find(|o| o.0 == obj.style.valign).map(|o| o.1).unwrap_or("Top");
                if let Some(v) = pick(ui, "pf-valign", w, shown, &opts, Some(obj.style.valign)) { obj.style.valign = v; *changed = true; }
            });
            row(ui, "Direction", |ui, w| {
                let opts = [(0i16, "Horizontal"), (90, "Rotate all text 90°"), (270, "Rotate all text 270°")];
                let shown = opts.iter().find(|o| o.0 == obj.text_rotation).map(|o| o.1).unwrap_or("Horizontal");
                if let Some(v) = pick(ui, "pf-direction", w, shown, &opts, Some(obj.text_rotation)) { obj.text_rotation = v; *changed = true; }
            });
        });
        if let Some(st) = list_click { self.set_list_selected(st); }
    }

    fn panel_arrange(&mut self, ui: &mut egui::Ui) {
        let has_sel = self.selected.is_some();
        let n_sel = self.selection().len();
        section(ui, "arrange", "Arrange", false, |ui| {
            ui.add_enabled_ui(has_sel, |ui| {
                row(ui, "Order", |ui, _| {
                    if icon_button(ui, FRONT, false, "Bring to front").clicked() { self.reorder_selected(true); }
                    if icon_button(ui, BACK, false, "Send to back").clicked() { self.reorder_selected(false); }
                });
                row(ui, "Group", |ui, _| {
                    ui.add_enabled_ui(n_sel > 1, |ui| { if icon_button(ui, STACK, false, "Group").clicked() { self.group_selected(); } });
                    if ui.button("Ungroup").clicked() { self.ungroup_selected(); }
                });
                let to_slide = n_sel < 2;
                row(ui, if to_slide { "Align to slide" } else { "Align to each other" }, |ui, _| {
                    for (icon, axis, mode, label) in [(AL_LEFT, 0u8, 0u8, "Left"), (AL_CENTER_H, 0, 1, "Center"), (AL_RIGHT, 0, 2, "Right")] {
                        if icon_button(ui, icon, false, label).clicked() { self.align_selected_or_slide(axis, mode); }
                    }
                });
                row(ui, "", |ui, _| {
                    for (icon, axis, mode, label) in [(AL_TOP, 1u8, 0u8, "Top"), (AL_CENTER_V, 1, 1, "Middle"), (AL_BOTTOM, 1, 2, "Bottom")] {
                        if icon_button(ui, icon, false, label).clicked() { self.align_selected_or_slide(axis, mode); }
                    }
                });
                ui.add_enabled_ui(n_sel > 2, |ui| {
                    row(ui, "Distribute", |ui, _| {
                        if icon_button(ui, DIST_H, false, "Distribute horizontally").clicked() { self.align_selected(0, 3); }
                        if icon_button(ui, DIST_V, false, "Distribute vertically").clicked() { self.align_selected(1, 3); }
                    });
                });
            });
        });
    }
}

fn panel_fill_and_line(ui: &mut egui::Ui, obj: &mut Object, changed: &mut bool) {
    let is_pic = matches!(obj.kind, Kind::Image);
    let is_line = matches!(obj.kind, Kind::Line | Kind::LineUp);
    section(ui, "fill-line", "Fill and Line", true, |ui| {
        if !is_line && !is_pic { row(ui, "Style", |ui, _| quick_styles(ui, obj, changed)); }
        if !is_line && !is_pic { row(ui, "Fill", |ui, w| { *changed |= color_field(ui, w, &mut obj.fill, Some("No fill")); }); }
        row(ui, if is_line { "Line" } else { "Outline" }, |ui, w| {
            let mut c = obj.stroke.map(|s| s.color);
            if color_field(ui, w, &mut c, Some("No outline")) {
                obj.stroke = c.map(|c| Stroke { color: c, width: obj.stroke.map(|s| s.width).unwrap_or(2.0) });
                *changed = true;
            }
        });
        if let Some(st) = obj.stroke.as_mut() {
            row(ui, "Thickness", |ui, w| { *changed |= number(ui, w, &mut st.width, 0.5..=40.0, " px"); });
            row(ui, "Pattern", |ui, w| {
                let opts = [(LineDash::Solid, "Solid"), (LineDash::Dotted, "Dotted"), (LineDash::Dashed, "Dashed")];
                let shown = opts.iter().find(|o| o.0 == obj.dash).map(|o| o.1).unwrap_or("Solid");
                if let Some(d) = pick(ui, "pf-dash", w, shown, &opts, Some(obj.dash)) { obj.dash = d; *changed = true; }
            });
        }
    });
}

fn panel_effects(ui: &mut egui::Ui, obj: &mut Object, changed: &mut bool) {
    let is_pic = matches!(obj.kind, Kind::Image);
    section(ui, "effects", "Effects", false, |ui| {
        ui.label(egui::RichText::new("Presets").strong());
        *changed |= effect_pane_in(ui, obj, EffectPane::Presets, true);
        for pane in [EffectPane::Shadow, EffectPane::Glow, EffectPane::Reflection, EffectPane::Soft, EffectPane::Corners] {
            if pane == EffectPane::Corners && is_pic { continue; }
            if pane == EffectPane::Corners { ui.label(egui::RichText::new("Corners").strong()); }
            *changed |= effect_pane_in(ui, obj, pane, true);
        }
    });
}
