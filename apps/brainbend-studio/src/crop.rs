//! The crop tool's geometry: dragging a crop edge, or panning the picture under its frame.
//!
//! A picture's crop says how much of the source image is hidden from each edge of its frame (see
//! `Object::crop`). Both drags work in the picture's own, unrotated axes. The picture is a rigid
//! body that turns about its frame's centre, so when a drag moves the frame's centre the whole body
//! is shifted to compensate; the picture then stays exactly where it was on screen.
use brainbend_doc::{crop_layout, Object};

/// Smallest frame a crop edge can be dragged to, in slide units.
const MIN_SIDE: f32 = 8.0;

/// A rectangle as (left, top, right, bottom).
type Rect = (f32, f32, f32, f32);

/// The whole source picture, in the same (unrotated) coordinates as the frame.
pub fn image_rect(o: &Object) -> Rect {
    let [x, y, w, h] = crop_layout(o.crop.unwrap_or([0.0; 4]));
    (o.x + x * o.w, o.y + y * o.h, o.x + (x + w) * o.w, o.y + (y + h) * o.h)
}

fn frame_rect(o: &Object) -> Rect {
    (o.x, o.y, o.x + o.w, o.y + o.h)
}

fn centre(r: Rect) -> (f32, f32) {
    ((r.0 + r.2) / 2.0, (r.1 + r.3) / 2.0)
}

/// Store `frame` and the crop that leaves `image` under it.
fn store(o: &mut Object, frame: Rect, image: Rect) {
    let (iw, ih) = ((image.2 - image.0).max(1.0), (image.3 - image.1).max(1.0));
    let crop = [(frame.0 - image.0) / iw, (frame.1 - image.1) / ih, (image.2 - frame.2) / iw, (image.3 - frame.3) / ih];
    o.crop = crop.iter().any(|v| v.abs() > 0.0005).then_some(crop);
    (o.x, o.y, o.w, o.h) = (frame.0, frame.1, frame.2 - frame.0, frame.3 - frame.1);
}

/// Move the frame's edges chosen by the handle `(hx, hy)` (-1, 0 or 1 on each axis) by `d`, given in the
/// picture's own axes. The frame stays inside the picture (or inside where it already reached).
pub fn drag_edges(o: &mut Object, (hx, hy): (i8, i8), d: (f32, f32)) {
    let (image, frame) = (image_rect(o), frame_rect(o));
    let bounds = (image.0.min(frame.0), image.1.min(frame.1), image.2.max(frame.2), image.3.max(frame.3));
    let mut f = frame;
    if hx < 0 { f.0 = (f.0 + d.0).clamp(bounds.0, f.2 - MIN_SIDE); } else if hx > 0 { f.2 = (f.2 + d.0).clamp(f.0 + MIN_SIDE, bounds.2); }
    if hy < 0 { f.1 = (f.1 + d.1).clamp(bounds.1, f.3 - MIN_SIDE); } else if hy > 0 { f.3 = (f.3 + d.1).clamp(f.1 + MIN_SIDE, bounds.3); }
    place(o, f, image);
}

/// The largest frame of shape `ratio` (width over height) that fits inside the current frame, centred on it.
/// The picture stays where it is, so the crop just closes in.
pub fn fit_aspect(o: &mut Object, ratio: f32) {
    if ratio <= 0.0 || !ratio.is_finite() { return; }
    let (image, frame) = (image_rect(o), frame_rect(o));
    let (w, h) = (frame.2 - frame.0, frame.3 - frame.1);
    let (nw, nh) = if w / h > ratio { (h * ratio, h) } else { (w, w / ratio) };
    let (c, half) = (centre(frame), (nw.max(MIN_SIDE) / 2.0, nh.max(MIN_SIDE) / 2.0));
    place(o, (c.0 - half.0, c.1 - half.1, c.0 + half.0, c.1 + half.1), image);
}

