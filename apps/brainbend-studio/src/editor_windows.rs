//! Editor windows: a slide tab popped out into a window of its own (#74).
//!
//! Such a window has its own tab at the top (drag it back onto the strip to put it back), its own insert bar, and its own canvas, drawn with
//! the view the window holds (the slide, its layer, selection, zoom and pan). It works on the same project as everything else, with the one
//! undo history. The panels act on the view that was last clicked, the main window's or one of these.
//!
//! The rest of the app reads `App::view`, so a window is drawn, and the panels are drawn for it, with its view swapped in and swapped
//! back out afterwards (`with_window_view`).
use super::*;
use crate::doc_tabs::SlideTab;

/// A slide tab or a 360° image tab that is a window of its own.
pub(crate) struct EditorWindow {
    pub(crate) id: u32,
    pub(crate) tab: SlideTab,
    /// Set for a 360° image: what it is and the editing session the window holds.
    pub(crate) pano: Option<PanoWindow>,
    /// Where the window is and how big: x, y, width, height. Known once it has been on screen, or from a saved layout.
    pub(crate) rect: Option<[f32; 4]>,
    /// Set for the Layout Builder: the window works on the master and the layouts (its slide is the one of them that is chosen).
    pub(crate) layouts: bool,
    /// Whether the window has been put where `rect` says (a window restored from a saved layout is, the first time it is drawn).
    pub(crate) placed: bool,
}

/// A Scene View tab that is a window of its own: it shows the same scenes as the tab did.
pub(crate) struct SceneWindow {
    pub(crate) id: u32,
    pub(crate) rect: Option<[f32; 4]>,
}

/// The 360° image of a window: its object, its editing session, and how long the undo history was when the visit began.
pub(crate) struct PanoWindow {
    pub(crate) obj_id: String,
    pub(crate) edit: Option<crate::panos::Edit>,
    pub(crate) undo_len: usize,
}

/// What a tab that was pulled out and is being carried is: a panel's tab or an editor window.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum TearTarget { Panel(usize), Window(u32) }

impl App {
    /// The key of the operating-system window of an editor window.
    pub(crate) fn editor_key(id: u32) -> String { format!("editor-{id}") }

    /// Pop slide tab `k` out of the strip into a window of its own. Returns the window's number.
    pub(crate) fn pop_out_slide_tab(&mut self, k: usize) -> Option<u32> {
        if k >= self.tabs.len() { return None; }
        if self.active_tab == Some(k) { self.save_active_tab(); }
        let tab = self.tabs.get(k)?.clone();
        self.close_tab(k);
        Some(self.add_editor_window(tab))
    }

    fn add_editor_window(&mut self, tab: SlideTab) -> u32 {
        self.editor_window_next += 1;
        let id = self.editor_window_next;
        self.editor_windows.push(EditorWindow { id, tab, pano: None, layouts: false, rect: None, placed: true });
        self.focus_window = Some(id);
        id
    }

    /// Open a slide in a window of its own, with a view of its own.
    pub(crate) fn open_slide_window(&mut self, slide_idx: usize) -> Option<u32> {
        let slide_id = self.project.as_ref()?.slide_get(slide_idx)?.id.clone();
        let id = self.add_editor_window(SlideTab::new(slide_id, crate::view::View::default(), self.bottom_tab));
        self.with_window_view(id, |a| a.reset_visibility());
        Some(id)
    }

    /// Put a window's tab back in the strip, showing.
    pub(crate) fn window_to_tab(&mut self, id: u32) {
        if self.editor_windows.iter().any(|w| w.id == id && w.pano.is_some()) { return self.pano_window_to_tab(id); }
        if self.editor_windows.iter().any(|w| w.id == id && w.layouts) { return self.layout_window_to_tab(id); }
        let Some(n) = self.editor_windows.iter().position(|w| w.id == id) else { return };
        let w = self.editor_windows.remove(n);
        if self.focus_window == Some(id) { self.focus_window = None; }
        let Some(slide) = self.project.as_ref().and_then(|p| p.slides.iter().position(|s| s.id == w.tab.slide_id)) else { return };
        // The tab takes the place of the view the main window is showing; that one is kept in its tab first.
        self.save_active_tab();
        let mut view = w.tab.view.clone();
        view.slide = slide;
        self.tabs.push(w.tab);
        self.active_tab = Some(self.tabs.len() - 1);
        self.view = view;
        self.scene_view = false;
    }

