//! The Button Styles gallery: ready-made looks for a button's states, laid out like Storyline's (nine colors across,
//! four looks down, and a choice of square, rounded or pill corners).
use super::*;
use crate::button_faces::FACES;

/// How a style's fill is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Look {
    /// A flat color with a darker outline.
    Flat,
    /// Lighter at the top, darker at the bottom, with a drop shadow.
    Gradient,
    /// Lighter down the middle and darker at the sides, like a cylinder, with a drop shadow.
    Sheen,
    /// The top half lighter than the bottom half, with a crisp line between, and a drop shadow.
    Gloss,
    /// A flat color with nothing else (for the markers, which draw their own look).
    Plain,
}

impl Look {
    /// The four looks of the gallery, top to bottom.
    pub const GALLERY: [Look; 4] = [Look::Flat, Look::Gradient, Look::Sheen, Look::Gloss];
    pub fn label(self) -> &'static str {
        match self { Look::Flat => "Flat", Look::Gradient => "Gradient", Look::Sheen => "Side shine", Look::Gloss => "Split gloss", Look::Plain => "Plain" }
    }
}

/// How round a style's corners are.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Corners {
    Square,
    #[default]
    Rounded,
    Pill,
}

impl Corners {
    pub const ALL: [Corners; 3] = [Corners::Square, Corners::Rounded, Corners::Pill];
    pub fn radius(self) -> f32 { match self { Corners::Square => 0.0, Corners::Rounded => 5.0, Corners::Pill => 200.0 } }
    pub fn label(self) -> &'static str { match self { Corners::Square => "Square", Corners::Rounded => "Rounded", Corners::Pill => "Pill" } }
}

/// One of the gallery's colors.
pub struct Hue {
    pub name: &'static str,
    pub fill: Rgba,
    pub text: Rgba,
}

pub(crate) const fn c(r: u8, g: u8, b: u8) -> Rgba { Rgba(r, g, b, 255) }
const WHITE: Rgba = c(255, 255, 255);
const BLACK: Rgba = c(0, 0, 0);

/// Storyline's nine colors: black, light grey, its six theme accents and a bright blue.
pub const HUES: [Hue; 9] = [
    Hue { name: "Black", fill: c(38, 38, 38), text: WHITE },
    Hue { name: "Grey", fill: c(217, 217, 217), text: BLACK },
    Hue { name: "Blue", fill: c(79, 129, 189), text: WHITE },
    Hue { name: "Red", fill: c(192, 80, 77), text: WHITE },
    Hue { name: "Green", fill: c(155, 187, 89), text: WHITE },
    Hue { name: "Purple", fill: c(128, 100, 162), text: WHITE },
    Hue { name: "Teal", fill: c(75, 172, 198), text: WHITE },
    Hue { name: "Orange", fill: c(247, 150, 70), text: WHITE },
    Hue { name: "Bright blue", fill: c(24, 136, 231), text: WHITE },
];

/// One style: the color its states are worked out from, how that color is drawn, and the corners.
#[derive(Clone, Debug)]
pub struct StyleDef {
    pub name: &'static str,
    pub fill: Rgba,
    pub text: Rgba,
    pub look: Look,
    pub radius: f32,
    /// Which of the gallery's colors it is (`None` for a plain color that is not one of them).
    pub hue: Option<usize>,
}

impl StyleDef {
    pub fn new(hue: &Hue, look: Look, corners: Corners) -> StyleDef {
        let index = HUES.iter().position(|h| h.name == hue.name);
        StyleDef { name: hue.name, fill: hue.fill, text: hue.text, look, radius: corners.radius(), hue: index }
    }
}

/// One face of a style: a flat color or gradient (`stops` empty means flat), and an outline with its own opacity.
pub struct Face {
    pub fill: Rgba,
    /// CSS angle of the gradient: 0 is towards the top, 90 towards the right, 180 towards the bottom.
    pub angle: f32,
    pub stops: &'static [(f32, Rgba)],
    pub stroke: Option<Rgba>,
}

/// A style's resting, hover and pressed faces.
pub struct Faces {
    pub normal: Face,
    pub hover: Face,
    pub down: Face,
}

/// The drop shadow of the shaded looks (4 px below, softly blurred), as Storyline draws it.
const DROP: Shadow = Shadow { dx: 0.0, dy: 4.0, blur: 3.5, spread: 0.0, color: Rgba(0, 0, 0, 74) };

/// The gradient, outline and drop shadow that draw a state's face of a style.
fn parts(def: &StyleDef, state: &str) -> (Rgba, Option<Gradient>, Option<Stroke>, Option<Shadow>) {
    let flat = |fill: Rgba| (fill, None, None, None);
    let Some(hue) = def.hue.filter(|_| def.look != Look::Plain) else { return flat(def.fill) };
    let look = Look::GALLERY.iter().position(|l| *l == def.look).unwrap_or(0);
    let faces = &FACES[hue][look];
    let face = match state { "Hover" => &faces.hover, "Down" | "Selected" => &faces.down, _ => &faces.normal };
    let gradient = (!face.stops.is_empty()).then(|| Gradient { angle: face.angle, stops: face.stops.to_vec() });
    let stroke = face.stroke.map(|color| Stroke { color, width: 1.0 });
    let shadow = (def.look != Look::Flat).then_some(DROP);
    (face.fill, gradient, stroke, shadow)
}

