//! The Web Lesson editor: an outline of sections and blocks, a preview of the lesson, and
//! properties for the selected block. Publishing and the player live in `crates/publish-html`.
use super::*;
use brainbend_doc::lesson::*;

/// Which file a block's properties are asking the app to pick.
#[derive(Clone, Copy, PartialEq)]
enum LessonAsset {
    Image,
    Video,
    Captions,
    Audio,
    CoverImage,
    QuestionImage,
    QuestionAudio,
}

/// Wrap characters `sel` of `text` in `pre`/`post` (or insert both at the cursor when nothing is selected).
pub(crate) fn wrap_selection(text: &str, sel: Option<(usize, usize)>, pre: &str, post: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let (a, b) = sel.map(|(a, b)| (a.min(chars.len()), b.min(chars.len()))).unwrap_or((chars.len(), chars.len()));
    let mid: String = chars[a..b].iter().collect();
    format!("{}{pre}{mid}{post}{}", chars[..a].iter().collect::<String>(), chars[b..].iter().collect::<String>())
}

/// Start every line touched by `sel` with `- `; with no selection, add a new bullet line at the end.
pub(crate) fn bulletize(text: &str, sel: Option<(usize, usize)>) -> String {
    let chars: Vec<char> = text.chars().collect();
    let Some((a, b)) = sel.filter(|(a, b)| a != b) else {
        return format!("{}{}- ", text, if text.is_empty() || text.ends_with('\n') { "" } else { "\n" });
    };
    let (a, b) = (a.min(chars.len()), b.min(chars.len()));
    let line_start = chars[..a].iter().rposition(|c| *c == '\n').map(|p| p + 1).unwrap_or(0);
    let mut out: String = chars[..line_start].iter().collect();
    let region: String = chars[line_start..b].iter().collect();
    out.push_str(&region.split('\n').map(|l| if l.trim().is_empty() || l.trim_start().starts_with("- ") { l.to_string() } else { format!("- {l}") }).collect::<Vec<_>>().join("\n"));
    out.push_str(&chars[b..].iter().collect::<String>());
    out
}

/// Draw one line of spans as wrapped text.
fn spans_label(ui: &mut egui::Ui, spans: &[Span], size: f32, prefix: &str) {
    let color = ui.visuals().text_color();
    let accent = ui.visuals().hyperlink_color;
    let mut job = egui::text::LayoutJob::default();
    let fmt = |bold: bool, italic: bool, col: egui::Color32, link: bool| egui::text::TextFormat {
        font_id: egui::FontId::proportional(size),
        color: col,
        italics: italic,
        underline: if link { egui::Stroke::new(1.0, col) } else { egui::Stroke::NONE },
        coords: egui::epaint::text::VariationCoords::new([(b"wght", if bold { 700.0 } else { 400.0 })]),
        ..Default::default()
    };
    if !prefix.is_empty() { job.append(prefix, 0.0, fmt(false, false, color, false)); }
    for s in spans {
        match s {
            Span::Text(t) => job.append(t, 0.0, fmt(false, false, color, false)),
            Span::Bold(t) => job.append(t, 0.0, fmt(true, false, color, false)),
            Span::Italic(t) => job.append(t, 0.0, fmt(false, true, color, false)),
            Span::Link(t, _) => job.append(t, 0.0, fmt(false, false, accent, true)),
        }
    }
    job.wrap.max_width = ui.available_width();
    ui.label(job);
}

fn show_markdown(ui: &mut egui::Ui, text: &str, size: f32) {
    for blk in parse_markdown(text) {
        match blk {
            MdBlock::Paragraph(lines) => { for l in &lines { spans_label(ui, l, size, ""); } }
            MdBlock::Bullets(items) => { for i in &items { spans_label(ui, i, size, "•  "); } }
            MdBlock::Numbers(items) => { for (n, i) in items.iter().enumerate() { spans_label(ui, i, size, &format!("{}.  ", n + 1)); } }
        }
        ui.add_space(4.0);
    }
}

fn short(text: &str, n: usize) -> String {
    let one: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if one.chars().count() > n { format!("{}…", one.chars().take(n).collect::<String>()) } else { one }
}

impl App {
    fn lesson_ref(&self) -> &WebLesson {
        self.project.as_ref().and_then(|p| p.lesson.as_ref()).expect("a Web Lesson project has lesson content")
    }

    fn lesson_mut(&mut self) -> &mut WebLesson {
        self.project.as_mut().and_then(|p| p.lesson.as_mut()).expect("a Web Lesson project has lesson content")
    }

    /// Keep the selection pointing at something that exists.
    fn clamp_lesson_selection(&mut self) {
        let (n, blocks) = { let l = self.lesson_ref(); (l.sections.len(), l.sections.get(self.sec.min(l.sections.len().saturating_sub(1))).map(|s| s.blocks.len()).unwrap_or(0)) };
        self.sec = self.sec.min(n.saturating_sub(1));
        if self.blk.is_some_and(|b| b >= blocks) { self.blk = None; }
    }

