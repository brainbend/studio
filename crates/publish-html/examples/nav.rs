// Slide navigation rules for the player tests: nav <out_dir>
use brainbend_doc::*;
fn page(id: &str) -> Slide {
    let mut s = Slide::new(id, id, 1920.0, 1080.0);
    let mut btn = Object::new(&format!("btn-{id}"), "Reveal", Kind::Rect, 10.0, 10.0, 200.0, 50.0);
    btn.text = "Reveal".into();
    let mut boxed = Object::new(&format!("box-{id}"), "Box", Kind::Text, 10.0, 100.0, 200.0, 50.0);
    boxed.text = format!("Box {id}");
    boxed.hidden = true;
    s.layers[0].objects = vec![btn, boxed];
    s.triggers.push(Trigger { id: format!("t-{id}"), owner: Owner::Object(format!("btn-{id}")), event: Event::Click, action: Action::ShowObject(format!("box-{id}")), conditions: vec![] });
    s
}
fn main() {
    let out = std::env::args().nth(1).unwrap();
    let mut p = Project::new("Navigation");
    p.show_menu = true;
    let mut a = page("a");
    a.layers[0].duration_ms = Some(150);
    a.nav.next = NextRule::AfterTimeline;
    let mut b = page("b");
    b.nav.previous_allowed = false;
    b.nav.revisit = Revisit::Resume;
    let mut c = page("c");
    c.nav.next = NextRule::Never;
    p.slides = vec![a, b, c];
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
