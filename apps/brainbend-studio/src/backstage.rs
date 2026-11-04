//! The full-screen File view ("backstage"): Back to Project, Project Info, New, Open, Save, Save As, Publish.
//! Project Info, New and Open are pages, like tabs. With no project open it is the app's start page, showing New.
use super::*;
use super::ribbon::{icons, tint, ORANGE, RIBBON_RADIUS};

const BACK: char = '\u{e058}';
const INFO: char = '\u{e2ce}';
const SAVE_AS: char = '\u{eaf4}';
const NEW: char = '\u{e236}';
const OPEN: char = '\u{e256}';
const SAVE: char = '\u{e248}';
const WEB: char = '\u{e0f6}';
const CLOSE: char = '\u{e4f6}';
const IMPORT: char = '\u{e20c}';
const MEDIA: char = '\u{e656}';

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum FilePage {
    #[default]
    New,
    Open,
    Info,
}

enum Choice {
    Back,
    Save,
    SaveAs,
    /// Back to the project with the Publish dialog open.
    Publish,
    Page(FilePage),
}

/// The bottom of a tile: an orange button, or a plain tag for something that is not ready.
#[derive(Clone, Copy)]
enum Footer { Action(&'static str), Tag(&'static str) }

/// A tile of the New and Open pages: a big icon, a name, a few words about it and a footer. Returns true when clicked
/// (a tile that is not `available` cannot be).
fn card(ui: &mut egui::Ui, height: f32, icon: char, name: &str, blurb: &str, footer: Footer, available: bool) -> bool {
    let dark = ui.visuals().dark_mode;
    let card_fill = if dark { egui::Color32::from_gray(46) } else { egui::Color32::WHITE };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(232.0, height), egui::Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, available, name));
    let hot = available && resp.hovered();
    ui.painter().rect_filled(rect, RIBBON_RADIUS, card_fill);
    ui.painter().rect_stroke(rect, RIBBON_RADIUS, egui::Stroke::new(if hot { 2.0 } else { 1.0 }, if hot { ORANGE } else { ui.visuals().text_color().gamma_multiply(0.18) }), egui::StrokeKind::Inside);
    let text = ui.visuals().text_color();
    let muted = if available { text } else { text.gamma_multiply(0.55) };
    let icon_col = if available { tint(icon) } else { tint(icon).gamma_multiply(0.45) };
    ui.painter().text(rect.left_top() + egui::vec2(20.0, 20.0), egui::Align2::LEFT_TOP, icon, egui::FontId::new(40.0, icons()), icon_col);
    ui.painter().text(rect.left_top() + egui::vec2(20.0, 76.0), egui::Align2::LEFT_TOP, name, egui::FontId::proportional(17.0), muted);
    let galley = ui.painter().layout(blurb.to_string(), egui::FontId::proportional(12.5), muted.gamma_multiply(0.8), rect.width() - 40.0);
    ui.painter().galley(rect.left_top() + egui::vec2(20.0, 106.0), galley, muted);
    let footer_rect = egui::Rect::from_min_size(rect.left_bottom() + egui::vec2(20.0, -44.0), egui::vec2(rect.width() - 40.0, 26.0));
    match footer {
        Footer::Action(label) => {
            ui.painter().rect_filled(footer_rect, RIBBON_RADIUS, ORANGE);
            ui.painter().text(footer_rect.center(), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(13.5), egui::Color32::WHITE);
        }
        Footer::Tag(label) => {
            ui.painter().rect_stroke(footer_rect, RIBBON_RADIUS, egui::Stroke::new(1.0, text.gamma_multiply(0.3)), egui::StrokeKind::Inside);
            ui.painter().text(footer_rect.center(), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(13.0), text.gamma_multiply(0.6));
        }
    }
    available && resp.clicked()
}

/// What the Open page asks for.
enum OpenAction {
    /// Pick a project file in a dialog.
    Browse,
    /// Pick a Storyline or PowerPoint file in a dialog.
    Import,
    /// Reopen a project from the Recent list.
    Recent(std::path::PathBuf),
    /// Take a project off the Recent list.
    Forget(std::path::PathBuf),
}

/// What the New page offers. Only the media lesson can be created today; web lessons are marked Coming soon until their
/// editor is finished.
struct Template {
    kind: ProjectKind,
    icon: char,
    blurb: &'static str,
    available: bool,
}

