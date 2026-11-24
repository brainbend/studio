//! Scene View and slide tabs: the Scene View is the first tab (more can be opened with the "+"), and each slide opened from it
//! (or from the slide list) gets a tab of its own that remembers its layer, selection, zoom and bottom panel.
//!
//! The editor keeps working on "the current slide" (`App::view`: the slide, `layer`, `selected`, ...). A tab holds the view of
//! a slide that is not current: the active tab's view is copied into it at the end of every frame, and copied back
//! when the tab becomes current again. Tabs are keyed by slide id, so reordering slides does not disturb them and
//! deleting a slide closes its tab.
use super::*;
use crate::view::View;

#[derive(Clone)]
pub(crate) struct SlideTab {
    pub slide_id: String,
    /// What the editor holds for this slide while another is showing: its layer, selection, zoom, pan and the rest (see `view.rs`).
    pub(crate) view: View,
    bottom_tab: usize,
}

impl SlideTab {
    pub(crate) fn new(slide_id: String, view: View, bottom_tab: usize) -> SlideTab { SlideTab { slide_id, view, bottom_tab } }
}

impl App {
    /// The slide's number, written scene.position, e.g. "1.5".
    pub(crate) fn slide_number(&self, i: usize) -> String {
        let Some(p) = &self.project else { return String::new() };
        let Some(s) = p.slide_get(i) else { return String::new() };
        let si = p.scenes.iter().position(|c| c.id == s.scene).unwrap_or(0);
        let n = p.scene_slides(si).iter().position(|&x| x == i).map(|k| k + 1).unwrap_or(i + 1);
        format!("{}.{}", si + 1, n)
    }

    fn tab_for(&self, slide_idx: usize) -> SlideTab {
        SlideTab {
            slide_id: self.project.as_ref().map(|p| p.slide(slide_idx).id.clone()).unwrap_or_default(),
            view: self.view.clone(),
            bottom_tab: self.bottom_tab,
        }
    }

    /// Keep the tabs in step with the slides and with the slide being edited. Runs at the start of every frame.
    pub(super) fn sync_tabs(&mut self) {
        // Master view swaps the project's slides for its own, so the tabs are left alone while it is open.
        if self.in_master_view() { return; }
        let Some(p) = &self.project else { self.tabs.clear(); self.active_tab = None; return };
        let ids: Vec<String> = p.slides.iter().map(|s| s.id.clone()).collect();
        let active_id = self.active_tab.and_then(|k| self.tabs.get(k)).map(|t| t.slide_id.clone());
        self.tabs.retain(|t| ids.contains(&t.slide_id));
        self.active_tab = active_id.clone().and_then(|id| self.tabs.iter().position(|t| t.slide_id == id));
        if self.scene_view { self.active_tab = None; return; }
        let cur_idx = self.view.slide.min(ids.len() - 1);
        let cur = ids[cur_idx].clone();
        // A slide chosen in the slide list is shown in the tab that is showing now. It gets a tab of its own only when asked for
        // (a double-click, a middle-click or the slide's menu), when it is opened from the Scene View, or when the person has chosen
        // to open every slide in a new tab.
        let want_new = std::mem::take(&mut self.new_tab_request);
        let from_scene = std::mem::take(&mut self.scene_to_slide);
        if from_scene { self.scene_tab_becomes(&cur); }
        if self.active_tab.is_some_and(|k| self.tabs[k].slide_id == cur) { return; }
        if let Some(k) = self.tabs.iter().position(|t| t.slide_id == cur) {
            let t = self.tabs[k].clone();
            let slide = &self.project.as_ref().unwrap().slide(cur_idx);
            let layers = slide.layers.len();
            // The tab's view comes back, for the slide as it is now: a layer or an object that has gone is dropped.
            let mut v = t.view;
            v.slide = cur_idx;
            v.layer = v.layer.min(layers - 1);
            v.selected = v.selected.filter(|&(l, o)| slide.layers.get(l).is_some_and(|ly| o < ly.objects.len()));
            if v.selected.is_none() { v.multi.clear(); }
            let visible_ok = v.visible.len() == layers;
            self.view = v;
            self.bottom_tab = t.bottom_tab;
            if !visible_ok { self.reset_visibility(); }
            self.active_tab = Some(k);
        } else {
            let t = self.tab_for(cur_idx);
            let own = want_new || from_scene || self.new_tab_each || self.tabs.is_empty();
            match if own { None } else { Some(self.active_tab.unwrap_or(0)) } {
                Some(k) => { self.tabs[k] = t; self.active_tab = Some(k); }
                None => { self.tabs.push(t); self.active_tab = Some(self.tabs.len() - 1); }
            }
        }
    }

    /// Copy the current view state into the active tab. Runs at the end of every frame; a frame in which the slide
    /// changed is skipped, since by then the view fields already belong to the next slide.
    pub(super) fn save_active_tab(&mut self) {
        if self.in_master_view() || self.scene_view { return; }
        let (Some(k), Some(p)) = (self.active_tab, self.project.as_ref()) else { return };
        let cur = p.slide(self.view.slide.min(p.slides.len() - 1)).id.clone();
        if self.tabs.get(k).is_some_and(|t| t.slide_id == cur) {
            let t = self.tab_for(self.view.slide.min(p.slides.len() - 1));
            self.tabs[k] = t;
        }
    }

