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

/// A slide tab that is a window of its own.
pub(crate) struct EditorWindow {
    pub(crate) id: u32,
    pub(crate) tab: SlideTab,
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
        self.editor_windows.push(EditorWindow { id, tab });
        self.focus_window = Some(id);
        id
    }

    /// Open a slide in a window of its own, with a view of its own.
    pub(crate) fn open_slide_window(&mut self, slide_idx: usize) -> Option<u32> {
        let slide_id = self.project.as_ref()?.slides.get(slide_idx)?.id.clone();
        let id = self.add_editor_window(SlideTab::new(slide_id, crate::view::View::default(), self.bottom_tab));
        self.with_window_view(id, |a| a.reset_visibility());
        Some(id)
    }

    /// Put a window's tab back in the strip, showing.
    pub(crate) fn window_to_tab(&mut self, id: u32) {
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
        self.editor_windows.retain(|w| w.id != id);
        if self.focus_window == Some(id) { self.focus_window = None; }
    }

    fn window_slide(&self, id: u32) -> Option<usize> {
        let w = self.editor_windows.iter().find(|w| w.id == id)?;
        self.project.as_ref()?.slides.iter().position(|s| s.id == w.tab.slide_id)
    }

    /// Run `f` with window `id`'s view as the current one, then put the main window's back. What `f` did to the view stays in the window; if it
    /// moved the window to another slide (the Slide List was used), the window follows.
    pub(crate) fn with_window_view<R>(&mut self, id: u32, f: impl FnOnce(&mut App) -> R) -> Option<R> {
        let slide = self.window_slide(id)?;
        let at = self.editor_windows.iter().position(|w| w.id == id)?;
        let mut other = self.editor_windows[at].tab.view.clone();
        other.slide = slide;
        std::mem::swap(&mut self.view, &mut other);
        let r = f(self);
        std::mem::swap(&mut self.view, &mut other);
        let new_slide = self.project.as_ref().and_then(|p| p.slides.get(other.slide.min(p.slides.len().saturating_sub(1)))).map(|s| s.id.clone());
        if let Some(w) = self.editor_windows.iter_mut().find(|w| w.id == id) {
            w.tab.view = other;
            if let Some(s) = new_slide { w.tab.slide_id = s; }
        }
        Some(r)
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
        // A window whose slide has been deleted goes with it.
        let gone: Vec<u32> = self.editor_windows.iter().map(|w| w.id).filter(|id| self.window_slide(*id).is_none()).collect();
        for id in gone { self.close_editor_window(id); }
        let ids: Vec<u32> = self.editor_windows.iter().map(|w| w.id).collect();
        for id in ids {
            let Some(idx) = self.window_slide(id) else { continue };
            let name = self.project.as_ref().map(|p| p.slides[idx].name.clone()).unwrap_or_default();
            let title = format!("{} {}", self.slide_number(idx), name);
            let (mut open, mut closed_here, mut dropped, mut pop_in) = (true, false, None::<egui::Pos2>, false);
            crate::os_window_inner(ctx, &Self::editor_key(id), &title, [1100.0, 760.0], &mut open, false, |ui| {
                let (tab, closed) = doc_tabs::slide_window_tab(ui, &title, if cfg!(target_os = "macos") { 76.0 } else { 0.0 });
                if closed { closed_here = true; }
                // At the right end of the tab row: a button that puts the tab back in the main window's strip.
                let at = egui::pos2(ui.max_rect().right() - 18.0, tab.rect.center().y);
                if pop_in_button(ui, at).clicked() { pop_in = true; }
                dropped = crate::drag_window(ui.ctx(), &tab, "editor-drag-pos");
                // A press in the window makes it the one the panels and the keys act on.
                if ui.input(|i| i.pointer.any_pressed()) && ui.rect_contains_pointer(ui.max_rect()) { self.focus_window = Some(id); }
                let ctx = ui.ctx().clone();
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