const TEMPLATES: [Template; 2] = [
    Template { kind: ProjectKind::WebLesson, icon: WEB, blurb: "A scrolling, mobile-friendly lesson built from content blocks: text, images, lists and knowledge checks.", available: false },
    Template { kind: ProjectKind::MediaLesson, icon: MEDIA, blurb: "A deck-style lesson: slides with layers, triggers, variables, animation, video and graded quizzes. Publishes to the web and SCORM.", available: true },
];

impl App {
    /// Draws the File view instead of the editor. Returns true while it is open.
    pub(super) fn backstage(&mut self, root: &mut egui::Ui, ctx: &egui::Context) -> bool {
        if !self.file_view { return false; }
        // The test harness draws a first frame before fonts exist.
        if !ctx.fonts(|f| f.families().contains(&icons())) { return false; }
        let has_project = self.project.is_some();
        if has_project && ctx.input(|i| i.key_pressed(egui::Key::Escape)) { self.file_view = false; return false; }
        // With nothing open there is no Project Info to show.
        if !has_project && !matches!(self.file_page, FilePage::New | FilePage::Open) { self.file_page = FilePage::New; }
        let mut choice: Option<Choice> = None;
        let mut create: Option<ProjectKind> = None;
        let mut open_action: Option<OpenAction> = None;
        let nav_fill = ribbon::ACCENT;
        egui::Panel::left("backstage-nav")
            .exact_size(250.0)
            .frame(egui::Frame::new().fill(nav_fill).inner_margin(egui::Margin::symmetric(12, 14)))
            .show_separator_line(false)
            .show(root, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                let items: [(char, &str, Choice, bool); 7] = [
                    (BACK, "Back to Project", Choice::Back, has_project),
                    (INFO, "Project Info", Choice::Page(FilePage::Info), has_project),
                    (NEW, "New", Choice::Page(FilePage::New), true),
                    (OPEN, "Open", Choice::Page(FilePage::Open), true),
                    (SAVE, "Save", Choice::Save, has_project),
                    (SAVE_AS, "Save As", Choice::SaveAs, has_project),
                    (ribbon::EXPORT, "Publish", Choice::Publish, has_project),
                ];
                for (n, (icon, label, ch, enabled)) in items.into_iter().enumerate() {
                    // The first entry is the way out; keep it apart from the file commands.
                    if n == 1 { ui.add_space(8.0); }
                    let selected = matches!(&ch, Choice::Page(p) if *p == self.file_page);
                    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 42.0), if enabled { egui::Sense::click() } else { egui::Sense::hover() });
                    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
                    if selected || (enabled && resp.hovered()) {
                        ui.painter().rect_filled(rect, RIBBON_RADIUS, egui::Color32::from_white_alpha(if selected { 48 } else { 26 }));
                    }
                    if selected {
                        ui.painter().rect_filled(egui::Rect::from_min_size(rect.min + egui::vec2(0.0, 8.0), egui::vec2(3.0, 26.0)), 1.0, egui::Color32::WHITE);
                    }
                    let alpha = if enabled { 255 } else { 130 };
                    let icon_col = egui::Color32::from_white_alpha(alpha);
                    ui.painter().text(egui::pos2(rect.left() + 20.0, rect.center().y), egui::Align2::LEFT_CENTER, icon, egui::FontId::new(20.0, icons()), icon_col);
                    ui.painter().text(egui::pos2(rect.left() + 54.0, rect.center().y), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(15.0), egui::Color32::from_white_alpha(alpha));
                    if enabled && resp.clicked() { choice = Some(ch); }
                }
            });
        // Every page shares one column: left-aligned, starting at the same place under its heading, up to 980px wide.
        egui::CentralPanel::default().frame(egui::Frame::new().fill(root.visuals().panel_fill).inner_margin(egui::Margin::symmetric(48, 36))).show(root, |ui| {
            let area = ui.max_rect();
            let w = 980.0_f32.min(ui.available_width());
            let used = ui.allocate_ui_with_layout(egui::vec2(w, ui.available_height()), egui::Layout::top_down(egui::Align::Min), |ui| {
                match self.file_page {
                    FilePage::Info if has_project => self.info_page(ui),
                    FilePage::Open => open_action = self.open_page(ui),
                    _ => create = self.new_page(ui),
                }
            }).response.rect;
            draw_watermark(ui, area, used);
        });
        match choice {
            Some(Choice::Back) => self.file_view = false,
            Some(Choice::Save) => { self.save(false); self.file_view = false; }
            Some(Choice::SaveAs) => { self.save(true); self.file_view = false; }
            Some(Choice::Publish) => { self.file_view = false; self.publish_open = true; }
            Some(Choice::Page(p)) => self.file_page = p,
            None => {}
        }
        // The view closes once a file is actually opened, so cancelling the dialog leaves it up.
        match open_action {
            Some(OpenAction::Browse) => self.open_requested = true,
            Some(OpenAction::Import) => self.guard(Pending::Import),
            Some(OpenAction::Recent(path)) => { self.drop_path = Some(path); self.guard(Pending::OpenDropped); }
            Some(OpenAction::Forget(path)) => self.forget_recent(&path),
            None => {}
        }
        if let Some(kind) = create {
            match kind {
                ProjectKind::MediaLesson => self.guard(Pending::New),
                ProjectKind::WebLesson => self.guard(Pending::NewLesson(kind)),
            }
        }
        true
    }

    /// The New page: one card per kind of project. Returns the kind whose card was clicked.
    fn new_page(&mut self, ui: &mut egui::Ui) -> Option<ProjectKind> {
        let mut picked = None;
        ui.heading("New");
        ui.add_space(10.0);
        ui.label(egui::RichText::new("What would you like to create?").size(15.0));
        ui.add_space(14.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(16.0, 16.0);
            for t in &TEMPLATES {
                let footer = if t.available { Footer::Action("Create") } else { Footer::Tag("Coming soon") };
                if card(ui, 236.0, t.icon, t.kind.label(), t.blurb, footer, t.available) { picked = Some(t.kind); }
            }
        });
        ui.add_space(22.0);
        if self.project.is_some() {
            ui.weak("Or choose Open in the list on the left to open a recent project or an existing .brainbend, .story or .pptx file.");
        } else {
            ui.horizontal(|ui| {
                ui.weak("Or");
                if ui.link("open an existing project").clicked() { self.file_page = FilePage::Open; }
                ui.weak("(.brainbend, .story or .pptx). You can also drop a file on this window.");
            });
        }
        picked
    }

    /// The Open page: recent projects (one click to reopen), a way to find one that is not listed, and a way to import a
    /// Storyline or PowerPoint file.
    fn open_page(&mut self, ui: &mut egui::Ui) -> Option<OpenAction> {
        let mut action = None;
        ui.heading("Open");
        ui.add_space(10.0);
        ui.label(egui::RichText::new("What would you like to open?").size(15.0));
        ui.add_space(14.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(16.0, 16.0);
            if card(ui, 236.0, OPEN, "Browse", "Open a .brainbend project that is not in the list below.", Footer::Action("Browse…"), true) { action = Some(OpenAction::Browse); }
            if card(ui, 236.0, IMPORT, "Import", "Bring in a Storyline .story or PowerPoint .pptx file as a new project.", Footer::Action("Import…"), true) { action = Some(OpenAction::Import); }
        });
        ui.add_space(6.0);
        ui.weak("You can also drop a .brainbend, .story or .pptx file on this window.");
        ui.add_space(22.0);
        ui.label(egui::RichText::new("Recent").size(17.0));
        ui.add_space(6.0);
        if self.recent.is_empty() {
            ui.weak("Projects you open or save will be listed here.");
            return action;
        }
        let dark = ui.visuals().dark_mode;
        let card_fill = if dark { egui::Color32::from_gray(46) } else { egui::Color32::WHITE };
        let (text, now) = (ui.visuals().text_color(), recent::now());
        let width = ui.available_width().min(760.0);
        ui.spacing_mut().item_spacing.y = 8.0;
        for (i, r) in self.recent.iter().enumerate() {
            let found = r.path.exists();
            let name = r.path.file_stem().and_then(|s| s.to_str()).unwrap_or("Untitled");
            let folder = r.path.parent().map(|d| d.display().to_string()).unwrap_or_default();
            let when = if found { recent::ago(now, r.opened) } else { "Not found".to_string() };
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 56.0), if found { egui::Sense::click() } else { egui::Sense::hover() });
            resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, found, format!("{name}, {folder}, {when}")));
            let hot = found && resp.hovered();
            ui.painter().rect_filled(rect, RIBBON_RADIUS, card_fill);
            ui.painter().rect_stroke(rect, RIBBON_RADIUS, egui::Stroke::new(if hot { 2.0 } else { 1.0 }, if hot { ORANGE } else { text.gamma_multiply(0.18) }), egui::StrokeKind::Inside);
            let ink = if found { text } else { text.gamma_multiply(0.55) };
            ui.painter().text(rect.left_top() + egui::vec2(16.0, 10.0), egui::Align2::LEFT_TOP, name, egui::FontId::proportional(15.0), ink);
            ui.painter().text(rect.left_top() + egui::vec2(16.0, 32.0), egui::Align2::LEFT_TOP, &folder, egui::FontId::proportional(12.0), ink.gamma_multiply(0.7));
            ui.painter().text(egui::pos2(rect.right() - 52.0, rect.center().y), egui::Align2::RIGHT_CENTER, &when, egui::FontId::proportional(12.5), if found { ink.gamma_multiply(0.7) } else { ui.visuals().warn_fg_color });
            // A button over the right end takes the project off the list (the file itself is not touched).
            let x_rect = egui::Rect::from_center_size(egui::pos2(rect.right() - 24.0, rect.center().y), egui::vec2(28.0, 28.0));
            let x = ui.interact(x_rect, ui.id().with(("recent-forget", i)), egui::Sense::click());
            x.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("Remove {name} from the list")));
            if x.hovered() { ui.painter().rect_filled(x_rect, RIBBON_RADIUS, text.gamma_multiply(0.12)); }
            ui.painter().text(x_rect.center(), egui::Align2::CENTER_CENTER, CLOSE, egui::FontId::new(14.0, icons()), ink.gamma_multiply(0.8));
            x.clone().on_hover_text("Remove from the list. The file is not deleted.");
            if x.clicked() { action = Some(OpenAction::Forget(r.path.clone())); }
            else if found && resp.clicked() { action = Some(OpenAction::Recent(r.path.clone())); }
        }
        action
    }

    fn info_page(&mut self, ui: &mut egui::Ui) {
        ui.heading("Project Info");
        ui.add_space(10.0);
        let path = self.path.as_ref().map(|p| p.display().to_string());
        let dirty = self.dirty;
        let Some(p) = self.project.as_mut() else { return };
        ui.horizontal(|ui| {
            ui.label("Project name");
            if ui.add(egui::TextEdit::singleline(&mut p.name).desired_width(320.0)).changed() { self.dirty = true; }
        });
        ui.add_space(8.0);
        let questions = p.slides.iter().filter(|s| matches!(s.kind, SlideKind::Question(_))).count();
        let objects: usize = p.slides.iter().flat_map(|s| s.layers.iter()).map(|l| l.objects.len()).sum();
        let media_bytes: usize = p.assets.values().map(|a| a.bytes.len()).sum();
        let (w, h) = p.slides.first().map(|s| (s.width as i32, s.height as i32)).unwrap_or((0, 0));
        egui::Grid::new("info-grid").num_columns(2).spacing([24.0, 8.0]).show(ui, |ui| {
            let mut row = |k: &str, v: String| { ui.label(egui::RichText::new(k).weak()); ui.label(v); ui.end_row(); };
            row("Type", p.kind.label().to_string());
            row("File", path.unwrap_or_else(|| "Not saved yet".into()));
            row("Status", if dirty { "Unsaved changes".into() } else { "All changes saved".into() });
            row("Slides", format!("{} ({} question slide{})", p.slides.len(), questions, if questions == 1 { "" } else { "s" }));
            row("Objects", objects.to_string());
            row("Media files", format!("{} ({:.1} MB)", p.assets.len(), media_bytes as f64 / 1_048_576.0));
            row("Slide size", format!("{w} x {h}"));
            row("Variables", p.variables.len().to_string());
            row("Pass mark", format!("{}%", p.pass_mark));
        });
        ui.add_space(24.0);
        ui.separator();
        ui.add_space(12.0);
        ui.heading("About BrainBend Studio");
        ui.add_space(6.0);
        ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
        self.update_controls(ui);
        ui.label("A free, lightweight authoring tool for interactive lessons that publish to the web and to LMSs (SCORM 1.2 and 2004).");
        ui.add_space(6.0);
        ui.weak("Fonts: Montserrat and Open Sans (SIL Open Font License 1.1). Icons: Phosphor (MIT). Spelling: SCOWL en_US word list (permissive license, crates/spell/dict).");
    }

    /// The update line of the About section: what the last check found, a button to check now, and the launch preference.
    fn update_controls(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        let checking = matches!(self.updater.state, update::State::Checking(_));
        let mut open: Option<String> = None;
        ui.horizontal(|ui| {
            match &self.updater.state {
                update::State::Checking(_) => { ui.add(egui::Spinner::new()); ui.label("Checking for updates…"); }
                update::State::UpToDate => { ui.label("You have the latest version."); }
                update::State::Failed(e) => { ui.weak(format!("Could not check for updates: {e}")); }
                update::State::Idle => {}
                _ => if let Some(r) = self.updater.available() {
                    ui.strong(format!("Version {} is available.", r.version));
                    if ui.link("Update").clicked() { open = Some(r.tag.clone()); }
                },
            }
        });
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            if ui.add_enabled(!checking, egui::Button::new("Check for updates")).clicked() { self.updater.start(ui.ctx()); }
            if ui.checkbox(&mut self.updater.on_launch, "Check when BrainBend Studio starts").changed() { self.updater.save(); }
        });
        ui.add_space(6.0);
        if open.is_some() { self.updater.dialog_open = true; }
    }

    pub(super) fn print(&mut self) {
        let Some(whole) = &self.project else { return };
        // A Web Lesson has no slides to pick from; a slide project prints the range the author chose.
        let part = if whole.is_web() { whole.clone() } else { whole.subset(&self.handout_slides()) };
        let p = &part;
        let dir = std::env::temp_dir().join("brainbend-print");
        if let Err(e) = std::fs::create_dir_all(&dir) { self.status = format!("Could not prepare the print view: {e}"); return; }
        let file = dir.join("handout.html");
        if let Err(e) = std::fs::write(&file, brainbend_publish_html::print_html_with(p, &self.handout)) { self.status = format!("Could not write the print view: {e}"); return; }
        let opener = if cfg!(target_os = "macos") { "open" } else if cfg!(target_os = "windows") { "explorer" } else { "xdg-open" };
        self.status = match std::process::Command::new(opener).arg(&file).spawn() {
            Ok(_) => "Print view opened in your browser".into(),
            Err(e) => format!("Wrote {} but could not open it: {e}", file.display()),
        };
    }
}