    /// Close a window: its tab is gone (the slide is still in the Slide List to open again).
    pub(crate) fn close_editor_window(&mut self, id: u32) {
        // A 360° visit that changed nothing leaves no undo step (if nothing else has been done since, so the step is the last one).
        if let Some(w) = self.editor_windows.iter().find(|w| w.id == id) {
            if let Some(p) = &w.pano {
                if p.edit.as_ref().is_some_and(|e| !e.changed) && self.undo.len() == p.undo_len { self.undo.pop(); }
            }
        }
        self.editor_windows.retain(|w| w.id != id);
        if self.focus_window == Some(id) { self.focus_window = None; }
        // The master and layouts stay open as slides only while the Layout Builder or a window of it needs them.
        if !self.layouts_wanted() { if let Some(p) = self.project.as_mut() { if p.close_layouts() { self.dirty = true; } } }
    }

    fn window_slide(&self, id: u32) -> Option<usize> {
        let w = self.editor_windows.iter().find(|w| w.id == id)?;
        self.project.as_ref()?.slide_index_of(&w.tab.slide_id)
    }

    /// Run `f` with window `id`'s view as the current one, then put the main window's back. What `f` did to the view stays in the window; if it
    /// moved the window to another slide (the Slide List was used), the window follows.
    pub(crate) fn with_window_view<R>(&mut self, id: u32, f: impl FnOnce(&mut App) -> R) -> Option<R> {
        let slide = self.window_slide(id)?;
        let at = self.editor_windows.iter().position(|w| w.id == id)?;
        let mut window_view = self.editor_windows[at].tab.view.clone();
        window_view.slide = slide;
        // The main window's view is kept aside while the window's is the current one (`with_main_view` reaches it).
        let main = std::mem::replace(&mut self.view, window_view);
        let outer = self.stashed_main.replace(main);
        let r = f(self);
        let main = self.stashed_main.take().unwrap_or_default();
        self.stashed_main = outer;
        let other = std::mem::replace(&mut self.view, main);
        let new_slide = self.project.as_ref().and_then(|p| p.slide_get(other.slide.min(p.slide_count().saturating_sub(1)))).map(|s| s.id.clone());
        if let Some(w) = self.editor_windows.iter_mut().find(|w| w.id == id) {
            w.tab.view = other;
            if let Some(s) = new_slide { w.tab.slide_id = s; }
        }
        Some(r)
    }

    /// Run `f` with the main window's own view as the current one, while an editor window's is. The Slide List uses it to keep showing the main
    /// window's slides while a window of the layouts is the active editor (the window has its own list of layouts).
    pub(crate) fn with_main_view(&mut self, f: impl FnOnce(&mut App)) {
        let Some(mut main) = self.stashed_main.take() else { return f(self) };
        std::mem::swap(&mut self.view, &mut main);
        f(self);
        std::mem::swap(&mut self.view, &mut main);
        self.stashed_main = Some(main);
    }

    /// What the Properties panel says while an editor window is the one it acts on: the window's slide, so it is clear where a change goes.
    /// Nothing while the main window is the active one.
    pub(crate) fn active_window_note(&self) -> Option<String> {
        let i = self.window_slide(self.focus_window?)?;
        Some(format!("{} {}", self.slide_number(i), self.project.as_ref()?.slide(i).name))
    }

    /// Run `f` for the view the panels act on: the editor window that was clicked last, or the main window's.
    pub(crate) fn with_focused_view(&mut self, f: impl FnOnce(&mut App)) {
        match self.focus_window.filter(|id| self.window_slide(*id).is_some()) {
            Some(id) => { self.with_window_view(id, f); }
            None => f(self),
        }
    }

