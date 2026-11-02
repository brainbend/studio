//! The hotspot outline editor's geometry: moving, adding and removing the corners of a hotspot's
//! outline, fitting its frame to the outline, and building a hotspot from points drawn on the slide.
//!
//! A hotspot's outline (`Object::hit_area`) is a polygon given as fractions of its frame, so it
//! follows the frame when the hotspot is moved or resized. Everything here takes points in slide
//! units and turns them into those fractions, allowing for a hotspot that has been turned about the
//! centre of its frame.
use brainbend_doc::Object;

pub type Pt = (f32, f32);

/// Fewest corners an outline can have.
pub const MIN_POINTS: usize = 3;
/// Smallest frame a drawn outline may have on either side, in slide units.
const MIN_SIDE: f32 = 8.0;

pub fn box_outline() -> Vec<Pt> {
    vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]
}

/// An ellipse that fills the frame, as the importer makes for an oval hotspot.
pub fn ellipse_outline() -> Vec<Pt> {
    (0..48).map(|i| {
        let a = i as f32 / 48.0 * std::f32::consts::TAU;
        (0.5 + 0.5 * a.cos(), 0.5 + 0.5 * a.sin())
    }).collect()
}

/// Whether the outline is just the frame's four corners, which needs no outline at all.
pub fn is_box(area: &[Pt]) -> bool {
    area.len() == 4 && box_outline().iter().all(|c| area.iter().any(|p| (p.0 - c.0).abs() < 0.002 && (p.1 - c.1).abs() < 0.002))
}

/// A point in slide units, as a fraction of the object's frame.
pub fn to_local(o: &Object, p: Pt) -> Pt {
    let (cx, cy) = (o.x + o.w / 2.0, o.y + o.h / 2.0);
    let a = -o.rotation.to_radians();
    let (dx, dy) = (p.0 - cx, p.1 - cy);
    let (rx, ry) = (cx + dx * a.cos() - dy * a.sin(), cy + dx * a.sin() + dy * a.cos());
    ((rx - o.x) / o.w.max(1.0), (ry - o.y) / o.h.max(1.0))
}

/// A fraction of the object's frame, in slide units.
pub fn to_slide(o: &Object, f: Pt) -> Pt {
    let (cx, cy) = (o.x + o.w / 2.0, o.y + o.h / 2.0);
    let a = o.rotation.to_radians();
    let (x, y) = (o.x + f.0 * o.w, o.y + f.1 * o.h);
    let (dx, dy) = (x - cx, y - cy);
    (cx + dx * a.cos() - dy * a.sin(), cy + dx * a.sin() + dy * a.cos())
}