    /// Run `f` as if the view of tab `k` were the current one, then put everything back: whatever `f` did to the view stays in the tab. This is how a
    /// tab that is not the one showing (a window of its own, later: see #74) is drawn and handled while the rest of the app reads `self.view`.
    /// For the tab that is showing, `f` just runs. `None` when there is no such tab or its slide has gone.
    #[allow(dead_code)]
    pub(crate) fn with_tab_view<R>(&mut self, k: usize, f: impl FnOnce(&mut App) -> R) -> Option<R> {
        if k >= self.tabs.len() { return None; }
        if self.active_tab == Some(k) { return Some(f(self)); }
        let slide = self.project.as_ref()?.slides.iter().position(|s| s.id == self.tabs[k].slide_id)?;
        let mut other = self.tabs[k].view.clone();
        other.slide = slide;
        std::mem::swap(&mut self.view, &mut other);
        let r = f(self);
        std::mem::swap(&mut self.view, &mut other);
        self.tabs[k].view = other;
        Some(r)
    }

    /// A new Scene View tab, at the right end of the strip, and the one showing.
    pub(super) fn new_scene_tab(&mut self) {
        let key = format!("scene:{}", self.scene_next);
        self.scene_next += 1;
        self.scene_tabs.get_or_insert_with(Vec::new).push(key.clone());
        self.scene_active = key;
    }

    /// The Scene View tab that was showing turns into the tab of a slide: it gives up its place in the strip to it.
    fn scene_tab_becomes(&mut self, slide_id: &str) {
        let key = self.scene_active.clone();
        let Some(tabs) = self.scene_tabs.as_mut() else { return };
        let Some(n) = tabs.iter().position(|k| *k == key) else { return };
        tabs.remove(n);
        let slide_key = format!("s:{slide_id}");
        if self.strip_order.contains(&slide_key) { self.strip_order.retain(|k| *k != key); }
        else if let Some(p) = self.strip_order.iter().position(|k| *k == key) { self.strip_order[p] = slide_key; }
    }

    /// Move a slide's tab to another place among the slide tabs; the tab that is showing stays the one showing.
    #[cfg(test)]
    pub(super) fn move_tab(&mut self, from: usize, to: usize) {
        if from >= self.tabs.len() || to >= self.tabs.len() || from == to { return; }
        let active = self.active_tab.and_then(|k| self.tabs.get(k)).map(|t| t.slide_id.clone());
        let t = self.tabs.remove(from);
        self.tabs.insert(to, t);
        self.active_tab = active.and_then(|id| self.tabs.iter().position(|t| t.slide_id == id));
        // The other tabs of the strip keep their places; the slide tabs fill the places the slides had, in their new order.
        let mut ids = self.tabs.iter().map(|t| format!("s:{}", t.slide_id));
        for k in self.strip_order.iter_mut().filter(|k| k.starts_with("s:")) {
            if let Some(n) = ids.next() { *k = n; }
        }
    }

    /// Move a tab of the strip (a slide, the Layout Builder or a 360° image) to another place; `from` and `to` are places in the strip.
    pub(super) fn move_strip(&mut self, from: usize, to: usize) {
        if from >= self.strip_order.len() || to >= self.strip_order.len() || from == to { return; }
        let k = self.strip_order.remove(from);
        self.strip_order.insert(to, k);
        // The slide tabs follow the strip's order.
        let active = self.active_tab.and_then(|k| self.tabs.get(k)).map(|t| t.slide_id.clone());
        let order = self.strip_order.clone();
        self.tabs.sort_by_key(|t| order.iter().position(|k| *k == format!("s:{}", t.slide_id)).unwrap_or(usize::MAX));
        self.active_tab = active.and_then(|id| self.tabs.iter().position(|t| t.slide_id == id));
    }

    pub(super) fn close_tab(&mut self, k: usize) {
        if k >= self.tabs.len() { return; }
        let was_active = self.active_tab == Some(k);
        self.tabs.remove(k);
        if !was_active {
            if let Some(a) = self.active_tab { if a > k { self.active_tab = Some(a - 1); } }
            return;
        }
        self.active_tab = None;
        // Go to the neighbour on the left (or right), else back to the Scene View.
        let next = if k > 0 { Some(k - 1) } else if !self.tabs.is_empty() { Some(0) } else { None };
        match next.and_then(|n| self.tabs.get(n)).map(|t| t.slide_id.clone()) {
            Some(id) => {
                if let Some(i) = self.project.as_ref().and_then(|p| p.slides.iter().position(|s| s.id == id)) { self.view.slide = i; self.scene_view = false; }
            }
            None => self.scene_view = true,
        }
    }

