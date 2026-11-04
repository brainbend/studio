//! Framework-neutral document model for BrainBend Studio projects.
pub mod check;
pub mod find;
pub mod lesson;
pub mod master;
pub mod pano;
pub mod passage;
pub mod player;
pub mod shapes;
pub mod tables;
pub mod variables;
pub mod richtext;
pub use master::{Background, Master, PlaceholderKind, SlideLayout, Theme};
pub use player::{ButtonStyle, GlossaryTerm, PlayerControls, PlayerPalette, PlayerSettings, PlayerTheme, Resource, SidebarSide};
pub use richtext::{Attr, Fmt, ListStyle};

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Rgba(pub u8, pub u8, pub u8, pub u8);

impl Rgba {
    /// "#4F81BD".
    pub fn hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.0, self.1, self.2)
    }

    /// A colour typed by hand: "#4f81bd", "4f81bd", "#48b", "79, 129, 189" or "rgb(79,129,189)" (opaque).
    pub fn parse(text: &str) -> Option<Rgba> {
        let t = text.trim().to_ascii_lowercase();
        let inner = t.strip_prefix("rgb(").and_then(|r| r.strip_suffix(')')).unwrap_or(&t);
        if inner.contains(',') || inner.contains(' ') && !inner.starts_with('#') && inner.chars().all(|c| c.is_ascii_digit() || c == ' ') {
            let parts: Vec<&str> = inner.split(|c| c == ',' || c == ' ').filter(|p| !p.is_empty()).collect();
            if parts.len() != 3 { return None; }
            let n: Vec<u8> = parts.iter().map(|p| p.trim().parse::<u8>().ok()).collect::<Option<_>>()?;
            return Some(Rgba(n[0], n[1], n[2], 255));
        }
        let h = inner.strip_prefix('#').unwrap_or(inner);
        if !h.chars().all(|c| c.is_ascii_hexdigit()) { return None; }
        let v = |s: &str| u8::from_str_radix(s, 16).ok();
        match h.len() {
            6 => Some(Rgba(v(&h[0..2])?, v(&h[2..4])?, v(&h[4..6])?, 255)),
            3 => Some(Rgba(v(&h[0..1])? * 17, v(&h[1..2])? * 17, v(&h[2..3])? * 17, 255)),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Align {
    Left,
    Center,
    Right,
    /// Both edges straight; the last line of a paragraph stays at the left.
    Justify,
}

/// What a text object is published as, for screen readers and navigation: ordinary text (a `p`) or a heading
/// (`h1` to `h4`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextRole {
    #[default]
    Normal,
    H1,
    H2,
    H3,
    H4,
}

impl TextRole {
    pub const ALL: [TextRole; 5] = [TextRole::Normal, TextRole::H1, TextRole::H2, TextRole::H3, TextRole::H4];

    /// The name authors see.
    pub fn label(self) -> &'static str {
        match self { TextRole::Normal => "Normal Text", TextRole::H1 => "Heading 1", TextRole::H2 => "Heading 2", TextRole::H3 => "Heading 3", TextRole::H4 => "Heading 4" }
    }

    /// The heading level, or `None` for ordinary text.
    pub fn level(self) -> Option<u8> {
        match self { TextRole::Normal => None, TextRole::H1 => Some(1), TextRole::H2 => Some(2), TextRole::H3 => Some(3), TextRole::H4 => Some(4) }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum VAlign {
    Top,
    #[default]
    Middle,
    Bottom,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextStyle {
    pub font_family: String,
    pub size: f32,
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub underline: bool,
    pub color: Rgba,
    pub align: Align,
    #[serde(default)]
    pub valign: VAlign,
}

/// A linear gradient: `angle` in degrees clockwise from "towards the top" (CSS style), stops `(0..1, colour)`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Gradient {
    pub angle: f32,
    pub stops: Vec<(f32, Rgba)>,
}

/// How a video or audio object plays. The default is what an object added in BrainBend Studio does: wait for the
/// viewer, with the player's controls showing.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MediaPlay {
    /// Starts by itself when the object's turn on the timeline comes.
    pub autoplay: bool,
    pub controls: bool,
    pub looped: bool,
    pub muted: bool,
    /// 0.0 - 1.0.
    pub volume: f32,
}

impl Default for MediaPlay {
    fn default() -> Self {
        MediaPlay { autoplay: false, controls: true, looped: false, muted: false, volume: 1.0 }
    }
}

/// How an outline is drawn: unbroken, in dots or in dashes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineDash {
    #[default]
    Solid,
    Dotted,
    Dashed,
}

impl LineDash {
    /// The `border-style` / SVG pattern name for CSS.
    pub fn css(self) -> &'static str {
        match self { LineDash::Solid => "solid", LineDash::Dotted => "dotted", LineDash::Dashed => "dashed" }
    }
    /// An SVG `stroke-dasharray` for a line of `width`, empty for solid.
    pub fn dash_array(self, width: f32) -> String {
        let w = width.max(1.0);
        match self { LineDash::Solid => String::new(), LineDash::Dotted => format!("{} {}", w, w * 1.5), LineDash::Dashed => format!("{} {}", w * 4.0, w * 2.0) }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stroke {
    pub color: Rgba,
    pub width: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Shadow {
    pub dx: f32,
    pub dy: f32,
    pub blur: f32,
    /// Grows (or, if negative, shrinks) the shadow on every side before blurring.
    #[serde(default)]
    pub spread: f32,
    pub color: Rgba,
}

/// A soft halo of colour around an object.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Glow {
    /// How far the halo reaches, in pixels.
    pub radius: f32,
    pub color: Rgba,
}

/// A faded, upside-down copy of an object just below it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Reflection {
    /// How much of the object's height the reflection shows, from 0 to 1.
    pub size: f32,
    /// How visible the reflection is where it meets the object, from 0 to 1.
    pub opacity: f32,
    /// The space between the object and its reflection, in pixels.
    pub gap: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectKind {
    Fade,
    SlideLeft,
    SlideRight,
    SlideUp,
    SlideDown,
    Zoom,
    Wipe,
    /// Grows from small while turning a quarter, half or whole turn clockwise.
    SpinQuarter,
    SpinHalf,
    SpinFull,
    /// A clockwise sweep from the top, like a clock hand uncovering the object.
    Wheel,
    /// Opens from a line through the middle (`Out`) or closes in from both edges (`In`).
    SplitVerticalOut,
    SplitVerticalIn,
    SplitHorizontalOut,
    SplitHorizontalIn,
}

/// Entrance animation played each time the object's layer or slide is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entrance {
    pub kind: EffectKind,
    pub delay_ms: u32,
    pub duration_ms: u32,
}

/// Emphasis effects: the object stays where it is and draws attention to itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmphasisKind {
    /// Swells and settles back.
    Pulse,
    /// Turns once around.
    Spin,
    /// Grows and stays larger.
    Grow,
    /// Shakes from side to side.
    Shake,
    /// Blinks.
    Flash,
    /// Hops up and down.
    Bounce,
    /// Rocks like a pendulum.
    Swing,
}

impl EmphasisKind {
    pub const ALL: [EmphasisKind; 7] = [EmphasisKind::Pulse, EmphasisKind::Spin, EmphasisKind::Grow, EmphasisKind::Shake, EmphasisKind::Flash, EmphasisKind::Bounce, EmphasisKind::Swing];

    pub fn label(self) -> &'static str {
        match self {
            EmphasisKind::Pulse => "Pulse",
            EmphasisKind::Spin => "Spin",
            EmphasisKind::Grow => "Grow",
            EmphasisKind::Shake => "Shake",
            EmphasisKind::Flash => "Flash",
            EmphasisKind::Bounce => "Bounce",
            EmphasisKind::Swing => "Swing",
        }
    }
}

/// An emphasis animation, played after the object appears (and again on a "play animation" action).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Emphasis {
    pub kind: EmphasisKind,
    pub delay_ms: u32,
    pub duration_ms: u32,
    /// How many times it plays; 0 = until the slide or layer ends.
    pub repeat: u32,
}

/// A motion path: the object travels through `points`, given as offsets in slide units from where it
/// sits (it starts at offset 0, 0 and ends at the last point).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MotionPath {
    pub points: Vec<(f32, f32)>,
    pub delay_ms: u32,
    pub duration_ms: u32,
    /// Curve through the points instead of straight segments.
    pub smooth: bool,
}

/// Ready-made paths to start from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathPreset {
    Right,
    Left,
    Up,
    Down,
    Arc,
    Circle,
    Zigzag,
}

impl PathPreset {
    pub const ALL: [PathPreset; 7] = [PathPreset::Right, PathPreset::Left, PathPreset::Up, PathPreset::Down, PathPreset::Arc, PathPreset::Circle, PathPreset::Zigzag];

    pub fn label(self) -> &'static str {
        match self {
            PathPreset::Right => "Line right",
            PathPreset::Left => "Line left",
            PathPreset::Up => "Line up",
            PathPreset::Down => "Line down",
            PathPreset::Arc => "Arc",
            PathPreset::Circle => "Circle",
            PathPreset::Zigzag => "Zigzag",
        }
    }
}

impl MotionPath {
    pub fn preset(kind: PathPreset) -> MotionPath {
        let d = 300.0;
        let (points, smooth) = match kind {
            PathPreset::Right => (vec![(0.0, 0.0), (d, 0.0)], false),
            PathPreset::Left => (vec![(0.0, 0.0), (-d, 0.0)], false),
            PathPreset::Up => (vec![(0.0, 0.0), (0.0, -d)], false),
            PathPreset::Down => (vec![(0.0, 0.0), (0.0, d)], false),
            PathPreset::Arc => (vec![(0.0, 0.0), (d * 0.5, -d * 0.6), (d, 0.0)], true),
            PathPreset::Circle => {
                let r = d / 2.0;
                // Starts at the circle's left edge and goes once around.
                ((0..=8).map(|i| { let a = std::f32::consts::PI - i as f32 * std::f32::consts::TAU / 8.0; (r + r * a.cos(), -r * a.sin()) }).collect::<Vec<_>>(), true)
            }
            PathPreset::Zigzag => (vec![(0.0, 0.0), (d * 0.25, -d * 0.3), (d * 0.5, 0.0), (d * 0.75, -d * 0.3), (d, 0.0)], false),
        };
        MotionPath { points, delay_ms: 0, duration_ms: 2000, smooth }
    }

    /// The path as drawn: the points themselves, or a smooth curve through them (Catmull-Rom).
    pub fn sampled(&self, per_segment: usize) -> Vec<(f32, f32)> {
        let p = &self.points;
        if !self.smooth || p.len() < 3 { return p.clone(); }
        let mut out = vec![p[0]];
        for i in 0..p.len() - 1 {
            let (p0, p1, p2, p3) = (p[i.saturating_sub(1)], p[i], p[i + 1], p[(i + 2).min(p.len() - 1)]);
            for k in 1..=per_segment {
                let t = k as f32 / per_segment as f32;
                let (t2, t3) = (t * t, t * t * t);
                let f = |a: f32, b: f32, c: f32, d: f32| 0.5 * ((2.0 * b) + (-a + c) * t + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2 + (-a + 3.0 * b - 3.0 * c + d) * t3);
                out.push((f(p0.0, p1.0, p2.0, p3.0), f(p0.1, p1.1, p2.1, p3.1)));
            }
        }
        out
    }
}

/// Which click starts each of an object's animations. 0 = automatically (on the timeline); 1, 2, ...
/// = the learner's first, second, ... click on the slide (as in a PowerPoint presentation). Effects with
/// the same number play together; their delays count from that click.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClickSteps {
    pub entrance: u32,
    pub emphasis: u32,
    pub motion: u32,
    pub exit: u32,
}

impl ClickSteps {
    pub fn any(&self) -> bool {
        self.entrance > 0 || self.emphasis > 0 || self.motion > 0 || self.exit > 0
    }
}

/// When an object is on screen within its layer's (or slide's) timeline. Objects without timing
/// are there for the whole timeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timing {
    pub start_ms: u32,
    /// `None` = until the end of the timeline.
    pub dur_ms: Option<u32>,
}

/// A run of text whose style differs from the object's. Unset fields inherit the object style.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Run {
    pub text: String,
    pub bold: Option<bool>,
    #[serde(default)]
    pub italic: Option<bool>,
    #[serde(default)]
    pub underline: Option<bool>,
    pub color: Option<Rgba>,
    pub size: Option<f32>,
    pub font_family: Option<String>,
    /// A web or mail address this text opens when clicked.
    #[serde(default)]
    pub link: Option<String>,
    #[serde(default)]
    pub strike: Option<bool>,
    /// A marker-pen colour behind the text.
    #[serde(default)]
    pub highlight: Option<Rgba>,
    /// Extra space between letters, in px (negative pulls them together).
    #[serde(default)]
    pub spacing: Option<f32>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Paragraph {
    pub runs: Vec<Run>,
    /// How many steps the whole paragraph is moved in from the left edge.
    #[serde(default)]
    pub indent: u8,
}

