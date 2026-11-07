//! The tools the MCP controller offers an AI tool, and how each one reads or changes the open project.
//!
//! Every tool runs on the app's own thread, one at a time, through the same steps the editor uses (an undo snapshot first,
//! then the change), so what an AI tool does can be undone and shows up on screen as it happens. Slides are numbered from
//! 1, as in the slide list. Positions and sizes are in slide pixels from the top left.
use super::*;
use brainbend_mcp::ToolDef;
use serde_json::{json, Value};

/// What a tool may do, which the author allows or not in the AI tools window.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Access {
    /// Looks at the project and changes nothing.
    Read,
    /// Changes the open project (undoable).
    Edit,
    /// Opens, saves or publishes files on the computer. No tool uses this yet; the file tools come next.
    #[allow(dead_code)]
    Files,
}

fn schema(properties: Value, required: &[&str]) -> Value {
    json!({ "type": "object", "properties": properties, "required": required, "additionalProperties": false })
}

fn slide_prop() -> Value { json!({ "type": "integer", "minimum": 1, "description": "The slide's number, counting from 1 as in the slide list." }) }
fn object_prop() -> Value { json!({ "type": "string", "description": "An object's id from get_slide (or its name, when that is unique on the slide)." }) }
fn color_prop(what: &str) -> Value { json!({ "type": "string", "description": format!("{what}, as a hex color such as #1F6FEB or 1F6FEB.") }) }
fn number_prop(what: &str) -> Value { json!({ "type": "number", "description": what }) }

/// Every tool, with what it may do.
fn table() -> Vec<(Access, ToolDef)> {
    let pos = |extra: Value| {
        let mut p = json!({
            "x": number_prop("Distance of the left edge from the slide's left, in slide pixels."),
            "y": number_prop("Distance of the top edge from the slide's top, in slide pixels."),
            "width": number_prop("Width in slide pixels."),
            "height": number_prop("Height in slide pixels."),
        });
        if let (Some(a), Some(b)) = (p.as_object_mut(), extra.as_object()) { for (k, v) in b { a.insert(k.clone(), v.clone()); } }
        p
    };
    let text_look = json!({
        "size": number_prop("Font size in slide pixels."),
        "bold": { "type": "boolean" },
        "italic": { "type": "boolean" },
        "color": color_prop("The text color"),
        "align": { "type": "string", "enum": ["left", "center", "right", "justify"] },
    });
    let tool = |access, name, description, schema, read_only| (access, ToolDef { name, description, schema, read_only });
    vec![
        tool(Access::Read, "get_project", "Facts about the open project: its name, kind, number of slides, whether it has unsaved changes and where it is saved. Start here.", schema(json!({}), &[]), true),
        tool(Access::Read, "list_slides", "Lists the slides in order: number, id, name, kind (normal, question or results), how many objects, and how long its timeline is.", schema(json!({}), &[]), true),
        tool(Access::Read, "get_slide", "Describes one slide: its size, notes, timeline length and every object on every layer with its id, type, position, size, text, fill, whether it starts hidden and when it shows on the timeline.", schema(json!({ "slide": slide_prop() }), &["slide"]), true),
        tool(Access::Edit, "add_slide", "Adds an empty slide and returns its number. It goes after the slide numbered `after` (default: at the end).", schema(json!({ "after": slide_prop(), "name": { "type": "string", "description": "The slide's name." } }), &[]), false),
        tool(Access::Edit, "delete_slide", "Deletes a slide. The last slide cannot be deleted.", schema(json!({ "slide": slide_prop() }), &["slide"]), false),
        tool(Access::Edit, "rename_slide", "Gives a slide a new name.", schema(json!({ "slide": slide_prop(), "name": { "type": "string" } }), &["slide", "name"]), false),
        tool(Access::Edit, "set_slide_notes", "Sets the notes of a slide (the speaker notes shown in the player's Notes tab).", schema(json!({ "slide": slide_prop(), "notes": { "type": "string" } }), &["slide", "notes"]), false),
        tool(Access::Edit, "add_text", "Adds a text box to a slide's base layer and returns its id.",
            schema({ let mut p = pos(text_look.clone()); p["slide"] = slide_prop(); p["text"] = json!({ "type": "string" }); p["name"] = json!({ "type": "string" }); p }, &["slide", "text"]), false),
        tool(Access::Edit, "add_shape", "Adds a rectangle, rounded rectangle or ellipse to a slide's base layer, optionally with text inside, and returns its id.",
            schema({ let mut p = pos(json!({})); p["slide"] = slide_prop(); p["shape"] = json!({ "type": "string", "enum": ["rectangle", "rounded_rectangle", "ellipse"] }); p["fill"] = color_prop("The fill color"); p["text"] = json!({ "type": "string" }); p["name"] = json!({ "type": "string" }); p }, &["slide", "shape"]), false),
        tool(Access::Edit, "set_object", "Changes an object: its name, text, position, size, fill, text look, whether it starts hidden, and when it shows on the slide's timeline. Only the properties you give change.",
            schema({
                let mut p = pos(text_look);
                p["slide"] = slide_prop();
                p["object"] = object_prop();
                p["name"] = json!({ "type": "string" });
                p["text"] = json!({ "type": "string" });
                p["fill"] = color_prop("The fill color");
                p["hidden"] = json!({ "type": "boolean", "description": "Starts hidden (a trigger shows it)." });
                p["start_ms"] = json!({ "type": "integer", "minimum": 0, "description": "When it appears on the timeline, in milliseconds. Setting this puts the object on the timeline." });
                p["duration_ms"] = json!({ "type": ["integer", "null"], "minimum": 0, "description": "How long it shows, in milliseconds, or null to last until the end." });
                p["always_visible"] = json!({ "type": "boolean", "description": "true shows it for the whole timeline (clears its start and duration)." });
                p
            }, &["slide", "object"]), false),
        tool(Access::Edit, "delete_object", "Deletes an object from a slide.", schema(json!({ "slide": slide_prop(), "object": object_prop() }), &["slide", "object"]), false),
        tool(Access::Edit, "undo", "Undoes the last change made to the project, whoever made it.", schema(json!({}), &[]), false),
        tool(Access::Edit, "redo", "Redoes the change that was just undone.", schema(json!({}), &[]), false),
    ]
}

