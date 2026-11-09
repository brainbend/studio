//! The document model as JSON, for AI tools: a schema that says exactly what can be set, strict checking of a change
//! before it is made, and atomic edits written as JSON Patch (RFC 6902) or JSON Merge Patch (RFC 7396).
//!
//! The model ignores fields it does not know, so that files from newer versions still open. An AI tool must not get that
//! kindness: a misspelt `colour` would change nothing and look as though it had worked. So an edit is checked against a
//! schema made from the real types, in which an unknown field is an error that names where it is.
//!
//! Only the project's content can be edited this way, not its assets (pictures, video and audio are added elsewhere) nor
//! the few things the editor's undo does not cover.
use crate::*;
use serde_json::{json, Value};
use std::sync::OnceLock;

/// The parts of a project a patch may not touch: what undo does not bring back, and the assets.
const FORBIDDEN: [&str; 7] = ["/assets", "/format", "/kind", "/recent_colors", "/custom_words", "/notes", "/lesson"];
/// The most operations in one patch.
const MAX_OPS: usize = 200;
/// The most problems named in one error, so that a long one stays readable.
const MAX_PROBLEMS: usize = 8;

fn strict(v: &mut Value) {
    match v {
        Value::Object(map) => {
            if map.contains_key("properties") && !map.contains_key("additionalProperties") { map.insert("additionalProperties".into(), Value::Bool(false)); }
            for child in map.values_mut() { strict(child); }
        }
        Value::Array(items) => items.iter_mut().for_each(strict),
        _ => {}
    }
}

/// The JSON Schema of a project, with unknown fields not allowed.
pub fn schema() -> &'static Value {
    static SCHEMA: OnceLock<Value> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        let mut v = serde_json::to_value(schemars::schema_for!(Project)).unwrap_or(Value::Null);
        strict(&mut v);
        v
    })
}

fn validator() -> &'static jsonschema::Validator {
    static VALIDATOR: OnceLock<jsonschema::Validator> = OnceLock::new();
    VALIDATOR.get_or_init(|| jsonschema::validator_for(schema()).expect("the project schema is valid"))
}

/// The project as JSON, without its assets (their bytes would be a long list of numbers).
pub fn project_value(p: &mut Project) -> Value {
    let assets = std::mem::take(&mut p.assets);
    let v = serde_json::to_value(&*p).unwrap_or(Value::Null);
    p.assets = assets;
    v
}

/// The part of a project at `pointer` (JSON Pointer, RFC 6901: `/slides/0/layers/0/objects/2`; `""` is the whole project).
pub fn read(p: &mut Project, pointer: &str) -> Result<Value, String> {
    let v = project_value(p);
    v.pointer(pointer).cloned().ok_or_else(|| format!("There is nothing at {pointer:?}. Paths start with / and count from 0, such as /slides/0/layers/0/objects/1/fill."))
}

fn touches_forbidden(path: &str) -> Option<&'static str> {
    FORBIDDEN.iter().find(|f| path == **f || path.starts_with(&format!("{f}/"))).copied()
}

/// What is wrong with `value`, as a project: each problem with where it is, or `None` when it is fine.
fn problems(value: &Value) -> Option<String> {
    let errors: Vec<String> = validator().iter_errors(value).map(|e| {
        let at = e.instance_path.to_string();
        format!("{} {}", if at.is_empty() { "(project)".to_string() } else { at }, e)
    }).collect();
    if errors.is_empty() { return None; }
    let more = errors.len().saturating_sub(MAX_PROBLEMS);
    let mut text = errors.into_iter().take(MAX_PROBLEMS).collect::<Vec<_>>().join("\n");
    if more > 0 { text.push_str(&format!("\n… and {more} more")); }
    Some(text)
}

/// Ids must be unique where the model looks things up by id: slides in the project, layers and objects on a slide.
fn check_ids(p: &Project) -> Result<(), String> {
    let mut slides = std::collections::HashSet::new();
    for (si, s) in p.slides.iter().enumerate() {
        if !slides.insert(&s.id) { return Err(format!("Two slides have the id {:?} (slide {}). Every slide needs its own id.", s.id, si + 1)); }
        let mut layers = std::collections::HashSet::new();
        let mut objects = std::collections::HashSet::new();
        for l in &s.layers {
            if !layers.insert(&l.id) { return Err(format!("Two layers on slide {} have the id {:?}. Every layer needs its own id.", si + 1, l.id)); }
            for o in &l.objects {
                if !objects.insert(&o.id) { return Err(format!("Two objects on slide {} have the id {:?}. Every object needs its own id.", si + 1, o.id)); }
            }
        }
    }
    Ok(())
}

