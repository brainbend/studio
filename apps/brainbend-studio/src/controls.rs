//! Editing sliders and dials in the properties panel.
use eframe::egui;
use crate::ribbon::combo;
use brainbend_doc::{Check, Dial, InputField, Object, Rgba, Slider};

/// Pick which number variable a control sets. Returns whether the choice changed.
fn variable_picker(ui: &mut egui::Ui, id: &str, current: &mut String, vars: &[(String, String)]) -> bool {
    let mut changed = false;
    let shown = vars.iter().find(|(k, _)| k == current).map(|(_, n)| n.as_str()).unwrap_or(if current.is_empty() { "none" } else { "(missing variable)" });
    ui.horizontal(|ui| {
        ui.label("Variable");
        combo(id).selected_text(shown).show_ui(ui, |ui| {
            if ui.selectable_label(current.is_empty(), "none").clicked() { current.clear(); changed = true; }
            for (key, name) in vars {
                if ui.selectable_label(key == current, name).clicked() { *current = key.clone(); changed = true; }
            }
        });
    });
    if vars.is_empty() {
        ui.weak("Add a matching variable on the Variables tab to link this control to.");
    }
    changed
}

fn range_fields(ui: &mut egui::Ui, min: &mut f32, max: &mut f32, step: &mut f32) -> bool {
    let mut changed = false;
    egui::Grid::new("control-range").show(ui, |ui| {
        ui.label("From");
        changed |= ui.add(egui::DragValue::new(min).speed(0.1)).changed();
        ui.end_row();
        ui.label("To");
        changed |= ui.add(egui::DragValue::new(max).speed(0.1)).changed();
        ui.end_row();
        ui.label("Step");
        changed |= ui.add(egui::DragValue::new(step).speed(0.05).range(0.0..=f32::MAX)).on_hover_text("The smallest change in value. 0 allows any value.").changed();
        ui.end_row();
    });
    if *max <= *min { ui.colored_label(egui::Color32::from_rgb(200, 120, 0), "\"To\" should be larger than \"From\"."); }
    changed
}

fn color_row(ui: &mut egui::Ui, label: &str, c: &mut Rgba) -> bool {
    ui.horizontal(|ui| { ui.label(label); crate::ribbon::color_swatch(ui, c) }).inner
}

/// The slider's variable, range, thumb and track. `pick_thumb` is set when the thumb picture should be chosen.
pub fn slider_editor(ui: &mut egui::Ui, sl: &mut Slider, vars: &[(String, String)], pick_thumb: &mut bool) -> bool {
    let mut changed = variable_picker(ui, "slider-var", &mut sl.variable, vars);
    changed |= range_fields(ui, &mut sl.min, &mut sl.max, &mut sl.step);
    ui.horizontal(|ui| {
        ui.label(if sl.thumb.is_some() { "Thumb: picture" } else { "Thumb: circle" });
        if ui.button("Choose picture…").clicked() { *pick_thumb = true; }
        if sl.thumb.is_some() && ui.small_button("Remove").clicked() { sl.thumb = None; changed = true; }
    });
    if sl.thumb.is_none() { changed |= color_row(ui, "Thumb color", &mut sl.thumb_color); }
    ui.horizontal(|ui| {
        ui.label("Thumb size");
        changed |= ui.add(egui::Slider::new(&mut sl.thumb_height, 0.1..=1.0).show_value(true)).on_hover_text("The thumb's height as a share of the slider's height").changed();
    });
    let mut visible = sl.track.is_some();
    if ui.checkbox(&mut visible, "Show the track").changed() { sl.track = visible.then_some(Rgba(150, 160, 175, 255)); changed = true; }
    if let Some(t) = sl.track.as_mut() { changed |= color_row(ui, "Track color", t); }
    changed
}

/// Make an object turn like a dial that sets a number variable.
fn dial_editor(ui: &mut egui::Ui, dial: &mut Option<Dial>, vars: &[(String, String)]) -> bool {
    let mut changed = false;
    if let Some(d) = dial.as_mut() {
        changed |= variable_picker(ui, "dial-var", &mut d.variable, vars);
        changed |= range_fields(ui, &mut d.min, &mut d.max, &mut d.step);
        ui.horizontal(|ui| {
            ui.label("Turn through");
            changed |= ui.add(egui::DragValue::new(&mut d.sweep).speed(1.0).range(10.0..=360.0).suffix("°")).on_hover_text("How far it turns from the lowest value to the highest. The middle of the range is how it is drawn.").changed();
        });
    }
    changed
}

/// What an object can be made to act as: nothing special, a check box (or radio button), or a dial.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Acts { Plain, Check, Dial }

