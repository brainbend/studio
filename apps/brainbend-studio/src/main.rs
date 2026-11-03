use eframe::egui;
use std::collections::HashMap;
use std::path::PathBuf;
use brainbend_doc::*;
mod equations;
mod panos;
mod icons;
mod markers;
mod backstage;
mod controls;
mod doc_tabs;
mod crop;
mod outline;
mod player_dialog;
mod button_styles;
mod master_view;
mod objects_panel;
mod effects_paint;
mod ribbon;
use ribbon::combo;
mod states_panel;
mod update;
mod lesson_view;

/// On macOS the content extends under a transparent title bar so our own bar can sit beside the native window buttons.
fn window_builder() -> egui::ViewportBuilder {
    let b = egui::ViewportBuilder::default().with_inner_size([1400.0, 900.0]).with_title("BrainBend Studio");
    if cfg!(target_os = "macos") {
        b.with_fullsize_content_view(true).with_title_shown(false).with_titlebar_shown(false)
    } else {
        b
    }
}

fn main() -> eframe::Result {
    let opts = eframe::NativeOptions {
        viewport: window_builder(),
        ..Default::default()
    };
    let arg = std::env::args().nth(1).map(PathBuf::from);
    eframe::run_native(
        "BrainBend Studio",
        opts,
        Box::new(move |cc| {
            install_fonts(&cc.egui_ctx);
            brainbend_spell::warm();
            install_theme(&cc.egui_ctx);
            let mut app = App::default();
            update::clean_up_after_update();
            app.updater = update::Updater::load();
            if app.updater.on_launch { app.updater.start(&cc.egui_ctx); }
            app.open_to_story_view = true;
            if let Some(p) = arg {
                app.open(&p);
            } else if recovery_path().is_some_and(|p| p.exists()) {
                app.recovery_offer = true;
            }
            Ok(Box::new(app))
        }),
    )
}

struct App {
    project: Option<Project>,
    path: Option<PathBuf>,
    slide: usize,
    /// Layer shown on top of the base layer (index into `slide.layers`, 0 = base only).
    layer: usize,
    selected: Option<(usize, usize)>, // (layer index, object index)
    /// Ids of further selected objects on the same layer as `selected` (shift-click / rubber band).
    multi: Vec<String>,
    /// Rubber-band start in slide coordinates while dragging on empty canvas.
    marquee: Option<egui::Pos2>,
    /// Which layers of the current slide are drawn (index = layer index; base layer is always drawn).
    visible: Vec<bool>,
    textures: HashMap<String, egui::TextureHandle>,
    undo: Vec<Snap>,
    redo: Vec<Snap>,
    status: String,
    /// Format Painter: the object whose look was copied (its id) and a copy of it, waiting for a target.
    painter: Option<(String, Box<Object>)>,
    dirty: bool,
    edit_open: bool,
    drag: Option<DragMode>,
    drag_changed: bool,
    id_counter: u64,
    new_var_name: String,
    new_var_kind: usize, // 0 = true/false, 1 = number, 2 = text
    /// Canvas view: `zoom` multiplies the fit-to-window scale; `pan` is a screen-space offset.
    zoom: f32,
    pan: egui::Vec2,
    fit_scale: f32,
    theme_colors_open: bool,
    /// A colour just picked, to be remembered as recent once the pointer is released.
    pending_recent: Option<Rgba>,
    /// The issue count in the status bar, when it was counted and what it was.
    status_issues: (f64, usize),
    status_sig: (usize, usize, usize),
    /// Object position at drag start plus raw (unsnapped) movement, so snapping never accumulates error.
    drag_origin: (f32, f32),
    drag_accum: egui::Vec2,
    guides: Vec<(bool, f32)>, // (vertical?, slide coordinate)
    clipboard: Option<Object>,
    /// What a copy put on the system clipboard (the object's text, or its name), so a ⌘V can tell its own copy from text copied elsewhere.
    clip_text: String,
    /// Set by a copy; the next frame hands it to the system clipboard.
    clip_out: Option<String>,
    drop_path: Option<PathBuf>,
    /// The Find and Replace window, when open.
    find: Option<FindState>,
    /// The Spelling window, when open.
    spell: Option<SpellState>,
    /// Web Lesson editor: the selected section, the selected block in it, and the settings window.
    sec: usize,
    blk: Option<usize>,
    lesson_settings: bool,
    /// Results of the last project check, shown in a window until closed.
    check_results: Option<Vec<brainbend_doc::check::Issue>>,
    /// Bottom panel tab: 0 = interactions, 1 = timeline.
    bottom_tab: usize,
    /// The States tab: (object id, state name) whose look the canvas shows.
    state_view: Option<(String, String)>,
    /// Dim the other objects while the States tab is open.
    state_dim: bool,
    /// "Try it" in the States tab: press buttons on the canvas to see their states, view only.
    try_mode: bool,
    /// The layers showing while trying (ids), and the slide they were worked out for.
    try_layers: Vec<String>,
    try_slide: Option<usize>,
    /// What presses left each tried object in (object id -> state name).
    try_states: HashMap<String, String>,
    /// What the last press in Try It did that Try It does not run (a jump to another slide, say), for the banner.
    try_note: Option<String>,
    /// The States tab's viewed state is on its object for the length of a frame.
    swap: Option<states_panel::Swap>,
    right_tab: usize,
    /// The selection, layer and slide the side panel last saw: picking another shows its Properties.
    inspector_seen: Option<(Option<(usize, usize)>, usize, usize)>,
    /// The Triggers tab picked an owner itself (a group heading), so the side panel should stay on Triggers.
    trigger_select: bool,
    /// The Triggers tab lists every trigger on the slide, grouped by owner, instead of only the selection's.
    trig_all: bool,
    /// The project's Variables window is open.
    variables_open: bool,
    /// The Variables window's search text and filters (type: 0 any, 1 true/false, 2 number, 3 text; use: 0 any, 1 in a trigger, 2 in no trigger).
    var_query: String,
    var_kind: usize,
    var_use: usize,
    /// A variable being renamed in the window: its id, the name typed so far, and whether the box still needs focus.
    var_rename: Option<(String, String, bool)>,
    /// What the last rename or type change did, or why it was refused (true when it was refused).
    var_note: Option<(String, bool)>,
    /// The Focus Order window (Home > Slide): the reading and tab order of the current layer.
    focus_open: bool,
    /// The Equation window (Insert > Equation, or Edit equation in Properties), while it is open.
    eq_dialog: Option<equations::Dialog>,
    pano: panos::State,
    /// Object row being renamed in the Objects tab, and the row being dragged to reorder.
    obj_edit: Option<(usize, usize)>,
    obj_drag: Option<usize>,
    /// Scene View (overview of all scenes) instead of the slide editor.
    scene_view: bool,
    /// The Player Properties window, and which pane of it is showing.
    player_open: bool,
    player_pane: usize,
    /// Open slide tabs (see `doc_tabs`), and which one is being edited (`None` = the Story View).
    tabs: Vec<doc_tabs::SlideTab>,
    active_tab: Option<usize>,
    /// Open a project into the Story View (the app turns this on; tests keep the slide view).
    open_to_story_view: bool,
    scene_rename: Option<usize>,
    /// Ids of further selected slides (besides the current one): ⌘-click toggles, ⇧-click selects a range.
    slide_multi: Vec<String>,
    /// Notes from the last import, shown once in a dialog.
    import_report: Vec<String>,
    /// What the last PowerPoint export could not carry over.
    export_report: Vec<String>,
    ribbon_tab: usize,
    /// Dim the base layer while another layer is active ("Dim").
    dim_base: bool,
    /// Layer row being renamed in place (double-click), and the row being dragged to reorder.
    layer_edit: Option<usize>,
    layer_drag: Option<usize>,
    open_requested: bool,
    /// The full-screen File view is open.
    file_view: bool,
    file_page: backstage::FilePage,
    /// Selected character range of the text being edited in place; lets ribbon buttons format just that part.
    text_sel: Option<(usize, usize)>,
    /// Where the text cursor last was while editing in place: (layer, object, character index). Insert > Symbol uses it,
    /// since clicking the ribbon ends the in-place edit.
    caret: Option<(usize, usize, usize)>,
    /// Timeline bar drag: (object index, which part, start ms, duration ms at drag start, accumulated px).
    tl_drag: Option<(usize, u8, u32, Option<u32>, f32)>,
    /// Groups whose members are listed under them in the timeline (group ids); others stay folded.
    tl_open: std::collections::HashSet<String>,
    /// A timeline drag that started on a group's own row moves the whole group.
    obj_drag_group: bool,
    /// The group (id) being renamed in the timeline.
    group_edit: Option<String>,
    /// A drag on a group's timing bar: (group id, 0 move / 1 length, each member's timing when it began, pixels dragged).
    tl_gdrag: Option<(String, u8, Vec<(usize, Option<Timing>)>, f32)>,
    /// Draw outlines for objects that are hidden when the slide starts.
    show_hidden: bool,
    /// The id of the picture being cropped on the canvas (crop mode), if any.
    crop_mode: Option<String>,
    /// While cropping: the shape (width over height) the frame is held to, if one has been chosen.
    crop_aspect: Option<f32>,
    /// The id of the hotspot whose outline is being edited on the canvas, and the corner chosen in it.
    outline_mode: Option<String>,
    outline_sel: Option<usize>,
    /// Corners (slide units) of a freeform hotspot being drawn by clicking on the slide.
    draw_outline: Option<Vec<(f32, f32)>>,
    draw_slide: usize,
    /// Object whose text is being edited in place on the canvas, and its text before editing.
    editing: Option<(usize, usize)>,
    editing_old: String,
    /// The edited object's rotation and text direction when last drawn: the edit box cannot turn, so a change ends the edit.
    edit_turn: Option<(u32, i16)>,
    /// Where the slide sits on screen (its top-left corner and scale) as of the last frame; the tests find handles with it.
    #[cfg_attr(not(test), allow(dead_code))]
    canvas_frame: Option<(egui::Pos2, f32)>,
    /// The id of a text box made by double-clicking blank space; it goes again if it is left empty.
    fresh_text: Option<String>,
    /// Typing starts here on the next frame (the canvas of this frame was drawn before the box existed).
    start_edit: Option<(usize, usize)>,
    /// Action waiting on the "unsaved changes" dialog.
    pending: Option<Pending>,
    allow_close: bool,
    /// Checks GitHub for a newer release.
    updater: update::Updater,
    last_autosave: std::time::Instant,
    recovery_offer: bool,
    /// Slides to delete once the frame is drawn: the slide list and Story View are still walking the old list.
    pending_delete: Option<Vec<usize>>,
    /// A hotspot area being drawn on a question preview: (start, current) as fractions of the picture.
    hot_drag: Option<((f32, f32), (f32, f32))>,
    /// Far corner of a block of table cells chosen with shift-click (the near corner is the selected cell).
    range_end: Option<(String, (u16, u16))>,
    /// Colour and width the Table panel's border buttons apply.
    border_color: Rgba,
    border_width: f32,
    /// The look the Table panel applies, and which parts of it.
    table_style: brainbend_doc::tables::TableStyle,
    table_opts: brainbend_doc::tables::StyleOptions,
    /// Set while the Slide Master view is open.
    master_stash: Option<master_view::MasterStash>,
}

/// Changes to a table's grid.
#[derive(Clone, Copy, PartialEq, Debug)]
enum TableOp {
    RowAbove,
    RowBelow,
    ColLeft,
    ColRight,
    DeleteRow,
    DeleteCol,
}

/// What the Preview button plays.
#[derive(Clone, Copy, PartialEq, Debug)]
enum PreviewScope {
    All,
    Scene,
    Slide,
    FromHere,
    /// Every selected slide, in lesson order.
    Selected,
}

#[derive(Clone, Copy, PartialEq)]
enum Pending {
    Quit,
    New,
    /// A new Web Lesson.
    NewLesson(ProjectKind),
    Open,
    /// Open the file waiting in `App::drop_path` (dragged onto the window).
    OpenDropped,
    /// Close the app so the downloaded update can replace it.
    Update,
}

/// Recovery copies live outside the project so they survive crashes: `~/.brainbend-studio/recovery.brainbend`.
fn recovery_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join(".brainbend-studio").join("recovery.brainbend"))
}

/// Project-level settings that undo also covers (everything that is not slide content).
#[derive(Clone)]
struct Meta {
    variables: Vec<Variable>,
    scenes: Vec<Scene>,
    pass_mark: u32,
    show_nav: bool,
    show_menu: bool,
    menu_free: bool,
    show_progress: bool,
    player: PlayerSettings,
    resources: Vec<Resource>,
    glossary: Vec<GlossaryTerm>,
    theme: Theme,
    palette: Vec<Rgba>,
    master: Master,
    layouts: Vec<SlideLayout>,
    lesson: Option<brainbend_doc::lesson::WebLesson>,
}

impl Meta {
    fn of(p: &Project) -> Self {
        Self {
            variables: p.variables.clone(),
            scenes: p.scenes.clone(),
            pass_mark: p.pass_mark,
            show_nav: p.show_nav,
            show_menu: p.show_menu,
            menu_free: p.menu_free,
            show_progress: p.show_progress,
            player: p.player.clone(),
            resources: p.resources.clone(),
            glossary: p.glossary.clone(),
            theme: p.theme.clone(),
            palette: p.palette.clone(),
            master: p.master.clone(),
            layouts: p.layouts.clone(),
            lesson: p.lesson.clone(),
        }
    }

    fn apply(self, p: &mut Project) {
        p.variables = self.variables;
        p.scenes = self.scenes;
        p.pass_mark = self.pass_mark;
        p.show_nav = self.show_nav;
        p.show_menu = self.show_menu;
        p.menu_free = self.menu_free;
        p.show_progress = self.show_progress;
        p.player = self.player;
        p.resources = self.resources;
        p.glossary = self.glossary;
        p.theme = self.theme;
        p.palette = self.palette;
        p.master = self.master;
        p.layouts = self.layouts;
        p.lesson = self.lesson;
    }
}

/// One undo step: the slides plus the project-level settings, as they were.
#[derive(Clone)]
struct Snap {
    slides: Vec<Slide>,
    meta: Meta,
}

impl Snap {
    fn of(p: &Project) -> Self {
        Self { slides: p.slides.clone(), meta: Meta::of(p) }
    }

    /// Install this snapshot into the project and return what it replaced.
    fn swap_into(self, p: &mut Project) -> Snap {
        let current = Snap::of(p);
        p.slides = self.slides;
        self.meta.apply(p);
        current
    }
}

#[derive(Clone, Copy, PartialEq)]
enum DragMode {
    Move,
    Marquee,
    /// Handle direction: -1, 0 or 1 on each axis (e.g. (-1, -1) is the top-left corner).
    Resize(i8, i8),
    /// Turning the object with the handle above it.
    Rotate,
    /// Scaling a multi-selection or group as one box.
    ResizeGroup(i8, i8),
    /// Dragging an inner table border: (column border?, index).
    TableBorder(bool, u16),
    /// Dragging a point of the selected object's motion path.
    PathPoint(usize),
    /// Crop mode: dragging a crop edge or corner of the picture's frame.
    Crop(i8, i8),
    /// Crop mode: sliding the picture under its frame.
    CropPan,
    /// Outline mode: dragging a corner of the hotspot's outline.
    OutlinePoint(usize),
}

impl Default for App {
    fn default() -> Self {
        Self {
            project: None, path: None, slide: 0, layer: 0, selected: None, multi: vec![], marquee: None, visible: vec![], textures: HashMap::new(),
            undo: vec![], redo: vec![], status: String::new(), painter: None, dirty: false, edit_open: false, drag: None,
            drag_changed: false, id_counter: 0, new_var_name: String::new(), new_var_kind: 0,
            zoom: 1.0, pan: egui::Vec2::ZERO, fit_scale: 1.0, theme_colors_open: false, pending_recent: None, status_issues: (f64::NEG_INFINITY, 0), status_sig: (0, 0, 0), drag_origin: (0.0, 0.0), drag_accum: egui::Vec2::ZERO,
            guides: vec![], clipboard: None, clip_text: String::new(), clip_out: None, drop_path: None, find: None, spell: None, sec: 0, blk: None, lesson_settings: false, check_results: None, bottom_tab: 0, state_view: None, state_dim: true, try_mode: false, try_layers: vec![], try_slide: None, try_states: HashMap::new(), try_note: None, swap: None, right_tab: 0, inspector_seen: None, trigger_select: false, trig_all: false, variables_open: false, var_query: String::new(), var_kind: 0, var_use: 0, var_rename: None, var_note: None, focus_open: false, eq_dialog: None, pano: Default::default(), obj_edit: None, obj_drag: None, scene_view: false, player_open: false, player_pane: 0, tabs: vec![], active_tab: None, open_to_story_view: false, scene_rename: None, slide_multi: vec![], import_report: vec![], export_report: vec![], ribbon_tab: 1, dim_base: true, layer_edit: None, layer_drag: None, open_requested: false, file_view: true, file_page: backstage::FilePage::New, text_sel: None, caret: None, range_end: None, hot_drag: None, border_color: Rgba(0, 0, 0, 255), border_width: 2.0, table_style: brainbend_doc::tables::TableStyle::Accent, table_opts: Default::default(), tl_drag: None, tl_open: Default::default(), obj_drag_group: false, group_edit: None, tl_gdrag: None, show_hidden: false, crop_mode: None, crop_aspect: None, outline_mode: None, outline_sel: None, draw_outline: None, draw_slide: 0, editing: None, editing_old: String::new(), edit_turn: None, canvas_frame: None, fresh_text: None, start_edit: None, pending: None, allow_close: false, updater: Default::default(),
            last_autosave: std::time::Instant::now(), recovery_offer: false, pending_delete: None, master_stash: None,
        }
    }
}

/// Snap distance in screen pixels.
const SNAP_PX: f32 = 6.0;

const HANDLES: [(i8, i8); 8] = [(-1, -1), (0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0)];
const HANDLE_PX: f32 = 9.0;
// How close to a handle's centre still grabs it (the drawn square is smaller).
const HANDLE_GRAB_PX: f32 = 11.0;

/// The styled text of `o` laid out in a box `max_width` wide (screen pixels at `scale`).
/// Strikethrough, marker highlight and letter spacing on one run's text format.
fn run_extras(f: &mut egui::text::TextFormat, strike: bool, highlight: Option<Rgba>, spacing: Option<f32>, scale: f32) {
    if strike { f.strikethrough = egui::Stroke::new(1.0, f.color); }
    if let Some(h) = highlight { f.background = color(h); }
    if let Some(z) = spacing { f.extra_letter_spacing = z * scale; }
}

/// Split a job into one job per paragraph (at each `\n`), each keeping the formats of its text. An empty paragraph
/// keeps a zero-length section so it still gets the height of its font.
fn split_paragraphs(job: &egui::text::LayoutJob) -> Vec<egui::text::LayoutJob> {
    let blank = || { let mut j = egui::text::LayoutJob::default(); j.wrap = job.wrap.clone(); j.halign = job.halign; j.justify = job.justify; j.round_output_to_gui = job.round_output_to_gui; j };
    let mut out = vec![];
    let mut cur = blank();
    let mut last_fmt = job.sections.first().map(|s| s.format.clone()).unwrap_or_default();
    for sec in &job.sections {
        let text = &job.text[sec.byte_range.start.0..sec.byte_range.end.0];
        let mut lead = sec.leading_space;
        last_fmt = sec.format.clone();
        let mut pieces = text.split('\n').peekable();
        while let Some(piece) = pieces.next() {
            if !piece.is_empty() || (cur.sections.is_empty() && pieces.peek().is_some()) { cur.append(piece, lead, sec.format.clone()); lead = 0.0; }
            if pieces.peek().is_some() {
                if cur.sections.is_empty() { cur.append("", 0.0, sec.format.clone()); }
                out.push(std::mem::replace(&mut cur, blank()));
            }
        }
    }
    if cur.sections.is_empty() { cur.append("", 0.0, last_fmt); }
    out.push(cur);
    out
}

/// How far a list item's text is set in from its marker, in ems of its font size.
const BULLET_HANG_EM: f32 = 1.3;
const NUMBER_HANG_EM: f32 = 1.6;

/// The paragraph job with its marker ("• " or "3. ") widened to exactly `hang` px and the first row started `hang`
/// to the left, so that once the whole paragraph is moved right by `hang` the marker hangs in the margin and every
/// wrapped line lines up with the text after it.
fn hang_marker(part: &egui::text::LayoutJob, m: usize, extra: f32, hang: f32) -> egui::text::LayoutJob {
    let mut out = egui::text::LayoutJob::default();
    out.wrap = part.wrap.clone(); out.halign = part.halign; out.justify = part.justify; out.round_output_to_gui = part.round_output_to_gui;
    let (b1, b2) = (m - 1, m);
    let mut first = true;
    for sec in &part.sections {
        let (s0, s1) = (sec.byte_range.start.0, sec.byte_range.end.0);
        for (lo, hi, widen) in [(s0, s1.min(b1), false), (s0.max(b1), s1.min(b2), true), (s0.max(b2), s1, false)] {
            if lo >= hi { continue; }
            let mut f = sec.format.clone();
            if widen { f.extra_letter_spacing += extra; }
            out.append(&part.text[lo..hi], if first { sec.leading_space - hang } else { 0.0 }, f);
            first = false;
        }
    }
    out
}

/// Lay a text job out with each paragraph moved in from the left by its own indent (in px): the paragraph wraps in the
/// narrower width, so every line of it is indented, not just the first. List items also hang: their marker sits at the
/// indent and the wrapped lines line up with the text after it. `layout` makes a galley from a job. A text edit places
/// its galley itself and needs it centred on zero, so it passes `centre_shift: false` (centred text then only wraps
/// narrower).
fn layout_indented(layout: &dyn Fn(egui::text::LayoutJob) -> std::sync::Arc<egui::Galley>, job: egui::text::LayoutJob, indents: &[f32], ppp: f32, centre_shift: bool) -> std::sync::Arc<egui::Galley> {
    let is_list = |t: &str| { let m = brainbend_doc::richtext::marker_len(t); m > 0 && t.len() > m };
    if indents.iter().all(|i| *i <= 0.0) && !job.text.split('\n').any(is_list) { return layout(job); }
    let parts = split_paragraphs(&job);
    let halign = job.halign;
    let wrap = job.wrap.max_width;
    let mut galleys = vec![];
    let mut shifts = vec![];
    for (i, mut part) in parts.into_iter().enumerate() {
        let ind = indents.get(i).copied().unwrap_or(0.0).max(0.0).min(wrap * 0.8);
        let m = brainbend_doc::richtext::marker_len(&part.text);
        let mut hang = 0.0;
        if is_list(&part.text) {
            let fmt = part.sections[0].format.clone();
            let em = fmt.font_id.size * if part.text.trim_start_matches('\u{a0}').starts_with('•') { BULLET_HANG_EM } else { NUMBER_HANG_EM };
            let mut probe = egui::text::LayoutJob::default();
            probe.append(&part.text[..m], 0.0, fmt);
            let natural = layout(probe).size().x;
            hang = em.max(natural);
            part = hang_marker(&part, m, hang - natural, hang);
        }
        part.wrap.max_width = (wrap - ind - hang).max(1.0);
        let shift = ind + hang;
        shifts.push(if halign == egui::Align::Center { if centre_shift { shift / 2.0 } else { 0.0 } } else if halign == egui::Align::RIGHT { 0.0 } else { shift });
        galleys.push(layout(part));
    }
    let widest = indents.iter().copied().fold(0.0_f32, f32::max) + wrap * 0.2;
    let mut merged = egui::Galley::concat(std::sync::Arc::new(job), &galleys, ppp);
    merged.mesh_bounds = merged.mesh_bounds.expand(widest);
    let mut row = 0;
    merged.rect = egui::Rect::ZERO;
    for (g, shift) in galleys.iter().zip(&shifts) {
        for _ in 0..g.rows.len() {
            merged.rows[row].pos.x += shift;
            merged.rect |= egui::Rect::from_min_size(merged.rows[row].pos, merged.rows[row].size);
            row += 1;
        }
    }
    std::sync::Arc::new(merged)
}

/// Each paragraph's indent in px, from the object's rich text.
fn indent_px(o: &Object, scale: f32) -> Vec<f32> {
    o.rich.as_ref().map(|ps| ps.iter().map(|p| p.indent as f32 * brainbend_doc::INDENT_STEP * scale).collect()).unwrap_or_default()
}

fn text_job(o: &Object, scale: f32, max_width: f32, link: egui::Color32) -> egui::text::LayoutJob {
    let fmt = egui::text::TextFormat {
        font_id: egui::FontId::new((o.style.size * scale).max(1.0), font_family(&o.style.font_family)),
        color: color(o.style.color),
        coords: egui::epaint::text::VariationCoords::new([(b"wght", if o.style.bold { 700.0 } else { 400.0 })]),
        italics: o.style.italic,
        underline: if o.style.underline { egui::Stroke::new(1.0, color(o.style.color)) } else { egui::Stroke::NONE },
        line_height: (o.line_spacing > 0.0).then(|| (o.style.size * scale * o.line_spacing).max(1.0)),
        ..Default::default()
    };
    let mut job = egui::text::LayoutJob::default();
    match &o.rich {
        Some(paras) => {
            for (pi, para) in paras.iter().enumerate() {
                if pi > 0 { job.append("\n", 0.0, fmt.clone()); }
                for run in &para.runs {
                    let mut f = fmt.clone();
                    run_extras(&mut f, run.strike.unwrap_or(false), run.highlight, run.spacing, scale);
                    if let Some(z) = run.size { f.font_id.size = (z * scale).max(1.0); }
                    if let Some(c) = run.color { f.color = color(c); f.underline = if o.style.underline { egui::Stroke::new(1.0, f.color) } else { egui::Stroke::NONE }; }
                    if let Some(i) = run.italic { f.italics = i; }
                    if let Some(u) = run.underline { f.underline = if u { egui::Stroke::new(1.0, f.color) } else { egui::Stroke::NONE }; }
                    if let Some(b) = run.bold { f.coords = egui::epaint::text::VariationCoords::new([(b"wght", if b { 700.0 } else { 400.0 })]); }
                    if let Some(fam) = &run.font_family { f.font_id.family = font_family(fam); }
                    // A link is the theme's hyperlink colour, underlined, unless the author set its colour or underline.
                    if run.link.is_some() {
                        f.color = run.color.map(color).unwrap_or(link);
                        f.underline = if run.underline.unwrap_or(true) { egui::Stroke::new(1.0, f.color) } else { egui::Stroke::NONE };
                    }
                    job.append(&run.text, 0.0, f);
                }
            }
        }
        None => job.append(&o.text, 0.0, fmt),
    }
    job.wrap.max_width = max_width;
    job.halign = match o.style.align {
        Align::Left => egui::Align::LEFT,
        Align::Center => egui::Align::Center,
        Align::Right => egui::Align::RIGHT,
        Align::Justify => egui::Align::LEFT,
    };
    job.justify = matches!(o.style.align, Align::Justify);
    job
}

/// An outline round `poly`: unbroken, or in dots or dashes.
fn outline_shapes(poly: Vec<egui::Pos2>, stroke: egui::Stroke, dash: brainbend_doc::LineDash) -> Vec<egui::Shape> {
    use brainbend_doc::LineDash;
    let w = stroke.width.max(1.0);
    let (on, off) = match dash { LineDash::Solid => return vec![egui::Shape::closed_line(poly, stroke)], LineDash::Dotted => (w, w * 1.5), LineDash::Dashed => (w * 4.0, w * 2.0) };
    let mut closed = poly;
    if let Some(first) = closed.first().copied() { closed.push(first); }
    egui::Shape::dashed_line(&closed, stroke, on, off)
}

fn g_is_empty(g: &Option<Gradient>) -> bool {
    g.as_ref().is_none_or(|g| g.stops.is_empty())
}

/// Colour of a gradient at position `t` (0..1 along its axis).
fn gradient_at(stops: &[(f32, Rgba)], t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let mut prev = stops[0];
    if t <= prev.0 { return color(prev.1); }
    for &st in &stops[1..] {
        if t <= st.0 {
            let k = if st.0 > prev.0 { (t - prev.0) / (st.0 - prev.0) } else { 1.0 };
            let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * k).round() as u8;
            return egui::Color32::from_rgba_unmultiplied(m(prev.1.0, st.1.0), m(prev.1.1, st.1.1), m(prev.1.2, st.1.2), m(prev.1.3, st.1.3));
        }
        prev = st;
    }
    color(prev.1)
}

/// Where along a CSS-angle gradient over `rect` the point `p` lies (0..1).
fn gradient_t(rect: egui::Rect, angle: f32, p: egui::Pos2) -> f32 {
    let a = angle.to_radians();
    let dir = egui::vec2(a.sin(), -a.cos());
    let len = (rect.width() * a.sin()).abs() + (rect.height() * a.cos()).abs();
    (p - rect.center()).dot(dir) / len.max(1.0) + 0.5
}

/// The topmost visible object at `p` (slide coordinates), as (layer, object) indices.
fn hit_object(slide: &Slide, shown: &[usize], p: egui::Vec2, scale: f32) -> Option<(usize, usize)> {
    for &li in shown.iter().rev() {
        for (oi, o) in slide.layers[li].objects.iter().enumerate().rev() {
            // Lines are thin (or zero-width): give them a few pixels of grab area.
            let pad = if matches!(o.kind, Kind::Line | Kind::LineUp) { 6.0 / scale } else { 0.0 };
            // Turned objects are tested in their own frame.
            let p = if o.rotation != 0.0 {
                let (c, a) = (egui::vec2(o.x + o.w / 2.0, o.y + o.h / 2.0), -o.rotation.to_radians());
                let d = p - c;
                c + egui::vec2(d.x * a.cos() - d.y * a.sin(), d.x * a.sin() + d.y * a.cos())
            } else { p };
            if p.x >= o.x - pad && p.x <= o.x + o.w + pad && p.y >= o.y - pad && p.y <= o.y + o.h + pad {
                // A hotspot with an outline is picked only inside it, so the ones under its bounding box stay reachable.
                if let (Kind::Hotspot, Some(area)) = (&o.kind, o.hit_area.as_ref().filter(|a| a.len() > 2)) {
                    if !outline::contains(area, ((p.x - o.x) / o.w.max(1.0), (p.y - o.y) / o.h.max(1.0))) { continue; }
                }
                return Some((li, oi));
            }
        }
    }
    None
}

/// Linear gradient over `rect` (CSS angle: 0 = towards the top, clockwise), as a vertex-coloured grid.
pub(crate) fn paint_gradient(painter: &egui::Painter, rect: egui::Rect, angle: f32, stops: &[(f32, Rgba)]) {
    if stops.is_empty() { return; }
    const N: usize = 16;
    let mut mesh = egui::epaint::Mesh::default();
    for j in 0..=N {
        for i in 0..=N {
            let p = rect.min + egui::vec2(rect.width() * i as f32 / N as f32, rect.height() * j as f32 / N as f32);
            mesh.colored_vertex(p, gradient_at(stops, gradient_t(rect, angle, p)));
        }
    }
    for j in 0..N {
        for i in 0..N {
            let k = (j * (N + 1) + i) as u32;
            mesh.add_triangle(k, k + 1, k + N as u32 + 1);
            mesh.add_triangle(k + 1, k + N as u32 + 2, k + N as u32 + 1);
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}

/// Fill a polygon (given in `rect`'s unrotated frame) with a gradient or a flat colour; `map` places each
/// vertex on screen (used to rotate).
fn fill_polygon(painter: &egui::Painter, poly: &[egui::Pos2], map: &dyn Fn(egui::Pos2) -> egui::Pos2, shade: &dyn Fn(egui::Pos2) -> egui::Color32) {
    if poly.len() < 3 { return; }
    let mut mesh = egui::epaint::Mesh::default();
    for p in poly { mesh.colored_vertex(map(*p), shade(*p)); }
    for t in triangulate(poly) { mesh.add_triangle(t[0] as u32, t[1] as u32, t[2] as u32); }
    painter.add(egui::Shape::mesh(mesh));
}

/// Where a picture's visible part is drawn inside its frame `r`, and which part of the texture
/// it shows. `None` when the crop leaves nothing visible.
/// A picture cropped to `shape`: the part of the texture inside the shape, as a mesh over the frame `r`.
/// `dest` and `uv` come from [`crop_uv`]; `turn` places each point on screen (it turns a rotated picture).
fn shaped_picture(tex: egui::TextureId, r: egui::Rect, dest: egui::Rect, uv: egui::Rect, shape: brainbend_doc::PictureShape, size: (f32, f32), turn: &dyn Fn(egui::Pos2) -> egui::Pos2) -> egui::epaint::Mesh {
    let frac = shape.polygon(size.0, size.1);
    let flat: Vec<egui::Pos2> = frac.iter().map(|&(x, y)| egui::pos2(r.left() + x * r.width(), r.top() + y * r.height())).collect();
    let mut mesh = egui::epaint::Mesh::with_texture(tex);
    for p in &flat {
        let (u, v) = (((p.x - dest.left()) / dest.width().max(0.001)).clamp(0.0, 1.0), ((p.y - dest.top()) / dest.height().max(0.001)).clamp(0.0, 1.0));
        mesh.vertices.push(egui::epaint::Vertex { pos: turn(*p), uv: egui::pos2(uv.left() + u * uv.width(), uv.top() + v * uv.height()), color: egui::Color32::WHITE });
    }
    for t in triangulate(&flat) { mesh.add_triangle(t[0] as u32, t[1] as u32, t[2] as u32); }
    mesh
}

fn crop_uv(r: egui::Rect, crop: Option<[f32; 4]>) -> Option<(egui::Rect, egui::Rect)> {
    let Some(c) = crop else { return Some((r, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)))) };
    let [x, y, w, h] = brainbend_doc::crop_layout(c);
    let full = egui::Rect::from_min_size(egui::pos2(r.min.x + x * r.width(), r.min.y + y * r.height()), egui::vec2(w * r.width(), h * r.height()));
    let dest = full.intersect(r);
    if dest.width() <= 0.0 || dest.height() <= 0.0 || full.width() <= 0.0 || full.height() <= 0.0 { return None; }
    let at = |p: egui::Pos2| egui::pos2((p.x - full.min.x) / full.width(), (p.y - full.min.y) / full.height());
    Some((dest, egui::Rect::from_min_max(at(dest.min), at(dest.max))))
}

/// Whether `p` is inside the screen rectangle `r` after it has been turned by `angle` radians about its centre.
fn inside_frame(r: egui::Rect, angle: f32, p: egui::Pos2) -> bool {
    let d = p - r.center();
    r.contains(r.center() + egui::vec2(d.x * angle.cos() + d.y * angle.sin(), -d.x * angle.sin() + d.y * angle.cos()))
}

fn rounded_rect_poly(r: egui::Rect, radius: f32) -> Vec<egui::Pos2> {
    let rad = radius.clamp(0.0, r.width().min(r.height()) / 2.0);
    if rad < 0.5 { return vec![r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()]; }
    let mut pts = vec![];
    let corners = [(r.right_top() + egui::vec2(-rad, rad), -90.0f32), (r.right_bottom() + egui::vec2(-rad, -rad), 0.0), (r.left_bottom() + egui::vec2(rad, -rad), 90.0), (r.left_top() + egui::vec2(rad, rad), 180.0)];
    for (c, start) in corners {
        for i in 0..=6 {
            let a = (start + 15.0 * i as f32).to_radians();
            pts.push(c + egui::vec2(a.cos(), a.sin()) * rad);
        }
    }
    pts
}

fn ellipse_poly(r: egui::Rect) -> Vec<egui::Pos2> {
    (0..64).map(|i| {
        let a = std::f32::consts::TAU * i as f32 / 64.0;
        r.center() + egui::vec2(a.cos() * r.width() / 2.0, a.sin() * r.height() / 2.0)
    }).collect()
}

/// Flatten a path into one screen-space polygon per figure (Beziers sampled at 12 steps).
fn path_polygons(cmds: &[PathCmd], r: egui::Rect) -> Vec<Vec<egui::Pos2>> {
    let at = |x: f32, y: f32| egui::pos2(r.min.x + x * r.width(), r.min.y + y * r.height());
    let mut out: Vec<Vec<egui::Pos2>> = vec![];
    let mut cur: Vec<egui::Pos2> = vec![];
    for c in cmds {
        match *c {
            PathCmd::Move(x, y) => {
                if cur.len() > 2 { out.push(std::mem::take(&mut cur)); } else { cur.clear(); }
                cur.push(at(x, y));
            }
            PathCmd::Line(x, y) => cur.push(at(x, y)),
            PathCmd::Curve(x1, y1, x2, y2, x3, y3) => {
                let p0 = cur.last().copied().unwrap_or(at(x1, y1));
                let (p1, p2, p3) = (at(x1, y1), at(x2, y2), at(x3, y3));
                for i in 1..=12 {
                    let t = i as f32 / 12.0;
                    let u = 1.0 - t;
                    let v = p0.to_vec2() * (u * u * u) + p1.to_vec2() * (3.0 * u * u * t) + p2.to_vec2() * (3.0 * u * t * t) + p3.to_vec2() * (t * t * t);
                    cur.push(v.to_pos2());
                }
            }
            PathCmd::Close => {
                if cur.len() > 2 { out.push(std::mem::take(&mut cur)); } else { cur.clear(); }
            }
        }
    }
    if cur.len() > 2 { out.push(cur); }
    out
}

/// Triangles for the figures of a path, filled the way the published SVG fills it (non-zero winding: a figure wound
/// against the one around it is a hole, overlapping figures merge). i_overlay turns the figures into plain shapes (an
/// outline and its holes); earcut triangulates those, and copes with the long, thin, nearly straight stretches a font's
/// outlines have (plain ear clipping gave up on them and left gaps). Each entry is the points of one shape (outline,
/// then holes) and triangles as indices into them.
fn figure_triangles(figs: &[Vec<egui::Pos2>]) -> Vec<(Vec<egui::Pos2>, Vec<usize>)> {
    use i_overlay::core::fill_rule::FillRule;
    use i_overlay::float::simplify::SimplifyShape;
    let contours: Vec<Vec<[f32; 2]>> = figs.iter().filter(|f| f.len() > 2).map(|f| f.iter().map(|p| [p.x, p.y]).collect()).collect();
    let mut out = vec![];
    for shape in contours.simplify_shape(FillRule::NonZero) {
        let mut pts: Vec<egui::Pos2> = vec![];
        let mut starts = vec![];
        for (k, contour) in shape.iter().enumerate() {
            if k > 0 { starts.push(pts.len()); }
            pts.extend(contour.iter().map(|p| egui::pos2(p[0], p[1])));
        }
        if pts.len() < 3 { continue; }
        let flat: Vec<f32> = pts.iter().flat_map(|p| [p.x, p.y]).collect();
        if let Ok(tris) = earcutr::earcut(&flat, &starts, 2) { out.push((pts, tris)); }
    }
    out
}

/// Fill the figures of a path (see [`figure_triangles`]). `map` places each point on screen and `shade` colours it.
fn fill_figures(painter: &egui::Painter, figs: &[Vec<egui::Pos2>], map: &dyn Fn(egui::Pos2) -> egui::Pos2, shade: &dyn Fn(egui::Pos2) -> egui::Color32) {
    let mut mesh = egui::epaint::Mesh::default();
    for (pts, tris) in figure_triangles(figs) {
        let base = mesh.vertices.len() as u32;
        for p in &pts { mesh.colored_vertex(map(*p), shade(*p)); }
        for t in tris.chunks_exact(3) { mesh.add_triangle(base + t[0] as u32, base + t[1] as u32, base + t[2] as u32); }
    }
    painter.add(egui::Shape::mesh(mesh));
}

/// Ear-clipping triangulation of a simple polygon (indices into `poly`); concave shapes fill correctly.
fn triangulate(poly: &[egui::Pos2]) -> Vec<[usize; 3]> {
    let n = poly.len();
    let area: f32 = (0..n).map(|i| { let (a, b) = (poly[i], poly[(i + 1) % n]); a.x * b.y - b.x * a.y }).sum();
    let mut idx: Vec<usize> = if area >= 0.0 { (0..n).collect() } else { (0..n).rev().collect() };
    let cross = |a: egui::Pos2, b: egui::Pos2, c: egui::Pos2| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    let mut tris = vec![];
    let mut guard = 0;
    while idx.len() > 3 && guard < n * n {
        guard += 1;
        let m = idx.len();
        let mut clipped = false;
        for i in 0..m {
            let (ia, ib, ic) = (idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]);
            let (a, b, c) = (poly[ia], poly[ib], poly[ic]);
            if cross(a, b, c) <= 0.0 { continue; }
            let inside = idx.iter().any(|&k| {
                if k == ia || k == ib || k == ic { return false; }
                let p = poly[k];
                cross(a, b, p) >= 0.0 && cross(b, c, p) >= 0.0 && cross(c, a, p) >= 0.0
            });
            if !inside {
                tris.push([ia, ib, ic]);
                idx.remove(i);
                clipped = true;
                break;
            }
        }
        if !clipped { break; }
    }
    if idx.len() == 3 { tris.push([idx[0], idx[1], idx[2]]); }
    tris
}

fn plain_style(size: f32, color: Rgba, bold: bool, align: Align) -> TextStyle {
    TextStyle { font_family: "Open Sans".into(), size, bold, italic: false, underline: false, color, align, valign: VAlign::Middle }
}

fn blank_object(id: String, name: &str, kind: Kind, x: f32, y: f32, w: f32, h: f32) -> Object {
    Object::new(&id, name, kind, x, y, w, h)
}

impl App {
    fn open(&mut self, path: &PathBuf) {
        self.leave_master_view();
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Untitled").to_string();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        let result = std::fs::File::open(path).map_err(|e| e.to_string()).and_then(|mut f| match ext.as_str() {
            "story" => brainbend_import_legacy::import(f, &name),
            "pptx" => brainbend_import_pptx::import(f, &name),
            _ => {
                let mut bytes = Vec::new();
                std::io::Read::read_to_end(&mut f, &mut bytes).map_err(|e| e.to_string())?;
                Project::from_bytes(&bytes)
            }
        });
        match result {
            Ok(mut p) => {
                p.sanitize();
                // Import notes are a one-time report; they are shown once and not kept in the project.
                let report = std::mem::take(&mut p.notes);
                self.status = format!("Opened {} ({} slides, {} import notes)", file_label(&path), p.slides.len(), report.len());
                self.import_report = report;
                self.scene_view = self.open_to_story_view;
                self.tabs.clear();
                self.active_tab = None;
                self.project = Some(p);
                self.file_view = false;
                self.file_page = backstage::FilePage::Info;
                // Imported files are saved as a new .brainbend; never overwrite the source.
                self.path = if ext == "story" || ext == "pptx" { None } else { Some(path.clone()) };
                self.slide = 0;
                self.layer = 0;
                self.selected = None;
                self.sec = 0;
                self.blk = None;
                self.textures.clear();
                self.reset_visibility();
                self.undo.clear();
                self.redo.clear();
                self.dirty = ext == "story" || ext == "pptx";
            }
            Err(e) => self.status = format!("Could not open {}: {e}", file_label(&path)),
        }
    }

    /// Hide the layer being worked on (its eye was clicked) and go back to the base layer.
    fn hide_selected_layer(&mut self) {
        if self.layer > 0 {
            if let Some(v) = self.visible.get_mut(self.layer) { *v = false; }
            self.layer = 0;
            self.selected = None;
        }
    }

    /// The layers drawn on the canvas: the base layer, the selected layer, and any layer whose eye is on.
    fn shown_layers(&self, layers: usize) -> Vec<usize> {
        (0..layers).filter(|&i| i == 0 || i == self.layer || self.visible.get(i).copied().unwrap_or(false)).collect()
    }

    /// Layers visible when the slide starts: the base layer plus any a slide-start trigger shows.
    fn reset_visibility(&mut self) {
        let Some(p) = &self.project else { return };
        let Some(slide) = p.slides.get(self.slide) else { return };
        let mut vis = vec![false; slide.layers.len()];
        vis[0] = true;
        for t in &slide.triggers {
            if let (Event::SlideStart, Action::ShowLayer(id), Owner::Slide) = (&t.event, &t.action, &t.owner) {
                if let Some(i) = slide.layers.iter().position(|l| &l.id == id) {
                    vis[i] = true;
                }
            }
        }
        self.visible = vis;
    }

    fn save(&mut self, as_new: bool) {
        self.end_state_swap();
        self.leave_master_view();
        let Some(p) = &self.project else { return };
        let target = if as_new || self.path.is_none() {
            rfd::FileDialog::new().add_filter("BrainBend Studio project", &["brainbend"]).set_file_name(format!("{}.brainbend", p.name)).save_file()
        } else {
            self.path.clone()
        };
        let Some(target) = target else { return };
        let tmp = target.with_extension("brainbend.tmp");
        let res = p.to_package().and_then(|b| {
            std::fs::write(&tmp, b).map_err(|e| e.to_string())?;
            std::fs::rename(&tmp, &target).map_err(|e| e.to_string())
        });
        match res {
            Ok(()) => {
                self.status = format!("Saved {}", file_label(&target));
                self.path = Some(target);
                self.dirty = false;
                Self::clear_recovery();
            }
            Err(e) => self.status = format!("Save failed: {e}"),
        }
    }

    /// Write a recovery copy when there are unsaved changes (called periodically).
    fn autosave(&mut self) {
        self.last_autosave = std::time::Instant::now();
        let (Some(p), Some(path)) = (self.committed(), recovery_path()) else { return };
        if !self.dirty { return; }
        if let Some(dir) = path.parent() { let _ = std::fs::create_dir_all(dir); }
        if let Ok(bytes) = p.to_package() {
            let tmp = path.with_extension("tmp");
            if std::fs::write(&tmp, bytes).is_ok() { let _ = std::fs::rename(&tmp, &path); }
        }
    }

    fn clear_recovery() {
        if let Some(p) = recovery_path() { let _ = std::fs::remove_file(p); }
    }

    /// Run `action` now, or ask about unsaved changes first.
    fn guard(&mut self, action: Pending) {
        if self.dirty && self.project.is_some() { self.pending = Some(action); } else { self.perform(action); }
    }

    fn perform(&mut self, action: Pending) {
        self.leave_master_view();
        match action {
            Pending::Quit => self.allow_close = true,
            Pending::Update => match self.updater.install_now() {
                Ok(()) => self.allow_close = true,
                Err(e) => { self.status = format!("Could not update: {e}"); self.updater.dialog_open = true; }
            },
            Pending::New => self.new_project(),
            Pending::NewLesson(kind) => self.new_lesson_project(kind),
            Pending::OpenDropped => {
                if let Some(p) = self.drop_path.take() { self.open(&p); }
            }
            Pending::Open => {
                if let Some(p) = rfd::FileDialog::new().add_filter("BrainBend Studio, .story or PowerPoint files", &["brainbend", "storyapp", "story", "pptx"]).pick_file() {
                    self.open(&p);
                }
            }
        }
    }

    /// Preview in the browser: all of the project, or a scene, or slides from the current one on.
    fn preview(&mut self, scope: PreviewScope) {
        self.end_state_swap();
        self.leave_master_view();
        let Some(whole) = &self.project else { return };
        let cur = self.slide.min(whole.slides.len() - 1);
        let indices: Vec<usize> = match scope {
            PreviewScope::All => (0..whole.slides.len()).collect(),
            PreviewScope::Scene => whole.scene_slides(self.current_scene()),
            PreviewScope::Slide => vec![cur],
            PreviewScope::FromHere => (cur..whole.slides.len()).collect(),
            PreviewScope::Selected => self.slide_selection(),
        };
        let part = if whole.is_web() { whole.clone() } else { whole.subset(&indices) };
        let p = &part;
        let dir = std::env::temp_dir().join("brainbend-preview");
        match brainbend_publish_html::export(p, &dir) {
            Ok(()) => {
                let index = dir.join("index.html");
                let opener = if cfg!(target_os = "macos") { "open" } else if cfg!(target_os = "windows") { "explorer" } else { "xdg-open" };
                match std::process::Command::new(opener).arg(&index).spawn() {
                    Ok(_) => self.status = format!("Preview opened from {}", index.display()),
                    Err(e) => self.status = format!("Exported preview to {} but could not open it: {e}", index.display()),
                }
            }
            Err(e) => self.status = format!("Preview failed: {e}"),
        }
    }

    fn export_html(&mut self) {
        self.end_state_swap();
        self.leave_master_view();
        let Some(p) = &self.project else { return };
        let Some(dir) = rfd::FileDialog::new().set_title("Choose an empty folder for the HTML5 export").pick_folder() else { return };
        self.status = match brainbend_publish_html::export(p, &dir) {
            Ok(()) => format!("Exported HTML5 to {}", dir.display()),
            Err(e) => format!("Export failed: {e}"),
        };
    }

    /// Save the project as a PowerPoint file.
    fn export_pptx(&mut self) {
        self.end_state_swap();
        self.leave_master_view();
        let Some(p) = &self.project else { return };
        let Some(path) = rfd::FileDialog::new().add_filter("PowerPoint presentation", &["pptx"]).set_file_name(format!("{}.pptx", p.name)).save_file() else { return };
        match brainbend_export_pptx::export(p) {
            Ok((bytes, notes)) => match std::fs::write(&path, bytes) {
                Ok(()) => {
                    self.status = format!("Exported PowerPoint file to {}", path.display());
                    self.export_report = notes;
                }
                Err(e) => self.status = format!("Could not write {}: {e}", path.display()),
            },
            Err(e) => self.status = format!("PowerPoint export failed: {e}"),
        }
    }

    fn export_scorm(&mut self, version: brainbend_publish_html::Lms) {
        self.end_state_swap();
        self.leave_master_view();
        let Some(p) = &self.project else { return };
        let tag = if version == brainbend_publish_html::Lms::Scorm12 { "scorm12" } else { "scorm2004" };
        let Some(path) = rfd::FileDialog::new()
            .add_filter("SCORM package", &["zip"])
            .set_file_name(format!("{}-{tag}.zip", p.name))
            .save_file()
        else {
            return;
        };
        self.status = match brainbend_publish_html::export_scorm(p, version, &path) {
            Ok(()) => format!("Exported SCORM package to {}", path.display()),
            Err(e) => format!("SCORM export failed: {e}"),
        };
    }

    /// The bar along the bottom: where you are, what is wrong, and the zoom.
    fn status_bar(&mut self, root: &mut egui::Ui, ctx: &egui::Context) {
        self.updater.poll();
        // Counting the issues walks the whole project, so it is done now and then, not every frame.
        let now = ctx.input(|i| i.time);
        let sig = (self.undo.len(), self.redo.len(), self.project.as_ref().map(|p| p.slides.len() + 1).unwrap_or(0));
        if now - self.status_issues.0 > 2.0 || sig != self.status_sig {
            self.status_sig = sig;
            self.status_issues = (now, self.project.as_ref().map(|p| p.check().len()).unwrap_or(0));
        }
        let info = self.project.as_ref().map(|p| {
            let s = &p.slides[self.slide.min(p.slides.len().saturating_sub(1))];
            (self.slide.min(p.slides.len().saturating_sub(1)) + 1, p.slides.len(), s.width as i32, s.height as i32, p.theme.name.clone())
        });
        let editing_slide = self.project.is_some() && !self.file_view && !self.scene_view && self.pano.edit.is_none() && !self.project.as_ref().is_some_and(|p| p.is_web());
        let mut open_checker = false;
        let mut zoom_to: Option<f32> = None;
        let mut fit = false;
        egui::Panel::bottom("status").show(root, |ui| {
            ui.horizontal(|ui| {
                if let Some((n, total, w, h, theme)) = &info {
                    ui.label(format!("Slide {n} of {total}"));
                    ui.separator();
                    ui.label(format!("{w} × {h}"));
                    ui.separator();
                    ui.label(format!("\"{theme}\""));
                    ui.separator();
                    let count = self.status_issues.1;
                    let glyph = ui.add(egui::Label::new(egui::RichText::new(ribbon::ACCESS.to_string()).family(ribbon::icons())).sense(egui::Sense::click())).on_hover_text("Open the Accessibility Checker");
                    let words = ui.add(egui::Label::new(format!("{} {}", count, if count == 1 { "issue" } else { "issues" })).sense(egui::Sense::click())).on_hover_text("Open the Accessibility Checker");
                    if glyph.clicked() || words.clicked() { open_checker = true; }
                    ui.separator();
                }
                ui.label(format!("{}{}", self.status, if self.dirty { "   • unsaved changes" } else { "" }));
                if editing_slide {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let scale = self.fit_scale * self.zoom;
                        if ui.add(egui::Button::new(egui::RichText::new(ribbon::FRAME.to_string()).family(ribbon::icons())).frame(false)).on_hover_text("Fit the slide in the window").clicked() { fit = true; }
                        if ui.add(egui::Button::new("+").frame(false)).on_hover_text("Zoom in").clicked() { zoom_to = Some(scale * 1.25); }
                        let mut pct = (scale * 100.0).clamp(10.0, 800.0);
                        ui.spacing_mut().slider_width = 110.0;
                        if ui.add(egui::Slider::new(&mut pct, 10.0..=800.0).logarithmic(true).show_value(false)).changed() { zoom_to = Some(pct / 100.0); }
                        if ui.add(egui::Button::new("−").frame(false)).on_hover_text("Zoom out").clicked() { zoom_to = Some(scale / 1.25); }
                        if ui.add(egui::Button::new(format!("{:.0}%", scale * 100.0)).frame(false)).on_hover_text("Actual size").clicked() { zoom_to = Some(1.0); }
                    });
                }
            });
        });
        if fit { self.zoom = 1.0; self.pan = egui::Vec2::ZERO; }
        if let Some(sc) = zoom_to {
            self.zoom = (sc / self.fit_scale.max(0.01)).clamp(0.1, 8.0);
            self.pan = egui::Vec2::ZERO;
        }
        if open_checker { self.check_results = self.project.as_ref().map(|p| p.check()); }
    }

    fn new_id(&mut self, prefix: &str) -> String {
        self.id_counter += 1;
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        format!("{prefix}-{t:x}-{}", self.id_counter)
    }

    fn new_project(&mut self) {
        self.leave_master_view();
        let mut p = Project::new("Untitled");
        p.slides.push(Slide {
            id: self.new_id("slide"),
            name: "Slide 1".into(),
            width: 1920.0,
            height: 1080.0,
            layers: vec![Layer { id: self.new_id("layer"), name: "Base layer".into(), objects: vec![], duration_ms: None, modal: true, hide_others: false, pause_base: false, group_names: Default::default(), focus_order: vec![] }],
            triggers: vec![],
            kind: SlideKind::Normal,
            background: None,
            layout: None,
            fade_ms: 0, hide_backdrop: false,
            auto_advance: false, nav: Default::default(),
            notes: String::new(),
            scene: String::new(),
        });
        p.normalize_scenes();
        p.ensure_layouts();
        self.project = Some(p);
        self.scene_view = false;
        self.file_view = false;
        self.file_page = backstage::FilePage::Info;
        self.path = None;
        self.slide = 0;
        self.layer = 0;
        self.selected = None;
        self.textures.clear();
        self.reset_visibility();
        self.undo.clear();
        self.redo.clear();
        self.dirty = true;
        self.status = "New project".into();
    }

    fn new_lesson_project(&mut self, kind: ProjectKind) {
        self.leave_master_view();
        let mut p = Project::new_web_lesson("Untitled", kind);
        if let Some(l) = p.lesson.as_mut() {
            l.sections[0].id = self.new_id("section");
            for b in l.sections[0].blocks.iter_mut() { b.id = self.new_id("block"); }
        }
        self.project = Some(p);
        self.scene_view = false;
        self.file_view = false;
        self.file_page = backstage::FilePage::Info;
        self.path = None;
        self.slide = 0;
        self.layer = 0;
        self.selected = None;
        self.sec = 0;
        self.blk = None;
        self.textures.clear();
        self.reset_visibility();
        self.undo.clear();
        self.redo.clear();
        self.dirty = true;
        self.status = format!("New {}", kind.label());
    }

    /// Add an object to the active layer (centered on the slide) and select it.
    fn insert(&mut self, mut obj: Object) {
        let Some(p) = &self.project else { return };
        let slide = &p.slides[self.slide];
        let li = self.layer.min(slide.layers.len() - 1);
        // Centered, then cascaded so consecutive inserts do not hide each other.
        let cascade = (slide.layers[li].objects.len() % 8) as f32 * 36.0;
        obj.x = ((slide.width - obj.w) / 2.0 + cascade - 120.0).round().max(0.0);
        obj.y = ((slide.height - obj.h) / 2.0 + cascade - 120.0).round().max(0.0);
        self.snapshot();
        let slide = &mut self.project.as_mut().unwrap().slides[self.slide];
        slide.layers[li].objects.push(obj);
        self.selected = Some((li, slide.layers[li].objects.len() - 1));
        // The new object's formatting is on Home; the author can click to another tab afterwards.
        self.ribbon_tab = ribbon::HOME_TAB;
    }

    fn insert_text(&mut self) {
        let mut o = blank_object(self.new_id("obj"), "Text", Kind::Text, 0.0, 0.0, 640.0, 110.0);
        o.text = "Type your text".into();
        // Like PowerPoint, a text box anchors its text at the top; shapes keep it centred.
        o.style.valign = VAlign::Top;
        self.insert(o);
    }

    /// A text box whose text (`shown`, else the address itself) links to `url`. The ribbon stays on Insert.
    fn insert_link_text(&mut self, shown: &str, url: &str) {
        let tab = self.ribbon_tab;
        self.insert_text();
        self.ribbon_tab = tab;
        let slide = self.slide;
        if let Some(o) = self.selected.and_then(|(li, oi)| self.project.as_mut()?.slides[slide].layers.get_mut(li)?.objects.get_mut(oi)) {
            o.edit_text(if shown.is_empty() { url } else { shown });
            let n = o.text.chars().count();
            o.format_range(0, n, &Attr::Link(Some(url.to_string())));
        }
        self.dirty = true;
    }

    fn insert_ellipse(&mut self) {
        let mut o = blank_object(self.new_id("obj"), "Ellipse", Kind::Ellipse, 0.0, 0.0, 360.0, 360.0);
        o.fill = Some(Rgba(3, 83, 158, 255));
        self.insert(o);
    }

    fn insert_line(&mut self) {
        let mut o = blank_object(self.new_id("obj"), "Line", Kind::Line, 0.0, 0.0, 400.0, 0.0);
        o.stroke = Some(Stroke { color: Rgba(0, 0, 0, 255), width: 4.0 });
        self.insert(o);
    }

    /// Insert a table of `rows` x `cols` cells on the active layer.
    fn insert_table(&mut self, rows: u16, cols: u16) {
        let Some(p) = &self.project else { return };
        let slide = &p.slides[self.slide];
        let li = self.layer.min(slide.layers.len() - 1);
        let w = (slide.width * 0.7).min(cols as f32 * 360.0);
        let h = (rows as f32 * 90.0).min(slide.height * 0.8);
        let origin = ((slide.width - w) / 2.0, (slide.height - h) / 2.0, w, h);
        let theme = p.theme.clone();
        let group = self.new_id("table");
        let mut ids: Vec<String> = (0..rows as usize * cols as usize).map(|_| self.new_id("obj")).collect();
        ids.reverse();
        let cells = brainbend_doc::tables::new_table(&group, rows, cols, origin, &theme, &mut || ids.pop().unwrap_or_default());
        self.snapshot();
        let layer = &mut self.project.as_mut().unwrap().slides[self.slide].layers[li];
        let first = layer.objects.len();
        layer.objects.extend(cells);
        self.layer = li;
        self.selected = Some((li, first));
        self.ribbon_tab = ribbon::HOME_TAB;
        self.multi.clear();
        self.dirty = true;
    }

    /// Insert or delete a row or column of the selected table cell's table.
    fn table_edit(&mut self, op: TableOp) {
        use brainbend_doc::tables as t;
        let Some((li, oi)) = self.selected else { return };
        let Some(o) = self.project.as_ref().and_then(|p| p.slides.get(self.slide)).and_then(|s| s.layers.get(li)).and_then(|l| l.objects.get(oi)) else { return };
        let (Some((r, c)), Some(g)) = (o.table, o.group.clone()) else { return };
        let mut ids: Vec<String> = (0..64).map(|_| self.new_id("obj")).collect();
        ids.reverse();
        let mut next = || ids.pop().unwrap_or_default();
        self.snapshot();
        let objs = &mut self.project.as_mut().unwrap().slides[self.slide].layers[li].objects;
        match op {
            TableOp::RowAbove => t::insert_row(objs, &g, r, false, &mut next),
            TableOp::RowBelow => t::insert_row(objs, &g, r, true, &mut next),
            TableOp::ColLeft => t::insert_col(objs, &g, c, false, &mut next),
            TableOp::ColRight => t::insert_col(objs, &g, c, true, &mut next),
            TableOp::DeleteRow => t::delete_row(objs, &g, r),
            TableOp::DeleteCol => t::delete_col(objs, &g, c),
        }
        // Deleting can remove the selected cell; keep a cell of the same table selected.
        let objs = &self.project.as_ref().unwrap().slides[self.slide].layers[li].objects;
        self.selected = objs.iter().position(|o| o.group.as_deref() == Some(g.as_str()) && o.table == Some((r, c)))
            .or_else(|| objs.iter().position(|o| o.group.as_deref() == Some(g.as_str()))).map(|k| (li, k));
        self.multi.clear();
        self.dirty = true;
    }

    /// Insert a shape from the gallery by its preset name.
    fn insert_shape(&mut self, name: &str) {
        use brainbend_doc::shapes::{preset, Preset};
        match name {
            "rect" => return self.insert_rect(),
            "ellipse" => return self.insert_ellipse(),
            "line" => return self.insert_line(),
            _ => {}
        }
        let accent = self.project.as_ref().map(|p| p.theme.color(4)).unwrap_or(Rgba(3, 83, 158, 255));
        let (w, h) = match name { "rightArrow" | "leftArrow" | "leftRightArrow" | "chevron" | "homePlate" | "trapezoid" | "parallelogram" | "wedgeRectCallout" | "roundRect" | "flowChartTerminator" => (480.0, 280.0), _ => (320.0, 320.0) };
        let (kind, radius) = match preset(name, w, h, &|_, d| d) {
            Preset::Path(cmds) => (Kind::Path(cmds), 0.0),
            Preset::Round(r) => (Kind::Rect, r * w.min(h)),
            Preset::Ellipse => (Kind::Ellipse, 0.0),
            _ => (Kind::Rect, 0.0),
        };
        let label = brainbend_doc::shapes::GALLERY.iter().find(|g| g.0 == name).map(|g| g.1).unwrap_or("Shape");
        let mut o = blank_object(self.new_id("obj"), label, kind, 0.0, 0.0, w, h);
        o.radius = radius;
        o.fill = Some(accent);
        self.insert(o);
    }

    fn insert_rect(&mut self) {
        let mut o = blank_object(self.new_id("obj"), "Rectangle", Kind::Rect, 0.0, 0.0, 480.0, 280.0);
        o.fill = Some(Rgba(3, 83, 158, 255));
        self.insert(o);
    }

    /// A button with every state's look (the first style of the Button Styles gallery).
    fn insert_button(&mut self) {
        let mut o = blank_object(self.new_id("obj"), "Button", Kind::Rect, 0.0, 0.0, 420.0, 120.0);
        o.text = "Button".into();
        o.style = plain_style(40.0, Rgba(255, 255, 255, 255), true, Align::Center);
        button_styles::apply(&mut o, &button_styles::STYLES[0]);
        self.insert(o);
    }

    /// Ask for a PNG/JPEG, store it as an asset and return (asset key, file name, width, height).
    fn load_image_asset(&mut self) -> Option<(String, String, f32, f32)> {
        self.load_image_asset_titled("Insert Picture")
    }

    fn load_image_asset_titled(&mut self, title: &str) -> Option<(String, String, f32, f32)> {
        let path = rfd::FileDialog::new().set_title(title).add_filter("Image", &["png", "jpg", "jpeg"]).pick_file()?;
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => { self.status = format!("Could not read {}: {e}", path.display()); return None; }
        };
        let (w, h) = match image::load_from_memory(&bytes) {
            Ok(img) => (img.width() as f32, img.height() as f32),
            Err(e) => { self.status = format!("Not a usable PNG/JPEG ({e})"); return None; }
        };
        let mime = if bytes.starts_with(&[0x89, b'P', b'N', b'G']) { "image/png" } else { "image/jpeg" };
        let key = self.new_id("asset");
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("image").to_string();
        self.project.as_mut()?.assets.insert(key.clone(), Asset { name: name.clone(), mime: mime.into(), bytes, file: None });
        Some((key, name, w, h))
    }

    /// A slider that sets a new number variable of its own.
    fn insert_slider(&mut self) {
        let Some(p) = &self.project else { return };
        let mut n = 1;
        while p.variables.iter().any(|v| v.name == format!("Slider{n}")) { n += 1; }
        let (var_id, var_name) = (self.new_id("var"), format!("Slider{n}"));
        let slider = Slider { variable: var_id.clone(), ..Slider::new() };
        let o = blank_object(self.new_id("obj"), "Slider", Kind::Slider(slider), 0.0, 0.0, 600.0, 60.0);
        // The insert records the undo step, so the variable added after it is part of the same step.
        self.insert(o);
        if let Some(p) = self.project.as_mut() { p.variables.push(Variable { id: var_id, name: var_name, initial: Value::Num(0.0), built_in: false }); }
    }

    fn insert_image(&mut self) {
        let Some((key, name, w, h)) = self.load_image_asset() else { return };
        let fit = (900.0 / w).min(700.0 / h).min(1.0);
        let mut o = blank_object(self.new_id("obj"), &name, Kind::Image, 0.0, 0.0, (w * fit).round(), (h * fit).round());
        o.asset = Some(key);
        self.insert(o);
    }

    fn add_slide(&mut self, kind: SlideKind) {
        let layout = self.project.as_ref().and_then(|p| p.slides.get(self.slide)).and_then(|s| s.layout.clone());
        self.add_slide_with_layout(kind, layout);
    }

    /// Add a slide after the current one, following `layout` (its placeholders are added).
    fn add_slide_with_layout(&mut self, kind: SlideKind, layout: Option<String>) {
        if self.project.is_none() { return; }
        self.snapshot();
        let id = self.new_id("slide");
        let (w, h) = {
            let s = &self.project.as_ref().unwrap().slides[self.slide];
            (s.width, s.height)
        };
        let n = self.project.as_ref().unwrap().slides.len() + 1;
        let mut slide = Slide::new(&id, &match &kind {
            SlideKind::Normal => format!("Slide {n}"),
            SlideKind::Question(_) => format!("Question {n}"),
            SlideKind::Results => "Results".to_string(),
        }, w, h);
        slide.kind = kind;
        slide.scene = self.project.as_ref().unwrap().slides[self.slide].scene.clone();
        let at = (self.slide + 1).min(self.project.as_ref().unwrap().slides.len());
        let follow = layout.filter(|_| matches!(slide.kind, SlideKind::Normal));
        self.project.as_mut().unwrap().slides.insert(at, slide);
        if let Some(l) = follow {
            let mut ids = (0..8).map(|_| self.new_id("obj")).collect::<Vec<_>>();
            self.project.as_mut().unwrap().apply_layout(at, &l, &mut || ids.pop().unwrap_or_default());
        }
        self.slide = at;
        self.layer = 0;
        self.selected = None;
        self.reset_visibility();
    }

    /// Switch `slide_idx` to `layout` (None detaches it, keeping its content).
    fn set_slide_layout(&mut self, slide_idx: usize, layout: Option<String>) {
        if self.project.is_none() { return; }
        self.snapshot();
        let mut ids = (0..8).map(|_| self.new_id("obj")).collect::<Vec<_>>();
        let p = self.project.as_mut().unwrap();
        match layout {
            Some(l) => p.apply_layout(slide_idx, &l, &mut || ids.pop().unwrap_or_default()),
            None => p.slides[slide_idx].layout = None,
        }
        self.selected = None;
        self.dirty = true;
    }

    /// Index (into `scenes`) of the scene the current slide is in.
    fn current_scene(&self) -> usize {
        let Some(p) = &self.project else { return 0 };
        let sid = &p.slides[self.slide.min(p.slides.len() - 1)].scene;
        p.scenes.iter().position(|c| c.id == *sid).unwrap_or(0)
    }

    /// Open a slide in the editor (leaving Scene View).
    fn open_slide(&mut self, i: usize) {
        self.slide = i;
        self.layer = 0;
        self.selected = None;
        self.scene_view = false;
        self.reset_visibility();
    }

    /// A new scene, with one blank slide, right after the current scene.
    fn add_scene(&mut self) {
        if self.project.is_none() { return; }
        self.snapshot();
        let sid = self.new_id("scene");
        let id = self.new_id("slide");
        let p = self.project.as_mut().unwrap();
        let cur = self.slide.min(p.slides.len() - 1);
        let ci = p.scenes.iter().position(|c| c.id == p.slides[cur].scene).unwrap_or(0);
        let mut slide = Slide::new(&id, "Slide 1", p.slides[cur].width, p.slides[cur].height);
        slide.scene = sid.clone();
        let name = format!("Scene {}", p.scenes.len() + 1);
        p.scenes.insert(ci + 1, Scene { id: sid, name, bank: None });
        let at = p.scene_slides(ci).last().map(|i| i + 1).unwrap_or(p.slides.len());
        p.slides.insert(at, slide);
        self.open_slide(at);
        self.scene_view = true;
    }

    /// Delete a scene and its slides (a course needs at least one scene).
    fn delete_scene(&mut self, si: usize) {
        let Some(p) = &self.project else { return };
        if p.scenes.len() <= 1 || si >= p.scenes.len() { self.status = "A project needs at least one scene".into(); return; }
        self.snapshot();
        let p = self.project.as_mut().unwrap();
        let sid = p.scenes[si].id.clone();
        let gone: Vec<String> = p.slides.iter().filter(|s| s.scene == sid).map(|s| s.id.clone()).collect();
        p.slides.retain(|s| s.scene != sid);
        p.scenes.remove(si);
        for sl in p.slides.iter_mut() {
            sl.triggers.retain(|t| !matches!(&t.action, Action::JumpToSlide(id) if gone.contains(id)));
        }
        let next = p.scene_slides(si.min(p.scenes.len() - 1)).first().copied().unwrap_or(0);
        self.slide = next;
        self.layer = 0;
        self.selected = None;
        self.reset_visibility();
    }

    /// Move a scene one place earlier or later in the project.
    fn move_scene(&mut self, si: usize, down: bool) {
        let n = self.project.as_ref().map(|p| p.scenes.len()).unwrap_or(0);
        let sj = if down { si + 1 } else { si.wrapping_sub(1) };
        if sj >= n { return; }
        self.snapshot();
        let p = self.project.as_mut().unwrap();
        let keep = p.slides[self.slide.min(p.slides.len() - 1)].id.clone();
        p.scenes.swap(si, sj);
        p.normalize_scenes();
        self.slide = p.slides.iter().position(|s| s.id == keep).unwrap_or(0);
    }

    /// Move a slide to the end of another scene.
    #[allow(dead_code)]
    fn move_slide_to_scene(&mut self, i: usize, si: usize) {
        let Some(p) = &self.project else { return };
        if si >= p.scenes.len() || p.slides[i].scene == p.scenes[si].id { return; }
        if p.scene_slides(p.scenes.iter().position(|c| c.id == p.slides[i].scene).unwrap_or(0)).len() <= 1 {
            self.status = "A scene needs at least one slide; delete the scene instead".into();
            return;
        }
        self.snapshot();
        let p = self.project.as_mut().unwrap();
        let id = p.slides[i].id.clone();
        p.slides[i].scene = p.scenes[si].id.clone();
        // Stable sort keeps the moved slide where it was among its new scene's slides, so put it last.
        let s = p.slides.remove(i);
        p.slides.push(s);
        p.normalize_scenes();
        self.slide = p.slides.iter().position(|s| s.id == id).unwrap_or(0);
        self.layer = 0;
        self.selected = None;
        self.reset_visibility();
    }

    /// A slide's thumbnail (drawn small, with a blue frame when selected), `w` wide.
    fn slide_thumbnail(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, i: usize, w: f32, selected: bool) -> egui::Response {
        let sl = self.project.as_ref().unwrap().slides[i].clone();
        let h = w * sl.height / sl.width;
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(w + 8.0, h + 8.0), egui::Sense::click());
        let thumb = egui::Rect::from_min_size(rect.min + egui::vec2(4.0, 4.0), egui::vec2(w, h));
        let painter = ui.painter_at(rect);
        if selected { painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(0, 90, 170)); }
        match &sl.kind {
            SlideKind::Normal => {
                self.paint_backdrop(ctx, &painter.with_clip_rect(thumb), thumb.min, w / sl.width, &sl);
                let layers = start_layers(&sl);
                self.paint_slide(ctx, &painter.with_clip_rect(thumb), thumb.min, w / sl.width, &sl, &layers, None);
            }
            SlideKind::Question(q) => {
                painter.rect_filled(thumb, 1.0, egui::Color32::from_rgb(244, 246, 249));
                painter.text(thumb.left_top() + egui::vec2(6.0, 6.0), egui::Align2::LEFT_TOP, "Question", egui::FontId::proportional(11.0), egui::Color32::from_rgb(70, 85, 100));
                painter.text(thumb.left_top() + egui::vec2(6.0, 22.0), egui::Align2::LEFT_TOP, q.prompt.chars().take(40).collect::<String>(), egui::FontId::proportional(9.0), egui::Color32::from_rgb(27, 42, 58));
            }
            SlideKind::Results => {
                painter.rect_filled(thumb, 1.0, egui::Color32::from_rgb(244, 246, 249));
                painter.text(thumb.center(), egui::Align2::CENTER_CENTER, "Results", egui::FontId::proportional(13.0), egui::Color32::from_rgb(3, 83, 158));
            }
        }
        resp
    }

    /// Flow layout of one scene: the first slide is at the top, the slides it jumps to
    /// (within the scene) hang below it, and a slide with no jumps leads to the next slide in order.
    /// Returns (slide index, row, column) per slide, and (parent, child) slide-index pairs for the connectors.
    fn scene_layout(&self, si: usize) -> (Vec<(usize, usize, f32)>, Vec<(usize, usize)>) {
        let p = self.project.as_ref().unwrap();
        let slides = p.scene_slides(si);
        let n = slides.len();
        let local = |id: &str| slides.iter().position(|&i| p.slides[i].id == id);
        let jumps: Vec<Vec<usize>> = slides.iter().map(|&i| {
            let mut v: Vec<usize> = Vec::new();
            for t in &p.slides[i].triggers {
                if let Action::JumpToSlide(to) = &t.action {
                    if let Some(l) = local(to) { if !v.contains(&l) { v.push(l); } }
                }
            }
            v
        }).collect();
        let mut placed = vec![false; n];
        let mut children: Vec<Vec<usize>> = vec![vec![]; n];
        let mut roots: Vec<usize> = Vec::new();
        for r in 0..n {
            if placed[r] { continue; }
            placed[r] = true;
            roots.push(r);
            let mut stack = vec![r];
            while let Some(v) = stack.pop() {
                let mut kids: Vec<usize> = jumps[v].iter().copied().filter(|&t| !placed[t]).collect();
                if jumps[v].is_empty() && v + 1 < n && !placed[v + 1] { kids.push(v + 1); }
                for &k in &kids { placed[k] = true; }
                children[v] = kids.clone();
                for &k in kids.iter().rev() { stack.push(k); }
            }
        }
        fn place(v: usize, depth: usize, children: &[Vec<usize>], next_col: &mut f32, out: &mut Vec<(usize, usize, f32)>) -> f32 {
            let kids = &children[v];
            let col = if kids.is_empty() {
                let c = *next_col;
                *next_col += 1.0;
                c
            } else {
                let cols: Vec<f32> = kids.iter().map(|&k| place(k, depth + 1, children, next_col, out)).collect();
                (cols[0] + cols[cols.len() - 1]) / 2.0
            };
            out.push((v, depth, col));
            col
        }
        let mut out: Vec<(usize, usize, f32)> = Vec::new();
        let mut next_col = 0.0;
        for &r in &roots { place(r, 0, &children, &mut next_col, &mut out); }
        let cells = out.into_iter().map(|(l, row, col)| (slides[l], row, col)).collect();
        let edges = (0..n).flat_map(|v| children[v].iter().map(move |&k| (v, k))).map(|(a, b)| (slides[a], slides[b])).collect();
        (cells, edges)
    }

    /// Display number of each slide in a scene: reading order of the flow (top to bottom, left to right),
    /// so the numbers follow what the layout shows. Keyed by slide index.
    fn scene_numbers(&self, si: usize) -> HashMap<usize, usize> {
        let mut cells = self.scene_layout(si).0;
        cells.sort_by(|a, b| (a.1, a.2).partial_cmp(&(b.1, b.2)).unwrap_or(std::cmp::Ordering::Equal));
        cells.iter().enumerate().map(|(n, c)| (c.0, n + 1)).collect()
    }

    /// Draw one scene as a flow of slide thumbnails `tw` wide. Returns the slide clicked and the slide double-clicked.
    fn scene_flow(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, si: usize, tw: f32, fit: Option<f32>) -> (Option<usize>, Option<usize>) {
        let (cells, edges) = self.scene_layout(si);
        // With `fit` (the width available), thumbnails scale to fill it and the flow is centred.
        let n_cols = cells.iter().map(|c| c.2).fold(0.0f32, f32::max) + 1.0;
        let tw = fit.map(|w| ((w + 14.0) / n_cols - 22.0).clamp(40.0, 130.0)).unwrap_or(tw);
        let (sw, sh) = {
            let p = self.project.as_ref().unwrap();
            let f = &p.slides[cells[0].0];
            (f.width, f.height)
        };
        let th = tw * sh / sw;
        let (cw, ch) = (tw + 8.0 + 14.0, th + 8.0 + 18.0 + 24.0);
        let cols = cells.iter().map(|c| c.2).fold(0.0f32, f32::max) + 1.0;
        let rows = cells.iter().map(|c| c.1).max().unwrap_or(0) + 1;
        // The last column has no trailing gap, so the flow is centred on what is actually drawn.
        let flow_w = cols * cw - 14.0;
        let (outer, _) = ui.allocate_exact_size(egui::vec2(flow_w.max(fit.unwrap_or(0.0)), rows as f32 * ch), egui::Sense::hover());
        let rect = egui::Rect::from_min_size(outer.min + egui::vec2((outer.width() - flow_w) / 2.0, 0.0), egui::vec2(flow_w, outer.height()));
        let cell_rect = |row: usize, col: f32| egui::Rect::from_min_size(rect.min + egui::vec2(col * cw, row as f32 * ch), egui::vec2(tw + 8.0, th + 8.0 + 18.0));
        let mut by_slide: HashMap<usize, egui::Rect> = HashMap::new();
        for &(i, row, col) in &cells { by_slide.insert(i, cell_rect(row, col)); }
        let stroke = egui::Stroke::new(1.5, ui.visuals().text_color().gamma_multiply(0.55));
        for &(a, b) in &edges {
            let (ra, rb) = (by_slide[&a], by_slide[&b]);
            let start = egui::pos2(ra.center().x, ra.bottom());
            let end = egui::pos2(rb.center().x, rb.top());
            let mid = (start.y + end.y) / 2.0;
            let pts = vec![start, egui::pos2(start.x, mid), egui::pos2(end.x, mid), end];
            ui.painter().add(egui::Shape::line(pts, stroke));
            ui.painter().add(egui::Shape::convex_polygon(vec![end, end + egui::vec2(-3.5, -7.0), end + egui::vec2(3.5, -7.0)], stroke.color, egui::Stroke::NONE));
        }
        let numbers = self.scene_numbers(si);
        let (mut select, mut open) = (None, None);
        // Jumps that are not a tree connector (back-links, sideways jumps, jumps out of the scene) show as a chip on the slide.
        let mut hover_line: Option<(egui::Pos2, egui::Rect)> = None;
        let mut chips: Vec<(usize, Vec<usize>)> = Vec::new();
        {
            let p = self.project.as_ref().unwrap();
            for &(i, _, _) in &cells {
                let mut targets: Vec<usize> = Vec::new();
                for t in &p.slides[i].triggers {
                    if let Action::JumpToSlide(to) = &t.action {
                        if let Some(j) = p.slides.iter().position(|x| x.id == *to) {
                            if j != i && !edges.contains(&(i, j)) && !targets.contains(&j) { targets.push(j); }
                        }
                    }
                }
                if !targets.is_empty() { chips.push((i, targets)); }
            }
        }
        for &(i, _, _) in &cells {
            let r = by_slide[&i];
            ui.scope_builder(egui::UiBuilder::new().max_rect(r), |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                let marked = i == self.slide || self.slide_multi.contains(&self.project.as_ref().unwrap().slides[i].id);
                let resp = self.slide_thumbnail(ui, ctx, i, tw, marked);
                if resp.secondary_clicked() && !marked {
                    // Right-clicking a slide outside the selection selects just that slide.
                    self.slide_multi.clear();
                    select = Some(i);
                    self.slide = i;
                }
                resp.context_menu(|ui| self.slide_context_menu(ui));
                if resp.double_clicked() { open = Some(i); }
                else if resp.clicked() {
                    let (cmd, shift) = ctx.input(|inp| (inp.modifiers.command, inp.modifiers.shift));
                    let id = self.project.as_ref().unwrap().slides[i].id.clone();
                    if cmd && i != self.slide {
                        // Add or remove this slide from the selection.
                        if let Some(k) = self.slide_multi.iter().position(|m| *m == id) { self.slide_multi.remove(k); } else { self.slide_multi.push(id); }
                    } else if shift {
                        // Everything between the current slide and this one.
                        let (a, b) = (self.slide.min(i), self.slide.max(i));
                        let ids: Vec<String> = self.project.as_ref().unwrap().slides[a..=b].iter().map(|s| s.id.clone()).collect();
                        let cur_id = self.project.as_ref().unwrap().slides[self.slide].id.clone();
                        self.slide_multi = ids.into_iter().filter(|x| *x != cur_id).collect();
                    } else {
                        self.slide_multi.clear();
                        select = Some(i);
                    }
                }
                let p = self.project.as_ref().unwrap();
                let n = numbers.get(&i).copied().unwrap_or(0);
                ui.add(egui::Label::new(egui::RichText::new(format!("{}.{} {}", si + 1, n, p.slides[i].name)).size(11.5)).truncate());
            });
            if let Some((_, targets)) = chips.iter().find(|c| c.0 == i) {
                let p = self.project.as_ref().unwrap();
                let icon = if targets[0] < i { ribbon::ARROW_UP } else { ribbon::ARROW_DOWN };
                let chip = egui::Rect::from_center_size(egui::pos2(r.left() + 4.0 + tw - 11.0, r.top() + 4.0 + th - 11.0), egui::vec2(16.0, 16.0));
                let resp = ui.interact(chip, egui::Id::new(("jump-chip", i)), egui::Sense::click());
                let fill = if resp.hovered() { egui::Color32::from_rgb(0, 140, 240) } else { egui::Color32::from_rgb(0, 100, 190) };
                ui.painter().circle_filled(chip.center(), 8.0, fill);
                ui.painter().text(chip.center(), egui::Align2::CENTER_CENTER, icon, egui::FontId::new(11.0, ribbon::icons()), egui::Color32::WHITE);
                let tip: Vec<String> = targets.iter().map(|&j| p.slides[j].name.clone()).collect();
                let resp = resp.on_hover_text(format!("Also jumps to:\n{}\n(click to go to the first)", tip.join("\n")));
                if resp.hovered() {
                    if let Some(tr) = targets.iter().find_map(|j| by_slide.get(j)) { hover_line = Some((chip.center(), *tr)); }
                }
                if resp.clicked() { select = Some(targets[0]); }
            }
        }
        if let Some((from, to)) = hover_line {
            let c = egui::Color32::from_rgb(0, 140, 240);
            ui.painter().line_segment([from, to.center()], egui::Stroke::new(2.0, c));
            ui.painter().rect_stroke(to, 3.0, egui::Stroke::new(2.0, c), egui::StrokeKind::Outside);
        }
        (select, open)
    }

    /// Scene View: each scene is a card showing its slide flow, cards side by side.
    fn scene_view_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let n_scenes = self.project.as_ref().unwrap().scenes.len();
        let mut open: Option<usize> = None;
        let mut select: Option<usize> = None;
        let mut delete: Option<usize> = None;
        let mut mv: Option<(usize, bool)> = None;
        let mut rename: Option<usize> = None;
        let card_bg = if ui.visuals().dark_mode { egui::Color32::from_gray(48) } else { egui::Color32::WHITE };
        let avail = ui.available_width();
        // Pack scene cards into rows that fit the window, then centre each row.
        let widths: Vec<f32> = (0..n_scenes).map(|si| (self.scene_layout(si).0.iter().map(|c| c.2).fold(0.0f32, f32::max) + 1.0) * 106.0).map(|w| (w - 14.0).max(190.0) + 18.0).collect();
        let mut card_rows: Vec<Vec<usize>> = vec![];
        let mut row_w = 0.0f32;
        for si in 0..n_scenes {
            if card_rows.is_empty() || row_w + 16.0 + widths[si] > avail {
                card_rows.push(vec![si]);
                row_w = widths[si];
            } else {
                card_rows.last_mut().unwrap().push(si);
                row_w += 16.0 + widths[si];
            }
        }
        let view_bg = ui.interact(ui.available_rect_before_wrap(), egui::Id::new("scene-view-bg"), egui::Sense::click());
        view_bg.context_menu(|ui| self.new_slide_items(ui, None));
        egui::ScrollArea::both().id_salt("scene-view").auto_shrink([false, false]).show(ui, |ui| {
            ui.add_space(8.0);
            ui.spacing_mut().item_spacing.y = 16.0;
            for row in &card_rows {
            let total: f32 = row.iter().map(|&si| widths[si]).sum::<f32>() + 16.0 * (row.len() as f32 - 1.0);
            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                ui.spacing_mut().item_spacing.x = 16.0;
                ui.add_space(((avail - total) / 2.0).max(0.0));
                for &si in row {
                    egui::Frame::NONE.fill(card_bg).stroke(egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color)).inner_margin(8).show(ui, |ui| {
                        let cols = self.scene_layout(si).0.iter().map(|c| c.2).fold(0.0f32, f32::max) + 1.0;
                        ui.set_width((cols * (84.0 + 22.0) - 14.0).max(190.0));
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                if self.scene_rename == Some(si) {
                                    let mut name = self.project.as_ref().unwrap().scenes[si].name.clone();
                                    let r = ui.add(egui::TextEdit::singleline(&mut name).desired_width(150.0));
                                    if !r.has_focus() && !r.lost_focus() { r.request_focus(); }
                                    if r.changed() && !name.trim().is_empty() {
                                        self.project.as_mut().unwrap().scenes[si].name = name;
                                        self.dirty = true;
                                    }
                                    if r.lost_focus() { self.scene_rename = None; }
                                } else {
                                    let name = self.project.as_ref().unwrap().scenes[si].name.clone();
                                    let l = ui.add(egui::Label::new(egui::RichText::new(format!("{} {}", si + 1, name)).strong()).sense(egui::Sense::click()));
                                    if l.double_clicked() { rename = Some(si); }
                                    l.context_menu(|ui| {
                                        ui.menu_button("New slide here", |ui| self.new_slide_items(ui, Some(si)));
                                        ui.separator();
                                        if ui.button("Rename").clicked() { rename = Some(si); ui.close(); }
                                        if ui.add_enabled(si > 0, egui::Button::new("Move earlier")).clicked() { mv = Some((si, false)); ui.close(); }
                                        if ui.add_enabled(si + 1 < n_scenes, egui::Button::new("Move later")).clicked() { mv = Some((si, true)); ui.close(); }
                                        ui.separator();
                                        if ui.add_enabled(n_scenes > 1, egui::Button::new("Delete scene")).clicked() { delete = Some(si); ui.close(); }
                                    });
                                }
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.menu_button(egui::RichText::new(ribbon::CARET.to_string()).family(ribbon::icons()), |ui| {
                                        if ui.button("Rename").clicked() { rename = Some(si); ui.close(); }
                                        if ui.add_enabled(si > 0, egui::Button::new("Move earlier")).clicked() { mv = Some((si, false)); ui.close(); }
                                        if ui.add_enabled(si + 1 < n_scenes, egui::Button::new("Move later")).clicked() { mv = Some((si, true)); ui.close(); }
                                        ui.separator();
                                        if ui.add_enabled(n_scenes > 1, egui::Button::new("Delete scene")).on_hover_text("Deletes the scene and its slides").clicked() { delete = Some(si); ui.close(); }
                                    });
                                });
                            });
                            ui.add_space(6.0);
                            let (sel, opn) = self.scene_flow(ui, ctx, si, 84.0, None);
                            if sel.is_some() { select = sel; }
                            if opn.is_some() { open = opn; }
                        });
                    });
                }
            });
            }
        });
        if rename.is_some() { self.snapshot(); self.scene_rename = rename; }
        if let Some(i) = select { self.slide = i; self.layer = 0; self.selected = None; self.reset_visibility(); }
        if let Some(i) = open { self.open_slide(i); }
        if let Some((si, down)) = mv { self.move_scene(si, down); }
        if let Some(si) = delete { self.delete_scene(si); }
    }

    #[allow(dead_code)]
    fn delete_slide(&mut self, i: usize) {
        self.delete_slides(vec![i]);
    }

    /// The current slide plus every slide selected with ⌘/⇧-click, as indices in lesson order.
    fn slide_selection(&self) -> Vec<usize> {
        let Some(p) = &self.project else { return vec![] };
        let cur = self.slide.min(p.slides.len().saturating_sub(1));
        (0..p.slides.len()).filter(|&i| i == cur || self.slide_multi.contains(&p.slides[i].id)).collect()
    }

    /// Delete several slides as one undo step (at least one slide always remains).
    fn delete_slides(&mut self, mut idxs: Vec<usize>) {
        let Some(p) = &self.project else { return };
        idxs.sort_unstable();
        idxs.dedup();
        idxs.retain(|&i| i < p.slides.len());
        if idxs.len() >= p.slides.len() { self.status = "A project needs at least one slide".into(); return; }
        if idxs.is_empty() { return; }
        self.snapshot();
        let p = self.project.as_mut().unwrap();
        for &i in idxs.iter().rev() {
            let gone = p.slides.remove(i);
            if !p.slides.iter().any(|s| s.scene == gone.scene) { p.scenes.retain(|c| c.id != gone.scene); }
            for sl in p.slides.iter_mut() {
                sl.triggers.retain(|t| !matches!(&t.action, Action::JumpToSlide(id) if *id == gone.id));
            }
        }
        self.slide_multi.clear();
        self.slide = self.slide.min(p.slides.len() - 1);
        self.layer = 0;
        self.selected = None;
        self.reset_visibility();
    }

    #[allow(dead_code)]
    fn move_slide(&mut self, i: usize, down: bool) {
        let n = self.project.as_ref().map(|p| p.slides.len()).unwrap_or(0);
        let j = if down { i + 1 } else { i.wrapping_sub(1) };
        if j >= n || self.project.as_ref().unwrap().slides[i].scene != self.project.as_ref().unwrap().slides[j].scene { return; }
        self.snapshot();
        self.project.as_mut().unwrap().slides.swap(i, j);
        self.slide = j;
    }

    /// Copy a slide, giving the copy fresh ids; its triggers are re-pointed at the copied layers/objects.
    fn copy_of_slide(&mut self, i: usize) -> Option<Slide> {
        let mut copy = self.project.as_ref().map(|p| p.slides[i].clone())?;
        let mut map: HashMap<String, String> = HashMap::new();
        copy.id = self.new_id("slide");
        copy.name = format!("{} copy", copy.name);
        for l in copy.layers.iter_mut() {
            let nid = self.new_id("layer");
            map.insert(std::mem::replace(&mut l.id, nid), l.id.clone());
            for o in l.objects.iter_mut() {
                let nid = self.new_id("obj");
                map.insert(std::mem::replace(&mut o.id, nid), o.id.clone());
            }
        }
        let remap = |s: &String| map.get(s).cloned().unwrap_or_else(|| s.clone());
        for t in copy.triggers.iter_mut() {
            t.id = format!("{}-c{}", t.id, self.id_counter);
            t.owner = match &t.owner {
                Owner::Layer(l) => Owner::Layer(remap(l)),
                Owner::Object(o) => Owner::Object(remap(o)),
                Owner::Slide => Owner::Slide,
            };
            t.event = t.event.map_objects(&|o| remap(&o.to_string()));
            t.action = match &t.action {
                Action::ShowLayer(l) => Action::ShowLayer(remap(l)),
                Action::HideLayer(l) => Action::HideLayer(remap(l)),
                a => a.map_objects(&|o| remap(&o.to_string())),
            };
        }
        Some(copy)
    }

    fn duplicate_slide(&mut self, i: usize) {
        let Some(copy) = self.copy_of_slide(i) else { return };
        self.snapshot();
        self.project.as_mut().unwrap().slides.insert(i + 1, copy);
        self.slide = i + 1;
        self.slide_multi.clear();
        self.layer = 0;
        self.selected = None;
        self.reset_visibility();
    }

    /// Copy every selected slide; each scene's copies go right after its last selected slide, in order.
    fn duplicate_slides(&mut self) {
        let sel = self.slide_selection();
        if sel.len() < 2 { if let Some(&i) = sel.first() { self.duplicate_slide(i); } return; }
        let mut copies: Vec<(usize, Slide)> = vec![];
        for &i in &sel { if let Some(c) = self.copy_of_slide(i) { copies.push((i, c)); } }
        self.snapshot();
        let p = self.project.as_mut().unwrap();
        // Insert from the back so earlier indices stay valid; copies of one scene stay together.
        let mut new_ids: Vec<String> = vec![];
        let scenes: Vec<String> = p.scenes.iter().map(|s| s.id.clone()).collect();
        for sc in scenes.iter().rev() {
            let group: Vec<Slide> = copies.iter().filter(|(i, _)| p.slides[*i].scene == *sc).map(|(_, c)| c.clone()).collect();
            let Some(last) = copies.iter().filter(|(i, _)| p.slides[*i].scene == *sc).map(|(i, _)| *i).max() else { continue };
            for (k, c) in group.into_iter().enumerate() { new_ids.push(c.id.clone()); p.slides.insert(last + 1 + k, c); }
        }
        let first = p.slides.iter().position(|s| new_ids.contains(&s.id)).unwrap_or(0);
        self.slide = first;
        self.slide_multi = new_ids.into_iter().filter(|id| *id != self.project.as_ref().unwrap().slides[first].id).collect();
        self.layer = 0;
        self.selected = None;
        self.reset_visibility();
    }

    /// Whether the selected slides can move one place up or down (they stay inside their scene).
    fn can_move_slides(&self, down: bool) -> bool {
        let Some(p) = &self.project else { return false };
        let sel = self.slide_selection();
        sel.iter().any(|&i| {
            let j = if down { i + 1 } else { i.wrapping_sub(1) };
            j < p.slides.len() && !sel.contains(&j) && p.slides[j].scene == p.slides[i].scene
        })
    }

    /// Move the selected slides one place, as a block.
    fn move_slides(&mut self, down: bool) {
        if !self.can_move_slides(down) { return; }
        self.snapshot();
        let keep = self.project.as_ref().unwrap().slides[self.slide].id.clone();
        let mut sel = self.slide_selection();
        if down { sel.reverse(); }
        let p = self.project.as_mut().unwrap();
        let mut now: Vec<usize> = sel.clone();
        for k in 0..sel.len() {
            let i = sel[k];
            let j = if down { i + 1 } else { i.wrapping_sub(1) };
            if j < p.slides.len() && !now.contains(&j) && p.slides[j].scene == p.slides[i].scene {
                p.slides.swap(i, j);
                now[k] = j;
            }
        }
        self.slide = p.slides.iter().position(|s| s.id == keep).unwrap_or(0);
    }

    /// Move every selected slide to the end of scene `si`.
    fn move_selected_slides_to_scene(&mut self, si: usize) {
        let sel = self.slide_selection();
        let Some(p) = &self.project else { return };
        if si >= p.scenes.len() { return; }
        let dest = p.scenes[si].id.clone();
        let moving: Vec<usize> = sel.iter().copied().filter(|&i| p.slides[i].scene != dest).collect();
        if moving.is_empty() { return; }
        // Every scene the slides leave must keep at least one slide.
        for sc in p.scenes.iter() {
            let total = p.slides.iter().filter(|s| s.scene == sc.id).count();
            let leaving = moving.iter().filter(|&&i| p.slides[i].scene == sc.id).count();
            if leaving > 0 && leaving >= total { self.status = "A scene needs at least one slide; delete the scene instead".into(); return; }
        }
        self.snapshot();
        let p = self.project.as_mut().unwrap();
        let keep = p.slides[self.slide].id.clone();
        let ids: Vec<String> = moving.iter().map(|&i| p.slides[i].id.clone()).collect();
        for id in &ids {
            let i = p.slides.iter().position(|s| s.id == *id).unwrap();
            let mut s = p.slides.remove(i);
            s.scene = dest.clone();
            p.slides.push(s);
        }
        p.normalize_scenes();
        self.slide = p.slides.iter().position(|s| s.id == keep).unwrap_or(0);
        self.layer = 0;
        self.selected = None;
        self.reset_visibility();
    }

    /// Right-click menu on the slide canvas.
    fn object_context_menu(&mut self, ui: &mut egui::Ui) {
        let has_sel = self.selected.is_some();
        let n = self.selection().len();
        let normal = self.project.is_some();
        ui.add_enabled_ui(has_sel, |ui| {
            if ui.button("Cut").clicked() { self.copy_selected(); self.delete_selected(); ui.close(); }
            if ui.button("Copy").clicked() { self.copy_selected(); ui.close(); }
        });
        ui.add_enabled_ui(normal && self.clipboard.is_some(), |ui| { if ui.button("Paste").clicked() { self.paste(); ui.close(); } });
        let in_table = self.selected_object().is_some_and(|o| o.table.is_some());
        if in_table {
            ui.separator();
            ui.menu_button("Table", |ui| {
                for (label, op) in [("Insert row above", TableOp::RowAbove), ("Insert row below", TableOp::RowBelow), ("Insert column left", TableOp::ColLeft), ("Insert column right", TableOp::ColRight), ("Delete row", TableOp::DeleteRow), ("Delete column", TableOp::DeleteCol)] {
                    if ui.button(label).clicked() { self.table_edit(op); ui.close(); }
                }
                ui.separator();
                let can_merge = self.cell_block().is_some_and(|(_, (r0, c0, r1, c1))| (r0, c0) != (r1, c1));
                let can_split = self.selected_object().is_some_and(|o| o.table_span.is_some());
                if ui.add_enabled(can_merge, egui::Button::new("Merge cells")).clicked() { self.merge_cells(); ui.close(); }
                if ui.add_enabled(can_split, egui::Button::new("Split cell")).clicked() { self.split_cell(); ui.close(); }
            });
            ui.menu_button("Table style", |ui| {
                for st in brainbend_doc::tables::TableStyle::ALL {
                    if ui.button(st.label()).clicked() { self.restyle_table(st, self.table_opts); ui.close(); }
                }
            });
            ui.separator();
        }
        ui.add_enabled_ui(has_sel, |ui| {
            if ui.button("Duplicate").clicked() { self.duplicate_selected(); ui.close(); }
            if ui.button("Delete").clicked() { self.delete_selected(); ui.close(); }
            ui.separator();
            ui.menu_button("Arrange", |ui| {
                if ui.button("Bring to front").clicked() { self.reorder_selected(true); ui.close(); }
                if ui.button("Bring forward").clicked() { self.reorder_step(true); ui.close(); }
                if ui.button("Send backward").clicked() { self.reorder_step(false); ui.close(); }
                if ui.button("Send to back").clicked() { self.reorder_selected(false); ui.close(); }
            });
            ui.menu_button(if n > 1 { "Align to each other" } else { "Align to the slide" }, |ui| {
                for (label, axis, mode) in [("Left", 0u8, 0u8), ("Center", 0, 1), ("Right", 0, 2), ("Top", 1, 0), ("Middle", 1, 1), ("Bottom", 1, 2)] {
                    if ui.button(label).clicked() { self.align_selected_or_slide(axis, mode); ui.close(); }
                }
            });
            ui.separator();
            ui.add_enabled_ui(n > 1, |ui| { if ui.button("Group").clicked() { self.group_selected(); ui.close(); } });
            if ui.button("Ungroup").clicked() { self.ungroup_selected(); ui.close(); }
            ui.separator();
            if self.selected_object().is_some_and(states_panel::supports_states) && ui.button("Edit states…").clicked() { self.bottom_tab = 2; ui.close(); }
            if ui.button("Triggers…").on_hover_text("Show this object's triggers").clicked() { self.right_tab = 1; ui.close(); }
        });
        if !has_sel {
            // On empty canvas: put something on the slide.
            ui.separator();
            ui.menu_button("Insert", |ui| {
                if ui.button("Text box").clicked() { self.insert_text(); ui.close(); }
                if ui.button("Rectangle").clicked() { self.insert_rect(); ui.close(); }
                if ui.button("Ellipse").clicked() { self.insert_ellipse(); ui.close(); }
                if ui.button("Button").clicked() { self.insert_button(); ui.close(); }
                if ui.button("Picture…").clicked() { self.insert_image(); ui.close(); }
            });
            if ui.button("New layer").clicked() { self.add_layer(); ui.close(); }
        }
    }

    /// Move the selected object one step in front of, or behind, its neighbour in its layer.
    fn reorder_step(&mut self, forward: bool) {
        let Some((li, oi)) = self.selected else { return };
        let n = self.project.as_ref().map(|p| p.slides[self.slide].layers[li].objects.len()).unwrap_or(0);
        let to = if forward { oi + 1 } else { oi.wrapping_sub(1) };
        if to >= n { return; }
        self.multi.clear();
        self.snapshot();
        self.project.as_mut().unwrap().slides[self.slide].layers[li].objects.swap(oi, to);
        self.selected = Some((li, to));
    }

    /// The "new slide" choices shared by every menu that adds one: a slide, a question slide, a results slide and a
    /// scene. With `scene`, the slide goes at the end of that scene; otherwise after the current slide.
    fn new_slide_items(&mut self, ui: &mut egui::Ui, scene: Option<usize>) {
        const QUESTIONS: [(&str, fn() -> Question); 16] = [
            ("Multiple choice", Question::new_single), ("True / false", Question::new_true_false), ("Fill in the blank", Question::new_text_entry),
            ("Matching", Question::new_matching), ("Sequencing", Question::new_sequence), ("Fill in the blanks", Question::new_fill_blanks),
            ("Drag the words", Question::new_drag_words), ("Mark the words", Question::new_mark_words), ("Number", Question::new_numeric),
            ("Sort into groups", Question::new_categorize), ("Find the hotspot", Question::new_hotspot), ("Label the picture", Question::new_label_picture),
            ("Choose from lists", Question::new_dropdowns), ("Dictation", Question::new_dictation), ("Survey scale (not graded)", Question::new_likert),
            ("Free text (not graded)", Question::new_reflection),
        ];
        let mut add: Option<SlideKind> = None;
        if ui.button("Slide").clicked() { add = Some(SlideKind::Normal); ui.close(); }
        ui.menu_button("Question", |ui| {
            for (label, make) in QUESTIONS {
                if ui.button(label).clicked() { add = Some(SlideKind::Question(make())); ui.close(); }
            }
        });
        if ui.button("Results").clicked() { add = Some(SlideKind::Results); ui.close(); }
        if let Some(kind) = add {
            if let Some(&last) = scene.and_then(|si| self.project.as_ref().map(|p| p.scene_slides(si))).as_ref().and_then(|v| v.last()) { self.slide = last; }
            self.add_slide(kind);
            self.scene_view = false;
        }
        ui.separator();
        if ui.button("New scene").clicked() { self.add_scene(); self.scene_view = false; ui.close(); }
    }

    /// Right-click menu of a slide in the slide list or Scene View.
    fn slide_context_menu(&mut self, ui: &mut egui::Ui) {
        let n = self.slide_selection().len();
        let many = n > 1;
        ui.menu_button("New slide after", |ui| self.new_slide_items(ui, None));
        if ui.button(if many { format!("Duplicate {n} slides") } else { "Duplicate".to_string() }).clicked() { self.duplicate_slides(); ui.close(); }
        ui.add_enabled_ui(self.can_move_slides(false), |ui| { if ui.button("Move up").clicked() { self.move_slides(false); ui.close(); } });
        ui.add_enabled_ui(self.can_move_slides(true), |ui| { if ui.button("Move down").clicked() { self.move_slides(true); ui.close(); } });
        ui.menu_button("Move to scene", |ui| {
            let names: Vec<String> = self.project.as_ref().unwrap().scenes.iter().map(|c| c.name.clone()).collect();
            for (si, nm) in names.iter().enumerate() {
                if ui.button(nm).clicked() { self.move_selected_slides_to_scene(si); ui.close(); }
            }
        });
        ui.separator();
        if ui.button("Preview from here").clicked() { self.preview(PreviewScope::FromHere); ui.close(); }
        if many && ui.button("Preview selected slides").clicked() { self.preview(PreviewScope::Selected); ui.close(); }
        ui.separator();
        if ui.button(if many { format!("Delete {n} slides") } else { "Delete".to_string() }).clicked() { self.pending_delete = Some(self.slide_selection()); ui.ctx().request_repaint(); ui.close(); }
    }

    fn add_layer(&mut self) {
        if self.project.is_none() { return; }
        self.snapshot();
        let id = self.new_id("layer");
        let slide = &mut self.project.as_mut().unwrap().slides[self.slide];
        let name = format!("Layer {}", slide.layers.len());
        slide.layers.push(Layer { id, name, objects: vec![], duration_ms: None, modal: true, hide_others: false, pause_base: false, group_names: Default::default(), focus_order: vec![] });
        self.layer = slide.layers.len() - 1;
        self.selected = None;
        self.reset_visibility_keep();
    }

    /// Keep current visibility flags, growing the list for new layers (shown by default).
    fn reset_visibility_keep(&mut self) {
        let n = self.project.as_ref().map(|p| p.slides[self.slide].layers.len()).unwrap_or(0);
        self.visible.resize(n, true);
    }

    fn delete_layer(&mut self, li: usize) {
        if li == 0 { return; }
        self.snapshot();
        let slide = &mut self.project.as_mut().unwrap().slides[self.slide];
        let removed = slide.layers.remove(li);
        // Drop triggers that live on, or point at, the removed layer or its objects.
        let gone: std::collections::HashSet<String> = removed.objects.iter().map(|o| o.id.clone()).collect();
        slide.triggers.retain(|t| {
            let owner_gone = match &t.owner {
                Owner::Layer(l) => *l == removed.id,
                Owner::Object(o) => gone.contains(o),
                Owner::Slide => false,
            };
            let target_gone = matches!(&t.action, Action::ShowLayer(l) | Action::HideLayer(l) if *l == removed.id);
            !owner_gone && !target_gone
        });
        self.layer = self.layer.min(slide.layers.len() - 1);
        self.selected = None;
        self.visible.remove(li.min(self.visible.len().saturating_sub(1)));
        self.reset_visibility_keep();
    }

    /// Add a media asset (video/audio/captions) from a file; returns its asset key.
    fn load_media(&mut self, filters: &[&str], label: &str) -> Option<(String, String)> {
        let path = rfd::FileDialog::new().add_filter(label, filters).pick_file()?;
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => { self.status = format!("Could not read {}: {e}", path.display()); return None; }
        };
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        let mime = match ext.as_str() {
            "mp4" | "m4v" => "video/mp4",
            "webm" => "video/webm",
            "mp3" => "audio/mpeg",
            "ogg" | "oga" => "audio/ogg",
            "m4a" => "audio/mp4",
            "wav" => "audio/wav",
            "vtt" => "text/vtt",
            _ => { self.status = format!("Unsupported file type .{ext}"); return None; }
        };
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("media").to_string();
        let key = self.new_id("asset");
        self.project.as_mut()?.assets.insert(key.clone(), Asset { name: name.clone(), mime: mime.into(), bytes, file: None });
        Some((key, name))
    }

    fn insert_video(&mut self) {
        let Some((key, name)) = self.load_media(&["mp4", "m4v", "webm"], "Video") else { return };
        let mut o = blank_object(self.new_id("obj"), &name, Kind::Video, 0.0, 0.0, 1280.0, 720.0);
        // A frame of the video to show before it plays (needs ffmpeg; without it there is none).
        if let Some(still) = self.project.as_ref().and_then(|p| p.assets.get(&key)).and_then(brainbend_import_legacy::still_asset) {
            let still_key = format!("{key}-still");
            if let Some(p) = self.project.as_mut() { p.assets.insert(still_key.clone(), still); }
            o.poster = Some(still_key);
        }
        o.asset = Some(key);
        self.insert(o);
    }

    /// An icon from the gallery as a freeform vector shape in the theme's text colour, named and described for screen
    /// readers by the icon's name (the author can change both).
    fn insert_icon(&mut self, icon: &icons::Icon) {
        let Some((cmds, aspect)) = icons::outline(icon) else { return };
        let (w, h) = if aspect >= 1.0 { (200.0, 200.0 / aspect) } else { (200.0 * aspect, 200.0) };
        let mut o = blank_object(self.new_id("obj"), &icon.title(), Kind::Path(cmds), 0.0, 0.0, w, h);
        o.fill = Some(self.project.as_ref().and_then(|p| p.theme.colors.first().copied()).unwrap_or(Rgba(34, 34, 34, 255)));
        o.alt = icon.title();
        self.insert(o);
    }

    /// A check box of one of the four looks (`Check::LOOKS`): a labelled text object with the box beside it.
    fn insert_check_look(&mut self, look: usize) {
        let mut o = blank_object(self.new_id("obj"), "Check Box", Kind::Text, 0.0, 0.0, 800.0, 96.0);
        o.text = "Check box".into();
        o.style = plain_style(40.0, Rgba(0, 0, 0, 255), false, Align::Left);
        let ck = Check::look(look, None);
        o.margins = [ck.size * 1.6, 0.0, 0.0, 0.0];
        o.check = Some(ck);
        o.initial_state = "Normal".into();
        self.insert(o);
    }

    /// A radio button of one of the four looks. A new one joins the group of the selected radio button, else starts a
    /// group of its own, so a few inserted in a row work together.
    fn insert_radio_look(&mut self, look: usize) {
        let group = self.selected_object().and_then(|o| o.check.as_ref().and_then(|c| c.radio.clone()))
            .unwrap_or_else(|| {
                let used: Vec<String> = self.project.as_ref().map(|p| p.slides[self.slide].layers.iter().flat_map(|l| l.objects.iter()).filter_map(|o| o.check.as_ref().and_then(|c| c.radio.clone())).collect()).unwrap_or_default();
                (1..).map(|n| format!("Choices{n}")).find(|g| !used.contains(g)).unwrap_or_default()
            });
        let mut o = blank_object(self.new_id("obj"), "Radio Button", Kind::Text, 0.0, 0.0, 800.0, 96.0);
        o.text = "Option".into();
        o.style = plain_style(40.0, Rgba(0, 0, 0, 255), false, Align::Left);
        let ck = Check::look(look, Some(group));
        o.margins = [ck.size * 1.6, 0.0, 0.0, 0.0];
        o.check = Some(ck);
        o.initial_state = "Normal".into();
        self.insert(o);
    }

    /// A text or number entry field with a variable of its own to keep what is typed.
    fn insert_input(&mut self, numeric: bool) {
        let field = if numeric { InputField::number("") } else { InputField::text("") };
        let mut o = blank_object(self.new_id("obj"), if numeric { "Number entry" } else { "Text entry" }, Kind::Input(field), 0.0, 0.0, if numeric { 320.0 } else { 640.0 }, 80.0);
        o.style = plain_style(36.0, Rgba(30, 30, 30, 255), false, Align::Left);
        o.fill = Some(Rgba(255, 255, 255, 255));
        o.stroke = Some(Stroke { color: Rgba(120, 120, 120, 255), width: 2.0 });
        o.radius = 6.0;
        o.alt = if numeric { "Number entry" } else { "Text entry" }.into();
        // The insert records the undo step, so the variable added after it is part of the same step.
        self.insert(o);
        let Some((li, oi)) = self.selected else { return };
        let Some(var) = (if numeric { self.new_variable("Number", Value::Num(0.0)) } else { self.new_variable("Text", Value::Text(String::new())) }) else { return };
        if let Some(Kind::Input(f)) = self.project.as_mut().and_then(|p| p.slides[self.slide].layers.get_mut(li)).and_then(|l| l.objects.get_mut(oi)).map(|o| &mut o.kind) { f.variable = var; }
    }

    /// A new number variable for a dial or a slider, named `{base}1`, `{base}2`... (the next free one).
    fn new_number_variable(&mut self, base: &str, initial: f32) -> Option<String> { self.new_variable(base, Value::Num(initial as f64)) }

    /// A new project variable named `{base}1`, `{base}2`... (the next free one) with this starting value.
    fn new_variable(&mut self, base: &str, initial: Value) -> Option<String> {
        let p = self.project.as_ref()?;
        let mut n = 1;
        while p.variables.iter().any(|v| v.name == format!("{base}{n}")) { n += 1; }
        let (id, name) = (self.new_id("var"), format!("{base}{n}"));
        self.project.as_mut()?.variables.push(Variable { id: id.clone(), name, initial, built_in: false });
        Some(id)
    }

    /// A dial: a knob (or a hub) and a pointer that turns with the learner's drag and sets a number variable of its own.
    /// The two are grouped so they move together; the pointer is the dial.
    fn insert_dial(&mut self, style: DialStyle) {
        let Some(p) = &self.project else { return };
        let accent = p.theme.colors.get(4).copied().unwrap_or(Rgba(3, 83, 158, 255));
        let size = 300.0;
        let grey = Some(Gradient { angle: 160.0, stops: vec![(0.0, Rgba(246, 246, 246, 255)), (1.0, Rgba(186, 186, 186, 255))] });
        let mut face = blank_object(self.new_id("obj"), if style == DialStyle::Needle { "Dial hub" } else { "Dial knob" }, Kind::Ellipse, 0.0, 0.0, size, size);
        face.gradient = grey;
        face.fill = Some(Rgba(220, 220, 220, 255));
        face.stroke = Some(Stroke { color: Rgba(160, 160, 160, 255), width: 3.0 });
        face.decorative = true;
        let mut pointer = blank_object(self.new_id("obj"), "Dial", Kind::Path(dial_pointer(style)), 0.0, 0.0, size, size);
        pointer.fill = Some(accent);
        pointer.alt = "Dial".into();
        // Insert snapshots; the variable is added after, so it is part of the same undo step.
        let first = if style == DialStyle::Needle { pointer.clone() } else { face.clone() };
        self.insert(first);
        let Some((li, at)) = self.selected else { return };
        let gid = self.new_id("group");
        let var = self.new_number_variable("Dial", 5.0).unwrap_or_default();
        pointer.dial = Some(Dial { variable: var, min: 0.0, max: 10.0, step: 1.0, sweep: 270.0, pivot: (0.0, 0.0) });
        if style == DialStyle::Needle { face.w = 80.0; face.h = 80.0; }
        let (x, y, w, h) = { let o = &self.project.as_ref().unwrap().slides[self.slide].layers[li].objects[at]; (o.x, o.y, o.w, o.h) };
        face.x = x + (w - face.w) / 2.0; face.y = y + (h - face.h) / 2.0;
        pointer.x = x; pointer.y = y;
        let layer = &mut self.project.as_mut().unwrap().slides[self.slide].layers[li];
        let rest: Vec<Object> = if style == DialStyle::Needle { vec![face] } else { vec![pointer.clone()] };
        if style == DialStyle::Needle { layer.objects[at].dial = pointer.dial.clone(); }
        layer.objects.extend(rest);
        let n = layer.objects.len();
        for o in layer.objects[at..].iter_mut() { o.group = Some(gid.clone()); }
        // Select the pointer: it is the part the properties belong to.
        self.selected = Some((li, if style == DialStyle::Needle { at } else { n - 1 }));
    }

    /// Make the selected object a dial, with a new number variable of its own.
    fn convert_to_dial(&mut self) {
        let Some((li, oi)) = self.selected else { return };
        if self.project.as_ref().and_then(|p| p.slides[self.slide].layers.get(li)?.objects.get(oi)).is_none_or(|o| o.dial.is_some()) { return; }
        self.snapshot();
        let var = self.new_number_variable("Dial", 5.0).unwrap_or_default();
        if let Some(o) = self.project.as_mut().and_then(|p| p.slides[self.slide].layers.get_mut(li)).and_then(|l| l.objects.get_mut(oi)) {
            o.dial = Some(Dial { variable: var, min: 0.0, max: 10.0, step: 1.0, sweep: 270.0, pivot: (0.0, 0.0) });
        }
        self.dirty = true;
    }

    /// Put `s` (a symbol) into the selected object's text: replacing the chosen text, else at the cursor where the last
    /// in-place edit left it, else at the end. The characters around it keep their formatting.
    fn insert_symbol(&mut self, s: &str) {
        if self.selected.is_none() && self.project.is_some() {
            // Nothing chosen: the symbol starts a new text box (and the ribbon stays on Insert).
            let tab = self.ribbon_tab;
            self.insert_text();
            self.ribbon_tab = tab;
            if let Some(o) = self.selected.and_then(|(li, oi)| self.project.as_mut()?.slides[self.slide].layers.get_mut(li)?.objects.get_mut(oi)) { o.edit_text(""); }
        }
        let Some((li, oi)) = self.selected else { return };
        let (part, caret) = (self.text_sel, self.caret);
        self.snapshot();
        let slide = self.slide;
        let mut at = 0;
        if let Some(o) = self.project.as_mut().and_then(|p| p.slides[slide].layers.get_mut(li)).and_then(|l| l.objects.get_mut(oi)) {
            if matches!(o.kind, Kind::Image | Kind::Video | Kind::Audio | Kind::Hotspot | Kind::Web(_) | Kind::Pano(_) | Kind::Input(_) | Kind::Slider(_) | Kind::Unsupported(_)) { return; }
            let chars: Vec<char> = o.text.chars().collect();
            let n = chars.len();
            let (a, b) = match (part, caret) {
                (Some((a, b)), _) => (a.min(n), b.min(n)),
                (None, Some((cl, co, i))) if (cl, co) == (li, oi) => (i.min(n), i.min(n)),
                _ => (n, n),
            };
            let text: String = chars[..a].iter().collect::<String>() + s + &chars[b..].iter().collect::<String>();
            let old = o.text.clone();
            o.edit_text(&text);
            for st in o.states.iter_mut() { if st.text == old { st.text = o.text.clone(); st.rich = o.rich.clone(); } }
            at = a + s.chars().count();
        }
        self.text_sel = None;
        self.caret = Some((li, oi, at));
        self.dirty = true;
    }

    /// A slide number: a text box at the bottom right whose text follows the slide ("3", or "3 of 12").
    fn insert_slide_number(&mut self, of_total: bool) {
        let (sw, sh) = self.project.as_ref().and_then(|p| p.slides.get(self.slide)).map(|s| (s.width, s.height)).unwrap_or((1920.0, 1080.0));
        let mut o = blank_object(self.new_id("obj"), "Slide number", Kind::Text, sw - 360.0, sh - 110.0, 320.0, 70.0);
        o.text = if of_total { "%Slide.Number% of %Slide.Total%" } else { "%Slide.Number%" }.into();
        o.style = plain_style(32.0, self.project.as_ref().and_then(|p| p.theme.colors.first().copied()).unwrap_or(Rgba(34, 34, 34, 255)), false, Align::Right);
        self.insert(o);
        self.status = "The slide number follows the slide. Put it on the slide master to show it on every slide.".into();
    }

    /// A caption (a speech or thought bubble) with its own text: shape `i` of `shapes::CAPTIONS` in the theme's accent
    /// colour with readable text, the text kept clear of the pointer.
    fn insert_caption(&mut self, i: usize) {
        use brainbend_doc::shapes::{caption, CAPTIONS, Preset};
        let accent = self.project.as_ref().map(|p| p.theme.color(4)).unwrap_or(Rgba(3, 83, 158, 255));
        let ink = self.project.as_ref().and_then(|p| p.theme.colors.first().copied()).unwrap_or(Rgba(34, 34, 34, 255));
        let (w, h) = (520.0, 300.0);
        let (preset, clear) = caption(i, w, h);
        let (kind, radius) = match preset {
            Preset::Path(cmds) => (Kind::Path(cmds), 0.0),
            Preset::Round(r) => (Kind::Rect, r * w.min(h)),
            Preset::Ellipse => (Kind::Ellipse, 0.0),
            _ => (Kind::Rect, 0.0),
        };
        let mut o = blank_object(self.new_id("obj"), CAPTIONS[i].0, kind, 0.0, 0.0, w, h);
        o.radius = radius;
        o.fill = Some(accent);
        o.stroke = Some(Stroke { color: Rgba(accent.0 / 2, accent.1 / 2, accent.2 / 2, 255), width: 2.0 });
        o.text = "Type your caption".into();
        let white = Rgba(255, 255, 255, 255);
        let contrast = |c: Rgba| brainbend_doc::check::contrast_ratio(c, accent);
        o.style = plain_style(36.0, if contrast(white) >= contrast(ink) { white } else { ink }, false, Align::Center);
        o.margins = [24.0, 12.0, 24.0, 12.0 + clear * h];
        self.insert(o);
    }

    /// A web page in a frame. The address is typed in Properties; until then the frame shows a prompt.
    fn insert_web_object(&mut self) {
        let o = blank_object(self.new_id("obj"), "Web object", Kind::Web(WebObject::new()), 0.0, 0.0, 1200.0, 700.0);
        self.insert(o);
        self.status = "Type the page's web address in Properties. Some sites refuse to be shown inside another page.".into();
    }

    fn insert_audio(&mut self) {
        let Some((key, name)) = self.load_media(&["mp3", "m4a", "ogg", "wav"], "Audio") else { return };
        let mut o = blank_object(self.new_id("obj"), &name, Kind::Audio, 0.0, 0.0, 800.0, 90.0);
        o.asset = Some(key);
        self.insert(o);
    }

    /// Timeline of the active layer: one row per object, a bar from its start to its end.
    /// Drag a bar to move it, drag its right end to change its duration; click to select.
    /// The States tab body: the cards and toolbar of the selected object's states. Edits made anywhere in the
    /// editor while a state is viewed go to that state (see `begin_state_swap`).
    fn states_tab(&mut self, ui: &mut egui::Ui, slide_idx: usize) {
        let Some((li, oi)) = self.selected else {
            ui.horizontal(|ui| {
                if ui.button("DONE").clicked() { self.bottom_tab = 0; }
                ui.weak("Select an object to see its states.");
            });
            return;
        };
        let proj = self.project.as_mut().unwrap();
        let before = proj.slides.clone();
        let Some(obj) = proj.slides[slide_idx].layers.get_mut(li).and_then(|l| l.objects.get_mut(oi)) else { return };
        if !states_panel::supports_states(obj) {
            ui.horizontal(|ui| {
                if ui.button("DONE").clicked() { self.bottom_tab = 0; }
                ui.weak("States are not available for this kind of object.");
            });
            return;
        }
        let mut viewed = match &self.state_view {
            Some((id, name)) if *id == obj.id => name.clone(),
            _ => obj.initial_state.clone(),
        };
        let id = obj.id.clone();
        let out = states_panel::editor(ui, obj, &mut viewed, &mut self.state_dim);
        self.state_view = Some((id, viewed.clone()));
        if out.changed {
            let mut before = before;
            self.unswapped(&mut before);
            self.push_undo(before);
            self.dirty = true;
        }
        if out.done {
            self.bottom_tab = 0;
            self.state_view = None;
        }
        if out.want_picture {
            if let Some((key, _, _, _)) = self.load_image_asset() {
                self.snapshot();
                if let Some(obj) = self.project.as_mut().and_then(|p| p.slides[slide_idx].layers.get_mut(li)).and_then(|l| l.objects.get_mut(oi)) {
                    // The object itself shows the viewed state this frame, so the new picture lands on that state.
                    obj.asset = Some(key);
                }
            }
        }
    }

    fn timeline_panel(&mut self, ui: &mut egui::Ui, slide_idx: usize) {
        const ROW_H: f32 = 22.0;
        const LABEL_W: f32 = 170.0;
        const SNAP_MS: f32 = 50.0;
        let Some(proj) = self.project.as_ref() else { return };
        let slide = &proj.slides[slide_idx];
        let li = self.layer.min(slide.layers.len() - 1);
        let layer = &slide.layers[li];
        // Front to back. A group is one row (folded unless opened), with its members under it when it is open.
        enum TlRow {
            Obj { oi: usize, name: String, timing: Option<Timing>, hidden: bool, icon: char, indent: f32 },
            Group { gid: String, members: Vec<usize>, top: usize, name: String, custom: Option<String>, hidden: bool, open: bool, table: bool, timings: Vec<(usize, Option<Timing>)> },
        }
        let mut rows: Vec<TlRow> = vec![];
        let mut emitted: Vec<&str> = vec![];
        for (i, o) in layer.objects.iter().enumerate().rev() {
            if let Some(g) = o.group.as_deref() {
                if emitted.contains(&g) { continue; }
                emitted.push(g);
                let members: Vec<usize> = layer.objects.iter().enumerate().rev().filter(|(_, m)| m.group.as_deref() == Some(g)).map(|(k, _)| k).collect();
                let table = members.iter().any(|&k| layer.objects[k].table.is_some());
                let open = self.tl_open.contains(g);
                let custom = layer.group_names.get(g).cloned();
                rows.push(TlRow::Group { gid: g.to_string(), top: i, name: format!("{} ({})", custom.clone().unwrap_or_else(|| if table { "Table".into() } else { "Group".into() }), members.len()), custom, hidden: members.iter().all(|&k| layer.objects[k].hidden), open, table, timings: members.iter().map(|&k| (k, layer.objects[k].timing)).collect(), members: members.clone() });
                if open {
                    for &k in &members { let m = &layer.objects[k]; rows.push(TlRow::Obj { oi: k, name: m.name.clone(), timing: m.timing, hidden: m.hidden, icon: objects_panel::kind_icon(m), indent: 16.0 }); }
                }
            } else {
                rows.push(TlRow::Obj { oi: i, name: o.name.clone(), timing: o.timing, hidden: o.hidden, icon: objects_panel::kind_icon(o), indent: 0.0 });
            }
        }
        let max_end = layer.objects.iter().filter_map(|o| o.timing).map(|t| t.start_ms + t.dur_ms.unwrap_or(0)).max().unwrap_or(0);
        let total = layer.duration_ms.unwrap_or(((max_end.max(4000) as f32 / 1000.0).ceil() as u32 + 1) * 1000) as f32;
        let layer_name = layer.name.clone();
        let timed_layer = layer.duration_ms.is_some();

        ui.horizontal(|ui| {
            ui.strong(format!("Timeline: {layer_name}"));
            if timed_layer { ui.label(format!("{:.1} s", total / 1000.0)); } else { ui.weak("no length set (Slide/Layer, Timeline, in Properties)"); }
        });
        let avail = ui.available_width();
        let track_w = (avail - LABEL_W - 24.0).max(120.0);
        let px = track_w / total;
        egui::ScrollArea::vertical().id_salt("tlscroll").show(ui, |ui| {
            // Ruler
            let (ruler, _) = ui.allocate_exact_size(egui::vec2(avail - 8.0, 16.0), egui::Sense::hover());
            let left = ruler.left() + LABEL_W;
            let step = if total > 60_000.0 { 10_000.0 } else if total > 20_000.0 { 5000.0 } else { 1000.0 };
            let mut t = 0.0;
            while t <= total {
                let x = left + t * px;
                ui.painter().line_segment([egui::pos2(x, ruler.bottom() - 6.0), egui::pos2(x, ruler.bottom())], egui::Stroke::new(1.0, egui::Color32::GRAY));
                ui.painter().text(egui::pos2(x + 3.0, ruler.top()), egui::Align2::LEFT_TOP, format!("{}s", t / 1000.0), egui::FontId::proportional(10.0), egui::Color32::GRAY);
                t += step;
            }
            let mut row_rects: Vec<(usize, egui::Rect)> = vec![];
            let mut row_action: Option<(usize, u8)> = None; // (object index, 0 delete, 1 duplicate, 2 front, 3 back)
            let in_selection = self.selection();
            for tl_row in rows {
                let (oi, name, timing, hidden, icon, indent) = match tl_row {
                    TlRow::Obj { oi, name, timing, hidden, icon, indent } => (oi, name, timing, hidden, icon, indent),
                    TlRow::Group { gid, members, top, name, custom, hidden, open, table, timings } => {
                        let (row, _) = ui.allocate_exact_size(egui::vec2(avail - 8.0, ROW_H), egui::Sense::hover());
                        row_rects.push((top, row));
                        let selected = members.iter().all(|&m| in_selection.contains(&(li, m)));
                        if selected { ui.painter().rect_filled(row, 2.0, egui::Color32::from_rgba_unmultiplied(0, 120, 255, 40)); }
                        let name_rect = egui::Rect::from_min_size(row.min, egui::vec2(LABEL_W - 6.0, ROW_H));
                        let name_resp = ui.interact(name_rect, ui.id().with(("tl-group", gid.as_str())), egui::Sense::click_and_drag());
                        let fg = if selected { egui::Color32::WHITE } else { egui::Color32::LIGHT_GRAY };
                        let dim = if hidden { 0.45 } else { 1.0 };
                        // Fold arrow, then the icon and the name.
                        let ac = egui::pos2(name_rect.left() + 8.0, row.center().y);
                        let tri = if open { vec![ac + egui::vec2(-4.0, -2.5), ac + egui::vec2(4.0, -2.5), ac + egui::vec2(0.0, 3.0)] } else { vec![ac + egui::vec2(-2.5, -4.0), ac + egui::vec2(-2.5, 4.0), ac + egui::vec2(3.0, 0.0)] };
                        ui.painter().add(egui::Shape::convex_polygon(tri, fg.gamma_multiply(dim), egui::Stroke::NONE));
                        let arrow = ui.interact(egui::Rect::from_center_size(ac, egui::vec2(18.0, ROW_H)), ui.id().with(("tl-fold", gid.as_str())), egui::Sense::click());
                        ui.painter().text(egui::pos2(name_rect.left() + 28.0, row.center().y), egui::Align2::CENTER_CENTER, if table { ribbon::LIST_NUM } else { ribbon::STACK }, egui::FontId::new(13.0, ribbon::icons()), fg.gamma_multiply(dim));
                        let clip = egui::Rect::from_min_max(egui::pos2(name_rect.left() + 40.0, row.top()), egui::pos2(name_rect.right() - 22.0, row.bottom()));
                        if self.group_edit.as_deref() == Some(gid.as_str()) {
                            let mut text = custom.clone().unwrap_or_else(|| if table { "Table".into() } else { "Group".into() });
                            let r = ui.put(clip.shrink2(egui::vec2(0.0, 1.0)), egui::TextEdit::singleline(&mut text).id(egui::Id::new("group-rename")));
                            if !r.has_focus() && !r.lost_focus() { r.request_focus(); }
                            if r.changed() && !text.trim().is_empty() {
                                if let Some(l) = self.project.as_mut().and_then(|p| p.slides[slide_idx].layers.get_mut(li)) { l.group_names.insert(gid.clone(), text); }
                                self.dirty = true;
                            }
                            if r.lost_focus() { self.group_edit = None; }
                        } else {
                            ui.painter().with_clip_rect(clip).text(egui::pos2(clip.left(), row.center().y), egui::Align2::LEFT_CENTER, &name, egui::FontId::proportional(12.0), fg.gamma_multiply(dim));
                        }
                        let eye_pos = egui::pos2(name_rect.right() - 10.0, row.center().y);
                        ui.painter().text(eye_pos, egui::Align2::CENTER_CENTER, if hidden { '\u{e224}' } else { '\u{e220}' }, egui::FontId::new(13.0, ribbon::icons()), fg.gamma_multiply(if hidden { 0.5 } else { 0.9 }));
                        let eye = ui.interact(egui::Rect::from_center_size(eye_pos, egui::vec2(20.0, ROW_H)), ui.id().with(("tl-geye", gid.as_str())), egui::Sense::click());
                        if arrow.clicked() {
                            if !self.tl_open.remove(&gid) { self.tl_open.insert(gid.clone()); }
                        } else if eye.on_hover_text(if hidden { "Hidden when the slide starts" } else { "Shown" }).clicked() {
                            self.snapshot();
                            if let Some(l) = self.project.as_mut().and_then(|p| p.slides[slide_idx].layers.get_mut(li)) {
                                for &m in &members { if let Some(o) = l.objects.get_mut(m) { o.hidden = !hidden; } }
                            }
                            self.dirty = true;
                        } else if name_resp.clicked() {
                            self.selected = Some((li, top));
                            self.multi.clear();
                        }
                        if name_resp.double_clicked() { self.snapshot(); self.selected = Some((li, top)); self.multi.clear(); self.group_edit = Some(gid.clone()); }
                        if name_resp.drag_started() { self.obj_drag = Some(top); self.obj_drag_group = true; self.selected = Some((li, top)); self.multi.clear(); }
                        if name_resp.secondary_clicked() && !selected { self.selected = Some((li, top)); self.multi.clear(); }
                        name_resp.context_menu(|ui| {
                            if ui.button(if open { "Fold" } else { "Show members" }).clicked() { if !self.tl_open.remove(&gid) { self.tl_open.insert(gid.clone()); } ui.close(); }
                            if ui.button("Rename").clicked() { self.snapshot(); self.group_edit = Some(gid.clone()); ui.close(); }
                            ui.separator();
                            if ui.button("Set start and length").on_hover_text("The same start and length for every member").clicked() { row_action = Some((top, 7)); ui.close(); }
                            if ui.button("Show for the whole timeline").clicked() { row_action = Some((top, 8)); ui.close(); }
                            ui.separator();
                            if ui.button("Duplicate").clicked() { row_action = Some((top, 1)); ui.close(); }
                            if ui.button("Bring to front").clicked() { row_action = Some((top, 5)); ui.close(); }
                            if ui.button("Send to back").clicked() { row_action = Some((top, 6)); ui.close(); }
                            if ui.button("Ungroup").clicked() { row_action = Some((top, 4)); ui.close(); }
                            ui.separator();
                            if ui.button("Delete").clicked() { row_action = Some((top, 0)); ui.close(); }
                        });
                        // The group's bar spans its members. Dragging it moves them all together (each keeps its own length);
                        // its right edge sets where they all end. Each member's own bar is under the fold.
                        let track = egui::Rect::from_min_size(egui::pos2(left, row.top() + 3.0), egui::vec2(track_w, ROW_H - 6.0));
                        ui.painter().rect_filled(track, 2.0, egui::Color32::from_gray(38));
                        let spans: Vec<(f32, f32)> = timings.iter().filter_map(|(_, t)| t.as_ref()).map(|t| (t.start_ms as f32, t.dur_ms.map(|d| t.start_ms as f32 + d as f32).unwrap_or(total))).collect();
                        let (a, b) = if spans.is_empty() { (0.0, total) } else { (spans.iter().map(|s| s.0).fold(f32::MAX, f32::min), spans.iter().map(|s| s.1).fold(0.0, f32::max)) };
                        let bar = egui::Rect::from_min_max(egui::pos2(left + a * px, track.top()), egui::pos2((left + b * px).max(left + a * px + 4.0), track.bottom()));
                        ui.painter().rect_filled(bar, 3.0, if spans.is_empty() { egui::Color32::from_gray(80) } else { egui::Color32::from_rgb(40, 120, 220) });
                        let edge = egui::Rect::from_min_max(egui::pos2(bar.right() - 6.0, bar.top()), bar.right_bottom());
                        let gid_id = ui.id().with(("tlg", gid.as_str()));
                        let edge_resp = ui.interact(edge, gid_id.with("edge"), egui::Sense::drag());
                        let body_resp = ui.interact(bar, gid_id.with("body"), egui::Sense::click_and_drag());
                        if edge_resp.hovered() || edge_resp.dragged() { ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal); }
                        if body_resp.clicked() || edge_resp.clicked() { self.selected = Some((li, top)); self.multi.clear(); }
                        let begun = if edge_resp.drag_started() { Some(1u8) } else if body_resp.drag_started() { Some(0u8) } else { None };
                        if let Some(kind) = begun {
                            self.selected = Some((li, top));
                            self.snapshot();
                            self.tl_gdrag = Some((gid.clone(), kind, timings.clone(), 0.0));
                        }
                        if edge_resp.dragged() || body_resp.dragged() {
                            if let Some((g, kind, originals, acc)) = self.tl_gdrag.as_mut().filter(|d| d.0 == gid) {
                                let _ = g;
                                *acc += if *kind == 1 { edge_resp.drag_delta().x } else { body_resp.drag_delta().x };
                                let ms = ((*acc / px) / SNAP_MS).round() * SNAP_MS;
                                let group_end = b;
                                let writes: Vec<(usize, Timing)> = originals.iter().map(|(m, t)| {
                                    let (s0, d0) = t.map(|t| (t.start_ms as f32, t.dur_ms.map(|d| d as f32))).unwrap_or((0.0, None));
                                    if *kind == 0 {
                                        (*m, Timing { start_ms: (s0 + ms).max(0.0) as u32, dur_ms: d0.map(|d| d as u32) })
                                    } else {
                                        let end = (group_end + ms).max(SNAP_MS);
                                        (*m, Timing { start_ms: s0 as u32, dur_ms: Some((end - s0).max(SNAP_MS) as u32) })
                                    }
                                }).collect();
                                if let Some(l) = self.project.as_mut().and_then(|p| p.slides[slide_idx].layers.get_mut(li)) {
                                    for (m, t) in writes { if let Some(o) = l.objects.get_mut(m) { o.timing = Some(t); } }
                                }
                                self.dirty = true;
                            }
                        }
                        if edge_resp.drag_stopped() || body_resp.drag_stopped() { self.tl_gdrag = None; }
                        continue;
                    }
                };
                let (row, _) = ui.allocate_exact_size(egui::vec2(avail - 8.0, ROW_H), egui::Sense::hover());
                row_rects.push((oi, row));
                let selected = in_selection.contains(&(li, oi));
                if selected { ui.painter().rect_filled(row, 2.0, egui::Color32::from_rgba_unmultiplied(0, 120, 255, 40)); }
                // The name column is the object list: select (⌘/⇧ adds), rename by double-click, drag to reorder, eye to hide.
                let name_rect = egui::Rect::from_min_size(row.min + egui::vec2(indent, 0.0), egui::vec2(LABEL_W - 6.0 - indent, ROW_H));
                let name_resp = ui.interact(name_rect, ui.id().with(("tl-name", oi)), egui::Sense::click_and_drag());
                let fg = if selected { egui::Color32::WHITE } else { egui::Color32::LIGHT_GRAY };
                let dim = if hidden { 0.45 } else { 1.0 };
                ui.painter().text(egui::pos2(name_rect.left() + 10.0, row.center().y), egui::Align2::CENTER_CENTER, icon, egui::FontId::new(13.0, ribbon::icons()), fg.gamma_multiply(dim));
                if self.obj_edit == Some((li, oi)) {
                    let er = egui::Rect::from_min_max(egui::pos2(name_rect.left() + 22.0, row.top() + 1.0), egui::pos2(name_rect.right() - 22.0, row.bottom() - 1.0));
                    let mut text = name.clone();
                    let r = ui.put(er, egui::TextEdit::singleline(&mut text).id(egui::Id::new("object-rename")));
                    if !r.has_focus() && !r.lost_focus() { r.request_focus(); }
                    if r.changed() && !text.trim().is_empty() {
                        if let Some(o) = self.project.as_mut().and_then(|p| p.slides[slide_idx].layers.get_mut(li)).and_then(|l| l.objects.get_mut(oi)) { o.name = text; }
                        self.dirty = true;
                    }
                    if r.lost_focus() { self.obj_edit = None; }
                } else {
                    let clip = egui::Rect::from_min_max(egui::pos2(name_rect.left() + 22.0, row.top()), egui::pos2(name_rect.right() - 22.0, row.bottom()));
                    ui.painter().with_clip_rect(clip).text(egui::pos2(clip.left(), row.center().y), egui::Align2::LEFT_CENTER, name.clone(), egui::FontId::proportional(12.0), fg.gamma_multiply(dim));
                }
                let eye_pos = egui::pos2(name_rect.right() - 10.0, row.center().y);
                ui.painter().text(eye_pos, egui::Align2::CENTER_CENTER, if hidden { '\u{e224}' } else { '\u{e220}' }, egui::FontId::new(13.0, ribbon::icons()), fg.gamma_multiply(if hidden { 0.5 } else { 0.9 }));
                let eye = ui.interact(egui::Rect::from_center_size(eye_pos, egui::vec2(20.0, ROW_H)), ui.id().with(("tl-eye", oi)), egui::Sense::click());
                if eye.on_hover_text(if hidden { "Hidden when the slide starts" } else { "Shown" }).clicked() {
                    self.snapshot();
                    if let Some(o) = self.project.as_mut().and_then(|p| p.slides[slide_idx].layers.get_mut(li)).and_then(|l| l.objects.get_mut(oi)) { o.hidden = !o.hidden; }
                    self.dirty = true;
                } else if name_resp.clicked() {
                    let cmd = ui.input(|i| i.modifiers.command || i.modifiers.shift);
                    let id = self.project.as_ref().unwrap().slides[slide_idx].layers[li].objects[oi].id.clone();
                    if cmd && self.selected.is_some_and(|sel| sel.0 == li) && self.selected != Some((li, oi)) {
                        if let Some(k) = self.multi.iter().position(|m| *m == id) { self.multi.remove(k); } else { self.multi.push(id); }
                    } else {
                        self.selected = Some((li, oi));
                        self.multi.clear();
                    }
                }
                if name_resp.double_clicked() { self.snapshot(); self.selected = Some((li, oi)); self.multi.clear(); self.obj_edit = Some((li, oi)); }
                if name_resp.drag_started() && self.obj_edit.is_none() { self.obj_drag = Some(oi); self.obj_drag_group = false; }
                if name_resp.secondary_clicked() && !selected { self.selected = Some((li, oi)); self.multi.clear(); }
                name_resp.context_menu(|ui| {
                    if ui.button("Rename").clicked() { self.snapshot(); self.obj_edit = Some((li, oi)); ui.close(); }
                    if ui.button("Duplicate").clicked() { row_action = Some((oi, 1)); ui.close(); }
                    if ui.button("Bring to front").clicked() { row_action = Some((oi, 2)); ui.close(); }
                    if ui.button("Send to back").clicked() { row_action = Some((oi, 3)); ui.close(); }
                    ui.separator();
                    if ui.button("Delete").clicked() { row_action = Some((oi, 0)); ui.close(); }
                });
                let track = egui::Rect::from_min_size(egui::pos2(left, row.top() + 3.0), egui::vec2(track_w, ROW_H - 6.0));
                ui.painter().rect_filled(track, 2.0, egui::Color32::from_gray(38));
                let (start, dur) = timing.map(|t| (t.start_ms as f32, t.dur_ms.map(|d| d as f32))).unwrap_or((0.0, None));
                let end = dur.map(|d| start + d).unwrap_or(total).min(total.max(start + 1.0));
                let bar = egui::Rect::from_min_max(egui::pos2(left + start * px, track.top()), egui::pos2((left + end * px).max(left + start * px + 4.0), track.bottom()));
                let col = if timing.is_some() { egui::Color32::from_rgb(40, 120, 220) } else { egui::Color32::from_gray(80) };
                ui.painter().rect_filled(bar, 3.0, col);
                // Interaction: right-edge handle (duration) first, then the body (move / select).
                let edge = egui::Rect::from_min_max(egui::pos2(bar.right() - 6.0, bar.top()), bar.right_bottom());
                let id = ui.id().with(("tl", oi));
                let edge_resp = ui.interact(edge, id.with("edge"), egui::Sense::drag());
                let body_resp = ui.interact(bar, id.with("body"), egui::Sense::click_and_drag());
                if edge_resp.hovered() || edge_resp.dragged() { ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal); }
                if body_resp.clicked() || edge_resp.clicked() || body_resp.secondary_clicked() { self.selected = Some((li, oi)); }
                let timed_now = timing.is_some();
                body_resp.context_menu(|ui| {
                    if timed_now {
                        if ui.button("Show for the whole timeline").clicked() { self.snapshot(); if let Some(o) = self.project.as_mut().and_then(|p| p.slides[slide_idx].layers.get_mut(li)).and_then(|l| l.objects.get_mut(oi)) { o.timing = None; } self.dirty = true; ui.close(); }
                        if ui.button("Start at the beginning").clicked() { self.snapshot(); if let Some(o) = self.project.as_mut().and_then(|p| p.slides[slide_idx].layers.get_mut(li)).and_then(|l| l.objects.get_mut(oi)) { if let Some(t) = o.timing.as_mut() { t.start_ms = 0; } } self.dirty = true; ui.close(); }
                        if ui.button("Last until the end").clicked() { self.snapshot(); if let Some(o) = self.project.as_mut().and_then(|p| p.slides[slide_idx].layers.get_mut(li)).and_then(|l| l.objects.get_mut(oi)) { if let Some(t) = o.timing.as_mut() { t.dur_ms = None; } } self.dirty = true; ui.close(); }
                    } else if ui.button("Set start and length").clicked() {
                        self.snapshot();
                        if let Some(o) = self.project.as_mut().and_then(|p| p.slides[slide_idx].layers.get_mut(li)).and_then(|l| l.objects.get_mut(oi)) { o.timing = Some(Timing { start_ms: 0, dur_ms: Some(2000) }); }
                        self.dirty = true;
                        ui.close();
                    }
                    ui.separator();
                    let hidden_now = hidden;
                    if ui.button(if hidden_now { "Show when the slide starts" } else { "Hide when the slide starts" }).clicked() { self.snapshot(); if let Some(o) = self.project.as_mut().and_then(|p| p.slides[slide_idx].layers.get_mut(li)).and_then(|l| l.objects.get_mut(oi)) { o.hidden = !o.hidden; } self.dirty = true; ui.close(); }
                    if ui.button("Delete").clicked() { self.selected = Some((li, oi)); self.multi.clear(); self.delete_selected(); ui.close(); }
                });
                let begun = if edge_resp.drag_started() { Some(1u8) } else if body_resp.drag_started() { Some(0u8) } else { None };
                if let Some(kind) = begun {
                    self.selected = Some((li, oi));
                    self.snapshot();
                    self.tl_drag = Some((oi, kind, start as u32, dur.map(|d| d as u32), 0.0));
                }
                let moving = edge_resp.dragged() || body_resp.dragged();
                if moving {
                    if let Some((doi, kind, s0, d0, acc)) = self.tl_drag.as_mut().filter(|d| d.0 == oi) {
                        *acc += if *kind == 1 { edge_resp.drag_delta().x } else { body_resp.drag_delta().x };
                        let ms = ((*acc / px) / SNAP_MS).round() * SNAP_MS;
                        let (new_start, new_dur) = if *kind == 0 {
                            ((*s0 as f32 + ms).max(0.0) as u32, *d0)
                        } else {
                            let base = d0.map(|d| d as f32).unwrap_or((total - *s0 as f32).max(SNAP_MS));
                            (*s0, Some((base + ms).max(SNAP_MS) as u32))
                        };
                        let _ = doi;
                        if let Some(o) = self.project.as_mut().and_then(|p| p.slides[slide_idx].layers.get_mut(li)).and_then(|l| l.objects.get_mut(oi)) {
                            o.timing = Some(Timing { start_ms: new_start, dur_ms: new_dur });
                        }
                        self.dirty = true;
                    }
                }
                if edge_resp.drag_stopped() || body_resp.drag_stopped() { self.tl_drag = None; }
            }
            // Drag to reorder: the object takes the z-position of the row it is dropped on (higher rows are in front).
            if let Some(src) = self.obj_drag {
                let py = ui.ctx().pointer_interact_pos().map(|p| p.y).unwrap_or(0.0);
                let dst = row_rects.iter().find(|(_, r)| py >= r.top() && py < r.bottom()).map(|(i, _)| *i)
                    .unwrap_or(if py < row_rects.first().map(|r| r.1.top()).unwrap_or(0.0) { row_rects.first().map(|r| r.0).unwrap_or(src) } else { row_rects.last().map(|r| r.0).unwrap_or(src) });
                if let Some((_, r)) = row_rects.iter().find(|(i, _)| *i == dst) {
                    if dst != src {
                        let y = if dst > src { r.top() } else { r.bottom() };
                        ui.painter().hline(r.x_range(), y, egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 120, 215)));
                    }
                }
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                if !ui.input(|i| i.pointer.primary_down()) {
                    self.obj_drag = None;
                    if dst != src { if self.obj_drag_group { self.move_group(li, src, dst); } else { self.reorder_object(li, src, dst); } }
                    self.obj_drag_group = false;
                }
            }
            if let Some((oi, what)) = row_action {
                self.selected = Some((li, oi));
                self.multi.clear();
                match what {
                    0 => self.delete_selected(),
                    1 => self.duplicate_selected(),
                    2 => self.reorder_selected(true),
                    3 => self.reorder_selected(false),
                    4 => self.ungroup_selected(),
                    5 => self.move_group_to_edge(li, oi, true),
                    6 => self.move_group_to_edge(li, oi, false),
                    7 | 8 => {
                        // The same timing for every member of the group (or none: shown for the whole timeline).
                        self.snapshot();
                        let sel = self.selection();
                        if let Some(l) = self.project.as_mut().and_then(|p| p.slides[slide_idx].layers.get_mut(li)) {
                            for (_, k) in sel { if let Some(o) = l.objects.get_mut(k) { o.timing = if what == 7 { Some(Timing { start_ms: 0, dur_ms: Some(2000) }) } else { None }; } }
                        }
                        self.dirty = true;
                    }
                    _ => {}
                }
            }
        });
    }

    /// Move the group that object `src` of layer `li` belongs to so it sits just in front of (or behind) object `dst`.
    fn move_group(&mut self, li: usize, src: usize, dst: usize) {
        let Some(layer) = self.project.as_ref().and_then(|p| p.slides.get(self.slide)).and_then(|s| s.layers.get(li)) else { return };
        let (Some(gid), Some(target)) = (layer.objects.get(src).and_then(|o| o.group.clone()), layer.objects.get(dst)) else { return };
        if target.group.as_ref() == Some(&gid) { return; }
        let target_id = target.id.clone();
        self.snapshot();
        let layer = &mut self.project.as_mut().unwrap().slides[self.slide].layers[li];
        let mut block = vec![];
        let mut k = 0;
        while k < layer.objects.len() {
            if layer.objects[k].group.as_ref() == Some(&gid) { block.push(layer.objects.remove(k)); } else { k += 1; }
        }
        let t = layer.objects.iter().position(|o| o.id == target_id).unwrap_or(0);
        let at = if dst > src { t + 1 } else { t };
        let first = at;
        for (n, o) in block.into_iter().enumerate() { layer.objects.insert(at + n, o); }
        self.selected = Some((li, first));
        self.multi.clear();
        self.dirty = true;
    }

    /// Bring the group of object `oi` to the very front, or send it to the very back.
    fn move_group_to_edge(&mut self, li: usize, oi: usize, front: bool) {
        let Some(gid) = self.project.as_ref().and_then(|p| p.slides.get(self.slide)).and_then(|s| s.layers.get(li)).and_then(|l| l.objects.get(oi)).and_then(|o| o.group.clone()) else { return };
        self.snapshot();
        let layer = &mut self.project.as_mut().unwrap().slides[self.slide].layers[li];
        let mut block = vec![];
        let mut k = 0;
        while k < layer.objects.len() {
            if layer.objects[k].group.as_ref() == Some(&gid) { block.push(layer.objects.remove(k)); } else { k += 1; }
        }
        let first = if front { layer.objects.len() } else { 0 };
        for (n, o) in block.into_iter().enumerate() { layer.objects.insert(first + n, o); }
        self.selected = Some((li, first));
        self.multi.clear();
        self.dirty = true;
    }

    /// Objects that formatting (font, fill, outline, alignment, lists) applies to: the chosen block of
    /// cells when a table cell is current, otherwise the whole selection.
    fn format_targets(&self) -> Vec<(usize, usize)> {
        let (Some((g, blk)), Some((li, _))) = (self.cell_block(), self.selected) else { return self.selection() };
        let Some(objs) = self.project.as_ref().and_then(|p| p.slides.get(self.slide)).and_then(|s| s.layers.get(li)).map(|l| &l.objects) else { return vec![] };
        let mut v: Vec<(usize, usize)> = brainbend_doc::tables::cells_in(objs, &g, blk).into_iter().map(|k| (li, k)).collect();
        v.sort();
        v
    }

    /// Choose the block of table cells between grid positions `a` and `b` of the selected table.
    fn select_cells(&mut self, a: (u16, u16), b: (u16, u16)) {
        let Some((li, _)) = self.selected else { return };
        let Some(g) = self.selected_object().filter(|o| o.table.is_some()).and_then(|o| o.group.clone()) else { return };
        let objs = &self.project.as_ref().unwrap().slides[self.slide].layers[li].objects;
        let Some(k) = brainbend_doc::tables::cell_at(objs, &g, a.0, a.1) else { return };
        self.selected = Some((li, k));
        self.range_end = Some((g, b));
    }

    /// Set borders on the chosen block of cells.
    fn set_cell_borders(&mut self, set: brainbend_doc::tables::BorderSet, line: Option<Stroke>) {
        let Some((g, blk)) = self.cell_block() else { return };
        let Some((li, _)) = self.selected else { return };
        self.snapshot();
        let objs = &mut self.project.as_mut().unwrap().slides[self.slide].layers[li].objects;
        brainbend_doc::tables::set_borders(objs, &g, blk, set, line);
        self.dirty = true;
    }

    /// The table cell block chosen: the selected cell, or the block from it to the shift-clicked cell
    /// (grown so it cuts no merged cell). Returns the table's group and (row0, col0, row1, col1).
    fn cell_block(&self) -> Option<(String, (u16, u16, u16, u16))> {
        let o = self.selected_object()?;
        let (g, a) = (o.group.clone()?, o.table?);
        let objs = &self.project.as_ref()?.slides.get(self.slide)?.layers.get(self.selected?.0)?.objects;
        let b = self.range_end.as_ref().filter(|(rg, _)| *rg == g).map(|(_, rc)| *rc).unwrap_or(a);
        Some((g.clone(), brainbend_doc::tables::expand_range(objs, &g, a, b)))
    }

    /// Merge the chosen block of table cells into one cell.
    fn merge_cells(&mut self) {
        let Some((g, (r0, c0, r1, c1))) = self.cell_block() else { return };
        let Some((li, _)) = self.selected else { return };
        if (r0, c0) == (r1, c1) { return; }
        self.snapshot();
        let objs = &mut self.project.as_mut().unwrap().slides[self.slide].layers[li].objects;
        brainbend_doc::tables::merge(objs, &g, (r0, c0), (r1, c1));
        self.selected = objs.iter().position(|o| o.group.as_deref() == Some(g.as_str()) && o.table == Some((r0, c0))).map(|k| (li, k));
        self.multi.clear();
        self.range_end = None;
        self.dirty = true;
    }

    /// Split the selected merged cell back into separate cells.
    fn split_cell(&mut self) {
        let Some((li, oi)) = self.selected else { return };
        let Some(o) = self.selected_object() else { return };
        let (Some(rc), Some(g), true) = (o.table, o.group.clone(), o.table_span.is_some()) else { return };
        let mut ids: Vec<String> = (0..256).map(|_| self.new_id("obj")).collect();
        self.snapshot();
        let objs = &mut self.project.as_mut().unwrap().slides[self.slide].layers[li].objects;
        brainbend_doc::tables::split(objs, &g, rc, &mut || ids.pop().unwrap_or_default());
        self.selected = Some((li, oi.min(objs.len() - 1)));
        self.range_end = None;
        self.dirty = true;
    }

    /// Give the selected table a look (and remember it as the one the Table panel shows).
    fn restyle_table(&mut self, style: brainbend_doc::tables::TableStyle, opts: brainbend_doc::tables::StyleOptions) {
        let Some((li, _)) = self.selected else { return };
        let Some(g) = self.selected_object().filter(|o| o.table.is_some()).and_then(|o| o.group.clone()) else { return };
        self.snapshot();
        let theme = self.project.as_ref().unwrap().theme.clone();
        let objs = &mut self.project.as_mut().unwrap().slides[self.slide].layers[li].objects;
        brainbend_doc::tables::apply_style(objs, &g, style, opts, &theme);
        self.table_style = style;
        self.table_opts = opts;
        self.dirty = true;
    }

    /// The Table section of the Properties panel and the right-click menu: merge, split and looks.
    /// The side panel for a selected table: its controls live in the ribbon's Table Tools tab.
    fn table_panel(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("Table").strong());
        ui.label(egui::RichText::new("Click a cell, then shift-click another to choose a block of cells.").weak().size(12.0));
        ui.add_space(4.0);
        ui.label("Styles, rows and columns, merging and borders are on the Home tab, under TABLE. Font, fill and alignment there apply to the chosen cells.");
        if ui.button("Open Home").clicked() { self.ribbon_tab = ribbon::HOME_TAB; }
    }

    /// What a hyperlink looks like in text: the theme's hyperlink colour (accent 1), as the PowerPoint export sets it.
    fn link_color(&self) -> egui::Color32 {
        color(self.project.as_ref().map(|p| p.theme.color(4)).unwrap_or(Rgba(3, 83, 158, 255)))
    }

    fn selected_object(&self) -> Option<&Object> {
        let (li, oi) = self.selected?;
        self.project.as_ref()?.slides.get(self.slide)?.layers.get(li)?.objects.get(oi)
    }

    /// Every selected object (primary plus shift-selected), in stacking order, all on one layer.
    fn selection(&self) -> Vec<(usize, usize)> {
        let Some((li, oi)) = self.selected else { return vec![] };
        let Some(layer) = self.project.as_ref().and_then(|p| p.slides.get(self.slide)).and_then(|s| s.layers.get(li)) else { return vec![] };
        let mut v: Vec<(usize, usize)> = layer.objects.iter().enumerate()
            .filter(|(k, o)| *k == oi || self.multi.contains(&o.id))
            .map(|(k, _)| (li, k)).collect();
        // Choosing one member of a group chooses all of them.
        let groups: Vec<&String> = v.iter().filter_map(|&(_, k)| layer.objects[k].group.as_ref()).collect();
        if !groups.is_empty() {
            for (k, o) in layer.objects.iter().enumerate() {
                if o.group.as_ref().is_some_and(|g| groups.contains(&g)) && !v.contains(&(li, k)) { v.push((li, k)); }
            }
        }
        v.sort();
        v
    }

    /// Bullets or numbers on every selected object's paragraphs.
    fn set_list_selected(&mut self, style: ListStyle) {
        let sel = self.format_targets();
        if sel.is_empty() { return; }
        self.snapshot();
        let layers = &mut self.project.as_mut().unwrap().slides[self.slide].layers;
        for (l, k) in sel { layers[l].objects[k].set_list(style); }
        self.dirty = true;
    }

    /// Tie the selected objects together so they select, move and resize as one.
    fn group_selected(&mut self) {
        let sel = self.selection();
        if sel.len() < 2 { return; }
        self.snapshot();
        let gid = self.new_id("group");
        let layers = &mut self.project.as_mut().unwrap().slides[self.slide].layers;
        for (l, k) in sel { layers[l].objects[k].group = Some(gid.clone()); }
        self.dirty = true;
    }

    fn ungroup_selected(&mut self) {
        let sel = self.selection();
        if !sel.iter().any(|&(l, k)| self.project.as_ref().unwrap().slides[self.slide].layers[l].objects[k].group.is_some()) { return; }
        self.snapshot();
        let layers = &mut self.project.as_mut().unwrap().slides[self.slide].layers;
        for (l, k) in sel { layers[l].objects[k].group = None; }
        self.dirty = true;
    }

    /// The hotspot being edited, mutably, found by id (it may no longer be the selection).
    fn outline_object_mut(&mut self) -> Option<&mut Object> {
        let id = self.outline_mode.clone()?;
        self.project.as_mut()?.slides.get_mut(self.slide)?.layers.iter_mut().flat_map(|l| l.objects.iter_mut()).find(|o| o.id == id)
    }

    /// Start editing the selected hotspot's outline (it gets the frame's four corners if it had none).
    fn begin_outline(&mut self) {
        let Some(o) = self.selected_object().filter(|o| matches!(o.kind, Kind::Hotspot)) else { return };
        self.outline_mode = Some(o.id.clone());
        self.outline_sel = None;
        self.crop_mode = None;
        if let Some(o) = self.outline_object_mut() { outline::begin(o); }
    }

    /// Stop editing the outline; one that is just the frame's corners goes back to being no outline.
    fn end_outline(&mut self) {
        if let Some(o) = self.outline_object_mut() { outline::finish(o); }
        self.outline_mode = None;
        self.outline_sel = None;
    }

    /// Give the hotspot its box back: no outline.
    fn reset_outline(&mut self) {
        let Some(o) = self.selected_object().filter(|o| matches!(o.kind, Kind::Hotspot) && o.hit_area.is_some()) else { return };
        let id = o.id.clone();
        self.snapshot();
        self.outline_mode = Some(id);
        if let Some(o) = self.outline_object_mut() { o.hit_area = None; }
        self.outline_mode = None;
        self.outline_sel = None;
        self.dirty = true;
    }

    /// Shrink the hotspot's frame to its outline.
    fn fit_outline(&mut self) {
        let Some(o) = self.selected_object().filter(|o| matches!(o.kind, Kind::Hotspot) && o.hit_area.as_ref().is_some_and(|a| !outline::is_box(a))) else { return };
        let keep = self.outline_mode.clone();
        self.outline_mode = Some(o.id.clone());
        self.snapshot();
        if let Some(o) = self.outline_object_mut() { outline::fit_frame(o); }
        self.outline_mode = keep;
        self.dirty = true;
    }

    /// Remove the chosen corner of the outline being edited.
    fn remove_outline_point(&mut self) {
        let Some(i) = self.outline_sel else { return };
        if !self.outline_object_mut().is_some_and(|o| o.hit_area.as_ref().is_some_and(|a| a.len() > outline::MIN_POINTS && i < a.len())) { return; }
        self.snapshot();
        if let Some(a) = self.outline_object_mut().and_then(|o| o.hit_area.as_mut()) { outline::remove(a, i); }
        self.outline_sel = None;
        self.dirty = true;
    }

    /// Move the chosen corner of the outline being edited.
    fn nudge_outline_point(&mut self, dx: f32, dy: f32) {
        let Some(i) = self.outline_sel else { return };
        self.snapshot();
        if let Some(o) = self.outline_object_mut() { outline::nudge_point(o, i, (dx, dy)); }
        self.dirty = true;
    }

    /// A new hotspot with a rectangle, an oval or (drawn next) a freeform outline.
    fn insert_hotspot(&mut self, oval: bool) {
        let mut o = blank_object(self.new_id("obj"), "Hotspot", Kind::Hotspot, 0.0, 0.0, 480.0, 280.0);
        if oval { o.name = "Oval hotspot".into(); o.hit_area = Some(outline::ellipse_outline()); }
        self.insert(o);
    }

    /// Finish the freeform hotspot being drawn: a hotspot whose frame fits the corners clicked.
    fn finish_draw_outline(&mut self) {
        let Some(points) = self.draw_outline.take() else { return };
        let Some((x, y, w, h, area)) = outline::from_points(&points) else {
            self.status = "A freeform hotspot needs at least three corners that do not lie on a line.".into();
            return;
        };
        let mut o = blank_object(self.new_id("obj"), "Freeform hotspot", Kind::Hotspot, x, y, w, h);
        o.hit_area = Some(area);
        self.snapshot();
        let li = self.layer.min(self.project.as_ref().unwrap().slides[self.slide].layers.len() - 1);
        let slide = &mut self.project.as_mut().unwrap().slides[self.slide];
        slide.layers[li].objects.push(o);
        self.selected = Some((li, slide.layers[li].objects.len() - 1));
        self.ribbon_tab = ribbon::HOME_TAB;
        self.multi.clear();
        self.dirty = true;
        self.status = "Freeform hotspot added. Double-click it to move its corners.".into();
    }

    /// Slide the picture being cropped by (`dx`, `dy`) screen-aligned slide units, under its frame.
    fn pan_cropped(&mut self, dx: f32, dy: f32) {
        let Some((li, oi)) = self.selected else { return };
        let Some(cur) = self.selected_object().cloned() else { return };
        let a = cur.rotation.to_radians();
        let local = (dx * a.cos() + dy * a.sin(), -dx * a.sin() + dy * a.cos());
        let mut next = cur.clone();
        crop::pan(&mut next, local);
        if next.crop == cur.crop { return; }
        self.snapshot();
        if let Some(o) = self.project.as_mut().and_then(|p| p.slides.get_mut(self.slide)).and_then(|s| s.layers.get_mut(li)).and_then(|l| l.objects.get_mut(oi)) { o.crop = next.crop; }
        self.dirty = true;
    }

    /// Crop the selected picture to a shape (or back to its whole frame).
    fn set_picture_shape(&mut self, shape: Option<brainbend_doc::PictureShape>) {
        let Some((li, oi)) = self.selected else { return };
        if !self.selected_object().is_some_and(|o| matches!(o.kind, Kind::Image) && o.mask != shape) { return; }
        self.snapshot();
        if let Some(o) = self.project.as_mut().and_then(|p| p.slides.get_mut(self.slide)).and_then(|s| s.layers.get_mut(li)).and_then(|l| l.objects.get_mut(oi)) { o.mask = shape; }
        self.dirty = true;
    }

    /// Width over height of the source picture of the object `id` on the current slide.
    fn picture_ratio(&self, id: &str) -> Option<f32> {
        let p = self.project.as_ref()?;
        let o = p.slides.get(self.slide)?.layers.iter().flat_map(|l| l.objects.iter()).find(|o| o.id == id)?;
        let a = p.assets.get(o.asset.as_deref()?)?;
        let (w, h) = image::ImageReader::new(std::io::Cursor::new(&a.bytes)).with_guessed_format().ok()?.into_dimensions().ok()?;
        (h > 0).then(|| w as f32 / h as f32)
    }

    /// Hold the crop frame to `ratio` (width over height) and close it in to that shape now.
    fn set_crop_aspect(&mut self, ratio: Option<f32>) {
        self.crop_aspect = ratio;
        let Some(r) = ratio else { return };
        let Some((li, oi)) = self.selected else { return };
        self.snapshot();
        if let Some(o) = self.project.as_mut().and_then(|p| p.slides.get_mut(self.slide)).and_then(|s| s.layers.get_mut(li)).and_then(|l| l.objects.get_mut(oi)) { crop::fit_aspect(o, r); }
        self.dirty = true;
    }

    /// Remove the selected picture's crop so the whole picture shows.
    fn reset_crop(&mut self) {
        if !self.selected_object().is_some_and(|o| o.crop.is_some()) { return; }
        self.snapshot();
        let Some((li, oi)) = self.selected else { return };
        if let Some(o) = self.project.as_mut().and_then(|p| p.slides.get_mut(self.slide)).and_then(|s| s.layers.get_mut(li)).and_then(|l| l.objects.get_mut(oi)) { crop::reset(o); }
        self.crop_mode = None;
    }

    fn delete_selected(&mut self) {
        let sel = self.selection();
        if sel.is_empty() { return; }
        self.snapshot();
        let slide = &mut self.project.as_mut().unwrap().slides[self.slide];
        for &(li, oi) in sel.iter().rev() {
            let id = slide.layers[li].objects.remove(oi).id;
            slide.triggers.retain(|t| !matches!(&t.owner, Owner::Object(o) if *o == id));
        }
        self.selected = None;
        self.multi.clear();
    }

    /// Align the selection to its bounding box (or space it out evenly). `axis`: 0 = horizontal, 1 = vertical;
    /// `mode`: 0 = start, 1 = center, 2 = end, 3 = distribute.
    fn align_selected(&mut self, axis: u8, mode: u8) {
        let sel = self.selection();
        if sel.len() < 2 || (mode == 3 && sel.len() < 3) { return; }
        self.snapshot();
        let layer = &mut self.project.as_mut().unwrap().slides[self.slide].layers[sel[0].0];
        let pos = |o: &Object| if axis == 0 { (o.x, o.w) } else { (o.y, o.h) };
        let lo = sel.iter().map(|&(_, k)| pos(&layer.objects[k]).0).fold(f32::MAX, f32::min);
        let hi = sel.iter().map(|&(_, k)| { let (p, l) = pos(&layer.objects[k]); p + l }).fold(f32::MIN, f32::max);
        if mode == 3 {
            let mut order: Vec<usize> = sel.iter().map(|s| s.1).collect();
            order.sort_by(|&a, &b| pos(&layer.objects[a]).0.total_cmp(&pos(&layer.objects[b]).0));
            let total: f32 = order.iter().map(|&k| pos(&layer.objects[k]).1).sum();
            let gap = (hi - lo - total) / (order.len() - 1) as f32;
            let mut cur = lo;
            for k in order {
                let len = pos(&layer.objects[k]).1;
                if axis == 0 { layer.objects[k].x = cur } else { layer.objects[k].y = cur }
                cur += len + gap;
            }
            return;
        }
        for &(_, k) in &sel {
            let o = &mut layer.objects[k];
            let len = pos(o).1;
            let v = match mode { 0 => lo, 1 => (lo + hi) / 2.0 - len / 2.0, _ => hi - len };
            if axis == 0 { o.x = v } else { o.y = v }
        }
    }

    fn duplicate_selected(&mut self) {
        let sel = self.selection();
        let Some(&(li, _)) = sel.first() else { return };
        let mut copies = vec![];
        let mut gmap: HashMap<String, String> = HashMap::new();
        for &(l, k) in &sel {
            let mut o = self.project.as_ref().unwrap().slides[self.slide].layers[l].objects[k].clone();
            o.id = self.new_id("obj");
            if let Some(g) = o.group.take() {
                let n = gmap.len();
                let new = gmap.entry(g).or_insert_with(|| format!("group-copy-{}-{n}", self.id_counter)).clone();
                o.group = Some(new);
            }
            o.name = format!("{} copy", o.name);
            o.x += 24.0;
            o.y += 24.0;
            copies.push(o);
        }
        self.snapshot();
        let layer = &mut self.project.as_mut().unwrap().slides[self.slide].layers[li];
        let first = layer.objects.len();
        self.multi = copies.iter().skip(1).map(|o| o.id.clone()).collect();
        layer.objects.extend(copies);
        self.selected = Some((li, first));
        self.ribbon_tab = ribbon::HOME_TAB;
    }

    fn copy_selected(&mut self) {
        self.clipboard = self.selected_object().cloned();
        // The desktop only sends a Paste event when the system clipboard holds text, so a copy leaves some there.
        self.clip_out = self.clipboard.as_ref().map(|o| if o.text.trim().is_empty() { o.name.clone() } else { o.text.clone() });
        if self.clipboard.is_some() { self.status = "Copied".into(); }
    }

    /// Paste the copied object onto the active layer, offset so it is visibly a copy.
    fn paste(&mut self) {
        let Some(mut o) = self.clipboard.clone() else { return };
        if self.project.is_none() { return; }
        o.id = self.new_id("obj");
        o.group = None;
        o.x += 24.0;
        o.y += 24.0;
        self.clipboard = Some(o.clone());
        let slide = &self.project.as_ref().unwrap().slides[self.slide];
        let li = self.layer.min(slide.layers.len() - 1);
        self.snapshot();
        let layer = &mut self.project.as_mut().unwrap().slides[self.slide].layers[li];
        layer.objects.push(o);
        self.selected = Some((li, layer.objects.len() - 1));
        self.ribbon_tab = ribbon::HOME_TAB;
    }

    fn reorder_selected(&mut self, to_front: bool) {
        let Some((li, oi)) = self.selected else { return };
        self.multi.clear();
        self.snapshot();
        let layer = &mut self.project.as_mut().unwrap().slides[self.slide].layers[li];
        let o = layer.objects.remove(oi);
        if to_front {
            layer.objects.push(o);
            self.selected = Some((li, layer.objects.len() - 1));
        } else {
            layer.objects.insert(0, o);
            self.selected = Some((li, 0));
        }
    }

    fn nudge_selected(&mut self, dx: f32, dy: f32) {
        let sel = self.selection();
        if sel.is_empty() { return; }
        self.snapshot_if_new_edit();
        for (li, oi) in sel {
            if let Some(o) = self.project.as_mut().and_then(|p| p.slides[self.slide].layers.get_mut(li)).and_then(|l| l.objects.get_mut(oi)) {
                o.x += dx;
                o.y += dy;
            }
        }
    }

    /// Undo step for keyboard nudging: one per key press burst is fine for now.
    fn snapshot_if_new_edit(&mut self) {
        self.snapshot();
    }

    fn snapshot(&mut self) {
        let snap = self.project.as_ref().map(|p| {
            let mut snap = Snap::of(p);
            self.unswapped(&mut snap.slides);
            snap
        });
        if let Some(snap) = snap {
            self.undo.push(snap);
            self.redo.clear();
            self.dirty = true;
        }
    }

    /// While the States tab is open its object wears the viewed state's look; undo steps hold the object's own.
    fn unswapped(&self, slides: &mut [Slide]) {
        if let Some(sw) = &self.swap {
            states_panel::unswap(slides, sw);
        }
    }

    /// Put the States tab's viewed state on its object for this frame.
    fn begin_state_swap(&mut self) {
        self.swap = None;
        if self.bottom_tab != 2 || self.try_mode { return; }
        let Some((li, oi)) = self.selected else { return };
        let slide_idx = self.slide;
        let Some(proj) = self.project.as_mut() else { return };
        let Some(o) = proj.slides.get(slide_idx).and_then(|s| s.layers.get(li)).and_then(|l| l.objects.get(oi)) else { return };
        let (id, init) = (o.id.clone(), o.initial_state.clone());
        let viewed = match &self.state_view {
            Some((vid, name)) if *vid == id && o.states.iter().any(|s| &s.name == name) => name.clone(),
            _ => init,
        };
        self.state_view = Some((id.clone(), viewed.clone()));
        self.swap = states_panel::begin(&mut proj.slides, slide_idx, &id, &viewed);
    }

    /// Copy the viewed state's edits back into the state and give the object its own look again.
    fn end_state_swap(&mut self) {
        if let (Some(sw), Some(p)) = (self.swap.take(), self.project.as_mut()) {
            states_panel::end(&mut p.slides, sw);
        }
    }

    /// Record an undo step for an edit whose "before" slides were captured by the caller
    /// (project-level settings are unchanged during such edits).
    fn push_undo(&mut self, mut before: Vec<Slide>) {
        self.unswapped(&mut before);
        if let Some(p) = &self.project {
            self.undo.push(Snap { slides: before, meta: Meta::of(p) });
            self.redo.clear();
        }
    }

    /// Record an undo step for a project-level settings edit made this frame.
    fn push_undo_meta(&mut self, before: Meta) {
        if let Some(p) = &self.project {
            let mut slides = p.slides.clone();
            self.unswapped(&mut slides);
            self.undo.push(Snap { slides, meta: before });
            self.redo.clear();
            self.dirty = true;
        }
    }

    fn undo(&mut self) {
        self.end_state_swap();
        if let (Some(p), Some(prev)) = (self.project.as_mut(), self.undo.pop()) {
            let cur = prev.swap_into(p);
            self.redo.push(cur);
            self.selected = None;
            self.dirty = true;
        }
    }

    fn redo(&mut self) {
        self.end_state_swap();
        if let (Some(p), Some(next)) = (self.project.as_mut(), self.redo.pop()) {
            let cur = next.swap_into(p);
            self.undo.push(cur);
            self.selected = None;
            self.dirty = true;
        }
    }

    /// Triggers for the selected object (or the layer / slide when nothing is selected), shown in the right panel.
    fn triggers_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, slide_idx: usize) {
        let (owner, title) = if let Some(o) = self.selected_object() {
            (Owner::Object(o.id.clone()), format!("Triggers: {}", o.name))
        } else {
            let slide = &self.project.as_ref().unwrap().slides[slide_idx];
            let l = &slide.layers[self.layer.min(slide.layers.len() - 1)];
            if self.layer == 0 { (Owner::Slide, "Triggers: slide (runs when the slide starts or a variable changes)".to_string()) }
            else { (Owner::Layer(l.id.clone()), format!("Triggers: layer {}", l.name)) }
        };
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.trig_all, false, "Selected");
            ui.selectable_value(&mut self.trig_all, true, "All");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("Variables…").on_hover_text("Add or change project variables").clicked() { self.variables_open = true; }
            });
        });
        if !self.trig_all { ui.strong(title); }
        // The slide's whole set: the slide itself, each layer, then each object, with how many triggers each has.
        let groups: Vec<(Owner, String, Option<(usize, usize)>, usize)> = if self.trig_all {
            let slide = &self.project.as_ref().unwrap().slides[slide_idx];
            let count = |o: &Owner| slide.triggers.iter().filter(|t| owner_eq(&t.owner, o)).count();
            let mut v = vec![(Owner::Slide, "Slide".to_string(), None, count(&Owner::Slide))];
            for (li, l) in slide.layers.iter().enumerate() {
                if li > 0 { let o = Owner::Layer(l.id.clone()); let n = count(&o); v.push((o, format!("Layer: {}", l.name), Some((li, usize::MAX)), n)); }
            }
            for (li, l) in slide.layers.iter().enumerate() {
                for (oi, ob) in l.objects.iter().enumerate() {
                    let o = Owner::Object(ob.id.clone());
                    let n = count(&o);
                    v.push((o, ob.name.clone(), Some((li, oi)), n));
                }
            }
            v.retain(|g| g.3 > 0);
            v
        } else { vec![] };
        let mut ctr = self.id_counter;
        let t0 = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let mut gen = || { ctr += 1; format!("trig-{t0:x}-{ctr}") };
        let proj = self.project.as_mut().unwrap();
        let before = proj.slides.clone();
        let vars = proj.variables.clone();
        let slide_list: Vec<(String, String)> = proj.slides.iter().map(|s| (s.id.clone(), s.name.clone())).collect();
        let mut changed = false;
        let mut pick: Option<(usize, usize)> = None;
        let all = self.trig_all;
        egui::ScrollArea::vertical().id_salt("trigscroll").show(ui, |ui| {
            if !all {
                changed = triggers_ui(ui, &mut proj.slides[slide_idx], &slide_list, &vars, &owner, None, &mut gen);
                return;
            }
            if groups.is_empty() { ui.weak("No triggers on this slide yet."); }
            for (gi, (o, name, at, n)) in groups.iter().enumerate() {
                let h = egui::CollapsingHeader::new(format!("{name}  ({n})")).id_salt(("trig-group", gi, name.as_str())).default_open(true).show(ui, |ui| {
                    changed |= triggers_ui(ui, &mut proj.slides[slide_idx], &slide_list, &vars, o, None, &mut gen);
                });
                if h.header_response.clicked() {
                    let already = match at { Some((li, oi)) if *oi != usize::MAX => self.selected == Some((*li, *oi)), _ => false };
                    pick = Some(at.unwrap_or((0, usize::MAX)));
                    // The first click on a group selects its owner and leaves it open; a click on the one already chosen folds it.
                    if !already {
                        let id = ui.make_persistent_id(("trig-group", gi, name.as_str()));
                        let mut st = egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true);
                        st.set_open(true);
                        st.store(ui.ctx());
                    }
                }
            }
        });
        self.id_counter = ctr;
        if let Some((li, oi)) = pick {
            // Clicking a group's heading selects its owner. That is a move made inside the Triggers tab, so it stays there.
            self.trigger_select = true;
            if oi == usize::MAX { self.layer = li; self.selected = None; } else { self.layer = li; self.selected = Some((li, oi)); }
        }
        if changed {
            let editing = ctx.input(|i| i.pointer.any_down()) || ctx.memory(|m| m.focused().is_some());
            if !(self.edit_open && editing) {
                self.push_undo(before);
            }
            self.edit_open = true;
            self.dirty = true;
        }
    }

    /// Free-form notes for the slide (speaker notes / author reminders), saved with the project.
    fn notes_panel(&mut self, ui: &mut egui::Ui, slide_idx: usize) {
        let mut notes = self.project.as_ref().unwrap().slides[slide_idx].notes.clone();
        let r = egui::ScrollArea::vertical().id_salt("slide-notes").show(ui, |ui| {
            ui.add_sized(ui.available_size(), egui::TextEdit::multiline(&mut notes).hint_text("Notes for this slide"))
        }).inner;
        if r.gained_focus() { self.snapshot(); }
        if r.changed() {
            self.project.as_mut().unwrap().slides[slide_idx].notes = notes;
            self.dirty = true;
        }
    }

    /// The project's variables in a window of their own: they belong to the whole project, not to a slide.
    /// Focus Order: the order a keyboard (Tab) and a screen reader reach the objects of the current layer.
    fn focus_order_window(&mut self, ctx: &egui::Context) {
        if !self.focus_open { return; }
        let Some(p) = self.project.as_ref() else { self.focus_open = false; return };
        let (si, li) = (self.slide, self.layer);
        let Some(layer) = p.slides.get(si).and_then(|s| s.layers.get(li)) else { self.focus_open = false; return };
        let rows: Vec<(usize, String, char, bool, bool)> = layer.focus_indices().into_iter().map(|i| { let o = &layer.objects[i]; (i, o.name.clone(), objects_panel::kind_icon(o), o.decorative, o.hidden) }).collect();
        let (title, custom) = (layer.name.clone(), !layer.focus_order.is_empty());
        let mut open = true;
        let mut mv: Option<(usize, i32)> = None;
        let mut skip: Option<usize> = None;
        let mut pick: Option<usize> = None;
        let mut reset = false;
        egui::Window::new("Focus Order").id(egui::Id::new("focus-order")).open(&mut open).collapsible(false).resizable(true).default_size([340.0, 360.0]).default_pos(ctx.content_rect().center() - egui::vec2(170.0, 180.0)).show(ctx, |ui| {
            ui.weak(format!("The order the Tab key and screen readers move through \"{title}\", first to last."));
            ui.add_space(4.0);
            let last = rows.len().saturating_sub(1);
            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                for (pos, (i, name, icon, decorative, hidden)) in rows.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.add_sized([22.0, 20.0], egui::Label::new(egui::RichText::new(format!("{}", pos + 1)).weak()));
                        ui.label(egui::RichText::new(icon.to_string()).family(ribbon::icons()).size(14.0));
                        let dim = *decorative || *hidden;
                        let mut t = egui::RichText::new(name);
                        if dim { t = t.weak(); }
                        if ui.selectable_label(self.selected == Some((li, *i)), t).clicked() { pick = Some(*i); }
                        if *decorative { ui.weak("skipped by screen readers"); } else if *hidden { ui.weak("hidden"); }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_enabled_ui(pos < last, |ui| { if ui.small_button(egui::RichText::new(ribbon::ARROW_DOWN.to_string()).family(ribbon::icons())).on_hover_text("Later").clicked() { mv = Some((*i, 1)); } });
                            ui.add_enabled_ui(pos > 0, |ui| { if ui.small_button(egui::RichText::new(ribbon::ARROW_UP.to_string()).family(ribbon::icons())).on_hover_text("Earlier").clicked() { mv = Some((*i, -1)); } });
                            if ui.small_button(egui::RichText::new(ribbon::EYE_OFF.to_string()).family(ribbon::icons())).on_hover_text("Skip this object for screen readers (decorative)").clicked() { skip = Some(*i); }
                        });
                    });
                }
            });
            ui.add_space(4.0);
            ui.add_enabled_ui(custom, |ui| { if ui.button("Reset Order").on_hover_text("Reach objects back to front, as they are stacked on the slide").clicked() { reset = true; } });
        });
        if !open { self.focus_open = false; }
        if let Some(i) = pick { self.selected = Some((li, i)); self.multi.clear(); }
        if mv.is_some() || skip.is_some() || reset {
            self.snapshot();
            if let Some(layer) = self.project.as_mut().and_then(|p| p.slides.get_mut(si)).and_then(|s| s.layers.get_mut(li)) {
                if let Some((i, by)) = mv { let id = layer.objects[i].id.clone(); layer.move_focus(&id, by); }
                if let Some(i) = skip { layer.objects[i].decorative = !layer.objects[i].decorative; }
                if reset { layer.focus_order.clear(); }
            }
            self.dirty = true;
        }
    }

    fn variables_window(&mut self, ctx: &egui::Context) {
        if ctx.data_mut(|d| d.remove_temp::<bool>(egui::Id::new("open-variables"))).unwrap_or(false) { self.variables_open = true; }
        if !self.variables_open { return; }
        if self.project.is_none() { self.variables_open = false; return; }
        let mut open = true;
        egui::Window::new("Variables").id(egui::Id::new("project-variables")).open(&mut open).collapsible(false).resizable(true).default_size([420.0, 360.0]).default_pos(ctx.content_rect().center() - egui::vec2(210.0, 180.0)).show(ctx, |ui| {
            ui.weak("Variables belong to the whole project. Triggers read and change them.");
            ui.add_space(4.0);
            self.variables_panel(ui, ctx);
        });
        if !open { self.variables_open = false; self.var_rename = None; self.var_note = None; }
    }

    /// Renames a variable, rewriting the `%Name%` text that shows it. One undo step.
    fn rename_variable(&mut self, id: &str, name: &str) -> Result<(), String> {
        let p = self.project.as_ref().ok_or("No project is open.")?;
        let clean = brainbend_doc::variables::check_new_name(p, id, name)?;
        let old = p.variables.iter().find(|v| v.id == id).map(|v| v.name.clone()).unwrap_or_default();
        if old == clean { return Ok(()); }
        self.snapshot();
        let report = brainbend_doc::variables::rename(self.project.as_mut().unwrap(), id, &clean)?;
        let mut note = format!("Renamed {old} to {}.", report.name);
        if report.objects > 0 { note.push_str(&format!(" Updated {} that show{} it in their text.", if report.objects == 1 { "1 object".to_string() } else { format!("{} objects", report.objects) }, if report.objects == 1 { "s" } else { "" })); }
        if report.javascript { note.push_str(" JavaScript you wrote that uses the old name was not changed."); }
        self.var_note = Some((note, false));
        self.dirty = true;
        Ok(())
    }

    /// Changes a variable's type, converting the values triggers set or compare. Refused (nothing changes) when something
    /// needs it to stay as it is. One undo step.
    fn convert_variable(&mut self, id: &str, to: brainbend_doc::variables::VarType) -> Result<(), String> {
        use brainbend_doc::variables as vars;
        let p = self.project.as_ref().ok_or("No project is open.")?;
        let v = p.variables.iter().find(|v| v.id == id).ok_or("That variable no longer exists.")?;
        let name = v.name.clone();
        if vars::type_of(&v.initial) == to { return Ok(()); }
        let blockers = vars::conversion_blockers(p, id, to);
        if !blockers.is_empty() {
            return Err(format!("Cannot make \"{name}\" {}. It is used by {}. Change those first, or keep it as it is.", to.label(), blockers.join(" and ")));
        }
        self.snapshot();
        let report = vars::convert(self.project.as_mut().unwrap(), id, to)?;
        let mut note = format!("\"{name}\" is now {}.", to.label());
        if report.triggers > 0 { note.push_str(&format!(" Updated the values in {} trigger{}.", report.triggers, if report.triggers == 1 { "" } else { "s" })); }
        if report.lossy { note.push_str(" Some values could not be carried over exactly."); }
        self.var_note = Some((note, false));
        self.dirty = true;
        Ok(())
    }

    /// Project variables (used by triggers): search and filter, add, set the initial value, delete.
    fn variables_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let in_triggers = variables_in_triggers(self.project.as_ref().unwrap());
        let total = self.project.as_ref().unwrap().variables.iter().filter(|v| !v.built_in).count();
        let filtering = !self.var_query.trim().is_empty() || self.var_kind != 0 || self.var_use != 0;
        if total > 0 {
            ui.horizontal_wrapped(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.var_query).hint_text("Search variables").desired_width(118.0));
                combo("varkindfilter").width(78.0).selected_text(["All types", "true/false", "number", "text"][self.var_kind]).show_ui(ui, |ui| {
                    for (i, l) in ["All types", "true/false", "number", "text"].iter().enumerate() { ui.selectable_value(&mut self.var_kind, i, *l); }
                });
                combo("varusefilter").width(112.0).selected_text(["Any use", "Used in a trigger", "In no trigger"][self.var_use]).show_ui(ui, |ui| {
                    for (i, l) in ["Any use", "Used in a trigger", "In no trigger"].iter().enumerate() { ui.selectable_value(&mut self.var_use, i, *l); }
                }).response.on_hover_text("Only triggers are checked. A slider, an input, a dial or text that shows %name% can still use a variable that is in no trigger.");
                if filtering && ui.small_button("Clear").clicked() { self.var_query.clear(); self.var_kind = 0; self.var_use = 0; }
            });
            if filtering {
                let shown = self.project.as_ref().unwrap().variables.iter().filter(|v| !v.built_in && variable_shown(v, &in_triggers, &self.var_query, self.var_kind, self.var_use)).count();
                ui.weak(format!("Showing {shown} of {total} variables"));
                if shown == 0 { ui.label("No variables match."); }
            }
            ui.add_space(2.0);
        }
        let mut clear_note = false;
        if let Some((note, refused)) = &self.var_note {
            ui.horizontal_wrapped(|ui| {
                if *refused { ui.colored_label(egui::Color32::from_rgb(230, 90, 80), note.as_str()); } else { ui.label(egui::RichText::new(note.as_str()).color(egui::Color32::from_rgb(90, 180, 120))); }
                if ui.small_button("OK").clicked() { clear_note = true; }
            });
            ui.add_space(2.0);
        }
        if clear_note { self.var_note = None; }
        let (query, kind, usage) = (self.var_query.clone(), self.var_kind, self.var_use);
        let mut start_rename: Option<(String, String)> = None;
        let mut new_var_problem: Option<String> = None;
        let mut commit_rename: Option<(String, String)> = None;
        let mut cancel_rename = false;
        let mut convert_var: Option<(String, brainbend_doc::variables::VarType)> = None;
        let mut add_var: Option<(String, usize)> = None;
        let mut remove_var: Option<String> = None;
        let mut dup_var: Option<String> = None;
        let mut var_edited = false;
        let var_meta_before = Meta::of(self.project.as_ref().unwrap());
        egui::ScrollArea::vertical().id_salt("vars").show(ui, |ui| {
            let proj = self.project.as_mut().unwrap();
            for v in proj.variables.iter_mut().filter(|v| !v.built_in && variable_shown(v, &in_triggers, &query, kind, usage)) {
                let row = ui.scope_builder(egui::UiBuilder::new().sense(egui::Sense::click()), |ui| { ui.style_mut().interaction.selectable_labels = false; ui.horizontal(|ui| {
                    let renaming = self.var_rename.as_ref().is_some_and(|(id, ..)| *id == v.id);
                    if renaming {
                        if let Some((_, buf, fresh)) = self.var_rename.as_mut() {
                            let r = ui.add(egui::TextEdit::singleline(buf).desired_width(120.0));
                            if *fresh { r.request_focus(); *fresh = false; }
                            if ui.input(|i| i.key_pressed(egui::Key::Escape)) { cancel_rename = true; }
                            else if r.lost_focus() { commit_rename = Some((v.id.clone(), buf.clone())); }
                        }
                        ui.label("=");
                    } else if ui.add(egui::Label::new(format!("{} =", v.name)).sense(egui::Sense::click())).on_hover_text("Double-click to rename").double_clicked() {
                        start_rename = Some((v.id.clone(), v.name.clone()));
                    }
                    if value_editor(ui, &mut v.initial) { var_edited = true; }
                    if ui.small_button("x").on_hover_text("Delete variable and the triggers that use it").clicked() {
                        remove_var = Some(v.id.clone());
                    }
                }) });
                row.response.context_menu(|ui| {
                    if ui.button("Copy name").clicked() { ui.ctx().copy_text(v.name.clone()); ui.close(); }
                    if ui.button("Duplicate").clicked() { dup_var = Some(v.id.clone()); ui.close(); }
                    if ui.button("Rename…").clicked() { start_rename = Some((v.id.clone(), v.name.clone())); ui.close(); }
                    ui.menu_button("Change type", |ui| {
                        use brainbend_doc::variables::{type_of, VarType};
                        let now = type_of(&v.initial);
                        for t in VarType::ALL {
                            let tip = match (now, t) {
                                (VarType::Text, VarType::Num) => "Text that is not a number becomes 0",
                                (VarType::Text, VarType::Bool) => "Only true, 1 or yes become true; anything else becomes false",
                                (VarType::Num, VarType::Bool) => "0 becomes false and anything else becomes true",
                                _ => "",
                            };
                            let b = ui.add_enabled(t != now, egui::Button::new(if t == now { format!("{} (current)", t.label()) } else { t.label().to_string() }));
                            let b = if tip.is_empty() { b } else { b.on_hover_text(tip) };
                            if b.clicked() { convert_var = Some((v.id.clone(), t)); ui.close(); }
                        }
                    });
                    ui.separator();
                    if ui.button("Delete").on_hover_text("Deletes the variable and the triggers that use it").clicked() { remove_var = Some(v.id.clone()); ui.close(); }
                });
            }
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.new_var_name).hint_text("new variable").desired_width(110.0));
                combo("newvarkind").width(80.0).selected_text(["true/false", "number", "text"][self.new_var_kind]).show_ui(ui, |ui| {
                    for (i, l) in ["true/false", "number", "text"].iter().enumerate() {
                        ui.selectable_value(&mut self.new_var_kind, i, *l);
                    }
                });
                let name = self.new_var_name.trim().to_string();
                let problem = if name.is_empty() { None } else { brainbend_doc::variables::check_name(&name).err().or_else(|| proj.variables.iter().any(|v| v.name == name).then(|| format!("A variable called \"{name}\" already exists."))) };
                if ui.add_enabled(!name.is_empty() && problem.is_none(), egui::Button::new("Add")).clicked() {
                    add_var = Some((name, self.new_var_kind));
                }
                new_var_problem = problem;
            });
            if let Some(why) = &new_var_problem { ui.colored_label(egui::Color32::from_rgb(230, 90, 80), why); }
        });
        if var_edited {
            let editing = ctx.input(|i| i.pointer.any_down()) || ctx.memory(|m| m.focused().is_some());
            if !(self.edit_open && editing) { self.push_undo_meta(var_meta_before); }
            self.edit_open = true;
            self.dirty = true;
        }
        if let Some((name, kind)) = add_var {
            self.snapshot();
            let id = self.new_id("var");
            let initial = match kind { 0 => Value::Bool(false), 1 => Value::Num(0.0), _ => Value::Text(String::new()) };
            self.project.as_mut().unwrap().variables.push(Variable { id, name, initial, built_in: false });
            self.new_var_name.clear();
            self.dirty = true;
        }
        if let Some((id, name)) = start_rename { self.var_rename = Some((id, name, true)); self.var_note = None; }
        if cancel_rename { self.var_rename = None; }
        if let Some((id, name)) = commit_rename {
            match self.rename_variable(&id, &name) {
                Ok(()) => self.var_rename = None,
                // Stay in the box so the name can be fixed.
                Err(e) => { self.var_note = Some((e, true)); if let Some(r) = self.var_rename.as_mut() { r.2 = true; } }
            }
        }
        if let Some((id, to)) = convert_var {
            if let Err(e) = self.convert_variable(&id, to) { self.var_note = Some((e, true)); }
        }
        if let Some(id) = dup_var {
            self.snapshot();
            let new = self.new_id("var");
            let proj = self.project.as_mut().unwrap();
            if let Some(v) = proj.variables.iter().find(|v| v.id == id).cloned() {
                let mut name = format!("{}_copy", v.name);
                while proj.variables.iter().any(|x| x.name == name) { name.push_str("_2"); }
                proj.variables.push(Variable { id: new, name, initial: v.initial, built_in: false });
            }
            self.dirty = true;
        }
        if let Some(id) = remove_var {
            self.snapshot();
            let proj = self.project.as_mut().unwrap();
            proj.variables.retain(|v| v.id != id);
            for sl in proj.slides.iter_mut() {
                sl.triggers.retain(|t| {
                    let uses = matches!(&t.event, Event::VariableChanged(v) if *v == id)
                        || matches!(&t.action, Action::SetVariable { variable, .. } if *variable == id)
                        || t.conditions.iter().any(|c| c.variable == id);
                    !uses
                });
            }
        }
    }

    /// Slide Layers: a panel pinned to the bottom of the right column.
    /// Topmost layer first, the base layer (named after the slide) last; each row has a thumbnail,
    /// the name, and an eye to show or hide the layer on the canvas.
    fn layers_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, slide_idx: usize) {
        use ribbon::icons;
        let slide = self.project.as_ref().unwrap().slides[slide_idx].clone();
        let n = slide.layers.len();
        if self.visible.len() != n { self.visible = vec![true; n]; }
        ui.add_space(2.0);
        section_label(ui, "Slide Layers");
        ui.add_space(2.0);
        let list_h = (ui.available_height() - 40.0).max(40.0);
        egui::ScrollArea::vertical().id_salt("layers").max_height(list_h).auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            // Rows sit at the bottom of the list area.
            ui.add_space((list_h - n as f32 * 34.0).max(0.0));
            let mut rows: Vec<(usize, egui::Rect)> = Vec::new();
            for li in (0..n).rev() {
                let selected = li == self.layer;
                let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 34.0), egui::Sense::click_and_drag());
                rows.push((li, rect));
                if resp.secondary_clicked() { self.layer = li; self.selected = None; }
                resp.context_menu(|ui| {
                    if ui.button("New layer").clicked() { self.add_layer(); ui.close(); }
                    ui.separator();
                    if ui.button("Rename").clicked() { self.snapshot(); self.layer = li; self.layer_edit = Some(li); ui.close(); }
                    ui.add_enabled_ui(li > 0, |ui| {
                        let shown = self.visible.get(li).copied().unwrap_or(true);
                        if ui.button(if shown { "Hide" } else { "Show" }).clicked() { if let Some(v) = self.visible.get_mut(li) { *v = !*v; } ui.close(); }
                        if ui.button("Duplicate").clicked() { self.duplicate_layer(li); ui.close(); }
                        ui.add_enabled_ui(li + 1 < n, |ui| { if ui.button("Bring forward").clicked() { self.move_layer(li, li + 1); ui.close(); } });
                        ui.add_enabled_ui(li > 1, |ui| { if ui.button("Send backward").clicked() { self.move_layer(li, li - 1); ui.close(); } });
                        ui.separator();
                        if ui.button("Delete").clicked() { self.delete_layer(li); ui.close(); }
                    });
                });
                if resp.drag_started() && li > 0 && self.layer_edit.is_none() { self.layer_drag = Some(li); }
                if resp.double_clicked() {
                    self.snapshot();
                    self.layer = li;
                    self.layer_edit = Some(li);
                    self.selected = None;
                }
                let blue = ui.visuals().selection.bg_fill;
                if selected {
                    ui.painter().rect_filled(rect, 0.0, blue);
                } else if resp.hovered() {
                    ui.painter().rect_filled(rect, 0.0, ui.visuals().text_color().gamma_multiply(0.08));
                }
                let fg = if selected { egui::Color32::WHITE } else { ui.visuals().text_color() };
                // Thumbnail: this layer drawn small on the slide background.
                let tw = 34.0;
                let th = (tw * slide.height / slide.width).max(14.0);
                let thumb = egui::Rect::from_min_size(egui::pos2(rect.left() + 10.0, rect.center().y - th / 2.0), egui::vec2(tw, th));
                let tp = ui.painter_at(thumb);
                if li == 0 { self.paint_backdrop(ctx, &tp, thumb.min, tw / slide.width, &slide); } else { tp.rect_filled(thumb, 0.0, egui::Color32::WHITE); }
                self.paint_slide(ctx, &tp, thumb.min, tw / slide.width, &slide, &[li], None);
                ui.painter().rect_stroke(thumb, 0.0, egui::Stroke::new(1.0, if selected { egui::Color32::from_white_alpha(160) } else { egui::Color32::from_gray(150) }), egui::StrokeKind::Inside);
                let name = if li == 0 { slide.name.clone() } else { slide.layers[li].name.clone() };
                if self.layer_edit == Some(li) {
                    let er = egui::Rect::from_min_max(egui::pos2(rect.left() + 52.0, rect.center().y - 12.0), egui::pos2(rect.right() - 44.0, rect.center().y + 12.0));
                    let mut text = name.clone();
                    let r = ui.put(er, egui::TextEdit::singleline(&mut text).id(egui::Id::new("layer-rename")));
                    if !r.has_focus() && !r.lost_focus() { r.request_focus(); }
                    if r.changed() && !text.trim().is_empty() {
                        let sl = &mut self.project.as_mut().unwrap().slides[slide_idx];
                        if li == 0 { sl.name = text; } else { sl.layers[li].name = text; }
                        self.dirty = true;
                    }
                    if r.lost_focus() { self.layer_edit = None; }
                } else {
                    // Long names stop short of the "(Base Layer)" tag and the eye.
                    let room = (rect.width() - 56.0 - if li == 0 { 112.0 } else { 40.0 }).max(20.0);
                    let clip = egui::Rect::from_min_size(egui::pos2(rect.left() + 56.0, rect.top()), egui::vec2(room, rect.height()));
                    ui.painter().with_clip_rect(clip).text(egui::pos2(rect.left() + 56.0, rect.center().y), egui::Align2::LEFT_CENTER, name, egui::FontId::proportional(13.0), fg);
                }
                // Eye at the right edge; the base layer is always shown.
                let eye_pos = egui::pos2(rect.right() - 20.0, rect.center().y);
                let eye_rect = egui::Rect::from_center_size(eye_pos, egui::vec2(24.0, 24.0));
                let on = li == 0 || self.visible[li] || li == self.layer;
                if li == 0 {
                    ui.painter().text(egui::pos2(rect.right() - 38.0, rect.center().y), egui::Align2::RIGHT_CENTER, "(Base Layer)", egui::FontId::proportional(11.5), fg.gamma_multiply(0.75));
                }
                ui.painter().text(eye_pos, egui::Align2::CENTER_CENTER, if on { '\u{e220}' } else { '\u{e224}' }, egui::FontId::new(16.0, icons()), if on { fg } else { fg.gamma_multiply(0.45) });
                let eye = ui.interact(eye_rect, egui::Id::new(("layer-eye", li)), egui::Sense::click());
                if eye.clicked() && li > 0 {
                    // The eye on the layer being worked on hides it and goes back to the base layer; on any other it keeps the layer showing or not.
                    if li == self.layer { self.hide_selected_layer(); } else { self.visible[li] = !self.visible[li]; }
                }
                else if resp.clicked() {
                    // Selecting a layer shows it while it is selected; only its eye keeps it showing afterwards.
                    self.layer = li;
                    self.selected = None;
                }
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, if li == 0 { "Base layer" } else { "Layer" }));
            }
            // Drag to reorder: the layer lands where the pointer is; the base layer stays last.
            if let Some(src) = self.layer_drag {
                let py = ctx.pointer_interact_pos().map(|p| p.y).unwrap_or(0.0);
                let top = rows.first().map(|r| r.1.top()).unwrap_or(0.0);
                let dst = if py < top { n - 1 } else {
                    rows.iter().find(|(_, r)| py >= r.top() && py < r.bottom()).map(|(l, _)| *l).unwrap_or(1)
                }.max(1);
                if let Some((_, r)) = rows.iter().find(|(l, _)| *l == dst) {
                    let y = if dst > src { r.top() } else if dst < src { r.bottom() } else { f32::NAN };
                    if !y.is_nan() {
                        ui.painter().hline(r.x_range(), y, egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 120, 215)));
                    }
                }
                ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
                if !ctx.input(|i| i.pointer.primary_down()) {
                    self.layer_drag = None;
                    if dst != src { self.move_layer(src, dst); }
                }
            }
        });
        ui.add_space(6.0);
        // Footer: new, duplicate, delete, Dim, and layer settings.
        let can_delete = self.layer > 0;
        ui.horizontal(|ui| {
            let btn = |ui: &mut egui::Ui, icon: char, tip: &str, enabled: bool| {
                ui.add_enabled(enabled, egui::Button::new(egui::RichText::new(icon.to_string()).family(icons()).size(17.0)).min_size(egui::vec2(30.0, 28.0))).on_hover_text(tip)
            };
            if btn(ui, '\u{e236}', "New layer", true).clicked() { self.add_layer(); }
            if btn(ui, '\u{e1ca}', "Duplicate layer", can_delete).clicked() { self.duplicate_layer(self.layer); }
            if btn(ui, '\u{e4a6}', "Delete layer", can_delete).clicked() { self.delete_layer(self.layer); }
            ui.checkbox(&mut self.dim_base, "Dim").on_hover_text("Dim the base layer while you work on another layer");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let gear = btn(ui, '\u{e270}', "Layer settings", true);
                egui::Popup::from_toggle_button_response(&gear).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
                    ui.set_min_width(220.0);
                    let li = self.layer.min(n - 1);
                    ui.strong(if li == 0 { "Base layer" } else { "Layer settings" });
                    if li > 0 {
                        ui.horizontal(|ui| {
                            ui.label("Name");
                            let mut name = self.project.as_ref().unwrap().slides[slide_idx].layers[li].name.clone();
                            let r = ui.text_edit_singleline(&mut name);
                            if r.gained_focus() { self.snapshot(); }
                            if r.changed() {
                                self.project.as_mut().unwrap().slides[slide_idx].layers[li].name = name;
                                self.dirty = true;
                            }
                        });
                    } else {
                        ui.weak("The base layer is named after the slide (rename the slide in Properties).");
                    }
                    let dur = self.project.as_ref().unwrap().slides[slide_idx].layers[li].duration_ms;
                    let mut timed = dur.is_some();
                    if ui.checkbox(&mut timed, if li == 0 { "Slide length" } else { "Layer length" }).on_hover_text("Length of the timeline").changed() {
                        self.snapshot();
                        self.project.as_mut().unwrap().slides[slide_idx].layers[li].duration_ms = if timed { Some(5000) } else { None };
                    }
                    if let Some(mut d) = self.project.as_ref().unwrap().slides[slide_idx].layers[li].duration_ms {
                        if ui.add(egui::DragValue::new(&mut d).range(250..=3_600_000).speed(50.0).suffix(" ms")).changed() {
                            self.project.as_mut().unwrap().slides[slide_idx].layers[li].duration_ms = Some(d);
                            self.dirty = true;
                        }
                    }
                });
            });
        });
    }

    /// Move layer `from` to index `to` (both above the base layer), keeping the selection and visibility with it.
    fn move_layer(&mut self, from: usize, to: usize) {
        if from == 0 || to == 0 || from == to || self.project.is_none() { return; }
        let n = self.project.as_ref().unwrap().slides[self.slide].layers.len();
        if from >= n || to >= n { return; }
        self.snapshot();
        let slide = &mut self.project.as_mut().unwrap().slides[self.slide];
        let l = slide.layers.remove(from);
        slide.layers.insert(to, l);
        if self.visible.len() == n {
            let v = self.visible.remove(from);
            self.visible.insert(to, v);
        }
        self.layer = to;
        self.selected = None;
        self.dirty = true;
    }

    /// Copy a layer (above the original), giving the copy fresh ids; triggers owned by the layer or its objects are copied too.
    fn duplicate_layer(&mut self, li: usize) {
        if li == 0 || self.project.is_none() { return; }
        let Some(mut copy) = self.project.as_ref().and_then(|p| p.slides[self.slide].layers.get(li).cloned()) else { return };
        let mut map: HashMap<String, String> = HashMap::new();
        let nid = self.new_id("layer");
        map.insert(std::mem::replace(&mut copy.id, nid), copy.id.clone());
        copy.name = format!("{} copy", copy.name);
        for o in copy.objects.iter_mut() {
            let nid = self.new_id("obj");
            map.insert(std::mem::replace(&mut o.id, nid), o.id.clone());
        }
        self.snapshot();
        let counter = self.id_counter;
        let slide = &mut self.project.as_mut().unwrap().slides[self.slide];
        let extra: Vec<Trigger> = slide.triggers.iter().filter_map(|t| {
            let owner = match &t.owner {
                Owner::Layer(l) => Owner::Layer(map.get(l)?.clone()),
                Owner::Object(o) => Owner::Object(map.get(o)?.clone()),
                Owner::Slide => return None,
            };
            let mut c = t.clone();
            c.id = format!("{}-c{}", t.id, counter);
            c.owner = owner;
            c.event = t.event.map_objects(&|o| map.get(o).cloned().unwrap_or_else(|| o.to_string()));
            c.action = match &t.action {
                a => a.map_objects(&|o| map.get(o).cloned().unwrap_or_else(|| o.to_string())),
            };
            Some(c)
        }).collect();
        slide.triggers.extend(extra);
        slide.layers.insert(li + 1, copy);
        self.layer = li + 1;
        self.selected = None;
        self.reset_visibility_keep();
    }

    #[allow(clippy::too_many_arguments)]
    /// Paint the given layers of `slide` (the same drawing the canvas and the thumbnails use).
    fn paint_slide(&mut self, ctx: &egui::Context, painter: &egui::Painter, origin: egui::Pos2, scale: f32, slide: &Slide, shown: &[usize], selected: Option<(usize, usize)>) {
        // Slide number text (`%Slide.Number%`) shows the slide's real number, as the published lesson will.
        let numbers = self.slide_numbers(slide);
        for &li in shown {
            match numbers {
                Some(nt) if slide.layers[li].objects.iter().any(has_slide_tokens) => {
                    let objects: Vec<Object> = slide.layers[li].objects.iter().cloned().map(|mut o| { number_tokens(&mut o, nt); o }).collect();
                    self.paint_objects(ctx, painter, origin, scale, &objects, li, selected);
                }
                _ => self.paint_objects(ctx, painter, origin, scale, &slide.layers[li].objects, li, selected),
            }
        }
    }

    /// This slide's place in the lesson and the lesson's length, when the slide is in the project.
    fn slide_numbers(&self, slide: &Slide) -> Option<(usize, usize)> {
        let p = self.project.as_ref()?;
        Some((p.slides.iter().position(|s| s.id == slide.id)? + 1, p.slides.len()))
    }

    /// The slide's background (its own, else the layout's or master's) and the master/layout
    /// shapes behind its content.
    fn paint_backdrop(&mut self, ctx: &egui::Context, painter: &egui::Painter, origin: egui::Pos2, scale: f32, slide: &Slide) {
        let rect = egui::Rect::from_min_size(origin, egui::vec2(slide.width, slide.height) * scale);
        let Some(p) = self.project.as_ref() else { return };
        let bg = p.resolved_background(slide).cloned();
        let objects: Vec<Object> = p.backdrop(slide).into_iter().cloned().collect();
        painter.rect_filled(rect, 0.0, egui::Color32::WHITE);
        match bg {
            None => {}
            Some(Background::Solid(c)) => { painter.rect_filled(rect, 0.0, color(c)); }
            Some(Background::Gradient { angle, stops }) => paint_gradient(painter, rect, angle, &stops),
            Some(Background::Image { asset }) => {
                if let Some(t) = self.texture(ctx, &asset) {
                    // Cover: scale to fill, cropping the overflow evenly.
                    let [iw, ih] = t.size();
                    let (ra, rb) = (rect.width() / rect.height(), iw as f32 / ih.max(1) as f32);
                    let uv = if rb > ra {
                        let k = ra / rb;
                        egui::Rect::from_min_max(egui::pos2((1.0 - k) / 2.0, 0.0), egui::pos2((1.0 + k) / 2.0, 1.0))
                    } else {
                        let k = rb / ra;
                        egui::Rect::from_min_max(egui::pos2(0.0, (1.0 - k) / 2.0), egui::pos2(1.0, (1.0 + k) / 2.0))
                    };
                    painter.image(t.id(), rect, uv, egui::Color32::WHITE);
                }
            }
        }
        let mut objects = objects;
        if let Some(nt) = self.slide_numbers(slide) { for o in objects.iter_mut() { number_tokens(o, nt); } }
        self.paint_objects(ctx, painter, origin, scale, &objects, usize::MAX, None);
    }

    /// A marker's card (title in bold, then the description) with its pointer on the marker, where the published page
    /// puts it.
    fn paint_marker_card(&self, painter: &egui::Painter, origin: egui::Pos2, scale: f32, o: &Object) {
        let Some(m) = &o.marker else { return };
        let Some(slide) = self.project.as_ref().and_then(|p| p.slides.get(self.slide)) else { return };
        let (cw, ch) = (m.card_w.max(120.0), m.card_h.max(80.0));
        let (left, top, notch, above) = MarkerInfo::card_at((o.x, o.y, o.w, o.h), (slide.width, slide.height), (cw, ch));
        let card = egui::Rect::from_min_size(origin + egui::vec2(left, top) * scale, egui::vec2(cw, ch) * scale);
        let edge = egui::Stroke::new((1.5 * scale).max(1.0), egui::Color32::from_gray(74));
        painter.rect(card, 14.0 * scale, egui::Color32::WHITE, edge, egui::StrokeKind::Inside);
        // The pointer: a small notch in the edge facing the marker.
        let nx = card.left() + notch * scale;
        let (y0, y1) = if above { (card.bottom(), card.bottom() + 16.5 * scale) } else { (card.top(), card.top() - 16.5 * scale) };
        let (a, b, c) = (egui::pos2(nx - 20.0 * scale, y0), egui::pos2(nx, y1), egui::pos2(nx + 20.0 * scale, y0));
        painter.add(egui::Shape::convex_polygon(vec![a, b, c], egui::Color32::WHITE, egui::Stroke::NONE));
        painter.add(egui::Shape::line(vec![a, b, c], edge));
        let pad = egui::vec2(28.0, 18.0) * scale;
        let fam = font_family("Open Sans");
        let run = |text: &str, size: f32, bold: bool, align: egui::Align, wrap: f32| {
            let mut job = egui::text::LayoutJob::default();
            job.append(text, 0.0, egui::text::TextFormat { font_id: egui::FontId::new((size * scale).max(8.0), fam.clone()), color: egui::Color32::from_gray(17), coords: egui::epaint::text::VariationCoords::new([(b"wght", if bold { 700.0 } else { 400.0 })]), ..Default::default() });
            job.wrap.max_width = wrap;
            job.halign = align;
            painter.layout_job(job)
        };
        let inner = card.shrink2(pad);
        let mut y = inner.top();
        if !m.title.trim().is_empty() {
            let t = run(m.title.trim(), 40.0, true, egui::Align::Center, inner.width());
            painter.with_clip_rect(card).galley(egui::pos2(inner.center().x, y), t.clone(), egui::Color32::from_gray(17));
            y += t.size().y + 14.0 * scale;
        }
        let body = run(&m.description, 32.0, false, egui::Align::LEFT, inner.width());
        painter.with_clip_rect(card.shrink(2.0)).galley(egui::pos2(inner.left(), y), body, egui::Color32::from_gray(17));
    }

    /// Paint `objects` (layer `li` of a slide); `selected` outlines the chosen object.
    #[allow(clippy::too_many_arguments)]
    fn paint_objects(&mut self, ctx: &egui::Context, painter: &egui::Painter, origin: egui::Pos2, scale: f32, objects: &[Object], li: usize, selected: Option<(usize, usize)>) {
        let show_hidden = self.show_hidden;
        let to_screen = |x: f32, y: f32, w: f32, h: f32| {
            egui::Rect::from_min_size(origin + egui::vec2(x, y) * scale, egui::vec2(w, h) * scale)
        };
        for (oi, o) in objects.iter().enumerate() {
            let r = to_screen(o.x, o.y, o.w, o.h);
            if o.hidden {
                // Hidden objects are not part of the published lesson: outline only, and only on request.
                if !show_hidden && selected != Some((li, oi)) { continue; }
                painter.rect_stroke(r, 0.0, egui::Stroke::new(1.0, egui::Color32::from_gray(150)), egui::StrokeKind::Inside);
                if selected == Some((li, oi)) {
                    painter.rect_stroke(r, 0.0, egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 120, 255)), egui::StrokeKind::Outside);
                }
                continue;
            }
            if (o.rotation.abs() > 0.05 || matches!(o.kind, Kind::Slider(_)) || o.check.is_some()) && !matches!(o.kind, Kind::Hotspot) {
                self.paint_rotated(ctx, painter, r, o, scale);
                if selected == Some((li, oi)) {
                    let c = r.center();
                    let a = o.rotation.to_radians();
                    let pts: Vec<egui::Pos2> = rounded_rect_poly(r, 0.0).into_iter().map(|p| { let d = p - c; c + egui::vec2(d.x * a.cos() - d.y * a.sin(), d.x * a.sin() + d.y * a.cos()) }).collect();
                    painter.add(egui::Shape::closed_line(pts, egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 120, 255))));
                }
                continue;
            }
            let ellipse = matches!(o.kind, Kind::Ellipse);
            let boxless = matches!(o.kind, Kind::Path(_) | Kind::Line | Kind::LineUp);
            // Shadow, glow and reflection go behind the object and follow its outline. Soft edges fade the fill itself.
            let soft = if o.soft_edge > 0.0 && !matches!(o.kind, Kind::Image) { Some(o.soft_edge * scale) } else { None };
            if o.fill.is_some() || matches!(o.kind, Kind::Image) {
                if o.shadow.is_some() || o.glow.is_some() || o.reflection.is_some() {
                    effects_paint::behind(painter, o, r, scale, o.fill.map(color), &|p| p);
                }
            }
            if let (Some(g), false, true) = (&o.gradient, boxless, !g_is_empty(&o.gradient)) {
                if !ellipse && o.radius <= 0.5 { paint_gradient(painter, r, g.angle, &g.stops); }
                else {
                    let poly = if ellipse { ellipse_poly(r) } else { rounded_rect_poly(r, o.radius * scale) };
                    fill_polygon(painter, &poly, &|p| p, &|p| gradient_at(&g.stops, gradient_t(r, g.angle, p)));
                }
            } else if let (Some(f), false) = (o.fill, boxless) {
                if let Some(edge) = soft { effects_paint::soft_fill(painter, &effects_paint::figures(o, r, scale), r, color(f), edge, &|p| p); }
                else if ellipse { painter.add(egui::epaint::EllipseShape::filled(r.center(), r.size() / 2.0, color(f))); }
                else { painter.rect_filled(r, o.radius * scale, color(f)); }
            }
            if let Kind::Path(cmds) = &o.kind {
                let figs = path_polygons(cmds, r);
                if let (Some(g), true) = (&o.gradient, !g_is_empty(&o.gradient)) {
                    fill_figures(painter, &figs, &|p| p, &|p| gradient_at(&g.stops, gradient_t(r, g.angle, p)));
                } else if let (Some(f), Some(edge)) = (o.fill, soft) {
                    effects_paint::soft_fill(painter, &figs, r, color(f), edge, &|p| p);
                } else if let Some(f) = o.fill {
                    fill_figures(painter, &figs, &|p| p, &|_| color(f));
                }
                if let Some(st) = o.stroke {
                    for poly in &figs { painter.extend(outline_shapes(poly.clone(), egui::Stroke::new((st.width * scale).max(1.0), color(st.color)), o.dash)); }
                }
            }
            if matches!(o.kind, Kind::Line | Kind::LineUp) {
                let (a, b) = if matches!(o.kind, Kind::Line) { (r.left_top(), r.right_bottom()) } else { (r.left_bottom(), r.right_top()) };
                let st = o.stroke.unwrap_or(Stroke { color: Rgba(0, 0, 0, 255), width: 1.0 });
                painter.line_segment([a, b], egui::Stroke::new((st.width * scale).max(1.0), color(st.color)));
            }
            if matches!(o.kind, Kind::Hotspot) {
                // Invisible in the published lesson; shown here so it can be found and selected.
                let (tint, edge) = (egui::Color32::from_rgba_unmultiplied(0, 140, 255, 36), egui::Color32::from_rgba_unmultiplied(0, 140, 255, 150));
                match o.hit_area.as_ref().filter(|a| a.len() > 2) {
                    // An outlined hotspot is clickable only inside its outline.
                    Some(area) => {
                        let poly: Vec<egui::Pos2> = area.iter().map(|&(x, y)| egui::pos2(r.left() + x * r.width(), r.top() + y * r.height())).collect();
                        fill_polygon(painter, &poly, &|p| p, &|_| tint);
                        painter.add(egui::Shape::closed_line(poly, egui::Stroke::new(1.0, edge)));
                    }
                    None => {
                        painter.rect_filled(r, 0.0, tint);
                        painter.rect_stroke(r, 0.0, egui::Stroke::new(1.0, edge), egui::StrokeKind::Inside);
                    }
                }
            }
            if let Kind::Input(f) = &o.kind {
                // The field's text (or its faint prompt), and the step arrows of a number field.
                let pad = (14.0 * scale).max(4.0);
                let start = self.project.as_ref().and_then(|p| p.variables.iter().find(|v| v.id == f.variable)).map(|v| match &v.initial { Value::Text(t) => t.clone(), Value::Num(n) => format!("{n}"), Value::Bool(b) => b.to_string() });
                let (txt, faint) = match start.filter(|t| !t.is_empty()) {
                    Some(t) => (t, false),
                    None => (if f.numeric { "0".to_string() } else { f.placeholder.clone() }, true),
                };
                let font = egui::FontId::new((o.style.size * scale).max(8.0), font_family(&o.style.font_family));
                let c = color(o.style.color);
                painter.with_clip_rect(r).text(egui::pos2(r.left() + pad, r.center().y), egui::Align2::LEFT_CENTER, txt, font, if faint { c.gamma_multiply(0.45) } else { c });
                if f.numeric {
                    let strip = egui::Rect::from_min_max(egui::pos2(r.right() - (36.0 * scale).max(14.0), r.top()), r.max);
                    painter.line_segment([strip.left_top(), strip.left_bottom()], egui::Stroke::new(1.0, egui::Color32::from_gray(190)));
                    painter.line_segment([strip.left_center(), strip.right_center()], egui::Stroke::new(1.0, egui::Color32::from_gray(190)));
                    let w = strip.width() * 0.22;
                    let up = strip.center() - egui::vec2(0.0, strip.height() / 4.0);
                    let down = strip.center() + egui::vec2(0.0, strip.height() / 4.0);
                    painter.add(egui::Shape::convex_polygon(vec![up + egui::vec2(-w, w * 0.6), up + egui::vec2(w, w * 0.6), up - egui::vec2(0.0, w * 0.6)], egui::Color32::from_gray(110), egui::Stroke::NONE));
                    painter.add(egui::Shape::convex_polygon(vec![down + egui::vec2(-w, -w * 0.6), down + egui::vec2(w, -w * 0.6), down + egui::vec2(0.0, w * 0.6)], egui::Color32::from_gray(110), egui::Stroke::NONE));
                }
            }
            if let Kind::Web(w) = &o.kind {
                // The page itself only shows when published; here a frame with its address.
                painter.rect_filled(r, 4.0, egui::Color32::from_gray(245));
                painter.rect_stroke(r, 4.0, egui::Stroke::new(1.5, egui::Color32::from_gray(150)), egui::StrokeKind::Inside);
                let bar = egui::Rect::from_min_size(r.min, egui::vec2(r.width(), (36.0 * scale).max(14.0)));
                painter.rect_filled(bar, 4.0, egui::Color32::from_gray(225));
                painter.text(bar.left_center() + egui::vec2(8.0 * scale.max(0.5), 0.0), egui::Align2::LEFT_CENTER, if w.url.trim().is_empty() { "No web address yet" } else { w.url.trim() }, egui::FontId::proportional((18.0 * scale).clamp(9.0, 22.0)), egui::Color32::from_gray(70));
                painter.text(r.center(), egui::Align2::CENTER_CENTER, ribbon::GLOBE.to_string(), egui::FontId::new((r.height().min(r.width()) * 0.3).clamp(14.0, 96.0), ribbon::icons()), egui::Color32::from_gray(175));
            }
            if let Kind::Pano(p) = &o.kind { self.paint_pano(ctx, painter, o, p, r, &|p| p); }
            if matches!(o.kind, Kind::Video | Kind::Audio) {
                painter.rect_filled(r, 4.0, egui::Color32::from_gray(35));
                // A video shows its still picture when the file has one; otherwise a play mark and its name.
                let poster = if matches!(o.kind, Kind::Video) { o.poster.as_ref().and_then(|k| self.texture(ctx, k)) } else { None };
                if let Some(t) = &poster {
                    painter.image(t.id(), r, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                }
                if matches!(o.kind, Kind::Video) {
                    let d = (r.height().min(r.width()) * 0.16).clamp(10.0, 48.0);
                    let c = r.center();
                    painter.add(egui::Shape::convex_polygon(vec![c + egui::vec2(-d * 0.35, -d * 0.5), c + egui::vec2(-d * 0.35, d * 0.5), c + egui::vec2(d * 0.6, 0.0)], egui::Color32::from_white_alpha(if poster.is_some() { 190 } else { 150 }), egui::Stroke::NONE));
                }
                if poster.is_none() {
                    let label = format!("{}  {}", if matches!(o.kind, Kind::Video) { "VIDEO" } else { "AUDIO" }, o.name);
                    let at = if matches!(o.kind, Kind::Video) { r.center_bottom() - egui::vec2(0.0, 14.0 * scale.max(0.5)) } else { r.center() };
                    painter.text(at, if matches!(o.kind, Kind::Video) { egui::Align2::CENTER_BOTTOM } else { egui::Align2::CENTER_CENTER }, label, egui::FontId::proportional((20.0 * scale).clamp(11.0, 32.0)), egui::Color32::from_gray(210));
                }
            }
            if let (Kind::Image, Some(key)) = (&o.kind, &o.asset) {
                if let Some(t) = self.texture(ctx, key) {
                    if let Some((dest, uv)) = crop_uv(r, o.crop) {
                        match o.mask {
                            Some(shape) => { painter.add(egui::Shape::mesh(shaped_picture(t.id(), r, dest, uv, shape, (o.w, o.h), &|p| p))); }
                            None => { painter.image(t.id(), dest, uv, egui::Color32::WHITE); }
                        }
                    }
                }
            }
            if !o.text.is_empty() {
                // Text area = frame minus margins; weight comes from the variable font's wght axis.
                let tr = egui::Rect::from_min_max(
                    r.min + egui::vec2(o.margins[0], o.margins[1]) * scale,
                    r.max - egui::vec2(o.margins[2], o.margins[3]) * scale,
                );
                let job = text_job(o, scale, if o.text_rotation != 0 { tr.height().max(1.0) } else { tr.width().max(1.0) }, self.link_color());
                let galley = layout_indented(&|j| painter.layout_job(j), job, &indent_px(o, scale), ctx.pixels_per_point(), true);
                if o.text_rotation != 0 {
                    let gh = galley.size().y;
                    let across = |free: f32| match o.style.valign { VAlign::Top => 0.0, VAlign::Middle => free / 2.0, VAlign::Bottom => free };
                    // egui anchors left/center/right text at its start/middle/end along the text direction.
                    let (pos, angle) = if o.text_rotation == 270 {
                        // reads bottom-to-top: starts at the bottom, its top edge faces left
                        let ay = match o.style.align { Align::Left | Align::Justify => tr.bottom(), Align::Center => tr.center().y, Align::Right => tr.top() };
                        (egui::pos2(tr.left() + across(tr.width() - gh), ay), -std::f32::consts::FRAC_PI_2)
                    } else {
                        let ay = match o.style.align { Align::Left | Align::Justify => tr.top(), Align::Center => tr.center().y, Align::Right => tr.bottom() };
                        (egui::pos2(tr.right() - across(tr.width() - gh), ay), std::f32::consts::FRAC_PI_2)
                    };
                    painter.with_clip_rect(r).add(egui::epaint::TextShape::new(pos, galley, color(o.style.color)).with_angle(angle));
                } else {
                let x = match o.style.align {
                    Align::Left | Align::Justify => tr.left(),
                    Align::Center => tr.center().x,
                    Align::Right => tr.right(),
                };
                // Text taller than its box overflows away from its anchor (top: down, middle: both, bottom: up) and is not clipped.
                let slack = tr.height() - galley.size().y;
                let y = tr.top() + match o.style.valign { VAlign::Top => 0.0, VAlign::Middle => slack / 2.0, VAlign::Bottom => slack };
                painter.galley(egui::pos2(x, y), galley, color(o.style.color));
                }
            }
            if let (true, Some(k), true) = (o.text.is_empty(), o.placeholder, scale > 0.25) {
                // An empty placeholder: outline and prompt while editing (never published).
                painter.rect_stroke(r, 0.0, egui::Stroke::new(1.0, egui::Color32::from_gray(170)), egui::StrokeKind::Inside);
                let at = r.left_top() + egui::vec2(o.margins[0], o.margins[1]) * scale;
                painter.with_clip_rect(r).text(at, egui::Align2::LEFT_TOP, k.prompt(), egui::FontId::new((o.style.size * scale).max(8.0), font_family(&o.style.font_family)), color(o.style.color).gamma_multiply(0.5));
            }
            if let (Some(b), false) = (o.table_borders, boxless) {
                // A table cell with its own border on each side.
                for (k, side) in b.iter().enumerate() {
                    let Some(st) = side else { continue };
                    let w = (st.width * scale).max(1.0);
                    let (a, z) = match k {
                        0 => (r.left_top() + egui::vec2(w / 2.0, 0.0), r.left_bottom() + egui::vec2(w / 2.0, 0.0)),
                        1 => (r.right_top() - egui::vec2(w / 2.0, 0.0), r.right_bottom() - egui::vec2(w / 2.0, 0.0)),
                        2 => (r.left_top() + egui::vec2(0.0, w / 2.0), r.right_top() + egui::vec2(0.0, w / 2.0)),
                        _ => (r.left_bottom() - egui::vec2(0.0, w / 2.0), r.right_bottom() - egui::vec2(0.0, w / 2.0)),
                    };
                    painter.line_segment([a, z], egui::Stroke::new(w, color(st.color)));
                }
            } else if let (Some(st), false) = (o.stroke, boxless) {
                let stroke = egui::Stroke::new((st.width * scale).max(1.0), color(st.color));
                if o.dash != brainbend_doc::LineDash::Solid {
                    let inner = r.shrink(stroke.width / 2.0);
                    let poly = if ellipse { ellipse_poly(inner) } else { rounded_rect_poly(inner, o.radius * scale) };
                    painter.extend(outline_shapes(poly, stroke, o.dash));
                } else if ellipse { painter.add(egui::epaint::EllipseShape::stroke(r.center(), r.size() / 2.0, stroke)); }
                else { painter.rect_stroke(r, o.radius * scale, stroke, egui::StrokeKind::Inside); }
            }
            if selected == Some((li, oi)) {
                painter.rect_stroke(r, 0.0, egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 120, 255)), egui::StrokeKind::Outside);
            }
        }
        // The selected marker's card, as the learner will see it when they click the marker.
        if let Some((sl, so)) = selected.filter(|(l, _)| *l == li) {
            let _ = sl;
            if let Some(o) = objects.get(so).filter(|o| o.marker.is_some() && !o.hidden) { self.paint_marker_card(painter, origin, scale, o); }
        }
    }

    /// Draw an object turned by `o.rotation` about its centre. Everything becomes polygons, so only the
    /// basics are supported: fills (flat or gradient), outlines, pictures and text.
    fn paint_rotated(&mut self, ctx: &egui::Context, painter: &egui::Painter, r: egui::Rect, o: &Object, scale: f32) {
        let c = r.center();
        let a = o.rotation.to_radians();
        let rot = move |p: egui::Pos2| { let d = p - c; c + egui::vec2(d.x * a.cos() - d.y * a.sin(), d.x * a.sin() + d.y * a.cos()) };
        let flat = o.fill.map(color);
        let shade = |p: egui::Pos2| match (&o.gradient, flat) {
            (Some(g), _) if !g.stops.is_empty() => gradient_at(&g.stops, gradient_t(r, g.angle, p)),
            (_, Some(f)) => f,
            _ => egui::Color32::TRANSPARENT,
        };
        let has_fill = o.fill.is_some() || o.gradient.as_ref().is_some_and(|g| !g.stops.is_empty());
        let stroke = o.stroke.map(|st| egui::Stroke::new((st.width * scale).max(1.0), color(st.color)));
        let soft = if o.soft_edge > 0.0 { Some(o.soft_edge * scale) } else { None };
        if (o.fill.is_some() || matches!(o.kind, Kind::Image)) && (o.shadow.is_some() || o.glow.is_some() || o.reflection.is_some()) {
            effects_paint::behind(painter, o, r, scale, o.fill.map(color), &rot);
        }
        match &o.kind {
            Kind::Path(cmds) => {
                let figs = path_polygons(cmds, r);
                if let (Some(f), Some(edge), true) = (o.fill, soft, o.gradient.is_none()) { effects_paint::soft_fill(painter, &figs, r, color(f), edge, &rot); }
                else if has_fill { fill_figures(painter, &figs, &rot, &shade); }
                if let Some(st) = stroke { for poly in &figs { painter.extend(outline_shapes(poly.iter().map(|p| rot(*p)).collect(), st, o.dash)); } }
            }
            Kind::Slider(sl) => {
                // The track, and the thumb where its variable starts.
                let start = self.project.as_ref().and_then(|p| p.variables.iter().find(|v| v.id == sl.variable)).and_then(|v| if let Value::Num(n) = v.initial { Some(n as f32) } else { None }).unwrap_or(sl.min);
                let th = (r.height() * sl.thumb_height).max(4.0);
                let tex = sl.thumb.as_deref().and_then(|k| self.texture(ctx, k));
                let tw = tex.as_ref().map(|t| { let [w, h] = t.size(); th * w as f32 / h.max(1) as f32 }).unwrap_or(th);
                let left = r.left() + sl.fraction(start) * (r.width() - tw).max(0.0);
                let thumb = egui::Rect::from_min_size(egui::pos2(left, r.center().y - th / 2.0), egui::vec2(tw, th));
                if let Some(t) = sl.track {
                    let y = r.center().y;
                    painter.line_segment([rot(egui::pos2(r.left(), y)), rot(egui::pos2(r.right(), y))], egui::Stroke::new((4.0 * scale).max(2.0), color(t)));
                }
                match tex {
                    Some(t) => {
                        let mut mesh = egui::epaint::Mesh::with_texture(t.id());
                        let corners = [(thumb.left_top(), egui::pos2(0.0, 0.0)), (thumb.right_top(), egui::pos2(1.0, 0.0)), (thumb.right_bottom(), egui::pos2(1.0, 1.0)), (thumb.left_bottom(), egui::pos2(0.0, 1.0))];
                        for (p, uv) in corners { mesh.vertices.push(egui::epaint::Vertex { pos: rot(p), uv, color: egui::Color32::WHITE }); }
                        mesh.add_triangle(0, 1, 2);
                        mesh.add_triangle(0, 2, 3);
                        painter.add(egui::Shape::mesh(mesh));
                    }
                    None => fill_polygon(painter, &ellipse_poly(thumb), &rot, &|_| color(sl.thumb_color)),
                }
            }
            Kind::Line | Kind::LineUp => {
                let (p, q) = if matches!(o.kind, Kind::Line) { (r.left_top(), r.right_bottom()) } else { (r.left_bottom(), r.right_top()) };
                let st = stroke.unwrap_or(egui::Stroke::new(1.0, egui::Color32::BLACK));
                painter.line_segment([rot(p), rot(q)], st);
            }
            Kind::Image => {
                if let Some(key) = &o.asset {
                    if let (Some(t), Some((d, uv)), Some(shape)) = (self.texture(ctx, key), crop_uv(r, o.crop), o.mask) {
                        painter.add(egui::Shape::mesh(shaped_picture(t.id(), r, d, uv, shape, (o.w, o.h), &rot)));
                    } else if let (Some(t), Some((d, uv))) = (self.texture(ctx, key), crop_uv(r, o.crop)) {
                        let mut mesh = egui::epaint::Mesh::with_texture(t.id());
                        let corners = [(d.left_top(), uv.left_top()), (d.right_top(), uv.right_top()), (d.right_bottom(), uv.right_bottom()), (d.left_bottom(), uv.left_bottom())];
                        for (p, uv) in corners { mesh.vertices.push(egui::epaint::Vertex { pos: rot(p), uv, color: egui::Color32::WHITE }); }
                        mesh.add_triangle(0, 1, 2);
                        mesh.add_triangle(0, 2, 3);
                        painter.add(egui::Shape::mesh(mesh));
                    }
                }
                if let Some(st) = stroke { painter.extend(outline_shapes(rounded_rect_poly(r, 0.0).iter().map(|p| rot(*p)).collect(), st, o.dash)); }
            }
            Kind::Pano(p) => self.paint_pano(ctx, painter, o, p, r, &rot),
            Kind::Video | Kind::Audio => {
                fill_polygon(painter, &rounded_rect_poly(r, 4.0), &rot, &|_| egui::Color32::from_gray(35));
            }
            _ => {
                let poly = if matches!(o.kind, Kind::Ellipse) { ellipse_poly(r) } else { rounded_rect_poly(r, o.radius * scale) };
                if let (Some(f), Some(edge), true) = (o.fill, soft, o.gradient.is_none()) { effects_paint::soft_fill(painter, &vec![poly.clone()], r, color(f), edge, &rot); }
                else if has_fill { fill_polygon(painter, &poly, &rot, &shade); }
                if let Some(st) = stroke { painter.extend(outline_shapes(poly.iter().map(|p| rot(*p)).collect(), st, o.dash)); }
            }
        }
        if let Some(ck) = &o.check {
            // The box (a circle for a radio button), with a tick (a dot) when the object starts in its Selected state.
            let (bx, by, side) = ck.box_in(o.h);
            let b = egui::Rect::from_min_size(r.min + egui::vec2(bx, by) * scale, egui::vec2(side, side) * scale);
            let radio = ck.radio.is_some();
            let poly = if radio { ellipse_poly(b) } else { rounded_rect_poly(b, 3.0 * scale) };
            fill_polygon(painter, &poly, &rot, &|_| color(ck.box_fill));
            if let Some(l) = ck.box_line { painter.add(egui::Shape::closed_line(poly.iter().map(|p| rot(*p)).collect(), egui::Stroke::new((scale).max(1.0), color(l)))); }
            if o.initial_state == "Selected" {
                if radio {
                    let dot = ellipse_poly(egui::Rect::from_center_size(b.center(), b.size() * 0.5));
                    fill_polygon(painter, &dot, &rot, &|_| color(ck.tick));
                } else {
                    let at = |x: f32, y: f32| rot(b.min + egui::vec2(x, y) * b.width() / 20.0);
                    painter.add(egui::Shape::line(vec![at(4.5, 10.5), at(8.0, 14.0), at(15.5, 6.0)], egui::Stroke::new((2.2 * b.width() / 20.0).max(1.5), color(ck.tick))));
                }
            }
        }
        if !o.text.is_empty() {
            let tr = egui::Rect::from_min_max(r.min + egui::vec2(o.margins[0], o.margins[1]) * scale, r.max - egui::vec2(o.margins[2], o.margins[3]) * scale);
            let job = text_job(o, scale, tr.width().max(1.0), self.link_color());
            let galley = layout_indented(&|j| painter.layout_job(j), job, &indent_px(o, scale), ctx.pixels_per_point(), true);
            let x = match o.style.align { Align::Left | Align::Justify => tr.left(), Align::Center => tr.center().x, Align::Right => tr.right() };
            let slack = tr.height() - galley.size().y;
            let y = tr.top() + match o.style.valign { VAlign::Top => 0.0, VAlign::Middle => slack / 2.0, VAlign::Bottom => slack };
            painter.add(egui::epaint::TextShape::new(rot(egui::pos2(x, y)), galley, color(o.style.color)).with_angle(a));
        }
    }

    fn texture(&mut self, ctx: &egui::Context, key: &str) -> Option<egui::TextureHandle> {
        if let Some(t) = self.textures.get(key) {
            return Some(t.clone());
        }
        let asset = self.project.as_ref()?.assets.get(key)?;
        let mut img = image::load_from_memory(&asset.bytes).ok()?;
        // Photos can be larger than the GPU allows in one texture; shrink those for display only.
        let max = ctx.input(|i| i.max_texture_side) as u32;
        if img.width() > max || img.height() > max { img = img.resize(max, max, image::imageops::FilterType::Triangle); }
        let img = img.to_rgba8();
        let size = [img.width() as usize, img.height() as usize];
        let tex = ctx.load_texture(key, egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()), Default::default());
        self.textures.insert(key.to_string(), tex.clone());
        Some(tex)
    }
}

/// Which kind of file a question editor is asking the app to pick.
#[derive(Clone, Copy, PartialEq)]
enum AssetWant {
    Image,
    Audio,
}

/// Where the question sits on its slide: the whole slide (the original look) or an area of the author's choosing,
/// which leaves room for the slide's own shapes, pictures, buttons and videos.
fn question_area_editor(ui: &mut egui::Ui, q: &mut Question, slide_w: f32, slide_h: f32) -> bool {
    let mut changed = false;
    ui.separator();
    let mut own = q.area.is_some();
    if ui.checkbox(&mut own, "Place the question in its own area").on_hover_text("Off: the question fills the slide on a pale ground. On: the slide's background and objects show around it.").changed() {
        q.area = own.then(|| QuestionArea::default_for(slide_w, slide_h));
        changed = true;
    }
    if let Some(a) = q.area.as_mut() {
        ui.horizontal(|ui| {
            for (label, v, max) in [("X", &mut a.x, slide_w), ("Y", &mut a.y, slide_h), ("W", &mut a.w, slide_w), ("H", &mut a.h, slide_h)] {
                ui.label(label);
                changed |= ui.add(egui::DragValue::new(v).range(0.0..=max * 2.0).speed(2.0).suffix(" px")).changed();
            }
        });
        a.w = a.w.max(200.0);
        a.h = a.h.max(120.0);
    }
    changed
}

fn question_editor(ui: &mut egui::Ui, q: &mut Question, new_id: &mut dyn FnMut() -> String, want: &mut Option<AssetWant>, number_vars: &[(String, String)]) -> bool {
    let mut changed = false;
    ui.label("Question");
    changed |= ui.add(egui::TextEdit::multiline(&mut q.prompt).desired_rows(3).desired_width(f32::INFINITY)).changed();
    let kind_label = match &q.style {
        QuestionStyle::Choice => if q.multiple { "Multiple response" } else { "Multiple choice" },
        QuestionStyle::TextEntry { .. } => "Fill in the blank",
        QuestionStyle::Matching { .. } => "Matching",
        QuestionStyle::Sequence { .. } => "Sequencing",
        QuestionStyle::FillBlanks { .. } => "Fill in the blanks",
        QuestionStyle::DragWords { .. } => "Drag the words",
        QuestionStyle::MarkWords { .. } => "Mark the words",
        QuestionStyle::Numeric { .. } => "Number",
        QuestionStyle::Categorize { .. } => "Sort into groups",
        QuestionStyle::Hotspot { .. } => "Find the hotspot",
        QuestionStyle::Likert { .. } => "Survey scale (not graded)",
        QuestionStyle::Reflection { .. } => "Free text (not graded)",
        QuestionStyle::Dictation { .. } => "Dictation",
        QuestionStyle::Dropdowns { .. } => "Choose from lists",
        QuestionStyle::LabelPicture { .. } => "Label the picture",
    };
    ui.horizontal(|ui| {
        ui.label("Type");
        combo("qstyle").selected_text(kind_label).show_ui(ui, |ui| {
            if ui.selectable_label(matches!(q.style, QuestionStyle::Choice) && !q.multiple, "Multiple choice").clicked() {
                q.style = QuestionStyle::Choice; q.multiple = false;
                if q.choices.len() < 2 { q.choices = Question::new_single().choices; }
                changed = true;
            }
            if ui.selectable_label(matches!(q.style, QuestionStyle::Choice) && q.multiple, "Multiple response").clicked() {
                q.style = QuestionStyle::Choice; q.multiple = true;
                if q.choices.len() < 2 { q.choices = Question::new_single().choices; }
                changed = true;
            }
            if ui.selectable_label(matches!(q.style, QuestionStyle::TextEntry { .. }), "Fill in the blank").clicked() && !matches!(q.style, QuestionStyle::TextEntry { .. }) {
                q.style = Question::new_text_entry().style; changed = true;
            }
            if ui.selectable_label(matches!(q.style, QuestionStyle::Matching { .. }), "Matching").clicked() && !matches!(q.style, QuestionStyle::Matching { .. }) {
                q.style = Question::new_matching().style; changed = true;
            }
            if ui.selectable_label(matches!(q.style, QuestionStyle::Sequence { .. }), "Sequencing").clicked() && !matches!(q.style, QuestionStyle::Sequence { .. }) {
                q.style = Question::new_sequence().style; changed = true;
            }
            if ui.selectable_label(matches!(q.style, QuestionStyle::FillBlanks { .. }), "Fill in the blanks").clicked() && !matches!(q.style, QuestionStyle::FillBlanks { .. }) {
                q.style = Question::new_fill_blanks().style; changed = true;
            }
            if ui.selectable_label(matches!(q.style, QuestionStyle::DragWords { .. }), "Drag the words").clicked() && !matches!(q.style, QuestionStyle::DragWords { .. }) {
                q.style = Question::new_drag_words().style; changed = true;
            }
            if ui.selectable_label(matches!(q.style, QuestionStyle::MarkWords { .. }), "Mark the words").clicked() && !matches!(q.style, QuestionStyle::MarkWords { .. }) {
                q.style = Question::new_mark_words().style; changed = true;
            }
            if ui.selectable_label(matches!(q.style, QuestionStyle::Numeric { .. }), "Number").clicked() && !matches!(q.style, QuestionStyle::Numeric { .. }) {
                q.style = Question::new_numeric().style; changed = true;
            }
            if ui.selectable_label(matches!(q.style, QuestionStyle::Categorize { .. }), "Sort into groups").clicked() && !matches!(q.style, QuestionStyle::Categorize { .. }) {
                q.style = Question::new_categorize().style; changed = true;
            }
            if ui.selectable_label(matches!(q.style, QuestionStyle::Hotspot { .. }), "Find the hotspot").clicked() && !matches!(q.style, QuestionStyle::Hotspot { .. }) {
                q.style = Question::new_hotspot().style; changed = true;
            }
            if ui.selectable_label(matches!(q.style, QuestionStyle::LabelPicture { .. }), "Label the picture").clicked() && !matches!(q.style, QuestionStyle::LabelPicture { .. }) {
                q.style = Question::new_label_picture().style; changed = true;
            }
            if ui.selectable_label(matches!(q.style, QuestionStyle::Dropdowns { .. }), "Choose from lists").clicked() && !matches!(q.style, QuestionStyle::Dropdowns { .. }) {
                q.style = Question::new_dropdowns().style; changed = true;
            }
            if ui.selectable_label(matches!(q.style, QuestionStyle::Dictation { .. }), "Dictation").clicked() && !matches!(q.style, QuestionStyle::Dictation { .. }) {
                q.style = Question::new_dictation().style; changed = true;
            }
            if ui.selectable_label(matches!(q.style, QuestionStyle::Likert { .. }), "Survey scale (not graded)").clicked() && !matches!(q.style, QuestionStyle::Likert { .. }) {
                let keep = Question::new_likert();
                q.style = keep.style; q.points = 0.0; q.feedback_correct = keep.feedback_correct; q.feedback_incorrect = keep.feedback_incorrect; changed = true;
            }
            if ui.selectable_label(matches!(q.style, QuestionStyle::Reflection { .. }), "Free text (not graded)").clicked() && !matches!(q.style, QuestionStyle::Reflection { .. }) {
                let keep = Question::new_reflection();
                q.style = keep.style; q.points = 0.0; q.feedback_correct = keep.feedback_correct; q.feedback_incorrect = keep.feedback_incorrect; changed = true;
            }
        });
        ui.label("Points");
        changed |= ui.add(egui::DragValue::new(&mut q.points).range(0.0..=100.0).speed(0.1)).changed();
    });
    let multiple = q.multiple;
    match &mut q.style {
        QuestionStyle::Choice => {
            ui.label(if multiple { "Choices (tick every correct one)" } else { "Choices (tick the correct one)" });
            let mut remove = None;
            for i in 0..q.choices.len() {
                ui.horizontal(|ui| {
                    let mut ok = q.choices[i].correct;
                    if ui.checkbox(&mut ok, "").on_hover_text("Correct answer").changed() {
                        if !multiple && ok {
                            for c in q.choices.iter_mut() { c.correct = false; }
                        }
                        q.choices[i].correct = ok;
                        changed = true;
                    }
                    changed |= ui.add(egui::TextEdit::singleline(&mut q.choices[i].text).desired_width(150.0)).changed();
                    if q.choices.len() > 2 && ui.small_button("x").clicked() { remove = Some(i); }
                });
            }
            if let Some(i) = remove { q.choices.remove(i); changed = true; }
            if ui.button("+ Add choice").clicked() {
                q.choices.push(Choice { id: new_id(), text: "New choice".into(), correct: false });
                changed = true;
            }
            if !q.choices.iter().any(|c| c.correct) {
                ui.colored_label(egui::Color32::from_rgb(200, 60, 40), "No correct answer is marked.");
            }
        }
        QuestionStyle::TextEntry { accepted, case_sensitive } => {
            ui.label("Accepted answers (any one is correct)");
            let mut remove = None;
            for (i, a) in accepted.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    changed |= ui.add(egui::TextEdit::singleline(a).desired_width(190.0)).changed();
                    if ui.small_button("x").clicked() { remove = Some(i); }
                });
            }
            if let Some(i) = remove { accepted.remove(i); changed = true; }
            if ui.button("+ Add answer").clicked() { accepted.push(String::new()); changed = true; }
            changed |= ui.checkbox(case_sensitive, "Match upper/lower case").changed();
            if accepted.iter().all(|a| a.trim().is_empty()) {
                ui.colored_label(egui::Color32::from_rgb(200, 60, 40), "Add at least one accepted answer.");
            }
        }
        QuestionStyle::Matching { pairs } => {
            ui.label("Pairs (left item, matching right item)");
            let mut remove = None;
            for (i, (l, r)) in pairs.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    changed |= ui.add(egui::TextEdit::singleline(l).desired_width(90.0)).changed();
                    ui.label("→");
                    changed |= ui.add(egui::TextEdit::singleline(r).desired_width(90.0)).changed();
                    if ui.small_button("x").clicked() { remove = Some(i); }
                });
            }
            if let Some(i) = remove { pairs.remove(i); changed = true; }
            if ui.button("+ Add pair").clicked() { pairs.push((String::new(), String::new())); changed = true; }
            if pairs.len() < 2 { ui.colored_label(egui::Color32::from_rgb(200, 60, 40), "Needs at least two pairs."); }
        }
        QuestionStyle::Sequence { items } => {
            ui.label("Items, in the correct order (learners see them shuffled)");
            let (mut remove, mut up) = (None, None);
            for (i, it) in items.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("{}.", i + 1));
                    changed |= ui.add(egui::TextEdit::singleline(it).desired_width(130.0)).changed();
                    if ui.small_button("up").clicked() && i > 0 { up = Some(i); }
                    if ui.small_button("x").clicked() { remove = Some(i); }
                });
            }
            if let Some(i) = up { items.swap(i, i - 1); changed = true; }
            if let Some(i) = remove { items.remove(i); changed = true; }
            if ui.button("+ Add item").clicked() { items.push(String::new()); changed = true; }
            if items.len() < 2 { ui.colored_label(egui::Color32::from_rgb(200, 60, 40), "Needs at least two items."); }
        }
        QuestionStyle::FillBlanks { text, case_sensitive } => {
            ui.label("Passage (put each answer in *stars*; *a/b* accepts either)");
            changed |= ui.add(egui::TextEdit::multiline(text).desired_rows(5).desired_width(f32::INFINITY)).changed();
            changed |= ui.checkbox(case_sensitive, "Match upper/lower case").changed();
            let n = brainbend_doc::passage::blanks(text).len();
            if n == 0 { ui.colored_label(egui::Color32::from_rgb(200, 60, 40), "Wrap at least one answer in *stars*."); } else { ui.weak(format!("{n} blank{}", if n == 1 { "" } else { "s" })); }
        }
        QuestionStyle::DragWords { text, distractors } => {
            ui.label("Passage (put each answer in *stars*)");
            changed |= ui.add(egui::TextEdit::multiline(text).desired_rows(5).desired_width(f32::INFINITY)).changed();
            ui.label("Spare words that belong in no blank");
            let mut remove = None;
            for (i, d) in distractors.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    changed |= ui.add(egui::TextEdit::singleline(d).desired_width(150.0)).changed();
                    if ui.small_button("x").clicked() { remove = Some(i); }
                });
            }
            if let Some(i) = remove { distractors.remove(i); changed = true; }
            if ui.button("+ Add spare word").clicked() { distractors.push(String::new()); changed = true; }
            if brainbend_doc::passage::blanks(text).is_empty() { ui.colored_label(egui::Color32::from_rgb(200, 60, 40), "Wrap at least one answer in *stars*."); }
        }
        QuestionStyle::MarkWords { text } => {
            ui.label("Passage (put each correct word in *stars*)");
            changed |= ui.add(egui::TextEdit::multiline(text).desired_rows(5).desired_width(f32::INFINITY)).changed();
            if brainbend_doc::passage::blanks(text).is_empty() { ui.colored_label(egui::Color32::from_rgb(200, 60, 40), "Mark at least one word with *stars*."); }
        }
        QuestionStyle::Numeric { answer, tolerance } => {
            ui.horizontal(|ui| {
                ui.label("Answer");
                changed |= ui.add(egui::DragValue::new(answer).speed(0.1)).changed();
                ui.label("within");
                changed |= ui.add(egui::DragValue::new(tolerance).speed(0.01).range(0.0..=1.0e9)).changed();
            });
        }
        QuestionStyle::Hotspot { asset, alt, regions } => {
            ui.horizontal(|ui| {
                ui.label(if asset.is_some() { "Picture: chosen" } else { "Picture: none" });
                if ui.button("Choose picture…").clicked() { *want = Some(AssetWant::Image); }
            });
            ui.label("Description for screen readers");
            changed |= ui.add(egui::TextEdit::singleline(alt).desired_width(f32::INFINITY)).changed();
            ui.label("Correct areas: drag on the picture in the slide preview. A click inside any of them is correct.");
            let mut remove = None;
            for (i, r) in regions.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("Area {}", i + 1));
                    changed |= ui.checkbox(&mut r.ellipse, "oval").changed();
                    if ui.small_button("x").clicked() { remove = Some(i); }
                });
            }
            if let Some(i) = remove { regions.remove(i); changed = true; }
            if regions.is_empty() { ui.colored_label(egui::Color32::from_rgb(200, 60, 40), "Draw at least one area."); }
        }
        QuestionStyle::Likert { statements, scale, reversed, total_variable } => {
            reversed.resize(statements.len(), false);
            ui.label("Statements to rate");
            let mut remove = None;
            for (i, t) in statements.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    changed |= ui.add(egui::TextEdit::singleline(t).desired_width(150.0)).changed();
                    changed |= ui.checkbox(&mut reversed[i], "Rev").on_hover_text("Reverse scored: the last scale point counts as 1").changed();
                    if ui.small_button("x").clicked() { remove = Some(i); }
                });
            }
            if let Some(i) = remove { statements.remove(i); reversed.remove(i); changed = true; }
            if ui.button("+ Add statement").clicked() { statements.push(String::new()); reversed.push(false); changed = true; }
            ui.label("Scale (left to right)");
            let mut remove = None;
            let can_remove = scale.len() > 2;
            for (i, t) in scale.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    changed |= ui.add(egui::TextEdit::singleline(t).desired_width(150.0)).changed();
                    if can_remove && ui.small_button("x").clicked() { remove = Some(i); }
                });
            }
            if let Some(i) = remove { scale.remove(i); changed = true; }
            if ui.button("+ Add scale point").clicked() { scale.push(String::new()); changed = true; }
            ui.label("Save the total score in");
            let shown = total_variable.as_ref().and_then(|id| number_vars.iter().find(|v| &v.0 == id)).map(|v| v.1.clone()).unwrap_or_else(|| "nothing".into());
            combo("likert_total").selected_text(shown).show_ui(ui, |ui| {
                if ui.selectable_label(total_variable.is_none(), "nothing").clicked() { *total_variable = None; changed = true; }
                for (id, name) in number_vars {
                    if ui.selectable_label(total_variable.as_ref() == Some(id), name).clicked() { *total_variable = Some(id.clone()); changed = true; }
                }
            });
            if number_vars.is_empty() { ui.weak("Add a number variable (Variables tab) to keep a total."); }
            ui.weak("A survey is not graded (points stay at 0). The total adds each statement's chosen position, 1 for the first point.");
        }
        QuestionStyle::Reflection { min_words } => {
            ui.horizontal(|ui| {
                ui.label("Minimum words");
                changed |= ui.add(egui::DragValue::new(min_words).range(0..=500)).changed();
            });
            ui.weak("The answer is not graded. Learners type in their own words and send it.");
        }
        QuestionStyle::Dictation { asset, text, max_plays } => {
            ui.horizontal(|ui| {
                ui.label(if asset.is_some() { "Audio: chosen" } else { "Audio: none" });
                if ui.button("Choose audio…").clicked() { *want = Some(AssetWant::Audio); }
            });
            ui.label("What the clip says (learners type this; case and punctuation are ignored)");
            changed |= ui.add(egui::TextEdit::multiline(text).desired_rows(3).desired_width(f32::INFINITY)).changed();
            ui.horizontal(|ui| {
                ui.label("Plays allowed (0 = no limit)");
                changed |= ui.add(egui::DragValue::new(max_plays).range(0..=20)).changed();
            });
        }
        QuestionStyle::Dropdowns { text } => {
            ui.label("Passage (write each blank as *right/wrong/wrong*; the first option is the correct one)");
            changed |= ui.add(egui::TextEdit::multiline(text).desired_rows(5).desired_width(f32::INFINITY)).changed();
            let blanks = brainbend_doc::passage::blanks(text);
            if blanks.is_empty() || blanks.iter().any(|b| b.len() < 2) { ui.colored_label(egui::Color32::from_rgb(200, 60, 40), "Every blank needs the right answer and at least one wrong one."); }
        }
        QuestionStyle::LabelPicture { asset, alt, zones, distractors } => {
            ui.horizontal(|ui| {
                ui.label(if asset.is_some() { "Picture: chosen" } else { "Picture: none" });
                if ui.button("Choose picture…").clicked() { *want = Some(AssetWant::Image); }
            });
            ui.label("Description for screen readers");
            changed |= ui.add(egui::TextEdit::singleline(alt).desired_width(f32::INFINITY)).changed();
            ui.label("Areas and their labels: drag on the picture in the slide preview to add an area.");
            let mut remove = None;
            for (i, z) in zones.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("{}.", i + 1));
                    changed |= ui.add(egui::TextEdit::singleline(&mut z.label).desired_width(120.0)).changed();
                    changed |= ui.checkbox(&mut z.region.ellipse, "oval").changed();
                    if ui.small_button("x").clicked() { remove = Some(i); }
                });
            }
            if let Some(i) = remove { zones.remove(i); changed = true; }
            ui.label("Spare labels that belong nowhere");
            let mut remove = None;
            for (i, d) in distractors.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    changed |= ui.add(egui::TextEdit::singleline(d).desired_width(150.0)).changed();
                    if ui.small_button("x").clicked() { remove = Some(i); }
                });
            }
            if let Some(i) = remove { distractors.remove(i); changed = true; }
            if ui.button("+ Add spare label").clicked() { distractors.push(String::new()); changed = true; }
            if zones.is_empty() { ui.colored_label(egui::Color32::from_rgb(200, 60, 40), "Draw at least one area."); }
        }
        QuestionStyle::Categorize { categories } => {
            ui.label("Groups and the items that belong in each (learners see the items shuffled)");
            let mut remove = None;
            for (ci, (name, items)) in categories.iter_mut().enumerate() {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        changed |= ui.add(egui::TextEdit::singleline(name).desired_width(130.0).hint_text("group name")).changed();
                        if ui.small_button("Delete group").clicked() { remove = Some(ci); }
                    });
                    let mut drop_item = None;
                    for (ii, it) in items.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            changed |= ui.add(egui::TextEdit::singleline(it).desired_width(150.0)).changed();
                            if ui.small_button("x").clicked() { drop_item = Some(ii); }
                        });
                    }
                    if let Some(ii) = drop_item { items.remove(ii); changed = true; }
                    if ui.small_button("+ Add item").clicked() { items.push(String::new()); changed = true; }
                });
            }
            if let Some(ci) = remove { categories.remove(ci); changed = true; }
            if ui.button("+ Add group").clicked() { categories.push((String::new(), vec![String::new()])); changed = true; }
            if categories.len() < 2 { ui.colored_label(egui::Color32::from_rgb(200, 60, 40), "Needs at least two groups."); }
        }
    }
    ui.label("Feedback when correct");
    changed |= ui.text_edit_singleline(&mut q.feedback_correct).changed();
    ui.label("Feedback when incorrect");
    changed |= ui.text_edit_singleline(&mut q.feedback_incorrect).changed();
    changed
}

/// Layers shown when a slide starts: the base layer plus any a slide-start trigger shows.
fn start_layers(slide: &Slide) -> Vec<usize> {
    let mut out = vec![0];
    for t in &slide.triggers {
        if let (Event::SlideStart, Action::ShowLayer(id), Owner::Slide) = (&t.event, &t.action, &t.owner) {
            if let Some(i) = slide.layers.iter().position(|l| &l.id == id) {
                if !out.contains(&i) { out.push(i); }
            }
        }
    }
    out.sort_unstable();
    out
}

fn effect_label(k: EffectKind) -> &'static str {
    match k {
        EffectKind::Fade => "fade",
        EffectKind::SlideLeft => "from left",
        EffectKind::SlideRight => "from right",
        EffectKind::SlideUp => "from top",
        EffectKind::SlideDown => "from bottom",
        EffectKind::Zoom => "zoom",
        EffectKind::Wipe => "wipe",
        EffectKind::SpinQuarter => "spin (quarter turn)",
        EffectKind::SpinHalf => "spin (half turn)",
        EffectKind::SpinFull => "spin (full turn)",
        EffectKind::Wheel => "wheel",
        EffectKind::SplitVerticalOut => "split, vertical out",
        EffectKind::SplitVerticalIn => "split, vertical in",
        EffectKind::SplitHorizontalOut => "split, horizontal out",
        EffectKind::SplitHorizontalIn => "split, horizontal in",
    }
}

/// Entrance/exit effect: none or a kind plus a duration (and a delay for entrances).
fn effect_picker(ui: &mut egui::Ui, salt: &str, effect: &mut Option<Entrance>, with_delay: bool) -> bool {
    let mut changed = false;
    const ALL: [EffectKind; 15] = [
        EffectKind::Fade, EffectKind::SlideLeft, EffectKind::SlideRight, EffectKind::SlideUp, EffectKind::SlideDown, EffectKind::Zoom, EffectKind::Wipe,
        EffectKind::SpinQuarter, EffectKind::SpinHalf, EffectKind::SpinFull, EffectKind::Wheel,
        EffectKind::SplitVerticalOut, EffectKind::SplitVerticalIn, EffectKind::SplitHorizontalOut, EffectKind::SplitHorizontalIn,
    ];
    ui.horizontal(|ui| {
        ui.label(salt);
        let text = effect.map(|e| effect_label(e.kind)).unwrap_or("none");
        combo(("effect", salt)).selected_text(text).show_ui(ui, |ui| {
            if ui.selectable_label(effect.is_none(), "none").clicked() { *effect = None; changed = true; }
            for k in ALL {
                if ui.selectable_label(effect.map(|e| e.kind) == Some(k), effect_label(k)).clicked() {
                    let prev = *effect;
                    *effect = Some(Entrance { kind: k, delay_ms: prev.map(|e| e.delay_ms).unwrap_or(0), duration_ms: prev.map(|e| e.duration_ms).unwrap_or(500) });
                    changed = true;
                }
            }
        });
        if let Some(en) = effect.as_mut() {
            if with_delay {
                ui.label("delay");
                changed |= ui.add(egui::DragValue::new(&mut en.delay_ms).range(0..=20000).speed(10.0).suffix(" ms")).changed();
            }
            ui.label("for");
            changed |= ui.add(egui::DragValue::new(&mut en.duration_ms).range(50..=10000).speed(10.0).suffix(" ms")).changed();
        }
    });
    changed
}

fn owner_eq(a: &Owner, b: &Owner) -> bool {
    match (a, b) {
        (Owner::Slide, Owner::Slide) => true,
        (Owner::Layer(x), Owner::Layer(y)) | (Owner::Object(x), Owner::Object(y)) => x == y,
        _ => false,
    }
}

fn default_value(vars: &[Variable], id: &str) -> Value {
    match vars.iter().find(|v| v.id == id).map(|v| &v.initial) {
        Some(Value::Bool(_)) => Value::Bool(true),
        Some(Value::Num(_)) => Value::Num(0.0),
        Some(Value::Text(_)) => Value::Text(String::new()),
        None => Value::Bool(true),
    }
}

fn name_of<'a>(list: &'a [(String, String)], id: &str) -> &'a str {
    list.iter().find(|(i, _)| i == id).map(|(_, n)| n.as_str()).unwrap_or("(none)")
}

/// A variable chooser: the project's variables, then a way to make a new one without leaving the trigger.
fn pick_var(ui: &mut egui::Ui, salt: impl std::hash::Hash + std::fmt::Debug, current: &mut String, list: &[(String, String)]) -> bool {
    let mut changed = false;
    combo(salt).selected_text(name_of(list, current)).width(150.0).show_ui(ui, |ui| {
        for (id, name) in list {
            if ui.selectable_label(current == id, name).clicked() {
                *current = id.clone();
                changed = true;
            }
        }
        ui.separator();
        if ui.button("+ New variable…").clicked() {
            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("open-variables"), true));
            ui.close();
        }
    });
    changed
}

fn pick(ui: &mut egui::Ui, salt: impl std::hash::Hash + std::fmt::Debug, current: &mut String, list: &[(String, String)]) -> bool {
    let mut changed = false;
    combo(salt).selected_text(name_of(list, current)).width(150.0).show_ui(ui, |ui| {
        for (id, name) in list {
            if ui.selectable_label(current == id, name).clicked() {
                *current = id.clone();
                changed = true;
            }
        }
    });
    changed
}

/// A colour button for an optional colour (with a "none" tick). Returns true when it changed.
fn value_editor(ui: &mut egui::Ui, v: &mut Value) -> bool {
    match v {
        Value::Bool(b) => ui.checkbox(b, if *b { "true" } else { "false" }).changed(),
        Value::Num(n) => ui.add(egui::DragValue::new(n).speed(0.1)).changed(),
        Value::Text(t) => ui.add(egui::TextEdit::singleline(t).desired_width(120.0)).changed(),
    }
}

/// The 360° images of a slide with their annotations (object id, name, [(annotation id, label)]).
type PanoList = Vec<(String, String, Vec<(String, String)>)>;

/// Choose a 360° image and one of its annotations. Returns true when it changed.
fn pano_pin_ui(ui: &mut egui::Ui, salt: (usize, &str), o: &mut String, pin: &mut String, panos: &PanoList) -> bool {
    let mut changed = false;
    let names: Vec<(String, String)> = panos.iter().map(|(i, n, _)| (i.clone(), n.clone())).collect();
    if pick(ui, (salt.0, salt.1, "o"), o, &names) {
        changed = true;
        pin.clear();
    }
    let pins = panos.iter().find(|(i, _, _)| i == o).map(|(_, _, p)| p.as_slice()).unwrap_or(&[]);
    if pin.is_empty() { if let Some((id, _)) = pins.first() { *pin = id.clone(); changed = true; } }
    changed |= pick(ui, (salt.0, salt.1, "p"), pin, pins);
    changed
}

/// Editor for the triggers owned by `owner`. Returns true when anything changed.
fn triggers_ui(ui: &mut egui::Ui, slide: &mut Slide, slides: &[(String, String)], vars: &[Variable], owner: &Owner, new_event: Option<Event>, new_id: &mut dyn FnMut() -> String) -> bool {
    let mut changed = false;
    let layers: Vec<(String, String)> = slide.layers.iter().map(|l| (l.id.clone(), l.name.clone())).collect();
    let objects: Vec<(String, String)> =
        slide.layers.iter().flat_map(|l| l.objects.iter()).map(|o| (o.id.clone(), o.name.clone())).collect();
    let var_list: Vec<(String, String)> = {
        let mut v: Vec<_> = vars.iter().filter(|v| !v.built_in).map(|v| (v.id.clone(), v.name.clone())).collect();
        v.extend(vars.iter().filter(|v| v.built_in).map(|v| (v.id.clone(), v.name.clone())));
        v
    };
    let first_var = var_list.first().map(|(i, _)| i.clone()).unwrap_or_default();
    let panos: PanoList = slide.layers.iter().flat_map(|l| l.objects.iter()).filter_map(|o| match &o.kind {
        Kind::Pano(pa) => Some((o.id.clone(), o.name.clone(), pa.pins.iter().enumerate().map(|(i, p)| (p.id.clone(), format!("{}  {}", i + 1, if p.title.trim().is_empty() { "(no title)" } else { p.title.trim() }))).collect())),
        _ => None,
    }).collect();
    let first_pano = panos.first().map(|(i, _, p)| (i.clone(), p.first().map(|(id, _)| id.clone()).unwrap_or_default()));
    let first_obj_id = objects.first().map(|(i, _)| i.clone()).unwrap_or_default();
    let mut remove = None;
    let idxs: Vec<usize> =
        slide.triggers.iter().enumerate().filter(|(_, t)| owner_eq(&t.owner, owner)).map(|(i, _)| i).collect();

    // 0 = duplicate, 1 = move up, 2 = move down; chosen from a trigger's right-click menu.
    let mut op: Option<(usize, u8)> = None;
    let first_idx = idxs.first().copied();
    let last_idx = idxs.last().copied();
    for &i in &idxs {
        let t = &mut slide.triggers[i];
        // The row senses clicks itself, beneath its controls, so a right-click anywhere on it opens the menu.
        let group = ui.scope_builder(egui::UiBuilder::new().sense(egui::Sense::click()), |ui| { ui.style_mut().interaction.selectable_labels = false; ui.group(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("When");
                match owner {
                    Owner::Object(_) => {
                        let label = match t.event { Event::Hover => "hovered", Event::MouseLeave => "mouse leaves", Event::MediaEnded => "media ends", Event::MediaReaches(_) => "media reaches", Event::DoubleClick => "double-clicked", Event::KeyPress(_) => "key pressed", Event::StateChanged(..) => "state changes", Event::PanoPin(..) => "annotation pressed", _ => "clicked" };
                        combo((i, "oev")).selected_text(label).show_ui(ui, |ui| {
                            if ui.selectable_label(matches!(t.event, Event::Click), "clicked").clicked() { t.event = Event::Click; changed = true; }
                            if ui.selectable_label(matches!(t.event, Event::Hover), "hovered").clicked() { t.event = Event::Hover; changed = true; }
                            if ui.selectable_label(matches!(t.event, Event::MouseLeave), "mouse leaves").clicked() { t.event = Event::MouseLeave; changed = true; }
                            if ui.selectable_label(matches!(t.event, Event::DoubleClick), "double-clicked").clicked() { t.event = Event::DoubleClick; changed = true; }
                            if ui.selectable_label(matches!(t.event, Event::MediaEnded), "media ends").clicked() { t.event = Event::MediaEnded; changed = true; }
                            if ui.selectable_label(matches!(t.event, Event::MediaReaches(_)), "media reaches").clicked() && !matches!(t.event, Event::MediaReaches(_)) { t.event = Event::MediaReaches(1000); changed = true; }
                            if ui.selectable_label(matches!(t.event, Event::KeyPress(_)), "key pressed").clicked() && !matches!(t.event, Event::KeyPress(_)) { t.event = Event::KeyPress("Enter".into()); changed = true; }
                            if ui.selectable_label(matches!(t.event, Event::StateChanged(..)), "state changes").clicked() && !matches!(t.event, Event::StateChanged(..)) { t.event = Event::StateChanged(first_obj_id.clone(), "Selected".into()); changed = true; }
                            if let Some((po, pp)) = &first_pano {
                                if ui.selectable_label(matches!(t.event, Event::PanoPin(..)), "annotation pressed").clicked() && !matches!(t.event, Event::PanoPin(..)) { t.event = Event::PanoPin(po.clone(), pp.clone()); changed = true; }
                            }
                        });
                        if let Event::PanoPin(o, pin) = &mut t.event {
                            changed |= pano_pin_ui(ui, (i, "evpin"), o, pin, &panos);
                        }
                        if let Event::StateChanged(o, st) = &mut t.event {
                            changed |= pick(ui, (i, "evobj"), o, &objects);
                            ui.label("enters");
                            changed |= ui.add(egui::TextEdit::singleline(st).desired_width(70.0).hint_text("any state")).changed();
                        }
                        if let Event::KeyPress(k) = &mut t.event {
                            changed |= ui.add(egui::TextEdit::singleline(k).desired_width(60.0).hint_text("key")).changed();
                        }
                        if let Event::MediaReaches(ms) = &mut t.event {
                            let mut secs = *ms as f32 / 1000.0;
                            if ui.add(egui::DragValue::new(&mut secs).speed(0.1).range(0.0..=36000.0).suffix(" s")).changed() { *ms = (secs * 1000.0).round() as u32; changed = true; }
                        }
                    }
                    _ => {
                        let is_var = matches!(t.event, Event::VariableChanged(_));
                        let is_key = matches!(t.event, Event::KeyPress(_));
                        let is_next = matches!(t.event, Event::NextButton);
                        let start_label = if matches!(owner, Owner::Slide) { "slide starts" } else { "layer starts" };
                        let current = match &t.event {
                            Event::VariableChanged(_) => "variable changes",
                            Event::KeyPress(_) => "key pressed",
                            Event::NextButton => "Next button pressed",
                            Event::PreviousButton => "Previous button pressed",
                            Event::SlideEnd => "timeline ends",
                            Event::TimelineReaches(_) => "timeline reaches",
                            Event::StateChanged(..) => "object state changes",
                            Event::PanoPin(..) => "annotation pressed",
                            Event::Click => "clicked (imported)",
                            Event::Hover => "hovered (imported)",
                            Event::DoubleClick => "double-clicked (imported)",
                            _ => start_label,
                        };
                        combo((i, "ev")).selected_text(current).show_ui(ui, |ui| {
                            if matches!(owner, Owner::Slide) && ui.selectable_label(is_next, "Next button pressed").clicked() && !is_next {
                                t.event = Event::NextButton;
                                changed = true;
                            }
                            if matches!(owner, Owner::Slide) && ui.selectable_label(matches!(t.event, Event::PreviousButton), "Previous button pressed").clicked() && !matches!(t.event, Event::PreviousButton) {
                                t.event = Event::PreviousButton;
                                changed = true;
                            }
                            if matches!(owner, Owner::Slide) && ui.selectable_label(matches!(t.event, Event::SlideEnd), "timeline ends").clicked() {
                                t.event = Event::SlideEnd;
                                changed = true;
                            }
                            if ui.selectable_label(is_key, "key pressed").clicked() && !is_key { t.event = Event::KeyPress("Enter".into()); changed = true; }
                            if ui.selectable_label(matches!(t.event, Event::TimelineReaches(_)), "timeline reaches").clicked() && !matches!(t.event, Event::TimelineReaches(_)) { t.event = Event::TimelineReaches(1000); changed = true; }
                            if ui.selectable_label(matches!(t.event, Event::SlideStart | Event::LayerStart), start_label).clicked() {
                                t.event = if matches!(owner, Owner::Slide) { Event::SlideStart } else { Event::LayerStart };
                                changed = true;
                            }
                            if ui.selectable_label(is_var, "variable changes").clicked() && !is_var {
                                t.event = Event::VariableChanged(first_var.clone());
                                changed = true;
                            }
                            if ui.selectable_label(matches!(t.event, Event::StateChanged(..)), "object state changes").clicked() && !matches!(t.event, Event::StateChanged(..)) { t.event = Event::StateChanged(first_obj_id.clone(), "Selected".into()); changed = true; }
                            if let Some((po, pp)) = &first_pano {
                                if ui.selectable_label(matches!(t.event, Event::PanoPin(..)), "annotation pressed").clicked() && !matches!(t.event, Event::PanoPin(..)) { t.event = Event::PanoPin(po.clone(), pp.clone()); changed = true; }
                            }
                        });
                        if let Event::PanoPin(o, pin) = &mut t.event {
                            changed |= pano_pin_ui(ui, (i, "evpin2"), o, pin, &panos);
                        }
                        if let Event::StateChanged(o, st) = &mut t.event {
                            changed |= pick(ui, (i, "evobj2"), o, &objects);
                            ui.label("enters");
                            changed |= ui.add(egui::TextEdit::singleline(st).desired_width(70.0).hint_text("any state")).changed();
                        }
                        if let Event::VariableChanged(v) = &mut t.event {
                            changed |= pick_var(ui, (i, "evvar"), v, &var_list);
                        }
                        if let Event::KeyPress(k) = &mut t.event {
                            changed |= ui.add(egui::TextEdit::singleline(k).desired_width(60.0).hint_text("key")).changed();
                        }
                        if let Event::TimelineReaches(ms) = &mut t.event {
                            let mut secs = *ms as f32 / 1000.0;
                            if ui.add(egui::DragValue::new(&mut secs).speed(0.1).range(0.0..=3600.0).suffix(" s")).changed() { *ms = (secs * 1000.0).round() as u32; changed = true; }
                        }
                    }
                }
                ui.label("→");
                let kind = match &t.action {
                    Action::ShowLayer(_) => "show layer",
                    Action::HideLayer(_) => "hide layer",
                    Action::SetVariable { .. } => "set variable",
                    Action::SetObjectState { .. } => "set object state",
                    Action::RunJavaScript(_) => "run JavaScript",
                    Action::JumpToSlide(_) => "go to slide",
                    Action::NextSlide => "next slide",
                    Action::PreviousSlide => "previous slide",
                    Action::ShowObject(_) => "show object",
                    Action::HideObject(_) => "hide object",
                    Action::AdjustVariable { .. } => "adjust variable",
                    Action::OpenUrl(_) => "open web page",
                    Action::PlayMedia(_) => "play media",
                    Action::PlayAnimation(_) => "play animation",
                    Action::PauseMedia(_) => "pause media",
                    Action::StopMedia(_) => "stop media",
                    Action::SeekTimeline { .. } => "seek timeline",
                    Action::CompleteCourse => "complete course",
                    Action::ExitCourse => "exit course",
                    Action::RestartCourse => "restart course",
                    Action::Unsupported(_) => "unsupported (imported)",
                };
                combo((i, "act")).selected_text(kind).show_ui(ui, |ui| {
                    let first_layer = layers.get(1).or(layers.first()).map(|(i, _)| i.clone()).unwrap_or_default();
                    let first_obj = objects.first().map(|(i, _)| i.clone()).unwrap_or_default();
                    if ui.selectable_label(kind == "show layer", "show layer").clicked() { t.action = Action::ShowLayer(first_layer.clone()); changed = true; }
                    if ui.selectable_label(kind == "hide layer", "hide layer").clicked() { t.action = Action::HideLayer(first_layer.clone()); changed = true; }
                    if ui.selectable_label(kind == "set variable", "set variable").clicked() {
                        t.action = Action::SetVariable { variable: first_var.clone(), value: default_value(vars, &first_var) };
                        changed = true;
                    }
                    if ui.selectable_label(kind == "set object state", "set object state").clicked() {
                        t.action = Action::SetObjectState { object: first_obj.clone(), state: "Normal".into() };
                        changed = true;
                    }
                    if ui.selectable_label(kind == "show object", "show object").clicked() { t.action = Action::ShowObject(first_obj.clone()); changed = true; }
                    if ui.selectable_label(kind == "hide object", "hide object").clicked() { t.action = Action::HideObject(first_obj.clone()); changed = true; }
                    if ui.selectable_label(kind == "adjust variable", "adjust variable").clicked() {
                        t.action = Action::AdjustVariable { variable: first_var.clone(), op: AdjustOp::Add, value: Value::Num(1.0) };
                        changed = true;
                    }
                    if ui.selectable_label(kind == "play animation", "play animation").clicked() { t.action = Action::PlayAnimation(first_obj.clone()); changed = true; }
                    if ui.selectable_label(kind == "play media", "play media").clicked() { t.action = Action::PlayMedia(first_obj.clone()); changed = true; }
                    if ui.selectable_label(kind == "pause media", "pause media").clicked() { t.action = Action::PauseMedia(first_obj.clone()); changed = true; }
                    if ui.selectable_label(kind == "stop media", "stop media").clicked() { t.action = Action::StopMedia(first_obj.clone()); changed = true; }
                    if ui.selectable_label(kind == "seek timeline", "seek timeline").clicked() { t.action = Action::SeekTimeline { target: String::new(), ms: 0 }; changed = true; }
                    if ui.selectable_label(kind == "complete course", "complete course").clicked() { t.action = Action::CompleteCourse; changed = true; }
                    if ui.selectable_label(kind == "exit course", "exit course").clicked() { t.action = Action::ExitCourse; changed = true; }
                    if ui.selectable_label(kind == "restart course", "restart course").clicked() { t.action = Action::RestartCourse; changed = true; }
                    if ui.selectable_label(kind == "open web page", "open web page").clicked() { t.action = Action::OpenUrl("https://".into()); changed = true; }
                    if ui.selectable_label(kind == "run JavaScript", "run JavaScript").clicked() { t.action = Action::RunJavaScript(String::new()); changed = true; }
                    if ui.selectable_label(kind == "go to slide", "go to slide").clicked() {
                        t.action = Action::JumpToSlide(slides.first().map(|(i, _)| i.clone()).unwrap_or_default());
                        changed = true;
                    }
                    if ui.selectable_label(kind == "next slide", "next slide").clicked() { t.action = Action::NextSlide; changed = true; }
                    if ui.selectable_label(kind == "previous slide", "previous slide").clicked() { t.action = Action::PreviousSlide; changed = true; }
                });
                match &mut t.action {
                    Action::ShowLayer(l) | Action::HideLayer(l) => changed |= pick(ui, (i, "lay"), l, &layers),
                    Action::SetVariable { variable, value } => {
                        if pick_var(ui, (i, "var"), variable, &var_list) {
                            *value = default_value(vars, variable);
                            changed = true;
                        }
                        ui.label("to");
                        changed |= value_editor(ui, value);
                    }
                    Action::SetObjectState { object, state } => {
                        changed |= pick(ui, (i, "obj"), object, &objects);
                        ui.label("to");
                        changed |= ui.add(egui::TextEdit::singleline(state).desired_width(90.0)).changed();
                    }
                    Action::ShowObject(o) | Action::HideObject(o) | Action::PlayMedia(o) | Action::PauseMedia(o) | Action::StopMedia(o) | Action::PlayAnimation(o) => changed |= pick(ui, (i, "tobj"), o, &objects),
                    Action::AdjustVariable { variable, op, value } => {
                        changed |= pick_var(ui, (i, "avar"), variable, &var_list);
                        combo((i, "aop")).width(90.0).selected_text(op.label()).show_ui(ui, |ui| {
                            for o in AdjustOp::ALL { if ui.selectable_label(*op == o, o.label()).clicked() { *op = o; changed = true; } }
                        });
                        changed |= value_editor(ui, value);
                    }
                    Action::OpenUrl(u) => changed |= ui.add(egui::TextEdit::singleline(u).desired_width(220.0).hint_text("https://")).changed(),
                    Action::RunJavaScript(js) => {
                        changed |= ui.add(egui::TextEdit::multiline(js).desired_rows(2).desired_width(320.0).code_editor()).changed();
                    }
                    Action::JumpToSlide(id) => changed |= pick(ui, (i, "sld"), id, slides),
                    Action::NextSlide | Action::PreviousSlide | Action::CompleteCourse | Action::ExitCourse | Action::RestartCourse => {}
                    Action::SeekTimeline { target, ms } => {
                        // A layer, or a video or audio object; "this layer" when empty.
                        let mut targets: Vec<(String, String)> = vec![(String::new(), "this layer".into())];
                        targets.extend(layers.iter().map(|(i, n)| (i.clone(), format!("layer {n}"))));
                        targets.extend(objects.iter().map(|(i, n)| (i.clone(), format!("object {n}"))));
                        changed |= pick(ui, (i, "seek"), target, &targets);
                        ui.label("to");
                        let mut secs = *ms as f32 / 1000.0;
                        if ui.add(egui::DragValue::new(&mut secs).speed(0.1).range(0.0..=36000.0).suffix(" s")).changed() { *ms = (secs * 1000.0).round() as u32; changed = true; }
                    }
                    Action::Unsupported(what) => { ui.weak(what.as_str()); }
                }
                if ui.button("Delete").clicked() { remove = Some(i); }
            });
            // Conditions, evaluated left to right.
            let mut drop_cond = None;
            for (ci, c) in t.conditions.iter_mut().enumerate() {
                ui.horizontal_wrapped(|ui| {
                    if ci == 0 {
                        ui.label("   only if");
                    } else {
                        let mut or = c.or_with_previous;
                        combo((i, ci, "andor")).width(50.0).selected_text(if or { "or" } else { "and" }).show_ui(ui, |ui| {
                            if ui.selectable_label(!or, "and").clicked() { or = false; }
                            if ui.selectable_label(or, "or").clicked() { or = true; }
                        });
                        if or != c.or_with_previous { c.or_with_previous = or; changed = true; }
                    }
                    if pick_var(ui, (i, ci, "cvar"), &mut c.variable, &var_list) {
                        c.value = default_value(vars, &c.variable);
                        changed = true;
                    }
                    let ops = [(Cmp::Eq, "=="), (Cmp::Ne, "!="), (Cmp::Gt, ">"), (Cmp::Lt, "<"), (Cmp::Ge, ">="), (Cmp::Le, "<=")];
                    let label = ops.iter().find(|(o, _)| *o == c.cmp).map(|(_, l)| *l).unwrap_or("==");
                    combo((i, ci, "cmp")).width(50.0).selected_text(label).show_ui(ui, |ui| {
                        for (o, l) in ops {
                            if ui.selectable_label(c.cmp == o, l).clicked() { c.cmp = o; changed = true; }
                        }
                    });
                    changed |= value_editor(ui, &mut c.value);
                    if ui.small_button("x").clicked() { drop_cond = Some(ci); }
                });
            }
            if let Some(ci) = drop_cond { t.conditions.remove(ci); changed = true; }
            if ui.small_button("+ condition").clicked() && !first_var.is_empty() {
                t.conditions.push(Condition {
                    variable: first_var.clone(),
                    cmp: Cmp::Eq,
                    value: default_value(vars, &first_var),
                    or_with_previous: false,
                });
                changed = true;
            }
        }) });
        group.response.context_menu(|ui| {
            if ui.button("Duplicate").clicked() { op = Some((i, 0)); ui.close(); }
            if ui.add_enabled(Some(i) != first_idx, egui::Button::new("Move up")).clicked() { op = Some((i, 1)); ui.close(); }
            if ui.add_enabled(Some(i) != last_idx, egui::Button::new("Move down")).clicked() { op = Some((i, 2)); ui.close(); }
            ui.separator();
            if ui.button("Delete").clicked() { remove = Some(i); ui.close(); }
        });
    }
    if let Some((i, what)) = op {
        match what {
            0 => { let mut copy = slide.triggers[i].clone(); copy.id = new_id(); slide.triggers.insert(i + 1, copy); }
            1 => { if let Some(&prev) = idxs.iter().rev().find(|&&k| k < i) { slide.triggers.swap(prev, i); } }
            _ => { if let Some(&next) = idxs.iter().find(|&&k| k > i) { slide.triggers.swap(next, i); } }
        }
        changed = true;
    }
    if let Some(i) = remove {
        slide.triggers.remove(i);
        changed = true;
    }
    if ui.button("+ Add trigger").clicked() {
        let first_layer = slide.layers.get(1).or(slide.layers.first()).map(|l| l.id.clone()).unwrap_or_default();
        slide.triggers.push(Trigger {
            id: new_id(),
            owner: owner.clone(),
            event: new_event.unwrap_or(match owner { Owner::Object(_) => Event::Click, Owner::Layer(_) => Event::LayerStart, Owner::Slide => Event::SlideStart }),
            action: Action::ShowLayer(first_layer),
            conditions: vec![],
        });
        changed = true;
    }
    changed
}

/// The looks of an inserted dial (three of them): a knob with a teardrop pointer, a knob with a dot, and a needle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DialStyle { Teardrop, Dot, Needle }

/// The pointer of a dial as a path in its 300 x 300 frame, drawn pointing straight up (the middle of the range).
pub(crate) fn dial_pointer(style: DialStyle) -> Vec<PathCmd> {
    match style {
        DialStyle::Teardrop => {
            // A round body at the centre drawn out to a tip at the top: the tip joins the body along its tangents.
            let (c, r, tip) = ((0.5_f32, 0.5_f32), 0.17_f32, (0.5_f32, 0.07_f32));
            let alpha = (r / (c.1 - tip.1)).acos();
            let (a0, a1) = (-std::f32::consts::FRAC_PI_2 + alpha, -std::f32::consts::FRAC_PI_2 - alpha + std::f32::consts::TAU);
            let mut cmds = vec![PathCmd::Move(tip.0, tip.1)];
            for i in 0..=40 {
                let a = a0 + (a1 - a0) * i as f32 / 40.0;
                cmds.push(PathCmd::Line(c.0 + r * a.cos(), c.1 + r * a.sin()));
            }
            cmds.push(PathCmd::Close);
            cmds
        }
        DialStyle::Dot => {
            let (cx, cy, r, k) = (0.5_f32, 0.18_f32, 0.065_f32, 0.5523_f32);
            vec![
                PathCmd::Move(cx + r, cy),
                PathCmd::Curve(cx + r, cy + k * r, cx + k * r, cy + r, cx, cy + r),
                PathCmd::Curve(cx - k * r, cy + r, cx - r, cy + k * r, cx - r, cy),
                PathCmd::Curve(cx - r, cy - k * r, cx - k * r, cy - r, cx, cy - r),
                PathCmd::Curve(cx + k * r, cy - r, cx + r, cy - k * r, cx + r, cy),
                PathCmd::Close,
            ]
        }
        DialStyle::Needle => vec![PathCmd::Move(0.5, 0.04), PathCmd::Line(0.56, 0.52), PathCmd::Line(0.44, 0.52), PathCmd::Close],
    }
}

/// Whether an object's text has a slide-number token (`%Slide.Number%` or `%Slide.Total%`).
fn has_slide_tokens(o: &Object) -> bool {
    let has = |t: &str| t.contains("%Slide.");
    has(&o.text) || o.rich.as_ref().is_some_and(|ps| ps.iter().any(|p| p.runs.iter().any(|r| has(&r.text))))
}

/// Put a slide's number and the lesson's length into an object's slide-number tokens, for drawing in the editor.
fn number_tokens(o: &mut Object, (n, total): (usize, usize)) {
    let fill = |t: &mut String| { if t.contains("%Slide.") { *t = t.replace("%Slide.Number%", &n.to_string()).replace("%Slide.Total%", &total.to_string()); } };
    fill(&mut o.text);
    if let Some(ps) = o.rich.as_mut() { for p in ps { for r in &mut p.runs { fill(&mut r.text); } } }
}

/// Montserrat and Open Sans are bundled (SIL OFL); other family names fall back to Open Sans,
/// as the web player does.
fn kind_name(k: &Kind) -> &'static str {
    match k {
        Kind::Rect => "Rectangle",
        Kind::Ellipse => "Ellipse",
        Kind::Text => "Text box",
        Kind::Image => "Picture",
        Kind::Video => "Video",
        Kind::Audio => "Audio",
        Kind::Hotspot => "Hotspot (invisible click area)",
        Kind::Path(_) => "Vector shape",
        Kind::Line | Kind::LineUp => "Line",
        Kind::Slider(_) => "Slider",
        Kind::Web(_) => "Web object",
        Kind::Pano(_) => "360° image",
        Kind::Input(f) => if f.numeric { "Number entry" } else { "Text entry" },
        Kind::Unsupported(_) => "Unsupported object",
    }
}

fn font_family(name: &str) -> egui::FontFamily {
    if name.eq_ignore_ascii_case("montserrat") {
        egui::FontFamily::Name("Montserrat".into())
    } else {
        egui::FontFamily::Name("Open Sans".into())
    }
}

/// Brand accent + softer geometry. Applied to both light and dark themes.
/// A small section heading for the side panel: quiet capitals with a thin rule above, instead of a large heading.
fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.add_space(4.0);
    let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().rect_filled(r, 0.0, ui.visuals().text_color().gamma_multiply(0.15));
    ui.add_space(2.0);
    ui.label(egui::RichText::new(text.to_uppercase()).size(10.5).strong().color(ui.visuals().text_color().gamma_multiply(0.6)));
}

/// A file as the status bar names it: just its name, never a path with `..` in it.
fn file_label(path: &std::path::Path) -> String {
    path.file_name().and_then(|n| n.to_str()).map(str::to_string).unwrap_or_else(|| path.display().to_string())
}

fn install_theme(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 4.0);
        style.spacing.interact_size.y = 24.0;
        let r = egui::CornerRadius::same(4);
        style.visuals.widgets.noninteractive.corner_radius = r;
        style.visuals.widgets.inactive.corner_radius = r;
        style.visuals.widgets.hovered.corner_radius = r;
        style.visuals.widgets.active.corner_radius = r;
        style.visuals.widgets.open.corner_radius = r;
        style.visuals.window_corner_radius = egui::CornerRadius::same(4);
        style.visuals.menu_corner_radius = egui::CornerRadius::same(4);
        style.visuals.selection.bg_fill = egui::Color32::from_rgb(3, 83, 158);
        style.visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        style.visuals.disabled_alpha = 0.62;
        style.visuals.hyperlink_color = egui::Color32::from_rgb(70, 150, 230);
    });
    // A soft grey window behind a white (or slightly lifted) ribbon card.
    ctx.style_mut_of(egui::Theme::Light, |s| { s.visuals.panel_fill = egui::Color32::from_rgb(238, 240, 244); });
    ctx.style_mut_of(egui::Theme::Dark, |s| { s.visuals.panel_fill = egui::Color32::from_rgb(30, 31, 35); });
}

fn install_fonts(ctx: &egui::Context) {
    use std::sync::Arc;
    let mut fonts = egui::FontDefinitions::default();
    let sources: [(&str, &'static [u8]); 2] = [
        ("Montserrat", include_bytes!("../../../assets/fonts/Montserrat[wght].ttf")),
        ("Open Sans", include_bytes!("../../../assets/fonts/OpenSans[wdth,wght].ttf")),
    ];
    for (name, bytes) in sources {
        fonts.font_data.insert(name.to_string(), Arc::new(egui::FontData::from_static(bytes)));
        // Default fonts stay as fallbacks for symbols the bundled fonts lack.
        let mut chain = vec![name.to_string()];
        chain.extend(fonts.families.get(&egui::FontFamily::Proportional).cloned().unwrap_or_default());
        // Hack covers the maths operators and arrows the other fonts lack (the Symbol picker offers them).
        if fonts.font_data.contains_key("Hack") { chain.push("Hack".to_string()); }
        fonts.families.insert(egui::FontFamily::Name(name.into()), chain);
    }
    fonts.font_data.insert("icons".into(), Arc::new(egui::FontData::from_static(include_bytes!("../../../assets/icons/Phosphor.ttf"))));
    fonts.families.insert(ribbon::icons(), vec!["icons".into()]);
    ctx.set_fonts(fonts);
}

fn color(c: Rgba) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(c.0, c.1, c.2, c.3)
}

/// The ids of the variables any trigger reads or changes: a condition, a set or adjust action, or a change event.
fn variables_in_triggers(p: &Project) -> std::collections::HashSet<String> {
    let mut ids = std::collections::HashSet::new();
    for t in p.slides.iter().flat_map(|s| s.triggers.iter()) {
        for c in &t.conditions { ids.insert(c.variable.clone()); }
        match (&t.event, &t.action) {
            (Event::VariableChanged(v), _) => { ids.insert(v.clone()); }
            _ => {}
        }
        if let Action::SetVariable { variable, .. } | Action::AdjustVariable { variable, .. } = &t.action { ids.insert(variable.clone()); }
    }
    ids
}

/// Does `v` pass the Variables window's search and filters? `kind`: 0 any, 1 true/false, 2 number, 3 text.
/// `usage`: 0 any, 1 named by a trigger, 2 named by no trigger.
fn variable_shown(v: &Variable, in_triggers: &std::collections::HashSet<String>, query: &str, kind: usize, usage: usize) -> bool {
    let q = query.trim().to_lowercase();
    (q.is_empty() || v.name.to_lowercase().contains(&q))
        && match kind { 1 => matches!(v.initial, Value::Bool(_)), 2 => matches!(v.initial, Value::Num(_)), 3 => matches!(v.initial, Value::Text(_)), _ => true }
        && match usage { 1 => in_triggers.contains(&v.id), 2 => !in_triggers.contains(&v.id), _ => true }
}

impl eframe::App for App {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.show(root);
    }
}

/// Spelling window state.
#[derive(Default)]
struct SpellState {
    /// Lowercase words skipped until the project is closed.
    ignore: std::collections::HashSet<String>,
    /// Suggestions already worked out (they are slow to compute every frame).
    suggestions: std::collections::HashMap<String, Vec<String>>,
}

/// Find and Replace window state.
#[derive(Default)]
struct FindState {
    query: String,
    with: String,
    opts: brainbend_doc::find::FindOpts,
    /// What the last Replace did.
    message: String,
    /// Take keyboard focus in the search box (set when the window opens).
    focus: bool,
}

impl App {
    fn open_spelling(&mut self) {
        if self.project.is_none() { return; }
        brainbend_spell::warm();
        self.spell.get_or_insert_with(Default::default);
    }

    /// One frame of the whole UI (separate from `eframe::App::ui` so tests can drive it).
    /// One frame. The States tab's viewed state is on its object for the length of it.
    fn show(&mut self, root: &mut egui::Ui) {
        // While trying, nothing is selected, so no panel keeps showing an object the author is no longer working on.
        if self.try_mode { self.selected = None; self.multi.clear(); }
        self.sync_tabs();
        self.begin_state_swap();
        self.show_frame(root);
        if let Some(sel) = self.pending_delete.take() { self.delete_slides(sel); }
        self.end_state_swap();
        self.save_active_tab();
    }

    fn show_frame(&mut self, root: &mut egui::Ui) {
        let ctx = &root.ctx().clone();
        let (undo_k, redo_k, save_k, open_k) = ctx.input(|i| {
            let cmd = i.modifiers.command;
            (
                cmd && !i.modifiers.shift && i.key_pressed(egui::Key::Z),
                cmd && i.modifiers.shift && i.key_pressed(egui::Key::Z),
                cmd && i.key_pressed(egui::Key::S),
                cmd && i.key_pressed(egui::Key::O),
            )
        });
        if let Some(e) = self.start_edit.take() { self.editing = Some(e); }
        if let Some(t) = self.clip_out.take() { ctx.copy_text(t.clone()); self.clip_text = t; }
        if self.project.is_some() && ctx.input(|i| i.key_pressed(egui::Key::F7)) { self.open_spelling(); }
        if self.project.is_some() && ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::F)) {
            self.find.get_or_insert_with(FindState::default).focus = true;
        }
        // Delete / arrows / duplicate only when no text field has focus.
        if self.project.is_some() && ctx.memory(|m| m.focused().is_none()) {
            // On a real desktop the window system turns ⌘C, ⌘X and ⌘V into Copy, Cut and Paste events, with no key press
            // at all; the key checks cover harnesses and platforms that send the keys instead.
            let (copy, cut, key_paste, pasted_text) = ctx.input(|i| {
                let ev = |f: fn(&egui::Event) -> bool| i.events.iter().any(f);
                (
                    ev(|e| matches!(e, egui::Event::Copy)) || (i.modifiers.command && i.key_pressed(egui::Key::C)),
                    ev(|e| matches!(e, egui::Event::Cut)) || (i.modifiers.command && i.key_pressed(egui::Key::X)),
                    i.modifiers.command && i.key_pressed(egui::Key::V),
                    i.events.iter().find_map(|e| if let egui::Event::Paste(t) = e { Some(t.clone()) } else { None }),
                )
            });
            if copy { self.copy_selected(); }
            if cut { self.copy_selected(); self.delete_selected(); }
            if key_paste { self.paste(); }
            if let Some(text) = pasted_text {
                // Our own copy comes back as the text we left on the system clipboard; anything else was copied elsewhere.
                if self.clipboard.is_some() && text == self.clip_text { self.paste(); }
                else if !text.trim().is_empty() {
                    self.insert_text();
                    if let (Some((li, oi)), Some(p)) = (self.selected, self.project.as_mut()) {
                        if let Some(o) = p.slides[self.slide].layers.get_mut(li).and_then(|l| l.objects.get_mut(oi)) { o.text = text; }
                    }
                }
            }
            let (del, dup, left, right, up, down, big) = ctx.input(|i| (
                i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace),
                i.modifiers.command && i.key_pressed(egui::Key::D),
                i.key_pressed(egui::Key::ArrowLeft), i.key_pressed(egui::Key::ArrowRight),
                i.key_pressed(egui::Key::ArrowUp), i.key_pressed(egui::Key::ArrowDown),
                i.modifiers.shift,
            ));
            let step = if big { 10.0 } else { 1.0 };
            // With a corner of a hotspot's outline chosen, the keys act on that corner and not on the hotspot.
            let corner = self.outline_mode.is_some() && self.outline_sel.is_some();
            let cropping_now = self.crop_mode.is_some();
            if del { if corner { self.remove_outline_point(); } else { self.delete_selected(); } }
            if dup { self.duplicate_selected(); }
            if corner {
                if left { self.nudge_outline_point(-step, 0.0); }
                if right { self.nudge_outline_point(step, 0.0); }
                if up { self.nudge_outline_point(0.0, -step); }
                if down { self.nudge_outline_point(0.0, step); }
            } else if cropping_now {
                // While cropping, the arrow keys slide the picture under its frame.
                let (dx, dy) = (if left { -step } else if right { step } else { 0.0 }, if up { -step } else if down { step } else { 0.0 });
                if dx != 0.0 || dy != 0.0 { self.pan_cropped(dx, dy); }
            } else {
                if left { self.nudge_selected(-step, 0.0); }
                if right { self.nudge_selected(step, 0.0); }
                if up { self.nudge_selected(0.0, -step); }
                if down { self.nudge_selected(0.0, step); }
            }
        }
        if undo_k { self.undo(); }
        if redo_k { self.redo(); }
        if save_k { self.save(false); }
        let mut do_open = open_k;

        self.sync_master();
        self.chrome(root, ctx);
        self.player_window(ctx);
        self.variables_window(ctx);
        self.focus_order_window(ctx);
        self.equation_window(ctx);
        self.theme_colors_window(ctx);
        if std::mem::take(&mut self.open_requested) { do_open = true; }
        if do_open { self.guard(Pending::Open); }
        // A .story/.brainbend dropped onto the window opens like File ▸ Open.
        let dropped = ctx.input(|i| i.raw.dropped_files.iter().next().map(|f| f.path().to_path_buf()));
        if let Some(path) = dropped {
            let ok = path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("story") || e.eq_ignore_ascii_case("brainbend") || e.eq_ignore_ascii_case("storyapp") || e.eq_ignore_ascii_case("pptx"));
            if ok {
                self.drop_path = Some(path);
                self.guard(Pending::OpenDropped);
            } else {
                self.status = "Only .brainbend, .story and .pptx files can be opened".into();
            }
        }

        // Window title, close guard, autosave.
        {
            let name = self.project.as_ref().map(|p| p.name.clone()).unwrap_or_default();
            let title = if name.is_empty() { "BrainBend Studio".to_string() } else { format!("BrainBend Studio — {name}{}", if self.dirty { " •" } else { "" }) };
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
            if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close {
                if self.dirty && self.project.is_some() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                    self.pending = Some(Pending::Quit);
                } else {
                    self.allow_close = true;
                }
            }
            if self.dirty && self.last_autosave.elapsed().as_secs() >= 30 { self.autosave(); }
            ctx.request_repaint_after(std::time::Duration::from_secs(5));
        }
        // Spelling: misspelled words across the project, each with suggestions.
        if self.spell.is_some() && self.project.is_some() {
            let mut open = true;
            let mut st = self.spell.take().unwrap();
            let found = brainbend_spell::check_project(self.project.as_ref().unwrap(), &st.ignore);
            let mut jump: Option<(usize, Option<(usize, usize)>, Option<usize>)> = None;
            let web = self.project.as_ref().unwrap().is_web();
            let mut change: Option<(brainbend_spell::Misspelling, String)> = None;
            let mut add: Option<String> = None;
            let mut ignore: Option<String> = None;
            egui::Window::new("Spelling").open(&mut open).default_width(460.0).show(ctx, |ui| {
                if found.is_empty() {
                    ui.label("No misspelled words found.");
                } else {
                    ui.label(format!("{} possible misspelling{}", found.len(), if found.len() == 1 { "" } else { "s" }));
                }
                ui.weak("English (US). Add names and terms you use to the dictionary, or ignore them.");
                ui.separator();
                egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                    for m in &found {
                        ui.group(|ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.strong(&m.word);
                                ui.label(format!("{} {}: {}", if web { "Section" } else { "Slide" }, m.slide + 1, m.context));
                            });
                            let sugg = st.suggestions.entry(m.word.clone()).or_insert_with(|| brainbend_spell::suggest(&m.word)).clone();
                            ui.horizontal_wrapped(|ui| {
                                for s in &sugg { if ui.small_button(s).on_hover_text("Change this word here").clicked() { change = Some((m.clone(), s.clone())); } }
                                if sugg.is_empty() { ui.weak("No suggestions"); }
                            });
                            ui.horizontal(|ui| {
                                if ui.small_button("Go to").clicked() {
                                    jump = Some((m.slide, if let brainbend_doc::find::Place::Object(l, o) = m.place { Some((l, o)) } else { None }, if let brainbend_doc::find::Place::Block(b) = m.place { Some(b) } else { None }));
                                }
                                if ui.small_button("Ignore").clicked() { ignore = Some(m.word.to_lowercase()); }
                                if ui.small_button("Add to dictionary").on_hover_text("Saved with the project").clicked() { add = Some(m.word.to_lowercase()); }
                            });
                        });
                    }
                });
            });
            if let Some((m, to)) = change {
                self.snapshot();
                let hit = brainbend_doc::find::Hit { slide: m.slide, place: m.place.clone(), count: 1, context: String::new() };
                let opts = brainbend_doc::find::FindOpts { match_case: true, whole_word: true };
                self.project.as_mut().unwrap().replace_text(&m.word, &to, opts, Some(&hit));
            }
            if let Some(w) = add {
                self.snapshot();
                let p = self.project.as_mut().unwrap();
                if !p.custom_words.contains(&w) { p.custom_words.push(w); }
            }
            if let Some(w) = ignore { st.ignore.insert(w); }
            if let Some((slide, object, block)) = jump {
                if web {
                    self.sec = slide;
                    self.blk = block;
                } else {
                    self.slide = slide;
                    self.layer = object.map(|o| o.0).unwrap_or(0);
                    self.selected = object;
                    self.reset_visibility();
                }
            }
            if open { self.spell = Some(st); }
        }
        // Find and Replace: matches across slide text, notes, names and question text.
        if self.find.is_some() && self.project.is_some() {
            let mut open = true;
            let mut jump: Option<(usize, Option<(usize, usize)>, Option<usize>)> = None;
            let mut replace: Option<Option<brainbend_doc::find::Hit>> = None; // Some(None) = all
            let web = self.project.as_ref().unwrap().is_web();
            let mut st = self.find.take().unwrap();
            let hits = self.project.as_ref().unwrap().find_text(&st.query, st.opts);
            egui::Window::new("Find and Replace").open(&mut open).default_width(420.0).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Find     ");
                    let r = ui.add(egui::TextEdit::singleline(&mut st.query).desired_width(f32::INFINITY));
                    if st.focus { r.request_focus(); st.focus = false; }
                });
                ui.horizontal(|ui| {
                    ui.label("Replace");
                    ui.add(egui::TextEdit::singleline(&mut st.with).desired_width(f32::INFINITY));
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut st.opts.match_case, "Match case");
                    ui.checkbox(&mut st.opts.whole_word, "Whole word");
                });
                let total: usize = hits.iter().map(|h| h.count).sum();
                ui.horizontal(|ui| {
                    ui.add_enabled_ui(total > 0, |ui| { if ui.button("Replace all").clicked() { replace = Some(None); } });
                    if !st.query.is_empty() { ui.label(format!("{total} match{} in {} place{}", if total == 1 { "" } else { "es" }, hits.len(), if hits.len() == 1 { "" } else { "s" })); }
                });
                if !st.message.is_empty() { ui.weak(&st.message); }
                ui.separator();
                egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                    for h in &hits {
                        ui.horizontal_wrapped(|ui| {
                            let what = match h.place {
                                brainbend_doc::find::Place::Object(..) => "text",
                                brainbend_doc::find::Place::SlideName => "slide name",
                                brainbend_doc::find::Place::Notes => "notes",
                                brainbend_doc::find::Place::Question => "question",
                                brainbend_doc::find::Place::Section => "section title",
                                brainbend_doc::find::Place::Block(_) => "block",
                            };
                            ui.label(format!("{} {} ({what}): {}", if web { "Section" } else { "Slide" }, h.slide + 1, h.context));
                            if ui.small_button("Go to").clicked() {
                                jump = Some((h.slide, if let brainbend_doc::find::Place::Object(l, o) = h.place { Some((l, o)) } else { None }, if let brainbend_doc::find::Place::Block(b) = h.place { Some(b) } else { None }));
                            }
                            if ui.small_button("Replace").clicked() { replace = Some(Some(h.clone())); }
                        });
                    }
                });
            });
            if let Some(which) = replace {
                self.snapshot();
                let n = self.project.as_mut().unwrap().replace_text(&st.query, &st.with, st.opts, which.as_ref());
                st.message = format!("Replaced {n}.");
            }
            if let Some((slide, object, block)) = jump {
                if web {
                    self.sec = slide;
                    self.blk = block;
                } else {
                    self.slide = slide;
                    self.layer = object.map(|o| o.0).unwrap_or(0);
                    self.selected = object;
                    self.reset_visibility();
                }
            }
            if open { self.find = Some(st); }
        }
        // Project check report: errors and warnings, each with a button that jumps to the problem.
        if let Some(issues) = self.check_results.clone() {
            let mut open = true;
            let mut jump: Option<brainbend_doc::check::Location> = None;
            let web_check = self.project.as_ref().is_some_and(|p| p.is_web());
            egui::Window::new("Project check").open(&mut open).default_width(520.0).show(ctx, |ui| {
                if issues.is_empty() {
                    ui.label("No problems found.");
                } else {
                    let errors = issues.iter().filter(|i| i.severity == brainbend_doc::check::Severity::Error).count();
                    ui.label(format!("{} problem(s): {} error(s), {} warning(s)", issues.len(), errors, issues.len() - errors));
                    ui.separator();
                    egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                        for i in &issues {
                            ui.horizontal_wrapped(|ui| {
                                let (tag, col) = match i.severity {
                                    brainbend_doc::check::Severity::Error => ("Error", egui::Color32::from_rgb(220, 70, 60)),
                                    brainbend_doc::check::Severity::Warning => ("Warning", egui::Color32::from_rgb(220, 170, 40)),
                                };
                                ui.colored_label(col, tag);
                                if web_check { ui.label(&i.message); } else { ui.label(format!("Slide {}: {}", i.at.slide + 1, i.message)); }
                                if ui.small_button("Go to").clicked() { jump = Some(i.at.clone()); }
                            });
                        }
                    });
                }
                if ui.button("Run again").clicked() {
                    self.check_results = self.project.as_ref().map(|p| p.check());
                }
            });
            if let Some(at) = jump {
                if web_check {
                    self.sec = at.slide;
                    self.blk = at.object.map(|o| o.0);
                } else {
                    self.slide = at.slide;
                    self.layer = at.object.map(|o| o.0).unwrap_or(0);
                    self.selected = at.object;
                    self.reset_visibility();
                }
            }
            if !open { self.check_results = None; }
        }
        let mut chosen: Option<(bool, bool)> = None; // (save first?, proceed?)
        if let Some(action) = self.pending {
            egui::Window::new("Unsaved changes").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
                ui.label("This project has changes that have not been saved.");
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() { chosen = Some((true, true)); }
                    if ui.button("Don't save").clicked() { chosen = Some((false, true)); }
                    if ui.button("Cancel").clicked() { chosen = Some((false, false)); }
                });
            });
            if let Some((save, proceed)) = chosen {
                self.pending = None;
                if save { self.save(false); }
                // If saving was cancelled (no file chosen) the project is still dirty: stay put.
                if proceed && (!save || !self.dirty) {
                    if !save { Self::clear_recovery(); self.dirty = false; }
                    self.perform(action);
                    if action == Pending::Quit { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
                }
            }
        }
        self.update_window(ctx);
        if self.updater.restarting { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
        if self.recovery_offer {
            let mut done = false;
            egui::Window::new("Recover unsaved work?").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
                ui.label("BrainBend Studio found a recovery copy from a previous session.");
                ui.horizontal(|ui| {
                    if ui.button("Recover").clicked() {
                        if let Some(p) = recovery_path() {
                            self.open(&p);
                            self.path = None; // recovered work is saved to a new file
                            self.dirty = true;
                        }
                        done = true;
                    }
                    if ui.button("Discard").clicked() { Self::clear_recovery(); done = true; }
                });
            });
            if done { self.recovery_offer = false; }
        }

        self.status_bar(root, ctx);
        if self.backstage(root, ctx) { return; }

        let Some(proj) = self.project.as_ref() else {
            egui::CentralPanel::default().show(root, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(120.0);
                    ui.heading("BrainBend Studio");
                    ui.label("Open a .brainbend, .story or .pptx file, or start from scratch.");
                    ui.add_space(12.0);
                    if ui.button("New project").clicked() { self.new_project(); }
                });
            });
            return;
        };
        if proj.is_web() {
            self.lesson_ui(root, ctx);
            return;
        }
        let slide_count = proj.slides.len();
        let _slide_idx = self.slide.min(slide_count - 1);

        if self.pano_edit_ui(root, ctx) { return; }
        if self.scene_view {
            egui::CentralPanel::default().show(root, |ui| self.scene_view_ui(ui, ctx));
            return;
        }

        egui::Panel::left("slides").default_size(240.0).show(root, |ui| {
            ui.add_space(6.0);
            if self.in_master_view() {
                self.master_panel(ui, ctx);
                return;
            }
            let cur_scene = self.current_scene();
            let mut goto_scene: Option<usize> = None;
            let mut new_scene = false;
            {
                let proj = self.project.as_ref().unwrap();
                let names: Vec<String> = proj.scenes.iter().map(|c| match c.bank { Some(b) => format!("{} (bank: {})", c.name, if b.draw == 0 { "all".to_string() } else { format!("draw {}", b.draw) }), None => c.name.clone() }).collect();
                combo("scene-picker").width(ui.available_width() - 8.0).selected_text(names.get(cur_scene).cloned().unwrap_or_default()).show_ui(ui, |ui| {
                    for (si, n) in names.iter().enumerate() {
                        if ui.selectable_label(si == cur_scene, n).clicked() { goto_scene = Some(si); }
                    }
                    ui.separator();
                    if ui.button("+ New scene").clicked() { new_scene = true; ui.close(); }
                });
            }
            if let Some(si) = goto_scene {
                if let Some(&i) = self.project.as_ref().unwrap().scene_slides(si).first() { self.open_slide(i); }
            }
            if new_scene { self.add_scene(); self.scene_view = false; }
            ui.add_space(4.0);
            let mut clicked: Option<usize> = None;
            let fit_w = ui.available_width() - 6.0;
            let list_h = (ui.available_height() - if self.slide_multi.is_empty() { 44.0 } else { 64.0 }).max(80.0);
            // Right-clicking between the slides adds one (or a scene); the thumbnails above this take their own clicks.
            let list_bg = ui.interact(egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), list_h)), egui::Id::new("slides-bg"), egui::Sense::click());
            list_bg.context_menu(|ui| self.new_slide_items(ui, Some(cur_scene)));
            egui::ScrollArea::both().id_salt("slides").max_height(list_h).auto_shrink([false, false]).show(ui, |ui| {
                let (sel, opn) = self.scene_flow(ui, ctx, cur_scene, 64.0, Some(fit_w));
                clicked = sel.or(opn);
            });
            if let Some(i) = clicked {
                self.slide = i; self.layer = 0; self.selected = None;
                self.reset_visibility();
            }
            if !self.slide_multi.is_empty() {
                ui.weak(format!("{} slides selected", self.slide_multi.len() + 1));
            }
            ui.horizontal(|ui| {
                ui.menu_button("+ Add slide", |ui| self.new_slide_items(ui, None));
                ui.menu_button("Slide", |ui| {
                    if ui.button("Duplicate").clicked() { self.duplicate_slides(); ui.close(); }
                    if ui.button("Move up").clicked() { self.move_slides(false); ui.close(); }
                    if ui.button("Move down").clicked() { self.move_slides(true); ui.close(); }
                    ui.separator();
                    ui.menu_button("Move to scene", |ui| {
                        let names: Vec<String> = self.project.as_ref().unwrap().scenes.iter().map(|c| c.name.clone()).collect();
                        let here = self.current_scene();
                        for (si, n) in names.iter().enumerate() {
                            if ui.add_enabled(si != here || !self.slide_multi.is_empty(), egui::Button::new(n)).clicked() { self.move_selected_slides_to_scene(si); ui.close(); }
                        }
                    });
                    ui.separator();
                    if ui.button(if self.slide_multi.is_empty() { "Delete" } else { "Delete selected slides" }).clicked() { self.pending_delete = Some(self.slide_selection()); ui.close(); }
                });
            });
        });

        if !self.import_report.is_empty() {
            let mut close = false;
            egui::Window::new("Import report").collapsible(false).resizable(true).anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0)).default_width(460.0).show(ctx, |ui| {
                ui.label(format!("The file opened, with {} note{} about things that could not be carried over exactly:", self.import_report.len(), if self.import_report.len() == 1 { "" } else { "s" }));
                ui.add_space(4.0);
                egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                    for n in &self.import_report { ui.label(format!("• {n}")); }
                });
                ui.add_space(6.0);
                ui.weak("This report is shown once and is not saved with the project.");
                if ui.button("OK").clicked() { close = true; }
            });
            if close { self.import_report.clear(); }
        }

        if !self.export_report.is_empty() {
            let mut close = false;
            egui::Window::new("PowerPoint export").collapsible(false).resizable(true).anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0)).default_width(460.0).show(ctx, |ui| {
                ui.label("The file was saved. These things have no PowerPoint equivalent and were left out or simplified:");
                ui.add_space(4.0);
                for n in &self.export_report { ui.label(format!("• {n}")); }
                ui.add_space(6.0);
                if ui.button("OK").clicked() { close = true; }
            });
            if close { self.export_report.clear(); }
        }

        // Inspector
        // The left panel can add or delete slides; re-read the index before anything else uses it.
        let slide_idx = self.slide.min(self.project.as_ref().map(|p| p.slides.len()).unwrap_or(1).saturating_sub(1));
        let mut want_captions: Option<(usize, usize)> = None;
        let mut want_equation: Option<(usize, usize)> = None;
        let mut want_pano: Option<(usize, usize)> = None;
        let mut want_thumb: Option<(usize, usize)> = None;
        let mut open_states = false;
        egui::Panel::right("inspector").default_size(326.0).show(root, |ui| {
            ui.add_space(6.0);
            let editable = !self.try_mode;
            // Picking an object, a layer or another slide shows its Properties: Triggers is somewhere to go on purpose, and
            // staying on it hid the Properties from anyone who did not know to look. Picking an owner from the Triggers tab
            // stays put.
            let seen = (self.selected, self.layer, self.slide);
            let from_triggers = std::mem::take(&mut self.trigger_select);
            if self.inspector_seen.is_some_and(|before| before != seen) && !from_triggers { self.right_tab = 0; }
            self.inspector_seen = Some(seen);
            ui.add_enabled_ui(editable, |ui| {
            if !self.in_master_view() {
                egui::Panel::bottom("layers-panel").resizable(true).default_size(250.0).min_size(130.0).show(ui, |ui| {
                    self.layers_panel(ui, ctx, slide_idx);
                });
            }
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.right_tab, 0, "Properties");
                let owner_id = self.selected_object().map(|o| o.id.clone());
                let count = {
                    let sl = &self.project.as_ref().unwrap().slides[slide_idx];
                    let lid = sl.layers[self.layer.min(sl.layers.len() - 1)].id.clone();
                    sl.triggers.iter().filter(|t| match (&t.owner, &owner_id) {
                        (Owner::Object(a), Some(b)) => a == b,
                        (Owner::Layer(l), None) => self.layer > 0 && *l == lid,
                        (Owner::Slide, None) => self.layer == 0,
                        _ => false,
                    }).count()
                };
                ui.selectable_value(&mut self.right_tab, 1, if count > 0 { format!("Triggers ({count})") } else { "Triggers".to_string() });
            });
            ui.separator();
            if self.right_tab == 1 {
                self.triggers_panel(ui, ctx, slide_idx);
                return;
            }
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let n_sel = self.selection().len();
            if n_sel > 1 && self.selected_object().is_some_and(|o| o.table.is_some()) {
                self.table_panel(ui);
                return;
            }
            if n_sel > 1 {
                ui.label(format!("{n_sel} objects selected"));
                ui.label("Drag to move them together. Shift-click adds or removes one. Formatting and alignment are on the Home tab.");
                return;
            }
            let Some((li, oi)) = self.selected else {
                ui.label("Nothing selected. Click an object on the canvas.");
                let mut ctr = self.id_counter;
                let t0 = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
                let mut gen = || { ctr += 1; format!("ch-{t0:x}-{ctr}") };
                let layer_idx = self.layer;
                let proj = self.project.as_mut().unwrap();
                let before = proj.slides.clone();
                let mut changed = false;
                let mut want_asset: Option<AssetWant> = None;
                let number_vars: Vec<(String, String)> = proj.variables.iter().filter(|v| !v.built_in && matches!(v.initial, Value::Num(_))).map(|v| (v.id.clone(), v.name.clone())).collect();
                let mut delete_layer = None;
                ui.separator();
                section_label(ui, "Slide");
                ui.horizontal(|ui| {
                    ui.label("Name");
                    changed |= ui.text_edit_singleline(&mut proj.slides[slide_idx].name).changed();
                });
                let (slide_w, slide_h) = (proj.slides[slide_idx].width, proj.slides[slide_idx].height);
                match &mut proj.slides[slide_idx].kind {
                    SlideKind::Question(q) => {
                        egui::ScrollArea::vertical().id_salt("qedit").max_height(440.0).show(ui, |ui| {
                            changed |= question_editor(ui, q, &mut gen, &mut want_asset, &number_vars);
                            changed |= question_area_editor(ui, q, slide_w, slide_h);
                        });
                    }
                    SlideKind::Results => {
                        ui.label("Shows the quiz score, pass or fail, and a Retry button. Put it after the last question.");
                    }
                    SlideKind::Normal => {}
                }
                ui.separator();
                section_label(ui, "Layer");
                if let Some(layer) = proj.slides[slide_idx].layers.get_mut(layer_idx) {
                    ui.horizontal(|ui| {
                        ui.label("Name");
                        changed |= ui.text_edit_singleline(&mut layer.name).changed();
                    });
                    if layer_idx > 0 {
                        changed |= ui.checkbox(&mut layer.modal, "Block clicks on the layers beneath").on_hover_text("While this layer shows, learners cannot click the base layer").changed();
                        changed |= ui.checkbox(&mut layer.hide_others, "Hide the slide's other layers when it shows").changed();
                        changed |= ui.checkbox(&mut layer.pause_base, "Pause the base layer while it shows").on_hover_text("The slide's timeline and its video or audio stop, and run on when this layer hides").changed();
                    }
                    if layer_idx > 0 && ui.button("Delete layer").clicked() { delete_layer = Some(layer_idx); }
                }
                ui.separator();
                section_label(ui, "Timeline");
                {
                    let last = proj.slides[slide_idx].layers.len() - 1;
                    let li = layer_idx.min(last);
                    ui.horizontal(|ui| {
                        let layer = &mut proj.slides[slide_idx].layers[li];
                        let mut timed = layer.duration_ms.is_some();
                        if ui.checkbox(&mut timed, if li == 0 { "Slide length" } else { "Layer length" }).on_hover_text("Length of the timeline; timed objects follow it").changed() {
                            layer.duration_ms = if timed { Some(5000) } else { None };
                            changed = true;
                        }
                        if let Some(d) = layer.duration_ms.as_mut() {
                            changed |= ui.add(egui::DragValue::new(d).range(250..=3_600_000).speed(50.0).suffix(" ms")).changed();
                        }
                    });
                    if li == 0 {
                        changed |= ui.checkbox(&mut proj.slides[slide_idx].auto_advance, "Go to the next slide when it ends").changed();
                    }
                }
                ui.separator();
                section_label(ui, "Navigation");
                {
                    let nav = &mut proj.slides[slide_idx].nav;
                    ui.horizontal(|ui| {
                        ui.label("Next button");
                        let label = match nav.next { NextRule::Always => "always works", NextRule::AfterTimeline => "after the timeline ends", NextRule::Never => "never works" };
                        combo("nav-next").selected_text(label).show_ui(ui, |ui| {
                            for (r, l) in [(NextRule::Always, "always works"), (NextRule::AfterTimeline, "after the timeline ends"), (NextRule::Never, "never works")] {
                                if ui.selectable_label(nav.next == r, l).clicked() { nav.next = r; changed = true; }
                            }
                        });
                    });
                    changed |= ui.checkbox(&mut nav.previous_allowed, "Previous button works").changed();
                    ui.horizontal(|ui| {
                        ui.label("On return");
                        let label = match nav.revisit { Revisit::Reset => "start over", Revisit::Resume => "resume where left" };
                        combo("nav-revisit").selected_text(label).show_ui(ui, |ui| {
                            for (r, l) in [(Revisit::Reset, "start over"), (Revisit::Resume, "resume where left")] {
                                if ui.selectable_label(nav.revisit == r, l).clicked() { nav.revisit = r; changed = true; }
                            }
                        });
                    });
                }
                // Random questions: a scene can show each learner a different selection of its question slides.
                let scene_id = proj.slides[slide_idx].scene.clone();
                let questions = proj.slides.iter().filter(|s| s.scene == scene_id && matches!(s.kind, SlideKind::Question(_))).count();
                if questions > 0 {
                    ui.separator();
                    section_label(ui, "Random questions");
                    if let Some(sc) = proj.scenes.iter_mut().find(|c| c.id == scene_id) {
                        let mut on = sc.bank.is_some();
                        if ui.checkbox(&mut on, "Show a random selection of this scene's questions").on_hover_text("Each attempt draws different question slides from this scene, so two learners (or two tries) do not see the same quiz. The score counts only the questions shown.").changed() {
                            sc.bank = if on { Some(Bank { draw: (questions as u32).min(5).max(1), shuffle: false }) } else { None };
                            changed = true;
                        }
                        if let Some(b) = sc.bank.as_mut() {
                            ui.horizontal(|ui| {
                                ui.label("Show");
                                changed |= ui.add(egui::DragValue::new(&mut b.draw).range(0..=999).speed(0.2)).changed();
                                ui.label(format!("of {questions} (0 = all)"));
                            });
                            changed |= ui.checkbox(&mut b.shuffle, "Shuffle the order").changed();
                        }
                    }
                }

                self.id_counter = ctr;
                if changed {
                    let editing = ctx.input(|i| i.pointer.any_down()) || ctx.memory(|m| m.focused().is_some());
                    if !(self.edit_open && editing) {
                        self.push_undo(before);
                    }
                    self.edit_open = true;
                    self.dirty = true;
                }
                if let Some(l) = delete_layer { self.delete_layer(l); }
                if let Some(w) = want_asset {
                    let key = match w {
                        AssetWant::Image => self.load_image_asset().map(|(k, _, _, _)| k),
                        AssetWant::Audio => self.load_media(&["mp3", "wav", "ogg", "m4a"], "Audio").map(|(k, _)| k),
                    };
                    if let Some(key) = key {
                        self.snapshot();
                        if let Some(SlideKind::Question(q)) = self.project.as_mut().map(|p| &mut p.slides[slide_idx].kind) { q.style.set_asset(key); }
                        self.dirty = true;
                    }
                }
                return;
            };
            let layer_choices: Vec<(String, String)> = self.project.as_ref().unwrap().slides[slide_idx].layers.iter().map(|l| (l.id.clone(), l.name.clone())).collect();
            let text_vars: Vec<(String, String)> = self.project.as_ref().unwrap().variables.iter().filter(|v| !v.built_in && matches!(v.initial, Value::Text(_))).map(|v| (v.id.clone(), v.name.clone())).collect();
            let num_vars: Vec<(String, String)> = self.project.as_ref().unwrap().variables.iter().filter(|v| !v.built_in && matches!(v.initial, Value::Num(_))).map(|v| (v.id.clone(), v.name.clone())).collect();
            let proj = self.project.as_mut().unwrap();
            // Slides are small (assets live on the project), so a per-frame copy is cheap.
            let before = proj.slides.clone();
            let Some(obj) = proj.slides[slide_idx].layers.get_mut(li).and_then(|l| l.objects.get_mut(oi)) else { return; };
            ui.label(egui::RichText::new(kind_name(&obj.kind)).weak());
            let mut changed = false;
            ui.horizontal(|ui| {
                ui.label("Name");
                changed |= ui.add(egui::TextEdit::singleline(&mut obj.name).desired_width(f32::INFINITY)).changed();
            });
            ui.horizontal(|ui| {
                ui.label("Alt text");
                changed |= ui.add(egui::TextEdit::singleline(&mut obj.alt).hint_text("describe for screen readers").desired_width(f32::INFINITY)).changed();
            });
            changed |= ui.checkbox(&mut obj.decorative, "Decorative (screen readers skip it)").changed();
            if !matches!(obj.kind, Kind::Image | Kind::Video | Kind::Audio | Kind::Hotspot | Kind::Line | Kind::LineUp | Kind::Path(_) | Kind::Web(_) | Kind::Pano(_) | Kind::Input(_) | Kind::Unsupported(_)) {
                ui.horizontal(|ui| {
                    ui.label("Read as");
                    combo("text-role").selected_text(obj.role.label()).show_ui(ui, |ui| {
                        for r in TextRole::ALL {
                            changed |= ui.selectable_value(&mut obj.role, r, r.label()).changed();
                        }
                    }).response.on_hover_text("What screen readers announce this text as: a heading (a Heading 1 is the page's main heading) or ordinary text");
                });
            }
            let mut pick_captions = false;
            if matches!(obj.kind, Kind::Video) {
                ui.horizontal(|ui| {
                    ui.label(if obj.captions.is_some() { "Captions: attached" } else { "Captions: none" });
                    if ui.button("Choose .vtt…").clicked() { pick_captions = true; }
                    if obj.captions.is_some() && ui.small_button("Remove").clicked() { obj.captions = None; changed = true; }
                });
            }
            if pick_captions { want_captions = Some((li, oi)); }
            if let Some(e) = &obj.equation {
                ui.horizontal(|ui| {
                    ui.label("Equation");
                    if ui.button("Edit equation…").clicked() { want_equation = Some((li, oi)); }
                });
                ui.label(egui::RichText::new(&e.latex).monospace().size(12.0).weak());
                ui.weak("Its colour is Shape Fill; resize it like any shape. Screen readers get the equation as MathML.");
            }
            if let Some(m) = obj.marker.as_mut() {
                ui.horizontal(|ui| {
                    ui.label("Title");
                    changed |= ui.add(egui::TextEdit::singleline(&mut m.title).desired_width(f32::INFINITY).hint_text("shown when the marker is hovered")).changed();
                });
                ui.label("Description");
                changed |= ui.add(egui::TextEdit::multiline(&mut m.description).desired_rows(4).desired_width(f32::INFINITY).hint_text("shown in a card when the marker is clicked")).changed();
                ui.horizontal(|ui| {
                    ui.label("Card");
                    changed |= ui.add(egui::DragValue::new(&mut m.card_w).range(160.0..=1800.0).speed(2.0).suffix(" wide")).changed();
                    changed |= ui.add(egui::DragValue::new(&mut m.card_h).range(100.0..=1000.0).speed(2.0).suffix(" high")).changed();
                });
                ui.weak("The title shows on hover or keyboard focus; clicking opens the card, and clicking anywhere else closes it. The card is shown here while the marker is selected.");
            }
            if let Kind::Input(f) = &mut obj.kind {
                changed |= controls::input_editor(ui, f, &text_vars, &num_vars);
                ui.weak("What the learner types is kept in the variable; triggers can react when it changes. Add alt text above: it is the field's label for screen readers.");
            }
            if let Kind::Pano(p) = &obj.kind {
                ui.horizontal(|ui| {
                    ui.label("360° image");
                    if ui.button("Edit markers and hotspots…").clicked() { want_pano = Some((li, oi)); }
                });
                ui.weak(format!("{} annotation{}. Starts looking {:.0}° across, {:.0}° up, zoom {:.0}°{}.", p.pins.len(), if p.pins.len() == 1 { "" } else { "s" }, p.yaw, p.pitch, p.fov, if p.auto_rotate { ", turning slowly" } else { "" }));
                ui.weak("Add alt text above: it describes the scene for screen readers. The learner can look around with the mouse, touch or the arrow keys.");
            }
            if let Kind::Web(w) = &mut obj.kind {
                ui.horizontal(|ui| {
                    ui.label("Web address");
                    changed |= ui.add(egui::TextEdit::singleline(&mut w.url).hint_text("https://example.com").desired_width(f32::INFINITY)).changed();
                });
                changed |= ui.checkbox(&mut w.scroll, "Learner can scroll the page").changed();
                if !w.url.trim().is_empty() && w.safe_url().is_none() { ui.colored_label(ui.visuals().warn_fg_color, "The address must start with https:// (or http://)."); }
                else if w.url.trim_start().starts_with("http://") { ui.colored_label(ui.visuals().warn_fg_color, "http:// pages are blocked inside https:// courses by most browsers. Use https:// if the site has it."); }
                ui.weak("Some sites refuse to be shown inside another page. Add alt text above: it names the frame for screen readers.");
            }
            if matches!(obj.kind, Kind::Video | Kind::Audio) {
                egui::CollapsingHeader::new("Playback").default_open(true).show(ui, |ui| {
                    let m = &mut obj.media;
                    changed |= ui.checkbox(&mut m.autoplay, "Start automatically").on_hover_text("Plays when the object's turn on the timeline comes; otherwise the learner presses play").changed();
                    if matches!(obj.kind, Kind::Video) {
                        changed |= ui.checkbox(&mut m.controls, "Show player controls").changed();
                    }
                    changed |= ui.checkbox(&mut m.looped, "Loop").changed();
                    changed |= ui.checkbox(&mut m.muted, "Muted").changed();
                    ui.horizontal(|ui| {
                        ui.label("Volume");
                        let mut pct = (m.volume * 100.0).round();
                        if ui.add(egui::Slider::new(&mut pct, 0.0..=100.0).suffix("%")).changed() { m.volume = pct / 100.0; changed = true; }
                    });
                });
            }
            if !matches!(obj.kind, Kind::Image | Kind::Video | Kind::Audio | Kind::Hotspot | Kind::Line | Kind::LineUp | Kind::Path(_) | Kind::Slider(_) | Kind::Web(_) | Kind::Pano(_) | Kind::Input(_) | Kind::Unsupported(_)) {
                let old_text = obj.text.clone();
                ui.label("Text");
                let mut buf = obj.text.clone();
                if ui.add(egui::TextEdit::multiline(&mut buf).desired_rows(4).desired_width(f32::INFINITY)).changed() {
                    // Characters you did not touch keep their formatting.
                    obj.edit_text(&buf);
                    // Objects with states (buttons) draw their text from the initial state, so keep it in step.
                    let (new_text, new_rich, init) = (obj.text.clone(), obj.rich.clone(), obj.initial_state.clone());
                    for s in obj.states.iter_mut() {
                        if s.text == old_text || s.name == init { s.text = new_text.clone(); s.rich = new_rich.clone(); }
                    }
                    changed = true;
                }
                ui.weak("Font, color and alignment are on the Home tab. To format part of the text, double-click it on the slide, select words, and press ⌘B / ⌘I / ⌘U.");
            }
            egui::CollapsingHeader::new("Size and position").default_open(false).show(ui, |ui| {
                egui::Grid::new("geom").show(ui, |ui| {
                    for (label, v) in [("X", &mut obj.x), ("Y", &mut obj.y), ("Width", &mut obj.w), ("Height", &mut obj.h)] {
                        ui.label(label);
                        changed |= ui.add(egui::DragValue::new(v).speed(1.0)).changed();
                        ui.end_row();
                    }
                    ui.label("Rotation");
                    changed |= ui.add(egui::DragValue::new(&mut obj.rotation).speed(1.0).range(0.0..=359.0).suffix("°")).on_hover_text("Turns the object clockwise about its centre").changed();
                    ui.end_row();
                });
            });
            if let Kind::Slider(sl) = &mut obj.kind {
                let mut pick_thumb = false;
                egui::CollapsingHeader::new("Slider").default_open(true).show(ui, |ui| { changed |= controls::slider_editor(ui, sl, &num_vars, &mut pick_thumb); });
                if pick_thumb { want_thumb = Some((li, oi)); }
            }
            if matches!(obj.kind, Kind::Text | Kind::Rect) {
                egui::CollapsingHeader::new("Check box").default_open(obj.check.is_some()).show(ui, |ui| { changed |= controls::check_editor(ui, obj); });
            }
            if !matches!(obj.kind, Kind::Video | Kind::Audio | Kind::Hotspot | Kind::Line | Kind::LineUp | Kind::Slider(_) | Kind::Web(_) | Kind::Pano(_) | Kind::Input(_) | Kind::Unsupported(_)) {
                egui::CollapsingHeader::new("Dial").default_open(obj.dial.is_some()).show(ui, |ui| { changed |= controls::dial_editor(ui, &mut obj.dial, &num_vars); });
            }
            if li == 0 && layer_choices.len() > 1 {
                egui::CollapsingHeader::new("Hide when layers show").default_open(!obj.hide_on_layers.is_empty()).show(ui, |ui| {
                    ui.weak("This object is not drawn while any ticked layer is showing.");
                    for (lid, lname) in layer_choices.iter().skip(1) {
                        let mut on = obj.hide_on_layers.contains(lid);
                        if ui.checkbox(&mut on, lname).changed() {
                            if on { obj.hide_on_layers.push(lid.clone()); } else { obj.hide_on_layers.retain(|x| x != lid); }
                            changed = true;
                        }
                    }
                });
            }
            if states_panel::supports_states(obj) && ui.button(format!("States ({})…", obj.states.len())).on_hover_text("Edit this object's states in the States tab").clicked() {
                open_states = true;
            }
            if states_panel::supports_states(obj) && !obj.states.is_empty() {
                ui.horizontal(|ui| {
                    ui.label("Button set").on_hover_text("Buttons that share a name work as a set: pressing one selects it and releases the others");
                    let mut set = obj.button_set.clone().unwrap_or_default();
                    if ui.add(egui::TextEdit::singleline(&mut set).desired_width(120.0).hint_text("none")).changed() {
                        obj.button_set = (!set.trim().is_empty()).then_some(set);
                        changed = true;
                    }
                });
            }
            if changed {
                // One undo step per continuous edit (drag or typing burst).
                let editing = ctx.input(|i| i.pointer.any_down()) || ctx.memory(|m| m.focused().is_some());
                if !(self.edit_open && editing) {
                    self.push_undo(before);
                }
                self.edit_open = true;
                self.dirty = true;
            } else if !ctx.input(|i| i.pointer.any_down()) && ctx.memory(|m| m.focused().is_none()) {
                self.edit_open = false;
            }
            });
                    });
        });

        if open_states { self.bottom_tab = 2; }
        if let Some((li, oi)) = want_thumb {
            if let Some((key, _, _, _)) = self.load_image_asset() {
                self.snapshot();
                if let Some(Kind::Slider(sl)) = self.project.as_mut().and_then(|p| p.slides[slide_idx].layers.get_mut(li)).and_then(|l| l.objects.get_mut(oi)).map(|o| &mut o.kind) {
                    sl.thumb = Some(key);
                }
            }
        }
        if let Some(t) = want_equation { self.open_equation_dialog(Some(t)); }
        if let Some(t) = want_pano { self.open_pano_dialog(t); }
        if let Some((li, oi)) = want_captions {
            if let Some((key, _)) = self.load_media(&["vtt"], "WebVTT captions") {
                self.snapshot();
                if let Some(o) = self.project.as_mut().and_then(|p| p.slides[slide_idx].layers.get_mut(li)).and_then(|l| l.objects.get_mut(oi)) {
                    o.captions = Some(key);
                }
            }
        }

        egui::Panel::bottom("timeline").resizable(true).default_size(200.0).show(root, |ui| {
            let editable = !self.try_mode;
            ui.add_enabled_ui(editable, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.bottom_tab, 0, "Timeline");
                ui.selectable_value(&mut self.bottom_tab, 1, "Slide Notes");
                ui.selectable_value(&mut self.bottom_tab, 2, "States");
            });
            if self.bottom_tab == 0 {
                self.timeline_panel(ui, slide_idx);
            } else if self.bottom_tab == 2 {
                self.states_tab(ui, slide_idx);
            } else {
                self.notes_panel(ui, slide_idx);
            }
                    });
        });

        egui::CentralPanel::default().show(root, |ui| {
            let slide = self.project.as_ref().unwrap().slides[slide_idx].clone();
            let avail = ui.available_size();
            let (resp, painter) = ui.allocate_painter(avail, egui::Sense::click_and_drag());
            let area = resp.rect;
            let painter = painter.with_clip_rect(area);
            painter.rect_filled(area, 0.0, if ui.visuals().dark_mode { egui::Color32::from_gray(42) } else { egui::Color32::from_gray(222) });
            let fit = ((avail.x - 40.0) / slide.width).min((avail.y - 40.0) / slide.height).max(0.02);
            self.fit_scale = fit;

            // Zoom (pinch or ⌘/Ctrl+scroll) about the pointer; plain scroll pans; middle-drag pans.
            if resp.hovered() {
                let (scroll, zd, pointer) = ctx.input(|i| (i.smooth_scroll_delta, i.zoom_delta(), i.pointer.hover_pos()));
                if zd != 1.0 {
                    if let Some(p) = pointer {
                        let old_scale = fit * self.zoom;
                        let old_origin = area.center() - egui::vec2(slide.width, slide.height) * old_scale / 2.0 + self.pan;
                        let q = (p - old_origin) / old_scale;
                        self.zoom = (self.zoom * zd).clamp(0.1, 8.0);
                        let new_scale = fit * self.zoom;
                        self.pan = p - q * new_scale - area.center() + egui::vec2(slide.width, slide.height) * new_scale / 2.0;
                    }
                } else if scroll != egui::Vec2::ZERO {
                    self.pan += scroll;
                }
            }
            if resp.dragged_by(egui::PointerButton::Middle) {
                self.pan += resp.drag_delta();
            }
            let scale = fit * self.zoom;
            let size = egui::vec2(slide.width * scale, slide.height * scale);
            let origin = area.center() - size / 2.0 + self.pan;
            self.canvas_frame = Some((origin, scale));
            self.paint_backdrop(ctx, &painter, origin, scale, &slide);
            let to_screen = |x: f32, y: f32, w: f32, h: f32| {
                egui::Rect::from_min_size(origin + egui::vec2(x, y) * scale, egui::vec2(w, h) * scale)
            };

            if !matches!(slide.kind, SlideKind::Normal) {
                // Question and results slides are drawn by the player; this mirrors its layout.
                let rect = egui::Rect::from_min_size(origin, size);
                let qarea = match &slide.kind { SlideKind::Question(q) => q.area, _ => None };
                if qarea.is_none() { painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(244, 246, 249)); }
                // With its own area the question is laid out at the player's scaled size, from the area's top-left.
                let qk = qarea.map(|a| (a.w / 1560.0).min(a.h / 860.0).clamp(0.3, 1.0)).unwrap_or(1.0);
                let area_screen = qarea.map(|a| egui::Rect::from_min_size(origin + egui::vec2(a.x, a.y) * scale, egui::vec2(a.w, a.h) * scale));
                let slide_scale = scale;
                let (origin, scale) = match qarea {
                    Some(a) => (origin + egui::vec2(a.x + 30.0, a.y + 24.0) * scale - egui::vec2(180.0, 110.0) * scale * qk, scale * qk),
                    None => (origin, scale),
                };
                let at = |x: f32, y: f32| origin + egui::vec2(x, y) * scale;
                let txt = |painter: &egui::Painter, x: f32, y: f32, text: String, px: f32, weight: f32, col: egui::Color32, width: f32| -> f32 {
                    let fmt = egui::text::TextFormat {
                        font_id: egui::FontId::new((px * scale).max(1.0), egui::FontFamily::Name("Open Sans".into())),
                        color: col,
                        coords: egui::epaint::text::VariationCoords::new([(b"wght", weight)]),
                        ..Default::default()
                    };
                    let mut job = egui::text::LayoutJob::single_section(text, fmt);
                    job.wrap.max_width = (width * scale).max(1.0);
                    let g = painter.layout_job(job);
                    let h = g.size().y / scale;
                    painter.galley(at(x, y), g, col);
                    h
                };
                let ink = egui::Color32::from_rgb(27, 42, 58);
                let blue = egui::Color32::from_rgb(3, 83, 158);
                let content_w = match qarea { Some(a) => (a.w - 60.0) / qk, None => slide.width - 360.0 };
                match &slide.kind {
                    SlideKind::Question(q) => {
                        txt(&painter, 180.0, 110.0, "QUESTION 1 OF 1".into(), 30.0, 400.0, egui::Color32::from_rgb(91, 107, 126), content_w);
                        let ph = txt(&painter, 180.0, 164.0, q.prompt.clone(), 58.0, 700.0, ink, content_w);
                        let mut y = 164.0 + ph + 48.0;
                        // Non-choice styles: one row per input, in the player's look.
                        let rows: Vec<String> = match &q.style {
                            QuestionStyle::TextEntry { .. } => vec!["Type your answer here".to_string()],
                            QuestionStyle::Matching { pairs } => pairs.iter().map(|(l, _)| format!("{l}      [ Choose… ]")).collect(),
                            QuestionStyle::Sequence { items } => items.iter().enumerate().map(|(i, t)| format!("{}.  {}", i + 1, t)).collect(),
                            QuestionStyle::FillBlanks { text, .. } => vec![brainbend_doc::passage::with_gaps(text)],
                            QuestionStyle::DragWords { text, distractors } => {
                                let mut bank: Vec<String> = brainbend_doc::passage::blanks(text).into_iter().map(|a| a[0].clone()).collect();
                                bank.extend(distractors.iter().cloned());
                                vec![brainbend_doc::passage::with_gaps(text), format!("Word bank:  {}", bank.join("   "))]
                            }
                            QuestionStyle::MarkWords { text } => vec![brainbend_doc::passage::answered(text)],
                            QuestionStyle::Numeric { .. } => vec!["Type a number".to_string()],
                            QuestionStyle::Categorize { categories } => categories.iter().map(|(n, items)| format!("{n}:  {}", items.join(",  "))).collect(),
                            QuestionStyle::Hotspot { .. } | QuestionStyle::LabelPicture { .. } => vec![],
                            QuestionStyle::Likert { statements, scale, .. } => statements.iter().map(|t| format!("{t}      {}", scale.iter().map(|_| "( )").collect::<Vec<_>>().join("  "))).collect(),
                            QuestionStyle::Reflection { .. } => vec!["Type your answer here".to_string()],
                            QuestionStyle::Dictation { .. } => vec!["[ play audio ]".to_string(), "Type what you hear".to_string()],
                            QuestionStyle::Dropdowns { text } => vec![brainbend_doc::passage::parse(text).into_iter().map(|p| match p { brainbend_doc::passage::Part::Text(t) => t, brainbend_doc::passage::Part::Blank(_) => "[ Choose… ]".to_string() }).collect::<String>()],
                            QuestionStyle::Choice => vec![],
                        };
                        for r in &rows {
                            let row = egui::Rect::from_min_size(at(180.0, y), egui::vec2(content_w, 100.0) * scale);
                            painter.rect_filled(row, 16.0 * scale, egui::Color32::WHITE);
                            painter.rect_stroke(row, 16.0 * scale, egui::Stroke::new((3.0 * scale).max(1.0), egui::Color32::from_rgb(184, 196, 214)), egui::StrokeKind::Inside);
                            txt(&painter, 180.0 + 34.0, y + 26.0, r.clone(), 40.0, 400.0, ink, content_w - 80.0);
                            y += 122.0;
                        }
                        for c in &q.choices {
                            let row = egui::Rect::from_min_size(at(180.0, y), egui::vec2(content_w, 100.0) * scale);
                            painter.rect_filled(row, 16.0 * scale, egui::Color32::WHITE);
                            let edge = if c.correct { egui::Color32::from_rgb(30, 142, 62) } else { egui::Color32::from_rgb(184, 196, 214) };
                            painter.rect_stroke(row, 16.0 * scale, egui::Stroke::new((3.0 * scale).max(1.0), edge), egui::StrokeKind::Inside);
                            let mark = at(180.0 + 34.0 + 20.0, y + 50.0);
                            if q.multiple {
                                painter.rect_stroke(egui::Rect::from_center_size(mark, egui::Vec2::splat(40.0 * scale)), 6.0 * scale, egui::Stroke::new((3.0 * scale).max(1.0), blue), egui::StrokeKind::Inside);
                            } else {
                                painter.circle_stroke(mark, 20.0 * scale, egui::Stroke::new((3.0 * scale).max(1.0), blue));
                            }
                            if c.correct {
                                painter.circle_filled(mark, 9.0 * scale, egui::Color32::from_rgb(30, 142, 62));
                            }
                            txt(&painter, 180.0 + 34.0 + 40.0 + 28.0, y + 26.0, c.text.clone(), 40.0, 400.0, ink, content_w - 140.0);
                            y += 122.0;
                        }
                        let picture: Option<(&Option<String>, Vec<(HotRegion, String)>)> = match &q.style {
                            QuestionStyle::Hotspot { asset, regions, .. } => Some((asset, regions.iter().map(|r| (*r, String::new())).collect())),
                            QuestionStyle::LabelPicture { asset, zones, .. } => Some((asset, zones.iter().map(|z| (z.region, z.label.clone())).collect())),
                            _ => None,
                        };
                        if let Some((asset, regions)) = picture {
                            // The picture, with its correct areas; dragging on it draws a new area.
                            let tex = asset.as_ref().and_then(|k| self.texture(ctx, k));
                            let room = egui::vec2(content_w, (slide.height - y - 190.0).clamp(160.0, 560.0));
                            let img = match &tex {
                                Some(t) => { let sz = t.size_vec2(); let k = (room.x / sz.x).min(room.y / sz.y); sz * k }
                                None => egui::vec2(room.x.min(900.0), room.y.min(420.0)),
                            };
                            let rect = egui::Rect::from_min_size(at(180.0, y), img * scale);
                            match &tex {
                                Some(t) => { painter.image(t.id(), rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE); }
                                None => {
                                    painter.rect_filled(rect, 8.0 * scale, egui::Color32::from_gray(225));
                                    painter.text(rect.center(), egui::Align2::CENTER_CENTER, "Choose a picture in the panel on the right", egui::FontId::proportional((30.0 * scale).max(10.0)), egui::Color32::from_gray(90));
                                }
                            }
                            let green = egui::Color32::from_rgb(30, 142, 62);
                            let to_rect = |r: &HotRegion| egui::Rect::from_min_size(rect.min + egui::vec2(r.x, r.y) * rect.size(), egui::vec2(r.w, r.h) * rect.size());
                            for (r, label) in &regions {
                                let rr = to_rect(r);
                                if !label.is_empty() { painter.text(rr.left_top() + egui::vec2(4.0, 2.0), egui::Align2::LEFT_TOP, label, egui::FontId::proportional((26.0 * scale).max(9.0)), green); }
                                if r.ellipse {
                                    painter.add(egui::epaint::EllipseShape { center: rr.center(), radius: rr.size() / 2.0, fill: green.gamma_multiply(0.25), stroke: egui::Stroke::new(2.0, green), angle: 0.0 });
                                } else {
                                    painter.rect(rr, 0.0, green.gamma_multiply(0.25), egui::Stroke::new(2.0, green), egui::StrokeKind::Inside);
                                }
                            }
                            let frac = |pos: egui::Pos2| { let d = (pos - rect.min) / rect.size(); (d.x.clamp(0.0, 1.0), d.y.clamp(0.0, 1.0)) };
                            if resp.drag_started() {
                                if let Some(pos) = ctx.input(|i| i.pointer.press_origin()).filter(|p| rect.contains(*p)) { self.hot_drag = Some((frac(pos), frac(pos))); }
                            }
                            if let (Some((start, _)), Some(pos)) = (self.hot_drag, resp.interact_pointer_pos()) { self.hot_drag = Some((start, frac(pos))); }
                            if let Some((a, b)) = self.hot_drag {
                                let band = egui::Rect::from_two_pos(rect.min + egui::vec2(a.0, a.1) * rect.size(), rect.min + egui::vec2(b.0, b.1) * rect.size());
                                painter.rect(band, 0.0, egui::Color32::from_rgba_unmultiplied(0, 120, 255, 40), egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 120, 255)), egui::StrokeKind::Inside);
                                if resp.drag_stopped() || !resp.dragged() {
                                    self.hot_drag = None;
                                    let (x0, x1, y0, y1) = (a.0.min(b.0), a.0.max(b.0), a.1.min(b.1), a.1.max(b.1));
                                    if x1 - x0 >= 0.02 && y1 - y0 >= 0.02 {
                                        self.snapshot();
                                        let region = HotRegion { x: x0, y: y0, w: x1 - x0, h: y1 - y0, ellipse: false };
                                        match self.project.as_mut().map(|p| &mut p.slides[slide_idx].kind) {
                                            Some(SlideKind::Question(Question { style: QuestionStyle::Hotspot { regions, .. }, .. })) => regions.push(region),
                                            Some(SlideKind::Question(Question { style: QuestionStyle::LabelPicture { zones, .. }, .. })) => { let n = zones.len() + 1; zones.push(LabelZone { region, label: format!("Label {n}") }); }
                                            _ => {}
                                        }
                                        self.dirty = true;
                                    }
                                }
                            }
                            y += img.y + 24.0;
                        }
                        let b = egui::Rect::from_min_size(at(180.0, y + 8.0), egui::vec2(300.0, 90.0) * scale);
                        painter.rect_filled(b, 14.0 * scale, blue);
                        painter.text(b.center(), egui::Align2::CENTER_CENTER, "Submit", egui::FontId::new((38.0 * scale).max(1.0), egui::FontFamily::Name("Open Sans".into())), egui::Color32::WHITE);
                    }
                    _ => {
                        txt(&painter, 180.0, 110.0, "Results".into(), 58.0, 700.0, ink, content_w);
                        txt(&painter, 180.0, 230.0, "86%".into(), 200.0, 800.0, blue, content_w);
                        txt(&painter, 180.0, 470.0, "6 of 7 points. Pass mark: 80%.".into(), 42.0, 400.0, ink, content_w);
                        txt(&painter, 180.0, 540.0, "You passed.".into(), 52.0, 700.0, egui::Color32::from_rgb(30, 142, 62), content_w);
                    }
                }
                painter.text(rect.right_bottom() - egui::vec2(12.0, 12.0), egui::Align2::RIGHT_BOTTOM,
                    "preview (sample score) - drawn by the player", egui::FontId::proportional(13.0), egui::Color32::GRAY);
                // The question's own area: a frame with a grip to move it and handles on its edges and corners to resize it.
                if let (Some(sr), false) = (area_screen, self.try_mode) {
                    let accent = egui::Color32::from_rgb(0, 140, 255);
                    painter.rect_stroke(sr, 0.0, egui::Stroke::new(1.5, accent), egui::StrokeKind::Outside);
                    let grip = egui::Rect::from_min_size(sr.left_top() - egui::vec2(0.0, 20.0), egui::vec2(120.0, 20.0));
                    painter.rect_filled(grip, 3.0, accent);
                    painter.text(grip.center(), egui::Align2::CENTER_CENTER, "Question area", egui::FontId::proportional(12.0), egui::Color32::WHITE);
                    let mut grabs: Vec<(String, egui::Rect, [f32; 4])> = vec![("move".into(), grip, [1.0, 1.0, 0.0, 0.0])];
                    // (dx of x, dy of y, dx of w, dy of h) per handle, in drag units.
                    let pts = [(0.0, 0.0, [1.0, 1.0, -1.0, -1.0]), (0.5, 0.0, [0.0, 1.0, 0.0, -1.0]), (1.0, 0.0, [0.0, 1.0, 1.0, -1.0]),
                               (0.0, 0.5, [1.0, 0.0, -1.0, 0.0]), (1.0, 0.5, [0.0, 0.0, 1.0, 0.0]),
                               (0.0, 1.0, [1.0, 0.0, -1.0, 1.0]), (0.5, 1.0, [0.0, 0.0, 0.0, 1.0]), (1.0, 1.0, [0.0, 0.0, 1.0, 1.0])];
                    for (i, (fx, fy, m)) in pts.iter().enumerate() {
                        let c = sr.min + egui::vec2(sr.width() * fx, sr.height() * fy);
                        let h = egui::Rect::from_center_size(c, egui::Vec2::splat(10.0));
                        painter.rect(h, 1.0, egui::Color32::WHITE, egui::Stroke::new(1.5, accent), egui::StrokeKind::Inside);
                        grabs.push((format!("h{i}"), h.expand(3.0), *m));
                    }
                    for (name, r, m) in grabs {
                        let id = egui::Id::new(("question-area", name.as_str()));
                        let g = ui.interact(r, id, egui::Sense::drag());
                        if name != "move" { g.clone().on_hover_cursor(egui::CursorIcon::ResizeNeSw); } else { g.clone().on_hover_cursor(egui::CursorIcon::Grab); }
                        if g.drag_started() { self.snapshot(); }
                        if g.dragged() {
                            let d = g.drag_delta() / slide_scale;
                            if let Some(SlideKind::Question(q)) = self.project.as_mut().map(|p| &mut p.slides[slide_idx].kind) {
                                if let Some(a) = q.area.as_mut() {
                                    a.x += d.x * m[0]; a.y += d.y * m[1]; a.w += d.x * m[2]; a.h += d.y * m[3];
                                    a.w = a.w.max(200.0); a.h = a.h.max(120.0);
                                }
                            }
                            self.dirty = true;
                        }
                    }
                }
            }
            let shown = self.shown_layers(slide.layers.len());
            // "Try it": the objects show the state a mouse would put them in, a press acts like the player's (it also
            // shows and hides layers and sets states), and nothing is saved.
            let try_on = self.try_mode;
            // Esc closes the unsupported-action dialog first; a second Esc stops Try It.
            if try_on && ctx.input(|i| i.key_pressed(egui::Key::Escape)) { if self.try_note.is_some() { self.try_note = None; } else { self.try_mode = false; } }
            let try_on = self.try_mode;
            if !try_on { self.try_slide = None; self.try_note = None; }
            let (slide, shown) = if try_on {
                if self.try_slide != Some(slide_idx) {
                    self.try_layers = states_panel::try_start(&slide);
                    self.try_states.clear();
                    self.try_note = None;
                    self.try_slide = Some(slide_idx);
                }
                let hidden = states_panel::hidden_by_layers(&slide, &self.try_layers);
                let mut tslide = slide.clone();
                tslide.layers[0].objects.retain(|o| !hidden.contains(&o.id));
                let tshown: Vec<usize> = std::iter::once(0).chain(slide.layers.iter().enumerate().skip(1).filter(|(_, l)| self.try_layers.contains(&l.id)).map(|(i, _)| i)).collect();
                let mut hit = resp.hover_pos().and_then(|p| hit_object(&tslide, &tshown, (p - origin) / scale, scale));
                // A showing layer that blocks the layers beneath it keeps presses from reaching them.
                let mut blocked_by: Option<usize> = None;
                if let Some(top_modal) = tshown.iter().copied().filter(|&i| i > 0 && tslide.layers[i].modal).max() {
                    if hit.is_some_and(|(l, _)| l < top_modal) { blocked_by = Some(top_modal); }
                    hit = hit.filter(|&(l, _)| l >= top_modal);
                }
                let hit_id = hit.and_then(|(l, o)| tslide.layers[l].objects.get(o).map(|ob| ob.id.clone()));
                let pressed = ctx.input(|i| i.pointer.primary_down());
                if resp.clicked() {
                    if let (Some(b), None) = (blocked_by, &hit_id) {
                        // The press landed on something a showing layer covers: say so, or it just seems dead.
                        self.try_note = Some(format!("The layer \"{}\" is showing and blocks clicks on what is beneath it. To let them through, select that layer and turn off \"Block clicks on the layers beneath\".", tslide.layers[b].name));
                        ctx.request_repaint();
                    } else if let Some(id) = &hit_id {
                        states_panel::try_press(&slide, id, &mut self.try_layers, &mut self.try_states);
                        let left_out = self.project.as_ref().map(|p| states_panel::not_run(&slide, &p.slides, id)).unwrap_or_default();
                        self.try_note = states_panel::not_run_note(&left_out);
                        ctx.request_repaint();
                    }
                }
                for l in tslide.layers.iter_mut() {
                    for o in l.objects.iter_mut() {
                        if o.states.is_empty() { continue; }
                        let under = hit_id.as_deref() == Some(o.id.as_str());
                        if let Some(st) = states_panel::tried_state(o, self.try_states.get(&o.id), under, under && pressed).cloned() {
                            states_panel::show_state(o, &st);
                        }
                    }
                }
                (tslide, tshown)
            } else {
                (slide, shown)
            };
            // Base objects a showing layer hides are not drawn (or hit) while it shows, as in the player.
            let slide = if try_on {
                slide
            } else {
                let ids: Vec<String> = shown.iter().filter(|&&i| i > 0).filter_map(|&i| slide.layers.get(i).map(|l| l.id.clone())).collect();
                let hide = states_panel::hidden_by_layers(&slide, &ids);
                if hide.is_empty() {
                    slide
                } else {
                    let mut s = slide;
                    for o in s.layers[0].objects.iter_mut().filter(|o| hide.contains(&o.id)) { o.hidden = true; }
                    s
                }
            };
            // The object being edited shows its text in the edit box alone, so it is not drawn twice.
            let slide = match self.editing.filter(|_| !try_on) {
                Some((li, oi)) if slide.layers.get(li).is_some_and(|l| oi < l.objects.len()) => {
                    let mut s = slide;
                    let o = &mut s.layers[li].objects[oi];
                    o.text.clear();
                    o.rich = None;
                    // The edit box cannot turn, so a turned object is shown upright (frame and handles too) while its
                    // text is edited. Only what is drawn changes: the saved rotation is untouched.
                    o.rotation = 0.0;
                    s
                }
                _ => slide,
            };
            // The States tab shows its object alone and brightly, with everything else faded behind it.
            let state_object = if self.bottom_tab == 2 && self.state_dim && self.swap.is_some() && !try_on { self.selected } else { None };
            if let Some((sli, soi)) = state_object.filter(|&(l, o)| slide.layers.get(l).is_some_and(|ly| o < ly.objects.len())) {
                let mut rest = slide.clone();
                let one = rest.layers[sli].objects.remove(soi);
                self.paint_slide(ctx, &painter, origin, scale, &rest, &shown, None);
                let bg = self.project.as_ref().and_then(|p| p.resolved_background(&slide)).map(|b| color(b.base_color())).unwrap_or(egui::Color32::WHITE);
                painter.rect_filled(egui::Rect::from_min_size(origin, egui::vec2(slide.width, slide.height) * scale), 0.0, bg.gamma_multiply(0.75));
                let mut only = slide.clone();
                for l in only.layers.iter_mut() { l.objects.clear(); }
                only.layers[sli].objects.push(one);
                self.paint_slide(ctx, &painter, origin, scale, &only, &[sli], Some((sli, 0)));
            } else if self.dim_base && self.layer > 0 && shown.len() > 1 && !try_on {
                // Working on a layer: show the base layer faded behind it.
                self.paint_slide(ctx, &painter, origin, scale, &slide, &[0], None);
                let bg = self.project.as_ref().and_then(|p| p.resolved_background(&slide)).map(|b| color(b.base_color())).unwrap_or(egui::Color32::WHITE);
                painter.rect_filled(egui::Rect::from_min_size(origin, egui::vec2(slide.width, slide.height) * scale), 0.0, bg.gamma_multiply(0.7));
                self.paint_slide(ctx, &painter, origin, scale, &slide, &shown[1..], self.selected);
            } else {
                self.paint_slide(ctx, &painter, origin, scale, &slide, &shown, if try_on { None } else { self.selected });
            }

            if try_on {
                // Nothing is selected, moved or saved while trying.
                let strip = egui::Rect::from_min_size(area.left_top(), egui::vec2(area.width(), 40.0));
                painter.rect_filled(strip, 0.0, egui::Color32::from_rgb(11, 110, 105).gamma_multiply(0.92));
                let font = egui::FontId::proportional(12.5);
                painter.text(strip.center_top() + egui::vec2(0.0, 12.0), egui::Align2::CENTER_CENTER, "Try It is Enabled. You may now test buttons, layers, and triggers on this slide only.", font.clone(), egui::Color32::WHITE);
                painter.text(strip.center_top() + egui::vec2(0.0, 29.0), egui::Align2::CENTER_CENTER, "Does not work: video, audio, animations, variables, JavaScript, etc. Use Preview for all functionality.", font, egui::Color32::from_white_alpha(215));
                // What the last press left out (or was blocked from) is a dialog that stays until the author presses OK.
                if let Some(note) = self.try_note.clone() {
                    let modal = egui::Modal::new(egui::Id::new("try_it_note")).show(ctx, |ui| {
                        ui.set_width(360.0);
                        ui.heading("Not run in Try It");
                        ui.add_space(6.0);
                        ui.label(note);
                        ui.add_space(12.0);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| ui.button("OK").clicked()).inner
                    });
                    if modal.inner { self.try_note = None; }
                }
                return;
            }
            let to_slide = |p: egui::Pos2| (p - origin) / scale;
            let sel_all = self.selection();
            // Crop mode ends when anything but the picture being cropped is selected.
            let cropping = self.crop_mode.as_deref().is_some_and(|id| sel_all.len() == 1 && self.selected_object().is_some_and(|o| o.id == id && matches!(o.kind, Kind::Image) && o.asset.is_some()));
            if !cropping { self.crop_mode = None; self.crop_aspect = None; }
            if cropping && ctx.input(|i| i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::Enter)) { self.crop_mode = None; self.crop_aspect = None; }
            let cropping = cropping && self.crop_mode.is_some();
            // Outline mode ends when anything but the hotspot being edited is selected.
            let outlining = self.outline_mode.as_deref().is_some_and(|id| sel_all.len() == 1 && self.selected_object().is_some_and(|o| o.id == id && matches!(o.kind, Kind::Hotspot)));
            if self.outline_mode.is_some() && !outlining { self.end_outline(); }
            if outlining && ctx.input(|i| i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::Enter)) { self.end_outline(); }
            let outlining = outlining && self.outline_mode.is_some();
            // Drawing a freeform hotspot is abandoned when the slide changes.
            if self.draw_outline.is_some() && self.draw_slide != slide_idx { self.draw_outline = None; }
            let drawing = self.draw_outline.is_some() || self.hot_drag.is_some();
            // A whole group (or table) gets one box around it instead of an outline per member.
            let one_group = sel_all.len() > 1 && sel_all.iter().all(|&(l, k)| slide.layers.get(l).and_then(|ly| ly.objects.get(k)).is_some_and(|o| o.group.is_some() && o.group == slide.layers[sel_all[0].0].objects[sel_all[0].1].group));
            if sel_all.len() > 1 && !one_group {
                for &(l, k) in &sel_all {
                    if let Some(o) = slide.layers.get(l).and_then(|ly| ly.objects.get(k)) {
                        painter.rect_stroke(to_screen(o.x, o.y, o.w, o.h), 0.0, egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 120, 255)), egui::StrokeKind::Outside);
                    }
                }
            }

            // Crop mode: the whole picture shows faintly around the part that is kept.
            if cropping {
                if let Some(o) = self.selected_object().cloned() {
                    if let Some(key) = o.asset.as_deref() {
                        if let Some(t) = self.texture(ctx, key) {
                            let frame = to_screen(o.x, o.y, o.w, o.h);
                            let (c, a) = (frame.center(), o.rotation.to_radians());
                            let turn = |p: egui::Pos2| { let d = p - c; c + egui::vec2(d.x * a.cos() - d.y * a.sin(), d.x * a.sin() + d.y * a.cos()) };
                            let (il, it, ir, ib) = crop::image_rect(&o);
                            let full = egui::Rect::from_min_max(origin + egui::vec2(il, it) * scale, origin + egui::vec2(ir, ib) * scale);
                            let mut mesh = egui::epaint::Mesh::with_texture(t.id());
                            let quad = [(full.left_top(), egui::pos2(0.0, 0.0)), (full.right_top(), egui::pos2(1.0, 0.0)), (full.right_bottom(), egui::pos2(1.0, 1.0)), (full.left_bottom(), egui::pos2(0.0, 1.0))];
                            for (p, uv) in quad { mesh.vertices.push(egui::epaint::Vertex { pos: turn(p), uv, color: egui::Color32::WHITE.gamma_multiply(0.45) }); }
                            mesh.add_triangle(0, 1, 2);
                            mesh.add_triangle(0, 2, 3);
                            painter.add(egui::Shape::mesh(mesh));
                            let outline: Vec<egui::Pos2> = [full.left_top(), full.right_top(), full.right_bottom(), full.left_bottom(), full.left_top()].into_iter().map(turn).collect();
                            painter.add(egui::Shape::line(outline, egui::Stroke::new(1.0, egui::Color32::from_gray(140))));
                            // The kept part, at full strength.
                            if let Some((d, uv)) = crop_uv(frame, o.crop) {
                                let mut mesh = egui::epaint::Mesh::with_texture(t.id());
                                let quad = [(d.left_top(), uv.left_top()), (d.right_top(), uv.right_top()), (d.right_bottom(), uv.right_bottom()), (d.left_bottom(), uv.left_bottom())];
                                for (p, uv) in quad { mesh.vertices.push(egui::epaint::Vertex { pos: turn(p), uv, color: egui::Color32::WHITE }); }
                                mesh.add_triangle(0, 1, 2);
                                mesh.add_triangle(0, 2, 3);
                                painter.add(egui::Shape::mesh(mesh));
                            }
                        }
                    }
                }
            }

            // Selection handles (drawn after content so they sit on top).
            let sel_rect = if sel_all.len() > 1 {
                // The box around a multi-selection or group.
                sel_all.iter().filter_map(|&(l, k)| slide.layers.get(l)?.objects.get(k)).map(|o| to_screen(o.x, o.y, o.w, o.h)).reduce(|a, b| a.union(b))
            } else {
                self.selected.and_then(|(li, oi)| slide.layers.get(li)?.objects.get(oi)).map(|o| to_screen(o.x, o.y, o.w, o.h))
            };
            let sel_rot = if sel_all.len() > 1 { 0.0 } else { self.selected.and_then(|(li, oi)| slide.layers.get(li)?.objects.get(oi)).map(|o| o.rotation).unwrap_or(0.0).to_radians() };
            let turn = |r: egui::Rect, p: egui::Pos2| {
                let d = p - r.center();
                r.center() + egui::vec2(d.x * sel_rot.cos() - d.y * sel_rot.sin(), d.x * sel_rot.sin() + d.y * sel_rot.cos())
            };
            let handle_pos = |r: egui::Rect, (hx, hy): (i8, i8)| {
                turn(r, egui::pos2(r.min.x + r.width() * (hx + 1) as f32 / 2.0, r.min.y + r.height() * (hy + 1) as f32 / 2.0))
            };
            let rotate_pos = |r: egui::Rect| turn(r, egui::pos2(r.center().x, r.top() - 26.0));
            if let (Some(r), true) = (sel_rect, outlining) {
                // Outline mode: the frame is only a dashed guide; the corners of the outline are what move.
                painter.rect_stroke(r, 0.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 120, 255)), egui::StrokeKind::Outside);
            }
            let outline_pts: Vec<egui::Pos2> = match (outlining, self.selected_object()) {
                (true, Some(o)) => o.hit_area.as_ref().map(|a| a.iter().map(|&f| { let p = outline::to_slide(o, f); origin + egui::vec2(p.0, p.1) * scale }).collect()).unwrap_or_default(),
                _ => vec![],
            };
            let orange = egui::Color32::from_rgb(226, 91, 28);
            for (i, p) in outline_pts.iter().enumerate() {
                let chosen = self.outline_sel == Some(i);
                // Orange squares like the crop handles; the chosen corner is larger and white inside.
                let hr = egui::Rect::from_center_size(*p, egui::Vec2::splat(HANDLE_PX + if chosen { 4.0 } else { 0.0 }));
                painter.rect_filled(hr, 1.0, if chosen { egui::Color32::WHITE } else { orange });
                painter.rect_stroke(hr, 1.0, egui::Stroke::new(if chosen { 3.0 } else { 1.5 }, if chosen { orange } else { egui::Color32::WHITE }), egui::StrokeKind::Inside);
            }
            if let (Some(r), false) = (sel_rect, outlining) {
                let handle_color = if cropping { egui::Color32::from_rgb(226, 91, 28) } else { egui::Color32::from_rgb(0, 120, 255) };
                for h in HANDLES {
                    let hr = egui::Rect::from_center_size(handle_pos(r, h), egui::Vec2::splat(HANDLE_PX));
                    painter.rect_filled(hr, 1.0, if cropping { handle_color } else { egui::Color32::WHITE });
                    painter.rect_stroke(hr, 1.0, egui::Stroke::new(1.5, handle_color), egui::StrokeKind::Inside);
                }
                if sel_all.len() == 1 && !cropping {
                    let (top, knob) = (handle_pos(r, (0, -1)), rotate_pos(r));
                    painter.line_segment([top, knob], egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 120, 255)));
                    painter.circle(knob, HANDLE_PX / 2.0 + 1.0, egui::Color32::WHITE, egui::Stroke::new(1.5, egui::Color32::from_rgb(0, 120, 255)));
                } else {
                    painter.rect_stroke(r, 0.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 120, 255)), egui::StrokeKind::Outside);
                }
            }
            // The selected object's motion path: a line through draggable points.
            let motion_pts: Vec<egui::Pos2> = match (sel_all.len(), self.selected.and_then(|(li, oi)| slide.layers.get(li)?.objects.get(oi))) {
                (1, Some(o)) => o.motion.as_ref().map(|m| {
                    let c = egui::vec2(o.x + o.w / 2.0, o.y + o.h / 2.0);
                    m.points.iter().map(|p| origin + (c + egui::vec2(p.0, p.1)) * scale).collect()
                }).unwrap_or_default(),
                _ => vec![],
            };
            if motion_pts.len() >= 2 {
                if let Some(o) = self.selected.and_then(|(li, oi)| slide.layers.get(li)?.objects.get(oi)) {
                    let c = egui::vec2(o.x + o.w / 2.0, o.y + o.h / 2.0);
                    let line: Vec<egui::Pos2> = o.motion.as_ref().map(|m| m.sampled(12).into_iter().map(|p| origin + (c + egui::vec2(p.0, p.1)) * scale).collect()).unwrap_or_default();
                    let orange = egui::Color32::from_rgb(226, 91, 28);
                    painter.add(egui::Shape::line(line.clone(), egui::Stroke::new(2.0, orange)));
                    if let [.., a, b] = line.as_slice() {
                        let dir = (*b - *a).normalized();
                        let n = egui::vec2(-dir.y, dir.x);
                        painter.add(egui::Shape::convex_polygon(vec![*b + dir * 6.0, *b - dir * 8.0 + n * 6.0, *b - dir * 8.0 - n * 6.0], orange, egui::Stroke::NONE));
                    }
                    for (i, p) in motion_pts.iter().enumerate().skip(1) {
                        painter.circle(*p, HANDLE_PX / 2.0 + 1.0, egui::Color32::WHITE, egui::Stroke::new(2.0, orange));
                        painter.text(*p, egui::Align2::CENTER_CENTER, i.to_string(), egui::FontId::proportional(9.0), orange);
                    }
                }
            }
            let path_at = |pos: egui::Pos2| motion_pts.iter().enumerate().skip(1).find(|(_, p)| p.distance(pos) <= HANDLE_PX).map(|(i, _)| i);
            let rotate_at = |pos: egui::Pos2| sel_all.len() == 1 && !cropping && !outlining && sel_rect.is_some_and(|r| rotate_pos(r).distance(pos) <= HANDLE_PX);
            let handle_at = |pos: egui::Pos2| -> Option<(i8, i8)> {
                if outlining { return None; }
                let r = sel_rect?;
                HANDLES.into_iter().find(|&h| handle_pos(r, h).distance(pos) <= HANDLE_GRAB_PX)
            };
            // Inner borders of a selected table can be dragged to resize its columns and rows.
            let table_group = if one_group { slide.layers[sel_all[0].0].objects[sel_all[0].1].group.clone().filter(|_| slide.layers[sel_all[0].0].objects[sel_all[0].1].table.is_some()) } else { None };
            let table_lines: Vec<(bool, u16, egui::Rect)> = match (&table_group, sel_rect) {
                (Some(g), Some(r)) => {
                    let (xs, ys) = brainbend_doc::tables::borders(&slide.layers[sel_all[0].0].objects, g);
                    let vx = xs.iter().enumerate().map(|(i, &x)| (true, i as u16, egui::Rect::from_x_y_ranges(origin.x + x * scale..=origin.x + x * scale, r.top()..=r.bottom())));
                    let hy = ys.iter().enumerate().map(|(i, &y)| (false, i as u16, egui::Rect::from_x_y_ranges(r.left()..=r.right(), origin.y + y * scale..=origin.y + y * scale)));
                    vx.chain(hy).collect()
                }
                _ => vec![],
            };
            if let (Some(g), Some((_, blk))) = (&table_group, self.cell_block()) {
                if let Some((x, y, w, h)) = brainbend_doc::tables::range_rect(&slide.layers[sel_all[0].0].objects, g, blk) {
                    let r = to_screen(x, y, w, h);
                    painter.rect_filled(r, 0.0, egui::Color32::from_rgba_unmultiplied(0, 120, 255, 36));
                    painter.rect_stroke(r, 0.0, egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 120, 255)), egui::StrokeKind::Inside);
                }
            }
            let border_at = |pos: egui::Pos2| table_lines.iter().find(|(_, _, l)| l.expand(5.0).contains(pos) && handle_at(pos).is_none() && !rotate_at(pos)).map(|&(v, i, _)| (v, i));
            if let Some(hp) = resp.hover_pos() {
                if let Some((vertical, i)) = border_at(hp) {
                    ui.ctx().set_cursor_icon(if vertical { egui::CursorIcon::ResizeColumn } else { egui::CursorIcon::ResizeRow });
                    if let Some((_, _, l)) = table_lines.iter().find(|(v, j, _)| *v == vertical && *j == i) {
                        painter.line_segment([l.min, l.max], egui::Stroke::new(3.0, egui::Color32::from_rgb(0, 120, 255)));
                    }
                }
            }
            if let Some(hp) = resp.hover_pos() {
                if rotate_at(hp) || path_at(hp).is_some() { ui.ctx().set_cursor_icon(egui::CursorIcon::Grab); }
                if drawing { ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair); }
                if outlining {
                    if let Some(o) = self.selected_object() {
                        let at = to_slide(hp);
                        let over_corner = o.hit_area.as_ref().is_some_and(|a| outline::nearest_vertex(o, a, (at.x, at.y), HANDLE_PX / scale).is_some());
                        let over_edge = !over_corner && o.hit_area.as_ref().is_some_and(|a| outline::nearest_edge(o, a, (at.x, at.y), 6.0 / scale).is_some());
                        if over_corner { ui.ctx().set_cursor_icon(egui::CursorIcon::Grab); }
                        else if over_edge { ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair); }
                    }
                }
                if cropping && handle_at(hp).is_none() && sel_rect.is_some_and(|r| inside_frame(r, sel_rot, hp)) { ui.ctx().set_cursor_icon(egui::CursorIcon::Move); }
                if let Some((hx, hy)) = handle_at(hp) {
                    ui.ctx().set_cursor_icon(match (hx, hy) {
                        (-1, -1) | (1, 1) => egui::CursorIcon::ResizeNwSe,
                        (1, -1) | (-1, 1) => egui::CursorIcon::ResizeNeSw,
                        (0, _) => egui::CursorIcon::ResizeVertical,
                        _ => egui::CursorIcon::ResizeHorizontal,
                    });
                } else if !drawing && !outlining && !cropping && !rotate_at(hp) && path_at(hp).is_none() && border_at(hp).is_none()
                    && (hit_object(&slide, &shown, to_slide(hp), scale).is_some() || sel_rect.is_some_and(|r| inside_frame(r, sel_rot, hp))) {
                    // Anywhere else on an object: it can be dragged.
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Move);
                }
            }

            if resp.secondary_clicked() {
                // Right-clicking an object outside the selection selects it (or clears the selection on empty canvas).
                if let Some(pos) = resp.interact_pointer_pos() {
                    match hit_object(&slide, &shown, to_slide(pos), scale) {
                        Some(h) if sel_all.contains(&h) && self.selected != Some(h) && slide.layers[h.0].objects[h.1].table.is_some() && self.selected_object().is_some_and(|s| s.group == slide.layers[h.0].objects[h.1].group) => {
                            // Right-clicking another cell of the selected table: table commands act on that cell.
                            self.selected = Some(h);
                            self.range_end = None;
                        }
                        Some(h) if !sel_all.contains(&h) => { self.selected = Some(h); self.multi.clear(); }
                        None => { self.selected = None; self.multi.clear(); }
                        _ => {}
                    }
                }
            }
            resp.context_menu(|ui| self.object_context_menu(ui));
            // Drawing a freeform hotspot: each click is a corner; Enter, a double-click or a click on the first corner ends it.
            if drawing {
                let (esc, enter, back) = ctx.input(|i| (i.key_pressed(egui::Key::Escape), i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Backspace)));
                if esc { self.draw_outline = None; self.status = "Freeform hotspot cancelled.".into(); }
                else if enter { self.finish_draw_outline(); }
                else if back { if let Some(p) = self.draw_outline.as_mut() { p.pop(); } }
                else if let (true, Some(pos)) = (resp.clicked() || resp.double_clicked(), resp.interact_pointer_pos()) {
                    let at = to_slide(pos);
                    let near_first = self.draw_outline.as_ref().and_then(|p| p.first()).is_some_and(|f| (origin + egui::vec2(f.0, f.1) * scale).distance(pos) <= HANDLE_PX && self.draw_outline.as_ref().is_some_and(|p| p.len() >= outline::MIN_POINTS));
                    if resp.double_clicked() || near_first { self.finish_draw_outline(); }
                    else if let Some(p) = self.draw_outline.as_mut() { p.push((at.x, at.y)); }
                }
            }
            if let Some(pts) = self.draw_outline.as_ref() {
                let screen: Vec<egui::Pos2> = pts.iter().map(|p| origin + egui::vec2(p.0, p.1) * scale).collect();
                let tail = resp.hover_pos().filter(|_| !screen.is_empty());
                let mut line = screen.clone();
                if let Some(t) = tail { line.push(t); }
                if screen.len() >= outline::MIN_POINTS {
                    painter.add(egui::Shape::convex_polygon(screen.clone(), egui::Color32::from_rgba_unmultiplied(0, 140, 255, 36), egui::Stroke::NONE));
                }
                painter.add(egui::Shape::line(line, egui::Stroke::new(2.0, orange)));
                for (i, p) in screen.iter().enumerate() {
                    painter.circle(*p, if i == 0 { HANDLE_PX / 2.0 + 2.0 } else { HANDLE_PX / 2.0 - 1.0 }, egui::Color32::WHITE, egui::Stroke::new(2.0, orange));
                }
                painter.text(origin + egui::vec2(8.0, 8.0), egui::Align2::LEFT_TOP, "Click each corner. Enter, double-click or the first corner finishes; Backspace undoes a corner; Esc cancels.", egui::FontId::proportional(13.0), orange);
            }
            if !drawing && (resp.drag_started() || resp.clicked()) {
                // A drag is only reported after the pointer has moved a few pixels, so grab where it was pressed.
                let pressed = if resp.drag_started() { ctx.input(|i| i.pointer.press_origin()) } else { None };
                if let Some(pos) = pressed.or(resp.interact_pointer_pos()) {
                    self.drag_changed = false;
                    self.text_sel = None;
                    self.caret = None;
                    let mut outline_hit = false;
                    if outlining {
                        let at = to_slide(pos);
                        let o = self.selected_object().cloned();
                        if let Some(o) = o {
                            let area = o.hit_area.clone().unwrap_or_default();
                            if let Some(i) = outline::nearest_vertex(&o, &area, (at.x, at.y), HANDLE_PX / scale) {
                                self.outline_sel = Some(i);
                                self.drag = Some(DragMode::OutlinePoint(i));
                                outline_hit = true;
                            } else if let Some((edge, p)) = outline::nearest_edge(&o, &area, (at.x, at.y), 6.0 / scale) {
                                // A click on an edge adds a corner there, which can be dragged straight away.
                                self.snapshot();
                                self.drag_changed = true;
                                let mut new_area = area.clone();
                                let k = outline::insert(&mut new_area, edge, p);
                                if let Some(o) = self.outline_object_mut() { o.hit_area = Some(new_area); }
                                self.outline_sel = Some(k);
                                self.drag = Some(DragMode::OutlinePoint(k));
                                self.dirty = true;
                                outline_hit = true;
                            } else if sel_rect.is_some_and(|r| inside_frame(r, sel_rot, pos)) {
                                // Inside the hotspot: nothing to grab, so just let go of any chosen corner.
                                self.outline_sel = None;
                                self.drag = None;
                                outline_hit = true;
                            } else {
                                // Clicking away finishes the outline and selects as usual.
                                self.end_outline();
                            }
                        }
                    }
                    let mut crop_hit = outline_hit;
                    if cropping {
                        if let Some((hx, hy)) = handle_at(pos) {
                            self.drag = Some(DragMode::Crop(hx, hy));
                            crop_hit = true;
                        } else if sel_rect.is_some_and(|r| inside_frame(r, sel_rot, pos)) {
                            self.drag = Some(DragMode::CropPan);
                            crop_hit = true;
                        } else {
                            // Clicking away finishes the crop and selects as usual.
                            self.crop_mode = None;
                        }
                    }
                    if crop_hit {
                    } else if let Some(i) = path_at(pos) {
                        self.drag = Some(DragMode::PathPoint(i));
                    } else if rotate_at(pos) {
                        self.drag = Some(DragMode::Rotate);
                    } else if let Some((vertical, i)) = border_at(pos) {
                        self.drag = Some(DragMode::TableBorder(vertical, i));
                    } else if let Some((hx, hy)) = handle_at(pos) {
                        self.drag = Some(if sel_all.len() > 1 { DragMode::ResizeGroup(hx, hy) } else { DragMode::Resize(hx, hy) });
                    } else {
                        let p = to_slide(pos);
                        let hit = hit_object(&slide, &shown, p, scale);
                        let shift = ctx.input(|i| i.modifiers.shift);
                        let in_sel = hit.is_some_and(|h| sel_all.contains(&h));
                        let same_table = hit.and_then(|(hl, ho)| {
                            let h = &slide.layers[hl].objects[ho];
                            let s = slide.layers.get(self.selected?.0)?.objects.get(self.selected?.1)?;
                            (h.table.is_some() && s.table.is_some() && h.group == s.group).then(|| (h.group.clone().unwrap_or_default(), h.table.unwrap_or_default()))
                        });
                        if let (true, Some(end)) = (shift, same_table) {
                            // Shift-click in a table chooses a block of cells.
                            self.range_end = Some(end);
                            self.drag = None;
                        } else if shift && hit.is_some() && self.selected.is_some() {
                            // Toggle membership (same layer only).
                            let (h_li, h_oi) = hit.unwrap();
                            if self.selected.map(|s| s.0) == Some(h_li) {
                                let id = slide.layers[h_li].objects[h_oi].id.clone();
                                if self.selected == hit {
                                    // Promote another member to primary, if any.
                                    if let Some(next) = sel_all.iter().find(|s| **s != hit.unwrap()).copied() {
                                        self.selected = Some(next);
                                        let nid = slide.layers[next.0].objects[next.1].id.clone();
                                        self.multi.retain(|m| *m != nid && *m != id);
                                    }
                                } else if let Some(pos_) = self.multi.iter().position(|m| *m == id) {
                                    self.multi.remove(pos_);
                                } else {
                                    self.multi.push(id);
                                }
                            }
                            self.drag = None;
                        } else if in_sel && sel_all.len() > 1 {
                            // Grabbing a member of a multi-selection moves the whole group.
                            if let Some(h) = hit.filter(|h| slide.layers[h.0].objects[h.1].table.is_some() && self.selected_object().is_some_and(|s| s.group == slide.layers[h.0].objects[h.1].group)) {
                                // Within a table the clicked cell becomes the current cell.
                                if self.selected != Some(h) { self.selected = Some(h); self.range_end = None; }
                            }
                            self.drag = Some(DragMode::Move);
                        } else {
                            self.selected = hit;
                            self.multi.clear();
                            self.range_end = None;
                            self.drag = Some(if hit.is_some() { DragMode::Move } else { DragMode::Marquee });
                            if hit.is_none() { self.marquee = Some(p.to_pos2()); }
                        }
                        self.drag_accum = egui::Vec2::ZERO;
                        if let Some((li, oi)) = self.selected.filter(|_| matches!(self.drag, Some(DragMode::Move))) {
                            let o = &slide.layers[li].objects[oi];
                            self.drag_origin = (o.x, o.y);
                        }
                    }
                }
            }
            if resp.double_clicked() {
                if let Some((li, oi)) = self.selected {
                    if let Some(o) = slide.layers.get(li).and_then(|l| l.objects.get(oi)) {
                        if o.equation.is_some() {
                            self.open_equation_dialog(Some((li, oi)));
                        } else if matches!(o.kind, Kind::Pano(_)) {
                            self.open_pano_dialog((li, oi));
                        } else if matches!(o.kind, Kind::Hotspot) {
                            if !outlining && !drawing { self.begin_outline(); }
                        } else if !matches!(o.kind, Kind::Image | Kind::Video | Kind::Audio | Kind::Web(_) | Kind::Pano(_) | Kind::Input(_) | Kind::Unsupported(_)) {
                            self.editing = Some((li, oi));
                            self.editing_old = o.text.clone();
                            self.snapshot();
                        }
                    }
                } else if !outlining && !drawing && self.crop_mode.is_none() && !try_on {
                    // Double-clicking blank space makes a text box there and starts typing in it.
                    if let Some(pos) = resp.interact_pointer_pos().or(ctx.pointer_interact_pos()) {
                        let at = to_slide(pos);
                        self.insert_text();
                        if let (Some((li, oi)), Some(p)) = (self.selected, self.project.as_mut()) {
                            let (sw, sh) = (p.slides[slide_idx].width, p.slides[slide_idx].height);
                            if let Some(o) = p.slides[slide_idx].layers.get_mut(li).and_then(|l| l.objects.get_mut(oi)) {
                                o.x = at.x.min(sw - o.w).max(0.0).round();
                                o.y = (at.y - 24.0).min(sh - o.h).max(0.0).round();
                                o.text = String::new();
                                self.fresh_text = Some(o.id.clone());
                            }
                            self.start_edit = Some((li, oi));
                            self.editing_old = String::new();
                        }
                    }
                }
            }
            self.guides.clear();
            if resp.dragged_by(egui::PointerButton::Primary) {
                if let (Some(mode), Some((li, oi))) = (self.drag, self.selected) {
                    let d = resp.drag_delta() / scale;
                    if d != egui::Vec2::ZERO && !self.drag_changed && !matches!(mode, DragMode::Marquee | DragMode::Crop(..) | DragMode::CropPan) {
                        // One undo step per drag, taken at the first actual movement.
                        self.drag_changed = true;
                        self.snapshot();
                    }
                    // Cropping records an undo step only when the drag actually changes the picture.
                    if matches!(mode, DragMode::Crop(..) | DragMode::CropPan) {
                        let current = self.project.as_ref().unwrap().slides[slide_idx].layers.get(li).and_then(|l| l.objects.get(oi)).cloned();
                        if let Some(cur) = current {
                            let mut next = cur.clone();
                            let a = next.rotation.to_radians();
                            let local = (d.x * a.cos() + d.y * a.sin(), -d.x * a.sin() + d.y * a.cos());
                            match mode {
                                DragMode::Crop(hx, hy) => match self.crop_aspect { Some(r) => crop::drag_edges_locked(&mut next, (hx, hy), local, r), None => crop::drag_edges(&mut next, (hx, hy), local) },
                                _ => crop::pan(&mut next, local),
                            }
                            // Rounding in the crop sums leaves tiny differences that are not a change.
                            let (a, b, ca, cb) = ([next.x, next.y, next.w, next.h], [cur.x, cur.y, cur.w, cur.h], next.crop.unwrap_or([0.0; 4]), cur.crop.unwrap_or([0.0; 4]));
                            if a.iter().zip(b).any(|(p, q)| (p - q).abs() > 0.02) || ca.iter().zip(cb).any(|(p, q)| (p - q).abs() > 0.0003) {
                                if !self.drag_changed { self.drag_changed = true; self.snapshot(); }
                                if let Some(o) = self.project.as_mut().unwrap().slides[slide_idx].layers.get_mut(li).and_then(|l| l.objects.get_mut(oi)) {
                                    (o.x, o.y, o.w, o.h, o.crop) = (next.x, next.y, next.w, next.h, next.crop);
                                }
                            }
                        }
                    }
                    let mut group_delta = None;
                    if let Some(o) = self.project.as_mut().unwrap().slides[slide_idx].layers.get_mut(li).and_then(|l| l.objects.get_mut(oi)) {
                        match mode {
                            DragMode::Marquee | DragMode::ResizeGroup(..) | DragMode::TableBorder(..) => {}
                            DragMode::PathPoint(i) => {
                                if let Some(pt) = o.motion.as_mut().and_then(|m| m.points.get_mut(i)) { pt.0 += d.x; pt.1 += d.y; }
                            }
                            DragMode::OutlinePoint(i) => {
                                if let Some(pos) = resp.interact_pointer_pos() {
                                    let at = to_slide(pos);
                                    outline::set_point(o, i, (at.x, at.y));
                                }
                            }
                            DragMode::Move => {
                                self.drag_accum += d;
                                let (mut x, mut y) = (self.drag_origin.0 + self.drag_accum.x, self.drag_origin.1 + self.drag_accum.y);
                                if !ctx.input(|i| i.modifiers.alt) {
                                    // Snap left/center/right and top/middle/bottom to the slide and to other objects.
                                    let mut xs = vec![0.0, slide.width / 2.0, slide.width];
                                    let mut ys = vec![0.0, slide.height / 2.0, slide.height];
                                    for &sl in &shown {
                                        for (k, other) in slide.layers[sl].objects.iter().enumerate() {
                                            if sel_all.contains(&(sl, k)) || other.hidden { continue; }
                                            xs.extend([other.x, other.x + other.w / 2.0, other.x + other.w]);
                                            ys.extend([other.y, other.y + other.h / 2.0, other.y + other.h]);
                                        }
                                    }
                                    let tol = SNAP_PX / scale;
                                    let best = |pos: f32, size: f32, cands: &[f32]| -> Option<(f32, f32)> {
                                        let mut best: Option<(f32, f32)> = None; // (delta, guide)
                                        for edge in [0.0, size / 2.0, size] {
                                            for &c in cands {
                                                let delta = c - (pos + edge);
                                                if delta.abs() <= tol && best.is_none_or(|b| delta.abs() < b.0.abs()) {
                                                    best = Some((delta, c));
                                                }
                                            }
                                        }
                                        best
                                    };
                                    if let Some((dx, g)) = best(x, o.w, &xs) { x += dx; self.guides.push((true, g)); }
                                    if let Some((dy, g)) = best(y, o.h, &ys) { y += dy; self.guides.push((false, g)); }
                                }
                                group_delta = Some((x - o.x, y - o.y));
                                o.x = x;
                                o.y = y;
                            }
                            // Handled above, where the undo step can be skipped for a drag that changes nothing.
                            DragMode::Crop(..) | DragMode::CropPan => {}
                            DragMode::Rotate => {
                                if let Some(pos) = resp.interact_pointer_pos() {
                                    let c = origin + egui::vec2(o.x + o.w / 2.0, o.y + o.h / 2.0) * scale;
                                    let v = pos - c;
                                    let mut deg = v.y.atan2(v.x).to_degrees() + 90.0;
                                    if ctx.input(|i| i.modifiers.shift) { deg = (deg / 15.0).round() * 15.0; }
                                    o.rotation = deg.rem_euclid(360.0);
                                }
                            }
                            DragMode::Resize(hx, hy) if o.rotation != 0.0 => {
                                // Drag in the object's own frame, keeping the opposite edge where it is.
                                let a = o.rotation.to_radians();
                                let (ldx, ldy) = (d.x * a.cos() + d.y * a.sin(), -d.x * a.sin() + d.y * a.cos());
                                let (el, er) = (if hx < 0 { ldx } else { 0.0 }, if hx > 0 { ldx } else { 0.0 });
                                let (et, eb) = (if hy < 0 { ldy } else { 0.0 }, if hy > 0 { ldy } else { 0.0 });
                                let (nw, nh) = ((o.w - el + er).max(4.0), (o.h - et + eb).max(4.0));
                                let shift = egui::vec2((el + er) / 2.0, (et + eb) / 2.0);
                                let world = egui::vec2(shift.x * a.cos() - shift.y * a.sin(), shift.x * a.sin() + shift.y * a.cos());
                                let (cxn, cyn) = (o.x + o.w / 2.0 + world.x, o.y + o.h / 2.0 + world.y);
                                (o.w, o.h) = (nw, nh);
                                (o.x, o.y) = (cxn - nw / 2.0, cyn - nh / 2.0);
                            }
                            DragMode::Resize(hx, hy) => {
                                if hx < 0 { o.x += d.x; o.w -= d.x; } else if hx > 0 { o.w += d.x; }
                                if hy < 0 { o.y += d.y; o.h -= d.y; } else if hy > 0 { o.h += d.y; }
                                if o.w < 4.0 { if hx < 0 { o.x -= 4.0 - o.w; } o.w = 4.0; }
                                if o.h < 4.0 { if hy < 0 { o.y -= 4.0 - o.h; } o.h = 4.0; }
                            }
                        }
                    }
                    if let (DragMode::TableBorder(vertical, i), Some(g)) = (mode, &table_group) {
                        let objs = &mut self.project.as_mut().unwrap().slides[slide_idx].layers[li].objects;
                        brainbend_doc::tables::drag_border(objs, g, vertical, i, if vertical { d.x } else { d.y });
                    }
                    if let DragMode::ResizeGroup(hx, hy) = mode {
                        let objs = &mut self.project.as_mut().unwrap().slides[slide_idx].layers[li].objects;
                        let members: Vec<usize> = sel_all.iter().filter(|s| s.0 == li).map(|s| s.1).collect();
                        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
                        for &k in &members { let o = &objs[k]; x0 = x0.min(o.x); y0 = y0.min(o.y); x1 = x1.max(o.x + o.w); y1 = y1.max(o.y + o.h); }
                        let (ow, oh) = ((x1 - x0).max(1.0), (y1 - y0).max(1.0));
                        let (mut nx0, mut ny0, mut nx1, mut ny1) = (x0, y0, x1, y1);
                        if hx < 0 { nx0 += d.x; } else if hx > 0 { nx1 += d.x; }
                        if hy < 0 { ny0 += d.y; } else if hy > 0 { ny1 += d.y; }
                        if nx1 - nx0 >= 8.0 && ny1 - ny0 >= 8.0 {
                            let (kx, ky) = ((nx1 - nx0) / ow, (ny1 - ny0) / oh);
                            for &k in &members {
                                let o = &mut objs[k];
                                o.x = nx0 + (o.x - x0) * kx;
                                o.y = ny0 + (o.y - y0) * ky;
                                o.w *= kx;
                                o.h *= ky;
                            }
                        }
                    }
                    if let Some((dx, dy)) = group_delta {
                        for &(l, k) in sel_all.iter().filter(|s| **s != (li, oi)) {
                            if let Some(o) = self.project.as_mut().unwrap().slides[slide_idx].layers.get_mut(l).and_then(|ly| ly.objects.get_mut(k)) {
                                o.x += dx;
                                o.y += dy;
                            }
                        }
                    }
                }
            }
            if let Some(start) = self.marquee {
                if resp.dragged_by(egui::PointerButton::Primary) {
                    if let Some(pos) = resp.interact_pointer_pos() {
                        let cur = to_slide(pos).to_pos2();
                        let band = egui::Rect::from_two_pos(start, cur);
                        painter.rect(egui::Rect::from_min_max(origin + band.min.to_vec2() * scale, origin + band.max.to_vec2() * scale),
                            0.0, egui::Color32::from_rgba_unmultiplied(0, 120, 255, 30), egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 120, 255)), egui::StrokeKind::Inside);
                        // Select every visible object on the topmost layer that has any inside the band.
                        let mut found: Vec<(usize, usize)> = vec![];
                        for &li in shown.iter().rev() {
                            for (oi, o) in slide.layers[li].objects.iter().enumerate() {
                                if !o.hidden && band.intersects(egui::Rect::from_min_size(egui::pos2(o.x, o.y), egui::vec2(o.w, o.h))) { found.push((li, oi)); }
                            }
                            if !found.is_empty() { break; }
                        }
                        self.selected = found.last().copied();
                        self.multi = found.iter().rev().skip(1).map(|&(l, k)| slide.layers[l].objects[k].id.clone()).collect();
                    }
                }
                if resp.drag_stopped() || !resp.dragged() { self.marquee = None; }
            }
            if resp.drag_stopped() { self.drag = None; self.guides.clear(); }
            if let Some((li, oi)) = self.editing {
                let valid = slide.layers.get(li).and_then(|l| l.objects.get(oi)).is_some();
                if !valid {
                    self.editing = None;
                    self.edit_turn = None;
                } else {
                    let Some(o_real) = self.project.as_ref().and_then(|p| p.slides.get(slide_idx)).and_then(|s| s.layers.get(li)).and_then(|l| l.objects.get(oi)).cloned() else { self.editing = None; self.edit_turn = None; return };
                    let o = &o_real;
                    // The edit box cannot turn with the object, so turning it (the handle, the Properties field or the
                    // text direction buttons) ends the edit. The text is already in the object, so nothing is lost.
                    let turn_now = (o.rotation.to_bits(), o.text_rotation);
                    let turned = self.edit_turn.is_some_and(|before| before != turn_now);
                    self.edit_turn = Some(turn_now);
                    let r = egui::Rect::from_min_size(origin + egui::vec2(o.x, o.y) * scale, egui::vec2(o.w, o.h) * scale);
                    let font = egui::FontId::new((o.style.size * scale).max(8.0), font_family(&o.style.font_family));
                    let mut text = o.text.clone();
                    let mut done = turned;
                    let edit_id = egui::Id::new("inplace-edit");
                    // Show mixed formatting while editing.
                    let fmts = o.char_fmts();
                    let obj_style = o.style.clone();
                    let link_col = self.link_color();
                    let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap: f32| {
                        let chars: Vec<char> = buf.as_str().chars().collect();
                        let ok = fmts.len() == chars.len();
                        let mut job = egui::text::LayoutJob::default();
                        let mut starts = true;
                        let mut indents: Vec<f32> = vec![];
                        for (i, c) in chars.iter().enumerate() {
                            let f = if ok { fmts[i].1.clone() } else { Fmt::default() };
                            if starts && *c != '\n' { indents.push(f.indent as f32 * brainbend_doc::INDENT_STEP * scale); }
                            else if starts { indents.push(0.0); }
                            starts = *c == '\n';
                            let linked = f.link.is_some();
                            let col = if linked { f.color.map(color).unwrap_or(link_col) } else { color(f.color.unwrap_or(obj_style.color)) };
                            let bold = f.bold.unwrap_or(obj_style.bold);
                            let ul = f.underline.unwrap_or(linked || obj_style.underline);
                            let mut fid = egui::FontId::new(((f.size.unwrap_or(obj_style.size)) * scale).max(8.0), font_family(f.font_family.as_deref().unwrap_or(&obj_style.font_family)));
                            fid.size = fid.size.max(8.0);
                            let mut tf = egui::text::TextFormat {
                                font_id: fid,
                                color: col,
                                coords: egui::epaint::text::VariationCoords::new([(b"wght", if bold { 700.0 } else { 400.0 })]),
                                italics: f.italic.unwrap_or(obj_style.italic),
                                underline: if ul { egui::Stroke::new(1.0, col) } else { egui::Stroke::NONE },
                                ..Default::default()
                            };
                            run_extras(&mut tf, f.strike.unwrap_or(false), f.highlight, f.spacing, scale);
                            job.append(&c.to_string(), 0.0, tf);
                        }
                        job.wrap.max_width = wrap;
                        job.justify = matches!(obj_style.align, Align::Justify);
                        job.halign = match obj_style.align { Align::Center => egui::Align::Center, Align::Right => egui::Align::RIGHT, _ => egui::Align::LEFT };
                        let ppp = ui.ctx().pixels_per_point();
                        let fonts = ui.ctx().clone();
                        layout_indented(&|j| fonts.fonts_mut(|f| f.layout_job(j)), job, &indents, ppp, false)
                    };
                    // Words the spell checker accepts without comment: the project's own and ignored ones.
                    let mut known: std::collections::HashSet<String> = self.project.as_ref().map(|p| p.custom_words.iter().map(|w| w.to_lowercase()).collect()).unwrap_or_default();
                    if let Some(sp) = &self.spell { known.extend(sp.ignore.iter().cloned()); }
                    let mut spell_action: Option<(bool, String)> = None; // (add to dictionary?, lowercase word)
                    // Overflowing text grows the same way it is drawn when not editing; the height is last frame's.
                    let grown_id = egui::Id::new("inplace-grown");
                    let grown: f32 = ctx.data(|d| d.get_temp(grown_id)).unwrap_or(0.0);
                    let over = (grown - r.height()).max(0.0);
                    let lift = match o.style.valign { VAlign::Top => 0.0, VAlign::Middle => over / 2.0, VAlign::Bottom => over };
                    // A %Name% placeholder counts as one character: Backspace or Delete beside it removes all of it.
                    let (bs, del) = ctx.input(|i| (i.key_pressed(egui::Key::Backspace), i.key_pressed(egui::Key::Delete)));
                    if bs || del {
                        if let Some(mut st) = egui::TextEdit::load_state(ctx, edit_id) {
                            if let Some(range) = st.cursor.char_range().filter(|r| r.primary.index == r.secondary.index) {
                                if let Some((a, b)) = brainbend_doc::variables::placeholder_to_delete(&text, range.primary.index.0, del && !bs) {
                                    st.cursor.set_char_range(Some(egui::text::CCursorRange::two(egui::text::CCursor::new(a), egui::text::CCursor::new(b))));
                                    st.store(ctx, edit_id);
                                }
                            }
                        }
                    }
                    egui::Area::new(egui::Id::new("inplace-text")).fixed_pos(r.min - egui::vec2(0.0, lift)).constrain(false).order(egui::Order::Foreground).show(ctx, |ui| {
                        // The box is the object's own frame: no background of its own, the object's margins and text
                        // alignment, and an outline so it is clear where typing goes.
                        let m = |v: f32| (v * scale).round().clamp(0.0, 100.0) as i8;
                        ui.painter().rect_stroke(r, 0.0, egui::Stroke::new(1.5, ui.visuals().selection.bg_fill), egui::StrokeKind::Outside);
                        let out = egui::TextEdit::multiline(&mut text)
                            .id(edit_id)
                            .font(font)
                            .frame(egui::Frame::NONE)
                            .desired_rows(1)
                            .margin(egui::Margin { left: m(o.margins[0]), top: m(o.margins[1]), right: m(o.margins[2]), bottom: m(o.margins[3]) })
                            .desired_width(r.width())
                            .min_size(r.size())
                            .horizontal_align(match o.style.align { Align::Left | Align::Justify => egui::Align::Min, Align::Center => egui::Align::Center, Align::Right => egui::Align::Max })
                            .vertical_align(match o.style.valign { VAlign::Top => egui::Align::Min, VAlign::Middle => egui::Align::Center, VAlign::Bottom => egui::Align::Max })
                            .text_color(color(o.style.color))
                            .layouter(&mut layouter)
                            .show(ui);
                        ctx.data_mut(|d| d.insert_temp(grown_id, out.response.rect.height()));
                        if !out.response.has_focus() && !out.response.lost_focus() { out.response.request_focus(); }
                        if out.response.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Escape)) { done = true; }
                        // Red squiggles under misspelled words; right-click one for suggestions.
                        let wrong: Vec<(usize, usize, String)> = brainbend_spell::words(&text).into_iter()
                            .map(|(a, b)| (a, b, text[a..b].to_string()))
                            .filter(|(_, _, w)| !brainbend_spell::is_correct(w, &known))
                            .map(|(a, b, w)| (text[..a].chars().count(), text[..b].chars().count(), w))
                            .collect();
                        for (a, b, _) in &wrong {
                            let (ra, rb) = (out.galley.pos_from_cursor(egui::text::CCursor::new(*a)), out.galley.pos_from_cursor(egui::text::CCursor::new(*b)));
                            if (ra.min.y - rb.min.y).abs() > 1.0 { continue; }
                            let (x0, x1, y) = (out.galley_pos.x + ra.min.x, out.galley_pos.x + rb.min.x, out.galley_pos.y + ra.max.y - 1.0);
                            let mut pts = vec![];
                            let mut x = x0;
                            let mut up = false;
                            while x < x1 { pts.push(egui::pos2(x, if up { y - 2.0 } else { y })); x += 2.5; up = !up; }
                            pts.push(egui::pos2(x1, y));
                            if pts.len() > 1 { ui.painter().add(egui::Shape::line(pts, egui::Stroke::new(1.2, egui::Color32::from_rgb(230, 60, 50)))); }
                        }
                        let pick = egui::Id::new("spell-pick");
                        if out.response.secondary_clicked() {
                            let hit = out.response.interact_pointer_pos().map(|p| out.galley.cursor_from_pos(p - out.galley_pos).index.0);
                            let word = hit.and_then(|i| wrong.iter().find(|(a, b, _)| i >= *a && i <= *b).cloned());
                            ui.data_mut(|d| d.insert_temp(pick, word));
                        }
                        out.response.context_menu(|ui| {
                            let Some((a, b, w)) = ui.data(|d| d.get_temp::<Option<(usize, usize, String)>>(pick)).flatten() else { ui.close(); return };
                            let sugg = brainbend_spell::suggest(&w);
                            for s in &sugg {
                                if ui.button(s).clicked() {
                                    let chars: Vec<char> = text.chars().collect();
                                    text = chars[..a].iter().collect::<String>() + s + &chars[b..].iter().collect::<String>();
                                    ui.close();
                                }
                            }
                            if sugg.is_empty() { ui.weak("No suggestions"); }
                            ui.separator();
                            if ui.button("Ignore").clicked() { spell_action = Some((false, w.to_lowercase())); ui.close(); }
                            if ui.button("Add to dictionary").clicked() { spell_action = Some((true, w.to_lowercase())); ui.close(); }
                        });
                    });
                    if let Some((add, w)) = spell_action {
                        if add {
                            self.snapshot();
                            let p = self.project.as_mut().unwrap();
                            if !p.custom_words.contains(&w) { p.custom_words.push(w); }
                        } else {
                            self.spell.get_or_insert_with(Default::default).ignore.insert(w);
                        }
                    }
                    // ⌘B / ⌘I / ⌘U format the selected characters.
                    let attr = ctx.input(|i| {
                        if !i.modifiers.command { None }
                        else if i.key_pressed(egui::Key::B) { Some(Attr::ToggleBold) }
                        else if i.key_pressed(egui::Key::I) { Some(Attr::ToggleItalic) }
                        else if i.key_pressed(egui::Key::U) { Some(Attr::ToggleUnderline) }
                        else { None }
                    });
                    // The cursor and selection never end inside a placeholder: it is one character.
                    let prev_id = egui::Id::new("inplace-prev-caret");
                    let prev: Option<usize> = ctx.data(|d| d.get_temp(prev_id));
                    if let Some(mut st) = egui::TextEdit::load_state(ctx, edit_id) {
                        if let Some(r) = st.cursor.char_range() {
                            let (p, s) = brainbend_doc::variables::snap_to_placeholders(&text, r.primary.index.0, r.secondary.index.0, prev);
                            if (p, s) != (r.primary.index.0, r.secondary.index.0) {
                                st.cursor.set_char_range(Some(egui::text::CCursorRange { primary: egui::text::CCursor::new(p), secondary: egui::text::CCursor::new(s), h_pos: None }));
                                st.store(ctx, edit_id);
                                ctx.request_repaint();
                            }
                            ctx.data_mut(|d| d.insert_temp(prev_id, p));
                        }
                    }
                    let cur = egui::TextEdit::load_state(ctx, edit_id).and_then(|st| st.cursor.char_range());
                    if let Some(range) = cur {
                        let (p, q) = (range.primary.index.0, range.secondary.index.0);
                        self.text_sel = (p != q).then(|| (p.min(q), p.max(q)));
                        self.caret = Some((li, oi, q));
                    }
                    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) { self.text_sel = None; }
                    if let (Some(attr), Some(range)) = (attr, cur) {
                        let (p, q) = (range.primary.index.0, range.secondary.index.0);
                        let (a, b) = (p.min(q), p.max(q));
                        if a < b {
                            self.snapshot();
                            if let Some(obj) = self.project.as_mut().unwrap().slides[slide_idx].layers.get_mut(li).and_then(|l| l.objects.get_mut(oi)) {
                                obj.format_range(a, b, &attr);
                                for st in obj.states.iter_mut() { st.text = obj.text.clone(); st.rich = obj.rich.clone(); }
                            }
                        }
                    }
                    if text != o.text {
                        let old = self.editing_old.clone();
                        if let Some(obj) = self.project.as_mut().unwrap().slides[slide_idx].layers.get_mut(li).and_then(|l| l.objects.get_mut(oi)) {
                            obj.edit_text(&text);
                            for st in obj.states.iter_mut() {
                                if st.text == old { st.text = obj.text.clone(); st.rich = obj.rich.clone(); }
                            }
                        }
                        self.editing_old = text;
                        self.dirty = true;
                    }
                    if done {
                        self.editing = None;
                        self.edit_turn = None;
                        // A text box made by double-clicking that was left empty is not wanted.
                        if let Some(id) = self.fresh_text.take() {
                            let empty = self.project.as_ref().and_then(|p| p.slides.get(slide_idx)).and_then(|s| s.layers.get(li)).and_then(|l| l.objects.get(oi)).is_some_and(|o| o.id == id && o.text.is_empty());
                            if empty {
                                self.snapshot();
                                if let Some(l) = self.project.as_mut().and_then(|p| p.slides.get_mut(slide_idx)).and_then(|s| s.layers.get_mut(li)) { l.objects.remove(oi); }
                                if self.selected == Some((li, oi)) { self.selected = None; }
                                self.dirty = true;
                            }
                        }
                    }
                }
            }
            for &(vertical, c) in &self.guides {
                let (a, b) = if vertical {
                    (egui::pos2(origin.x + c * scale, area.top()), egui::pos2(origin.x + c * scale, area.bottom()))
                } else {
                    (egui::pos2(area.left(), origin.y + c * scale), egui::pos2(area.right(), origin.y + c * scale))
                };
                painter.line_segment([a, b], egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 0, 140)));
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn harness(app: App) -> egui_kittest::Harness<'static, App> {
        // The harness runs a frame while building, before fonts can be installed, so build it with
        // an empty app (which draws no text), install the fonts, and only then swap the real app in.
        let mut h = egui_kittest::Harness::builder()
            .with_size(egui::vec2(1500.0, 950.0))
            .build_ui_state(|ui, app: &mut App| app.show(ui), App::default());
        install_fonts(&h.ctx);
        install_theme(&h.ctx);
        h.run_steps(2);
        *h.state_mut() = app;
        h
    }

    /// Like `harness`, but its frames are 20 ms apart, so two clicks a few frames apart still count as a double-click.
    fn harness_fast(app: App) -> egui_kittest::Harness<'static, App> {
        let mut h = egui_kittest::Harness::builder()
            .with_size(egui::vec2(1500.0, 950.0))
            .with_step_dt(0.02)
            .build_ui_state(|ui, app: &mut App| app.show(ui), App::default());
        install_fonts(&h.ctx);
        install_theme(&h.ctx);
        h.run_steps(2);
        *h.state_mut() = app;
        h
    }

    fn sample() -> Option<App> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/safety-model.story");
        if !p.exists() { return None; }
        let mut a = App::default();
        a.open(&p);
        a.project.as_ref()?;
        Some(a)
    }

    /// Opens the file named by BRAINBEND_OPEN_STORY at slide BRAINBEND_OPEN_SLIDE (default 0) and renders it to
    /// `open-story.png` in BRAINBEND_SHOT_DIR (visual review of imports only).
    #[test]
    fn gui_open_story_screenshot() {
        let (Some(file), Some(dir)) = (std::env::var_os("BRAINBEND_OPEN_STORY"), std::env::var_os("BRAINBEND_SHOT_DIR")) else { return };
        let mut a = App::default();
        a.open(&std::path::PathBuf::from(file));
        assert!(a.project.is_some(), "the file opens");
        a.slide = std::env::var("BRAINBEND_OPEN_SLIDE").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
        let mut h = harness(a);
        h.run_steps(4);
        h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("open-story.png")).unwrap();
    }

    #[test]
    fn try_it_preview_and_publish_are_pinned_in_the_ribbon_on_every_tab() {
        use egui_kittest::kittest::{NodeT, Queryable};
        let shot = |h: &mut egui_kittest::Harness<'static, App>, file: &str| {
            if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") { h.render().expect("renderer").save(std::path::PathBuf::from(dir).join(file)).unwrap(); }
        };
        let mut a = App::default();
        a.new_project();
        let mut h = harness(a);
        h.run_steps(4);
        for tab in [ribbon::HOME_TAB, 2, 3, 4, 5, ribbon::HELP_TAB] {
            h.state_mut().ribbon_tab = tab;
            h.run_steps(3);
            for label in ["Try It", "Preview", "Publish"] {
                assert!(h.query_by_label(label).is_some(), "{label} is in the ribbon on tab {tab}");
            }
        }
        h.state_mut().ribbon_tab = ribbon::HOME_TAB;
        h.run_steps(3);
        shot(&mut h, "pinned-1-home-dark.png");
        h.ctx.set_theme(egui::ThemePreference::Light);
        h.run_steps(3);
        shot(&mut h, "pinned-2-home-light.png");
        h.ctx.set_theme(egui::ThemePreference::Dark);
        // Trying a slide greys the ribbon out, but not the pinned group: Try It must stay clickable to stop.
        h.state_mut().try_mode = true;
        h.run_steps(3);
        assert!(!h.get_by_label("Try It").accesskit_node().is_disabled(), "Try It stays enabled while trying");
        assert!(!h.get_by_label("Publish").accesskit_node().is_disabled());
        shot(&mut h, "pinned-3-try-mode.png");
        h.state_mut().try_mode = false;
        h.set_size(egui::vec2(820.0, 600.0));
        h.run_steps(4);
        assert!(h.query_by_label("Publish").is_some(), "still there in a narrow window");
        shot(&mut h, "pinned-4-narrow.png");
    }

    #[test]
    fn a_web_lesson_keeps_preview_and_publish_in_its_toolbar_and_has_no_try_it() {
        use egui_kittest::kittest::Queryable;
        let mut a = App::default();
        a.new_lesson_project(ProjectKind::WebLesson);
        let mut h = harness(a);
        h.run_steps(4);
        assert!(h.query_by_label("Publish").is_some());
        assert!(h.query_by_label("Preview").is_some());
        assert!(h.query_by_label("Try It").is_none(), "slides only");
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") { h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("pinned-5-web-lesson.png")).unwrap(); }
    }

    #[test]
    fn the_community_button_uses_the_projects_own_redirect_not_a_discord_invite() {
        assert!(ribbon::COMMUNITY_URL.starts_with("https://brainbend.org/"), "{}", ribbon::COMMUNITY_URL);
        assert!(!ribbon::COMMUNITY_URL.contains("discord.gg"), "an invite link built into the app expires with the invite");
    }

    #[test]
    fn the_player_dialog_offers_play_pause_a_seekbar_and_replay() {
        use egui_kittest::kittest::Queryable;
        let mut a = App::default();
        a.new_project();
        a.player_open = true;
        let mut h = harness(a);
        h.run_steps(4);
        for label in ["Play / Pause", "Seekbar", "Replay"] { assert!(h.query_by_label(label).is_some(), "{label} is listed with the player controls"); }
        assert!(!h.state().project.as_ref().unwrap().player.controls.play_pause);
        for label in ["Play / Pause", "Seekbar", "Replay"] { h.get_by_label(label).focus(); h.run_steps(2); h.key_press(egui::Key::Space); h.run_steps(3); }   // below the fold, so by keyboard
        let c = h.state().project.as_ref().unwrap().player.controls;
        assert!(c.play_pause && c.seekbar && c.replay, "ticking them turns them on");
    }

    #[test]
    fn the_player_dialog_offers_a_banner_photo_and_shows_it_in_its_preview() {
        use egui_kittest::kittest::Queryable;
        let mut a = App::default();
        a.new_project();
        a.player_open = true;
        let mut h = harness(a);
        h.run_steps(4);
        assert!(h.query_by_label("Banner photo").is_some(), "the setting is listed beside the logo");
        assert!(h.query_by_label("+ Banner photo").is_some(), "with nothing chosen it offers to add one");
        assert!(h.query_by_label("Remove").is_none());
        // With one chosen it names the file and can change or remove it, and the preview shows a banner strip.
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(16, 3, image::Rgba([3, 83, 158, 255]))).write_to(&mut png, image::ImageFormat::Png).unwrap();
        let p = h.state_mut().project.as_mut().unwrap();
        p.assets.insert("ban".into(), Asset { name: "header.png".into(), mime: "image/png".into(), bytes: png.into_inner(), file: None });
        p.player.banner = Some("ban".into());
        h.run_steps(4);
        assert!(h.query_by_label_contains("Choose a sidebar above").is_some(), "a banner with no sidebar says it will not show");
        h.state_mut().project.as_mut().unwrap().player.sidebar = SidebarSide::Left;
        h.run_steps(3);
        assert!(h.query_by_label_contains("Choose a sidebar above").is_none(), "with a sidebar the warning goes");
        assert!(h.query_by_label("header.png").is_some(), "the file's name is shown");
        assert!(h.query_by_label("+ Banner photo").is_none());
        assert!(h.query_by_label("Remove").is_some(), "and it can be removed");
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") { h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("player-banner.png")).unwrap(); }
        h.get_by_label("Remove").click();
        h.run_steps(4);
        assert!(h.state().project.as_ref().unwrap().player.banner.is_none(), "Remove clears it");
        assert!(h.query_by_label("+ Banner photo").is_some());
    }

    #[test]
    fn the_player_preview_shows_the_banner_in_the_sidebar_and_the_logo_by_the_title() {
        let mut a = App::default();
        a.new_project();
        a.player_open = true;
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(8, 4, image::Rgba([3, 83, 158, 255]))).write_to(&mut png, image::ImageFormat::Png).unwrap();
        let asset = |name: &str, bytes: Vec<u8>| Asset { name: name.into(), mime: "image/png".into(), bytes, file: None };
        let p = a.project.as_mut().unwrap();
        p.assets.insert("ban".into(), asset("header.png", png.get_ref().clone()));
        p.assets.insert("lg".into(), asset("logo.png", png.into_inner()));
        p.player.banner = Some("ban".into());
        p.player.logo = Some("lg".into());
        p.player.show_title = true;
        p.player.tab_notes = true;
        p.show_menu = true;
        p.player.sidebar = SidebarSide::Left;
        let mut h = harness(a);
        h.run_steps(5);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") { h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("player-preview-layout.png")).unwrap(); }
    }

    #[test]
    fn help_tab_offers_docs_issues_community_and_update_check() {
        use egui_kittest::kittest::Queryable;
        let mut a = App::default();
        a.new_project();
        a.ribbon_tab = ribbon::HELP_TAB;
        let mut h = harness(a);
        h.run_steps(4);
        for label in ["Docs", "Submit Issue", "BrainBend Community", "Check for Updates"] {
            assert!(h.query_by_label(label).is_some(), "{label} is on the Help tab");
        }
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("help-tab.png")).unwrap();
        }
    }

    #[test]
    fn the_update_window_walks_from_available_to_restart() {
        use egui_kittest::kittest::Queryable;
        let suffix = update::platform_suffix().unwrap();
        let name = format!("BrainBend-Studio-v9.9.9{suffix}");
        let json = format!(r###"{{"tag_name":"v9.9.9","html_url":"https://example.com/r","body":"## What's new\n- Faster saving\n- Fixes","assets":[{{"name":"{name}","browser_download_url":"https://github.com/brainbend/studio/releases/download/v9.9.9/{name}","size":31457280}},{{"name":"{name}.minisig","browser_download_url":"https://github.com/brainbend/studio/releases/download/v9.9.9/{name}.minisig","size":300}}]}}"###);
        let release = update::parse_release(&json).unwrap();
        let mut a = App::default();
        a.new_project();
        let mut h = harness(a);
        let shot = |h: &mut egui_kittest::Harness<'static, App>, file: &str| {
            if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") { h.render().expect("renderer").save(std::path::PathBuf::from(dir).join(file)).unwrap(); }
        };
        h.run_steps(3);
        h.state_mut().updater.state = update::State::Available(release.clone());
        h.state_mut().updater.dialog_open = true;
        h.run_steps(4);
        assert!(h.query_by_label("Update now").is_some(), "a signed release offers Update now");
        assert!(h.query_by_label("Release page").is_some());
        shot(&mut h, "update-1-available.png");

        let progress = std::sync::Arc::new(update::Progress::default());
        progress.total.store(31_457_280, std::sync::atomic::Ordering::Relaxed);
        progress.done.store(12_000_000, std::sync::atomic::Ordering::Relaxed);
        let (_tx, rx) = std::sync::mpsc::channel();
        h.state_mut().updater.state = update::State::Downloading { release: release.clone(), progress, rx };
        h.run_steps(3);
        assert!(h.query_by_label("Update now").is_none(), "no second download while one runs");
        shot(&mut h, "update-2-downloading.png");

        h.state_mut().updater.state = update::State::Ready { release: release.clone(), staged: std::path::PathBuf::from("/nonexistent") };
        h.run_steps(3);
        assert!(h.query_by_label("Restart now").is_some());
        shot(&mut h, "update-3-ready.png");

        h.state_mut().updater.state = update::State::UpdateFailed { release, error: "no permission to replace the app".into() };
        h.run_steps(3);
        assert!(h.query_by_label("Try again").is_some());
        shot(&mut h, "update-4-failed.png");
    }

    #[test]
    fn a_release_without_a_signed_download_only_offers_the_release_page() {
        use egui_kittest::kittest::Queryable;
        let release = update::parse_release(r#"{"tag_name":"v9.9.9","html_url":"https://example.com/r"}"#).unwrap();
        let mut a = App::default();
        a.new_project();
        let mut h = harness(a);
        h.run_steps(2);
        h.state_mut().updater.state = update::State::Available(release);
        h.state_mut().updater.dialog_open = true;
        h.run_steps(4);
        assert!(h.query_by_label("Update now").is_none());
        assert!(h.query_by_label("Release page").is_some());
    }

    #[test]
    fn update_button_appears_in_the_title_bar_only_when_a_release_is_newer() {
        use egui_kittest::kittest::Queryable;
        let mut a = App::default();
        a.file_view = false;
        let mut h = harness(a);
        h.run_steps(3);
        assert!(h.query_by_label_contains("Update available").is_none(), "nothing to show before a newer release is found");
        let release = update::parse_release(r#"{"tag_name":"v9.9.9","html_url":"https://example.com/r"}"#).unwrap();
        h.state_mut().updater.state = update::State::Available(release);
        h.run_steps(3);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("update-button.png")).unwrap();
        }
        assert!(h.query_by_label_contains("Update available").is_some(), "the red button shows once a newer release is known");
    }

    /// Renders several editor states to PNGs when BRAINBEND_SHOT_DIR is set (visual review only).
    #[test]
    fn gui_screenshots() {
        let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") else { return };
        let dir = std::path::PathBuf::from(dir);
        let shot = |h: &mut egui_kittest::Harness<'static, App>, name: &str| {
            h.run_steps(3);
            h.render().expect("renderer").save(dir.join(name)).unwrap();
        };
        if let Some(app) = sample() {
            let mut h = harness(app);
            shot(&mut h, "1-sample.png");
            h.state_mut().check_results = h.state().project.as_ref().map(|p| p.check());
            shot(&mut h, "8-project-check.png");
            h.state_mut().check_results = None;
            // Select the first button on layer 1 and look at inspector + interactions.
            let found = {
                let p = h.state().project.as_ref().unwrap();
                p.slides[0].layers.iter().enumerate().find_map(|(li, l)| l.objects.iter().position(|o| !o.states.is_empty()).map(|oi| (li, oi)))
            };
            if let Some((li, oi)) = found {
                h.state_mut().layer = li;
                h.state_mut().visible[li] = true;
                h.state_mut().selected = Some((li, oi));
                shot(&mut h, "2-sample-button-selected.png");
            }
        }
        {
            let mut a = App::default();
            a.new_project();
            a.insert_ellipse();
            let mut h = harness(a);
            shot(&mut h, "6-ellipse.png");
        }
        {
            // A cropped photo: selected, then in crop mode.
            let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/SummarySlides.story");
            if p.exists() {
                let mut a = App::default();
                a.open(&p);
                let found = a.project.as_ref().unwrap().slides[0].layers.iter().enumerate().find_map(|(li, l)| l.objects.iter().position(|o| o.crop.is_some()).map(|oi| (li, oi)));
                if let Some((li, oi)) = found {
                    a.selected = Some((li, oi));
                    let id = a.project.as_ref().unwrap().slides[0].layers[li].objects[oi].id.clone();
                    let mut h = harness(a);
                    shot(&mut h, "9-picture-selected.png");
                    h.state_mut().crop_mode = Some(id);
                    shot(&mut h, "9-crop-mode.png");
                }
            }
        }
        {
            let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/tabs-interaction.story");
            if p.exists() {
                let mut a = App::default();
                a.open(&p);
                a.import_report.clear();
                let (li, oi) = a.project.as_ref().unwrap().slides[0].layers[0].objects.iter().position(|o| o.name == "Button 01").map(|oi| (0, oi)).unwrap();
                let oi_btn = oi;
                a.selected = Some((li, oi));
                a.bottom_tab = 2;
                a.state_view = Some((a.project.as_ref().unwrap().slides[0].layers[0].objects[oi].id.clone(), "Hover".into()));
                let mut h = harness(a);
                // Editing Layer 1: the base body text and character it replaces are not drawn.
                h.state_mut().try_mode = false;
                h.state_mut().bottom_tab = 0;
                h.state_mut().selected = None;
                h.state_mut().layer = 1;
                shot(&mut h, "hide-on-layers.png");
                h.state_mut().layer = 0;
                h.state_mut().bottom_tab = 2;
                h.state_mut().selected = Some((0, oi_btn));
                shot(&mut h, "states-tab.png");
                // Try it: the second tab has been pressed, so it is Selected and the first is back to Normal.
                let id2 = h.state().project.as_ref().unwrap().slides[0].layers[0].objects.iter().find(|o| o.name == "Button 02").map(|o| o.id.clone()).unwrap();
                h.state_mut().try_mode = true;
                h.state_mut().try_states.insert(id2, "Selected".into());
                // ...and its layer is showing, replacing the base text and character.
                let layer2 = h.state().project.as_ref().unwrap().slides[0].layers.iter().find(|l| l.name == "Layer 2").map(|l| l.id.clone()).unwrap();
                h.state_mut().try_slide = Some(0);
                h.state_mut().try_layers = vec![layer2];
                shot(&mut h, "states-try.png");
            }
        }
        {
            // Player Properties, with a left sidebar, a title and some controls on.
            let mut a = App::default();
            a.new_project();
            if let Some(p) = a.project.as_mut() {
                p.player.sidebar = SidebarSide::Left;
                p.player.show_title = true;
                p.player.title = "Safety Basics".into();
                p.player.tab_notes = true;
                p.player.controls.volume = true;
                p.player.controls.captions = true;
                p.player.controls.fullscreen = true;
                p.player.buttons = ButtonStyle::IconText;
                p.player.theme = PlayerTheme::Custom;
                p.player.background = Rgba(40, 55, 90, 255);
                p.player.accent = Rgba(230, 120, 30, 255);
            }
            a.player_open = true;
            let mut h = harness(a);
            h.run_steps(3);
            shot(&mut h, "player-features.png");
            h.state_mut().player_pane = 1;
            h.run_steps(2);
            shot(&mut h, "player-colors.png");
        }
        {
            // Story View and slide tabs.
            let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/SummarySlides.story");
            if p.exists() {
                let mut a = App::default();
                a.open(&p);
                a.import_report.clear();
                let mut h = harness(a);
                for i in [0usize, 4, 7] {
                    h.state_mut().slide = i;
                    h.state_mut().scene_view = false;
                    h.run_steps(3);
                }
                shot(&mut h, "tabs-slide.png");
                h.ctx.set_theme(egui::ThemePreference::Light);
                h.run_steps(2);
                shot(&mut h, "tabs-slide-light.png");
                h.ctx.set_theme(egui::ThemePreference::Dark);
                h.run_steps(2);
                h.state_mut().scene_view = true;
                shot(&mut h, "tabs-story-view.png");
            }
        }
        {
            // Videos with a still picture show it, and the others a play mark.
            let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/ELH_283_-_Interactive_graphic_2.story");
            if p.exists() {
                let mut a = App::default();
                a.open(&p);
                a.import_report.clear();
                a.layer = 1;
                let mut h = harness(a);
                shot(&mut h, "video-poster.png");
            }
            let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/Interactive-Video-Examples.story");
            if p.exists() {
                let mut a = App::default();
                a.open(&p);
                a.import_report.clear();
                let mut h = harness(a);
                shot(&mut h, "video-placeholder.png");
            }
        }
        {
            let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/motion-graphics-titles.story");
            if p.exists() {
                let mut a = App::default();
                a.open(&p);
                a.bottom_tab = 0;
                let mut h = harness(a);
                shot(&mut h, "7-timeline.png");
            }
        }
        {
            let mut a = App::default();
            a.new_project();
            a.add_slide(SlideKind::Question(Question::new_matching()));
            let mut h = harness(a);
            shot(&mut h, "9-matching-question.png");
        }
        let mut a = App::default();
        a.new_project();
        a.insert_text();
        a.insert_rect();
        a.insert_ellipse();
        a.insert_button();
        a.add_layer();
        a.add_slide(SlideKind::Question(Question::new_single()));
        let mut h = harness(a);
        shot(&mut h, "3-question-slide.png");
        h.state_mut().add_slide(SlideKind::Results);
        shot(&mut h, "4-results-slide.png");
        h.state_mut().slide = 0;
        h.state_mut().layer = 0;
        h.state_mut().reset_visibility();
        h.state_mut().selected = Some((0, 3));
        shot(&mut h, "5-new-project-button-selected.png");
        {
            let ids: Vec<String> = h.state().project.as_ref().unwrap().slides[0].layers[0].objects.iter().map(|o| o.id.clone()).collect();
            h.state_mut().selected = Some((0, 0));
            h.state_mut().multi = ids[1..].to_vec();
            shot(&mut h, "5b-multi-selected.png");
            h.state_mut().multi.clear();
        }
        for (tab, name) in [(0usize, "5b2-ribbon-file.png"), (2, "5c-ribbon-insert.png"), (3, "5d-ribbon-slides.png"), (4, "5e-ribbon-design.png"), (5, "5g-ribbon-animations.png")] {
            h.state_mut().ribbon_tab = tab;
            shot(&mut h, name);
        }
        h.state_mut().add_layer();
        h.state_mut().insert_ellipse();
        h.run_steps(3);
        shot(&mut h, "5l-layers.png");
        h.state_mut().add_scene();
        h.state_mut().add_slide(SlideKind::Normal);
        {
            let p = h.state_mut().project.as_mut().unwrap();
            let (s1, s2) = (p.slides[1].id.clone(), p.slides[2].id.clone());
            let mk = |id: &str, a: Action| Trigger { id: id.into(), owner: Owner::Slide, event: Event::SlideStart, conditions: vec![], action: a };
            p.slides[0].triggers.push(mk("jt1", Action::JumpToSlide(s1)));
            p.slides[0].triggers.push(mk("jt2", Action::JumpToSlide(s2)));
            let (s0, s4) = (p.slides[0].id.clone(), p.slides[4].id.clone());
            p.slides[1].triggers.push(mk("jt3", Action::JumpToSlide(s0)));
            p.slides[2].triggers.push(mk("jt4", Action::JumpToSlide(s4)));
        }
        h.state_mut().scene_view = true;
        h.run_steps(3);
        shot(&mut h, "5o-scene-view.png");
        h.state_mut().scene_view = false;
        h.state_mut().slide = 0;
        h.run_steps(2);
        shot(&mut h, "5p-scene-dropdown.png");
        h.state_mut().slide = 0;
        h.state_mut().right_tab = 1;
        h.state_mut().bottom_tab = 1;
        h.run_steps(3);
        shot(&mut h, "5m-triggers-notes.png");
        h.state_mut().find = Some(FindState { query: "layer".into(), with: "level".into(), ..Default::default() });
        h.run_steps(3);
        shot(&mut h, "5m2-find.png");
        h.state_mut().find = None;
        h.state_mut().project.as_mut().unwrap().slides[0].notes = "Remember the recieve step".into();
        h.state_mut().open_spelling();
        h.run_steps(3);
        shot(&mut h, "5m3-spelling.png");
        h.state_mut().spell = None;
        h.state_mut().project.as_mut().unwrap().slides[0].layers[0].objects[0].text = "Safty first, teh plan".into();
        h.state_mut().editing = Some((0, 0));
        h.run_steps(4);
        shot(&mut h, "5m4-squiggles.png");
        h.state_mut().editing = None;
        h.run_steps(2);
        h.state_mut().find = None;
        h.run_steps(2);
        h.state_mut().variables_open = true;
        h.state_mut().import_report = vec!["a transition was replaced with Fade".into(), "a freeform shape was simplified".into()];
        h.run_steps(3);
        shot(&mut h, "5n-variables-import-report.png");
        h.state_mut().variables_open = false;
        h.state_mut().right_tab = 0;
        h.state_mut().import_report.clear();
        h.state_mut().right_tab = 0;
        h.state_mut().bottom_tab = 0;
        h.state_mut().layer_edit = Some(1);
        h.run_steps(3);
        shot(&mut h, "5l2-layer-rename.png");
        h.state_mut().layer_edit = None;
        h.state_mut().layer = 0;
        h.state_mut().selected = None;
        h.state_mut().file_view = true;
        h.state_mut().file_page = backstage::FilePage::New;
        shot(&mut h, "6c-file-new.png");
        h.state_mut().file_page = backstage::FilePage::Info;
        shot(&mut h, "6a-file-info.png");
        h.state_mut().file_page = backstage::FilePage::Print;
        shot(&mut h, "6b-file-print.png");
        h.state_mut().file_view = false;
        h.state_mut().file_page = backstage::FilePage::Info;
        h.state_mut().ribbon_tab = 1;
        h.ctx.set_theme(egui::ThemePreference::Light);
        h.run_steps(3);
        shot(&mut h, "5j-light-home.png");
        h.state_mut().ribbon_tab = 2;
        shot(&mut h, "5k-light-insert.png");
        h.state_mut().ribbon_tab = 1;
        h.ctx.set_theme(egui::ThemePreference::Dark);
        h.run_steps(3);
    }

    /// Renders the first slides of every .story in samples (visual review only).
    #[test]
    fn gui_screenshot_inputs() {
        let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") else { return };
        let inputs = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples");
        let Ok(rd) = std::fs::read_dir(&inputs) else { return };
        let only = std::env::var("BRAINBEND_ONLY").ok();
        for entry in rd.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("story") { continue; }
            let stem = path.file_stem().unwrap().to_string_lossy().to_string();
            if only.as_ref().is_some_and(|o| !stem.contains(o.as_str())) { continue; }
            let mut app = App::default();
            app.open(&path);
            if app.project.is_none() { continue; }
            let n = app.project.as_ref().unwrap().slides.len();
            let mut h = harness(app);
            let picks: Vec<usize> = match std::env::var("BRAINBEND_SLIDES") {
                Ok(v) => v.split(',').filter_map(|x| x.trim().parse::<usize>().ok()).filter(|&i| i < n).collect(),
                Err(_) => vec![0, 1.min(n - 1)],
            };
            for i in picks {
                h.state_mut().slide = i;
                h.state_mut().layer = 0;
                h.state_mut().reset_visibility();
                h.run_steps(3);
                let name = format!("in-{}-{i}.png", stem.chars().take(24).collect::<String>());
                h.render().expect("renderer").save(std::path::Path::new(&dir).join(name)).unwrap();
                if n == 1 { break; }
            }
        }
    }

    /// Renders slides of the .pptx files named in BRAINBEND_PPTX (colon separated) for visual review.
    #[test]
    fn gui_screenshot_pptx() {
        let (Some(dir), Ok(files)) = (std::env::var_os("BRAINBEND_SHOT_DIR"), std::env::var("BRAINBEND_PPTX")) else { return };
        for f in files.split(':').filter(|f| !f.is_empty()) {
            let path = std::path::PathBuf::from(f);
            let stem = path.file_stem().unwrap().to_string_lossy().to_string();
            let mut app = App::default();
            app.open(&path);
            if app.project.is_none() { continue; }
            let n = app.project.as_ref().unwrap().slides.len();
            let mut h = harness(app);
            let picks: Vec<usize> = match std::env::var("BRAINBEND_SLIDES") {
                Ok(v) => v.split(',').filter_map(|x| x.trim().parse::<usize>().ok()).filter(|&i| i < n).collect(),
                Err(_) => (0..n.min(4)).collect(),
            };
            if let Ok(v) = std::env::var("BRAINBEND_MASTER") {
                // Slide Master view on layout number `v` (1-based), instead of the slides.
                h.state_mut().enter_master_view();
                h.state_mut().slide = v.parse().unwrap_or(1);
                h.run_steps(3);
                h.render().expect("renderer").save(std::path::Path::new(&dir).join(format!("pptx-{}-master.png", stem.chars().filter(|c| c.is_alphanumeric()).take(20).collect::<String>()))).unwrap();
                continue;
            }
            for i in picks {
                h.state_mut().slide = i;
                h.state_mut().layer = 0;
                h.state_mut().reset_visibility();
                h.run_steps(3);
                let name = format!("pptx-{}-{i}.png", stem.chars().filter(|c| c.is_alphanumeric()).take(20).collect::<String>());
                h.render().expect("renderer").save(std::path::Path::new(&dir).join(name)).unwrap();
            }
        }
    }

    #[test]
    fn gui_empty_start_screen_renders() {
        let mut h = harness(App::default());
        h.run_steps(3);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.render().expect("renderer").save(std::path::Path::new(&dir).join("0-start-page.png")).unwrap();
        }
    }

    #[test]
    fn gui_imported_sample_survives_selection_and_layer_changes() {
        let Some(app) = sample() else { return };
        let mut h = harness(app);
        h.run_steps(3);
        let layers = h.state().project.as_ref().unwrap().slides[0].layers.len();
        for li in 0..layers.min(12) {
            h.state_mut().layer = li;
            if li > 0 { h.state_mut().visible[li] = true; }
            let n = h.state().project.as_ref().unwrap().slides[0].layers[li].objects.len();
            for oi in 0..n.min(4) {
                h.state_mut().selected = Some((li, oi));
                h.run_steps(2);
            }
            h.state_mut().selected = None;
            h.run_steps(2);
        }
    }

    #[test]
    fn gui_every_slide_kind_and_the_trigger_panel_render() {
        let Some(app) = sample() else { return };
        let mut h = harness(app);
        h.run_steps(2);
        for q in [Question::new_single(), Question::new_true_false(), Question::new_text_entry(), Question::new_matching(), Question::new_sequence(), Question::new_fill_blanks(), Question::new_drag_words(), Question::new_mark_words(), Question::new_numeric(), Question::new_categorize(), Question::new_hotspot(), Question::new_likert(), Question::new_reflection(), Question::new_dictation(), Question::new_dropdowns(), Question::new_label_picture()] {
            h.state_mut().add_slide(SlideKind::Question(q));
            h.run_steps(3);
        }
        h.state_mut().add_slide(SlideKind::Results);
        h.run_steps(3);
        h.state_mut().add_slide(SlideKind::Normal);
        h.state_mut().insert_button();
        h.state_mut().insert_text();
        h.state_mut().insert_ellipse();
        h.state_mut().add_layer();
        h.run_steps(3);
        // Selecting an object shows its triggers; give it one of each action kind and render.
        let id = h.state().selected_object().map(|o| o.id.clone());
        if let Some(id) = id {
            let slide = &mut h.state_mut().project.as_mut().unwrap().slides[3];
            let target = slide.layers.last().unwrap().id.clone();
            for (i, a) in [Action::ShowLayer(target.clone()), Action::HideLayer(target), Action::NextSlide, Action::PreviousSlide,
                           Action::JumpToSlide("nope".into()), Action::RunJavaScript("1".into()),
                           Action::SetObjectState { object: id.clone(), state: "Hover".into() },
                           Action::ShowObject(id.clone()), Action::HideObject(id.clone()), Action::PlayMedia(id.clone()), Action::PauseMedia(id.clone()), Action::StopMedia(id.clone()), Action::PlayAnimation(id.clone()),
                           Action::AdjustVariable { variable: "none".into(), op: AdjustOp::Add, value: Value::Num(1.0) },
                           Action::OpenUrl("https://example.com".into()),
                           Action::SeekTimeline { target: String::new(), ms: 2500 }, Action::CompleteCourse, Action::ExitCourse, Action::RestartCourse].into_iter().enumerate() {
                slide.triggers.push(Trigger { id: format!("t{i}"), owner: Owner::Object(id.clone()), event: Event::Click, action: a,
                    conditions: vec![Condition { variable: "none".into(), cmp: Cmp::Eq, value: Value::Bool(true), or_with_previous: i % 2 == 0 }] });
            }
            h.state_mut().selected = Some((0, 0));
        }
        h.run_steps(4);
        h.state_mut().editing = Some((0, 0));
        h.run_steps(3);
        // Delete the slide the UI is looking at; the next frames must not index past the end.
        let last = h.state().project.as_ref().unwrap().slides.len() - 1;
        h.state_mut().slide = last;
        h.state_mut().delete_slide(last);
        h.run_steps(3);
        h.state_mut().undo();
        h.run_steps(3);
    }

    #[test]
    fn gui_format_painter_copies_the_look_to_the_next_object() {
        let mut a = App::default();
        a.new_project();
        a.insert_text();
        a.insert_text();
        {
            let o = &mut a.project.as_mut().unwrap().slides[0].layers[0].objects[0];
            o.style.size = 77.0;
            o.style.align = Align::Justify;
            o.fill = Some(Rgba(10, 20, 30, 255));
        }
        let src = a.project.as_ref().unwrap().slides[0].layers[0].objects[0].clone();
        a.selected = Some((0, 1));
        a.painter = Some((src.id.clone(), Box::new(src)));
        let mut h = harness(a);
        h.run_steps(4);
        let o = &h.state().project.as_ref().unwrap().slides[0].layers[0].objects[1];
        assert_eq!((o.style.size, o.style.align, o.fill), (77.0, Align::Justify, Some(Rgba(10, 20, 30, 255))));
        assert!(h.state().painter.is_none(), "one use only");
    }

    /// Renders struck, highlighted, spaced and indented text to `text-formats.png` in BRAINBEND_SHOT_DIR (visual review only).
    #[test]
    fn gui_text_formats_render() {
        use brainbend_doc::richtext::Attr;
        let mut a = App::default();
        a.new_project();
        a.insert_text();
        {
            let o = &mut a.project.as_mut().unwrap().slides[0].layers[0].objects[0];
            o.w = 900.0; o.h = 700.0;
            o.text = "Struck words here\nHighlighted words here\nSpaced out letters\nIndented twice and long enough to wrap onto a second and third line of the box\nBack to normal".into();
            let at = |t: &str, s: &str| t.find(s).map(|b| t[..b].chars().count()).unwrap();
            let t = o.text.clone();
            o.format_range(0, 6, &Attr::ToggleStrike);
            o.format_range(at(&t, "Highlighted"), at(&t, "Highlighted") + 11, &Attr::Highlight(Some(Rgba(255, 240, 0, 255))));
            o.format_range(at(&t, "Spaced"), at(&t, "Spaced") + 6, &Attr::Spacing(6.0));
            let i = at(&t, "Indented");
            o.format_range(i, i + 8, &Attr::Indent(2));
            o.text.push_str("\nA bullet item long enough to wrap onto a second line in this box\nAnother item, indented once and also long enough to wrap around");
            o.edit_text(&o.text.clone());
            let t = o.text.clone();
            let b = at(&t, "A bullet");
            o.format_range(b, t.chars().count(), &Attr::ToggleStrike);
            o.format_range(b, t.chars().count(), &Attr::ToggleStrike);
            o.set_list(brainbend_doc::richtext::ListStyle::Bullet);
            let t = o.text.clone();
            let c = at(&t, "• Another");
            o.format_range(c, c + 3, &Attr::Indent(1));
        }
        a.selected = Some((0, 0));
        let mut h = harness(a);
        h.run_steps(4);
        let o = &h.state().project.as_ref().unwrap().slides[0].layers[0].objects[0];
        assert_eq!(o.rich.as_ref().unwrap()[3].indent, 2);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.render().expect("renderer").save(std::path::Path::new(&dir).join("text-formats.png")).unwrap();
            // The same text while it is being edited in place.
            h.state_mut().editing = Some((0, 0));
            h.run_steps(4);
            h.render().expect("renderer").save(std::path::Path::new(&dir).join("text-formats-editing.png")).unwrap();
            // Centred and right-aligned text edits keep their alignment and indent too.
            h.state_mut().project.as_mut().unwrap().slides[0].layers[0].objects[0].style.align = Align::Center;
            h.run_steps(4);
            h.render().expect("renderer").save(std::path::Path::new(&dir).join("text-formats-editing-centre.png")).unwrap();
            h.state_mut().editing = None;
            h.run_steps(3);
            h.render().expect("renderer").save(std::path::Path::new(&dir).join("text-formats-centre.png")).unwrap();
            { use egui_kittest::kittest::Queryable; h.get_by_label("Normal Text").click(); }
            h.run_steps(4);
            h.render().expect("renderer").save(std::path::Path::new(&dir).join("text-styles-menu.png")).unwrap();
        }
    }

    #[test]
    fn text_edits_without_a_chosen_range_act_on_every_selected_object() {
        let mut a = App::default();
        a.new_project();
        a.insert_text();
        a.insert_text();
        a.selected = Some((0, 0));
        a.multi = vec![a.project.as_ref().unwrap().slides[0].layers[0].objects[1].id.clone()];
        a.edit_text_range(|o, s, e, whole| { assert!(whole); o.format_range(s, e, &brainbend_doc::richtext::Attr::ToggleStrike); });
        for o in &a.project.as_ref().unwrap().slides[0].layers[0].objects {
            assert!(o.char_fmts().iter().all(|(_, f)| f.strike == Some(true)), "{} is struck through", o.name);
        }
        a.undo();
        assert!(a.project.as_ref().unwrap().slides[0].layers[0].objects.iter().all(|o| o.rich.is_none()), "one undo step");
    }

    #[test]
    fn a_text_style_restyles_the_whole_text_from_the_theme() {
        use brainbend_doc::richtext::Attr;
        let mut a = App::default();
        a.new_project();
        a.insert_text();
        {
            let o = &mut a.project.as_mut().unwrap().slides[0].layers[0].objects[0];
            o.text = "Some words here".into();
            o.format_range(0, 4, &Attr::ToggleBold);
            o.format_range(5, 10, &Attr::Size(99.0));
        }
        a.selected = Some((0, 0));
        a.apply_text_style(TextRole::H1);
        let theme = a.project.as_ref().unwrap().theme.clone();
        let o = &a.project.as_ref().unwrap().slides[0].layers[0].objects[0];
        assert_eq!((o.style.size, o.style.bold, o.style.font_family.as_str(), o.style.color, o.role), (64.0, false, theme.heading_font.as_str(), theme.colors[0], TextRole::H1));
        assert!(o.rich.is_none(), "character formatting is gone");
        a.undo();
        assert!(a.project.as_ref().unwrap().slides[0].layers[0].objects[0].rich.is_some(), "one undo step");
    }

    #[test]
    fn quick_styles_pick_readable_text_and_apply_to_the_selected_shape() {
        let theme = brainbend_doc::Theme::default().colors;
        let white = Rgba(255, 255, 255, 255);
        let solid = ribbon::quick_look(2, theme[4], &theme);
        assert_eq!((solid.fill, solid.stroke, solid.text), (theme[4], None, white), "white on a dark accent");
        let pale = ribbon::quick_look(2, Rgba(250, 230, 120, 255), &theme);
        assert_eq!(pale.text, theme[0], "dark text on a light colour");
        assert!(ribbon::quick_look(3, theme[5], &theme).shadow.is_some());
        let mut a = App::default();
        a.new_project();
        a.insert_shape("rect");
        a.selected = Some((0, 0));
        let mut h = harness(a);
        h.run_steps(4);
        { use egui_kittest::kittest::Queryable; h.get_by_label("Quick Styles").click(); }
        h.run_steps(3);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.render().expect("renderer").save(std::path::Path::new(&dir).join("quick-styles-menu.png")).unwrap();
        }
        { use egui_kittest::kittest::Queryable; h.get_by_label("Intense, Accent 1").click(); }
        h.run_steps(3);
        let o = &h.state().project.as_ref().unwrap().slides[0].layers[0].objects[0];
        assert_eq!(o.fill, Some(theme[4]));
        assert!(o.shadow.is_some() && o.stroke.is_some());
    }

    #[test]
    fn gui_focus_order_window_reorders_and_resets() {
        let mut a = App::default();
        a.new_project();
        a.insert_text();
        a.insert_text();
        a.insert_text();
        a.focus_open = true;
        let ids: Vec<String> = a.project.as_ref().unwrap().slides[0].layers[0].objects.iter().map(|o| o.id.clone()).collect();
        let mut h = harness(a);
        h.run_steps(4);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.render().expect("renderer").save(std::path::Path::new(&dir).join("focus-order.png")).unwrap();
        }
        // The first row's down arrow moves it one place later.
        { use egui_kittest::kittest::Queryable; h.get_all_by_label(&ribbon::ARROW_DOWN.to_string()).next().unwrap().click(); }
        h.run_steps(3);
        let order = |h: &egui_kittest::Harness<'_, App>| h.state().project.as_ref().unwrap().slides[0].layers[0].focus_order.clone();
        assert_eq!(order(&h), vec![ids[1].clone(), ids[0].clone(), ids[2].clone()]);
        { use egui_kittest::kittest::Queryable; h.get_by_label("Reset Order").click(); }
        h.run_steps(3);
        assert!(order(&h).is_empty());
        h.state_mut().undo();
        h.run_steps(2);
        assert_eq!(order(&h).len(), 3, "reset is one undo step");
    }

    #[test]
    fn gui_web_object_is_inserted_and_shows_its_address() {
        let mut a = App::default();
        a.new_project();
        a.insert_web_object();
        assert!(matches!(a.selected_object().map(|o| &o.kind), Some(Kind::Web(w)) if w.url.is_empty() && w.scroll));
        if let Kind::Web(w) = &mut a.project.as_mut().unwrap().slides[0].layers[0].objects[0].kind { w.url = "https://example.com/glossary".into(); }
        let mut h = harness(a);
        h.run_steps(4);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.render().expect("renderer").save(std::path::Path::new(&dir).join("web-object.png")).unwrap();
        }
        { use egui_kittest::kittest::Queryable; assert!(h.query_by_label("Web address").is_some(), "Properties offers the address"); }
        let issues = h.state().project.as_ref().unwrap().check();
        assert!(issues.iter().any(|i| i.message.contains("no title for screen readers")), "the checker asks for a title");
    }

    #[test]
    fn gui_icons_insert_as_vector_shapes_with_their_holes() {
        let mut a = App::default();
        a.new_project();
        for name in ["caret-double-up", "arrow-bend-up-right", "arrow-circle-left", "bus", "envelope", "arrow-line-up-right", "chat-centered", "check-circle"] {
            let icon = icons::all().iter().find(|i| i.names[0] == name).unwrap();
            a.insert_icon(icon);
            let o = a.selected_object().unwrap();
            assert!(matches!(o.kind, Kind::Path(_)) && o.fill.is_some() && o.stroke.is_none());
            assert_eq!(o.alt, name.replace('-', " "), "described for screen readers");
        }
        {
            let p = a.project.as_mut().unwrap();
            let objs = &mut p.slides[0].layers[0].objects;
            for (i, o) in objs.iter_mut().enumerate() { o.x = 60.0 + (i % 4) as f32 * 240.0; o.y = 100.0 + (i / 4) as f32 * 260.0; }
        }
        a.selected = None;
        let mut h = harness(a);
        h.run_steps(4);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.render().expect("renderer").save(std::path::Path::new(&dir).join("icons-on-slide.png")).unwrap();
            h.state_mut().ribbon_tab = 2;
            h.run_steps(2);
            { use egui_kittest::kittest::Queryable; h.get_by_label("Icons").click(); }
            h.run_steps(4);
            h.render().expect("renderer").save(std::path::Path::new(&dir).join("icons-gallery.png")).unwrap();
        }
    }

    #[test]
    fn every_icon_fills_completely() {
        // The triangles must cover exactly the shape: outer pieces minus holes, summed over figures by nesting depth.
        let frame = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1000.0, 1000.0));
        let poly_area = |f: &[egui::Pos2]| -> f32 { (0..f.len()).map(|i| { let (a, b) = (f[i], f[(i + 1) % f.len()]); a.x * b.y - b.x * a.y }).sum::<f32>().abs() / 2.0 };
        let mut bad = vec![];
        for icon in icons::all() {
            let Some((cmds, _)) = icons::outline(icon) else { continue };
            let figs = path_polygons(&cmds, frame);
            let expected: f32 = {
                use i_overlay::core::fill_rule::FillRule;
                use i_overlay::float::simplify::SimplifyShape;
                let contours: Vec<Vec<[f32; 2]>> = figs.iter().filter(|f| f.len() > 2).map(|f| f.iter().map(|p| [p.x, p.y]).collect()).collect();
                contours.simplify_shape(FillRule::NonZero).iter().map(|sh| sh.iter().enumerate().map(|(k, c)| { let a = poly_area(&c.iter().map(|p| egui::pos2(p[0], p[1])).collect::<Vec<_>>()); if k == 0 { a } else { -a } }).sum::<f32>()).sum()
            };
            let covered: f32 = figure_triangles(&figs).iter().map(|(pts, tris)| tris.chunks_exact(3).map(|t| poly_area(&[pts[t[0]], pts[t[1]], pts[t[2]]])).sum::<f32>()).sum();
            if (covered - expected).abs() > expected.abs() * 0.03 + 5.0 { bad.push((icon.names[0], expected as i32, covered as i32)); }
        }
        assert!(bad.is_empty(), "{} icons are not fully covered: {:?}", bad.len(), &bad[..bad.len().min(8)]);
    }

    #[test]
    fn slide_number_text_shows_the_real_number_in_the_editor() {
        let mut a = App::default();
        a.new_project();
        a.add_slide(SlideKind::Normal);
        a.add_slide(SlideKind::Normal);
        a.slide = 1;
        a.insert_slide_number(true);
        let o = a.selected_object().unwrap().clone();
        assert_eq!(o.text, "%Slide.Number% of %Slide.Total%");
        assert!(has_slide_tokens(&o) && o.style.align == Align::Right);
        let slide = a.project.as_ref().unwrap().slides[1].clone();
        let nt = a.slide_numbers(&slide).unwrap();
        assert_eq!(nt, (2, 3));
        let mut shown = o.clone();
        number_tokens(&mut shown, nt);
        assert_eq!(shown.text, "2 of 3");
        assert_eq!(o.text, "%Slide.Number% of %Slide.Total%", "the stored text keeps the token");
        let mut rich = o.clone();
        rich.rich = Some(vec![brainbend_doc::Paragraph { runs: vec![brainbend_doc::Run { text: "No. %Slide.Number%".into(), ..Default::default() }], ..Default::default() }]);
        number_tokens(&mut rich, nt);
        assert_eq!(rich.rich.unwrap()[0].runs[0].text, "No. 2");
    }

    /// Renders the three dial looks and the Dial menu (visual review only; needs BRAINBEND_SHOT_DIR).
    #[test]
    fn gui_dial_looks_render() {
        let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") else { return };
        let mut a = App::default();
        a.new_project();
        for style in [DialStyle::Teardrop, DialStyle::Dot, DialStyle::Needle] { a.insert_dial(style); }
        for (i, o) in a.project.as_mut().unwrap().slides[0].layers[0].objects.iter_mut().enumerate() { o.x = 120.0 + (i / 2) as f32 * 420.0; o.y = 260.0; }
        a.selected = None;
        a.ribbon_tab = 2;
        let mut h = harness(a);
        h.run_steps(4);
        h.render().expect("renderer").save(std::path::Path::new(&dir).join("dials-on-slide.png")).unwrap();
        { use egui_kittest::kittest::Queryable; h.get_by_label("Dial").click(); }
        h.run_steps(4);
        h.render().expect("renderer").save(std::path::Path::new(&dir).join("dial-menu.png")).unwrap();
    }

    #[test]
    fn gui_dials_are_inserted_grouped_with_a_variable_of_their_own() {
        let mut a = App::default();
        a.new_project();
        for style in [DialStyle::Teardrop, DialStyle::Dot, DialStyle::Needle] {
            let before = a.project.as_ref().unwrap().slides[0].layers[0].objects.len();
            a.insert_dial(style);
            let objs = &a.project.as_ref().unwrap().slides[0].layers[0].objects;
            assert_eq!(objs.len(), before + 2, "a knob or hub and a pointer");
            let pair = &objs[before..];
            assert!(pair[0].group.is_some() && pair[0].group == pair[1].group, "grouped");
            let dials: Vec<_> = pair.iter().filter(|o| o.dial.is_some()).collect();
            assert_eq!(dials.len(), 1, "only the pointer turns");
            assert!(matches!(dials[0].kind, Kind::Path(_)) && !dials[0].alt.is_empty());
            assert_eq!(a.selected_object().map(|o| o.id.clone()), Some(dials[0].id.clone()), "the pointer is selected");
            let var = dials[0].dial.as_ref().unwrap().variable.clone();
            assert!(a.project.as_ref().unwrap().variables.iter().any(|v| v.id == var), "its variable exists");
        }
        let names: std::collections::HashSet<_> = a.project.as_ref().unwrap().variables.iter().filter(|v| v.name.starts_with("Dial")).map(|v| v.name.clone()).collect();
        assert_eq!(names.len(), 3, "each dial has its own: {names:?}");
        a.undo();
        assert_eq!(a.project.as_ref().unwrap().slides[0].layers[0].objects.len(), 4, "one undo step per dial");
        // Convert to Dial: any selected object.
        a.insert_rect();
        assert!(a.selected_object().unwrap().dial.is_none());
        a.convert_to_dial();
        assert!(a.selected_object().unwrap().dial.is_some());
        let n = a.project.as_ref().unwrap().variables.iter().filter(|v| v.name.starts_with("Dial")).count();
        a.convert_to_dial();
        assert_eq!(a.project.as_ref().unwrap().variables.iter().filter(|v| v.name.starts_with("Dial")).count(), n, "already a dial: nothing added");
    }

    #[test]
    fn gui_inputs_check_boxes_radio_buttons_and_entry_fields() {
        let mut a = App::default();
        a.new_project();
        // Radio buttons in a row join one group; a check box between them does not start a new one by itself.
        a.insert_radio_look(0);
        a.insert_radio_look(2);
        let g1 = { let o = a.selected_object().unwrap(); o.check.as_ref().unwrap().radio.clone().unwrap() };
        let first = a.project.as_ref().unwrap().slides[0].layers[0].objects[0].check.as_ref().unwrap().radio.clone().unwrap();
        assert_eq!(first, g1, "the second joins the first's group");
        a.insert_check_look(1);
        assert!(a.selected_object().unwrap().check.as_ref().unwrap().radio.is_none() && a.selected_object().unwrap().check.as_ref().unwrap().size > 60.0, "the Large look");
        a.insert_radio_look(3);
        let g2 = a.selected_object().unwrap().check.as_ref().unwrap().radio.clone().unwrap();
        assert_ne!(g2, g1, "after a check box a radio button starts a new group");
        // Entry fields each get a variable of the right type, in the same undo step.
        let vars = |a: &App| a.project.as_ref().unwrap().variables.len();
        let before = vars(&a);
        a.insert_input(false);
        a.insert_input(true);
        assert_eq!(vars(&a), before + 2);
        let objs = &a.project.as_ref().unwrap().slides[0].layers[0].objects;
        let (t, n) = (&objs[objs.len() - 2], &objs[objs.len() - 1]);
        let (Kind::Input(tf), Kind::Input(nf)) = (&t.kind, &n.kind) else { panic!("two entry fields") };
        let kind_of = |id: &str| a.project.as_ref().unwrap().variables.iter().find(|v| v.id == id).map(|v| v.initial.clone());
        assert!(matches!(kind_of(&tf.variable), Some(Value::Text(_))) && !tf.numeric);
        assert!(matches!(kind_of(&nf.variable), Some(Value::Num(_))) && nf.numeric);
        assert!(!t.alt.is_empty() && !n.alt.is_empty(), "labelled for screen readers from the start");
        a.undo();
        assert_eq!(vars(&a), before + 1, "undo takes the field and its variable away together");
    }

    /// Renders markers on a slide, the Marker gallery and the Symbol popup (visual review only; needs BRAINBEND_SHOT_DIR).
    #[test]
    fn gui_marker_and_symbol_render() {
        use crate::markers::Face;
        let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") else { return };
        let mut a = App::default();
        a.new_project();
        a.insert_text();
        for name in ["house", "magnifying-glass", "arrow-circle-right", "chat-text", "gear", "thumbs-up", "chart-bar"] {
            let icon = icons::all().iter().find(|i| i.names[0] == name).unwrap();
            a.insert_marker(Face::Icon(icon));
        }
        a.insert_marker(Face::Number(7));
        {
            let objs = &mut a.project.as_mut().unwrap().slides[0].layers[0].objects;
            for (i, o) in objs.iter_mut().enumerate().skip(1) { o.x = 60.0 + ((i - 1) % 4) as f32 * 180.0; o.y = 60.0 + ((i - 1) / 4) as f32 * 170.0; }
        }
        a.selected = None;
        a.ribbon_tab = 2;
        let mut h = harness(a);
        h.run_steps(4);
        h.render().expect("renderer").save(std::path::Path::new(&dir).join("markers-on-slide.png")).unwrap();
        // A selected marker shows its card above it, as the learner will see it.
        {
            let o = &mut h.state_mut().project.as_mut().unwrap().slides[0].layers[0].objects[8];
            o.x = 700.0; o.y = 640.0;
            if let Some(m) = o.marker.as_mut() { m.title = "Testing".into(); m.description = "Testing testing testing".into(); }
        }
        h.state_mut().selected = Some((0, 8));
        h.run_steps(3);
        h.render().expect("renderer").save(std::path::Path::new(&dir).join("marker-card.png")).unwrap();
        h.state_mut().selected = None;
        h.run_steps(2);
        { use egui_kittest::kittest::Queryable; h.get_by_label("Marker").click(); }
        h.run_steps(4);
        h.render().expect("renderer").save(std::path::Path::new(&dir).join("marker-gallery.png")).unwrap();
        h.key_press(egui::Key::Escape);
        h.run_steps(3);
        h.state_mut().selected = Some((0, 0));
        h.run_steps(2);
        { use egui_kittest::kittest::Queryable; h.get_by_label("Ω").click(); }
        h.run_steps(4);
        h.render().expect("renderer").save(std::path::Path::new(&dir).join("symbols.png")).unwrap();
    }

    #[test]
    fn gui_captions_insert_with_text_clear_of_the_pointer() {
        let mut a = App::default();
        a.new_project();
        for i in 0..brainbend_doc::shapes::CAPTIONS.len() {
            a.insert_caption(i);
            let o = a.selected_object().unwrap();
            assert_eq!(o.name, brainbend_doc::shapes::CAPTIONS[i].0);
            assert_eq!(o.text, "Type your caption");
            assert!(o.fill.is_some() && o.style.align == Align::Center);
            assert_eq!(o.margins[3] > 14.0, brainbend_doc::shapes::CAPTIONS[i].1, "only pointer captions keep the text clear of the bottom");
        }
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            {
                let objs = &mut a.project.as_mut().unwrap().slides[0].layers[0].objects;
                for (i, o) in objs.iter_mut().enumerate() { o.w = 400.0; o.h = 240.0; o.x = 40.0 + (i % 4) as f32 * 450.0; o.y = 40.0 + (i / 4) as f32 * 300.0; o.margins[3] = if i % 2 == 0 { 12.0 + 0.26 * 240.0 } else { 12.0 }; }
            }
            a.selected = None;
            a.ribbon_tab = 2;
            let mut h = harness(a);
            h.run_steps(4);
            h.render().expect("renderer").save(std::path::Path::new(&dir).join("captions-on-slide.png")).unwrap();
            { use egui_kittest::kittest::Queryable; h.get_by_label("Caption").click(); }
            h.run_steps(4);
            h.render().expect("renderer").save(std::path::Path::new(&dir).join("caption-menu.png")).unwrap();
        }
    }

    /// Renders the Input menu and the entry fields and radio buttons on a slide (visual review only).
    #[test]
    fn gui_input_looks_render() {
        let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") else { return };
        let mut a = App::default();
        a.new_project();
        a.insert_check_look(0);
        a.insert_radio_look(0);
        a.insert_radio_look(1);
        a.insert_input(false);
        a.insert_input(true);
        {
            let objs = &mut a.project.as_mut().unwrap().slides[0].layers[0].objects;
            for (i, o) in objs.iter_mut().enumerate() { o.x = 100.0; o.y = 100.0 + i as f32 * 150.0; if i == 2 { o.initial_state = "Selected".into(); } if i == 0 { o.initial_state = "Selected".into(); } }
        }
        a.selected = None;
        a.ribbon_tab = 2;
        let mut h = harness(a);
        h.run_steps(4);
        h.render().expect("renderer").save(std::path::Path::new(&dir).join("inputs-on-slide.png")).unwrap();
        { use egui_kittest::kittest::Queryable; h.get_by_label("Input").click(); }
        h.run_steps(4);
        h.render().expect("renderer").save(std::path::Path::new(&dir).join("input-menu.png")).unwrap();
    }

    #[test]
    fn a_hyperlink_with_nothing_chosen_makes_a_linked_text_box() {
        let mut a = App::default();
        a.new_project();
        a.insert_link_text("", "https://example.com");
        let o = a.selected_object().unwrap();
        assert_eq!(o.text, "https://example.com");
        assert!(o.char_fmts().iter().all(|(_, f)| f.link.as_deref() == Some("https://example.com")));
        a.insert_link_text("Our site", "https://example.com");
        assert_eq!(a.selected_object().unwrap().text, "Our site");
    }

    #[test]
    fn linked_text_is_drawn_in_the_link_colour_underlined() {
        use brainbend_doc::richtext::Attr;
        let mut a = App::default();
        a.new_project();
        a.insert_link_text("", "https://example.com");
        let mut o = a.selected_object().unwrap().clone();
        o.edit_text("go now");
        o.format_range(0, 6, &Attr::Link(None));
        o.format_range(0, 2, &Attr::Link(Some("https://example.com".into())));
        let blue = egui::Color32::from_rgb(1, 2, 3);
        let job = text_job(&o, 1.0, 500.0, blue);
        assert_eq!(job.sections[0].format.color, blue);
        assert_ne!(job.sections[0].format.underline, egui::Stroke::NONE);
        assert_ne!(job.sections.last().unwrap().format.color, blue, "unlinked text keeps its colour");
        // The author's own colour and underline on a link win, even when they equal the object's style.
        o.format_range(0, 2, &Attr::Color(o.style.color));
        o.format_range(0, 2, &Attr::ToggleUnderline);
        let job = text_job(&o, 1.0, 500.0, blue);
        assert_eq!(job.sections[0].format.color, color(o.style.color));
        assert_eq!(job.sections[0].format.underline, egui::Stroke::NONE);
    }

    #[test]
    fn symbols_go_at_the_cursor_or_replace_the_chosen_text() {
        use brainbend_doc::richtext::Attr;
        let mut a = App::default();
        a.new_project();
        a.insert_text();
        a.selected = Some((0, 0));
        { let o = &mut a.project.as_mut().unwrap().slides[0].layers[0].objects[0]; o.text = "abcd".into(); o.format_range(0, 2, &Attr::ToggleBold); }
        a.insert_symbol("Ω");
        assert_eq!(a.selected_object().unwrap().text, "abcdΩ", "with no cursor known it goes at the end");
        a.caret = Some((0, 0, 1));
        a.insert_symbol("π");
        assert_eq!(a.selected_object().unwrap().text, "aπbcdΩ", "at the cursor");
        assert_eq!(a.selected_object().unwrap().char_fmts()[1].1.bold, Some(true), "it takes the formatting of the character before");
        a.text_sel = Some((2, 4));
        a.insert_symbol("≤");
        assert_eq!(a.selected_object().unwrap().text, "aπ≤dΩ", "it replaces the chosen text");
        a.undo();
        assert_eq!(a.project.as_ref().unwrap().slides[0].layers[0].objects[0].text, "aπbcdΩ", "one undo step");
        a.selected = Some((0, 0));
        // Pictures take no text.
        a.insert_web_object();
        a.insert_symbol("x");
        assert_eq!(a.selected_object().unwrap().text, "");
        a.selected = None;
        a.insert_symbol("€");
        assert_eq!(a.selected_object().unwrap().text, "€", "with nothing selected a symbol makes a text box");
    }

    #[test]
    fn gui_markers_are_round_buttons_with_an_icon_or_a_number() {
        use crate::markers::{Face, CATEGORIES};
        let mut a = App::default();
        a.new_project();
        let home = icons::all().iter().find(|i| i.names[0] == "house").unwrap();
        a.insert_marker(Face::Icon(home));
        let o = a.selected_object().unwrap().clone();
        assert!(matches!(o.kind, Kind::Path(_)) && o.has_button_states() && o.alt == "house" && o.fill.is_some(), "an icon marker is a button with a path face");
        assert_eq!(o.w, o.h);
        let info = o.marker.as_ref().expect("a marker has a title and a description to show");
        assert!(!info.title.is_empty() && !info.description.is_empty() && info.card_w > 100.0);
        a.insert_marker(Face::Number(7));
        let n = a.selected_object().unwrap();
        assert!(matches!(n.kind, Kind::Ellipse) && n.text == "7" && n.has_button_states() && n.style.bold, "a number marker is a round button with the number in it");
        assert_eq!(CATEGORIES.len(), 9);
        assert!(a.project.as_ref().unwrap().slides[0].triggers.is_empty() && a.project.as_ref().unwrap().slides[0].layers.len() == 1, "a marker brings nothing else with it");
    }

    #[test]
    fn gui_equations_insert_edit_and_keep_their_scale() {
        let mut a = App::default();
        a.new_project();
        a.insert_equation(r"\frac{a}{b}", true).unwrap();
        let o = a.selected_object().unwrap().clone();
        let info = o.equation.clone().expect("the LaTeX is kept");
        assert!(matches!(o.kind, Kind::Path(_)) && o.fill.is_some() && info.mathml.starts_with("<math") && info.latex == r"\frac{a}{b}");
        assert!((o.w / info.width_pt - equations::DEFAULT_SCALE).abs() < 0.01, "set at the default size");
        assert!(a.insert_equation(r"\SI{5}{m}", true).is_err(), "unsupported LaTeX is refused");
        assert_eq!(a.project.as_ref().unwrap().slides[0].layers[0].objects.len(), 1, "nothing was added");
        // Resize it, then change the LaTeX: it keeps the size the author gave it (the same scale).
        let pos = a.selected.unwrap();
        { let q = &mut a.project.as_mut().unwrap().slides[0].layers[0].objects[pos.1]; q.w *= 2.0; q.h *= 2.0; }
        let before = a.selected_object().unwrap().w;
        a.update_equation(pos, r"\frac{a}{b}+\frac{c}{d}", true).unwrap();
        let after = a.selected_object().unwrap();
        let info2 = after.equation.as_ref().unwrap();
        assert!(after.w > before, "a longer equation is wider");
        assert!((after.w / info2.width_pt - equations::DEFAULT_SCALE * 2.0).abs() < 0.01, "at twice the default scale, as it was resized");
        a.undo();
        assert_eq!(a.project.as_ref().unwrap().slides[0].layers[0].objects[pos.1].equation.as_ref().unwrap().latex, r"\frac{a}{b}", "one undo step");
    }

    /// A 512 x 256 test panorama: sky above, ground below, and a vertical stripe every 45 degrees.
    fn test_panorama() -> Vec<u8> {
        let mut img = image::RgbImage::new(512, 256);
        for (x, y, px) in img.enumerate_pixels_mut() {
            let stripe = (x * 8 / 512) % 2 == 0;
            *px = if y < 128 { image::Rgb([90, 160, if stripe { 255 } else { 220 }]) } else { image::Rgb([if stripe { 90 } else { 60 }, 120, 60]) };
        }
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(img).write_to(&mut png, image::ImageFormat::Png).unwrap();
        png.into_inner()
    }

    #[test]
    fn gui_360_image_is_inserted_then_edited_with_markers_and_hotspots() {
        use egui_kittest::kittest::Queryable;
        use brainbend_doc::pano::PinKind;
        let mut a = App::default();
        a.new_project();
        a.project.as_mut().unwrap().assets.insert("pano".into(), Asset { name: "lab.png".into(), mime: "image/png".into(), bytes: test_panorama(), file: None });
        a.add_pano("pano".into(), "lab.png", 512.0, 256.0);
        let o = a.selected_object().unwrap();
        assert!(matches!(&o.kind, Kind::Pano(p) if p.pins.is_empty() && p.fov == 90.0), "{:?}", o.kind);
        assert_eq!(o.asset.as_deref(), Some("pano"));
        assert!(o.w > 600.0 && (o.w / o.h - 16.0 / 9.0).abs() < 0.05, "a 16:9 viewer, {} by {}", o.w, o.h);
        assert!(a.status.starts_with("Added lab.png"), "{}", a.status);
        let at = a.selected.unwrap();
        let undo_before = a.undo.len();
        // A visit that changes nothing leaves no undo step.
        a.open_pano_dialog(at);
        a.close_pano_edit();
        assert_eq!(a.undo.len(), undo_before);
        a.open_pano_dialog(at);
        let mut h = harness(a);
        h.run_steps(3);
        assert!(h.state().pano.edit.is_some());
        // The mode replaces the slide editor: no slide list, a ribbon of its own.
        assert!(h.query_by_label("Marker").is_some() && h.query_by_label("Hotspot").is_some());
        // Look around, then add a marker where the view points.
        h.state_mut().pano.edit.as_mut().unwrap().view = (90.0, 10.0, 70.0);
        h.run_steps(2);
        h.get_by_label("Marker").click();
        h.run_steps(2);
        let pano = |h: &egui_kittest::Harness<'_, App>| match &h.state().project.as_ref().unwrap().slides[0].layers[0].objects[at.1].kind { Kind::Pano(p) => p.clone(), _ => panic!() };
        let p = pano(&h);
        assert_eq!(p.pins.len(), 1);
        assert_eq!((p.pins[0].kind, p.pins[0].yaw, p.pins[0].pitch), (PinKind::Marker, 90.0, 10.0));
        assert_eq!(p.pins[0].icon, "plus");
        assert!(!p.pins[0].icon_path.is_empty());
        assert_eq!(h.state().pano.edit.as_ref().unwrap().selected, Some(0), "the new marker is selected");
        // Switch it into a hotspot and back.
        h.get_by_label("Switch Type").click();
        h.run_steps(2);
        let p = pano(&h);
        assert_eq!(p.pins[0].kind, PinKind::Hotspot);
        assert_eq!(p.pins[0].points.len(), 4);
        assert!((p.pins[0].yaw - 90.0).abs() < 1.0, "the hotspot is round where the marker was");
        h.get_by_label("Switch Type").click();
        h.run_steps(2);
        assert_eq!(pano(&h).pins[0].kind, PinKind::Marker);
        // A trigger on the selected marker is its own event.
        h.get_by_label("Trigger").click();
        h.run_steps(2);
        let ev = h.state().project.as_ref().unwrap().slides[0].triggers.last().map(|t| t.event.clone());
        let obj = h.state().project.as_ref().unwrap().slides[0].layers[0].objects[at.1].id.clone();
        assert!(matches!(&ev, Some(Event::PanoPin(o, q)) if *o == obj && *q == pano(&h).pins[0].id), "{ev:?}");
        // Title it, take this view as the initial one, and close.
        h.state_mut().project.as_mut().unwrap().slides[0].layers[0].objects[at.1].kind = Kind::Pano({ let mut p = pano(&h); p.pins[0].title = "Fume hood".into(); p });
        h.get_by_label("Initial View").click();
        h.run_steps(2);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.render().expect("renderer").save(std::path::Path::new(&dir).join("pano-edit.png")).unwrap();
        }
        h.get_by_label("Save & Close").click();
        h.run_steps(3);
        assert!(h.state().pano.edit.is_none(), "the mode closes");
        let p = pano(&h);
        assert_eq!((p.yaw, p.pitch, p.fov), (90.0, 10.0, 70.0), "initial view set from the canvas");
        assert_eq!(p.pins[0].title, "Fume hood");
        // Undo reverts the whole visit in one step.
        h.state_mut().undo();
        let p = pano(&h);
        assert!(p.pins.is_empty() && p.yaw == 0.0, "one undo step");
        assert!(h.state().project.as_ref().unwrap().slides[0].triggers.is_empty());
    }

    #[test]
    fn gui_360_image_shows_on_the_canvas_and_in_properties() {
        use egui_kittest::kittest::Queryable;
        let mut a = App::default();
        a.new_project();
        a.project.as_mut().unwrap().assets.insert("pano".into(), Asset { name: "lab.png".into(), mime: "image/png".into(), bytes: test_panorama(), file: None });
        a.add_pano("pano".into(), "lab.png", 512.0, 256.0);
        if let Kind::Pano(p) = &mut a.project.as_mut().unwrap().slides[0].layers[0].objects[0].kind {
            p.pins.push({ let mut m = brainbend_doc::pano::PanoPin::marker("pin1".into(), 20.0, 5.0); m.title = "Door".into(); m });
        }
        let mut h = harness(a);
        h.run_steps(4);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.render().expect("renderer").save(std::path::Path::new(&dir).join("pano-canvas.png")).unwrap();
        }
        let id = format!("obj-{}-", h.state().selected_object().unwrap().id);
        let slots = h.state().pano.views.keys().filter(|k| k.starts_with(&id)).count();
        assert!((1..=2).contains(&slots), "the canvas and the thumbnail each keep one rendered view, got {slots}");
        h.get_by_label("Edit markers and hotspots…").click();
        h.run_steps(3);
        assert!(h.state().pano.edit.is_some(), "Properties opens the editing mode");
    }

    #[test]
    fn gui_equation_window_previews_inserts_and_edits() {
        use egui_kittest::kittest::Queryable;
        let mut a = App::default();
        a.new_project();
        a.open_equation_dialog(None);
        let mut h = harness(a);
        h.run_steps(3);
        // Nothing typed: Insert is not offered as an action yet.
        h.state_mut().eq_dialog.as_mut().unwrap().latex = r"x=\frac{-b\pm\sqrt{b^2-4ac}}{2a}".into();
        h.run_steps(3);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.render().expect("renderer").save(std::path::Path::new(&dir).join("equation-window.png")).unwrap();
        }
        // The ribbon has an Insert tab of the same name; the window's button is the last one.
        h.get_all_by_label("Insert").last().unwrap().click();
        h.run_steps(3);
        assert!(h.state().eq_dialog.is_none(), "the window closes");
        let o = h.state().selected_object().unwrap();
        assert_eq!(o.equation.as_ref().unwrap().latex, r"x=\frac{-b\pm\sqrt{b^2-4ac}}{2a}");
        // Edit it again from the object.
        let at = h.state().selected.unwrap();
        h.state_mut().open_equation_dialog(Some(at));
        h.run_steps(3);
        assert_eq!(h.state().eq_dialog.as_ref().unwrap().latex, r"x=\frac{-b\pm\sqrt{b^2-4ac}}{2a}", "the window opens on the equation's own LaTeX");
        h.state_mut().eq_dialog.as_mut().unwrap().latex = r"\sqrt{2}".into();
        h.run_steps(3);
        h.get_by_label("Update").click();
        h.run_steps(3);
        assert_eq!(h.state().selected_object().unwrap().equation.as_ref().unwrap().latex, r"\sqrt{2}");
    }

    /// Publishes a slide of equations to `equations-site` in BRAINBEND_SHOT_DIR (visual review in a browser only).
    #[test]
    fn publish_equations_for_review() {
        let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") else { return };
        let mut a = App::default();
        a.new_project();
        let samples = [r"x=\frac{-b\pm\sqrt{b^2-4ac}}{2a}", r"\int_0^\infty e^{-x^2}\,dx=\frac{\sqrt{\pi}}{2}", r"\sum_{n=1}^{\infty}\frac{1}{n^2}=\frac{\pi^2}{6}",
            r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}\begin{pmatrix} x \\ y \end{pmatrix}", r"f(x)=\begin{cases} x^2 & x\ge 0 \\ -x & x<0\end{cases}",
            r"\oint_C \vec F\cdot d\vec r = \iint_S (\nabla\times\vec F)\cdot d\vec S", r"\ce{2H2 + O2 -> 2H2O}", r"a \bmod n,\ x \equiv y \pmod{n}",
            r"\nabla \times \vec{B} = \mu_0 \vec{J} + \mu_0 \varepsilon_0 \frac{\partial \vec{E}}{\partial t}", r"\ce{CO2 + H2O <=> H2CO3}"];
        for (i, l) in samples.iter().enumerate() {
            a.insert_equation(l, true).unwrap();
            let q = &mut a.project.as_mut().unwrap().slides[0].layers[0].objects[i];
            q.x = 60.0 + (i % 2) as f32 * 940.0;
            q.y = 40.0 + (i / 2) as f32 * 190.0;
        }
        brainbend_publish_html::export(a.project.as_ref().unwrap(), &std::path::Path::new(&dir).join("equations-site")).unwrap();
    }

    #[test]
    fn grow_and_shrink_step_through_the_common_sizes() {
        assert_eq!(ribbon::step_size(40.0, true), 48.0);
        assert_eq!(ribbon::step_size(40.0, false), 36.0);
        assert_eq!(ribbon::step_size(13.0, true), 14.0);
        assert_eq!(ribbon::step_size(120.0, true), 120.0);
        assert_eq!(ribbon::step_size(8.0, false), 8.0);
    }

    #[test]
    fn gui_deleting_the_last_slide_with_tabs_open_does_not_crash() {
        let mut a = App::default();
        a.new_project();
        a.insert_text();
        a.add_slide(SlideKind::Normal);
        a.insert_text();
        let mut h = harness(a);
        h.run_steps(3);
        // Visit both slides so each has a tab, then delete the last one through the ribbon's path.
        h.state_mut().slide = 0;
        h.run_steps(3);
        h.state_mut().slide = 1;
        h.state_mut().selected = Some((0, 0));
        h.run_steps(3);
        h.state_mut().ribbon_tab = 3;
        h.run_steps(2);
        { let sel = h.state().slide_selection(); h.state_mut().pending_delete = Some(sel); }
        h.run_steps(4);
        assert_eq!(h.state().project.as_ref().unwrap().slides.len(), 1);
        h.state_mut().undo();
        h.run_steps(3);
    }

    #[test]
    fn gui_deleting_trailing_slides_of_a_real_project_in_every_bottom_tab() {
        for bottom in 0..4 {
            let Some(app) = sample() else { return };
            let mut h = harness(app);
            h.run_steps(2);
            let n = h.state().project.as_ref().unwrap().slides.len();
            for k in 0..n.min(4) { h.state_mut().slide = k; h.state_mut().selected = Some((0, 0)); h.run_steps(2); }
            h.state_mut().bottom_tab = bottom;
            while h.state().project.as_ref().unwrap().slides.len() > 1 {
                let last = h.state().project.as_ref().unwrap().slides.len() - 1;
                h.state_mut().slide = last;
                h.state_mut().selected = Some((0, 0));
                h.run_steps(2);
                let sel = h.state().slide_selection();
                h.state_mut().delete_slides(sel);
                h.run_steps(3);
            }
        }
    }

    #[test]
    fn gui_a_question_slide_takes_shapes_buttons_and_layers() {
        let mut a = App::default();
        a.new_project();
        a.add_slide(SlideKind::Question(Question::new_single()));
        a.insert_text();
        a.insert_shape("Rectangle");
        a.insert_button();
        a.add_layer();
        let slide = &a.project.as_ref().unwrap().slides[a.slide];
        assert!(matches!(slide.kind, SlideKind::Question(_)));
        assert_eq!(slide.layers[0].objects.len(), 3, "the question slide holds objects like any slide");
        assert_eq!(slide.layers.len(), 2);
        if let SlideKind::Question(q) = &mut a.project.as_mut().unwrap().slides[1].kind { q.area = Some(QuestionArea { x: 60.0, y: 150.0, w: 1100.0, h: 800.0 }); }
        let mut h = harness(a);
        h.run_steps(4);
        h.state_mut().layer = 0;
        h.state_mut().selected = Some((0, 1));
        h.run_steps(3);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("question-with-objects.png")).unwrap();
        }
        // Dragging the area's grip moves it; one undo step puts it back.
        let area = |h: &egui_kittest::Harness<'_, App>| match &h.state().project.as_ref().unwrap().slides[1].kind { SlideKind::Question(q) => q.area.unwrap(), _ => unreachable!() };
        let before = area(&h);
        let (from, to) = (egui::pos2(362.0, 306.0), egui::pos2(462.0, 356.0));
        h.event(egui::Event::PointerMoved(from));
        h.run_steps(1);
        h.event(egui::Event::PointerButton { pos: from, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() });
        h.run_steps(1);
        h.event(egui::Event::PointerMoved(to));
        h.run_steps(2);
        h.event(egui::Event::PointerButton { pos: to, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() });
        h.run_steps(2);
        let moved = area(&h);
        assert!(moved.x > before.x + 50.0 && moved.y > before.y + 20.0, "{before:?} -> {moved:?}");
        assert_eq!((moved.w, moved.h), (before.w, before.h), "moving keeps the size");
        h.state_mut().undo();
        h.run_steps(2);
        assert_eq!(area(&h).x, before.x);
        // The bottom-right handle resizes it.
        let (from, to) = (egui::pos2(786.0, 669.0), egui::pos2(736.0, 619.0));
        h.event(egui::Event::PointerMoved(from));
        h.run_steps(1);
        h.event(egui::Event::PointerButton { pos: from, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() });
        h.run_steps(1);
        h.event(egui::Event::PointerMoved(to));
        h.run_steps(2);
        h.event(egui::Event::PointerButton { pos: to, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() });
        h.run_steps(2);
        let sized = area(&h);
        assert!(sized.w < before.w - 30.0 && sized.h < before.h - 30.0 && sized.x == before.x, "{before:?} -> {sized:?}");
    }

    #[test]
    fn right_click_arrange_steps_an_object_one_place_and_undoes() {
        let mut a = App::default();
        a.new_project();
        a.insert_text();
        a.insert_rect();
        a.insert_ellipse();
        let names = |a: &App| a.project.as_ref().unwrap().slides[0].layers[0].objects.iter().map(|o| o.name.clone()).collect::<Vec<_>>();
        let before = names(&a);
        a.selected = Some((0, 0));
        a.reorder_step(true);
        assert_eq!(a.selected, Some((0, 1)), "it moved forward by one");
        assert_eq!(names(&a)[1], before[0]);
        a.reorder_step(false);
        a.reorder_step(false); // already at the back: nothing happens
        assert_eq!(names(&a), before);
        a.selected = Some((0, 2));
        a.reorder_step(true); // already in front
        assert_eq!(names(&a), before);
        // Align one object to the slide through the same call the menu makes.
        a.selected = Some((0, 0));
        a.align_selected_or_slide(0, 2);
        let o = &a.project.as_ref().unwrap().slides[0].layers[0].objects[0];
        assert_eq!(o.x + o.w, a.project.as_ref().unwrap().slides[0].width);
    }

    #[test]
    fn gui_new_slide_menu_adds_a_slide_to_the_chosen_scene() {
        let mut a = App::default();
        a.new_project();
        a.add_scene();
        let scenes = a.project.as_ref().unwrap().scenes.len();
        assert_eq!(scenes, 2);
        a.slide = 0;
        let before = a.project.as_ref().unwrap().slides.len();
        // The same steps the "New slide here" menu takes for the second scene.
        let last = *a.project.as_ref().unwrap().scene_slides(1).last().unwrap();
        a.slide = last;
        a.add_slide(SlideKind::Normal);
        let p = a.project.as_ref().unwrap();
        assert_eq!(p.slides.len(), before + 1);
        assert_eq!(p.scene_slides(1).len(), 2, "it joined the second scene");
        let mut h = harness(a);
        h.run_steps(3);
        // Right-click the empty part of the slide list: the menu offers a new scene.
        let at = egui::pos2(130.0, 700.0);
        h.event(egui::Event::PointerMoved(at));
        h.run_steps(1);
        h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Secondary, pressed: true, modifiers: Default::default() });
        h.run_steps(1);
        h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Secondary, pressed: false, modifiers: Default::default() });
        h.run_steps(3);
        { use egui_kittest::kittest::Queryable; h.get_by_label("New scene").click(); }
        h.run_steps(3);
        assert_eq!(h.state().project.as_ref().unwrap().scenes.len(), 3, "the list's right-click menu added a scene");
        h.state_mut().scene_view = true;
        h.run_steps(3);
    }

    #[test]
    fn gui_trigger_rows_keep_their_controls_and_have_a_right_click_menu() {
        use egui_kittest::kittest::Queryable;
        let mut a = App::default();
        a.new_project();
        a.insert_rect();
        let id = a.project.as_ref().unwrap().slides[0].layers[0].objects[0].id.clone();
        a.project.as_mut().unwrap().slides[0].triggers.push(Trigger { id: "t1".into(), owner: Owner::Object(id), event: Event::Click, action: Action::CompleteCourse, conditions: vec![] });
        a.selected = Some((0, 0));
        a.right_tab = 1;
        let mut h = harness(a);
        h.run_steps(4);
        // Right-click the row's "When" label: the menu offers Duplicate.
        let when = h.get_by_label("When").rect().center();
        h.event(egui::Event::PointerMoved(when));
        h.run_steps(1);
        h.event(egui::Event::PointerButton { pos: when, button: egui::PointerButton::Secondary, pressed: true, modifiers: Default::default() });
        h.run_steps(1);
        h.event(egui::Event::PointerButton { pos: when, button: egui::PointerButton::Secondary, pressed: false, modifiers: Default::default() });
        h.run_steps(3);
        assert!(h.query_by_label("Move up").is_some(), "the row menu opened");
        // The Home tab's Duplicate slide button also reads "Duplicate"; the menu entry is the one that opened last.
        h.get_all_by_label("Duplicate").last().unwrap().click();
        h.run_steps(3);
        assert_eq!(h.state().project.as_ref().unwrap().slides[0].triggers.len(), 2, "Duplicate copied the trigger");
        // The overlay that makes the row right-clickable must not swallow clicks on its own controls.
        h.get_all_by_value("clicked").next().unwrap().click();
        h.run_steps(3);
        assert!(h.query_by_label("double-clicked").is_some(), "the event drop-down still opens");
        h.key_press(egui::Key::Escape);
        h.run_steps(2);
    }

    /// Two shapes and a layer on a slide, the side panel left on the Triggers tab on purpose.
    fn on_the_triggers_tab() -> egui_kittest::Harness<'static, App> {
        let mut a = App::default();
        a.new_project();
        a.insert_rect();
        a.insert_ellipse();
        a.add_layer();
        a.layer = 0;
        a.selected = Some((0, 0));
        let mut h = harness(a);
        h.run_steps(4);
        h.state_mut().right_tab = 1;
        h.run_steps(4);
        assert_eq!(h.state().right_tab, 1, "Triggers stays open while nothing is picked");
        h
    }

    #[test]
    fn picking_another_object_shows_its_properties_instead_of_staying_on_triggers() {
        let mut h = on_the_triggers_tab();
        h.state_mut().selected = Some((0, 1));
        h.run_steps(3);
        assert_eq!(h.state().right_tab, 0, "Properties, so the object's settings are not missed");
        // Going back to Triggers is a choice, and it sticks until something else is picked.
        h.state_mut().right_tab = 1;
        h.run_steps(4);
        assert_eq!(h.state().right_tab, 1);
    }

    fn var(id: &str, name: &str, initial: Value) -> Variable { Variable { id: id.into(), name: name.into(), initial, built_in: false } }

    fn sample_variables() -> Vec<Variable> {
        vec![var("v1", "Score", Value::Num(0.0)), var("v2", "Passed", Value::Bool(false)), var("v3", "Learner Name", Value::Text(String::new())), var("v4", "Attempts", Value::Num(0.0))]
    }

    #[test]
    fn variables_are_found_by_name_type_and_whether_a_trigger_uses_them() {
        let vars = sample_variables();
        let used: std::collections::HashSet<String> = ["v1".to_string(), "v2".to_string()].into_iter().collect();
        let names = |q: &str, kind: usize, usage: usize| vars.iter().filter(|v| variable_shown(v, &used, q, kind, usage)).map(|v| v.name.as_str()).collect::<Vec<_>>();
        assert_eq!(names("", 0, 0), ["Score", "Passed", "Learner Name", "Attempts"], "no filter shows everything");
        assert_eq!(names("sc", 0, 0), ["Score"]);
        assert_eq!(names("  NAME ", 0, 0), ["Learner Name"], "case and surrounding spaces do not matter");
        assert_eq!(names("", 2, 0), ["Score", "Attempts"], "numbers");
        assert_eq!(names("", 1, 0), ["Passed"], "true/false");
        assert_eq!(names("", 3, 0), ["Learner Name"], "text");
        assert_eq!(names("", 0, 1), ["Score", "Passed"], "named by a trigger");
        assert_eq!(names("", 0, 2), ["Learner Name", "Attempts"], "named by no trigger");
        assert_eq!(names("s", 2, 1), ["Score"], "filters combine");
        assert!(names("zzz", 0, 0).is_empty());
    }

    #[test]
    fn a_trigger_names_a_variable_in_its_conditions_actions_and_events() {
        let mut p = Project::new("p");
        p.slides.push(Slide::new("s", "S", 1280.0, 720.0));
        p.variables = sample_variables();
        p.slides[0].triggers = vec![
            Trigger { id: "a".into(), owner: Owner::Slide, event: Event::SlideStart, action: Action::SetVariable { variable: "v1".into(), value: Value::Num(1.0) }, conditions: vec![] },
            Trigger { id: "b".into(), owner: Owner::Slide, event: Event::SlideStart, action: Action::NextSlide, conditions: vec![brainbend_doc::Condition { variable: "v2".into(), cmp: brainbend_doc::Cmp::Eq, value: Value::Bool(true), or_with_previous: false }] },
            Trigger { id: "c".into(), owner: Owner::Slide, event: Event::VariableChanged("v3".into()), action: Action::NextSlide, conditions: vec![] },
            Trigger { id: "d".into(), owner: Owner::Slide, event: Event::SlideStart, action: Action::AdjustVariable { variable: "v4".into(), op: brainbend_doc::AdjustOp::Add, value: Value::Num(1.0) }, conditions: vec![] },
        ];
        let mut used: Vec<String> = variables_in_triggers(&p).into_iter().collect();
        used.sort();
        assert_eq!(used, ["v1", "v2", "v3", "v4"]);
        assert!(variables_in_triggers(&Project::new("empty")).is_empty());
    }

    /// A project with variables Score (number), Passed (true/false) and a text box that shows %Score%.
    fn app_with_variables() -> App {
        let mut a = App::default();
        a.new_project();
        a.insert_text();
        let (li, oi) = a.selected.unwrap();
        a.project.as_mut().unwrap().slides[0].layers[li].objects[oi].text = "You scored %Score% points".into();
        a.project.as_mut().unwrap().variables.extend(sample_variables());
        a
    }

    #[test]
    fn renaming_a_variable_updates_the_text_that_shows_it_and_is_one_undo_step() {
        let mut a = app_with_variables();
        a.rename_variable("v1", "Points").unwrap();
        let p = a.project.as_ref().unwrap();
        assert_eq!(p.variables.iter().find(|v| v.id == "v1").unwrap().name, "Points");
        assert_eq!(p.slides[0].layers[0].objects[0].text, "You scored %Points% points");
        assert!(a.var_note.as_ref().is_some_and(|(n, refused)| !refused && n.contains("Renamed Score to Points") && n.contains("1 object")), "{:?}", a.var_note);
        a.undo();
        let p = a.project.as_ref().unwrap();
        assert_eq!(p.variables.iter().find(|v| v.id == "v1").unwrap().name, "Score", "undo brings the name back");
        assert_eq!(p.slides[0].layers[0].objects[0].text, "You scored %Score% points", "and the text, in the same step");
    }

    #[test]
    fn a_name_that_is_taken_or_empty_is_refused_and_leaves_everything_alone() {
        let mut a = app_with_variables();
        let undo_before = a.undo.len();
        assert!(a.rename_variable("v1", "Passed").unwrap_err().contains("already exists"));
        assert!(a.rename_variable("v1", "  ").is_err());
        assert_eq!(a.undo.len(), undo_before, "a refused rename leaves no undo step");
        assert_eq!(a.project.as_ref().unwrap().slides[0].layers[0].objects[0].text, "You scored %Score% points");
    }

    #[test]
    fn changing_a_variables_type_is_one_undo_step_and_refused_when_a_slider_needs_a_number() {
        use brainbend_doc::variables::VarType;
        let mut a = app_with_variables();
        a.convert_variable("v1", VarType::Text).unwrap();
        assert_eq!(a.project.as_ref().unwrap().variables.iter().find(|v| v.id == "v1").unwrap().initial, Value::Text("0".into()));
        assert!(a.var_note.as_ref().is_some_and(|(n, _)| n.contains("\"Score\" is now text")), "{:?}", a.var_note);
        a.undo();
        assert_eq!(a.project.as_ref().unwrap().variables.iter().find(|v| v.id == "v1").unwrap().initial, Value::Num(0.0), "undo brings the number back");

        a.insert(Object::new("sl", "Slider", Kind::Slider(Slider { variable: "v1".into(), ..Slider::new() }), 0.0, 0.0, 200.0, 24.0));
        let undo_before = a.undo.len();
        let err = a.convert_variable("v1", VarType::Bool).unwrap_err();
        assert!(err.contains("Cannot make \"Score\" true/false") && err.contains("1 slider"), "{err}");
        assert_eq!(a.undo.len(), undo_before, "a refused change leaves no undo step");
        assert_eq!(a.project.as_ref().unwrap().variables.iter().find(|v| v.id == "v1").unwrap().initial, Value::Num(0.0));
    }

    const PLACEHOLDER_TEXT: &str = "Score: %Score% now"; // the placeholder is characters 7..14

    /// Editing a text box that shows %Score%, with the cursor placed by the test.
    fn editing_placeholder_text() -> egui_kittest::Harness<'static, App> {
        let mut a = app();
        a.insert_text();
        let (li, oi) = a.selected.unwrap();
        a.project.as_mut().unwrap().slides[0].layers[li].objects[oi].text = PLACEHOLDER_TEXT.into();
        let mut h = harness(a);
        h.run_steps(3);
        h.state_mut().editing = Some((li, oi));
        h.state_mut().editing_old = PLACEHOLDER_TEXT.into();
        h.run_steps(4);
        h
    }

    fn put_cursor(h: &mut egui_kittest::Harness<'static, App>, primary: usize, secondary: usize) {
        let id = egui::Id::new("inplace-edit");
        let mut st = egui::TextEdit::load_state(&h.ctx, id).unwrap_or_default();
        st.cursor.set_char_range(Some(egui::text::CCursorRange { primary: egui::text::CCursor::new(primary), secondary: egui::text::CCursor::new(secondary), h_pos: None }));
        st.store(&h.ctx, id);
        h.run_steps(3);
    }

    fn cursor(h: &egui_kittest::Harness<'static, App>) -> (usize, usize) {
        let r = egui::TextEdit::load_state(&h.ctx, egui::Id::new("inplace-edit")).and_then(|s| s.cursor.char_range()).expect("a cursor");
        (r.primary.index.0, r.secondary.index.0)
    }

    fn box_text(h: &egui_kittest::Harness<'static, App>) -> String {
        let (li, oi) = h.state().selected.unwrap();
        h.state().project.as_ref().unwrap().slides[0].layers[li].objects[oi].text.clone()
    }

    #[test]
    fn the_cursor_will_not_stay_inside_a_variable_in_text() {
        let mut h = editing_placeholder_text();
        put_cursor(&mut h, 14, 14);
        assert_eq!(cursor(&h), (14, 14), "after it is fine");
        put_cursor(&mut h, 10, 10);
        assert_eq!(cursor(&h), (7, 7), "moved left into it, so it goes to the start");
        put_cursor(&mut h, 8, 8);
        assert_eq!(cursor(&h), (14, 14), "moved right into it, so it goes to the end");
    }

    #[test]
    fn a_selection_takes_in_all_of_a_variable_it_touches() {
        let mut h = editing_placeholder_text();
        put_cursor(&mut h, 9, 12);
        assert_eq!(cursor(&h), (7, 14), "a selection inside grows to the whole variable");
        put_cursor(&mut h, 3, 10);
        assert_eq!(cursor(&h), (3, 14));
    }

    #[test]
    fn backspace_and_delete_remove_a_whole_variable() {
        let mut h = editing_placeholder_text();
        put_cursor(&mut h, 14, 14);
        h.key_press(egui::Key::Backspace);
        h.run_steps(3);
        assert_eq!(box_text(&h), "Score:  now", "one Backspace after it removes all of it");

        let mut h = editing_placeholder_text();
        put_cursor(&mut h, 7, 7);
        h.key_press(egui::Key::Delete);
        h.run_steps(3);
        assert_eq!(box_text(&h), "Score:  now", "one Delete before it removes all of it");

        // Away from a variable the keys work as usual.
        let mut h = editing_placeholder_text();
        put_cursor(&mut h, 14, 14);
        h.key_press(egui::Key::Delete);
        h.run_steps(3);
        assert_eq!(box_text(&h), "Score: %Score%now", "Delete after it removes just the space");
        let mut h = editing_placeholder_text();
        put_cursor(&mut h, 6, 6);
        h.key_press(egui::Key::Backspace);
        h.run_steps(3);
        assert_eq!(box_text(&h), "Score %Score% now", "Backspace two characters before it removes just the colon");
    }

    #[test]
    fn a_new_variable_gets_a_name_without_spaces_and_the_add_box_says_why_not() {
        use egui_kittest::kittest::Queryable;
        let mut a = App::default();
        a.new_project();
        a.variables_open = true;
        a.new_var_name = "My Score".into();
        let mut h = harness(a);
        h.run_steps(4);
        assert!(h.query_by_label_contains("No spaces").is_some(), "the reason shows under the box");
        a_add_is_disabled(&h, true);
        h.state_mut().new_var_name = "MyScore".into();
        h.run_steps(3);
        assert!(h.query_by_label_contains("No spaces").is_none());
        a_add_is_disabled(&h, false);
    }

    fn a_add_is_disabled(h: &egui_kittest::Harness<'static, App>, disabled: bool) {
        use egui_kittest::kittest::{NodeT, Queryable};
        assert_eq!(h.get_by_label("Add").accesskit_node().is_disabled(), disabled);
    }

    #[test]
    fn a_duplicated_variable_gets_a_name_without_a_space() {
        let mut a = app_with_variables();
        a.variables_open = true;
        let mut h = harness(a);
        h.run_steps(4);
        // The window's context menu calls the same code; run it the way the menu does.
        let id = "v1".to_string();
        let new = h.state_mut().new_id("var");
        let p = h.state_mut().project.as_mut().unwrap();
        let v = p.variables.iter().find(|v| v.id == id).cloned().unwrap();
        let mut name = format!("{}_copy", v.name);
        while p.variables.iter().any(|x| x.name == name) { name.push_str("_2"); }
        p.variables.push(Variable { id: new, name: name.clone(), initial: v.initial, built_in: false });
        assert_eq!(name, "Score_copy");
        assert!(brainbend_doc::variables::check_name(&name).is_ok());
    }

    #[test]
    fn the_rename_box_commits_on_enter_and_cancels_on_escape() {
        use egui_kittest::kittest::Queryable;
        let mut a = app_with_variables();
        a.variables_open = true;
        let mut h = harness(a);
        h.run_steps(4);
        // Escape puts the old name back.
        h.state_mut().var_rename = Some(("v1".into(), "Scored".into(), true));
        h.run_steps(3);
        assert!(h.query_by_label("Score =").is_none(), "the name is a box while it is renamed");
        h.key_press(egui::Key::Escape);
        h.run_steps(3);
        assert!(h.state().var_rename.is_none());
        assert!(h.query_by_label("Score =").is_some(), "Escape keeps the old name");
        // Enter keeps the new one.
        h.state_mut().var_rename = Some(("v1".into(), "Points".into(), true));
        h.run_steps(3);
        h.key_press(egui::Key::Enter);
        h.run_steps(4);
        assert!(h.state().var_rename.is_none(), "{:?}", h.state().var_rename);
        assert!(h.query_by_label("Points =").is_some(), "the new name is listed");
        assert_eq!(h.state().project.as_ref().unwrap().slides[0].layers[0].objects[0].text, "You scored %Points% points");
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") { h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("variables-3-renamed.png")).unwrap(); }
    }

    #[test]
    fn a_refused_type_change_reads_clearly_in_the_window() {
        use brainbend_doc::variables::VarType;
        let mut a = app_with_variables();
        a.insert(Object::new("sl", "Slider", Kind::Slider(Slider { variable: "v1".into(), ..Slider::new() }), 0.0, 0.0, 200.0, 24.0));
        a.variables_open = true;
        let err = a.convert_variable("v1", VarType::Text).unwrap_err();
        a.var_note = Some((err, true));
        let mut h = harness(a);
        h.run_steps(4);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") { h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("variables-4-refused.png")).unwrap(); }
    }

    #[test]
    fn the_variables_window_narrows_its_list_as_you_search_and_filter() {
        use egui_kittest::kittest::Queryable;
        let mut a = App::default();
        a.new_project();
        a.project.as_mut().unwrap().variables.extend(sample_variables());
        a.variables_open = true;
        let mut h = harness(a);
        h.run_steps(4);
        for name in ["Score", "Passed", "Learner Name", "Attempts"] { assert!(h.query_by_label(&format!("{name} =")).is_some(), "{name} is listed"); }
        assert!(h.query_by_label_contains("Showing").is_none(), "no count while nothing is filtered");
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") { h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("variables-1-all.png")).unwrap(); }

        h.state_mut().var_query = "a".into();
        h.run_steps(3);
        assert!(h.query_by_label("Score =").is_none(), "Score has no 'a'");
        assert!(h.query_by_label("Attempts =").is_some() && h.query_by_label("Learner Name =").is_some() && h.query_by_label("Passed =").is_some());
        assert!(h.query_by_label("Showing 3 of 4 variables").is_some());

        h.state_mut().var_kind = 2;
        h.run_steps(3);
        assert!(h.query_by_label("Attempts =").is_some() && h.query_by_label("Passed =").is_none(), "number variables with an 'a': Attempts");
        assert!(h.query_by_label("Showing 1 of 4 variables").is_some());
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") { h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("variables-2-filtered.png")).unwrap(); }

        h.state_mut().var_query = "zzz".into();
        h.run_steps(3);
        assert!(h.query_by_label("No variables match.").is_some());
        h.get_by_label("Clear").click();
        h.run_steps(3);
        assert!((h.state().var_query.is_empty(), h.state().var_kind, h.state().var_use) == (true, 0, 0), "Clear resets the search and both filters");
        assert!(h.query_by_label("Score =").is_some(), "everything is back");
    }

    #[test]
    fn switching_slides_shows_the_new_slides_properties() {
        let mut h = on_the_triggers_tab();
        h.state_mut().add_slide(SlideKind::Normal);
        h.run_steps(3);
        assert_eq!(h.state().slide, 1, "the new slide is showing");
        assert_eq!(h.state().right_tab, 0, "a new slide shows its own Properties");
        h.state_mut().right_tab = 1;
        h.run_steps(3);
        assert_eq!(h.state().right_tab, 1, "Triggers stays when it is chosen");
        h.state_mut().slide = 0;
        h.run_steps(3);
        assert_eq!(h.state().right_tab, 0, "going back to the first slide too, even with nothing selected on either");
    }

    #[test]
    fn picking_a_layer_or_clearing_the_selection_also_shows_properties() {
        let mut h = on_the_triggers_tab();
        h.state_mut().selected = None;
        h.state_mut().layer = 1;
        h.run_steps(3);
        assert_eq!(h.state().right_tab, 0, "a layer was picked");
        h.state_mut().right_tab = 1;
        h.run_steps(3);
        h.state_mut().layer = 0;
        h.run_steps(3);
        assert_eq!(h.state().right_tab, 0, "back to the base layer");
    }

    #[test]
    fn a_group_heading_in_the_triggers_tab_picks_its_owner_without_leaving_the_tab() {
        use egui_kittest::kittest::Queryable;
        let mut h = on_the_triggers_tab();
        let ids: Vec<String> = h.state().project.as_ref().unwrap().slides[0].layers[0].objects.iter().map(|o| o.id.clone()).collect();
        for id in &ids {
            h.state_mut().project.as_mut().unwrap().slides[0].triggers.push(Trigger { id: format!("t-{id}"), owner: Owner::Object(id.clone()), event: Event::Click, action: Action::NextSlide, conditions: vec![] });
        }
        h.state_mut().trig_all = true;
        h.run_steps(4);
        let name1 = h.state().project.as_ref().unwrap().slides[0].layers[0].objects[1].name.clone();
        h.get_by_label(&format!("{name1}  (1)")).click();
        h.run_steps(4);
        assert_eq!(h.state().selected, Some((0, 1)), "the heading picked its object");
        assert_eq!(h.state().right_tab, 1, "and the panel stayed on Triggers, where the click was made");
    }

    #[test]
    fn gui_triggers_tab_can_list_every_trigger_on_the_slide() {
        use egui_kittest::kittest::Queryable;
        let mut a = App::default();
        a.new_project();
        a.insert_rect();
        a.insert_ellipse();
        let ids: Vec<String> = a.project.as_ref().unwrap().slides[0].layers[0].objects.iter().map(|o| o.id.clone()).collect();
        let slide = &mut a.project.as_mut().unwrap().slides[0];
        slide.triggers.push(Trigger { id: "s1".into(), owner: Owner::Slide, event: Event::SlideStart, action: Action::CompleteCourse, conditions: vec![] });
        slide.triggers.push(Trigger { id: "o1".into(), owner: Owner::Object(ids[0].clone()), event: Event::Click, action: Action::CompleteCourse, conditions: vec![] });
        slide.triggers.push(Trigger { id: "o2".into(), owner: Owner::Object(ids[1].clone()), event: Event::Hover, action: Action::ExitCourse, conditions: vec![] });
        slide.triggers.push(Trigger { id: "o3".into(), owner: Owner::Object(ids[1].clone()), event: Event::Click, action: Action::RestartCourse, conditions: vec![] });
        a.selected = None;
        a.right_tab = 1;
        a.trig_all = true;
        let name0 = a.project.as_ref().unwrap().slides[0].layers[0].objects[0].name.clone();
        let name1 = a.project.as_ref().unwrap().slides[0].layers[0].objects[1].name.clone();
        let mut h = harness(a);
        h.run_steps(4);
        assert!(h.query_by_label("Slide  (1)").is_some(), "the slide's group");
        assert!(h.query_by_label(&format!("{name0}  (1)")).is_some(), "the first object's group");
        assert!(h.query_by_label(&format!("{name1}  (2)")).is_some(), "the second object's group");
        // A group's heading selects its object.
        h.get_by_label(&format!("{name1}  (2)")).click();
        h.run_steps(3);
        assert_eq!(h.state().selected, Some((0, 1)));
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("triggers-all.png")).unwrap();
        }
    }

    #[test]
    fn gui_variables_live_in_their_own_window() {
        use egui_kittest::kittest::Queryable;
        let mut a = App::default();
        a.new_project();
        let mut h = harness(a);
        h.run_steps(3);
        assert!(!h.state().variables_open);
        assert!(h.query_by_label("Variables").is_none() || h.state().right_tab == 0, "no Variables tab in the side panel");
        // "+ New variable…" in a trigger's variable list asks for the window through the context.
        h.ctx.data_mut(|d| d.insert_temp(egui::Id::new("open-variables"), true));
        h.run_steps(3);
        assert!(h.state().variables_open, "the window opened");
        h.state_mut().new_var_name = "score".into();
        h.run_steps(2);
        h.get_by_label("Add").click();
        h.run_steps(3);
        assert!(h.state().project.as_ref().unwrap().variables.iter().any(|v| v.name == "score"), "a variable was added from the window");
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("variables-window.png")).unwrap();
        }
    }

    #[test]
    fn gui_find_and_replace_changes_text_and_undoes() {
        let mut a = App::default();
        a.new_project();
        a.insert_text();
        a.project.as_mut().unwrap().slides[0].layers[0].objects[0].text = "Hello world, hello again".into();
        let mut h = harness(a);
        h.run_steps(2);
        h.state_mut().find = Some(FindState { query: "hello".into(), with: "goodbye".into(), ..Default::default() });
        h.run_steps(3);
        { use egui_kittest::kittest::Queryable; h.get_by_label("Replace all").click(); }
        h.run_steps(3);
        assert_eq!(h.state().project.as_ref().unwrap().slides[0].layers[0].objects[0].text, "goodbye world, goodbye again");
        assert_eq!(h.state().find.as_ref().unwrap().message, "Replaced 2.");
        h.state_mut().undo();
        h.run_steps(2);
        assert_eq!(h.state().project.as_ref().unwrap().slides[0].layers[0].objects[0].text, "Hello world, hello again");
    }

    #[test]
    fn gui_spelling_suggests_changes_and_keeps_custom_words() {
        use egui_kittest::kittest::Queryable;
        let mut a = App::default();
        a.new_project();
        a.insert_text();
        a.project.as_mut().unwrap().slides[0].layers[0].objects[0].text = "Safty first at Brainbend".into();
        let mut h = harness(a);
        h.run_steps(2);
        h.state_mut().open_spelling();
        h.run_steps(3);
        h.get_by_label("Safety").click();
        h.run_steps(3);
        assert_eq!(h.state().project.as_ref().unwrap().slides[0].layers[0].objects[0].text, "Safety first at Brainbend");
        h.get_by_label("Add to dictionary").click();
        h.run_steps(3);
        assert_eq!(h.state().project.as_ref().unwrap().custom_words, vec!["brainbend".to_string()]);
        assert!(brainbend_spell::check_project(h.state().project.as_ref().unwrap(), &Default::default()).is_empty());
    }

    fn lesson_app(kind: ProjectKind) -> egui_kittest::Harness<'static, App> {
        let mut a = App::default();
        a.new_lesson_project(kind);
        let mut h = harness(a);
        h.run_steps(2);
        h
    }

    #[test]
    fn gui_web_lesson_every_block_kind_renders_edits_and_undoes() {
        let mut h = lesson_app(ProjectKind::WebLesson);
        assert!(h.state().project.as_ref().unwrap().is_web());
        // Add one of every block, select each in turn and draw the frames (no panics).
        let kinds: Vec<brainbend_doc::lesson::BlockKind> = brainbend_doc::lesson::BlockKind::catalog().into_iter().flat_map(|(_, v)| v).collect();
        let n = kinds.len();
        for k in kinds { h.state_mut().add_block(k); h.run_steps(2); }
        assert_eq!(h.state().project.as_ref().unwrap().lesson.as_ref().unwrap().sections[0].blocks.len(), 2 + n);
        for b in 0..2 + n { h.state_mut().blk = Some(b); h.run_steps(2); }
        h.state_mut().lesson_settings = true;
        h.run_steps(3);
        h.state_mut().lesson_settings = false;
        h.state_mut().check_results = h.state().project.as_ref().map(|p| p.check());
        h.run_steps(2);
        // One undo step removes the last block added.
        h.state_mut().undo();
        h.run_steps(2);
        assert_eq!(h.state().project.as_ref().unwrap().lesson.as_ref().unwrap().sections[0].blocks.len(), 1 + n);
        // Sections: add, rename through the model, delete.
        h.state_mut().add_section();
        h.run_steps(2);
        assert_eq!(h.state().project.as_ref().unwrap().lesson.as_ref().unwrap().sections.len(), 2);
        h.state_mut().delete_section();
        h.run_steps(2);
        assert_eq!(h.state().project.as_ref().unwrap().lesson.as_ref().unwrap().sections.len(), 1);
        // It publishes.
        let files = brainbend_publish_html::build(h.state().project.as_ref().unwrap(), brainbend_publish_html::Lms::Scorm12).unwrap();
        assert!(files.iter().any(|f| f.0 == "index.html") && files.iter().any(|f| f.0 == "imsmanifest.xml"));
    }

    #[test]
    fn gui_web_lesson_picture_block_crops_to_a_shape_and_keeps_a_chosen_part() {
        use brainbend_doc::lesson::BlockKind;
        use egui_kittest::kittest::Queryable;
        let mut h = lesson_app(ProjectKind::WebLesson);
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_fn(400, 200, |x, _| if x < 200 { image::Rgba([200, 30, 30, 255]) } else { image::Rgba([30, 30, 200, 255]) }))
            .write_to(&mut png, image::ImageFormat::Png).unwrap();
        h.state_mut().project.as_mut().unwrap().assets.insert("pic".into(), Asset { name: "wide.png".into(), mime: "image/png".into(), bytes: png.into_inner(), file: None });
        h.state_mut().add_block(BlockKind::Image { asset: Some("pic".into()), alt: "A wide picture".into(), caption: String::new(), width: brainbend_doc::lesson::ImageWidth::Full, decorative: false, crop: None });
        h.run_steps(2);
        let last = h.state().project.as_ref().unwrap().lesson.as_ref().unwrap().sections[0].blocks.len() - 1;
        h.state_mut().blk = Some(last);
        h.run_steps(3);
        let crop_of = |h: &egui_kittest::Harness<'static, App>| match &h.state().project.as_ref().unwrap().lesson.as_ref().unwrap().sections[0].blocks[last].kind { BlockKind::Image { crop, .. } => *crop, _ => panic!("an image block") };
        assert_eq!(crop_of(&h), None, "the whole picture to start with");
        let whole = h.render().expect("renderer");
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") { whole.save(std::path::PathBuf::from(dir).join("9x-web-whole.png")).unwrap(); }
        // Choose a square from the block's editor.
        h.get_by_role(egui::accesskit::Role::ComboBox).click();
        h.run_steps(3);
        h.get_by_label_contains("1:1").click();
        h.run_steps(4);
        let c = crop_of(&h).expect("a crop");
        assert!((c[0] - 0.25).abs() < 1e-4 && (c[2] - 0.25).abs() < 1e-4 && c[1] == 0.0 && c[3] == 0.0, "the middle half stays: {c:?}");
        let square = h.render().expect("renderer");
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") { square.save(std::path::PathBuf::from(dir).join("9x-web-crop.png")).unwrap(); }
        let red = |img: &image::RgbaImage| img.pixels().filter(|p| p[0] > 180 && p[1] < 60 && p[2] < 60).count();
        let blue = |img: &image::RgbaImage| img.pixels().filter(|p| p[2] > 180 && p[0] < 60 && p[1] < 60).count();
        assert!(red(&whole) > 1000 && blue(&whole) > 1000, "both halves show before cropping");
        // Keeping from the far left leaves only the red half; the preview follows.
        h.state_mut().project.as_mut().unwrap().lesson.as_mut().unwrap().sections[0].blocks[last].kind = BlockKind::Image { asset: Some("pic".into()), alt: "A wide picture".into(), caption: String::new(), width: brainbend_doc::lesson::ImageWidth::Full, decorative: false, crop: crop_for_ratio(400.0, 200.0, 1.0, 0.0, 0.5) };
        h.run_steps(3);
        let left = h.render().expect("renderer");
        assert!(blue(&left) < 100 && red(&left) > 1000, "only the red half is left: {} red, {} blue", red(&left), blue(&left));
        // It publishes with the crop in the lesson data.
        let files = brainbend_publish_html::build(h.state().project.as_ref().unwrap(), brainbend_publish_html::Lms::None).unwrap();
        let html = String::from_utf8(files.iter().find(|f| f.0 == "index.html").unwrap().1.clone()).unwrap();
        assert!(html.contains("\"crop\":[0.0,0.0,0.5,0.0]"), "the crop is published");
    }

    #[test]
    fn gui_web_lesson_find_replace_and_spelling_cover_blocks() {
        use egui_kittest::kittest::Queryable;
        let mut h = lesson_app(ProjectKind::WebLesson);
        h.state_mut().project.as_mut().unwrap().lesson.as_mut().unwrap().sections[0].blocks[1].kind = brainbend_doc::lesson::BlockKind::Text { text: "Safty first. Safty always.".into() };
        h.run_steps(2);
        h.state_mut().open_spelling();
        h.run_steps(3);
        h.get_by_label("Safety").click();
        h.run_steps(3);
        let text = |h: &egui_kittest::Harness<'static, App>| h.state().project.as_ref().unwrap().lesson.as_ref().unwrap().sections[0].blocks[1].kind.texts()[0].clone();
        assert_eq!(text(&h), "Safety first. Safety always.", "a suggestion changes the word wherever it appears in that block");
        h.state_mut().spell = None;
        h.state_mut().find = Some(FindState { query: "first".into(), with: "second".into(), opts: brainbend_doc::find::FindOpts { match_case: false, whole_word: true }, ..Default::default() });
        h.run_steps(3);
        h.get_by_label("Replace all").click();
        h.run_steps(3);
        assert_eq!(text(&h), "Safety second. Safety always.");
    }

    #[test]
    fn gui_web_lesson_screenshots() {
        let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") else { return };
        let dir = std::path::PathBuf::from(dir);
        let mut h = lesson_app(ProjectKind::WebLesson);
        let mut q = Question::new_single();
        q.prompt = "Which is safe?".into();
        for k in [
            brainbend_doc::lesson::BlockKind::Statement { text: "Safety first.".into() },
            brainbend_doc::lesson::BlockKind::Items { style: brainbend_doc::lesson::ItemsStyle::Process, items: vec![brainbend_doc::lesson::Item { title: "Plan".into(), body: "Make a plan".into() }, brainbend_doc::lesson::Item { title: "Do".into(), body: "Do the work".into() }] },
            brainbend_doc::lesson::BlockKind::KnowledgeCheck(q),
        ] { h.state_mut().add_block(k); }
        h.run_steps(3);
        h.render().expect("renderer").save(dir.join("6a-lesson.png")).unwrap();
        h.state_mut().blk = Some(1);
        h.run_steps(3);
        h.render().expect("renderer").save(dir.join("6b-lesson-text-selected.png")).unwrap();
        h.state_mut().blk = Some(4);
        h.state_mut().lesson_settings = true;
        h.run_steps(3);
        h.render().expect("renderer").save(dir.join("6c-lesson-settings.png")).unwrap();
    }

    #[test]
    fn gui_new_project_authoring_flow_renders() {
        let mut a = App::default();
        a.new_project();
        let mut h = harness(a);
        h.run_steps(2);
        h.state_mut().check_results = h.state().project.as_ref().map(|p| p.check());
        h.run_steps(2);
        h.state_mut().bottom_tab = 0;
        h.state_mut().insert_text();
        h.state_mut().insert_rect();
        h.state_mut().project.as_mut().unwrap().slides[0].layers[0].objects[0].timing = Some(Timing { start_ms: 500, dur_ms: Some(1500) });
        h.state_mut().project.as_mut().unwrap().slides[0].layers[0].duration_ms = Some(4000);
        h.run_steps(3);
        h.state_mut().add_slide(SlideKind::Results);
        h.run_steps(2); // a slide kind with no timeline
        h.state_mut().slide = 0;
        h.state_mut().bottom_tab = 0;
        h.state_mut().insert_rect();
        h.state_mut().insert_button();
        h.state_mut().copy_selected();
        h.state_mut().paste();
        h.run_steps(3);
        h.state_mut().right_tab = 1;
        h.state_mut().bottom_tab = 1;
        h.run_steps(3);
        h.state_mut().right_tab = 0;
        h.state_mut().bottom_tab = 0;
        h.run_steps(2);
        h.state_mut().zoom = 3.0;
        h.state_mut().pan = egui::vec2(200.0, -100.0);
        h.run_steps(2);
        h.state_mut().reorder_selected(false);
        h.state_mut().duplicate_selected();
        h.run_steps(2);
        h.state_mut().delete_selected();
        h.run_steps(2);
    }

    fn app() -> App {
        let mut a = App::default();
        a.new_project();
        a
    }

    #[test]
    fn editing_while_a_state_is_viewed_changes_that_state_and_undo_puts_it_back() {
        let mut a = app();
        a.insert_button();
        let (li, oi) = a.selected.unwrap();
        let hover_before = a.project.as_ref().unwrap().slides[0].layers[li].objects[oi].states.iter().find(|s| s.name == "Hover").unwrap().fill;
        let own_before = a.project.as_ref().unwrap().slides[0].layers[li].objects[oi].fill;
        a.bottom_tab = 2;
        let id = a.project.as_ref().unwrap().slides[0].layers[li].objects[oi].id.clone();
        a.state_view = Some((id, "Hover".into()));
        // One frame: the state goes on the object, an edit (what the ribbon does) snapshots then recolours, the frame ends.
        a.begin_state_swap();
        assert!(a.swap.is_some());
        a.snapshot();
        a.project.as_mut().unwrap().slides[0].layers[li].objects[oi].fill = Some(Rgba(1, 2, 3, 255));
        a.end_state_swap();
        let o = &a.project.as_ref().unwrap().slides[0].layers[li].objects[oi];
        assert_eq!(o.fill, own_before, "the object keeps its own look");
        assert_eq!(o.states.iter().find(|s| s.name == "Hover").unwrap().fill, Some(Rgba(1, 2, 3, 255)));
        a.undo();
        let o = &a.project.as_ref().unwrap().slides[0].layers[li].objects[oi];
        assert_eq!(o.fill, own_before, "undo did not leave the state's look on the object");
        assert_eq!(o.states.iter().find(|s| s.name == "Hover").unwrap().fill, hover_before, "and Hover is as it was");
    }

    #[test]
    fn player_settings_resources_and_glossary_are_covered_by_undo() {
        let mut a = app();
        let before = Snap::of(a.project.as_ref().unwrap());
        {
            let p = a.project.as_mut().unwrap();
            p.player.sidebar = SidebarSide::Left;
            p.resources.push(Resource { title: "Handbook".into(), url: "https://example.com".into(), asset: None });
            p.glossary.push(GlossaryTerm { term: "PPE".into(), definition: "Gear".into() });
        }
        a.undo.push(before);
        a.undo();
        let p = a.project.as_ref().unwrap();
        assert_eq!((p.player.sidebar, p.resources.len(), p.glossary.len()), (SidebarSide::Off, 0, 0));
    }

    #[test]
    fn slide_tabs_open_remember_their_view_and_close_with_their_slide() {
        let mut a = app();
        a.sync_tabs();
        assert_eq!((a.tabs.len(), a.active_tab), (1, Some(0)), "the slide being edited has a tab");
        a.add_layer();
        a.layer = 1;
        a.save_active_tab();
        a.add_slide(SlideKind::Normal);
        a.sync_tabs();
        assert_eq!((a.tabs.len(), a.active_tab), (2, Some(1)), "a second slide opens a second tab");
        a.save_active_tab();
        // Back to the first slide: its layer comes back.
        a.slide = 0;
        a.layer = 0;
        a.sync_tabs();
        assert_eq!((a.active_tab, a.layer), (Some(0), 1), "the first tab remembers its layer");
        // The Story View is no tab of a slide.
        a.scene_view = true;
        a.sync_tabs();
        assert_eq!((a.active_tab, a.tabs.len()), (None, 2));
        // Closing the active tab goes to its neighbour; closing the last goes to the Story View.
        a.scene_view = false;
        a.slide = 1;
        a.sync_tabs();
        a.close_tab(1);
        a.sync_tabs();
        assert_eq!((a.tabs.len(), a.slide, a.scene_view), (1, 0, false), "closing the second tab lands on the first slide");
        a.close_tab(0);
        assert!(a.scene_view && a.tabs.is_empty(), "closing the last tab shows the Story View");
        // A deleted slide takes its tab with it.
        a.scene_view = false;
        a.slide = 1;
        a.sync_tabs();
        a.slide = 0;
        a.sync_tabs();
        assert_eq!(a.tabs.len(), 2);
        a.delete_slide(1);
        a.sync_tabs();
        assert_eq!(a.tabs.len(), 1, "the deleted slide's tab is gone");
    }

    #[test]
    fn a_selected_layer_shows_and_only_an_eye_keeps_it_showing_after() {
        let mut a = app();
        a.add_layer();
        a.add_layer();
        a.visible = vec![true, false, false];
        a.layer = 1;
        assert_eq!(a.shown_layers(3), vec![0, 1], "selecting a hidden layer shows it");
        a.layer = 2;
        assert_eq!(a.shown_layers(3), vec![0, 2], "moving to another layer hides the first again");
        a.visible[1] = true;
        assert_eq!(a.shown_layers(3), vec![0, 1, 2], "a layer whose eye is on stays");
        a.layer = 0;
        assert_eq!(a.shown_layers(3), vec![0, 1], "an unselected layer without an eye goes away");
        a.layer = 1;
        a.hide_selected_layer();
        assert_eq!((a.layer, a.visible[1], a.shown_layers(3)), (0, false, vec![0]), "the eye on the selected layer hides it and selects the base layer");
    }

    fn trig(id: &str, owner: Owner, action: Action) -> Trigger {
        Trigger { id: id.into(), owner, event: Event::Click, action, conditions: vec![] }
    }

    #[test]
    fn slides_can_be_added_moved_and_deleted() {
        let mut a = app();
        a.add_slide(SlideKind::Question(Question::new_single()));
        a.add_slide(SlideKind::Results);
        let kinds = |a: &App| a.project.as_ref().unwrap().slides.iter().map(|s| matches!(s.kind, SlideKind::Normal) as u8).collect::<Vec<_>>();
        assert_eq!(kinds(&a), vec![1, 0, 0]);
        assert_eq!(a.slide, 2);
        a.move_slide(2, false);
        assert_eq!(a.slide, 1);
        a.delete_slide(1);
        assert_eq!(a.project.as_ref().unwrap().slides.len(), 2);
        a.delete_slide(0);
        a.delete_slide(0); // refuses to delete the last slide
        assert_eq!(a.project.as_ref().unwrap().slides.len(), 1);
    }

    #[test]
    fn deleting_a_slide_drops_jumps_to_it() {
        let mut a = app();
        a.add_slide(SlideKind::Normal);
        let target = a.project.as_ref().unwrap().slides[1].id.clone();
        a.project.as_mut().unwrap().slides[0].triggers.push(trig("t", Owner::Slide, Action::JumpToSlide(target)));
        a.delete_slide(1);
        assert!(a.project.as_ref().unwrap().slides[0].triggers.is_empty());
    }

    #[test]
    fn duplicated_slide_has_fresh_ids_and_repointed_triggers() {
        let mut a = app();
        a.insert_button();
        a.add_layer();
        let slide = &mut a.project.as_mut().unwrap().slides[0];
        let (btn, layer) = (slide.layers[0].objects[0].id.clone(), slide.layers[1].id.clone());
        slide.triggers.push(trig("t", Owner::Object(btn.clone()), Action::ShowLayer(layer.clone())));
        a.duplicate_slide(0);
        let p = a.project.as_ref().unwrap();
        assert_eq!(p.slides.len(), 2);
        let copy = &p.slides[1];
        let (nbtn, nlayer) = (copy.layers[0].objects[0].id.clone(), copy.layers[1].id.clone());
        assert_ne!(nbtn, btn);
        assert_ne!(nlayer, layer);
        match (&copy.triggers[0].owner, &copy.triggers[0].action) {
            (Owner::Object(o), Action::ShowLayer(l)) => assert_eq!((o, l), (&nbtn, &nlayer)),
            other => panic!("unexpected {other:?}"),
        }
        // The original is untouched.
        assert!(matches!(&p.slides[0].triggers[0].action, Action::ShowLayer(l) if *l == layer));
    }

    #[test]
    fn deleting_a_layer_removes_its_triggers_and_references() {
        let mut a = app();
        a.insert_button();
        a.add_layer();
        let slide = &mut a.project.as_mut().unwrap().slides[0];
        let (btn, layer) = (slide.layers[0].objects[0].id.clone(), slide.layers[1].id.clone());
        slide.triggers.push(trig("show", Owner::Object(btn), Action::ShowLayer(layer.clone())));
        slide.triggers.push(trig("own", Owner::Layer(layer), Action::NextSlide));
        slide.triggers.push(trig("keep", Owner::Slide, Action::NextSlide));
        a.delete_layer(1);
        let ids: Vec<_> = a.project.as_ref().unwrap().slides[0].triggers.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["keep"]);
    }

    #[test]
    fn a_button_style_sets_every_state_and_keeps_the_text() {
        let mut a = app();
        a.insert_button();
        let (li, oi) = a.selected.unwrap();
        { let o = &mut a.project.as_mut().unwrap().slides[0].layers[li].objects[oi]; o.text = "Start".into(); for s in o.states.iter_mut() { s.text = "Start".into(); } }
        let orange = button_styles::STYLES.iter().position(|d| d.name == "Orange").unwrap();
        a.apply_button_style(orange);
        let o = a.selected_object().unwrap().clone();
        let def = &button_styles::STYLES[orange];
        assert_eq!(o.fill, Some(def.fill), "the object wears the Normal fill");
        assert_eq!(o.radius, def.radius);
        assert!(o.states.iter().all(|s| s.text == "Start"), "each state keeps its text");
        let hover = o.states.iter().find(|s| s.name == "Hover").unwrap();
        assert_ne!(hover.fill, o.states[0].fill, "hover differs from rest");
        assert!(hover.shine.is_some() && o.states[0].shine.is_some(), "a glossy style has shine");
        let outline = button_styles::STYLES.iter().position(|d| d.name == "Outline").unwrap();
        a.apply_button_style(outline);
        let o = a.selected_object().unwrap().clone();
        assert!(o.stroke.is_some() && o.states.iter().all(|s| s.shine.is_none()), "a flat outline style has no shine");
        assert_eq!(o.style.color, button_styles::STYLES[outline].text);
        a.undo();
        a.selected = Some((li, oi));
        assert_eq!(a.selected_object().unwrap().fill, Some(def.fill), "one undo step goes back to the previous style");
    }

    #[test]
    fn gui_button_styles_gallery_shows_for_a_button_and_applies() {
        let mut a = app();
        a.insert_button();
        let mut h = harness(a);
        h.run_steps(4);
        assert_eq!(h.state().format_title(), Some("BUTTON"), "a selected button has its own tab");
        h.state_mut().ribbon_tab = ribbon::HOME_TAB;
        h.run_steps(3);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("button-styles.png")).unwrap();
        }
        // Click the Teal swatch (second in the first row) on Home: found by its colour, so the ribbon can be rearranged.
        let teal = button_styles::STYLES.iter().find(|d| d.name == "Teal").unwrap().fill;
        let img = h.render().expect("renderer");
        let (mut sx, mut sy, mut n) = (0.0f32, 0.0f32, 0.0f32);
        for y in 60..160u32 {
            for x in 0..1200u32 {
                let p = img.get_pixel(x, y);
                if (p[0] as i32 - teal.0 as i32).abs() < 3 && (p[1] as i32 - teal.1 as i32).abs() < 3 && (p[2] as i32 - teal.2 as i32).abs() < 3 { sx += x as f32; sy += y as f32; n += 1.0; }
            }
        }
        assert!(n > 100.0, "the Teal swatch is on screen");
        let at = egui::pos2(sx / n, sy / n);
        h.event(egui::Event::PointerMoved(at));
        h.run_steps(1);
        h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() });
        h.run_steps(1);
        h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() });
        h.run_steps(3);
        let teal = button_styles::STYLES.iter().find(|d| d.name == "Teal").unwrap().fill;
        assert_eq!(h.state().project.as_ref().unwrap().slides[0].layers[0].objects[0].fill, Some(teal), "the swatch restyled the button");
    }

    #[test]
    fn gui_a_heading_shows_its_level_on_the_text_styles_button() {
        let mut a = app();
        a.insert_text();
        a.apply_text_style(TextRole::H2);
        let mut h = harness(a);
        h.run_steps(4);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("heading-badge.png")).unwrap();
        }
        { use egui_kittest::kittest::Queryable; h.get_by_label("Heading 2"); }
    }

    #[test]
    fn gui_the_tab_line_glides_between_tabs() {
        let mut a = app();
        a.ribbon_tab = ribbon::HOME_TAB;
        let mut h = harness(a);
        h.run_steps(8);
        // The blue pixels of the line, as the first and last column holding any, in the tab strip's band.
        let span = |h: &mut egui_kittest::Harness<'static, App>| {
            let img = h.render().expect("renderer");
            let (mut lo, mut hi) = (u32::MAX, 0);
            for y in 62..72 {
                for x in 0..600 {
                    let p = img.get_pixel(x, y);
                    if p[2] > 140 && p[0] < 40 && p[1] < 120 { lo = lo.min(x); hi = hi.max(x); }
                }
            }
            (lo, hi)
        };
        let home = span(&mut h);
        h.state_mut().ribbon_tab = 3;
        h.step();
        let mid = span(&mut h);
        h.run_steps(30);
        let slides = span(&mut h);
        assert!(home.1 < slides.0, "the line moved right: {home:?} -> {slides:?}");
        assert!(mid.0 < slides.0 || mid.1 < slides.1, "one frame in, the line is still on its way: {mid:?} vs {slides:?}");
    }

    #[test]
    fn gui_copy_cut_and_paste_events_work_on_the_slide() {
        let mut a = app();
        a.insert_rect();
        let count = |h: &egui_kittest::Harness<'static, App>| h.state().project.as_ref().unwrap().slides[0].layers[0].objects.len();
        let mut h = harness(a);
        h.run_steps(3);
        let n = count(&h);
        // What a desktop window sends for ⌘C, ⌘V and ⌘X: events, not key presses. The Paste event carries whatever text
        // the system clipboard holds, which after a copy is the text the copy left there.
        h.event(egui::Event::Copy);
        h.run_steps(3);
        assert!(h.state().clipboard.is_some(), "copy filled the clipboard");
        let left = h.state().clip_text.clone();
        assert!(!left.is_empty(), "and left text on the system clipboard so a paste event will come");
        h.event(egui::Event::Paste(left.clone()));
        h.run_steps(2);
        assert_eq!(count(&h), n + 1, "paste added a copy");
        h.event(egui::Event::Cut);
        h.run_steps(3);
        assert_eq!(count(&h), n, "cut removed the object");
        h.event(egui::Event::Paste(h.state().clip_text.clone()));
        h.run_steps(2);
        assert_eq!(count(&h), n + 1, "and what was cut can be pasted back");
        // Text copied somewhere else becomes a text box.
        h.event(egui::Event::Paste("from the browser".into()));
        h.run_steps(2);
        assert_eq!(count(&h), n + 2);
        assert_eq!(h.state().selected_object().unwrap().text, "from the browser");
    }

    #[test]
    fn gui_double_clicking_blank_space_makes_a_text_box_to_type_in() {
        let mut a = app();
        a.insert_rect();
        a.selected = None;
        let mut h = harness_fast(a);
        h.run_steps(4);
        let count = |h: &egui_kittest::Harness<'static, App>| h.state().project.as_ref().unwrap().slides[0].layers[0].objects.len();
        let n = count(&h);
        let at = egui::pos2(900.0, 650.0);
        h.event(egui::Event::PointerMoved(at));
        h.run_steps(2);
        for _ in 0..2 {
            h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() });
            h.run_steps(1);
            h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() });
            h.run_steps(1);
        }
        h.run_steps(3);
        assert_eq!(count(&h), n + 1, "a text box was made");
        assert!(h.state().editing.is_some(), "and it is ready for typing");
        assert_eq!(h.state().ribbon_tab, ribbon::HOME_TAB);
        // Leaving it empty takes it away again.
        press(&mut h, egui::Key::Escape);
        h.run_steps(2);
        assert_eq!(count(&h), n, "an empty text box is not kept");
        assert!(h.state().editing.is_none());
    }

    #[test]
    fn inserting_an_object_switches_to_home_and_leaves_you_free_to_leave() {
        let mut a = app();
        a.ribbon_tab = 2;
        a.insert_table(2, 2);
        assert_eq!(a.ribbon_tab, ribbon::HOME_TAB, "inserting a table opens Home");
        a.ribbon_tab = 2;
        a.insert_rect();
        assert_eq!(a.ribbon_tab, ribbon::HOME_TAB, "adding an object opens Home");
        assert!(a.selected.is_some(), "and keeps it selected");
        a.ribbon_tab = 2;
        a.duplicate_selected();
        assert_eq!(a.ribbon_tab, ribbon::HOME_TAB, "duplicating opens Home");
        a.ribbon_tab = 2;
        a.copy_selected();
        a.paste();
        assert_eq!(a.ribbon_tab, ribbon::HOME_TAB, "pasting opens Home");
        a.ribbon_tab = 2;
        let mut h = harness(a);
        h.run_steps(4);
        assert_eq!(h.state().ribbon_tab, 2, "clicking away from Home sticks while the object stays selected");
    }

    #[test]
    fn the_format_tab_follows_the_selection_and_its_size_fields_move_the_object() {
        let mut a = app();
        a.insert_rect();
        assert_eq!(a.format_title(), Some("SHAPE"));
        a.insert_text();
        assert_eq!(a.format_title(), Some("TEXT BOX"));
        a.insert(Object::new("pic", "Pic", Kind::Image, 0.0, 0.0, 100.0, 100.0));
        assert_eq!(a.format_title(), Some("PICTURE"), "a selected picture has its own tab");
        a.selected = None;
        assert_eq!(a.format_title(), None, "nothing selected: no contextual tab");
        a.selected = Some((0, 0));
        let before = a.selected_object().unwrap().clone();
        let mut after = before.clone();
        after.x = 123.0; after.y = 45.0; after.w = 300.0; after.h = 90.0;
        let mut h = harness(a);
        h.run_steps(2);
        let ctx = h.ctx.clone();
        h.state_mut().apply_object_edit(&ctx, &before, &after);
        let o = h.state().selected_object().unwrap().clone();
        assert_eq!((o.x, o.y, o.w, o.h), (123.0, 45.0, 300.0, 90.0), "the Size and Position fields write the frame");
        h.state_mut().undo();
        h.state_mut().selected = Some((0, 0));
        assert_eq!(h.state().selected_object().unwrap().x, before.x);
    }

    #[test]
    fn effects_follow_the_outline_of_rounded_boxes_ellipses_stars_and_turned_shapes() {
        let mut a = app();
        let star: Vec<PathCmd> = {
            let mut v = vec![];
            for i in 0..10 {
                let ang = (i as f32 * 36.0 - 90.0).to_radians();
                let rad = if i % 2 == 0 { 0.5 } else { 0.2 };
                let (x, y) = (0.5 + rad * ang.cos(), 0.5 + rad * ang.sin());
                v.push(if i == 0 { PathCmd::Move(x, y) } else { PathCmd::Line(x, y) });
            }
            v.push(PathCmd::Close);
            v
        };
        let mut add = |name: &str, kind: Kind, x: f32, y: f32, f: &dyn Fn(&mut Object)| {
            let mut o = Object::new(name, name, kind, x, y, 300.0, 140.0);
            o.fill = Some(Rgba(3, 83, 158, 255));
            f(&mut o);
            a.project.as_mut().unwrap().slides[0].layers[0].objects.push(o);
        };
        add("shadow", Kind::Rect, 60.0, 60.0, &|o| { o.radius = 40.0; o.shadow = Some(Shadow { dx: 0.0, dy: 12.0, blur: 30.0, spread: 0.0, color: Rgba(0, 0, 0, 200) }); });
        add("glow", Kind::Rect, 470.0, 60.0, &|o| { o.radius = 40.0; o.glow = Some(Glow { radius: 30.0, color: Rgba(255, 190, 40, 255) }); });
        add("soft", Kind::Rect, 880.0, 60.0, &|o| { o.radius = 40.0; o.soft_edge = 30.0; });
        add("reflect", Kind::Rect, 60.0, 360.0, &|o| { o.radius = 40.0; o.reflection = Some(Reflection { size: 0.6, opacity: 0.5, gap: 6.0 }); });
        add("star", Kind::Path(star.clone()), 470.0, 330.0, &|o| { o.h = 220.0; o.w = 220.0; o.reflection = Some(Reflection { size: 0.6, opacity: 0.5, gap: 6.0 }); o.glow = Some(Glow { radius: 24.0, color: Rgba(255, 190, 40, 255) }); });
        add("star-soft", Kind::Path(star), 780.0, 330.0, &|o| { o.h = 220.0; o.w = 220.0; o.soft_edge = 24.0; });
        add("oval", Kind::Ellipse, 60.0, 700.0, &|o| { o.soft_edge = 30.0; });
        add("turned", Kind::Rect, 470.0, 700.0, &|o| { o.radius = 30.0; o.rotation = 12.0; o.shadow = Some(Shadow { dx: 14.0, dy: 14.0, blur: 24.0, spread: 0.0, color: Rgba(0, 0, 0, 200) }); });
        let mut h = harness(a);
        h.run_steps(4);
        let img = h.render().expect("renderer");
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") { img.save(std::path::PathBuf::from(dir).join("button-effects.png")).unwrap(); }
        let o = &h.state().project.as_ref().unwrap().slides[0].layers[0].objects;
        assert!(o[0].shadow.is_some() && o[1].glow.is_some() && o[2].soft_edge == 30.0 && o[3].reflection.is_some(), "the effects are kept on the objects");
    }

    #[test]
    fn the_effects_group_has_a_button_and_a_short_popup_for_each_effect() {
        use egui_kittest::kittest::Queryable;
        let mut a = app();
        let mut o = Object::new("b", "Button", Kind::Rect, 100.0, 100.0, 400.0, 120.0);
        o.fill = Some(Rgba(3, 83, 158, 255));
        a.project.as_mut().unwrap().slides[0].layers[0].objects.push(o);
        a.selected = Some((0, 0));
        let mut h = harness(a);
        h.state_mut().ribbon_tab = ribbon::HOME_TAB;
        h.run_steps(4);
        let obj = |h: &egui_kittest::Harness<'_, App>| h.state().project.as_ref().unwrap().slides[0].layers[0].objects[0].clone();
        for label in ["Presets", "Shadow", "Glow", "Reflection", "Soft edges", "Corners"] {
            assert!(h.query_by_label(label).is_some(), "{label} has its own button in the Effects group");
        }
        assert!(h.query_by_label("Distance").is_none() && h.query_by_label("Fade").is_none(), "nothing is open yet");
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") { h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("effects-group.png")).unwrap(); }
        // Presets.
        h.get_by_label("Presets").click();
        h.run_steps(4);
        h.get_by_label("Soft shadow").click();
        h.run_steps(4);
        assert!(obj(&h).shadow.is_some() && obj(&h).glow.is_none(), "the preset turns on a shadow");
        h.key_press(egui::Key::Escape);
        h.run_steps(3);
        // Shadow: the switch, then its controls.
        h.get_by_label("Shadow").click();
        h.run_steps(4);
        for label in ["Use shadow", "Distance", "Direction", "Blur", "Spread"] { assert!(h.query_by_label(label).is_some(), "{label} is in the shadow popup"); }
        assert!(h.query_by_label("Fade").is_none() && h.query_by_label("Radius").is_none(), "only this effect's controls");
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") { h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("effects-shadow-popup.png")).unwrap(); }
        h.get_by_label("Use shadow").click();
        h.run_steps(4);
        assert!(obj(&h).shadow.is_none(), "the switch turns the shadow off again");
        h.key_press(egui::Key::Escape);
        h.run_steps(3);
        // Glow, reflection and soft edges each switch on from their own popup.
        for (button, switch) in [("Glow", "Use glow"), ("Reflection", "Use reflection"), ("Soft edges", "Use soft edges")] {
            h.get_by_label(button).click();
            h.run_steps(4);
            h.get_by_label(switch).click();
            h.run_steps(4);
            h.key_press(egui::Key::Escape);
            h.run_steps(3);
        }
        let o = obj(&h);
        assert!(o.glow.is_some() && o.reflection.is_some() && o.soft_edge > 0.0, "{:?}", (o.glow, o.reflection, o.soft_edge));
        h.get_by_label("Presets").click();
        h.run_steps(4);
        h.get_by_label("None").click();
        h.run_steps(4);
        let o = obj(&h);
        assert!(o.shadow.is_none() && o.glow.is_none() && o.reflection.is_none() && o.soft_edge == 0.0, "None clears every effect");
        assert_eq!(o.fill, Some(Rgba(3, 83, 158, 255)), "and leaves the look alone");
    }

    #[test]
    fn gui_picture_format_tab_crops_and_outlines() {
        let mut a = app();
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_fn(400, 200, |x, _| if x < 200 { image::Rgba([200, 60, 60, 255]) } else { image::Rgba([60, 60, 200, 255]) }))
            .write_to(&mut png, image::ImageFormat::Png).unwrap();
        a.project.as_mut().unwrap().assets.insert("pic".into(), Asset { name: "p.png".into(), mime: "image/png".into(), bytes: png.into_inner(), file: None });
        let mut o = Object::new("photo", "Photo", Kind::Image, 100.0, 100.0, 600.0, 300.0);
        o.asset = Some("pic".into());
        o.stroke = Some(Stroke { color: Rgba(255, 140, 0, 255), width: 8.0 });
        o.shadow = Some(Shadow { dx: 10.0, dy: 10.0, blur: 10.0, spread: 0.0, color: Rgba(0, 0, 0, 120) });
        a.insert(o);
        a.selected = Some((0, 0));
        let mut h = harness(a);
        h.run_steps(3);
        assert_eq!(h.state().format_title(), Some("PICTURE"));
        h.state_mut().ribbon_tab = ribbon::HOME_TAB;
        h.run_steps(3);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("picture-format.png")).unwrap();
        }
        {
            use egui_kittest::kittest::Queryable;
            h.get_by_label("Crop").click();
            h.run_steps(3);
        }
        assert_eq!(h.state().crop_mode.as_deref(), Some("photo"), "the Crop button on Home starts cropping");
    }

    #[test]
    fn gui_table_format_tab_edits_rows_and_columns() {
        use egui_kittest::kittest::Queryable;
        let mut a = app();
        a.insert_table(3, 3);
        assert_eq!(a.selected.map(|s| s.0), Some(0));
        let mut h = harness(a);
        h.run_steps(3);
        assert_eq!(h.state().format_title(), Some("TABLE"), "a selected table cell has the Table tab");
        h.state_mut().ribbon_tab = ribbon::HOME_TAB;
        h.run_steps(3);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("table-format.png")).unwrap();
        }
        let cells = |h: &egui_kittest::Harness<'static, App>| h.state().project.as_ref().unwrap().slides[0].layers[0].objects.iter().filter(|o| o.table.is_some()).count();
        assert_eq!(cells(&h), 9);
        // Home is wider than the window now: scroll the ribbon to the table's groups.
        h.event(egui::Event::PointerMoved(egui::pos2(700.0, 100.0)));
        h.event(egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta: egui::vec2(-600.0, 0.0), modifiers: Default::default(), phase: egui::TouchPhase::Move });
        h.run_steps(4);
        h.get_by_label("Add").click();
        h.run_steps(2);
        h.get_by_label_contains("Row below").click();
        h.run_steps(3);
        assert_eq!(cells(&h), 12, "Row below added a row of three cells");
        h.get_by_label("Add").click();
        h.run_steps(2);
        h.get_by_label_contains("Column right").click();
        h.run_steps(3);
        assert_eq!(cells(&h), 16, "Column right added a column of four cells");
    }

    #[test]
    fn gui_media_and_hotspot_format_tabs() {
        use egui_kittest::kittest::Queryable;
        let mut a = app();
        let v = Object::new("vid", "Clip", Kind::Video, 100.0, 100.0, 640.0, 360.0);
        a.insert(v);
        let mut h = harness(a);
        h.run_steps(3);
        assert_eq!(h.state().format_title(), Some("VIDEO"));
        h.state_mut().ribbon_tab = ribbon::HOME_TAB;
        h.run_steps(3);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("video-format.png")).unwrap();
        }
        h.get_all_by_label("Loop").next().unwrap().click(); // the ribbon's comes first
        h.run_steps(3);
        assert!(h.state().selected_object().unwrap().media.looped, "the Loop checkbox on Home sets the video's loop");
        // A hotspot has its own tab with the outline tools.
        { let st = h.state_mut(); let sl = st.slide; st.insert(Object::new("spot", "Hotspot", Kind::Hotspot, 50.0, 50.0, 200.0, 100.0)); let _ = sl; }
        h.run_steps(3);
        assert_eq!(h.state().format_title(), Some("HOTSPOT"));
        h.state_mut().ribbon_tab = ribbon::HOME_TAB;
        h.run_steps(3);
        assert!(h.query_all_by_label("Outline").count() > 0, "the outline tool is in the hotspot's tab");
    }

    #[test]
    fn gui_editing_text_in_place_stays_inside_the_object_and_shows_the_text() {
        let mut a = app();
        a.insert_text();
        let (li, oi) = a.selected.unwrap();
        a.project.as_mut().unwrap().slides[0].layers[li].objects[oi].text = "Hello there".into();
        let mut h = harness(a);
        h.run_steps(3);
        h.state_mut().editing = Some((li, oi));
        h.state_mut().editing_old = "Hello there".into();
        h.run_steps(4);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("text-editing.png")).unwrap();
        }
        // Typing still lands in the object.
        h.event(egui::Event::Text("!".into()));
        h.run_steps(3);
        assert!(h.state().project.as_ref().unwrap().slides[0].layers[li].objects[oi].text.contains('!'), "typing reaches the object while it is edited in place");
    }

    /// Starts editing a text box in place, with some text typed into it.
    fn editing_text_box() -> egui_kittest::Harness<'static, App> {
        let mut a = app();
        a.insert_text();
        let (li, oi) = a.selected.unwrap();
        a.project.as_mut().unwrap().slides[0].layers[li].objects[oi].text = "Hello there".into();
        let mut h = harness(a);
        h.run_steps(3);
        h.state_mut().editing = Some((li, oi));
        h.state_mut().editing_old = "Hello there".into();
        h.run_steps(4);
        assert!(h.state().editing.is_some(), "the box is being edited");
        h
    }

    #[test]
    fn pressing_a_button_in_try_it_says_when_it_would_jump_to_another_slide() {
        let mut a = app();
        a.insert_button();
        let (li, oi) = a.selected.unwrap();
        let (id, name) = ("Quiz time".to_string(), "Quiz time");
        let target = {
            let p = a.project.as_mut().unwrap();
            let mut s2 = Slide::new("slide-2", name, 1280.0, 720.0);
            s2.name = id;
            let sid = s2.id.clone();
            p.slides.push(s2);
            sid
        };
        let oid = a.project.as_ref().unwrap().slides[0].layers[li].objects[oi].id.clone();
        a.project.as_mut().unwrap().slides[0].triggers.push(Trigger { id: "t1".into(), owner: Owner::Object(oid), event: Event::Click, action: Action::JumpToSlide(target), conditions: vec![] });
        let o = a.project.as_ref().unwrap().slides[0].layers[li].objects[oi].clone();
        a.try_mode = true;
        let mut h = harness(a);
        h.run_steps(4);
        assert!(h.state().try_note.is_none(), "nothing pressed yet");
        let (origin, scale) = h.state().canvas_frame.expect("the canvas has been drawn");
        click_at(&mut h, origin + egui::vec2(o.x + o.w / 2.0, o.y + o.h / 2.0) * scale);
        let note = h.state().try_note.clone().expect("the press says what Try It did not do");
        assert!(note.contains("jumps to Quiz time") && note.contains("Preview"), "{note}");
        assert_eq!(h.state().slide, 0, "Try It stays on the slide: the jump is explained, not followed");
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") { h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("try-it-jump-note.png")).unwrap(); }
        use egui_kittest::kittest::Queryable;
        // It is a dialog with an OK button; OK dismisses it and leaves Try It on.
        h.run_steps(3);
        h.get_by_label("Not run in Try It");
        h.get_by_label("OK").click();
        h.run_steps(3);
        assert!(h.state().try_note.is_none() && h.state().try_mode, "OK closes the dialog and Try It stays on");
        assert!(h.query_by_label("Not run in Try It").is_none());
        // Pressing again brings it back, and Esc closes it before it stops Try It.
        click_at(&mut h, origin + egui::vec2(o.x + o.w / 2.0, o.y + o.h / 2.0) * scale);
        assert!(h.state().try_note.is_some());
        h.key_press(egui::Key::Escape);
        h.run_steps(3);
        assert!(h.state().try_note.is_none() && h.state().try_mode, "the first Esc closes the dialog");
        // Stopping Try It also clears a note.
        click_at(&mut h, origin + egui::vec2(o.x + o.w / 2.0, o.y + o.h / 2.0) * scale);
        h.state_mut().try_mode = false;
        h.run_steps(3);
        assert!(h.state().try_note.is_none(), "the note goes when Try It stops");
    }

    /// A button on the base layer, a layer over it that is showing, and Try It on with that layer already shown.
    fn try_it_over_a_layer(modal: bool) -> (egui_kittest::Harness<'static, App>, Object) {
        let mut a = app();
        a.insert_button();
        let (li, oi) = a.selected.unwrap();
        a.add_layer();
        let layer_id = {
            let p = a.project.as_mut().unwrap();
            let l = p.slides[0].layers.last_mut().unwrap();
            l.name = "Pop-up".into();
            l.modal = modal;
            l.id.clone()
        };
        a.layer = 0;
        let o = a.project.as_ref().unwrap().slides[0].layers[li].objects[oi].clone();
        a.try_mode = true;
        let mut h = harness(a);
        h.run_steps(3);
        h.state_mut().try_slide = Some(0);
        h.state_mut().try_layers = vec![layer_id];
        h.run_steps(3);
        (h, o)
    }

    #[test]
    fn pressing_something_a_blocking_layer_covers_says_so() {
        let (mut h, o) = try_it_over_a_layer(true);
        let (origin, scale) = h.state().canvas_frame.expect("the canvas has been drawn");
        click_at(&mut h, origin + egui::vec2(o.x + o.w / 2.0, o.y + o.h / 2.0) * scale);
        let note = h.state().try_note.clone().expect("a dead-looking press explains itself");
        assert!(note.contains("Pop-up") && note.contains("Block clicks on the layers beneath"), "{note}");
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") { h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("try-it-blocked-note.png")).unwrap(); }
    }

    #[test]
    fn a_layer_that_lets_clicks_through_gives_no_blocked_note() {
        let (mut h, o) = try_it_over_a_layer(false);
        let (origin, scale) = h.state().canvas_frame.expect("the canvas has been drawn");
        click_at(&mut h, origin + egui::vec2(o.x + o.w / 2.0, o.y + o.h / 2.0) * scale);
        assert!(h.state().try_note.is_none(), "{:?}", h.state().try_note);
    }

    #[test]
    fn dragging_the_rotate_handle_while_editing_text_ends_the_edit_and_turns_the_box() {
        let mut h = editing_text_box();
        let (li, oi) = h.state().selected.unwrap();
        let o = h.state().project.as_ref().unwrap().slides[0].layers[li].objects[oi].clone();
        let (origin, scale) = h.state().canvas_frame.expect("the canvas has been drawn");
        let knob = origin + egui::vec2(o.x + o.w / 2.0, o.y) * scale - egui::vec2(0.0, 26.0);
        let centre = origin + egui::vec2(o.x + o.w / 2.0, o.y + o.h / 2.0) * scale;
        drag(&mut h, knob, centre + egui::vec2(150.0, 0.0));
        let after = h.state().project.as_ref().unwrap().slides[0].layers[li].objects[oi].clone();
        assert!(after.rotation.abs() > 1.0, "the handle still turns the box: {}", after.rotation);
        assert!(h.state().editing.is_none(), "an edit box that cannot turn is not left behind a turned frame");
        assert_eq!(after.text, "Hello there", "the typed text is kept");
    }

    #[test]
    fn turning_a_text_box_from_properties_or_the_ribbon_also_ends_the_edit() {
        for change in [|o: &mut Object| o.rotation = 40.0, |o: &mut Object| o.text_rotation = 270] {
            let mut h = editing_text_box();
            let (li, oi) = h.state().selected.unwrap();
            change(&mut h.state_mut().project.as_mut().unwrap().slides[0].layers[li].objects[oi]);
            h.run_steps(3);
            assert!(h.state().editing.is_none(), "turning the object ends the edit");
            assert_eq!(h.state().project.as_ref().unwrap().slides[0].layers[li].objects[oi].text, "Hello there");
        }
    }

    fn editing_a_turned_text_box() -> egui_kittest::Harness<'static, App> {
        let mut a = app();
        a.insert_text();
        let (li, oi) = a.selected.unwrap();
        {
            let o = &mut a.project.as_mut().unwrap().slides[0].layers[li].objects[oi];
            o.text = "Hello there".into();
            o.rotation = 30.0;
        }
        let mut h = harness(a);
        h.run_steps(3);
        h.state_mut().editing = Some((li, oi));
        h.state_mut().editing_old = "Hello there".into();
        h.run_steps(5);
        h
    }

    #[test]
    fn a_turned_box_is_edited_upright_and_keeps_its_rotation() {
        let mut h = editing_a_turned_text_box();
        let (li, oi) = h.state().selected.unwrap();
        assert!(h.state().editing.is_some(), "a rotation that was already there does not end the edit");
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") { h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("turned-box-editing.png")).unwrap(); }
        h.event(egui::Event::Text("!".into()));
        h.run_steps(3);
        // Finish with Escape, as a person would.
        h.event(egui::Event::Key { key: egui::Key::Escape, physical_key: Some(egui::Key::Escape), pressed: true, repeat: false, modifiers: Default::default() });
        h.run_steps(3);
        assert!(h.state().editing.is_none());
        let o = h.state().project.as_ref().unwrap().slides[0].layers[li].objects[oi].clone();
        assert!(o.text.contains('!'), "the typing reached the box");
        assert_eq!(o.rotation, 30.0, "only what is drawn was straightened, not the box itself");
    }

    #[test]
    fn while_a_turned_box_is_edited_its_handles_are_drawn_upright_with_the_edit_box() {
        let mut h = editing_a_turned_text_box();
        let (li, oi) = h.state().selected.unwrap();
        let o = h.state().project.as_ref().unwrap().slides[0].layers[li].objects[oi].clone();
        let (origin, scale) = h.state().canvas_frame.expect("the canvas has been drawn");
        // Where the rotate handle is when the frame is upright: straight above the middle of the top edge.
        let upright_knob = origin + egui::vec2(o.x + o.w / 2.0, o.y) * scale - egui::vec2(0.0, 26.0);
        let centre = origin + egui::vec2(o.x + o.w / 2.0, o.y + o.h / 2.0) * scale;
        drag(&mut h, upright_knob, centre + egui::vec2(150.0, 0.0));
        let after = h.state().project.as_ref().unwrap().slides[0].layers[li].objects[oi].clone();
        assert!((after.rotation - 30.0).abs() > 5.0, "the handle was found where an upright frame has it: {}", after.rotation);
    }

    #[test]
    fn editing_a_text_box_that_is_already_turned_is_not_cut_short() {
        let mut a = app();
        a.insert_text();
        let (li, oi) = a.selected.unwrap();
        a.project.as_mut().unwrap().slides[0].layers[li].objects[oi].rotation = 30.0;
        let mut h = harness(a);
        h.run_steps(3);
        h.state_mut().editing = Some((li, oi));
        h.run_steps(5);
        assert!(h.state().editing.is_some(), "only a change of rotation ends the edit, not a rotation that was already there");
    }

    #[test]
    fn inserted_button_has_states_and_undo_restores() {
        let mut a = app();
        a.insert_button();
        let o = a.selected_object().unwrap().clone();
        assert_eq!(o.states.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), vec!["Normal", "Hover", "Down", "Visited", "Disabled", "Selected"]);
        assert_eq!(o.initial_state, "Normal");
        a.delete_selected();
        assert!(a.project.as_ref().unwrap().slides[0].layers[0].objects.is_empty());
        a.undo();
        assert_eq!(a.project.as_ref().unwrap().slides[0].layers[0].objects.len(), 1);
    }

    #[test]
    fn undo_also_restores_variables_and_project_settings() {
        let mut a = app();
        a.insert_text();
        // A settings edit, as the Design tab makes it.
        let before = Meta::of(a.project.as_ref().unwrap());
        {
            let p = a.project.as_mut().unwrap();
            p.pass_mark = 55;
            p.show_menu = true;
            p.variables.push(Variable { id: "v".into(), name: "Score".into(), initial: Value::Num(1.0), built_in: false });
        }
        a.push_undo_meta(before);
        a.undo();
        let p = a.project.as_ref().unwrap();
        assert_eq!(p.pass_mark, 80);
        assert!(!p.show_menu);
        assert!(p.variables.is_empty());
        assert_eq!(p.slides[0].layers[0].objects.len(), 1, "slide content is untouched by that step");
        a.redo();
        let p = a.project.as_ref().unwrap();
        assert_eq!((p.pass_mark, p.show_menu, p.variables.len()), (55, true, 1));
        // A slide edit afterwards still undoes cleanly without disturbing the settings.
        a.selected = Some((0, 0)); // undo/redo clear the selection
        a.delete_selected();
        a.undo();
        let p = a.project.as_ref().unwrap();
        assert_eq!(p.slides[0].layers[0].objects.len(), 1);
        assert_eq!(p.pass_mark, 55);
    }

    #[test]
    fn autosave_writes_a_recoverable_copy_and_saving_clears_it() {
        let home = std::env::temp_dir().join(format!("brainbend-home-{}", std::process::id()));
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("HOME", &home);
        let mut a = app();
        a.insert_text();
        a.autosave();
        let rec = recovery_path().unwrap();
        assert!(rec.exists());
        let back = Project::from_bytes(&std::fs::read(&rec).unwrap()).unwrap();
        assert_eq!(back.slides[0].layers[0].objects.len(), 1);
        App::clear_recovery();
        assert!(!rec.exists());
        // A clean project is not autosaved.
        a.dirty = false;
        a.autosave();
        assert!(!rec.exists());
        let _ = std::fs::remove_dir_all(home);
    }

    #[test]
    fn copy_and_paste_makes_an_independent_offset_copy() {
        let mut a = app();
        a.insert_rect();
        let orig = a.selected_object().unwrap().clone();
        a.copy_selected();
        a.paste();
        a.paste();
        let objs = &a.project.as_ref().unwrap().slides[0].layers[0].objects;
        assert_eq!(objs.len(), 3);
        assert_ne!(objs[1].id, objs[0].id);
        assert_ne!(objs[2].id, objs[1].id);
        assert!((objs[1].x - (orig.x + 24.0)).abs() < 0.01);
        assert!((objs[2].x - (orig.x + 48.0)).abs() < 0.01);
    }

    fn three_rects(a: &mut App) {
        for (x, y, w) in [(10.0, 20.0, 100.0), (300.0, 80.0, 50.0), (500.0, 40.0, 200.0)] {
            a.insert_rect();
            let (li, oi) = a.selected.unwrap();
            let o = &mut a.project.as_mut().unwrap().slides[0].layers[li].objects[oi];
            o.x = x; o.y = y; o.w = w; o.h = 30.0;
        }
        let ids: Vec<String> = a.project.as_ref().unwrap().slides[0].layers[0].objects.iter().map(|o| o.id.clone()).collect();
        a.selected = Some((0, 0));
        a.multi = ids[1..].to_vec();
    }

    #[test]
    fn multi_selection_aligns_distributes_and_undoes() {
        let mut a = app();
        three_rects(&mut a);
        assert_eq!(a.selection().len(), 3);
        a.align_selected(0, 2); // right edges to the group's right edge (700)
        let objs = a.project.as_ref().unwrap().slides[0].layers[0].objects.clone();
        assert!(objs.iter().all(|o| (o.x + o.w - 700.0).abs() < 0.01));
        a.undo();
        a.selected = Some((0, 0)); // undo clears the selection; reselect the group
        a.multi = a.project.as_ref().unwrap().slides[0].layers[0].objects.iter().skip(1).map(|o| o.id.clone()).collect();
        a.align_selected(0, 3); // equal gaps: widths 350 over span 10..700 leaves 340, 170 each
        let o = &a.project.as_ref().unwrap().slides[0].layers[0].objects;
        assert!((o[1].x - (10.0 + 100.0 + 170.0)).abs() < 0.01);
        assert!((o[2].x - (o[1].x + 50.0 + 170.0)).abs() < 0.01);
        a.align_selected(1, 1); // vertical centres coincide
        let o = &a.project.as_ref().unwrap().slides[0].layers[0].objects;
        let c0 = o[0].y + o[0].h / 2.0;
        assert!(o.iter().all(|q| ((q.y + q.h / 2.0) - c0).abs() < 0.01));
    }

    #[test]
    fn multi_selection_moves_deletes_and_duplicates_together() {
        let mut a = app();
        three_rects(&mut a);
        a.nudge_selected(5.0, 7.0);
        let o = &a.project.as_ref().unwrap().slides[0].layers[0].objects;
        assert!((o[0].x - 15.0).abs() < 0.01 && (o[2].y - 47.0).abs() < 0.01);
        a.duplicate_selected();
        assert_eq!(a.project.as_ref().unwrap().slides[0].layers[0].objects.len(), 6);
        assert_eq!(a.selection().len(), 3);
        a.delete_selected();
        assert_eq!(a.project.as_ref().unwrap().slides[0].layers[0].objects.len(), 3);
        assert!(a.selected.is_none());
    }

    #[test]
    fn gui_in_place_editor_renders_mixed_formatting_without_panicking() {
        let mut a = app();
        a.insert_text();
        let (li, oi) = a.selected.unwrap();
        {
            let o = &mut a.project.as_mut().unwrap().slides[0].layers[li].objects[oi];
            o.edit_text("Hello bold world");
            o.format_range(6, 10, &Attr::ToggleBold);
            o.format_range(0, 5, &Attr::ToggleItalic);
        }
        a.editing = Some((li, oi));
        a.editing_old = "Hello bold world".into();
        let mut h = harness(a);
        h.run_steps(4);
        assert!(h.state().project.as_ref().unwrap().slides[0].layers[li].objects[oi].rich.is_some());
    }

    #[test]
    fn ribbon_text_buttons_format_only_the_selected_characters() {
        let mut a = app();
        a.insert_text();
        let (li, oi) = a.selected.unwrap();
        a.project.as_mut().unwrap().slides[0].layers[li].objects[oi].edit_text("Hello world");
        a.text_sel = Some((0, 5));
        a.format_selection(Attr::ToggleBold);
        let o = a.selected_object().unwrap();
        let runs = &o.rich.as_ref().unwrap()[0].runs;
        assert_eq!(runs[0].text, "Hello");
        assert_eq!(runs[0].bold, Some(true));
        assert_eq!(runs[1].bold, None);
        a.undo();
        assert!(a.project.as_ref().unwrap().slides[0].layers[li].objects[oi].rich.is_none());
    }

    #[test]
    fn ribbon_edits_apply_to_every_selected_object() {
        let mut a = app();
        three_rects(&mut a);
        let ctx = egui::Context::default();
        let before = a.selected_object().unwrap().clone();
        let mut after = before.clone();
        after.fill = Some(Rgba(1, 2, 3, 255));
        after.style.size = 33.0;
        a.apply_object_edit(&ctx, &before, &after);
        for o in &a.project.as_ref().unwrap().slides[0].layers[0].objects {
            assert_eq!(o.fill, Some(Rgba(1, 2, 3, 255)));
            assert_eq!(o.style.size, 33.0);
        }
        a.undo();
        assert_ne!(a.project.as_ref().unwrap().slides[0].layers[0].objects[2].fill, Some(Rgba(1, 2, 3, 255)));
    }

    #[test]
    fn gui_design_tab_changes_colors_and_fonts_and_hides_background_graphics() {
        use egui_kittest::kittest::Queryable;
        let mut a = App::default();
        a.new_project();
        {
            let p = a.project.as_mut().unwrap();
            p.master.objects.push(Object::new("logo", "Logo", Kind::Rect, 0.0, 0.0, 50.0, 50.0));
            let id = p.layouts[1].id.clone();
            p.slides[0].layout = Some(id);
        }
        a.ribbon_tab = 4;
        let mut h = harness(a);
        h.run_steps(3);
        let theme = |h: &egui_kittest::Harness<'_, App>| h.state().project.as_ref().unwrap().theme.clone();
        let start = theme(&h);
        let start_fonts = (start.heading_font.clone(), start.body_font.clone());
        // Fonts: another pair makes it a custom theme.
        h.get_by_label_contains("Fonts").click();
        h.run_steps(3);
        h.get_by_label("Montserrat / Montserrat").click();
        h.run_steps(3);
        let t = theme(&h);
        assert_eq!((t.heading_font.as_str(), t.body_font.as_str()), ("Montserrat", "Montserrat"));
        assert_ne!((t.heading_font.clone(), t.body_font.clone()), start_fonts);
        assert_eq!(t.name, "Custom");
        // Hide Background Graphics: this slide stops inheriting the master's shapes.
        assert_eq!(h.state().project.as_ref().unwrap().backdrop(&h.state().project.as_ref().unwrap().slides[0]).len(), 1);
        h.get_by_label("Hide Background Graphics").click();
        h.run_steps(3);
        let p = h.state().project.as_ref().unwrap();
        assert!(p.slides[0].hide_backdrop && p.backdrop(&p.slides[0]).is_empty());
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.state_mut().theme_colors_open = true;
            h.run_steps(3);
            h.render().expect("renderer").save(std::path::Path::new(&dir).join("design-theme-colors.png")).unwrap();
            h.state_mut().theme_colors_open = false;
            h.run_steps(2);
        }
        // "Edit…" in a colour menu asks for the same window.
        h.ctx.data_mut(|d| d.insert_temp(egui::Id::new("open-project-colors"), true));
        h.run_steps(3);
        assert!(h.state().theme_colors_open);
        h.state_mut().theme_colors_open = false;
        h.run_steps(2);
        // One undo each.
        h.state_mut().undo();
        assert!(!h.state().project.as_ref().unwrap().slides[0].hide_backdrop);
        h.state_mut().undo();
        assert_eq!(theme(&h), start);
    }

    #[test]
    fn gui_brand_colors_recent_colors_and_the_colour_menu_notes() {
        use egui_kittest::kittest::Queryable;
        let mut a = App::default();
        a.new_project();
        a.ribbon_tab = 4;
        let mut h = harness(a);
        h.run_steps(3);
        let proj = |h: &egui_kittest::Harness<'_, App>| h.state().project.as_ref().unwrap().clone();
        assert!(proj(&h).palette.is_empty() && proj(&h).recent_colors.is_empty(), "nothing is preset");
        // The Brand Colors window: add a colour, change it.
        h.get_by_label_contains("Brand").click();
        h.run_steps(3);
        assert!(h.state().theme_colors_open);
        h.get_by_label("+ Add colour").click();
        h.run_steps(3);
        assert_eq!(proj(&h).palette.len(), 1);
        // A colour menu's + adds the colour it is showing.
        let teal = Rgba(0, 128, 128, 255);
        h.ctx.data_mut(|d| d.insert_temp(egui::Id::new("palette-add"), teal));
        h.run_steps(3);
        assert_eq!(proj(&h).palette, vec![Rgba(128, 128, 128, 255), teal]);
        // Using a colour makes it recent (not if it is in the palette).
        let coral = Rgba(255, 127, 80, 255);
        h.ctx.data_mut(|d| d.insert_temp(egui::Id::new("picked-color"), coral));
        h.run_steps(3);
        h.ctx.data_mut(|d| d.insert_temp(egui::Id::new("picked-color"), teal));
        h.run_steps(3);
        assert_eq!(proj(&h).recent_colors, vec![coral]);
        // Adding a recent colour to the palette takes it out of the recent ones.
        h.ctx.data_mut(|d| d.insert_temp(egui::Id::new("palette-add"), coral));
        h.run_steps(3);
        assert!(proj(&h).recent_colors.is_empty() && proj(&h).palette.contains(&coral));
        // The menus are told what to show, and one undo takes the last addition back.
        let shown: Vec<Rgba> = h.ctx.data(|d| d.get_temp(egui::Id::new("palette-colors"))).unwrap();
        assert_eq!(shown.len(), 3);
        h.state_mut().undo();
        assert_eq!(proj(&h).palette.len(), 2);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.run_steps(3);
            h.render().expect("renderer").save(std::path::Path::new(&dir).join("brand-colors.png")).unwrap();
            // A colour menu, from Home with a text box selected.
            h.state_mut().theme_colors_open = false;
            h.state_mut().insert_text();
            h.state_mut().ribbon_tab = 1;
            h.run_steps(4);
            h.get_by_label("Text color").click();
            h.run_steps(4);
            h.render().expect("renderer").save(std::path::Path::new(&dir).join("colour-menu.png")).unwrap();
        }
    }

    #[test]
    fn gui_ribbon_dropdowns_open_and_add_slides() {
        use egui_kittest::kittest::Queryable;
        let mut a = app();
        a.ribbon_tab = 3;
        let mut h = harness(a);
        h.run_steps(3);
        let before = h.state().project.as_ref().unwrap().slides.len();
        h.get_by_label_contains("Sequencing").click();
        h.run_steps(4);
        assert_eq!(h.state().project.as_ref().unwrap().slides.len(), before + 1);
        assert!(matches!(h.state().project.as_ref().unwrap().slides[h.state().slide].kind, SlideKind::Question(_)));
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.state_mut().selected = None;
            h.run_steps(3);
            h.render().expect("renderer").save(std::path::Path::new(&dir).join("slides-tab.png")).unwrap();
        }
    }

    #[test]
    fn app_starts_on_the_new_page_and_creating_a_media_lesson_opens_the_editor() {
        use egui_kittest::kittest::Queryable;
        let mut h = harness(App::default());
        h.run_steps(3);
        assert!(h.state().file_view && h.state().project.is_none());
        assert!(h.state().file_page == backstage::FilePage::New);
        assert!(h.query_by_label("Project Info").is_none(), "no sidebar on the start page");
        h.get_by_label("Media Lesson").click();
        h.run_steps(4);
        let st = h.state();
        assert!(st.project.is_some() && !st.file_view);
        assert_eq!(st.project.as_ref().unwrap().kind, ProjectKind::MediaLesson);
    }

    #[test]
    fn file_view_opens_closes_and_acts() {
        use egui_kittest::kittest::Queryable;
        let mut a = app();
        a.file_view = true;
        let mut h = harness(a);
        h.run_steps(3);
        // Save As / Save need a dialog or path, so just exercise navigation: Print page, then Home closes.
        h.get_by_label("Print").click();
        h.run_steps(3);
        assert!(h.state().file_page == backstage::FilePage::Print);
        h.get_by_label("Back to Project").click();
        h.run_steps(3);
        assert!(!h.state().file_view);
    }

    #[test]
    fn scenes_group_slides_and_undo_restores_them() {
        let mut a = app();
        a.add_slide(SlideKind::Normal);
        a.add_scene();
        assert_eq!(a.project.as_ref().unwrap().scenes.len(), 2);
        assert_eq!(a.current_scene(), 1);
        assert_eq!(a.project.as_ref().unwrap().slides.len(), 3);
        // Slides stay in scene order and moving never crosses a scene boundary.
        a.slide = 1;
        a.move_slide(1, true);
        assert_eq!(a.slide, 1);
        a.move_slide_to_scene(1, 1);
        let p = a.project.as_ref().unwrap();
        assert_eq!(p.scene_slides(0).len(), 1);
        assert_eq!(p.scene_slides(1).len(), 2);
        a.move_scene(1, false);
        assert_eq!(a.project.as_ref().unwrap().scenes[0].name, "Scene 2");
        a.undo();
        assert_eq!(a.project.as_ref().unwrap().scenes[0].name, "Scene 1");
        a.delete_scene(1);
        assert_eq!(a.project.as_ref().unwrap().scenes.len(), 1);
        a.delete_scene(0); // the last scene stays
        assert_eq!(a.project.as_ref().unwrap().scenes.len(), 1);
    }

    #[test]
    fn master_view_edits_round_trip_and_new_slides_follow_layouts() {
        let mut a = app();
        let real_slides = a.project.as_ref().unwrap().slides.len();
        a.enter_master_view();
        assert!(a.in_master_view());
        let n_layouts = a.project.as_ref().unwrap().layouts.len();
        assert_eq!(a.project.as_ref().unwrap().slides.len(), n_layouts + 1, "master + one stand-in per layout");
        // A logo on the master, and a new layout.
        a.slide = 0;
        a.insert_rect();
        a.insert_layout(None);
        assert_eq!(a.project.as_ref().unwrap().layouts.len(), n_layouts + 1);
        // Saving/publishing sees the folded-back project even while the view is open.
        let committed = a.committed().unwrap();
        assert_eq!(committed.slides.len(), real_slides);
        assert_eq!(committed.master.objects.len(), 1);
        a.leave_master_view();
        let p = a.project.as_ref().unwrap();
        assert_eq!(p.slides.len(), real_slides);
        assert_eq!(p.master.objects.len(), 1);
        assert_eq!(p.layouts.len(), n_layouts + 1);
        // New slides follow a chosen layout and get its placeholders.
        a.add_slide_with_layout(SlideKind::Normal, Some("layout-content".into()));
        let s = &a.project.as_ref().unwrap().slides[a.slide];
        assert_eq!(s.layout.as_deref(), Some("layout-content"));
        assert_eq!(s.layers[0].objects.len(), 2);
        // The master shape is part of the published slide, below the content.
        let flat = a.project.as_ref().unwrap().flattened();
        assert!(flat.slides[a.slide].layers[0].objects[0].id.starts_with("bd-"));
        // The whole visit is one undo step.
        a.undo();
        assert_eq!(a.project.as_ref().unwrap().slides.len(), real_slides);
        a.undo();
        assert!(a.project.as_ref().unwrap().master.objects.is_empty());
        assert_eq!(a.project.as_ref().unwrap().layouts.len(), n_layouts);
    }

    #[test]
    fn master_edits_reach_untouched_placeholders_but_not_customised_ones() {
        let mut a = app();
        a.add_slide_with_layout(SlideKind::Normal, Some("layout-content".into()));
        a.add_slide_with_layout(SlideKind::Normal, Some("layout-content".into()));
        let (s1, s2) = (a.slide - 1, a.slide);
        // The second slide's title is restyled by the author.
        let t = a.project.as_mut().unwrap().slides[s2].layers[0].objects.iter_mut().find(|o| o.placeholder == Some(PlaceholderKind::Title)).unwrap();
        t.style.size = 33.0;
        a.enter_master_view();
        let li = a.project.as_ref().unwrap().slides.iter().position(|s| s.layout.as_deref() == Some("layout-content")).unwrap();
        let title = a.project.as_mut().unwrap().slides[li].layers[0].objects.iter_mut().find(|o| o.placeholder == Some(PlaceholderKind::Title)).unwrap();
        title.style.size = 99.0;
        a.leave_master_view();
        let size = |i: usize| a.project.as_ref().unwrap().slides[i].layers[0].objects.iter().find(|o| o.placeholder == Some(PlaceholderKind::Title)).unwrap().style.size;
        assert_eq!(size(s1), 99.0, "follows the layout");
        assert_eq!(size(s2), 33.0, "keeps the author's change");
    }

    #[test]
    fn gui_ribbon_keeps_its_controls_when_nothing_is_selected() {
        use egui_kittest::kittest::Queryable;
        let mut h = harness(app());
        h.run_steps(3);
        // The Font, Drawing and Animations controls exist (disabled) with no selection.
        assert!(h.query_all_by_label("Arrange").count() > 0);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            let dir = std::path::PathBuf::from(dir);
            h.render().expect("renderer").save(dir.join("7a-ribbon-no-selection.png")).unwrap();
            h.state_mut().insert_text();
            h.run_steps(3);
            h.render().expect("renderer").save(dir.join("7b-ribbon-text-selected.png")).unwrap();
            h.state_mut().ribbon_tab = 5;
            h.state_mut().selected = None;
            h.run_steps(3);
            h.render().expect("renderer").save(dir.join("7c-animations-no-selection.png")).unwrap();
        }
    }

    #[test]
    fn groups_select_move_duplicate_and_resize_together() {
        let mut a = app();
        three_rects(&mut a);
        let n = a.project.as_ref().unwrap().slides[0].layers[0].objects.len();
        assert!(n >= 3);
        // Select the first two and group them.
        a.selected = Some((0, 0));
        a.multi = vec![a.project.as_ref().unwrap().slides[0].layers[0].objects[1].id.clone()];
        a.group_selected();
        a.multi.clear();
        a.selected = Some((0, 1));
        assert_eq!(a.selection().len(), 2, "choosing one member chooses the group");
        // Duplicating gives the copies their own group.
        a.duplicate_selected();
        let objs = &a.project.as_ref().unwrap().slides[0].layers[0].objects;
        let groups: std::collections::HashSet<_> = objs.iter().filter_map(|o| o.group.clone()).collect();
        assert_eq!(groups.len(), 2);
        a.ungroup_selected();
        a.multi.clear();
        a.selected = Some((0, 0));
        assert_eq!(a.selection().len(), 2, "the original group is intact");
        a.undo();
        a.undo();
    }

    #[test]
    fn every_gallery_shape_can_be_inserted() {
        let mut a = app();
        for (name, _) in brainbend_doc::shapes::GALLERY { a.insert_shape(name); }
        assert_eq!(a.project.as_ref().unwrap().slides[0].layers[0].objects.len(), brainbend_doc::shapes::GALLERY.len());
    }

    #[test]
    fn several_slides_can_be_selected_previewed_and_deleted_together() {
        let mut a = app();
        for _ in 0..4 { a.add_slide(SlideKind::Normal); }
        let ids: Vec<String> = a.project.as_ref().unwrap().slides.iter().map(|s| s.id.clone()).collect();
        assert_eq!(ids.len(), 5);
        a.slide = 1;
        a.slide_multi = vec![ids[3].clone()];
        assert_eq!(a.slide_selection(), vec![1, 3]);
        let part = a.project.as_ref().unwrap().subset(&a.slide_selection());
        assert_eq!(part.slides.iter().map(|s| s.id.clone()).collect::<Vec<_>>(), vec![ids[1].clone(), ids[3].clone()]);
        a.delete_slides(a.slide_selection());
        assert_eq!(a.project.as_ref().unwrap().slides.len(), 3);
        assert!(a.slide_multi.is_empty());
        a.undo();
        assert_eq!(a.project.as_ref().unwrap().slides.len(), 5, "one undo step restores both");
        // The last slides cannot all be deleted.
        a.slide_multi = ids.iter().cloned().collect();
        a.delete_slides(a.slide_selection());
        assert_eq!(a.project.as_ref().unwrap().slides.len(), 5);
    }

    #[test]
    fn gui_slide_multi_selection_renders() {
        let mut a = app();
        for _ in 0..3 { a.add_slide(SlideKind::Normal); }
        a.slide = 0;
        let id = a.project.as_ref().unwrap().slides[2].id.clone();
        a.slide_multi = vec![id];
        let mut h = harness(a);
        h.run_steps(3);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("8a-multi-select.png")).unwrap();
        }
    }

    #[test]
    fn selected_slides_duplicate_and_move_as_a_block() {
        let mut a = app();
        for _ in 0..4 { a.add_slide(SlideKind::Normal); }
        let ids = |a: &App| a.project.as_ref().unwrap().slides.iter().map(|s| s.id.clone()).collect::<Vec<_>>();
        let start = ids(&a);
        // Move the middle two down together.
        a.slide = 1;
        a.slide_multi = vec![start[2].clone()];
        assert!(a.can_move_slides(true) && a.can_move_slides(false));
        a.move_slides(true);
        assert_eq!(ids(&a), vec![start[0].clone(), start[3].clone(), start[1].clone(), start[2].clone(), start[4].clone()]);
        a.move_slides(true);
        assert!(!a.can_move_slides(true), "the block is at the end");
        a.undo();
        a.undo();
        assert_eq!(ids(&a), start);
        // Duplicate two slides: the copies follow the last selected one, in order.
        a.slide = 0;
        a.slide_multi = vec![start[1].clone()];
        a.duplicate_slides();
        let after = ids(&a);
        assert_eq!(after.len(), 7);
        assert_eq!(&after[0..2], &start[0..2]);
        assert_eq!(a.slide_selection().len(), 2, "the copies end up selected");
        a.undo();
        assert_eq!(ids(&a), start, "one undo step");
        // Move two slides to a new scene.
        a.add_scene();
        a.slide = 0;
        a.slide_multi = vec![start[1].clone()];
        a.move_selected_slides_to_scene(1);
        let p = a.project.as_ref().unwrap();
        assert_eq!(p.scene_slides(1).len(), 3, "the new scene's own slide plus the two moved ones");
    }

    #[test]
    fn objects_can_be_reordered_from_the_objects_tab() {
        let mut a = app();
        three_rects(&mut a);
        let names = |a: &App| a.project.as_ref().unwrap().slides[0].layers[0].objects.iter().map(|o| o.id.clone()).collect::<Vec<_>>();
        let start = names(&a);
        a.reorder_object(0, 0, 2);
        let after = names(&a);
        assert_eq!(after, vec![start[1].clone(), start[2].clone(), start[0].clone()]);
        assert_eq!(a.selected, Some((0, 2)), "the moved object stays selected");
        a.undo();
        assert_eq!(names(&a), start);
    }

    #[test]
    fn gui_timeline_names_are_the_object_list() {
        let mut a = app();
        three_rects(&mut a);
        a.selected = None;
        a.multi.clear();
        let mut h = harness(a);
        h.run_steps(4);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("9a-timeline-names.png")).unwrap();
        }
        let click = |h: &mut egui_kittest::Harness<'static, App>, pos: egui::Pos2| {
            h.event(egui::Event::PointerMoved(pos));
            h.run_steps(1);
            h.event(egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() });
            h.run_steps(1);
            h.event(egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() });
            h.run_steps(2);
        };
        // Rows run front to back: the first listed is the last object.
        let (x, y0, step) = (300.0, 853.0, 28.0);
        click(&mut h, egui::pos2(x, y0));
        assert_eq!(h.state().selected, Some((0, 2)), "clicking a name in the timeline selects that object");
        click(&mut h, egui::pos2(x, y0 + step));
        assert_eq!(h.state().selected, Some((0, 1)));
        // The eye hides the object when the slide starts.
        click(&mut h, egui::pos2(410.0, y0));
        assert!(h.state().project.as_ref().unwrap().slides[0].layers[0].objects[2].hidden, "the eye hid it");
        // Renaming (double-click starts it): the edit box takes typing and keeps the new name.
        h.state_mut().obj_edit = Some((0, 1));
        h.run_steps(3);
        h.event(egui::Event::Key { key: egui::Key::A, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::COMMAND });
        h.event(egui::Event::Text("Hero".into()));
        h.run_steps(3);
        assert_eq!(h.state().project.as_ref().unwrap().slides[0].layers[0].objects[1].name, "Hero", "the name changed");
        h.state_mut().obj_edit = None;
        h.run_steps(2);
        // Dragging the first row to the last sends that object to the back.
        let ids = |h: &egui_kittest::Harness<'static, App>| h.state().project.as_ref().unwrap().slides[0].layers[0].objects.iter().map(|o| o.id.clone()).collect::<Vec<_>>();
        let before = ids(&h);
        let (from, to) = (egui::pos2(x, y0), egui::pos2(x, y0 + 2.0 * step));
        h.event(egui::Event::PointerMoved(from));
        h.run_steps(1);
        h.event(egui::Event::PointerButton { pos: from, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() });
        h.run_steps(1);
        h.event(egui::Event::PointerMoved(egui::pos2(x, y0 + step)));
        h.run_steps(2);
        h.event(egui::Event::PointerMoved(to));
        h.run_steps(2);
        h.event(egui::Event::PointerButton { pos: to, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() });
        h.run_steps(3);
        let after = ids(&h);
        assert_eq!(after[0], before[2], "the dragged object is now at the back");
    }

    #[test]
    fn a_group_moves_as_one_block_in_the_stack() {
        let mut a = app();
        three_rects(&mut a);
        // A fourth, loose object in front, then group the first two.
        a.insert_rect();
        let ids = |a: &App| a.project.as_ref().unwrap().slides[0].layers[0].objects.iter().map(|o| o.id.clone()).collect::<Vec<_>>();
        let start = ids(&a);
        a.selected = Some((0, 0));
        a.multi = vec![start[1].clone()];
        a.group_selected();
        // Send the group in front of the last object: both members end up after it, in their own order.
        a.move_group(0, 0, 3);
        let after = ids(&a);
        assert_eq!(after, vec![start[2].clone(), start[3].clone(), start[0].clone(), start[1].clone()]);
        a.undo();
        assert_eq!(ids(&a), start);
        // To the very back and to the very front.
        a.move_group_to_edge(0, 0, true);
        assert_eq!(ids(&a)[2..], [start[0].clone(), start[1].clone()]);
        a.move_group_to_edge(0, 2, false);
        assert_eq!(ids(&a)[..2], [start[0].clone(), start[1].clone()]);
    }

    #[test]
    fn gui_timeline_shows_a_group_as_one_row_that_unfolds() {
        let mut a = app();
        three_rects(&mut a);
        a.group_selected();
        a.selected = None;
        a.multi.clear();
        let mut h = harness(a);
        h.run_steps(4);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("9a-timeline-group-folded.png")).unwrap();
        }
        assert!(h.state().tl_open.is_empty());
        // Click the fold arrow at the left of the first row.
        let at = egui::pos2(264.0, 909.0);
        h.event(egui::Event::PointerMoved(at));
        h.run_steps(1);
        h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() });
        h.run_steps(1);
        h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() });
        h.run_steps(3);
        assert_eq!(h.state().tl_open.len(), 1, "the group unfolded");
        h.state_mut().tl_open.clear();
        h.run_steps(2);
        // Dragging the group's bar moves every member's start together.
        let timings = |h: &egui_kittest::Harness<'static, App>| h.state().project.as_ref().unwrap().slides[0].layers[0].objects.iter().map(|o| o.timing).collect::<Vec<_>>();
        assert!(timings(&h).iter().all(|t| t.is_none()));
        let (from, to) = (egui::pos2(700.0, 909.0), egui::pos2(800.0, 909.0));
        h.event(egui::Event::PointerMoved(from));
        h.run_steps(1);
        h.event(egui::Event::PointerButton { pos: from, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() });
        h.run_steps(1);
        h.event(egui::Event::PointerMoved(to));
        h.run_steps(2);
        h.event(egui::Event::PointerButton { pos: to, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() });
        h.run_steps(2);
        let after = timings(&h);
        assert!(after.iter().all(|t| t.is_some_and(|t| t.start_ms > 300)), "every member started later: {after:?}");
        assert!(after.windows(2).all(|w| w[0] == w[1]), "all by the same amount");
        h.state_mut().undo();
        h.run_steps(2);
        assert!(timings(&h).iter().all(|t| t.is_none()), "one undo step puts it back");
        // Renaming the group.
        let gid = h.state().project.as_ref().unwrap().slides[0].layers[0].objects[0].group.clone().unwrap();
        h.state_mut().group_edit = Some(gid.clone());
        h.run_steps(3);
        h.event(egui::Event::Key { key: egui::Key::A, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::COMMAND });
        h.event(egui::Event::Text("Hero set".into()));
        h.run_steps(3);
        assert_eq!(h.state().project.as_ref().unwrap().slides[0].layers[0].group_names.get(&gid).map(String::as_str), Some("Hero set"));
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("9a-timeline-group-open.png")).unwrap();
        }
    }

    #[test]
    fn tables_are_inserted_and_edited_row_by_row() {
        let mut a = app();
        a.insert_table(3, 2);
        let objs = |a: &App| a.project.as_ref().unwrap().slides[0].layers[0].objects.clone();
        assert_eq!(objs(&a).len(), 6);
        assert_eq!(a.selection().len(), 6, "choosing a cell chooses the table");
        // Type into a body cell, then add a row below it and a column to its right.
        let k = objs(&a).iter().position(|o| o.table == Some((1, 0))).unwrap();
        a.project.as_mut().unwrap().slides[0].layers[0].objects[k].text = "hello".into();
        a.selected = Some((0, k));
        a.table_edit(TableOp::RowBelow);
        assert_eq!(objs(&a).len(), 8);
        a.table_edit(TableOp::ColRight);
        assert_eq!(objs(&a).len(), 12);
        assert_eq!(brainbend_doc::tables::dims(&objs(&a), objs(&a)[0].group.as_deref().unwrap()), (4, 3));
        a.table_edit(TableOp::DeleteRow);
        assert_eq!(objs(&a).len(), 9);
        assert!(objs(&a).iter().all(|o| o.text != "hello"), "the row with the text was removed");
        a.undo();
        assert_eq!(objs(&a).len(), 12);
        // Deleting works on the whole table at once.
        a.selected = Some((0, 0));
        a.multi.clear();
        a.delete_selected();
        assert!(objs(&a).is_empty());
    }

    #[test]
    fn gui_table_renders() {
        let mut a = app();
        a.insert_table(4, 3);
        let mut h = harness(a);
        h.run_steps(3);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("9b-table.png")).unwrap();
        }
    }

    #[test]
    fn tables_merge_split_and_restyle() {
        let mut a = app();
        a.insert_table(3, 3);
        let objs = |a: &App| a.project.as_ref().unwrap().slides[0].layers[0].objects.clone();
        let g = objs(&a)[0].group.clone().unwrap();
        let k = objs(&a).iter().position(|o| o.table == Some((1, 0))).unwrap();
        a.selected = Some((0, k));
        a.range_end = Some((g.clone(), (2, 1)));
        assert_eq!(a.cell_block().unwrap().1, (1, 0, 2, 1));
        a.merge_cells();
        assert_eq!(objs(&a).len(), 9 - 3);
        let m = objs(&a).into_iter().find(|o| o.table == Some((1, 0))).unwrap();
        assert_eq!(m.table_span, Some((2, 2)));
        assert_eq!(a.selected_object().unwrap().table, Some((1, 0)), "the merged cell is selected");
        a.restyle_table(brainbend_doc::tables::TableStyle::Dark, Default::default());
        assert_eq!(objs(&a).iter().find(|o| o.table == Some((1, 0))).unwrap().style.color, Rgba(255, 255, 255, 255));
        a.split_cell();
        assert_eq!(objs(&a).len(), 9);
        a.undo();
        a.undo();
        assert_eq!(objs(&a).iter().find(|o| o.table == Some((1, 0))).unwrap().table_span, Some((2, 2)), "split undone");
    }

    #[test]
    fn table_formatting_targets_the_chosen_cells_and_borders_can_be_set() {
        let mut a = app();
        a.insert_table(3, 3);
        let objs = |a: &App| a.project.as_ref().unwrap().slides[0].layers[0].objects.clone();
        let k = objs(&a).iter().position(|o| o.table == Some((1, 1))).unwrap();
        a.selected = Some((0, k));
        assert_eq!(a.format_targets().len(), 1, "a clicked cell is formatted alone");
        let cols = objs(&a).iter().filter(|o| o.table.is_some_and(|t| t.1 == 1)).count();
        a.select_cells((0, 1), (2, 1));
        assert_eq!(a.format_targets().len(), cols, "Column chooses the whole column");
        a.select_cells((1, 0), (1, 2));
        assert_eq!(a.format_targets().len(), 3, "Row chooses the whole row");
        a.select_cells((0, 0), (2, 2));
        assert_eq!(a.format_targets().len(), 9);
        a.select_cells((1, 0), (1, 2));
        let red = Some(Stroke { color: Rgba(200, 0, 0, 255), width: 5.0 });
        a.set_cell_borders(brainbend_doc::tables::BorderSet::Bottom, red);
        let o = objs(&a);
        let bottom = |r: u16, c: u16| brainbend_doc::tables::sides(o.iter().find(|x| x.table == Some((r, c))).unwrap())[3];
        assert_eq!((bottom(1, 0), bottom(1, 2)), (red, red));
        assert_ne!(bottom(0, 0), red);
        a.undo();
        let o = objs(&a);
        assert!(o.iter().all(|x| x.table_borders.is_none()), "border change is one undo step");
    }

    #[test]
    fn gui_new_question_types_render() {
        let mut a = app();
        a.add_slide(SlideKind::Question(Question::new_categorize()));
        let mut h = harness(a);
        h.run_steps(3);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            let dir = std::path::PathBuf::from(dir);
            h.render().expect("renderer").save(dir.join("9f-question-sort.png")).unwrap();
            h.state_mut().add_slide(SlideKind::Question(Question::new_drag_words()));
            h.run_steps(3);
            h.render().expect("renderer").save(dir.join("9g-question-drag.png")).unwrap();
            for (n, q) in [("9j-likert", Question::new_likert()), ("9k-dictation", Question::new_dictation()), ("9l-label", Question::new_label_picture()), ("9m-dropdowns", Question::new_dropdowns())] {
                h.state_mut().add_slide(SlideKind::Question(q));
                h.run_steps(3);
                h.render().expect("renderer").save(dir.join(format!("{n}.png"))).unwrap();
            }
        }
    }

    #[test]
    fn gui_hotspot_areas_are_drawn_on_the_picture_preview() {
        let mut a = app();
        let img = image::RgbaImage::from_pixel(200, 100, image::Rgba([120, 170, 220, 255]));
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(img).write_to(&mut png, image::ImageFormat::Png).unwrap();
        a.project.as_mut().unwrap().assets.insert("pic".into(), Asset { name: "p.png".into(), mime: "image/png".into(), bytes: png.into_inner(), file: None });
        let mut q = Question::new_hotspot();
        q.style = QuestionStyle::Hotspot { asset: Some("pic".into()), alt: String::new(), regions: vec![] };
        a.add_slide(SlideKind::Question(q));
        let mut h = harness(a);
        h.run_steps(4);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("9h-hotspot-empty.png")).unwrap();
        }
        let regions = |h: &egui_kittest::Harness<'_, App>| match &h.state().project.as_ref().unwrap().slides[h.state().slide].kind {
            SlideKind::Question(Question { style: QuestionStyle::Hotspot { regions, .. }, .. }) => regions.clone(),
            _ => vec![],
        };
        assert!(regions(&h).is_empty());
        let (from, to) = (egui::pos2(480.0, 470.0), egui::pos2(640.0, 540.0));
        h.event(egui::Event::PointerMoved(from));
        h.run_steps(1);
        h.event(egui::Event::PointerButton { pos: from, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() });
        h.run_steps(1);
        h.event(egui::Event::PointerMoved(to));
        h.run_steps(2);
        h.event(egui::Event::PointerButton { pos: to, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() });
        h.run_steps(2);
        assert_eq!(regions(&h).len(), 1, "dragging on the picture adds an area");
        let r = regions(&h)[0];
        assert!(r.w > 0.05 && r.h > 0.05 && r.x >= 0.0 && r.x + r.w <= 1.0, "{r:?}");
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("9i-hotspot-area.png")).unwrap();
        }
    }

    #[test]
    fn gui_motion_path_points_can_be_dragged_on_the_slide() {
        let mut a = app();
        let mut o = Object::new("mover", "Mover", Kind::Rect, 860.0, 480.0, 200.0, 120.0);
        o.fill = Some(Rgba(3, 83, 158, 255));
        o.motion = Some(MotionPath::preset(PathPreset::Arc));
        o.emphasis = Some(Emphasis { kind: EmphasisKind::Pulse, delay_ms: 0, duration_ms: 800, repeat: 1 });
        a.insert(o);
        let mut h = harness(a);
        h.state_mut().ribbon_tab = 5;
        h.run_steps(4);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("9n-motion-path.png")).unwrap();
        }
        let point = |h: &egui_kittest::Harness<'_, App>| h.state().selected_object().unwrap().motion.as_ref().unwrap().points[1];
        let before = point(&h);
        let (from, to) = (egui::pos2(720.0, 364.0), egui::pos2(760.0, 404.0)); // path point 1 in the 9n screenshot
        h.event(egui::Event::PointerMoved(from));
        h.run_steps(1);
        h.event(egui::Event::PointerButton { pos: from, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() });
        h.run_steps(1);
        h.event(egui::Event::PointerMoved(from + egui::vec2(7.0, 0.0))); // a real mouse moves a little before the drag begins
        h.run_steps(1);
        h.event(egui::Event::PointerMoved(to));
        h.run_steps(2);
        h.event(egui::Event::PointerButton { pos: to, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() });
        h.run_steps(2);
        let after = point(&h);
        assert!((after.0 - before.0).abs() > 40.0 && (after.1 - before.1).abs() > 40.0, "{before:?} -> {after:?}");
        let o = h.state().selected_object().unwrap();
        let (ox, oy) = (o.x, o.y);
        assert!(ox > 0.0 && oy > 0.0);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("9o-motion-path-dragged.png")).unwrap();
        }
    }

    /// The centres of the orange crop handles in a rendered frame, as screen points.
    fn orange_handles(img: &image::RgbaImage) -> Vec<egui::Pos2> {
        let near = |p: &image::Rgba<u8>| (p[0] as i32 - 226).abs() < 12 && (p[1] as i32 - 91).abs() < 12 && (p[2] as i32 - 28).abs() < 12;
        let mut seen = vec![false; (img.width() * img.height()) as usize];
        let mut out = vec![];
        for y in 0..img.height() {
            for x in 0..img.width() {
                let i = (y * img.width() + x) as usize;
                if seen[i] || !near(img.get_pixel(x, y)) { continue; }
                let (mut stack, mut sum, mut n) = (vec![(x, y)], (0.0f32, 0.0f32), 0.0f32);
                seen[i] = true;
                while let Some((cx, cy)) = stack.pop() {
                    sum = (sum.0 + cx as f32, sum.1 + cy as f32);
                    n += 1.0;
                    for (nx, ny) in [(cx.wrapping_sub(1), cy), (cx + 1, cy), (cx, cy.wrapping_sub(1)), (cx, cy + 1)] {
                        if nx < img.width() && ny < img.height() {
                            let j = (ny * img.width() + nx) as usize;
                            if !seen[j] && near(img.get_pixel(nx, ny)) { seen[j] = true; stack.push((nx, ny)); }
                        }
                    }
                }
                let c = egui::pos2(sum.0 / n, sum.1 / n);
                // Only the canvas (not the orange Publish button or the ribbon icons), one point per handle.
                if n >= 6.0 && (240.0..1170.0).contains(&c.x) && (180.0..720.0).contains(&c.y) && !out.iter().any(|o: &egui::Pos2| o.distance(c) < 14.0) { out.push(c); }
            }
        }
        out
    }

    fn drag(h: &mut egui_kittest::Harness<'static, App>, from: egui::Pos2, to: egui::Pos2) {
        h.event(egui::Event::PointerMoved(from));
        h.run_steps(1);
        h.event(egui::Event::PointerButton { pos: from, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() });
        h.run_steps(1);
        h.event(egui::Event::PointerMoved(from + egui::vec2(7.0, 0.0)));
        h.run_steps(1);
        h.event(egui::Event::PointerMoved(to));
        h.run_steps(2);
        h.event(egui::Event::PointerButton { pos: to, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() });
        h.run_steps(2);
    }

    #[test]
    fn gui_crop_mode_crops_by_dragging_edges_and_pans_the_picture() {
        let mut a = app();
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_fn(400, 200, |x, _| if x < 200 { image::Rgba([200, 60, 60, 255]) } else { image::Rgba([60, 60, 200, 255]) }))
            .write_to(&mut png, image::ImageFormat::Png).unwrap();
        a.project.as_mut().unwrap().assets.insert("pic".into(), Asset { name: "p.png".into(), mime: "image/png".into(), bytes: png.into_inner(), file: None });
        let mut o = Object::new("photo", "Photo", Kind::Image, 0.0, 0.0, 600.0, 300.0);
        o.asset = Some("pic".into());
        a.insert(o);
        a.selected = Some((0, 0));
        a.crop_mode = Some("photo".into());
        let mut h = harness(a);
        h.run_steps(3);
        let state = |h: &egui_kittest::Harness<'_, App>| h.state().selected_object().unwrap().clone();
        let before = state(&h);
        let whole = crop::image_rect(&before);

        // The handle on the left edge, found by its colour in the rendered frame.
        let img = h.render().expect("renderer");
        let handles = orange_handles(&img);
        assert_eq!(handles.len(), 8, "eight crop handles: {handles:?}");
        let mut by_x = handles.clone();
        by_x.sort_by(|a, b| a.x.total_cmp(&b.x));
        let mut left = by_x[..3].to_vec();
        left.sort_by(|a, b| a.y.total_cmp(&b.y));
        let mid_left = left[1];

        drag(&mut h, mid_left, mid_left + egui::vec2(80.0, 0.0));
        let after = state(&h);
        let c = after.crop.expect("dragging the edge crops");
        assert!(c[0] > 0.05 && c[1] == 0.0 && c[2] == 0.0 && c[3] == 0.0, "{c:?}");
        assert!(after.w < before.w && (after.h - before.h).abs() < 0.01 && after.x > before.x, "{before:?} -> {after:?}");
        let now = crop::image_rect(&after);
        assert!((now.0 - whole.0).abs() < 0.1 && (now.2 - whole.2).abs() < 0.1, "the picture itself did not move: {whole:?} -> {now:?}");
        assert_eq!(h.state().crop_mode.as_deref(), Some("photo"), "still in crop mode");

        // Dragging inside the frame slides the picture under it.
        let centre = egui::pos2((by_x[0].x + by_x[7].x) / 2.0 + 40.0, (by_x[0].y + by_x[7].y) / 2.0);
        let before_pan = state(&h).crop.unwrap();
        // The picture's right edge is flush with the frame, so it cannot slide left; sliding it right works.
        drag(&mut h, centre, centre + egui::vec2(-30.0, 0.0));
        assert_eq!(state(&h).crop.unwrap(), before_pan, "the picture cannot be pulled off its frame");
        drag(&mut h, centre, centre + egui::vec2(30.0, 0.0));
        let after_pan = state(&h);
        let c = after_pan.crop.unwrap();
        assert!(c[0] < before_pan[0] && c[2] > before_pan[2], "the picture moved right under the frame: {before_pan:?} -> {c:?}");
        assert!((after_pan.w - after.w).abs() < 0.01, "the frame did not change");
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("9p-crop-dragged.png")).unwrap();
        }
        h.key_press(egui::Key::Escape);
        h.run_steps(2);
        assert_eq!(h.state().crop_mode, None, "Escape finishes cropping");
        // Reset shows the whole picture again, where it was.
        let framed = state(&h);
        let picture = crop::image_rect(&framed);
        h.state_mut().reset_crop();
        h.run_steps(2);
        let reset = state(&h);
        assert_eq!(reset.crop, None);
        assert!((reset.x - picture.0).abs() < 0.1 && (reset.w - (picture.2 - picture.0)).abs() < 0.1 && (reset.h - (picture.3 - picture.1)).abs() < 0.1, "{picture:?} -> {reset:?}");
        // Undo takes the reset back, and the pan before it.
        h.state_mut().undo();
        h.run_steps(2);
        assert_eq!(h.state().project.as_ref().unwrap().slides[0].layers[0].objects[0].crop.is_some(), framed.crop.is_some(), "undo brings the crop back");

    }

    #[test]
    fn gui_a_check_box_is_inserted_drawn_with_its_box_and_edited_in_the_panel() {
        let mut a = app();
        a.insert_check_look(0);
        a.selected = Some((0, 0));
        let o = &a.project.as_ref().unwrap().slides[0].layers[0].objects[0];
        assert!(o.check.is_some() && o.text == "Check box" && o.margins[0] > 0.0, "a labelled check box with room for the box");
        let mut h = harness(a);
        h.run_steps(4);
        let shot = h.render().expect("renderer");
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") { shot.save(std::path::PathBuf::from(dir).join("9r-checkbox.png")).unwrap(); }
        // Ticked at the start draws a tick inside the box, which changes some pixels and no others near it.
        h.state_mut().project.as_mut().unwrap().slides[0].layers[0].objects[0].initial_state = "Selected".into();
        h.run_steps(3);
        let after = h.render().expect("renderer");
        let changed = shot.pixels().zip(after.pixels()).filter(|(a, b)| a != b).count();
        assert!((20..5000).contains(&changed), "only the tick changed: {changed} pixels");
        h.state_mut().undo();
        assert!(h.state().project.as_ref().unwrap().slides[0].layers[0].objects.is_empty(), "undo removes it");
    }

    #[test]
    fn gui_an_outlined_hotspot_is_drawn_as_its_outline_not_its_box() {
        let mut a = app();
        let mut spot = blank_object("spot".into(), "Hotspot", Kind::Hotspot, 100.0, 100.0, 600.0, 400.0);
        spot.hit_area = Some(vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)]);
        a.insert(spot);
        a.selected = None;
        let mut h = harness(a);
        h.run_steps(3);
        let outlined = h.render().expect("renderer");
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") { outlined.save(std::path::PathBuf::from(dir).join("9s-hotspot-outline.png")).unwrap(); }
        h.state_mut().project.as_mut().unwrap().slides[0].layers[0].objects[0].hit_area = None;
        h.run_steps(3);
        let boxed = h.render().expect("renderer");
        let changed = outlined.pixels().zip(boxed.pixels()).filter(|(a, b)| a != b).count();
        assert!(changed > 2000, "the half of the box outside the triangle is no longer tinted: {changed} pixels differ");
    }

    fn press(h: &mut egui_kittest::Harness<'static, App>, key: egui::Key) {
        h.event(egui::Event::Key { key, physical_key: Some(key), pressed: true, repeat: false, modifiers: Default::default() });
        h.run_steps(2);
        h.event(egui::Event::Key { key, physical_key: Some(key), pressed: false, repeat: false, modifiers: Default::default() });
        h.run_steps(2);
    }

    fn click_at(h: &mut egui_kittest::Harness<'static, App>, at: egui::Pos2) {
        h.event(egui::Event::PointerMoved(at));
        h.run_steps(1);
        h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() });
        h.run_steps(1);
        h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() });
        h.run_steps(2);
    }

    fn spot_of(h: &egui_kittest::Harness<'static, App>) -> Object {
        h.state().project.as_ref().unwrap().slides[0].layers[0].objects.iter().find(|o| matches!(o.kind, Kind::Hotspot)).cloned().expect("a hotspot")
    }

    #[test]
    fn gui_crop_shape_holds_the_frame_to_a_ratio_and_the_arrow_keys_slide_the_picture() {
        let mut a = app();
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_fn(400, 200, |x, _| if x < 200 { image::Rgba([200, 60, 60, 255]) } else { image::Rgba([60, 60, 200, 255]) }))
            .write_to(&mut png, image::ImageFormat::Png).unwrap();
        a.project.as_mut().unwrap().assets.insert("pic".into(), Asset { name: "p.png".into(), mime: "image/png".into(), bytes: png.into_inner(), file: None });
        let mut o = Object::new("photo", "Photo", Kind::Image, 0.0, 0.0, 800.0, 400.0);
        o.asset = Some("pic".into());
        a.insert(o);
        a.selected = Some((0, 0));
        a.crop_mode = Some("photo".into());
        assert_eq!(a.picture_ratio("photo"), Some(2.0), "the source picture is twice as wide as tall");
        a.set_crop_aspect(Some(1.0));
        let sq = a.project.as_ref().unwrap().slides[0].layers[0].objects[0].clone();
        assert!((sq.w - sq.h).abs() < 0.01 && (sq.w - 400.0).abs() < 0.01, "closed in to a square: {} x {}", sq.w, sq.h);
        let mut h = harness(a);
        h.run_steps(3);
        let img = h.render().expect("renderer");
        let handles = orange_handles(&img);
        assert_eq!(handles.len(), 8, "{handles:?}");
        // Drag the bottom-right corner: the frame stays square.
        let br = *handles.iter().max_by(|a, b| (a.x + a.y).total_cmp(&(b.x + b.y))).unwrap();
        drag(&mut h, br, br + egui::vec2(-60.0, -10.0));
        let o = h.state().project.as_ref().unwrap().slides[0].layers[0].objects[0].clone();
        assert!((o.w - o.h).abs() < 0.5 && o.w < 399.0, "still square after the drag: {} x {}", o.w, o.h);
        // Arrow keys slide the picture under the frame; the frame itself stays.
        let before = o.crop.unwrap();
        press(&mut h, egui::Key::ArrowRight);
        let after = h.state().project.as_ref().unwrap().slides[0].layers[0].objects[0].clone();
        assert_eq!((after.x, after.y, after.w, after.h), (o.x, o.y, o.w, o.h), "the frame did not move");
        let c = after.crop.unwrap();
        assert!(c[0] < before[0] || c[2] > before[2], "the picture moved: {before:?} -> {c:?}");
        h.state_mut().undo();
        assert_eq!(h.state().project.as_ref().unwrap().slides[0].layers[0].objects[0].crop.unwrap(), before, "one undo step per key press");
        // Free lets go of the ratio.
        h.state_mut().set_crop_aspect(None);
        assert_eq!(h.state().crop_aspect, None);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") { h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("9v-crop-square.png")).unwrap(); }
    }

    #[test]
    fn gui_a_picture_cropped_to_a_shape_shows_only_what_is_inside_it_and_undo_restores_it() {
        let mut a = app();
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(100, 100, image::Rgba([200, 30, 30, 255]))).write_to(&mut png, image::ImageFormat::Png).unwrap();
        a.project.as_mut().unwrap().assets.insert("pic".into(), Asset { name: "p.png".into(), mime: "image/png".into(), bytes: png.into_inner(), file: None });
        let mut o = Object::new("photo", "Photo", Kind::Image, 0.0, 0.0, 700.0, 500.0);
        o.asset = Some("pic".into());
        a.insert(o);
        a.selected = None;
        let mut h = harness(a);
        h.run_steps(3);
        let boxed = h.render().expect("renderer");
        let red = |img: &image::RgbaImage| img.pixels().filter(|p| p[0] > 180 && p[1] < 60 && p[2] < 60).count();
        for shape in PictureShape::ALL {
            h.state_mut().selected = Some((0, 0));
            h.state_mut().set_picture_shape(Some(shape));
            h.state_mut().selected = None;
            h.run_steps(3);
            let shaped = h.render().expect("renderer");
            let (full, part) = (red(&boxed), red(&shaped));
            assert!(part > full / 5 && part < full * 995 / 1000, "{shape:?}: {part} of {full} red pixels are left");
            if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") { shaped.save(std::path::PathBuf::from(dir).join(format!("9w-shape-{:?}.png", shape))).unwrap(); }
        }
        h.state_mut().selected = Some((0, 0));
        h.state_mut().set_picture_shape(None);
        h.state_mut().selected = None;
        h.run_steps(3);
        assert_eq!(red(&h.render().expect("renderer")), red(&boxed), "no shape: the whole picture again");
        h.state_mut().undo();
        assert_eq!(h.state().project.as_ref().unwrap().slides[0].layers[0].objects[0].mask, Some(PictureShape::Star), "undo brings the last shape back");
    }

    #[test]
    fn gui_outline_mode_moves_corners_adds_one_on_an_edge_and_deletes_the_chosen_one() {
        let mut a = app();
        a.insert(blank_object("spot".into(), "Hotspot", Kind::Hotspot, 100.0, 100.0, 600.0, 400.0));
        a.selected = Some((0, 0));
        a.begin_outline();
        assert_eq!(a.outline_mode.as_deref(), Some("spot"));
        assert_eq!(a.project.as_ref().unwrap().slides[0].layers[0].objects[0].hit_area.as_ref().map(|a| a.len()), Some(4), "the box's four corners to start from");
        let mut h = harness(a);
        h.run_steps(3);
        let img = h.render().expect("renderer");
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") { img.save(std::path::PathBuf::from(dir).join("9t-outline-mode.png")).unwrap(); }
        let handles = orange_handles(&img);
        assert_eq!(handles.len(), 4, "a handle on each corner: {handles:?}");
        // Drag the bottom-right corner up and in.
        let br = *handles.iter().max_by(|a, b| (a.x + a.y).total_cmp(&(b.x + b.y))).unwrap();
        drag(&mut h, br, br + egui::vec2(-120.0, -90.0));
        let area = spot_of(&h).hit_area.unwrap();
        assert_eq!(area.len(), 4);
        let moved = area.iter().filter(|p| p.0 > 0.05 && p.0 < 0.95 && p.1 > 0.05 && p.1 < 0.95).count();
        assert_eq!(moved, 1, "one corner moved in from the bottom-right: {area:?}");
        assert!(h.state().outline_sel.is_some(), "and it is the chosen corner");
        // Click the middle of the top edge: a corner is added there, and dragging moves it.
        let tl = *handles.iter().min_by(|a, b| (a.x + a.y).total_cmp(&(b.x + b.y))).unwrap();
        let top_right = *handles.iter().filter(|p| (p.y - tl.y).abs() < 3.0 && p.x > tl.x + 20.0).next().unwrap();
        let mid = egui::pos2((tl.x + top_right.x) / 2.0, tl.y);
        drag(&mut h, mid, mid + egui::vec2(0.0, 70.0));
        let area = spot_of(&h).hit_area.unwrap();
        assert_eq!(area.len(), 5, "a fifth corner: {area:?}");
        assert!(area.iter().any(|p| (p.0 - 0.5).abs() < 0.05 && p.1 > 0.1), "pulled down from the top edge: {area:?}");
        h.run_steps(2);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") { h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("9t-outline-edited.png")).unwrap(); }
        // Delete removes the chosen (new) corner, not the hotspot.
        press(&mut h, egui::Key::Delete);
        assert_eq!(spot_of(&h).hit_area.unwrap().len(), 4);
        assert!(h.state().outline_mode.is_some());
        // Undo brings the corner back; undo again takes the edge drag away.
        h.state_mut().undo();
        assert_eq!(spot_of(&h).hit_area.unwrap().len(), 5);
        // The arrow keys move the chosen corner.
        let before = spot_of(&h).hit_area.unwrap()[h.state().outline_sel.unwrap_or(0).min(4)];
        h.state_mut().outline_sel = Some(2);
        let p2 = spot_of(&h).hit_area.unwrap()[2];
        press(&mut h, egui::Key::ArrowLeft);
        let after = spot_of(&h).hit_area.unwrap()[2];
        assert!(after.0 < p2.0, "moved left: {p2:?} -> {after:?} ({before:?})");
        // Escape finishes; the outline stays because it is no longer the box.
        press(&mut h, egui::Key::Escape);
        assert_eq!(h.state().outline_mode, None);
        assert!(spot_of(&h).hit_area.is_some());
    }

    #[test]
    fn gui_an_outline_that_is_still_the_box_is_dropped_when_editing_ends() {
        let mut a = app();
        a.insert(blank_object("spot".into(), "Hotspot", Kind::Hotspot, 100.0, 100.0, 600.0, 400.0));
        a.selected = Some((0, 0));
        a.begin_outline();
        let mut h = harness(a);
        h.run_steps(3);
        press(&mut h, egui::Key::Enter);
        assert_eq!(h.state().outline_mode, None);
        assert_eq!(spot_of(&h).hit_area, None, "nothing was changed, so there is no outline");
    }

    #[test]
    fn gui_a_freeform_hotspot_is_drawn_by_clicking_its_corners() {
        let mut a = app();
        a.draw_outline = Some(vec![]);
        a.draw_slide = 0;
        let mut h = harness(a);
        h.run_steps(3);
        // Canvas clicks well inside the slide area.
        let pts = [egui::pos2(500.0, 300.0), egui::pos2(800.0, 320.0), egui::pos2(760.0, 520.0), egui::pos2(520.0, 480.0)];
        for p in pts { click_at(&mut h, p); }
        assert_eq!(h.state().draw_outline.as_ref().map(|p| p.len()), Some(4), "four corners so far");
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") { h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("9u-drawing-outline.png")).unwrap(); }
        press(&mut h, egui::Key::Backspace);
        assert_eq!(h.state().draw_outline.as_ref().map(|p| p.len()), Some(3), "Backspace takes the last corner back");
        click_at(&mut h, pts[3]);
        press(&mut h, egui::Key::Enter);
        assert_eq!(h.state().draw_outline, None);
        let spot = spot_of(&h);
        let area = spot.hit_area.clone().expect("an outline");
        assert_eq!(area.len(), 4);
        assert!(area.iter().any(|p| p.0 == 0.0) && area.iter().any(|p| p.0 == 1.0) && area.iter().any(|p| p.1 == 0.0) && area.iter().any(|p| p.1 == 1.0), "the frame fits the corners: {area:?}");
        assert_eq!(h.state().selected, Some((0, 0)));
        // Undo takes the hotspot away again.
        h.state_mut().undo();
        assert!(h.state().project.as_ref().unwrap().slides[0].layers[0].objects.is_empty());
    }

    #[test]
    fn gui_escape_cancels_a_freeform_hotspot_and_too_few_corners_make_none() {
        let mut a = app();
        a.draw_outline = Some(vec![]);
        let mut h = harness(a);
        h.run_steps(3);
        click_at(&mut h, egui::pos2(500.0, 300.0));
        click_at(&mut h, egui::pos2(800.0, 320.0));
        press(&mut h, egui::Key::Enter);
        assert_eq!(h.state().draw_outline, None);
        assert!(h.state().project.as_ref().unwrap().slides[0].layers[0].objects.is_empty(), "two corners make no hotspot");
        assert!(h.state().status.contains("at least three"), "{}", h.state().status);
        h.state_mut().draw_outline = Some(vec![]);
        h.run_steps(2);
        click_at(&mut h, egui::pos2(500.0, 300.0));
        press(&mut h, egui::Key::Escape);
        assert_eq!(h.state().draw_outline, None);
    }

    #[test]
    fn an_outlined_hotspot_is_picked_only_inside_its_outline() {
        let mut under = blank_object("under".into(), "Under", Kind::Rect, 0.0, 0.0, 400.0, 400.0);
        under.fill = Some(Rgba(255, 0, 0, 255));
        let mut spot = blank_object("spot".into(), "Hotspot", Kind::Hotspot, 0.0, 0.0, 400.0, 400.0);
        spot.hit_area = Some(vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)]);
        let mut slide = Slide::new("s", "S", 1920.0, 1080.0);
        slide.layers[0].objects = vec![under, spot];
        assert_eq!(hit_object(&slide, &[0], egui::vec2(100.0, 100.0), 1.0), Some((0, 1)), "inside the triangle: the hotspot");
        assert_eq!(hit_object(&slide, &[0], egui::vec2(350.0, 350.0), 1.0), Some((0, 0)), "in its box but outside the triangle: what is under it");
    }

    #[test]
    fn inserting_an_oval_or_rectangle_hotspot_and_resetting_or_fitting_its_outline() {
        let mut a = app();
        a.insert_hotspot(true);
        let o = &a.project.as_ref().unwrap().slides[0].layers[0].objects[0];
        assert!(matches!(o.kind, Kind::Hotspot) && o.hit_area.as_ref().map(|a| a.len()) == Some(48));
        a.insert_hotspot(false);
        assert_eq!(a.project.as_ref().unwrap().slides[0].layers[0].objects[1].hit_area, None);
        a.selected = Some((0, 0));
        a.reset_outline();
        assert_eq!(a.project.as_ref().unwrap().slides[0].layers[0].objects[0].hit_area, None, "the oval is a box again");
        a.undo();
        assert!(a.project.as_ref().unwrap().slides[0].layers[0].objects[0].hit_area.is_some(), "undo brings the oval back");
        a.project.as_mut().unwrap().slides[0].layers[0].objects[0].hit_area = Some(vec![(0.25, 0.25), (0.75, 0.25), (0.5, 0.75)]);
        let w0 = a.project.as_ref().unwrap().slides[0].layers[0].objects[0].w;
        a.selected = Some((0, 0));
        a.fit_outline();
        let o = &a.project.as_ref().unwrap().slides[0].layers[0].objects[0];
        assert!((o.w - w0 / 2.0).abs() < 0.01, "the frame shrank to the outline");
    }

    #[test]
    fn gui_a_slider_is_inserted_with_its_variable_in_one_undo_step_and_is_edited_in_the_panel() {
        let mut a = app();
        a.insert_slider();
        let p = a.project.as_ref().unwrap();
        assert_eq!(p.variables.iter().filter(|v| v.name == "Slider1").count(), 1, "a variable of its own");
        let Kind::Slider(sl) = &p.slides[0].layers[0].objects[0].kind else { panic!("a slider") };
        assert!(p.variables.iter().any(|v| v.id == sl.variable && matches!(v.initial, Value::Num(_))), "linked to it");
        a.insert_slider();
        assert!(a.project.as_ref().unwrap().variables.iter().any(|v| v.name == "Slider2"), "the next one gets a fresh name");
        a.selected = Some((0, 0));
        let mut h = harness(a);
        h.run_steps(4);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            h.render().expect("renderer").save(std::path::PathBuf::from(dir).join("9q-slider.png")).unwrap();
        }
        h.state_mut().undo();
        let p = h.state().project.as_ref().unwrap();
        assert_eq!(p.slides[0].layers[0].objects.len(), 1, "undo removed the second slider");
        assert!(!p.variables.iter().any(|v| v.name == "Slider2"), "and its variable with it");
        assert!(p.variables.iter().any(|v| v.name == "Slider1"));
    }

    #[test]
    fn gui_table_panel_and_merged_cells_render() {
        let mut a = app();
        a.insert_table(4, 3);
        let g = a.project.as_ref().unwrap().slides[0].layers[0].objects[0].group.clone().unwrap();
        let k = a.project.as_ref().unwrap().slides[0].layers[0].objects.iter().position(|o| o.table == Some((1, 0))).unwrap();
        a.selected = Some((0, k));
        a.range_end = Some((g, (2, 1)));
        let mut h = harness(a);
        h.run_steps(3);
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            let dir = std::path::PathBuf::from(dir);
            h.render().expect("renderer").save(dir.join("9c-table-block.png")).unwrap();
            h.state_mut().set_cell_borders(brainbend_doc::tables::BorderSet::Outside, Some(Stroke { color: Rgba(220, 40, 40, 255), width: 8.0 }));
            h.run_steps(8);
            h.render().expect("renderer").save(dir.join("9c2-table-borders.png")).unwrap();
            h.state_mut().merge_cells();
            h.run_steps(3);
            h.render().expect("renderer").save(dir.join("9d-table-merged.png")).unwrap();
        }
    }

    #[test]
    fn gui_dragging_a_table_border_resizes_a_column() {
        let mut a = app();
        a.insert_table(3, 3);
        let mut h = harness(a);
        h.run_steps(3);
        let widths = |h: &egui_kittest::Harness<'_, App>| {
            let o = &h.state().project.as_ref().unwrap().slides[0].layers[0].objects;
            (0..3).map(|c| o.iter().find(|x| x.table == Some((0, c))).unwrap().w).collect::<Vec<_>>()
        };
        let before = widths(&h);
        // The first column border sits near x=626 in the 1500x950 window (see the 9b-table screenshot).
        let mut moved = false;
        'scan: for y in [456, 396] {
            for x in 622..632 {
                let (from, to) = (egui::pos2(x as f32, y as f32), egui::pos2(x as f32 + 60.0, y as f32));
                h.event(egui::Event::PointerMoved(from));
                h.run_steps(1);
                h.event(egui::Event::PointerButton { pos: from, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() });
                h.run_steps(1);
                h.event(egui::Event::PointerMoved(to));
                h.run_steps(2);
                h.event(egui::Event::PointerButton { pos: to, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() });
                h.run_steps(1);
                let now = widths(&h);
                if now != before && (now[0] - before[0]).abs() > 1.0 && now.iter().sum::<f32>() - before.iter().sum::<f32>() < 0.5 {
                    moved = true;
                    break 'scan;
                }
                if now != before { break 'scan; } // dragged something else (moved the table): scan failed
            }
        }
        assert!(moved, "dragging the border changed column widths: {before:?} -> {:?}", widths(&h));
        assert!(h.state().undo.len() >= 1);
    }

    #[test]
    fn gui_master_view_and_backgrounds_render() {
        let mut a = app();
        a.add_slide_with_layout(SlideKind::Normal, Some("layout-title".into()));
        a.project.as_mut().unwrap().slides[a.slide].background = Some(Background::gradient(135.0, Rgba(3, 83, 158, 255), Rgba(232, 119, 34, 255)));
        let mut h = harness(a);
        h.run_steps(3);
        h.state_mut().enter_master_view();
        h.state_mut().slide = 2;
        h.run_steps(3);
        h.state_mut().leave_master_view();
        h.run_steps(3);
        assert!(!h.state().in_master_view());
        if let Some(dir) = std::env::var_os("BRAINBEND_SHOT_DIR") {
            let dir = std::path::PathBuf::from(dir);
            h.state_mut().enter_master_view();
            h.state_mut().slide = 2;
            h.run_steps(3);
            h.render().expect("renderer").save(dir.join("6a-master-view.png")).unwrap();
            h.state_mut().leave_master_view();
            h.state_mut().slide = 1;
            h.run_steps(3);
            h.render().expect("renderer").save(dir.join("6b-layout-slide-gradient.png")).unwrap();
        }
    }

    #[test]
    fn move_layer_reorders_and_keeps_base_first() {
        let mut a = app();
        a.add_layer();
        a.add_layer();
        a.add_layer();
        let names = |a: &App| a.project.as_ref().unwrap().slides[a.slide].layers.iter().map(|l| l.name.clone()).collect::<Vec<_>>();
        let before = names(&a);
        a.visible = vec![true, false, true, true];
        a.move_layer(1, 3);
        let after = names(&a);
        assert_eq!(after[0], before[0]);
        assert_eq!(after[3], before[1]);
        assert_eq!(after[1], before[2]);
        assert_eq!(a.visible, vec![true, true, true, false]);
        assert_eq!(a.layer, 3);
        a.move_layer(0, 2);
        assert_eq!(names(&a), after);
    }

    #[test]
    fn project_with_quiz_round_trips_through_json() {
        let mut a = app();
        a.add_slide(SlideKind::Question(Question::new_single()));
        let p = a.project.as_ref().unwrap();
        let back = Project::from_json(&p.to_json().unwrap()).unwrap();
        assert_eq!(back.slides[1].kind, p.slides[1].kind);
        assert_eq!(back.pass_mark, 80);
    }
}
