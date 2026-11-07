// A question slide that also carries shapes and a button, for the player tests: qobjs <out_dir>
use brainbend_doc::*;
fn main() {
    let out = std::env::args().nth(1).unwrap();
    let mut p = Project::new("Question with objects");
    let mut s = Slide::new("q1", "Q1", 1920.0, 1080.0);
    s.kind = SlideKind::Question(Question { area: Some(QuestionArea { x: 100.0, y: 150.0, w: 1000.0, h: 700.0 }), ..Question::new_single() });
    let mut hint = Object::new("hint", "Hint button", Kind::Rect, 1500.0, 20.0, 300.0, 80.0);
    hint.text = "Hint".into();
    let mut note = Object::new("note", "Hint text", Kind::Text, 1500.0, 120.0, 300.0, 80.0);
    note.text = "Think about the color of the sky".into();
    note.hidden = true;
    let badge = Object::new("badge", "Badge", Kind::Ellipse, 20.0, 20.0, 100.0, 100.0);
    s.layers[0].objects = vec![hint, note, badge];
    s.triggers = vec![Trigger { id: "t1".into(), owner: Owner::Object("hint".into()), event: Event::Click, action: Action::ShowObject("note".into()), conditions: vec![] }];
    p.slides = vec![s];
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
