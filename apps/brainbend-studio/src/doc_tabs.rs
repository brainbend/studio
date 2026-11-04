//! Story View and slide tabs: the Story View is always the first tab, and each slide opened from it
//! (or from the slide list) gets a tab of its own that remembers its layer, selection, zoom and bottom panel.
//!
//! The editor keeps working on "the current slide" (`App::slide`, `layer`, `selected`, ...). A tab holds the view state of
//! a slide that is not current: the active tab's state is copied into it at the end of every frame, and copied back
//! when the tab becomes current again. Tabs are keyed by slide id, so reordering slides does not disturb them and
//! deleting a slide closes its tab.
use super::*;

#[derive(Clone, Debug)]
pub struct SlideTab {
    pub slide_id: String,
    layer: usize,
    selected: Option<(usize, usize)>,
    zoom: f32,
    pan: egui::Vec2,
    bottom_tab: usize,
    visible: Vec<bool>,
    state_view: Option<(String, String)>,
}

impl App {
    /// The slide's number, written scene.position, e.g. "1.5".
    fn slide_number(&self, i: usize) -> String {
        let Some(p) = &self.project else { return String::new() };
        let Some(s) = p.slides.get(i) else { return String::new() };
        let si = p.scenes.iter().position(|c| c.id == s.scene).unwrap_or(0);
        let n = p.scene_slides(si).iter().position(|&x| x == i).map(|k| k + 1).unwrap_or(i + 1);
        format!("{}.{}", si + 1, n)
    }

    fn tab_for(&self, slide_idx: usize) -> SlideTab {
        SlideTab {
            slide_id: self.project.as_ref().map(|p| p.slides[slide_idx].id.clone()).unwrap_or_default(),
            layer: self.layer,
            selected: self.selected,
            zoom: self.zoom,
            pan: self.pan,
            bottom_tab: self.bottom_tab,
            visible: self.visible.clone(),
            state_view: self.state_view.clone(),
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
        self.active_tab = active_id.and_then(|id| self.tabs.iter().position(|t| t.slide_id == id));
        if self.scene_view { self.active_tab = None; return; }
        let cur_idx = self.slide.min(ids.len() - 1);
        let cur = ids[cur_idx].clone();
        if self.active_tab.is_some_and(|k| self.tabs[k].slide_id == cur) { return; }
        if let Some(k) = self.tabs.iter().position(|t| t.slide_id == cur) {
            let t = self.tabs[k].clone();
            let layers = self.project.as_ref().unwrap().slides[cur_idx].layers.len();
            self.layer = t.layer.min(layers - 1);
            self.selected = t.selected.filter(|&(l, o)| self.project.as_ref().unwrap().slides[cur_idx].layers.get(l).is_some_and(|ly| o < ly.objects.len()));
            self.zoom = t.zoom;
            self.pan = t.pan;
            self.bottom_tab = t.bottom_tab;
            self.state_view = t.state_view;
            if t.visible.len() == layers { self.visible = t.visible; } else { self.reset_visibility(); }
            self.active_tab = Some(k);
        } else {
            let t = self.tab_for(cur_idx);
            self.tabs.push(t);
            self.active_tab = Some(self.tabs.len() - 1);
        }
    }

    /// Copy the current view state into the active tab. Runs at the end of every frame; a frame in which the slide
    /// changed is skipped, since by then the view fields already belong to the next slide.
    pub(super) fn save_active_tab(&mut self) {
        if self.in_master_view() || self.scene_view { return; }
        let (Some(k), Some(p)) = (self.active_tab, self.project.as_ref()) else { return };
        let cur = p.slides[self.slide.min(p.slides.len() - 1)].id.clone();
        if self.tabs.get(k).is_some_and(|t| t.slide_id == cur) {
            let t = self.tab_for(self.slide.min(p.slides.len() - 1));
            self.tabs[k] = t;
        }
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
        // Go to the neighbour on the left (or right), else back to the Story View.
        let next = if k > 0 { Some(k - 1) } else if !self.tabs.is_empty() { Some(0) } else { None };
        match next.and_then(|n| self.tabs.get(n)).map(|t| t.slide_id.clone()) {
            Some(id) => {
                if let Some(i) = self.project.as_ref().and_then(|p| p.slides.iter().position(|s| s.id == id)) { self.slide = i; self.scene_view = false; }
            }
            None => self.scene_view = true,
        }
    }

    /// The tab strip under the ribbon: STORY VIEW, then one tab per open slide.
    /// The strip in the 360° editing mode: the slide's tab, then the 360° image's own, which closes the mode.
    pub(super) fn pano_tabs(&mut self, root: &mut egui::Ui) {
        let Some(slide) = self.pano.edit.as_ref().map(|e| e.slide) else { return };
        let label = self.project.as_ref().and_then(|p| p.slides.get(slide)).map(|s| format!("{} {}", self.slide_number(slide), s.name)).unwrap_or_default();
        let name = self.pano_object_name();
        let dark = root.visuals().dark_mode;
        let band = if dark { egui::Color32::from_gray(46) } else { egui::Color32::WHITE };
        let line = root.visuals().selection.bg_fill;
        let mut leave = false;
        egui::Panel::top("doc-tabs").frame(egui::Frame::new().fill(band).inner_margin(egui::Margin { left: 8, right: 8, top: 0, bottom: 0 })).show_separator_line(false).show(root, |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let row = ui.horizontal(|ui| {
                let (story, _) = tab_chip(ui, Some(ribbon::STACK), "STORY VIEW", false, false, 138.0);
                if story.clicked() { leave = true; self.scene_view = true; }
                let tw = ui.fonts_mut(|f| f.layout_no_wrap(label.clone(), egui::FontId::proportional(13.0), egui::Color32::WHITE).size().x);
                let (r, _) = tab_chip(ui, None, &label, false, false, (tw + 58.0).clamp(96.0, 190.0));
                if r.clicked() { leave = true; }
                let (_, closed) = tab_chip(ui, None, &name, true, true, 150.0);
                if closed { leave = true; }
            });
            let full = egui::Rect::from_min_max(egui::pos2(row.response.rect.left() - 8.0, row.response.rect.bottom() - 2.0), egui::pos2(ui.clip_rect().right().max(row.response.rect.right() + 8.0), row.response.rect.bottom()));
            ui.painter().rect_filled(full, 0.0, line);
        });
        if leave { self.close_pano_edit(); }
    }

