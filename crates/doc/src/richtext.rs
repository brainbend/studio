//! Character-level editing of an object's mixed-style text.
//!
//! `Object::rich` stores paragraphs of runs that only record how they differ from the object's
//! own style. Editing is easier per character, so these helpers expand to one format per
//! character, change it, and fold equal neighbours back into runs.
use crate::*;

/// How one character differs from the object's base style (`None` = same as the object).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fmt {
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub color: Option<Rgba>,
    pub size: Option<f32>,
    pub font_family: Option<String>,
    pub link: Option<String>,
    pub strike: Option<bool>,
    pub highlight: Option<Rgba>,
    pub spacing: Option<f32>,
    /// The paragraph's indent steps (every character of a paragraph carries the same value).
    pub indent: u8,
}

/// A text attribute a toolbar or shortcut can toggle or set.
#[derive(Clone, Debug, PartialEq)]
pub enum Attr {
    ToggleBold,
    ToggleItalic,
    ToggleUnderline,
    Color(Rgba),
    Size(f32),
    /// Make the text open this address (`None` removes the link).
    Link(Option<String>),
    /// Remove all character formatting (a link stays).
    Clear,
    ToggleStrike,
    /// A marker colour behind the text (`None` removes it).
    Highlight(Option<Rgba>),
    /// Letter spacing in px (0 is normal).
    Spacing(f32),
    /// Move whole paragraphs in (+1) or out (-1) one step.
    Indent(i8),
}

/// How Change Case rewrites text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Case {
    Sentence,
    Lower,
    Upper,
    Capitalize,
    Toggle,
}

impl Object {
    /// One entry per character of `text` (paragraph breaks are `'\n'` characters).
    pub fn char_fmts(&self) -> Vec<(char, Fmt)> {
        let Some(paras) = &self.rich else {
            return self.text.chars().map(|c| (c, Fmt::default())).collect();
        };
        let mut out = vec![];
        for (pi, para) in paras.iter().enumerate() {
            if pi > 0 {
                out.push(('\n', Fmt::default()));
            }
            for run in &para.runs {
                let f = Fmt { bold: run.bold, italic: run.italic, underline: run.underline, color: run.color, size: run.size, font_family: run.font_family.clone(), link: run.link.clone(), strike: run.strike, highlight: run.highlight, spacing: run.spacing, indent: para.indent };
                out.extend(run.text.chars().map(|c| (c, f.clone())));
            }
        }
        out
    }

    pub(crate) fn set_chars(&mut self, chars: Vec<(char, Fmt)>) {
        self.text = chars.iter().map(|c| c.0).collect();
        if chars.iter().all(|c| c.1 == Fmt::default()) {
            self.rich = None;
            return;
        }
        let mut paras = vec![Paragraph::default()];
        let mut fresh = true;
        for (c, f) in chars {
            if c == '\n' {
                paras.push(Paragraph::default());
                fresh = true;
                continue;
            }
            let para = paras.last_mut().unwrap();
            if fresh { para.indent = f.indent; fresh = false; }
            let same = |r: &Run| (r.bold, r.italic, r.underline, r.color, r.size, &r.font_family, &r.link, r.strike, r.highlight, r.spacing) == (f.bold, f.italic, f.underline, f.color, f.size, &f.font_family, &f.link, f.strike, f.highlight, f.spacing);
            match para.runs.last_mut() {
                Some(run) if same(run) => run.text.push(c),
                _ => para.runs.push(Run { text: c.to_string(), bold: f.bold, italic: f.italic, underline: f.underline, color: f.color, size: f.size, font_family: f.font_family, link: f.link, strike: f.strike, highlight: f.highlight, spacing: f.spacing }),
            }
        }
        self.rich = Some(paras);
    }

