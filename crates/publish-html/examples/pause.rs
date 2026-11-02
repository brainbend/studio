// A layer that pauses the base layer's timeline, for the player tests: pause <out_dir>
//
// The base layer has an object that appears 400 ms in and a button that shows a layer; that layer pauses the base
// timeline and has a button that hides it again. (An interactive video's question layers work the same way.)
use brainbend_doc::*;

fn text(id: &str, t: &str, x: f32) -> Object {
    let mut o = Object::new(id, id, Kind::Rect, x, 100.0, 200.0, 60.0);
    o.text = t.into();
    o
}

fn click(id: &str, owner: &str, action: Action) -> Trigger {
    Trigger { id: id.into(), owner: Owner::Object(owner.into()), event: Event::Click, action, conditions: vec![] }
}

fn main() {
    let out = std::env::args().nth(1).unwrap();
    let mut p = Project::new("Pause");
    let mut a = Slide::new("a", "A", 1280.0, 720.0);
    let mut later = text("later", "Later", 20.0);
    later.timing = Some(Timing { start_ms: 400, dur_ms: None });
    a.layers[0].objects = vec![later, text("open", "Open", 300.0)];
    a.layers[0].duration_ms = Some(2000);
    a.layers.push(Layer { id: "q".into(), name: "Question".into(), objects: vec![text("close", "Close", 20.0)], duration_ms: None, modal: true, hide_others: false, pause_base: true, group_names: Default::default(), focus_order: vec![] });
    a.triggers = vec![click("o", "open", Action::ShowLayer("q".into())), click("c", "close", Action::HideLayer("q".into()))];
    p.slides = vec![a];
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
