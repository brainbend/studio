// A quiz with one question of each style, for the player tests: quiz2 <out_dir>
use brainbend_doc::*;
fn question(id: &str, q: Question) -> Slide {
    let mut s = Slide::new(id, id, 1920.0, 1080.0);
    s.kind = SlideKind::Question(q);
    s
}
fn main() {
    let out = std::env::args().nth(1).unwrap();
    let mut p = Project::new("Styles");
    p.pass_mark = 75;
    p.show_menu = true;
    let mut text = Question::new_text_entry();
    text.prompt = "Capital of France?".into();
    text.style = QuestionStyle::TextEntry { accepted: vec!["Paris".into(), "Paris, France".into()], case_sensitive: false };
    let mut tf = Question::new_true_false();
    tf.prompt = "The sky is blue.".into();
    let mut results = Slide::new("res", "Results", 1920.0, 1080.0);
    results.kind = SlideKind::Results;
    p.slides = vec![
        question("text", text),
        question("match", Question::new_matching()),
        question("seq", Question::new_sequence()),
        question("tf", tf),
        results,
    ];
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