    /// Draw every editor window: its tab, its insert bar and its canvas.
    pub(crate) fn editor_windows_ui(&mut self, ctx: &egui::Context) {
        if self.file_view || self.project.is_none() { return; }
        self.scene_windows_ui(ctx);
        // A window of the layouts whose layout has gone (deleted, or undone) goes to the master; a slide's window whose slide has been deleted goes with it.
        for w in self.editor_windows.iter_mut().filter(|w| w.layouts) {
            if self.project.as_ref().is_some_and(|p| p.layouts_open() && p.slide_index_of(&w.tab.slide_id).is_none()) { w.tab.slide_id = brainbend_doc::master::MASTER_STAND_IN.to_string(); }
        }
        let gone: Vec<u32> = self.editor_windows.iter().map(|w| w.id).filter(|id| self.window_slide(*id).is_none()).collect();
        for id in gone { self.close_editor_window(id); }
        let ids: Vec<u32> = self.editor_windows.iter().map(|w| w.id).collect();
        for id in ids {
            let Some(idx) = self.window_slide(id) else { continue };
            let pano_name = self.editor_windows.iter().find(|w| w.id == id).and_then(|w| w.pano.as_ref()).and_then(|p| self.project.as_ref().and_then(|pr| pr.slide(idx).layers.iter().flat_map(|l| l.objects.iter()).find(|o| o.id == p.obj_id).map(|o| o.name.clone())));
            let name = pano_name.clone().unwrap_or_else(|| self.project.as_ref().map(|p| p.slide(idx).name.clone()).unwrap_or_default());
            let is_layouts = self.editor_windows.iter().any(|w| w.id == id && w.layouts);
            let title = if pano_name.is_some() || is_layouts { name } else { format!("{} {}", self.slide_number(idx), name) };
            let (mut open, mut closed_here, mut dropped, mut pop_in) = (true, false, None::<egui::Pos2>, false);
            // A window brought back from a saved layout opens where it was and as big as it was.
            let saved = self.editor_windows.iter_mut().find(|w| w.id == id).and_then(|w| { if w.placed { None } else { w.placed = true; w.rect } });
            if let Some([x, y, _, _]) = saved { ctx.data_mut(|d| d.insert_temp(egui::Id::new(("pop-out-start", Self::editor_key(id))), egui::pos2(x, y))); }
            let size = self.editor_windows.iter().find(|w| w.id == id).and_then(|w| w.rect).map(|r| [r[2].max(300.0), r[3].max(220.0)]).unwrap_or([1100.0, 760.0]);
            crate::os_window_inner(ctx, &Self::editor_key(id), &title, size, &mut open, false, |ui| {
                // Where the window is now, for the saved layout.
                if let (Some(outer), Some(inner)) = ui.ctx().input(|i| (i.viewport().outer_rect, i.viewport().inner_rect)) {
                    if let Some(w) = self.editor_windows.iter_mut().find(|w| w.id == id) { w.rect = Some([outer.min.x, outer.min.y, inner.width(), inner.height()]); }
                }
                // A window's tab is blue while it is the one the panels act on, and gray while another editor is.
                let is_pano = self.editor_windows.iter().any(|w| w.id == id && w.pano.is_some());
                let dim = !is_pano && self.focus_window != Some(id);
                ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("doc-tab-dim"), dim));
                let (tab, closed) = doc_tabs::slide_window_tab(ui, &title, if cfg!(target_os = "macos") { 76.0 } else { 0.0 });
                ui.ctx().data_mut(|d| d.remove_temp::<bool>(egui::Id::new("doc-tab-dim")));
                if closed { closed_here = true; }
                // At the right end of the tab row: a button that puts the tab back in the main window's strip.
                let at = egui::pos2(ui.max_rect().right() - 18.0, tab.rect.center().y);
                if pop_in_button(ui, at).clicked() { pop_in = true; }
                dropped = crate::drag_window(ui.ctx(), &tab, "editor-drag-pos");
                // A press in the window makes it the one the panels and the keys act on.
                if !is_pano && ui.input(|i| i.pointer.any_pressed()) && ui.rect_contains_pointer(ui.max_rect()) { self.focus_window = Some(id); }
                // So does the window coming to the front (the system gave it the focus without a click in it).
                let key = egui::Id::new(("window-focused", id));
                let now = ui.ctx().input(|i| i.viewport().focused);
                let was: Option<bool> = ui.ctx().data(|d| d.get_temp(key));
                if let Some(now) = now { ui.ctx().data_mut(|d| d.insert_temp(key, now)); }
                if !is_pano && was == Some(false) && now == Some(true) { self.focus_window = Some(id); }
                let ctx = ui.ctx().clone();
                if is_pano { if !self.pano_window_ui(ui, &ctx, id) { closed_here = true; } return; }
                // The layouts' window has the list of them at its left, then the same bar and canvas as a slide's window.
                if is_layouts {
                    egui::Panel::left(format!("layouts-list-{id}")).default_size(210.0).show(ui, |ui| { self.with_window_view(id, |a| a.master_panel(ui, &ctx)); });
                }
                self.with_window_view(id, |a| {
                    a.window_keys(&ctx);
                    a.insert_bar_in(ui);
                    let slide_idx = a.current_slide();
                    a.canvas_ui(ui, &ctx, slide_idx, false);
                });
            });
            if pop_in { self.window_to_tab(id); continue; }
            if let Some(p) = dropped {
                // Let go over the main window's tab strip, the tab goes back to it.
                let strip: Option<egui::Rect> = ctx.data(|d| d.get_temp(egui::Id::new("doc-strip-rect")));
                if strip.is_some_and(|r| r.expand(4.0).contains(p)) { self.window_to_tab(id); continue; }
            }
            if closed_here || !open { self.close_editor_window(id); }
        }
    }

    /// While a pulled-out slide tab is carried: see `App::drive_tear`.
    pub(crate) fn drive_window_tear(&mut self, ctx: &egui::Context, id: u32, grab: egui::Vec2) {
        let (down, local, inner) = ctx.input_for(egui::ViewportId::ROOT, |i| (i.pointer.primary_down(), i.pointer.latest_pos(), i.viewport().inner_rect));
        let inside = local.filter(|p| ctx.content_rect().contains(*p));
        let pos_id = egui::Id::new("editor-drag-pos");
        if let (Some(p), Some(inner)) = (local, inner) {
            ctx.send_viewport_cmd_to(egui::ViewportId::from_hash_of(("pop-out", Self::editor_key(id))), egui::ViewportCommand::OuterPosition(inner.min + p.to_vec2() - grab));
        }
        if down {
            ctx.data_mut(|d| match inside { Some(p) => { d.insert_temp(pos_id, p); } None => { d.remove_temp::<egui::Pos2>(pos_id); } });
            ctx.request_repaint();
            return;
        }
        self.tear = None;
        ctx.data_mut(|d| d.remove_temp::<egui::Pos2>(pos_id));
        let strip: Option<egui::Rect> = ctx.data(|d| d.get_temp(egui::Id::new("doc-strip-rect")));
        if inside.is_some_and(|p| strip.is_some_and(|r| r.expand(4.0).contains(p))) { self.window_to_tab(id); }
    }

    /// A slide tab was pulled out of the strip: it is a window under the pointer now, carried until the button is let go.
    pub(crate) fn begin_window_tear(&mut self, ctx: &egui::Context, k: usize) {
        let (local, inner) = ctx.input_for(egui::ViewportId::ROOT, |i| (i.pointer.latest_pos(), i.viewport().inner_rect));
        let grab = egui::vec2(if cfg!(target_os = "macos") { 76.0 + 50.0 } else { 58.0 }, 16.0);
        let Some(id) = self.pop_out_slide_tab(k) else { return };
        if let (Some(p), Some(inner)) = (local, inner) {
            ctx.data_mut(|d| d.insert_temp(egui::Id::new(("pop-out-start", Self::editor_key(id))), inner.min + p.to_vec2() - grab));
        }
        self.tear = Some((TearTarget::Window(id), grab));
    }
}