/// The tools to offer.
pub fn tools() -> Vec<ToolDef> { table().into_iter().map(|(_, t)| t).collect() }

/// What a tool may do, `None` for a name that is not a tool.
pub fn access(name: &str) -> Option<Access> { table().into_iter().find(|(_, t)| t.name == name).map(|(a, _)| a) }

/// What the AI tool is told about the controller when it connects.
pub const INSTRUCTIONS: &str = "This controls BrainBend Studio, an app for building interactive lessons, while the author watches. \
Slides are numbered from 1; positions and sizes are in slide pixels from the top left (a slide is usually 1280 by 720). \
Call get_project and list_slides first, and get_slide to see what is on a slide and the ids to change. Every change can be undone with undo. \
Work in small steps and say what you did, since the author sees the slides change as you work.";

// ---- reading what a tool was given ----

fn bad(key: &str, want: &str) -> String { format!("`{key}` must be {want}.") }

fn text_arg(a: &Value, key: &str) -> Result<Option<String>, String> {
    match a.get(key) { None | Some(Value::Null) => Ok(None), Some(Value::String(s)) => Ok(Some(s.clone())), Some(_) => Err(bad(key, "text")) }
}

fn num_arg(a: &Value, key: &str, min: f64, max: f64) -> Result<Option<f32>, String> {
    match a.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => match v.as_f64() {
            Some(n) if n.is_finite() && n >= min && n <= max => Ok(Some(n as f32)),
            Some(_) => Err(format!("`{key}` must be between {min} and {max}.")),
            None => Err(bad(key, "a number")),
        },
    }
}

fn bool_arg(a: &Value, key: &str) -> Result<Option<bool>, String> {
    match a.get(key) { None | Some(Value::Null) => Ok(None), Some(Value::Bool(b)) => Ok(Some(*b)), Some(_) => Err(bad(key, "true or false")) }
}

fn color_arg(a: &Value, key: &str) -> Result<Option<Rgba>, String> {
    match text_arg(a, key)? {
        None => Ok(None),
        Some(t) => Rgba::parse(&t).map(Some).ok_or_else(|| format!("`{key}` is not a color I can read: {t:?}. Use a hex color such as #1F6FEB.")),
    }
}

fn align_arg(a: &Value) -> Result<Option<Align>, String> {
    match text_arg(a, "align")?.as_deref() {
        None => Ok(None),
        Some("left") => Ok(Some(Align::Left)),
        Some("center") => Ok(Some(Align::Center)),
        Some("right") => Ok(Some(Align::Right)),
        Some("justify") => Ok(Some(Align::Justify)),
        Some(other) => Err(format!("`align` must be left, center, right or justify, not {other:?}.")),
    }
}

