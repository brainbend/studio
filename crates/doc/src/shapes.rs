//! Preset shapes (the shapes gallery) as unit-square paths.
use crate::PathCmd;

pub fn poly(pts: &[(f32, f32)]) -> Vec<PathCmd> {
    let mut v = Vec::with_capacity(pts.len() + 1);
    for (i, &(x, y)) in pts.iter().enumerate() {
        v.push(if i == 0 { PathCmd::Move(x, y) } else { PathCmd::Line(x, y) });
    }
    v.push(PathCmd::Close);
    v
}

/// Scale a point list so it exactly fills the unit square (for stars, whose presets fill their frame).
fn fit(mut pts: Vec<(f32, f32)>) -> Vec<(f32, f32)> {
    let (x0, x1) = pts.iter().fold((f32::MAX, f32::MIN), |a, p| (a.0.min(p.0), a.1.max(p.0)));
    let (y0, y1) = pts.iter().fold((f32::MAX, f32::MIN), |a, p| (a.0.min(p.1), a.1.max(p.1)));
    for p in pts.iter_mut() {
        p.0 = (p.0 - x0) / (x1 - x0).max(1e-6);
        p.1 = (p.1 - y0) / (y1 - y0).max(1e-6);
    }
    pts
}

fn star(points: usize, inner: f32) -> Vec<(f32, f32)> {
    let mut pts = vec![];
    for i in 0..points * 2 {
        let r = if i % 2 == 0 { 0.5 } else { 0.5 * inner };
        let a = -std::f32::consts::FRAC_PI_2 + std::f32::consts::PI * i as f32 / points as f32;
        pts.push((0.5 + r * a.cos(), 0.5 + r * a.sin()));
    }
    fit(pts)
}

/// A rectangle whose corners (top-left, top-right, bottom-right, bottom-left) are each rounded or
/// snipped. Each corner is `(round, rx, ry)`: the cut's size along x and y as fractions of the
/// frame; zero leaves the corner square.
pub fn cut_rect(corners: [(bool, f32, f32); 4]) -> Vec<PathCmd> {
    const K: f32 = 0.5523;
    let [tl, tr, br, bl] = corners;
    // A corner from the point where its first edge ends to where its second begins, around `pivot`.
    let corner = |v: &mut Vec<PathCmd>, c: (bool, f32, f32), from: (f32, f32), to: (f32, f32), pivot: (f32, f32)| {
        if c.1 <= 0.0 || c.2 <= 0.0 { return; }
        if c.0 {
            let (c1, c2) = ((from.0 + K * (pivot.0 - from.0), from.1 + K * (pivot.1 - from.1)), (to.0 + K * (pivot.0 - to.0), to.1 + K * (pivot.1 - to.1)));
            v.push(PathCmd::Curve(c1.0, c1.1, c2.0, c2.1, to.0, to.1));
        } else {
            v.push(PathCmd::Line(to.0, to.1));
        }
    };
    let (tlx, tly) = if tl.1 > 0.0 && tl.2 > 0.0 { (tl.1, tl.2) } else { (0.0, 0.0) };
    let mut v = vec![PathCmd::Move(tlx, 0.0)];
    let (trx, try_) = if tr.1 > 0.0 && tr.2 > 0.0 { (tr.1, tr.2) } else { (0.0, 0.0) };
    v.push(PathCmd::Line(1.0 - trx, 0.0));
    corner(&mut v, tr, (1.0 - trx, 0.0), (1.0, try_), (1.0, 0.0));
    let (brx, bry) = if br.1 > 0.0 && br.2 > 0.0 { (br.1, br.2) } else { (0.0, 0.0) };
    v.push(PathCmd::Line(1.0, 1.0 - bry));
    corner(&mut v, br, (1.0, 1.0 - bry), (1.0 - brx, 1.0), (1.0, 1.0));
    let (blx, bly) = if bl.1 > 0.0 && bl.2 > 0.0 { (bl.1, bl.2) } else { (0.0, 0.0) };
    v.push(PathCmd::Line(blx, 1.0));
    corner(&mut v, bl, (blx, 1.0), (0.0, 1.0 - bly), (0.0, 1.0));
    v.push(PathCmd::Line(0.0, tly));
    corner(&mut v, tl, (0.0, tly), (tlx, 0.0), (0.0, 0.0));
    v.push(PathCmd::Close);
    v
}

