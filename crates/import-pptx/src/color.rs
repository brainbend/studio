//! DrawingML colours, fills and the theme's colour/format scheme.
use roxmltree::Node;
use std::collections::HashMap;
use brainbend_doc::Rgba;

pub fn child<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    n.children().find(|c| c.is_element() && c.tag_name().name() == name)
}

pub fn children<'a, 'i>(n: Node<'a, 'i>, name: &'a str) -> impl Iterator<Item = Node<'a, 'i>> {
    n.children().filter(move |c| c.is_element() && c.tag_name().name() == name)
}

pub fn attr_f(n: Node, name: &str) -> Option<f32> {
    n.attribute(name).and_then(|v| v.trim().parse::<f32>().ok())
}

#[derive(Clone, Debug)]
pub enum Base {
    Rgb(u8, u8, u8),
    Scheme(String),
    /// The colour a style reference passes in ("placeholder colour").
    PhClr,
}

/// A colour as written (base plus the transforms applied in order); resolved against a theme later.
#[derive(Clone, Debug)]
pub struct ColorSpec {
    pub base: Base,
    pub mods: Vec<(String, f32)>,
}

pub struct ColorCtx<'a> {
    pub scheme: &'a HashMap<String, (u8, u8, u8)>,
    pub clr_map: &'a HashMap<String, String>,
    pub ph: Option<Rgba>,
}

fn hex(v: &str) -> Option<(u8, u8, u8)> {
    if v.len() != 6 { return None; }
    let p = |i: usize| u8::from_str_radix(&v[i..i + 2], 16).ok();
    Some((p(0)?, p(2)?, p(4)?))
}

fn preset(name: &str) -> (u8, u8, u8) {
    match name {
        "black" => (0, 0, 0),
        "white" => (255, 255, 255),
        "red" => (255, 0, 0),
        "green" => (0, 128, 0),
        "blue" => (0, 0, 255),
        "yellow" => (255, 255, 0),
        "gray" | "grey" => (128, 128, 128),
        "orange" => (255, 165, 0),
        "purple" => (128, 0, 128),
        "cyan" | "aqua" => (0, 255, 255),
        "magenta" | "fuchsia" => (255, 0, 255),
        "silver" => (192, 192, 192),
        "darkBlue" => (0, 0, 139),
        "lightGray" | "lightGrey" => (211, 211, 211),
        "darkGray" | "darkGrey" => (169, 169, 169),
        _ => (0, 0, 0),
    }
}

/// Parse a colour element (`srgbClr`, `schemeClr`, `sysClr`, `prstClr`, ...).
pub fn parse_color(n: Node) -> Option<ColorSpec> {
    let val = n.attribute("val").unwrap_or("");
    let base = match n.tag_name().name() {
        "srgbClr" => { let (r, g, b) = hex(val)?; Base::Rgb(r, g, b) }
        "sysClr" => match n.attribute("lastClr").and_then(hex) {
            Some((r, g, b)) => Base::Rgb(r, g, b),
            None => if val == "window" { Base::Rgb(255, 255, 255) } else { Base::Rgb(0, 0, 0) },
        },
        "prstClr" => { let (r, g, b) = preset(val); Base::Rgb(r, g, b) }
        "schemeClr" => if val == "phClr" { Base::PhClr } else { Base::Scheme(val.to_string()) },
        "scrgbClr" => {
            let c = |a: &str| (attr_f(n, a).unwrap_or(0.0) / 100000.0 * 255.0).round().clamp(0.0, 255.0) as u8;
            Base::Rgb(c("r"), c("g"), c("b"))
        }
        _ => return None,
    };
    let mods = n.children().filter(|c| c.is_element()).filter_map(|c| Some((c.tag_name().name().to_string(), attr_f(c, "val")?))).collect();
    Some(ColorSpec { base, mods })
}

/// The first colour child of `parent` (e.g. inside `solidFill`).
pub fn first_color(parent: Node) -> Option<ColorSpec> {
    parent.children().filter(|c| c.is_element()).find_map(parse_color)
}

fn to_hsl(c: (f32, f32, f32)) -> (f32, f32, f32) {
    let (r, g, b) = (c.0 / 255.0, c.1 / 255.0, c.2 / 255.0);
    let (mx, mn) = (r.max(g).max(b), r.min(g).min(b));
    let l = (mx + mn) / 2.0;
    if (mx - mn).abs() < 1e-6 { return (0.0, 0.0, l); }
    let d = mx - mn;
    let s = if l > 0.5 { d / (2.0 - mx - mn) } else { d / (mx + mn) };
    let h = if mx == r { (g - b) / d + if g < b { 6.0 } else { 0.0 } } else if mx == g { (b - r) / d + 2.0 } else { (r - g) / d + 4.0 };
    (h / 6.0, s, l)
}