    /// Insert a block after the selected one (or at the end of the section) and select it.
    pub(super) fn add_block(&mut self, kind: BlockKind) {
        self.snapshot();
        let id = self.new_id("block");
        let sec = self.sec;
        let blk = self.blk;
        let l = self.lesson_mut();
        let Some(s) = l.sections.get_mut(sec) else { return };
        let at = blk.map(|b| (b + 1).min(s.blocks.len())).unwrap_or(s.blocks.len());
        s.blocks.insert(at, Block { id, kind });
        self.blk = Some(at);
    }

    fn move_block(&mut self, delta: i32) {
        let (sec, Some(b)) = (self.sec, self.blk) else { return };
        let len = self.lesson_ref().sections[sec].blocks.len();
        let to = b as i32 + delta;
        if to < 0 || to as usize >= len { return; }
        self.snapshot();
        self.lesson_mut().sections[sec].blocks.swap(b, to as usize);
        self.blk = Some(to as usize);
    }

    fn duplicate_block(&mut self) {
        let (sec, Some(b)) = (self.sec, self.blk) else { return };
        self.snapshot();
        let id = self.new_id("block");
        let l = self.lesson_mut();
        let mut copy = l.sections[sec].blocks[b].clone();
        copy.id = id;
        l.sections[sec].blocks.insert(b + 1, copy);
        self.blk = Some(b + 1);
    }

    fn delete_block(&mut self) {
        let (sec, Some(b)) = (self.sec, self.blk) else { return };
        self.snapshot();
        self.lesson_mut().sections[sec].blocks.remove(b);
        self.blk = None;
    }

    pub(super) fn add_section(&mut self) {
        self.snapshot();
        let id = self.new_id("section");
        let sec = self.sec;
        let l = self.lesson_mut();
        let n = l.sections.len() + 1;
        let at = (sec + 1).min(l.sections.len());
        l.sections.insert(at, Section { id, title: format!("Section {n}"), summary: String::new(), blocks: vec![] });
        self.sec = at;
        self.blk = None;
    }

    fn move_section(&mut self, delta: i32) {
        let len = self.lesson_ref().sections.len();
        let to = self.sec as i32 + delta;
        if to < 0 || to as usize >= len { return; }
        self.snapshot();
        let sec = self.sec;
        self.lesson_mut().sections.swap(sec, to as usize);
        self.sec = to as usize;
    }

    pub(super) fn delete_section(&mut self) {
        if self.lesson_ref().sections.len() < 2 { return; }
        self.snapshot();
        let sec = self.sec;
        self.lesson_mut().sections.remove(sec);
        self.sec = sec.saturating_sub(1);
        self.blk = None;
    }

