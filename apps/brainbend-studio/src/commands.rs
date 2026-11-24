//! The commands of the app, written down once.
//!
//! A command knows its name, its shortcut, whether it can run right now and whether it is switched on; running it does the same thing wherever
//! it is asked for from: a menu, a button, the keyboard. The Edit commands do the right thing for what has the keyboard (the slides when the slide
//! list or Scene View has it, the objects on the slide otherwise); the Window commands show and hide the panels and lay them out.
use crate::{dock, App};
use eframe::egui;


#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Cmd {
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Delete,
    Duplicate,
    SelectAll,
    Find,
    /// Show or hide a panel (a tab number, see `main.rs`).
    TogglePanel(usize),
    /// Lay the panels out the way a preset says.
    Preset(dock::Preset),
    New,
    Open,
    Save,
    SaveAs,
    Publish,
    Settings,
    /// The File pages: the project's name and facts, its settings, and the About page.
    ProjectInfo,
    ProjectSettings,
    About,
    Quit,
    ZoomIn,
    ZoomOut,
    ZoomFit,
    ZoomActual,
    DarkMode,
    ShowMenuBar,
    ShowInsertBar,
    Docs,
    SubmitIssue,
    Community,
    CheckForUpdates,
    /// Open one of the working tools (Variables, Focus Order, Spelling, the accessibility check, Brand Colors).
    Tool(crate::Tool),
    LayoutBuilder,
    PlayerSettings,
    /// Make every slide this size, in points.
    SlideSize(u32, u32),
}

/// The slide sizes the Tools menu offers.
pub(crate) const SLIDE_SIZES: [(u32, u32); 4] = [(1920, 1080), (1280, 720), (1440, 1080), (1024, 768)];

