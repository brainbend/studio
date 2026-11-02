//! Find and replace over a project's text: object text (all layers), slide names, slide notes
//! and question prompts, choices and feedback.
use crate::*;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FindOpts {
    pub match_case: bool,
    pub whole_word: bool,
}

/// Where a match lives.
#[derive(Clone, Debug, PartialEq)]
pub enum Place {
    /// (layer index, object index)
    Object(usize, usize),
    SlideName,
    Notes,
    /// The question prompt, a choice, or feedback.
    Question,
    /// A Web Lesson section title (`Hit::slide` is the section).
    Section,
    /// A Web Lesson block, by index in its section (`Hit::slide` is the section).
    Block(usize),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Hit {
    pub slide: usize,
    pub place: Place,
    /// Number of matches in this place.
    pub count: usize,
    /// A short stretch of text around the first match.
    pub context: String,
}

fn same(a: char, b: char, match_case: bool) -> bool {
    a == b || (!match_case && a.to_lowercase().eq(b.to_lowercase()))
}

/// Start indices of non-overlapping matches of `q` in `hay`.
fn matches(hay: &[char], q: &[char], o: FindOpts) -> Vec<usize> {
    let mut out = vec![];
    if q.is_empty() || hay.len() < q.len() { return out; }
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let mut i = 0;
    while i + q.len() <= hay.len() {
        let hit = (0..q.len()).all(|k| same(hay[i + k], q[k], o.match_case))
            && (!o.whole_word || ((i == 0 || !word(hay[i - 1])) && (i + q.len() == hay.len() || !word(hay[i + q.len()]))));
        if hit { out.push(i); i += q.len(); } else { i += 1; }
    }
    out
}

fn count_in(text: &str, q: &str, o: FindOpts) -> usize {
    matches(&text.chars().collect::<Vec<_>>(), &q.chars().collect::<Vec<_>>(), o).len()
}

fn context(text: &str, q: &str, o: FindOpts) -> String {
    let chars: Vec<char> = text.chars().collect();
    let Some(&at) = matches(&chars, &q.chars().collect::<Vec<_>>(), o).first() else { return String::new() };
    let from = at.saturating_sub(18);
    let to = (at + q.chars().count() + 18).min(chars.len());
    let s: String = chars[from..to].iter().map(|c| if *c == '\n' { ' ' } else { *c }).collect();
    format!("{}{}{}", if from > 0 { "…" } else { "" }, s, if to < chars.len() { "…" } else { "" })
}

/// Replace every match in a plain string; returns the new text and how many were replaced.
fn replace_str(text: &str, q: &str, with: &str, o: FindOpts) -> (String, usize) {
    let chars: Vec<char> = text.chars().collect();
    let qc: Vec<char> = q.chars().collect();
    let ms = matches(&chars, &qc, o);
    if ms.is_empty() { return (text.to_string(), 0); }
    let mut out = String::new();
    let mut i = 0;
    for m in &ms {
        out.extend(&chars[i..*m]);
        out.push_str(with);
        i = m + qc.len();
    }
    out.extend(&chars[i..]);
    (out, ms.len())
}

impl Object {
    /// Replace matches in this object's text, keeping formatting (a replacement takes the format
    /// of the first character it replaces). Returns how many were replaced.
    pub fn replace_text(&mut self, q: &str, with: &str, o: FindOpts) -> usize {
        let old_text = self.text.clone();
        let n = if self.rich.is_none() {
            let (t, n) = replace_str(&self.text, q, with, o);
            if n > 0 { self.text = t; }
            n
        } else {
            let chars = self.char_fmts();
            let plain: Vec<char> = chars.iter().map(|c| c.0).collect();
            let qc: Vec<char> = q.chars().collect();
            let ms = matches(&plain, &qc, o);
            if ms.is_empty() { return 0; }
            let mut out = Vec::with_capacity(chars.len());
            let mut i = 0;
            for m in &ms {
                out.extend_from_slice(&chars[i..*m]);
                let f = chars[*m].1.clone();
                out.extend(with.chars().map(|c| (c, f.clone())));
                i = m + qc.len();
            }
            out.extend_from_slice(&chars[i..]);
            self.set_chars(out);
            ms.len()
        };
        if n > 0 {
            let (t, r) = (self.text.clone(), self.rich.clone());
            for st in self.states.iter_mut().filter(|s| s.text == old_text) { st.text = t.clone(); st.rich = r.clone(); }
        }
        n
    }
}

impl Project {
    /// All the searchable text: (slide index, where it is, the text).
    pub fn texts(&self) -> Vec<(usize, Place, String)> {
        let mut out = vec![];
        if self.is_web() {
            for (si, s) in self.lesson.as_ref().unwrap().sections.iter().enumerate() {
                out.push((si, Place::Section, s.title.clone()));
                for (bi, b) in s.blocks.iter().enumerate() { out.push((si, Place::Block(bi), b.kind.texts().join("\n"))); }
            }
            return out;
        }
        for (si, s) in self.slides.iter().enumerate() {
            out.push((si, Place::SlideName, s.name.clone()));
            for (li, l) in s.layers.iter().enumerate() {
                for (oi, ob) in l.objects.iter().enumerate() { out.push((si, Place::Object(li, oi), ob.text.clone())); }
            }
            out.push((si, Place::Notes, s.notes.clone()));
            if let SlideKind::Question(qn) = &s.kind {
                let mut all = vec![qn.prompt.clone(), qn.feedback_correct.clone(), qn.feedback_incorrect.clone()];
                all.extend(qn.choices.iter().map(|c| c.text.clone()));
                out.push((si, Place::Question, all.join("\n")));
            }
        }
        out
    }