/// A slide number (from 1) as a position in the project's list.
fn slide_arg(a: &Value, key: &str, count: usize) -> Result<usize, String> {
    let n = a.get(key).and_then(Value::as_u64).ok_or_else(|| format!("`{key}` must be a slide number, counting from 1."))? as usize;
    if n == 0 || n > count { return Err(format!("There is no slide {n}; the project has {count} slide{}.", if count == 1 { "" } else { "s" })); }
    Ok(n - 1)
}

/// The place of an object on a slide, by id or by a name that only one object has.
fn find_object(slide: &Slide, which: &str) -> Result<(usize, usize), String> {
    let all: Vec<(usize, usize, &Object)> = slide.layers.iter().enumerate().flat_map(|(li, l)| l.objects.iter().enumerate().map(move |(oi, o)| (li, oi, o))).collect();
    if let Some((li, oi, _)) = all.iter().find(|(_, _, o)| o.id == which) { return Ok((*li, *oi)); }
    let named: Vec<_> = all.iter().filter(|(_, _, o)| o.name == which).collect();
    match named.len() {
        1 => Ok((named[0].0, named[0].1)),
        0 => Err(format!("There is no object {which:?} on this slide. get_slide lists the objects and their ids.")),
        n => Err(format!("{n} objects on this slide are called {which:?}; use an id from get_slide.")),
    }
}

fn kind_label(o: &Object) -> &'static str {
    match &o.kind {
        Kind::Rect => "rectangle", Kind::Ellipse => "ellipse", Kind::Text => "text", Kind::Image => "picture", Kind::Video => "video", Kind::Audio => "audio",
        Kind::Hotspot => "hotspot", Kind::Path(_) => "vector shape", Kind::Line | Kind::LineUp => "line", Kind::Slider(_) => "slider", Kind::Web(_) => "web object",
        Kind::Pano(_) => "360 image", Kind::Input(_) => "input", _ => "other",
    }
}

fn describe_object(o: &Object) -> Value {
    let mut v = json!({
        "id": o.id, "name": o.name, "type": kind_label(o),
        "x": o.x, "y": o.y, "width": o.w, "height": o.h,
        "hidden": o.hidden,
    });
    if !o.text.is_empty() { v["text"] = json!(o.text); }
    if let Some(f) = o.fill { v["fill"] = json!(f.hex()); }
    if let Some(t) = o.timing { v["start_ms"] = json!(t.start_ms); v["duration_ms"] = json!(t.dur_ms); }
    v
}

fn slide_kind(s: &Slide) -> &'static str { match s.kind { SlideKind::Normal => "normal", SlideKind::Question(_) => "question", SlideKind::Results => "results" } }

fn timeline_ms(s: &Slide) -> u32 { s.layers.first().and_then(|l| l.duration_ms).unwrap_or(0) }

fn done(v: Value) -> Result<String, String> { serde_json::to_string_pretty(&v).map_err(|e| e.to_string()) }