/// The button that puts a window's tab back in the main window: a small window with an arrow going into it.
fn pop_in_button(ui: &mut egui::Ui, center: egui::Pos2) -> egui::Response {
    let rect = egui::Rect::from_center_size(center, egui::vec2(26.0, 26.0));
    let resp = ui.interact(rect, ui.id().with("pop-in"), egui::Sense::click());
    if resp.hovered() { ui.painter().rect_filled(rect, 4.0, ui.visuals().text_color().gamma_multiply(0.12)); }
    let ink = if resp.hovered() { ui.visuals().strong_text_color() } else { ui.visuals().text_color().gamma_multiply(0.75) };
    let st = egui::Stroke::new(1.4, ink);
    let c = rect.center();
    // The window (open at its top right, where the arrow comes in) and the arrow.
    let p = ui.painter();
    p.line_segment([c + egui::vec2(1.0, -6.0), c + egui::vec2(-6.0, -6.0)], st);
    p.line_segment([c + egui::vec2(-6.0, -6.0), c + egui::vec2(-6.0, 6.0)], st);
    p.line_segment([c + egui::vec2(-6.0, 6.0), c + egui::vec2(6.0, 6.0)], st);
    p.line_segment([c + egui::vec2(6.0, 6.0), c + egui::vec2(6.0, 0.0)], st);
    p.line_segment([c + egui::vec2(6.0, -6.0), c + egui::vec2(-1.0, 1.0)], st);
    p.line_segment([c + egui::vec2(-1.0, 1.0), c + egui::vec2(-1.0, -3.5)], st);
    p.line_segment([c + egui::vec2(-1.0, 1.0), c + egui::vec2(3.5, 1.0)], st);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Put back in the main window"));
    resp.on_hover_text("Put this tab back in the main window")
}