    /// Apply `attr` to characters `start..end` (character indices). Toggles turn the attribute off
    /// if every selected character already has it, otherwise on for all.
    pub fn format_range(&mut self, start: usize, end: usize, attr: &Attr) {
        let mut chars = self.char_fmts();
        let end = end.min(chars.len());
        if start >= end {
            return;
        }
        let st = self.style.clone();
        let eff = |f: &Fmt, which: u8| match which {
            0 => f.bold.unwrap_or(st.bold),
            1 => f.italic.unwrap_or(st.italic),
            // A link is underlined unless the author says otherwise.
            2 => f.underline.unwrap_or(st.underline || f.link.is_some()),
            _ => f.strike.unwrap_or(false),
        };
        let which = match attr {
            Attr::ToggleBold => Some(0),
            Attr::ToggleItalic => Some(1),
            Attr::ToggleUnderline => Some(2),
            Attr::ToggleStrike => Some(3),
            _ => None,
        };
        let target = which.map(|w| !chars[start..end].iter().all(|(_, f)| eff(f, w)));
        for (_, f) in &mut chars[start..end] {
            // Values equal to the object's own style are stored as None so runs stay minimal.
            let norm = |v: bool, base: bool| (v != base).then_some(v);
            match attr {
                Attr::ToggleBold => f.bold = norm(target.unwrap(), st.bold),
                Attr::ToggleItalic => f.italic = norm(target.unwrap(), st.italic),
                // A link's look is its own default, so what the author sets on it is always kept, even when it equals the object's style.
                Attr::ToggleUnderline => f.underline = if f.link.is_some() { target } else { norm(target.unwrap(), st.underline) },
                Attr::Color(c) => f.color = (f.link.is_some() || *c != st.color).then_some(*c),
                Attr::Size(z) => f.size = ((*z - st.size).abs() > 0.01).then_some(*z),
                Attr::Link(l) => f.link = l.clone(),
                Attr::Clear => *f = Fmt { link: f.link.take(), indent: f.indent, ..Fmt::default() },
                Attr::ToggleStrike => f.strike = target.unwrap().then_some(true),
                Attr::Highlight(c) => f.highlight = *c,
                Attr::Spacing(z) => f.spacing = (z.abs() > 0.01).then_some(*z),
                Attr::Indent(_) => {}
            }
        }
        if let Attr::Indent(d) = attr {
            // Whole paragraphs: every paragraph the range touches.
            let (mut first, mut last) = (0usize, 0usize);
            let mut para = 0usize;
            let mut marks = vec![];
            for (i, (c, _)) in chars.iter().enumerate() {
                marks.push(para);
                if *c == '\n' { para += 1; }
                let _ = i;
            }
            first = marks[start];
            last = marks[end - 1];
            for (i, (_, f)) in chars.iter_mut().enumerate() {
                if marks[i] >= first && marks[i] <= last { f.indent = (f.indent as i16 + *d as i16).clamp(0, 8) as u8; }
            }
        }
        self.set_chars(chars);
    }

    /// Rewrite the letters of characters `start..end` in the given case, keeping every character's formatting.
    pub fn change_case(&mut self, start: usize, end: usize, case: Case) {
        let mut chars = self.char_fmts();
        let end = end.min(chars.len());
        if start >= end { return; }
        let one = |s: String, keep: char| { let mut it = s.chars(); match (it.next(), it.next()) { (Some(c), None) => c, _ => keep } };
        let mut new_word = true;
        let mut new_sentence = true;
        for (c, _) in &mut chars[start..end] {
            let old = *c;
            *c = match case {
                Case::Lower => one(old.to_lowercase().collect(), old),
                Case::Upper => one(old.to_uppercase().collect(), old),
                Case::Toggle => if old.is_uppercase() { one(old.to_lowercase().collect(), old) } else { one(old.to_uppercase().collect(), old) },
                Case::Capitalize => if new_word && old.is_alphabetic() { one(old.to_uppercase().collect(), old) } else { old },
                Case::Sentence => if old.is_alphabetic() { if new_sentence { one(old.to_uppercase().collect(), old) } else { one(old.to_lowercase().collect(), old) } } else { old },
            };
            if old.is_alphabetic() { new_word = false; new_sentence = false; }
            if old.is_whitespace() { new_word = true; }
            if matches!(old, '.' | '!' | '?' | '\n') { new_sentence = true; }
        }
        self.set_chars(chars);
    }

