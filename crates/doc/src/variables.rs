//! Renaming a variable and changing its type, without breaking what points at it.
//!
//! Triggers, sliders, dials, input fields and a survey's total refer to a variable by its id, so a rename never
//! touches them. Only `%Name%` in text refers to it by name, and the player fills that in from the name, so a rename
//! rewrites those. A type change converts the values triggers compare with or set, and refuses when something needs
//! the variable to stay a number or text (a slider, a dial, a survey total, an input field, arithmetic on it).
use crate::{Action, AdjustOp, Kind, Object, Project, QuestionStyle, SlideKind, Value};

/// The three kinds of value a variable holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VarType {
    Bool,
    Num,
    Text,
}

impl VarType {
    pub const ALL: [VarType; 3] = [VarType::Bool, VarType::Num, VarType::Text];

    /// The name the editor shows.
    pub fn label(self) -> &'static str {
        match self {
            VarType::Bool => "true/false",
            VarType::Num => "number",
            VarType::Text => "text",
        }
    }
}

pub fn type_of(v: &Value) -> VarType {
    match v {
        Value::Bool(_) => VarType::Bool,
        Value::Num(_) => VarType::Num,
        Value::Text(_) => VarType::Text,
    }
}

/// Names the player gives its own placeholders, so no variable may use them.
const RESERVED: [&str; 5] = ["Slide.Number", "Slide.Total", "Quiz.ScorePercent", "Quiz.PointsEarned", "Quiz.PointsPossible"];

/// A number as text: whole numbers without a decimal point.
fn number_text(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 { format!("{}", n as i64) } else { format!("{n}") }
}

/// `v` as another type, and whether something was lost on the way (text that is not a number becomes 0, text that
/// is not a yes or no becomes false, a number other than 0 or 1 becomes true).
pub fn convert_value(v: &Value, to: VarType) -> (Value, bool) {
    match (v, to) {
        (Value::Bool(b), VarType::Bool) => (Value::Bool(*b), false),
        (Value::Num(n), VarType::Num) => (Value::Num(*n), false),
        (Value::Text(t), VarType::Text) => (Value::Text(t.clone()), false),
        (Value::Bool(b), VarType::Num) => (Value::Num(if *b { 1.0 } else { 0.0 }), false),
        (Value::Bool(b), VarType::Text) => (Value::Text(if *b { "True" } else { "False" }.into()), false),
        (Value::Num(n), VarType::Bool) => (Value::Bool(*n != 0.0), *n != 0.0 && *n != 1.0),
        (Value::Num(n), VarType::Text) => (Value::Text(number_text(*n)), false),
        (Value::Text(t), VarType::Num) => {
            let s = t.trim();
            match s.parse::<f64>() {
                Ok(n) if n.is_finite() => (Value::Num(n), false),
                _ => (Value::Num(0.0), !s.is_empty()),
            }
        }
        (Value::Text(t), VarType::Bool) => {
            let s = t.trim().to_lowercase();
            let yes = matches!(s.as_str(), "true" | "1" | "yes");
            let known = yes || matches!(s.as_str(), "" | "false" | "0" | "no");
            (Value::Bool(yes), !known)
        }
    }
}

/// Every object in the project: on slides, in the master and in the layouts.
fn all_objects_mut(p: &mut Project) -> Box<dyn Iterator<Item = &mut Object> + '_> {
    Box::new(
        p.slides
            .iter_mut()
            .flat_map(|s| s.layers.iter_mut())
            .flat_map(|l| l.objects.iter_mut())
            .chain(p.master.objects.iter_mut())
            .chain(p.layouts.iter_mut().flat_map(|l| l.objects.iter_mut())),
    )
}

fn all_objects(p: &Project) -> Box<dyn Iterator<Item = &Object> + '_> {
    Box::new(
        p.slides
            .iter()
            .flat_map(|s| s.layers.iter())
            .flat_map(|l| l.objects.iter())
            .chain(p.master.objects.iter())
            .chain(p.layouts.iter().flat_map(|l| l.objects.iter())),
    )
}

