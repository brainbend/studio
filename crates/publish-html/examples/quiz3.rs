// A quiz with one question of each passage/number/sorting style, for the player tests: quiz3 <out_dir>
use brainbend_doc::*;
fn question(id: &str, q: Question) -> Slide {
    let mut s = Slide::new(id, id, 1920.0, 1080.0);
    s.kind = SlideKind::Question(q);
    s
}
fn main() {
    let out = std::env::args().nth(1).unwrap();
    let mut p = Project::new("More styles");
    p.pass_mark = 75;
    p.show_menu = true;
    let mut fill = Question::new_fill_blanks();
    fill.style = QuestionStyle::FillBlanks { text: "A *cat/feline* and a *dog*.".into(), case_sensitive: false };
    let mut num = Question::new_numeric();
    #[allow(clippy::approx_constant)] // 3.14 is the quiz answer on purpose
    let numeric = QuestionStyle::Numeric { answer: 3.14, tolerance: 0.01 };
    num.style = numeric;
    p.assets.insert("pic".into(), Asset { name: "pic.png".into(), mime: "image/png".into(), bytes: vec![137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 218, 99, 252, 207, 192, 80, 15, 0, 4, 133, 1, 128, 132, 169, 140, 33, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130], file: None });
    let mut hot = Question::new_hotspot();
    hot.style = QuestionStyle::Hotspot { asset: Some("pic".into()), alt: "A map".into(), regions: vec![HotRegion { x: 0.1, y: 0.1, w: 0.3, h: 0.3, ellipse: false }, HotRegion { x: 0.6, y: 0.6, w: 0.3, h: 0.3, ellipse: true }] };
    let mut results = Slide::new("res", "Results", 1920.0, 1080.0);
    results.kind = SlideKind::Results;
    p.slides = vec![
        question("fill", fill),
        question("drag", Question::new_drag_words()),
        question("mark", Question::new_mark_words()),
        question("num", num),
        question("sort", Question::new_categorize()),
        question("hot", hot),
        results,
    ];
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
