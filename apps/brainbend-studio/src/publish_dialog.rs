//! The Publish dialog: every way to publish a project in one place, in the style of Storyline's Publish window. A list of
//! destinations on the left, the chosen destination's options on the right, Publish and Cancel along the bottom.
//!
//! Destinations that do not exist yet (BrainBend Review, BrainBend LMS, Video) are listed and greyed with a Coming soon
//! tag, so they show up as features to come.
use super::*;
use super::ribbon::{icons, on_primary, primary, tint, RIBBON_RADIUS};
use brainbend_publish_html::{HandoutOrientation, Lms};

const PRINTER: char = '\u{e3dc}';
const FILE_PDF: char = '\u{e702}';

/// Which slides the PDF handout holds.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum HandoutRange {
    #[default]
    All,
    /// The scene the current slide is in.
    Scene,
    /// The slides selected in the slide list (the current one when none are).
    Selected,
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum PublishPage {
    Review,
    BrainBendLms,
    OtherLms,
    Video,
    #[default]
    Web,
    Pdf,
    Print,
    Word,
    PowerPoint,
}

impl PublishPage {
    /// In the order the list shows them.
    pub const ALL: [PublishPage; 9] = [Self::Review, Self::BrainBendLms, Self::OtherLms, Self::Video, Self::Web, Self::Pdf, Self::Print, Self::Word, Self::PowerPoint];

    pub fn label(self) -> &'static str {
        match self {
            Self::Review => "BrainBend Review",
            Self::BrainBendLms => "BrainBend LMS",
            Self::OtherLms => "Other LMS",
            Self::Video => "Video",
            Self::Web => "Web",
            Self::Pdf => "PDF",
            Self::Print => "Print",
            Self::Word => "Word",
            Self::PowerPoint => "PowerPoint",
        }
    }

    fn icon(self) -> char {
        match self {
            Self::Review => ribbon::CHAT,
            Self::BrainBendLms => ribbon::BOOK_OPEN,
            Self::OtherLms => ribbon::STACK,
            Self::Video => ribbon::FILM,
            Self::Web => ribbon::GLOBE,
            Self::Pdf => FILE_PDF,
            Self::Print => PRINTER,
            Self::Word => ribbon::TEXT_AA,
            Self::PowerPoint => ribbon::FRAME,
        }
    }

    /// Whether this version of BrainBend Studio can publish there.
    pub fn available(self) -> bool {
        !matches!(self, Self::Review | Self::BrainBendLms | Self::Video)
    }

    /// What the destination is for.
    fn blurb(self) -> &'static str {
        match self {
            Self::Review => "Share a link so customers can review a project in their browser and leave comments, without installing anything. Projects are kept in BrainBend cloud storage.",
            Self::BrainBendLms => "Publish straight to a BrainBend LMS, where learners take the course and their results are tracked.",
            Self::OtherLms => "Make a SCORM package for any learning management system. Upload the .zip file to your LMS.",
            Self::Video => "Turn the lesson into a video file that plays anywhere. This needs a video encoder that is not part of BrainBend Studio yet.",
            Self::Web => "Publish an HTML5 version of the project to a folder, ready to put on any web server or open from your computer.",
            Self::Pdf => "Save a PDF handout of the slides. Question slides look as they do in the lesson, with the correct answers marked only if you choose to include them. Video and audio show as a placeholder.",
            Self::Print => "Print the handout. It opens in your browser, which shows its print dialog: choose a printer, or \"Save as PDF\". It has the same choices as PDF. A Web Lesson prints from here too.",
            Self::Word => "Make a Word (.docx) handout for people who want to edit it or add to it. The page is portrait, with one slide and its notes, or two slides to a page. Question slides look as they do in the lesson, with the correct answers marked only if you choose to include them.",
            Self::PowerPoint => "Save the project as a PowerPoint (.pptx) file. Things PowerPoint cannot do, such as triggers and variables, are listed in a report afterwards.",
        }
    }

    /// What pressing Publish asks you for next.
    fn next_step(self) -> &'static str {
        match self {
            Self::Web => "You choose an empty folder when you press Publish.",
            Self::OtherLms => "You choose where to save the .zip file when you press Publish.",
            Self::Pdf => "You choose where to save the .pdf file when you press Publish.",
            Self::Print => "The handout opens in your browser when you press Publish.",
            Self::PowerPoint | Self::Word => "You choose where to save the file when you press Publish.",
            _ => "",
        }
    }
}

