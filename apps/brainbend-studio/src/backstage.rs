//! The full-screen File view ("backstage"): Back to Project, Project Info, New, Open, Save, Save As, Print.
//! With no project open it is the app's start page, showing New.
use super::*;
use super::ribbon::{icons, tint, ORANGE, RIBBON_RADIUS};

const BACK: char = '\u{e058}';
const INFO: char = '\u{e2ce}';
const PRINTER: char = '\u{e3dc}';
const SAVE_AS: char = '\u{eaf4}';
const NEW: char = '\u{e236}';
const OPEN: char = '\u{e256}';
const SAVE: char = '\u{e248}';
const WEB: char = '\u{e0f6}';
const MEDIA: char = '\u{e656}';

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum FilePage {
    #[default]
    New,
    Info,
    Print,
}

enum Choice {
    Back,
    Open,
    Save,
    SaveAs,
    Page(FilePage),
}

/// What the New page offers. Only the media lesson can be created today.
struct Template {
    kind: ProjectKind,
    icon: char,
    blurb: &'static str,
    available: bool,
}

const TEMPLATES: [Template; 2] = [
    Template { kind: ProjectKind::WebLesson, icon: WEB, blurb: "A scrolling, mobile-friendly lesson built from content blocks: text, images, lists and knowledge checks.", available: true },
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
        if !has_project && self.file_page != FilePage::New { self.file_page = FilePage::New; }
        let mut choice: Option<Choice> = None;
        let mut create: Option<ProjectKind> = None;
        let nav_fill = egui::Color32::from_rgb(24, 34, 56);
        // With nothing open this is the start page: just the New page, no sidebar.
        if has_project { egui::Panel::left("backstage-nav")
            .exact_size(250.0)
            .frame(egui::Frame::new().fill(nav_fill).inner_margin(egui::Margin::symmetric(12, 14)))
            .show_separator_line(false)
            .show(root, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                let items: [(char, &str, Choice, bool); 7] = [
                    (BACK, "Back to Project", Choice::Back, has_project),
                    (INFO, "Project Info", Choice::Page(FilePage::Info), has_project),
                    (NEW, "New", Choice::Page(FilePage::New), true),
                    (OPEN, "Open", Choice::Open, true),
                    (SAVE, "Save", Choice::Save, has_project),
                    (SAVE_AS, "Save As", Choice::SaveAs, has_project),
                    (PRINTER, "Print", Choice::Page(FilePage::Print), has_project),
                ];
                for (n, (icon, label, ch, enabled)) in items.into_iter().enumerate() {
                    // The first entry is the way out; keep it apart from the file commands.
                    if n == 1 { ui.add_space(8.0); }
                    let selected = matches!(&ch, Choice::Page(p) if *p == self.file_page);
                    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 42.0), if enabled { egui::Sense::click() } else { egui::Sense::hover() });
                    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
                    if selected || (enabled && resp.hovered()) {
                        ui.painter().rect_filled(rect, RIBBON_RADIUS, egui::Color32::from_white_alpha(if selected { 34 } else { 18 }));
                    }
                    if selected {
                        ui.painter().rect_filled(egui::Rect::from_min_size(rect.min + egui::vec2(0.0, 8.0), egui::vec2(3.0, 26.0)), 1.0, ORANGE);
                    }
                    let alpha = if enabled { 255 } else { 90 };
                    let icon_col = if enabled { tint(icon).gamma_multiply(1.25) } else { egui::Color32::from_white_alpha(alpha) };
                    ui.painter().text(egui::pos2(rect.left() + 20.0, rect.center().y), egui::Align2::LEFT_CENTER, icon, egui::FontId::new(20.0, icons()), icon_col);
                    ui.painter().text(egui::pos2(rect.left() + 54.0, rect.center().y), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(15.0), egui::Color32::from_white_alpha(alpha));
                    if enabled && resp.clicked() { choice = Some(ch); }
                }
            }); }
        // The New page's content block (about 980px wide) is centred in the window.
        egui::CentralPanel::default().frame(egui::Frame::new().fill(root.visuals().panel_fill).inner_margin(egui::Margin::symmetric(48, 36))).show(root, |ui| {
            match self.file_page {
                FilePage::Info if has_project => self.info_page(ui),
                FilePage::Print if has_project => self.print_page(ui),
                _ => {
                    let top = ((ui.available_height() - 470.0) / 3.0).max(0.0);
                    let w = 980.0_f32.min(ui.available_width());
                    ui.vertical_centered(|ui| {
                        ui.add_space(top);
                        ui.allocate_ui_with_layout(egui::vec2(w, ui.available_height()), egui::Layout::top_down(egui::Align::Min), |ui| {
                            create = self.new_page(ui);
                        });
                    });
                }
            }
        });
        match choice {
            Some(Choice::Back) => self.file_view = false,
            // The view closes once a file is actually opened, so cancelling the dialog leaves it up.
            Some(Choice::Open) => self.open_requested = true,
            Some(Choice::Save) => { self.save(false); self.file_view = false; }
            Some(Choice::SaveAs) => { self.save(true); self.file_view = false; }
            Some(Choice::Page(p)) => self.file_page = p,
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
        ui.add_space(4.0);
        ui.label(egui::RichText::new("What would you like to create?").size(15.0));
        ui.add_space(18.0);
        let dark = ui.visuals().dark_mode;
        let card_fill = if dark { egui::Color32::from_gray(46) } else { egui::Color32::WHITE };
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(16.0, 16.0);
            for t in &TEMPLATES {
                let (rect, resp) = ui.allocate_exact_size(egui::vec2(232.0, 236.0), egui::Sense::click());
                let name = t.kind.label();
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, t.available, name));
                let hot = t.available && resp.hovered();
                ui.painter().rect_filled(rect, RIBBON_RADIUS, card_fill);
                ui.painter().rect_stroke(rect, RIBBON_RADIUS, egui::Stroke::new(if hot { 2.0 } else { 1.0 }, if hot { ORANGE } else { ui.visuals().text_color().gamma_multiply(0.18) }), egui::StrokeKind::Inside);
                let text = ui.visuals().text_color();
                let muted = if t.available { text } else { text.gamma_multiply(0.55) };
                let icon_col = if t.available { tint(t.icon) } else { tint(t.icon).gamma_multiply(0.45) };
                ui.painter().text(rect.left_top() + egui::vec2(20.0, 20.0), egui::Align2::LEFT_TOP, t.icon, egui::FontId::new(40.0, icons()), icon_col);
                ui.painter().text(rect.left_top() + egui::vec2(20.0, 76.0), egui::Align2::LEFT_TOP, name, egui::FontId::proportional(17.0), muted);
                let galley = ui.painter().layout(t.blurb.to_string(), egui::FontId::proportional(12.5), muted.gamma_multiply(0.8), rect.width() - 40.0);
                ui.painter().galley(rect.left_top() + egui::vec2(20.0, 106.0), galley, muted);
                // Footer: Create (orange) or a Coming soon tag.
                let footer = egui::Rect::from_min_size(rect.left_bottom() + egui::vec2(20.0, -44.0), egui::vec2(rect.width() - 40.0, 26.0));
                if t.available {
                    ui.painter().rect_filled(footer, RIBBON_RADIUS, ORANGE);
                    ui.painter().text(footer.center(), egui::Align2::CENTER_CENTER, "Create", egui::FontId::proportional(13.5), egui::Color32::WHITE);
                } else {
                    ui.painter().rect_stroke(footer, RIBBON_RADIUS, egui::Stroke::new(1.0, text.gamma_multiply(0.3)), egui::StrokeKind::Inside);
                    ui.painter().text(footer.center(), egui::Align2::CENTER_CENTER, "Coming soon", egui::FontId::proportional(13.0), text.gamma_multiply(0.6));
                }
                if t.available && resp.clicked() { picked = Some(t.kind); }
            }
        });
        ui.add_space(22.0);
        if self.project.is_some() {
            ui.weak("Or choose Open in the list on the left to open an existing .brainbend, .story or .pptx file.");
        } else {
            ui.horizontal(|ui| {
                ui.weak("Or");
                if ui.link("open an existing project").clicked() { self.open_requested = true; }
                ui.weak("(.brainbend, .story or .pptx). You can also drop a file on this window.");
            });
        }
        picked
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
        ui.label("A free, lightweight authoring tool for interactive lessons that publish to the web and to LMSs (SCORM 1.2 and 2004).");
        ui.add_space(6.0);
        ui.weak("Fonts: Montserrat and Open Sans (SIL Open Font License 1.1). Icons: Phosphor (MIT). Spelling: SCOWL en_US word list (permissive license, crates/spell/dict).");
    }

    fn print_page(&mut self, ui: &mut egui::Ui) {
        ui.heading("Print");
        ui.add_space(10.0);
        ui.label("Print a handout with one slide per page. The slides open in your browser, which shows its print dialog; choose a printer or \"Save as PDF\" there.");
        ui.add_space(6.0);
        ui.weak("Question slides print the question and its answers (correct ones ticked). Video and audio print as a placeholder.");
        ui.add_space(16.0);
        if ui.add(egui::Button::new(egui::RichText::new("Print…").strong()).corner_radius(RIBBON_RADIUS as u8).min_size(egui::vec2(140.0, 34.0))).clicked() {
            self.print();
        }
        if !self.status.is_empty() {
            ui.add_space(10.0);
            ui.weak(self.status.clone());
        }
    }

    pub(super) fn print(&mut self) {
        let Some(p) = &self.project else { return };
        let dir = std::env::temp_dir().join("brainbend-print");
        if let Err(e) = std::fs::create_dir_all(&dir) { self.status = format!("Could not prepare the print view: {e}"); return; }
        let file = dir.join("handout.html");
        if let Err(e) = std::fs::write(&file, brainbend_publish_html::print_html(p)) { self.status = format!("Could not write the print view: {e}"); return; }
        let opener = if cfg!(target_os = "macos") { "open" } else if cfg!(target_os = "windows") { "explorer" } else { "xdg-open" };
        self.status = match std::process::Command::new(opener).arg(&file).spawn() {
            Ok(_) => "Print view opened in your browser".into(),
            Err(e) => format!("Wrote {} but could not open it: {e}", file.display()),
        };
    }
}
