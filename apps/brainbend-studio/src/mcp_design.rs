//! The tools that set up and use a lesson's look: its theme (colors and fonts), brand palette, master and layouts.
//!
//! These are what let an AI tool leave a lesson the author can keep editing in the same style: it sets the colors and fonts
//! once, makes layouts with placeholders (the same thing the layout builder makes), and puts each slide on a layout, so a new
//! slide the author adds later follows the same design. Like every tool they run on the app's own thread, after an undo
//! snapshot; the theme, palette, master and layouts are all part of that snapshot.
use super::*;
use crate::master_view::MASTER_SCENE;
use crate::mcp_tools::*;
use brainbend_doc::master::{placeholder_object, Background, PlaceholderKind, SlideLayout, THEME_COLOR_NAMES};
use serde_json::{json, Value};

/// A name with only letters and digits, in lower case, so "Dark accent", "dark_accent" and "DarkAccent" are the same.
fn norm(s: &str) -> String { s.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_ascii_lowercase() }

fn placeholder_kind(s: &str) -> Result<PlaceholderKind, String> {
    Ok(match norm(s).as_str() {
        "heading2" | "title" | "heading" => PlaceholderKind::Heading2,
        "heading3" | "subtitle" => PlaceholderKind::Heading3,
        "heading4" => PlaceholderKind::Heading4,
        "text" | "normaltext" | "body" | "content" => PlaceholderKind::Text,
        _ => return Err(format!("`kind` must be \"Heading 2\" (a slide title), \"Heading 3\" (a subtitle), \"Heading 4\" or \"Normal Text\" (body text), not {s:?}.")),
    })
}

/// A decorative shape asked for, before it has an id.
struct ShapeSpec { name: String, kind: Kind, radius: f32, x: f32, y: f32, w: f32, h: f32, fill: Rgba }

fn required_num(a: &Value, key: &str) -> Result<f32, String> { num_arg(a, key, -20_000.0, 20_000.0)?.ok_or_else(|| format!("`{key}` is needed.")) }

fn shape_specs(a: &Value, key: &str, accent: Rgba) -> Result<Vec<ShapeSpec>, String> {
    let Some(list) = a.get(key).filter(|v| !v.is_null()) else { return Ok(vec![]) };
    let list = list.as_array().ok_or_else(|| format!("`{key}` must be a list of shapes."))?;
    list.iter().enumerate().map(|(i, s)| {
        let at = |e: String| format!("{key}[{i}]: {e}");
        let (kind, label, radius) = match text_arg(s, "shape").map_err(at)?.as_deref() {
            None | Some("rectangle") => (Kind::Rect, "Rectangle", 0.0),
            Some("rounded_rectangle") => (Kind::Rect, "Rounded rectangle", 24.0),
            Some("ellipse") => (Kind::Ellipse, "Ellipse", 0.0),
            Some(other) => return Err(at(format!("`shape` must be rectangle, rounded_rectangle or ellipse, not {other:?}."))),
        };
        let (x, y) = (required_num(s, "x").map_err(at)?, required_num(s, "y").map_err(at)?);
        let (w, h) = (num_arg(s, "width", 1.0, 20_000.0).map_err(at)?.ok_or_else(|| at("`width` is needed.".into()))?, num_arg(s, "height", 1.0, 20_000.0).map_err(at)?.ok_or_else(|| at("`height` is needed.".into()))?);
        Ok(ShapeSpec { name: text_arg(s, "name").map_err(at)?.unwrap_or_else(|| label.to_string()), kind, radius, x, y, w, h, fill: color_arg(s, "fill").map_err(at)?.unwrap_or(accent) })
    }).collect()
}

