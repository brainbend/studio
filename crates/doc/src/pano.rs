//! 360° images: the settings and annotations of a panorama, and the viewing maths shared by the editor's preview
//! and the tests. The player (`runtime.js`) does the same maths in a shader.
//!
//! The picture is equirectangular (2:1): its left edge is longitude -180°, its middle 0°, its top latitude +90°.
//! A view is a heading (`yaw`, degrees clockwise seen from above, 0 = the picture's middle), a `pitch` (degrees,
//! positive looking up) and a horizontal field of view (`fov`, degrees).
use crate::{PathCmd, Rgba};
use serde::{Deserialize, Serialize};

pub const FOV_MIN: f32 = 30.0;
pub const FOV_MAX: f32 = 110.0;
pub const PITCH_LIMIT: f32 = 85.0;

/// Auto-rotation speed in degrees per second.
pub const SPEED_DEFAULT: f32 = 6.0;

/// What an item on the picture is: a round marker, or a hotspot (a shaded area with corners).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PinKind {
    #[default]
    Marker,
    Hotspot,
}

/// A marker or hotspot the learner can press, with a title and text shown in a card and triggers it can run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PanoPin {
    pub id: String,
    /// Where a marker sits (heading and pitch in degrees); for a hotspot, the middle of its corners.
    pub yaw: f32,
    pub pitch: f32,
    pub title: String,
    #[serde(default)]
    pub description: String,
    /// No card: pressing it only runs its triggers (kept from the first version of annotations).
    #[serde(default)]
    pub button: bool,
    #[serde(default)]
    pub kind: PinKind,
    /// A marker's icon: its name, and its outline in a 0..1 box (drawn white on the marker's color). Empty = none.
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub icon_path: Vec<PathCmd>,
    /// The marker's or hotspot's color; the default when `None`.
    #[serde(default)]
    pub color: Option<Rgba>,
    /// A hotspot's corners as (heading, pitch), in order round its edge.
    #[serde(default)]
    pub points: Vec<(f32, f32)>,
    /// Where to look for a hotspot that is not revealed (shown in a guided tour).
    #[serde(default)]
    pub hint: String,
}

pub const MARKER_COLOR: Rgba = Rgba(0x4F, 0x81, 0xBD, 255);
pub const HOTSPOT_COLOR: Rgba = Rgba(0x0D, 0xDA, 0xEF, 255);

impl PanoPin {
    /// A marker at a place with no title or text yet.
    pub fn marker(id: String, yaw: f32, pitch: f32) -> Self {
        Self { id, yaw, pitch, title: String::new(), description: String::new(), button: false, kind: PinKind::Marker, icon: String::new(), icon_path: Vec::new(), color: None, points: Vec::new(), hint: String::new() }
    }

    /// A hotspot over the given corners.
    pub fn hotspot(id: String, points: Vec<(f32, f32)>) -> Self {
        let mut p = Self::marker(id, 0.0, 0.0);
        p.kind = PinKind::Hotspot;
        p.points = points;
        p.recenter();
        p
    }

    /// The color it is drawn in.
    pub fn color(&self) -> Rgba {
        self.color.unwrap_or(if self.kind == PinKind::Hotspot { HOTSPOT_COLOR } else { MARKER_COLOR })
    }

    /// Put a hotspot's `yaw` and `pitch` in the middle of its corners (the average direction).
    pub fn recenter(&mut self) {
        if self.kind != PinKind::Hotspot || self.points.is_empty() { return; }
        let mut sum = [0.0f32; 3];
        for &(y, p) in &self.points {
            let d = direction(y, p);
            for k in 0..3 { sum[k] += d[k]; }
        }
        let len = dot(sum, sum).sqrt();
        if len < 1e-4 { return; }
        self.yaw = sum[0].atan2(sum[2]).to_degrees();
        self.pitch = (sum[1] / len).clamp(-1.0, 1.0).asin().to_degrees();
    }

    /// Whether the place `(yaw, pitch)` is inside a hotspot, as seen from the middle of it (a flat view of the corners).
    pub fn hotspot_contains(&self, yaw: f32, pitch: f32) -> bool {
        if self.kind != PinKind::Hotspot || self.points.len() < 3 { return false; }
        let view = (self.yaw, self.pitch, 90.0);
        let flat: Vec<(f32, f32)> = self.points.iter().filter_map(|&(y, p)| project(view, 1.0, y, p)).collect();
        if flat.len() != self.points.len() { return false; }
        let Some(at) = project(view, 1.0, yaw, pitch) else { return false };
        let mut inside = false;
        for i in 0..flat.len() {
            let (a, b) = (flat[i], flat[(i + 1) % flat.len()]);
            if (a.1 > at.1) != (b.1 > at.1) && at.0 < (b.0 - a.0) * (at.1 - a.1) / (b.1 - a.1) + a.0 { inside = !inside; }
        }
        inside
    }
}

