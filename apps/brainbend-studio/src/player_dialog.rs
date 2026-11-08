//! Player Properties: how the published player looks (colors, font, buttons) and what it offers (title, sidebar, logo,
//! controls), with a small preview that follows the settings.
use super::*;
use brainbend_doc::{ButtonStyle, GlossaryTerm, PlayerSettings, PlayerTheme, Resource, SidebarSide};

fn c32(c: Rgba) -> egui::Color32 {
    egui::Color32::from_rgb(c.0, c.1, c.2)
}

fn color_row(ui: &mut egui::Ui, label: &str, c: &mut Rgba, enabled: bool) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.add_enabled_ui(enabled, |ui| {
            changed |= crate::ribbon::color_swatch(ui, c);
            ui.label(label);
        });
    });
    changed
}

impl App {
    /// The Player Properties window (opened from Player Settings on the Review tab).
    pub(super) fn player_window(&mut self, ctx: &egui::Context) {
        if !self.player_open { return; }
        let Some(proj) = self.project.as_mut() else { self.player_open = false; return };
        let before = Snap::of(proj);
        let mut changed = false;
        let mut open = true;
        let mut pick_logo = false;
        let mut pick_banner = false;
        let mut pick_resource: Option<usize> = None;
        let logo_name = proj.player.logo.as_ref().and_then(|k| proj.assets.get(k)).map(|a| a.name.clone());
        let banner_name = proj.player.banner.as_ref().and_then(|k| proj.assets.get(k)).map(|a| a.name.clone());
        let name = proj.name.clone();
        let pane = &mut self.player_pane;
        crate::tool_window(ctx, "player-properties", "Player Properties", [920.0, 600.0], &mut open, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(pane, 0, "Features");
                ui.selectable_value(pane, 1, "Colors & Effects");
                ui.selectable_value(pane, 2, "Resources & Glossary");
            });
            ui.separator();
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(380.0);
                    egui::ScrollArea::vertical().id_salt("player-left").show(ui, |ui| {
                        if *pane == 0 {
                            features_pane(ui, proj, &name, &logo_name, &banner_name, &mut changed, &mut pick_logo, &mut pick_banner);
                        } else if *pane == 1 {
                            colors_pane(ui, &mut proj.player, &mut changed);
                        } else {
                            resources_pane(ui, proj, &mut changed, &mut pick_resource);
                        }
                    });
                });
                ui.separator();
                preview(ui, proj, &name);
            });
        });
        if changed { self.commit_project_edit(ctx, before); }
        if pick_logo {
            if let Some((key, _, _, _)) = self.load_image_asset() {
                let before = self.project.as_ref().map(Snap::of);
                if let (Some(p), Some(before)) = (self.project.as_mut(), before) {
                    p.player.logo = Some(key);
                    self.commit_project_edit(ctx, before);
                }
            }
        }
        if pick_banner {
            if let Some((key, _, _, _)) = self.load_image_asset() {
                let before = self.project.as_ref().map(Snap::of);
                if let (Some(p), Some(before)) = (self.project.as_mut(), before) {
                    p.player.banner = Some(key);
                    self.commit_project_edit(ctx, before);
                }
            }
        }
        if let Some(i) = pick_resource {
            if let Some(path) = rfd::FileDialog::new().pick_file() {
                match std::fs::read(&path) {
                    Ok(bytes) => {
                        let key = self.new_id("asset");
                        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file").to_string();
                        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
                        let mime = match ext.as_str() { "pdf" => "application/pdf", "png" => "image/png", "jpg" | "jpeg" => "image/jpeg", "txt" => "text/plain", "csv" => "text/csv", "zip" => "application/zip", "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document", "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation", _ => "application/octet-stream" };
                        let before = self.project.as_ref().map(Snap::of);
                        if let (Some(p), Some(before)) = (self.project.as_mut(), before) {
                            p.assets.insert(key.clone(), Asset { name: name.clone(), mime: mime.into(), bytes, file: None });
                            if let Some(r) = p.resources.get_mut(i) {
                                r.asset = Some(key);
                                r.url.clear();
                                if r.title.trim().is_empty() { r.title = name; }
                            }
                            self.commit_project_edit(ctx, before);
                        }
                    }
                    Err(e) => self.status = format!("Could not read {}: {e}", path.display()),
                }
            }
        }
        if !open { self.player_open = false; }
    }
}