    /// Every place `q` occurs.
    pub fn find_text(&self, q: &str, o: FindOpts) -> Vec<Hit> {
        let mut hits = vec![];
        if q.is_empty() { return hits; }
        for (slide, place, text) in self.texts() {
            let count = count_in(&text, q, o);
            if count > 0 { hits.push(Hit { slide, place, count, context: context(&text, q, o) }); }
        }
        hits
    }

    /// Replace matches everywhere, or only in `only` (a hit from `find_text`). Returns the number replaced.
    pub fn replace_text(&mut self, q: &str, with: &str, o: FindOpts, only: Option<&Hit>) -> usize {
        if q.is_empty() { return 0; }
        let mut total = 0;
        if self.is_web() {
            for (si, s) in self.lesson.as_mut().unwrap().sections.iter_mut().enumerate() {
                if only.is_none_or(|h| h.slide == si && h.place == Place::Section) { let (t, n) = replace_str(&s.title, q, with, o); s.title = t; total += n; }
                for (bi, b) in s.blocks.iter_mut().enumerate() {
                    if only.is_none_or(|h| h.slide == si && h.place == Place::Block(bi)) {
                        for t in b.kind.texts_mut() { let (new, n) = replace_str(t, q, with, o); *t = new; total += n; }
                    }
                }
            }
            return total;
        }
        for (si, s) in self.slides.iter_mut().enumerate() {
            let want = |p: &Place| only.is_none_or(|h| h.slide == si && &h.place == p);
            if want(&Place::SlideName) { let (t, n) = replace_str(&s.name, q, with, o); s.name = t; total += n; }
            if want(&Place::Notes) { let (t, n) = replace_str(&s.notes, q, with, o); s.notes = t; total += n; }
            for (li, l) in s.layers.iter_mut().enumerate() {
                for (oi, ob) in l.objects.iter_mut().enumerate() {
                    if want(&Place::Object(li, oi)) { total += ob.replace_text(q, with, o); }
                }
            }
            if want(&Place::Question) {
                if let SlideKind::Question(qn) = &mut s.kind {
                    for t in [&mut qn.prompt, &mut qn.feedback_correct, &mut qn.feedback_incorrect].into_iter().chain(qn.choices.iter_mut().map(|c| &mut c.text)) {
                        let (new, n) = replace_str(t, q, with, o);
                        *t = new;
                        total += n;
                    }
                }
            }
        }
        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> Project {
        let mut p = Project::new("F");
        p.slides.push(Slide::new("s1", "Safety intro", 1920.0, 1080.0));
        p.sanitize();
        let mut a = Object::new("a", "A", Kind::Text, 0.0, 0.0, 100.0, 50.0);
        a.text = "Safety first, safety always. Unsafe is not safety.".into();
        let mut b = Object::new("b", "B", Kind::Text, 0.0, 0.0, 100.0, 50.0);
        b.text = "Plain".into();
        b.rich = Some(vec![Paragraph { runs: vec![Run { text: "Plain ".into(), ..Default::default() }, Run { text: "safety".into(), bold: Some(true), ..Default::default() }], ..Default::default() }]);
        b.text = "Plain safety".into();
        p.slides[0].layers[0].objects = vec![a, b];
        p.slides[0].notes = "Mention SAFETY".into();
        p
    }

    #[test]
    fn finds_across_objects_notes_and_names() {
        let p = project();
        let all = p.find_text("safety", FindOpts::default());
        assert_eq!(all.iter().map(|h| h.count).sum::<usize>(), 1 + 3 + 1 + 1, "name, 3 in the first text, 1 in the rich text, 1 in the notes");
        assert_eq!(p.find_text("safety", FindOpts { match_case: true, whole_word: false }).iter().map(|h| h.count).sum::<usize>(), 3);
        // Whole word skips "Unsafe" but "safety" in "Unsafe is not safety" still counts.
        assert_eq!(p.find_text("safe", FindOpts { match_case: false, whole_word: true }).len(), 0);
        assert!(p.find_text("", FindOpts::default()).is_empty());
    }

    #[test]
    fn replacing_keeps_formatting_and_can_target_one_hit() {
        let mut p = project();
        let n = p.replace_text("safety", "security", FindOpts::default(), None);
        assert_eq!(n, 6);
        let o = &p.slides[0].layers[0].objects;
        assert_eq!(o[0].text, "security first, security always. Unsafe is not security.");
        let runs = &o[1].rich.as_ref().unwrap()[0].runs;
        assert_eq!((runs[1].text.as_str(), runs[1].bold), ("security", Some(true)));
        assert_eq!(p.slides[0].notes, "Mention security");
        // Only the notes.
        let mut q = project();
        let hit = q.find_text("safety", FindOpts::default()).into_iter().find(|h| h.place == Place::Notes).unwrap();
        assert_eq!(q.replace_text("safety", "x", FindOpts::default(), Some(&hit)), 1);
        assert_eq!(q.slides[0].notes, "Mention x");
        assert!(q.slides[0].layers[0].objects[0].text.contains("Safety"));
    }
}