/// How far one indent step moves a paragraph, in slide px.
pub const INDENT_STEP: f32 = 40.0;

/// One drawing command of a freeform path, in 0..1 coordinates of the object's frame.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum PathCmd {
    Move(f32, f32),
    Line(f32, f32),
    /// Cubic Bezier: two control points, then the end point.
    Curve(f32, f32, f32, f32, f32, f32),
    Close,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Kind {
    Rect,
    Ellipse,
    Text,
    Image,
    /// Video or audio from an asset; the player shows native controls.
    Video,
    Audio,
    /// An invisible click target (a "hotspot"): no visuals in the published lesson.
    Hotspot,
    /// A freeform vector shape stretched to the frame, filled and stroked like other shapes.
    Path(Vec<PathCmd>),
    /// A straight line from the frame's top-left to bottom-right corner, drawn with the stroke.
    Line,
    /// A straight line from the frame's bottom-left to top-right corner.
    LineUp,
    /// A track and thumb the learner drags to set a number variable.
    Slider(Slider),
    /// A web page shown inside the slide (a "web object").
    Web(WebObject),
    /// A field the learner types into (text or a number); what is typed is kept in a variable.
    Input(InputField),
    /// A 360° image: the equirectangular picture is the object's `asset`; the learner looks around by dragging.
    Pano(pano::Pano),
    /// A source object the importer could not convert yet; kept for diagnostics.
    Unsupported(String),
}

/// An equation: its LaTeX source, kept so it can be edited again, and the MathML that screen readers get. The object
/// itself is an ordinary freeform shape (`Kind::Path`) holding the typeset outlines, so every renderer draws it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EquationInfo {
    pub latex: String,
    /// Display style (large operators, stacked limits) rather than inline style.
    #[serde(default = "yes")]
    pub display: bool,
    /// MathML for assistive technology (empty when it could not be made).
    #[serde(default)]
    pub mathml: String,
    /// The typeset width in points; the object's width over this is its scale, kept when the equation is edited.
    #[serde(default)]
    pub width_pt: f32,
}

/// What a marker shows: a title on hover (or keyboard focus) and, when it is clicked, a card with the title and a
/// description. Clicking anywhere else closes the card.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MarkerInfo {
    pub title: String,
    pub description: String,
    /// The card's size in slide units.
    #[serde(default = "card_w")]
    pub card_w: f32,
    #[serde(default = "card_h")]
    pub card_h: f32,
}

fn card_w() -> f32 { 640.0 }
fn card_h() -> f32 { 340.0 }

impl MarkerInfo {
    pub fn new() -> Self { Self { title: "Title".into(), description: "Type the description here.".into(), card_w: card_w(), card_h: card_h() } }

    /// Where a card `(cw, ch)` goes for a marker at `(x, y, w, h)` on a `(sw, sh)` slide: above the marker when it fits
    /// (else below), centred on it but kept on the slide. Returns the card's left and top, where its pointer sits along
    /// it (from the card's left), and whether the card is above the marker.
    pub fn card_at(marker: (f32, f32, f32, f32), slide: (f32, f32), card: (f32, f32)) -> (f32, f32, f32, bool) {
        const GAP: f32 = 18.0;
        let (x, y, w, h) = marker;
        let (cx, above) = (x + w / 2.0, y - GAP - card.1 >= 0.0);
        let left = (cx - card.0 / 2.0).clamp(8.0, (slide.0 - card.0 - 8.0).max(8.0));
        let top = if above { y - GAP - card.1 } else { y + h + GAP };
        let notch = (cx - left).clamp(34.0, (card.0 - 34.0).max(34.0));
        (left, top, notch, above)
    }
}

/// A data-entry field. Its look comes from the object's own style (font, text colour), fill and outline.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InputField {
    /// The id of the variable that holds what is typed: a text variable, or a number variable for a number field.
    pub variable: String,
    /// A number field (with step arrows) rather than a text field.
    #[serde(default)]
    pub numeric: bool,
    /// Faint text shown while the field is empty.
    #[serde(default)]
    pub placeholder: String,
    /// The range and step of a number field.
    #[serde(default)]
    pub min: f32,
    #[serde(default = "ten")]
    pub max: f32,
    #[serde(default = "one")]
    pub step: f32,
}

fn ten() -> f32 { 10.0 }
fn one() -> f32 { 1.0 }

impl InputField {
    pub fn text(variable: &str) -> Self { Self { variable: variable.into(), numeric: false, placeholder: "Type here".into(), min: 0.0, max: 10.0, step: 1.0 } }
    pub fn number(variable: &str) -> Self { Self { variable: variable.into(), numeric: true, placeholder: String::new(), min: 0.0, max: 100.0, step: 1.0 } }
}

/// A web page shown in a frame on the slide. The object's `alt` is the frame's title for screen readers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WebObject {
    /// The page's address (`https://`; many sites refuse to be shown inside another page).
    pub url: String,
    /// Whether the learner can scroll the page inside the frame.
    #[serde(default = "yes")]
    pub scroll: bool,
}

impl WebObject {
    pub fn new() -> Self { Self { url: String::new(), scroll: true } }

    /// The address if it is one a frame may show: `https://` (or `http://`).
    pub fn safe_url(&self) -> Option<&str> {
        let u = self.url.trim();
        (u.starts_with("https://") || u.starts_with("http://")).then_some(u)
    }
}

/// A slider: the thumb's place along the track sets a number variable, and the variable's value
/// places the thumb. The frame is the track area; the thumb is as tall as `thumb_height` of it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Slider {
    /// The id of the number variable it sets; empty while it is not linked to one.
    pub variable: String,
    pub min: f32,
    pub max: f32,
    /// The smallest change in value; the thumb snaps to multiples of it from `min`.
    pub step: f32,
    /// A picture used as the thumb; without one the thumb is a circle in `thumb_color`.
    pub thumb: Option<String>,
    pub thumb_color: Rgba,
    /// The thumb's height as a fraction of the frame's height.
    pub thumb_height: f32,
    /// The track line's colour; `None` leaves the track invisible.
    pub track: Option<Rgba>,
}

impl Slider {
    pub fn new() -> Self {
        Slider { variable: String::new(), min: 0.0, max: 10.0, step: 1.0, thumb: None, thumb_color: Rgba(3, 83, 158, 255), thumb_height: 0.6, track: Some(Rgba(150, 160, 175, 255)) }
    }

    /// How far along the track a value is, from 0 to 1.
    pub fn fraction(&self, value: f32) -> f32 {
        range_fraction(self.min, self.max, value)
    }

    /// The value (snapped to the step) for a position along the track, 0 to 1.
    pub fn value_at(&self, fraction: f32) -> f32 {
        snap_value(self.min, self.max, self.step, self.min + fraction.clamp(0.0, 1.0) * (self.max - self.min))
    }
}

impl Default for Slider {
    fn default() -> Self {
        Self::new()
    }
}

/// A check box: a small box ticked by clicking, drawn at the left of the object's text. Clicking
/// moves the object between its Normal and Selected states; Disabled greys it out and ignores clicks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Check {
    /// The box's side, in slide units.
    pub size: f32,
    pub box_fill: Rgba,
    /// The box's outline.
    pub box_line: Option<Rgba>,
    /// The tick mark.
    pub tick: Rgba,
    /// Makes it a radio button of this group: a round box with a dot, and choosing it clears the others of the group on
    /// the slide (a radio button cannot be unticked by clicking it).
    #[serde(default)]
    pub radio: Option<String>,
}

impl Check {
    pub fn new() -> Self {
        Check { size: 20.0, box_fill: Rgba(244, 244, 244, 255), box_line: Some(Rgba(150, 150, 150, 255)), tick: Rgba(0, 0, 0, 255), radio: None }
    }

    /// The names of the four looks offered for check boxes and radio buttons.
    pub const LOOKS: [&'static str; 4] = ["Soft", "Large", "Dark frame", "Small"];

    /// One of the four looks (`LOOKS`), as a check box, or as a radio button of `group`.
    pub fn look(n: usize, radio: Option<String>) -> Self {
        let g = |v: u8| Rgba(v, v, v, 255);
        let (size, box_fill, box_line, tick) = match n % 4 {
            0 => (48.0, Rgba(238, 242, 247, 255), Rgba(170, 176, 186, 255), Rgba(52, 120, 220, 255)),
            1 => (68.0, g(255), g(185), Rgba(52, 120, 220, 255)),
            2 => (48.0, g(226), g(40), g(40)),
            _ => (32.0, g(255), g(90), g(30)),
        };
        Check { size, box_fill, box_line: Some(box_line), tick, radio }
    }

    /// The box inside a frame of height `h`, as (left, top, side) relative to the frame's top-left.
    pub fn box_in(&self, h: f32) -> (f32, f32, f32) {
        let side = self.size.min(h).max(1.0);
        (self.size * 0.3, (h - side) / 2.0, side)
    }
}

impl Default for Check {
    fn default() -> Self {
        Self::new()
    }
}

/// An object that turns like a dial: turning it sets a number variable, and the variable's value
/// sets how far it is turned. The object turns through `sweep` degrees from `min` to `max`, with
/// the middle of the range at its drawn orientation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Dial {
    /// The id of the number variable it sets.
    pub variable: String,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    /// The whole turn from `min` to `max`, in degrees (at most 360).
    pub sweep: f32,
    /// The point it turns about, as an offset from the object's centre.
    #[serde(default)]
    pub pivot: (f32, f32),
}

impl Dial {
    /// The turn, in degrees clockwise from the drawn orientation, for a value.
    pub fn angle(&self, value: f32) -> f32 {
        range_fraction(self.min, self.max, value) * self.sweep - self.sweep / 2.0
    }

    /// The value (snapped to the step) for a turn in degrees clockwise from the drawn orientation.
    pub fn value_at(&self, angle: f32) -> f32 {
        let half = self.sweep / 2.0;
        let a = if self.sweep >= 360.0 { (angle + 180.0).rem_euclid(360.0) - 180.0 } else { angle.clamp(-half, half) };
        snap_value(self.min, self.max, self.step, self.min + (a + half) / self.sweep.max(1.0) * (self.max - self.min))
    }
}

fn range_fraction(min: f32, max: f32, value: f32) -> f32 {
    if max <= min { 0.0 } else { ((value - min) / (max - min)).clamp(0.0, 1.0) }
}