/// A multiplication sign (an X) whose arms are `e` wide, as a fraction of the frame.
pub fn multiply(e: f32) -> Vec<PathCmd> {
    let e = e.clamp(0.01, 0.49);
    poly(&[(e, 0.0), (0.5, 0.5 - e), (1.0 - e, 0.0), (1.0, e), (0.5 + e, 0.5), (1.0, 1.0 - e), (1.0 - e, 1.0), (0.5, 0.5 + e), (e, 1.0), (0.0, 1.0 - e), (0.5 - e, 0.5), (0.0, e)])
}

/// A check mark.
pub fn check_mark() -> Vec<PathCmd> {
    poly(&[(0.0, 0.58), (0.17, 0.44), (0.36, 0.66), (0.83, 0.0), (1.0, 0.14), (0.36, 1.0)])
}

/// A rectangle with a straight leader line from `from` to `tip` (both in frame fractions; the tip
/// is usually outside the frame). The leader is a zero-area sliver so every renderer strokes it.
pub fn line_callout(from: (f32, f32), tip: (f32, f32)) -> Vec<PathCmd> {
    let mut v = poly(&[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]);
    v.extend([PathCmd::Move(from.0, from.1), PathCmd::Line(tip.0, tip.1), PathCmd::Line(from.0, from.1), PathCmd::Close]);
    v
}

/// How a preset is drawn.
pub enum Preset {
    Rect,
    /// Rounded rectangle; the corner radius as a fraction of the shorter side.
    Round(f32),
    Ellipse,
    Line,
    Path(Vec<PathCmd>),
    /// Not supported: drawn as a rectangle.
    Unknown,
}

