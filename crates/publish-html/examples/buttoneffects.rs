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
    let mut plain = plain; plain.y = 900.0; plain.name = "plain".into();
    let star: Vec<PathCmd> = {
        let mut v = vec![];
        for i in 0..10 {
            let ang = (i as f32 * 36.0 - 90.0).to_radians();
            let rad = if i % 2 == 0 { 0.5 } else { 0.2 };
            let (x, y) = (0.5 + rad * ang.cos(), 0.5 + rad * ang.sin());
            v.push(if i == 0 { PathCmd::Move(x, y) } else { PathCmd::Line(x, y) });
        }
        v.push(PathCmd::Close);
        v
    };
    let mut star_soft = Object::new("star", "star", Kind::Path(star.clone()), 100.0, 500.0, 300.0, 300.0);
    star_soft.fill = Some(Rgba(3, 83, 158, 255));
    star_soft.soft_edge = 24.0;
    let mut star_refl = Object::new("starglow", "starglow", Kind::Path(star), 600.0, 450.0, 300.0, 300.0);
    star_refl.fill = Some(Rgba(3, 83, 158, 255));
    star_refl.glow = Some(Glow { radius: 24.0, color: Rgba(255, 190, 40, 255) });
    star_refl.reflection = Some(Reflection { size: 0.6, opacity: 0.5, gap: 6.0 });
    let mut oval = Object::new("oval", "oval", Kind::Ellipse, 1100.0, 450.0, 360.0, 160.0);
    oval.fill = Some(Rgba(3, 83, 158, 255));
    oval.soft_edge = 30.0;
    s.layers[0].objects = vec![glow, soft, refl, plain, star_soft, star_refl, oval];
    p.slides = vec![s];
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