    /// The tool strip above a Web Lesson: add and arrange blocks, search, check and settings.
    pub(super) fn lesson_toolbar(&mut self, root: &mut egui::Ui, _ctx: &egui::Context) {
        let bg = root.visuals().panel_fill;
        egui::Panel::top("lessonbar").frame(egui::Frame::new().fill(bg).inner_margin(egui::Margin::symmetric(10, 6))).show_separator_line(true).show(root, |ui| {
            // Preview and Publish stay at the right; the editing controls wrap in the space that is left.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                if ui.button("Publish").clicked() { self.publish_open = true; }
                ui.menu_button("Preview", |ui| self.preview_items(ui));
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.menu_button("+ Add block", |ui| {
                    for (group, kinds) in BlockKind::catalog() {
                        ui.weak(group);
                        for k in kinds {
                            if ui.button(k.label()).clicked() { self.add_block(k); ui.close(); }
                        }
                        ui.separator();
                    }
                });
                let has = self.blk.is_some();
                if ui.add_enabled(has, egui::Button::new("Move up")).clicked() { self.move_block(-1); }
                if ui.add_enabled(has, egui::Button::new("Move down")).clicked() { self.move_block(1); }
                if ui.add_enabled(has, egui::Button::new("Duplicate")).clicked() { self.duplicate_block(); }
                if ui.add_enabled(has, egui::Button::new("Delete")).clicked() { self.delete_block(); }
                ui.separator();
                if ui.button("+ Section").clicked() { self.add_section(); }
                ui.separator();
                if ui.button("Find").on_hover_text("Find and replace (⌘F)").clicked() { self.find.get_or_insert_with(Default::default).focus = true; }
                if ui.button("Spelling").on_hover_text("Check spelling (F7)").clicked() { self.open_spelling(); }
                if ui.button("Check").on_hover_text("Check the lesson for problems").clicked() { self.check_results = self.project.as_ref().map(|p| p.check()); }
                if ui.button("Lesson settings").clicked() { self.lesson_settings = true; }
            });
                });
            });
        });
    }

    /// The whole Web Lesson editor: outline, preview and properties.
    pub(super) fn lesson_ui(&mut self, root: &mut egui::Ui, ctx: &egui::Context) {
        self.clamp_lesson_selection();

        // ---- outline ----
        egui::Panel::left("lesson-outline").default_size(250.0).show(root, |ui| {
            ui.horizontal(|ui| {
                ui.strong("Outline");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("+ Section").clicked() { self.add_section(); }
                });
            });
            ui.separator();
            let lesson = self.lesson_ref().clone();
            let mut pick: Option<(usize, Option<usize>)> = None;
            egui::ScrollArea::vertical().id_salt("outline-scroll").show(ui, |ui| {
                for (si, s) in lesson.sections.iter().enumerate() {
                    let title = if s.title.trim().is_empty() { "(untitled section)".to_string() } else { s.title.clone() };
                    if ui.selectable_label(self.sec == si && self.blk.is_none(), egui::RichText::new(format!("{}. {}", si + 1, title)).strong()).clicked() { pick = Some((si, None)); }
                    for (bi, b) in s.blocks.iter().enumerate() {
                        let first = b.kind.texts().into_iter().find(|t| !t.trim().is_empty()).unwrap_or_default();
                        let label = if first.is_empty() { b.kind.label().to_string() } else { format!("{}: {}", b.kind.label(), short(&first, 26)) };
                        ui.horizontal(|ui| {
                            ui.add_space(14.0);
                            if ui.selectable_label(self.sec == si && self.blk == Some(bi), label).clicked() { pick = Some((si, Some(bi))); }
                        });
                    }
                    ui.add_space(6.0);
                }
            });
            if let Some((s, b)) = pick { self.sec = s; self.blk = b; }
        });

        // ---- properties ----
        egui::Panel::right("lesson-props").default_size(330.0).show(root, |ui| {
            self.lesson_properties(ui, ctx);
        });

        // ---- preview ----
        egui::CentralPanel::default().show(root, |ui| self.lesson_preview(ui, ctx));
        self.lesson_settings_window(ctx);
    }

    /// A close-enough picture of the published lesson; the real page is one click away in Preview.
    fn lesson_preview(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let lesson = self.lesson_ref().clone();
        let (name, theme) = { let p = self.project.as_ref().unwrap(); (p.name.clone(), p.theme.clone()) };
        let accent = { let c = theme.color(4); egui::Color32::from_rgb(c.0, c.1, c.2) };
        let mut pick: Option<(usize, Option<usize>)> = None;
        egui::ScrollArea::vertical().id_salt("lesson-preview").auto_shrink([false, false]).show(ui, |ui| {
            let w = ui.available_width().min(760.0);
            let pad = ((ui.available_width() - w) / 2.0).max(0.0);
            ui.horizontal_top(|ui| {
                ui.add_space(pad);
                ui.vertical(|ui| {
                    ui.set_width(w);
                    ui.add_space(12.0);
                    if let Some(c) = &lesson.cover {
                        egui::Frame::new().fill({ let d = theme.color(2); egui::Color32::from_rgb(d.0, d.1, d.2) }).corner_radius(12).inner_margin(24).show(ui, |ui| {
                            ui.set_width(w - 48.0);
                            let on = egui::Color32::WHITE;
                            ui.label(egui::RichText::new(&name).size(30.0).strong().color(on));
                            if !c.subtitle.is_empty() { ui.label(egui::RichText::new(&c.subtitle).size(16.0).color(on)); }
                            if !c.description.is_empty() { ui.label(egui::RichText::new(&c.description).size(14.0).color(on)); }
                        });
                        ui.add_space(16.0);
                    }
                    for (si, s) in lesson.sections.iter().enumerate() {
                        let r = ui.add(egui::Label::new(egui::RichText::new(if s.title.is_empty() { "(untitled section)" } else { &s.title }).size(24.0).strong()).sense(egui::Sense::click()));
                        if r.clicked() { pick = Some((si, None)); }
                        ui.add_space(8.0);
                        for (bi, b) in s.blocks.iter().enumerate() {
                            let selected = self.sec == si && self.blk == Some(bi);
                            let stroke = if selected { egui::Stroke::new(2.0, accent) } else { egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color) };
                            let frame = egui::Frame::new().stroke(stroke).corner_radius(8).inner_margin(10).show(ui, |ui| {
                                ui.set_width(w - 22.0);
                                self.block_preview(ui, ctx, &b.kind, accent);
                            });
                            let resp = ui.interact(frame.response.rect, egui::Id::new(("block-click", si, bi)), egui::Sense::click());
                            if resp.clicked() { pick = Some((si, Some(bi))); }
                            ui.add_space(8.0);
                        }
                        if s.blocks.is_empty() { ui.weak("This section has no blocks. Use + Add block."); }
                        ui.add_space(24.0);
                    }
                });
            });
        });
        if let Some((s, b)) = pick { self.sec = s; self.blk = b; }
    }

    fn block_preview(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, kind: &BlockKind, accent: egui::Color32) {
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let note = |ui: &mut egui::Ui, s: &str| { ui.label(egui::RichText::new(s).color(weak).italics()); };
        match kind {
            BlockKind::Heading { text: t, level } => { ui.label(egui::RichText::new(t).size(match level { 1 => 26.0, 2 => 21.0, _ => 17.0 }).strong()); }
            BlockKind::Text { text: t } => show_markdown(ui, t, 15.0),
            BlockKind::Image { asset, alt, caption, crop, .. } => {
                self.picture(ui, ctx, asset.as_deref(), *crop, "No picture chosen", 520.0);
                if !alt.is_empty() { note(ui, &format!("Alt text: {alt}")); }
                if !caption.is_empty() { ui.label(egui::RichText::new(caption).size(13.0).color(weak)); }
            }
            BlockKind::ImageText { asset, text: t, image_right, crop, .. } => {
                ui.columns(2, |cols| {
                    let (a, b) = if *image_right { (1, 0) } else { (0, 1) };
                    cols[b].vertical(|ui| self.picture(ui, ctx, asset.as_deref(), *crop, "No picture chosen", 240.0));
                    show_markdown(&mut cols[a], t, 15.0);
                });
            }
            BlockKind::Video { asset, caption, .. } => { note(ui, &format!("Video: {}", self.asset_name(asset.as_deref()))); if !caption.is_empty() { ui.label(caption); } }
            BlockKind::Audio { asset, caption } => { note(ui, &format!("Audio: {}", self.asset_name(asset.as_deref()))); if !caption.is_empty() { ui.label(caption); } }
            BlockKind::Button { label, .. } => { ui.vertical_centered(|ui| { ui.add(egui::Button::new(egui::RichText::new(label).strong().color(egui::Color32::WHITE)).fill(accent).corner_radius(18).min_size(egui::vec2(120.0, 32.0))); }); }
            BlockKind::Continue { label } => {
                ui.vertical_centered(|ui| { ui.add(egui::Button::new(egui::RichText::new(label).strong().color(egui::Color32::WHITE)).fill(accent).corner_radius(18).min_size(egui::vec2(120.0, 32.0))); });
                note(ui, "Everything below stays hidden until the learner presses Continue.");
            }
            BlockKind::Divider => { ui.separator(); }
            BlockKind::Spacer => { ui.add_space(30.0); }
            BlockKind::Quote { text: t, by } => {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(4.0, 40.0), egui::Sense::hover());
                    ui.painter().rect_filled(r, 2.0, accent);
                    ui.vertical(|ui| { ui.label(egui::RichText::new(t).italics().size(17.0)); if !by.is_empty() { ui.label(egui::RichText::new(format!("— {by}")).color(weak)); } });
                });
            }
            BlockKind::Statement { text: t } => { ui.vertical_centered(|ui| { ui.label(egui::RichText::new(t).size(22.0).strong().color(text)); }); }
            BlockKind::Items { style, items } => {
                ui.label(egui::RichText::new(style.label()).small().color(weak));
                for (i, it) in items.iter().enumerate() {
                    let head = match style { ItemsStyle::Process => format!("{}. {}", i + 1, it.title), ItemsStyle::Accordion => format!("▸ {}", it.title), _ => it.title.clone() };
                    ui.label(egui::RichText::new(head).strong());
                    if i == 0 || matches!(style, ItemsStyle::Process | ItemsStyle::Timeline) { show_markdown(ui, &it.body, 14.0); }
                }
            }
            BlockKind::KnowledgeCheck(q) => {
                ui.label(egui::RichText::new("Knowledge check").small().color(weak));
                ui.label(egui::RichText::new(&q.prompt).strong().size(16.0));
                for c in &q.choices { ui.label(format!("{}  {}", if q.multiple { "☐" } else { "○" }, c.text)); }
                if !matches!(q.style, QuestionStyle::Choice) { note(ui, "Shown in the published lesson; edit it in Properties."); }
            }
        }
    }

    fn asset_name(&self, key: Option<&str>) -> String {
        key.and_then(|k| self.project.as_ref().and_then(|p| p.assets.get(k))).map(|a| a.name.clone()).unwrap_or_else(|| "none chosen".into())
    }

    fn picture(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, asset: Option<&str>, crop: Option<[f32; 4]>, empty: &str, max_w: f32) {
        match asset.and_then(|k| self.texture(ctx, k)) {
            Some(tex) => match crop {
                // Only the kept part shows, at the size it would have if the picture were not cropped.
                Some(c) => {
                    let (cw, ch) = ((1.0 - c[0] - c[2]).max(0.01), (1.0 - c[1] - c[3]).max(0.01));
                    let [tw, th] = tex.size().map(|v| v as f32);
                    let (nw, nh) = (tw * cw, th * ch);
                    let k = (max_w.min(ui.available_width()) / nw.max(1.0)).min(1.0);
                    let uv = egui::Rect::from_min_max(egui::pos2(c[0], c[1]), egui::pos2(1.0 - c[2], 1.0 - c[3]));
                    ui.add(egui::Image::new(&tex).uv(uv).fit_to_exact_size(egui::vec2(nw * k, nh * k)).corner_radius(6));
                }
                None => { ui.add(egui::Image::new(&tex).max_width(max_w.min(ui.available_width())).corner_radius(6)); }
            },
            None => { ui.label(egui::RichText::new(empty).italics().color(ui.visuals().weak_text_color())); }
        }
    }
}