/// What a 360° image object holds besides its picture (the object's `asset`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pano {
    /// The view the learner starts with.
    #[serde(default)]
    pub yaw: f32,
    #[serde(default)]
    pub pitch: f32,
    #[serde(default = "fov_default")]
    pub fov: f32,
    /// Turn slowly on its own until the learner takes over (they can always pause it).
    #[serde(default)]
    pub auto_rotate: bool,
    #[serde(default = "speed_default")]
    pub speed: f32,
    /// The learner may zoom (mouse wheel, pinch, + and - keys).
    #[serde(default = "yes")]
    pub zoom: bool,
    /// The markers and hotspots, in the order a guided tour visits them.
    #[serde(default)]
    pub pins: Vec<PanoPin>,
    /// A guided tour: the learner steps through the items in order, instead of looking around freely.
    #[serde(default)]
    pub tour: bool,
    /// Show how many items the learner has visited.
    #[serde(default = "yes")]
    pub progress: bool,
    /// The cards are dark (white text) rather than light.
    #[serde(default = "yes")]
    pub dark: bool,
    #[serde(default = "yes")]
    pub rounded: bool,
    /// The little pointer joining a card to its item.
    #[serde(default = "yes")]
    pub connector: bool,
    /// Items the learner has not visited yet pulse.
    #[serde(default = "yes")]
    pub pulse: bool,
    /// Hotspots are shaded all the time; when not, they show only under the pointer (and a hint is given in a tour).
    #[serde(default = "yes")]
    pub reveal: bool,
}

fn fov_default() -> f32 { 90.0 }
fn speed_default() -> f32 { SPEED_DEFAULT }
fn yes() -> bool { true }

impl Pano {
    pub fn new() -> Self {
        Self { yaw: 0.0, pitch: 0.0, fov: fov_default(), auto_rotate: false, speed: SPEED_DEFAULT, zoom: true, pins: Vec::new(), tour: false, progress: true, dark: true, rounded: true, connector: true, pulse: true, reveal: true }
    }

    /// An id for a new pin that no other pin has.
    pub fn new_pin_id(&self) -> String {
        let mut n = self.pins.len() + 1;
        while self.pins.iter().any(|p| p.id == format!("pin{n}")) { n += 1; }
        format!("pin{n}")
    }
}

impl Default for Pano {
    fn default() -> Self { Self::new() }
}

/// Wrap a heading into -180..180.
pub fn wrap_yaw(yaw: f32) -> f32 {
    let y = (yaw + 180.0).rem_euclid(360.0) - 180.0;
    if y == -180.0 { 180.0 } else { y }
}

pub fn clamp_pitch(p: f32) -> f32 { p.clamp(-PITCH_LIMIT, PITCH_LIMIT) }
pub fn clamp_fov(f: f32) -> f32 { f.clamp(FOV_MIN, FOV_MAX) }

type V3 = [f32; 3];
fn dot(a: V3, b: V3) -> f32 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
fn cross(a: V3, b: V3) -> V3 { [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]] }

/// The unit direction for a heading and pitch.
pub fn direction(yaw: f32, pitch: f32) -> V3 {
    let (y, p) = (yaw.to_radians(), pitch.to_radians());
    [p.cos() * y.sin(), p.sin(), p.cos() * y.cos()]
}

/// The camera's (forward, right, up) for a heading and pitch.
pub fn camera(yaw: f32, pitch: f32) -> (V3, V3, V3) {
    let f = direction(yaw, pitch);
    let y = yaw.to_radians();
    let r = [y.cos(), 0.0, -y.sin()];
    (f, r, cross(f, r))
}

/// The heading and pitch seen through the point `(x, y)` of the view, each -1..1 from the middle (x to the right,
/// y up), for a view `aspect` (width over height) wide.
pub fn look_at(yaw: f32, pitch: f32, fov: f32, aspect: f32, x: f32, y: f32) -> (f32, f32) {
    let (f, r, u) = camera(yaw, pitch);
    let t = (fov.to_radians() / 2.0).tan();
    let d: V3 = std::array::from_fn(|i| f[i] + r[i] * x * t + u[i] * y * t / aspect);
    let len = dot(d, d).sqrt();
    (d[0].atan2(d[2]).to_degrees(), (d[1] / len).clamp(-1.0, 1.0).asin().to_degrees())
}