    /// The tab strip under the ribbon: SCENE VIEW, one tab per open slide, then the tabs of the Layout Builder and of a 360° image.
    pub(super) fn doc_tabs(&mut self, root: &mut egui::Ui) {
        if self.project.is_none() || self.file_view { return; }
        if self.project.as_ref().is_some_and(|p| p.is_web()) { return; }
        let master = self.in_master_view();
        let pano_open = self.pano.edit.is_some();
        // In the Layout Builder or the 360° editing mode, the tab of that mode is the active one and no slide's is.
        let in_mode = master || pano_open;
        let pano_name = self.pano_tab_name();
        let mut close_pano = false;
        let mut open_pano = false;
        let mut close_builder = false;
        let mut open_builder = false;
        let mut go_story = false;
        let mut go_slide: Option<String> = None;
        let mut close: Option<usize> = None;
        let mut close_others: Option<usize> = None;
        let mut close_all = false;
        let mut pop_out: Option<usize> = None;
        let mut pop_out_pano = false;
        let mut pop_out_layouts = false;
        let mut chip_keys: Vec<String> = vec![];
        let mut chip_tear: Option<usize> = None;
        let mut moved_tab: Option<(usize, usize)> = None;
        // The slides are the project's own while the Layout Builder is open too, so the tabs are named the same way.
        let labels: Vec<(String, String)> = self.tabs.iter().filter_map(|t| {
            let i = self.project.as_ref()?.slides.iter().position(|s| s.id == t.slide_id)?;
            Some((t.slide_id.clone(), format!("{} {}", self.slide_number(i), self.project.as_ref()?.slide(i).name)))
        }).collect();
        // The strip's order: what was kept, then whatever is new. The tabs of the Layout Builder and of a 360° image can be
        // dragged among the slide tabs like any other.
        if self.scene_tabs.is_none() { self.scene_tabs = Some(vec![]); self.new_scene_tab(); }
        // The Scene View is showing, so one of its tabs has to be there.
        if self.scene_view && !in_mode && self.scene_tabs.as_ref().is_some_and(|t| t.is_empty()) { self.new_scene_tab(); }
        let scene_keys: Vec<String> = self.scene_tabs.clone().unwrap_or_default();
        if !scene_keys.contains(&self.scene_active) { self.scene_active = scene_keys.first().cloned().unwrap_or_default(); }
        let mut present: Vec<String> = scene_keys.clone();
        present.extend(self.tabs.iter().map(|t| format!("s:{}", t.slide_id)));
        if master || self.layout_tab { present.push("layout".into()); }
        if pano_name.is_some() { present.push("pano".into()); }
        let first_run = self.strip_order.is_empty();
        let mut order: Vec<String> = self.strip_order.iter().filter(|k| present.contains(k)).cloned().collect();
        for k in present { if !order.contains(&k) { if k.starts_with("scene:") && first_run { order.insert(0, k); } else { order.push(k); } } }
        let first_slide = order.iter().find_map(|k| k.strip_prefix("s:").map(String::from));
        let mut close_scene: Option<String> = None;
        let mut pop_scene = false;
        let mut open_scene = false;
        let mut scene_pick: Option<String> = None;
        self.strip_order = order.clone();
        // One flat band that belongs to the ribbon, with a border line under it that the active tab sits on.
        let dark = root.visuals().dark_mode;
        let band = if dark { egui::Color32::from_gray(46) } else { egui::Color32::WHITE };
        let line = root.visuals().selection.bg_fill;
        // The tab showing in the main window is gray while an editor window is the one the panels act on.
        root.ctx().data_mut(|d| d.insert_temp(egui::Id::new("doc-tab-dim"), self.focus_window.is_some_and(|f| self.editor_windows.iter().any(|w| w.id == f && w.pano.is_none()))));
        let strip_panel = egui::Panel::top("doc-tabs").frame(egui::Frame::new().fill(band).inner_margin(egui::Margin { left: 8, right: 8, top: 0, bottom: 0 })).show_separator_line(false).show(root, |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let row = ui.horizontal(|ui| {
                let mut slide_chips: Vec<egui::Response> = vec![];
                for key in &order {
                    if key.starts_with("scene:") {
                        let shown = self.scene_view && !in_mode && *key == self.scene_active;
                        // Closing it needs somewhere to go: another tab, unless it is not the one showing.
                        let closable = order.len() > 1 && (!shown || !self.tabs.is_empty() || scene_keys.len() > 1);
                        let (r, closed) = tab_chip(ui, Some(ribbon::STACK), "SCENE VIEW", shown, closable, 160.0);
                        if closed { close_scene = Some(key.clone()); } else if r.clicked() { go_story = true; scene_pick = Some(key.clone()); }
                        r.context_menu(|ui| { if ui.button("Open in new window").clicked() { close_scene = Some(key.clone()); pop_scene = true; ui.close(); } });
                        slide_chips.push(r);
                        chip_keys.push(key.clone());
                        continue;
                    }
                    if key == "layout" {
                        // The Layout Builder is a tab of its own. It stays when another tab is chosen (the layouts
                        // are saved on the way out) and opens again when clicked; its × closes it.
                        let (r, closed) = tab_chip(ui, Some(ribbon::LAYOUT), "LAYOUT BUILDER", master, true, 176.0);
                        if closed { close_builder = true; }
                        else if r.clicked() && !master { open_builder = true; }
                        r.context_menu(|ui| { if ui.button("Open in new window").clicked() { pop_out_layouts = true; ui.close(); } });
                        slide_chips.push(r);
                        chip_keys.push(key.clone());
                        continue;
                    }
                    if key == "pano" {
                        // A 360° image's tab works the same way: it stays until its × is pressed and opens the editing mode when clicked.
                        let Some(name) = &pano_name else { continue };
                        let tw = ui.fonts_mut(|f| f.layout_no_wrap(name.clone(), egui::FontId::proportional(13.0), egui::Color32::WHITE).size().x);
                        let (r, closed) = tab_chip(ui, Some(ribbon::PANORAMA), name, pano_open, true, (tw + 82.0).clamp(130.0, 230.0));
                        if closed { close_pano = true; }
                        else if r.clicked() && !pano_open { open_pano = true; }
                        r.context_menu(|ui| { if ui.button("Open in new window").clicked() { pop_out_pano = true; ui.close(); } });
                        slide_chips.push(r);
                        chip_keys.push(key.clone());
                        continue;
                    }
                    let Some(k) = self.tabs.iter().position(|t| format!("s:{}", t.slide_id) == *key) else { continue };
                    let Some((id, label)) = labels.iter().find(|(i, _)| format!("s:{i}") == *key) else { continue };
                    let active = !in_mode && !self.scene_view && self.active_tab == Some(k);
                    // As wide as its name needs, so the strip is not mostly gaps.
                    let tw = ui.fonts_mut(|f| f.layout_no_wrap(label.clone(), egui::FontId::proportional(13.0), egui::Color32::WHITE).size().x);
                    let (r, closed) = tab_chip(ui, None, label, active, true, (tw + 58.0).clamp(96.0, 190.0));
                    // Inside the Layout Builder the slide tabs are named too, since nothing else on screen repeats them.
                    if in_mode { r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label.clone())); }
                    if closed || r.middle_clicked() { close = Some(k); } else if r.clicked() { go_slide = Some(id.clone()); }
                    slide_chips.push(r.clone());
                    chip_keys.push(key.clone());
                    if !in_mode { r.context_menu(|ui| {
                        if ui.button("Open in new window").clicked() { pop_out = Some(k); ui.close(); }
                        if ui.button("Close").clicked() { close = Some(k); ui.close(); }
                        if ui.add_enabled(labels.len() > 1, egui::Button::new("Close other tabs")).clicked() { close_others = Some(k); ui.close(); }
                        if ui.button("Close all tabs").clicked() { close_all = true; ui.close(); }
                    }); }
                }
                moved_tab = drag_reorder(ui, "strip", &slide_chips);
                chip_tear = ui.ctx().data_mut(|d| d.remove_temp::<usize>(egui::Id::new(("strip-tear", 0u8))));
                // A "+" at the end opens the Scene View; if its tab was closed, the tab comes back at the right end.
                let (plus, _) = tab_chip_sized(ui, None, "+", false, false, 34.0, 16.0);
                plus.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Open Scene View"));
                if plus.clicked() { open_scene = true; }
            });
            let full = egui::Rect::from_min_max(egui::pos2(row.response.rect.left() - 8.0, row.response.rect.bottom() - 2.0), egui::pos2(ui.clip_rect().right().max(row.response.rect.right() + 8.0), row.response.rect.bottom()));
            ui.painter().rect_filled(full, 0.0, line);
            open_under_active_tab(ui, full);
        });
        root.ctx().data_mut(|d| d.remove_temp::<bool>(egui::Id::new("doc-tab-dim")));
        // Where the strip is, so a slide's window carried over it can be put back.
        root.ctx().data_mut(|d| d.insert_temp(egui::Id::new("doc-strip-rect"), strip_panel.response.rect));
        if let Some((from, to)) = moved_tab { self.move_strip(from, to); }
        if let Some(k) = pop_out { self.pop_out_slide_tab(k); }
        else if pop_out_pano { self.pop_out_pano_tab(); }
        else if pop_out_layouts { self.pop_out_layout_tab(); return; }
        else if chip_tear.and_then(|n| chip_keys.get(n)).is_some_and(|key| key == "layout") {
            root.ctx().data_mut(|d| d.remove_temp::<usize>(egui::Id::new(("tab-drag", "strip"))));
            self.begin_layout_window_tear(&root.ctx().clone());
            return;
        }
        else if chip_tear.and_then(|n| chip_keys.get(n)).is_some_and(|key| key == "pano") {
            root.ctx().data_mut(|d| d.remove_temp::<usize>(egui::Id::new(("tab-drag", "strip"))));
            self.begin_pano_window_tear(&root.ctx().clone());
        }
        else if let Some(k) = chip_tear.and_then(|n| chip_keys.get(n)).and_then(|key| key.strip_prefix("s:")).and_then(|id| self.tabs.iter().position(|t| t.slide_id == id)) {
            root.ctx().data_mut(|d| d.remove_temp::<usize>(egui::Id::new(("tab-drag", "strip"))));
            self.begin_window_tear(&root.ctx().clone(), k);
        }
        if open_scene { self.new_scene_tab(); go_story = true; }
        else if let Some(k) = scene_pick { self.scene_active = k; }
        if let Some(k) = close_scene {
            let was_showing = self.scene_view && !in_mode && k == self.scene_active;
            if let Some(t) = self.scene_tabs.as_mut() { t.retain(|x| *x != k); }
            self.strip_order.retain(|x| *x != k);
            if was_showing {
                match first_slide {
                    Some(id) => { go_slide = Some(id); go_story = false; }
                    None => if let Some(n) = self.scene_tabs.as_ref().and_then(|t| t.first().cloned()) { self.scene_active = n; }
                }
            }
            if pop_scene { self.add_scene_window(); }
        }
        // Leaving the 360° editing mode, by any other tab or by its own ×, ends it first.
        if pano_open && (go_story || go_slide.is_some() || close_pano || open_builder || close_builder || close.is_some() || close_others.is_some() || close_all) { self.close_pano_edit(); }
        if close_pano { self.pano.tab = None; }
        else if open_pano { self.reopen_pano_tab(); return; }
        if master {
            // Leaving writes the layouts back first, so the slide indexes below are the real ones.
            if close_builder { self.layout_tab = false; self.leave_master_view(); }
            else if go_story { self.leave_master_view(); self.scene_view = true; }
            else if let Some(id) = go_slide {
                self.leave_master_view();
                if let Some(i) = self.project.as_ref().and_then(|p| p.slides.iter().position(|s| s.id == id)) { self.view.slide = i; self.scene_view = false; }
            }
            return;
        }
        if close_builder { self.layout_tab = false; }
        else if open_builder { self.enter_master_view(); return; }
        if close_all {
            self.tabs.clear();
            self.active_tab = None;
            self.scene_view = true;
        } else if let Some(k) = close_others {
            let keep = self.tabs.get(k).cloned();
            self.tabs = keep.into_iter().collect();
            self.active_tab = if self.tabs.is_empty() { None } else { Some(0) };
            if let Some(id) = self.tabs.first().map(|t| t.slide_id.clone()) {
                if let Some(i) = self.project.as_ref().and_then(|p| p.slides.iter().position(|s| s.id == id)) { self.view.slide = i; self.scene_view = false; }
            }
        } else if let Some(k) = close { self.close_tab(k); }
        else if go_story { self.scene_view = true; }
        else if let Some(id) = go_slide {
            if let Some(i) = self.project.as_ref().and_then(|p| p.slides.iter().position(|s| s.id == id)) {
                self.view.slide = i;
                self.scene_view = false;
            }
        }
    }
}

