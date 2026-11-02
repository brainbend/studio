// Slide actions and events for the player tests: actions <out_dir>
use brainbend_doc::*;
fn trig(id: &str, owner: Owner, event: Event, action: Action) -> Trigger {
    Trigger { id: id.into(), owner, event, action, conditions: vec![] }
}
fn main() {
    let out = std::env::args().nth(1).unwrap();
    let mut p = Project::new("Actions");
    p.variables.push(Variable { id: "n".into(), name: "N".into(), initial: Value::Num(10.0), built_in: false });
    p.variables.push(Variable { id: "s".into(), name: "S".into(), initial: Value::Text("a".into()), built_in: false });
    let mut a = Slide::new("a", "A", 1920.0, 1080.0);
    let mut btn = Object::new("btn", "Go", Kind::Rect, 10.0, 10.0, 200.0, 50.0);
    btn.text = "Go".into();
    let mut target = Object::new("target", "Target", Kind::Text, 10.0, 100.0, 200.0, 50.0);
    target.text = "Target".into();
    target.hidden = true;
    p.assets.insert("clip".into(), Asset { name: "clip.mp4".into(), mime: "video/mp4".into(), bytes: vec![0, 0, 0, 24, 102, 116, 121, 112], file: None });
    let mut vid = Object::new("vid", "Clip", Kind::Video, 300.0, 10.0, 400.0, 225.0);
    vid.asset = Some("clip".into());
    a.layers[0].objects = vec![btn, target, vid];
    a.triggers = vec![
        trig("t1", Owner::Object("btn".into()), Event::Click, Action::ShowObject("target".into())),
        trig("t2", Owner::Object("btn".into()), Event::Click, Action::AdjustVariable { variable: "n".into(), op: AdjustOp::Add, value: Value::Num(5.0) }),
        trig("t3", Owner::Object("btn".into()), Event::Click, Action::AdjustVariable { variable: "s".into(), op: AdjustOp::Append, value: Value::Text("b".into()) }),
        trig("t4", Owner::Object("btn".into()), Event::MouseLeave, Action::AdjustVariable { variable: "n".into(), op: AdjustOp::Multiply, value: Value::Num(2.0) }),
        trig("t5", Owner::Object("btn".into()), Event::DoubleClick, Action::HideObject("target".into())),
        trig("t6", Owner::Object("btn".into()), Event::DoubleClick, Action::OpenUrl("https://example.com/".into())),
        trig("t9", Owner::Object("btn".into()), Event::DoubleClick, Action::PauseMedia("vid".into())),
        trig("t10", Owner::Object("target".into()), Event::Click, Action::StopMedia("vid".into())),
        trig("t8", Owner::Object("vid".into()), Event::MediaReaches(1500), Action::AdjustVariable { variable: "n".into(), op: AdjustOp::Add, value: Value::Num(100.0) }),
        trig("t7", Owner::Slide, Event::TimelineReaches(120), Action::AdjustVariable { variable: "n".into(), op: AdjustOp::Subtract, value: Value::Num(1.0) }),
    ];
    p.slides = vec![a];
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
