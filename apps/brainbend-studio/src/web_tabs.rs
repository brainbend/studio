//! The tab strip of a web lesson: the Table of Contents (the title page with a card for each section) and a tab for each section
//! that is open, the way a media lesson has the Scene View and a tab for each slide.
//!
//! - A lesson opens on its Table of Contents, the first tab. The "+" adds another one; opening a section from an extra one turns it into the
//!   section's tab. Opening a section from the only Table of Contents tab leaves that tab where it is and opens the section beside it.
//! - A section's tab shows that section alone. The Outline, the cards and the insert bar all work on the chosen section (`App::sec`).
//! - The strip is kept for the session; the file does not remember which tabs were open.
use super::*;
use crate::doc_tabs::{drag_reorder, open_under_active_tab, tab_chip, tab_chip_sized};

/// What is chosen on the Table of Contents: nothing, the lesson's header (the banner), the page itself, or a card (a section).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum TocPick { #[default] Nothing, Header, Page, Card }

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum WebTab {
    /// The Table of Contents.
    Toc,
    /// A section, by its id (sections can be moved and renamed under it).
    Section(String),
}

impl App {
    fn web_section_ids(&self) -> Vec<String> {
        self.project.as_ref().and_then(|p| p.lesson.as_ref()).map(|l| l.sections.iter().map(|s| s.id.clone()).collect()).unwrap_or_default()
    }

    /// Whether the Table of Contents is what the editor shows (a web lesson, with one of its tabs showing).
    pub(crate) fn toc_showing(&self) -> bool {
        self.is_web_lesson() && matches!(self.web_tabs.get(self.web_active), Some(WebTab::Toc))
    }

    /// Keep the tabs in step with the lesson: a section that is gone loses its tab, and the section chosen (in the Outline, say) is the one
    /// the section tab that is showing shows. Runs at the start of every frame.
    pub(super) fn web_sync_tabs(&mut self) {
        if !self.is_web_lesson() { return; }
        let ids = self.web_section_ids();
        if ids.is_empty() { return; }
        if self.web_tabs.is_empty() {
            // A lesson opens on its Table of Contents: the author adds sections there and opens one to write in it.
            self.web_tabs = vec![WebTab::Toc];
            self.web_active = 0;
            self.toc_pick = TocPick::Nothing;
            self.sec = 0;
            self.blk = None;
        }
        let before = self.web_active.min(self.web_tabs.len() - 1);
        let mut kept: Vec<WebTab> = vec![];
        let mut active = 0;
        for (k, t) in std::mem::take(&mut self.web_tabs).into_iter().enumerate() {
            let alive = match &t { WebTab::Section(id) => ids.contains(id), WebTab::Toc => true };
            if k == before { active = if alive { kept.len() } else { kept.len().saturating_sub(1) }; }
            if alive { kept.push(t); }
        }
        if kept.is_empty() { kept.push(WebTab::Toc); }
        self.web_active = active.min(kept.len() - 1);
        self.web_tabs = kept;
        let sec = self.sec.min(ids.len() - 1);
        // The section being shown follows the choice made elsewhere.
        if let Some(WebTab::Section(id)) = self.web_tabs.get(self.web_active) {
            if *id != ids[sec] { self.web_show_section(sec); }
        }
    }

    /// Show section `si` in a tab: its own if it has one, else the section tab that is showing gives way to it, else it opens beside the Table of Contents.
    pub(super) fn web_show_section(&mut self, si: usize) {
        let ids = self.web_section_ids();
        let Some(id) = ids.get(si).cloned() else { return };
        self.sec = si;
        let tab = WebTab::Section(id);
        if let Some(k) = self.web_tabs.iter().position(|t| *t == tab) { self.web_active = k; return; }
        if self.web_tabs.is_empty() { self.web_tabs.push(WebTab::Toc); }
        let a = self.web_active.min(self.web_tabs.len() - 1);
        let tocs = self.web_tabs.iter().filter(|t| **t == WebTab::Toc).count();
        match &self.web_tabs[a] {
            WebTab::Section(_) => self.web_tabs[a] = tab,
            // An extra Table of Contents turns into the section; the only one stays, and the section opens beside it.
            WebTab::Toc if tocs > 1 => self.web_tabs[a] = tab,
            WebTab::Toc => { self.web_tabs.insert(a + 1, tab); self.web_active = a + 1; }
        }
    }

    /// Open a section from a card of the Table of Contents.
    pub(super) fn web_open_section(&mut self, si: usize) {
        self.blk = None;
        self.toc_pick = TocPick::Nothing;
        self.web_show_section(si);
    }

    /// Make tab `k` the one showing, and choose what it shows.
    fn web_go(&mut self, k: usize) {
        if k >= self.web_tabs.len() { return; }
        self.web_active = k;
        self.blk = None;
        match self.web_tabs[k].clone() {
            WebTab::Section(id) => { if let Some(i) = self.web_section_ids().iter().position(|s| *s == id) { self.sec = i; } }
            WebTab::Toc => self.toc_pick = TocPick::Nothing,
        }
    }

