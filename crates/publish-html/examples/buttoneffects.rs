// Buttons with a glow, soft edges and a reflection, and a freeform shape with a glow: buttoneffects <out_dir>
use brainbend_doc::*;
fn button(id: &str, x: f32) -> Object {
    let mut o = Object::new(id, id, Kind::Rect, x, 200.0, 360.0, 120.0);
    o.fill = Some(Rgba(3, 83, 158, 255));
    o.radius = 10.0;
    o.text = id.into();
    o.style.color = Rgba(255, 255, 255, 255);
    o.style.size = 40.0;
    o
}
fn main() {
    let out = std::env::args().nth(1).unwrap();
    let mut p = Project::new("Button effects");
    let mut s = Slide::new("a", "Effects", 1920.0, 1080.0);
    let mut glow = button("glow", 100.0);
    glow.glow = Some(Glow { radius: 24.0, color: Rgba(255, 190, 40, 255) });
    let mut soft = button("soft", 600.0);
    soft.soft_edge = 24.0;
    let mut refl = button("reflect", 1100.0);
    refl.reflection = Some(Reflection { size: 0.6, opacity: 0.5, gap: 6.0 });
    let plain = button("plain", 1100.0);
    let mut plain = plain; plain.y = 700.0; plain.name = "plain".into();
    s.layers[0].objects = vec![glow, soft, refl, plain];
    p.slides = vec![s];
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