/// The layout with this id or name (names are matched without regard to case).
fn find_layout(p: &Project, which: &str) -> Result<String, String> {
    if let Some(l) = p.layouts.iter().find(|l| l.id == which) { return Ok(l.id.clone()); }
    if let Some(l) = p.layouts.iter().find(|l| l.name.eq_ignore_ascii_case(which.trim())) { return Ok(l.id.clone()); }
    let names: Vec<String> = p.layouts.iter().map(|l| format!("{:?}", l.name)).collect();
    Err(format!("There is no layout {which:?}. The layouts are: {}. create_layout makes a new one.", if names.is_empty() { "none yet".to_string() } else { names.join(", ") }))
}

fn font_arg(a: &Value, key: &str) -> Result<Option<String>, String> {
    let Some(name) = text_arg(a, key)? else { return Ok(None) };
    if brainbend_fonts::available(&name) { return Ok(Some(name)); }
    let some: Vec<String> = brainbend_fonts::families().into_iter().map(|(n, _)| n).take(25).collect();
    Err(format!("`{key}`: the font {name:?} is not available. Some that are: {}.", some.join(", ")))
}

impl App {
    fn build_shapes(&mut self, specs: Vec<ShapeSpec>) -> Vec<Object> {
        specs.into_iter().map(|s| {
            let mut o = blank_object(self.new_id("obj"), &s.name, s.kind, s.x.round(), s.y.round(), s.w.round(), s.h.round());
            o.radius = s.radius;
            o.fill = Some(s.fill);
            o
        }).collect()
    }


    /// A fingerprint of everything the design is made of.
    fn design_print(&mut self) -> u64 {
        use std::hash::{Hash, Hasher};
        let Some(p) = self.committed() else { return 0 };
        let mut h = std::collections::hash_map::DefaultHasher::new();
        serde_json::to_string(&(&p.theme, &p.palette, &p.master, &p.layouts)).unwrap_or_default().hash(&mut h);
        h.finish()
    }

    /// Remembers the design as it now is, as the AI tool's own work.
    fn design_made(&mut self) { self.mcp.design_seen = Some(self.design_print()); }

    /// What the author has already set up, in words, or None when the design is empty or is still exactly what an AI tool made.
    fn design_of_the_author(&mut self) -> Option<String> {
        let print = self.design_print();
        if self.mcp.design_seen == Some(print) { return None; }
        let p = self.committed()?;
        let stock = Theme::default();
        let mut have: Vec<String> = vec![];
        if !p.palette.is_empty() { have.push(format!("a brand palette of {} colors", p.palette.len())); }
        if p.theme.colors != stock.colors || p.theme.heading_font != stock.heading_font || p.theme.body_font != stock.body_font { have.push(format!("its own theme ({})", p.theme.name)); }
        if !p.master.objects.is_empty() || p.master.background.is_some() { have.push("a master with its own background or shapes".into()); }
        let own: Vec<&str> = p.layouts.iter().filter(|l| !(l.name == "Blank" && l.objects.is_empty())).map(|l| l.name.as_str()).collect();
        if !own.is_empty() { have.push(format!("{} layout{} ({})", own.len(), if own.len() == 1 { "" } else { "s" }, own.join(", "))); }
        if have.is_empty() { None } else { Some(have.join(", ")) }
    }

    /// Lets a change to the design go ahead only on a lesson with no design of the author's, or once the author has agreed to it.
    fn design_gate(&mut self, a: &Value, what: &str) -> Result<(), String> {
        let Some(have) = self.design_of_the_author() else { return Ok(()) };
        if bool_arg(a, "approved")?.unwrap_or(false) { return Ok(()); }
        Err(format!(
            "Not done: this lesson already has a design of its own: {have}. Do not {what} without asking. Tell the author what you would add or change and why, and ask whether to go ahead; \
if they say yes, call this again with approved set to true. Otherwise build with what is there (get_design lists it). Nothing was changed."
        ))
    }

