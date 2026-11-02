// Survey, free text, dictation, drop-down and labelling questions for the player tests: quiz4 <out_dir>
use brainbend_doc::*;
fn question(id: &str, q: Question) -> Slide {
    let mut s = Slide::new(id, id, 1920.0, 1080.0);
    s.kind = SlideKind::Question(q);
    s
}
fn main() {
    let out = std::env::args().nth(1).unwrap();
    let mut p = Project::new("More styles 2");
    p.pass_mark = 75;
    p.show_menu = true;
    p.assets.insert("pic".into(), Asset { name: "pic.png".into(), mime: "image/png".into(), bytes: vec![137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 218, 99, 252, 207, 192, 80, 15, 0, 4, 133, 1, 128, 132, 169, 140, 33, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130], file: None });
    p.assets.insert("clip".into(), Asset { name: "clip.mp3".into(), mime: "audio/mpeg".into(), bytes: vec![73, 68, 51], file: None });
    let mut reflect = Question::new_reflection();
    reflect.style = QuestionStyle::Reflection { min_words: 3 };
    let mut dict = Question::new_dictation();
    dict.style = QuestionStyle::Dictation { asset: Some("clip".into()), text: "Safety first.".into(), max_plays: 2 };
    let mut label = Question::new_label_picture();
    let zone = |x: f32, label: &str| LabelZone { region: HotRegion { x, y: 0.2, w: 0.3, h: 0.3, ellipse: false }, label: label.into() };
    label.style = QuestionStyle::LabelPicture { asset: Some("pic".into()), alt: "A diagram".into(), zones: vec![zone(0.1, "Engine"), zone(0.6, "Wheel")], distractors: vec!["Wing".into()] };
    let mut results = Slide::new("res", "Results", 1920.0, 1080.0);
    results.kind = SlideKind::Results;
    p.variables.push(Variable { id: "score".into(), name: "Score".into(), initial: Value::Num(0.0), built_in: false });
    p.slides = vec![
        question("likert", {
            let mut q = Question::new_likert();
            if let QuestionStyle::Likert { reversed, total_variable, .. } = &mut q.style { *reversed = vec![false, true]; *total_variable = Some("score".into()); }
            q
        }),
        question("reflect", reflect),
        question("dict", dict),
        question("drop", Question::new_dropdowns()),
        question("label", label),
        results,
    ];
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