/// One line of a menu.
#[derive(Clone)]
pub(crate) enum Item { Do(Cmd), Gap, Sub(&'static str, Vec<Cmd>), /// The gallery of objects to add to a slide (see `App::insert_menu`).
    Gallery }

/// The menus, in order: what the menu bar (and later the macOS menu bar) shows.
pub(crate) fn menus() -> Vec<(&'static str, Vec<Item>)> {
    use Item::{Do, Gap, Sub};
    vec![
        ("File", vec![Do(Cmd::New), Do(Cmd::Open), Gap, Do(Cmd::Save), Do(Cmd::SaveAs), Gap, Do(Cmd::ProjectInfo), Do(Cmd::ProjectSettings), Do(Cmd::PlayerSettings), Do(Cmd::Publish), Gap, Do(Cmd::Settings), Gap, Do(Cmd::Quit)]),
        ("Edit", vec![Do(Cmd::Undo), Do(Cmd::Redo), Gap, Do(Cmd::Cut), Do(Cmd::Copy), Do(Cmd::Paste), Do(Cmd::Delete), Gap, Do(Cmd::Duplicate), Do(Cmd::SelectAll), Gap, Do(Cmd::Find)]),
        ("View", vec![Do(Cmd::ZoomIn), Do(Cmd::ZoomOut), Do(Cmd::ZoomFit), Do(Cmd::ZoomActual), Gap, Do(Cmd::ShowMenuBar), Do(Cmd::ShowInsertBar), Do(Cmd::DarkMode)]),
        ("Insert", vec![Item::Gallery]),
        ("Tools", vec![Do(Cmd::LayoutBuilder), Sub("Slide Size", SLIDE_SIZES.iter().map(|&(w, h)| Cmd::SlideSize(w, h)).collect()), Do(Cmd::Tool(crate::Tool::Colors)), Do(Cmd::PlayerSettings), Gap, Do(Cmd::Tool(crate::Tool::Variables)), Do(Cmd::Tool(crate::Tool::FocusOrder)), Gap, Do(Cmd::Tool(crate::Tool::Spelling)), Do(Cmd::Tool(crate::Tool::Check))]),
        ("Window", PANELS.iter().map(|&g| Do(Cmd::TogglePanel(g))).chain([Gap]).chain(dock::Preset::ALL.iter().map(|&p| Do(Cmd::Preset(p)))).collect()),
        ("Help", vec![Do(Cmd::Docs), Do(Cmd::SubmitIssue), Do(Cmd::Community), Gap, Do(Cmd::CheckForUpdates), Do(Cmd::About)]),
    ]
}

/// The Edit menu, in order (a gap between the groups goes after Redo, Delete and SelectAll).
pub(crate) const EDIT: [Cmd; 9] = [Cmd::Undo, Cmd::Redo, Cmd::Cut, Cmd::Copy, Cmd::Paste, Cmd::Delete, Cmd::Duplicate, Cmd::SelectAll, Cmd::Find];

/// The panels the Window menu switches on and off, in the order they sit on screen.
pub(crate) const PANELS: [usize; 8] = [crate::P_SLIDES, 0, crate::P_TRIGGERS, 2, crate::P_TIMELINE, crate::P_NOTES, crate::P_STATES, dock::LAYERS];

/// The Window menu: a switch for each panel, then the layouts.
pub(crate) fn window() -> Vec<Cmd> {
    PANELS.iter().map(|&g| Cmd::TogglePanel(g)).chain(dock::Preset::ALL.iter().map(|&p| Cmd::Preset(p))).collect()
}

impl Cmd {
    pub(crate) fn label(self) -> String {
        match self {
            Cmd::Undo => "Undo".into(),
            Cmd::Redo => "Redo".into(),
            Cmd::Cut => "Cut".into(),
            Cmd::Copy => "Copy".into(),
            Cmd::Paste => "Paste".into(),
            Cmd::Delete => "Delete".into(),
            Cmd::Duplicate => "Duplicate".into(),
            Cmd::SelectAll => "Select All".into(),
            Cmd::Find => "Find and Replace".into(),
            Cmd::TogglePanel(g) => App::panel_title(g).into(),
            Cmd::Preset(p) => p.name().into(),
            Cmd::New => "New Project…".into(),
            Cmd::Open => "Open…".into(),
            Cmd::Save => "Save".into(),
            Cmd::SaveAs => "Save As…".into(),
            Cmd::Publish => "Publish…".into(),
            Cmd::Settings => "Studio Settings…".into(),
            Cmd::ProjectInfo => "Project Info…".into(),
            Cmd::ProjectSettings => "Project Settings…".into(),
            Cmd::About => "About BrainBend Studio".into(),
            Cmd::Quit => if cfg!(target_os = "macos") { "Quit BrainBend Studio".into() } else { "Exit".into() },
            Cmd::ZoomIn => "Zoom In".into(),
            Cmd::ZoomOut => "Zoom Out".into(),
            Cmd::ZoomFit => "Fit to Window".into(),
            Cmd::ZoomActual => "Actual Size".into(),
            Cmd::DarkMode => "Dark Mode".into(),
            Cmd::ShowMenuBar => "Show Menu Bar".into(),
            Cmd::ShowInsertBar => "Show Insert Bar".into(),
            Cmd::Docs => "Documentation".into(),
            Cmd::SubmitIssue => "Submit an Issue…".into(),
            Cmd::Community => "BrainBend Community…".into(),
            Cmd::CheckForUpdates => "Check for Updates".into(),
            Cmd::Tool(t) => match t { crate::Tool::Check => "Accessibility Check", t => t.title() }.into(),
            Cmd::LayoutBuilder => "Layout Builder".into(),
            Cmd::PlayerSettings => "Player Settings…".into(),
            Cmd::SlideSize(w, h) => format!("{w} × {h} ({})", if w * 9 == h * 16 { "16:9" } else { "4:3" }),
        }
    }