fn snap_value(min: f32, max: f32, step: f32, v: f32) -> f32 {
    let v = if step > 0.0 { min + ((v - min) / step).round() * step } else { v };
    // Steps that do not divide the range evenly stop at the last whole step.
    v.clamp(min.min(max), max.max(min))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Object {
    pub id: String,
    pub name: String,
    pub kind: Kind,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub fill: Option<Rgba>,
    /// A gradient drawn instead of `fill` (which then holds a representative flat colour).
    #[serde(default)]
    pub gradient: Option<Gradient>,
    /// Clockwise turn about the frame's centre, in degrees.
    #[serde(default)]
    pub rotation: f32,
    /// Line height as a multiple of the font size; 0 = the font's normal spacing.
    #[serde(default)]
    pub line_spacing: f32,
    /// Objects on one layer with the same group id select, move and resize together.
    #[serde(default)]
    pub group: Option<String>,
    /// (row, column) when this object is a cell of a table (the cells share a `group`).
    #[serde(default)]
    pub table: Option<(u16, u16)>,
    /// (rows, columns) a merged table cell covers; the cells it covers have no objects of their own.
    #[serde(default)]
    pub table_span: Option<(u16, u16)>,
    /// Per-side borders of a table cell (left, right, top, bottom); `None` means `stroke` on every side.
    #[serde(default)]
    pub table_borders: Option<[Option<Stroke>; 4]>,
    /// Emphasis animation (pulse, spin, ...).
    #[serde(default)]
    pub emphasis: Option<Emphasis>,
    /// Motion path animation.
    #[serde(default)]
    pub motion: Option<MotionPath>,
    /// Which click starts each animation (0 = automatically).
    #[serde(default)]
    pub click_steps: ClickSteps,
    #[serde(default)]
    pub stroke: Option<Stroke>,
    #[serde(default)]
    pub shadow: Option<Shadow>,
    /// A halo of colour around the object.
    #[serde(default)]
    pub glow: Option<Glow>,
    /// Fades the object's edges inward over this many pixels (0 is off). A softened object shows no shadow or glow.
    #[serde(default)]
    pub soft_edge: f32,
    /// A faded mirror image below the object.
    #[serde(default)]
    pub reflection: Option<Reflection>,
    /// Initially hidden (inferred from the source's state flag 0x20; also inherited from groups).
    #[serde(default)]
    pub hidden: bool,
    /// Text insets: left, top, right, bottom.
    #[serde(default)]
    pub margins: [f32; 4],
    /// Corner radius in slide units.
    #[serde(default)]
    pub radius: f32,
    /// Alternative text for assistive technology.
    #[serde(default)]
    pub alt: String,
    /// Purely decorative: assistive technology skips it and it needs no alt text.
    #[serde(default)]
    pub decorative: bool,
    /// A picture's crop: the fraction of the source image hidden from the left, top, right and
    /// bottom edges (as in PowerPoint's `srcRect`). Negative values pad the picture with space.
    /// The picture is stretched so the part that stays visible fills the frame.
    #[serde(default)]
    pub crop: Option<[f32; 4]>,
    /// Turning this object (like a dial) sets a number variable.
    #[serde(default)]
    pub dial: Option<Dial>,
    /// The object is a check box.
    #[serde(default)]
    pub check: Option<Check>,
    /// A hotspot's click area, when it is not the whole frame: a polygon as (x, y) points in
    /// fractions of the frame. Imported freeform and oval hotspots carry one.
    #[serde(default)]
    pub hit_area: Option<Vec<(f32, f32)>>,
    /// A picture cropped to a shape: only the part inside it shows.
    #[serde(default)]
    pub mask: Option<PictureShape>,
    /// How the object's outline (`stroke`) is drawn.
    #[serde(default)]
    pub dash: LineDash,
    /// Video / audio playback settings.
    #[serde(default)]
    pub media: MediaPlay,
    /// Layers (by id) that hide this object while they show ("hide objects on base layer").
    #[serde(default)]
    pub hide_on_layers: Vec<String>,
    /// Buttons that share a set (a button set, e.g. a row of tabs): pressing one puts it in its
    /// Selected state and puts the others of the set back to their starting state.
    #[serde(default)]
    pub button_set: Option<String>,
    /// A video's still picture (asset key): shown in the editor and before playback in the player.
    #[serde(default)]
    pub poster: Option<String>,
    /// Text direction: 0 = horizontal, 90 = reads top to bottom, 270 = reads bottom to top.
    #[serde(default)]
    pub text_rotation: i16,
    /// The semantic element the text is published as: a heading level, or ordinary text.
    #[serde(default)]
    pub role: TextRole,
    /// Makes the object a marker: its title shows on hover and its description, in a card, on click.
    #[serde(default)]
    pub marker: Option<MarkerInfo>,
    /// The LaTeX behind a freeform shape that is an equation (the shape is the typeset result).
    #[serde(default)]
    pub equation: Option<EquationInfo>,
    /// Key into `Project::assets` for a WebVTT caption file (video).
    #[serde(default)]
    pub captions: Option<String>,
    #[serde(default)]
    pub entrance: Option<Entrance>,
    /// Played at the end of the object's time on screen (`delay_ms` is unused).
    #[serde(default)]
    pub exit: Option<Entrance>,
    #[serde(default)]
    pub timing: Option<Timing>,
    /// Mixed-style text (imported). `text` always holds the plain version; editing the text in
    /// the editor drops this and falls back to the single object style.
    #[serde(default)]
    pub rich: Option<Vec<Paragraph>>,
    pub text: String,
    pub style: TextStyle,
    /// Key into `Project::assets`.
    pub asset: Option<String>,
    /// Named appearances (Normal, Hover, Down, Disabled, ...). Empty for plain shapes.
    #[serde(default)]
    pub states: Vec<ObjectState>,
    #[serde(default)]
    pub initial_state: String,
    /// Set on a text object that fills a layout placeholder (title, text, ...).
    #[serde(default)]
    pub placeholder: Option<PlaceholderKind>,
}

impl Object {
    /// A plain object with default text style; callers set what they need.
    /// Whether the object has states that make it behave as a button. A picture's own states (like
    /// Car-01 and Car-02) do not.
    pub fn has_button_states(&self) -> bool {
        self.states.iter().any(|s| matches!(s.name.as_str(), "Hover" | "Down" | "Disabled" | "Visited" | "Selected"))
    }

    pub fn new(id: &str, name: &str, kind: Kind, x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind,
            x,
            y,
            w,
            h,
            glow: None,
            soft_edge: 0.0,
            reflection: None,
            fill: None,
            gradient: None,
            rotation: 0.0,
            line_spacing: 0.0,
            group: None,
            table: None, table_span: None, table_borders: None, emphasis: None, motion: None, click_steps: Default::default(), decorative: false, crop: None, dial: None, check: None, hit_area: None, mask: None, dash: LineDash::Solid, media: Default::default(), hide_on_layers: vec![], button_set: None, poster: None,
            stroke: None,
            shadow: None,
            hidden: false,
            margins: [0.0; 4],
            radius: 0.0,
            alt: String::new(),
            text_rotation: 0,
            role: TextRole::Normal,
            marker: None,
            equation: None,
            captions: None,
            rich: None,
            entrance: None,
            exit: None,
            timing: None,
            text: String::new(),
            style: TextStyle {
                font_family: "Open Sans".into(),
                size: 48.0,
                bold: false,
                italic: false,
                underline: false,
                color: Rgba(34, 34, 34, 255),
                align: Align::Left,
                valign: VAlign::Middle,
            },
            asset: None,
            states: vec![],
            initial_state: String::new(),
            placeholder: None,
        }
    }
}