    /// Replace the text with `new`, keeping the formatting of untouched characters. New characters
    /// take the format of the character before the edit.
    pub fn edit_text(&mut self, new: &str) {
        if self.rich.is_none() {
            self.text = new.to_string();
            return;
        }
        let old = self.char_fmts();
        let new_chars: Vec<char> = new.chars().collect();
        let mut p = 0;
        while p < old.len() && p < new_chars.len() && old[p].0 == new_chars[p] {
            p += 1;
        }
        let mut s = 0;
        while s < old.len() - p && s < new_chars.len() - p && old[old.len() - 1 - s].0 == new_chars[new_chars.len() - 1 - s] {
            s += 1;
        }
        let fill = if p > 0 { old[p - 1].1.clone() } else { old.first().map(|c| c.1.clone()).unwrap_or_default() };
        let mut out: Vec<(char, Fmt)> = old[..p].to_vec();
        out.extend(new_chars[p..new_chars.len() - s].iter().map(|&c| (c, fill.clone())));
        out.extend_from_slice(&old[old.len() - s..]);
        self.set_chars(out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(text: &str) -> Object {
        Object {
            id: "o".into(), name: "o".into(), kind: Kind::Text, x: 0.0, y: 0.0, w: 100.0, h: 50.0,
            fill: None, gradient: None, rotation: 0.0, line_spacing: 0.0, group: None, table: None, table_span: None, table_borders: None, emphasis: None, motion: None, click_steps: Default::default(), decorative: false, crop: None, dial: None, check: None, hit_area: None, mask: None, dash: LineDash::Solid, media: Default::default(), hide_on_layers: vec![], button_set: None, poster: None, stroke: None, shadow: None, hidden: false, margins: [0.0; 4], radius: 0.0, alt: String::new(), text_rotation: 0, role: TextRole::Normal, marker: None, equation: None,
            captions: None, rich: None, entrance: None, exit: None, timing: None, text: text.into(),
            style: TextStyle { font_family: "Open Sans".into(), size: 20.0, bold: false, italic: false, underline: false, color: Rgba(0, 0, 0, 255), align: Align::Left, valign: VAlign::Middle },
            asset: None, states: vec![], initial_state: String::new(), placeholder: None,
        }
    }

    #[test]
    fn a_link_covers_just_the_chosen_characters_and_can_be_removed() {
        let mut o = obj("Read the docs now");
        o.format_range(9, 13, &Attr::Link(Some("https://example.com".into())));
        let runs = &o.rich.as_ref().unwrap()[0].runs;
        assert_eq!(runs.len(), 3);
        assert_eq!((runs[1].text.as_str(), runs[1].link.as_deref()), ("docs", Some("https://example.com")));
        assert!(runs[0].link.is_none() && runs[2].link.is_none());
        o.format_range(0, 17, &Attr::Link(None));
        assert!(o.rich.is_none(), "no overrides left");
    }

    #[test]
    fn toggling_bold_on_a_range_splits_runs_and_toggles_back() {
        let mut o = obj("Hello world");
        o.format_range(0, 5, &Attr::ToggleBold);
        let runs = &o.rich.as_ref().unwrap()[0].runs;
        assert_eq!((runs[0].text.as_str(), runs[0].bold), ("Hello", Some(!o.style.bold)));
        assert_eq!((runs[1].text.as_str(), runs[1].bold), (" world", None));
        o.format_range(0, 5, &Attr::ToggleBold);
        assert!(o.rich.is_none(), "back to plain text");
        assert_eq!(o.text, "Hello world");
    }

    #[test]
    fn typing_inherits_format_and_keeps_the_rest() {
        let mut o = obj("ab cd");
        o.format_range(0, 2, &Attr::ToggleItalic);
        o.edit_text("abX cd"); // typed inside the italic run
        let runs = &o.rich.as_ref().unwrap()[0].runs;
        assert_eq!(runs[0].text, "abX");
        assert_eq!(runs[0].italic, Some(true));
        assert_eq!(runs[1].italic, None);
        o.edit_text("abX c"); // delete from the plain tail
        assert_eq!(o.text, "abX c");
        assert_eq!(o.rich.as_ref().unwrap()[0].runs[0].text, "abX");
    }

    #[test]
    fn paragraphs_survive_formatting() {
        let mut o = obj("one\ntwo");
        o.format_range(4, 7, &Attr::Color(Rgba(255, 0, 0, 255)));
        let p = o.rich.as_ref().unwrap();
        assert_eq!(p.len(), 2);
        assert_eq!(p[1].runs[0].color, Some(Rgba(255, 0, 0, 255)));
        assert_eq!(o.text, "one\ntwo");
    }
}

/// Bullets or numbers at the start of each paragraph (they are ordinary text, so they edit and export like any other).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListStyle {
    None,
    Bullet,
    Number,
}

/// Length in bytes of a leading "• " or "12. " marker (0 when there is none).
pub fn marker_len(s: &str) -> usize {
    let t = s.trim_start_matches('\u{a0}');
    let pad = s.len() - t.len();
    if let Some(r) = t.strip_prefix("• ") { return pad + (t.len() - r.len()); }
    let digits = t.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits > 0 && t[digits..].starts_with(". ") { return pad + digits + 2; }
    0
}

impl Object {
    fn paragraph_texts(&self) -> Vec<String> {
        match &self.rich {
            Some(p) => p.iter().map(|p| p.runs.iter().map(|r| r.text.as_str()).collect()).collect(),
            None => self.text.split('\n').map(String::from).collect(),
        }
    }