impl App {
    /// The indices of the slides the handout holds, for the range the author chose.
    pub(super) fn handout_slides(&self) -> Vec<usize> {
        let Some(p) = &self.project else { return vec![] };
        match self.handout_range {
            HandoutRange::All => (0..p.slides.len()).collect(),
            HandoutRange::Scene => p.scene_slides(self.current_scene()),
            HandoutRange::Selected => self.slide_selection(),
        }
    }

    pub(super) fn publish_window(&mut self, ctx: &egui::Context) {
        if !self.publish_open { return; }
        let Some(p) = self.project.as_ref() else { self.publish_open = false; return };
        let is_web = p.is_web();
        let facts: Vec<(&str, String)> = {
            let mut v = vec![("Project", p.name.clone()), ("Type", p.kind.label().to_string())];
            if !is_web { v.push(("Slides", p.slides.len().to_string())); }
            v.push(("Pass mark", format!("{}%", p.pass_mark)));
            v
        };
        // PowerPoint, PDF and Word are made from slides, so a Web Lesson does not offer them; it can still be printed.
        let pages: Vec<PublishPage> = PublishPage::ALL.into_iter().filter(|pg| !(is_web && matches!(pg, PublishPage::PowerPoint | PublishPage::Pdf | PublishPage::Word))).collect();
        if !pages.contains(&self.publish_page) { self.publish_page = PublishPage::Web; }
        let mut page = self.publish_page;
        let mut lms = self.publish_lms;
        let mut handout = self.handout;
        let mut range = self.handout_range;
        let counts = (p.slides.len(), p.scene_slides(self.current_scene()).len(), self.slide_selection().len());
        // Which fonts travel is worked out once per opening: it reads the font files.
        let fonts_id = egui::Id::new("publish-fonts-report");
        let fonts: brainbend_publish_html::FontsReport = match ctx.data(|d| d.get_temp::<brainbend_publish_html::FontsReport>(fonts_id)) {
            Some(r) => r,
            None => { let r = brainbend_publish_html::fonts_report(p); ctx.data_mut(|d| d.insert_temp(fonts_id, r.clone())); r }
        };
        let (mut go, mut cancel) = (false, false);
        let modal = egui::Modal::new(egui::Id::new("publish-dialog")).show(ctx, |ui| {
            // A fixed size: a dialog that resizes itself as its contents change moves under the pointer.
            ui.set_width(780.0);
            ui.set_height(620.0);
            ui.heading("Publish");
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(egui::vec2(240.0, 470.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    for pg in &pages { destination_row(ui, *pg, &mut page); }
                });
                ui.separator();
                ui.allocate_ui_with_layout(egui::vec2(500.0, 470.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(500.0);
                    egui::ScrollArea::vertical().id_salt("publish-right").auto_shrink([false, false]).show(ui, |ui| {
                        ui.label(egui::RichText::new(page.label()).size(18.0).strong());
                        ui.add_space(8.0);
                        ui.label(page.blurb());
                        ui.add_space(12.0);
                        if !page.available() {
                            ui.label(egui::RichText::new("Coming soon").strong().color(primary(ui)));
                            ui.weak("This option is not available in this version of BrainBend Studio.");
                            return;
                        }
                        if page == PublishPage::OtherLms {
                            ui.label(egui::RichText::new("Standard").strong());
                            ui.radio_value(&mut lms, Lms::Scorm12, "SCORM 1.2 (most LMSs, Canvas)");
                            ui.radio_value(&mut lms, Lms::Scorm2004, "SCORM 2004 4th Edition");
                            ui.add_space(12.0);
                        }
                        if matches!(page, PublishPage::Pdf | PublishPage::Print | PublishPage::Word) {
                            ui.label(egui::RichText::new("Page").strong());
                            if page == PublishPage::Word {
                                // Word cannot fit content to a page the way PDF can, so it offers a page that is always portrait,
                                // with one slide and its notes, or two slides and nothing else.
                                handout.orientation = HandoutOrientation::Portrait;
                                handout.per_page = handout.per_page.clamp(1, 2);
                                ui.horizontal(|ui| {
                                    ui.label("Slides per page");
                                    for n in [1u8, 2] { ui.radio_value(&mut handout.per_page, n, if n == 1 { "1 slide".to_string() } else { format!("{n} slides") }); }
                                });
                                ui.weak("Portrait.");
                            } else {
                                ui.horizontal(|ui| {
                                    for o in HandoutOrientation::ALL { ui.radio_value(&mut handout.orientation, o, o.label()); }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Slides per page");
                                    for n in brainbend_publish_html::HandoutOptions::PER_PAGE { ui.radio_value(&mut handout.per_page, n, if n == 1 { "1 slide".to_string() } else { format!("{n} slides") }); }
                                });
                            }
                            ui.add_space(8.0);
                            if !is_web {
                                ui.label(egui::RichText::new("Slides to include").strong());
                                ui.radio_value(&mut range, HandoutRange::All, format!("All slides ({})", counts.0));
                                ui.radio_value(&mut range, HandoutRange::Scene, format!("The current scene ({})", counts.1));
                                ui.radio_value(&mut range, HandoutRange::Selected, format!("The selected slides ({})", counts.2));
                                ui.add_space(8.0);
                            }
                            ui.label(egui::RichText::new("Include").strong());
                            ui.checkbox(&mut handout.correct_answers, "Correct answers").on_hover_text("Off prints question slides as a blank worksheet");
                            if page != PublishPage::Word {
                                ui.checkbox(&mut handout.note_lines, "Space for notes").on_hover_text("Lined space beside or under each slide to write on");
                                ui.checkbox(&mut handout.slide_notes, "Slide notes").on_hover_text("The notes written for each slide");
                                ui.checkbox(&mut handout.alt_text, "Alt text").on_hover_text("The descriptions of each slide's pictures and shapes");
                                ui.weak("Slide sizes, borders and spacing are chosen for you to fit the page.");
                            } else if handout.per_page == 2 {
                                handout.note_lines = false; handout.slide_notes = false; handout.alt_text = false;
                                ui.weak("Two slides to a page leave no room for notes, alt text or lines to write on.");
                            } else {
                                // One slide to a page: with its notes and alt text under it, or just the slide. No lines to
                                // write on: in Word the reader can type their own notes.
                                handout.note_lines = false;
                                let mut with_notes = handout.slide_notes || handout.alt_text;
                                let before = with_notes;
                                ui.radio_value(&mut with_notes, false, "Just the slides");
                                ui.radio_value(&mut with_notes, true, "Slide notes and alt text under each slide");
                                if with_notes != before { handout.slide_notes = with_notes; handout.alt_text = with_notes; }
                                if with_notes {
                                    ui.indent("word-notes", |ui| {
                                        ui.checkbox(&mut handout.slide_notes, "Slide notes").on_hover_text("The notes written for each slide");
                                        ui.checkbox(&mut handout.alt_text, "Alt text").on_hover_text("The descriptions of each slide's pictures and shapes");
                                    });
                                }
                                ui.weak("Long notes run on to the next page. In Word the slides stay editable around the pictures.");
                            }
                            ui.add_space(12.0);
                        }
                        ui.weak(page.next_step());
                        ui.add_space(16.0);
                        ui.label(egui::RichText::new("Properties").strong());
                        ui.add_space(4.0);
                        egui::Grid::new("publish-properties").num_columns(2).spacing([24.0, 6.0]).show(ui, |ui| {
                            for (k, v) in &facts { ui.label(egui::RichText::new(*k).weak()); ui.label(v); ui.end_row(); }
                            if page == PublishPage::OtherLms {
                                ui.label(egui::RichText::new("Format").weak());
                                ui.label(if lms == Lms::Scorm12 { "SCORM 1.2" } else { "SCORM 2004 4th Edition" });
                                ui.end_row();
                            }
                        });
                        if matches!(page, PublishPage::Web | PublishPage::OtherLms | PublishPage::Pdf | PublishPage::Print) {
                            ui.add_space(14.0);
                            ui.label(egui::RichText::new("Fonts").strong());
                            ui.add_space(4.0);
                            if !fonts.embedded.is_empty() {
                                ui.label(format!("Included with the lesson: {}.", fonts.embedded.join(", ")));
                            }
                            if !fonts.fallback.is_empty() {
                                ui.colored_label(ui.visuals().warn_fg_color, format!("Not included: {}.", fonts.fallback.join(", ")));
                                ui.weak("These are not installed here, or their license does not allow embedding. Learners see them only if they have the font; otherwise a similar one. Choose another font, or add it from Google Fonts, to be sure.");
                            }
                        }
                    });
                });
            });
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let publish = egui::Button::new(egui::RichText::new("Publish").strong().color(on_primary(ui))).fill(primary(ui)).corner_radius(RIBBON_RADIUS as u8).min_size(egui::vec2(120.0, 34.0));
                let enabled = page.available();
                if ui.add_enabled(enabled, publish).on_disabled_hover_text("Coming soon").clicked() { go = true; }
                if ui.add(egui::Button::new("Cancel").corner_radius(RIBBON_RADIUS as u8).min_size(egui::vec2(100.0, 34.0))).clicked() { cancel = true; }
            });
        });
        self.publish_page = page;
        self.publish_lms = lms;
        self.handout = handout;
        self.handout_range = range;
        if modal.should_close() || cancel || go { ctx.data_mut(|d| d.remove_temp::<brainbend_publish_html::FontsReport>(fonts_id)); }
        if modal.should_close() || cancel { self.publish_open = false; }
        if go && page.available() {
            self.publish_open = false;
            match page {
                PublishPage::Web => self.export_html(),
                PublishPage::OtherLms => self.export_scorm(lms),
                PublishPage::Pdf => self.export_pdf(),
                PublishPage::Print => self.print(),
                PublishPage::PowerPoint => self.export_pptx(),
                PublishPage::Word => self.export_word(),
                PublishPage::Review | PublishPage::BrainBendLms | PublishPage::Video => {}
            }
        }
    }
}