/// Like [`drag_edges`], but the frame keeps its shape `ratio` (width over height). A corner keeps the corner
/// opposite it still; an edge keeps the opposite edge still and the frame centred on the other axis.
pub fn drag_edges_locked(o: &mut Object, (hx, hy): (i8, i8), d: (f32, f32), ratio: f32) {
    if ratio <= 0.0 || !ratio.is_finite() || (hx == 0 && hy == 0) { return drag_edges(o, (hx, hy), d); }
    let (image, frame) = (image_rect(o), frame_rect(o));
    let b = (image.0.min(frame.0), image.1.min(frame.1), image.2.max(frame.2), image.3.max(frame.3));
    let (cx, cy) = centre(frame);
    // The edges that stay put, and how far each side may grow before it meets the picture's bounds.
    let (ax, ay) = (if hx < 0 { frame.2 } else { frame.0 }, if hy < 0 { frame.3 } else { frame.1 });
    let room_x = if hx < 0 { ax - b.0 } else { b.2 - ax };
    let room_y = if hy < 0 { ay - b.1 } else { b.3 - ay };
    // The width the drag asks for, and the height that goes with it.
    let (mut w, mut h);
    if hx != 0 && hy != 0 {
        let (mx, my) = (if hx < 0 { frame.0 + d.0 } else { frame.2 + d.0 }, if hy < 0 { frame.1 + d.1 } else { frame.3 + d.1 });
        let (dw, dh) = ((mx - ax).abs(), (my - ay).abs());
        w = dw.max(dh * ratio);
        h = w / ratio;
        let k = (room_x / w).min(room_y / h).min(1.0);
        (w, h) = (w * k, h * k);
    } else if hx != 0 {
        w = ((if hx < 0 { frame.0 + d.0 } else { frame.2 + d.0 }) - ax).abs();
        // Both ends of the height must stay inside the bounds, as the frame stays centred.
        let room_h = 2.0 * (cy - b.1).min(b.3 - cy);
        w = w.min(room_x).min(room_h * ratio);
        h = w / ratio;
    } else {
        h = ((if hy < 0 { frame.1 + d.1 } else { frame.3 + d.1 }) - ay).abs();
        let room_w = 2.0 * (cx - b.0).min(b.2 - cx);
        h = h.min(room_y).min(room_w / ratio);
        w = h * ratio;
    }
    if w < MIN_SIDE || h < MIN_SIDE {
        // Too small to keep: stop at the smallest frame of this shape.
        let k = (MIN_SIDE / w.max(0.001)).max(MIN_SIDE / h.max(0.001));
        (w, h) = (w * k, h * k);
    }
    let f = if hx != 0 && hy != 0 {
        (if hx < 0 { ax - w } else { ax }, if hy < 0 { ay - h } else { ay }, if hx < 0 { ax } else { ax + w }, if hy < 0 { ay } else { ay + h })
    } else if hx != 0 {
        (if hx < 0 { ax - w } else { ax }, cy - h / 2.0, if hx < 0 { ax } else { ax + w }, cy + h / 2.0)
    } else {
        (cx - w / 2.0, if hy < 0 { ay - h } else { ay }, cx + w / 2.0, if hy < 0 { ay } else { ay + h })
    };
    place(o, f, image);
}

/// Put `frame` over `image`, shifting both so the picture stays where it is on screen even though a
/// turned object's centre (and so its turning point) has moved.
fn place(o: &mut Object, frame: Rect, image: Rect) {
    let (c0, q) = (centre(frame_rect(o)), centre(frame));
    let a = o.rotation.to_radians();
    let (rx, ry) = (q.0 - c0.0, q.1 - c0.1);
    let c1 = (c0.0 + rx * a.cos() - ry * a.sin(), c0.1 + rx * a.sin() + ry * a.cos());
    let shift = (c1.0 - q.0, c1.1 - q.1);
    let moved = |r: Rect| (r.0 + shift.0, r.1 + shift.1, r.2 + shift.0, r.3 + shift.1);
    store(o, moved(frame), moved(image));
}

/// Remove the crop: the frame grows to the whole picture, which stays where it is.
pub fn reset(o: &mut Object) {
    let image = image_rect(o);
    place(o, image, image);
}

