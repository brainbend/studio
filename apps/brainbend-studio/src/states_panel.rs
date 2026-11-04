//! The States tab: a strip of cards (one per state of the selected object) and a toolbar.
//! While the tab is open the object takes the look of the state being viewed, so the ribbon, the Properties
//! panel and the canvas edit that state like any object; the look is copied back into the state when the frame ends.
use eframe::egui;
use brainbend_doc::{Action, Event, Gradient, Kind, Object, ObjectState, Owner, Paragraph, Rgba, Shadow, Shine, Slide, Stroke, TextStyle};
use std::collections::HashMap;

/// Objects that can have states. Media, hotspots, lines and sliders keep to their own controls.
pub fn supports_states(o: &Object) -> bool {
    !matches!(o.kind, Kind::Video | Kind::Audio | Kind::Hotspot | Kind::Line | Kind::LineUp | Kind::Slider(_) | Kind::Web(_) | Kind::Pano(_) | Kind::Unsupported(_))
}

/// The parts of an object a state restyles.
#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    pub fill: Option<Rgba>,
    pub gradient: Option<Gradient>,
    pub stroke: Option<Stroke>,
    pub shadow: Option<Shadow>,
    pub text: String,
    pub style: TextStyle,
    pub rich: Option<Vec<Paragraph>>,
    pub asset: Option<String>,
}

impl Look {
    pub fn of(o: &Object) -> Look {
        Look { fill: o.fill, gradient: o.gradient.clone(), stroke: o.stroke, shadow: o.shadow, text: o.text.clone(), style: o.style.clone(), rich: o.rich.clone(), asset: o.asset.clone() }
    }

    pub fn restore(&self, o: &mut Object) {
        o.fill = self.fill;
        o.gradient = self.gradient.clone();
        o.stroke = self.stroke;
        o.shadow = self.shadow;
        o.text = self.text.clone();
        o.style = self.style.clone();
        o.rich = self.rich.clone();
        o.asset = self.asset.clone();
    }
}

/// The object looks like `st` (a state's fill replaces any gradient; a picture state swaps the picture).
pub fn show_state(o: &mut Object, st: &ObjectState) {
    o.fill = st.fill;
    o.gradient = st.gradient.clone();
    o.stroke = st.stroke;
    o.shadow = st.shadow;
    o.text = st.text.clone();
    o.style = st.style.clone();
    o.rich = st.rich.clone();
    if let (Kind::Image, Some(a)) = (&o.kind, &st.asset) {
        o.asset = Some(a.clone());
    }
}

/// The object takes the look of its starting state.
pub fn sync_initial(o: &mut Object) {
    let Some(st) = o.states.iter().find(|s| s.name == o.initial_state).cloned() else { return };
    show_state(o, &st);
}

/// An open States tab. The viewed state's look sits on the object while the frame runs.
#[derive(Clone, Debug)]
pub struct Swap {
    slide: usize,
    id: String,
    viewed: String,
    /// The starting state's name when the swap began.
    initial_name: String,
    /// The object's own look before the viewed state's was put on it; `None` when the viewed state is the starting one.
    own: Option<Look>,
}

fn find_object<'a>(slides: &'a mut [Slide], slide: usize, id: &str) -> Option<&'a mut Object> {
    slides.get_mut(slide)?.layers.iter_mut().flat_map(|l| l.objects.iter_mut()).find(|o| o.id == id)
}

/// Put the viewed state's look on the object. Returns the swap to undo at the end of the frame.
pub fn begin(slides: &mut [Slide], slide: usize, id: &str, viewed: &str) -> Option<Swap> {
    let o = find_object(slides, slide, id)?;
    if o.states.is_empty() || !supports_states(o) { return None; }
    let st = o.states.iter().find(|s| s.name == viewed)?.clone();
    let own = (st.name != o.initial_state).then(|| Look::of(o));
    if own.is_some() { show_state(o, &st); }
    Some(Swap { slide, id: id.into(), viewed: viewed.into(), initial_name: o.initial_state.clone(), own })
}