impl App {
    /// Pop the 360° image's tab out of the strip into a window of its own. The editing session goes with it.
    pub(crate) fn pop_out_pano_tab(&mut self) -> Option<u32> {
        let tab = self.pano.tab.clone()?;
        // The session the main window has (if the tab is showing) moves to the window; otherwise a new one starts.
        let edit = match self.pano.edit.take() {
            Some(e) => Some(e),
            None => self.new_pano_session(&tab.slide_id, &tab.obj_id),
        };
        edit.as_ref()?;
        self.pano.tab = None;
        let undo_len = self.undo.len();
        self.editor_window_next += 1;
        let id = self.editor_window_next;
        self.editor_windows.push(EditorWindow { id, tab: SlideTab::new(tab.slide_id, crate::view::View::default(), self.bottom_tab), pano: Some(PanoWindow { obj_id: tab.obj_id, edit, undo_len }), layouts: false, rect: None, placed: true });
        Some(id)
    }

    /// Put a 360° image's window back as its tab in the main window, with its editing mode showing.
    pub(crate) fn pano_window_to_tab(&mut self, id: u32) {
        let Some(n) = self.editor_windows.iter().position(|w| w.id == id) else { return };
        let mut w = self.editor_windows.remove(n);
        let Some(p) = w.pano.take() else { return };
        // Whatever the main window was editing of a 360° image is put away first.
        self.close_pano_edit();
        self.leave_master_view();
        let Some((si, _, _)) = self.pano_position(&w.tab.slide_id, &p.obj_id) else { return };
        self.view.slide = si;
        self.pano.edit = p.edit;
        self.pano.tab = Some(crate::panos::Tab { slide_id: w.tab.slide_id, obj_id: p.obj_id });
        self.scene_view = false;
    }

    /// The window of a 360° image: a bar with the things to add, then the editing mode's panels and picture. False when it should close
    /// (the picture or its slide is gone).
    fn pano_window_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, id: u32) -> bool {
        let Some(at) = self.editor_windows.iter().position(|w| w.id == id) else { return false };
        let slide_id = self.editor_windows[at].tab.slide_id.clone();
        let Some(obj_id) = self.editor_windows[at].pano.as_ref().map(|p| p.obj_id.clone()) else { return false };
        // The slide may have moved in the list since the last frame: find the picture again.
        let Some((si, li, oi)) = self.pano_position(&slide_id, &obj_id) else { return false };
        let Some(edit) = self.editor_windows[at].pano.as_mut().and_then(|p| p.edit.take()) else { return false };
        let mut edit = edit;
        edit.slide = si;
        edit.target = (li, oi);
        // The mode's code works on `App::pano.edit`: the window's session is put there while it is drawn, and taken back after.
        let saved = std::mem::replace(&mut self.pano.edit, Some(edit));
        // (The bar for what goes on the picture is part of the editing mode's own screen.)
        let alive = self.pano_edit_ui(ui, ctx);
        let back = std::mem::replace(&mut self.pano.edit, saved);
        if let Some(w) = self.editor_windows.iter_mut().find(|w| w.id == id) { if let Some(p) = w.pano.as_mut() { p.edit = back; } }
        alive
    }

    /// A 360° image's tab was pulled out of the strip: it is a window under the pointer now.
    pub(crate) fn begin_pano_window_tear(&mut self, ctx: &egui::Context) {
        let (local, inner) = ctx.input_for(egui::ViewportId::ROOT, |i| (i.pointer.latest_pos(), i.viewport().inner_rect));
        let grab = egui::vec2(if cfg!(target_os = "macos") { 76.0 + 50.0 } else { 58.0 }, 16.0);
        let Some(id) = self.pop_out_pano_tab() else { return };
        if let (Some(p), Some(inner)) = (local, inner) {
            ctx.data_mut(|d| d.insert_temp(egui::Id::new(("pop-out-start", Self::editor_key(id))), inner.min + p.to_vec2() - grab));
        }
        self.tear = Some((TearTarget::Window(id), grab));
    }
}

