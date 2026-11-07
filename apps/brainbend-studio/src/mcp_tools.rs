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
        tool(Access::Read, "describe_model", "Tells you how the project is written and what can be set. With no `type` it gives an overview and lists every type; with a type name (Object, Entrance, Emphasis, Trigger, Action, Event, TextStyle, Slide, Layer, Theme, ...) it lists that type's fields, what each takes and what each does. Use it before changing anything you are not sure of.",
            schema(json!({ "type": { "type": "string", "description": "A type name, such as Object or Entrance. Leave out for the overview." } }), &[]), true),
        tool(Access::Read, "read", "Reads any part of the project as JSON, by path (JSON Pointer, counting from 0): /slides/0, /slides/0/layers/0/objects/2, /slides/0/triggers, /theme, /variables, /player. With no path, or a part too big to show, it lists what is inside and how big each part is. Pictures, video and audio files are not included.",
            schema(json!({ "path": { "type": "string", "description": "A JSON Pointer such as /slides/0/layers/0/objects/1. Leave out to see what the project contains." } }), &[]), true),
        tool(Access::Edit, "patch", "Changes anything in the project with JSON Patch (RFC 6902) operations: add, remove, replace, move, copy, test. All or nothing: it is checked against the model first and nothing changes if any part is wrong; the error says where. `add` also sets a field (replacing it if there); `-` at the end of a list's path means append. Cannot change assets, the project's kind or format. Example: [{\"op\":\"add\",\"path\":\"/slides/0/triggers/-\",\"value\":{...}}]. Use describe_model to see what each thing takes.",
            schema(json!({ "operations": { "type": "array", "items": { "type": "object" }, "description": "JSON Patch operations, each with op, path and (for add, replace, test) value, or from (for move, copy)." } }), &["operations"]), false),
        tool(Access::Edit, "create_object", "Adds an object of any kind to a slide and returns its id and path. `object` holds any properties of the object (kind, name, x, y, w, h, text, style, fill, stroke, shadow, entrance, exit, emphasis, motion, timing, states, and so on) and what you leave out takes its default. It goes on layer 0 (the base layer) unless you give `layer`, at the end unless you give `index`. An id is made for it. Use describe_model with Object for everything it can have.",
            schema(json!({ "slide": slide_prop(), "layer": { "type": "integer", "minimum": 0, "description": "The layer, counting from 0 (0 is the base layer)." }, "index": { "type": "integer", "minimum": 0, "description": "Where in the layer's list; stacking order is the list order, the last is in front." }, "object": { "type": "object", "description": "The object's properties." } }), &["slide", "object"]), false),
        tool(Access::Edit, "update_object", "Changes any properties of an object. `set` holds the fields to change: a field you give is set, a field set to null is removed (back to its default), and a field that is itself an object, such as style, is merged into rather than replaced. A list, such as a path's points, is replaced whole. Returns the object. Use describe_model with Object for everything it can have, and with Entrance, Emphasis or MotionPath for animation.",
            schema(json!({ "slide": slide_prop(), "object": object_prop(), "set": { "type": "object", "description": "The properties to change." } }), &["slide", "object", "set"]), false),
        tool(Access::Edit, "update_slide", "Changes any properties of a slide, the same way as update_object: its name, notes, size, background, transition (transition and fade_ms), whether it moves on by itself (auto_advance), navigation rules, its triggers, and so on. A list such as triggers or layers is replaced whole; to change one item in it, use patch. Returns the slide's number. Use describe_model with Slide.",
            schema(json!({ "slide": slide_prop(), "set": { "type": "object", "description": "The properties to change." } }), &["slide", "set"]), false),
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
                    "layers": s.layers.iter().enumerate().map(|(li, l)| json!({ "layer": li, "name": l.name, "duration_ms": l.duration_ms, "objects": l.objects.iter().enumerate().map(|(oi, o)| { let mut d = describe_object(o); d["path"] = json!(format!("/slides/{i}/layers/{li}/objects/{oi}")); d }).collect::<Vec<_>>() })).collect::<Vec<_>>(),
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
            "describe_model" => brainbend_doc::model_json::describe(text_arg(a, "type")?.as_deref()),
            "read" => self.mcp_read(a),
            "patch" => self.mcp_patch(a),
            "create_object" => self.mcp_create_object(a, count),
            "update_object" => self.mcp_update(a, count, true),
            "update_slide" => self.mcp_update(a, count, false),
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