/// Where a place `(yaw, pitch)` appears in a view, as `(x, y)` each -1..1 from the middle; `None` when it is behind
/// the viewer. Points outside -1..1 are off the view.
pub fn project(view: (f32, f32, f32), aspect: f32, yaw: f32, pitch: f32) -> Option<(f32, f32)> {
    let (f, r, u) = camera(view.0, view.1);
    let d = direction(yaw, pitch);
    let depth = dot(d, f);
    if depth <= 0.02 { return None; }
    let t = (view.2.to_radians() / 2.0).tan();
    Some((dot(d, r) / depth / t, dot(d, u) / depth / t * aspect))
}

/// The picture coordinates (0..1 across, 0..1 down) of a heading and pitch.
pub fn picture_uv(yaw: f32, pitch: f32) -> (f32, f32) {
    (0.5 + wrap_yaw(yaw) / 360.0, 0.5 - pitch / 180.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_middle_of_the_view_looks_where_the_view_points() {
        for (yaw, pitch) in [(0.0, 0.0), (90.0, 20.0), (-135.0, -40.0), (170.0, 60.0)] {
            let (y, p) = look_at(yaw, pitch, 90.0, 16.0 / 9.0, 0.0, 0.0);
            assert!((wrap_yaw(y - yaw)).abs() < 1e-3 && (p - pitch).abs() < 1e-3, "{yaw},{pitch} -> {y},{p}");
        }
    }

    #[test]
    fn projecting_undoes_looking_at() {
        let view = (30.0, -10.0, 75.0);
        let aspect = 16.0 / 9.0;
        for (x, y) in [(0.5, 0.25), (-0.8, 0.6), (0.9, -0.9), (0.0, 0.0)] {
            let (lon, lat) = look_at(view.0, view.1, view.2, aspect, x, y);
            let (px, py) = project(view, aspect, lon, lat).unwrap();
            assert!((px - x).abs() < 1e-3 && (py - y).abs() < 1e-3, "{x},{y} -> {px},{py}");
        }
    }

    #[test]
    fn right_is_clockwise_and_up_is_up() {
        let view = (0.0, 0.0, 90.0);
        let (x, _) = project(view, 1.0, 20.0, 0.0).unwrap();
        assert!(x > 0.0, "a larger heading is to the right");
        let (_, y) = project(view, 1.0, 0.0, 20.0).unwrap();
        assert!(y > 0.0, "a higher pitch is up");
        assert!(project(view, 1.0, 180.0, 0.0).is_none(), "behind the viewer");
    }

    #[test]
    fn a_narrower_view_magnifies() {
        let near = project((0.0, 0.0, 40.0), 1.0, 10.0, 0.0).unwrap().0;
        let far = project((0.0, 0.0, 100.0), 1.0, 10.0, 0.0).unwrap().0;
        assert!(near > far * 2.0);
    }

    #[test]
    fn headings_wrap_and_picture_coordinates_follow() {
        assert_eq!(wrap_yaw(190.0), -170.0);
        assert_eq!(wrap_yaw(-180.0), 180.0);
        assert_eq!(wrap_yaw(540.0), 180.0);
        assert_eq!(picture_uv(0.0, 0.0), (0.5, 0.5));
        assert_eq!(picture_uv(-90.0, 90.0), (0.25, 0.0));
    }

    #[test]
    fn a_hotspot_sits_in_the_middle_of_its_corners_and_contains_what_is_inside() {
        let h = PanoPin::hotspot("h".into(), vec![(10.0, 10.0), (30.0, 10.0), (30.0, -10.0), (10.0, -10.0)]);
        assert!((h.yaw - 20.0).abs() < 0.5 && h.pitch.abs() < 0.5, "{} {}", h.yaw, h.pitch);
        assert!(h.hotspot_contains(20.0, 0.0));
        assert!(!h.hotspot_contains(40.0, 0.0));
        assert!(!h.hotspot_contains(20.0, 25.0));
        assert!(!PanoPin::marker("m".into(), 0.0, 0.0).hotspot_contains(0.0, 0.0));
    }

    #[test]
    fn pin_ids_are_unique() {
        let mut p = Pano::new();
        p.pins.push(PanoPin::marker("pin2".into(), 0.0, 0.0));
        assert_eq!(p.new_pin_id(), "pin3");
        let q: Pano = serde_json::from_str("{}").unwrap();
        assert_eq!(q, Pano::new());
    }
}