/// Copy the object's look back into the viewed state and give the object its own look again.
pub fn end(slides: &mut [Slide], swap: Swap) {
    let Some(o) = find_object(slides, swap.slide, &swap.id) else { return };
    let seen = Look::of(o);
    let own_asset = swap.own.as_ref().map(|l| l.asset.clone()).unwrap_or_else(|| seen.asset.clone());
    let is_image = matches!(o.kind, Kind::Image);
    if let Some(st) = o.states.iter_mut().find(|s| s.name == swap.viewed) {
        st.fill = seen.fill;
        st.gradient = seen.gradient.clone();
        st.stroke = seen.stroke;
        st.shadow = seen.shadow;
        st.text = seen.text.clone();
        st.style = seen.style.clone();
        st.rich = seen.rich.clone();
        // A picture the state did not choose stays unset, so it keeps following the object's.
        if is_image && (st.asset.is_some() || seen.asset != own_asset) { st.asset = seen.asset.clone(); }
    }
    match swap.own {
        Some(own) if o.initial_state == swap.initial_name => own.restore(o),
        Some(_) => sync_initial(o),
        None => {}
    }
}

/// What an undo step captured mid-frame should hold: the object with its own look, not the viewed state's.
pub fn unswap(slides: &mut [Slide], swap: &Swap) {
    if let (Some(own), Some(o)) = (&swap.own, find_object(slides, swap.slide, &swap.id)) {
        own.restore(o);
    }
}

fn state_of(o: &Object) -> ObjectState {
    ObjectState { shine: None, gradient: o.gradient.clone(), name: "Normal".into(), rich: o.rich.clone(), fill: o.fill, stroke: o.stroke, shadow: o.shadow, text: o.text.clone(), style: o.style.clone(), asset: o.asset.clone().filter(|_| matches!(o.kind, Kind::Image)), size: None }
}

/// Add whichever of Normal, Hover, Down, Visited and Disabled the object does not have yet.
pub fn add_button_states(o: &mut Object) {
    if o.states.is_empty() {
        o.states.push(state_of(o));
        o.initial_state = "Normal".into();
    }
    let base = o.states.iter().find(|s| s.name == o.initial_state).cloned().unwrap_or_else(|| o.states[0].clone());
    let has = |o: &Object, n: &str| o.states.iter().any(|s| s.name == n);
    if !has(o, "Hover") {
        let mut s = base.clone();
        s.name = "Hover".into();
        s.shine = Some(Shine { alpha: 0.6, reverse: false });
        o.states.push(s);
    }
    if !has(o, "Down") {
        let mut s = base.clone();
        s.name = "Down".into();
        s.stroke = Some(Stroke { color: Rgba(0, 0, 0, 102), width: 3.0 });
        s.shine = Some(Shine { alpha: 0.25, reverse: true });
        o.states.push(s);
    }
    if !has(o, "Visited") {
        let mut s = base.clone();
        s.name = "Visited".into();
        o.states.push(s);
    }
    if !has(o, "Disabled") {
        let mut s = base.clone();
        s.name = "Disabled".into();
        s.fill = s.fill.map(|_| Rgba(190, 195, 200, 255));
        s.style.color = Rgba(120, 125, 130, 255);
        o.states.push(s);
    }
}

fn next_name(o: &Object) -> String {
    let mut n = o.states.len() + 1;
    while o.states.iter().any(|s| s.name == format!("State {n}")) {
        n += 1;
    }
    format!("State {n}")
}

/// "Try it": what each object with states shows while the author presses buttons on the canvas, as the player would.
/// `chosen` maps an object id to the state a press left it in (absent = its starting state).
pub fn tried_state<'a>(o: &'a Object, chosen: Option<&String>, hover: bool, down: bool) -> Option<&'a ObjectState> {
    if o.states.is_empty() { return None; }
    let name = chosen.map(String::as_str).unwrap_or(o.initial_state.as_str());
    let has = |n: &str| o.states.iter().any(|s| s.name == n);
    let name = if name == o.initial_state || name == "Normal" || name == "Visited" {
        if down && has("Down") { "Down" } else if hover && has("Hover") { "Hover" } else { name }
    } else { name };
    o.states.iter().find(|s| s.name == name).or_else(|| o.states.iter().find(|s| s.name == o.initial_state))
}

