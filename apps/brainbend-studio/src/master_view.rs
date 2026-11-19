//! Layout Builder view.
//!
//! While it is open the master and the layouts are open as slides (`Project::open_layouts`): the editor's own canvas, ribbon and panels edit them
//! in place, after the real slides in the numbering, and what is done to them is copied into the master and the layouts every frame. The
//! project's slides are not touched, and what is done in the builder is part of the one undo history.
use super::*;

pub(super) use brainbend_doc::master::LAYOUT_SCENE as MASTER_SCENE;

/// What the Layout Builder remembers while it is open.
pub(super) struct MasterStash {
    /// The slide that was showing when it opened, to go back to.
    slide: usize,
}

impl App {
    pub(super) fn in_master_view(&self) -> bool {
        self.master_stash.is_some()
    }

    /// Where the master's slide is in the editor's numbering: the layouts' slides follow it.
    fn layout_base(&self) -> usize { self.project.as_ref().map(|p| p.slides.len()).unwrap_or(0) }

    pub(super) fn enter_master_view(&mut self) {
        if self.master_stash.is_some() { return; }
        self.close_pano_edit();
        let Some(p) = self.project.as_mut() else { return };
        p.open_layouts();
        let base = p.slides.len();
        self.master_stash = Some(MasterStash { slide: self.view.slide });
        self.layout_tab = true;
        self.view.slide = base;
        self.view.layer = 0;
        self.view.selected = None;
        self.view.multi.clear();
        self.scene_view = false;
        self.ribbon_tab = ribbon::OBJECTS_TAB;
        self.reset_visibility();
    }

    /// Close the Layout Builder and go back to the slide that was showing.
    pub(super) fn leave_master_view(&mut self) {
        let Some(stash) = self.master_stash.take() else { return };
        if let Some(p) = self.project.as_mut() {
            // A window of the layouts keeps them open; otherwise what is in them is copied back and they are closed.
            let wanted = self.editor_windows.iter().any(|w| w.layouts);
            let changed = if wanted { p.sync_layouts() } else { p.close_layouts() };
            if changed { self.dirty = true; }
        }
        self.view.slide = stash.slide.min(self.project.as_ref().map(|p| p.slides.len()).unwrap_or(1).saturating_sub(1));
        self.view.layer = 0;
        self.view.selected = None;
        self.view.multi.clear();
        self.ribbon_tab = ribbon::OBJECTS_TAB;
        self.reset_visibility();
    }

    /// After Undo or Redo the master's and layouts' slides are as they were in the step that came back: while the Layout Builder is closed they are not
    /// wanted, and while it is open they are opened again if the step is from before it opened.
    pub(super) fn settle_layouts(&mut self) {
        let open = self.layouts_wanted();
        let in_builder = self.master_stash.is_some();
        let Some(p) = self.project.as_mut() else { return };
        if open {
            p.open_layouts();
            if in_builder { self.view.slide = self.view.slide.min(p.slide_count().saturating_sub(1)).max(p.slides.len()); }
        } else {
            p.layout_slides.clear();
        }
    }

    /// The project as it should be saved or published: what was done to the master and layouts this frame included.
    pub(super) fn committed(&self) -> Option<Project> {
        let mut p = self.project.clone()?;
        p.sync_layouts();
        p.layout_slides.clear();
        Some(p)
    }

    pub(super) fn insert_layout(&mut self, copy_of: Option<usize>) {
        if !self.in_master_view() && !self.on_layout() { return; }
        self.snapshot();
        let id = self.new_id("layout");
        let (oid, tid) = (self.new_id("obj"), self.new_id("obj"));
        let base = self.layout_base();
        let p = self.project.as_mut().unwrap();
        let (w, h) = (p.slides[0].width, p.slides[0].height);
        let mut s = match copy_of {
            Some(i) if i > 0 && i < p.layout_slides.len() => {
                let mut s = p.layout_slides[i].clone();
                s.name = format!("{} copy", s.name);
                for (n, o) in s.layers[0].objects.iter_mut().enumerate() { o.id = format!("{id}-{n}"); }
                s
            }
            _ => {
                let mut s = Slide::new("", "Custom Layout", w, h);
                let m = w * 0.06;
                s.layers[0].objects = vec![brainbend_doc::master::placeholder_object(&tid, PlaceholderKind::Heading2, &p.theme, m, h * 0.06, w - 2.0 * m, h * 0.16)];
                s
            }
        };
        let _ = oid;
        s.id = format!("m:{id}");
        s.layout = Some(id.clone());
        s.scene = MASTER_SCENE.into();
        let show_master = copy_of.and_then(|i| p.layout_slides.get(i)).and_then(|s| s.layout.clone()).and_then(|l| p.layout(&l).map(|l| l.show_master)).unwrap_or(true);
        let mut meta = SlideLayout::new(&id, &s.name);
        meta.show_master = show_master;
        p.layouts.push(meta);
        p.layout_slides.push(s);
        self.view.slide = base + p.layout_slides.len() - 1;
        self.view.layer = 0;
        self.view.selected = None;
        self.reset_visibility();
        self.dirty = true;
    }