/// `adj` holds the shape's `avLst` guides by name (value / 100000 already applied by the caller as raw).
pub fn preset(name: &str, w: f32, h: f32, adj: &dyn Fn(&str, f32) -> f32) -> Preset {
    let ss = w.min(h).max(1.0);
    let a = |n: &str, d: f32| adj(n, d) / 100000.0;
    match name {
        "rect" | "flowChartProcess" | "flowChartPredefinedProcess" | "flowChartAlternateProcess" | "frame" | "bevel" | "foldedCorner" | "snipRoundRect" | "plaque" | "flowChartManualOperation" | "flowChartDocument" => Preset::Rect,
        // Rectangles with some corners rounded or snipped; the cut is a fraction of the shorter side.
        "round1Rect" | "snip1Rect" | "round2SameRect" | "snip2SameRect" | "round2DiagRect" | "snip2DiagRect" => {
            let round = name.starts_with("round");
            let cut = |v: f32| { let v = v.clamp(0.0, 0.5); (round, ss * v / w, ss * v / h) };
            let none = (round, 0.0, 0.0);
            let (a1, a2) = match name {
                "round1Rect" | "snip1Rect" => (0.0, a("adj", 16667.0)),
                "snip2DiagRect" => (a("adj1", 0.0), a("adj2", 16667.0)),
                _ => (a("adj1", 16667.0), a("adj2", 0.0)),
            };
            Preset::Path(cut_rect(match name {
                "round1Rect" | "snip1Rect" => [none, cut(a2), none, none],
                "round2SameRect" | "snip2SameRect" => [cut(a1), cut(a1), cut(a2), cut(a2)],
                _ => [cut(a1), cut(a2), cut(a1), cut(a2)],
            }))
        }
        "mathMultiply" => Preset::Path(multiply(a("adj1", 23520.0) * 0.7809)),
        "roundRect" => Preset::Round(a("adj", 16667.0).clamp(0.0, 0.5)),
        "flowChartTerminator" => Preset::Round(0.5),
        "ellipse" | "flowChartConnector" | "donut" | "noSmoking" | "flowChartSummingJunction" | "flowChartOr" => Preset::Ellipse,
        "line" | "straightConnector1" | "bentConnector2" | "bentConnector3" | "curvedConnector3" => Preset::Line,
        "triangle" => Preset::Path(poly(&[(a("adj", 50000.0), 0.0), (1.0, 1.0), (0.0, 1.0)])),
        "rtTriangle" => Preset::Path(poly(&[(0.0, 0.0), (0.0, 1.0), (1.0, 1.0)])),
        "diamond" | "flowChartDecision" => Preset::Path(poly(&[(0.5, 0.0), (1.0, 0.5), (0.5, 1.0), (0.0, 0.5)])),
        "parallelogram" | "flowChartInputOutput" => {
            let o = (ss * a("adj", 25000.0) / w).min(0.9);
            Preset::Path(poly(&[(o, 0.0), (1.0, 0.0), (1.0 - o, 1.0), (0.0, 1.0)]))
        }
        "trapezoid" => {
            let o = (ss * a("adj", 25000.0) / w).min(0.49);
            Preset::Path(poly(&[(o, 0.0), (1.0 - o, 0.0), (1.0, 1.0), (0.0, 1.0)]))
        }
        "hexagon" | "flowChartPreparation" => {
            let o = (ss * a("adj", 25000.0) / w).min(0.49);
            Preset::Path(poly(&[(0.0, 0.5), (o, 0.0), (1.0 - o, 0.0), (1.0, 0.5), (1.0 - o, 1.0), (o, 1.0)]))
        }
        "octagon" => {
            let (ox, oy) = ((ss * a("adj", 29289.0) / w).min(0.49), (ss * a("adj", 29289.0) / h).min(0.49));
            Preset::Path(poly(&[(ox, 0.0), (1.0 - ox, 0.0), (1.0, oy), (1.0, 1.0 - oy), (1.0 - ox, 1.0), (ox, 1.0), (0.0, 1.0 - oy), (0.0, oy)]))
        }
        "pentagon" => Preset::Path(poly(&fit((0..5).map(|i| { let t = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * i as f32 / 5.0; (0.5 + 0.5 * t.cos(), 0.5 + 0.5 * t.sin()) }).collect()))),
        "homePlate" => {
            let i = (ss * a("adj", 50000.0) / w).min(1.0);
            Preset::Path(poly(&[(0.0, 0.0), (1.0 - i, 0.0), (1.0, 0.5), (1.0 - i, 1.0), (0.0, 1.0)]))
        }
        "chevron" => {
            let i = (ss * a("adj", 50000.0) / w).min(0.9);
            Preset::Path(poly(&[(0.0, 0.0), (1.0 - i, 0.0), (1.0, 0.5), (1.0 - i, 1.0), (0.0, 1.0), (i, 0.5)]))
        }
        "rightArrow" | "leftArrow" => {
            let t = a("adj1", 50000.0).clamp(0.0, 1.0);
            let hl = (ss * a("adj2", 50000.0) / w).min(1.0);
            let (y0, y1) = (0.5 - t / 2.0, 0.5 + t / 2.0);
            let right = vec![(0.0, y0), (1.0 - hl, y0), (1.0 - hl, 0.0), (1.0, 0.5), (1.0 - hl, 1.0), (1.0 - hl, y1), (0.0, y1)];
            Preset::Path(poly(&if name == "rightArrow" { right } else { right.into_iter().map(|(x, y)| (1.0 - x, y)).collect::<Vec<_>>() }))
        }
        "upArrow" | "downArrow" => {
            let t = a("adj1", 50000.0).clamp(0.0, 1.0);
            let hl = (ss * a("adj2", 50000.0) / h).min(1.0);
            let (x0, x1) = (0.5 - t / 2.0, 0.5 + t / 2.0);
            let down = vec![(x0, 0.0), (x0, 1.0 - hl), (0.0, 1.0 - hl), (0.5, 1.0), (1.0, 1.0 - hl), (x1, 1.0 - hl), (x1, 0.0)];
            Preset::Path(poly(&if name == "downArrow" { down } else { down.into_iter().map(|(x, y)| (x, 1.0 - y)).collect::<Vec<_>>() }))
        }
        "plus" | "mathPlus" | "cross" => {
            let o = a("adj", 25000.0).clamp(0.0, 0.5);
            let (ox, oy) = (o * ss / w, o * ss / h);
            Preset::Path(poly(&[(ox, 0.0), (1.0 - ox, 0.0), (1.0 - ox, oy), (1.0, oy), (1.0, 1.0 - oy), (1.0 - ox, 1.0 - oy), (1.0 - ox, 1.0), (ox, 1.0), (ox, 1.0 - oy), (0.0, 1.0 - oy), (0.0, oy), (ox, oy)]))
        }
        "star4" => Preset::Path(poly(&star(4, 0.38))),
        "star5" => Preset::Path(poly(&star(5, 0.382))),
        "star6" => Preset::Path(poly(&star(6, 0.58))),
        "star8" => Preset::Path(poly(&star(8, 0.7))),
        "star10" => Preset::Path(poly(&star(10, 0.75))),
        "star12" => Preset::Path(poly(&star(12, 0.8))),
        "leftRightArrow" => {
            let t = a("adj1", 50000.0).clamp(0.0, 1.0);
            let hl = (ss * a("adj2", 50000.0) / w).min(0.5);
            let (y0, y1) = (0.5 - t / 2.0, 0.5 + t / 2.0);
            Preset::Path(poly(&[(0.0, 0.5), (hl, 0.0), (hl, y0), (1.0 - hl, y0), (1.0 - hl, 0.0), (1.0, 0.5), (1.0 - hl, 1.0), (1.0 - hl, y1), (hl, y1), (hl, 1.0)]))
        }
        "lightningBolt" => Preset::Path(poly(&[(0.42, 0.0), (0.12, 0.55), (0.42, 0.55), (0.28, 1.0), (0.88, 0.38), (0.55, 0.38), (0.75, 0.0)])),
        "wedgeRectCallout" | "wedgeRoundRectCallout" => {
            // A box with a pointer at the bottom left.
            Preset::Path(poly(&[(0.0, 0.0), (1.0, 0.0), (1.0, 0.75), (0.45, 0.75), (0.2, 1.0), (0.25, 0.75), (0.0, 0.75)]))
        }
        "heart" => Preset::Path(vec![
            PathCmd::Move(0.5, 1.0),
            PathCmd::Curve(0.1, 0.72, 0.0, 0.45, 0.0, 0.28),
            PathCmd::Curve(0.0, 0.1, 0.14, 0.0, 0.28, 0.0),
            PathCmd::Curve(0.4, 0.0, 0.48, 0.07, 0.5, 0.18),
            PathCmd::Curve(0.52, 0.07, 0.6, 0.0, 0.72, 0.0),
            PathCmd::Curve(0.86, 0.0, 1.0, 0.1, 1.0, 0.28),
            PathCmd::Curve(1.0, 0.45, 0.9, 0.72, 0.5, 1.0),
            PathCmd::Close,
        ]),
        _ => Preset::Unknown,
    }
}