/// The BrainBend logo, faint, in the bottom-right corner of the page. It only appears where it fits without touching
/// the page's content: below it or beside it. It shrinks to the room there is, and is left out when there is little.
fn draw_watermark(ui: &egui::Ui, area: egui::Rect, content: egui::Rect) {
    const MARGIN: f32 = 16.0;
    let room_below = area.bottom() - content.bottom() - MARGIN;
    let room_beside = area.right() - content.right() - MARGIN;
    let size = room_below.max(room_beside).min(area.width().min(area.height()) * 0.6).min(480.0);
    if size < 90.0 { return; }
    let id = egui::Id::new("brainbend-logo-texture");
    let ctx = ui.ctx();
    let tex = match ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        Some(t) => t,
        None => {
            let Ok(img) = image::load_from_memory(include_bytes!("../../../assets/brainbend-logo.png")) else { return };
            let img = img.to_rgba8();
            let size = [img.width() as usize, img.height() as usize];
            let t = ctx.load_texture("brainbend-logo", egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()), egui::TextureOptions::LINEAR);
            ctx.data_mut(|d| d.insert_temp(id, t.clone()));
            t
        }
    };
    let rect = egui::Rect::from_min_size(area.right_bottom() - egui::vec2(size, size) - egui::vec2(8.0, 8.0), egui::vec2(size, size));
    ui.painter().image(tex.id(), rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::from_white_alpha(60));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_media_lesson_can_be_created_until_web_lessons_are_finished() {
        for t in &TEMPLATES {
            assert_eq!(t.available, matches!(t.kind, ProjectKind::MediaLesson), "{} availability", t.kind.label());
        }
    }
}