/// Slide the picture by `d` (in its own axes) under a frame that stays put. A picture that covers its
/// frame keeps covering it.
pub fn pan(o: &mut Object, d: (f32, f32)) {
    let (image, frame) = (image_rect(o), frame_rect(o));
    let slide = |lo: f32, hi: f32, flo: f32, fhi: f32, d: f32| if lo <= flo && hi >= fhi { d.clamp(fhi - hi, flo - lo) } else { d };
    let (dx, dy) = (slide(image.0, image.2, frame.0, frame.2, d.0), slide(image.1, image.3, frame.1, frame.3, d.1));
    store(o, frame, (image.0 + dx, image.1 + dy, image.2 + dx, image.3 + dy));
}

#[cfg(test)]
mod tests {
    use super::*;
    use brainbend_doc::Kind;

    fn picture() -> Object {
        Object::new("p", "Picture", Kind::Image, 100.0, 100.0, 400.0, 300.0)
    }

    /// Where the picture's centre is on screen: the body turns about the frame's centre.
    fn on_screen(o: &Object) -> (f32, f32) {
        let (c, i) = (centre(frame_rect(o)), centre(image_rect(o)));
        let a = o.rotation.to_radians();
        let (dx, dy) = (i.0 - c.0, i.1 - c.1);
        (c.0 + dx * a.cos() - dy * a.sin(), c.1 + dx * a.sin() + dy * a.cos())
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }

    #[test]
    fn cropping_an_edge_shrinks_the_frame_and_leaves_the_picture_where_it_is() {
        let mut o = picture();
        let before = image_rect(&o);
        drag_edges(&mut o, (-1, 0), (40.0, 0.0));
        assert!(close(o.x, 140.0) && close(o.w, 360.0) && close(o.h, 300.0));
        let c = o.crop.unwrap();
        assert!(close(c[0], 0.1) && c[1] == 0.0 && c[2] == 0.0 && c[3] == 0.0, "{c:?}");
        let after = image_rect(&o);
        assert!(close(before.0, after.0) && close(before.2, after.2) && close(before.1, after.1) && close(before.3, after.3));
    }

    #[test]
    fn resetting_shows_the_whole_picture_in_place_even_when_turned() {
        for deg in [0.0, 90.0, 33.0] {
            let mut o = picture();
            o.rotation = deg;
            drag_edges(&mut o, (-1, -1), (60.0, 40.0));
            drag_edges(&mut o, (1, 0), (-80.0, 0.0));
            let before = on_screen(&o);
            reset(&mut o);
            assert_eq!(o.crop, None);
            assert!(close(o.w, 400.0) && close(o.h, 300.0), "{deg}: {} x {}", o.w, o.h);
            let after = on_screen(&o);
            assert!(close(before.0, after.0) && close(before.1, after.1), "{deg}");
        }
    }

    #[test]
    fn a_corner_crops_both_edges_and_dragging_back_restores_the_picture() {
        let mut o = picture();
        drag_edges(&mut o, (1, 1), (-100.0, -60.0));
        assert!(close(o.w, 300.0) && close(o.h, 240.0));
        drag_edges(&mut o, (1, 1), (100.0, 60.0));
        assert!(close(o.w, 400.0) && close(o.h, 300.0));
        assert_eq!(o.crop, None, "no crop left once the frame fills the picture again");
    }

    #[test]
    fn an_edge_cannot_be_dragged_past_the_picture_or_past_the_other_edge() {
        let mut o = picture();
        drag_edges(&mut o, (-1, 0), (-50.0, 0.0));
        assert!(close(o.x, 100.0) && close(o.w, 400.0), "cannot grow beyond the picture");
        drag_edges(&mut o, (-1, 0), (9999.0, 0.0));
        assert!(close(o.w, MIN_SIDE), "cannot cross the other edge");
    }

    #[test]
    fn panning_moves_the_picture_under_the_frame_but_never_off_it() {
        let mut o = picture();
        o.crop = Some([0.1, 0.0, 0.1, 0.0]);
        o.w = 400.0 * 0.8;
        pan(&mut o, (-20.0, 0.0));
        let c = o.crop.unwrap();
        assert!(c[0] > 0.1 && c[2] < 0.1, "the picture moved left under the frame: {c:?}");
        pan(&mut o, (-9999.0, 0.0));
        let c = o.crop.unwrap();
        assert!(c[2].abs() < 0.001 && c[0] > 0.19, "stopped with the picture's right edge at the frame: {c:?}");
        pan(&mut o, (0.0, 50.0));
        assert!(o.crop.unwrap()[1].abs() < 0.001, "no room to pan up or down when the height is not cropped");
    }

