//! Themes, the slide master, layouts and slide backgrounds.
//!
//! A slide inherits from its layout, and a layout from the master: the background (the slide's
//! own, else the layout's, else the master's) and the decorative objects drawn behind the slide's
//! content. Placeholders are different: they are real objects on the slide that remember which
//! layout placeholder they came from, so the layout can restyle them without hiding the content.
use crate::{Action, Align, Kind, Object, Project, Rgba, Slide, TextStyle, VAlign};
use serde::{Deserialize, Serialize};

/// What fills a slide's background.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Background {
    /// A flat colour. (Serialized as a bare colour so older files stay readable.)
    Solid(Rgba),
    /// A linear gradient. `angle` is in degrees clockwise from "towards the top" (CSS style);
    /// stops are `(position 0..1, colour)`.
    Gradient { angle: f32, stops: Vec<(f32, Rgba)> },
    /// A picture stretched to cover the slide (`asset` is a key into `Project::assets`).
    Image { asset: String },
}

impl Background {
    /// A colour that represents the background, for contrast checks and fallbacks.
    pub fn base_color(&self) -> Rgba {
        match self {
            Background::Solid(c) => *c,
            Background::Gradient { stops, .. } => {
                // The average of the stops is a fair stand-in for text contrast.
                let n = stops.len().max(1) as u32;
                let (r, g, b) = stops.iter().fold((0u32, 0u32, 0u32), |a, (_, c)| (a.0 + c.0 as u32, a.1 + c.1 as u32, a.2 + c.2 as u32));
                Rgba((r / n) as u8, (g / n) as u8, (b / n) as u8, 255)
            }
            Background::Image { .. } => Rgba(255, 255, 255, 255),
        }
    }

    pub fn asset(&self) -> Option<&str> {
        match self {
            Background::Image { asset } => Some(asset),
            _ => None,
        }
    }

    /// A two-stop gradient.
    pub fn gradient(angle: f32, a: Rgba, b: Rgba) -> Self {
        Background::Gradient { angle, stops: vec![(0.0, a), (1.0, b)] }
    }
}

pub const THEME_COLOR_NAMES: [&str; 10] =
    ["Text", "Background", "Dark accent", "Light accent", "Accent 1", "Accent 2", "Accent 3", "Accent 4", "Accent 5", "Accent 6"];

/// A palette and a pair of fonts that new content starts from. Objects keep literal colours, so
/// changing the theme does not recolour what already exists.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    /// In the order of `THEME_COLOR_NAMES`.
    pub colors: Vec<Rgba>,
    pub heading_font: String,
    pub body_font: String,
}

impl Default for Theme {
    fn default() -> Self {
        Self::presets().remove(0)
    }
}

impl Theme {
    fn make(name: &str, c: [(u8, u8, u8); 10], heading: &str, body: &str) -> Self {
        Self { name: name.into(), colors: c.iter().map(|&(r, g, b)| Rgba(r, g, b, 255)).collect(), heading_font: heading.into(), body_font: body.into() }
    }

    /// Built-in themes (original palettes).
    pub fn presets() -> Vec<Theme> {
        vec![
            Self::make("BrainBend Studio", [(34, 34, 34), (255, 255, 255), (31, 58, 95), (238, 242, 247), (3, 83, 158), (232, 119, 34), (46, 139, 87), (142, 68, 173), (200, 50, 70), (90, 100, 115)], "Montserrat", "Open Sans"),
            Self::make("Ocean", [(16, 42, 67), (255, 255, 255), (11, 61, 99), (230, 244, 250), (0, 119, 182), (0, 180, 216), (72, 202, 228), (3, 4, 94), (244, 162, 97), (42, 157, 143)], "Montserrat", "Open Sans"),
            Self::make("Forest", [(30, 45, 35), (250, 250, 245), (38, 70, 50), (235, 242, 232), (45, 106, 79), (116, 165, 127), (216, 169, 61), (153, 88, 42), (176, 58, 46), (86, 98, 70)], "Open Sans", "Open Sans"),
            Self::make("Sunset", [(51, 28, 38), (255, 250, 245), (110, 36, 60), (253, 236, 226), (214, 69, 80), (245, 140, 60), (250, 190, 70), (140, 60, 140), (60, 90, 160), (110, 80, 70)], "Montserrat", "Montserrat"),
            Self::make("Slate", [(36, 41, 51), (245, 247, 250), (52, 61, 77), (225, 230, 238), (88, 101, 242), (56, 189, 148), (240, 173, 60), (231, 76, 110), (93, 156, 236), (130, 140, 155)], "Open Sans", "Open Sans"),
            Self::make("Midnight", [(236, 240, 247), (18, 22, 33), (10, 14, 24), (36, 44, 66), (92, 160, 255), (255, 170, 80), (110, 220, 170), (200, 140, 255), (255, 110, 130), (150, 160, 180)], "Montserrat", "Open Sans"),
        ]
    }

