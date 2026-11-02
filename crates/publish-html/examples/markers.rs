// A marker with a title and a description, and one with only a title, for the player tests: markers <out_dir>
use brainbend_doc::*;
fn main() {
    let out = std::env::args().nth(1).unwrap();
    let mut p = Project::new("Markers");
    let mut a = Slide::new("a", "A", 1920.0, 1080.0);
    let marker = |id: &str, x: f32, y: f32, title: &str, desc: &str| {
        let mut o = Object::new(id, id, Kind::Ellipse, x, y, 120.0, 120.0);
        o.fill = Some(Rgba(58, 122, 214, 255));
        o.marker = Some(MarkerInfo { title: title.into(), description: desc.into(), card_w: 640.0, card_h: 340.0 });
        o
    };
    // A freeform marker with a shadow: the shadow must follow the round outline, not a box.
    let mut ring = marker("ring", 400.0, 300.0, "Round", "A round marker with a soft shadow");
    ring.kind = Kind::Path({
        let k = 0.5523_f32;
        vec![PathCmd::Move(1.0, 0.5), PathCmd::Curve(1.0, 0.5 + 0.5 * k, 0.5 + 0.5 * k, 1.0, 0.5, 1.0), PathCmd::Curve(0.5 - 0.5 * k, 1.0, 0.0, 0.5 + 0.5 * k, 0.0, 0.5), PathCmd::Curve(0.0, 0.5 - 0.5 * k, 0.5 - 0.5 * k, 0.0, 0.5, 0.0), PathCmd::Curve(0.5 + 0.5 * k, 0.0, 1.0, 0.5 - 0.5 * k, 1.0, 0.5), PathCmd::Close]
    });
    ring.shadow = Some(Shadow { dx: 6.0, dy: 8.0, blur: 14.0, spread: 0.0, color: Rgba(0, 0, 0, 120) });
    a.layers[0].objects = vec![
        ring,
        marker("tv", 900.0, 600.0, "Testing", "Testing testing testing"),
        marker("top", 1700.0, 20.0, "Near the top", "A card that has to go below this marker"),
        marker("tiponly", 100.0, 700.0, "Just a title", ""),
    ];
    p.slides = vec![a];
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
