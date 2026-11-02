// Timeline seeking and the course actions, for the player tests: course <out.zip> (a SCORM 1.2 package)
use brainbend_doc::*;
fn trig(id: &str, owner: Owner, event: Event, action: Action) -> Trigger {
    Trigger { id: id.into(), owner, event, action, conditions: vec![] }
}
fn button(id: &str, text: &str, y: f32) -> Object {
    let mut b = Object::new(id, text, Kind::Rect, 10.0, y, 200.0, 50.0);
    b.text = text.into();
    b
}
fn main() {
    let out = std::env::args().nth(1).unwrap();
    let mut p = Project::new("Course");
    p.variables.push(Variable { id: "n".into(), name: "N".into(), initial: Value::Num(0.0), built_in: false });
    let mut a = Slide::new("a", "A", 1920.0, 1080.0);
    let mut late = Object::new("late", "Late", Kind::Text, 400.0, 10.0, 200.0, 50.0);
    late.text = "Late".into();
    late.timing = Some(Timing { start_ms: 400, dur_ms: None });
    a.layers[0].objects = vec![late, button("seek0", "Seek0", 100.0), button("seek600", "Seek600", 160.0), button("bump", "Bump", 220.0), button("done", "Done", 280.0), button("leave", "Leave", 340.0), button("next", "Next", 400.0)];
    let on = |id: &str| Owner::Object(id.into());
    a.triggers = vec![
        trig("t1", on("seek0"), Event::Click, Action::SeekTimeline { target: String::new(), ms: 0 }),
        trig("t2", on("seek600"), Event::Click, Action::SeekTimeline { target: String::new(), ms: 600 }),
        trig("t3", on("bump"), Event::Click, Action::AdjustVariable { variable: "n".into(), op: AdjustOp::Add, value: Value::Num(1.0) }),
        trig("t4", on("done"), Event::Click, Action::CompleteCourse),
        trig("t5", on("leave"), Event::Click, Action::ExitCourse),
        trig("t6", on("next"), Event::Click, Action::JumpToSlide("b".into())),
    ];
    let mut b = Slide::new("b", "B", 1920.0, 1080.0);
    b.layers[0].objects = vec![button("restart", "Restart", 100.0)];
    b.triggers = vec![trig("t7", on("restart"), Event::Click, Action::RestartCourse)];
    p.slides = vec![a, b];
    p.sanitize();
    brainbend_publish_html::export_scorm(&p, brainbend_publish_html::Lms::Scorm12, std::path::Path::new(&out)).unwrap();
}