    /// The theme with its name set to that of the preset whose colours and fonts it has, else "Custom".
    pub fn named(mut self) -> Self {
        self.name = Self::presets().into_iter().find(|p| p.colors == self.colors && p.heading_font == self.heading_font && p.body_font == self.body_font).map(|p| p.name).unwrap_or_else(|| "Custom".into());
        self
    }

    pub fn color(&self, i: usize) -> Rgba {
        self.colors.get(i).copied().unwrap_or(Rgba(0, 0, 0, 255))
    }

    /// Text colour, page colour.
    pub fn text(&self) -> Rgba { self.color(0) }
    pub fn page(&self) -> Rgba { self.color(1) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaceholderKind {
    Title,
    Subtitle,
    Body,
}

impl PlaceholderKind {
    pub fn label(self) -> &'static str {
        match self {
            PlaceholderKind::Title => "Title",
            PlaceholderKind::Subtitle => "Subtitle",
            PlaceholderKind::Body => "Text",
        }
    }

    /// The prompt shown in an empty placeholder while editing.
    pub fn prompt(self) -> &'static str {
        match self {
            PlaceholderKind::Title => "Click to add title",
            PlaceholderKind::Subtitle => "Click to add subtitle",
            PlaceholderKind::Body => "Click to add text",
        }
    }
}

/// The shapes and background every slide starts from.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Master {
    #[serde(default)]
    pub background: Option<Background>,
    #[serde(default)]
    pub objects: Vec<Object>,
}

/// A page design (Title Slide, Title and Content, ...): placeholders plus optional extra shapes
/// and a background of its own.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SlideLayout {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub background: Option<Background>,
    #[serde(default)]
    pub objects: Vec<Object>,
    /// Draw the master's shapes behind this layout.
    #[serde(default = "yes")]
    pub show_master: bool,
}

fn yes() -> bool {
    true
}

/// A text placeholder object in the theme's fonts.
pub fn placeholder_object(id: &str, kind: PlaceholderKind, theme: &Theme, x: f32, y: f32, w: f32, h: f32) -> Object {
    let mut o = Object::new(id, kind.label(), Kind::Text, x, y, w, h);
    o.placeholder = Some(kind);
    o.text = kind.prompt().into();
    o.margins = [12.0, 8.0, 12.0, 8.0];
    o.style = match kind {
        PlaceholderKind::Title => TextStyle { font_family: theme.heading_font.clone(), size: 72.0, bold: true, italic: false, underline: false, color: theme.text(), align: Align::Left, valign: VAlign::Middle },
        PlaceholderKind::Subtitle => TextStyle { font_family: theme.body_font.clone(), size: 40.0, bold: false, italic: false, underline: false, color: theme.color(9), align: Align::Left, valign: VAlign::Top },
        PlaceholderKind::Body => TextStyle { font_family: theme.body_font.clone(), size: 40.0, bold: false, italic: false, underline: false, color: theme.text(), align: Align::Left, valign: VAlign::Top },
    };
    o
}

impl SlideLayout {
    pub fn new(id: &str, name: &str) -> Self {
        Self { id: id.into(), name: name.into(), background: None, objects: vec![], show_master: true }
    }
}

