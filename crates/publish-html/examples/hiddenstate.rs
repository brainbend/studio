// An object that starts in the Hidden state and is shown while the pointer is over another: hiddenstate <out_dir>
use brainbend_doc::*;
fn main() {
    let out = std::env::args().nth(1).unwrap();
    let mut p = Project::new("Hidden");
    let mut a = Slide::new("a", "A", 1920.0, 1080.0);
    let mut b = Object::new("box", "Box", Kind::Rect, 100.0, 100.0, 300.0, 100.0);
    b.text = "Box".into();
    let mut tip = Object::new("tip", "Tip", Kind::Rect, 100.0, 220.0, 300.0, 100.0);
    tip.text = "Tip".into();
    tip.initial_state = "Hidden".into();
    tip.hidden = true;
    a.layers[0].objects = vec![b, tip];
    let t = |id: &str, ev: Event, state: &str| Trigger { id: id.into(), owner: Owner::Object("box".into()), event: ev, action: Action::SetObjectState { object: "tip".into(), state: state.into() }, conditions: vec![] };
    // Two layers that do not block the base layer; the second hides the first when it shows.
    let mut one = Object::new("one", "One", Kind::Rect, 500.0, 100.0, 200.0, 100.0);
    one.text = "L1".into();
    let mut two = Object::new("two", "Two", Kind::Rect, 500.0, 100.0, 200.0, 100.0);
    two.text = "L2".into();
    a.layers.push(Layer { id: "l1".into(), name: "L1".into(), objects: vec![one], duration_ms: None, modal: false, hide_others: true, pause_base: false, group_names: Default::default(), focus_order: vec![] });
    a.layers.push(Layer { id: "l2".into(), name: "L2".into(), objects: vec![two], duration_ms: None, modal: true, hide_others: true, pause_base: false, group_names: Default::default(), focus_order: vec![] });
    let show = |id: &str, owner: &str, layer: &str| Trigger { id: id.into(), owner: Owner::Object(owner.into()), event: Event::Click, action: Action::ShowLayer(layer.into()), conditions: vec![] };
    a.triggers = vec![t("t1", Event::Hover, "Normal"), t("t2", Event::MouseLeave, "Hidden"), show("s1", "box", "l1"), show("s2", "tip", "l2")];
    p.slides = vec![a];
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