impl App {
    /// The open editor windows as saved text: each one's kind, slide, 360° image, view and place.
    pub(crate) fn windows_json(&self) -> String {
        use serde_json::json;
        let list: Vec<serde_json::Value> = self.editor_windows.iter().map(|w| {
            let mut o = json!({ "slide": w.tab.slide_id, "zoom": w.tab.view.zoom, "pan": [w.tab.view.pan.x, w.tab.view.pan.y], "layer": w.tab.view.layer });
            if let Some(p) = &w.pano { o["kind"] = "pano".into(); o["obj"] = p.obj_id.clone().into(); } else if w.layouts { o["kind"] = "layouts".into(); } else { o["kind"] = "slide".into(); }
            if let Some(r) = w.rect { o["rect"] = json!(r); }
            o
        }).collect();
        serde_json::Value::Array(list).to_string()
    }

    fn windows_key(&self) -> Option<String> { self.path.as_ref().map(|p| format!("editor_windows:{}", p.display())) }

    /// Remember which windows are open for this project (it needs a file: an unsaved project is not remembered). Runs every frame and writes
    /// only when something changed, and not in the middle of a drag.
    pub(crate) fn save_editor_windows(&mut self, ctx: &egui::Context) {
        let Some(key) = self.windows_key() else { return };
        if ctx.input_for(egui::ViewportId::ROOT, |i| i.pointer.any_down()) { return; }
        let now = self.windows_json();
        let stamp = format!("{key}\n{now}");
        if stamp == self.editor_windows_saved { return; }
        crate::prefs::set_str(&key, &now);
        self.editor_windows_saved = stamp;
    }

    /// Open the windows this project had open the last time (after the project has been opened).
    pub(crate) fn restore_editor_windows(&mut self) {
        let Some(key) = self.windows_key() else { return };
        let Some(text) = crate::prefs::get_str(&key) else { return };
        self.restore_windows_from(&text);
        self.editor_windows_saved = format!("{key}\n{}", self.windows_json());
    }