impl App {
    /// Runs a tool for an AI tool and says what happened, or why it could not. The caller has already checked that the
    /// author allows this kind of tool.
    pub(super) fn mcp_run(&mut self, name: &str, a: &Value) -> Result<String, String> {
        let Some(p) = self.project.as_ref() else { return Err("No project is open. Ask the author to open or create one in BrainBend Studio.".into()) };
        if p.is_web() { return Err("This is a Web Lesson, and the controller works on Media Lessons (projects of slides) for now.".into()); }
        let editing = access(name) == Some(Access::Edit);
        if editing {
            // The author is typing into a text box right now; changing the project under their cursor would lose their words.
            if self.editing.is_some() { return Err("The author is typing in a text box. Try again in a moment.".into()); }
            self.end_state_swap();
            self.leave_master_view();
        }
        let count = self.project.as_ref().map_or(0, |p| p.slides.len());
        match name {
            "get_project" => {
                let p = self.project.as_ref().unwrap();
                done(json!({
                    "name": p.name, "kind": p.kind.label(), "slides": p.slides.len(), "pass_mark_percent": p.pass_mark,
                    "unsaved_changes": self.dirty, "saved_at": self.path.as_ref().map(|p| p.display().to_string()),
                    "current_slide": self.slide + 1,
                }))
            }
            "list_slides" => {
                let p = self.project.as_ref().unwrap();
                done(json!(p.slides.iter().enumerate().map(|(i, s)| json!({
                    "number": i + 1, "id": s.id, "name": s.name, "kind": slide_kind(s),
                    "objects": s.layers.iter().map(|l| l.objects.len()).sum::<usize>(), "layers": s.layers.len(), "timeline_ms": timeline_ms(s),
                })).collect::<Vec<_>>()))
            }
            "get_slide" => {
                let i = slide_arg(a, "slide", count)?;
                let s = &self.project.as_ref().unwrap().slides[i];
                done(json!({
                    "number": i + 1, "id": s.id, "name": s.name, "kind": slide_kind(s), "width": s.width, "height": s.height,
                    "notes": s.notes, "timeline_ms": timeline_ms(s),
                    "layers": s.layers.iter().enumerate().map(|(li, l)| json!({ "layer": li, "name": l.name, "objects": l.objects.iter().map(describe_object).collect::<Vec<_>>() })).collect::<Vec<_>>(),
                }))
            }
            "add_slide" => {
                let after = match a.get("after") { None | Some(Value::Null) => count, Some(_) => slide_arg(a, "after", count)? + 1 };
                let title = text_arg(a, "name")?;
                self.slide = after - 1;
                self.add_slide(SlideKind::Normal);
                if let (Some(t), Some(p)) = (title.filter(|t| !t.trim().is_empty()), self.project.as_mut()) { p.slides[self.slide].name = t.trim().to_string(); }
                self.status = format!("AI tool: added slide {}", self.slide + 1);
                let s = &self.project.as_ref().unwrap().slides[self.slide];
                done(json!({ "number": self.slide + 1, "id": s.id, "name": s.name, "width": s.width, "height": s.height }))
            }
            "delete_slide" => {
                let i = slide_arg(a, "slide", count)?;
                if count <= 1 { return Err("The last slide cannot be deleted.".into()); }
                self.delete_slide(i);
                self.status = format!("AI tool: deleted slide {}", i + 1);
                done(json!({ "deleted": i + 1, "slides_left": self.project.as_ref().map_or(0, |p| p.slides.len()) }))
            }
            "rename_slide" => {
                let i = slide_arg(a, "slide", count)?;
                let new = text_arg(a, "name")?.filter(|n| !n.trim().is_empty()).ok_or("`name` must be some text.")?;
                self.snapshot();
                self.project.as_mut().unwrap().slides[i].name = new.trim().to_string();
                self.slide = i;
                self.status = format!("AI tool: renamed slide {}", i + 1);
                done(json!({ "number": i + 1, "name": new.trim() }))
            }
            "set_slide_notes" => {
                let i = slide_arg(a, "slide", count)?;
                let notes = text_arg(a, "notes")?.ok_or("`notes` must be text.")?;
                self.snapshot();
                self.project.as_mut().unwrap().slides[i].notes = notes;
                self.slide = i;
                self.status = format!("AI tool: set the notes of slide {}", i + 1);
                done(json!({ "number": i + 1 }))
            }
            "add_text" | "add_shape" => self.mcp_add_object(name, a, count),
            "set_object" => self.mcp_set_object(a, count),
            "delete_object" => {
                let i = slide_arg(a, "slide", count)?;
                let which = text_arg(a, "object")?.ok_or("`object` must be an id or a name.")?;
                let (li, oi) = find_object(&self.project.as_ref().unwrap().slides[i], &which)?;
                self.snapshot();
                let removed = self.project.as_mut().unwrap().slides[i].layers[li].objects.remove(oi);
                self.slide = i;
                self.selected = None;
                self.status = format!("AI tool: deleted {} from slide {}", removed.name, i + 1);
                done(json!({ "deleted": removed.id }))
            }
            "undo" => {
                if self.undo.is_empty() { return Err("There is nothing to undo.".into()); }
                self.undo();
                self.status = "AI tool: undid the last change".into();
                done(json!({ "undone": true, "more_to_undo": !self.undo.is_empty() }))
            }
            "redo" => {
                if self.redo.is_empty() { return Err("There is nothing to redo.".into()); }
                self.redo();
                self.status = "AI tool: redid a change".into();
                done(json!({ "redone": true, "more_to_redo": !self.redo.is_empty() }))
            }
            other => Err(format!("There is no tool called {other}.")),
        }
    }