    /// The keys that run it, as a menu shows them (⌘ on a Mac, Ctrl elsewhere).
    pub(crate) fn shortcut(self) -> Option<String> {
        let key = match self {
            Cmd::Undo => "Z",
            Cmd::Redo => "⇧Z",
            Cmd::Cut => "X",
            Cmd::Copy => "C",
            Cmd::Paste => "V",
            Cmd::Duplicate => "D",
            Cmd::SelectAll => "A",
            Cmd::Find => "F",
            Cmd::Delete => return Some(if cfg!(target_os = "macos") { "⌫".into() } else { "Del".into() }),
            Cmd::New => "N",
            Cmd::Open => "O",
            Cmd::Save => "S",
            Cmd::SaveAs => "⇧S",
            Cmd::Quit => return Some(if cfg!(target_os = "macos") { "⌘Q".into() } else { "Alt+F4".into() }),
            Cmd::Settings => return Some(if cfg!(target_os = "macos") { "⌘,".into() } else { "Ctrl+,".into() }),
            Cmd::Tool(crate::Tool::Spelling) => return Some("F7".into()),
            _ => return None,
        };
        Some(if cfg!(target_os = "macos") { format!("⌘{}", key.replace("⇧", "⇧")) } else { format!("Ctrl+{}", key.replace('⇧', "Shift+")) })
    }
}

impl App {
    /// Whether the system's menu bar (macOS) has been made.
    pub(crate) fn native_menu_active(&self) -> bool {
        #[cfg(target_os = "macos")]
        { self.native_menu.is_some() }
        #[cfg(not(target_os = "macos"))]
        { false }
    }

    /// What the menu at the top of the screen asked for since the last frame is done, and its lines are greyed out or ticked as they should be.
    #[cfg(target_os = "macos")]
    pub(crate) fn native_menu_frame(&mut self) {
        let Some(mut menu) = self.native_menu.take() else { return };
        for a in menu.clicked() {
            match a {
                crate::native_menu::Action::Cmd(c) => self.run_cmd(c),
                crate::native_menu::Action::Insert(i) => self.insert_leaf_act(i),
            }
        }
        menu.refresh(self);
        self.native_menu = Some(menu);
    }

    /// Whether the keys and the Edit menu are about slides: the slide list or the Scene View is what was last clicked.
    pub(crate) fn editing_slides(&self) -> bool { self.scene_view || self.slides_focus }

    /// Whether a command can run now (a menu greys it out when not).
    pub(crate) fn cmd_enabled(&self, c: Cmd) -> bool {
        let has = self.project.is_some();
        let slides = self.editing_slides();
        match c {
            Cmd::New | Cmd::Open | Cmd::Settings | Cmd::About | Cmd::Quit | Cmd::DarkMode | Cmd::ShowMenuBar | Cmd::ShowInsertBar | Cmd::SubmitIssue | Cmd::Community | Cmd::TogglePanel(_) | Cmd::Preset(_) => true,
            Cmd::Docs => false,
            Cmd::Tool(crate::Tool::Find) => true,
            Cmd::PlayerSettings | Cmd::Tool(_) | Cmd::LayoutBuilder | Cmd::SlideSize(..) if !has => false,
            Cmd::LayoutBuilder => !self.in_master_view() && !self.project.as_ref().is_some_and(|p| p.is_web()),
            Cmd::PlayerSettings | Cmd::Tool(_) => true,
            Cmd::SlideSize(..) => true,
            Cmd::CheckForUpdates => !matches!(self.updater.state, crate::update::State::Checking(_)),
            _ if !has => false,
            Cmd::Undo => !self.undo.is_empty(),
            Cmd::Redo => !self.redo.is_empty(),
            Cmd::Cut | Cmd::Delete => if slides { self.project.as_ref().is_some_and(|p| p.slides.len() > self.slide_selection().len()) } else { self.view.selected.is_some() },
            Cmd::Copy | Cmd::Duplicate => slides || self.view.selected.is_some(),
            Cmd::Paste => if slides { !self.slide_clip.is_empty() } else { self.clipboard.is_some() },
            Cmd::SelectAll | Cmd::Find | Cmd::Save | Cmd::SaveAs | Cmd::Publish | Cmd::ProjectInfo | Cmd::ProjectSettings => true,
            Cmd::ZoomIn | Cmd::ZoomOut | Cmd::ZoomFit | Cmd::ZoomActual => !self.scene_view && !self.file_view,
        }
    }