/// One tab: flat, so the active one is a block that meets the border line under the strip. The others have no fill
/// and muted text, with a faint fill on hover. The × shows on the active tab and on hover. Returns the tab and
/// whether its × was pressed.
pub(crate) fn tab_chip(ui: &mut egui::Ui, icon: Option<char>, label: &str, active: bool, closable: bool, width: f32) -> (egui::Response, bool) {
    tab_chip_ex(ui, icon, label, active, closable, width, if closable { 12.5 } else { 11.5 }, true)
}

/// `panel_tabs` for tabs that carry their own number, so a panel can leave some out and keep the numbers of the rest.
pub(super) fn panel_tabs_of(ui: &mut egui::Ui, tabs: &[(usize, String)], current: &mut usize) -> bool {
    panel_tabs_inner(ui, tabs, current, None).0
}

/// `panel_tabs_of` with tabs that can be dragged to a new place. Also returns the move made, as (place it was, place it goes to).
pub(super) fn panel_tabs_movable(ui: &mut egui::Ui, tabs: &[(usize, String)], current: &mut usize, salt: &str) -> Option<(usize, usize)> {
    panel_tabs_inner(ui, tabs, current, Some(salt)).1
}

fn panel_tabs_inner(ui: &mut egui::Ui, tabs: &[(usize, String)], current: &mut usize, salt: Option<&str>) -> (bool, Option<(usize, usize)>) {
    let mut changed = false;
    let mut chips: Vec<egui::Response> = vec![];
    let before = ui.spacing().item_spacing.x;
    ui.spacing_mut().item_spacing.x = 0.0;
    // The same gap above as between the ribbon and the document tabs, and the same grey band behind the tabs,
    // reaching the panel's edges.
    ui.add_space(PANEL_TABS_GAP);
    let band = if ui.visuals().dark_mode { egui::Color32::from_gray(46) } else { egui::Color32::WHITE };
    let top = ui.cursor().min.y;
    let full = ui.max_rect().expand2(egui::vec2(8.0, 0.0));
    let painter = ui.painter().with_clip_rect(full);
    painter.rect_filled(egui::Rect::from_min_max(egui::pos2(full.left(), top), egui::pos2(full.right(), top + 30.0)), 0.0, band);
    let row = ui.horizontal(|ui| {
        for (i, label) in tabs.iter().map(|(i, l)| (*i, l)) {
            let tw = ui.fonts_mut(|f| f.layout_no_wrap(label.clone(), egui::FontId::proportional(12.5), egui::Color32::WHITE).size().x);
            // The tab showing has a button to close it. Pulling the tab out (or its menu) makes it a window of its own.
            let buttons = (salt.is_some() || super::App::is_tool_id(i)) && *current == i;
            let (r, closed) = tab_chip_ex(ui, None, label, *current == i, buttons, tw + 28.0 + if buttons { 22.0 } else { 0.0 }, 12.5, false);
            let at = salt.unwrap_or("side");
            if closed { ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(("tab-closed", at)), i)); }
            r.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, *current == i, label));
            if r.clicked() && *current != i { *current = i; changed = true; }
            // Without dragging: the same moves from a menu, for the keyboard and for anyone who would rather not drag.
            if let Some(s) = salt {
                r.context_menu(|ui| {
                    for (n, text) in [(0u8, "Pop out into a window"), (1, "Move to the side panel"), (2, "Move to the bottom panel"), (3, "Move to the left panel"), (4, "Move to the top panel"), (5, "New column at the right"), (6, "New row at the bottom"), (7, "New group beside this one")] {
                        if ui.button(text).clicked() { ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(("tab-move", s)), (i, n))); ui.close(); }
                    }
                });
            }
            chips.push(r);
        }
    });
    let moved = salt.and_then(|s| drag_reorder(ui, s, &chips));
    let bottom = row.response.rect.bottom();
    painter.rect_filled(egui::Rect::from_min_max(egui::pos2(full.left(), bottom - 2.0), egui::pos2(full.right(), bottom)), 0.0, ui.visuals().selection.bg_fill);
    ui.spacing_mut().item_spacing.x = before;
    ui.add_space(PANEL_TABS_GAP + 2.0);
    (changed, moved)
}