/// A multi-line Markdown editor with small buttons for bold, italic, links and bullets.
fn md_edit(ui: &mut egui::Ui, id_salt: impl std::hash::Hash + std::fmt::Debug, text: &mut String, rows: usize) -> bool {
    let ctx = ui.ctx().clone();
    let id = egui::Id::new(("lesson-md", id_salt));
    let sel = egui::TextEdit::load_state(&ctx, id).and_then(|s| s.cursor.char_range()).map(|r| (r.primary.index.0.min(r.secondary.index.0), r.primary.index.0.max(r.secondary.index.0)));
    let mut changed = false;
    ui.horizontal(|ui| {
        if ui.small_button(egui::RichText::new("B").strong()).on_hover_text("Bold the selected words").clicked() { *text = wrap_selection(text, sel, "**", "**"); changed = true; }
        if ui.small_button(egui::RichText::new("I").italics()).on_hover_text("Italic").clicked() { *text = wrap_selection(text, sel, "*", "*"); changed = true; }
        if ui.small_button("Link").on_hover_text("Turn the selected words into a link, then replace the address").clicked() { *text = wrap_selection(text, sel, "[", "](https://)"); changed = true; }
        if ui.small_button("• List").on_hover_text("Make bullet points").clicked() { *text = bulletize(text, sel); changed = true; }
    });
    changed |= ui.add(egui::TextEdit::multiline(text).id(id).desired_rows(rows).desired_width(f32::INFINITY)).changed();
    changed
}