/// A press on object `id` while trying: a button in a set becomes Selected and releases the rest of its set;
/// any other button with a Visited state becomes Visited.
pub fn press_in_try(chosen: &mut std::collections::HashMap<String, String>, slide: &Slide, id: &str) {
    let all: Vec<&Object> = slide.layers.iter().flat_map(|l| l.objects.iter()).collect();
    let Some(o) = all.iter().find(|o| o.id == id) else { return };
    let has = |n: &str| o.states.iter().any(|s| s.name == n);
    let current = chosen.get(id).map(String::as_str).unwrap_or(o.initial_state.as_str());
    if current == "Disabled" { return; }
    if let (Some(set), true) = (o.button_set.as_deref(), has("Selected")) {
        for other in all.iter().filter(|x| x.id != id && x.button_set.as_deref() == Some(set)) {
            if chosen.get(&other.id).is_some_and(|s| s == "Selected") { chosen.remove(&other.id); }
        }
        chosen.insert(id.to_string(), "Selected".into());
    } else if has("Visited") && (current == "Normal" || current == o.initial_state) {
        chosen.insert(id.to_string(), "Visited".into());
    }
}

/// Show `layer` as the player would: the slide's other layers go if it hides them.
fn show_layer(slide: &Slide, shown: &mut Vec<String>, layer: &str) {
    let Some(l) = slide.layers.iter().skip(1).find(|l| l.id == layer) else { return };
    if l.hide_others { shown.clear(); }
    if !shown.iter().any(|s| s == layer) { shown.push(layer.to_string()); }
}

/// The layers showing when "Try it" starts: those a slide-start trigger shows.
pub fn try_start(slide: &Slide) -> Vec<String> {
    let mut shown = vec![];
    for t in slide.triggers.iter().filter(|t| t.conditions.is_empty() && matches!((&t.event, &t.owner), (Event::SlideStart, Owner::Slide))) {
        if let Action::ShowLayer(id) = &t.action { show_layer(slide, &mut shown, id); }
    }
    shown
}

/// Base-layer objects a showing layer hides ("hide objects on base layer").
pub fn hidden_by_layers(slide: &Slide, shown: &[String]) -> Vec<String> {
    slide.layers.first().map(|b| b.objects.iter().filter(|o| o.hide_on_layers.iter().any(|l| shown.contains(l))).map(|o| o.id.clone()).collect()).unwrap_or_default()
}

/// A press while trying: the button's own state change (sets, Visited), then its click triggers that show and hide
/// layers or set a state. Triggers with conditions, variables, jumps and the rest are left out: this is a look, not a run.
pub fn try_press(slide: &Slide, id: &str, shown: &mut Vec<String>, states: &mut HashMap<String, String>) {
    press_in_try(states, slide, id);
    for t in slide.triggers.iter().filter(|t| t.conditions.is_empty() && matches!(t.event, Event::Click) && matches!(&t.owner, Owner::Object(o) if o == id)) {
        match &t.action {
            Action::ShowLayer(l) => show_layer(slide, shown, l),
            Action::HideLayer(l) => shown.retain(|s| s != l),
            Action::SetObjectState { object, state } => { states.insert(object.clone(), state.clone()); }
            _ => {}
        }
    }
}

/// What a trigger does, in a few words ("jumps to Slide 3"), for telling the author what Try It left out.
fn action_phrase(action: &Action, slides: &[Slide]) -> String {
    match action {
        Action::ShowLayer(_) => "shows a layer".into(),
        Action::HideLayer(_) => "hides a layer".into(),
        Action::SetObjectState { .. } => "changes an object's state".into(),
        Action::SetVariable { .. } | Action::AdjustVariable { .. } => "changes a variable".into(),
        Action::RunJavaScript(_) => "runs JavaScript".into(),
        Action::JumpToSlide(id) => match slides.iter().find(|s| &s.id == id) {
            Some(s) => format!("jumps to {}", s.name),
            None => "jumps to another slide".into(),
        },
        Action::NextSlide => "goes to the next slide".into(),
        Action::PreviousSlide => "goes to the previous slide".into(),
        Action::SeekTimeline { .. } => "moves a timeline or a video to a time".into(),
        Action::ShowObject(_) => "shows an object".into(),
        Action::HideObject(_) => "hides an object".into(),
        Action::OpenUrl(_) => "opens a web page".into(),
        Action::PlayAnimation(_) => "plays an animation".into(),
        Action::PlayMedia(_) | Action::PauseMedia(_) | Action::StopMedia(_) => "plays or stops a video or audio".into(),
        Action::CompleteCourse => "completes the course".into(),
        Action::ExitCourse => "exits the course".into(),
        Action::RestartCourse => "restarts the course".into(),
        Action::Unsupported(_) => "does something this version cannot run yet".into(),
    }
}

