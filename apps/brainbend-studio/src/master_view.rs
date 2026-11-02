//! Slide Master view.
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
        let mut slides = vec![stand_in("m:master".into(), "Slide Master", &p.master.background, &p.master.objects, None)];
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
        let stash = MasterStash { entry, slide: self.slide, undo: std::mem::take(&mut self.undo), redo: std::mem::take(&mut self.redo) };
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
            let changed = {
                let mut real = p.clone();
                merge(&mut real, &stash.entry);
                let mut before = p.clone();
                stash.entry.clone().swap_into(&mut before);
                real.to_json().ok() != before.to_json().ok()
            };
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
                s.layers[0].objects = vec![brainbend_doc::master::placeholder_object(&tid, PlaceholderKind::Title, &p.theme, m, h * 0.06, w - 2.0 * m, h * 0.16)];
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

    /// Left panel in master view: the master, then each layout.
    pub(super) fn master_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.add_space(4.0);
        ui.label(egui::RichText::new("Slide Master").strong());
        ui.add_space(2.0);
        let n = self.project.as_ref().map(|p| p.slides.len()).unwrap_or(0);
        let tw = (ui.available_width() - 24.0).clamp(80.0, 150.0);
        let mut pick = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for i in 0..n {
                if i == 1 {
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("Layouts").weak().size(12.0));
                }
                let name = self.project.as_ref().unwrap().slides[i].name.clone();
                let sel = i == self.slide;
                if self.slide_thumbnail(ui, ctx, i, tw, sel).clicked() { pick = Some(i); }
                ui.label(egui::RichText::new(name).size(12.0));
                ui.add_space(4.0);
            }
        });
        if let Some(i) = pick {
            self.slide = i;
            self.layer = 0;
            self.selected = None;
            self.reset_visibility();
        }
    }
}
