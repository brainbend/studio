// A picture cropped to an oval and one to a rounded rectangle, for the player tests: maskpic <out_dir>
use brainbend_doc::*;
fn main() {
    let out = std::env::args().nth(1).unwrap();
    let mut p = Project::new("Masks");
    let png = vec![137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 218, 99, 252, 207, 192, 80, 15, 0, 4, 133, 1, 128, 132, 169, 140, 33, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130];
    p.assets.insert("pic".into(), Asset { name: "pic.png".into(), mime: "image/png".into(), bytes: png, file: None });
    let mut a = Slide::new("a", "A", 1920.0, 1080.0);
    let mut oval = Object::new("oval", "Oval", Kind::Image, 100.0, 100.0, 400.0, 300.0);
    oval.asset = Some("pic".into());
    oval.alt = "An oval picture".into();
    oval.mask = Some(PictureShape::Oval);
    oval.crop = Some([0.1, 0.0, 0.1, 0.0]);
    let mut plain = Object::new("plain", "Plain", Kind::Image, 600.0, 100.0, 400.0, 300.0);
    plain.asset = Some("pic".into());
    plain.alt = "A plain picture".into();
    let mut round = Object::new("round", "Round", Kind::Image, 1100.0, 100.0, 600.0, 300.0);
    round.asset = Some("pic".into());
    round.alt = "A rounded picture".into();
    round.mask = Some(PictureShape::Rounded);
    a.layers[0].objects = vec![oval, plain, round];
    p.slides = vec![a];
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
