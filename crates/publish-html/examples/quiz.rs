// Builds an intro + 2 questions + results course: quiz <out_dir> [scorm12|scorm2004]
use brainbend_doc::*;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mut p = Project::new("Quiz demo");
    p.pass_mark = 50;
    let mut intro = Slide::new("intro", "Intro", 1920.0, 1080.0);
    intro.layers[0].objects.push(Object {
        id: "go".into(), name: "Start".into(), kind: Kind::Rect, x: 700.0, y: 400.0, w: 500.0, h: 140.0,
        fill: Some(Rgba(3, 83, 158, 255)), stroke: None, shadow: None, glow: None, soft_edge: 0.0, reflection: None, hidden: false, margins: [0.0; 4], radius: 0.0, alt: String::new(), text_rotation: 0, role: TextRole::Normal, marker: None, equation: None, captions: None, rich: None, entrance: None, exit: None, timing: None, text: "Start quiz".into(),
        style: TextStyle { font_family: "Open Sans".into(), size: 48.0, bold: true, italic: false, underline: false, color: Rgba(255, 255, 255, 255), align: Align::Center, valign: VAlign::Middle },
        asset: None, states: vec![], initial_state: String::new(), placeholder: None, gradient: None, rotation: 0.0, line_spacing: 0.0, group: None, table: None, table_span: None, table_borders: None, emphasis: None, motion: None, click_steps: Default::default(), decorative: false, crop: None, dial: None, check: None, hit_area: None, mask: None, dash: LineDash::Solid, media: Default::default(), hide_on_layers: vec![], button_set: None, poster: None,
    });
    intro.triggers.push(Trigger { id: "t".into(), owner: Owner::Object("go".into()), event: Event::Click, action: Action::NextSlide, conditions: vec![] });
    let mut q1 = Slide::new("q1", "Q1", 1920.0, 1080.0);
    q1.kind = SlideKind::Question(Question::new_single());
    let mut q2 = Slide::new("q2", "Q2", 1920.0, 1080.0);
    let mut qq = Question::new_single();
    qq.multiple = true;
    qq.prompt = "Pick both correct".into();
    qq.choices = vec![
        Choice { id: "a".into(), text: "One".into(), correct: true },
        Choice { id: "b".into(), text: "Two".into(), correct: true },
        Choice { id: "c".into(), text: "Three".into(), correct: false },
    ];
    qq.points = 3.0;
    q2.kind = SlideKind::Question(qq);
    let mut r = Slide::new("res", "Results", 1920.0, 1080.0);
    r.kind = SlideKind::Results;
    p.slides = vec![intro, q1, q2, r];
    let out = std::path::Path::new(&a[1]);
    match a.get(2).map(String::as_str) {
        Some("scorm12") => brainbend_publish_html::export_scorm(&p, brainbend_publish_html::Lms::Scorm12, out).unwrap(),
        Some("scorm2004") => brainbend_publish_html::export_scorm(&p, brainbend_publish_html::Lms::Scorm2004, out).unwrap(),
        _ => brainbend_publish_html::export(&p, out).unwrap(),
    }
}