/// The most of the project one `read` shows; anything bigger gets an outline of what is inside.
const MAX_READ: usize = 30_000;

/// Each part of `v` with its place and size, for choosing what to read.
fn outline(path: &str, v: &Value) -> String {
    let size = |c: &Value| { let n = c.to_string().len(); if n >= 1024 { format!("{:.1} KB", n as f64 / 1024.0) } else { format!("{n} bytes") } };
    let rows: Vec<String> = match v {
        Value::Object(m) => m.iter().map(|(k, c)| format!("  {path}/{k} ({})", size(c))).collect(),
        Value::Array(a) => a.iter().enumerate().take(60).map(|(i, c)| format!("  {path}/{i} ({})", size(c))).collect(),
        _ => vec![],
    };
    rows.join("\n")
}

/// Problems in a project as short sentences, to tell what a change newly broke.
fn issue_lines(p: &Project) -> std::collections::BTreeSet<String> {
    p.check().iter().map(|i| format!("slide {}: {}", i.at.slide + 1, i.message)).collect()
}

/// The slide a patch is about, from the first path that names one: `/slides/3/...` is slide 4.
fn slide_in(ops: &Value) -> Option<usize> {
    ops.as_array()?.iter().find_map(|op| op.get("path")?.as_str()?.strip_prefix("/slides/")?.split('/').next()?.parse::<usize>().ok())
}

impl App {
    /// Puts a project an edit has made in place of the open one, as one undo step, and says what it newly broke.
    fn mcp_install(&mut self, new: Project, what: &str, slide: Option<usize>, select: Option<(usize, usize, usize)>) -> Value {
        let before = self.project.as_ref().map(issue_lines).unwrap_or_default();
        self.snapshot();
        let after = issue_lines(&new);
        self.project = Some(new);
        let slides = self.project.as_ref().map_or(1, |p| p.slides.len()).max(1);
        self.slide = slide.unwrap_or(self.slide).min(slides - 1);
        match select {
            Some((si, li, oi)) => { self.slide = si.min(slides - 1); self.layer = li; self.selected = Some((li, oi)); }
            None => { self.layer = 0; self.selected = None; }
        }
        self.reset_visibility();
        self.status = format!("AI tool: {what}");
        let warnings: Vec<String> = after.difference(&before).take(12).cloned().collect();
        json!({ "ok": true, "warnings": warnings })
    }

    fn mcp_read(&mut self, a: &Value) -> Result<String, String> {
        let path = text_arg(a, "path")?.unwrap_or_default();
        let project = self.project.as_mut().ok_or("No project is open.")?;
        let value = brainbend_doc::model_json::read(project, &path)?;
        let text = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
        if path.is_empty() || text.len() > MAX_READ {
            let inside = outline(&path, &value);
            return if path.is_empty() {
                Ok(format!("The project, by part (read a path to see one):\n{inside}\nA slide's objects are at /slides/N/layers/L/objects/I; get_slide lists them with their paths."))
            } else if inside.is_empty() {
                Err(format!("That is {} KB of text; it is too big to show.", text.len() / 1024))
            } else {
                Err(format!("That is {} KB, too big to show at once. It contains:\n{inside}\nRead one of those.", text.len() / 1024))
            };
        }
        Ok(text)
    }

    fn mcp_patch(&mut self, a: &Value) -> Result<String, String> {
        let ops = a.get("operations").cloned().ok_or("`operations` must be a list of JSON Patch operations.")?;
        let new = brainbend_doc::model_json::patch(self.project.as_mut().ok_or("No project is open.")?, &ops)?;
        let n = ops.as_array().map_or(0, Vec::len);
        let mut v = self.mcp_install(new, &format!("made {n} change{}", if n == 1 { "" } else { "s" }), slide_in(&ops), None);
        v["applied"] = json!(n);
        done(v)
    }

