// 360° images for the player tests: one that turns on its own, with annotations, and one that is plain: panorama <out_dir>
use brainbend_doc::pano::{Pano, PanoPin};
use brainbend_doc::*;
fn main() {
    let out = std::env::args().nth(1).unwrap();
    let mut p = Project::new("Panoramas");
    let png = vec![137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 218, 99, 252, 207, 192, 80, 15, 0, 4, 133, 1, 128, 132, 169, 140, 33, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130];
    p.assets.insert("pic".into(), Asset { name: "lab.png".into(), mime: "image/png".into(), bytes: png, file: None });
    let mut a = Slide::new("a", "A", 1920.0, 1080.0);
    let pin = |id: &str, yaw: f32, pitch: f32, title: &str, description: &str| { let mut m = PanoPin::marker(id.into(), yaw, pitch); m.title = title.into(); m.description = description.into(); m };
    let mut tour = Pano::new();
    tour.auto_rotate = true;
    tour.pins = vec![
        pin("door", 20.0, 0.0, "Door", "The way out. <img src=x onerror=\"window.pwned=1\">"),
        pin("hood", 180.0, 0.0, "Fume hood", "Behind you."),
        pin("untitled", -30.0, 10.0, "", ""),
    ];
    let mut lamp = pin("lamp", 5.0, 10.0, "Lamp", "");
    lamp.button = true;
    tour.pins.push(lamp);
    // A hotspot: a shaded area with a card of its own, and its own color.
    let mut bench = brainbend_doc::pano::PanoPin::hotspot("bench".into(), vec![(30.0, -6.0), (42.0, -6.0), (42.0, 8.0), (30.0, 8.0)]);
    bench.title = "Bench".into();
    bench.description = "A workbench.".into();
    bench.color = Some(brainbend_doc::Rgba(200, 40, 40, 255));
    tour.pins.push(bench);
    // The first marker wears an icon: a square drawn in a 0..1 box.
    tour.pins[0].icon = "plus".into();
    tour.pins[0].icon_path = vec![brainbend_doc::PathCmd::Move(0.25, 0.25), brainbend_doc::PathCmd::Line(0.75, 0.25), brainbend_doc::PathCmd::Line(0.75, 0.75), brainbend_doc::PathCmd::Line(0.25, 0.75), brainbend_doc::PathCmd::Close];
    let mut o1 = Object::new("tour", "Lab tour", Kind::Pano(tour), 100.0, 100.0, 1280.0, 720.0);
    o1.asset = Some("pic".into());
    o1.alt = "A teaching laboratory with a door and a fume hood".into();
    let mut plain = Pano::new();
    plain.zoom = false;
    let mut o2 = Object::new("plain", "Plain", Kind::Pano(plain), 1400.0, 100.0, 400.0, 225.0);
    o2.asset = Some("pic".into());
    let mut guided = Pano::new();
    guided.tour = true;
    guided.reveal = false;
    guided.dark = false;
    guided.rounded = false;
    let mut g1 = PanoPin::marker("g1".into(), 0.0, 0.0);
    g1.title = "Start".into();
    g1.description = "Where the tour begins.".into();
    let mut g2 = PanoPin::hotspot("g2".into(), vec![(100.0, -5.0), (120.0, -5.0), (120.0, 10.0), (100.0, 10.0)]);
    g2.title = "Shelf".into();
    g2.description = "Chemicals live here.".into();
    g2.hint = "Behind you, on the right".into();
    let mut g3 = PanoPin::marker("g3".into(), -120.0, 5.0);
    g3.title = "Door".into();
    g3.description = "The way out.".into();
    guided.pins = vec![g1, g2, g3];
    let mut o3 = Object::new("guided", "Guided", Kind::Pano(guided), 100.0, 700.0, 640.0, 360.0);
    o3.asset = Some("pic".into());
    o3.alt = "A guided tour of the lab".into();
    a.layers[0].objects = vec![o1, o2, o3];
    a.triggers = vec![brainbend_doc::Trigger {
        id: "t1".into(),
        owner: brainbend_doc::Owner::Object("tour".into()),
        event: brainbend_doc::Event::PanoPin("tour".into(), "lamp".into()),
        action: brainbend_doc::Action::RunJavaScript("window.lampPressed = (window.lampPressed || 0) + 1;".into()),
        conditions: vec![],
    }];
    p.slides = vec![a];
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