/// The tab at the top of a tool's window of its own, drawn like the tabs of the panels: dragging it onto the side panel docks the tool
/// there. Returns the tab.
pub(super) fn window_tab(ui: &mut egui::Ui, label: &str, inset: f32) -> (egui::Response, bool) { window_tab_styled(ui, label, inset, false) }

/// `window_tab` for the window of a slide: its tab looks like the tabs of the strip it came from (lifted, with a bar on top), not like a panel's.
pub(super) fn slide_window_tab(ui: &mut egui::Ui, label: &str, inset: f32) -> (egui::Response, bool) { window_tab_styled(ui, label, inset, true) }

fn window_tab_styled(ui: &mut egui::Ui, label: &str, inset: f32, document: bool) -> (egui::Response, bool) {
    let before = ui.spacing().item_spacing.x;
    ui.spacing_mut().item_spacing.x = 0.0;
    let band = if ui.visuals().dark_mode { egui::Color32::from_gray(46) } else { egui::Color32::WHITE };
    let top = ui.cursor().min.y;
    let full = ui.max_rect().expand2(egui::vec2(8.0, 0.0));
    let painter = ui.painter().with_clip_rect(full);
    painter.rect_filled(egui::Rect::from_min_max(egui::pos2(full.left(), top), egui::pos2(full.right(), top + 30.0)), 0.0, band);
    let tw = ui.fonts_mut(|f| f.layout_no_wrap(label.to_string(), egui::FontId::proportional(12.5), egui::Color32::WHITE).size().x);
    // The strip is the window's title bar: its empty part moves the window, and `inset` leaves room for the system's window buttons.
    let strip = ui.interact(egui::Rect::from_min_max(egui::pos2(full.left(), top), egui::pos2(full.right(), top + 30.0)), ui.id().with("window-strip"), egui::Sense::click_and_drag());
    if strip.drag_started() { ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag); }
    let row = ui.horizontal(|ui| { ui.add_space(inset); if document { tab_chip(ui, None, label, true, true, (tw + 58.0).clamp(96.0, 220.0)) } else { tab_chip_sized(ui, None, label, true, true, tw + 44.0, 12.5) } });
    let bottom = row.response.rect.bottom();
    painter.rect_filled(egui::Rect::from_min_max(egui::pos2(full.left(), bottom - 2.0), egui::pos2(full.right(), bottom)), 0.0, ui.visuals().selection.bg_fill);
    if document { open_under_active_tab(ui, egui::Rect::from_min_max(egui::pos2(full.left(), bottom - 2.0), egui::pos2(full.right(), bottom))); }
    ui.spacing_mut().item_spacing.x = before;
    ui.add_space(PANEL_TABS_GAP + 2.0);
    (row.inner.0.on_hover_text("Drag this onto the side panel to dock it there"), row.inner.1)
}