    /// Text styling and the look shared by `add_text`, `add_shape` and `set_object`.
    fn mcp_apply_look(o: &mut Object, a: &Value) -> Result<(), String> {
        if let Some(n) = num_arg(a, "size", 1.0, 1000.0)? { o.style.size = n; }
        if let Some(b) = bool_arg(a, "bold")? { o.style.bold = b; }
        if let Some(b) = bool_arg(a, "italic")? { o.style.italic = b; }
        if let Some(c) = color_arg(a, "color")? { o.style.color = c; }
        if let Some(al) = align_arg(a)? { o.style.align = al; }
        if let Some(c) = color_arg(a, "fill")? { o.fill = Some(c); o.gradient = None; }
        Ok(())
    }

    fn mcp_add_object(&mut self, name: &str, a: &Value, count: usize) -> Result<String, String> {
        let i = slide_arg(a, "slide", count)?;
        let (sw, sh) = { let s = &self.project.as_ref().unwrap().slides[i]; (s.width, s.height) };
        let text = text_arg(a, "text")?;
        let label = text_arg(a, "name")?;
        let (x, y) = (num_arg(a, "x", -20_000.0, 20_000.0)?, num_arg(a, "y", -20_000.0, 20_000.0)?);
        let (w, h) = (num_arg(a, "width", 1.0, 20_000.0)?, num_arg(a, "height", 1.0, 20_000.0)?);
        let mut o = if name == "add_text" {
            let text = text.ok_or("`text` must be some text.")?;
            let mut o = blank_object(self.new_id("obj"), label.as_deref().unwrap_or("Text"), Kind::Text, 0.0, 0.0, w.unwrap_or(sw * 0.5), h.unwrap_or(110.0));
            o.text = text;
            // Like the editor's own text box, text is anchored at the top.
            o.style.valign = VAlign::Top;
            o
        } else {
            let (kind, label_default, radius) = match text_arg(a, "shape")?.as_deref() {
                Some("rectangle") => (Kind::Rect, "Rectangle", 0.0),
                Some("rounded_rectangle") => (Kind::Rect, "Rounded rectangle", 24.0),
                Some("ellipse") => (Kind::Ellipse, "Ellipse", 0.0),
                _ => return Err("`shape` must be rectangle, rounded_rectangle or ellipse.".into()),
            };
            let accent = self.project.as_ref().map(|p| p.theme.color(4)).unwrap_or(Rgba(3, 83, 158, 255));
            let mut o = blank_object(self.new_id("obj"), label.as_deref().unwrap_or(label_default), kind, 0.0, 0.0, w.unwrap_or(sw * 0.3), h.unwrap_or(sh * 0.3));
            o.radius = radius;
            o.fill = Some(accent);
            if let Some(t) = text { o.text = t; }
            o
        };
        Self::mcp_apply_look(&mut o, a)?;
        // Without a place given, new objects start a little in from the corner and step along so they do not hide each other.
        let existing = self.project.as_ref().unwrap().slides[i].layers[0].objects.len();
        o.x = x.unwrap_or(sw * 0.08 + (existing % 8) as f32 * 36.0).round();
        o.y = y.unwrap_or(sh * 0.08 + (existing % 8) as f32 * 36.0).round();
        let described = describe_object(&o);
        self.snapshot();
        let slide = &mut self.project.as_mut().unwrap().slides[i];
        slide.layers[0].objects.push(o);
        self.selected = Some((0, slide.layers[0].objects.len() - 1));
        self.slide = i;
        self.layer = 0;
        self.status = format!("AI tool: added {} to slide {}", if name == "add_text" { "a text box" } else { "a shape" }, i + 1);
        done(described)
    }