    fn mcp_create_object(&mut self, a: &Value, count: usize) -> Result<String, String> {
        let si = slide_arg(a, "slide", count)?;
        let layers = self.project.as_ref().unwrap().slides[si].layers.len();
        let li = match a.get("layer") { None | Some(Value::Null) => 0, Some(v) => v.as_u64().map(|n| n as usize).filter(|n| *n < layers).ok_or_else(|| format!("`layer` must be a layer number from 0 to {}.", layers - 1))? };
        let given = a.get("object").ok_or("`object` must be an object of properties.")?;
        let have = self.project.as_ref().unwrap().slides[si].layers[li].objects.len();
        let at = match a.get("index") { None | Some(Value::Null) => have, Some(v) => v.as_u64().map(|n| n as usize).filter(|n| *n <= have).ok_or_else(|| format!("`index` must be from 0 to {have}."))? };
        let id = given.get("id").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| self.new_id("obj"));
        let mut value = brainbend_doc::model_json::new_object(&id, given)?;
        value["id"] = json!(id);
        let ops = json!([{ "op": "add", "path": format!("/slides/{si}/layers/{li}/objects/{at}"), "value": value }]);
        let new = brainbend_doc::model_json::patch(self.project.as_mut().unwrap(), &ops)?;
        let mut v = self.mcp_install(new, &format!("added an object to slide {}", si + 1), Some(si), Some((si, li, at)));
        v["id"] = json!(id);
        v["path"] = json!(format!("/slides/{si}/layers/{li}/objects/{at}"));
        done(v)
    }

    /// `update_object` (`object` true) and `update_slide`: a merge patch of the fields in `set`.
    fn mcp_update(&mut self, a: &Value, count: usize, object: bool) -> Result<String, String> {
        let si = slide_arg(a, "slide", count)?;
        let set = a.get("set").filter(|s| s.is_object()).ok_or("`set` must be an object of the properties to change.")?;
        let (pointer, id) = if object {
            let which = text_arg(a, "object")?.ok_or("`object` must be an id or a name.")?;
            let slide = &self.project.as_ref().unwrap().slides[si];
            let (li, oi) = find_object(slide, &which)?;
            (format!("/slides/{si}/layers/{li}/objects/{oi}"), slide.layers[li].objects[oi].id.clone())
        } else {
            (format!("/slides/{si}"), self.project.as_ref().unwrap().slides[si].id.clone())
        };
        if set.get("id").is_some_and(|v| v.as_str() != Some(id.as_str())) { return Err("An id cannot be changed.".into()); }
        let new = brainbend_doc::model_json::merge(self.project.as_mut().unwrap(), &pointer, set)?;
        let what = if object { format!("changed an object on slide {}", si + 1) } else { format!("changed slide {}", si + 1) };
        let select = if object { new.slides[si].layers.iter().enumerate().find_map(|(li, l)| l.objects.iter().position(|o| o.id == id).map(|oi| (si, li, oi))) } else { None };
        let described = if object { select.map(|(_, li, oi)| { let mut d = describe_object(&new.slides[si].layers[li].objects[oi]); d["path"] = json!(format!("/slides/{si}/layers/{li}/objects/{oi}")); d }) } else { None };
        let mut v = self.mcp_install(new, &what, Some(si), select);
        if let Some(d) = described { v["object"] = d; } else { v["slide"] = json!(si + 1); }
        done(v)
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
        assert_eq!(t.len(), 19);
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

    fn text_run(a: &mut App, name: &str, args: Value) -> Result<String, String> { a.mcp_run(name, &args) }

    #[test]
    fn the_model_is_described_and_any_part_of_it_can_be_read() {
        let mut a = app_with(2);
        let overview = text_run(&mut a, "describe_model", json!({})).unwrap();
        assert!(overview.contains("JSON Pointers") && overview.contains("Entrance"), "{overview}");
        let object = text_run(&mut a, "describe_model", json!({ "type": "Object" })).unwrap();
        assert!(object.contains("entrance") && object.contains("timing"), "{object}");
        assert!(text_run(&mut a, "describe_model", json!({ "type": "Entranc" })).unwrap_err().contains("Did you mean"));
        let name = text_run(&mut a, "read", json!({ "path": "/slides/1/name" })).unwrap();
        assert_eq!(name.trim(), "\"Slide 2\"");
        let whole = text_run(&mut a, "read", json!({})).unwrap();
        assert!(whole.contains("/slides (") && whole.contains("/theme (") && !whole.contains("\"layers\""), "with no path it outlines the project: {whole}");
        assert!(text_run(&mut a, "read", json!({ "path": "/nothing" })).unwrap_err().contains("nothing at"));
        assert!(!a.dirty && a.undo.is_empty(), "reading changes nothing");
    }

    #[test]
    fn something_too_big_to_show_is_outlined_instead() {
        let mut a = app_with(1);
        let long = "x".repeat(40_000);
        run(&mut a, "set_slide_notes", json!({ "slide": 1, "notes": long })).unwrap();
        let e = text_run(&mut a, "read", json!({ "path": "/slides/0" })).unwrap_err();
        assert!(e.contains("too big") && e.contains("/slides/0/notes"), "{e}");
    }

    #[test]
    fn a_patch_changes_anything_in_one_undoable_step() {
        let mut a = app_with(2);
        let r = run(&mut a, "patch", json!({ "operations": [
            { "op": "replace", "path": "/slides/1/name", "value": "Patched" },
            { "op": "add", "path": "/slides/1/notes", "value": "A note" },
            { "op": "replace", "path": "/pass_mark", "value": 70 },
        ] })).unwrap();
        assert_eq!((r["ok"].as_bool(), r["applied"].as_u64()), (Some(true), Some(3)));
        let p = a.project.as_ref().unwrap();
        assert_eq!((p.slides[1].name.as_str(), p.slides[1].notes.as_str(), p.pass_mark), ("Patched", "A note", 70));
        assert_eq!(a.slide, 1, "the slide it worked on is on screen");
        assert!(a.dirty);
        run(&mut a, "undo", json!({})).unwrap();
        let p = a.project.as_ref().unwrap();
        assert_eq!((p.slides[1].name.as_str(), p.slides[1].notes.as_str(), p.pass_mark), ("Slide 2", "", 80), "one undo takes back all three");
    }

    #[test]
    fn a_patch_that_is_wrong_changes_nothing_and_leaves_no_undo_step() {
        let mut a = app_with(1);
        for (ops, wants) in [
            (json!([{ "op": "add", "path": "/slides/0/layers/0/objects/-", "value": { "id": "x", "colour": "red" } }]), "invalid"),
            (json!([{ "op": "replace", "path": "/slides/0/name", "value": 5 }]), "/slides/0/name"),
            (json!([{ "op": "remove", "path": "/slides/9" }]), "could not be applied"),
            (json!([{ "op": "remove", "path": "/assets" }]), "cannot be changed"),
            (json!("not a list"), "list"),
        ] {
            let e = run(&mut a, "patch", json!({ "operations": ops })).unwrap_err();
            assert!(e.contains(wants), "{e} should mention {wants}");
        }
        assert!(!a.dirty && a.undo.is_empty(), "no failed patch is recorded");
        assert_eq!(a.project.as_ref().unwrap().slides[0].name, "Slide 1");
    }

    #[test]
    fn objects_of_any_kind_are_created_with_everything_they_can_have() {
        let mut a = app_with(1);
        let r = run(&mut a, "create_object", json!({ "slide": 1, "object": {
            "kind": "Ellipse", "name": "Dot", "x": 50, "y": 60, "w": 120, "h": 120, "fill": [200, 30, 30, 255],
            "shadow": null,
            "entrance": { "kind": "Zoom", "delay_ms": 0, "duration_ms": 500 },
            "timing": { "start_ms": 1000, "dur_ms": 3000 },
            "style": { "size": 30, "bold": true },
        } })).unwrap();
        let id = r["id"].as_str().unwrap().to_string();
        assert_eq!(r["path"], "/slides/0/layers/0/objects/0");
        let o = &a.project.as_ref().unwrap().slides[0].layers[0].objects[0];
        assert_eq!((o.id.as_str(), o.name.as_str(), o.x, o.w), (id.as_str(), "Dot", 50.0, 120.0));
        assert!(matches!(o.kind, Kind::Ellipse) && o.entrance.is_some() && o.timing == Some(Timing { start_ms: 1000, dur_ms: Some(3000) }));
        assert_eq!((o.style.size, o.style.bold), (30.0, true));
        assert_eq!(a.selected, Some((0, 0)), "it is selected, so the author sees it");
        let second = run(&mut a, "create_object", json!({ "slide": 1, "index": 0, "object": { "name": "Behind", "kind": "Rect" } })).unwrap();
        assert_eq!(second["path"], "/slides/0/layers/0/objects/0", "an index puts it there");
        assert_eq!(a.project.as_ref().unwrap().slides[0].layers[0].objects[1].name, "Dot");
        let g = run(&mut a, "get_slide", json!({ "slide": 1 })).unwrap();
        assert_eq!(g["layers"][0]["objects"][1]["path"], "/slides/0/layers/0/objects/1", "get_slide gives each object's path");
    }

    #[test]
    fn creating_an_object_checks_what_it_is_given() {
        let mut a = app_with(1);
        assert!(run(&mut a, "create_object", json!({ "slide": 1, "object": { "kind": "Teleporter" } })).unwrap_err().contains("invalid"));
        assert!(run(&mut a, "create_object", json!({ "slide": 1, "object": { "nonsense": 1 } })).unwrap_err().contains("nonsense"));
        assert!(run(&mut a, "create_object", json!({ "slide": 1, "layer": 4, "object": {} })).unwrap_err().contains("layer number"));
        assert!(run(&mut a, "create_object", json!({ "slide": 1, "index": 9, "object": {} })).unwrap_err().contains("`index`"));
        assert!(run(&mut a, "create_object", json!({ "slide": 1, "object": "text" })).is_err());
        assert!(a.project.as_ref().unwrap().slides[0].layers[0].objects.is_empty());
        let first = run(&mut a, "create_object", json!({ "slide": 1, "object": { "id": "mine" } })).unwrap();
        assert_eq!(first["id"], "mine", "an id can be chosen");
        assert!(run(&mut a, "create_object", json!({ "slide": 1, "object": { "id": "mine" } })).unwrap_err().contains("Two objects"), "but not one that is taken");
    }

    #[test]
    fn an_object_can_be_restyled_animated_and_timed_by_name_or_id() {
        let mut a = app_with(1);
        let id = run(&mut a, "create_object", json!({ "slide": 1, "object": { "kind": "Text", "name": "Headline", "text": "Hi" } })).unwrap()["id"].as_str().unwrap().to_string();
        let r = run(&mut a, "update_object", json!({ "slide": 1, "object": "Headline", "set": {
            "fill": [10, 20, 30, 255], "style": { "size": 90, "color": [255, 255, 255, 255], "align": "Center" },
            "entrance": { "kind": "Fade", "delay_ms": 100, "duration_ms": 700 },
            "emphasis": { "kind": "Pulse", "delay_ms": 0, "duration_ms": 900, "repeat": 3 },
            "exit": { "kind": "SlideLeft", "delay_ms": 0, "duration_ms": 400 },
            "timing": { "start_ms": 500, "dur_ms": 4000 },
            "shadow": { "color": [0, 0, 0, 120], "dx": 4.0, "dy": 4.0, "blur": 8.0 },
        } })).unwrap();
        assert_eq!(r["object"]["id"], id.as_str());
        let o = &a.project.as_ref().unwrap().slides[0].layers[0].objects[0];
        assert_eq!((o.text.as_str(), o.style.size, o.style.align, o.fill), ("Hi", 90.0, Align::Center, Some(Rgba(10, 20, 30, 255))), "what was not set stays");
        assert!(o.entrance.is_some() && o.emphasis.is_some() && o.exit.is_some() && o.shadow.is_some());
        assert_eq!(o.timing, Some(Timing { start_ms: 500, dur_ms: Some(4000) }));
        run(&mut a, "update_object", json!({ "slide": 1, "object": id, "set": { "entrance": null, "timing": null } })).unwrap();
        let o = &a.project.as_ref().unwrap().slides[0].layers[0].objects[0];
        assert!(o.entrance.is_none() && o.timing.is_none(), "null takes a property away");
        assert!(run(&mut a, "update_object", json!({ "slide": 1, "object": id, "set": { "id": "other" } })).unwrap_err().contains("id cannot be changed"));
        assert!(run(&mut a, "update_object", json!({ "slide": 1, "object": id, "set": { "style": { "size": "huge" } } })).unwrap_err().contains("invalid"));
        assert!(run(&mut a, "update_object", json!({ "slide": 1, "object": id, "set": [] })).is_err());
    }

    #[test]
    fn a_slide_can_be_given_a_transition_and_triggers() {
        let mut a = app_with(2);
        let layer = run(&mut a, "patch", json!({ "operations": [{ "op": "add", "path": "/slides/1/layers/-", "value": {
            "id": "pop", "name": "Pop-up", "objects": [], "duration_ms": null, "modal": true, "hide_others": false, "pause_base": false
        } }] })).unwrap();
        assert_eq!(layer["ok"], true);
        let r = run(&mut a, "update_slide", json!({ "slide": 2, "set": { "fade_ms": 600, "transition": { "kind": "Wipe", "from": "Left", "inward": false } } })).unwrap();
        assert_eq!(r["slide"], 2);
        let s = &a.project.as_ref().unwrap().slides[1];
        assert_eq!((s.fade_ms, s.transition.kind), (600, TransitionKind::Wipe));
        let t = run(&mut a, "patch", json!({ "operations": [{ "op": "add", "path": "/slides/1/triggers/-", "value": {
            "id": "t1", "owner": "Slide", "event": "SlideStart", "action": { "ShowLayer": "pop" }, "conditions": []
        } }] })).unwrap();
        assert_eq!(a.project.as_ref().unwrap().slides[1].triggers.len(), 1);
        assert_eq!(t["warnings"].as_array().unwrap().len(), 0, "a trigger that shows a layer that exists is fine");
        assert!(run(&mut a, "update_slide", json!({ "slide": 2, "set": { "id": "x" } })).is_err());
    }

    #[test]
    fn a_change_that_newly_breaks_something_comes_back_with_a_warning() {
        let mut a = app_with(1);
        let r = run(&mut a, "patch", json!({ "operations": [{ "op": "add", "path": "/slides/0/triggers/-", "value": {
            "id": "t1", "owner": "Slide", "event": "SlideStart", "action": { "ShowLayer": "no-such-layer" }, "conditions": []
        } }] })).unwrap();
        let w = r["warnings"].as_array().unwrap();
        assert!(!w.is_empty(), "the checker found a trigger pointing at nothing: {r}");
        assert!(w[0].as_str().unwrap().starts_with("slide 1: "), "{w:?}");
        let again = run(&mut a, "patch", json!({ "operations": [{ "op": "replace", "path": "/slides/0/name", "value": "Renamed" }] })).unwrap();
        assert_eq!(again["warnings"].as_array().unwrap().len(), 0, "only what the change broke is reported, not what was already wrong");
    }

    #[test]
    fn the_models_enum_values_are_found_by_describing_the_type() {
        let mut a = app_with(1);
        let kinds = text_run(&mut a, "describe_model", json!({ "type": "Kind" })).unwrap();
        assert!(kinds.contains("Rect") && kinds.contains("Video"), "{kinds}");
        let trigger = text_run(&mut a, "describe_model", json!({ "type": "Trigger" })).unwrap();
        assert!(trigger.contains("event") && trigger.contains("action") && trigger.contains("conditions"), "{trigger}");
        assert!(text_run(&mut a, "describe_model", json!({ "type": "Action" })).unwrap().contains("ShowLayer"));
    }

}