    pub(super) fn delete_layout(&mut self, i: usize) {
        if (!self.in_master_view() && !self.on_layout()) || i == 0 { return; }
        let Some(p) = self.project.as_ref() else { return };
        if i >= p.layout_slides.len() { return; }
        self.snapshot();
        let base = self.layout_base();
        let p = self.project.as_mut().unwrap();
        p.layout_slides.remove(i);
        self.view.slide = base + i.min(p.layout_slides.len() - 1);
        self.view.layer = 0;
        self.view.selected = None;
        self.reset_visibility();
        self.dirty = true;
    }

    pub(super) fn insert_placeholder(&mut self, kind: PlaceholderKind) {
        let id = self.new_id("obj");
        let Some(p) = self.project.as_ref() else { return };
        let (w, h) = (p.slide(self.view.slide).width, p.slide(self.view.slide).height);
        let m = w * 0.06;
        let mut o = brainbend_doc::master::placeholder_object(&id, kind, &p.theme, m, h * 0.3, w - 2.0 * m, h * 0.3);
        o.name = format!("{} placeholder", kind.label());
        self.insert(o);
    }

    /// Swap layout `i` with its neighbour (never the "Layout Master" stand-in at the top).
    pub(super) fn move_layout(&mut self, i: usize, down: bool) {
        if !self.in_master_view() && !self.on_layout() { return; }
        let Some(p) = self.project.as_ref() else { return };
        let j = if down { i + 1 } else { i.wrapping_sub(1) };
        if i == 0 || j == 0 || i >= p.layout_slides.len() || j >= p.layout_slides.len() { return; }
        self.snapshot();
        let base = self.layout_base();
        self.project.as_mut().unwrap().layout_slides.swap(i, j);
        self.view.slide = base + j;
        self.dirty = true;
    }

    /// Give layout `i` a new name.
    pub(super) fn rename_layout(&mut self, i: usize, name: &str) {
        // Not trimmed here: a space typed at the end must stay while the name is being typed.
        if name.trim().is_empty() || i == 0 { return; }
        let Some(p) = self.project.as_mut() else { return };
        if i >= p.layout_slides.len() || p.layout_slides[i].name == name { return; }
        p.layout_slides[i].name = name.to_string();
        self.dirty = true;
    }