fn from_hsl(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s == 0.0 { let v = l * 255.0; return (v, v, v); }
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    let f = |mut t: f32| {
        if t < 0.0 { t += 1.0; }
        if t > 1.0 { t -= 1.0; }
        let v = if t < 1.0 / 6.0 { p + (q - p) * 6.0 * t } else if t < 0.5 { q } else if t < 2.0 / 3.0 { p + (q - p) * (2.0 / 3.0 - t) * 6.0 } else { p };
        v * 255.0
    };
    (f(h + 1.0 / 3.0), f(h), f(h - 1.0 / 3.0))
}

impl ColorSpec {
    pub fn resolve(&self, cx: &ColorCtx) -> Rgba {
        let mut alpha = 1.0f32;
        let (mut rgb, mut ph_alpha) = match &self.base {
            Base::Rgb(r, g, b) => ((*r as f32, *g as f32, *b as f32), 1.0),
            Base::PhClr => match cx.ph { Some(c) => ((c.0 as f32, c.1 as f32, c.2 as f32), c.3 as f32 / 255.0), None => ((0.0, 0.0, 0.0), 1.0) },
            Base::Scheme(name) => {
                let mapped = cx.clr_map.get(name).map(String::as_str).unwrap_or(name.as_str());
                let c = cx.scheme.get(mapped).copied().unwrap_or(match mapped { "lt1" | "bg1" => (255, 255, 255), _ => (0, 0, 0) });
                ((c.0 as f32, c.1 as f32, c.2 as f32), 1.0)
            }
        };
        alpha *= ph_alpha;
        ph_alpha = 1.0;
        let _ = ph_alpha;
        for (name, v) in &self.mods {
            let f = v / 100000.0;
            match name.as_str() {
                "lumMod" | "lumOff" | "satMod" | "satOff" | "hueMod" | "hueOff" => {
                    let (mut h, mut s, mut l) = to_hsl(rgb);
                    match name.as_str() {
                        "lumMod" => l = (l * f).clamp(0.0, 1.0),
                        "lumOff" => l = (l + f).clamp(0.0, 1.0),
                        "satMod" => s = (s * f).clamp(0.0, 1.0),
                        "satOff" => s = (s + f).clamp(0.0, 1.0),
                        "hueMod" => h = (h * f).rem_euclid(1.0),
                        _ => h = (h + v / 21600000.0).rem_euclid(1.0),
                    }
                    rgb = from_hsl(h, s, l);
                }
                "tint" => rgb = (rgb.0 * f + 255.0 * (1.0 - f), rgb.1 * f + 255.0 * (1.0 - f), rgb.2 * f + 255.0 * (1.0 - f)),
                "shade" => rgb = (rgb.0 * f, rgb.1 * f, rgb.2 * f),
                "alpha" => alpha = f,
                "alphaMod" => alpha *= f,
                "alphaOff" => alpha = (alpha + f).clamp(0.0, 1.0),
                "inv" => rgb = (255.0 - rgb.0, 255.0 - rgb.1, 255.0 - rgb.2),
                "gray" => { let g = 0.3 * rgb.0 + 0.59 * rgb.1 + 0.11 * rgb.2; rgb = (g, g, g); }
                _ => {}
            }
        }
        let b = |v: f32| v.round().clamp(0.0, 255.0) as u8;
        Rgba(b(rgb.0), b(rgb.1), b(rgb.2), b(alpha * 255.0))
    }
}

/// A fill as written.
#[derive(Clone, Debug)]
pub enum FillSpec {
    NoFill,
    Solid(ColorSpec),
    /// CSS-style angle (degrees clockwise from "up") and stops.
    Grad { angle: f32, stops: Vec<(f32, ColorSpec)> },
    /// A picture: relationship id and crop fractions (left, top, right, bottom).
    Blip { embed: String, crop: [f32; 4] },
    Pattern(ColorSpec),
}