/// Makes `value` into a project (with `assets` put back), after checking it against the schema.
fn build(value: Value, old: &mut Project) -> Result<Project, String> {
    if let Some(p) = problems(&value) { return Err(format!("The change would make the project invalid:\n{p}\nUse describe_model with a type name to see what each field takes.")); }
    let mut new: Project = serde_json::from_value(value).map_err(|e| format!("The change would make the project unreadable: {e}"))?;
    check_ids(&new)?;
    new.assets = std::mem::take(&mut old.assets);
    new.sanitize();
    Ok(new)
}

/// Applies a JSON Patch (a list of `add`, `remove`, `replace`, `move`, `copy` and `test` operations) and returns the
/// project it makes, or why it cannot. Nothing changes unless every operation works and the result is valid.
pub fn patch(old: &mut Project, operations: &Value) -> Result<Project, String> {
    let ops = operations.as_array().ok_or("`operations` must be a list of JSON Patch operations, such as [{\"op\":\"replace\",\"path\":\"/slides/0/name\",\"value\":\"Intro\"}].")?;
    if ops.is_empty() { return Err("`operations` is empty.".into()); }
    if ops.len() > MAX_OPS { return Err(format!("Send at most {MAX_OPS} operations at a time.")); }
    for (i, op) in ops.iter().enumerate() {
        for key in ["path", "from"] {
            let Some(path) = op.get(key).and_then(Value::as_str) else { continue };
            if path.is_empty() { return Err(format!("Operation {i}: the whole project cannot be replaced; change the parts of it.")); }
            if let Some(f) = touches_forbidden(path) { return Err(format!("Operation {i}: {f} cannot be changed this way.")); }
        }
    }
    let list: json_patch::Patch = serde_json::from_value(operations.clone()).map_err(|e| format!("These are not valid JSON Patch operations: {e}"))?;
    let mut value = project_value(old);
    for (i, op) in list.0.iter().enumerate() {
        if json_patch::patch(&mut value, std::slice::from_ref(op)).is_ok() { continue; }
        // A field at its default is left out of the JSON, so there is nothing there to replace; setting it is what was meant.
        if let json_patch::PatchOperation::Replace(r) = op {
            let add = json_patch::PatchOperation::Add(json_patch::AddOperation { path: r.path.clone(), value: r.value.clone() });
            if json_patch::patch(&mut value, std::slice::from_ref(&add)).is_ok() { continue; }
        }
        let why = json_patch::patch(&mut value, std::slice::from_ref(op)).err().map(|e| e.to_string()).unwrap_or_default();
        return Err(format!("The patch could not be applied: operation {i} ({}) failed: {why}", ops[i]["op"].as_str().unwrap_or("?")));
    }
    build(value, old)
}

/// Changes the JSON at `pointer` by a JSON Merge Patch: the fields given are set, a field given as `null` is removed, and an
/// object is merged into rather than replaced. Returns the project it makes.
pub fn merge(old: &mut Project, pointer: &str, changes: &Value) -> Result<Project, String> {
    if pointer.is_empty() { return Err("The whole project cannot be replaced; change the parts of it.".into()); }
    if let Some(f) = touches_forbidden(pointer) { return Err(format!("{f} cannot be changed this way.")); }
    let mut value = project_value(old);
    let target = value.pointer_mut(pointer).ok_or_else(|| format!("There is nothing at {pointer:?}."))?;
    json_patch::merge(target, changes);
    build(value, old)
}

/// A new object as JSON: the defaults of its kind with `given` merged over them. `id` is used when `given` has none.
pub fn new_object(id: &str, given: &Value) -> Result<Value, String> {
    let mut base = serde_json::to_value(Object::new(id, "Object", Kind::Rect, 100.0, 100.0, 300.0, 200.0)).map_err(|e| e.to_string())?;
    if !given.is_object() { return Err("`object` must be an object of properties.".into()); }
    json_patch::merge(&mut base, given);
    Ok(base)
}

