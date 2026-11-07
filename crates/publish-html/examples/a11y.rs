// A small course for accessibility tests: a button that opens a pop-up layer holding a Close button,
// a hover-only panel, a picture-less shape with alt text, a disabled button, and a second slide.
// Usage: a11y <out_dir>
use brainbend_doc::*;

fn text(id: &str, label: &str, x: f32, y: f32) -> Object {
    let mut o = Object::new(id, id, Kind::Rect, x, y, 300.0, 80.0);
    o.text = label.into();
    o.fill = Some(Rgba(3, 83, 158, 255));
    o.style.color = Rgba(255, 255, 255, 255);
    o
}

fn main() {
    let out = std::env::args().nth(1).unwrap();
    let mut p = Project::new("Access");
    p.variables.push(Variable { id: "v-hover".into(), name: "Hovered".into(), initial: Value::Num(0.0), built_in: false });
    let trig = |id: &str, owner: Owner, event: Event, action: Action| Trigger { id: id.into(), owner, event, action, conditions: vec![] };

    let open = text("open", "Open", 100.0, 100.0);
    let mut tip = text("tip", "Hover over me", 100.0, 220.0);
    tip.hidden = false;
    let mut badge = Object::new("badge", "Badge", Kind::Ellipse, 100.0, 340.0, 80.0, 80.0);
    badge.alt = "Gold star badge".into();
    badge.fill = Some(Rgba(240, 180, 0, 255));
    let mut locked = text("locked", "Locked", 100.0, 460.0);
    locked.initial_state = "Disabled".into();
    for name in ["Normal", "Disabled"] {
        locked.states.push(ObjectState { shine: None, gradient: None, name: name.into(), rich: None, fill: locked.fill, stroke: None, shadow: None, text: "Locked".into(), style: locked.style.clone(), asset: None, size: None });
    }
    let close = text("close", "Close", 100.0, 100.0);
    let next = text("next", "Next topic", 100.0, 100.0);

    p.slides.push(Slide {
        id: "s1".into(), name: "First topic".into(), width: 1920.0, height: 1080.0,
        layers: vec![
            Layer { id: "base".into(), name: "Base".into(), objects: vec![open, tip, badge, locked], duration_ms: None, modal: true, hide_others: false, pause_base: false, group_names: Default::default(), focus_order: vec![] },
            Layer { id: "pop".into(), name: "More detail".into(), objects: vec![close], duration_ms: None, modal: true, hide_others: false, pause_base: false, group_names: Default::default(), focus_order: vec![] },
        ],
        triggers: vec![
            trig("t1", Owner::Object("open".into()), Event::Click, Action::ShowLayer("pop".into())),
            trig("t2", Owner::Object("close".into()), Event::Click, Action::HideLayer("pop".into())),
            trig("t3", Owner::Object("tip".into()), Event::Hover, Action::SetVariable { variable: "v-hover".into(), value: Value::Num(1.0) }),
            trig("t4", Owner::Object("tip".into()), Event::MouseLeave, Action::SetVariable { variable: "v-hover".into(), value: Value::Num(2.0) }),
            trig("t5", Owner::Object("locked".into()), Event::Click, Action::SetVariable { variable: "v-hover".into(), value: Value::Num(9.0) }),
        ],
        kind: SlideKind::Normal, background: None, layout: None, fade_ms: 0, transition: Default::default(), hide_backdrop: false, auto_advance: false, nav: Default::default(), notes: String::new(), scene: String::new(),
    });
    p.slides.push(Slide {
        id: "s2".into(), name: "Second topic".into(), width: 1920.0, height: 1080.0,
        layers: vec![Layer { id: "base2".into(), name: "Base".into(), objects: vec![next], duration_ms: None, modal: true, hide_others: false, pause_base: false, group_names: Default::default(), focus_order: vec![] }],
        triggers: vec![trig("n1", Owner::Object("next".into()), Event::Click, Action::JumpToSlide("s1".into()))],
        kind: SlideKind::Normal, background: None, layout: None, fade_ms: 0, transition: Default::default(), hide_backdrop: false, auto_advance: false, nav: Default::default(), notes: String::new(), scene: String::new(),
    });
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
