//! Accessibility for the AI tools: a button built as one correct object, and the same checker the author uses.
//!
//! What an AI tool makes should be accessible without the author having to find and fix it. So a button is made whole
//! (the words inside the shape, one click trigger, readable colors), and `check_accessibility` gives the AI the editor's
//! own report to work through before it says it is finished.

use super::*;
use crate::mcp_tools::*;
use brainbend_doc::check::{contrast_ratio, Severity};
use serde_json::{json, Value};

impl App {
    /// The editor's accessibility report, as an AI tool can use it.
    pub(super) fn mcp_check_accessibility(&mut self, a: &Value) -> Result<String, String> {
        let p = self.committed().ok_or("No project is open.")?;
        let only = match a.get("slide").filter(|v| !v.is_null()) { None => None, Some(_) => Some(slide_arg(a, "slide", p.slides.len())?) };
        let issues: Vec<Value> = p.check().into_iter().filter(|i| only.is_none_or(|s| i.at.slide == s)).map(|i| {
            let object = i.at.object.and_then(|(li, oi)| p.slides.get(i.at.slide).and_then(|s| s.layers.get(li)).and_then(|l| l.objects.get(oi))).map(|o| o.name.clone());
            json!({ "severity": if i.severity == Severity::Error { "error" } else { "warning" }, "slide": i.at.slide + 1, "object": object, "message": i.message })
        }).collect();
        let errors = issues.iter().filter(|i| i["severity"] == "error").count();
        done(json!({
            "errors": errors, "warnings": issues.len() - errors, "issues": issues,
            "next": if issues.is_empty() { "No problems found." } else { "Fix every error and warning (alt text, labels, contrast, text laid over a clickable object, heading order) and check again. An issue that is a deliberate choice of the author's is the only thing to leave, and say so." },
        }))
    }

    /// A button as one object: the words inside a rounded shape, with its click trigger, in colors that can be read.
    pub(super) fn mcp_add_button(&mut self, a: &Value, count: usize) -> Result<String, String> {
        let i = slide_arg(a, "slide", count)?;
        let text = text_arg(a, "text")?.map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).ok_or("`text` must be the words on the button, such as \"Forklifts\" or \"Home\".")?;
        let action = text_arg(a, "action")?.unwrap_or_else(|| "go_to_slide".into());
        let p = self.project.as_ref().unwrap();
        let act = match action.as_str() {
            "next" => Action::NextSlide,
            "previous" => Action::PreviousSlide,
            "go_to_slide" => {
                let t = slide_arg(a, "target", count).map_err(|e| format!("For go_to_slide, {e}"))?;
                Action::JumpToSlide(p.slides[t].id.clone())
            }
            _ => return Err("`action` must be go_to_slide (with `target`, a slide number), next or previous.".into()),
        };
        let (sw, sh) = { let s = &p.slides[i]; (s.width, s.height) };
        let theme = p.theme.clone();
        let fill = color_arg(a, "fill")?.unwrap_or_else(|| theme.color(4));
        // White or the lesson's text color, whichever reads better on the fill.
        let white = Rgba(255, 255, 255, 255);
        let ink = if contrast_ratio(white, fill) >= contrast_ratio(theme.text(), fill) { white } else { theme.text() };
        let size = num_arg(a, "size", 8.0, 400.0)?.unwrap_or(40.0);
        let (w, h) = (num_arg(a, "width", 40.0, 20_000.0)?.unwrap_or(360.0), num_arg(a, "height", 30.0, 20_000.0)?.unwrap_or(100.0));
        let (x, y) = (num_arg(a, "x", -20_000.0, 20_000.0)?.unwrap_or(((sw - w) / 2.0).max(0.0)), num_arg(a, "y", -20_000.0, 20_000.0)?.unwrap_or((sh * 0.75).min((sh - h).max(0.0))));
        let icon = match text_arg(a, "icon")?.map(|n| n.trim().to_lowercase().replace(' ', "-")).filter(|n| !n.is_empty()) {
            None => None,
            Some(n) => Some(crate::icons::all().iter().find(|ic| ic.names.contains(&n.as_str())).ok_or_else(|| format!("There is no icon called {n:?}. Use find_icons to search the names."))?),
        };
        let name = text_arg(a, "name")?.unwrap_or_else(|| format!("{text} button"));
        let mut o = blank_object(self.new_id("obj"), &name, Kind::Rect, x.round(), y.round(), w.round(), h.round());
        o.radius = 24.0;
        o.fill = Some(fill);
        o.text = text.clone();
        o.style.font_family = theme.body_font.clone();
        o.style.size = size;
        o.style.bold = true;
        o.style.color = ink;
        o.style.align = Align::Center;
        o.style.valign = VAlign::Middle;
        let mut parts = vec![];
        if let Some(ic) = icon {
            let (cmds, aspect) = crate::icons::outline(ic).ok_or("That icon could not be drawn.")?;
            let side = (h * 0.5).round().max(8.0);
            let (iw, ih) = if aspect >= 1.0 { (side, (side / aspect).round().max(1.0)) } else { ((side * aspect).round().max(1.0), side) };
            let cmds_for_silhouette = cmds.clone();
            let mut ico = blank_object(self.new_id("obj"), &format!("{} icon", ic.title()), Kind::Path(cmds), (x + h * 0.25).round(), (y + (h - ih) / 2.0).round(), iw, ih);
            ico.stroke = Some(Stroke { color: ink, width: 0.0 });
            ico.icon = Some(brainbend_doc::shapes::silhouette(&cmds_for_silhouette));
            // Part of the button: the button's own words name it, so the icon is skipped by screen readers.
            ico.decorative = true;
            let group = self.new_id("group");
            ico.group = Some(group.clone());
            o.group = Some(group);
            // The words sit beside the icon, not over it.
            o.margins = [h * 0.25 + iw + h * 0.15, 0.0, h * 0.25, 0.0];
            parts.push(ico);
        }
        let trigger = Trigger { id: self.new_id("trig"), owner: Owner::Object(o.id.clone()), event: Event::Click, action: act, conditions: vec![] };
        let described = describe_object(&o);
        self.snapshot();
        let slide = &mut self.project.as_mut().unwrap().slides[i];
        slide.layers[0].objects.push(o);
        slide.layers[0].objects.extend(parts);
        slide.triggers.push(trigger);
        self.selected = Some((0, slide.layers[0].objects.len() - 1 - usize::from(icon.is_some())));
        self.slide = i;
        self.layer = 0;
        self.dirty = true;
        self.status = format!("AI tool: added the {text} button to slide {}", i + 1);
        done(json!({ "button": described, "does": action, "with_icon": icon.map(|ic| ic.title()), "note": "One object: the words are inside the shape and the click is on it, so it works with the keyboard and screen readers as it is." }))
    }
}

