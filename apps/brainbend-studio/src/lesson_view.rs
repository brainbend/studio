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

/// The icon of a kind of block, in the lists.
fn block_icon(kind: &BlockKind) -> char {
    match kind {
        BlockKind::Heading { .. } => ribbon::TEXT_T,
        BlockKind::Text { .. } => ribbon::TEXTBOX,
        BlockKind::Image { .. } => ribbon::IMAGE,
        BlockKind::ImageText { .. } => ribbon::FRAME,
        BlockKind::Video { .. } => ribbon::VIDEO,
        BlockKind::Audio { .. } => ribbon::SPEAKER,
        BlockKind::Button { .. } => ribbon::CLICK,
        BlockKind::Divider => ribbon::LINE,
        BlockKind::Spacer => ribbon::DIST_V,
        BlockKind::Quote { .. } => ribbon::CHAT,
        BlockKind::Statement { .. } => ribbon::STAR,
        BlockKind::Items { style, .. } => match style {
            brainbend_doc::lesson::ItemsStyle::Process => ribbon::LIST_NUM,
            brainbend_doc::lesson::ItemsStyle::Timeline => ribbon::TIMER,
            brainbend_doc::lesson::ItemsStyle::Flashcards => ribbon::STACK,
            brainbend_doc::lesson::ItemsStyle::Tabs => ribbon::TABLE,
            _ => ribbon::LIST_BULLETS,
        },
        BlockKind::KnowledgeCheck(_) => ribbon::QUESTION,
        BlockKind::Continue { .. } => ribbon::ARROW_DOWN,
    }
}

fn short(text: &str, n: usize) -> String {
    let one: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if one.chars().count() > n { format!("{}…", one.chars().take(n).collect::<String>()) } else { one }
}

impl App {
    pub(super) fn lesson_ref(&self) -> &WebLesson {
        self.project.as_ref().and_then(|p| p.lesson.as_ref()).expect("a Web Lesson project has lesson content")
    }

