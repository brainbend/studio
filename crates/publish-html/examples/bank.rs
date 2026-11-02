// A quiz with a question bank scene (draw 2 of 4), for the player tests: bank <out_dir> [scorm12]
use brainbend_doc::*;
fn question(id: &str, scene: &str) -> Slide {
    let mut q = Question::new_single();
    q.prompt = format!("Question {id}");
    let mut s = Slide::new(id, id, 1920.0, 1080.0);
    s.kind = SlideKind::Question(q);
    s.scene = scene.into();
    s
}
fn main() {
    let out = std::env::args().nth(1).unwrap();
    let mode = std::env::args().nth(2);
    let mut p = Project::new("Bank");
    p.pass_mark = 50;
    p.show_menu = true;
    let mut intro = Slide::new("intro", "Intro", 1920.0, 1080.0);
    intro.scene = "s1".into();
    let mut results = Slide::new("res", "Results", 1920.0, 1080.0);
    results.kind = SlideKind::Results;
    results.scene = "s3".into();
    p.slides = vec![intro, question("q1", "s2"), question("q2", "s2"), question("q3", "s2"), question("q4", "s2"), results];
    p.scenes = vec![
        Scene { id: "s1".into(), name: "Intro".into(), bank: None },
        Scene { id: "s2".into(), name: "Quiz".into(), bank: Some(Bank { draw: 2, shuffle: false }) },
        Scene { id: "s3".into(), name: "End".into(), bank: None },
    ];
    let out = std::path::Path::new(&out);
    match mode.as_deref() {
        Some("scorm12") => brainbend_publish_html::export_scorm(&p, brainbend_publish_html::Lms::Scorm12, out).unwrap(),
        _ => brainbend_publish_html::export(&p, out).unwrap(),
    }
}