/// One destination in the list: its icon and name, with a Coming soon tag when it cannot be used yet.
fn destination_row(ui: &mut egui::Ui, pg: PublishPage, selected: &mut PublishPage) {
    let on = *selected == pg;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 42.0), egui::Sense::click());
    let name = if pg.available() { pg.label().to_string() } else { format!("{} (coming soon)", pg.label()) };
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, on, &name));
    let text = ui.visuals().text_color();
    if on || resp.hovered() {
        ui.painter().rect_filled(rect, RIBBON_RADIUS, text.gamma_multiply(if on { 0.14 } else { 0.07 }));
    }
    if on { ui.painter().rect_filled(egui::Rect::from_min_size(rect.min + egui::vec2(0.0, 8.0), egui::vec2(3.0, 26.0)), 1.0, primary(ui)); }
    let icon_col = if pg.available() { tint(pg.icon()) } else { tint(pg.icon()).gamma_multiply(0.45) };
    let ink = if pg.available() { text } else { text.gamma_multiply(0.55) };
    ui.painter().text(egui::pos2(rect.left() + 18.0, rect.center().y), egui::Align2::LEFT_CENTER, pg.icon(), egui::FontId::new(20.0, icons()), icon_col);
    ui.painter().text(egui::pos2(rect.left() + 52.0, rect.center().y), egui::Align2::LEFT_CENTER, pg.label(), egui::FontId::proportional(15.0), ink);
    if !pg.available() {
        ui.painter().text(egui::pos2(rect.right() - 10.0, rect.center().y), egui::Align2::RIGHT_CENTER, "Soon", egui::FontId::proportional(11.5), text.gamma_multiply(0.5));
    }
    if resp.clicked() { *selected = pg; }
}