    pub(super) fn lesson_mut(&mut self) -> &mut WebLesson {
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

    pub(super) fn delete_block(&mut self) {
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

    pub(super) fn move_section(&mut self, delta: i32) {
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

    /// The insert bar of a Web Lesson, the same slim band as a media lesson's and in the same place, between the panels: a button for each
    /// kind of block, grouped as in the Add block menu, then the buttons that arrange blocks, then the tools. What does not fit goes in the "…"
    /// menu at its end. Preview and Publish are in the title bar with the other buttons there.
    pub(super) fn lesson_toolbar(&mut self, root: &mut egui::Ui, _ctx: &egui::Context) {
        if !self.show_insert_bar { return; }
        let band = if root.visuals().dark_mode { egui::Color32::from_gray(40) } else { egui::Color32::from_gray(246) };
        let has = self.blk.is_some();
        let n_sections = self.project.as_ref().and_then(|p| p.lesson.as_ref()).map(|l| l.sections.len()).unwrap_or(1);
        // (icon, name, tip, enabled, what it does) with None for a divider. An action is 0 move up, 1 move down, 2 duplicate, 3 delete,
        // 4 section, 5 find, 6 spelling, 7 check, 8 settings.
        enum Do { Add(BlockKind), Act(u8) }
        let mut items: Vec<Option<(char, &str, String, bool, Do)>> = vec![];
        for (n, (group, kinds)) in BlockKind::catalog().into_iter().enumerate() {
            if n > 0 { items.push(None); }
            for k in kinds { items.push(Some((block_icon(&k), k.label(), group.to_string(), true, Do::Add(k)))); }
        }
        items.push(None);
        for (i, icon, label, tip, enabled) in [
            (0u8, ribbon::ARROW_UP, "Move up", "Move the chosen block up", has),
            (1, ribbon::ARROW_DOWN, "Move down", "Move the chosen block down", has),
            (2, ribbon::COPY, "Duplicate", "Copy the chosen block below itself", has),
            (3, ribbon::TRASH, "Delete", "Delete the chosen block", has),
            (4, ribbon::FILE_PLUS, "+ Section", "Add a section after this one", n_sections > 0),
        ] { items.push(Some((icon, label, tip.to_string(), enabled, Do::Act(i)))); }
        items.push(None);
        for (i, icon, label, tip) in [
            (5u8, ribbon::MAGNIFY, "Find", "Find and replace"),
            (6, ribbon::TEXT_AA, "Spelling", "Check spelling (F7)"),
            (7, ribbon::CHECK, "Check", "Check the lesson for problems"),
            (8, ribbon::GEAR, "Lesson settings", "The lesson's title, theme and options"),
        ] { items.push(Some((icon, label, tip.to_string(), true, Do::Act(i)))); }
        let dividers: Vec<bool> = items.iter().map(|e| e.is_none()).collect();
        let mut chosen: Option<usize> = None;
        egui::Panel::top("lessonbar").frame(egui::Frame::new().fill(band).inner_margin(egui::Margin::symmetric(8, 3))).show_separator_line(false).show(root, |ui| {
            ribbon::overflow_row(ui, &dividers, &mut |ui, i, in_menu| {
                let Some((icon, label, tip, enabled, _)) = items[i].as_ref() else { return };
                if in_menu {
                    let r = ui.add_enabled_ui(*enabled, |ui| ribbon::menu_item(ui, *icon, label)).inner;
                    if r.on_hover_text(tip).clicked() { chosen = Some(i); ui.close(); }
                    return;
                }
                let b = egui::Button::new(egui::RichText::new(icon.to_string()).font(egui::FontId::new(18.0, ribbon::icons())).color(ui.visuals().text_color())).frame(false);
                let r = ui.add_enabled(*enabled, b.min_size(egui::vec2(ribbon::BAR_BTN, ribbon::BAR_H)));
                r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, *enabled, *label));
                let r = r.on_hover_ui(|ui| { ui.strong(*label); if !tip.is_empty() { ui.weak(tip); } });
                if r.clicked() { chosen = Some(i); }
            });
        });
        let Some(i) = chosen else { return };
        match items.swap_remove(i) {
            Some((_, _, _, _, Do::Add(k))) => self.add_block(k),
            Some((_, _, _, _, Do::Act(a))) => match a {
                0 => self.move_block(-1),
                1 => self.move_block(1),
                2 => self.duplicate_block(),
                3 => self.delete_block(),
                4 => self.add_section(),
                5 => { self.find.get_or_insert_with(Default::default).focus = true; }
                6 => self.open_spelling(),
                7 => { self.check_results = self.project.as_ref().map(|p| p.check()); }
                _ => self.lesson_settings = true,
            },
            None => {}
        }
    }

    /// Move a block to a place in the lesson: before or after another block, or to the start of a section.
    pub(super) fn move_block_to(&mut self, from: (usize, usize), to_sec: usize, to_at: usize) {
        let l = self.lesson_ref();
        if from.0 >= l.sections.len() || from.1 >= l.sections[from.0].blocks.len() || to_sec >= l.sections.len() { return; }
        self.snapshot();
        let l = self.lesson_mut();
        let b = l.sections[from.0].blocks.remove(from.1);
        // Taking it out shifted what came after it in its own section.
        let at = if to_sec == from.0 && to_at > from.1 { to_at - 1 } else { to_at }.min(l.sections[to_sec].blocks.len());
        l.sections[to_sec].blocks.insert(at, b);
        self.sec = to_sec;
        self.blk = Some(at);
    }

    /// Move a section so that it sits at place `to` (counted before it is taken out).
    pub(super) fn move_section_to(&mut self, from: usize, to: usize) {
        let n = self.lesson_ref().sections.len();
        if from >= n || to > n { return; }
        self.snapshot();
        let l = self.lesson_mut();
        let s = l.sections.remove(from);
        let at = if to > from { to - 1 } else { to }.min(l.sections.len());
        l.sections.insert(at, s);
        self.sec = at;
        self.blk = None;
    }

    /// The Outline: the sections and their blocks in the order of the page, in the mode the panel was left in. A section opens and closes;
    /// double-click renames it; drag moves a block or a section; right-click has the rest.
    pub(super) fn lesson_outline(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        use list_view::{drop_line, mode_bar, row, RowSpec};
        ui.horizontal(|ui| {
            ui.strong("Outline");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("+ Section").clicked() { self.add_section(); }
            });
        });
        mode_bar(ui, &mut self.outline_list, "outline");
        ui.separator();
        let mode = self.outline_list;
        let lesson = self.lesson_ref().clone();
        let mut pick: Option<(usize, Option<usize>)> = None;
        // What was asked of a row by its menu or a double-click.
        enum Do { Rename(usize), MoveSec(i32), DeleteSec, MoveBlk(i32), Duplicate, DeleteBlk }
        let mut act: Option<((usize, Option<usize>), Do)> = None;
        let rename_id = egui::Id::new("outline-rename");
        let mut renaming: Option<usize> = ui.data(|d| d.get_temp(rename_id));
        let drag_id = egui::Id::new("outline-drag");
        let mut dragging: Option<(usize, Option<usize>)> = ui.data(|d| d.get_temp(drag_id));
        let mut rects: Vec<(egui::Rect, (usize, Option<usize>))> = vec![];
        let mut row_ids: Vec<egui::Id> = vec![];
        let over_panel = ui.ui_contains_pointer();
        let accent = { let c = self.project.as_ref().map(|p| p.theme.color(4)).unwrap_or(Rgba(3, 83, 158, 255)); egui::Color32::from_rgb(c.0, c.1, c.2) };
        egui::ScrollArea::vertical().id_salt("outline-scroll").auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            for (si, s) in lesson.sections.iter().enumerate() {
                let title = if s.title.trim().is_empty() { "(untitled section)" } else { s.title.as_str() };
                let closed_id = egui::Id::new(("outline-closed", &s.id));
                let closed: bool = ui.data(|d| d.get_temp(closed_id)).unwrap_or(false);
                if renaming == Some(si) {
                    let mut text = s.title.clone();
                    let r = ui.add(egui::TextEdit::singleline(&mut text).desired_width(ui.available_width()));
                    if !r.has_focus() && !r.lost_focus() { r.request_focus(); }
                    if r.changed() { self.lesson_mut().sections[si].title = text; self.dirty = true; }
                    if r.lost_focus() { renaming = None; }
                    continue;
                }
                let spec = RowSpec { icon: ribbon::BOOK_OPEN, title, detail: &format!("{} block{}", s.blocks.len(), if s.blocks.len() == 1 { "" } else { "s" }), number: if mode == list_view::ListMode::Large { None } else { Some(format!("{}.", si + 1)) }, depth: 0, selected: self.sec == si && self.blk.is_none(), bold: true, caret: Some(!closed) };
                let out = row(ui, mode, &spec, &mut |p, r| {
                    p.rect_filled(r, 3.0, accent);
                    p.text(r.center(), egui::Align2::CENTER_CENTER, format!("{}", si + 1), egui::FontId::proportional(r.height() * 0.5), egui::Color32::WHITE);
                });
                rects.push((out.resp.rect, (si, None)));
                row_ids.push(out.resp.id);
                if out.caret_clicked { ui.data_mut(|d| d.insert_temp(closed_id, !closed)); }
                else if out.resp.double_clicked() { pick = Some((si, None)); renaming = Some(si); }
                else if out.resp.clicked() { pick = Some((si, None)); }
                if out.resp.drag_started() { dragging = Some((si, None)); }
                out.resp.context_menu(|ui| {
                    if ui.add(egui::Button::new("Rename").shortcut_text("F2")).clicked() { act = Some(((si, None), Do::Rename(si))); ui.close(); }
                    if ui.add_enabled(si > 0, egui::Button::new("Move up")).clicked() { act = Some(((si, None), Do::MoveSec(-1))); ui.close(); }
                    if ui.add_enabled(si + 1 < lesson.sections.len(), egui::Button::new("Move down")).clicked() { act = Some(((si, None), Do::MoveSec(1))); ui.close(); }
                    ui.separator();
                    if ui.add_enabled(lesson.sections.len() > 1, egui::Button::new("Delete section").shortcut_text("Del")).clicked() { act = Some(((si, None), Do::DeleteSec)); ui.close(); }
                });
                if closed { continue; }
                for (bi, b) in s.blocks.iter().enumerate() {
                    let first = b.kind.texts().into_iter().find(|t| !t.trim().is_empty()).unwrap_or_default();
                    let name = b.kind.label();
                    let picture = match &b.kind {
                        BlockKind::Image { asset, .. } | BlockKind::ImageText { asset, .. } => asset.clone(),
                        _ => None,
                    };
                    let tex = if mode == list_view::ListMode::Large { picture.as_deref().and_then(|k| self.texture(ctx, k)) } else { None };
                    let (icon, detail) = (block_icon(&b.kind), short(&first, 60));
                    let title = if mode == list_view::ListMode::Text && !first.is_empty() { format!("{name}: {}", short(&first, 26)) } else { name.to_string() };
                    let spec = RowSpec { icon, title: &title, detail: &detail, number: None, depth: 1, selected: self.sec == si && self.blk == Some(bi), bold: false, caret: None };
                    let fill = ui.visuals().extreme_bg_color;
                    let weak = ui.visuals().weak_text_color();
                    let out = row(ui, mode, &spec, &mut |p, r| {
                        if let Some(t) = &tex {
                            let [w, h] = t.size().map(|v| v as f32);
                            // Fill the thumbnail, cropping what does not fit.
                            let k = (r.width() / w).max(r.height() / h);
                            let (sw, sh) = (r.width() / (w * k), r.height() / (h * k));
                            let uv = egui::Rect::from_center_size(egui::pos2(0.5, 0.5), egui::vec2(sw, sh));
                            p.image(t.id(), r, uv, egui::Color32::WHITE);
                        } else {
                            p.rect_filled(r, 3.0, fill);
                            p.text(egui::pos2(r.center().x, r.top() + 12.0), egui::Align2::CENTER_CENTER, icon.to_string(), egui::FontId::new(14.0, ribbon::icons()), accent);
                            if !first.is_empty() {
                                let g = p.layout(short(&first, 40), egui::FontId::proportional(8.0), weak, r.width() - 8.0);
                                p.galley(egui::pos2(r.left() + 4.0, r.top() + 24.0), g, weak);
                            }
                        }
                    });
                    rects.push((out.resp.rect, (si, Some(bi))));
                    row_ids.push(out.resp.id);
                    if out.resp.clicked() { pick = Some((si, Some(bi))); }
                    if out.resp.drag_started() { dragging = Some((si, Some(bi))); }
                    out.resp.context_menu(|ui| {
                        if ui.add_enabled(bi > 0, egui::Button::new("Move up")).clicked() { act = Some(((si, Some(bi)), Do::MoveBlk(-1))); ui.close(); }
                        if ui.add_enabled(bi + 1 < s.blocks.len(), egui::Button::new("Move down")).clicked() { act = Some(((si, Some(bi)), Do::MoveBlk(1))); ui.close(); }
                        if ui.button("Duplicate").clicked() { act = Some(((si, Some(bi)), Do::Duplicate)); ui.close(); }
                        ui.separator();
                        if ui.add(egui::Button::new("Delete").shortcut_text("Del")).clicked() { act = Some(((si, Some(bi)), Do::DeleteBlk)); ui.close(); }
                    });
                }
            }
        });
        // F2 and Delete work on the chosen row while the list has the keyboard: one of its rows is in focus, or no field is and the pointer is over it.
        let has_keys = row_ids.iter().any(|id| ctx.memory(|m| m.has_focus(*id))) || (over_panel && ctx.memory(|m| m.focused().is_none()));
        if has_keys && act.is_none() && renaming.is_none() {
            let (f2, del) = ctx.input(|i| (i.key_pressed(egui::Key::F2), i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)));
            let here = (self.sec, self.blk);
            if f2 && here.1.is_none() { act = Some((here, Do::Rename(here.0))); }
            else if del { act = Some((here, if here.1.is_some() { Do::DeleteBlk } else { Do::DeleteSec })); }
        }
        // A drag in progress: the line where it would land, and the move when the pointer is let go.
        if let Some(d) = dragging {
            let pos = ctx.input(|i| i.pointer.interact_pos());
            let over = pos.and_then(|p| rects.iter().find(|(r, _)| r.contains(p)).map(|(r, it)| (*r, *it, p.y > r.center().y)));
            if let Some((r, _, below)) = over { drop_line(ui, r, below); }
            if ctx.input(|i| i.pointer.any_released()) {
                if let Some((_, target, below)) = over {
                    match (d, target) {
                        ((fs, None), (ts, _)) => { let to = if below { ts + 1 } else { ts }; if to != fs && to != fs + 1 { self.move_section_to(fs, to); } }
                        // On a section's heading: just under it is the start of that section, just over it the end of the one before.
                        ((fs, Some(fb)), (ts, None)) => {
                            if !below && ts > 0 { let end = self.lesson_ref().sections[ts - 1].blocks.len(); self.move_block_to((fs, fb), ts - 1, end); }
                            else { self.move_block_to((fs, fb), ts, 0); }
                        }
                        ((fs, Some(fb)), (ts, Some(tb))) => { let at = if below { tb + 1 } else { tb }; if (ts, at) != (fs, fb) && (ts, at) != (fs, fb + 1) { self.move_block_to((fs, fb), ts, at); } }
                    }
                }
                dragging = None;
            }
        }
        ui.data_mut(|d| { match dragging { Some(v) => { d.insert_temp(drag_id, v); } None => d.remove::<(usize, Option<usize>)>(drag_id) }; match renaming { Some(v) => { d.insert_temp(rename_id, v); } None => d.remove::<usize>(rename_id) } });
        if let Some((s, b)) = pick { self.sec = s; self.blk = b; self.toc_pick = web_tabs::TocPick::Nothing; self.web_show_section(s); }
        if let Some(((s, b), what)) = act {
            self.sec = s; self.blk = b;
            match what {
                Do::Rename(i) => ui.data_mut(|d| { d.insert_temp(rename_id, i); }),
                Do::MoveSec(d) => self.move_section(d),
                Do::DeleteSec => self.delete_section(),
                Do::MoveBlk(d) => self.move_block(d),
                Do::Duplicate => self.duplicate_block(),
                Do::DeleteBlk => self.delete_block(),
            }
        }
    }

    /// The whole Web Lesson editor: outline, preview and properties.
    pub(super) fn lesson_ui(&mut self, root: &mut egui::Ui, ctx: &egui::Context) {
        self.clamp_lesson_selection();
        self.web_sync_tabs();

        // The Outline and Properties are tabs of the same dock as in a media lesson: pinned to any side, or windows of their own.
        self.lesson_panels(root, ctx);
        // The insert bar sits at the top of what is left, between the panels, as it does for a slide.
        if !self.toc_showing() { self.lesson_toolbar(root, ctx); }
        // The preview is the working area: the drop strips for the left and top edges run along it, as they do along a slide.
        egui::CentralPanel::default().show(root, |ui| {
            let area = ui.max_rect();
            ctx.data_mut(|d| d.insert_temp(egui::Id::new("canvas-rect"), area));
            if self.toc_showing() { self.lesson_toc(ui, ctx); } else { self.lesson_preview(ui, ctx); }
        });
        self.lesson_settings_window(ctx);
    }

    /// The dock's edges and the panels that are windows, drawn around the preview.
    fn lesson_panels(&mut self, root: &mut egui::Ui, ctx: &egui::Context) {
        // The Table of Contents has no panels until a card or the banner is chosen (Properties then has its settings).
        if self.toc_showing() && self.toc_pick == web_tabs::TocPick::Nothing && !TOOLS.iter().any(|t| self.tool_open(*t)) {
            let window = ctx.content_rect();
            ctx.data_mut(|d| { d.remove_temp::<Vec<GroupRect>>(egui::Id::new("group-rects")); d.remove_temp::<Vec<GroupRect>>(egui::Id::new("group-rects-next")); d.insert_temp(egui::Id::new("root-rect"), window); });
            return;
        }
        let mut wants = SideWants::default();
        self.sync_current_in();
        let window = ctx.content_rect();
        ctx.data_mut(|d| { d.insert_temp(egui::Id::new("root-rect"), window); d.remove_temp::<Vec<GroupRect>>(egui::Id::new("group-rects-next")); });
        for edge in [dock::Edge::Left, dock::Edge::Right] { self.dock_edge(root, ctx, edge, 0, &mut wants); }
        self.panel_windows(ctx, 0, &mut wants);
        for edge in [dock::Edge::Top, dock::Edge::Bottom] { self.dock_edge(root, ctx, edge, 0, &mut wants); }
        ctx.data_mut(|d| { let next: Vec<GroupRect> = d.get_temp(egui::Id::new("group-rects-next")).unwrap_or_default(); d.insert_temp(egui::Id::new("group-rects"), next); });
        dock_drag_overlay(ctx);
        self.sync_current_out();
    }

    /// The Table of Contents: the title page of the lesson with a card for each section. Clicking the banner chooses the page itself
    /// (Properties shows its settings); clicking a card chooses its section; double-clicking a card, or its Open button, opens the section in a tab.
    fn lesson_toc(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
                let lesson = self.lesson_ref().clone();
        let (name, theme) = { let p = self.project.as_ref().unwrap(); (p.name.clone(), p.theme.clone()) };
        let accent = { let c = theme.color(4); egui::Color32::from_rgb(c.0, c.1, c.2) };
        let dark = { let d = theme.color(2); egui::Color32::from_rgb(d.0, d.1, d.2) };
        let mut open: Option<usize> = None;
        let mut pick: Option<usize> = None;
        // Edits made on the cards (section, title, summary), what was asked of a card's menu (section, 0 up, 1 down, 2 delete) and the + Add section button.
        let mut edits: Vec<(usize, String, String)> = vec![];
        let mut act: Option<(usize, u8)> = None;
        let mut add = false;
        let before = Snap::of(self.project.as_ref().unwrap());
        let mut pick_toc = false;
        let mut turn_on = false;
        let mut settings = false;
        // Clicking the page between the cards chooses the page itself.
        let cleared = ui.interact(ui.max_rect(), egui::Id::new("toc-bg"), egui::Sense::click()).clicked();
        egui::ScrollArea::vertical().id_salt("lesson-toc").auto_shrink([false, false]).show(ui, |ui| {
            let w = ui.available_width().min(760.0);
            let pad = ((ui.available_width() - w) / 2.0).max(0.0);
            ui.horizontal_top(|ui| {
                ui.add_space(pad);
                ui.vertical(|ui| {
                    ui.set_width(w);
                    ui.add_space(12.0);
                    if !lesson.show_toc {
                        egui::Frame::new().stroke(egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color)).corner_radius(4).inner_margin(10).show(ui, |ui| {
                            ui.set_width(w - 22.0);
                            ui.label("The table of contents is off, so learners start in the first section. It stays here so you can turn it back on.");
                            if ui.button("Turn it on").clicked() { turn_on = true; }
                        });
                        ui.add_space(12.0);
                    }
                    let c = lesson.cover.clone().unwrap_or_default();
                    let banner = egui::Frame::new().fill(dark).corner_radius(4).inner_margin(24).stroke(if self.toc_pick == web_tabs::TocPick::Header { egui::Stroke::new(2.0, accent) } else { egui::Stroke::NONE }).show(ui, |ui| {
                        ui.set_width(w - 48.0);
                        let on = egui::Color32::WHITE;
                        ui.label(egui::RichText::new(&name).size(30.0).strong().color(on));
                        if !c.subtitle.is_empty() { ui.label(egui::RichText::new(&c.subtitle).size(16.0).color(on)); }
                        if !c.description.is_empty() { ui.label(egui::RichText::new(&c.description).size(14.0).color(on)); }
                    });
                    let b = ui.interact(banner.response.rect, egui::Id::new("toc-banner"), egui::Sense::click());
                    if b.double_clicked() { settings = true; } else if b.clicked() { pick_toc = true; }
                    b.on_hover_text("Double-click for the lesson's settings");
                    ui.add_space(16.0);
                    for (si, s) in lesson.sections.iter().enumerate() {
                        let selected = self.toc_pick == web_tabs::TocPick::Card && self.sec == si;
                        let stroke = if selected { egui::Stroke::new(2.0, accent) } else { egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color) };
                        let (mut title, mut summary) = (s.title.clone(), s.summary.clone());
                        let card = egui::Frame::new().stroke(stroke).corner_radius(4).inner_margin(12).show(ui, |ui| {
                            ui.set_width(w - 26.0);
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(format!("Module {}", si + 1)).size(12.0).color(accent));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.menu_button("...", |ui| {
                                        if ui.add_enabled(si > 0, egui::Button::new("Move up")).clicked() { act = Some((si, 0)); ui.close(); }
                                        if ui.add_enabled(si + 1 < lesson.sections.len(), egui::Button::new("Move down")).clicked() { act = Some((si, 1)); ui.close(); }
                                        ui.separator();
                                        if ui.add_enabled(lesson.sections.len() > 1, egui::Button::new("Delete section")).clicked() { act = Some((si, 2)); ui.close(); }
                                    }).response.on_hover_text("Move or delete this section");
                                    if ui.small_button("Open").on_hover_text("Open this section in a tab").clicked() { open = Some(si); }
                                });
                            });
                            let t = ui.add(egui::TextEdit::singleline(&mut title).font(egui::FontId::proportional(18.0)).hint_text("Section title").desired_width(f32::INFINITY));
                            let u = ui.add(egui::TextEdit::multiline(&mut summary).hint_text("Summary shown on this card").desired_rows(2).desired_width(f32::INFINITY));
                            if t.changed() || u.changed() { edits.push((si, title.clone(), summary.clone())); }
                            ui.weak(format!("{} block{}", s.blocks.len(), if s.blocks.len() == 1 { "" } else { "s" }));
                        });
                        // Pressing anywhere on a card, its fields included, chooses it.
                        let r = card.response.rect;
                        if ui.input(|i| i.pointer.primary_pressed() && i.pointer.interact_pos().is_some_and(|p| r.contains(p))) && ui.rect_contains_pointer(r) { pick = Some(si); }
                        ui.add_space(8.0);
                    }
                    if ui.button("+ Add section").clicked() { add = true; }
                    ui.add_space(24.0);
                });
            });
        });
        if !edits.is_empty() {
            let editing = ctx.input(|i| i.pointer.any_down()) || ctx.memory(|m| m.focused().is_some());
            if !(self.edit_open && editing) { self.undo.push(before); self.redo.clear(); }
            self.edit_open = true;
            self.dirty = true;
            for (si, t, u) in edits { if let Some(s) = self.lesson_mut().sections.get_mut(si) { s.title = t; s.summary = u; } }
        }
        if let Some((si, what)) = act {
            self.sec = si; self.blk = None; self.toc_pick = web_tabs::TocPick::Card;
            match what { 0 => self.move_section(-1), 1 => self.move_section(1), _ => self.delete_section() }
        }
        if add { self.add_section(); self.toc_pick = web_tabs::TocPick::Card; }
        if settings { self.lesson_settings = true; }
        if turn_on { self.snapshot(); self.lesson_mut().show_toc = true; }
        if pick_toc { self.toc_pick = web_tabs::TocPick::Header; self.blk = None; }
        if let Some(si) = pick { self.sec = si; self.blk = None; self.toc_pick = web_tabs::TocPick::Card; }
        else if cleared && !pick_toc && open.is_none() { self.toc_pick = web_tabs::TocPick::Page; self.blk = None; }
        if let Some(si) = open { self.web_open_section(si); }
    }

    /// A close-enough picture of the published lesson; the real page is one click away in Preview.
    fn lesson_preview(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let lesson = self.lesson_ref().clone();
        let (name, theme) = { let p = self.project.as_ref().unwrap(); (p.name.clone(), p.theme.clone()) };
        let accent = { let c = theme.color(4); egui::Color32::from_rgb(c.0, c.1, c.2) };
        let mut pick: Option<(usize, Option<usize>)> = None;
        let shown = self.sec;
        egui::ScrollArea::vertical().id_salt("lesson-preview").auto_shrink([false, false]).show(ui, |ui| {
            let w = ui.available_width().min(760.0);
            let pad = ((ui.available_width() - w) / 2.0).max(0.0);
            ui.horizontal_top(|ui| {
                ui.add_space(pad);
                ui.vertical(|ui| {
                    ui.set_width(w);
                    ui.add_space(12.0);
                    // A section's tab shows that section alone.
                    for (si, s) in lesson.sections.iter().enumerate().filter(|(si, _)| *si == shown) {
                        let r = ui.add(egui::Label::new(egui::RichText::new(if s.title.is_empty() { "(untitled section)" } else { &s.title }).size(24.0).strong()).sense(egui::Sense::click()));
                        if r.clicked() { pick = Some((si, None)); }
                        if !s.summary.is_empty() { ui.label(egui::RichText::new(&s.summary).size(14.0).color(ui.visuals().weak_text_color())); }
                        ui.add_space(8.0);
                        for (bi, b) in s.blocks.iter().enumerate() {
                            let selected = self.sec == si && self.blk == Some(bi);
                            let stroke = if selected { egui::Stroke::new(2.0, accent) } else { egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color) };
                            let frame = egui::Frame::new().stroke(stroke).corner_radius(4).inner_margin(10).show(ui, |ui| {
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
                    ui.add(egui::Image::new(&tex).uv(uv).fit_to_exact_size(egui::vec2(nw * k, nh * k)).corner_radius(4));
                }
                None => { ui.add(egui::Image::new(&tex).max_width(max_w.min(ui.available_width())).corner_radius(4)); }
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

/// One group of a block's properties: a bar that opens and closes, the same as the groups of a media lesson's Properties panel.
fn sec(ui: &mut egui::Ui, id: &str, title: &str, open: bool, body: impl FnOnce(&mut egui::Ui)) {
    crate::ribbon::format_panel::section(ui, id, title, open, |ui| {
        ui.add_space(4.0);
        egui::Frame::new().inner_margin(egui::Margin::symmetric(8, 0)).show(ui, |ui| body(ui));
        ui.add_space(6.0);
    });
}

#[allow(clippy::too_many_arguments)]
fn block_editor(ui: &mut egui::Ui, id: &str, kind: &mut BlockKind, names: &std::collections::BTreeMap<String, String>, dims: &dyn Fn(&str) -> Option<(u32, u32)>, number_vars: &[(String, String)], gen: &mut dyn FnMut() -> String, want: &mut Option<LessonAsset>) -> bool {
    let mut changed = false;
    // Each kind of block remembers which of its groups was open.
    let scope = 10 + (kind.label().bytes().fold(0u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32)) % 200) as u8;
    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("format-bar-scope"), scope));
    match kind {
        BlockKind::Heading { text, level } => {
            sec(ui, "content", "Content", true, |ui| { changed |= single(ui, "Heading", text); });
            sec(ui, "appearance", "Appearance", false, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Size");
                    for (n, label) in [(1u8, "Large"), (2, "Medium"), (3, "Small")] { if ui.selectable_label(*level == n, label).clicked() { *level = n; changed = true; } }
                });
            });
        }
        BlockKind::Text { text } => {
            sec(ui, "content", "Content", true, |ui| { changed |= md_edit(ui, id, text, 10); ui.weak(markdown_hint()); });
        }
        BlockKind::Image { asset, alt, caption, width, decorative, crop } => {
            sec(ui, "picture", "Picture", true, |ui| {
                asset_row(ui, "Picture", asset.as_deref(), names, want, LessonAsset::Image, "Choose picture…");
                changed |= crop_row(ui, asset.as_deref(), crop, dims);
            });
            sec(ui, "content", "Caption", false, |ui| { changed |= single(ui, "Caption", caption); });
            sec(ui, "appearance", "Appearance", false, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Width");
                    for (w, label) in [(ImageWidth::Full, "Full"), (ImageWidth::Medium, "Medium"), (ImageWidth::Small, "Small")] { if ui.selectable_label(*width == w, label).clicked() { *width = w; changed = true; } }
                });
            });
            sec(ui, "access", "Accessibility", false, |ui| {
                changed |= ui.checkbox(decorative, "Decorative (screen readers skip it)").changed();
                if !*decorative { changed |= single(ui, "Alt text (describe the picture)", alt); }
            });
        }
        BlockKind::ImageText { asset, alt, text, image_right, crop } => {
            sec(ui, "picture", "Picture", true, |ui| {
                asset_row(ui, "Picture", asset.as_deref(), names, want, LessonAsset::Image, "Choose picture…");
                changed |= crop_row(ui, asset.as_deref(), crop, dims);
            });
            sec(ui, "content", "Text", false, |ui| { changed |= md_edit(ui, id, text, 8); });
            sec(ui, "appearance", "Layout", false, |ui| { changed |= ui.checkbox(image_right, "Picture on the right").changed(); });
            sec(ui, "access", "Accessibility", false, |ui| { changed |= single(ui, "Alt text (describe the picture)", alt); });
        }
        BlockKind::Video { asset, captions, caption } => {
            sec(ui, "picture", "Video", true, |ui| { asset_row(ui, "Video", asset.as_deref(), names, want, LessonAsset::Video, "Choose video…"); });
            sec(ui, "content", "Caption", false, |ui| { changed |= single(ui, "Caption", caption); });
            sec(ui, "access", "Accessibility", false, |ui| {
                asset_row(ui, "Captions", captions.as_deref(), names, want, LessonAsset::Captions, "Choose captions (.vtt)…");
                if captions.is_some() && ui.small_button("Remove captions").clicked() { *captions = None; changed = true; }
            });
        }
        BlockKind::Audio { asset, caption } => {
            sec(ui, "picture", "Audio", true, |ui| { asset_row(ui, "Audio", asset.as_deref(), names, want, LessonAsset::Audio, "Choose audio…"); });
            sec(ui, "content", "Caption", false, |ui| { changed |= single(ui, "Caption", caption); });
        }
        BlockKind::Button { label, url } => {
            sec(ui, "content", "Content", true, |ui| { changed |= single(ui, "Label", label); });
            sec(ui, "link", "Link", true, |ui| {
                changed |= single(ui, "Address (https:// or mailto:)", url);
                if !(url.starts_with("https://") || url.starts_with("http://") || url.starts_with("mailto:")) { ui.colored_label(egui::Color32::from_rgb(200, 60, 40), "The address must start with https://, http:// or mailto:."); }
            });
        }
        BlockKind::Continue { label } => {
            sec(ui, "content", "Content", true, |ui| {
                changed |= single(ui, "Button label", label);
                ui.weak("Hides everything below until the learner presses the button.");
            });
        }
        BlockKind::Quote { text, by } => {
            sec(ui, "content", "Content", true, |ui| {
                ui.label("Quote");
                changed |= ui.add(egui::TextEdit::multiline(text).desired_rows(4).desired_width(f32::INFINITY)).changed();
                changed |= single(ui, "Said by", by);
            });
        }
        BlockKind::Statement { text } => {
            sec(ui, "content", "Content", true, |ui| {
                ui.label("Statement");
                changed |= ui.add(egui::TextEdit::multiline(text).desired_rows(3).desired_width(f32::INFINITY)).changed();
            });
        }
        BlockKind::Items { style, items } => {
            sec(ui, "appearance", "Layout", false, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Layout");
                    combo(("items-style", id)).selected_text(style.label()).show_ui(ui, |ui| {
                        for s in ItemsStyle::ALL { if ui.selectable_label(*style == s, s.label()).clicked() { *style = s; changed = true; } }
                    });
                });
            });
            let (title_label, body_label) = match style { ItemsStyle::Timeline => ("Date or label", "Details"), ItemsStyle::Flashcards => ("Front of card", "Back of card"), _ => ("Title", "Content") };
            sec(ui, "content", "Items", true, |ui| {
                let mut remove = None;
                let mut swap: Option<(usize, usize)> = None;
                let n = items.len();
                for (i, it) in items.iter_mut().enumerate() {
                    if i > 0 { ui.separator(); }
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
            });
        }
        BlockKind::KnowledgeCheck(q) => {
            let mut qwant: Option<AssetWant> = None;
            sec(ui, "content", "Question", true, |ui| { changed |= question_editor(ui, q, gen, &mut qwant, number_vars); });
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
    pub(super) fn lesson_properties(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
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
        let toc = if self.toc_showing() { self.toc_pick } else { web_tabs::TocPick::Nothing };
        let toc_page = toc == web_tabs::TocPick::Page;
        let toc = matches!(toc, web_tabs::TocPick::Page | web_tabs::TocPick::Header);
        let mut lesson = self.project.as_mut().unwrap().lesson.take().unwrap();
        let mut changed = false;
        let mut want: Option<LessonAsset> = None;
        let mut section_action: Option<u8> = None; // 0 up, 1 down, 2 delete
        let mut title = self.project.as_ref().unwrap().name.clone();
        let n_sections = lesson.sections.len();
        egui::ScrollArea::vertical().id_salt("lesson-props-scroll").show(ui, |ui| {
            if toc {
                ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("format-bar-scope"), 8u8));
                ui.strong(if toc_page { "Table of contents" } else { "Lesson header" });
                ui.add_space(4.0);
                if toc_page {
                    self::sec(ui, "toc", "Table of contents", true, |ui| {
                        changed |= ui.checkbox(&mut lesson.show_toc, "Open on the table of contents").changed();
                        ui.weak("Off, learners start in the first section.");
                        changed |= ui.checkbox(&mut lesson.show_menu, "Show the section menu").changed();
                    });
                } else {
                    self::sec(ui, "banner", "Title banner", true, |ui| {
                        changed |= single(ui, "Lesson title", &mut title);
                        let mut has_cover = lesson.cover.is_some();
                        if ui.checkbox(&mut has_cover, "Show a title banner").changed() { lesson.cover = has_cover.then(Cover::default); changed = true; }
                        if let Some(c) = lesson.cover.as_mut() {
                            changed |= single(ui, "Subtitle", &mut c.subtitle);
                            ui.label("Description (under the title)");
                            changed |= ui.add(egui::TextEdit::multiline(&mut c.description).desired_rows(3).desired_width(f32::INFINITY)).changed();
                            asset_row(ui, "Picture", c.image.as_deref(), &names, &mut want, LessonAsset::CoverImage, "Choose picture…");
                            if c.image.is_some() && ui.small_button("Remove picture").clicked() { c.image = None; changed = true; }
                        }
                    });
                }
                return;
            }
            let Some(s) = lesson.sections.get_mut(sec) else { return };
            match blk.and_then(|b| s.blocks.get_mut(b)) {
                None => {
                    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("format-bar-scope"), 9u8));
                    ui.strong(format!("Section {}", sec + 1));
                    ui.add_space(4.0);
                    self::sec(ui, "content", "Content", true, |ui| {
                        changed |= single(ui, "Title (shown in the menu)", &mut s.title);
                        ui.label("Summary (shown on the title page)");
                        changed |= ui.add(egui::TextEdit::multiline(&mut s.summary).desired_rows(3).desired_width(f32::INFINITY)).changed();
                    });
                    self::sec(ui, "arrange", "Arrange", false, |ui| {
                        ui.horizontal(|ui| {
                            if ui.add_enabled(sec > 0, egui::Button::new("Move up")).clicked() { section_action = Some(0); }
                            if ui.add_enabled(sec + 1 < n_sections, egui::Button::new("Move down")).clicked() { section_action = Some(1); }
                            if ui.add_enabled(n_sections > 1, egui::Button::new("Delete section")).clicked() { section_action = Some(2); }
                        });
                    });
                    ui.add_space(4.0);
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
        if title != self.project.as_ref().unwrap().name { self.project.as_mut().unwrap().name = title; }
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
        crate::dialog_window(ctx, "Lesson settings").open(&mut open).default_width(420.0).show(ctx, |ui| {
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
                changed |= ui.checkbox(&mut l.show_toc, "Open on the table of contents").changed();
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
            ui.label("Colors and fonts");
            ui.horizontal_wrapped(|ui| {
                for t in Theme::presets() {
                    if ui.selectable_label(p.theme.name == t.name, &t.name).clicked() { p.theme = t; changed = true; }
                }
            });
            if let Some(l) = p.lesson.as_mut() {
                for (label, slot, idx) in [("Primary color (buttons, progress)", &mut l.primary, 4usize), ("Secondary color (highlights)", &mut l.secondary, 5)] {
                    ui.horizontal(|ui| {
                        let theme_c = p.theme.color(idx);
                        let mut c = slot.unwrap_or(theme_c);
                        if crate::ribbon::color_swatch(ui, &mut c) { *slot = Some(c); changed = true; }
                        ui.label(label);
                        if slot.is_some() && ui.small_button("Use theme color").clicked() { *slot = None; changed = true; }
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
