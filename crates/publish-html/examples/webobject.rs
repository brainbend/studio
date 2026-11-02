// Web objects for the player tests: webobject <out_dir>
use brainbend_doc::*;
fn main() {
    let out = std::env::args().nth(1).unwrap();
    let mut p = Project::new("Web");
    let mut a = Slide::new("a", "A", 1920.0, 1080.0);
    let web = |id: &str, url: &str, alt: &str, scroll: bool| {
        let mut o = Object::new(id, id, Kind::Web(WebObject { url: url.into(), scroll }), 100.0, 100.0, 800.0, 500.0);
        o.alt = alt.into();
        o
    };
    a.layers[0].objects = vec![
        web("glossary", "https://example.com/glossary", "Course glossary", false),
        web("scripty", "javascript:alert(1)", "Not a page", true),
        web("blank", "  ", "Empty", true),
    ];
    p.slides = vec![a];
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