/// The captions of Insert > Caption: (name, has a pointer). Index `i` is the order of the menu, two to a row.
pub const CAPTIONS: [(&str, bool); 8] = [
    ("Rectangular Caption", true), ("Rectangular Caption (No Pointer)", false),
    ("Rounded Rectangular Caption", true), ("Rounded Rectangular Caption (No Pointer)", false),
    ("Oval Caption", true), ("Oval Caption (No Pointer)", false),
    ("Cloud Caption", true), ("Cloud Caption (No Pointer)", false),
];

/// A rectangle whose corners are rounded by `rx` and `ry` (fractions of the width and height), with a pointer hanging
/// from the bottom right when `pointer`. The box takes the top `body` of the frame; the pointer fills the rest.
fn speech_box(rx: f32, ry: f32, body: f32, pointer: bool) -> Vec<PathCmd> {
    let k = 0.5523_f32;
    let mut v = vec![PathCmd::Move(rx, 0.0), PathCmd::Line(1.0 - rx, 0.0)];
    if rx > 0.0 { v.push(PathCmd::Curve(1.0 - rx + k * rx, 0.0, 1.0, ry - k * ry, 1.0, ry)); }
    v.push(PathCmd::Line(1.0, body - ry));
    if rx > 0.0 { v.push(PathCmd::Curve(1.0, body - ry + k * ry, 1.0 - rx + k * rx, body, 1.0 - rx, body)); }
    if pointer { v.extend([PathCmd::Line(0.9, body), PathCmd::Line(0.9, 1.0), PathCmd::Line(0.79, body)]); }
    v.push(PathCmd::Line(rx, body));
    if rx > 0.0 { v.push(PathCmd::Curve(rx - k * rx, body, 0.0, body - ry + k * ry, 0.0, body - ry)); }
    v.push(PathCmd::Line(0.0, ry));
    if rx > 0.0 { v.push(PathCmd::Curve(0.0, ry - k * ry, rx - k * rx, 0.0, rx, 0.0)); }
    v.push(PathCmd::Close);
    v
}