    /// The theme, brand palette, master and layouts, as an AI tool needs to see them to use them.
    pub(super) fn mcp_get_design(&mut self) -> Result<String, String> {
        let p = self.committed().ok_or("No project is open.")?;
        let (sw, sh) = p.slides.first().map(|s| (s.width, s.height)).unwrap_or((1920.0, 1080.0));
        let layout_of = |s: &Slide| s.layout.as_ref().and_then(|id| p.layout(id)).map(|l| l.name.clone());
        done(json!({
            "slide_size": { "width": sw, "height": sh },
            "theme": {
                "name": p.theme.name, "heading_font": p.theme.heading_font, "body_font": p.theme.body_font,
                "colors": THEME_COLOR_NAMES.iter().zip(&p.theme.colors).map(|(n, c)| json!({ "name": n, "hex": c.hex() })).collect::<Vec<_>>(),
            },
            "brand_palette": p.palette.iter().map(|c| c.hex()).collect::<Vec<_>>(),
            "master": { "background": p.master.background.as_ref().map(|b| b.base_color().hex()), "shapes": p.master.objects.iter().filter(|o| o.placeholder.is_none()).map(describe_object).collect::<Vec<_>>() },
            "layouts": p.layouts.iter().map(|l| json!({
                "id": l.id, "name": l.name, "show_master": l.show_master, "background": l.background.as_ref().map(|b| b.base_color().hex()),
                "placeholders": l.objects.iter().filter(|o| o.placeholder.is_some()).map(describe_object).collect::<Vec<_>>(),
                "shapes": l.objects.iter().filter(|o| o.placeholder.is_none()).map(describe_object).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "slides": p.slides.iter().enumerate().filter(|(_, s)| s.scene != MASTER_SCENE).map(|(i, s)| json!({ "number": i + 1, "layout": layout_of(s) })).collect::<Vec<_>>(),
            "note": "Objects keep the colors they were made with, so changing the theme does not recolor slides that already exist.",
        }))
    }

    /// Sets the theme colors and fonts, the brand palette and the master.
    pub(super) fn mcp_set_design(&mut self, a: &Value) -> Result<String, String> {
        let p = self.project.as_ref().unwrap();
        let mut theme = p.theme.clone();
        while theme.colors.len() < THEME_COLOR_NAMES.len() { theme.colors.push(Rgba(0, 0, 0, 255)); }
        let mut changed: Vec<&str> = vec![];
        if let Some(n) = text_arg(a, "theme_name")? { theme.name = n; changed.push("theme name"); }
        if let Some(map) = a.get("colors").filter(|v| !v.is_null()) {
            let map = map.as_object().ok_or("`colors` must be an object such as {\"Accent 1\": \"#1F6FEB\"}.")?;
            for (k, v) in map {
                let i = THEME_COLOR_NAMES.iter().position(|n| norm(n) == norm(k)).ok_or_else(|| format!("`colors` has no color called {k:?}. The names are: {}.", THEME_COLOR_NAMES.join(", ")))?;
                theme.colors[i] = v.as_str().and_then(Rgba::parse).ok_or_else(|| format!("`colors`: {k:?} is not a color I can read: {v}. Use a hex color such as #1F6FEB."))?;
            }
            changed.push("theme colors");
        }
        if let Some(f) = font_arg(a, "heading_font")? { theme.heading_font = f; changed.push("heading font"); }
        if let Some(f) = font_arg(a, "body_font")? { theme.body_font = f; changed.push("body font"); }
        let palette = match a.get("palette").filter(|v| !v.is_null()) {
            None => None,
            Some(list) => {
                let list = list.as_array().ok_or("`palette` must be a list of hex colors.")?;
                let mut out: Vec<Rgba> = vec![];
                for v in list {
                    let c = v.as_str().and_then(Rgba::parse).ok_or_else(|| format!("`palette`: {v} is not a color I can read. Use a hex color such as #1F6FEB."))?;
                    if !out.contains(&c) { out.push(c); }
                }
                if out.len() > Project::PALETTE_MAX { return Err(format!("`palette` can hold {} colors at most.", Project::PALETTE_MAX)); }
                changed.push("brand palette");
                Some(out)
            }
        };
        let master_bg = color_arg(a, "master_background")?;
        if master_bg.is_some() { changed.push("master background"); }
        let master_shapes = if a.get("master_shapes").is_some_and(|v| !v.is_null()) { Some(shape_specs(a, "master_shapes", theme.color(4))?) } else { None };
        if master_shapes.is_some() { changed.push("master shapes"); }
        if changed.is_empty() { return Err("Give at least one of theme_name, colors, heading_font, body_font, palette, master_background or master_shapes.".into()); }
        self.design_gate(a, &format!("change its {}", changed.join(", ")))?;
        self.snapshot();
        let shapes = master_shapes.map(|s| self.build_shapes(s));
        let p = self.project.as_mut().unwrap();
        p.theme = theme;
        if let Some(pal) = palette { p.palette = pal; }
        if let Some(c) = master_bg { p.master.background = Some(Background::Solid(c)); }
        if let Some(s) = shapes { p.master.objects = s; }
        self.dirty = true;
        self.status = "AI tool: set the theme".into();
        self.design_made();
        done(json!({ "changed": changed, "note": "Slides and layouts that already exist keep their colors and fonts. Layouts made from now on use these." }))
    }

    /// Makes a layout of placeholders (and decoration), or replaces the one of the same name.
    pub(super) fn mcp_create_layout(&mut self, a: &Value) -> Result<String, String> {
        let name = text_arg(a, "name")?.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()).ok_or("`name` must be some text, such as \"Title and Content\".")?;
        let p = self.project.as_ref().unwrap();
        let theme = p.theme.clone();
        let (sw, sh) = p.slides.first().map(|s| (s.width, s.height)).unwrap_or((1920.0, 1080.0));
        let background = color_arg(a, "background")?.map(Background::Solid);
        let show_master = bool_arg(a, "show_master")?.unwrap_or(true);
        let shapes = shape_specs(a, "shapes", theme.color(4))?;
        let list = a.get("placeholders").and_then(Value::as_array).ok_or("`placeholders` must be a list, such as [{\"kind\": \"Heading 2\", \"x\": 120, \"y\": 80, \"width\": 1680, \"height\": 180}].")?;
        if list.is_empty() && shapes.is_empty() { return Err("A layout needs at least one placeholder or shape.".into()); }
        struct Ph { kind: PlaceholderKind, name: Option<String>, x: f32, y: f32, w: f32, h: f32, look: Value }
        let mut phs: Vec<Ph> = vec![];
        for (i, v) in list.iter().enumerate() {
            let at = |e: String| format!("placeholders[{i}]: {e}");
            let kind = placeholder_kind(&text_arg(v, "kind").map_err(at)?.ok_or_else(|| at("`kind` is needed.".into()))?).map_err(at)?;
            let (x, y) = (required_num(v, "x").map_err(at)?, required_num(v, "y").map_err(at)?);
            let w = num_arg(v, "width", 1.0, 20_000.0).map_err(at)?.ok_or_else(|| at("`width` is needed.".into()))?;
            let h = num_arg(v, "height", 1.0, 20_000.0).map_err(at)?.ok_or_else(|| at("`height` is needed.".into()))?;
            if x < 0.0 || y < 0.0 || x + w > sw + 1.0 || y + h > sh + 1.0 { return Err(at(format!("it does not fit on the slide, which is {sw} by {sh}."))); }
            phs.push(Ph { kind, name: text_arg(v, "name").map_err(at)?, x, y, w, h, look: v.clone() });
        }
        for s in &shapes { if s.x < 0.0 || s.y < 0.0 || s.x + s.w > sw + 1.0 || s.y + s.h > sh + 1.0 { return Err(format!("the shape {:?} does not fit on the slide, which is {sw} by {sh}.", s.name)); } }
        // Built before anything changes, so a bad value anywhere leaves the project as it was.
        let mut ids: Vec<String> = (0..phs.len()).map(|_| self.new_id("obj")).collect();
        let mut objects = self.build_shapes(shapes);
        for ph in &phs {
            let mut o = placeholder_object(&ids.remove(0), ph.kind, &theme, ph.x.round(), ph.y.round(), ph.w.round(), ph.h.round());
            if let Some(n) = &ph.name { o.name = n.clone(); }
            Self::mcp_apply_look(&mut o, &ph.look).map_err(|e| format!("placeholder {:?}: {e}", o.name))?;
            objects.push(o);
        }
        let collides = self.project.as_ref().unwrap().layouts.iter().any(|l| l.name.eq_ignore_ascii_case(&name) && !l.objects.is_empty());
        if collides && self.design_of_the_author().is_some() && !bool_arg(a, "approved")?.unwrap_or(false) {
            return Err(format!("Not done: the lesson already has a layout called {name:?}, and it is not yours to replace. Give the new layout another name, such as \"{name} 2\", or ask the author whether to replace it and call this again with approved set to true. Nothing was changed."));
        }
        self.design_gate(a, &format!("add the layout {name:?}"))?;
        self.snapshot();
        let fresh_id = self.new_id("layout");
        let p = self.project.as_mut().unwrap();
        let existing = p.layouts.iter().position(|l| l.name.eq_ignore_ascii_case(&name));
        let id = existing.map(|i| p.layouts[i].id.clone()).unwrap_or(fresh_id);
        let mut layout = SlideLayout::new(&id, &name);
        layout.background = background;
        layout.show_master = show_master;
        layout.objects = objects;
        let described: Vec<Value> = layout.objects.iter().filter(|o| o.placeholder.is_some()).map(|o| json!({ "name": o.name, "kind": o.placeholder.map(|k| k.label()) })).collect();
        match existing { Some(i) => p.layouts[i] = layout, None => p.layouts.push(layout) }
        self.dirty = true;
        self.status = format!("AI tool: made the layout {name}");
        self.design_made();
        done(json!({ "id": id, "name": name, "replaced_one_of_the_same_name": existing.is_some(), "placeholders": described }))
    }

    /// Deletes a layout. Slides that used it keep their content and are detached.
    pub(super) fn mcp_delete_layout(&mut self, a: &Value) -> Result<String, String> {
        let which = text_arg(a, "layout")?.ok_or("`layout` must be a layout's name or id.")?;
        let id = find_layout(self.project.as_ref().unwrap(), &which)?;
        self.design_gate(a, &format!("delete the layout {which:?}"))?;
        self.snapshot();
        let p = self.project.as_mut().unwrap();
        p.layouts.retain(|l| l.id != id);
        let mut detached = 0;
        for s in p.slides.iter_mut().filter(|s| s.layout.as_deref() == Some(id.as_str())) { s.layout = None; detached += 1; }
        self.dirty = true;
        self.status = "AI tool: deleted a layout".into();
        self.design_made();
        done(json!({ "deleted": id, "slides_detached": detached }))
    }

    /// Puts a slide on a layout: its matching placeholders take the layout's place and look, keeping their text.
    pub(super) fn mcp_apply_layout(&mut self, a: &Value, count: usize) -> Result<String, String> {
        let i = slide_arg(a, "slide", count)?;
        let which = text_arg(a, "layout")?.ok_or("`layout` must be a layout's name or id.")?;
        let id = find_layout(self.project.as_ref().unwrap(), &which)?;
        self.set_slide_layout(i, Some(id.clone()));
        self.slide = i;
        self.status = format!("AI tool: changed the layout of slide {}", i + 1);
        let s = &self.project.as_ref().unwrap().slides[i];
        done(json!({ "slide": i + 1, "layout": id, "objects": s.layers[0].objects.iter().map(describe_object).collect::<Vec<_>>() }))
    }

    /// A layout named in a tool's arguments, if one was.
    pub(super) fn layout_arg(&self, a: &Value) -> Result<Option<String>, String> {
        match text_arg(a, "layout")? { None => Ok(None), Some(w) => find_layout(self.project.as_ref().unwrap(), &w).map(Some) }
    }
}