/// A button's glossy overlay ("shine"): a light-to-clear gradient over the fill. A stronger `alpha`
/// is how a button's Hover state brightens it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Shine {
    pub alpha: f32,
    /// Lightest at the bottom instead of the top (the Down state).
    pub reverse: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObjectState {
    pub name: String,
    #[serde(default)]
    pub rich: Option<Vec<Paragraph>>,
    pub fill: Option<Rgba>,
    #[serde(default)]
    pub stroke: Option<Stroke>,
    #[serde(default)]
    pub shadow: Option<Shadow>,
    pub text: String,
    pub style: TextStyle,
    /// A picture object shows this picture in the state (instead of its own).
    #[serde(default)]
    pub asset: Option<String>,
    /// The size the object takes in this state, if it differs from its frame; the object resizes
    /// about its centre.
    #[serde(default)]
    pub size: Option<(f32, f32)>,
    #[serde(default)]
    pub shine: Option<Shine>,
    /// A gradient drawn instead of `fill` while the object is in this state (`fill` then holds a representative flat colour).
    #[serde(default)]
    pub gradient: Option<Gradient>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Bool(bool),
    Num(f64),
    Text(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Variable {
    pub id: String,
    pub name: String,
    pub initial: Value,
    /// Provided by the player rather than authored (e.g. `Project.Progress`).
    pub built_in: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cmp {
    Eq,
    Ne,
    Gt,
    Lt,
    Ge,
    Le,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Condition {
    pub variable: String,
    pub cmp: Cmp,
    pub value: Value,
    /// Combines with the previous condition; ignored on the first one. Evaluated left to right.
    pub or_with_previous: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Event {
    SlideStart,
    LayerStart,
    Click,
    /// The slide's (base layer's) timeline reached its end.
    SlideEnd,
    /// Pointer enters the owning object.
    Hover,
    DoubleClick,
    /// The player's Next button is pressed while this slide is showing. If a trigger runs, the
    /// default move to the next slide is skipped.
    NextButton,
    /// The player's Previous button is pressed while this slide is showing. If a trigger runs, the
    /// default move to the previous slide is skipped.
    PreviousButton,
    /// A key is pressed while the owner is on screen (a `KeyboardEvent.key` value such as "Enter" or "a").
    KeyPress(String),
    /// Pointer leaves the owning object.
    MouseLeave,
    /// The owning video or audio object finishes playing.
    MediaEnded,
    /// The owning video or audio object plays to this many milliseconds (a cue point).
    MediaReaches(u32),
    /// The owning slide's or layer's timeline reaches this many milliseconds.
    TimelineReaches(u32),
    /// Fires when the variable's value changes.
    VariableChanged(String),
    /// Fires when the object with this id enters the named state (a check box ticked, a button
    /// disabled, ...). An empty state name means any state.
    StateChanged(String, String),
    /// Fires when the learner presses the annotation (second id) of the 360° image (first id) that is set to be a button.
    PanoPin(String, String),
}

impl Event {
    /// The same event with the object id it names (if any) passed through `f`.
    pub fn map_objects(&self, f: &dyn Fn(&str) -> String) -> Event {
        match self {
            Event::StateChanged(o, s) => Event::StateChanged(f(o), s.clone()),
            Event::PanoPin(o, p) => Event::PanoPin(f(o), p.clone()),
            e => e.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Action {
    ShowLayer(String),
    HideLayer(String),
    SetVariable { variable: String, value: Value },
    SetObjectState { object: String, state: String },
    RunJavaScript(String),
    /// Go to the slide with this id.
    JumpToSlide(String),
    NextSlide,
    PreviousSlide,
    /// Jump a layer's timeline, or a video or audio object's playback, to a time. An empty target
    /// means the layer (or slide) the trigger sits on.
    SeekTimeline { target: String, ms: u32 },
    /// Mark the course complete in the LMS.
    CompleteCourse,
    /// Close the course; the LMS session ends.
    ExitCourse,
    /// Start over: variables, answers and progress are reset and the first slide shows.
    RestartCourse,
    /// Make an object visible / hidden (as if its "hidden at start" were switched).
    ShowObject(String),
    HideObject(String),
    /// Change a variable relative to its current value.
    AdjustVariable { variable: String, op: AdjustOp, value: Value },
    /// Open a web page in a new tab.
    OpenUrl(String),
    /// Run an object's emphasis and motion path animations now.
    PlayAnimation(String),
    /// Start or pause a video or audio object.
    PlayMedia(String),
    PauseMedia(String),
    /// Stop: pauses a video or audio object and winds it back to the start.
    StopMedia(String),
    /// Imported but not executable yet; kept so nothing is silently dropped.
    Unsupported(String),
}

/// How `Action::AdjustVariable` combines the variable with its value.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum AdjustOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    /// Text: put the value after the current text.
    Append,
}

impl AdjustOp {
    pub const ALL: [AdjustOp; 5] = [AdjustOp::Add, AdjustOp::Subtract, AdjustOp::Multiply, AdjustOp::Divide, AdjustOp::Append];

    pub fn label(self) -> &'static str {
        match self {
            AdjustOp::Add => "add",
            AdjustOp::Subtract => "subtract",
            AdjustOp::Multiply => "multiply by",
            AdjustOp::Divide => "divide by",
            AdjustOp::Append => "append",
        }
    }
}

/// Where the whole source picture sits relative to its frame, as fractions of the frame:
/// `[x, y, width, height]`. Anything outside `0..1` is cropped away.
/// The shapes a picture can be cropped to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PictureShape {
    Oval,
    Rounded,
    Diamond,
    Triangle,
    Pentagon,
    Hexagon,
    Star,
}

impl PictureShape {
    pub const ALL: [PictureShape; 7] = [PictureShape::Oval, PictureShape::Rounded, PictureShape::Diamond, PictureShape::Triangle, PictureShape::Pentagon, PictureShape::Hexagon, PictureShape::Star];

    pub fn label(self) -> &'static str {
        match self {
            PictureShape::Oval => "Oval",
            PictureShape::Rounded => "Rounded rectangle",
            PictureShape::Diamond => "Diamond",
            PictureShape::Triangle => "Triangle",
            PictureShape::Pentagon => "Pentagon",
            PictureShape::Hexagon => "Hexagon",
            PictureShape::Star => "Star",
        }
    }

    /// The name of the same shape in PowerPoint's preset geometry.
    pub fn preset(self) -> &'static str {
        match self {
            PictureShape::Oval => "ellipse",
            PictureShape::Rounded => "roundRect",
            PictureShape::Diamond => "diamond",
            PictureShape::Triangle => "triangle",
            PictureShape::Pentagon => "pentagon",
            PictureShape::Hexagon => "hexagon",
            PictureShape::Star => "star5",
        }
    }

    pub fn from_preset(name: &str) -> Option<PictureShape> {
        PictureShape::ALL.into_iter().find(|s| s.preset() == name)
    }

    /// The outline as a CSS `clip-path: polygon(...)` value for a `w` x `h` frame.
    pub fn css_polygon(self, w: f32, h: f32) -> String {
        let pts: Vec<String> = self.polygon(w, h).iter().map(|p| format!("{:.2}% {:.2}%", p.0 * 100.0, p.1 * 100.0)).collect();
        format!("polygon({})", pts.join(","))
    }

    /// The outline as points in fractions of a `w` x `h` frame (the rounded rectangle's corners are
    /// round for that shape of frame). The points go clockwise.
    pub fn polygon(self, w: f32, h: f32) -> Vec<(f32, f32)> {
        use std::f32::consts::{PI, TAU};
        // Stretch a shape so it touches all four sides of the frame, as PowerPoint draws these presets.
        let fill = |pts: Vec<(f32, f32)>| -> Vec<(f32, f32)> {
            let (x0, x1) = pts.iter().fold((f32::MAX, f32::MIN), |m, p| (m.0.min(p.0), m.1.max(p.0)));
            let (y0, y1) = pts.iter().fold((f32::MAX, f32::MIN), |m, p| (m.0.min(p.1), m.1.max(p.1)));
            pts.into_iter().map(|p| ((p.0 - x0) / (x1 - x0), (p.1 - y0) / (y1 - y0))).collect()
        };
        match self {
            PictureShape::Oval => (0..64).map(|i| { let a = -PI / 2.0 + TAU * i as f32 / 64.0; (0.5 + 0.5 * a.cos(), 0.5 + 0.5 * a.sin()) }).collect(),
            PictureShape::Diamond => vec![(0.5, 0.0), (1.0, 0.5), (0.5, 1.0), (0.0, 0.5)],
            PictureShape::Triangle => vec![(0.5, 0.0), (1.0, 1.0), (0.0, 1.0)],
            PictureShape::Pentagon => fill((0..5).map(|i| { let a = -PI / 2.0 + TAU * i as f32 / 5.0; (a.cos(), a.sin()) }).collect()),
            PictureShape::Hexagon => vec![(0.0, 0.5), (0.25, 0.0), (0.75, 0.0), (1.0, 0.5), (0.75, 1.0), (0.25, 1.0)],
            PictureShape::Star => fill((0..10).map(|i| { let a = -PI / 2.0 + TAU * i as f32 / 10.0; let r = if i % 2 == 0 { 1.0 } else { 0.4 }; (r * a.cos(), r * a.sin()) }).collect()),
            PictureShape::Rounded => {
                // Corners a sixth of the shorter side, in fractions of each side.
                let r = w.min(h).max(1.0) / 6.0;
                let (rx, ry) = (r / w.max(1.0), r / h.max(1.0));
                let mut pts = vec![];
                // Each corner: its centre, and the quarter turn it sweeps from `start`.
                for (cx, cy, start) in [(1.0 - rx, ry, -PI / 2.0), (1.0 - rx, 1.0 - ry, 0.0), (rx, 1.0 - ry, PI / 2.0), (rx, ry, PI)] {
                    for k in 0..=8 {
                        let a = start + (PI / 2.0) * k as f32 / 8.0;
                        pts.push((cx + rx * a.cos(), cy + ry * a.sin()));
                    }
                }
                pts
            }
        }
    }
}

/// The crop that shows the `ratio` (width over height) part of an `iw` x `ih` picture. `fx` and `fy`, 0 to 1,
/// say how far along the spare width and height the visible part sits (0.5 is centred). `None` when the
/// whole picture already has that shape.
pub fn crop_for_ratio(iw: f32, ih: f32, ratio: f32, fx: f32, fy: f32) -> Option<[f32; 4]> {
    if iw <= 0.0 || ih <= 0.0 || ratio <= 0.0 || !ratio.is_finite() { return None; }
    let native = iw / ih;
    if (native - ratio).abs() / ratio < 0.005 { return None; }
    // The visible part, as fractions of the picture.
    let (vw, vh) = if native > ratio { (ratio * ih / iw, 1.0) } else { (1.0, iw / (ratio * ih)) };
    let (sx, sy) = (1.0 - vw, 1.0 - vh);
    let (l, t) = (sx * fx.clamp(0.0, 1.0), sy * fy.clamp(0.0, 1.0));
    Some([l, t, sx - l, sy - t])
}

/// The opposite of [`crop_for_ratio`]: the shape (width over height) of what a crop leaves of an `iw` x `ih`
/// picture, and where along the spare width and height it sits.
pub fn crop_shape(iw: f32, ih: f32, crop: [f32; 4]) -> (f32, f32, f32) {
    let (vw, vh) = ((1.0 - crop[0] - crop[2]).max(0.01), (1.0 - crop[1] - crop[3]).max(0.01));
    let spare = |a: f32, b: f32| if a + b > 1e-4 { a / (a + b) } else { 0.5 };
    ((iw * vw) / (ih * vh).max(1e-3), spare(crop[0], crop[2]), spare(crop[1], crop[3]))
}

pub fn crop_layout(crop: [f32; 4]) -> [f32; 4] {
    let cw = (1.0 - crop[0] - crop[2]).max(0.01);
    let ch = (1.0 - crop[1] - crop[3]).max(0.01);
    [-crop[0] / cw, -crop[1] / ch, 1.0 / cw, 1.0 / ch]
}

impl Action {
    /// The same action with every object id it names passed through `f` (for copies of slides and objects).
    pub fn map_objects(&self, f: &dyn Fn(&str) -> String) -> Action {
        match self {
            Action::SetObjectState { object, state } => Action::SetObjectState { object: f(object), state: state.clone() },
            Action::ShowObject(o) => Action::ShowObject(f(o)),
            Action::HideObject(o) => Action::HideObject(f(o)),
            Action::PlayMedia(o) => Action::PlayMedia(f(o)),
            Action::PlayAnimation(o) => Action::PlayAnimation(f(o)),
            Action::PauseMedia(o) => Action::PauseMedia(f(o)),
            Action::StopMedia(o) => Action::StopMedia(f(o)),
            Action::SeekTimeline { target, ms } if !target.is_empty() => Action::SeekTimeline { target: f(target), ms: *ms },
            a => a.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Owner {
    Slide,
    Layer(String),
    Object(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Trigger {
    pub id: String,
    pub owner: Owner,
    pub event: Event,
    pub action: Action,
    pub conditions: Vec<Condition>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Layer {
    pub id: String,
    pub name: String,
    pub objects: Vec<Object>,
    /// Length of this layer's timeline once shown (`None` = untimed).
    #[serde(default)]
    pub duration_ms: Option<u32>,
    /// A slide layer that blocks clicks on what is under it while it shows ("prevent the user
    /// from clicking on the base layer"). Without it, clicks reach the objects beneath, outside this layer's own.
    #[serde(default = "yes")]
    pub modal: bool,
    /// Showing this layer hides the slide's other layers ("hide other slide layers").
    #[serde(default)]
    pub hide_others: bool,
    /// Showing this layer pauses the base layer's timeline and any media on it ("pause timeline of base layer"),
    /// which runs on again when the layer hides.
    #[serde(default)]
    pub pause_base: bool,
    /// Names the author gave to groups of objects on this layer (group id -> name).
    #[serde(default)]
    pub group_names: std::collections::BTreeMap<String, String>,
    /// Object ids in the order a keyboard and a screen reader reach them. Empty means the stacking order (back to
    /// front); objects missing from the list follow it in stacking order.
    #[serde(default)]
    pub focus_order: Vec<String>,
}

impl Layer {
    /// Indices into `objects`, in focus order: the listed ids first, then every other object in stacking order.
    pub fn focus_indices(&self) -> Vec<usize> {
        let mut out: Vec<usize> = vec![];
        for id in &self.focus_order {
            if let Some(i) = self.objects.iter().position(|o| &o.id == id) { if !out.contains(&i) { out.push(i); } }
        }
        let rest: Vec<usize> = (0..self.objects.len()).filter(|i| !out.contains(i)).collect();
        out.extend(rest);
        out
    }

    /// Move an object `by` places later (positive) or earlier in the focus order. Returns whether it moved.
    pub fn move_focus(&mut self, id: &str, by: i32) -> bool {
        let mut ids: Vec<String> = self.focus_indices().into_iter().map(|i| self.objects[i].id.clone()).collect();
        let Some(at) = ids.iter().position(|x| x == id) else { return false };
        let to = (at as i32 + by).clamp(0, ids.len() as i32 - 1) as usize;
        if to == at { return false; }
        let moved = ids.remove(at);
        ids.insert(to, moved);
        self.focus_order = ids;
        true
    }
}

fn yes() -> bool { true }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Choice {
    pub id: String,
    pub text: String,
    pub correct: bool,
}

/// How a question is answered. Choice questions use `Question::choices`/`multiple`; the others
/// carry their own data.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum QuestionStyle {
    #[default]
    Choice,
    /// A typed answer, correct if it equals any accepted answer (ignoring surrounding spaces).
    TextEntry { accepted: Vec<String>, case_sensitive: bool },
    /// Each left item is paired with one right item chosen from a list.
    Matching { pairs: Vec<(String, String)> },
    /// Put the items in order. Listed here in the correct order; the player shuffles them.
    Sequence { items: Vec<String> },
    /// A passage with several typed blanks: `*answer*` or `*a/b*` marks each blank (see `passage`).
    FillBlanks { text: String, case_sensitive: bool },
    /// The same passage, but the answers are dragged (or clicked) into the blanks from a word bank
    /// that also holds the `distractors`.
    DragWords { text: String, distractors: Vec<String> },
    /// Mark the words in the passage that are wrapped in `*...*`.
    MarkWords { text: String },
    /// A number, correct when within `tolerance` of `answer`.
    Numeric { answer: f64, tolerance: f64 },
    /// Sort every item into the category it belongs to. The player shuffles the items.
    Categorize { categories: Vec<(String, Vec<String>)> },
    /// Click the right place on a picture. Correct when the click lands inside any region.
    Hotspot { asset: Option<String>, alt: String, regions: Vec<HotRegion> },
    /// A survey: rate each statement on the scale. Not graded (give the question 0 points).
    /// Optionally scored: the chosen position (1 = first) of every statement is added up, and
    /// the total goes into the number variable `total_variable`. `reversed[i]` flips statement i.
    Likert { statements: Vec<String>, scale: Vec<String>, #[serde(default)] reversed: Vec<bool>, #[serde(default)] total_variable: Option<String> },
    /// A free-text answer that is kept but not graded; `min_words` words are needed before it can be sent.
    Reflection { min_words: u32 },
    /// Listen to an audio clip and type what is said. Case and punctuation are ignored.
    /// `max_plays` limits how often the clip can be played (0 = as often as wanted).
    Dictation { asset: Option<String>, text: String, #[serde(default)] max_plays: u32 },
    /// A passage with drop-down blanks: `*right/wrong/wrong*`, the first option is correct.
    Dropdowns { text: String },
    /// Put the right label on each area of a picture, from a bank that also holds spare labels.
    LabelPicture { asset: Option<String>, alt: String, zones: Vec<LabelZone>, distractors: Vec<String> },
}

/// One area of a labelled picture and the label that belongs there.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LabelZone {
    pub region: HotRegion,
    pub label: String,
}

impl QuestionStyle {
    /// Point the question at a picture or audio asset (for styles that have one). Returns whether it applied.
    pub fn set_asset(&mut self, key: String) -> bool {
        match self {
            QuestionStyle::Hotspot { asset, .. } | QuestionStyle::Dictation { asset, .. } | QuestionStyle::LabelPicture { asset, .. } => { *asset = Some(key); true }
            _ => false,
        }
    }

    /// The picture or audio asset this question uses, if any.
    pub fn asset(&self) -> Option<&str> {
        match self {
            QuestionStyle::Hotspot { asset, .. } | QuestionStyle::Dictation { asset, .. } | QuestionStyle::LabelPicture { asset, .. } => asset.as_deref(),
            _ => None,
        }
    }
}

/// An area of a picture, as fractions (0 to 1) of its width and height.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HotRegion {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    #[serde(default)]
    pub ellipse: bool,
}

impl HotRegion {
    /// Whether the point (fractions of the picture) lies inside.
    pub fn contains(&self, px: f32, py: f32) -> bool {
        if self.ellipse {
            let (rx, ry) = (self.w / 2.0, self.h / 2.0);
            if rx <= 0.0 || ry <= 0.0 { return false; }
            let (dx, dy) = ((px - self.x - rx) / rx, (py - self.y - ry) / ry);
            dx * dx + dy * dy <= 1.0
        } else {
            px >= self.x && px <= self.x + self.w && py >= self.y && py <= self.y + self.h
        }
    }
}

/// A graded question. The player draws the question UI itself (prompt, choices, submit,
/// feedback); the slide's layers are not used for question slides.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Question {
    pub prompt: String,
    /// True: several choices may be correct and must all be selected (no partial credit).
    pub multiple: bool,
    pub choices: Vec<Choice>,
    pub points: f32,
    pub feedback_correct: String,
    pub feedback_incorrect: String,
    #[serde(default)]
    pub style: QuestionStyle,
    /// Where the question sits on its slide. `None` fills the slide on a pale ground, as question slides always
    /// did; with an area, the slide's own background and objects show around the question.
    #[serde(default)]
    pub area: Option<QuestionArea>,
}

/// The part of a slide a question is drawn in, in slide pixels.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuestionArea {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl QuestionArea {
    /// The area the full-slide layout uses, so switching to an area leaves the question where it was.
    pub fn default_for(width: f32, height: f32) -> Self {
        QuestionArea { x: 0.0, y: 90.0, w: width, h: height - 90.0 }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum SlideKind {
    #[default]
    Normal,
    Question(Question),
    /// Shows the quiz score, pass/fail, and a retry button.
    Results,
}

/// A named group of related slides. Slides are kept in scene order, so the flat slide list is also the lesson order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Scene {
    pub id: String,
    pub name: String,
    /// Makes the scene a question bank (see `Bank`).
    #[serde(default)]
    pub bank: Option<Bank>,
}

/// A scene whose question slides are drawn from at random: each attempt shows `draw` of them
/// (all when 0 or more than there are), in their order or shuffled. Other slides of the scene always show.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bank {
    pub draw: u32,
    pub shuffle: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Slide {
    pub id: String,
    pub name: String,
    pub width: f32,
    pub height: f32,
    /// `layers[0]` is the base layer.
    pub layers: Vec<Layer>,
    #[serde(default)]
    pub triggers: Vec<Trigger>,
    #[serde(default)]
    pub kind: SlideKind,
    /// This slide's own background; `None` takes the layout's (or master's), else white.
    #[serde(default)]
    pub background: Option<Background>,
    /// Id of the layout this slide follows, if any.
    #[serde(default)]
    pub layout: Option<String>,
    /// Fade-in time when navigating to this slide; 0 = cut.
    #[serde(default)]
    pub fade_ms: u32,
    /// Do not draw the master's and layout's shapes on this slide (Hide Background Graphics).
    #[serde(default)]
    pub hide_backdrop: bool,
    /// Move on by itself when the base layer's timeline ends.
    #[serde(default)]
    pub auto_advance: bool,
    /// Navigation rules for this slide.
    #[serde(default)]
    pub nav: SlideNav,
    /// Author notes for the slide.
    #[serde(default)]
    pub notes: String,
    /// Id of the scene (group of related slides) this slide belongs to.
    #[serde(default)]
    pub scene: String,
}

/// When the player's Next button works on a slide.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum NextRule {
    #[default]
    Always,
    /// Only after the slide's timeline has finished.
    AfterTimeline,
    /// Never: the slide moves on through its own triggers.
    Never,
}

/// What a slide looks like when the learner comes back to it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Revisit {
    /// Back to how it starts.
    #[default]
    Reset,
    /// As it was left (object states, shown layers).
    Resume,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SlideNav {
    pub next: NextRule,
    /// False hides the way back: the Previous button is off while this slide shows.
    pub previous_allowed: bool,
    pub revisit: Revisit,
}

impl Default for SlideNav {
    fn default() -> Self { SlideNav { next: NextRule::Always, previous_allowed: true, revisit: Revisit::Reset } }
}

impl Slide {
    /// The distinct click numbers the base layer's animations use, in order (empty = nothing waits for a click).
    pub fn click_steps(&self) -> Vec<u32> {
        let mut v: Vec<u32> = self.layers.first().into_iter().flat_map(|l| l.objects.iter())
            .flat_map(|o| [o.click_steps.entrance, o.click_steps.emphasis, o.click_steps.motion, o.click_steps.exit])
            .filter(|n| *n > 0).collect();
        v.sort_unstable();
        v.dedup();
        v
    }

    pub fn new(id: &str, name: &str, width: f32, height: f32) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            width,
            height,
            layers: vec![Layer { id: format!("{id}-base"), name: "Base layer".into(), objects: vec![], duration_ms: None, modal: true, hide_others: false, pause_base: false, group_names: Default::default(), focus_order: vec![] }],
            triggers: vec![],
            kind: SlideKind::Normal,
            background: None,
            layout: None,
            fade_ms: 0, hide_backdrop: false,
            auto_advance: false, nav: Default::default(),
            notes: String::new(),
            scene: String::new(),
        }
    }
}

impl Question {
    pub fn new_single() -> Self {
        let c = |i: &str, t: &str, ok: bool| Choice { id: i.into(), text: t.into(), correct: ok };
        Self {
            prompt: "Type your question here".into(),
            multiple: false,
            choices: vec![c("a", "Choice A", true), c("b", "Choice B", false), c("c", "Choice C", false)],
            points: 1.0,
            feedback_correct: "Correct!".into(),
            feedback_incorrect: "That is not correct.".into(),
            style: QuestionStyle::Choice,
            area: None,
        }
    }

    /// True or false, as a two-choice question.
    pub fn new_true_false() -> Self {
        let c = |i: &str, t: &str, ok: bool| Choice { id: i.into(), text: t.into(), correct: ok };
        Self { prompt: "Type a statement here".into(), choices: vec![c("t", "True", true), c("f", "False", false)], ..Self::new_single() }
    }

    pub fn new_text_entry() -> Self {
        Self {
            prompt: "Type your question here".into(),
            choices: vec![],
            style: QuestionStyle::TextEntry { accepted: vec!["answer".into()], case_sensitive: false },
            ..Self::new_single()
        }
    }

    pub fn new_matching() -> Self {
        let p = |a: &str, b: &str| (a.to_string(), b.to_string());
        Self {
            prompt: "Match each item".into(),
            choices: vec![],
            style: QuestionStyle::Matching { pairs: vec![p("Cat", "Meows"), p("Dog", "Barks"), p("Cow", "Moos")] },
            ..Self::new_single()
        }
    }

    pub fn new_fill_blanks() -> Self {
        Self {
            prompt: "Fill in the blanks".into(),
            choices: vec![],
            style: QuestionStyle::FillBlanks { text: "The capital of France is *Paris* and its river is the *Seine*.".into(), case_sensitive: false },
            ..Self::new_single()
        }
    }

    pub fn new_drag_words() -> Self {
        Self {
            prompt: "Drag each word into the right blank".into(),
            choices: vec![],
            style: QuestionStyle::DragWords { text: "Plants make food from *sunlight*, *water* and *air*.".into(), distractors: vec!["metal".into()] },
            ..Self::new_single()
        }
    }

    pub fn new_mark_words() -> Self {
        Self {
            prompt: "Select all the animals".into(),
            choices: vec![],
            style: QuestionStyle::MarkWords { text: "The *dog* chased the *cat* past a tree and a *horse*.".into() },
            ..Self::new_single()
        }
    }

    pub fn new_numeric() -> Self {
        Self {
            prompt: "What is 6 times 7?".into(),
            choices: vec![],
            style: QuestionStyle::Numeric { answer: 42.0, tolerance: 0.0 },
            ..Self::new_single()
        }
    }

    pub fn new_categorize() -> Self {
        let c = |n: &str, items: &[&str]| (n.to_string(), items.iter().map(|i| i.to_string()).collect::<Vec<_>>());
        Self {
            prompt: "Sort each item into its group".into(),
            choices: vec![],
            style: QuestionStyle::Categorize { categories: vec![c("Fruit", &["Apple", "Banana"]), c("Vegetable", &["Carrot", "Spinach"])] },
            ..Self::new_single()
        }
    }

    pub fn new_hotspot() -> Self {
        Self {
            prompt: "Click the correct spot on the picture".into(),
            choices: vec![],
            style: QuestionStyle::Hotspot { asset: None, alt: String::new(), regions: vec![] },
            ..Self::new_single()
        }
    }

    pub fn new_likert() -> Self {
        let scale = ["Strongly disagree", "Disagree", "Neutral", "Agree", "Strongly agree"].iter().map(|s| s.to_string()).collect();
        Self {
            prompt: "How much do you agree?".into(),
            points: 0.0,
            choices: vec![],
            feedback_correct: "Thank you for your answer.".into(),
            feedback_incorrect: "Thank you for your answer.".into(),
            style: QuestionStyle::Likert { statements: vec!["The training was useful.".into(), "The pace was right.".into()], scale, reversed: vec![], total_variable: None },
            ..Self::new_single()
        }
    }

    pub fn new_reflection() -> Self {
        Self {
            prompt: "In your own words, what will you do differently?".into(),
            points: 0.0,
            choices: vec![],
            feedback_correct: "Thank you for your answer.".into(),
            feedback_incorrect: "Thank you for your answer.".into(),
            style: QuestionStyle::Reflection { min_words: 0 },
            ..Self::new_single()
        }
    }

    pub fn new_dictation() -> Self {
        Self {
            prompt: "Listen, then type what you hear".into(),
            choices: vec![],
            style: QuestionStyle::Dictation { asset: None, text: "Safety first.".into(), max_plays: 0 },
            ..Self::new_single()
        }
    }

    pub fn new_dropdowns() -> Self {
        Self {
            prompt: "Choose the right word for each blank".into(),
            choices: vec![],
            style: QuestionStyle::Dropdowns { text: "The *sun/moon/star* is at the centre of our *solar system/galaxy*.".into() },
            ..Self::new_single()
        }
    }

    pub fn new_label_picture() -> Self {
        Self {
            prompt: "Drag each label to the right place".into(),
            choices: vec![],
            style: QuestionStyle::LabelPicture { asset: None, alt: String::new(), zones: vec![], distractors: vec![] },
            ..Self::new_single()
        }
    }

    pub fn new_sequence() -> Self {
        Self {
            prompt: "Put these in order".into(),
            choices: vec![],
            style: QuestionStyle::Sequence { items: vec!["First".into(), "Second".into(), "Third".into()] },
            ..Self::new_single()
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Asset {
    pub name: String,
    pub mime: String,
    /// Raw bytes. In a `.brainbend` package these live in `assets/<file>` rather than in the JSON.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bytes: Vec<u8>,
    /// Package member holding the bytes (set when writing a package).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
}

/// What kind of lesson a project is. The aliases read files written before the types were renamed
/// or removed (the short "micro" kinds are now ordinary lessons).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectKind {
    /// A deck-style lesson of slides with layers, triggers and quizzes.
    #[default]
    #[serde(alias = "Media", alias = "MicroMediaLesson", alias = "Widget")]
    MediaLesson,
    /// A scrolling, web-style lesson built from content blocks.
    #[serde(alias = "WebCourse", alias = "MicroWebLesson", alias = "Microcourse")]
    WebLesson,
}

impl ProjectKind {
    pub fn label(self) -> &'static str {
        match self {
            ProjectKind::MediaLesson => "Media Lesson",
            ProjectKind::WebLesson => "Web Lesson",
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Project {
    pub format: String,
    #[serde(default)]
    pub kind: ProjectKind,
    pub name: String,
    pub slides: Vec<Slide>,
    #[serde(default)]
    pub scenes: Vec<Scene>,
    pub assets: BTreeMap<String, Asset>,
    #[serde(default)]
    pub variables: Vec<Variable>,
    /// The author's brand colours, offered first in every colour menu. Empty until the author makes one.
    #[serde(default)]
    pub palette: Vec<Rgba>,
    /// Colours picked lately that are not in the palette, newest first.
    #[serde(default)]
    pub recent_colors: Vec<Rgba>,
    /// Percent of quiz points needed to pass.
    #[serde(default = "default_pass_mark")]
    pub pass_mark: u32,
    /// Show the player's Previous/Next bar (only appears when there are several slides).
    #[serde(default = "default_true")]
    pub show_nav: bool,
    /// Add a slide menu (table of contents) to the player's bar.
    #[serde(default)]
    pub show_menu: bool,
    /// Let learners jump to any slide from the menu; otherwise they cannot skip past an
    /// unanswered question.
    #[serde(default)]
    pub menu_free: bool,
    /// Show a progress bar (share of slides visited) in the player's bar.
    #[serde(default = "default_true")]
    pub show_progress: bool,
    /// How the published player looks and what it offers.
    #[serde(default)]
    pub player: PlayerSettings,
    /// Links and files listed in the player's Notes tab.
    #[serde(default)]
    pub resources: Vec<Resource>,
    /// Terms listed in the player's Notes tab.
    #[serde(default)]
    pub glossary: Vec<GlossaryTerm>,
    /// Colours and fonts new content starts from.
    #[serde(default)]
    pub theme: Theme,
    #[serde(default)]
    pub master: Master,
    #[serde(default)]
    pub layouts: Vec<SlideLayout>,
    /// Import diagnostics (unsupported features, assumptions).
    pub notes: Vec<String>,
    /// Words the spell checker should accept (added by the author).
    #[serde(default)]
    pub custom_words: Vec<String>,
    /// The content of a Web Lesson (`None` for slide-based projects).
    #[serde(default)]
    pub lesson: Option<lesson::WebLesson>,
}

fn default_pass_mark() -> u32 {
    80
}

fn default_true() -> bool {
    true
}

/// Written format. Files from before the rename (`storyapp.project/1`, and `/0`: plain JSON with inline
/// bytes) are still readable and are rewritten with this id when saved.
pub const FORMAT_ID: &str = "brainbend.project/1";
const LEGACY_FORMAT_IDS: [&str; 2] = ["storyapp.project/1", "storyapp.project/0"];
const MAX_PACKAGE_ENTRY: u64 = 512 * 1024 * 1024;
const MAX_PACKAGE_TOTAL: u64 = 2 * 1024 * 1024 * 1024;

impl Project {
    /// The most recent colours kept.
    pub const RECENT_COLORS: usize = 10;
    /// The most colours a palette holds.
    pub const PALETTE_MAX: usize = 24;

    /// Remember a colour the author just used (opaque colours only matter: the colour, not its see-through amount).
    pub fn note_color(&mut self, c: Rgba) {
        let c = Rgba(c.0, c.1, c.2, 255);
        if self.palette.contains(&c) { return; }
        self.recent_colors.retain(|r| *r != c);
        self.recent_colors.insert(0, c);
        self.recent_colors.truncate(Self::RECENT_COLORS);
    }

    /// Add a colour to the palette (once); it leaves the recent colours.
    pub fn add_to_palette(&mut self, c: Rgba) {
        let c = Rgba(c.0, c.1, c.2, 255);
        if self.palette.contains(&c) || self.palette.len() >= Self::PALETTE_MAX { return; }
        self.palette.push(c);
        self.recent_colors.retain(|r| *r != c);
    }

    /// The names of every font the project uses: text styles and runs, the theme's heading and body fonts, and the
    /// player's font. Found by walking the saved form, so a new place that holds a font is covered without a change here.
    pub fn fonts_used(&self) -> std::collections::BTreeSet<String> {
        fn walk(v: &serde_json::Value, out: &mut std::collections::BTreeSet<String>) {
            match v {
                serde_json::Value::Object(m) => {
                    for (k, x) in m {
                        if matches!(k.as_str(), "font_family" | "font" | "heading_font" | "body_font") {
                            if let Some(name) = x.as_str().filter(|n| !n.trim().is_empty()) { out.insert(name.to_string()); }
                        }
                        walk(x, out);
                    }
                }
                serde_json::Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
                _ => {}
            }
        }
        let mut out = std::collections::BTreeSet::new();
        if let Ok(v) = serde_json::to_value(self) { walk(&v, &mut out); }
        out
    }

    /// Asset keys that objects (media, captions) still reference.
    pub fn referenced_assets(&self) -> std::collections::BTreeSet<&str> {
        let mut out = std::collections::BTreeSet::new();
        let layout_objects = self.layouts.iter().flat_map(|l| l.objects.iter());
        let slide_objects = self.slides.iter().flat_map(|s| s.layers.iter()).flat_map(|l| l.objects.iter());
        for o in slide_objects.chain(layout_objects).chain(self.master.objects.iter()) {
            if let Some(a) = &o.asset { out.insert(a.as_str()); }
            if let Some(a) = &o.captions { out.insert(a.as_str()); }
            if let Some(a) = &o.poster { out.insert(a.as_str()); }
            // Pictures shown by an object's states, and a slider's thumb.
            out.extend(o.states.iter().filter_map(|st| st.asset.as_deref()));
            if let Kind::Slider(Slider { thumb: Some(a), .. }) = &o.kind { out.insert(a.as_str()); }
        }
        if let Some(a) = &self.player.logo { out.insert(a.as_str()); }
        if let Some(a) = &self.player.banner { out.insert(a.as_str()); }
        out.extend(self.resources.iter().filter_map(|r| r.asset.as_deref()));
        if let Some(l) = &self.lesson {
            for (_, _, b) in l.blocks() { out.extend(b.kind.assets()); }
            if let Some(a) = l.cover.as_ref().and_then(|c| c.image.as_deref()) { out.insert(a); }
        }
        for s in &self.slides {
            if let SlideKind::Question(q) = &s.kind {
                if let Some(a) = q.style.asset() { out.insert(a); }
            }
        }
        let backgrounds = self.slides.iter().filter_map(|s| s.background.as_ref())
            .chain(self.layouts.iter().filter_map(|l| l.background.as_ref()))
            .chain(self.master.background.as_ref());
        for b in backgrounds {
            if let Some(a) = b.asset() { out.insert(a); }
        }
        out
    }

    /// Drop assets nothing references (e.g. after deleting images). Returns how many were removed.
    pub fn prune_unused_assets(&mut self) -> usize {
        let keep: std::collections::BTreeSet<String> = self.referenced_assets().into_iter().map(String::from).collect();
        let before = self.assets.len();
        self.assets.retain(|k, _| keep.contains(k));
        before - self.assets.len()
    }

    pub fn new(name: &str) -> Self {
        Self { format: FORMAT_ID.into(), name: name.into(), pass_mark: 80, show_nav: true, show_progress: true, ..Default::default() }
    }

    pub fn to_json(&self) -> serde_json::Result<Vec<u8>> {
        serde_json::to_vec_pretty(self)
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        let p: Project = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if p.format != FORMAT_ID && !LEGACY_FORMAT_IDS.contains(&p.format.as_str()) {
            return Err(format!("unsupported project format {:?}", p.format));
        }
        Ok(p)
    }

    /// Make a loaded project safe to edit: at least one slide, and every slide has a base layer.
    /// (Hand-edited or damaged files can lack these; the editor indexes `layers[0]` freely.)
    /// Make sure there is at least one scene, every slide names an existing scene, and slides are
    /// ordered by scene (keeping their order within a scene).
    pub fn normalize_scenes(&mut self) {
        if self.scenes.is_empty() {
            self.scenes.push(Scene { id: "scene-1".into(), name: "Scene 1".into(), bank: None });
        }
        let first = self.scenes[0].id.clone();
        for s in self.slides.iter_mut() {
            if !self.scenes.iter().any(|c| c.id == s.scene) { s.scene = first.clone(); }
        }
        let scenes = &self.scenes;
        self.slides.sort_by_key(|s| scenes.iter().position(|c| c.id == s.scene).unwrap_or(0));
    }

    /// Indices of the slides in scene `scene` (an index into `scenes`).
    pub fn scene_slides(&self, scene: usize) -> Vec<usize> {
        let Some(sc) = self.scenes.get(scene) else { return vec![] };
        (0..self.slides.len()).filter(|&i| self.slides[i].scene == sc.id).collect()
    }

    pub fn sanitize(&mut self) {
        if self.slides.is_empty() {
            self.slides.push(Slide::new("slide-1", "Slide 1", 1920.0, 1080.0));
        }
        self.normalize_scenes();
        self.ensure_layouts();
        for s in self.slides.iter_mut() {
            if s.layers.is_empty() {
                s.layers.push(Layer { id: format!("{}-base", s.id), name: "Base layer".into(), objects: vec![], duration_ms: None, modal: true, hide_others: false, pause_base: false, group_names: Default::default(), focus_order: vec![] });
            }
        }
    }

    /// Serialize as a `.brainbend` package: a ZIP with `project.json` and one `assets/<n>` member
    /// per asset, so media is stored as bytes instead of JSON numbers.
    pub fn to_package(&self) -> Result<Vec<u8>, String> {
        use std::io::Write;
        let mut meta = self.clone();
        meta.format = FORMAT_ID.into();
        let mut members: Vec<(String, Vec<u8>)> = Vec::new();
        for (i, a) in meta.assets.values_mut().enumerate() {
            let file = format!("assets/{i}");
            members.push((file.clone(), std::mem::take(&mut a.bytes)));
            a.file = Some(file);
        }
        let json = serde_json::to_vec_pretty(&meta).map_err(|e| e.to_string())?;
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut buf);
            let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            z.start_file("project.json", opts).map_err(|e| e.to_string())?;
            z.write_all(&json).map_err(|e| e.to_string())?;
            // Images are already compressed; store them as-is.
            let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
            for (name, bytes) in members {
                z.start_file(name, stored).map_err(|e| e.to_string())?;
                z.write_all(&bytes).map_err(|e| e.to_string())?;
            }
            z.finish().map_err(|e| e.to_string())?;
        }
        Ok(buf.into_inner())
    }

    /// Write the project as a package straight to `w`, one asset at a time, without making a second copy of the media
    /// (`to_package` copies every asset, and builds the whole package in memory, so it needs several times the size of
    /// the project). `&mut self` is only so the assets can be set aside while the rest is copied; they are put back
    /// before anything that can fail.
    pub fn write_package<W: std::io::Write + std::io::Seek>(&mut self, w: W) -> Result<(), String> {
        use std::io::Write;
        let assets = std::mem::take(&mut self.assets);
        let mut meta = self.clone();
        self.assets = assets;
        meta.format = FORMAT_ID.into();
        let mut members: Vec<(String, &Asset)> = Vec::new();
        for (i, (key, a)) in self.assets.iter().enumerate() {
            let file = format!("assets/{i}");
            meta.assets.insert(key.clone(), Asset { name: a.name.clone(), mime: a.mime.clone(), bytes: Vec::new(), file: Some(file.clone()) });
            members.push((file, a));
        }
        let json = serde_json::to_vec_pretty(&meta).map_err(|e| e.to_string())?;
        let mut z = zip::ZipWriter::new(w);
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        z.start_file("project.json", opts).map_err(|e| e.to_string())?;
        z.write_all(&json).map_err(|e| e.to_string())?;
        for (name, asset) in members {
            // Media is already compressed; store it as it is.
            let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored).large_file(asset.bytes.len() as u64 > u32::MAX as u64);
            z.start_file(name, stored).map_err(|e| e.to_string())?;
            z.write_all(&asset.bytes).map_err(|e| e.to_string())?;
        }
        z.finish().map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Read either a package (ZIP) or the legacy plain-JSON form.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        use std::io::Read;
        if !bytes.starts_with(b"PK") {
            return Self::from_json(bytes);
        }
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| format!("not a BrainBend Studio package: {e}"))?;
        let mut read = |name: &str| -> Result<Vec<u8>, String> {
            let f = z.by_name(name).map_err(|_| format!("package is missing {name}"))?;
            if f.size() > MAX_PACKAGE_ENTRY {
                return Err(format!("{name} is too large"));
            }
            let mut out = Vec::new();
            f.take(MAX_PACKAGE_ENTRY + 1).read_to_end(&mut out).map_err(|e| e.to_string())?;
            if out.len() as u64 > MAX_PACKAGE_ENTRY {
                return Err(format!("{name} is too large"));
            }
            Ok(out)
        };
        let mut project = Self::from_json(&read("project.json")?)?;
        let mut total = 0u64;
        for asset in project.assets.values_mut() {
            if let Some(file) = asset.file.clone() {
                if !file.starts_with("assets/") || file.contains("..") {
                    return Err(format!("unsafe asset path {file:?}"));
                }
                asset.bytes = read(&file)?;
                total += asset.bytes.len() as u64;
                if total > MAX_PACKAGE_TOTAL {
                    return Err("package exceeds size limits".into());
                }
            }
        }
        Ok(project)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fonts_used_cover_text_runs_the_theme_and_the_player() {
        let mut p = Project::new("t");
        p.slides.push(Slide::new("s", "S", 100.0, 100.0));
        let mut o = Object::new("a", "A", Kind::Rect, 0.0, 0.0, 10.0, 10.0);
        o.style.font_family = "Lora".into();
        p.slides[0].layers[0].objects.push(o);
        p.player.font = "Poppins".into();
        p.theme.heading_font = "Oswald".into();
        let used = p.fonts_used();
        for f in ["Lora", "Poppins", "Oswald"] { assert!(used.contains(f), "{f}"); }
        assert!(!used.contains("Nothing"));
    }

    #[test]
    fn the_players_logo_and_banner_count_as_used_assets_and_are_not_pruned() {
        let mut p = Project::new("t");
        p.slides.push(Slide::new("s", "S", 100.0, 100.0));
        for k in ["logo", "banner", "spare"] {
            p.assets.insert(k.into(), Asset { name: k.into(), mime: "image/png".into(), bytes: vec![1], file: None });
        }
        p.player.logo = Some("logo".into());
        p.player.banner = Some("banner".into());
        assert_eq!(p.referenced_assets().into_iter().collect::<Vec<_>>(), ["banner", "logo"]);
        assert_eq!(p.prune_unused_assets(), 1, "only the picture nothing uses goes");
        assert!(p.assets.contains_key("banner") && p.assets.contains_key("logo") && !p.assets.contains_key("spare"));
    }

    #[test]
    fn state_pictures_and_slider_thumbs_count_as_used_assets() {
        let mut p = Project::new("t");
        p.slides.push(Slide::new("s", "S", 100.0, 100.0));
        let mut pic = Object::new("p", "Pic", Kind::Image, 0.0, 0.0, 10.0, 10.0);
        pic.asset = Some("front".into());
        let st = |asset: &str| ObjectState { shine: None, gradient: None, name: asset.into(), rich: None, fill: None, stroke: None, shadow: None, text: String::new(), style: pic.style.clone(), asset: Some(asset.into()), size: None };
        pic.states = vec![st("front"), st("side")];
        let slider = Object::new("sl", "Slider", Kind::Slider(Slider { thumb: Some("knob".into()), ..Slider::new() }), 0.0, 20.0, 100.0, 20.0);
        p.slides[0].layers[0].objects.extend([pic, slider]);
        for k in ["front", "side", "knob", "spare"] {
            p.assets.insert(k.into(), Asset { name: k.into(), mime: "image/png".into(), bytes: vec![1], file: None });
        }
        assert_eq!(p.referenced_assets().into_iter().collect::<Vec<_>>(), ["front", "knob", "side"]);
        assert_eq!(p.prune_unused_assets(), 1, "only the unused picture goes");
        assert!(p.assets.contains_key("side") && p.assets.contains_key("knob"));
    }

    #[test]
    fn a_dial_turns_through_its_sweep_with_the_middle_of_the_range_upright() {
        let d = Dial { variable: "v".into(), min: 1.0, max: 17.0, step: 1.0, sweep: 180.0, pivot: (0.0, 0.0) };
        assert_eq!(d.angle(9.0), 0.0);
        assert_eq!((d.angle(1.0), d.angle(17.0)), (-90.0, 90.0));
        // Values outside the range stop at its ends.
        assert_eq!(d.angle(40.0), 90.0);
        // Turning back gives the value, snapped to the step and held at the ends.
        assert_eq!(d.value_at(0.0), 9.0);
        assert_eq!(d.value_at(-90.0), 1.0);
        assert_eq!(d.value_at(-170.0), 1.0, "past the end stays at the end");
        assert_eq!(d.value_at(11.0), 10.0, "11 degrees is 9.97, which snaps to 10");
        // A full circle wraps instead of clamping.
        let full = Dial { sweep: 360.0, ..d.clone() };
        assert_eq!(full.value_at(-180.0), 1.0);
        assert_eq!(full.value_at(179.0), 17.0);
        assert_eq!(full.value_at(181.0), 1.0, "just past straight down wraps round to the start");
    }

    #[test]
    fn a_slider_maps_its_track_to_its_range_in_whole_steps() {
        let s = Slider { min: 0.0, max: 10.0, step: 2.5, ..Slider::new() };
        assert_eq!((s.fraction(0.0), s.fraction(5.0), s.fraction(10.0), s.fraction(99.0)), (0.0, 0.5, 1.0, 1.0));
        assert_eq!(s.value_at(0.3), 2.5, "3.0 snaps to the nearest step");
        assert_eq!(s.value_at(1.5), 10.0, "past the end stays at the end");
        let odd = Slider { min: 0.0, max: 10.0, step: 4.0, ..Slider::new() };
        assert_eq!(odd.value_at(1.0), 10.0, "10 is not a multiple of 4 but is the end of the range");
        let flat = Slider { min: 5.0, max: 5.0, ..Slider::new() };
        assert_eq!(flat.fraction(5.0), 0.0);
    }

    #[test]
    fn a_shape_of_picture_is_cropped_from_the_centre_or_toward_a_focus() {
        // A 400 x 200 picture to a square: half the width goes, split by the focus.
        let c = crop_for_ratio(400.0, 200.0, 1.0, 0.5, 0.5).unwrap();
        assert!((c[0] - 0.25).abs() < 1e-5 && (c[2] - 0.25).abs() < 1e-5 && c[1] == 0.0 && c[3] == 0.0, "{c:?}");
        let left = crop_for_ratio(400.0, 200.0, 1.0, 0.0, 0.5).unwrap();
        assert!(left[0] == 0.0 && (left[2] - 0.5).abs() < 1e-5, "the left part stays: {left:?}");
        // Taller than the picture: the height goes.
        let wide = crop_for_ratio(400.0, 400.0, 2.0, 0.5, 1.0).unwrap();
        assert!(wide[0] == 0.0 && wide[2] == 0.0 && (wide[1] - 0.5).abs() < 1e-5 && wide[3] == 0.0, "the bottom part stays: {wide:?}");
        assert_eq!(crop_for_ratio(400.0, 200.0, 2.0, 0.5, 0.5), None, "already that shape");
        assert_eq!(crop_for_ratio(0.0, 200.0, 2.0, 0.5, 0.5), None);
        // And back again.
        let (ratio, fx, fy) = crop_shape(400.0, 200.0, left);
        assert!((ratio - 1.0).abs() < 1e-4 && fx == 0.0 && fy == 0.5, "{ratio} {fx} {fy}");
        let (ratio, _, fy) = crop_shape(400.0, 400.0, wide);
        assert!((ratio - 2.0).abs() < 1e-4 && fy == 1.0);
    }

    #[test]
    fn picture_shapes_are_polygons_inside_the_frame_and_know_their_powerpoint_names() {
        for shape in PictureShape::ALL {
            let pts = shape.polygon(400.0, 300.0);
            assert!(pts.len() >= 3, "{shape:?}");
            assert!(pts.iter().all(|p| p.0 >= -1e-4 && p.0 <= 1.0001 && p.1 >= -1e-4 && p.1 <= 1.0001), "{shape:?} stays in the frame: {pts:?}");
            assert_eq!(PictureShape::from_preset(shape.preset()), Some(shape));
            assert!(!shape.label().is_empty());
        }
        assert_eq!(PictureShape::from_preset("rect"), None);
        // The oval reaches the middle of each side; the star has ten points; a rounded corner is round for a wide frame.
        let oval = PictureShape::Oval.polygon(100.0, 100.0);
        assert!(oval.iter().any(|p| (p.0 - 0.5).abs() < 1e-4 && p.1.abs() < 1e-4));
        assert_eq!(PictureShape::Star.polygon(1.0, 1.0).len(), 10);
        let r = PictureShape::Rounded.polygon(600.0, 200.0);
        let corner = r.iter().find(|p| p.1 < 0.01 && p.0 > 0.9).copied().unwrap();
        assert!(corner.0 < 1.0 - 0.01, "the top-right corner is cut: {corner:?}");
        let (dx, dy) = ((1.0 - r.iter().map(|p| p.0).fold(0.0f32, f32::max)).abs(), 0.0);
        assert!(dx < 1e-4 && dy == 0.0, "the sides still reach the frame");
    }

    #[test]
    fn crop_layout_places_the_whole_picture_around_the_frame() {
        // Nothing cropped: the picture is the frame.
        assert_eq!(crop_layout([0.0; 4]), [0.0, 0.0, 1.0, 1.0]);
        // 10% off each side: the picture is 1.25 frames wide and starts an eighth of a frame to the left.
        let [x, y, w, h] = crop_layout([0.1, 0.0, 0.1, 0.0]);
        assert!((x + 0.125).abs() < 1e-6 && y == 0.0 && (w - 1.25).abs() < 1e-6 && h == 1.0);
        // Padding (negative crop) shrinks the picture inside the frame.
        let [x, _, w, _] = crop_layout([-0.25, 0.0, -0.25, 0.0]);
        assert!((x - 0.1667).abs() < 1e-3 && (w - 0.6667).abs() < 1e-3);
    }

    #[test]
    fn motion_path_presets_start_at_the_origin_and_curves_pass_through_their_points() {
        for k in PathPreset::ALL {
            let m = MotionPath::preset(k);
            assert!(m.points.len() >= 2 && m.points[0].0.abs() < 0.01 && m.points[0].1.abs() < 0.01, "{k:?} starts at 0,0");
            let s = m.sampled(8);
            assert!(s.len() >= m.points.len());
            let (a, b) = (s[s.len() - 1], m.points[m.points.len() - 1]);
            assert!((a.0 - b.0).abs() < 0.01 && (a.1 - b.1).abs() < 0.01, "{k:?} ends at its last point");
        }
        let circle = MotionPath::preset(PathPreset::Circle);
        let end = circle.points[circle.points.len() - 1];
        assert!(end.0.abs() < 0.01 && end.1.abs() < 0.01, "a circle comes back to where it began");
        let arc = MotionPath::preset(PathPreset::Arc).sampled(10);
        assert!(arc.iter().any(|p| (p.0 - 150.0).abs() < 8.0 && p.1 < -60.0), "the arc rises in the middle");
    }

    #[test]
    fn audio_and_label_questions_keep_their_assets_and_are_checked() {
        let mut p = Project::new("q");
        for k in ["clip", "pic", "orphan"] { p.assets.insert(k.into(), Asset { name: k.into(), mime: "audio/mpeg".into(), bytes: vec![1], file: None }); }
        let mut d = Question::new_dictation();
        d.style = QuestionStyle::Dictation { asset: Some("clip".into()), text: "hi".into(), max_plays: 0 };
        let mut l = Question::new_label_picture();
        l.style = QuestionStyle::LabelPicture { asset: Some("pic".into()), alt: "x".into(), zones: vec![LabelZone { region: HotRegion { x: 0.1, y: 0.1, w: 0.2, h: 0.2, ellipse: false }, label: "A".into() }], distractors: vec![] };
        for (i, q) in [d, l, Question::new_likert(), Question::new_reflection(), Question::new_dropdowns()].into_iter().enumerate() {
            let mut s = Slide::new(&format!("s{i}"), "s", 100.0, 100.0);
            s.kind = SlideKind::Question(q);
            p.slides.push(s);
        }
        assert_eq!(p.prune_unused_assets(), 1, "only the unused clip goes");
        assert!(p.check().iter().all(|i| !i.message.contains("dictation") && !i.message.contains("labelling")), "{:?}", p.check());
        assert_eq!(Question::new_likert().points, 0.0, "surveys are not graded");
        // A survey total must point at a number variable.
        let mut l = Question::new_likert();
        if let QuestionStyle::Likert { total_variable, .. } = &mut l.style { *total_variable = Some("gone".into()); }
        let mut q = p.clone();
        q.slides[0].kind = SlideKind::Question(l);
        assert!(q.check().iter().any(|i| i.message.contains("keeps its total")), "{:?}", q.check());
        // A dropdown blank needs a wrong option; a dictation needs audio.
        p.slides[4].kind = SlideKind::Question(Question { style: QuestionStyle::Dropdowns { text: "only *one*".into() }, ..Question::new_dropdowns() });
        p.slides[0].kind = SlideKind::Question(Question::new_dictation());
        let msgs: Vec<String> = p.check().into_iter().map(|i| i.message).collect();
        assert!(msgs.iter().any(|m| m.contains("drop-down blank needs")) && msgs.iter().any(|m| m.contains("no audio clip")), "{msgs:?}");
    }
    #[test]
    fn question_banks_round_trip_and_old_scenes_have_none() {
        let mut p = Project::new("b");
        p.slides.push(Slide::new("s", "s", 100.0, 100.0));
        p.sanitize();
        p.scenes[0].bank = Some(Bank { draw: 3, shuffle: true });
        let back = Project::from_json(&p.to_json().unwrap()).unwrap();
        assert_eq!(back.scenes[0].bank, Some(Bank { draw: 3, shuffle: true }));
        let old: Scene = serde_json::from_str(r#"{"id":"a","name":"A"}"#).unwrap();
        assert_eq!(old.bank, None);
    }
    #[test]
    fn hotspot_regions_contain_points_and_their_picture_is_kept() {
        let r = HotRegion { x: 0.2, y: 0.2, w: 0.4, h: 0.2, ellipse: false };
        assert!(r.contains(0.3, 0.3) && !r.contains(0.7, 0.3) && !r.contains(0.3, 0.5));
        let e = HotRegion { ellipse: true, ..r };
        assert!(e.contains(0.4, 0.3), "the centre");
        assert!(!e.contains(0.21, 0.21), "the corner of the box is outside an ellipse");
        let mut p = Project::new("h");
        p.assets.insert("pic".into(), Asset { name: "p.png".into(), mime: "image/png".into(), bytes: vec![1], file: None });
        let mut q = Question::new_hotspot();
        q.style = QuestionStyle::Hotspot { asset: Some("pic".into()), alt: "x".into(), regions: vec![r] };
        let mut s = Slide::new("s", "s", 100.0, 100.0);
        s.kind = SlideKind::Question(q);
        p.slides.push(s);
        assert_eq!(p.prune_unused_assets(), 0, "a hotspot picture counts as used");
        assert!(p.check().iter().all(|i| !i.message.contains("hotspot")));
    }

    use super::*;

    #[test]
    fn json_round_trip() {
        let mut p = Project::new("t");
        p.notes.push("n".into());
        let back = Project::from_json(&p.to_json().unwrap()).unwrap();
        assert_eq!(back.name, "t");
        assert_eq!(back.notes, vec!["n"]);
    }

    #[test]
    fn rejects_unknown_format() {
        assert!(Project::from_json(br#"{"format":"x","name":"","slides":[],"assets":{},"notes":[]}"#).is_err());
    }

    #[test]
    fn prune_removes_only_unreferenced_assets() {
        let mut p = Project::new("x");
        p.sanitize();
        let a = |n: &str| Asset { name: n.into(), mime: "image/png".into(), bytes: vec![1], file: None };
        for k in ["used", "cap", "orphan"] { p.assets.insert(k.into(), a(k)); }
        let mut o = Object::new("o", "o", Kind::Video, 0.0, 0.0, 1.0, 1.0);
        o.asset = Some("used".into());
        o.captions = Some("cap".into());
        p.slides[0].layers[0].objects.push(o);
        assert_eq!(p.prune_unused_assets(), 1);
        assert!(p.assets.contains_key("used") && p.assets.contains_key("cap") && !p.assets.contains_key("orphan"));
    }

    #[test]
    fn scenes_are_created_and_slides_sorted_by_scene() {
        let mut p = Project::new("x");
        p.scenes = vec![Scene { id: "a".into(), name: "A".into(), bank: None }, Scene { id: "b".into(), name: "B".into(), bank: None }];
        for (id, sc) in [("s1", "b"), ("s2", "a"), ("s3", "zzz"), ("s4", "b")] {
            let mut s = Slide::new(id, id, 100.0, 100.0);
            s.scene = sc.into();
            p.slides.push(s);
        }
        p.normalize_scenes();
        let ids: Vec<_> = p.slides.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, vec!["s2", "s3", "s1", "s4"]); // unknown scene falls back to the first
        assert_eq!(p.scene_slides(1), vec![2, 3]);
        let mut q = Project::new("y");
        q.sanitize();
        assert_eq!(q.scenes.len(), 1);
        assert_eq!(q.slides[0].scene, q.scenes[0].id);
    }

    #[test]
    fn sanitize_adds_missing_slide_and_layers() {
        let mut p = Project::new("x");
        p.sanitize();
        assert_eq!(p.slides.len(), 1);
        assert_eq!(p.slides[0].layers.len(), 1);
        p.slides[0].layers.clear();
        p.sanitize();
        assert_eq!(p.slides[0].layers[0].name, "Base layer");
    }

    #[test]
    fn package_round_trip_keeps_media_bytes() {
        let mut p = Project::new("pkg");
        let bytes: Vec<u8> = (0..=255u8).cycle().take(5000).collect();
        p.assets.insert("k1".into(), Asset { name: "a.png".into(), mime: "image/png".into(), bytes: bytes.clone(), file: None });
        let pkg = p.to_package().unwrap();
        assert!(pkg.starts_with(b"PK"));
        // Bytes are stored as a member, not as a JSON number list.
        assert!(pkg.len() < 5000 + 1500);
        let back = Project::from_bytes(&pkg).unwrap();
        assert_eq!(back.assets["k1"].bytes, bytes);
        assert_eq!(back.format, FORMAT_ID);
    }

    #[test]
    fn legacy_json_still_opens() {
        let legacy = br#"{"format":"storyapp.project/0","name":"old","slides":[],"assets":{"k":{"name":"n","mime":"image/png","bytes":[1,2,3]}},"notes":[]}"#;
        let p = Project::from_bytes(legacy).unwrap();
        assert_eq!(p.assets["k"].bytes, vec![1, 2, 3]);
    }

    #[test]
    fn package_with_unsafe_asset_path_is_rejected() {
        use std::io::Write;
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut buf);
            let o = zip::write::SimpleFileOptions::default();
            z.start_file("project.json", o).unwrap();
            z.write_all(br#"{"format":"storyapp.project/1","name":"x","slides":[],"assets":{"k":{"name":"n","mime":"m","file":"../evil"}},"notes":[]}"#).unwrap();
            z.finish().unwrap();
        }
        assert!(Project::from_bytes(&buf.into_inner()).unwrap_err().contains("unsafe"));
    }

#[cfg(test)]
mod focus_tests {
    use super::*;

    fn layer(ids: &[&str]) -> Layer {
        let mut l = Layer { id: "l".into(), name: "L".into(), objects: vec![], duration_ms: None, modal: true, hide_others: false, pause_base: false, group_names: Default::default(), focus_order: vec![] };
        l.objects = ids.iter().map(|id| Object::new(id, id, Kind::Rect, 0.0, 0.0, 10.0, 10.0)).collect();
        l
    }

    #[test]
    fn focus_order_defaults_to_stacking_and_ignores_unknown_ids() {
        let mut l = layer(&["a", "b", "c"]);
        assert_eq!(l.focus_indices(), vec![0, 1, 2]);
        l.focus_order = vec!["c".into(), "gone".into(), "a".into(), "c".into()];
        assert_eq!(l.focus_indices(), vec![2, 0, 1], "listed first (once), the rest after in stacking order");
    }

    #[test]
    fn moving_in_the_focus_order_keeps_every_object() {
        let mut l = layer(&["a", "b", "c"]);
        assert!(l.move_focus("a", 1));
        assert_eq!(l.focus_order, vec!["b", "a", "c"]);
        assert!(l.move_focus("c", -5), "clamped to the front");
        assert_eq!(l.focus_order, vec!["c", "b", "a"]);
        assert!(!l.move_focus("c", -1), "already first");
        assert!(!l.move_focus("zzz", 1));
        l.objects.push(Object::new("d", "d", Kind::Rect, 0.0, 0.0, 1.0, 1.0));
        assert_eq!(l.focus_indices(), vec![2, 1, 0, 3], "a new object comes last");
    }
}

#[cfg(test)]
mod marker_tests {
    use super::*;

    #[test]
    fn a_marker_card_sits_above_when_it_fits_and_stays_on_the_slide() {
        let slide = (1920.0, 1080.0);
        let (left, top, notch, above) = MarkerInfo::card_at((900.0, 600.0, 120.0, 120.0), slide, (640.0, 340.0));
        assert_eq!((left, top, above), (640.0, 242.0, true), "centred above, with a gap of 18");
        assert_eq!(notch, 320.0, "the pointer is over the marker's middle");
        let (left, top, _, above) = MarkerInfo::card_at((1700.0, 20.0, 120.0, 120.0), slide, (640.0, 340.0));
        assert_eq!((left, top, above), (1272.0, 158.0, false), "no room above: below, and pulled back onto the slide");
        let (left, _, notch, _) = MarkerInfo::card_at((0.0, 600.0, 120.0, 120.0), slide, (640.0, 340.0));
        assert_eq!(left, 8.0);
        assert_eq!(notch, 52.0, "the pointer stays over the marker (its middle, 60, minus the card's left, 8)");
    }

    #[test]
    fn colours_typed_by_hand_are_understood() {
        let blue = Some(Rgba(79, 129, 189, 255));
        for t in ["#4F81BD", "4f81bd", " #4f81bd ", "79,129,189", "79, 129, 189", "rgb(79,129,189)", "RGB(79, 129, 189)"] { assert_eq!(Rgba::parse(t), blue, "{t}"); }
        assert_eq!(Rgba::parse("#48b"), Some(Rgba(68, 136, 187, 255)));
        for t in ["", "#12", "#12345", "#gg0000", "300,0,0", "1,2", "rgb(1,2,3,4)", "blue"] { assert_eq!(Rgba::parse(t), None, "{t}"); }
        assert_eq!(Rgba(79, 129, 189, 255).hex(), "#4F81BD");
    }

    #[test]
    fn recent_colours_skip_the_palette_and_the_palette_takes_from_them() {
        let mut p = Project::new("t");
        let (a, b, c) = (Rgba(1, 2, 3, 255), Rgba(4, 5, 6, 255), Rgba(7, 8, 9, 255));
        p.note_color(a);
        p.note_color(b);
        p.note_color(a);
        assert_eq!(p.recent_colors, vec![a, b], "newest first, no repeats");
        p.add_to_palette(b);
        assert_eq!(p.palette, vec![b]);
        assert_eq!(p.recent_colors, vec![a], "a colour in the palette is not also recent");
        p.note_color(b);
        assert_eq!(p.recent_colors, vec![a], "using a palette colour adds nothing to the recent ones");
        p.add_to_palette(b);
        assert_eq!(p.palette.len(), 1, "once");
        for i in 0..40u8 { p.note_color(Rgba(100, i, 0, 255)); }
        assert_eq!(p.recent_colors.len(), Project::RECENT_COLORS);
        for i in 0..40u8 { p.add_to_palette(Rgba(0, 100, i, 255)); }
        assert_eq!(p.palette.len(), Project::PALETTE_MAX);
        p.note_color(c);
        assert_eq!(p.recent_colors[0], c);
        assert!(Project::from_json(&p.to_json().unwrap()).unwrap().palette.len() == Project::PALETTE_MAX);
    }
}
}

#[cfg(test)]
mod object_effects_tests {
    use super::*;

    #[test]
    fn glow_soft_edges_and_reflection_round_trip_and_older_objects_have_none() {
        let mut o = Object::new("b", "Button", Kind::Rect, 0.0, 0.0, 100.0, 40.0);
        assert!(o.glow.is_none() && o.reflection.is_none() && o.soft_edge == 0.0);
        o.glow = Some(Glow { radius: 12.0, color: Rgba(255, 200, 0, 200) });
        o.soft_edge = 6.0;
        o.reflection = Some(Reflection { size: 0.5, opacity: 0.4, gap: 3.0 });
        let back: Object = serde_json::from_value(serde_json::to_value(&o).unwrap()).unwrap();
        assert_eq!((back.glow, back.soft_edge, back.reflection), (o.glow, o.soft_edge, o.reflection));
        let mut v = serde_json::to_value(&o).unwrap();
        for k in ["glow", "soft_edge", "reflection"] { v.as_object_mut().unwrap().remove(k); }
        let old: Object = serde_json::from_value(v).unwrap();
        assert!(old.glow.is_none() && old.reflection.is_none() && old.soft_edge == 0.0, "a project saved before these effects opens without them");
    }
}

#[cfg(test)]
mod streamed_package_tests {
    use super::*;

    #[test]
    fn a_streamed_package_reads_back_like_the_buffered_one_and_leaves_the_project_whole() {
        let mut p = Project::new("Streamed");
        p.assets.insert("a".into(), Asset { name: "a.bin".into(), mime: "video/mp4".into(), bytes: (0..5000u32).map(|i| (i % 251) as u8).collect(), file: None });
        p.assets.insert("b".into(), Asset { name: "b.png".into(), mime: "image/png".into(), bytes: vec![1, 2, 3], file: None });
        let buffered = Project::from_bytes(&p.to_package().unwrap()).unwrap();
        let mut out = std::io::Cursor::new(Vec::new());
        p.write_package(&mut out).unwrap();
        assert_eq!(p.assets["a"].bytes.len(), 5000, "the project keeps its media");
        let back = Project::from_bytes(&out.into_inner()).unwrap();
        assert_eq!(back.name, "Streamed");
        for k in ["a", "b"] {
            assert_eq!(back.assets[k].bytes, p.assets[k].bytes, "{k}");
            assert_eq!(back.assets[k].bytes, buffered.assets[k].bytes, "{k}");
            assert_eq!((&back.assets[k].name, &back.assets[k].mime), (&p.assets[k].name, &p.assets[k].mime));
        }
    }
}
