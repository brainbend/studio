//! The Icons gallery: Phosphor's regular-weight icons, inserted as ordinary vector shapes.
//!
//! The icon font (MIT, in `assets/icons`) is already bundled for the app's own buttons. Its glyphs are outlines, so an
//! icon placed on a slide is a freeform path: it scales without blurring, takes Shape Fill and publishes as SVG.
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::raw::FontRef;
use skrifa::MetadataProvider;
use std::sync::OnceLock;
use brainbend_doc::PathCmd;

static FONT: &[u8] = include_bytes!("../../../assets/icons/Phosphor.ttf");
static NAMES: &str = include_str!("../../../assets/icons/names.txt");

/// One icon: the character it has in the icon font, and its names (the first is the main one).
pub struct Icon {
    pub ch: char,
    pub names: Vec<&'static str>,
}

impl Icon {
    /// The name as words, for alt text and the object's name: "magnifying glass".
    pub fn title(&self) -> String { self.names[0].replace('-', " ") }
}

/// Every icon, in the font's order.
pub fn all() -> &'static [Icon] {
    static ALL: OnceLock<Vec<Icon>> = OnceLock::new();
    ALL.get_or_init(|| {
        NAMES.lines().filter_map(|l| {
            let mut it = l.split(' ');
            let ch = char::from_u32(u32::from_str_radix(it.next()?, 16).ok()?)?;
            let names: Vec<&str> = it.collect();
            (!names.is_empty()).then_some(Icon { ch, names })
        }).collect()
    })
}

/// The icons whose name contains every word of `query` (all of them for an empty query).
pub fn search(query: &str) -> Vec<&'static Icon> {
    let words: Vec<String> = query.to_lowercase().split_whitespace().map(|w| w.replace('-', " ")).collect();
    all().iter().filter(|i| {
        let hay = i.names.join(" ").replace('-', " ");
        words.iter().all(|w| hay.contains(w.as_str()))
    }).collect()
}

/// Collects a glyph's outline as path commands in font units.
#[derive(Default)]
struct Pen { cmds: Vec<Seg>, at: (f32, f32) }

#[derive(Clone, Copy)]
enum Seg { Move(f32, f32), Line(f32, f32), Cubic([f32; 6]), Close }

impl OutlinePen for Pen {
    fn move_to(&mut self, x: f32, y: f32) { self.cmds.push(Seg::Move(x, y)); self.at = (x, y); }
    fn line_to(&mut self, x: f32, y: f32) { self.cmds.push(Seg::Line(x, y)); self.at = (x, y); }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        // A quadratic curve as the equal cubic: both control points two thirds of the way to the quadratic one.
        let (px, py) = self.at;
        self.cmds.push(Seg::Cubic([px + 2.0 / 3.0 * (cx - px), py + 2.0 / 3.0 * (cy - py), x + 2.0 / 3.0 * (cx - x), y + 2.0 / 3.0 * (cy - y), x, y]));
        self.at = (x, y);
    }
    fn curve_to(&mut self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) { self.cmds.push(Seg::Cubic([c1x, c1y, c2x, c2y, x, y])); self.at = (x, y); }
    fn close(&mut self) { self.cmds.push(Seg::Close); }
}

/// The icon's outline as a path in 0..1 coordinates of its own bounding box (y down), and the box's width over height.
pub fn outline(icon: &Icon) -> Option<(Vec<PathCmd>, f32)> {
    let font = FontRef::new(FONT).ok()?;
    let gid = font.charmap().map(icon.ch as u32)?;
    let glyph = font.outline_glyphs().get(gid)?;
    let mut pen = Pen::default();
    glyph.draw(DrawSettings::unhinted(Size::unscaled(), LocationRef::default()), &mut pen).ok()?;
    // The box the drawing really fills: on-curve points plus a few samples along each curve.
    let (mut lo, mut hi) = ((f32::MAX, f32::MAX), (f32::MIN, f32::MIN));
    let mut grow = |x: f32, y: f32| { lo = (lo.0.min(x), lo.1.min(y)); hi = (hi.0.max(x), hi.1.max(y)); };
    let mut cur = (0.0, 0.0);
    for s in &pen.cmds {
        match *s {
            Seg::Move(x, y) | Seg::Line(x, y) => { grow(x, y); cur = (x, y); }
            Seg::Cubic([a, b, c, d, x, y]) => {
                for i in 1..=8 {
                    let (t, u) = (i as f32 / 8.0, 1.0 - i as f32 / 8.0);
                    grow(cur.0 * u * u * u + 3.0 * a * u * u * t + 3.0 * c * u * t * t + x * t * t * t, cur.1 * u * u * u + 3.0 * b * u * u * t + 3.0 * d * u * t * t + y * t * t * t);
                }
                cur = (x, y);
            }
            Seg::Close => {}
        }
    }
    let (w, h) = ((hi.0 - lo.0).max(1.0), (hi.1 - lo.1).max(1.0));
    let (nx, ny) = (move |x: f32| (x - lo.0) / w, move |y: f32| (hi.1 - y) / h);
    let cmds = pen.cmds.iter().map(|s| match *s {
        Seg::Move(x, y) => PathCmd::Move(nx(x), ny(y)),
        Seg::Line(x, y) => PathCmd::Line(nx(x), ny(y)),
        Seg::Cubic([a, b, c, d, x, y]) => PathCmd::Curve(nx(a), ny(b), nx(c), ny(d), nx(x), ny(y)),
        Seg::Close => PathCmd::Close,
    }).collect();
    Some((cmds, w / h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_names_cover_the_font_and_search_finds_by_any_word() {
        assert!(all().len() > 1500);
        let globe = search("globe");
        assert!(globe.iter().any(|i| i.names[0] == "globe"));
        assert!(search("magnifying glass").iter().any(|i| i.names[0] == "magnifying-glass"), "words match across the hyphen");
        assert!(search("zzzz-not-an-icon").is_empty());
        // The names were checked against the bundled font: these codes are the icons the ribbon already uses.
        let named = |ch: char| all().iter().find(|i| i.ch == ch).map(|i| i.names[0]);
        assert_eq!((named('\u{e288}'), named('\u{e30c}'), named('\u{e4a6}')), (Some("globe"), Some("magnifying-glass"), Some("trash")));
    }

    #[test]
    fn an_icon_becomes_a_closed_path_in_the_unit_box_with_its_holes() {
        let ring = all().iter().find(|i| i.names[0] == "circle").unwrap();
        let (cmds, aspect) = outline(ring).unwrap();
        assert!((aspect - 1.0).abs() < 0.05, "a circle is square: {aspect}");
        assert_eq!(cmds.iter().filter(|c| matches!(c, PathCmd::Move(..))).count(), 2, "an outer and an inner contour");
        for c in &cmds {
            let pts: Vec<f32> = match *c { PathCmd::Move(x, y) | PathCmd::Line(x, y) => vec![x, y], PathCmd::Curve(a, b, c, d, e, f) => vec![a, b, c, d, e, f], PathCmd::Close => vec![] };
            assert!(pts.iter().all(|v| (-0.2..=1.2).contains(v)), "{pts:?} stays inside the box (curve handles may poke out a little)");
        }
    }

    #[test]
    fn every_icon_has_an_outline() {
        let missing: Vec<_> = all().iter().filter(|i| outline(i).is_none()).map(|i| i.names[0]).collect();
        assert!(missing.is_empty(), "{missing:?}");
    }
}