/// The shapes a picture block can be cropped to, by name.
const CROP_SHAPES: [(&str, f32); 8] = [("1:1  square", 1.0), ("4:3", 4.0 / 3.0), ("3:2", 1.5), ("16:9", 16.0 / 9.0), ("21:9", 21.0 / 9.0), ("3:4  portrait", 0.75), ("2:3  portrait", 2.0 / 3.0), ("9:16  portrait", 9.0 / 16.0)];

/// Crop a picture block to a shape, and say which part of the picture stays. Returns whether anything changed.
fn crop_row(ui: &mut egui::Ui, asset: Option<&str>, crop: &mut Option<[f32; 4]>, dims: &dyn Fn(&str) -> Option<(u32, u32)>) -> bool {
    let Some((iw, ih)) = asset.and_then(dims).map(|(w, h)| (w as f32, h as f32)) else { return false };
    let (ratio, fx, fy) = crop.map(|c| crop_shape(iw, ih, c)).unwrap_or((iw / ih, 0.5, 0.5));
    let current = match crop {
        None => "Whole picture",
        Some(_) => CROP_SHAPES.iter().find(|(_, r)| (r - ratio).abs() / r < 0.01).map(|(n, _)| *n).unwrap_or("Custom"),
    };
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("Crop to");
        combo("block-crop").selected_text(current).show_ui(ui, |ui| {
            if ui.selectable_label(crop.is_none(), "Whole picture").clicked() { *crop = None; changed = true; }
            for (name, r) in CROP_SHAPES {
                if ui.selectable_label(current == name, name).clicked() { *crop = crop_for_ratio(iw, ih, r, fx, fy); changed = true; }
            }
        });
    });
    if let Some(c) = *crop {
        // Where the kept part sits along each side that has been cut.
        let (cut_x, cut_y) = (c[0] + c[2] > 1e-4, c[1] + c[3] > 1e-4);
        let (ratio, mut fx, mut fy) = crop_shape(iw, ih, c);
        let (mut px, mut py, mut moved) = (fx * 100.0, fy * 100.0, false);
        if cut_x {
            ui.horizontal(|ui| { ui.label("Keep"); moved |= ui.add(egui::Slider::new(&mut px, 0.0..=100.0).suffix("%  from the left")).changed(); });
            fx = px / 100.0;
        }
        if cut_y {
            ui.horizontal(|ui| { ui.label("Keep"); moved |= ui.add(egui::Slider::new(&mut py, 0.0..=100.0).suffix("%  from the top")).changed(); });
            fy = py / 100.0;
        }
        if moved {
            *crop = crop_for_ratio(iw, ih, ratio, fx, fy).or(*crop);
            changed = true;
        }
    }
    changed
}

fn asset_row(ui: &mut egui::Ui, label: &str, key: Option<&str>, names: &std::collections::BTreeMap<String, String>, want: &mut Option<LessonAsset>, which: LessonAsset, button: &str) {
    ui.horizontal(|ui| {
        ui.label(format!("{label}: {}", key.and_then(|k| names.get(k)).map(String::as_str).unwrap_or("none")));
        if ui.button(button).clicked() { *want = Some(which); }
    });
}

fn single(ui: &mut egui::Ui, label: &str, text: &mut String) -> bool {
    ui.label(label);
    ui.add(egui::TextEdit::singleline(text).desired_width(f32::INFINITY)).changed()
}