/// A new slide as JSON, like [`new_object`].
pub fn new_slide(id: &str, name: &str, width: f32, height: f32, given: &Value) -> Result<Value, String> {
    let mut base = serde_json::to_value(Slide::new(id, name, width, height)).map_err(|e| e.to_string())?;
    if !given.is_object() { return Err("`slide` must be an object of properties.".into()); }
    json_patch::merge(&mut base, given);
    Ok(base)
}

// ---- telling an AI tool what can be set ----

fn first_sentence(text: &str) -> String {
    let t = text.trim().replace('\n', " ");
    match t.find(". ") { Some(i) => t[..=i].to_string(), None => t }
}

/// A type or field written the short way: `number`, `Rgba or null`, `list of Object`.
fn ty(s: &Value) -> String {
    if let Some(r) = s.get("$ref").and_then(Value::as_str) { return r.rsplit('/').next().unwrap_or("?").to_string(); }
    if let Some(any) = s.get("anyOf").and_then(Value::as_array) { return any.iter().map(ty).collect::<Vec<_>>().join(" or "); }
    if let Some(vals) = s.get("enum").and_then(Value::as_array) { return vals.iter().filter_map(Value::as_str).map(|v| format!("\"{v}\"")).collect::<Vec<_>>().join(" | "); }
    if let Some(c) = s.get("const") { return c.to_string(); }
    if s.get("oneOf").is_some() { return "one of its variants (see the type)".into(); }
    let kind = s.get("type");
    let mut names: Vec<&str> = match kind { Some(Value::String(t)) => vec![t.as_str()], Some(Value::Array(a)) => a.iter().filter_map(Value::as_str).collect(), _ => vec![] };
    if names.len() == 1 && names[0] == "array" {
        if let Some(prefix) = s.get("prefixItems").and_then(Value::as_array) { return format!("[{}]", prefix.iter().map(ty).collect::<Vec<_>>().join(", ")); }
        return format!("list of {}", s.get("items").map(ty).unwrap_or_else(|| "anything".into()));
    }
    if names.len() == 1 && names[0] == "object" {
        if let Some(extra) = s.get("additionalProperties").filter(|a| a.is_object()) { return format!("map of text to {}", ty(extra)); }
        return "object".into();
    }
    if names.is_empty() { return "anything".into(); }
    names.sort_by_key(|n| *n == "null");
    names.iter().map(|n| if *n == "integer" { "whole number" } else { n }).collect::<Vec<_>>().join(" or ")
}