    /// The list style of the first paragraph that has text.
    pub fn list_style(&self) -> ListStyle {
        let Some(first) = self.paragraph_texts().into_iter().find(|p| !p.trim().is_empty()) else { return ListStyle::None };
        let n = marker_len(&first);
        if n == 0 { ListStyle::None } else if first.trim_start_matches('\u{a0}').starts_with('•') { ListStyle::Bullet } else { ListStyle::Number }
    }

    /// Give every paragraph with text the bullet or number (or remove it). Formatting of the words is kept.
    pub fn set_list(&mut self, style: ListStyle) {
        let mut count = 0;
        let mut marker_for = |has_text: bool| -> String {
            if !has_text { return String::new(); }
            match style {
                ListStyle::None => String::new(),
                ListStyle::Bullet => "• ".into(),
                ListStyle::Number => { count += 1; format!("{count}. ") }
            }
        };
        match self.rich.as_mut() {
            Some(paras) => {
                for p in paras.iter_mut() {
                    let plain: String = p.runs.iter().map(|r| r.text.as_str()).collect();
                    let n = marker_len(&plain);
                    // Strip the old marker from the front of the runs.
                    let mut left = n;
                    for r in p.runs.iter_mut() {
                        if left == 0 { break; }
                        let cut = left.min(r.text.len());
                        r.text = r.text[cut..].to_string();
                        left -= cut;
                    }
                    p.runs.retain(|r| !r.text.is_empty());
                    let m = marker_for(!plain.trim().is_empty());
                    if !m.is_empty() { p.runs.insert(0, Run { text: m, ..Default::default() }); }
                }
                self.text = paras.iter().map(|p| p.runs.iter().map(|r| r.text.as_str()).collect::<String>()).collect::<Vec<_>>().join("\n");
            }
            None => {
                let lines: Vec<String> = self.text.split('\n').map(|l| {
                    let body = &l[marker_len(l)..];
                    format!("{}{}", marker_for(!body.trim().is_empty()), body)
                }).collect();
                self.text = lines.join("\n");
            }
        }
    }
}

#[cfg(test)]
mod list_tests {
    use super::*;

    fn obj(text: &str) -> Object {
        let mut o = Object::new("o", "o", Kind::Text, 0.0, 0.0, 10.0, 10.0);
        o.text = text.into();
        o
    }

    #[test]
    fn bullets_and_numbers_toggle_on_every_paragraph() {
        let mut o = obj("one\n\ntwo");
        o.set_list(ListStyle::Number);
        assert_eq!(o.text, "1. one\n\n2. two");
        assert_eq!(o.list_style(), ListStyle::Number);
        o.set_list(ListStyle::Bullet);
        assert_eq!(o.text, "• one\n\n• two");
        o.set_list(ListStyle::None);
        assert_eq!(o.text, "one\n\ntwo");
        assert_eq!(o.list_style(), ListStyle::None);
    }