/// Does any trigger run JavaScript? That code may use a variable's name, and a rename cannot update it.
pub fn has_javascript(p: &Project) -> bool {
    p.slides.iter().flat_map(|s| s.triggers.iter()).any(|t| matches!(t.action, Action::RunJavaScript(_)))
}

/// Is `name` a name a variable may have, whatever else exists? Returns it trimmed, or what is wrong with it.
/// A name is a letter or underscore followed by letters, digits, underscores and dots: exactly what the player can
/// show as `%Name%` in text, so a name that passes can always be used there.
pub fn check_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() { return Err("A variable needs a name.".into()); }
    let mut chars = name.chars();
    let first = chars.next().unwrap_or(' ');
    if !(first.is_ascii_alphabetic() || first == '_') || !chars.all(is_name_char) {
        return Err("Names can only use letters, digits, underscores and dots, and must start with a letter or an underscore. No spaces.".into());
    }
    if RESERVED.contains(&name) { return Err(format!("\"{name}\" is a name the player uses itself.")); }
    Ok(name.to_string())
}

fn is_name_char(c: char) -> bool { c.is_ascii_alphanumeric() || c == '_' || c == '.' }

/// Is `name` fine for variable `id`? Returns the name trimmed, or what is wrong with it.
pub fn check_new_name(p: &Project, id: &str, name: &str) -> Result<String, String> {
    let Some(v) = p.variables.iter().find(|v| v.id == id) else { return Err("That variable no longer exists.".into()) };
    if v.built_in { return Err("A built-in variable cannot be renamed.".into()); }
    let name = check_name(name)?;
    if p.variables.iter().any(|o| o.id != id && o.name == name) { return Err(format!("A variable called \"{name}\" already exists.")); }
    Ok(name)
}

