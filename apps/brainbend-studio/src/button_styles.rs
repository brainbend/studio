//! The Button Styles gallery: ready-made looks for a button's states (fill, outline, text colour, shine, corners).
use super::*;

/// One style: the colours the states are worked out from.
pub struct StyleDef {
    pub name: &'static str,
    pub fill: Rgba,
    pub text: Rgba,
    pub outline: Option<Rgba>,
    pub radius: f32,
    /// The glossy highlight: faint at rest, stronger on hover and press.
    pub glossy: bool,
}

const fn c(r: u8, g: u8, b: u8) -> Rgba { Rgba(r, g, b, 255) }
const WHITE: Rgba = c(255, 255, 255);

pub const STYLES: [StyleDef; 10] = [
    StyleDef { name: "Blue", fill: c(3, 83, 158), text: WHITE, outline: None, radius: 5.0, glossy: true },
    StyleDef { name: "Teal", fill: c(58, 148, 168), text: WHITE, outline: None, radius: 5.0, glossy: true },
    StyleDef { name: "Orange", fill: c(237, 125, 49), text: WHITE, outline: None, radius: 5.0, glossy: true },
    StyleDef { name: "Green", fill: c(46, 140, 70), text: WHITE, outline: None, radius: 5.0, glossy: false },
    StyleDef { name: "Red", fill: c(192, 48, 48), text: WHITE, outline: None, radius: 5.0, glossy: false },
    StyleDef { name: "Purple", fill: c(108, 70, 160), text: WHITE, outline: None, radius: 5.0, glossy: false },
    StyleDef { name: "Charcoal", fill: c(58, 62, 70), text: WHITE, outline: None, radius: 0.0, glossy: false },
    StyleDef { name: "Outline", fill: WHITE, text: c(3, 83, 158), outline: Some(c(3, 83, 158)), radius: 5.0, glossy: false },
    StyleDef { name: "Soft", fill: c(222, 234, 248), text: c(20, 50, 90), outline: None, radius: 5.0, glossy: false },
    StyleDef { name: "Pill", fill: c(3, 83, 158), text: WHITE, outline: None, radius: 200.0, glossy: false },
];

fn mix(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round().clamp(0.0, 255.0) as u8;
    Rgba(m(a.0, b.0), m(a.1, b.1), m(a.2, b.2), 255)
}

/// The fill a state shows: hover lifts it, a press darkens it, disabled greys it, and a chosen button looks pressed.
fn state_fill(def: &StyleDef, state: &str) -> Rgba {
    let light = def.fill.0 as u32 + def.fill.1 as u32 + def.fill.2 as u32 > 690; // a pale fill is darkened on hover instead
    match state {
        "Hover" => if light { mix(def.fill, c(0, 0, 0), 0.07) } else { mix(def.fill, WHITE, 0.14) },
        "Down" | "Selected" => mix(def.fill, c(0, 0, 0), 0.22),
        "Disabled" => c(200, 202, 206),
        _ => def.fill,
    }
}

/// Give `o` this style's look in every button state (Normal, Hover, Down, Visited, Disabled, Selected), keeping its
/// text, picture and size per state. An object with no states becomes a button.
pub fn apply(o: &mut Object, def: &StyleDef) {
    const NAMES: [&str; 6] = ["Normal", "Hover", "Down", "Visited", "Disabled", "Selected"];
    o.radius = def.radius;
    let mut style = o.style.clone();
    style.color = def.text;
    let outline = def.outline.map(|color| Stroke { color, width: 3.0 });
    let shadow = if def.outline.is_some() || def.fill.0 as u32 + def.fill.1 as u32 + def.fill.2 as u32 > 690 { None } else { Some(Shadow { dx: 0.0, dy: 3.0, blur: 6.0, spread: 0.0, color: Rgba(0, 0, 0, 80) }) };
    let old = std::mem::take(&mut o.states);
    o.states = NAMES.iter().map(|&name| {
        let prev = old.iter().find(|s| s.name == name).or_else(|| old.iter().find(|s| s.name == "Normal"));
        let shine = def.glossy.then(|| match name {
            "Hover" => Shine { alpha: 0.30, reverse: false },
            "Down" | "Selected" => Shine { alpha: 0.30, reverse: true },
            "Disabled" => Shine { alpha: 0.05, reverse: false },
            _ => Shine { alpha: 0.14, reverse: false },
        });
        let mut st = style.clone();
        if name == "Disabled" { st.color = c(120, 124, 130); }
        ObjectState {
            name: name.into(),
            rich: prev.and_then(|s| s.rich.clone()).or_else(|| o.rich.clone()),
            fill: Some(state_fill(def, name)),
            stroke: if name == "Disabled" { outline.map(|s| Stroke { color: c(160, 164, 170), width: s.width }) } else { outline },
            shadow: if name == "Disabled" { None } else { shadow },
            text: prev.map(|s| s.text.clone()).unwrap_or_else(|| o.text.clone()),
            style: st,
            asset: prev.and_then(|s| s.asset.clone()),
            size: prev.and_then(|s| s.size),
            shine,
        }
    }).collect();
    o.initial_state = "Normal".into();
    o.fill = o.states[0].fill;
    o.stroke = o.states[0].stroke;
    o.shadow = o.states[0].shadow;
    o.style = o.states[0].style.clone();
}

impl App {
    /// Apply style `i` of the gallery to every selected object that can be a button.
    pub(super) fn apply_button_style(&mut self, i: usize) {
        let Some(def) = STYLES.get(i) else { return };
        let sel: Vec<(usize, usize)> = self.selection().into_iter().filter(|&(l, k)| {
            self.project.as_ref().and_then(|p| p.slides.get(self.slide)).and_then(|s| s.layers.get(l)).and_then(|l| l.objects.get(k)).is_some_and(states_panel::supports_states)
        }).collect();
        if sel.is_empty() { return; }
        self.end_state_swap();
        self.snapshot();
        let layers = &mut self.project.as_mut().unwrap().slides[self.slide].layers;
        for (l, k) in sel { apply(&mut layers[l].objects[k], def); }
        self.dirty = true;
        self.status = format!("{} button style applied", def.name);
    }
}

/// A swatch for the gallery: the style's resting look with "Abc" on it. Returns whether it was clicked.
pub fn swatch(ui: &mut egui::Ui, def: &StyleDef) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(46.0, 28.0), egui::Sense::click());
    let f = egui::Color32::from_rgb(def.fill.0, def.fill.1, def.fill.2);
    let radius = (def.radius * 0.3).min(14.0);
    ui.painter().rect_filled(rect, radius, f);
    if let Some(o) = def.outline { ui.painter().rect_stroke(rect.shrink(1.0), radius, egui::Stroke::new(1.5, egui::Color32::from_rgb(o.0, o.1, o.2)), egui::StrokeKind::Inside); }
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "Abc", egui::FontId::proportional(12.0), egui::Color32::from_rgb(def.text.0, def.text.1, def.text.2));
    if resp.hovered() { ui.painter().rect_stroke(rect.expand(1.0), radius, egui::Stroke::new(1.5, ui.visuals().selection.bg_fill), egui::StrokeKind::Outside); }
    resp.on_hover_text(def.name).clicked()
}
