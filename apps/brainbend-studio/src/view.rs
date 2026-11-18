//! A view of the project: the part of the editor's state that belongs to one place the project is being edited in.
//!
//! The editor works on one slide at a time, with a layer, a selection, a zoom and a pan. These used to be fields of the app itself; they are
//! a record of their own so that more than one place can hold them (a tab of the main window now, a window of its own later: see #74).
//! The app has one of these for the view that is current. A tab keeps one for a slide that is not showing.
use super::*;

#[derive(Clone, Debug)]
pub struct View {
    /// The slide being edited (an index into the project's slides).
    pub slide: usize,
    /// Layer shown on top of the base layer (index into `slide.layers`, 0 = base only).
    pub layer: usize,
    /// The selected object: (layer index, object index).
    pub selected: Option<(usize, usize)>,
    /// Ids of further selected objects on the same layer as `selected` (shift-click / rubber band).
    pub multi: Vec<String>,
    /// Canvas view: `zoom` multiplies the fit-to-window scale; `pan` is a screen-space offset.
    pub zoom: f32,
    pub pan: egui::Vec2,
    /// Which layers of the slide are drawn (index = layer index; base layer is always drawn).
    pub visible: Vec<bool>,
    /// The States tab: (object id, state name) whose look the canvas shows.
    pub state_view: Option<(String, String)>,
}

impl Default for View {
    fn default() -> Self {
        View { slide: 0, layer: 0, selected: None, multi: vec![], zoom: 1.0, pan: egui::Vec2::ZERO, visible: vec![], state_view: None }
    }
}
