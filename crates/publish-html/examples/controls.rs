// A slide with a slider, a dial, a picture that changes with a variable, and two check boxes that enable a
// Submit button, for the player tests:
// controls <out_dir>
use brainbend_doc::*;

fn main() {
    let out = std::env::args().nth(1).unwrap();
    let mut p = Project::new("Controls");
    let png = vec![137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 218, 99, 252, 207, 192, 80, 15, 0, 4, 133, 1, 128, 132, 169, 140, 33, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130];
    // Two different files (identical ones would be stored once, with one address).
    for (k, extra) in [("a", 0u8), ("b", 1)] {
        let mut bytes = png.clone();
        bytes.push(extra);
        p.assets.insert(k.into(), Asset { name: format!("{k}.png"), mime: "image/png".into(), bytes, file: None });
    }
    p.variables.push(Variable { id: "v".into(), name: "Level".into(), initial: Value::Num(5.0), built_in: false });
    p.variables.push(Variable { id: "w".into(), name: "Turn".into(), initial: Value::Num(5.0), built_in: false });
    let mut s = Slide::new("s1", "Controls", 1920.0, 1080.0);

    let mut slider = Object::new("slider", "Level slider", Kind::Slider(Slider { variable: "v".into(), min: 0.0, max: 10.0, step: 1.0, thumb_height: 0.5, ..Slider::new() }), 100.0, 400.0, 800.0, 80.0);
    slider.alt = "Level".into();
    let mut dial = Object::new("dial", "Dial", Kind::Rect, 200.0, 100.0, 100.0, 100.0);
    dial.fill = Some(Rgba(200, 50, 50, 255));
    dial.dial = Some(Dial { variable: "w".into(), min: 0.0, max: 10.0, step: 1.0, sweep: 180.0, pivot: (0.0, 0.0) });
    let mut pic = Object::new("pic", "Picture", Kind::Image, 500.0, 100.0, 100.0, 50.0);
    pic.asset = Some("a".into());
    pic.initial_state = "A".into();
    let state = |name: &str, asset: &str, size: (f32, f32)| ObjectState { shine: None, gradient: None, name: name.into(), rich: None, fill: None, stroke: None, shadow: None, text: String::new(), style: pic.style.clone(), asset: Some(asset.into()), size: Some(size) };
    pic.states = vec![state("A", "a", (100.0, 50.0)), state("B", "b", (60.0, 30.0))];
    let check = |id: &str, y: f32| {
        let mut o = Object::new(id, id, Kind::Text, 100.0, y, 600.0, 60.0);
        o.text = id.into();
        o.check = Some(Check::new());
        o.margins = [32.0, 0.0, 0.0, 0.0];
        o.initial_state = "Normal".into();
        o
    };
    let mut submit = Object::new("submit", "Submit", Kind::Rect, 100.0, 700.0, 200.0, 60.0);
    submit.text = "Submit".into();
    submit.initial_state = "Disabled".into();
    let look = |name: &str, fill: Rgba| ObjectState { shine: None, gradient: None, name: name.into(), rich: None, fill: Some(fill), stroke: None, shadow: None, text: "Submit".into(), style: submit.style.clone(), asset: None, size: None };
    submit.states = vec![look("Normal", Rgba(3, 83, 158, 255)), look("Disabled", Rgba(150, 150, 150, 255))];
    let mut spot = Object::new("spot", "Triangle", Kind::Hotspot, 1000.0, 100.0, 200.0, 200.0);
    spot.hit_area = Some(vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)]);
    s.layers[0].objects.extend([slider, dial, pic, check("box1", 500.0), check("box2", 580.0), submit, spot]);
    let when = |id: &str, n: f64, to: &str| Trigger {
        id: id.into(),
        owner: Owner::Object("slider".into()),
        event: Event::VariableChanged("v".into()),
        action: Action::SetObjectState { object: "pic".into(), state: to.into() },
        conditions: vec![Condition { variable: "v".into(), cmp: Cmp::Eq, value: Value::Num(n), or_with_previous: false }],
    };
    let watch = |id: &str, boxed: &str| Trigger {
        id: id.into(),
        owner: Owner::Object("submit".into()),
        event: Event::StateChanged(boxed.into(), "Selected".into()),
        action: Action::SetObjectState { object: "submit".into(), state: "Normal".into() },
        conditions: vec![],
    };
    s.triggers = vec![when("t7", 7.0, "B"), when("t3", 3.0, "A"), watch("w1", "box1"), watch("w2", "box2")];
    p.slides = vec![s];
    p.sanitize();
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
