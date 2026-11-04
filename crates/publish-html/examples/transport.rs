// A slide with a 3 second timeline and one without, with play/pause, seekbar and replay: transport <out_dir> [icons]
use brainbend_doc::*;
fn text(id: &str, timing: Option<Timing>) -> Object {
    let mut o = Object::new(id, id, Kind::Text, 10.0, 10.0, 200.0, 50.0);
    o.text = id.into();
    o.timing = timing;
    o
}
fn main() {
    let out = std::env::args().nth(1).unwrap();
    let mut p = Project::new("Transport");
    p.player.controls = PlayerControls { play_pause: true, seekbar: true, replay: true, ..Default::default() };
    if std::env::args().nth(2).as_deref() == Some("icons") {
        p.player.buttons = ButtonStyle::Icon;
    }
    let mut a = Slide::new("a", "Timed", 1920.0, 1080.0);
    a.layers[0].duration_ms = Some(3000);
    a.layers[0].objects = vec![
        text("early", Some(Timing { start_ms: 0, dur_ms: Some(1000) })),
        text("late", Some(Timing { start_ms: 1500, dur_ms: None })),
        text("always", None),
    ];
    let mut b = Slide::new("b", "Plain", 1920.0, 1080.0);
    b.layers[0].objects = vec![text("still", None)];
    p.slides = vec![a, b];
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