// ---- groups ----

impl App {
    /// Ties objects into a group that works as one control: it moves together, and when it has a click action a click on
    /// any part runs it. Screen readers read the group once, by the words on its parts or by its alt text.
    pub(super) fn mcp_group_objects(&mut self, a: &Value, count: usize) -> Result<String, String> {
        let i = slide_arg(a, "slide", count)?;
        let list = a.get("objects").and_then(Value::as_array).filter(|l| l.len() >= 2).ok_or("`objects` must be a list of at least two object names or ids from get_slide.")?;
        let slide = &self.project.as_ref().unwrap().slides[i];
        let mut found: Vec<(usize, usize)> = vec![];
        for v in list {
            let which = v.as_str().ok_or("`objects` must be a list of object names or ids.")?;
            let at = find_object(slide, which)?;
            if found.contains(&at) { return Err(format!("{which:?} is listed twice.")); }
            found.push(at);
        }
        let li = found[0].0;
        if found.iter().any(|&(l, _)| l != li) { return Err("The objects are on different layers of the slide. A group must be on one layer.".into()); }
        for &(l, k) in &found {
            let o = &slide.layers[l].objects[k];
            if o.table.is_some() { return Err(format!("{:?} is part of a table, which cannot be grouped.", o.name)); }
            if let Some(g) = o.group.as_deref() {
                let members = slide.group_members(l, g);
                if members.iter().any(|m| !found.contains(&(l, *m))) { return Err(format!("{:?} is already in a group with other objects. Include every object of that group, or leave it out.", o.name)); }
            }
        }
        let action = text_arg(a, "action")?;
        let trigger_action = match action.as_deref() {
            None => None,
            Some("next") => Some(Action::NextSlide),
            Some("previous") => Some(Action::PreviousSlide),
            Some("go_to_slide") => {
                let t = slide_arg(a, "target", count).map_err(|e| format!("For go_to_slide, {e}"))?;
                Some(Action::JumpToSlide(self.project.as_ref().unwrap().slides[t].id.clone()))
            }
            Some(_) => return Err("`action` must be go_to_slide (with `target`, a slide number), next or previous.".into()),
        };
        let alt = text_arg(a, "alt")?.map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
        let name = text_arg(a, "name")?.map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
        let owner_id = slide.layers[li].objects[found[0].1].id.clone();
        let gid = self.new_id("group");
        let trigger = trigger_action.map(|act| Trigger { id: self.new_id("trig"), owner: Owner::Object(owner_id.clone()), event: Event::Click, action: act, conditions: vec![] });
        self.snapshot();
        let slide = &mut self.project.as_mut().unwrap().slides[i];
        for &(l, k) in &found { slide.layers[l].objects[k].group = Some(gid.clone()); }
        if let Some(n) = &name { slide.layers[li].group_names.insert(gid.clone(), n.clone()); }
        let clickable = trigger.is_some();
        if let Some(t) = trigger { slide.triggers.push(t); }
        if clickable { if let Some(t) = &alt { slide.layers[li].objects[found[0].1].alt = t.clone(); } }
        self.selected = Some((li, found[0].1));
        self.slide = i;
        self.layer = li;
        self.dirty = true;
        self.status = format!("AI tool: grouped {} objects on slide {}", found.len(), i + 1);
        let slide = &self.project.as_ref().unwrap().slides[i];
        let read_as = if clickable { slide.group_name(li, &gid) } else { String::new() };
        let mut warnings: Vec<&str> = vec![];
        if clickable && read_as.is_empty() { warnings.push("The group is clickable but has no words on any part and no alt text, so screen readers cannot name it. Give it alt (call group_objects again with alt, or set_object alt on the first object)."); }
        if !clickable && alt.is_some() { warnings.push("alt was ignored: it names a clickable group. Give an action to make the group clickable."); }
        done(json!({
            "group": gid, "objects": found.iter().map(|&(l, k)| slide.layers[l].objects[k].name.clone()).collect::<Vec<_>>(),
            "clickable": clickable, "owner": slide.layers[li].objects[found[0].1].name, "screen_readers_read": read_as, "warnings": warnings,
            "note": "A click on any part runs the group's action. Do not add click triggers to other parts of the group.",
        }))
    }
}