/// Dragging a tab to a new place: while it is dragged a line shows where it will go, and when it is let go the move is returned as
/// (place it was, place it goes to). `chips` are the tabs in order.
pub(crate) fn drag_reorder(ui: &egui::Ui, salt: &str, chips: &[egui::Response]) -> Option<(usize, usize)> {
    let key = egui::Id::new(("tab-drag", salt));
    if let Some(k) = chips.iter().position(|r| r.drag_started()) { ui.ctx().data_mut(|d| d.insert_temp(key, k)); }
    let from: usize = ui.ctx().data(|d| d.get_temp(key))?;
    let Some(dragged) = chips.get(from) else { ui.ctx().data_mut(|d| d.remove_temp::<usize>(key)); return None };
    let x = ui.ctx().pointer_latest_pos().map(|p| p.x).unwrap_or(dragged.rect.center().x);
    // Where it would go: after every tab whose middle is to its left.
    let slot = chips.iter().filter(|r| r.rect.center().x < x).count();
    // A tab of the strip pulled well away from it comes out as a window of its own (the caller decides whether this tab can).
    if salt == "strip" && dragged.dragged() && ui.ctx().pointer_latest_pos().is_some_and(|p| (p.y - dragged.rect.center().y).abs() > 34.0) {
        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(("strip-tear", 0u8)), from));
    }
    // Tabs of the panels (their salt starts with "g") can also be let go over another group: the place it would land shows while it is dragged.
    let panel = salt.starts_with('g');
    let pos_id = egui::Id::new("tab-drag-pos");
    let pointer = ui.ctx().pointer_latest_pos();
    let in_row = pointer.is_some_and(|p| p.y >= dragged.rect.top() - 8.0 && p.y <= dragged.rect.bottom() + 8.0);
    // A docked tab that leaves its row comes away as a window of its own, at once, and the app carries the window with the pointer
    // (see `App::drive_tear`). Moving along the row still reorders the tabs.
    if dragged.dragged() && panel && !in_row {
        ui.ctx().data_mut(|d| { d.remove_temp::<usize>(key); d.remove_temp::<egui::Pos2>(pos_id); d.insert_temp(egui::Id::new(("tab-tear", salt)), from); });
        ui.ctx().request_repaint();
        return None;
    }
    if dragged.dragged() {
        if in_row || !panel {
            let at = if slot == 0 { chips[0].rect.left() } else { chips[slot - 1].rect.right() };
            let top = chips[0].rect.top();
            ui.painter().line_segment([egui::pos2(at, top + 2.0), egui::pos2(at, top + 28.0)], egui::Stroke::new(2.0, ui.visuals().selection.bg_fill));
            ui.ctx().data_mut(|d| d.remove_temp::<egui::Pos2>(pos_id));
        } else if let Some(p) = pointer {
            ui.ctx().data_mut(|d| d.insert_temp(pos_id, p));
        }
        ui.ctx().request_repaint();
    }
    if dragged.drag_stopped() {
        ui.ctx().data_mut(|d| { d.remove_temp::<usize>(key); d.remove_temp::<egui::Pos2>(pos_id); });
        // Let go outside the window: the tab is being taken out of the panel, not moved within it.
        let inside = pointer.is_some_and(|p| ui.ctx().content_rect().contains(p));
        if !inside { ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(("tab-dropped-outside", salt)), from)); return None; }
        // Let go over another place in the window: the tab goes there (the panels work out where).
        if panel && !in_row { if let Some(p) = pointer { ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(("tab-released", salt)), (from, p))); } return None; }
        let to = (if slot > from { slot - 1 } else { slot }).min(chips.len() - 1);
        if to != from { return Some((from, to)); }
    }
    None
}