/// Give `o` this style's look in every button state (Normal, Hover, Down, Visited, Disabled, Selected), keeping its
/// text, picture and size per state. An object with no states becomes a button.
pub fn apply(o: &mut Object, def: &StyleDef) {
    const NAMES: [&str; 6] = ["Normal", "Hover", "Down", "Visited", "Disabled", "Selected"];
    o.radius = def.radius;
    let mut style = o.style.clone();
    style.color = def.text;
    if def.look != Look::Plain { style.bold = true; }
    let old = std::mem::take(&mut o.states);
    o.states = NAMES.iter().map(|&name| {
        let prev = old.iter().find(|s| s.name == name).or_else(|| old.iter().find(|s| s.name == "Normal"));
        let disabled = name == "Disabled";
        let (base, gradient, stroke, shadow) = if disabled {
            let (_, _, stroke, _) = parts(def, "Normal");
            (c(200, 202, 206), None, stroke.map(|s| Stroke { color: c(160, 164, 170), width: s.width }), None)
        } else { parts(def, name) };
        let mut st = style.clone();
        if disabled { st.color = c(120, 124, 130); }
        ObjectState {
            name: name.into(),
            rich: prev.and_then(|s| s.rich.clone()).or_else(|| o.rich.clone()),
            fill: Some(base),
            gradient,
            stroke,
            shadow,
            text: prev.map(|s| s.text.clone()).unwrap_or_else(|| o.text.clone()),
            style: st,
            asset: prev.and_then(|s| s.asset.clone()),
            size: prev.and_then(|s| s.size),
            shine: None,
        }
    }).collect();
    o.initial_state = "Normal".into();
    o.fill = o.states[0].fill;
    o.gradient = o.states[0].gradient.clone();
    o.stroke = o.states[0].stroke;
    o.shadow = o.states[0].shadow;
    o.style = o.states[0].style.clone();
}

impl App {
    /// Apply a gallery style to every selected object that can be a button.
    pub(super) fn apply_button_style(&mut self, def: &StyleDef) {
        let sel: Vec<(usize, usize)> = self.selection().into_iter().filter(|&(l, k)| {
            self.project.as_ref().and_then(|p| p.slides.get(self.slide)).and_then(|s| s.layers.get(l)).and_then(|l| l.objects.get(k)).is_some_and(states_panel::supports_states)
        }).collect();
        if sel.is_empty() { return; }
        self.end_state_swap();
        self.snapshot();
        let layers = &mut self.project.as_mut().unwrap().slides[self.slide].layers;
        for (l, k) in sel { apply(&mut layers[l].objects[k], def); }
        self.dirty = true;
        self.status = format!("{} {} button style applied", def.name, def.look.label().to_lowercase());
    }
}

/// A swatch for the gallery: the style's resting look with "Abc" on it. Returns whether it was clicked.
pub fn swatch(ui: &mut egui::Ui, def: &StyleDef) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(46.0, 28.0), egui::Sense::click());
    let (base, gradient, stroke, _) = parts(def, "Normal");
    let radius = (def.radius * 0.3).min(14.0);
    let poly = rounded_rect_poly(rect, radius);
    match &gradient {
        Some(g) => fill_polygon(ui.painter(), &poly, &|p| p, &|p| gradient_at(&g.stops, gradient_t(rect, g.angle, p))),
        None => fill_polygon(ui.painter(), &poly, &|p| p, &|_| color(base)),
    }
    if let Some(s) = stroke { ui.painter().add(egui::Shape::closed_line(poly, egui::Stroke::new(1.0, color(s.color)))); }
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "Abc", egui::FontId::proportional(12.0), color(def.text));
    if resp.hovered() { ui.painter().rect_stroke(rect.expand(1.5), radius + 1.0, egui::Stroke::new(1.5, ui.visuals().selection.bg_fill), egui::StrokeKind::Outside); }
    resp.on_hover_text(format!("{}, {}", def.name, def.look.label().to_lowercase())).clicked()
}

/// The gallery itself: the corner choice, then a grid of nine colors by four looks. Returns the style clicked.
pub fn gallery(ui: &mut egui::Ui, corners: &mut Corners) -> Option<StyleDef> {
    let mut picked = None;
    ui.horizontal(|ui| {
        ui.label("Corners");
        for c in Corners::ALL { ui.selectable_value(corners, c, c.label()); }
    });
    ui.add_space(4.0);
    ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
    for look in Look::GALLERY {
        ui.horizontal(|ui| {
            for hue in &HUES {
                let def = StyleDef::new(hue, look, *corners);
                if swatch(ui, &def) { picked = Some(def); }
            }
        });
    }
    picked
}