fn features_pane(ui: &mut egui::Ui, proj: &mut Project, project_name: &str, logo_name: &Option<String>, banner_name: &Option<String>, changed: &mut bool, pick_logo: &mut bool, pick_banner: &mut bool) {
    ui.heading("Player tabs");
    ui.horizontal(|ui| {
        *changed |= ui.checkbox(&mut proj.show_menu, "Menu").on_hover_text("Scenes and slides").changed();
        *changed |= ui.checkbox(&mut proj.player.tab_glossary, "Glossary").on_hover_text("The project's terms, in the sidebar").changed();
        *changed |= ui.checkbox(&mut proj.player.tab_notes, "Notes").on_hover_text("The current slide's notes, in the sidebar").changed();
        *changed |= ui.checkbox(&mut proj.player.tab_resources, "Resources").on_hover_text("Links and files, at the right of the top bar").changed();
        *changed |= ui.checkbox(&mut proj.player.search, "Search").on_hover_text("Filter the slide list by name").changed();
    });
    ui.weak("Sidebar tabs always run Menu, Glossary, Notes. A tab with nothing in it is left out.");
    ui.add_space(8.0);
    ui.heading("Player features");
    ui.horizontal(|ui| {
        *changed |= ui.checkbox(&mut proj.player.show_title, "Title").changed();
        ui.add_enabled_ui(proj.player.show_title, |ui| {
            *changed |= ui.add(egui::TextEdit::singleline(&mut proj.player.title).hint_text(project_name).desired_width(220.0)).changed();
        });
    });
    ui.horizontal(|ui| {
        ui.label("Sidebar");
        let label = |s: SidebarSide| match s { SidebarSide::Off => "Off", SidebarSide::Left => "On the left", SidebarSide::Right => "On the right" };
        combo("sidebar-side").selected_text(label(proj.player.sidebar)).show_ui(ui, |ui| {
            for s in [SidebarSide::Off, SidebarSide::Left, SidebarSide::Right] {
                if ui.selectable_label(proj.player.sidebar == s, label(s)).clicked() && proj.player.sidebar != s { proj.player.sidebar = s; *changed = true; }
            }
        });
        ui.add_enabled_ui(proj.player.sidebar != SidebarSide::Off, |ui| {
            *changed |= ui.checkbox(&mut proj.player.sidebar_collapsed, "Starts collapsed").changed();
        });
    });
    ui.horizontal(|ui| {
        ui.label("Logo");
        match logo_name {
            Some(n) => {
                ui.label(n);
                if ui.small_button("Change").clicked() { *pick_logo = true; }
                if ui.small_button("Remove").clicked() { proj.player.logo = None; *changed = true; }
            }
            None => { if ui.button("+ Logo").clicked() { *pick_logo = true; } }
        }
    });
    ui.weak("The logo appears small in the top bar, next to the title.");
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label("Banner photo");
        match banner_name {
            Some(n) => {
                ui.label(n);
                if ui.small_button("Change").clicked() { *pick_banner = true; }
                if ui.small_button("Remove").clicked() { proj.player.banner = None; *changed = true; }
            }
            None => { if ui.button("+ Banner photo").clicked() { *pick_banner = true; } }
        }
    });
    ui.weak("A picture across the top of the sidebar, above the navigation. It is cropped to fit (about 560 x 240 pixels works well). It shows only when there is a sidebar, and not while the sidebar is collapsed.");
    if proj.player.banner.is_some() && proj.player.sidebar == SidebarSide::Off {
        ui.colored_label(egui::Color32::from_rgb(230, 160, 60), "Choose a sidebar above to show the banner.");
    }
    ui.add_space(8.0);
    ui.heading("Navigation");
    *changed |= ui.checkbox(&mut proj.show_nav, "Previous / Next buttons").changed();
    *changed |= ui.checkbox(&mut proj.show_progress, "Progress bar").on_hover_text("The thin bar along the top of the player that fills as learners move through the slides. Turn it off for lessons that are not meant to be followed in order.").changed();
    *changed |= ui.checkbox(&mut proj.menu_free, "Learners can skip questions from the menu").changed();
    ui.add_space(8.0);
    ui.heading("Player controls");
    let c = &mut proj.player.controls;
    *changed |= ui.checkbox(&mut c.play_pause, "Play / Pause").on_hover_text("Pause and resume the slide: its timeline, or its video or audio when it has no timeline").changed();
    *changed |= ui.checkbox(&mut c.seekbar, "Seekbar").on_hover_text("Shows how far along the slide is, and lets learners move through it with the mouse or the arrow keys").changed();
    *changed |= ui.checkbox(&mut c.replay, "Replay").on_hover_text("Start the slide again from the beginning").changed();
    *changed |= ui.checkbox(&mut c.volume, "Volume").on_hover_text("Mute and unmute the slide's video and audio").changed();
    *changed |= ui.checkbox(&mut c.captions, "Captions").on_hover_text("Turn captions on and off").changed();
    *changed |= ui.checkbox(&mut c.speed, "Playback speed").on_hover_text("Step through speeds for video and audio").changed();
    *changed |= ui.checkbox(&mut c.fullscreen, "Full-screen").changed();
}