    pub(super) fn web_new_toc_tab(&mut self) {
        self.web_tabs.push(WebTab::Toc);
        self.web_go(self.web_tabs.len() - 1);
    }

    pub(super) fn web_close_tab(&mut self, k: usize) {
        if k >= self.web_tabs.len() { return; }
        let was = k == self.web_active;
        self.web_tabs.remove(k);
        if self.web_tabs.is_empty() { self.web_tabs.push(WebTab::Toc); }
        if was { self.web_go(k.saturating_sub(1).min(self.web_tabs.len() - 1)); }
        else if k < self.web_active { self.web_active -= 1; }
    }

    pub(super) fn web_close_others(&mut self, k: usize) {
        let Some(keep) = self.web_tabs.get(k).cloned() else { return };
        self.web_tabs = vec![keep];
        self.web_go(0);
    }

    pub(super) fn web_move_tab(&mut self, from: usize, to: usize) {
        if from >= self.web_tabs.len() || to >= self.web_tabs.len() || from == to { return; }
        let t = self.web_tabs.remove(from);
        self.web_tabs.insert(to, t);
        self.web_active = if self.web_active == from { to } else if from < self.web_active && self.web_active <= to { self.web_active - 1 } else if to <= self.web_active && self.web_active < from { self.web_active + 1 } else { self.web_active };
    }

    /// The strip under the menu bar: the Table of Contents, a tab for each section that is open, and a "+".
    pub(super) fn web_tab_strip(&mut self, root: &mut egui::Ui) {
        self.web_sync_tabs();
        if self.web_tabs.is_empty() { return; }
        let labels: Vec<String> = {
            let l = self.lesson_ref();
            self.web_tabs.iter().map(|t| match t {
                WebTab::Toc => "TABLE OF CONTENTS".to_string(),
                WebTab::Section(id) => l.sections.iter().position(|s| s.id == *id).map(|i| format!("{}.  {}", i + 1, if l.sections[i].title.trim().is_empty() { "(untitled)" } else { l.sections[i].title.trim() })).unwrap_or_default(),
            }).collect()
        };
        let n = self.web_tabs.len();
        let active = self.web_active.min(n - 1);
        let mut go: Option<usize> = None;
        let mut close: Option<usize> = None;
        let mut close_others: Option<usize> = None;
        let mut close_all = false;
        let mut plus = false;
        let mut moved: Option<(usize, usize)> = None;
        let dark = root.visuals().dark_mode;
        let band = if dark { egui::Color32::from_gray(46) } else { egui::Color32::WHITE };
        let line = root.visuals().selection.bg_fill;
        egui::Panel::top("doc-tabs").frame(egui::Frame::new().fill(band).inner_margin(egui::Margin { left: 8, right: 8, top: 0, bottom: 0 })).show_separator_line(false).show(root, |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let row = ui.horizontal(|ui| {
                let mut chips: Vec<egui::Response> = vec![];
                for (k, t) in self.web_tabs.iter().enumerate() {
                    let label = &labels[k];
                    let (r, closed) = match t {
                        WebTab::Toc => tab_chip(ui, Some(ribbon::LIST_BULLETS), label, k == active, n > 1, 218.0),
                        WebTab::Section(_) => {
                            let tw = ui.fonts_mut(|f| f.layout_no_wrap(label.clone(), egui::FontId::proportional(13.0), egui::Color32::WHITE).size().x);
                            tab_chip(ui, None, label, k == active, true, (tw + 58.0).clamp(96.0, 190.0))
                        }
                    };
                    if closed || r.middle_clicked() { close = Some(k); } else if r.clicked() { go = Some(k); }
                    r.context_menu(|ui| {
                        if ui.add_enabled(n > 1 || matches!(t, WebTab::Section(_)), egui::Button::new("Close")).clicked() { close = Some(k); ui.close(); }
                        if ui.add_enabled(n > 1, egui::Button::new("Close other tabs")).clicked() { close_others = Some(k); ui.close(); }
                        if ui.add_enabled(n > 1, egui::Button::new("Close all tabs")).clicked() { close_all = true; ui.close(); }
                    });
                    chips.push(r);
                }
                moved = drag_reorder(ui, "web-strip", &chips);
                let (p, _) = tab_chip_sized(ui, None, "+", false, false, 34.0, 16.0);
                p.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "New tab"));
                if p.clicked() { plus = true; }
            });
            let full = egui::Rect::from_min_max(egui::pos2(row.response.rect.left() - 8.0, row.response.rect.bottom() - 2.0), egui::pos2(ui.clip_rect().right().max(row.response.rect.right() + 8.0), row.response.rect.bottom()));
            ui.painter().rect_filled(full, 0.0, line);
            open_under_active_tab(ui, full);
        });
        if let Some((a, b)) = moved { self.web_move_tab(a, b); }
        if plus { self.web_new_toc_tab(); }
        else if close_all { self.web_tabs = vec![WebTab::Toc]; self.web_go(0); }
        else if let Some(k) = close_others { self.web_close_others(k); }
        else if let Some(k) = close { self.web_close_tab(k); }
        else if let Some(k) = go { if k != self.web_active { self.web_go(k); } }
    }
}