    fn mcp_set_object(&mut self, a: &Value, count: usize) -> Result<String, String> {
        let i = slide_arg(a, "slide", count)?;
        let which = text_arg(a, "object")?.ok_or("`object` must be an id or a name.")?;
        let (li, oi) = find_object(&self.project.as_ref().unwrap().slides[i], &which)?;
        // Work on a copy so that a bad value anywhere leaves the object as it was.
        let mut o = self.project.as_ref().unwrap().slides[i].layers[li].objects[oi].clone();
        if let Some(n) = text_arg(a, "name")? { o.name = n; }
        if let Some(t) = text_arg(a, "text")? { o.text = t; o.rich = None; }
        if let Some(v) = num_arg(a, "x", -20_000.0, 20_000.0)? { o.x = v; }
        if let Some(v) = num_arg(a, "y", -20_000.0, 20_000.0)? { o.y = v; }
        if let Some(v) = num_arg(a, "width", 1.0, 20_000.0)? { o.w = v; }
        if let Some(v) = num_arg(a, "height", 1.0, 20_000.0)? { o.h = v; }
        if let Some(b) = bool_arg(a, "hidden")? { o.hidden = b; }
        Self::mcp_apply_look(&mut o, a)?;
        if bool_arg(a, "always_visible")? == Some(true) {
            o.timing = None;
        } else {
            let start = num_arg(a, "start_ms", 0.0, 86_400_000.0)?.map(|v| v as u32);
            let duration = match a.get("duration_ms") {
                None => None,
                Some(Value::Null) => Some(None),
                Some(_) => Some(num_arg(a, "duration_ms", 0.0, 86_400_000.0)?.map(|v| v as u32)),
            };
            if start.is_some() || duration.is_some() {
                let current = o.timing.unwrap_or(Timing { start_ms: 0, dur_ms: None });
                o.timing = Some(Timing { start_ms: start.unwrap_or(current.start_ms), dur_ms: duration.unwrap_or(current.dur_ms) });
            }
        }
        let described = describe_object(&o);
        self.snapshot();
        self.project.as_mut().unwrap().slides[i].layers[li].objects[oi] = o;
        self.slide = i;
        self.layer = li;
        self.selected = Some((li, oi));
        self.status = format!("AI tool: changed an object on slide {}", i + 1);
        done(described)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app_with(slides: usize) -> App {
        let mut a = App::default();
        a.new_project();
        for _ in 1..slides { a.add_slide(SlideKind::Normal); }
        a.slide = 0;
        a.undo.clear();
        a.dirty = false;
        a
    }

    fn run(a: &mut App, name: &str, args: Value) -> Result<Value, String> {
        a.mcp_run(name, &args).map(|t| serde_json::from_str(&t).expect("a tool answers in JSON"))
    }

    #[test]
    fn every_tool_has_a_schema_a_description_and_an_access_level() {
        let t = tools();
        assert_eq!(t.len(), 13);
        let mut names: Vec<_> = t.iter().map(|d| d.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), t.len(), "no name is used twice");
        for d in &t {
            assert_eq!(d.schema["type"], "object", "{}", d.name);
            assert!(d.description.len() > 20, "{} needs a description an AI tool can use", d.name);
            let a = access(d.name).expect("has an access level");
            assert_eq!(d.read_only, a == Access::Read, "{}: the read-only hint matches the access level", d.name);
            for r in d.schema["required"].as_array().unwrap() { assert!(d.schema["properties"].get(r.as_str().unwrap()).is_some(), "{} requires a property it does not describe", d.name); }
        }
        assert_eq!(access("get_slide"), Some(Access::Read));
        assert_eq!(access("add_text"), Some(Access::Edit));
        assert_eq!(access("nope"), None);
    }

    #[test]
    fn nothing_works_without_a_project_or_on_a_web_lesson() {
        let mut a = App::default();
        let e = run(&mut a, "get_project", json!({})).unwrap_err();
        assert!(e.contains("No project is open"), "{e}");
        let mut w = App::default();
        w.new_lesson_project(ProjectKind::WebLesson);
        assert!(run(&mut w, "list_slides", json!({})).unwrap_err().contains("Web Lesson"));
    }

    #[test]
    fn the_project_and_its_slides_can_be_read() {
        let mut a = app_with(3);
        a.project.as_mut().unwrap().name = "Safety".into();
        let p = run(&mut a, "get_project", json!({})).unwrap();
        assert_eq!((p["name"].as_str(), p["slides"].as_u64(), p["unsaved_changes"].as_bool(), p["current_slide"].as_u64()), (Some("Safety"), Some(3), Some(false), Some(1)));
        let l = run(&mut a, "list_slides", json!({})).unwrap();
        assert_eq!(l.as_array().unwrap().len(), 3);
        assert_eq!(l[1]["number"], 2);
        assert_eq!(l[0]["kind"], "normal");
        let s = run(&mut a, "get_slide", json!({ "slide": 2 })).unwrap();
        assert_eq!(s["number"], 2);
        assert_eq!(s["layers"][0]["name"], "Base layer");
        assert!(run(&mut a, "get_slide", json!({ "slide": 9 })).unwrap_err().contains("There is no slide 9; the project has 3 slides."));
        assert!(run(&mut a, "get_slide", json!({ "slide": 0 })).is_err());
        assert!(run(&mut a, "get_slide", json!({})).unwrap_err().contains("slide number"));
        assert!(!a.dirty && a.undo.is_empty(), "reading changes nothing");
    }