/// The fill element directly inside `parent` (`spPr`, `bgPr`, `ln`, `tcPr`, ...), if there is one.
pub fn parse_fill(parent: Node) -> Option<FillSpec> {
    for c in parent.children().filter(|c| c.is_element()) {
        match c.tag_name().name() {
            "noFill" => return Some(FillSpec::NoFill),
            "solidFill" => return Some(first_color(c).map(FillSpec::Solid).unwrap_or(FillSpec::NoFill)),
            "gradFill" => {
                let mut stops: Vec<(f32, ColorSpec)> = child(c, "gsLst")
                    .map(|l| children(l, "gs").filter_map(|g| Some((attr_f(g, "pos")? / 100000.0, first_color(g)?))).collect())
                    .unwrap_or_default();
                stops.sort_by(|a, b| a.0.total_cmp(&b.0));
                if stops.is_empty() { return Some(FillSpec::NoFill); }
                let ang = child(c, "lin").and_then(|l| attr_f(l, "ang")).unwrap_or(5400000.0) / 60000.0;
                // DrawingML: 0 = left to right, clockwise. CSS: 90 = to the right.
                return Some(FillSpec::Grad { angle: (ang + 90.0).rem_euclid(360.0), stops });
            }
            "blipFill" => {
                let blip = child(c, "blip")?;
                let embed = blip.attributes().find(|a| a.name() == "embed")?.value().to_string();
                let mut crop = [0.0; 4];
                if let Some(r) = child(c, "srcRect") {
                    for (i, k) in ["l", "t", "r", "b"].iter().enumerate() { crop[i] = (attr_f(r, k).unwrap_or(0.0) / 100000.0).clamp(0.0, 0.95); }
                }
                return Some(FillSpec::Blip { embed, crop });
            }
            "pattFill" => return Some(child(c, "fgClr").and_then(first_color).map(FillSpec::Pattern).unwrap_or(FillSpec::NoFill)),
            "grpFill" => return Some(FillSpec::NoFill),
            _ => {}
        }
    }
    None
}

impl FillSpec {
    /// A single colour standing for the fill (gradients use their middle stop).
    pub fn flat(&self, cx: &ColorCtx) -> Option<Rgba> {
        match self {
            FillSpec::NoFill | FillSpec::Blip { .. } => None,
            FillSpec::Solid(c) | FillSpec::Pattern(c) => Some(c.resolve(cx)),
            FillSpec::Grad { stops, .. } => {
                let mid = stops.len() / 2;
                stops.get(mid).map(|(_, c)| c.resolve(cx))
            }
        }
    }
}

/// Theme data needed to resolve colours and style references.
#[derive(Clone, Debug, Default)]
pub struct ThemeData {
    pub name: String,
    pub scheme: HashMap<String, (u8, u8, u8)>,
    pub major: String,
    pub minor: String,
    pub fills: Vec<FillSpec>,
    pub bg_fills: Vec<FillSpec>,
    /// Line widths (EMU) of the theme's line styles.
    pub line_widths: Vec<f32>,
}

pub fn parse_theme(root: Node) -> ThemeData {
    let mut t = ThemeData::default();
    t.name = root.attribute("name").unwrap_or("Imported").to_string();
    for n in root.descendants().filter(|n| n.is_element() && n.tag_name().name() == "clrScheme") {
        for c in n.children().filter(|c| c.is_element()) {
            let name = c.tag_name().name().to_string();
            if let Some(ColorSpec { base: Base::Rgb(r, g, b), .. }) = first_color(c) { t.scheme.insert(name, (r, g, b)); }
        }
    }
    let font = |which: &str| {
        root.descendants().find(|n| n.is_element() && n.tag_name().name() == which).and_then(|f| child(f, "latin")).and_then(|l| l.attribute("typeface")).unwrap_or("Open Sans").to_string()
    };
    t.major = font("majorFont");
    t.minor = font("minorFont");
    if let Some(fs) = root.descendants().find(|n| n.is_element() && n.tag_name().name() == "fmtScheme") {
        let list = |name: &str| -> Vec<FillSpec> {
            child(fs, name).map(|l| l.children().filter(|c| c.is_element()).map(|c| {
                // Wrap so `parse_fill` sees the fill as a child.
                parse_fill_self(c).unwrap_or(FillSpec::NoFill)
            }).collect()).unwrap_or_default()
        };
        t.fills = list("fillStyleLst");
        t.bg_fills = list("bgFillStyleLst");
        t.line_widths = child(fs, "lnStyleLst").map(|l| children(l, "ln").map(|n| attr_f(n, "w").unwrap_or(9525.0)).collect()).unwrap_or_default();
    }
    t
}

/// Parse a fill element that is itself the node (`solidFill`, `gradFill`, ...).
fn parse_fill_self(n: Node) -> Option<FillSpec> {
    match n.tag_name().name() {
        "noFill" => Some(FillSpec::NoFill),
        "solidFill" => Some(first_color(n).map(FillSpec::Solid).unwrap_or(FillSpec::NoFill)),
        "gradFill" => {
            // Reuse the parent-based parser through a tiny shim: the node's own children are what it reads.
            let mut stops: Vec<(f32, ColorSpec)> = child(n, "gsLst").map(|l| children(l, "gs").filter_map(|g| Some((attr_f(g, "pos")? / 100000.0, first_color(g)?))).collect()).unwrap_or_default();
            stops.sort_by(|a, b| a.0.total_cmp(&b.0));
            if stops.is_empty() { return Some(FillSpec::NoFill); }
            let ang = child(n, "lin").and_then(|l| attr_f(l, "ang")).unwrap_or(5400000.0) / 60000.0;
            Some(FillSpec::Grad { angle: (ang + 90.0).rem_euclid(360.0), stops })
        }
        _ => None,
    }
}
