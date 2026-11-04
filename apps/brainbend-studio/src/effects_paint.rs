//! Shadow, glow, reflection and soft edges as the editor canvas draws them.
//!
//! Every effect follows the object's real outline (rounded corners, ellipses and freeform shapes such as stars), and a
//! blur is faked with a stack of slightly different outlines, each drawn faintly, so that together they fade out like the
//! browser's blur does in the published player.
use brainbend_doc::{Kind, Object};
use eframe::egui::{self, Color32, Pos2, Rect};

type Figures = Vec<Vec<Pos2>>;
type Map<'a> = &'a dyn Fn(Pos2) -> Pos2;

/// The object's outline as polygons in screen space (unrotated).
pub(crate) fn figures(o: &Object, r: Rect, scale: f32) -> Figures {
    match &o.kind {
        Kind::Path(cmds) => super::path_polygons(cmds, r),
        Kind::Ellipse => vec![super::ellipse_poly(r)],
        Kind::Image => vec![super::rounded_rect_poly(r, 0.0)],
        _ => vec![super::rounded_rect_poly(r, o.radius * scale)],
    }
}

/// An outline pushed outwards by `d` pixels (inwards when negative), with sharp corners kept in check.
fn offset_polygon(poly: &[Pos2], d: f32) -> Vec<Pos2> {
    let mut pts: Vec<Pos2> = Vec::with_capacity(poly.len());
    for p in poly { if pts.last().is_none_or(|l| (*l - *p).length() > 0.01) { pts.push(*p); } }
    if pts.len() > 1 && (pts[0] - pts[pts.len() - 1]).length() <= 0.01 { pts.pop(); }
    let n = pts.len();
    if n < 3 || d.abs() < 0.01 { return pts; }
    // Screen space has y pointing down, so a positive signed area is clockwise on screen.
    let area: f32 = (0..n).map(|i| { let (a, b) = (pts[i], pts[(i + 1) % n]); a.x * b.y - b.x * a.y }).sum::<f32>();
    let sign = if area >= 0.0 { 1.0 } else { -1.0 };
    let normal = |e: egui::Vec2| egui::vec2(e.y, -e.x) * sign;
    (0..n).map(|i| {
        let (prev, cur, next) = (pts[(i + n - 1) % n], pts[i], pts[(i + 1) % n]);
        let n1 = normal((cur - prev).normalized());
        let n2 = normal((next - cur).normalized());
        let m = n1 + n2;
        let len = m.length();
        if len < 1e-3 { return cur + n1 * d; }
        // The miter: further out at a corner, but never more than three times the distance.
        let k = (2.0 / len).min(3.0);
        cur + m / len * d * k
    }).collect()
}

fn smooth(t: f32) -> f32 { let t = t.clamp(0.0, 1.0); t * t * (3.0 - 2.0 * t) }

/// Draw the outline offset by each distance in `ds` (outermost first), every layer faint enough that the stack reaches
/// `profile(d)` of opacity at distance `d`. `shift` moves everything (a shadow's offset) after `map` has turned it.
fn layers(painter: &egui::Painter, figs: &Figures, bounds: Rect, ds: &[f32], profile: &dyn Fn(f32) -> f32, color: Color32, map: Map, shift: egui::Vec2) {
    let floor = -0.45 * bounds.width().min(bounds.height());   // an outline pushed in further than this has collapsed
    let place = |p: Pos2| map(p) + shift;
    let base = Color32::from_rgb(color.r(), color.g(), color.b());
    let mut cum_prev = 0.0f32;
    for &d in ds {
        let cum = profile(d).clamp(0.0, 1.0);
        if cum <= cum_prev + 0.002 || cum_prev >= 0.999 { continue; }
        let a = 1.0 - (1.0 - cum) / (1.0 - cum_prev);
        cum_prev = cum;
        let moved: Figures = figs.iter().map(|f| offset_polygon(f, d.max(floor))).collect();
        super::fill_figures(painter, &moved, &place, &|_| base.gamma_multiply(a));
    }
}