    #[test]
    fn slides_can_be_added_renamed_given_notes_and_deleted() {
        let mut a = app_with(2);
        let added = run(&mut a, "add_slide", json!({ "after": 1, "name": "  Welcome " })).unwrap();
        assert_eq!((added["number"].as_u64(), added["name"].as_str()), (Some(2), Some("Welcome")), "it goes after slide 1");
        assert_eq!(a.project.as_ref().unwrap().slides.len(), 3);
        assert!(a.dirty, "the project has unsaved changes");
        let last = run(&mut a, "add_slide", json!({})).unwrap();
        assert_eq!(last["number"], 4, "no `after` means at the end");
        run(&mut a, "rename_slide", json!({ "slide": 4, "name": "Wrap-up" })).unwrap();
        run(&mut a, "set_slide_notes", json!({ "slide": 4, "notes": "Say thanks." })).unwrap();
        let s = run(&mut a, "get_slide", json!({ "slide": 4 })).unwrap();
        assert_eq!((s["name"].as_str(), s["notes"].as_str()), (Some("Wrap-up"), Some("Say thanks.")));
        assert!(run(&mut a, "rename_slide", json!({ "slide": 4, "name": "  " })).is_err(), "a name needs some text");
        let d = run(&mut a, "delete_slide", json!({ "slide": 4 })).unwrap();
        assert_eq!((d["deleted"].as_u64(), d["slides_left"].as_u64()), (Some(4), Some(3)));
        let mut one = app_with(1);
        assert!(run(&mut one, "delete_slide", json!({ "slide": 1 })).unwrap_err().contains("last slide"));
    }

    #[test]
    fn text_and_shapes_are_added_with_a_look_and_can_be_changed() {
        let mut a = app_with(1);
        let t = run(&mut a, "add_text", json!({ "slide": 1, "text": "Hello", "x": 100, "y": 50, "width": 600, "size": 64, "bold": true, "color": "#C0392B", "align": "center", "name": "Title" })).unwrap();
        assert_eq!((t["type"].as_str(), t["text"].as_str(), t["name"].as_str(), t["x"].as_f64(), t["width"].as_f64()), (Some("text"), Some("Hello"), Some("Title"), Some(100.0), Some(600.0)));
        let o = &a.project.as_ref().unwrap().slides[0].layers[0].objects[0];
        assert_eq!((o.style.size, o.style.bold, o.style.color, o.style.align), (64.0, true, Rgba(192, 57, 43, 255), Align::Center));
        let r = run(&mut a, "add_shape", json!({ "slide": 1, "shape": "ellipse", "fill": "1F6FEB", "text": "OK", "x": 10, "y": 10, "width": 200, "height": 100 })).unwrap();
        assert_eq!((r["type"].as_str(), r["fill"].as_str()), (Some("ellipse"), Some("#1F6FEB")));
        let id = r["id"].as_str().unwrap().to_string();
        let c = run(&mut a, "set_object", json!({ "slide": 1, "object": id, "text": "Changed", "x": 500, "fill": "#000", "hidden": true })).unwrap();
        assert_eq!((c["text"].as_str(), c["x"].as_f64(), c["fill"].as_str(), c["hidden"].as_bool()), (Some("Changed"), Some(500.0), Some("#000000"), Some(true)));
        let by_name = run(&mut a, "set_object", json!({ "slide": 1, "object": "Title", "text": "Renamed by name" })).unwrap();
        assert_eq!(by_name["text"], "Renamed by name", "an object can be found by its name");
        let s = run(&mut a, "get_slide", json!({ "slide": 1 })).unwrap();
        assert_eq!(s["layers"][0]["objects"].as_array().unwrap().len(), 2);
        run(&mut a, "delete_object", json!({ "slide": 1, "object": id })).unwrap();
        assert_eq!(a.project.as_ref().unwrap().slides[0].layers[0].objects.len(), 1);
    }

    #[test]
    fn objects_without_a_place_do_not_pile_up() {
        let mut a = app_with(1);
        let first = run(&mut a, "add_text", json!({ "slide": 1, "text": "a" })).unwrap();
        let second = run(&mut a, "add_text", json!({ "slide": 1, "text": "b" })).unwrap();
        assert_ne!((first["x"].as_f64(), first["y"].as_f64()), (second["x"].as_f64(), second["y"].as_f64()));
    }