/// The starter set of layouts for a slide of `w` x `h`.
pub fn default_layouts(theme: &Theme, w: f32, h: f32) -> Vec<SlideLayout> {
    use PlaceholderKind::*;
    let m = w * 0.06;
    let ph = |id: &str, k, x, y, ww, hh| placeholder_object(id, k, theme, x, y, ww, hh);
    let mut title_slide = SlideLayout::new("layout-title", "Title Slide");
    let mut t = ph("layout-title-t", Title, m, h * 0.30, w - 2.0 * m, h * 0.22);
    t.style.size = 96.0;
    t.style.valign = VAlign::Bottom;
    title_slide.objects = vec![t, ph("layout-title-s", Subtitle, m, h * 0.54, w - 2.0 * m, h * 0.14)];

    let mut content = SlideLayout::new("layout-content", "Title and Content");
    content.objects = vec![ph("layout-content-t", Title, m, h * 0.06, w - 2.0 * m, h * 0.16), ph("layout-content-b", Body, m, h * 0.26, w - 2.0 * m, h * 0.62)];

    let mut section = SlideLayout::new("layout-section", "Section Header");
    let mut st = ph("layout-section-t", Title, m, h * 0.36, w - 2.0 * m, h * 0.2);
    st.style.size = 84.0;
    section.objects = vec![st, ph("layout-section-s", Subtitle, m, h * 0.58, w - 2.0 * m, h * 0.12)];

    let mut two = SlideLayout::new("layout-two", "Two Content");
    let half = (w - 3.0 * m) / 2.0;
    two.objects = vec![ph("layout-two-t", Title, m, h * 0.06, w - 2.0 * m, h * 0.16), ph("layout-two-l", Body, m, h * 0.26, half, h * 0.62), ph("layout-two-r", Body, 2.0 * m + half, h * 0.26, half, h * 0.62)];

    let mut title_only = SlideLayout::new("layout-title-only", "Title Only");
    title_only.objects = vec![ph("layout-to-t", Title, m, h * 0.06, w - 2.0 * m, h * 0.16)];

    let blank = SlideLayout::new("layout-blank", "Blank");
    vec![title_slide, content, section, two, title_only, blank]
}

impl Project {
    pub fn layout(&self, id: &str) -> Option<&SlideLayout> {
        self.layouts.iter().find(|l| l.id == id)
    }

    /// Give a project that has no layouts the starter set.
    pub fn ensure_layouts(&mut self) {
        if self.layouts.is_empty() {
            let (w, h) = self.slides.first().map(|s| (s.width, s.height)).unwrap_or((1920.0, 1080.0));
            self.layouts = default_layouts(&self.theme, w, h);
        }
    }

    /// The background `slide` shows: its own, else its layout's, else the master's.
    pub fn resolved_background<'a>(&'a self, slide: &'a Slide) -> Option<&'a Background> {
        slide.background.as_ref().or_else(|| slide.layout.as_deref().and_then(|id| self.layout(id)).and_then(|l| l.background.as_ref())).or(self.master.background.as_ref())
    }

    /// Decorative objects drawn behind `slide`'s own content: the master's shapes (unless the
    /// layout hides them), then the layout's. Placeholders are templates, never drawn behind.
    /// Slides without a layout inherit nothing.
    pub fn backdrop(&self, slide: &Slide) -> Vec<&Object> {
        let Some(id) = slide.layout.as_deref() else { return vec![] };
        if slide.hide_backdrop { return vec![]; }
        let layout = self.layout(id);
        let mut out: Vec<&Object> = vec![];
        if layout.is_none_or(|l| l.show_master) {
            out.extend(self.master.objects.iter().filter(|o| o.placeholder.is_none() && !o.hidden));
        }
        if let Some(l) = layout {
            out.extend(l.objects.iter().filter(|o| o.placeholder.is_none() && !o.hidden));
        }
        out
    }