    /// Open the windows described by `text` (see `windows_json`), leaving out those whose slide or picture is gone.
    pub(crate) fn restore_windows_from(&mut self, text: &str) {
        let Ok(serde_json::Value::Array(list)) = serde_json::from_str::<serde_json::Value>(text) else { return };
        for o in list {
            let Some(slide_id) = o.get("slide").and_then(|s| s.as_str()).map(str::to_string) else { continue };
            let is_layouts = o.get("kind").and_then(|k| k.as_str()) == Some("layouts");
            // The layouts' window opens the master and layouts as slides; its slide is the layout it was on, or the master if that has gone.
            if is_layouts {
                if let Some(p) = self.project.as_mut() { p.open_layouts(); }
                let slide_id = if self.project.as_ref().is_some_and(|p| p.slide_index_of(&slide_id).is_some()) { slide_id } else { brainbend_doc::master::MASTER_STAND_IN.to_string() };
                self.editor_window_next += 1;
                let id = self.editor_window_next;
                let rect = o.get("rect").and_then(|r| r.as_array()).filter(|r| r.len() == 4).map(|r| { let v: Vec<f32> = r.iter().map(|x| x.as_f64().unwrap_or(0.0) as f32).collect(); [v[0], v[1], v[2], v[3]] }).filter(|r| r[0] > -3000.0 && r[0] < 8000.0 && r[1] > -500.0 && r[1] < 5000.0 && r[2] >= 200.0 && r[3] >= 150.0);
                self.editor_windows.push(EditorWindow { id, tab: SlideTab::new(slide_id, crate::view::View::default(), self.bottom_tab), pano: None, layouts: true, rect, placed: rect.is_none() });
                continue;
            }
            if !self.project.as_ref().is_some_and(|p| p.slides.iter().any(|s| s.id == slide_id)) { continue; }
            let num = |k: &str, d: f32| o.get(k).and_then(|v| v.as_f64()).map(|v| v as f32).unwrap_or(d);
            let rect = o.get("rect").and_then(|r| r.as_array()).filter(|r| r.len() == 4).map(|r| { let v: Vec<f32> = r.iter().map(|x| x.as_f64().unwrap_or(0.0) as f32).collect(); [v[0], v[1], v[2], v[3]] });
            // A place that is plainly nowhere (a monitor that was unplugged) is not used: the window opens where the system puts it.
            let rect = rect.filter(|r| r[0] > -3000.0 && r[0] < 8000.0 && r[1] > -500.0 && r[1] < 5000.0 && r[2] >= 200.0 && r[3] >= 150.0);
            let id = if o.get("kind").and_then(|k| k.as_str()) == Some("pano") {
                let Some(obj) = o.get("obj").and_then(|s| s.as_str()).map(str::to_string) else { continue };
                // The visit takes an undo step as when it opens by hand; the project is not changed by opening it, so it stays as saved.
                let dirty = self.dirty;
                let edit = self.new_pano_session(&slide_id, &obj);
                self.dirty = dirty;
                let Some(edit) = edit else { continue };
                self.editor_window_next += 1;
                let id = self.editor_window_next;
                let undo_len = self.undo.len();
                self.editor_windows.push(EditorWindow { id, tab: SlideTab::new(slide_id, crate::view::View::default(), self.bottom_tab), pano: Some(PanoWindow { obj_id: obj, edit: Some(edit), undo_len }), layouts: false, rect, placed: rect.is_none() });
                id
            } else {
                let Some(idx) = self.project.as_ref().and_then(|p| p.slides.iter().position(|s| s.id == slide_id)) else { continue };
                let Some(id) = self.open_slide_window(idx) else { continue };
                self.with_window_view(id, |a| {
                    a.view.zoom = num("zoom", 1.0).clamp(0.1, 8.0);
                    let pan = o.get("pan").and_then(|p| p.as_array()).filter(|p| p.len() == 2).map(|p| egui::vec2(p[0].as_f64().unwrap_or(0.0) as f32, p[1].as_f64().unwrap_or(0.0) as f32)).unwrap_or(egui::Vec2::ZERO);
                    a.view.pan = pan;
                    let layers = a.project.as_ref().map(|p| p.slide(a.view.slide).layers.len()).unwrap_or(1);
                    a.view.layer = (num("layer", 0.0) as usize).min(layers.saturating_sub(1));
                });
                if let Some(w) = self.editor_windows.iter_mut().find(|w| w.id == id) { w.rect = rect; w.placed = rect.is_none(); }
                id
            };
            let _ = id;
        }
        // The main window is the one the panels act on to begin with.
        self.focus_window = None;
    }
}

impl App {
    /// Whether the master and the layouts have to stay open as slides: the Layout Builder is open, or a window of it is.
    pub(crate) fn layouts_wanted(&self) -> bool { self.master_stash.is_some() || self.editor_windows.iter().any(|w| w.layouts) }

    /// Whether the view being worked on (the main window's, or the one swapped in for a window) is a layout or the master: its slide comes
    /// after the project's own.
    pub(crate) fn on_layout(&self) -> bool { self.project.as_ref().is_some_and(|p| p.layouts_open() && self.view.slide >= p.slides.len()) }

    /// Pop the Layout Builder out of the main window into a window of its own, on the layout it is showing (or the master).
    pub(crate) fn pop_out_layout_tab(&mut self) -> Option<u32> {
        let p = self.project.as_mut()?;
        p.open_layouts();
        let in_builder = self.master_stash.is_some();
        let (slide_id, view) = match p.slide_get(self.view.slide).filter(|_| in_builder && self.view.slide >= p.slides.len()) {
            Some(s) => (s.id.clone(), self.view.clone()),
            None => (brainbend_doc::master::MASTER_STAND_IN.to_string(), crate::view::View::default()),
        };
        self.editor_window_next += 1;
        let id = self.editor_window_next;
        self.editor_windows.push(EditorWindow { id, tab: SlideTab::new(slide_id, view, self.bottom_tab), pano: None, layouts: true, rect: None, placed: true });
        self.layout_tab = false;
        // The builder leaves the main window (the window keeps the master and layouts open).
        self.leave_master_view();
        self.focus_window = Some(id);
        Some(id)
    }