    #[test]
    fn timing_can_be_set_changed_and_cleared() {
        let mut a = app_with(1);
        let id = run(&mut a, "add_text", json!({ "slide": 1, "text": "t" })).unwrap()["id"].as_str().unwrap().to_string();
        let t = run(&mut a, "set_object", json!({ "slide": 1, "object": id, "start_ms": 1000, "duration_ms": 2000 })).unwrap();
        assert_eq!((t["start_ms"].as_u64(), t["duration_ms"].as_u64()), (Some(1000), Some(2000)));
        let t = run(&mut a, "set_object", json!({ "slide": 1, "object": id, "duration_ms": null })).unwrap();
        assert_eq!((t["start_ms"].as_u64(), t["duration_ms"].clone()), (Some(1000), Value::Null), "null lasts to the end and the start stays");
        let t = run(&mut a, "set_object", json!({ "slide": 1, "object": id, "always_visible": true })).unwrap();
        assert!(t.get("start_ms").is_none(), "no longer on the timeline");
    }

    #[test]
    fn a_bad_value_changes_nothing_and_says_what_was_wrong() {
        let mut a = app_with(1);
        let id = run(&mut a, "add_text", json!({ "slide": 1, "text": "keep" })).unwrap()["id"].as_str().unwrap().to_string();
        a.undo.clear();
        a.dirty = false;
        let before = a.project.as_ref().unwrap().slides[0].layers[0].objects[0].clone();
        for (args, wants) in [
            (json!({ "slide": 1, "object": id, "text": "new", "color": "not a color" }), "not a color I can read"),
            (json!({ "slide": 1, "object": id, "text": "new", "width": -5 }), "between"),
            (json!({ "slide": 1, "object": id, "text": "new", "width": "wide" }), "must be a number"),
            (json!({ "slide": 1, "object": id, "text": "new", "hidden": "yes" }), "true or false"),
            (json!({ "slide": 1, "object": id, "text": "new", "align": "middle" }), "left, center, right or justify"),
            (json!({ "slide": 1, "object": "missing" }), "There is no object"),
            (json!({ "slide": 1, "object": id, "text": 5 }), "must be text"),
        ] {
            let e = run(&mut a, "set_object", args).unwrap_err();
            assert!(e.contains(wants), "{e} should mention {wants}");
        }
        let after = &a.project.as_ref().unwrap().slides[0].layers[0].objects[0];
        assert_eq!((after.text.as_str(), after.w), (before.text.as_str(), before.w), "the object is as it was");
        assert!(!a.dirty && a.undo.is_empty(), "and nothing was recorded as a change");
        assert!(run(&mut a, "add_shape", json!({ "slide": 1, "shape": "star" })).unwrap_err().contains("rectangle"));
    }

    #[test]
    fn two_objects_with_the_same_name_must_be_told_apart_by_id() {
        let mut a = app_with(1);
        run(&mut a, "add_text", json!({ "slide": 1, "text": "a", "name": "Label" })).unwrap();
        run(&mut a, "add_text", json!({ "slide": 1, "text": "b", "name": "Label" })).unwrap();
        let e = run(&mut a, "set_object", json!({ "slide": 1, "object": "Label", "text": "c" })).unwrap_err();
        assert!(e.contains("2 objects") && e.contains("id"), "{e}");
    }

    #[test]
    fn what_a_tool_did_can_be_undone_and_redone() {
        let mut a = app_with(1);
        run(&mut a, "add_text", json!({ "slide": 1, "text": "x" })).unwrap();
        assert_eq!(a.project.as_ref().unwrap().slides[0].layers[0].objects.len(), 1);
        let u = run(&mut a, "undo", json!({})).unwrap();
        assert_eq!(u["undone"], true);
        assert_eq!(a.project.as_ref().unwrap().slides[0].layers[0].objects.len(), 0, "the text box is gone");
        assert!(run(&mut a, "undo", json!({})).unwrap_err().contains("nothing to undo"));
        run(&mut a, "redo", json!({})).unwrap();
        assert_eq!(a.project.as_ref().unwrap().slides[0].layers[0].objects.len(), 1, "and back");
        assert!(run(&mut a, "redo", json!({})).unwrap_err().contains("nothing to redo"));
    }

    #[test]
    fn changes_wait_while_the_author_is_typing() {
        let mut a = app_with(1);
        a.editing = Some((0, 0));
        let e = run(&mut a, "add_text", json!({ "slide": 1, "text": "x" })).unwrap_err();
        assert!(e.contains("typing"), "{e}");
        assert_eq!(a.project.as_ref().unwrap().slides[0].layers[0].objects.len(), 0);
        assert!(run(&mut a, "list_slides", json!({})).is_ok(), "reading is fine while they type");
    }
}
