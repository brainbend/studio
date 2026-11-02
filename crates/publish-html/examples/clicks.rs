// Animations that wait for clicks, for the player tests: clicks <out_dir>
use brainbend_doc::*;
fn shape(id: &str, x: f32) -> Object {
    let mut o = Object::new(id, id, Kind::Text, x, 100.0, 200.0, 60.0);
    o.text = id.into();
    o
}
fn main() {
    let out = std::env::args().nth(1).unwrap();
    let mut p = Project::new("Click steps");
    let mut a = Slide::new("a", "A", 1920.0, 1080.0);
    let fade = Entrance { kind: EffectKind::Fade, delay_ms: 0, duration_ms: 100 };
    let mut one = shape("one", 100.0);
    one.entrance = Some(fade);
    one.click_steps.entrance = 1;
    let mut two = shape("two", 400.0);
    two.entrance = Some(fade);
    two.click_steps.entrance = 1;
    two.emphasis = Some(Emphasis { kind: EmphasisKind::Spin, delay_ms: 0, duration_ms: 500, repeat: 1 });
    two.click_steps.emphasis = 2;
    let mut three = shape("three", 700.0);
    three.exit = Some(Entrance { kind: EffectKind::Fade, delay_ms: 0, duration_ms: 100 });
    three.click_steps.exit = 3;
    let mut auto = shape("auto", 1000.0);
    auto.entrance = Some(fade);
    a.layers[0].objects = vec![one, two, three, auto];
    let b = Slide::new("b", "B", 1920.0, 1080.0);
    p.slides = vec![a, b];
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