    pub(super) fn doc_tabs(&mut self, root: &mut egui::Ui) {
        if self.project.is_none() || self.file_view || self.in_master_view() { return; }
        if self.project.as_ref().is_some_and(|p| p.is_web()) { return; }
        let mut go_story = false;
        let mut go_slide: Option<String> = None;
        let mut close: Option<usize> = None;
        let mut close_others: Option<usize> = None;
        let mut close_all = false;
        let labels: Vec<(String, String)> = self.tabs.iter().filter_map(|t| {
            let i = self.project.as_ref()?.slides.iter().position(|s| s.id == t.slide_id)?;
            Some((t.slide_id.clone(), format!("{} {}", self.slide_number(i), self.project.as_ref()?.slides[i].name)))
        }).collect();
        // One flat band that belongs to the ribbon, with a border line under it that the active tab sits on.
        let dark = root.visuals().dark_mode;
        let band = if dark { egui::Color32::from_gray(46) } else { egui::Color32::WHITE };
        let line = root.visuals().selection.bg_fill;
        egui::Panel::top("doc-tabs").frame(egui::Frame::new().fill(band).inner_margin(egui::Margin { left: 8, right: 8, top: 0, bottom: 0 })).show_separator_line(false).show(root, |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let row = ui.horizontal(|ui| {
                let (story, _) = tab_chip(ui, Some(ribbon::STACK), "STORY VIEW", self.scene_view, false, 138.0);
                if story.clicked() { go_story = true; }
                for (k, (id, label)) in labels.iter().enumerate() {
                    let active = !self.scene_view && self.active_tab == Some(k);
                    // As wide as its name needs, so the strip is not mostly gaps.
                    let tw = ui.fonts_mut(|f| f.layout_no_wrap(label.clone(), egui::FontId::proportional(13.0), egui::Color32::WHITE).size().x);
                    let (r, closed) = tab_chip(ui, None, label, active, true, (tw + 58.0).clamp(96.0, 190.0));
                    if closed || r.middle_clicked() { close = Some(k); } else if r.clicked() { go_slide = Some(id.clone()); }
                    r.context_menu(|ui| {
                        if ui.button("Close").clicked() { close = Some(k); ui.close(); }
                        if ui.add_enabled(labels.len() > 1, egui::Button::new("Close other tabs")).clicked() { close_others = Some(k); ui.close(); }
                        if ui.button("Close all tabs").clicked() { close_all = true; ui.close(); }
                    });
                }
            });
            let full = egui::Rect::from_min_max(egui::pos2(row.response.rect.left() - 8.0, row.response.rect.bottom() - 2.0), egui::pos2(ui.clip_rect().right().max(row.response.rect.right() + 8.0), row.response.rect.bottom()));
            ui.painter().rect_filled(full, 0.0, line);
        });
        if close_all {
            self.tabs.clear();
            self.active_tab = None;
            self.scene_view = true;
        } else if let Some(k) = close_others {
            let keep = self.tabs.get(k).cloned();
            self.tabs = keep.into_iter().collect();
            self.active_tab = if self.tabs.is_empty() { None } else { Some(0) };
            if let Some(id) = self.tabs.first().map(|t| t.slide_id.clone()) {
                if let Some(i) = self.project.as_ref().and_then(|p| p.slides.iter().position(|s| s.id == id)) { self.slide = i; self.scene_view = false; }
            }
        } else if let Some(k) = close { self.close_tab(k); }
        else if go_story { self.scene_view = true; }
        else if let Some(id) = go_slide {
            if let Some(i) = self.project.as_ref().and_then(|p| p.slides.iter().position(|s| s.id == id)) {
                self.slide = i;
                self.scene_view = false;
            }
        }
    }
}

