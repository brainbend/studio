// A timed course for the player tests: timeline <out_dir>
use brainbend_doc::*;
fn text(id: &str, timing: Option<Timing>, exit: bool) -> Object {
    let mut o = Object::new(id, id, Kind::Text, 10.0, 10.0, 200.0, 50.0);
    o.text = id.into();
    o.timing = timing;
    if exit {
        o.exit = Some(Entrance { kind: EffectKind::Wipe, delay_ms: 0, duration_ms: 30 });
    }
    o.entrance = Some(Entrance { kind: EffectKind::Fade, delay_ms: 0, duration_ms: 20 });
    o
}
fn main() {
    let out = std::env::args().nth(1).unwrap();
    let mut p = Project::new("Timeline");
    p.variables.push(Variable { id: "done".into(), name: "Done".into(), initial: Value::Bool(false), built_in: false });
    let mut a = Slide::new("a", "A", 1920.0, 1080.0);
    a.auto_advance = true;
    a.layers[0].duration_ms = Some(300);
    a.layers[0].objects = vec![
        text("early", Some(Timing { start_ms: 0, dur_ms: Some(100) }), true),
        text("late", Some(Timing { start_ms: 150, dur_ms: None }), false),
        text("always", None, false),
    ];
    let mut b = Slide::new("b", "B", 1920.0, 1080.0);
    b.layers[0].duration_ms = Some(100);
    b.triggers.push(Trigger {
        id: "end".into(), owner: Owner::Slide, event: Event::SlideEnd,
        action: Action::SetVariable { variable: "done".into(), value: Value::Bool(true) }, conditions: vec![],
    });
    p.slides = vec![a, b];
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
