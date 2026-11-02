//! A course whose look comes from the master and layouts: gradient master background, a master
//! logo, one slide on a layout that overrides the background with a colour, one with a picture.
use brainbend_doc::*;

fn main() {
    let out = std::env::args().nth(1).expect("output dir");
    let mut p = Project::new("Master demo");
    p.slides.push(Slide::new("s1", "Follows the master", 640.0, 360.0));
    p.slides.push(Slide::new("s2", "Own colour", 640.0, 360.0));
    p.slides.push(Slide::new("s3", "Picture", 640.0, 360.0));
    p.sanitize();
    p.master.background = Some(Background::gradient(90.0, Rgba(255, 0, 0, 255), Rgba(0, 0, 255, 255)));
    let mut logo = Object::new("logo", "Logo", Kind::Text, 10.0, 10.0, 100.0, 30.0);
    logo.text = "MASTERLOGO".into();
    p.master.objects.push(logo);
    for s in p.slides.iter_mut() { s.layout = Some("layout-blank".into()); }
    p.slides[1].background = Some(Background::Solid(Rgba(0, 128, 0, 255)));
    // 1x1 transparent PNG.
    let png: Vec<u8> = vec![
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 0x0d, 0x49, 0x48, 0x44, 0x52, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0, 0x1f, 0x15, 0xc4, 0x89,
        0, 0, 0, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8, 0xff, 0xff, 0x3f, 0, 5, 0xfe, 2, 0xfe, 0xa7, 0x35, 0x81, 0x84, 0, 0, 0, 0, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];
    p.assets.insert("bg".into(), Asset { name: "bg.png".into(), mime: "image/png".into(), bytes: png, file: None });
    p.slides[2].background = Some(Background::Image { asset: "bg".into() });
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