    #[test]
    fn a_turned_picture_does_not_jump_when_its_frame_changes() {
        for deg in [90.0, 37.0, 180.0, 270.0] {
            let mut o = picture();
            o.rotation = deg;
            let before = on_screen(&o);
            drag_edges(&mut o, (-1, -1), (50.0, 30.0));
            let after = on_screen(&o);
            assert!(close(before.0, after.0) && close(before.1, after.1), "{deg}: {before:?} -> {after:?}");
            drag_edges(&mut o, (1, 0), (-70.0, 0.0));
            let after = on_screen(&o);
            assert!(close(before.0, after.0) && close(before.1, after.1), "{deg}: {before:?} -> {after:?}");
        }
    }

    fn ratio_of(o: &Object) -> f32 { o.w / o.h }

    #[test]
    fn a_chosen_shape_closes_the_frame_in_on_its_centre() {
        let mut o = picture();                       // 400 x 300
        let before = image_rect(&o);
        fit_aspect(&mut o, 1.0);
        assert!(close(o.w, 300.0) && close(o.h, 300.0), "{} x {}", o.w, o.h);
        assert!(close(o.x, 150.0), "centred: {}", o.x);
        let after = image_rect(&o);
        assert!(close(before.0, after.0) && close(before.2, after.2), "the picture did not move");
        fit_aspect(&mut o, 2.0);
        assert!(close(ratio_of(&o), 2.0) && close(o.w, 300.0) && close(o.h, 150.0), "{} x {}", o.w, o.h);
        fit_aspect(&mut o, 0.0);
        assert!(close(o.w, 300.0), "a nonsense shape changes nothing");
    }

    #[test]
    fn dragging_with_the_shape_locked_keeps_it_and_stays_inside_the_picture() {
        for handle in [(1, 1), (-1, -1), (1, -1), (-1, 1), (1, 0), (-1, 0), (0, 1), (0, -1)] {
            for d in [(-60.0, -20.0), (30.0, 45.0), (-300.0, 10.0), (500.0, 500.0), (-9999.0, -9999.0)] {
                let mut o = picture();
                fit_aspect(&mut o, 4.0 / 3.0);
                drag_edges_locked(&mut o, handle, d, 4.0 / 3.0);
                assert!(close(ratio_of(&o), 4.0 / 3.0) || o.w <= MIN_SIDE * 1.4, "{handle:?} {d:?}: {} x {}", o.w, o.h);
                let image = image_rect(&o);
                assert!(o.x >= image.0 - 0.01 && o.y >= image.1 - 0.01 && o.x + o.w <= image.2 + 0.01 && o.y + o.h <= image.3 + 0.01, "{handle:?} {d:?}: left the picture: {:?} {image:?}", (o.x, o.y, o.w, o.h));
                assert!(o.w >= MIN_SIDE - 0.01 && o.h >= MIN_SIDE - 0.01, "{handle:?} {d:?}: {} x {}", o.w, o.h);
            }
        }
    }

    #[test]
    fn a_locked_corner_keeps_the_opposite_corner_still_and_an_edge_keeps_the_frame_centred() {
        let mut o = picture();
        fit_aspect(&mut o, 1.0);                     // 300 x 300 at (150, 100)
        let (right, bottom) = (o.x + o.w, o.y + o.h);
        drag_edges_locked(&mut o, (-1, -1), (60.0, 10.0), 1.0);
        assert!(close(o.x + o.w, right) && close(o.y + o.h, bottom), "the far corner stayed");
        assert!(close(ratio_of(&o), 1.0) && o.w < 300.0);
        let mut e = picture();
        fit_aspect(&mut e, 1.0);
        let cy = e.y + e.h / 2.0;
        drag_edges_locked(&mut e, (1, 0), (-100.0, 0.0), 1.0);
        assert!(close(e.w, 200.0) && close(e.h, 200.0), "{} x {}", e.w, e.h);
        assert!(close(e.x, 150.0) && close(e.y + e.h / 2.0, cy), "the left edge and the middle stayed");
    }
}
