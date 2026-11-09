//! Project checks: accessibility and consistency problems an author should fix before publishing.
use crate::*;
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Error,
    Warning,
}

/// Where an issue is, so the editor can jump to it.
#[derive(Clone, Debug, PartialEq)]
pub struct Location {
    pub slide: usize,
    /// (layer index, object index) when the issue is about one object.
    pub object: Option<(usize, usize)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Issue {
    pub severity: Severity,
    pub at: Location,
    pub message: String,
}

/// WCAG relative luminance of an sRGB color.
fn luminance(c: Rgba) -> f32 {
    let f = |v: u8| {
        let s = v as f32 / 255.0;
        if s <= 0.03928 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * f(c.0) + 0.7152 * f(c.1) + 0.0722 * f(c.2)
}

/// WCAG contrast ratio (1.0 to 21.0).
pub fn contrast_ratio(a: Rgba, b: Rgba) -> f32 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// What is behind the center of object `(li, oi)`: the fill of the topmost earlier object that
/// covers that point (base layer first, then the object's own layer), else the slide background.
/// `None` means an image or video is behind it, whose colors we cannot know.
fn backdrop(slide: &Slide, bg: Rgba, li: usize, oi: usize) -> Option<Rgba> {
    let me = &slide.layers[li].objects[oi];
    let (cx, cy) = (me.x + me.w / 2.0, me.y + me.h / 2.0);
    let mut under: Vec<&Object> = Vec::new();
    if li > 0 {
        under.extend(slide.layers[0].objects.iter());
    }
    under.extend(slide.layers[li].objects[..oi].iter());
    let mut found: Option<Option<Rgba>> = None;
    for o in under {
        if o.hidden || cx < o.x || cx > o.x + o.w || cy < o.y || cy > o.y + o.h {
            continue;
        }
        match o.kind {
            Kind::Image | Kind::Video => found = Some(None),
            _ => {
                if let Some(f) = o.fill {
                    found = Some(Some(blend_over_white(f)));
                }
            }
        }
    }
    match found {
        Some(c) => c,
        None => Some(bg),
    }
}

fn blend_over_white(c: Rgba) -> Rgba {
    let a = c.3 as f32 / 255.0;
    let m = |v: u8| (v as f32 * a + 255.0 * (1.0 - a)).round() as u8;
    Rgba(m(c.0), m(c.1), m(c.2), 255)
}

impl Project {
    /// Problems with one question: empty text, too few choices, missing answers or media.
    pub(crate) fn check_question(&self, q: &Question, slide_at: &Location, out: &mut Vec<Issue>) {

                if q.prompt.trim().is_empty() {
                    out.push(Issue { severity: Severity::Error, at: slide_at.clone(), message: "The question has no text.".into() });
                }
                let err = |m: &str| Issue { severity: Severity::Error, at: slide_at.clone(), message: m.into() };
                match &q.style {
                    QuestionStyle::Choice => {
                        if q.choices.len() < 2 { out.push(err("A question needs at least two choices.")); }
                        if !q.choices.iter().any(|c| c.correct) { out.push(err("No choice is marked correct.")); }
                        if q.choices.iter().any(|c| c.text.trim().is_empty()) {
                            out.push(Issue { severity: Severity::Warning, at: slide_at.clone(), message: "A choice has no text.".into() });
                        }
                    }
                    QuestionStyle::TextEntry { accepted, .. } => {
                        if accepted.iter().all(|a| a.trim().is_empty()) { out.push(err("The fill-in-the-blank question has no accepted answer.")); }
                    }
                    QuestionStyle::Matching { pairs } => {
                        if pairs.len() < 2 { out.push(err("A matching question needs at least two pairs.")); }
                        if pairs.iter().any(|(a, b)| a.trim().is_empty() || b.trim().is_empty()) { out.push(err("A matching pair is missing text.")); }
                    }
                    QuestionStyle::Sequence { items } => {
                        if items.len() < 2 { out.push(err("A sequencing question needs at least two items.")); }
                        if items.iter().any(|i| i.trim().is_empty()) { out.push(err("A sequence item has no text.")); }
                    }
                    QuestionStyle::FillBlanks { text, .. } => {
                        if crate::passage::blanks(text).is_empty() { out.push(err("The fill-in-the-blanks passage has no blank. Wrap each answer in *stars*.")); }
                    }
                    QuestionStyle::DragWords { text, distractors } => {
                        if crate::passage::blanks(text).is_empty() { out.push(err("The drag-the-words passage has no blank. Wrap each answer in *stars*.")); }
                        if distractors.iter().any(|d| d.trim().is_empty()) { out.push(err("A spare word in the word bank is empty.")); }
                    }
                    QuestionStyle::MarkWords { text } => {
                        if crate::passage::blanks(text).is_empty() { out.push(err("No word is marked correct. Wrap each correct word in *stars*.")); }
                    }
                    QuestionStyle::Numeric { answer, tolerance } => {
                        if !answer.is_finite() || !tolerance.is_finite() || *tolerance < 0.0 { out.push(err("The numeric question needs a number and a tolerance of zero or more.")); }
                    }
                    QuestionStyle::Hotspot { asset, alt, regions } => {
                        if !asset.as_ref().is_some_and(|a| self.assets.contains_key(a)) { out.push(err("The hotspot question has no picture.")); }
                        if regions.is_empty() { out.push(err("The hotspot question has no correct area. Drag on the picture to draw one.")); }
                        if alt.trim().is_empty() {
                            out.push(Issue { severity: Severity::Warning, at: slide_at.clone(), message: "The hotspot picture has no description for screen readers.".into() });
                        }
                    }
                    QuestionStyle::Likert { statements, scale, total_variable, .. } => {
                        if let Some(v) = total_variable {
                            if !self.variables.iter().any(|x| &x.id == v && matches!(x.initial, Value::Num(_))) { out.push(err("The survey keeps its total in a variable that no longer exists or is not a number.")); }
                        }
                        if statements.is_empty() || statements.iter().any(|t| t.trim().is_empty()) { out.push(err("A survey statement is missing or empty.")); }
                        if scale.len() < 2 || scale.iter().any(|t| t.trim().is_empty()) { out.push(err("The survey scale needs at least two labelled points.")); }
                    }
                    QuestionStyle::Reflection { .. } => {}
                    QuestionStyle::Dictation { asset, text, .. } => {
                        if !asset.as_ref().is_some_and(|a| self.assets.contains_key(a)) { out.push(err("The dictation question has no audio clip.")); }
                        if text.trim().is_empty() { out.push(err("The dictation question has no answer text.")); }
                    }
                    QuestionStyle::Dropdowns { text } => {
                        let blanks = crate::passage::blanks(text);
                        if blanks.is_empty() { out.push(err("The drop-down passage has no blank. Write each as *right/wrong/wrong*.")); }
                        if blanks.iter().any(|b| b.len() < 2) { out.push(err("A drop-down blank needs the right answer and at least one wrong one: *right/wrong*.")); }
                    }
                    QuestionStyle::LabelPicture { asset, alt, zones, distractors } => {
                        if !asset.as_ref().is_some_and(|a| self.assets.contains_key(a)) { out.push(err("The labelling question has no picture.")); }
                        if zones.is_empty() { out.push(err("The labelling question has no labelled area. Drag on the picture to draw one.")); }
                        if zones.iter().any(|z| z.label.trim().is_empty()) || distractors.iter().any(|d| d.trim().is_empty()) { out.push(err("A label is empty.")); }
                        if alt.trim().is_empty() {
                            out.push(Issue { severity: Severity::Warning, at: slide_at.clone(), message: "The labelled picture has no description for screen readers.".into() });
                        }
                    }
                    QuestionStyle::Categorize { categories } => {
                        if categories.len() < 2 { out.push(err("A sorting question needs at least two categories.")); }
                        if categories.iter().any(|(n, items)| n.trim().is_empty() || items.is_empty() || items.iter().any(|i| i.trim().is_empty())) { out.push(err("A category has no name, no items, or an empty item.")); }
                    }
                }
                }

    /// Run every check. Results are ordered by slide, then severity.
    pub fn check(&self) -> Vec<Issue> {
        if self.is_web() { return self.check_lesson(); }
        let mut out = Vec::new();
        let var_ids: HashSet<&str> = self.variables.iter().map(|v| v.id.as_str()).collect();
        let slide_ids: HashSet<&str> = self.slides.iter().map(|s| s.id.as_str()).collect();
        let has_questions = self.slides.iter().any(|s| matches!(s.kind, SlideKind::Question(_)));
        let has_results = self.slides.iter().any(|s| matches!(s.kind, SlideKind::Results));
        if has_questions && !has_results {
            out.push(Issue {
                severity: Severity::Warning,
                at: Location { slide: self.slides.len().saturating_sub(1), object: None },
                message: "The project has questions but no results slide, so learners never see their score.".into(),
            });
        }
        for (si, slide) in self.slides.iter().enumerate() {
            let slide_at = Location { slide: si, object: None };
            if let SlideKind::Question(q) = &slide.kind {
                self.check_question(q, &slide_at, &mut out);
            }
            let layer_ids: HashSet<&str> = slide.layers.iter().map(|l| l.id.as_str()).collect();
            let object_ids: HashSet<&str> = slide.layers.iter().flat_map(|l| l.objects.iter()).map(|o| o.id.as_str()).collect();
            let clickable: HashSet<&str> = slide
                .triggers
                .iter()
                .filter(|t| matches!(t.event, Event::Click | Event::DoubleClick))
                .filter_map(|t| if let Owner::Object(o) = &t.owner { Some(o.as_str()) } else { None })
                .collect();
            let owners_of = |wanted: fn(&Event) -> bool| -> HashSet<&str> {
                slide.triggers.iter().filter(|t| wanted(&t.event)).filter_map(|t| if let Owner::Object(o) = &t.owner { Some(o.as_str()) } else { None }).collect()
            };
            let double_click = owners_of(|e| matches!(e, Event::DoubleClick));
            let single_click = owners_of(|e| matches!(e, Event::Click));
            // Words laid over something clickable: a click on the words does not reach it, and a screen reader reads two things.
            for (li, layer) in slide.layers.iter().enumerate() {
                for (ti, t) in layer.objects.iter().enumerate() {
                    if t.hidden || t.text.trim().is_empty() || clickable.contains(t.id.as_str()) || t.has_button_states() { continue; }
                    let area = (t.w * t.h).max(1.0);
                    for c in layer.objects.iter().take(ti).filter(|c| !c.hidden && c.id != t.id && (clickable.contains(c.id.as_str()) || c.has_button_states()) && !(c.group.is_some() && c.group == t.group && c.table.is_none())) {
                        let (w, h) = ((t.x + t.w).min(c.x + c.w) - t.x.max(c.x), (t.y + t.h).min(c.y + c.h) - t.y.max(c.y));
                        if w > 0.0 && h > 0.0 && w * h / area >= 0.5 {
                            out.push(Issue { severity: Severity::Warning, at: Location { slide: si, object: Some((li, ti)) }, message: format!("\"{}\" sits on top of \"{}\", which is clickable, so clicking the words does not press it and screen readers read two things. Put the words in \"{}\" itself and delete this text box.", t.name, c.name, c.name) });
                            break;
                        }
                    }
                }
            }
            // A group is one control: a click trigger on one part speaks for all, so a second owner is a mistake.
            for (li, layer) in slide.layers.iter().enumerate() {
                let mut seen: HashSet<&str> = HashSet::new();
                for o in layer.objects.iter().filter(|o| o.table.is_none()) {
                    let Some(g) = o.group.as_deref() else { continue };
                    if !seen.insert(g) { continue; }
                    let owners: Vec<&str> = slide.group_members(li, g).into_iter().map(|k| &layer.objects[k]).filter(|m| slide.has_click_trigger(&m.id)).map(|m| m.name.as_str()).collect();
                    if owners.len() > 1 {
                        out.push(Issue { severity: Severity::Warning, at: Location { slide: si, object: None }, message: format!("The group with {} has click triggers on more than one of its parts ({}). A click anywhere in the group runs them all, and screen readers find several buttons. Keep the triggers on one part.", owners[0], owners.join(", ")) });
                    }
                }
            }
            // Headings give screen-reader users the outline of the slide. The project's name is the page's Heading 1 and
            // the slide's own (hidden) title a Heading 2, so the first one an author adds may be a 2 or a 3.
            let mut last_level = 2u8;
            for (li, layer) in slide.layers.iter().enumerate() {
                for (oi, o) in layer.objects.iter().enumerate() {
                    let Some(level) = o.role.level().filter(|_| !o.hidden && !o.decorative) else { continue };
                    let at = Location { slide: si, object: Some((li, oi)) };
                    if level == 1 {
                        out.push(Issue { severity: Severity::Warning, at: at.clone(), message: format!("\"{}\" is a Heading 1, which is the project's name. Use Heading 2 or lower.", o.name) });
                    } else if level > last_level + 1 {
                        out.push(Issue { severity: Severity::Warning, at: at.clone(), message: format!("\"{}\" is Heading {} but follows Heading {}. Headings should not skip a level.", o.name, level, last_level) });
                    }
                    if level > 1 { last_level = level; }
                }
            }
            for (li, layer) in slide.layers.iter().enumerate() {
                for (oi, o) in layer.objects.iter().enumerate() {
                    if o.hidden { continue; }
                    let at = Location { slide: si, object: Some((li, oi)) };
                    // Nothing in a course should depend on a gesture a keyboard cannot make.
                    if double_click.contains(o.id.as_str()) && !single_click.contains(o.id.as_str()) {
                        out.push(Issue { severity: Severity::Warning, at: at.clone(), message: format!("\"{}\" reacts only to a double-click, which keyboard and switch users cannot do. Use a click trigger.", o.name) });
                    }
                    if let Some(e) = &o.emphasis {
                        if e.kind == EmphasisKind::Flash && e.duration_ms < 334 && (e.repeat == 0 || e.repeat >= 3) {
                            out.push(Issue { severity: Severity::Warning, at: at.clone(), message: format!("\"{}\" flashes more than three times a second, which can trigger seizures. Make the flash last at least a third of a second.", o.name) });
                        } else if e.repeat == 0 {
                            out.push(Issue { severity: Severity::Warning, at: at.clone(), message: format!("\"{}\" keeps moving until the slide ends and learners cannot stop it. Give it a number of repeats.", o.name) });
                        }
                    }
                    if let Some(m) = &o.marker {
                        if m.title.trim().is_empty() && !o.decorative { out.push(Issue { severity: Severity::Warning, at: at.clone(), message: format!("Marker \"{}\" has no title, so screen readers cannot say what it is.", o.name) }); }
                        if m.title.trim().is_empty() && m.description.trim().is_empty() { out.push(Issue { severity: Severity::Warning, at: at.clone(), message: format!("Marker \"{}\" shows nothing: give it a title or a description.", o.name) }); }
                    }
                    if let Kind::Input(f) = &o.kind {
                        let want_num = f.numeric;
                        if !self.variables.iter().any(|v| v.id == f.variable && matches!(v.initial, Value::Num(_)) == want_num && !matches!(v.initial, Value::Bool(_))) {
                            out.push(Issue { severity: Severity::Warning, at: at.clone(), message: format!("{} \"{}\" is not linked to a {} variable, so what is typed is lost.", if want_num { "Number entry" } else { "Text entry" }, o.name, if want_num { "number" } else { "text" }) });
                        }
                        if o.alt.trim().is_empty() && !o.decorative { out.push(Issue { severity: Severity::Warning, at: at.clone(), message: format!("Entry field \"{}\" has no label for screen readers. Describe what to type in its alt text.", o.name) }); }
                    }
                    if let Some(g) = o.check.as_ref().and_then(|c| c.radio.as_ref()) {
                        let same = slide.layers.iter().flat_map(|l| l.objects.iter()).filter(|x| x.check.as_ref().and_then(|c| c.radio.as_ref()) == Some(g)).count();
                        if same < 2 { out.push(Issue { severity: Severity::Warning, at: at.clone(), message: format!("Radio button \"{}\" is the only one in its group \"{}\", so it can only ever be chosen. Add the other choices to the same group.", o.name, g) }); }
                    }
                    if let Kind::Web(w) = &o.kind {
                        let u = w.url.trim();
                        if u.is_empty() { out.push(Issue { severity: Severity::Warning, at: at.clone(), message: format!("Web object \"{}\" has no web address.", o.name) }); }
                        else if w.safe_url().is_none() { out.push(Issue { severity: Severity::Warning, at: at.clone(), message: format!("Web object \"{}\" has an address that does not start with https://, so it will not show.", o.name) }); }
                        else if u.starts_with("http://") { out.push(Issue { severity: Severity::Warning, at: at.clone(), message: format!("Web object \"{}\" uses http://, which most browsers block inside an https:// course. Use an https:// address.", o.name) }); }
                        if o.alt.trim().is_empty() && !o.decorative { out.push(Issue { severity: Severity::Warning, at: at.clone(), message: format!("Web object \"{}\" has no title for screen readers. Describe it in its alt text.", o.name) }); }
                    }
                    if let Kind::Pano(p) = &o.kind {
                        if o.asset.is_none() { out.push(Issue { severity: Severity::Warning, at: at.clone(), message: format!("360° image \"{}\" has no picture.", o.name) }); }
                        if o.alt.trim().is_empty() && !o.decorative { out.push(Issue { severity: Severity::Warning, at: at.clone(), message: format!("360° image \"{}\" has no description for screen readers. Describe the scene in its alt text.", o.name) }); }
                        for pin in &p.pins {
                            if pin.title.trim().is_empty() { out.push(Issue { severity: Severity::Warning, at: at.clone(), message: format!("An annotation on 360° image \"{}\" has no title, so screen reader users cannot tell what it is.", o.name) }); }
                        }
                    }
                    let is_button = o.has_button_states() || clickable.contains(o.id.as_str()) || matches!(o.kind, Kind::Hotspot);
                    // Part of a clickable group: the group is the control and is named as a whole, so this part is not on its own.
                    let in_group = slide.group_role(li, oi);
                    match o.kind {
                        Kind::Image if o.alt.trim().is_empty() && !is_button && in_group.is_none() && !o.decorative => out.push(Issue {
                            severity: Severity::Warning,
                            at: at.clone(),
                            message: format!("Image \"{}\" has no alt text (describe it, or leave it only if purely decorative).", o.name),
                        }),
                        Kind::Video => {
                            if o.captions.is_none() {
                                out.push(Issue { severity: Severity::Warning, at: at.clone(), message: format!("Video \"{}\" has no captions.", o.name) });
                            }
                            if o.alt.trim().is_empty() {
                                out.push(Issue { severity: Severity::Warning, at: at.clone(), message: format!("Video \"{}\" has no description.", o.name) });
                            }
                        }
                        Kind::Audio if o.alt.trim().is_empty() => out.push(Issue {
                            severity: Severity::Warning,
                            at: at.clone(),
                            message: format!("Audio \"{}\" has no description or transcript note.", o.name),
                        }),
                        _ => {}
                    }
                    // Sliders and dials set a number variable, which has to exist and have a range.
                    let control = match &o.kind {
                        Kind::Slider(sl) => Some(("Slider", sl.variable.as_str(), sl.min, sl.max)),
                        _ => o.dial.as_ref().map(|d| ("Dial", d.variable.as_str(), d.min, d.max)),
                    };
                    if let Some((what, var, min, max)) = control {
                        if !self.variables.iter().any(|v| v.id == var && matches!(v.initial, Value::Num(_))) {
                            out.push(Issue { severity: Severity::Warning, at: at.clone(), message: format!("{what} \"{}\" is not linked to a number variable, so it sets nothing.", o.name) });
                        } else if max <= min {
                            out.push(Issue { severity: Severity::Warning, at: at.clone(), message: format!("{what} \"{}\" ends its range at or before where it starts.", o.name) });
                        }
                    }
                    if is_button && o.text.trim().is_empty() && o.alt.trim().is_empty() && !in_group.is_some_and(|g| g.has_text) {
                        out.push(Issue {
                            severity: Severity::Error,
                            at: at.clone(),
                            message: format!("\"{}\" is clickable but has no text or alt text, so screen readers cannot name it.", o.name),
                        });
                    }
                    if !o.text.trim().is_empty() && !matches!(o.kind, Kind::Image | Kind::Video | Kind::Audio | Kind::Hotspot) {
                        // Own fill if it has one; otherwise whatever is behind the text.
                        let bg = match o.fill {
                            Some(f) => Some(blend_over_white(f)),
                            None => backdrop(slide, self.resolved_background(slide).map(|b| b.base_color()).unwrap_or(Rgba(255, 255, 255, 255)), li, oi),
                        };
                        let Some(bg) = bg else { continue };
                        let ratio = contrast_ratio(blend_over_white(o.style.color), bg);
                        // Large text (24px or bold 19px, in canvas units) needs only 3:1.
                        let need = if o.style.size >= 24.0 || (o.style.bold && o.style.size >= 19.0) { 3.0 } else { 4.5 };
                        if ratio < need {
                            out.push(Issue {
                                severity: Severity::Warning,
                                at,
                                message: format!("Text in \"{}\" has low contrast ({:.1}:1; {:.1}:1 needed).", o.name, ratio, need),
                            });
                        }
                    }
                }
            }
            for t in &slide.triggers {
                let owner_ok = match &t.owner {
                    Owner::Slide => true,
                    Owner::Layer(l) => layer_ids.contains(l.as_str()),
                    Owner::Object(o) => object_ids.contains(o.as_str()),
                };
                let mut problems: Vec<&str> = Vec::new();
                if !owner_ok { problems.push("belongs to something that no longer exists"); }
                match &t.action {
                    Action::ShowLayer(l) | Action::HideLayer(l) if !layer_ids.contains(l.as_str()) => problems.push("points at a layer that does not exist"),
                    Action::JumpToSlide(s) if !slide_ids.contains(s.as_str()) => problems.push("jumps to a slide that does not exist"),
                    Action::SetVariable { variable, .. } if !var_ids.contains(variable.as_str()) => problems.push("sets a variable that does not exist"),
                    Action::SeekTimeline { target, .. } if !target.is_empty() && !layer_ids.contains(target.as_str()) && !object_ids.contains(target.as_str()) => problems.push("seeks something that does not exist"),
                    Action::SetObjectState { object, .. } if !object_ids.contains(object.as_str()) => problems.push("changes an object that does not exist"),
                    _ => {}
                }
                if let Event::VariableChanged(v) = &t.event {
                    if !var_ids.contains(v.as_str()) { problems.push("watches a variable that does not exist"); }
                }
                if let Event::StateChanged(o, _) = &t.event {
                    if !object_ids.contains(o.as_str()) { problems.push("watches an object that does not exist"); }
                }
                if let Event::PanoPin(o, pin) = &t.event {
                    let found = slide.layers.iter().flat_map(|l| &l.objects).any(|ob| &ob.id == o && matches!(&ob.kind, Kind::Pano(p) if p.pins.iter().any(|q| &q.id == pin)));
                    if !found { problems.push("watches a marker or hotspot that does not exist"); }
                }
                if t.conditions.iter().any(|c| !var_ids.contains(c.variable.as_str())) {
                    problems.push("has a condition on a variable that does not exist");
                }
                for p in problems {
                    out.push(Issue { severity: Severity::Error, at: slide_at.clone(), message: format!("A trigger {p}.") });
                }
            }
        }
        out.sort_by(|a, b| a.at.slide.cmp(&b.at.slide).then(a.severity.cmp(&b.severity)));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> Project {
        let mut p = Project::new("t");
        p.sanitize();
        p
    }

    fn msgs(p: &Project) -> Vec<String> {
        p.check().into_iter().map(|i| i.message).collect()
    }

    fn text(role: TextRole, name: &str) -> Object {
        let mut o = Object::new(name, name, Kind::Text, 0.0, 0.0, 100.0, 40.0);
        o.text = name.into();
        o.role = role;
        o
    }

    #[test]
    fn entry_fields_and_radio_groups_are_checked() {
        let mut p = project();
        p.variables.push(Variable { id: "t".into(), name: "Name".into(), initial: Value::Text(String::new()), built_in: false });
        p.variables.push(Variable { id: "n".into(), name: "Age".into(), initial: Value::Num(0.0), built_in: false });
        let field = |var: &str, numeric: bool, alt: &str| { let mut o = Object::new("f", "Field", Kind::Input(InputField { variable: var.into(), numeric, placeholder: String::new(), min: 0.0, max: 10.0, step: 1.0 }), 0.0, 0.0, 100.0, 40.0); o.alt = alt.into(); o };
        p.slides[0].layers[0].objects = vec![field("t", false, "Your name")];
        assert!(msgs(&p).iter().all(|m| !m.contains("Text entry") && !m.contains("Entry field")), "{:?}", msgs(&p));
        p.slides[0].layers[0].objects = vec![field("n", false, "Your name")];
        assert!(msgs(&p).iter().any(|m| m.contains("not linked to a text variable")), "a text field needs a text variable");
        p.slides[0].layers[0].objects = vec![field("t", true, "Age")];
        assert!(msgs(&p).iter().any(|m| m.contains("not linked to a number variable")), "a number field needs a number variable");
        p.slides[0].layers[0].objects = vec![field("t", false, "")];
        assert!(msgs(&p).iter().any(|m| m.contains("no label for screen readers")));
        let radio = |id: &str, group: &str| { let mut o = Object::new(id, id, Kind::Text, 0.0, 0.0, 100.0, 40.0); o.text = id.into(); o.check = Some(Check { radio: Some(group.into()), ..Check::new() }); o };
        p.slides[0].layers[0].objects = vec![radio("a", "g"), radio("b", "g")];
        assert!(msgs(&p).iter().all(|m| !m.contains("Radio button")), "two in a group is fine");
        p.slides[0].layers[0].objects = vec![radio("a", "g"), radio("b", "h")];
        assert!(msgs(&p).iter().any(|m| m.contains("only one in its group")));
    }

    #[test]
    fn a_360_image_needs_a_picture_a_description_and_titled_annotations() {
        use crate::pano::{Pano, PanoPin};
        let mut p = project();
        let pano = |asset: Option<&str>, alt: &str, titles: &[&str]| {
            let mut cfg = Pano::new();
            for (i, t) in titles.iter().enumerate() { cfg.pins.push({ let mut m = PanoPin::marker(format!("p{i}"), 0.0, 0.0); m.title = (*t).into(); m }); }
            let mut o = Object::new("v", "Lab", Kind::Pano(cfg), 0.0, 0.0, 100.0, 60.0);
            o.asset = asset.map(String::from);
            o.alt = alt.into();
            o
        };
        p.slides[0].layers[0].objects = vec![pano(None, "A lab", &["Door"])];
        assert!(msgs(&p).iter().any(|m| m.contains("has no picture")), "{:?}", msgs(&p));
        p.slides[0].layers[0].objects = vec![pano(Some("pic"), "", &["Door"])];
        assert!(msgs(&p).iter().any(|m| m.contains("no description for screen readers")));
        p.slides[0].layers[0].objects = vec![pano(Some("pic"), "A lab", &["Door", "  "])];
        assert!(msgs(&p).iter().any(|m| m.contains("annotation") && m.contains("has no title")), "{:?}", msgs(&p));
        p.slides[0].layers[0].objects = vec![pano(Some("pic"), "A lab", &["Door", "Hood"])];
        assert!(msgs(&p).iter().all(|m| !m.contains("360")), "{:?}", msgs(&p));
    }

    #[test]
    fn a_trigger_on_a_360_item_needs_a_marker_or_hotspot_that_exists() {
        use crate::pano::{Pano, PanoPin};
        let mut p = project();
        let mut cfg = Pano::new();
        cfg.pins.push({ let mut m = PanoPin::marker("p0".into(), 0.0, 0.0); m.title = "Door".into(); m });
        let mut o = Object::new("v", "Lab", Kind::Pano(cfg), 0.0, 0.0, 100.0, 60.0);
        o.asset = Some("pic".into());
        o.alt = "A lab".into();
        p.slides[0].layers[0].objects = vec![o];
        p.slides[0].triggers = vec![Trigger { id: "t".into(), owner: Owner::Object("v".into()), event: Event::PanoPin("v".into(), "p0".into()), action: Action::NextSlide, conditions: vec![] }];
        assert!(msgs(&p).iter().all(|m| !m.contains("does not exist")), "{:?}", msgs(&p));
        p.slides[0].triggers[0].event = Event::PanoPin("v".into(), "gone".into());
        assert!(msgs(&p).iter().any(|m| m.contains("marker or hotspot that does not exist")), "{:?}", msgs(&p));
    }

    #[test]
    fn web_objects_need_a_safe_address_and_a_title() {
        let mut p = project();
        let mut web = |url: &str, alt: &str| { let mut o = Object::new("w", "Page", Kind::Web(WebObject { url: url.into(), scroll: true }), 0.0, 0.0, 100.0, 100.0); o.alt = alt.into(); o };
        p.slides[0].layers[0].objects = vec![web("", "A page")];
        assert!(msgs(&p).iter().any(|m| m.contains("has no web address")));
        p.slides[0].layers[0].objects = vec![web("javascript:alert(1)", "A page")];
        assert!(msgs(&p).iter().any(|m| m.contains("does not start with https://")), "{:?}", msgs(&p));
        p.slides[0].layers[0].objects = vec![web("http://example.com", "A page")];
        assert!(msgs(&p).iter().any(|m| m.contains("uses http://")));
        p.slides[0].layers[0].objects = vec![web("https://example.com", "")];
        assert!(msgs(&p).iter().any(|m| m.contains("no title for screen readers")));
        p.slides[0].layers[0].objects = vec![web("https://example.com", "Course glossary")];
        assert!(msgs(&p).iter().all(|m| !m.contains("Web object")), "a good one raises nothing: {:?}", msgs(&p));
    }

    #[test]
    fn heading_one_is_the_project_name_and_levels_may_not_be_skipped() {
        let mut p = project();
        p.slides[0].layers[0].objects = vec![text(TextRole::H2, "Intro"), text(TextRole::H3, "Detail"), text(TextRole::H4, "More"), text(TextRole::Normal, "Body")];
        assert!(!msgs(&p).iter().any(|m| m.contains("Intro") || m.contains("Detail") || m.contains("More")), "a 2 or a 3 may follow the slide title: {:?}", msgs(&p));
        p.slides[0].layers[0].objects = vec![text(TextRole::H3, "Starts at three"), text(TextRole::H1, "Main")];
        let m = msgs(&p);
        assert!(!m.iter().any(|m| m.contains("Starts at three")), "{m:?}");
        assert!(m.iter().any(|m| m.contains("\"Main\" is a Heading 1, which is the project's name")), "{m:?}");
        p.slides[0].layers[0].objects = vec![text(TextRole::H2, "Main"), text(TextRole::H4, "Skips")];
        assert!(msgs(&p).iter().any(|m| m.contains("\"Skips\" is Heading 4 but follows Heading 2")));
        p.slides[0].layers[0].objects[1].decorative = true;
        assert!(!msgs(&p).iter().any(|m| m.contains("Skips")), "a decorative heading is not read");
    }

    #[test]
    fn contrast_math_matches_wcag() {
        let black = Rgba(0, 0, 0, 255);
        let white = Rgba(255, 255, 255, 255);
        assert!((contrast_ratio(black, white) - 21.0).abs() < 0.01);
        assert!((contrast_ratio(white, white) - 1.0).abs() < 0.001);
        // #767676 on white is the classic 4.54:1 pass boundary.
        assert!(contrast_ratio(Rgba(0x76, 0x76, 0x76, 255), white) > 4.5);
        assert!(contrast_ratio(Rgba(0x77, 0x77, 0x77, 255), white) < 4.5);
    }

    #[test]
    fn controls_must_be_linked_to_a_number_variable_with_a_range() {
        let mut p = project();
        p.variables.push(Variable { id: "n".into(), name: "Level".into(), initial: Value::Num(0.0), built_in: false });
        p.variables.push(Variable { id: "t".into(), name: "Label".into(), initial: Value::Text(String::new()), built_in: false });
        let slider = |id: &str, var: &str, min: f32, max: f32| Object::new(id, id, Kind::Slider(Slider { variable: var.into(), min, max, ..Slider::new() }), 0.0, 0.0, 100.0, 20.0);
        let mut dial = Object::new("d", "Wheel", Kind::Rect, 0.0, 0.0, 50.0, 50.0);
        dial.dial = Some(Dial { variable: String::new(), min: 0.0, max: 10.0, step: 1.0, sweep: 180.0, pivot: (0.0, 0.0) });
        p.slides[0].layers[0].objects = vec![slider("ok", "n", 0.0, 10.0), slider("none", "", 0.0, 10.0), slider("text", "t", 0.0, 10.0), slider("flat", "n", 5.0, 5.0), dial];
        let m = msgs(&p).join("\n");
        assert!(!m.contains("\"ok\""), "{m}");
        assert!(m.contains("Slider \"none\" is not linked to a number variable"), "{m}");
        assert!(m.contains("Slider \"text\" is not linked to a number variable"), "a text variable does not count: {m}");
        assert!(m.contains("Slider \"flat\" ends its range at or before where it starts"), "{m}");
        assert!(m.contains("Dial \"Wheel\" is not linked to a number variable"), "{m}");
    }

    #[test]
    fn a_pictures_own_states_do_not_make_it_a_button() {
        let mut p = project();
        let mut car = Object::new("car", "Car", Kind::Image, 0.0, 0.0, 100.0, 50.0);
        car.asset = Some("a".into());
        car.alt = "A car".into();
        let st = |n: &str| ObjectState { shine: None, gradient: None, name: n.into(), rich: None, fill: None, stroke: None, shadow: None, text: String::new(), style: car.style.clone(), asset: Some("a".into()), size: None };
        car.states = vec![st("Car-01"), st("Car-02")];
        p.slides[0].layers[0].objects = vec![car.clone()];
        let m = msgs(&p).join("\n");
        assert!(!m.contains("clickable"), "{m}");
        car.alt.clear();
        p.slides[0].layers[0].objects = vec![car];
        assert!(msgs(&p).join("\n").contains("has no alt text"), "a picture with picture states still needs alt text");
    }

    #[test]
    fn clean_project_has_no_issues() {
        assert!(project().check().is_empty());
    }

    #[test]
    fn words_laid_over_a_clickable_object_are_flagged() {
        let mut p = project();
        let mut card = Object::new("c", "Card", Kind::Rect, 100.0, 100.0, 400.0, 200.0);
        card.fill = Some(Rgba(11, 37, 69, 255));
        let mut words = Object::new("w", "Words", Kind::Text, 120.0, 150.0, 360.0, 80.0);
        words.text = "Forklifts".into();
        words.style.color = Rgba(255, 255, 255, 255);
        let mut far = Object::new("f", "Elsewhere", Kind::Text, 900.0, 900.0, 300.0, 80.0);
        far.text = "Not on the card".into();
        p.slides[0].layers[0].objects = vec![card.clone(), words.clone(), far];
        p.slides[0].triggers.push(Trigger { id: "t".into(), owner: Owner::Object("c".into()), event: Event::Click, action: Action::NextSlide, conditions: vec![] });
        let m = msgs(&p);
        assert!(m.iter().any(|x| x.contains("\"Words\" sits on top of \"Card\"") && x.contains("Put the words in")), "{m:?}");
        assert!(!m.iter().any(|x| x.contains("Elsewhere\" sits on top")), "text away from the card is fine: {m:?}");
        // Words inside the object itself, text under it, or words that are clickable too are all fine.
        let mut inside = card.clone();
        inside.text = "Forklifts".into();
        p.slides[0].layers[0].objects = vec![inside];
        assert!(!msgs(&p).iter().any(|x| x.contains("sits on top")));
        p.slides[0].layers[0].objects = vec![words.clone(), card.clone()];
        assert!(!msgs(&p).iter().any(|x| x.contains("sits on top")), "text behind it is not covering it");
        p.slides[0].layers[0].objects = vec![card, words];
        p.slides[0].triggers.push(Trigger { id: "t2".into(), owner: Owner::Object("w".into()), event: Event::Click, action: Action::NextSlide, conditions: vec![] });
        assert!(!msgs(&p).iter().any(|x| x.contains("sits on top")), "words that do the same click are fine");
    }

    #[test]
    fn a_clickable_group_is_one_control_named_as_a_whole() {
        let mut p = project();
        let mut card = Object::new("c", "Card", Kind::Rect, 100.0, 100.0, 400.0, 200.0);
        card.fill = Some(Rgba(11, 37, 69, 255));
        let mut words = Object::new("w", "Words", Kind::Text, 120.0, 150.0, 360.0, 80.0);
        words.text = "Forklifts".into();
        words.style.color = Rgba(255, 255, 255, 255);
        let mut icon = Object::new("i", "Icon", Kind::Image, 400.0, 110.0, 60.0, 60.0);
        icon.asset = Some("a".into());
        for o in [&mut card, &mut words, &mut icon] { o.group = Some("g".into()); }
        p.slides[0].layers[0].objects = vec![card, words, icon];
        p.slides[0].triggers.push(Trigger { id: "t".into(), owner: Owner::Object("c".into()), event: Event::Click, action: Action::NextSlide, conditions: vec![] });
        let m = msgs(&p);
        assert!(m.is_empty(), "a card of shape, words and picture with one trigger is fine: {m:?}");
        let s = &p.slides[0];
        assert_eq!(s.group_owner(0, "g"), Some(0));
        assert_eq!(s.group_name(0, "g"), "Forklifts", "named by the words of its parts");
        assert_eq!(s.group_role(0, 0), Some(GroupRole { is_owner: true, has_text: true }));
        assert_eq!(s.group_role(0, 2), Some(GroupRole { is_owner: false, has_text: true }));
        // Alt text of the owner names it instead.
        p.slides[0].layers[0].objects[0].alt = "Go to forklift safety".into();
        assert_eq!(p.slides[0].group_name(0, "g"), "Go to forklift safety");
        // With no words anywhere it cannot be named.
        p.slides[0].layers[0].objects[0].alt.clear();
        p.slides[0].layers[0].objects[1].text.clear();
        assert!(msgs(&p).iter().any(|x| x.contains("\"Card\" is clickable but has no text or alt text")), "{:?}", msgs(&p));
        p.slides[0].layers[0].objects[0].alt = "Forklifts".into();
        assert!(msgs(&p).is_empty(), "{:?}", msgs(&p));
        // A second part with its own click trigger is a mistake.
        p.slides[0].triggers.push(Trigger { id: "t2".into(), owner: Owner::Object("w".into()), event: Event::Click, action: Action::NextSlide, conditions: vec![] });
        assert!(msgs(&p).iter().any(|x| x.contains("click triggers on more than one of its parts")), "{:?}", msgs(&p));
        // A table's cells share a group id and are not a control.
        let mut cell = Object::new("x", "Cell", Kind::Rect, 0.0, 0.0, 50.0, 50.0);
        cell.group = Some("tbl".into());
        cell.table = Some((0, 0));
        p.slides[0].layers[0].objects = vec![cell];
        assert_eq!(p.slides[0].group_owner(0, "tbl"), None);
    }

    #[test]
    fn accessibility_problems_are_found() {
        let mut p = project();
        let mut img = Object::new("i", "Photo", Kind::Image, 0.0, 0.0, 10.0, 10.0);
        img.asset = Some("a".into());
        let mut vid = Object::new("v", "Clip", Kind::Video, 0.0, 0.0, 10.0, 10.0);
        vid.asset = Some("b".into());
        let mut btn = Object::new("b", "Go", Kind::Rect, 0.0, 0.0, 10.0, 10.0);
        btn.fill = Some(Rgba(255, 255, 255, 255));
        let mut pale = Object::new("t", "Pale", Kind::Text, 0.0, 0.0, 10.0, 10.0);
        pale.text = "hard to read".into();
        pale.style.size = 20.0;
        pale.style.color = Rgba(200, 200, 200, 255);
        p.slides[0].layers[0].objects = vec![img, vid, btn, pale];
        p.slides[0].triggers.push(Trigger { id: "t".into(), owner: Owner::Object("b".into()), event: Event::Click, action: Action::NextSlide, conditions: vec![] });
        let m = msgs(&p).join("\n");
        assert!(m.contains("Photo\" has no alt text"), "{m}");
        assert!(m.contains("Clip\" has no captions"), "{m}");
        assert!(m.contains("Clip\" has no description"), "{m}");
        assert!(m.contains("\"Go\" is clickable but has no text"), "{m}");
        assert!(m.contains("low contrast"), "{m}");
        // A decorative picture needs no alt text.
        p.slides[0].layers[0].objects[0].decorative = true;
        assert!(!msgs(&p).join("\n").contains("Photo\" has no alt text"));
    }

    #[test]
    fn large_text_gets_the_lower_contrast_threshold() {
        let mut p = project();
        let mut o = Object::new("t", "Big", Kind::Text, 0.0, 0.0, 10.0, 10.0);
        o.text = "Title".into();
        o.style.size = 60.0;
        o.style.color = Rgba(0x90, 0x90, 0x90, 255); // about 3.2:1 on white
        p.slides[0].layers[0].objects.push(o.clone());
        assert!(msgs(&p).is_empty(), "{:?}", msgs(&p));
        o.style.size = 16.0;
        p.slides[0].layers[0].objects[0] = o;
        assert_eq!(msgs(&p).len(), 1);
    }

    #[test]
    fn text_is_judged_against_the_shape_behind_it() {
        let mut p = project();
        let mut header = Object::new("h", "Header", Kind::Rect, 0.0, 0.0, 500.0, 100.0);
        header.fill = Some(Rgba(3, 83, 158, 255));
        let mut title = Object::new("t", "Title", Kind::Text, 10.0, 10.0, 200.0, 50.0);
        title.text = "Welcome".into();
        title.style.color = Rgba(255, 255, 255, 255); // white on blue: fine
        title.style.size = 16.0;
        let mut over_photo = Object::new("p", "Caption", Kind::Text, 10.0, 200.0, 200.0, 50.0);
        over_photo.text = "On a photo".into();
        over_photo.style.color = Rgba(255, 255, 255, 255);
        over_photo.style.size = 16.0;
        let photo = Object::new("ph", "Photo", Kind::Image, 0.0, 150.0, 500.0, 200.0);
        let mut alt_photo = photo.clone();
        alt_photo.alt = "A photo".into();
        p.slides[0].layers[0].objects = vec![header, title, alt_photo, over_photo];
        assert!(msgs(&p).is_empty(), "{:?}", msgs(&p));
        // The same white text with nothing behind it is a problem.
        p.slides[0].layers[0].objects.remove(0);
        assert_eq!(msgs(&p).len(), 1, "{:?}", msgs(&p));
    }

    #[test]
    fn other_question_styles_are_validated() {
        let mut p = project();
        let mut bad = Vec::new();
        let mut t = Question::new_text_entry();
        t.style = QuestionStyle::TextEntry { accepted: vec!["  ".into()], case_sensitive: false };
        bad.push((t, "no accepted answer"));
        let mut m = Question::new_matching();
        m.style = QuestionStyle::Matching { pairs: vec![("a".into(), "".into())] };
        bad.push((m, "at least two pairs"));
        let mut s = Question::new_sequence();
        s.style = QuestionStyle::Sequence { items: vec!["only".into()] };
        bad.push((s, "at least two items"));
        for (i, (q, want)) in bad.iter().enumerate() {
            let mut sl = Slide::new(&format!("q{i}"), "Q", 10.0, 10.0);
            sl.kind = SlideKind::Question(q.clone());
            p.slides.push(sl);
            assert!(msgs(&p).iter().any(|m| m.contains(want)), "{want}: {:?}", msgs(&p));
        }
        // The default examples of each style are valid (only the missing results slide is flagged).
        let mut ok = project();
        for q in [Question::new_text_entry(), Question::new_matching(), Question::new_sequence(), Question::new_true_false()] {
            let mut sl = Slide::new("x", "Q", 10.0, 10.0);
            sl.kind = SlideKind::Question(q);
            ok.slides.push(sl);
        }
        let mut r = Slide::new("r", "R", 10.0, 10.0);
        r.kind = SlideKind::Results;
        ok.slides.push(r);
        assert!(ok.check().is_empty(), "{:?}", ok.check());
    }

    #[test]
    fn hidden_objects_are_not_checked() {
        let mut p = project();
        let mut img = Object::new("i", "Photo", Kind::Image, 0.0, 0.0, 10.0, 10.0);
        img.hidden = true;
        p.slides[0].layers[0].objects.push(img);
        assert!(p.check().is_empty());
    }

    #[test]
    fn question_and_trigger_problems_are_errors() {
        let mut p = project();
        let mut q = Question::new_single();
        q.choices.iter_mut().for_each(|c| c.correct = false);
        let mut s = Slide::new("q", "Q", 100.0, 100.0);
        s.kind = SlideKind::Question(q);
        p.slides.push(s);
        p.slides[0].triggers.push(Trigger { id: "1".into(), owner: Owner::Slide, event: Event::SlideStart, action: Action::ShowLayer("nope".into()), conditions: vec![] });
        p.slides[0].triggers.push(Trigger { id: "2".into(), owner: Owner::Slide, event: Event::SlideStart, action: Action::JumpToSlide("gone".into()), conditions: vec![] });
        p.slides[0].triggers.push(Trigger { id: "3".into(), owner: Owner::Slide, event: Event::VariableChanged("x".into()), action: Action::NextSlide, conditions: vec![] });
        let issues = p.check();
        let m: Vec<_> = issues.iter().map(|i| i.message.as_str()).collect();
        assert!(m.iter().any(|x| x.contains("No choice is marked correct")));
        assert!(m.iter().any(|x| x.contains("no results slide")));
        assert!(m.iter().any(|x| x.contains("layer that does not exist")));
        assert!(m.iter().any(|x| x.contains("slide that does not exist")));
        assert!(m.iter().any(|x| x.contains("watches a variable")));
        assert!(issues.iter().filter(|i| i.severity == Severity::Error).count() >= 4);
        // Sorted by slide, errors first within a slide.
        let slides: Vec<usize> = issues.iter().map(|i| i.at.slide).collect();
        assert!(slides.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn a_trigger_that_watches_a_missing_object_is_an_error() {
        let mut p = project();
        let b = Object::new("b", "B", Kind::Rect, 0.0, 0.0, 10.0, 10.0);
        p.slides[0].layers[0].objects = vec![b];
        let t = |o: &str| Trigger { id: o.into(), owner: Owner::Slide, event: Event::StateChanged(o.into(), "Selected".into()), action: Action::NextSlide, conditions: vec![] };
        p.slides[0].triggers = vec![t("b"), t("gone")];
        let errors: Vec<_> = msgs(&p).into_iter().filter(|m| m.contains("watches an object")).collect();
        assert_eq!(errors.len(), 1, "{errors:?}");
    }

    #[test]
    fn gestures_and_motion_that_exclude_people_are_flagged() {
        let mut p = project();
        let mut a = Object::new("dbl", "Secret", Kind::Rect, 0.0, 0.0, 50.0, 50.0);
        a.text = "Secret".into();
        let mut b = Object::new("both", "Both", Kind::Rect, 0.0, 60.0, 50.0, 50.0);
        b.text = "Both".into();
        let mut flash = Object::new("fl", "Blinker", Kind::Rect, 0.0, 120.0, 50.0, 50.0);
        flash.text = "x".into();
        flash.emphasis = Some(Emphasis { kind: EmphasisKind::Flash, delay_ms: 0, duration_ms: 200, repeat: 0 });
        let mut loops = Object::new("lp", "Looper", Kind::Rect, 0.0, 180.0, 50.0, 50.0);
        loops.text = "x".into();
        loops.emphasis = Some(Emphasis { kind: EmphasisKind::Pulse, delay_ms: 0, duration_ms: 800, repeat: 0 });
        let mut calm = Object::new("ok", "Calm", Kind::Rect, 0.0, 240.0, 50.0, 50.0);
        calm.text = "x".into();
        calm.emphasis = Some(Emphasis { kind: EmphasisKind::Pulse, delay_ms: 0, duration_ms: 800, repeat: 2 });
        p.slides[0].layers[0].objects = vec![a, b, flash, loops, calm];
        let trig = |id: &str, ev: Event| Trigger { id: id.into(), owner: Owner::Object(id.split('_').next().unwrap().into()), event: ev, conditions: vec![], action: Action::NextSlide };
        p.slides[0].triggers = vec![trig("dbl_1", Event::DoubleClick), trig("both_1", Event::DoubleClick), trig("both_2", Event::Click)];
        let m = msgs(&p).join("\n");
        assert!(m.contains("\"Secret\" reacts only to a double-click"), "{m}");
        assert!(!m.contains("\"Both\" reacts only"), "a click alternative is enough: {m}");
        assert!(m.contains("\"Blinker\" flashes more than three times a second"), "{m}");
        assert!(m.contains("\"Looper\" keeps moving until the slide ends"), "{m}");
        assert!(!m.contains("\"Calm\""), "{m}");
    }
}