/// An oval speech bubble: an ellipse with a pointer reaching the bottom right corner of the frame.
fn speech_oval() -> Vec<PathCmd> {
    let (cx, cy, rx, ry) = (0.5_f32, 0.45_f32, 0.5_f32, 0.45_f32);
    let at = |deg: f32| { let a = deg.to_radians(); (cx + rx * a.cos(), cy + ry * a.sin()) };
    let mut pts = vec![];
    // From the bottom, round the left, top and right, to the lower right where the pointer starts.
    let (from, to) = (68.0_f32, 360.0 + 24.0);
    for i in 0..=64 { pts.push(at(from + (to - from) * i as f32 / 64.0)); }
    pts.push((1.0, 1.0));
    poly(&pts)
}

/// A cloud: a ring of round bumps whose outline is traced from the outside, so it can be stroked as one line.
fn cloud_outline() -> Vec<(f32, f32)> {
    let n = 9;
    let (cx, cy, rx, ry, r) = (0.5_f32, 0.42_f32, 0.34_f32, 0.25_f32, 0.17_f32);
    let centres: Vec<(f32, f32)> = (0..n).map(|i| { let a = i as f32 / n as f32 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2; (cx + rx * a.cos(), cy + ry * a.sin()) }).collect();
    // The bumps are circles of one radius; draw each one's part that no neighbour covers.
    let inside_other = |i: usize, p: (f32, f32)| centres.iter().enumerate().any(|(j, c)| j != i && ((p.0 - c.0).powi(2) + (p.1 - c.1).powi(2)).sqrt() < r * 0.999);
    let mut out = vec![];
    for i in 0..n {
        let c = centres[i];
        let samples: Vec<(f32, f32)> = (0..90).map(|k| { let a = k as f32 / 90.0 * std::f32::consts::TAU; (c.0 + r * a.cos(), c.1 + r * a.sin()) }).collect();
        // Only the half facing away from the middle: the other half of a bump is inside the cloud.
        let out_dir = { let (dx, dy) = (c.0 - cx, c.1 - cy); let l = (dx * dx + dy * dy).sqrt().max(1e-6); (dx / l, dy / l) };
        let kept: Vec<bool> = samples.iter().map(|&p| !inside_other(i, p) && (p.0 - c.0) * out_dir.0 + (p.1 - c.1) * out_dir.1 > -0.35 * r).collect();
        // Start where the visible part begins, so the run is contiguous even across the wrap.
        let Some(start) = (0..90).find(|&k| kept[k] && !kept[(k + 89) % 90]) else { continue };
        for k in 0..90 { let idx = (start + k) % 90; if kept[idx] { out.push(samples[idx]); } }
    }
    out
}

