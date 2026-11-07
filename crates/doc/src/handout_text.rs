//! A question slide as lines of text, for handouts that list it instead of drawing it (Word and PDF): the question and
//! its answers (correct ones ticked, or a blank worksheet).
use crate::*;

pub struct Line {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub indent: u8,
}

fn line(text: impl Into<String>) -> Line { Line { text: text.into(), bold: false, italic: false, indent: 0 } }
fn item(text: impl Into<String>) -> Line { Line { indent: 1, ..line(text) } }
fn note(text: impl Into<String>) -> Line { Line { italic: true, ..line(text) } }

fn sorted(mut items: Vec<String>) -> Vec<String> {
    items.sort_by_key(|x| x.to_lowercase());
    items
}

pub fn question_lines(q: &Question, project: &Project, answers: bool) -> Vec<Line> {
    let mut out = vec![Line { bold: true, ..line(&q.prompt) }];
    match &q.style {
        QuestionStyle::Choice => {
            for c in &q.choices { out.push(item(format!("{} {}", if answers && c.correct { "\u{2611}" } else { "\u{2610}" }, c.text))); }
            if answers { out.push(note("\u{2611} marks a correct answer.")); }
        }
        QuestionStyle::TextEntry { accepted, .. } => {
            out.push(line(if answers { format!("Accepted answers: {}", accepted.join(", ")) } else { "Answer: ______________________".into() }));
        }
        QuestionStyle::Matching { pairs } => {
            for (a, b) in pairs { out.push(item(format!("{a}  \u{2192}  {}", if answers { b.as_str() } else { "______________" }))); }
            if !answers { out.push(note(format!("Match each one with: {}", sorted(pairs.iter().map(|(_, b)| b.clone()).collect()).join(", ")))); }
        }
        QuestionStyle::Sequence { items } => {
            if answers { for (n, i) in items.iter().enumerate() { out.push(item(format!("{}. {i}", n + 1))); } }
            else { out.push(line(format!("Put in order: {}", sorted(items.clone()).join(", ")))); }
        }
        QuestionStyle::FillBlanks { text, .. } => {
            out.push(line(crate::passage::with_gaps(text)));
            if answers { out.push(note(format!("Answers: {}", crate::passage::blanks(text).iter().map(|a| a.join(" / ")).collect::<Vec<_>>().join(", ")))); }
        }
        QuestionStyle::DragWords { text, distractors } => {
            let mut bank: Vec<String> = crate::passage::blanks(text).into_iter().map(|a| a[0].clone()).collect();
            bank.extend(distractors.iter().cloned());
            if !answers { bank = sorted(bank); }
            out.push(line(crate::passage::with_gaps(text)));
            out.push(note(format!("Word bank: {}", bank.join(", "))));
        }
        QuestionStyle::MarkWords { text } => {
            let mut t = String::new();
            for part in crate::passage::parse(text) {
                match part {
                    crate::passage::Part::Text(x) => t.push_str(&x),
                    crate::passage::Part::Blank(a) => { if answers { t.push_str(&format!("[{}]", a[0])); } else { t.push_str(&a[0]); } }
                }
            }
            out.push(line(t));
            out.push(note(if answers { "Words in [brackets] are the ones to mark." } else { "Mark the correct words." }));
        }
        QuestionStyle::Numeric { answer, tolerance } => {
            out.push(line(if answers { format!("Answer: {answer}{}", if *tolerance > 0.0 { format!(" (within {tolerance})") } else { String::new() }) } else { "Answer: ______________".into() }));
        }
        QuestionStyle::Hotspot { regions, .. } => {
            out.push(note(if answers { format!("Click the correct area of the picture ({} marked in the lesson).", regions.len()) } else { "Mark the correct area of the picture.".to_string() }));
        }
        QuestionStyle::Likert { statements, scale, .. } => {
            out.push(note(format!("Scale: {}", scale.join(" / "))));
            for s in statements { out.push(item(format!("{s}  {}", "\u{2610} ".repeat(scale.len()).trim_end()))); }
            out.push(note("Survey: not graded."));
        }
        QuestionStyle::Reflection { min_words } => {
            out.push(line("______________________________________________"));
            out.push(note(format!("Free-text answer, not graded.{}", if *min_words > 0 { format!(" At least {min_words} words.") } else { String::new() })));
        }
        QuestionStyle::Dictation { text, .. } => {
            out.push(line(if answers { format!("Audio question. Transcript: {text}") } else { "Audio question. Write what you hear: ______________________".into() }));
        }
        QuestionStyle::Dropdowns { text } => {
            let mut t = String::new();
            for part in crate::passage::parse(text) {
                match part {
                    crate::passage::Part::Text(x) => t.push_str(&x),
                    crate::passage::Part::Blank(a) => {
                        if answers { t.push_str(&format!("[{}{}]", a[0], a[1..].iter().map(|w| format!(" / {w}")).collect::<String>())); }
                        else { t.push_str(&format!("[{}]", sorted(a.clone()).join(" / "))); }
                    }
                }
            }
            out.push(line(t));
            if answers { out.push(note("The first option in each bracket is correct.")); }
        }
        QuestionStyle::LabelPicture { zones, distractors, .. } => {
            if answers {
                for (n, z) in zones.iter().enumerate() { out.push(item(format!("{}. {}", n + 1, z.label))); }
                if !distractors.is_empty() { out.push(note(format!("Spare labels: {}", distractors.join(", ")))); }
            } else {
                let labels: Vec<String> = zones.iter().map(|z| z.label.clone()).chain(distractors.iter().cloned()).collect();
                out.push(note(format!("Label the numbered areas of the picture with: {}", sorted(labels).join(", "))));
            }
        }
        QuestionStyle::Categorize { categories } => {
            for (name, items) in categories { out.push(item(format!("{name}: {}", if answers { items.join(", ") } else { "______________".into() }))); }
            if !answers { out.push(note(format!("Sort these: {}", sorted(categories.iter().flat_map(|(_, i)| i.iter().cloned()).collect()).join(", ")))); }
        }
    }
    let _ = project;
    out.push(note(format!("Worth {} point(s). Correct: {} Incorrect: {}", q.points, q.feedback_correct, q.feedback_incorrect)));
    out
}

/// A normal slide's visible text, top to bottom then left to right, one line per paragraph. For handouts that have no
/// picture of the slide to show.
pub fn slide_text_lines(slide: &Slide) -> Vec<Line> {
    let mut objects: Vec<&Object> = slide.layers.first().map(|l| l.objects.iter().collect()).unwrap_or_default();
    objects.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
    let mut out = vec![];
    for o in objects {
        if let Some(rich) = &o.rich {
            for p in rich {
                let text: String = p.runs.iter().map(|r| r.text.as_str()).collect();
                if !text.trim().is_empty() { out.push(Line { text, bold: false, italic: false, indent: p.indent }); }
            }
        } else {
            for l in o.text.lines().filter(|l| !l.trim().is_empty()) { out.push(line(l)); }
        }
    }
    out
}
