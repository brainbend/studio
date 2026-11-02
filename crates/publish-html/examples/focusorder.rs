// Three text objects whose focus order differs from their stacking order, for the player tests: focusorder <out_dir>
use brainbend_doc::*;
fn main() {
    let out = std::env::args().nth(1).unwrap();
    let mut p = Project::new("Focus");
    let mut a = Slide::new("a", "A", 1920.0, 1080.0);
    let text = |id: &str, y: f32| {
        let mut o = Object::new(id, id, Kind::Text, 100.0, y, 400.0, 60.0);
        o.text = id.to_uppercase();
        o
    };
    a.layers[0].objects = vec![text("first", 100.0), text("second", 200.0), text("third", 300.0)];
    a.layers[0].focus_order = vec!["third".into(), "first".into(), "gone".into()];
    p.slides = vec![a];
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
