// A button whose Visited and Selected states each hold a shape of their own (a checkmark, a badge): stateshapes <out_dir>
use brainbend_doc::*;
fn main() {
    let out = std::env::args().nth(1).unwrap();
    let mut p = Project::new("State shapes");
    let mut a = Slide::new("a", "A", 1920.0, 1080.0);
    let mut go = Object::new("go", "Go", Kind::Rect, 100.0, 100.0, 300.0, 100.0);
    go.text = "Go".into();
    go.fill = Some(Rgba(20, 90, 160, 255));
    let state = |name: &str, extras: Vec<Object>| ObjectState { extras, shine: None, gradient: None, name: name.into(), rich: None, fill: go.fill, stroke: None, shadow: None, text: "Go".into(), style: go.style.clone(), asset: None, size: None };
    let mut tick = Object::new("tick", "Tick", Kind::Text, 6.0, 8.0, 40.0, 40.0);
    tick.text = "TICK".into();
    let mut badge = Object::new("badge", "Badge", Kind::Rect, 250.0, 4.0, 40.0, 20.0);
    badge.text = "SEL".into();
    go.states = vec![state("Normal", vec![]), state("Hover", vec![]), state("Visited", vec![tick]), state("Selected", vec![badge])];
    go.initial_state = "Normal".into();
    let mut other = Object::new("other", "Other", Kind::Rect, 100.0, 300.0, 300.0, 100.0);
    other.text = "Other".into();
    a.layers[0].objects = vec![go, other];
    a.triggers = vec![Trigger { id: "t1".into(), owner: Owner::Object("other".into()), event: Event::Click, action: Action::SetObjectState { object: "go".into(), state: "Selected".into() }, conditions: vec![] }];
    p.slides = vec![a];
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