/// A blurred copy of the outline: solid out to `spread`, fading over `blur` either side of that (as a browser's
/// box-shadow does), `shift` away from the object.
fn halo(painter: &egui::Painter, figs: &Figures, bounds: Rect, spread: f32, blur: f32, color: Color32, map: Map, shift: egui::Vec2) {
    let alpha = color.a() as f32 / 255.0;
    let (lo, hi) = (spread - blur, spread + blur);
    if hi - lo < 0.5 {
        layers(painter, figs, bounds, &[spread], &|_| alpha, color, map, shift);
        return;
    }
    let n = (((hi - lo) / 2.0).ceil() as usize).clamp(3, 14);
    let ds: Vec<f32> = (0..=n).map(|k| hi - (hi - lo) * k as f32 / n as f32).collect();
    layers(painter, figs, bounds, &ds, &|d| alpha * (1.0 - smooth((d - lo) / (hi - lo))), color, map, shift);
}

/// The faded mirror image under the object: `size` of its height, `opacity` where it meets the object, `gap` below it.
fn reflection(painter: &egui::Painter, figs: &Figures, bounds: Rect, color: Color32, size: f32, opacity: f32, gap: f32, map: Map) {
    let h = bounds.height() * size.clamp(0.0, 1.0);
    if h <= 0.5 { return; }
    let (bottom, top) = (bounds.bottom(), bounds.bottom() + gap);
    let limit = top + h;
    let mirrored: Figures = figs.iter().map(|f| {
        let flipped: Vec<Pos2> = f.iter().map(|p| egui::pos2(p.x, top + (bottom - p.y))).collect();
        clip_above(&flipped, limit)
    }).filter(|f| f.len() >= 3).collect();
    super::fill_figures(painter, &mirrored, map, &|p| color.gamma_multiply(opacity.clamp(0.0, 1.0) * (1.0 - (p.y - top) / h).clamp(0.0, 1.0)));
}

/// The part of a polygon with y at or above `limit`.
fn clip_above(poly: &[Pos2], limit: f32) -> Vec<Pos2> {
    let mut out = Vec::with_capacity(poly.len() + 2);
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        let (ain, bin) = (a.y <= limit, b.y <= limit);
        if ain { out.push(a); }
        if ain != bin {
            let t = (limit - a.y) / (b.y - a.y);
            out.push(egui::pos2(a.x + (b.x - a.x) * t, limit));
        }
    }
    out
}

/// Everything that goes behind the object: its shadow, its glow and its reflection. A softened object has no shadow or
/// glow (the fade would cut them off in the published player).
pub(crate) fn behind(painter: &egui::Painter, o: &Object, r: Rect, scale: f32, fill: Option<Color32>, map: Map) {
    if !(r.width().is_finite() && r.height().is_finite() && r.width() < 1.0e5 && r.height() < 1.0e5) { return; }
    let figs = figures(o, r, scale);
    let soft = o.soft_edge > 0.0;
    if let (Some(g), false) = (o.glow, soft) {
        // The published player draws a glow as a shadow with no offset, a quarter of its size solid and the rest blurred.
        halo(painter, &figs, r, g.radius.clamp(0.0, 500.0) / 4.0 * scale, g.radius.clamp(0.0, 500.0) * scale, super::color(g.color), map, egui::Vec2::ZERO);
    }
    if let (Some(sh), false) = (o.shadow, soft) {
        halo(painter, &figs, r, sh.spread.clamp(-500.0, 500.0) * scale, sh.blur.clamp(0.0, 500.0) * scale, super::color(sh.color), map, egui::vec2(sh.dx, sh.dy) * scale);
    }
    if let (Some(rf), Some(f)) = (o.reflection, fill) {
        reflection(painter, &figs, r, f, rf.size, rf.opacity, rf.gap * scale, map);
    }
}

/// Fill the outline with `color`, fading to nothing over `edge` pixels at its border.
pub(crate) fn soft_fill(painter: &egui::Painter, figs: &Figures, bounds: Rect, color: Color32, edge: f32, map: Map) {
    let e = edge.min(0.45 * bounds.width().min(bounds.height())).clamp(1.0, 500.0);
    let n = ((e / 1.5).ceil() as usize).clamp(3, 16);
    let alpha = color.a() as f32 / 255.0;
    let ds: Vec<f32> = (0..=n).map(|k| -e * k as f32 / n as f32).collect();
    layers(painter, figs, bounds, &ds, &|d| alpha * smooth(-d / e), color, map, egui::Vec2::ZERO);
}