    /// Whether a switch is on: `None` for a command that is not a switch.
    pub(crate) fn cmd_checked(&self, c: Cmd) -> Option<bool> {
        match c {
            Cmd::TogglePanel(g) => Some(self.panel_shown(g)),
            Cmd::ShowMenuBar => Some(self.show_menu_bar),
            Cmd::ShowInsertBar => Some(self.show_insert_bar),
            Cmd::SlideSize(w, h) => self.project.as_ref().map(|p| p.slides.first().is_some_and(|s| s.width as u32 == w && s.height as u32 == h)),
            Cmd::DarkMode => Some(self.ctx_copy.as_ref().is_some_and(|c| c.theme() == egui::Theme::Dark)),
            _ => None,
        }
    }

    /// Do it.
    pub(crate) fn run_cmd(&mut self, c: Cmd) {
        let slides = self.editing_slides();
        // With the States panel in front, Delete and Duplicate (the menu's, or the native menu's keys) are about its states.
        if !slides && self.states_focus && self.states_showing() && !self.try_mode {
            let key = match c { Cmd::Delete => Some(crate::states_panel::Cmd::Delete), Cmd::Duplicate => Some(crate::states_panel::Cmd::Duplicate), _ => None };
            if key.is_some() { self.state_cmd = key; return; }
        }
        match c {
            Cmd::Undo => self.undo(),
            Cmd::Redo => self.redo(),
            Cmd::Copy => if slides { self.copy_slides() } else { self.copy_selected() },
            Cmd::Cut => if slides { self.copy_slides(); let sel = self.slide_selection(); self.delete_slides(sel); } else { self.copy_selected(); self.delete_selected(); },
            Cmd::Paste => if slides { self.paste_slides() } else { self.paste() },
            Cmd::Delete => if slides { let sel = self.slide_selection(); self.delete_slides(sel); } else { self.delete_selected() },
            Cmd::Duplicate => if slides { self.duplicate_slides() } else { self.duplicate_selected() },
            Cmd::SelectAll => if slides { self.select_all_slides() } else { self.select_all_objects() },
            Cmd::Find => { self.find.get_or_insert_with(Default::default).focus = true; }
            Cmd::TogglePanel(g) => { let on = !self.panel_shown(g); self.set_panel_shown(g, on); }
            Cmd::Preset(p) => self.apply_preset(p),
            // The File view's own pages choose what kind of project to start and which file to open.
            Cmd::New => self.open_file_page(crate::backstage::FilePage::New),
            Cmd::Open => self.open_file_page(crate::backstage::FilePage::Open),
            Cmd::Save => self.save(false),
            Cmd::SaveAs => self.save(true),
            Cmd::Publish => self.open_file_page(crate::backstage::FilePage::Publish),
            Cmd::Settings => self.open_file_page(crate::backstage::FilePage::Settings),
            Cmd::ProjectInfo => self.open_file_page(crate::backstage::FilePage::Info),
            Cmd::ProjectSettings => self.open_file_page(crate::backstage::FilePage::Project),
            Cmd::About => self.open_file_page(crate::backstage::FilePage::About),
            Cmd::Quit => if let Some(ctx) = &self.ctx_copy { ctx.send_viewport_cmd(egui::ViewportCommand::Close); },
            Cmd::ZoomIn => self.view.zoom = (self.view.zoom * 1.25).clamp(0.1, 8.0),
            Cmd::ZoomOut => self.view.zoom = (self.view.zoom / 1.25).clamp(0.1, 8.0),
            Cmd::ZoomFit => { self.view.zoom = 1.0; self.view.pan = egui::Vec2::ZERO; }
            Cmd::ZoomActual => { self.view.zoom = (1.0 / self.view.fit_scale.max(0.01)).clamp(0.1, 8.0); self.view.pan = egui::Vec2::ZERO; }
            Cmd::ShowMenuBar => {
                // The menu bar has to stay unless the system's has taken its place, or File and the rest are out of reach.
                if self.show_menu_bar && !self.native_menu_active() { self.status = "The menu bar stays: it is the way to File and the other menus".into(); }
                else { self.show_menu_bar = !self.show_menu_bar; crate::prefs::set_bool("show_menu_bar", self.show_menu_bar); }
            }
            Cmd::ShowInsertBar => { self.show_insert_bar = !self.show_insert_bar; crate::prefs::set_bool("show_insert_bar", self.show_insert_bar); }
            Cmd::DarkMode => if let Some(ctx) = &self.ctx_copy {
                let dark = ctx.theme() == egui::Theme::Dark;
                ctx.set_theme(if dark { egui::ThemePreference::Light } else { egui::ThemePreference::Dark });
            },
            Cmd::Docs => {}
            Cmd::Tool(t) => match t {
                crate::Tool::Variables => self.variables_open = true,
                crate::Tool::FocusOrder => self.focus_open = !self.focus_open,
                crate::Tool::Spelling => self.open_spelling(),
                crate::Tool::Find => { self.find.get_or_insert_with(Default::default).focus = true; }
                crate::Tool::Check => self.check_results = self.project.as_ref().map(|p| p.check()),
                crate::Tool::Colors => self.theme_colors_open = true,
            },
            Cmd::LayoutBuilder => self.enter_master_view(),
            Cmd::PlayerSettings => self.open_file_page(crate::backstage::FilePage::Player),
            Cmd::SlideSize(w, h) => {
                let Some(ctx) = self.ctx_copy.clone() else { return };
                let Some(proj) = self.project.as_mut() else { return };
                let before = crate::Snap::of(proj);
                for sl in proj.slides.iter_mut() { sl.width = w as f32; sl.height = h as f32; }
                self.commit_project_edit(&ctx, before);
            }
            Cmd::SubmitIssue => crate::update::open_in_browser(crate::ribbon::ISSUES_URL),
            Cmd::Community => crate::update::open_in_browser(crate::ribbon::COMMUNITY_URL),
            Cmd::CheckForUpdates => if let Some(ctx) = self.ctx_copy.clone() { self.updater.check_now(&ctx); },
        }
    }