#[allow(clippy::too_many_arguments)]
fn block_editor(ui: &mut egui::Ui, id: &str, kind: &mut BlockKind, names: &std::collections::BTreeMap<String, String>, dims: &dyn Fn(&str) -> Option<(u32, u32)>, number_vars: &[(String, String)], gen: &mut dyn FnMut() -> String, want: &mut Option<LessonAsset>) -> bool {
    let mut changed = false;
    match kind {
        BlockKind::Heading { text, level } => {
            changed |= single(ui, "Heading", text);
            ui.horizontal(|ui| {
                ui.label("Size");
                for (n, label) in [(1u8, "Large"), (2, "Medium"), (3, "Small")] { if ui.selectable_label(*level == n, label).clicked() { *level = n; changed = true; } }
            });
        }
        BlockKind::Text { text } => { changed |= md_edit(ui, id, text, 10); ui.weak(markdown_hint()); }
        BlockKind::Image { asset, alt, caption, width, decorative, crop } => {
            asset_row(ui, "Picture", asset.as_deref(), names, want, LessonAsset::Image, "Choose picture…");
            changed |= crop_row(ui, asset.as_deref(), crop, dims);
            changed |= ui.checkbox(decorative, "Decorative (screen readers skip it)").changed();
            if !*decorative { changed |= single(ui, "Alt text (describe the picture)", alt); }
            changed |= single(ui, "Caption", caption);
            ui.horizontal(|ui| {
                ui.label("Width");
                for (w, label) in [(ImageWidth::Full, "Full"), (ImageWidth::Medium, "Medium"), (ImageWidth::Small, "Small")] { if ui.selectable_label(*width == w, label).clicked() { *width = w; changed = true; } }
            });
        }
        BlockKind::ImageText { asset, alt, text, image_right, crop } => {
            asset_row(ui, "Picture", asset.as_deref(), names, want, LessonAsset::Image, "Choose picture…");
            changed |= crop_row(ui, asset.as_deref(), crop, dims);
            changed |= single(ui, "Alt text (describe the picture)", alt);
            changed |= ui.checkbox(image_right, "Picture on the right").changed();
            ui.label("Text");
            changed |= md_edit(ui, id, text, 8);
        }
        BlockKind::Video { asset, captions, caption } => {
            asset_row(ui, "Video", asset.as_deref(), names, want, LessonAsset::Video, "Choose video…");
            asset_row(ui, "Captions", captions.as_deref(), names, want, LessonAsset::Captions, "Choose captions (.vtt)…");
            if captions.is_some() && ui.small_button("Remove captions").clicked() { *captions = None; changed = true; }
            changed |= single(ui, "Caption", caption);
        }
        BlockKind::Audio { asset, caption } => {
            asset_row(ui, "Audio", asset.as_deref(), names, want, LessonAsset::Audio, "Choose audio…");
            changed |= single(ui, "Caption", caption);
        }
        BlockKind::Button { label, url } => {
            changed |= single(ui, "Label", label);
            changed |= single(ui, "Address (https:// or mailto:)", url);
            if !(url.starts_with("https://") || url.starts_with("http://") || url.starts_with("mailto:")) { ui.colored_label(egui::Color32::from_rgb(200, 60, 40), "The address must start with https://, http:// or mailto:."); }
        }
        BlockKind::Continue { label } => {
            changed |= single(ui, "Button label", label);
            ui.weak("Hides everything below until the learner presses the button.");
        }
        BlockKind::Quote { text, by } => {
            ui.label("Quote");
            changed |= ui.add(egui::TextEdit::multiline(text).desired_rows(4).desired_width(f32::INFINITY)).changed();
            changed |= single(ui, "Said by", by);
        }
        BlockKind::Statement { text } => {
            ui.label("Statement");
            changed |= ui.add(egui::TextEdit::multiline(text).desired_rows(3).desired_width(f32::INFINITY)).changed();
        }
        BlockKind::Items { style, items } => {
            ui.horizontal(|ui| {
                ui.label("Layout");
                combo(("items-style", id)).selected_text(style.label()).show_ui(ui, |ui| {
                    for s in ItemsStyle::ALL { if ui.selectable_label(*style == s, s.label()).clicked() { *style = s; changed = true; } }
                });
            });
            let (title_label, body_label) = match style { ItemsStyle::Timeline => ("Date or label", "Details"), ItemsStyle::Flashcards => ("Front of card", "Back of card"), _ => ("Title", "Content") };
            let mut remove = None;
            let mut swap: Option<(usize, usize)> = None;
            let n = items.len();
            for (i, it) in items.iter_mut().enumerate() {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.strong(format!("Item {}", i + 1));
                    if ui.add_enabled(i > 0, egui::Button::new("↑").small()).clicked() { swap = Some((i, i - 1)); }
                    if ui.add_enabled(i + 1 < n, egui::Button::new("↓").small()).clicked() { swap = Some((i, i + 1)); }
                    if ui.small_button("Remove").clicked() { remove = Some(i); }
                });
                changed |= single(ui, title_label, &mut it.title);
                ui.label(body_label);
                changed |= md_edit(ui, (id, i), &mut it.body, 4);
            }
            if let Some(i) = remove { items.remove(i); changed = true; }
            if let Some((a, b)) = swap { items.swap(a, b); changed = true; }
            if ui.button("+ Add item").clicked() { items.push(Item { title: format!("Item {}", items.len() + 1), body: String::new() }); changed = true; }
        }
        BlockKind::KnowledgeCheck(q) => {
            let mut qwant: Option<AssetWant> = None;
            changed |= question_editor(ui, q, gen, &mut qwant, number_vars);
            match qwant {
                Some(AssetWant::Image) => *want = Some(LessonAsset::QuestionImage),
                Some(AssetWant::Audio) => *want = Some(LessonAsset::QuestionAudio),
                None => {}
            }
        }
        BlockKind::Divider | BlockKind::Spacer => { ui.weak("This block has no settings."); }
    }
    changed
}