/// The space above a row of panel tabs, as between the ribbon and the document tabs. The space below the blue line is 2 more.
const PANEL_TABS_GAP: f32 = 6.0;

/// The frame for a panel that starts with a row of tabs: no space above them, so the gap is the same everywhere.
/// `nested` is for a panel inside another, whose edges already have the outer panel's margin.
pub(super) fn tabbed_panel_frame(style: &egui::Style, nested: bool) -> egui::Frame {
    let side = if nested { 0 } else { 8 };
    egui::Frame::side_top_panel(style).inner_margin(egui::Margin { left: side, right: side, top: 0, bottom: 8 })
}

pub(crate) fn tab_chip_sized(ui: &mut egui::Ui, icon: Option<char>, label: &str, active: bool, closable: bool, width: f32, size: f32) -> (egui::Response, bool) {
    tab_chip_ex(ui, icon, label, active, closable, width, size, false)
}

/// A tab. `doc` is for the tabs of what is being edited (slides, Scene View and the editing modes): a lifted tab with a bar on top, not
/// the blue block of a panel's tabs.
#[allow(clippy::too_many_arguments)]
fn tab_chip_ex(ui: &mut egui::Ui, icon: Option<char>, label: &str, active: bool, closable: bool, width: f32, size: f32, doc: bool) -> (egui::Response, bool) {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 30.0), egui::Sense::click_and_drag());
    let dark = ui.visuals().dark_mode;
    let text = ui.visuals().text_color();
    // Panel tabs are a blue block with white text; document tabs are lifted off the band, with the theme's own text.
    let blue = active && !doc;
    let dimmed: bool = doc && active && ui.ctx().data(|d| d.get_temp(egui::Id::new("doc-tab-dim"))).unwrap_or(false);
    let ink = if blue { egui::Color32::WHITE } else if dimmed { text.gamma_multiply(0.75) } else if active { ui.visuals().strong_text_color() } else { text.gamma_multiply(0.7) };
    // Round the top corners only, like the ribbon tabs above; the bottom edge stays square on the line under the strip.
    let top_round = egui::CornerRadius { nw: 4, ne: 4, sw: 0, se: 0 };
    if blue {
        ui.painter().rect_filled(rect, top_round, ui.visuals().selection.bg_fill);
    } else if active {
        // The tab showing is open at the bottom, like a folder tab: blue along its top and both sides, and the same color as the bar under the
        // strip, so it joins what is below it. (The line under the strip is left out beneath it: see `open_under_active_tab`.)
        let fill = if dark { egui::Color32::from_gray(40) } else { egui::Color32::from_gray(246) };
        ui.painter().rect_filled(rect, top_round, fill);
        // Blue while its editor is the one the panels act on; gray while another editor is (see `doc-tab-dim`).
        let dim: bool = ui.ctx().data(|d| d.get_temp(egui::Id::new("doc-tab-dim"))).unwrap_or(false);
        let blue = if dim { text.gamma_multiply(0.4) } else { ui.visuals().selection.bg_fill };
        ui.painter().add(egui::Shape::line(open_bottom_outline(rect.shrink(0.75), 4.0), egui::Stroke::new(1.5, blue)));
        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("doc-tab-active"), Some((rect, fill))));
    } else if ui.rect_contains_pointer(rect) {
        ui.painter().rect_filled(rect, top_round, if dark { egui::Color32::from_white_alpha(14) } else { egui::Color32::from_black_alpha(12) });
    }
    let mid = rect.center().y - 1.0;
    let mut left = rect.left() + 12.0;
    if let Some(ch) = icon {
        ui.painter().text(egui::pos2(left, mid), egui::Align2::LEFT_CENTER, ch.to_string(), egui::FontId::new(15.0, ribbon::icons()), ink);
        left += 24.0;
    }
    let text_rect = egui::Rect::from_min_max(egui::pos2(left, rect.top()), rect.max - egui::vec2(if closable { 28.0 } else { 8.0 }, 0.0));
    let font = egui::FontId::proportional(size);
    ui.painter().with_clip_rect(text_rect).text(egui::pos2(text_rect.left(), mid), egui::Align2::LEFT_CENTER, label, font, ink);
    let mut closed = false;
    if closable && (active || ui.rect_contains_pointer(rect)) {
        let x = egui::Rect::from_center_size(egui::pos2(rect.right() - 15.0, mid), egui::vec2(18.0, 18.0));
        let xr = ui.interact(x, resp.id.with("close"), egui::Sense::click());
        if xr.hovered() { ui.painter().rect_filled(x, 9.0, if blue { egui::Color32::from_white_alpha(45) } else if dark { egui::Color32::from_white_alpha(28) } else { egui::Color32::from_black_alpha(24) }); }
        ui.painter().text(x.center(), egui::Align2::CENTER_CENTER, "\u{00d7}", egui::FontId::proportional(15.0), if active || xr.hovered() { if blue { egui::Color32::WHITE } else { text } } else { ink });
        closed = xr.clicked();
    }
    // Slide tabs share their words with the slide list, so only the tabs with an icon are named for screen readers here.
    if icon.is_some() { resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label)); }
    (resp.on_hover_text(label), closed)
}