/// Where the `%name%` placeholders are in `chars`, as `(start, end)` with `end` just past the closing `%`. A name
/// follows the rules of [`check_name`], so ordinary text such as "50% off, 100%" holds none.
pub fn placeholder_spans(chars: &[char]) -> Vec<(usize, usize)> {
    let mut out = vec![];
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '%' && chars.get(i + 1).is_some_and(|c| c.is_ascii_alphabetic() || *c == '_') {
            let mut j = i + 2;
            while j < chars.len() && is_name_char(chars[j]) { j += 1; }
            if chars.get(j) == Some(&'%') {
                out.push((i, j + 1));
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Where to put the cursor (`primary`) and the other end of the selection (`secondary`) so that neither lands inside
/// a placeholder, which counts as one character. A selection grows to take in whole placeholders; a lone cursor goes
/// to the near edge of the one it is in, on the side it was heading (`was` is where it was a moment ago).
pub fn snap_to_placeholders(text: &str, primary: usize, secondary: usize, was: Option<usize>) -> (usize, usize) {
    let chars: Vec<char> = text.chars().collect();
    let spans = placeholder_spans(&chars);
    let inside = |i: usize| spans.iter().find(|&&(a, b)| i > a && i < b).copied();
    if primary == secondary {
        return match inside(primary) {
            Some((a, b)) => { let to = if was.is_some_and(|w| w > primary) { a } else { b }; (to, to) }
            None => (primary, secondary),
        };
    }
    let (mut lo, mut hi) = (primary.min(secondary), primary.max(secondary));
    if let Some((a, _)) = inside(lo) { lo = a; }
    if let Some((_, b)) = inside(hi) { hi = b; }
    if primary > secondary { (hi, lo) } else { (lo, hi) }
}

/// The placeholder a Backspace (or, with `forward`, a Delete) at `caret` should remove whole, as `(start, end)`: the
/// one just before the cursor, or just after it for Delete. `None` when the key should act as usual.
pub fn placeholder_to_delete(text: &str, caret: usize, forward: bool) -> Option<(usize, usize)> {
    let chars: Vec<char> = text.chars().collect();
    placeholder_spans(&chars).into_iter().find(|&(a, b)| if forward { caret >= a && caret < b } else { caret > a && caret <= b })
}

/// Rewrites `from` to `to` in a piece of text; true when it changed anything.
fn rewrite(text: &mut String, from: &str, to: &str) -> bool {
    if text.contains(from) {
        *text = text.replace(from, to);
        true
    } else {
        false
    }
}

/// Rewrites the `%old%` placeholders in an object's text and in its states' text.
fn rewrite_object(o: &mut Object, from: &str, to: &str) -> bool {
    let mut changed = rewrite(&mut o.text, from, to);
    for para in o.rich.iter_mut().flatten() {
        for run in &mut para.runs { changed |= rewrite(&mut run.text, from, to); }
    }
    for st in &mut o.states {
        changed |= rewrite(&mut st.text, from, to);
        for para in st.rich.iter_mut().flatten() {
            for run in &mut para.runs { changed |= rewrite(&mut run.text, from, to); }
        }
    }
    changed
}

/// What a rename did.
#[derive(Debug, PartialEq)]
pub struct RenameReport {
    /// The name now.
    pub name: String,
    /// How many objects had `%old name%` in their text, and now say `%new name%`.
    pub objects: usize,
    /// The project has JavaScript, which may still use the old name.
    pub javascript: bool,
}

/// Renames variable `id` and rewrites the `%Name%` placeholders that showed it.
pub fn rename(p: &mut Project, id: &str, name: &str) -> Result<RenameReport, String> {
    let name = check_new_name(p, id, name)?;
    let old = p.variables.iter().find(|v| v.id == id).map(|v| v.name.clone()).unwrap_or_default();
    let mut objects = 0;
    if old != name {
        let (from, to) = (format!("%{old}%"), format!("%{name}%"));
        objects = all_objects_mut(p).map(|o| rewrite_object(o, &from, &to)).filter(|&c| c).count();
        if let Some(v) = p.variables.iter_mut().find(|v| v.id == id) { v.name = name.clone(); }
    }
    Ok(RenameReport { name, objects, javascript: has_javascript(p) })
}

/// What stops variable `id` becoming `to`: the things that need it to stay a number or text, in words ("1 slider").
pub fn conversion_blockers(p: &Project, id: &str, to: VarType) -> Vec<String> {
    let (mut sliders, mut dials, mut num_fields, mut text_fields, mut totals, mut arithmetic, mut appends) = (0, 0, 0, 0, 0, 0, 0);
    for o in all_objects(p) {
        match &o.kind {
            Kind::Slider(s) if s.variable == id => sliders += 1,
            Kind::Input(f) if f.variable == id => { if f.numeric { num_fields += 1 } else { text_fields += 1 } }
            _ => {}
        }
        if o.dial.as_ref().is_some_and(|d| d.variable == id) { dials += 1; }
    }
    for s in &p.slides {
        if let SlideKind::Question(q) = &s.kind {
            if let QuestionStyle::Likert { total_variable: Some(v), .. } = &q.style { if v == id { totals += 1; } }
        }
        for t in &s.triggers {
            if let Action::AdjustVariable { variable, op, .. } = &t.action {
                if variable == id { if *op == AdjustOp::Append { appends += 1 } else { arithmetic += 1 } }
            }
        }
    }
    let plural = |n: usize, one: &str, many: &str| if n == 1 { format!("1 {one}") } else { format!("{n} {many}") };
    let mut out = vec![];
    if to != VarType::Num {
        if sliders > 0 { out.push(plural(sliders, "slider", "sliders")); }
        if dials > 0 { out.push(plural(dials, "dial", "dials")); }
        if num_fields > 0 { out.push(plural(num_fields, "number field", "number fields")); }
        if totals > 0 { out.push(plural(totals, "survey total", "survey totals")); }
        if arithmetic > 0 { out.push(plural(arithmetic, "trigger that does arithmetic on it", "triggers that do arithmetic on it")); }
    }
    if to != VarType::Text {
        if text_fields > 0 { out.push(plural(text_fields, "text field", "text fields")); }
        if appends > 0 { out.push(plural(appends, "trigger that adds text to it", "triggers that add text to it")); }
    }
    out
}

/// What a type change did.
#[derive(Debug, PartialEq)]
pub struct ConvertReport {
    /// How many triggers had a value converted.
    pub triggers: usize,
    /// Some value could not be carried over exactly.
    pub lossy: bool,
}

/// Changes variable `id` to type `to`: its starting value, and the values triggers set it to or compare it with.
/// Refuses (and changes nothing) when something needs it to stay as it is; see [`conversion_blockers`].
pub fn convert(p: &mut Project, id: &str, to: VarType) -> Result<ConvertReport, String> {
    let Some(v) = p.variables.iter().find(|v| v.id == id) else { return Err("That variable no longer exists.".into()) };
    if v.built_in { return Err("A built-in variable cannot change type.".into()); }
    if type_of(&v.initial) == to { return Ok(ConvertReport { triggers: 0, lossy: false }); }
    let blockers = conversion_blockers(p, id, to);
    if !blockers.is_empty() {
        return Err(format!("It is used by {}. Change those first, or keep it as it is.", blockers.join(" and ")));
    }
    let mut lossy = false;
    if let Some(v) = p.variables.iter_mut().find(|v| v.id == id) {
        let (value, l) = convert_value(&v.initial, to);
        v.initial = value;
        lossy |= l;
    }
    let mut triggers = 0;
    for t in p.slides.iter_mut().flat_map(|s| s.triggers.iter_mut()) {
        let mut touched = false;
        if let Action::SetVariable { variable, value } = &mut t.action {
            if variable == id {
                let (nv, l) = convert_value(value, to);
                *value = nv;
                lossy |= l;
                touched = true;
            }
        }
        for c in t.conditions.iter_mut().filter(|c| c.variable == id) {
            let (nv, l) = convert_value(&c.value, to);
            c.value = nv;
            lossy |= l;
            // Greater and less only mean something for numbers.
            if to != VarType::Num && !matches!(c.cmp, crate::Cmp::Eq | crate::Cmp::Ne) { c.cmp = crate::Cmp::Eq; }
            touched = true;
        }
        if touched { triggers += 1; }
    }
    Ok(ConvertReport { triggers, lossy })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Cmp, Condition, Dial, Event, InputField, Owner, Slider, Slide, Trigger, Variable};

    fn var(id: &str, name: &str, initial: Value) -> Variable { Variable { id: id.into(), name: name.into(), initial, built_in: false } }

    fn project() -> Project {
        let mut p = Project::new("p");
        p.slides.push(Slide::new("s", "S", 1280.0, 720.0));
        p.variables = vec![var("v1", "Score", Value::Num(2.0)), var("v2", "Passed", Value::Bool(true)), var("v3", "Who", Value::Text("Sam".into()))];
        p
    }

    fn text_box(id: &str, text: &str) -> Object {
        let mut o = Object::new(id, id, Kind::Text, 0.0, 0.0, 100.0, 30.0);
        o.text = text.into();
        o
    }

    fn trigger(id: &str, action: Action, conditions: Vec<Condition>) -> Trigger {
        Trigger { id: id.into(), owner: Owner::Slide, event: Event::SlideStart, action, conditions }
    }

    fn cond(variable: &str, cmp: Cmp, value: Value) -> Condition { Condition { variable: variable.into(), cmp, value, or_with_previous: false } }

    #[test]
    fn values_convert_between_types_and_say_when_something_is_lost() {
        let t = |s: &str| Value::Text(s.into());
        assert_eq!(convert_value(&Value::Bool(true), VarType::Num), (Value::Num(1.0), false));
        assert_eq!(convert_value(&Value::Bool(false), VarType::Text), (t("False"), false));
        assert_eq!(convert_value(&Value::Num(0.0), VarType::Bool), (Value::Bool(false), false));
        assert_eq!(convert_value(&Value::Num(1.0), VarType::Bool), (Value::Bool(true), false));
        assert_eq!(convert_value(&Value::Num(7.0), VarType::Bool), (Value::Bool(true), true), "7 is not just yes or no");
        assert_eq!(convert_value(&Value::Num(3.0), VarType::Text), (t("3"), false), "no stray decimal point");
        assert_eq!(convert_value(&Value::Num(2.5), VarType::Text), (t("2.5"), false));
        assert_eq!(convert_value(&t(" 12.5 "), VarType::Num), (Value::Num(12.5), false));
        assert_eq!(convert_value(&t("abc"), VarType::Num), (Value::Num(0.0), true), "not a number");
        assert_eq!(convert_value(&t(""), VarType::Num), (Value::Num(0.0), false), "empty is just 0");
        assert_eq!(convert_value(&t("Yes"), VarType::Bool), (Value::Bool(true), false));
        assert_eq!(convert_value(&t("no"), VarType::Bool), (Value::Bool(false), false));
        assert_eq!(convert_value(&t("maybe"), VarType::Bool), (Value::Bool(false), true));
    }

    #[test]
    fn a_name_is_letters_digits_underscores_and_dots_with_no_spaces() {
        for ok in ["Score", "_score", "Attempts2", "Quiz_Score", "Section.Total", "  Padded  "] {
            assert!(check_name(ok).is_ok(), "{ok:?} should be fine");
        }
        assert_eq!(check_name("  Padded  ").unwrap(), "Padded");
        for bad in ["Learner Name", "2fast", "Total%", "naïve", "a-b", ".x", "", "   ", "Slide.Number"] {
            assert!(check_name(bad).is_err(), "{bad:?} should be refused");
        }
        assert!(check_name("Learner Name").unwrap_err().contains("No spaces"));
        let p = project();
        assert!(check_new_name(&p, "v1", "My Score").unwrap_err().contains("No spaces"), "a rename follows the same rules");
    }

    #[test]
    fn placeholders_are_found_only_when_they_look_like_a_variable() {
        let spans = |t: &str| placeholder_spans(&t.chars().collect::<Vec<_>>());
        assert_eq!(spans("Score: %Score% now"), [(7, 14)]);
        assert_eq!(spans("%A% and %B_2.x%"), [(0, 3), (8, 15)]);
        assert!(spans("50% off, 100%").is_empty(), "ordinary percent signs are not placeholders");
        assert!(spans("100%100%").is_empty(), "a name cannot start with a digit");
        assert!(spans("%%").is_empty() && spans("%").is_empty() && spans("%Open").is_empty());
        assert_eq!(spans("%a%b%"), [(0, 3)], "the leftover b% is just text");
    }

    #[test]
    fn the_cursor_and_selections_never_end_inside_a_placeholder() {
        let t = "Score: %Score% now"; // the placeholder is characters 7..14
        assert_eq!(snap_to_placeholders(t, 5, 5, Some(4)), (5, 5), "outside: untouched");
        assert_eq!(snap_to_placeholders(t, 10, 10, Some(14)), (7, 7), "heading left, it goes to the start");
        assert_eq!(snap_to_placeholders(t, 10, 10, Some(7)), (14, 14), "heading right, it goes to the end");
        assert_eq!(snap_to_placeholders(t, 10, 10, None), (14, 14), "a click lands after it");
        assert_eq!(snap_to_placeholders(t, 7, 7, Some(6)), (7, 7), "an edge is fine");
        assert_eq!(snap_to_placeholders(t, 9, 12, None), (7, 14), "a selection inside takes in all of it");
        assert_eq!(snap_to_placeholders(t, 3, 10, None), (3, 14), "one end inside grows to the far edge");
        assert_eq!(snap_to_placeholders(t, 12, 3, None), (14, 3), "and which end is the cursor is kept");
    }

    #[test]
    fn backspace_and_delete_take_a_whole_placeholder() {
        let t = "Score: %Score% now"; // 7..14
        assert_eq!(placeholder_to_delete(t, 14, false), Some((7, 14)), "backspace just after it");
        assert_eq!(placeholder_to_delete(t, 7, true), Some((7, 14)), "delete just before it");
        assert_eq!(placeholder_to_delete(t, 14, true), None, "delete after it is an ordinary delete");
        assert_eq!(placeholder_to_delete(t, 7, false), None, "backspace before it is ordinary");
        assert_eq!(placeholder_to_delete(t, 3, false), None);
    }

    #[test]
    fn names_must_be_new_filled_in_and_not_the_players_own() {
        let p = project();
        assert_eq!(check_new_name(&p, "v1", "  Points ").unwrap(), "Points");
        assert_eq!(check_new_name(&p, "v1", "Score").unwrap(), "Score", "its own name is fine");
        assert!(check_new_name(&p, "v1", "   ").is_err());
        assert!(check_new_name(&p, "v1", "Passed").unwrap_err().contains("already exists"));
        assert!(check_new_name(&p, "v1", "Quiz.ScorePercent").unwrap_err().contains("player uses"));
        assert!(check_new_name(&p, "nope", "X").is_err());
    }

    #[test]
    fn renaming_rewrites_the_text_that_shows_the_variable_and_nothing_else() {
        let mut p = project();
        let mut shape = text_box("a", "You scored %Score% of 10, %Score% again, and %Passed%");
        shape.rich = Some(vec![crate::Paragraph { runs: vec![crate::Run { text: "Rich %Score%".into(), bold: Some(true), ..Default::default() }, crate::Run { text: " tail".into(), ..Default::default() }], indent: 0 }]);
        shape.states = vec![crate::ObjectState { name: "Hover".into(), rich: None, fill: None, stroke: None, shadow: None, text: "Hover %Score%".into(), style: shape.style.clone(), asset: None, size: None, shine: None, gradient: None }];
        p.slides[0].layers[0].objects = vec![shape, text_box("b", "No variable here, and Score without percent signs")];
        p.master.objects = vec![text_box("m", "Master shows %Score%")];
        let report = rename(&mut p, "v1", "Points").unwrap();
        assert_eq!(report, RenameReport { name: "Points".into(), objects: 2, javascript: false }, "the two objects that showed it, one on the master");
        let a = &p.slides[0].layers[0].objects[0];
        assert_eq!(a.text, "You scored %Points% of 10, %Points% again, and %Passed%");
        assert_eq!(a.rich.as_ref().unwrap()[0].runs[0].text, "Rich %Points%");
        assert_eq!(a.rich.as_ref().unwrap()[0].runs[0].bold, Some(true), "formatting is kept");
        assert_eq!(a.states[0].text, "Hover %Points%");
        assert_eq!(p.slides[0].layers[0].objects[1].text, "No variable here, and Score without percent signs");
        assert_eq!(p.master.objects[0].text, "Master shows %Points%");
        assert_eq!(p.variables[0].name, "Points");
        assert_eq!(p.variables[0].id, "v1", "the id, which triggers use, is untouched");
    }

    #[test]
    fn a_rename_says_when_javascript_may_use_the_old_name() {
        let mut p = project();
        p.slides[0].triggers.push(trigger("t", Action::RunJavaScript("alert(1)".into()), vec![]));
        assert!(rename(&mut p, "v1", "Points").unwrap().javascript);
    }

    #[test]
    fn a_failed_rename_changes_nothing() {
        let mut p = project();
        p.slides[0].layers[0].objects = vec![text_box("a", "%Score%")];
        assert!(rename(&mut p, "v1", "Passed").is_err());
        assert_eq!(p.variables[0].name, "Score");
        assert_eq!(p.slides[0].layers[0].objects[0].text, "%Score%");
    }

    #[test]
    fn changing_type_converts_the_start_value_and_the_values_triggers_use() {
        let mut p = project();
        p.slides[0].triggers = vec![
            trigger("set", Action::SetVariable { variable: "v1".into(), value: Value::Num(5.0) }, vec![]),
            trigger("if", Action::NextSlide, vec![cond("v1", Cmp::Gt, Value::Num(3.0)), cond("v2", Cmp::Eq, Value::Bool(true))]),
            trigger("other", Action::SetVariable { variable: "v2".into(), value: Value::Bool(false) }, vec![]),
        ];
        let report = convert(&mut p, "v1", VarType::Text).unwrap();
        assert_eq!(report, ConvertReport { triggers: 2, lossy: false });
        assert_eq!(p.variables[0].initial, Value::Text("2".into()));
        assert!(matches!(&p.slides[0].triggers[0].action, Action::SetVariable { value: Value::Text(t), .. } if t == "5"));
        let c = &p.slides[0].triggers[1].conditions[0];
        assert_eq!((c.cmp, &c.value), (Cmp::Eq, &Value::Text("3".into())), "greater-than has no meaning for text, so it becomes equals");
        assert_eq!(p.slides[0].triggers[1].conditions[1].value, Value::Bool(true), "another variable's condition is untouched");
        assert!(matches!(&p.slides[0].triggers[2].action, Action::SetVariable { value: Value::Bool(false), .. }));
    }

    #[test]
    fn a_lossy_change_says_so() {
        let mut p = project();
        let report = convert(&mut p, "v3", VarType::Num).unwrap();
        assert!(report.lossy, "\"Sam\" is not a number");
        assert_eq!(p.variables[2].initial, Value::Num(0.0));
        assert!(!convert(&mut p, "v2", VarType::Num).unwrap().lossy, "true/false to number loses nothing");
    }

    #[test]
    fn a_change_to_the_same_type_does_nothing() {
        let mut p = project();
        assert_eq!(convert(&mut p, "v1", VarType::Num).unwrap(), ConvertReport { triggers: 0, lossy: false });
    }

    #[test]
    fn something_that_needs_a_number_or_text_blocks_the_change_and_nothing_is_touched() {
        let mut p = project();
        let slider = Object::new("sl", "Slider", Kind::Slider(Slider { variable: "v1".into(), ..Slider::new() }), 0.0, 0.0, 100.0, 20.0);
        let mut dial = text_box("d", "");
        dial.dial = Some(Dial { variable: "v1".into(), min: 0.0, max: 10.0, step: 1.0, sweep: 180.0, pivot: (0.0, 0.0) });
        let number_field = Object::new("nf", "Field", Kind::Input(InputField::number("v1")), 0.0, 0.0, 100.0, 20.0);
        let text_field = Object::new("tf", "Field", Kind::Input(InputField::text("v3")), 0.0, 0.0, 100.0, 20.0);
        p.slides[0].layers[0].objects = vec![slider, dial, number_field, text_field];
        p.slides[0].triggers = vec![
            trigger("add", Action::AdjustVariable { variable: "v1".into(), op: AdjustOp::Add, value: Value::Num(1.0) }, vec![]),
            trigger("mul", Action::AdjustVariable { variable: "v1".into(), op: AdjustOp::Multiply, value: Value::Num(2.0) }, vec![]),
            trigger("app", Action::AdjustVariable { variable: "v3".into(), op: AdjustOp::Append, value: Value::Text("!".into()) }, vec![]),
        ];
        let blockers = conversion_blockers(&p, "v1", VarType::Text);
        assert_eq!(blockers, ["1 slider", "1 dial", "1 number field", "2 triggers that do arithmetic on it"]);
        let err = convert(&mut p, "v1", VarType::Text).unwrap_err();
        assert!(err.contains("1 slider") && err.contains("2 triggers that do arithmetic on it"), "{err}");
        assert_eq!(p.variables[0].initial, Value::Num(2.0), "nothing changed");
        // A number is fine to keep as a number, and a text variable is blocked from becoming a number by its text field.
        assert!(conversion_blockers(&p, "v1", VarType::Num).is_empty());
        assert_eq!(conversion_blockers(&p, "v3", VarType::Num), ["1 text field", "1 trigger that adds text to it"]);
        assert_eq!(conversion_blockers(&p, "v3", VarType::Bool), ["1 text field", "1 trigger that adds text to it"]);
        assert!(conversion_blockers(&p, "v2", VarType::Text).is_empty(), "nothing is tied to the true/false one");
    }

    #[test]
    fn a_survey_total_needs_a_number() {
        let mut p = project();
        let mut s = Slide::new("q", "Survey", 1280.0, 720.0);
        s.kind = SlideKind::Question(crate::Question::new_likert());
        if let SlideKind::Question(q) = &mut s.kind {
            if let QuestionStyle::Likert { total_variable, .. } = &mut q.style { *total_variable = Some("v1".into()); }
        }
        p.slides.push(s);
        assert_eq!(conversion_blockers(&p, "v1", VarType::Bool), ["1 survey total"]);
        assert!(convert(&mut p, "v1", VarType::Bool).is_err());
    }
}
