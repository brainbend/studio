//! Writes a sample handout: cargo run -p brainbend-export-pdf --example pdf_sample out.pdf [slide.png] [per_page] [landscape]
use brainbend_doc::*;
use brainbend_publish_html::{HandoutOptions, HandoutOrientation};
fn main() {
    let mut args = std::env::args().skip(1);
    let out = args.next().expect("a path to write");
    let pic = args.next().map(|p| std::fs::read(p).unwrap());
    let per: u8 = args.next().map(|v| v.parse().unwrap()).unwrap_or(3);
    let portrait = args.next().as_deref() != Some("landscape");
    let mut p = Project::new("Sample lesson");
    for (i, name) in ["Welcome", "Safety", "Quiz"].iter().enumerate() {
        let mut s = Slide::new(&format!("s{i}"), name, 1920.0, 1080.0);
        s.notes = "Greet everyone and explain what the session covers. This note is long enough to wrap onto more than one line.".into();
        p.slides.push(s);
    }
    p.slides[2].kind = SlideKind::Question(Question { prompt: "Which is right?".into(), multiple: false, choices: vec![Choice { id: "a".into(), text: "Wear a helmet".into(), correct: true }, Choice { id: "b".into(), text: "Run".into(), correct: false }], points: 1.0, feedback_correct: "Yes".into(), feedback_incorrect: "No".into(), style: QuestionStyle::Choice, area: None });
    let mut pics = std::collections::BTreeMap::new();
    if let Some(b) = pic { pics.insert(0, b.clone()); pics.insert(1, b); }
    let o = HandoutOptions { orientation: if portrait { HandoutOrientation::Portrait } else { HandoutOrientation::Landscape }, per_page: per, slide_notes: true, alt_text: true, note_lines: true, ..Default::default() };
    std::fs::write(out, brainbend_export_pdf::export(&p, &[0, 1, 2], &o, &pics).unwrap()).unwrap();
}
