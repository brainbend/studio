//! A view of the project: the part of the editor's state that belongs to one place the project is being edited in.
//!
//! The editor works on one slide at a time, with a layer, a selection, a zoom and a pan. These used to be fields of the app itself; they are
//! a record of their own so that more than one place can hold them (a tab of the main window now, a window of its own later: see #74).
//! The app has one of these for the view that is current. A tab keeps one for a slide that is not showing.
use super::*;

#[derive(Clone)]
pub(crate) struct View {
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
    /// The States tab is in Edit mode: shapes added go into the viewed state (see `states_panel::begin`).
    pub state_editing: bool,
    // What is going on on this canvas right now: the drag, the text being edited, the crop or outline being worked on.
    /// Rubber-band start in slide coordinates while dragging on empty canvas.
    pub(crate) marquee: Option<egui::Pos2>,
    pub(crate) drag: Option<DragMode>,
    pub(crate) drag_changed: bool,
    pub(crate) fit_scale: f32,
    /// Object row being renamed in the Objects tab, and the row being dragged to reorder.
    pub(crate) obj_edit: Option<(usize, usize)>,
    pub(crate) obj_drag: Option<usize>,
    /// Selected character range of the text being edited in place; lets ribbon buttons format just that part.
    pub(crate) text_sel: Option<(usize, usize)>,
    /// Where the text cursor last was while editing in place: (layer, object, character index). Insert > Symbol uses it,
    /// since clicking the ribbon ends the in-place edit.
    pub(crate) caret: Option<(usize, usize, usize)>,
    /// A timeline drag that started on a group's own row moves the whole group.
    pub(crate) obj_drag_group: bool,
    /// The group (id) being renamed in the timeline.
    pub(crate) group_edit: Option<String>,
    /// Draw outlines for objects that are hidden when the slide starts.
    pub(crate) show_hidden: bool,
    /// The id of the picture being cropped on the canvas (crop mode), if any.
    pub(crate) crop_mode: Option<String>,
    /// While cropping: the shape (width over height) the frame is held to, if one has been chosen.
    pub(crate) crop_aspect: Option<f32>,
    /// The id of the hotspot whose outline is being edited on the canvas, and the corner chosen in it.
    pub(crate) outline_mode: Option<String>,
    pub(crate) outline_sel: Option<usize>,
    /// Corners (slide units) of a freeform hotspot being drawn by clicking on the slide.
    pub(crate) draw_outline: Option<Vec<(f32, f32)>>,
    pub(crate) draw_slide: usize,
    /// Object whose text is being edited in place on the canvas, and its text before editing.
    pub(crate) editing: Option<(usize, usize)>,
    pub(crate) editing_old: String,
    /// The edited object's rotation and text direction when last drawn: the edit box cannot turn, so a change ends the edit.
    pub(crate) edit_turn: Option<(u32, i16)>,
    pub(crate) canvas_frame: Option<(egui::Pos2, f32)>,
    /// The id of a text box made by double-clicking blank space; it goes again if it is left empty.
    pub(crate) fresh_text: Option<String>,
    /// Typing starts here on the next frame (the canvas of this frame was drawn before the box existed).
    pub(crate) start_edit: Option<(usize, usize)>,
    /// A hotspot area being drawn on a question preview: (start, current) as fractions of the picture.
    pub(crate) hot_drag: Option<((f32, f32), (f32, f32))>,
    /// Far corner of a block of table cells chosen with shift-click (the near corner is the selected cell).
    pub(crate) range_end: Option<(String, (u16, u16))>,
}

impl Default for View {
    fn default() -> Self {
        View { slide: 0, layer: 0, selected: None, multi: vec![], zoom: 1.0, pan: egui::Vec2::ZERO, visible: vec![], state_view: None, state_editing: false, marquee: None, fit_scale: 1.0, canvas_frame: None, drag: None, drag_changed: false, obj_drag: None, obj_edit: None, obj_drag_group: false, group_edit: None, hot_drag: None, crop_mode: None, crop_aspect: None, outline_mode: None, outline_sel: None, draw_outline: None, draw_slide: 0, editing: None, editing_old: String::new(), edit_turn: None, text_sel: None, caret: None, range_end: None, fresh_text: None, start_edit: None, show_hidden: false }
    }
}