/// A cloud caption; the thought bubble adds two small circles trailing off below it.
fn speech_cloud(trail: bool) -> Vec<PathCmd> {
    let (lo, hi) = cloud_outline().iter().fold(((f32::MAX, f32::MAX), (f32::MIN, f32::MIN)), |(lo, hi), p| ((lo.0.min(p.0), lo.1.min(p.1)), (hi.0.max(p.0), hi.1.max(p.1))));
    // Stretch the cloud to fill the top of the frame (all of it without a trail).
    let body = if trail { 0.8 } else { 1.0 };
    let map = |p: (f32, f32)| ((p.0 - lo.0) / (hi.0 - lo.0), (p.1 - lo.1) / (hi.1 - lo.1) * body);
    let mut v = poly(&cloud_outline().into_iter().map(map).collect::<Vec<_>>());
    if trail {
        for (cx, cy, r) in [(0.36, 0.89, 0.065_f32), (0.30, 0.975, 0.03_f32)] {
            let k = 0.5523_f32;
            v.extend([
                PathCmd::Move(cx + r, cy),
                PathCmd::Curve(cx + r, cy + k * r, cx + k * r, cy + r, cx, cy + r),
                PathCmd::Curve(cx - k * r, cy + r, cx - r, cy + k * r, cx - r, cy),
                PathCmd::Curve(cx - r, cy - k * r, cx - k * r, cy - r, cx, cy - r),
                PathCmd::Curve(cx + k * r, cy - r, cx + r, cy - k * r, cx + r, cy),
                PathCmd::Close,
            ]);
        }
    }
    v
}

/// How caption `i` of `CAPTIONS` is drawn in a frame `w` by `h`, and the share of the frame's height at the bottom that
/// the pointer takes (the text is kept out of it).
pub fn caption(i: usize, w: f32, h: f32) -> (Preset, f32) {
    let r = 0.06 * w.min(h);
    match i {
        0 => (Preset::Path(speech_box(0.0, 0.0, 0.74, true)), 0.26),
        1 => (Preset::Rect, 0.0),
        2 => (Preset::Path(speech_box(r / w, r / h, 0.74, true)), 0.26),
        3 => (Preset::Round(0.06), 0.0),
        4 => (Preset::Path(speech_oval()), 0.1),
        5 => (Preset::Ellipse, 0.0),
        6 => (Preset::Path(speech_cloud(true)), 0.2),
        _ => (Preset::Path(speech_cloud(false)), 0.0),
    }
}

