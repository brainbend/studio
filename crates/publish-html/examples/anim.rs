// Emphasis and motion-path animations for the player tests: anim <out_dir>
use brainbend_doc::*;
fn shape(id: &str, x: f32) -> Object {
    let mut o = Object::new(id, id, Kind::Text, x, 100.0, 200.0, 60.0);
    o.text = id.into();
    o
}
fn main() {
    let out = std::env::args().nth(1).unwrap();
    let mut p = Project::new("Animations");
    let mut a = Slide::new("a", "A", 1920.0, 1080.0);
    let mut spin = shape("spin", 100.0);
    spin.emphasis = Some(Emphasis { kind: EmphasisKind::Spin, delay_ms: 50, duration_ms: 600, repeat: 2 });
    let mut mover = shape("mover", 100.0);
    mover.motion = Some(MotionPath::preset(PathPreset::Right));
    let mut curved = shape("curved", 100.0);
    curved.motion = Some(MotionPath::preset(PathPreset::Arc));
    let mut later = shape("later", 100.0);
    later.timing = Some(Timing { start_ms: 120, dur_ms: None });
    later.emphasis = Some(Emphasis { kind: EmphasisKind::Pulse, delay_ms: 0, duration_ms: 300, repeat: 0 });
    let mut btn = Object::new("btn", "Again", Kind::Rect, 600.0, 400.0, 200.0, 60.0);
    btn.text = "Again".into();
    a.layers[0].objects = vec![spin, mover, curved, later, btn];
    a.layers[0].duration_ms = Some(2000);
    a.triggers.push(Trigger { id: "t".into(), owner: Owner::Object("btn".into()), event: Event::Click, action: Action::PlayAnimation("spin".into()), conditions: vec![] });
    p.slides = vec![a];
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