/// The "Make Interactive" bar: choose what the object acts as, then set that up. An object is one or the other.
pub fn interactive_editor(ui: &mut egui::Ui, o: &mut Object, vars: &[(String, String)], can_check: bool, can_dial: bool) -> bool {
    let mut changed = false;
    let now = if o.check.is_some() { Acts::Check } else if o.dial.is_some() { Acts::Dial } else { Acts::Plain };
    let mut next = now;
    crate::ribbon::format_panel::row(ui, "Acts as", |ui, w| {
        let shown = match now { Acts::Plain => "Nothing special", Acts::Check => "Check box", Acts::Dial => "Dial" };
        combo("acts-as").width(w - 8.0).selected_text(shown).show_ui(ui, |ui| {
            if ui.selectable_label(now == Acts::Plain, "Nothing special").clicked() { next = Acts::Plain; }
            if can_check && ui.selectable_label(now == Acts::Check, "Check box").on_hover_text("Clicking toggles the object between its Normal and Selected states. Triggers can watch the state.").clicked() { next = Acts::Check; }
            if can_dial && ui.selectable_label(now == Acts::Dial, "Dial").on_hover_text("The learner turns the object by dragging it; the turn sets a number variable.").clicked() { next = Acts::Dial; }
        });
    });
    if next != now {
        o.check = None;
        o.dial = None;
        match next {
            Acts::Plain => {}
            Acts::Check => {
                // Leave room for the box beside the text.
                let size = (o.h * 0.5).clamp(12.0, 60.0);
                o.check = Some(Check { size, ..Check::new() });
                o.margins[0] = o.margins[0].max(size * 1.6);
            }
            Acts::Dial => o.dial = Some(Dial { variable: vars.first().map(|(k, _)| k.clone()).unwrap_or_default(), min: 0.0, max: 10.0, step: 1.0, sweep: 180.0, pivot: (0.0, 0.0) }),
        }
        changed = true;
    }
    match next {
        Acts::Check => changed |= check_editor(ui, o),
        Acts::Dial => changed |= dial_editor(ui, &mut o.dial, vars),
        Acts::Plain => {}
    }
    changed
}

/// Make a text object a check box: the learner ticks the box beside its text.
fn check_editor(ui: &mut egui::Ui, o: &mut Object) -> bool {
    let mut changed = false;
    if let Some(ck) = o.check.as_mut() {
        let mut radio = ck.radio.is_some();
        if ui.checkbox(&mut radio, "Radio button").on_hover_text("A round button: choosing it clears the others in its group on this slide, and it cannot be unticked by clicking it").changed() {
            ck.radio = radio.then(|| "Choices".to_string());
            changed = true;
        }
        if let Some(g) = ck.radio.as_mut() {
            ui.horizontal(|ui| {
                ui.label("Group");
                changed |= ui.add(egui::TextEdit::singleline(g).desired_width(f32::INFINITY).hint_text("buttons with the same group name go together")).changed();
            });
        }
        ui.horizontal(|ui| {
            ui.label("Box size");
            changed |= ui.add(egui::DragValue::new(&mut ck.size).speed(0.5).range(8.0..=200.0)).changed();
        });
        changed |= color_row(ui, "Box color", &mut ck.box_fill);
        let mut outline = ck.box_line.is_some();
        if ui.checkbox(&mut outline, "Outline").changed() { ck.box_line = outline.then_some(Rgba(150, 150, 150, 255)); changed = true; }
        if let Some(l) = ck.box_line.as_mut() { changed |= color_row(ui, "Outline color", l); }
        changed |= color_row(ui, "Tick color", &mut ck.tick);
        let mut ticked = o.initial_state == "Selected";
        if ui.checkbox(&mut ticked, "Ticked at the start").changed() { o.initial_state = if ticked { "Selected".into() } else { "Normal".into() }; changed = true; }
    }
    changed
}

/// A data-entry field: the variable it fills (a text variable, or a number variable for a number field), the faint
/// prompt, and for a number field its range and step.
pub fn input_editor(ui: &mut egui::Ui, f: &mut InputField, text_vars: &[(String, String)], num_vars: &[(String, String)]) -> bool {
    let mut changed = false;
    let vars = if f.numeric { num_vars } else { text_vars };
    changed |= variable_picker(ui, "input-var", &mut f.variable, vars);
    if !f.numeric {
        ui.horizontal(|ui| {
            ui.label("Prompt");
            changed |= ui.add(egui::TextEdit::singleline(&mut f.placeholder).desired_width(f32::INFINITY).hint_text("faint text while empty")).changed();
        });
    } else {
        changed |= range_fields(ui, &mut f.min, &mut f.max, &mut f.step);
    }
    changed
}