fn describe_def(name: &str, def: &Value) -> String {
    let mut out = name.to_string();
    if let Some(d) = def.get("description").and_then(Value::as_str) { out.push_str(&format!(" — {}", d.trim().replace('\n', " "))); }
    out.push('\n');
    if let Some(props) = def.get("properties").and_then(Value::as_object) {
        let required: Vec<&str> = def.get("required").and_then(Value::as_array).map(|r| r.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
        out.push_str("Fields (a field marked * must be given; the others have a default):\n");
        for (k, v) in props {
            out.push_str(&format!("  {k}{}: {}", if required.contains(&k.as_str()) { "*" } else { "" }, ty(v)));
            if let Some(d) = v.get("description").and_then(Value::as_str) { out.push_str(&format!(" — {}", d.trim().replace('\n', " "))); }
            out.push('\n');
        }
    } else if let Some(variants) = def.get("oneOf").and_then(Value::as_array) {
        out.push_str("One of:\n");
        for v in variants {
            let doc = v.get("description").and_then(Value::as_str).map(|d| format!(" — {}", d.trim().replace('\n', " "))).unwrap_or_default();
            if let Some(vals) = v.get("enum").and_then(Value::as_array) {
                out.push_str(&format!("  {}\n", vals.iter().filter_map(Value::as_str).map(|x| format!("\"{x}\"")).collect::<Vec<_>>().join(", ")));
            } else if let Some(c) = v.get("const") {
                out.push_str(&format!("  {c}{doc}\n"));
            } else if let Some(props) = v.get("properties").and_then(Value::as_object) {
                for (k, inner) in props { out.push_str(&format!("  {{\"{k}\": {}}}{doc}\n", ty(inner))); }
            } else {
                out.push_str(&format!("  {}{doc}\n", ty(v)));
            }
        }
    } else if let Some(vals) = def.get("enum").and_then(Value::as_array) {
        out.push_str(&format!("One of: {}\n", vals.iter().filter_map(Value::as_str).map(|v| format!("\"{v}\"")).collect::<Vec<_>>().join(", ")));
    } else {
        out.push_str(&format!("A {}.\n", ty(def)));
    }
    out
}

/// How the model is written in JSON, for an AI tool that is about to read or change it.
pub const GUIDE: &str = "\
The project is JSON. Slides are in /slides; each slide has /layers (layer 0 is the base layer) and each layer has /objects; \
an object's frame is x, y, w, h in slide pixels from the top left. Paths are JSON Pointers and count from 0 \
(/slides/0/layers/0/objects/2/fill is the third object on the first slide). Times are in milliseconds. \
A color is a list of four numbers [red, green, blue, alpha], each 0 to 255. A type that is one of several variants is written \
as a bare string for a variant with no data (\"Rect\") and as an object with the variant's name for one with data ({\"Path\": [...]}). \
Every object, slide and layer needs an id that is unique in the project.\n\
To look: read (a path), get_slide, list_slides. To change: update_object or update_slide with the fields to set (a field set to null is removed; an \
object is merged into, not replaced), create_object to add one, or patch for anything else, written as JSON Patch operations \
([{\"op\":\"add\",\"path\":\"/slides/0/triggers/-\",\"value\":{...}}], where - means the end of a list; add also sets a field, replacing it if it is there, and a field at its default may be missing from what you read, so set it with add or replace). A change is checked against the model \
before it is made: nothing changes unless all of it is valid, and an error says where. Fields the model does not have are errors, so a \
misspelt name is caught. Use describe_model with a type name (for example Object, Entrance, Trigger, Action, Event, TextStyle) to see its \
fields and what each takes. Pictures, video and audio files cannot be added this way yet.";

/// An overview of the model (no `name`), or one type in detail.
pub fn describe(name: Option<&str>) -> Result<String, String> {
    let defs = schema().get("$defs").and_then(Value::as_object).ok_or("the schema has no types")?;
    let Some(name) = name.map(str::trim).filter(|n| !n.is_empty()) else {
        let mut out = format!("{GUIDE}\n\nThe types (call describe_model with one to see its fields):\n");
        let mut names: Vec<&String> = defs.keys().collect();
        names.sort();
        out.push_str("  Project (the whole document): ");
        out.push_str(&schema().get("description").and_then(Value::as_str).map(first_sentence).unwrap_or_default());
        out.push('\n');
        for n in names { out.push_str(&format!("  {n}{}\n", defs[n].get("description").and_then(Value::as_str).map(|d| format!(" — {}", first_sentence(d))).unwrap_or_default())); }
        return Ok(out);
    };
    if name == "Project" {
        let mut root = schema().clone();
        if let Some(m) = root.as_object_mut() { m.remove("$defs"); }
        return Ok(describe_def("Project", &root));
    }
    match defs.get(name) {
        Some(def) => Ok(describe_def(name, def)),
        None => {
            let lower = name.to_ascii_lowercase();
            let near: Vec<&str> = defs.keys().map(String::as_str).filter(|k| { let kl = k.to_ascii_lowercase(); kl.contains(&lower) || lower.contains(&kl) }).take(6).collect();
            Err(format!("There is no type called {name}.{}", if near.is_empty() { " Call describe_model with no name to list them.".to_string() } else { format!(" Did you mean {}?", near.join(", ")) }))
        }
    }
}

/// A short summary of how a patch went, for the tool's answer.
pub fn summary(old: &mut Project, new: &mut Project) -> Value {
    let (a, b) = (project_value(old), project_value(new));
    json!({ "changed": a != b })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> Project {
        let mut p = Project::new("Demo");
        p.slides = vec![Slide::new("s1", "Intro", 1280.0, 720.0), Slide::new("s2", "Second", 1280.0, 720.0)];
        p.sanitize();
        let mut o = Object::new("o1", "Title", Kind::Text, 10.0, 20.0, 400.0, 100.0);
        o.text = "Hello".into();
        p.slides[0].layers[0].objects.push(o);
        p.assets.insert("pic".into(), Asset { name: "pic.png".into(), mime: "image/png".into(), bytes: vec![1, 2, 3], file: None });
        p
    }

    #[test]
    fn the_schema_knows_the_types_and_refuses_fields_it_does_not_have() {
        let s = schema();
        assert!(s["$defs"].as_object().unwrap().len() > 60);
        assert_eq!(s["additionalProperties"], false);
        assert_eq!(s["$defs"]["Object"]["additionalProperties"], false, "an unknown field is an error");
        assert!(s["$defs"]["EffectKind"].to_string().contains("Fade"));
    }

    #[test]
    fn a_project_is_read_by_path_without_its_assets() {
        let mut p = project();
        assert_eq!(read(&mut p, "/slides/0/name").unwrap(), json!("Intro"));
        assert_eq!(read(&mut p, "/slides/0/layers/0/objects/0/text").unwrap(), json!("Hello"));
        assert!(read(&mut p, "/slides/9").unwrap_err().contains("nothing at"));
        let whole = project_value(&mut p);
        assert!(!whole.to_string().contains("[1,2,3]"), "no asset bytes");
        assert_eq!(p.assets["pic"].bytes, vec![1, 2, 3], "and the assets are still in the project");
    }

    #[test]
    fn a_valid_patch_makes_a_new_project_that_keeps_the_assets() {
        let mut p = project();
        let new = patch(&mut p, &json!([
            { "op": "replace", "path": "/slides/0/name", "value": "Welcome" },
            { "op": "replace", "path": "/slides/0/layers/0/objects/0/fill", "value": [255, 0, 0, 255] },
            { "op": "add", "path": "/slides/1/layers/0/objects/-", "value": new_object("o2", &json!({ "name": "Box", "x": 5 })).unwrap() },
        ])).unwrap();
        assert_eq!(new.slides[0].name, "Welcome");
        assert_eq!(new.slides[0].layers[0].objects[0].fill, Some(Rgba(255, 0, 0, 255)));
        assert_eq!((new.slides[1].layers[0].objects[0].name.as_str(), new.slides[1].layers[0].objects[0].x), ("Box", 5.0));
        assert_eq!(new.assets["pic"].bytes, vec![1, 2, 3], "the pictures came along");
        assert_eq!(p.slides[0].name, "Intro", "and the old project was not touched");
    }

    #[test]
    fn a_misspelt_or_wrong_field_is_an_error_that_says_where() {
        let mut p = project();
        let e = patch(&mut p, &json!([{ "op": "add", "path": "/slides/0/layers/0/objects/0/colour", "value": [1, 2, 3, 255] }])).unwrap_err();
        assert!(e.contains("/slides/0/layers/0/objects/0") && e.contains("colour"), "{e}");
        let e = patch(&mut p, &json!([{ "op": "replace", "path": "/slides/0/layers/0/objects/0/fill", "value": "red" }])).unwrap_err();
        assert!(e.contains("/slides/0/layers/0/objects/0/fill"), "{e}");
        let e = patch(&mut p, &json!([{ "op": "replace", "path": "/slides/0/layers/0/objects/0/w", "value": "wide" }])).unwrap_err();
        assert!(e.contains("objects/0/w"), "{e}");
        let e = patch(&mut p, &json!([{ "op": "replace", "path": "/slides/0/layers/0/objects/0/entrance", "value": { "kind": "Teleport", "delay_ms": 0, "duration_ms": 500 } }])).unwrap_err();
        assert!(e.contains("entrance"), "{e}");
        assert_eq!(read(&mut p, "/slides/0/layers/0/objects/0/text").unwrap(), json!("Hello"), "nothing changed");
    }

    #[test]
    fn replacing_a_field_that_is_at_its_default_and_so_missing_sets_it() {
        let mut p = project();
        assert!(read(&mut p, "/slides/0/transition").is_err(), "a default transition is left out of the JSON");
        let new = patch(&mut p, &json!([{ "op": "replace", "path": "/slides/0/transition", "value": { "kind": "Zoom", "from": "Right", "inward": true } }])).unwrap();
        assert_eq!(new.slides[0].transition.kind, TransitionKind::Zoom);
        let e = patch(&mut p, &json!([{ "op": "replace", "path": "/slides/0/nonsense/deeper", "value": 1 }])).unwrap_err();
        assert!(e.contains("operation 0 (replace) failed"), "a path that cannot be made is still an error: {e}");
    }

    #[test]
    fn a_change_that_cannot_work_changes_nothing() {
        let mut p = project();
        let e = patch(&mut p, &json!([
            { "op": "replace", "path": "/slides/0/name", "value": "Changed first" },
            { "op": "remove", "path": "/slides/7" },
        ])).unwrap_err();
        assert!(e.contains("could not be applied"), "{e}");
        assert_eq!(p.slides[0].name, "Intro");
        assert!(patch(&mut p, &json!([{ "op": "test", "path": "/slides/0/name", "value": "Nope" }, { "op": "replace", "path": "/slides/0/name", "value": "X" }])).is_err(), "a failed test stops the patch");
    }

    #[test]
    fn two_things_with_the_same_id_are_refused() {
        let mut p = project();
        let dup = new_object("o1", &json!({})).unwrap();
        let e = patch(&mut p, &json!([{ "op": "add", "path": "/slides/0/layers/0/objects/-", "value": dup }])).unwrap_err();
        assert!(e.contains("Two objects on slide 1") && e.contains("o1"), "{e}");
        let slide = new_slide("s1", "Again", 1280.0, 720.0, &json!({})).unwrap();
        let e = patch(&mut p, &json!([{ "op": "add", "path": "/slides/-", "value": slide }])).unwrap_err();
        assert!(e.contains("Two slides"), "{e}");
        let other = new_slide("s3", "Third", 1280.0, 720.0, &json!({})).unwrap();
        assert!(patch(&mut p, &json!([{ "op": "add", "path": "/slides/-", "value": other }])).is_ok(), "a new id is fine");
    }

    #[test]
    fn what_a_patch_may_not_touch_is_refused() {
        let mut p = project();
        for path in ["/assets", "/assets/pic", "/format", "/kind", "/recent_colors", "/notes", "/lesson"] {
            let e = patch(&mut p, &json!([{ "op": "remove", "path": path }])).unwrap_err();
            assert!(e.contains("cannot be changed"), "{path}: {e}");
        }
        assert!(patch(&mut p, &json!([{ "op": "move", "from": "/assets/pic", "path": "/slides/0/name" }])).unwrap_err().contains("cannot be changed"), "moving out of assets too");
        assert!(patch(&mut p, &json!([{ "op": "replace", "path": "", "value": {} }])).unwrap_err().contains("whole project"));
        assert!(patch(&mut p, &json!([])).is_err());
        assert!(patch(&mut p, &json!({ "op": "add" })).is_err(), "a list is needed");
        assert!(patch(&mut p, &json!(vec![json!({ "op": "test", "path": "/slides/0/name", "value": "Intro" }); MAX_OPS + 1])).unwrap_err().contains("at most"));
    }

    #[test]
    fn a_merge_sets_what_is_given_removes_what_is_null_and_keeps_the_rest() {
        let mut p = project();
        let new = merge(&mut p, "/slides/0/layers/0/objects/0", &json!({ "x": 99, "text": "Hi", "fill": [0, 128, 0, 255], "style": { "size": 72 } })).unwrap();
        let o = &new.slides[0].layers[0].objects[0];
        assert_eq!((o.x, o.text.as_str(), o.fill, o.style.size), (99.0, "Hi", Some(Rgba(0, 128, 0, 255)), 72.0));
        assert_eq!((o.y, o.w, o.style.font_family.as_str()), (20.0, 400.0, "Open Sans"), "what was not given stays, even inside style");
        let cleared = merge(&mut p, "/slides/0/layers/0/objects/0", &json!({ "timing": { "start_ms": 500, "dur_ms": 1000 } })).unwrap();
        assert_eq!(cleared.slides[0].layers[0].objects[0].timing, Some(Timing { start_ms: 500, dur_ms: Some(1000) }));
        assert!(merge(&mut p, "/slides/0/layers/0/objects/0", &json!({ "bogus": 1 })).unwrap_err().contains("bogus"));
        assert!(merge(&mut p, "/slides/5", &json!({})).unwrap_err().contains("nothing at"));
        assert!(merge(&mut p, "/assets/pic", &json!({ "name": "x" })).is_err());
    }

    #[test]
    fn animation_triggers_and_states_can_be_set_with_the_shapes_the_model_describes() {
        let mut p = project();
        let new = patch(&mut p, &json!([
            { "op": "replace", "path": "/slides/0/layers/0/objects/0/entrance", "value": { "kind": "Fade", "delay_ms": 200, "duration_ms": 600 } },
            { "op": "replace", "path": "/slides/0/layers/0/objects/0/emphasis", "value": { "kind": "Pulse", "delay_ms": 0, "duration_ms": 800, "repeat": 2 } },
            { "op": "replace", "path": "/slides/0/layers/0/objects/0/timing", "value": { "start_ms": 1000, "dur_ms": null } },
            { "op": "replace", "path": "/slides/0/fade_ms", "value": 500 },
            { "op": "replace", "path": "/slides/0/transition", "value": { "kind": "Push", "from": "Left", "inward": false } },
        ])).unwrap();
        let o = &new.slides[0].layers[0].objects[0];
        assert!(o.entrance.is_some() && o.emphasis.is_some());
        assert_eq!((new.slides[0].fade_ms, new.slides[0].transition.kind), (500, TransitionKind::Push));
    }

    #[test]
    fn new_objects_and_slides_start_from_the_defaults_with_what_is_given_merged_in() {
        let o = new_object("id-1", &json!({ "name": "Box", "kind": "Ellipse", "w": 50 })).unwrap();
        assert_eq!((o["id"].as_str(), o["name"].as_str(), o["kind"].as_str(), o["w"].as_f64(), o["h"].as_f64()), (Some("id-1"), Some("Box"), Some("Ellipse"), Some(50.0), Some(200.0)));
        assert!(new_object("x", &json!("nope")).is_err());
        let s = new_slide("sl", "New", 1280.0, 720.0, &json!({ "notes": "n" })).unwrap();
        assert_eq!((s["id"].as_str(), s["notes"].as_str(), s["layers"].as_array().map(Vec::len)), (Some("sl"), Some("n"), Some(1)));
    }

    #[test]
    fn the_model_is_described_for_an_ai_tool() {
        let overview = describe(None).unwrap();
        assert!(overview.contains("JSON Pointers") && overview.contains("Object") && overview.contains("Entrance") && overview.contains("Trigger"));
        let object = describe(Some("Object")).unwrap();
        assert!(object.contains("entrance") && object.contains("Entrance or null") && object.contains("fill") && object.contains("Rgba"), "{object}");
        assert!(object.contains("timeline") || object.contains("timing"), "the field docs are carried over");
        let effect = describe(Some("EffectKind")).unwrap();
        assert!(effect.contains("\"Fade\"") && effect.contains("Wheel"), "{effect}");
        assert!(describe(Some("Rgba")).unwrap().contains("[whole number"), "a color is a list of four numbers");
        let e = describe(Some("Entranc")).unwrap_err();
        assert!(e.contains("Did you mean") && e.contains("Entrance"), "{e}");
        assert!(describe(Some("Project")).unwrap().contains("slides"));
        assert!(describe(Some("Nothing")).unwrap_err().contains("no type called"));
    }

    #[test]
    fn the_overview_is_short_enough_to_read() {
        assert!(describe(None).unwrap().len() < 12_000, "{}", describe(None).unwrap().len());
        assert!(describe(Some("Object")).unwrap().len() < 12_000);
    }

    #[test]
    fn a_patch_that_changes_nothing_gives_the_same_project() {
        let mut p = project();
        let mut same = patch(&mut p, &json!([{ "op": "test", "path": "/slides/0/name", "value": "Intro" }])).unwrap();
        assert_eq!(summary(&mut p, &mut same), json!({ "changed": false }));
        let mut changed = patch(&mut p, &json!([{ "op": "replace", "path": "/slides/0/name", "value": "Other" }])).unwrap();
        assert_eq!(summary(&mut p, &mut changed), json!({ "changed": true }));
    }
}