impl App {
    fn lesson_properties(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let before = Snap::of(self.project.as_ref().unwrap());
        let mut ctr = self.id_counter;
        let t0 = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let mut gen = || { ctr += 1; format!("lq-{t0:x}-{ctr}") };
        let (sec, blk) = (self.sec, self.blk);
        let (names, number_vars): (std::collections::BTreeMap<String, String>, Vec<(String, String)>) = {
            let p = self.project.as_ref().unwrap();
            (p.assets.iter().map(|(k, a)| (k.clone(), a.name.clone())).collect(),
             p.variables.iter().filter(|v| !v.built_in && matches!(v.initial, Value::Num(_))).map(|v| (v.id.clone(), v.name.clone())).collect())
        };
        let sizes: std::collections::BTreeMap<String, (u32, u32)> = {
            // Only the pictures a block can show are measured, and only their headers are read.
            let p = self.project.as_ref().unwrap();
            p.assets.iter().filter(|(_, a)| a.mime.starts_with("image/")).filter_map(|(k, a)| {
                image::ImageReader::new(std::io::Cursor::new(&a.bytes)).with_guessed_format().ok()?.into_dimensions().ok().map(|d| (k.clone(), d))
            }).collect()
        };
        let dims = |k: &str| sizes.get(k).copied();
        let mut lesson = self.project.as_mut().unwrap().lesson.take().unwrap();
        let mut changed = false;
        let mut want: Option<LessonAsset> = None;
        let mut section_action: Option<u8> = None; // 0 up, 1 down, 2 delete
        let n_sections = lesson.sections.len();
        egui::ScrollArea::vertical().id_salt("lesson-props-scroll").show(ui, |ui| {
            let Some(s) = lesson.sections.get_mut(sec) else { return };
            match blk.and_then(|b| s.blocks.get_mut(b)) {
                None => {
                    ui.strong(format!("Section {}", sec + 1));
                    changed |= single(ui, "Title (shown in the menu)", &mut s.title);
                    ui.label("Summary (shown on the title page)");
                    changed |= ui.add(egui::TextEdit::multiline(&mut s.summary).desired_rows(3).desired_width(f32::INFINITY)).changed();
                    ui.horizontal(|ui| {
                        if ui.add_enabled(sec > 0, egui::Button::new("Move up")).clicked() { section_action = Some(0); }
                        if ui.add_enabled(sec + 1 < n_sections, egui::Button::new("Move down")).clicked() { section_action = Some(1); }
                        if ui.add_enabled(n_sections > 1, egui::Button::new("Delete section")).clicked() { section_action = Some(2); }
                    });
                    ui.weak(format!("{} block{}. Select a block to edit it, or use + Add block.", s.blocks.len(), if s.blocks.len() == 1 { "" } else { "s" }));
                }
                Some(b) => {
                    ui.strong(b.kind.label());
                    ui.add_space(4.0);
                    let id = b.id.clone();
                    changed |= block_editor(ui, &id, &mut b.kind, &names, &dims, &number_vars, &mut gen, &mut want);
                }
            }
        });
        self.project.as_mut().unwrap().lesson = Some(lesson);
        self.id_counter = ctr;
        if changed {
            let editing = ctx.input(|i| i.pointer.any_down()) || ctx.memory(|m| m.focused().is_some());
            if !(self.edit_open && editing) {
                self.undo.push(before);
                self.redo.clear();
            }
            self.edit_open = true;
            self.dirty = true;
        } else if !ctx.input(|i| i.pointer.any_down()) && ctx.memory(|m| m.focused().is_none()) {
            self.edit_open = false;
        }
        match section_action { Some(0) => self.move_section(-1), Some(1) => self.move_section(1), Some(2) => self.delete_section(), _ => {} }
        if let Some(w) = want { self.pick_lesson_asset(w); }
    }