    /// Give `slide_idx` the layout `layout_id`: placeholders on the slide that match one on the
    /// layout take its position and style (keeping their text), missing ones are added, and
    /// empty leftovers are removed.
    pub fn apply_layout(&mut self, slide_idx: usize, layout_id: &str, new_id: &mut dyn FnMut() -> String) {
        let Some(layout) = self.layout(layout_id).cloned() else { return };
        let slide = &mut self.slides[slide_idx];
        slide.layout = Some(layout.id.clone());
        let Some(base) = slide.layers.first_mut() else { return };
        let mut used = vec![false; base.objects.len()];
        for lo in layout.objects.iter().filter(|o| o.placeholder.is_some()) {
            let found = (0..base.objects.len()).find(|&i| !used[i] && base.objects[i].placeholder == lo.placeholder);
            match found {
                Some(i) => {
                    used[i] = true;
                    let o = &mut base.objects[i];
                    (o.x, o.y, o.w, o.h) = (lo.x, lo.y, lo.w, lo.h);
                    o.style = lo.style.clone();
                    o.margins = lo.margins;
                    o.fill = lo.fill;
                    o.stroke = lo.stroke;
                    o.rich = None;
                }
                None => {
                    let mut o = lo.clone();
                    o.id = new_id();
                    o.text = String::new();
                    base.objects.push(o);
                    used.push(true);
                }
            }
        }
        // Placeholders the new layout has no use for: drop the empty ones, keep content as plain objects.
        let mut i = base.objects.len();
        while i > 0 {
            i -= 1;
            let o = &mut base.objects[i];
            if o.placeholder.is_some() && !used.get(i).copied().unwrap_or(false) {
                if o.text.trim().is_empty() { base.objects.remove(i); } else { o.placeholder = None; }
            }
        }
    }

    /// A copy with everything resolved for publishing: each slide has its final background and
    /// its backdrop objects at the bottom of the base layer, and the master/layouts are gone.
    pub fn flattened(&self) -> Project {
        let mut out = self.clone();
        for (i, s) in out.slides.iter_mut().enumerate() {
            s.background = self.resolved_background(&self.slides[i]).cloned();
            let back: Vec<Object> = self
                .backdrop(&self.slides[i])
                .into_iter()
                .enumerate()
                .map(|(n, o)| {
                    let mut o = o.clone();
                    o.id = format!("bd-{}-{n}", self.slides[i].id);
                    o
                })
                .collect();
            if !back.is_empty() {
                if let Some(base) = s.layers.first_mut() {
                    base.objects.splice(0..0, back);
                }
            }
            s.layout = None;
        }
        out.master = Master::default();
        out.layouts.clear();
        out
    }
}

/// Placeholders of `kind` in the order they appear on `objects`.
fn placeholders(objects: &[Object], kind: PlaceholderKind) -> Vec<usize> {
    (0..objects.len()).filter(|&i| objects[i].placeholder == Some(kind)).collect()
}

/// Carry changes made to a layout's placeholders over to slides that still use the old look.
pub fn propagate(old: &SlideLayout, new: &SlideLayout, slide: &mut Slide) {
    let Some(base) = slide.layers.first_mut() else { return };
    for kind in [PlaceholderKind::Title, PlaceholderKind::Subtitle, PlaceholderKind::Body] {
        let (olds, news) = (placeholders(&old.objects, kind), placeholders(&new.objects, kind));
        let mine = placeholders(&base.objects, kind);
        for (n, &mi) in mine.iter().enumerate() {
            let (Some(&oi), Some(&ni)) = (olds.get(n), news.get(n)) else { continue };
            let (o, nw) = (&old.objects[oi], &new.objects[ni]);
            let so = &mut base.objects[mi];
            // Only placeholders the author has not restyled or moved follow the layout.
            if (so.x, so.y, so.w, so.h) == (o.x, o.y, o.w, o.h) {
                (so.x, so.y, so.w, so.h) = (nw.x, nw.y, nw.w, nw.h);
            }
            if so.style == o.style {
                so.style = nw.style.clone();
                so.margins = nw.margins;
            }
        }
    }
}


impl Project {
    /// A copy holding only the slides at `indices` (in slide order), for previewing part of a project.
    /// Jumps to slides that are not in the copy are dropped so the player never goes missing.
    pub fn subset(&self, indices: &[usize]) -> Project {
        let mut out = self.clone();
        out.slides = self.slides.iter().enumerate().filter(|(i, _)| indices.contains(i)).map(|(_, s)| s.clone()).collect();
        let ids: Vec<String> = out.slides.iter().map(|s| s.id.clone()).collect();
        for s in out.slides.iter_mut() {
            s.triggers.retain(|t| !matches!(&t.action, Action::JumpToSlide(id) if !ids.contains(id)));
        }
        let used: Vec<String> = out.slides.iter().map(|s| s.scene.clone()).collect();
        out.scenes.retain(|sc| used.contains(&sc.id));
        out
    }
}