/// The outline of a tab that is open at the bottom: up the left side, across the top and down the right side, with rounded top corners.
fn open_bottom_outline(r: egui::Rect, radius: f32) -> Vec<egui::Pos2> {
    let mut pts = vec![r.left_bottom()];
    let corner = |pts: &mut Vec<egui::Pos2>, c: egui::Pos2, from: f32| {
        for k in 0..=6 {
            let a = from + (k as f32 / 6.0) * std::f32::consts::FRAC_PI_2;
            pts.push(c + egui::vec2(a.cos(), a.sin()) * radius);
        }
    };
    // Top left corner: from the left side round to the top.
    corner(&mut pts, egui::pos2(r.left() + radius, r.top() + radius), std::f32::consts::PI);
    // Top right corner: from the top round to the right side.
    corner(&mut pts, egui::pos2(r.right() - radius, r.top() + radius), -std::f32::consts::FRAC_PI_2);
    pts.push(r.right_bottom());
    pts
}

/// The line under the strip stops under the tab that is showing, so the tab and what it shows are one piece. `line` is where the line was drawn.
pub(crate) fn open_under_active_tab(ui: &egui::Ui, line: egui::Rect) {
    let Some(Some((tab, fill))) = ui.ctx().data_mut(|d| d.remove_temp::<Option<(egui::Rect, egui::Color32)>>(egui::Id::new("doc-tab-active"))) else { return };
    let gap = egui::Rect::from_min_max(egui::pos2(tab.left() + 1.6, line.top() - 0.5), egui::pos2(tab.right() - 1.6, line.bottom() + 0.5));
    ui.painter().rect_filled(gap, 0.0, fill);
}