/// One tab: flat, so the active one is a block that meets the border line under the strip. The others have no fill
/// and muted text, with a faint fill on hover. The × shows on the active tab and on hover. Returns the tab and
/// whether its × was pressed.
fn tab_chip(ui: &mut egui::Ui, icon: Option<char>, label: &str, active: bool, closable: bool, width: f32) -> (egui::Response, bool) {
    tab_chip_sized(ui, icon, label, active, closable, width, if closable { 12.5 } else { 11.5 })
}

/// A row of tabs for a panel (Properties/Triggers, Timeline/Slide Notes/States, Slide Layers), drawn like the
/// document tabs: the active one is a blue block that meets a blue line under the row. Returns whether a tab
/// other than the current one was clicked; `current` is updated to it.
pub(super) fn panel_tabs(ui: &mut egui::Ui, labels: &[String], current: &mut usize) -> bool {
    let mut changed = false;
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
        for (i, label) in labels.iter().enumerate() {
            let tw = ui.fonts_mut(|f| f.layout_no_wrap(label.clone(), egui::FontId::proportional(12.5), egui::Color32::WHITE).size().x);
            let (r, _) = tab_chip_sized(ui, None, label, *current == i, false, tw + 28.0, 12.5);
            r.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, *current == i, label));
            if r.clicked() && *current != i { *current = i; changed = true; }
        }
    });
    let bottom = row.response.rect.bottom();
    painter.rect_filled(egui::Rect::from_min_max(egui::pos2(full.left(), bottom - 2.0), egui::pos2(full.right(), bottom)), 0.0, ui.visuals().selection.bg_fill);
    ui.spacing_mut().item_spacing.x = before;
    ui.add_space(PANEL_TABS_GAP + 2.0);
    changed
}

/// The space above a row of panel tabs, as between the ribbon and the document tabs. The space below the blue line is 2 more.
const PANEL_TABS_GAP: f32 = 6.0;

/// The frame for a panel that starts with a row of tabs: no space above them, so the gap is the same everywhere.
/// `nested` is for a panel inside another, whose edges already have the outer panel's margin.
pub(super) fn tabbed_panel_frame(style: &egui::Style, nested: bool) -> egui::Frame {
    let side = if nested { 0 } else { 8 };
    egui::Frame::side_top_panel(style).inner_margin(egui::Margin { left: side, right: side, top: 0, bottom: 8 })
}

fn tab_chip_sized(ui: &mut egui::Ui, icon: Option<char>, label: &str, active: bool, closable: bool, width: f32, size: f32) -> (egui::Response, bool) {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 30.0), egui::Sense::click());
    let dark = ui.visuals().dark_mode;
    let text = ui.visuals().text_color();
    let ink = if active { egui::Color32::WHITE } else { text.gamma_multiply(0.7) };
    // Round the top corners only, like the ribbon tabs above; the bottom edge stays square on the line under the strip.
    let top_round = egui::CornerRadius { nw: 4, ne: 4, sw: 0, se: 0 };
    if active {
        ui.painter().rect_filled(rect, top_round, ui.visuals().selection.bg_fill);
    } else if resp.hovered() {
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
    if closable && (active || resp.hovered()) {
        let x = egui::Rect::from_center_size(egui::pos2(rect.right() - 15.0, mid), egui::vec2(18.0, 18.0));
        let xr = ui.interact(x, resp.id.with("close"), egui::Sense::click());
        if xr.hovered() { ui.painter().rect_filled(x, 9.0, if active { egui::Color32::from_white_alpha(45) } else if dark { egui::Color32::from_white_alpha(28) } else { egui::Color32::from_black_alpha(24) }); }
        ui.painter().text(x.center(), egui::Align2::CENTER_CENTER, "\u{00d7}", egui::FontId::proportional(15.0), if active || xr.hovered() { if active { egui::Color32::WHITE } else { text } } else { ink });
        closed = xr.clicked();
    }
    (resp.on_hover_text(label), closed)
}