    /// Put the Layout Builder back in the main window, on the layout the window was showing.
    pub(crate) fn layout_window_to_tab(&mut self, id: u32) {
        let Some(n) = self.editor_windows.iter().position(|w| w.id == id) else { return };
        let w = self.editor_windows.remove(n);
        if self.focus_window == Some(id) { self.focus_window = None; }
        self.enter_master_view();
        if let Some(i) = self.project.as_ref().and_then(|p| p.slide_index_of(&w.tab.slide_id)) {
            self.view.slide = i;
            self.view.layer = 0;
            self.view.selected = None;
            self.reset_visibility();
        }
        // Another window of the layouts may still need them; if none does and the builder is closed again, close_editor_window's rule applies.
    }

    /// The Layout Builder's tab was pulled out of the strip: it is a window under the pointer now.
    pub(crate) fn begin_layout_window_tear(&mut self, ctx: &egui::Context) {
        let (local, inner) = ctx.input_for(egui::ViewportId::ROOT, |i| (i.pointer.latest_pos(), i.viewport().inner_rect));
        let grab = egui::vec2(if cfg!(target_os = "macos") { 76.0 + 50.0 } else { 58.0 }, 16.0);
        let Some(id) = self.pop_out_layout_tab() else { return };
        if let (Some(p), Some(inner)) = (local, inner) {
            ctx.data_mut(|d| d.insert_temp(egui::Id::new(("pop-out-start", Self::editor_key(id))), inner.min + p.to_vec2() - grab));
        }
        self.tear = Some((TearTarget::Window(id), grab));
    }
}

impl App {
    /// A Scene View tab was popped out of the strip: a window of its own shows the scenes.
    pub(crate) fn add_scene_window(&mut self) -> u32 {
        self.editor_window_next += 1;
        let id = self.editor_window_next;
        self.scene_windows.push(SceneWindow { id, rect: None });
        id
    }

    /// Put a Scene View window back in the strip, as a tab that is showing.
    pub(crate) fn scene_window_to_tab(&mut self, id: u32) {
        let Some(n) = self.scene_windows.iter().position(|w| w.id == id) else { return };
        self.scene_windows.remove(n);
        self.save_active_tab();
        self.new_scene_tab();
        self.scene_view = true;
    }

    fn scene_windows_ui(&mut self, ctx: &egui::Context) {
        let ids: Vec<u32> = self.scene_windows.iter().map(|w| w.id).collect();
        for id in ids {
            let size = self.scene_windows.iter().find(|w| w.id == id).and_then(|w| w.rect).map(|r| [r[2].max(300.0), r[3].max(220.0)]).unwrap_or([1000.0, 700.0]);
            let (mut open, mut closed_here, mut dropped, mut pop_in) = (true, false, None::<egui::Pos2>, false);
            crate::os_window_inner(ctx, &Self::editor_key(id), "Scene View", size, &mut open, false, |ui| {
                if let (Some(outer), Some(inner)) = ui.ctx().input(|i| (i.viewport().outer_rect, i.viewport().inner_rect)) {
                    if let Some(w) = self.scene_windows.iter_mut().find(|w| w.id == id) { w.rect = Some([outer.min.x, outer.min.y, inner.width(), inner.height()]); }
                }
                let (tab, closed) = doc_tabs::slide_window_tab(ui, "SCENE VIEW", if cfg!(target_os = "macos") { 76.0 } else { 0.0 });
                if closed { closed_here = true; }
                let at = egui::pos2(ui.max_rect().right() - 18.0, tab.rect.center().y);
                if pop_in_button(ui, at).clicked() { pop_in = true; }
                dropped = crate::drag_window(ui.ctx(), &tab, "editor-drag-pos");
                let ctx = ui.ctx().clone();
                // The scenes are drawn as the Scene View is (it counts as showing while its window is drawn).
                let was = std::mem::replace(&mut self.scene_view, true);
                self.scene_view_ui(ui, &ctx);
                self.scene_view = was;
            });
            if pop_in { self.scene_window_to_tab(id); continue; }
            if let Some(p) = dropped {
                let strip: Option<egui::Rect> = ctx.data(|d| d.get_temp(egui::Id::new("doc-strip-rect")));
                if strip.is_some_and(|r| r.expand(4.0).contains(p)) { self.scene_window_to_tab(id); continue; }
            }
            if closed_here || !open { self.scene_windows.retain(|w| w.id != id); }
        }
    }
}