/// What pressing object `id` does in the player that Try It does not run: its click triggers that have conditions, and
/// those whose action is not one of the three Try It runs (show or hide a layer, set a state).
pub fn not_run(slide: &Slide, slides: &[Slide], id: &str) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for t in slide.triggers.iter().filter(|t| matches!(t.event, Event::Click) && matches!(&t.owner, Owner::Object(o) if o == id)) {
        let runs = matches!(t.action, Action::ShowLayer(_) | Action::HideLayer(_) | Action::SetObjectState { .. });
        if runs && t.conditions.is_empty() { continue; }
        let mut phrase = action_phrase(&t.action, slides);
        if !t.conditions.is_empty() { phrase.push_str(" only when its conditions are met"); }
        if !out.contains(&phrase) { out.push(phrase); }
    }
    out
}

/// The sentence for the Try It banner, or `None` when the press left nothing out.
pub fn not_run_note(parts: &[String]) -> Option<String> {
    let shown: Vec<&str> = parts.iter().take(3).map(String::as_str).collect();
    let list = match shown.as_slice() {
        [] => return None,
        [one] => one.to_string(),
        [a, b] => format!("{a} and {b}"),
        [a, b, c] => format!("{a}, {b} and {c}"),
        _ => unreachable!(),
    };
    let more = if parts.len() > 3 { " and more" } else { "" };
    Some(format!("This button {list}{more}. Use Preview to test this action."))
}

/// What the panel asks the app to do.
#[derive(Default)]
pub struct Outcome {
    pub changed: bool,
    pub want_picture: bool,
    pub done: bool,
}

/// One card: the state drawn small, its name under it, a flag on the starting state.
fn card(ui: &mut egui::Ui, o: &Object, st: &ObjectState, selected: bool, starts: bool) -> egui::Response {
    let size = egui::vec2(112.0, 98.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let p = ui.painter();
    let accent = ui.visuals().selection.bg_fill;
    p.rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);
    p.rect_stroke(rect, 4.0, egui::Stroke::new(if selected { 2.0 } else { 1.0 }, if selected { accent } else { ui.visuals().widgets.noninteractive.bg_stroke.color }), egui::StrokeKind::Inside);
    // The preview keeps the object's proportions inside the card.
    let area = egui::Rect::from_min_max(rect.min + egui::vec2(8.0, 8.0), rect.max - egui::vec2(8.0, 26.0));
    let k = (area.width() / o.w.max(1.0)).min(area.height() / o.h.max(1.0));
    let pv = egui::Rect::from_center_size(area.center(), egui::vec2(o.w * k, o.h * k));
    let q = p.with_clip_rect(pv);
    if matches!(o.kind, Kind::Image) {
        q.rect_filled(pv, 0.0, egui::Color32::from_gray(120));
        q.text(pv.center(), egui::Align2::CENTER_CENTER, "Picture", egui::FontId::proportional(11.0), egui::Color32::WHITE);
    } else {
        if let Some(g) = st.gradient.as_ref().filter(|g| !g.stops.is_empty()) { crate::paint_gradient(&q, pv, g.angle, &g.stops); }
        else if let Some(f) = st.fill { q.rect_filled(pv, 2.0, crate::color(f)); }
        if let Some(s) = st.stroke { q.rect_stroke(pv, 2.0, egui::Stroke::new((s.width * k).max(1.0), crate::color(s.color)), egui::StrokeKind::Inside); }
        if let Some(sh) = st.shine {
            q.rect_filled(pv, 2.0, egui::Color32::from_white_alpha((sh.alpha * 90.0) as u8));
        }
        let first = st.text.lines().next().unwrap_or("");
        if !first.is_empty() {
            let size = (st.style.size * k).clamp(6.0, 14.0);
            q.text(pv.left_center() + egui::vec2(3.0, 0.0), egui::Align2::LEFT_CENTER, first, egui::FontId::proportional(size), crate::color(st.style.color));
        }
    }
    p.text(egui::pos2(rect.center().x, rect.bottom() - 12.0), egui::Align2::CENTER_CENTER, &st.name, egui::FontId::proportional(12.0), ui.visuals().text_color());
    if starts {
        p.text(rect.left_top() + egui::vec2(6.0, 5.0), egui::Align2::LEFT_TOP, "\u{2691}", egui::FontId::proportional(11.0), accent);
    }
    resp
}