/// The shapes offered in the Insert gallery: (preset name, label).
pub const GALLERY: [(&str, &str); 28] = [
    ("rect", "Rectangle"), ("roundRect", "Rounded rectangle"), ("ellipse", "Ellipse"), ("line", "Line"),
    ("triangle", "Triangle"), ("rtTriangle", "Right triangle"), ("diamond", "Diamond"), ("parallelogram", "Parallelogram"),
    ("trapezoid", "Trapezoid"), ("pentagon", "Pentagon"), ("hexagon", "Hexagon"), ("octagon", "Octagon"),
    ("rightArrow", "Right arrow"), ("leftArrow", "Left arrow"), ("upArrow", "Up arrow"), ("downArrow", "Down arrow"),
    ("leftRightArrow", "Double arrow"), ("chevron", "Chevron"), ("homePlate", "Pentagon arrow"), ("plus", "Plus"),
    ("star4", "4-point star"), ("star5", "5-point star"), ("star6", "6-point star"), ("star8", "8-point star"),
    ("wedgeRectCallout", "Callout"), ("heart", "Heart"), ("lightningBolt", "Lightning bolt"), ("flowChartTerminator", "Pill"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_gallery_shape_is_drawable_and_stays_in_the_unit_square() {
        for (name, _) in GALLERY {
            match preset(name, 300.0, 300.0, &|_, d| d) {
                Preset::Unknown => panic!("{name} has no drawing"),
                Preset::Path(cmds) => {
                    assert!(cmds.len() >= 3, "{name}");
                    for c in cmds {
                        let pts: Vec<f32> = match c { PathCmd::Move(x, y) | PathCmd::Line(x, y) => vec![x, y], PathCmd::Curve(a, b, c2, d, e, f) => vec![a, b, c2, d, e, f], PathCmd::Close => vec![] };
                        assert!(pts.iter().all(|v| (-0.001..=1.001).contains(v)), "{name}: {pts:?}");
                    }
                }
                _ => {}
            }
        }
    }

    fn points(cmds: &[PathCmd]) -> Vec<(f32, f32)> {
        cmds.iter().filter_map(|c| match *c { PathCmd::Move(x, y) | PathCmd::Line(x, y) => Some((x, y)), PathCmd::Curve(.., x, y) => Some((x, y)), PathCmd::Close => None }).collect()
    }

    #[test]
    fn snipped_and_rounded_corners_stay_inside_the_frame() {
        for round in [false, true] {
            let p = cut_rect([(round, 0.2, 0.1), (round, 0.0, 0.0), (round, 0.2, 0.1), (round, 0.0, 0.0)]);
            assert!(points(&p).iter().all(|&(x, y)| (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y)), "{p:?}");
            assert_eq!(p.last(), Some(&PathCmd::Close));
        }
        // A snipped corner replaces the corner point with two points on the edges.
        let snip = points(&cut_rect([(false, 0.2, 0.1), (false, 0.0, 0.0), (false, 0.0, 0.0), (false, 0.0, 0.0)]));
        assert!(snip.contains(&(0.2, 0.0)) && snip.contains(&(0.0, 0.1)) && !snip.iter().any(|&p| p == (0.0, 0.0) && false));
    }

    #[test]
    fn corner_presets_are_real_shapes_not_rectangles() {
        let flat = |_: &str, d: f32| d;
        for name in ["round2SameRect", "snip2DiagRect", "snip1Rect", "round1Rect", "mathMultiply"] {
            assert!(matches!(preset(name, 200.0, 100.0, &flat), Preset::Path(_)), "{name}");
        }
    }

    #[test]
    fn captions_fit_their_frame_and_pointers_reach_the_bottom() {
        for i in 0..CAPTIONS.len() {
            let (preset, clear) = caption(i, 520.0, 300.0);
            match preset {
                Preset::Path(cmds) => {
                    let mut max_y: f32 = 0.0;
                    for c in &cmds {
                        let pts: Vec<f32> = match *c { PathCmd::Move(x, y) | PathCmd::Line(x, y) => vec![x, y], PathCmd::Curve(a, b, c2, d, e, f) => vec![a, b, c2, d, e, f], PathCmd::Close => vec![] };
                        assert!(pts.iter().all(|v| (-0.01..=1.01).contains(v)), "{}: {pts:?} is inside the frame", CAPTIONS[i].0);
                        for y in pts.iter().skip(1).step_by(2) { max_y = max_y.max(*y); }
                    }
                    assert!(max_y > 0.97, "{} reaches the bottom of the frame ({max_y})", CAPTIONS[i].0);
                    assert_eq!(clear > 0.0, CAPTIONS[i].1, "text keeps clear of a pointer, and only of one");
                }
                _ => assert!(!CAPTIONS[i].1, "a bubble without a pointer is a plain rectangle, rounded rectangle or oval"),
            }
        }
        // The cloud outline is one closed loop traced from the outside (not a ring of separate circles).
        let Preset::Path(cloud) = caption(7, 520.0, 300.0).0 else { panic!("a path") };
        assert_eq!(cloud.iter().filter(|c| matches!(c, PathCmd::Move(..))).count(), 1);
    }
}

/// The solid shape of a path made of several outlines: just the outlines that are not inside another one. An icon is
/// the band its lines make, with an outline round its outside and another round its inside; this is the outside alone,
/// so what the lines enclose can be filled behind them.
pub fn silhouette(cmds: &[PathCmd]) -> Vec<PathCmd> {
    let mut figs: Vec<Vec<PathCmd>> = vec![];
    for c in cmds {
        match c {
            PathCmd::Move(..) => figs.push(vec![*c]),
            other => { if let Some(f) = figs.last_mut() { f.push(*other); } }
        }
    }
    let polys: Vec<Vec<(f32, f32)>> = figs.iter().map(|f| flatten(f)).collect();
    let inside = |p: (f32, f32), poly: &[(f32, f32)]| {
        let mut c = false;
        let mut j = poly.len().wrapping_sub(1);
        for i in 0..poly.len() {
            let (a, b) = (poly[i], poly[j]);
            if (a.1 > p.1) != (b.1 > p.1) && p.0 < (b.0 - a.0) * (p.1 - a.1) / (b.1 - a.1) + a.0 { c = !c; }
            j = i;
        }
        c
    };
    let mut out = vec![];
    for (i, f) in figs.iter().enumerate() {
        let Some(&first) = polys[i].first() else { continue };
        if !polys.iter().enumerate().any(|(j, other)| j != i && inside(first, other)) { out.extend(f.iter().copied()); }
    }
    out
}

/// A figure as points, curves sampled.
fn flatten(fig: &[PathCmd]) -> Vec<(f32, f32)> {
    let mut pts = vec![];
    let mut cur = (0.0f32, 0.0f32);
    for c in fig {
        match *c {
            PathCmd::Move(x, y) | PathCmd::Line(x, y) => { pts.push((x, y)); cur = (x, y); }
            PathCmd::Curve(a, b, c2, d, x, y) => {
                for k in 1..=8 {
                    let (t, u) = (k as f32 / 8.0, 1.0 - k as f32 / 8.0);
                    pts.push((cur.0 * u * u * u + 3.0 * a * u * u * t + 3.0 * c2 * u * t * t + x * t * t * t, cur.1 * u * u * u + 3.0 * b * u * u * t + 3.0 * d * u * t * t + y * t * t * t));
                }
                cur = (x, y);
            }
            PathCmd::Close => {}
        }
    }
    pts
}

#[cfg(test)]
mod silhouette_tests {
    use super::*;

    #[test]
    fn the_silhouette_is_the_outside_outlines_only() {
        // A square ring: an outer square and, inside it, a reversed inner square (the hole).
        let ring: Vec<PathCmd> = poly(&[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]).into_iter().chain(poly(&[(0.25, 0.25), (0.25, 0.75), (0.75, 0.75), (0.75, 0.25)])).collect();
        let s = silhouette(&ring);
        assert_eq!(s, poly(&[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]), "the hole is covered");
        // Two separate shapes both stay, and an island inside a hole does not (the outside already covers it).
        let two: Vec<PathCmd> = ring.iter().copied().chain(poly(&[(2.0, 0.0), (3.0, 0.0), (3.0, 1.0), (2.0, 1.0)])).chain(poly(&[(0.4, 0.4), (0.6, 0.4), (0.6, 0.6), (0.4, 0.6)])).collect();
        let s = silhouette(&two);
        assert_eq!(s.iter().filter(|c| matches!(c, PathCmd::Move(..))).count(), 2, "{s:?}");
        assert!(silhouette(&[]).is_empty());
    }
}