    /// Left panel in the Layout Builder: the "Layout Master" stand-in, then each layout, like the slide list outside.
    pub(super) fn master_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.add_space(4.0);
        ui.label(egui::RichText::new("Layout Builder").strong());
        ui.add_space(2.0);
        // The layouts are numbered from 0 here (0 is the master); the editor numbers their slides after the project's own.
        let base = self.layout_base();
        let n = self.project.as_ref().map(|p| p.layout_slides.len()).unwrap_or(0);
        let tw = (ui.available_width() - 24.0).clamp(80.0, 150.0);
        let mut pick = None;
        let mut action: Option<(u8, usize)> = None;
        let list_h = (ui.available_height() - 44.0).max(80.0);
        // Right-clicking anywhere in the list adds a layout; the thumbnails take their own clicks.
        let list_bg = ui.interact(egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), list_h)), ui.id().with("layouts-bg"), egui::Sense::click());
        list_bg.context_menu(|ui| { if ui.button("New layout").clicked() { action = Some((0, 0)); ui.close(); } });
        let renaming = self.layout_rename;
        let mut rename_to: Option<(usize, String)> = None;
        let mut stop_rename = false;
        let mut start_rename = None;
        egui::ScrollArea::vertical().id_salt("layouts").max_height(list_h).auto_shrink([false, false]).show(ui, |ui| {
            for i in 0..n {
                if i == 1 {
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("Layouts").weak().size(12.0));
                }
                let name = self.project.as_ref().unwrap().layout_slides[i].name.clone();
                let sel = base + i == self.view.slide;
                let thumb = self.slide_thumbnail(ui, ctx, base + i, tw, sel);
                if thumb.clicked() || thumb.secondary_clicked() { pick = Some(i); }
                if thumb.double_clicked() && i > 0 { start_rename = Some(i); }
                thumb.context_menu(|ui| {
                    if ui.button("New layout").clicked() { action = Some((0, i)); ui.close(); }
                    if ui.add_enabled(i > 0, egui::Button::new("Duplicate")).clicked() { action = Some((1, i)); ui.close(); }
                    if ui.add_enabled(i > 0, egui::Button::new("Rename")).clicked() { start_rename = Some(i); ui.close(); }
                    ui.add_enabled_ui(i > 1, |ui| { if ui.button("Move up").clicked() { action = Some((3, i)); ui.close(); } });
                    ui.add_enabled_ui(i > 0 && i + 1 < n, |ui| { if ui.button("Move down").clicked() { action = Some((4, i)); ui.close(); } });
                    ui.separator();
                    if ui.add_enabled(i > 0, egui::Button::new("Delete")).clicked() { action = Some((2, i)); ui.close(); }
                });
                if renaming == Some(i) {
                    let mut text = name.clone();
                    let r = ui.add(egui::TextEdit::singleline(&mut text).desired_width(tw));
                    if !r.has_focus() && !r.lost_focus() { r.request_focus(); }
                    if r.changed() { rename_to = Some((i, text)); }
                    if r.lost_focus() { stop_rename = true; }
                } else {
                    let l = ui.add(egui::Label::new(egui::RichText::new(name).size(12.0)).sense(egui::Sense::click()));
                    if l.double_clicked() && i > 0 { start_rename = Some(i); }
                }
                ui.add_space(4.0);
            }
        });
        ui.horizontal(|ui| {
            if ui.button("+ Add layout").clicked() { action = Some((0, 0)); }
            let cur = self.view.slide.saturating_sub(base);
            ui.menu_button("Layout", |ui| {
                if ui.add_enabled(cur > 0, egui::Button::new("Duplicate")).clicked() { action = Some((1, cur)); ui.close(); }
                if ui.add_enabled(cur > 0, egui::Button::new("Rename")).clicked() { start_rename = Some(cur); ui.close(); }
                ui.add_enabled_ui(cur > 1, |ui| { if ui.button("Move up").clicked() { action = Some((3, cur)); ui.close(); } });
                ui.add_enabled_ui(cur > 0 && cur + 1 < n, |ui| { if ui.button("Move down").clicked() { action = Some((4, cur)); ui.close(); } });
                ui.separator();
                if ui.add_enabled(cur > 0, egui::Button::new("Delete")).clicked() { action = Some((2, cur)); ui.close(); }
            });
        });
        if let Some(i) = pick {
            self.view.slide = base + i;
            self.view.layer = 0;
            self.view.selected = None;
            self.reset_visibility();
        }
        if let Some((i, text)) = rename_to {
            if self.layout_rename == Some(i) {
                // One undo step for the whole rename: the snapshot is taken when it starts.
                self.rename_layout(i, &text);
            }
        }
        if stop_rename {
            if let Some(i) = self.layout_rename.take() {
                let t = self.project.as_ref().and_then(|p| p.layout_slides.get(i)).map(|s| s.name.trim().to_string());
                if let Some(t) = t { self.rename_layout(i, &t); }
            }
        }
        if let Some(i) = start_rename {
            self.snapshot();
            self.layout_rename = Some(i);
        }
        match action {
            Some((0, _)) => self.insert_layout(None),
            Some((1, i)) => self.insert_layout(Some(i)),
            Some((2, i)) => self.delete_layout(i),
            Some((3, i)) => self.move_layout(i, false),
            Some((4, i)) => self.move_layout(i, true),
            _ => {}
        }
    }
}
