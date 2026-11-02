//! Shape geometry: custom paths (presets live in `brainbend_doc::shapes`).
use crate::color::*;
use roxmltree::Node;
use brainbend_doc::PathCmd;
pub use brainbend_doc::shapes::{preset, Preset};

fn pt(n: Node) -> Option<(f32, f32)> {
    Some((attr_f(n, "x")?, attr_f(n, "y")?))
}

/// Cubic segments approximating an elliptical arc from the current point `p`.
fn arc(p: (f32, f32), wr: f32, hr: f32, st: f32, sw: f32, out: &mut Vec<PathCmd>, norm: &dyn Fn((f32, f32)) -> (f32, f32)) -> (f32, f32) {
    let (st, sw) = (st.to_radians(), sw.to_radians());
    let c = (p.0 - wr * st.cos(), p.1 - hr * st.sin());
    let n = ((sw.abs() / std::f32::consts::FRAC_PI_2).ceil() as usize).max(1);
    let step = sw / n as f32;
    let k = 4.0 / 3.0 * (step / 4.0).tan();
    let at = |t: f32| (c.0 + wr * t.cos(), c.1 + hr * t.sin());
    let d = |t: f32| (-wr * t.sin(), hr * t.cos());
    let mut cur = st;
    let mut last = p;
    for _ in 0..n {
        let next = cur + step;
        let (p0, p3) = (at(cur), at(next));
        let (d0, d3) = (d(cur), d(next));
        let c1 = norm((p0.0 + k * d0.0, p0.1 + k * d0.1));
        let c2 = norm((p3.0 - k * d3.0, p3.1 - k * d3.1));
        let e = norm(p3);
        out.push(PathCmd::Curve(c1.0, c1.1, c2.0, c2.1, e.0, e.1));
        last = p3;
        cur = next;
    }
    last
}

/// Parse `a:custGeom` into unit-square path commands (all `a:path` elements, in order).
pub fn custom_path(geom: Node) -> Vec<PathCmd> {
    let mut out = vec![];
    let Some(list) = child(geom, "pathLst") else { return out };
    for path in children(list, "path") {
        if path.attribute("fill") == Some("none") && path.attribute("stroke") != Some("0") && children(list, "path").count() > 1 { /* outline-only helper path */ }
        let (pw, ph) = (attr_f(path, "w").unwrap_or(0.0), attr_f(path, "h").unwrap_or(0.0));
        // Without a path size the coordinates are already in shape units; fall back to their extent.
        let (mut mx, mut my) = (pw, ph);
        if mx <= 0.0 || my <= 0.0 {
            for p in path.descendants().filter(|n| n.tag_name().name() == "pt").filter_map(pt) { mx = mx.max(p.0); my = my.max(p.1); }
        }
        let (mx, my) = (mx.max(1.0), my.max(1.0));
        let norm = |p: (f32, f32)| (p.0 / mx, p.1 / my);
        let mut cur = (0.0f32, 0.0f32);
        for c in path.children().filter(|c| c.is_element()) {
            let pts: Vec<(f32, f32)> = c.children().filter(|n| n.is_element() && n.tag_name().name() == "pt").filter_map(pt).collect();
            match c.tag_name().name() {
                "moveTo" => if let Some(&p) = pts.first() { cur = p; let n = norm(p); out.push(PathCmd::Move(n.0, n.1)); },
                "lnTo" => if let Some(&p) = pts.first() { cur = p; let n = norm(p); out.push(PathCmd::Line(n.0, n.1)); },
                "cubicBezTo" if pts.len() >= 3 => {
                    let (a, b, e) = (norm(pts[0]), norm(pts[1]), norm(pts[2]));
                    out.push(PathCmd::Curve(a.0, a.1, b.0, b.1, e.0, e.1));
                    cur = pts[2];
                }
                "quadBezTo" if pts.len() >= 2 => {
                    // Raise the quadratic to a cubic.
                    let (p0, q, e) = (cur, pts[0], pts[1]);
                    let c1 = norm((p0.0 + 2.0 / 3.0 * (q.0 - p0.0), p0.1 + 2.0 / 3.0 * (q.1 - p0.1)));
                    let c2 = norm((e.0 + 2.0 / 3.0 * (q.0 - e.0), e.1 + 2.0 / 3.0 * (q.1 - e.1)));
                    let en = norm(e);
                    out.push(PathCmd::Curve(c1.0, c1.1, c2.0, c2.1, en.0, en.1));
                    cur = e;
                }
                "arcTo" => {
                    let (wr, hr) = (attr_f(c, "wR").unwrap_or(0.0), attr_f(c, "hR").unwrap_or(0.0));
                    let (st, sw) = (attr_f(c, "stAng").unwrap_or(0.0) / 60000.0, attr_f(c, "swAng").unwrap_or(0.0) / 60000.0);
                    if wr > 0.0 && hr > 0.0 { cur = arc(cur, wr, hr, st, sw, &mut out, &norm); }
                }
                "close" => out.push(PathCmd::Close),
                _ => {}
            }
        }
    }
    out
}
