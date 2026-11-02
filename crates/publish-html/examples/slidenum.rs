// Three slides that each show their own number: slidenum <out_dir>
use brainbend_doc::*;
fn main() {
    let out = std::env::args().nth(1).unwrap();
    let mut p = Project::new("Numbers");
    p.slides = (0..3).map(|i| {
        let mut s = Slide::new(&format!("s{i}"), &format!("Slide {i}"), 1920.0, 1080.0);
        let mut o = Object::new("n", "Number", Kind::Text, 100.0, 100.0, 600.0, 80.0);
        o.text = "Slide %Slide.Number% of %Slide.Total%".into();
        s.layers[0].objects = vec![o];
        s
    }).collect();
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