impl Project {
    /// Switch to `theme`: the master's and layouts' placeholders take its fonts and colours, and
    /// slides whose placeholders still look like their layout's follow. Other objects keep their colours.
    pub fn set_theme(&mut self, theme: Theme) {
        let old_layouts = self.layouts.clone();
        let restyle = |o: &mut Object| {
            let Some(k) = o.placeholder else { return };
            o.style.font_family = if k == PlaceholderKind::Title { theme.heading_font.clone() } else { theme.body_font.clone() };
            o.style.color = if k == PlaceholderKind::Subtitle { theme.color(9) } else { theme.text() };
        };
        self.master.objects.iter_mut().for_each(restyle);
        for l in self.layouts.iter_mut() { l.objects.iter_mut().for_each(restyle); }
        let new_layouts = self.layouts.clone();
        for s in self.slides.iter_mut() {
            let Some(id) = s.layout.clone() else { continue };
            if let (Some(old), Some(new)) = (old_layouts.iter().find(|l| l.id == id), new_layouts.iter().find(|l| l.id == id)) { propagate(old, new, s); }
        }
        self.theme = theme;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> Project {
        let mut p = Project::new("t");
        p.slides.push(Slide::new("s1", "S1", 1920.0, 1080.0));
        p.sanitize();
        p
    }

    #[test]
    fn solid_backgrounds_keep_the_old_file_shape_and_others_round_trip() {
        let solid: Background = serde_json::from_str("[1,2,3,255]").unwrap();
        assert_eq!(solid, Background::Solid(Rgba(1, 2, 3, 255)));
        assert_eq!(serde_json::to_string(&solid).unwrap(), "[1,2,3,255]");
        for b in [Background::gradient(90.0, Rgba(0, 0, 0, 255), Rgba(255, 255, 255, 255)), Background::Image { asset: "a".into() }] {
            let back: Background = serde_json::from_str(&serde_json::to_string(&b).unwrap()).unwrap();
            assert_eq!(back, b);
        }
    }

    #[test]
    fn old_projects_without_layouts_load_with_starter_layouts() {
        let mut p = project();
        assert!(p.layouts.len() >= 5);
        p.layouts.clear();
        let json = p.to_json().unwrap();
        let mut back = Project::from_json(&json).unwrap();
        back.sanitize();
        assert!(back.layout("layout-content").is_some());
    }

    #[test]
    fn a_changed_theme_is_named_for_the_preset_it_matches_or_custom() {
        let base = Theme::presets().remove(1);
        assert_eq!(base.clone().named().name, base.name);
        let mut changed = base.clone();
        changed.colors[4] = Rgba(1, 2, 3, 255);
        assert_eq!(changed.clone().named().name, "Custom");
        changed.colors[4] = base.colors[4];
        assert_eq!(changed.named().name, base.name, "put back, it is the preset again");
    }

    #[test]
    fn slide_follows_layout_background_and_master_shapes() {
        let mut p = project();
        p.master.background = Some(Background::Solid(Rgba(10, 20, 30, 255)));
        p.master.objects.push(Object::new("logo", "Logo", Kind::Rect, 0.0, 0.0, 50.0, 50.0));
        p.layouts[1].background = Some(Background::Solid(Rgba(1, 1, 1, 255)));
        let id = p.layouts[1].id.clone();
        p.slides[0].layout = Some(id.clone());
        assert_eq!(p.resolved_background(&p.slides[0]), Some(&Background::Solid(Rgba(1, 1, 1, 255))));
        assert_eq!(p.backdrop(&p.slides[0]).len(), 1);
        p.slides[0].background = Some(Background::Solid(Rgba(9, 9, 9, 255)));
        assert_eq!(p.resolved_background(&p.slides[0]), Some(&Background::Solid(Rgba(9, 9, 9, 255))));
        p.layouts[1].show_master = false;
        assert!(p.backdrop(&p.slides[0]).is_empty(), "layout can hide master shapes");
        p.layouts[1].show_master = true;
        assert_eq!(p.backdrop(&p.slides[0]).len(), 1);
        p.slides[0].hide_backdrop = true;
        assert!(p.backdrop(&p.slides[0]).is_empty(), "a slide can hide the background graphics itself");
        p.slides[0].layout = None;
        assert!(p.backdrop(&p.slides[0]).is_empty(), "no layout, no inheritance");
    }

    #[test]
    fn applying_a_layout_adds_placeholders_keeps_content_and_drops_empty_leftovers() {
        let mut p = project();
        let mut n = 0;
        let mut ids = || { n += 1; format!("new{n}") };
        p.apply_layout(0, "layout-content", &mut ids);
        let objs = &p.slides[0].layers[0].objects;
        assert_eq!(objs.iter().filter(|o| o.placeholder == Some(PlaceholderKind::Title)).count(), 1);
        assert_eq!(objs.iter().filter(|o| o.placeholder == Some(PlaceholderKind::Body)).count(), 1);
        p.slides[0].layers[0].objects.iter_mut().find(|o| o.placeholder == Some(PlaceholderKind::Title)).unwrap().text = "Hello".into();
        // Title Only: Title keeps its text and moves; the empty Body goes away.
        p.apply_layout(0, "layout-title-only", &mut ids);
        let objs = &p.slides[0].layers[0].objects;
        assert_eq!(objs.len(), 1);
        assert_eq!(objs[0].text, "Hello");
        // Blank: the filled title stays as plain content.
        p.apply_layout(0, "layout-blank", &mut ids);
        let objs = &p.slides[0].layers[0].objects;
        assert_eq!(objs.len(), 1);
        assert!(objs[0].placeholder.is_none());
    }

    #[test]
    fn flattening_bakes_backgrounds_and_backdrop_into_slides() {
        let mut p = project();
        p.master.background = Some(Background::Solid(Rgba(5, 5, 5, 255)));
        p.master.objects.push(Object::new("logo", "Logo", Kind::Rect, 0.0, 0.0, 50.0, 50.0));
        p.slides[0].layout = Some("layout-blank".into());
        p.slides[0].layers[0].objects.push(Object::new("mine", "Mine", Kind::Rect, 0.0, 0.0, 10.0, 10.0));
        let f = p.flattened();
        assert_eq!(f.slides[0].background, Some(Background::Solid(Rgba(5, 5, 5, 255))));
        let ids: Vec<&str> = f.slides[0].layers[0].objects.iter().map(|o| o.id.as_str()).collect();
        assert_eq!(ids, ["bd-s1-0", "mine"], "master shapes sit below the slide's own");
        assert!(f.layouts.is_empty() && f.slides[0].layout.is_none());
    }

    #[test]
    fn a_subset_keeps_chosen_slides_and_drops_jumps_out_of_it() {
        use crate::{Action, Event, Owner, Trigger};
        let mut p = project();
        p.slides.push(Slide::new("s2", "S2", 1920.0, 1080.0));
        p.slides.push(Slide::new("s3", "S3", 1920.0, 1080.0));
        p.slides[0].triggers.push(Trigger { id: "a".into(), owner: Owner::Slide, event: Event::SlideStart, action: Action::JumpToSlide("s2".into()), conditions: vec![] });
        p.slides[0].triggers.push(Trigger { id: "b".into(), owner: Owner::Slide, event: Event::SlideStart, action: Action::JumpToSlide("s3".into()), conditions: vec![] });
        let sub = p.subset(&[0, 1]);
        assert_eq!(sub.slides.len(), 2);
        assert_eq!(sub.slides[0].triggers.len(), 1, "the jump to the missing slide is gone");
        assert_eq!(p.slides.len(), 3, "the original is untouched");
    }

    #[test]
    fn background_pictures_count_as_used_assets() {
        let mut p = project();
        p.assets.insert("bg".into(), crate::Asset { name: "bg.png".into(), mime: "image/png".into(), bytes: vec![1], file: None });
        p.layouts[0].background = Some(Background::Image { asset: "bg".into() });
        assert!(p.referenced_assets().contains("bg"));
        assert_eq!(p.prune_unused_assets(), 0);
    }
}