    /// Ask for a file and store it in the selected block (or the cover).
    fn pick_lesson_asset(&mut self, w: LessonAsset) {
        let key = match w {
            LessonAsset::Image | LessonAsset::CoverImage | LessonAsset::QuestionImage => self.load_image_asset().map(|(k, _, _, _)| k),
            LessonAsset::Video => self.load_media(&["mp4", "m4v", "webm"], "Video").map(|(k, _)| k),
            LessonAsset::Captions => self.load_media(&["vtt"], "Captions").map(|(k, _)| k),
            LessonAsset::Audio | LessonAsset::QuestionAudio => self.load_media(&["mp3", "wav", "ogg", "m4a"], "Audio").map(|(k, _)| k),
        };
        let Some(key) = key else { return };
        self.snapshot();
        let (sec, blk) = (self.sec, self.blk);
        let l = self.lesson_mut();
        if w == LessonAsset::CoverImage {
            if let Some(c) = l.cover.as_mut() { c.image = Some(key); }
            return;
        }
        let Some(b) = blk.and_then(|b| l.sections.get_mut(sec).and_then(|s| s.blocks.get_mut(b))) else { return };
        match (&mut b.kind, w) {
            (BlockKind::Image { asset, .. } | BlockKind::ImageText { asset, .. }, LessonAsset::Image) => *asset = Some(key),
            (BlockKind::Video { asset, .. }, LessonAsset::Video) => *asset = Some(key),
            (BlockKind::Video { captions, .. }, LessonAsset::Captions) => *captions = Some(key),
            (BlockKind::Audio { asset, .. }, LessonAsset::Audio) => *asset = Some(key),
            (BlockKind::KnowledgeCheck(q), LessonAsset::QuestionImage | LessonAsset::QuestionAudio) => { q.style.set_asset(key); }
            _ => {}
        }
    }

    /// Lesson-wide settings: title, completion rule, menu, cover and theme.
    pub(super) fn lesson_settings_window(&mut self, ctx: &egui::Context) {
        if !self.lesson_settings { return; }
        let mut open = true;
        let before = Snap::of(self.project.as_ref().unwrap());
        let mut want_cover = false;
        let mut changed = false;
        egui::Window::new("Lesson settings").open(&mut open).default_width(420.0).show(ctx, |ui| {
            let p = self.project.as_mut().unwrap();
            changed |= single(ui, "Lesson title", &mut p.name);
            ui.separator();
            ui.label("The lesson counts as complete when");
            if let Some(l) = p.lesson.as_mut() {
                for c in Completion::ALL { if ui.radio_value(&mut l.completion, c, c.label()).changed() { changed = true; } }
            }
            ui.horizontal(|ui| {
                ui.label("Pass mark for knowledge checks");
                changed |= ui.add(egui::DragValue::new(&mut p.pass_mark).range(0..=100).suffix("%")).changed();
            });
            ui.separator();
            if let Some(l) = p.lesson.as_mut() {
                changed |= ui.checkbox(&mut l.show_menu, "Show the section menu").changed();
                let mut has_cover = l.cover.is_some();
                if ui.checkbox(&mut has_cover, "Show a title banner").changed() { l.cover = has_cover.then(Cover::default); changed = true; }
                if let Some(c) = l.cover.as_mut() {
                    changed |= single(ui, "Banner subtitle", &mut c.subtitle);
                    ui.label("Description (under the title)");
                    changed |= ui.add(egui::TextEdit::multiline(&mut c.description).desired_rows(3).desired_width(f32::INFINITY)).changed();
                    ui.horizontal(|ui| {
                        ui.label(format!("Banner picture: {}", if c.image.is_some() { "chosen" } else { "none" }));
                        if ui.button("Choose picture…").clicked() { want_cover = true; }
                        if c.image.is_some() && ui.small_button("Remove").clicked() { c.image = None; changed = true; }
                    });
                }
            }
            ui.separator();
            ui.label("Colours and fonts");
            ui.horizontal_wrapped(|ui| {
                for t in Theme::presets() {
                    if ui.selectable_label(p.theme.name == t.name, &t.name).clicked() { p.theme = t; changed = true; }
                }
            });
            if let Some(l) = p.lesson.as_mut() {
                for (label, slot, idx) in [("Primary colour (buttons, progress)", &mut l.primary, 4usize), ("Secondary colour (highlights)", &mut l.secondary, 5)] {
                    ui.horizontal(|ui| {
                        let theme_c = p.theme.color(idx);
                        let mut c = slot.unwrap_or(theme_c);
                        if crate::ribbon::color_swatch(ui, &mut c) { *slot = Some(c); changed = true; }
                        ui.label(label);
                        if slot.is_some() && ui.small_button("Use theme colour").clicked() { *slot = None; changed = true; }
                    });
                }
            }
        });
        if changed { self.undo.push(before); self.redo.clear(); self.dirty = true; }
        if want_cover { self.pick_lesson_asset(LessonAsset::CoverImage); }
        if !open { self.lesson_settings = false; }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapping_and_bullets_work_on_character_positions() {
        assert_eq!(wrap_selection("say héllo now", Some((4, 9)), "**", "**"), "say **héllo** now");
        assert_eq!(wrap_selection("abc", None, "*", "*"), "abc**");
        assert_eq!(bulletize("one\ntwo\nthree", Some((0, 7))), "- one\n- two\nthree");
        assert_eq!(bulletize("one", None), "one\n- ");
    }
}