fn colors_pane(ui: &mut egui::Ui, p: &mut PlayerSettings, changed: &mut bool) {
    ui.heading("Player");
    ui.horizontal(|ui| {
        for (t, label) in [(PlayerTheme::Dark, "Dark"), (PlayerTheme::Light, "Light"), (PlayerTheme::Custom, "Custom")] {
            let pal = PlayerSettings { theme: t, background: p.background, accent: p.accent, ..Default::default() }.palette();
            ui.vertical(|ui| {
                let (r, resp) = ui.allocate_exact_size(egui::vec2(104.0, 66.0), egui::Sense::click());
                ui.painter().rect_filled(r, 4.0, c32(pal.body));
                let bar = egui::Rect::from_min_max(egui::pos2(r.left(), r.bottom() - 16.0), r.max);
                ui.painter().rect_filled(egui::Rect::from_min_max(r.min + egui::vec2(8.0, 8.0), egui::pos2(r.right() - 8.0, r.bottom() - 20.0)), 2.0, c32(pal.panel));
                ui.painter().rect_filled(bar, 0.0, c32(pal.panel));
                ui.painter().rect_filled(egui::Rect::from_min_size(bar.left_top() + egui::vec2(8.0, 6.0), egui::vec2(24.0, 5.0)), 2.0, c32(pal.accent));
                ui.painter().rect_stroke(r, 4.0, egui::Stroke::new(if p.theme == t { 2.0 } else { 1.0 }, if p.theme == t { c32(Rgba(0, 120, 255, 255)) } else { egui::Color32::GRAY }), egui::StrokeKind::Inside);
                if resp.clicked() && p.theme != t { p.theme = t; *changed = true; }
                if ui.radio(p.theme == t, label).clicked() && p.theme != t { p.theme = t; *changed = true; }
            });
        }
    });
    ui.add_space(8.0);
    *changed |= color_row(ui, "Custom background color", &mut p.background, p.theme == PlayerTheme::Custom);
    *changed |= color_row(ui, "Accent color", &mut p.accent, true);
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label("Font");
        *changed |= crate::font_store::font_menu(ui, "player-font", &mut p.font, 150.0);
        let mut pct = p.font_scale;
        if ui.add(egui::DragValue::new(&mut pct).range(70..=160).suffix(" %")).changed() { p.font_scale = pct; *changed = true; }
    });
    ui.horizontal(|ui| {
        ui.label("Navigation buttons");
        let label = |b: ButtonStyle| match b { ButtonStyle::Icon => "Icons", ButtonStyle::IconText => "Icons and text", ButtonStyle::Text => "Text" };
        combo("button-style").selected_text(label(p.buttons)).show_ui(ui, |ui| {
            for b in [ButtonStyle::Icon, ButtonStyle::IconText, ButtonStyle::Text] {
                if ui.selectable_label(p.buttons == b, label(b)).clicked() && p.buttons != b { p.buttons = b; *changed = true; }
            }
        });
    });
}