    #[test]
    fn lists_keep_word_formatting() {
        let mut o = obj("a bold");
        o.rich = Some(vec![Paragraph { runs: vec![Run { text: "a ".into(), ..Default::default() }, Run { text: "bold".into(), bold: Some(true), ..Default::default() }], ..Default::default() }]);
        o.set_list(ListStyle::Bullet);
        assert_eq!(o.text, "• a bold");
        assert_eq!(o.rich.as_ref().unwrap()[0].runs.last().unwrap().bold, Some(true));
        o.set_list(ListStyle::None);
        assert_eq!(o.text, "a bold");
    }

    #[test]
    fn clear_removes_character_formatting_but_keeps_links() {
        let mut o = obj("hello world");
        o.format_range(0, 5, &Attr::ToggleBold);
        o.format_range(0, 5, &Attr::Size(60.0));
        o.format_range(6, 11, &Attr::Link(Some("https://example.com".into())));
        o.format_range(0, 11, &Attr::Clear);
        let f = o.char_fmts();
        assert!(f[..6].iter().all(|(_, f)| *f == Fmt::default()), "bold and size are gone");
        assert!(f[6..].iter().all(|(_, f)| f.link.is_some()), "the link stays");
    }

    #[test]
    fn strike_highlight_and_spacing_are_per_character_and_stay_minimal() {
        let mut o = obj("one two");
        o.format_range(0, 3, &Attr::ToggleStrike);
        o.format_range(4, 7, &Attr::Highlight(Some(Rgba(255, 240, 0, 255))));
        o.format_range(0, 7, &Attr::Spacing(2.0));
        let runs = &o.rich.as_ref().unwrap()[0].runs;
        assert_eq!(runs.len(), 3, "one, the space, two");
        assert_eq!((runs[0].strike, runs[0].spacing), (Some(true), Some(2.0)));
        assert_eq!(runs[2].highlight, Some(Rgba(255, 240, 0, 255)));
        o.format_range(0, 3, &Attr::ToggleStrike);
        assert_eq!(o.rich.as_ref().unwrap()[0].runs[0].strike, None, "toggled off again");
        o.format_range(0, 7, &Attr::Spacing(0.0));
        o.format_range(4, 7, &Attr::Highlight(None));
        assert!(o.rich.is_none(), "no formatting left, no rich text");
    }

    #[test]
    fn indent_moves_whole_paragraphs_and_survives_edits() {
        let mut o = obj("first\nsecond\nthird");
        o.format_range(7, 9, &Attr::Indent(1));
        let p = o.rich.as_ref().unwrap();
        assert_eq!(p.iter().map(|p| p.indent).collect::<Vec<_>>(), vec![0, 1, 0]);
        o.format_range(0, 19, &Attr::Indent(1));
        assert_eq!(o.rich.as_ref().unwrap().iter().map(|p| p.indent).collect::<Vec<_>>(), vec![1, 2, 1]);
        o.format_range(0, 19, &Attr::Indent(-1));
        o.format_range(0, 19, &Attr::Indent(-1));
        assert!(o.rich.is_none(), "all back to zero");
        o.format_range(0, 5, &Attr::Indent(1));
        o.edit_text("first!\nsecond\nthird");
        assert_eq!(o.rich.as_ref().unwrap()[0].indent, 1);
    }

    #[test]
    fn change_case_keeps_formatting() {
        let mut o = obj("hello world. how are you");
        o.format_range(0, 5, &Attr::ToggleBold);
        o.change_case(0, 24, Case::Upper);
        assert_eq!(o.text, "HELLO WORLD. HOW ARE YOU");
        assert_eq!(o.rich.as_ref().unwrap()[0].runs[0].bold, Some(true));
        o.change_case(0, 24, Case::Sentence);
        assert_eq!(o.text, "Hello world. How are you");
        o.change_case(0, 24, Case::Capitalize);
        assert_eq!(o.text, "Hello World. How Are You");
        o.change_case(0, 5, Case::Lower);
        assert_eq!(o.text, "hello World. How Are You");
        o.change_case(0, 5, Case::Toggle);
        assert_eq!(o.text, "HELLO World. How Are You");
    }
}