    /// The menu bar drawn inside the window: the menus of `menus()`, each line greyed out or ticked by the command's own rules.
    pub(crate) fn menu_bar(&mut self, root: &mut egui::Ui) {
        if !self.show_menu_bar { return; }
        let bg = root.visuals().panel_fill;
        egui::Panel::top("menubar").frame(egui::Frame::new().fill(bg).inner_margin(egui::Margin::symmetric(8, 0))).show_separator_line(false).show(root, |ui| {
            let mut run: Option<Cmd> = None;
            egui::MenuBar::new().ui(ui, |ui| {
                ui.set_min_height(26.0);
                self.menu_buttons(ui, &mut run);
            });
            if let Some(c) = run { self.run_cmd(c); }
        });
    }

    /// The one bar of Windows and Linux, below the system's own title bar: the menus, then Save, Undo, Redo and Find, and at the right the theme
    /// toggle, Try It, Preview and Publish as ordinary buttons. (On a Mac the title bar and the menus of the system take these places.)
    pub(crate) fn window_bar(&mut self, root: &mut egui::Ui, ctx: &egui::Context) {
        let bg = root.visuals().panel_fill;
        egui::Panel::top("menubar").frame(egui::Frame::new().fill(bg).inner_margin(egui::Margin::symmetric(8, 4))).show_separator_line(false).show(root, |ui| {
            let mut run: Option<Cmd> = None;
            egui::MenuBar::new().ui(ui, |ui| {
                ui.set_min_height(28.0);
                self.menu_buttons(ui, &mut run);
                // With a caption bar of its own the icons are up there beside the app's icon.
                if self.project.is_some() && !self.caption {
                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(2.0);
                    self.quick_icons(ui, true);
                }
                // (With a caption bar of its own they are up there, beside the window's buttons.)
                if !self.caption { self.bar_actions(ui, ctx, false, 6.0); }
            });
            if let Some(c) = run { self.run_cmd(c); }
        });
    }