/// A small picture of the player with the current settings.
fn preview(ui: &mut egui::Ui, proj: &Project, project_name: &str) {
    let p = &proj.player;
    let pal = p.palette();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width().max(320.0), 380.0), egui::Sense::hover());
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(rect, 4.0, c32(pal.body));
    let text = |pos: egui::Pos2, align: egui::Align2, s: &str, size: f32, col: Rgba| painter.text(pos, align, s, egui::FontId::proportional(size), c32(col));
    let (top_h, bar_h, side_w) = (if p.show_title || p.tab_resources || p.logo.is_some() || p.sidebar != SidebarSide::Off { 30.0 } else { 0.0 }, 34.0, 120.0);
    let side = p.sidebar != SidebarSide::Off && !p.sidebar_collapsed;
    let mut main = rect;
    if side {
        let s = if p.sidebar == SidebarSide::Left { egui::Rect::from_min_size(rect.min, egui::vec2(side_w, rect.height())) } else { egui::Rect::from_min_size(egui::pos2(rect.right() - side_w, rect.top()), egui::vec2(side_w, rect.height())) };
        painter.rect_filled(s, 0.0, c32(pal.panel));
        // The banner photo tops the sidebar, edge to edge, above the tabs.
        let mut y = s.top() + 8.0;
        if p.banner.is_some() {
            painter.rect_filled(egui::Rect::from_min_size(s.min, egui::vec2(side_w, 38.0)), 0.0, c32(pal.line));
            text(egui::pos2(s.left() + side_w / 2.0, s.top() + 19.0), egui::Align2::CENTER_CENTER, "banner", 10.0, pal.muted);
            y = s.top() + 46.0;
        }
        let mut tx = s.left() + 8.0;
        for (on, label) in [(true, "Menu"), (p.tab_glossary, "Glossary"), (p.tab_notes, "Notes")] {
            if on { text(egui::pos2(tx, y + 6.0), egui::Align2::LEFT_CENTER, label, 9.0, pal.muted); tx += 38.0; }
        }
        y += 16.0;
        for (i, label) in ["Slide 1", "Slide 2", "Slide 3"].iter().enumerate() {
            let row = egui::Rect::from_min_size(egui::pos2(s.left() + 6.0, y + i as f32 * 24.0), egui::vec2(side_w - 12.0, 20.0));
            if i == 0 { painter.rect_filled(row, 4.0, c32(pal.accent)); }
            text(row.left_center() + egui::vec2(8.0, 0.0), egui::Align2::LEFT_CENTER, label, 11.0, if i == 0 { pal.accent_ink } else { pal.fg });
        }
        main = if p.sidebar == SidebarSide::Left { egui::Rect::from_min_max(egui::pos2(rect.left() + side_w, rect.top()), rect.max) } else { egui::Rect::from_min_max(rect.min, egui::pos2(rect.right() - side_w, rect.bottom())) };
    }
    if top_h > 0.0 {
        let t = egui::Rect::from_min_size(main.min, egui::vec2(main.width(), top_h));
        painter.rect_filled(t, 0.0, c32(pal.panel));
        // A small logo, then the title.
        let mut tx = t.left() + 10.0;
        if p.logo.is_some() {
            painter.rect_filled(egui::Rect::from_min_size(egui::pos2(tx, t.top() + 6.0), egui::vec2(28.0, 18.0)), 3.0, c32(pal.line));
            text(egui::pos2(tx + 14.0, t.top() + 15.0), egui::Align2::CENTER_CENTER, "logo", 8.0, pal.muted);
            tx += 36.0;
        }
        text(egui::pos2(tx, t.center().y), egui::Align2::LEFT_CENTER, if p.show_title { p.shown_title(project_name) } else { "" }, 12.0 * p.font_scale as f32 / 100.0, pal.fg);
        if p.tab_resources { text(t.right_center() - egui::vec2(10.0, 0.0), egui::Align2::RIGHT_CENTER, "RESOURCES", 9.0, pal.fg); }
    }
    let area = egui::Rect::from_min_max(egui::pos2(main.left() + 10.0, main.top() + top_h + 10.0), egui::pos2(main.right() - 10.0, main.bottom() - bar_h - 10.0));
    painter.rect_filled(area, 2.0, egui::Color32::from_gray(210));
    text(area.center(), egui::Align2::CENTER_CENTER, "SLIDE", 22.0, Rgba(150, 150, 150, 255));
    let bar = egui::Rect::from_min_max(egui::pos2(main.left(), main.bottom() - bar_h), main.max);
    painter.rect_filled(bar, 0.0, c32(pal.panel));
    // Buttons, drawn as the chosen style.
    let glyph = |b: &str| match (p.buttons, b) { (ButtonStyle::Icon, "p") => "\u{25c0}".to_string(), (ButtonStyle::Icon, _) => "\u{25b6}".to_string(), (ButtonStyle::IconText, "p") => "\u{25c0} Previous".to_string(), (ButtonStyle::IconText, _) => "Next \u{25b6}".to_string(), (ButtonStyle::Text, "p") => "Previous".to_string(), (ButtonStyle::Text, _) => "Next".to_string() };
    // Previous, the count and Next sit together at the right of the bar; the controls take the left.
    if proj.show_nav {
        let w = if p.buttons == ButtonStyle::Icon { 28.0 } else { 74.0 };
        let total = 2.0 * w + 12.0 + 44.0;
        let mut x = bar.right() - 12.0 - total;
        for (k, b) in ["p", "c", "n"].iter().enumerate() {
            if *b == "c" {
                text(egui::pos2(x + 20.0, bar.center().y), egui::Align2::CENTER_CENTER, "1 / 3", 11.0, pal.fg);
                x += 44.0;
                continue;
            }
            let r = egui::Rect::from_min_size(egui::pos2(x, bar.top() + 6.0), egui::vec2(w, 22.0));
            painter.rect_filled(r, 6.0, c32(pal.accent));
            text(r.center(), egui::Align2::CENTER_CENTER, &glyph(b), 11.0 * p.font_scale as f32 / 100.0, pal.accent_ink);
            x += w + 6.0;
            let _ = k;
        }
    }
    let mut x = bar.left() + 14.0;
    for (on, label) in [(p.controls.play_pause, "\u{23ef}"), (p.controls.replay, "\u{21ba}"), (p.controls.seekbar, "\u{2014}\u{25cf}\u{2014}\u{2014}"), (p.controls.volume, "\u{1f50a}"), (p.controls.captions, "CC"), (p.controls.speed, "1x"), (p.controls.fullscreen, "\u{26f6}")] {
        if on { text(egui::pos2(x, bar.center().y), egui::Align2::LEFT_CENTER, label, 12.0, pal.fg); x += if label.chars().count() > 2 { 56.0 } else { 30.0 }; }
    }
    if proj.show_progress { painter.rect_filled(egui::Rect::from_min_size(bar.left_top(), egui::vec2(bar.width() * 0.4, 3.0)), 0.0, c32(pal.progress)); }
}