/// The panel body. `viewed` is the state on the canvas; `dim` dims the other objects while it is open.
pub fn editor(ui: &mut egui::Ui, o: &mut Object, viewed: &mut String, dim: &mut bool) -> Outcome {
    let mut out = Outcome::default();
    if o.states.is_empty() {
        ui.horizontal(|ui| {
            if ui.button("DONE").clicked() { out.done = true; }
            ui.label(format!("{} has no states yet.", if o.name.is_empty() { "This object" } else { &o.name }));
            if ui.button("Add button states").on_hover_text("Normal, Hover, Down, Visited and Disabled").clicked() {
                add_button_states(o);
                *viewed = o.initial_state.clone();
                out.changed = true;
            }
            if ui.button("Add a state").clicked() {
                o.states.push(state_of(o));
                o.initial_state = "Normal".into();
                *viewed = "Normal".into();
                out.changed = true;
            }
        });
        ui.weak("A state is a look the object can switch to. A trigger sets it, and buttons use Hover, Down and Disabled by themselves.");
        return out;
    }
    if !o.states.iter().any(|s| &s.name == viewed) {
        *viewed = o.initial_state.clone();
    }
    let init = o.initial_state.clone();
    let is_image = matches!(o.kind, Kind::Image);
    let v = o.states.iter().position(|s| &s.name == viewed).unwrap_or(0);
    let mut remove = false;
    let mut rename: Option<String> = None;
    let mut make_initial = false;
    ui.horizontal_wrapped(|ui| {
        crate::ribbon::flat(ui);
        if ui.add(egui::Button::new(egui::RichText::new("DONE").strong().color(egui::Color32::WHITE)).fill(egui::Color32::from_rgb(11, 110, 105))).clicked() { out.done = true; }
        ui.separator();
        if ui.button("+ New").on_hover_text("A copy of the state you are viewing").clicked() {
            let mut copy = o.states[v].clone();
            copy.name = next_name(o);
            *viewed = copy.name.clone();
            o.states.push(copy);
            out.changed = true;
        }
        if ui.button("Button set").on_hover_text("Add any missing Hover, Down, Visited and Disabled").clicked() {
            add_button_states(o);
            out.changed = true;
        }
        ui.add_enabled_ui(o.states[v].name != init, |ui| {
            if ui.button("Start as this").on_hover_text("The object begins in this state").clicked() { make_initial = true; }
            if ui.button("Delete").clicked() { remove = true; }
        });
        ui.separator();
        ui.label("Name");
        let mut name = o.states[v].name.clone();
        crate::ribbon::framed(ui, |ui| { if ui.add(egui::TextEdit::singleline(&mut name).desired_width(110.0)).changed() { rename = Some(name.clone()); } });
        ui.separator();
        ui.checkbox(dim, "Dim background objects");
        let st = &mut o.states[v];
        if is_image {
            ui.separator();
            if ui.button("Choose picture…").clicked() { out.want_picture = true; }
        } else {
            ui.separator();
            let mut on = st.shine.is_some();
            if ui.checkbox(&mut on, "Shine").changed() {
                st.shine = on.then_some(Shine { alpha: 0.4, reverse: false });
                out.changed = true;
            }
            if let Some(sh) = &mut st.shine {
                out.changed |= ui.add(egui::Slider::new(&mut sh.alpha, 0.0..=1.0).show_value(false)).changed();
                out.changed |= ui.checkbox(&mut sh.reverse, "reversed").changed();
            }
        }
    });
    egui::ScrollArea::horizontal().id_salt("state-cards").show(ui, |ui| {
        ui.horizontal(|ui| {
            let snapshot: Vec<ObjectState> = o.states.clone();
            for s in &snapshot {
                if card(ui, o, s, &s.name == viewed, s.name == init).clicked() {
                    *viewed = s.name.clone();
                }
            }
        });
    });
    if let Some(to) = rename {
        let from = o.states[v].name.clone();
        if !to.is_empty() && !o.states.iter().enumerate().any(|(i, s)| i != v && s.name == to) {
            if o.initial_state == from { o.initial_state = to.clone(); }
            o.states[v].name = to.clone();
            *viewed = to;
            out.changed = true;
        }
    }
    if make_initial {
        o.initial_state = o.states[v].name.clone();
        out.changed = true;
    }
    if remove && o.states[v].name != o.initial_state {
        o.states.remove(v);
        *viewed = o.initial_state.clone();
        out.changed = true;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> Object {
        let mut o = Object::new("a", "a", Kind::Rect, 100.0, 100.0, 200.0, 80.0);
        o.fill = Some(Rgba(10, 20, 30, 255));
        o.text = "Go".into();
        o
    }

    fn slides_with(o: Object) -> Vec<Slide> {
        let mut s = Slide::new("s", "S", 1280.0, 720.0);
        s.layers[0].objects.push(o);
        vec![s]
    }

    fn obj(slides: &[Slide]) -> &Object {
        &slides[0].layers[0].objects[0]
    }

    #[test]
    fn button_states_start_from_the_object_and_only_fill_in_what_is_missing() {
        let mut o = rect();
        add_button_states(&mut o);
        let names: Vec<_> = o.states.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["Normal", "Hover", "Down", "Visited", "Disabled"]);
        assert_eq!(o.initial_state, "Normal");
        assert_eq!(o.states[1].fill, Some(Rgba(10, 20, 30, 255)));
        assert!(o.states[1].shine.is_some_and(|s| s.alpha > o.states[0].shine.map_or(0.0, |n| n.alpha)), "hover shines brighter");
        assert_eq!(o.states[4].text, "Go");
        o.states[1].fill = Some(Rgba(1, 2, 3, 255));
        add_button_states(&mut o);
        assert_eq!(o.states.len(), 5, "nothing added twice");
        assert_eq!(o.states[1].fill, Some(Rgba(1, 2, 3, 255)), "existing states are left alone");
    }

    fn click_trigger(owner: &str, action: Action, conditions: bool) -> brainbend_doc::Trigger {
        brainbend_doc::Trigger {
            id: format!("t-{owner}"),
            owner: Owner::Object(owner.into()),
            event: Event::Click,
            action,
            conditions: if conditions { vec![brainbend_doc::Condition { variable: "v".into(), cmp: brainbend_doc::Cmp::Eq, value: brainbend_doc::Value::Num(1.0), or_with_previous: false }] } else { vec![] },
        }
    }

    #[test]
    fn a_press_says_what_try_it_left_out() {
        let mut slides = vec![Slide::new("s1", "Intro", 1280.0, 720.0), Slide::new("s2", "Quiz 1", 1280.0, 720.0)];
        slides[0].triggers = vec![
            click_trigger("b", Action::ShowLayer("L".into()), false),
            click_trigger("b", Action::JumpToSlide("s2".into()), false),
            click_trigger("b", Action::RunJavaScript("x()".into()), false),
            click_trigger("c", Action::HideLayer("L".into()), true),
            click_trigger("d", Action::ShowLayer("L".into()), false),
        ];
        let s = slides[0].clone();
        // Showing a layer runs; the jump and the script do not.
        assert_eq!(not_run(&s, &slides, "b"), ["jumps to Quiz 1", "runs JavaScript"]);
        // A conditional trigger is left out even when its action is one Try It runs.
        assert_eq!(not_run(&s, &slides, "c"), ["hides a layer only when its conditions are met"]);
        // A button whose triggers all run says nothing.
        assert!(not_run(&s, &slides, "d").is_empty());
        assert!(not_run(&s, &slides, "no-such-button").is_empty());
    }

    #[test]
    fn the_banner_note_reads_as_a_sentence() {
        assert_eq!(not_run_note(&[]), None);
        assert_eq!(not_run_note(&["jumps to Quiz 1".into()]).unwrap(), "This button jumps to Quiz 1. Use Preview to test this action.");
        assert_eq!(not_run_note(&["jumps to Quiz 1".into(), "runs JavaScript".into()]).unwrap(), "This button jumps to Quiz 1 and runs JavaScript. Use Preview to test this action.");
        let many: Vec<String> = ["a", "b", "c", "d"].iter().map(|s| s.to_string()).collect();
        assert_eq!(not_run_note(&many).unwrap(), "This button a, b and c and more. Use Preview to test this action.");
    }

    fn button(id: &str, set: Option<&str>) -> Object {
        let mut o = rect();
        o.id = id.into();
        o.button_set = set.map(String::from);
        add_button_states(&mut o);
        o.states.push(ObjectState { name: "Selected".into(), ..o.states[0].clone() });
        o
    }

    #[test]
    fn trying_a_set_selects_the_pressed_button_and_releases_the_others() {
        let mut s = Slide::new("s", "S", 1280.0, 720.0);
        s.layers[0].objects = vec![button("a", Some("tabs")), button("b", Some("tabs")), button("c", None)];
        let mut chosen = std::collections::HashMap::new();
        press_in_try(&mut chosen, &s, "a");
        press_in_try(&mut chosen, &s, "c");
        assert_eq!((chosen.get("a").map(String::as_str), chosen.get("c").map(String::as_str)), (Some("Selected"), Some("Visited")));
        press_in_try(&mut chosen, &s, "b");
        assert_eq!((chosen.get("a"), chosen.get("b").map(String::as_str)), (None, Some("Selected")), "a is released back to its starting state");
        assert_eq!(chosen.get("c").map(String::as_str), Some("Visited"), "a button outside the set is untouched");
    }

    #[test]
    fn pressing_a_tab_while_trying_shows_its_layer_hides_the_others_and_replaces_base_objects() {
        use brainbend_doc::{Layer, Trigger};
        let mut s = Slide::new("s", "S", 1280.0, 720.0);
        let mut body = rect();
        body.id = "body".into();
        body.hide_on_layers = vec!["l1".into(), "l2".into()];
        s.layers[0].objects = vec![button("t1", Some("tabs")), button("t2", Some("tabs")), body, rect()];
        let layer = |id: &str, hide: bool| Layer { id: id.into(), name: id.into(), objects: vec![], duration_ms: None, modal: false, hide_others: hide, pause_base: false, group_names: Default::default(), focus_order: vec![] };
        s.layers.push(layer("l1", true));
        s.layers.push(layer("l2", true));
        let show = |id: &str, owner: &str, l: &str| Trigger { id: id.into(), owner: Owner::Object(owner.into()), event: Event::Click, action: Action::ShowLayer(l.into()), conditions: vec![] };
        s.triggers = vec![show("a", "t1", "l1"), show("b", "t2", "l2")];
        let (mut shown, mut states) = (try_start(&s), HashMap::new());
        assert!(shown.is_empty() && hidden_by_layers(&s, &shown).is_empty());
        try_press(&s, "t1", &mut shown, &mut states);
        assert_eq!(shown, ["l1"]);
        assert_eq!(hidden_by_layers(&s, &shown), ["body"], "the base body is replaced while l1 shows");
        try_press(&s, "t2", &mut shown, &mut states);
        assert_eq!(shown, ["l2"], "l2 hides l1");
        assert_eq!((states.get("t1"), states.get("t2").map(String::as_str)), (None, Some("Selected")), "the set follows");
    }

    #[test]
    fn layers_a_slide_start_trigger_shows_are_showing_when_trying_starts() {
        use brainbend_doc::{Layer, Trigger};
        let mut s = Slide::new("s", "S", 1280.0, 720.0);
        let layer = |id: &str| Layer { id: id.into(), name: id.into(), objects: vec![], duration_ms: None, modal: false, hide_others: false, pause_base: false, group_names: Default::default(), focus_order: vec![] };
        s.layers.push(layer("k1"));
        s.layers.push(layer("k2"));
        let at_start = |id: &str, l: &str| Trigger { id: id.into(), owner: Owner::Slide, event: Event::SlideStart, action: Action::ShowLayer(l.into()), conditions: vec![] };
        s.triggers = vec![at_start("a", "k1"), at_start("b", "k2")];
        assert_eq!(try_start(&s), ["k1", "k2"], "layers that do not hide the others stack");
    }

    #[test]
    fn a_tried_button_shows_hover_and_down_over_normal_and_visited_but_not_over_selected() {
        let o = button("a", None);
        let name = |chosen: Option<&str>, hover, down| tried_state(&o, chosen.map(String::from).as_ref(), hover, down).map(|s| s.name.clone());
        assert_eq!(name(None, false, false).as_deref(), Some("Normal"));
        assert_eq!(name(None, true, false).as_deref(), Some("Hover"));
        assert_eq!(name(None, true, true).as_deref(), Some("Down"));
        assert_eq!(name(Some("Visited"), true, false).as_deref(), Some("Hover"));
        assert_eq!(name(Some("Selected"), true, true).as_deref(), Some("Selected"));
    }

    #[test]
    fn a_viewed_state_sits_on_the_object_and_edits_go_back_to_the_state() {
        let mut o = rect();
        add_button_states(&mut o);
        o.states[1].fill = Some(Rgba(200, 0, 0, 255));
        let mut slides = slides_with(o);
        let swap = begin(&mut slides, 0, "a", "Hover").unwrap();
        assert_eq!(obj(&slides).fill, Some(Rgba(200, 0, 0, 255)), "the object shows Hover");
        // The ribbon recolours the object.
        slides[0].layers[0].objects[0].fill = Some(Rgba(0, 200, 0, 255));
        slides[0].layers[0].objects[0].text = "Over".into();
        end(&mut slides, swap);
        let o = obj(&slides);
        assert_eq!((o.fill, o.text.as_str()), (Some(Rgba(10, 20, 30, 255)), "Go"), "the object has its own look again");
        assert_eq!(o.states[1].fill, Some(Rgba(0, 200, 0, 255)), "Hover took the edit");
        assert_eq!(o.states[1].text, "Over");
        assert_eq!(o.states[0].fill, Some(Rgba(10, 20, 30, 255)), "Normal is untouched");
    }

    #[test]
    fn editing_the_starting_state_edits_the_object_and_that_state() {
        let mut o = rect();
        add_button_states(&mut o);
        let mut slides = slides_with(o);
        let swap = begin(&mut slides, 0, "a", "Normal").unwrap();
        slides[0].layers[0].objects[0].fill = Some(Rgba(9, 9, 9, 255));
        end(&mut slides, swap);
        assert_eq!(obj(&slides).fill, Some(Rgba(9, 9, 9, 255)));
        assert_eq!(obj(&slides).states[0].fill, Some(Rgba(9, 9, 9, 255)));
        assert_eq!(obj(&slides).states[1].fill, Some(Rgba(10, 20, 30, 255)), "other states keep their own");
    }

    #[test]
    fn an_undo_step_taken_mid_frame_holds_the_objects_own_look() {
        let mut o = rect();
        add_button_states(&mut o);
        o.states[1].fill = Some(Rgba(200, 0, 0, 255));
        let mut slides = slides_with(o);
        let swap = begin(&mut slides, 0, "a", "Hover").unwrap();
        let mut before = slides.clone();
        unswap(&mut before, &swap);
        assert_eq!(obj(&before).fill, Some(Rgba(10, 20, 30, 255)), "not Hover's red");
        assert_eq!(obj(&before).states[1].fill, Some(Rgba(200, 0, 0, 255)), "Hover is as it was");
    }

    #[test]
    fn making_the_viewed_state_the_starting_one_keeps_its_look_on_the_object() {
        let mut o = rect();
        add_button_states(&mut o);
        o.states[1].fill = Some(Rgba(200, 0, 0, 255));
        let mut slides = slides_with(o);
        let swap = begin(&mut slides, 0, "a", "Hover").unwrap();
        slides[0].layers[0].objects[0].initial_state = "Hover".into();
        end(&mut slides, swap);
        assert_eq!(obj(&slides).fill, Some(Rgba(200, 0, 0, 255)), "the object now starts as Hover");
    }

    #[test]
    fn a_picture_states_own_picture_comes_back_to_it_and_not_to_the_object() {
        let mut o = Object::new("p", "p", Kind::Image, 0.0, 0.0, 100.0, 100.0);
        o.asset = Some("one".into());
        add_button_states(&mut o);
        let mut slides = slides_with(o);
        let swap = begin(&mut slides, 0, "p", "Hover").unwrap();
        end(&mut slides, swap);
        assert_eq!(obj(&slides).states[1].asset.as_deref(), Some("one"), "viewing without editing changes nothing");
        let swap = begin(&mut slides, 0, "p", "Hover").unwrap();
        slides[0].layers[0].objects[0].asset = Some("two".into());
        end(&mut slides, swap);
        assert_eq!(obj(&slides).states[1].asset.as_deref(), Some("two"));
        assert_eq!(obj(&slides).asset.as_deref(), Some("one"), "the object keeps its own picture");
    }
}