/// Whether `p` (a fraction of the frame) is inside the polygon.
pub fn contains(area: &[Pt], p: Pt) -> bool {
    let mut inside = false;
    let mut j = area.len().wrapping_sub(1);
    for i in 0..area.len() {
        let (a, b) = (area[i], area[j]);
        if (a.1 > p.1) != (b.1 > p.1) && p.0 < (b.0 - a.0) * (p.1 - a.1) / (b.1 - a.1) + a.0 {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn dist(a: Pt, b: Pt) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

/// The corner within `tol` slide units of `p` (slide units), the nearest if several.
pub fn nearest_vertex(o: &Object, area: &[Pt], p: Pt, tol: f32) -> Option<usize> {
    area.iter().enumerate()
        .map(|(i, &f)| (i, dist(to_slide(o, f), p)))
        .filter(|&(_, d)| d <= tol)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

/// The edge within `tol` slide units of `p`: the corner it starts from (it ends at the next one) and the
/// closest point on it, as a fraction of the frame.
pub fn nearest_edge(o: &Object, area: &[Pt], p: Pt, tol: f32) -> Option<(usize, Pt)> {
    let mut best: Option<(usize, Pt, f32)> = None;
    for i in 0..area.len() {
        let (a, b) = (to_slide(o, area[i]), to_slide(o, area[(i + 1) % area.len()]));
        let (vx, vy) = (b.0 - a.0, b.1 - a.1);
        let len2 = vx * vx + vy * vy;
        if len2 < 1e-6 { continue; }
        let t = (((p.0 - a.0) * vx + (p.1 - a.1) * vy) / len2).clamp(0.0, 1.0);
        let q = (a.0 + vx * t, a.1 + vy * t);
        let d = dist(p, q);
        if d <= tol && best.is_none_or(|b| d < b.2) { best = Some((i, to_local(o, q), d)); }
    }
    best.map(|(i, q, _)| (i, q))
}

/// Add a corner after corner `after`, and return its index.
pub fn insert(area: &mut Vec<Pt>, after: usize, at: Pt) -> usize {
    area.insert(after + 1, at);
    after + 1
}

/// Remove corner `i` unless that would leave fewer than three.
pub fn remove(area: &mut Vec<Pt>, i: usize) -> bool {
    if area.len() <= MIN_POINTS || i >= area.len() { return false; }
    area.remove(i);
    true
}

/// Make sure the hotspot has an outline to edit (the frame's four corners if it had none).
pub fn begin(o: &mut Object) {
    if o.hit_area.as_ref().is_none_or(|a| a.len() < MIN_POINTS) { o.hit_area = Some(box_outline()); }
}

/// Leave an outline that is just the frame's corners as no outline.
pub fn finish(o: &mut Object) {
    if o.hit_area.as_ref().is_some_and(|a| is_box(a)) { o.hit_area = None; }
}

/// Put corner `i` at `p` (slide units), kept inside the frame.
pub fn set_point(o: &mut Object, i: usize, p: Pt) {
    let f = to_local(o, p);
    if let Some(pt) = o.hit_area.as_mut().and_then(|a| a.get_mut(i)) { *pt = (f.0.clamp(0.0, 1.0), f.1.clamp(0.0, 1.0)); }
}

/// Move corner `i` by `d` slide units.
pub fn nudge_point(o: &mut Object, i: usize, d: Pt) {
    let Some(f) = o.hit_area.as_ref().and_then(|a| a.get(i)).copied() else { return };
    let s = to_slide(o, f);
    set_point(o, i, (s.0 + d.0, s.1 + d.1));
}

/// Shrink the frame to the outline's bounding box, so the outline fills it and the frame's handles and
/// outline line up. The hotspot does not move on screen.
pub fn fit_frame(o: &mut Object) {
    let Some(area) = o.hit_area.clone().filter(|a| a.len() >= MIN_POINTS) else { return };
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for p in &area { x0 = x0.min(p.0); y0 = y0.min(p.1); x1 = x1.max(p.0); y1 = y1.max(p.1); }
    let (fw, fh) = (x1 - x0, y1 - y0);
    if fw * o.w < MIN_SIDE || fh * o.h < MIN_SIDE { return; }
    // The new frame's centre, in slide units, from where its middle sits in the old frame.
    let middle = to_slide(o, ((x0 + x1) / 2.0, (y0 + y1) / 2.0));
    let (w, h) = (fw * o.w, fh * o.h);
    let out: Vec<Pt> = area.iter().map(|p| ((p.0 - x0) / fw, (p.1 - y0) / fh)).collect();
    (o.x, o.y, o.w, o.h) = (middle.0 - w / 2.0, middle.1 - h / 2.0, w, h);
    o.hit_area = Some(out);
}

/// A frame and an outline from corners drawn on the slide (slide units); `None` for fewer than three
/// corners or a shape too thin to click.
pub fn from_points(pts: &[Pt]) -> Option<(f32, f32, f32, f32, Vec<Pt>)> {
    if pts.len() < MIN_POINTS { return None; }
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for p in pts { x0 = x0.min(p.0); y0 = y0.min(p.1); x1 = x1.max(p.0); y1 = y1.max(p.1); }
    let (w, h) = (x1 - x0, y1 - y0);
    if w < MIN_SIDE || h < MIN_SIDE { return None; }
    Some((x0, y0, w, h, pts.iter().map(|p| ((p.0 - x0) / w, (p.1 - y0) / h)).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use brainbend_doc::Kind;

    fn spot() -> Object {
        Object::new("h", "Hotspot", Kind::Hotspot, 100.0, 200.0, 400.0, 200.0)
    }

    #[test]
    fn points_convert_between_the_slide_and_the_frame_even_when_turned() {
        let mut o = spot();
        assert_eq!(to_local(&o, (300.0, 300.0)), (0.5, 0.5));
        assert_eq!(to_slide(&o, (1.0, 0.0)), (500.0, 200.0));
        o.rotation = 90.0;
        // Turned a quarter about its centre (300, 300), the frame's right-middle point lands below the centre.
        let p = to_slide(&o, (1.0, 0.5));
        assert!((p.0 - 300.0).abs() < 0.01 && (p.1 - 500.0).abs() < 0.01, "{p:?}");
        let back = to_local(&o, p);
        assert!((back.0 - 1.0).abs() < 1e-4 && (back.1 - 0.5).abs() < 1e-4, "{back:?}");
    }

    #[test]
    fn a_frame_with_no_outline_starts_from_its_corners_and_goes_back_when_they_are_unchanged() {
        let mut o = spot();
        begin(&mut o);
        assert_eq!(o.hit_area.as_ref().unwrap().len(), 4);
        finish(&mut o);
        assert_eq!(o.hit_area, None, "an outline that is just the box is no outline");
        begin(&mut o);
        set_point(&mut o, 2, (300.0, 400.0));
        finish(&mut o);
        assert!(o.hit_area.is_some(), "a changed outline is kept");
        assert!(is_box(&[(1.0, 1.0), (0.0, 1.0), (0.0, 0.0), (1.0, 0.0)]), "in any order");
        assert!(!is_box(&ellipse_outline()));
    }

    #[test]
    fn corners_are_found_moved_added_and_removed() {
        let mut o = spot();
        begin(&mut o);
        let area = o.hit_area.clone().unwrap();
        assert_eq!(nearest_vertex(&o, &area, (105.0, 203.0), 12.0), Some(0));
        assert_eq!(nearest_vertex(&o, &area, (300.0, 300.0), 12.0), None);
        // An edge: the middle of the top edge is 4 units away.
        let (i, at) = nearest_edge(&o, &area, (300.0, 204.0), 8.0).unwrap();
        assert_eq!(i, 0);
        assert!((at.0 - 0.5).abs() < 1e-4 && at.1.abs() < 1e-4, "{at:?}");
        assert!(nearest_edge(&o, &area, (300.0, 300.0), 8.0).is_none());
        let mut area = area;
        let k = insert(&mut area, i, at);
        assert_eq!((k, area.len()), (1, 5));
        o.hit_area = Some(area);
        set_point(&mut o, k, (300.0, 150.0));
        assert_eq!(o.hit_area.as_ref().unwrap()[k], (0.5, 0.0), "kept inside the frame");
        nudge_point(&mut o, k, (0.0, 30.0));
        assert!((o.hit_area.as_ref().unwrap()[k].1 - 0.15).abs() < 1e-4);
        let mut a = o.hit_area.clone().unwrap();
        assert!(remove(&mut a, k) && a.len() == 4);
        assert!(remove(&mut a, 0) && a.len() == 3);
        assert!(!remove(&mut a, 0), "three corners is the least");
    }

    #[test]
    fn inside_means_inside_the_outline_not_the_frame() {
        let tri = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)];
        assert!(contains(&tri, (0.2, 0.2)));
        assert!(!contains(&tri, (0.8, 0.8)));
        assert!(contains(&ellipse_outline(), (0.5, 0.5)) && !contains(&ellipse_outline(), (0.03, 0.03)));
    }

    #[test]
    fn fitting_the_frame_keeps_the_outline_where_it_is_on_the_slide() {
        let mut o = spot();
        o.hit_area = Some(vec![(0.25, 0.5), (0.75, 0.5), (0.5, 1.0)]);
        let before: Vec<Pt> = o.hit_area.as_ref().unwrap().iter().map(|&f| to_slide(&o, f)).collect();
        fit_frame(&mut o);
        assert_eq!((o.x, o.y, o.w, o.h), (200.0, 300.0, 200.0, 100.0));
        let after: Vec<Pt> = o.hit_area.as_ref().unwrap().iter().map(|&f| to_slide(&o, f)).collect();
        for (a, b) in before.iter().zip(&after) { assert!((a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-3, "{a:?} {b:?}"); }
        assert!(o.hit_area.as_ref().unwrap().iter().any(|p| p.0 == 0.0) && o.hit_area.as_ref().unwrap().iter().any(|p| p.1 == 1.0));
        // The same, turned: the points stay put on the slide.
        let mut r = spot();
        r.rotation = 30.0;
        r.hit_area = Some(vec![(0.25, 0.5), (0.75, 0.5), (0.5, 1.0)]);
        let before: Vec<Pt> = r.hit_area.as_ref().unwrap().iter().map(|&f| to_slide(&r, f)).collect();
        fit_frame(&mut r);
        let after: Vec<Pt> = r.hit_area.as_ref().unwrap().iter().map(|&f| to_slide(&r, f)).collect();
        for (a, b) in before.iter().zip(&after) { assert!((a.0 - b.0).abs() < 1e-2 && (a.1 - b.1).abs() < 1e-2, "{a:?} {b:?}"); }
    }

    #[test]
    fn a_hotspot_is_built_from_corners_drawn_on_the_slide() {
        let (x, y, w, h, area) = from_points(&[(100.0, 100.0), (300.0, 100.0), (200.0, 260.0)]).unwrap();
        assert_eq!((x, y, w, h), (100.0, 100.0, 200.0, 160.0));
        assert_eq!(area, vec![(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)]);
        assert!(from_points(&[(0.0, 0.0), (10.0, 0.0)]).is_none(), "two points make no area");
        assert!(from_points(&[(0.0, 0.0), (100.0, 0.0), (200.0, 0.0)]).is_none(), "a flat line cannot be clicked");
    }
}