fn resources_pane(ui: &mut egui::Ui, proj: &mut Project, changed: &mut bool, pick: &mut Option<usize>) {
    ui.heading("Resources");
    ui.weak("Links and files learners can open from the Resources link in the top bar.");
    let mut remove: Option<usize> = None;
    for i in 0..proj.resources.len() {
        let attached = proj.resources[i].asset.as_ref().and_then(|k| proj.assets.get(k)).map(|a| a.name.clone());
        let r = &mut proj.resources[i];
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label("Title");
                *changed |= ui.add(egui::TextEdit::singleline(&mut r.title).desired_width(220.0)).changed();
                if ui.small_button("Delete").clicked() { remove = Some(i); }
            });
            ui.horizontal(|ui| {
                match &attached {
                    Some(n) => {
                        ui.label(format!("File: {n}"));
                        if ui.small_button("Change file").clicked() { *pick = Some(i); }
                        if ui.small_button("Use a link instead").clicked() { r.asset = None; *changed = true; }
                    }
                    None => {
                        ui.label("Link");
                        *changed |= ui.add(egui::TextEdit::singleline(&mut r.url).hint_text("https://").desired_width(220.0)).changed();
                        if ui.small_button("Attach a file").clicked() { *pick = Some(i); }
                    }
                }
            });
        });
    }
    if let Some(i) = remove { proj.resources.remove(i); *changed = true; }
    if ui.button("+ Add resource").clicked() { proj.resources.push(Resource::default()); *changed = true; }
    ui.add_space(10.0);
    ui.heading("Glossary");
    ui.weak("Terms and what they mean, listed alphabetically.");
    let mut remove: Option<usize> = None;
    for i in 0..proj.glossary.len() {
        let g = &mut proj.glossary[i];
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label("Term");
                *changed |= ui.add(egui::TextEdit::singleline(&mut g.term).desired_width(220.0)).changed();
                if ui.small_button("Delete").clicked() { remove = Some(i); }
            });
            *changed |= ui.add(egui::TextEdit::multiline(&mut g.definition).hint_text("Definition").desired_rows(2).desired_width(f32::INFINITY)).changed();
        });
    }
    if let Some(i) = remove { proj.glossary.remove(i); *changed = true; }
    if ui.button("+ Add term").clicked() { proj.glossary.push(GlossaryTerm::default()); *changed = true; }
}
