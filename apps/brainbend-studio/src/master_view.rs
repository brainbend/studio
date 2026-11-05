//! Layout Builder view.
//!
//! While it is open, the project's slides are swapped for editable stand-ins: slide 0 is the
//! master and each following slide is one layout. That lets the ordinary canvas, ribbon and
//! property panels edit them. Closing the view (or saving, exporting, ...) writes the stand-ins
//! back into `Project::master` / `Project::layouts` and restores the real slides.
use super::*;

pub(super) const MASTER_SCENE: &str = "master-scene";

/// What to restore when the view closes.
pub(super) struct MasterStash {
    /// The project exactly as it was when the view opened (also the undo step for the whole visit).
    entry: Snap,
    slide: usize,
    undo: Vec<Snap>,
    redo: Vec<Snap>,
    /// The layout whose name is being edited in the left panel.
    rename: Option<usize>,
}

/// Turn a project in master-view form back into a real one.
fn merge(p: &mut Project, entry: &Snap) {
    let master = Master { background: p.slides[0].background.clone(), objects: p.slides[0].layers[0].objects.clone() };
    let layouts: Vec<SlideLayout> = p.slides[1..]
        .iter()
        .filter_map(|s| {
            let id = s.layout.clone()?;
            let show_master = p.layouts.iter().find(|l| l.id == id).is_none_or(|l| l.show_master);
            Some(SlideLayout { id, name: s.name.clone(), background: s.background.clone(), objects: s.layers[0].objects.clone(), show_master })
        })
        .collect();
    p.slides = entry.slides.clone();
    p.scenes = entry.meta.scenes.clone();
    for s in p.slides.iter_mut() {
        let Some(id) = s.layout.clone() else { continue };
        match (layouts.iter().find(|l| l.id == id), entry.meta.layouts.iter().find(|l| l.id == id)) {
            (Some(new), Some(old)) => brainbend_doc::master::propagate(old, new, s),
            (None, _) => s.layout = None,
            _ => {}
        }
    }
    p.layouts = layouts;
    p.master = master;
}

impl App {
    pub(super) fn in_master_view(&self) -> bool {
        self.master_stash.is_some()
    }

    pub(super) fn enter_master_view(&mut self) {
        if self.master_stash.is_some() { return; }
        let Some(p) = self.project.as_mut() else { return };
        let entry = Snap::of(p);
        let (w, h) = (p.slides[0].width, p.slides[0].height);
        let stand_in = |id: String, name: &str, bg: &Option<Background>, objects: &[Object], layout: Option<String>| {
            let mut s = Slide::new(&id, name, w, h);
            s.background = bg.clone();
            s.layers[0].objects = objects.to_vec();
            s.layout = layout;
            s.scene = MASTER_SCENE.into();
            s
        };
        let mut slides = vec![stand_in("m:master".into(), "All layouts", &p.master.background, &p.master.objects, None)];
        for l in &p.layouts {
            slides.push(stand_in(format!("m:{}", l.id), &l.name, &l.background, &l.objects, Some(l.id.clone())));
        }
        // The layouts keep only their flags here; their content now lives in the stand-in slides.
        for l in p.layouts.iter_mut() {
            l.objects.clear();
            l.background = None;
        }
        p.slides = slides;
        p.scenes = vec![Scene { id: MASTER_SCENE.into(), name: "Master".into(), bank: None }];
        let stash = MasterStash { entry, slide: self.slide, undo: std::mem::take(&mut self.undo), redo: std::mem::take(&mut self.redo), rename: None };
        self.master_stash = Some(stash);
        self.slide = 0;
        self.layer = 0;
        self.selected = None;
        self.multi.clear();
        self.scene_view = false;
        self.ribbon_tab = ribbon::MASTER_TAB;
        self.reset_visibility();
    }

    /// Write the stand-ins back and return to the normal editor.
    pub(super) fn leave_master_view(&mut self) {
        let Some(stash) = self.master_stash.take() else { return };
        if let Some(p) = self.project.as_mut() {
            // Only the layouts and slides can change here, so compare those and leave the media out of the copies.
            let assets = std::mem::take(&mut p.assets);
            let changed = {
                let key = |p: &Project| serde_json::to_string(&(&p.slides, &p.scenes, &p.layouts, &p.master)).ok();
                let mut real = p.clone();
                merge(&mut real, &stash.entry);
                let mut before = p.clone();
                stash.entry.clone().swap_into(&mut before);
                key(&real) != key(&before)
            };
            p.assets = assets;
            merge(p, &stash.entry);
            self.undo = stash.undo;
            self.redo = stash.redo;
            if changed {
                self.undo.push(stash.entry);
                self.redo.clear();
                self.dirty = true;
            }
        }
        self.slide = stash.slide.min(self.project.as_ref().map(|p| p.slides.len()).unwrap_or(1).saturating_sub(1));
        self.layer = 0;
        self.selected = None;
        self.multi.clear();
        self.ribbon_tab = 1;
        self.reset_visibility();
    }

    /// The project as it should be saved or published: master-view stand-ins folded back in.
    pub(super) fn committed(&self) -> Option<Project> {
        let p = self.project.as_ref()?;
        match &self.master_stash {
            None => Some(p.clone()),
            Some(st) => {
                let mut real = p.clone();
                merge(&mut real, &st.entry);
                Some(real)
            }
        }
    }

    /// Keep `Project::master` in step with the master stand-in so layout stand-ins show its shapes.
    pub(super) fn sync_master(&mut self) {
        if self.master_stash.is_none() { return; }
        let Some(p) = self.project.as_mut() else { return };
        p.master.background = p.slides[0].background.clone();
        p.master.objects = p.slides[0].layers[0].objects.clone();
    }