    /// The menus as buttons that open their lists; a command that is chosen is left in `run`.
    fn menu_buttons(&mut self, ui: &mut egui::Ui, run: &mut Option<Cmd>) {
        ui.spacing_mut().item_spacing.x = 2.0;
        ui.spacing_mut().button_padding = egui::vec2(9.0, 3.0);
        ui.add_space(4.0);
        for (name, items) in menus() {
            ui.menu_button(name, |ui| {
                ui.set_min_width(210.0);
                for item in items {
                    match item {
                        Item::Gap => { ui.separator(); }
                        Item::Gallery => { if self.project.is_some() && !self.try_mode { self.insert_menu(ui); } else { ui.weak("Open a project to add objects"); } }
                        Item::Sub(name, cmds) => {
                            ui.menu_button(name, |ui| {
                                for c in cmds {
                                    let mut button = egui::Button::new(c.label());
                                    if let Some(on) = self.cmd_checked(c) { button = button.selected(on); }
                                    if ui.add_enabled(self.cmd_enabled(c), button).clicked() { *run = Some(c); ui.close(); }
                                }
                            });
                        }
                        Item::Do(c) => {
                            let mut button = egui::Button::new(c.label());
                            if let Some(sc) = c.shortcut() { button = button.shortcut_text(sc); }
                            if let Some(on) = self.cmd_checked(c) { button = button.selected(on); }
                            if ui.add_enabled(self.cmd_enabled(c), button).clicked() { *run = Some(c); ui.close(); }
                        }
                    }
                }
            });
        }
    }

    /// Every slide of the scene being shown (or every slide, in the Scene View).
    fn select_all_slides(&mut self) {
        let Some(p) = &self.project else { return };
        let scene = p.slide_get(self.view.slide).map(|s| s.scene.clone());
        let all = self.scene_view;
        self.slide_multi = p.slides.iter().enumerate().filter(|(i, s)| *i != self.view.slide && (all || Some(&s.scene) == scene.as_ref())).map(|(_, s)| s.id.clone()).collect();
    }

    /// Every object on the layer being worked on.
    fn select_all_objects(&mut self) {
        let Some(p) = &self.project else { return };
        let li = self.view.layer.min(p.slide(self.view.slide).layers.len().saturating_sub(1));
        let objects = &p.slide(self.view.slide).layers[li].objects;
        let Some(first) = objects.iter().position(|o| !o.hidden || self.view.show_hidden) else { return };
        let ids: Vec<String> = objects.iter().enumerate().filter(|(k, o)| *k != first && (!o.hidden || self.view.show_hidden)).map(|(_, o)| o.id.clone()).collect();
        self.view.selected = Some((li, first));
        self.view.multi = ids.into_iter().collect();
    }

    /// Show or hide a panel, and remember it. A panel that is shown comes back where it was.
    pub(crate) fn set_panel_shown(&mut self, g: usize, on: bool) {
        let (key, flag) = match g {
            0 => ("show_properties", &mut self.panels.properties), 1 => ("show_triggers", &mut self.panels.triggers), 2 => ("show_ai", &mut self.panels.ai),
            crate::P_TIMELINE => ("show_timeline", &mut self.panels.timeline), crate::P_NOTES => ("show_notes", &mut self.panels.notes), crate::P_STATES => ("show_states", &mut self.panels.states),
            dock::LAYERS => ("show_layers", &mut self.panels.layers), crate::P_SLIDES => ("show_slides", &mut self.panels.slides),
            _ => return,
        };
        *flag = on;
        crate::prefs::set_bool(key, on);
        if on { self.show_panel_tab(g); }
    }
}