    pub(super) fn insert_layout(&mut self, copy_of: Option<usize>) {
        if !self.in_master_view() { return; }
        self.snapshot();
        let id = self.new_id("layout");
        let (oid, tid) = (self.new_id("obj"), self.new_id("obj"));
        let p = self.project.as_mut().unwrap();
        let (w, h) = (p.slides[0].width, p.slides[0].height);
        let mut s = match copy_of {
            Some(i) if i > 0 => {
                let mut s = p.slides[i].clone();
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
        let show_master = copy_of.and_then(|i| p.slides[i].layout.clone()).and_then(|l| p.layout(&l).map(|l| l.show_master)).unwrap_or(true);
        let mut meta = SlideLayout::new(&id, &s.name);
        meta.show_master = show_master;
        p.layouts.push(meta);
        p.slides.push(s);
        self.slide = p.slides.len() - 1;
        self.layer = 0;
        self.selected = None;
        self.reset_visibility();
        self.dirty = true;
    }

    pub(super) fn delete_layout(&mut self, i: usize) {
        if !self.in_master_view() || i == 0 { return; }
        let Some(p) = self.project.as_ref() else { return };
        if i >= p.slides.len() { return; }
        self.snapshot();
        let p = self.project.as_mut().unwrap();
        let gone = p.slides.remove(i);
        if let Some(id) = gone.layout { p.layouts.retain(|l| l.id != id); }
        self.slide = i.min(p.slides.len() - 1);
        self.layer = 0;
        self.selected = None;
        self.reset_visibility();
        self.dirty = true;
    }

    pub(super) fn insert_placeholder(&mut self, kind: PlaceholderKind) {
        let id = self.new_id("obj");
        let Some(p) = self.project.as_ref() else { return };
        let (w, h) = (p.slides[self.slide].width, p.slides[self.slide].height);
        let m = w * 0.06;
        let mut o = brainbend_doc::master::placeholder_object(&id, kind, &p.theme, m, h * 0.3, w - 2.0 * m, h * 0.3);
        o.name = format!("{} placeholder", kind.label());
        self.insert(o);
    }

    /// Swap layout `i` with its neighbour (never the "All layouts" stand-in at the top).
    pub(super) fn move_layout(&mut self, i: usize, down: bool) {
        if !self.in_master_view() { return; }
        let Some(p) = self.project.as_ref() else { return };
        let j = if down { i + 1 } else { i.wrapping_sub(1) };
        if i == 0 || j == 0 || i >= p.slides.len() || j >= p.slides.len() { return; }
        self.snapshot();
        self.project.as_mut().unwrap().slides.swap(i, j);
        self.slide = j;
        self.dirty = true;
    }

    /// Give layout `i` a new name.
    pub(super) fn rename_layout(&mut self, i: usize, name: &str) {
        // Not trimmed here: a space typed at the end must stay while the name is being typed.
        if name.trim().is_empty() || i == 0 { return; }
        let Some(p) = self.project.as_mut() else { return };
        if i >= p.slides.len() || p.slides[i].name == name { return; }
        p.slides[i].name = name.to_string();
        self.dirty = true;
    }

    /// Left panel in the Layout Builder: the "All layouts" stand-in, then each layout, like the slide list outside.
    pub(super) fn master_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.add_space(4.0);
        ui.label(egui::RichText::new("Layout Builder").strong());
        ui.add_space(2.0);
        let n = self.project.as_ref().map(|p| p.slides.len()).unwrap_or(0);
        let tw = (ui.available_width() - 24.0).clamp(80.0, 150.0);
        let mut pick = None;
        let mut action: Option<(u8, usize)> = None;
        let list_h = (ui.available_height() - 44.0).max(80.0);
        // Right-clicking anywhere in the list adds a layout; the thumbnails take their own clicks.
        let list_bg = ui.interact(egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), list_h)), egui::Id::new("layouts-bg"), egui::Sense::click());
        list_bg.context_menu(|ui| { if ui.button("New layout").clicked() { action = Some((0, 0)); ui.close(); } });
        let renaming = self.master_stash.as_ref().and_then(|s| s.rename);
        let mut rename_to: Option<(usize, String)> = None;
        let mut stop_rename = false;
        let mut start_rename = None;
        egui::ScrollArea::vertical().id_salt("layouts").max_height(list_h).auto_shrink([false, false]).show(ui, |ui| {
            for i in 0..n {
                if i == 1 {
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("Layouts").weak().size(12.0));
                }
                let name = self.project.as_ref().unwrap().slides[i].name.clone();
                let sel = i == self.slide;
                let thumb = self.slide_thumbnail(ui, ctx, i, tw, sel);
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
            let cur = self.slide;
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
            self.slide = i;
            self.layer = 0;
            self.selected = None;
            self.reset_visibility();
        }
        if let Some((i, text)) = rename_to {
            if self.master_stash.as_ref().is_some_and(|s| s.rename == Some(i)) {
                // One undo step for the whole rename: the snapshot is taken when it starts.
                self.rename_layout(i, &text);
            }
        }
        if stop_rename {
            if let Some(i) = self.master_stash.as_mut().and_then(|s| s.rename.take()) {
                let t = self.project.as_ref().and_then(|p| p.slides.get(i)).map(|s| s.name.trim().to_string());
                if let Some(t) = t { self.rename_layout(i, &t); }
            }
        }
        if let Some(i) = start_rename {
            self.snapshot();
            if let Some(s) = self.master_stash.as_mut() { s.rename = Some(i); }
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
