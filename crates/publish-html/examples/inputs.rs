// Radio buttons and entry fields for the player tests: inputs <out_dir>
use brainbend_doc::*;
fn main() {
    let out = std::env::args().nth(1).unwrap();
    let mut p = Project::new("Inputs");
    p.variables.push(Variable { id: "v-name".into(), name: "Name".into(), initial: Value::Text("hi".into()), built_in: false });
    p.variables.push(Variable { id: "v-age".into(), name: "Age".into(), initial: Value::Num(7.0), built_in: false });
    let mut a = Slide::new("a", "A", 1920.0, 1080.0);
    let radio = |id: &str, group: &str, y: f32, on: bool| {
        let mut o = Object::new(id, id, Kind::Text, 100.0, y, 600.0, 80.0);
        o.text = id.to_string();
        o.check = Some(Check { radio: Some(group.into()), ..Check::new() });
        o.margins = [40.0, 0.0, 0.0, 0.0];
        if on { o.initial_state = "Selected".into(); }
        o
    };
    let mut name = Object::new("name", "Name field", Kind::Input(InputField::text("v-name")), 800.0, 100.0, 600.0, 80.0);
    name.alt = "Your name".into();
    name.style.size = 30.0;
    let mut age = Object::new("age", "Age field", Kind::Input(InputField { variable: "v-age".into(), numeric: true, placeholder: String::new(), min: 0.0, max: 120.0, step: 1.0 }), 800.0, 220.0, 300.0, 80.0);
    age.alt = "Your age".into();
    let mut hello = Object::new("hello", "Hello", Kind::Text, 800.0, 340.0, 800.0, 80.0);
    hello.text = "Hello %Name%, age %Age%".into();
    a.layers[0].objects = vec![radio("one", "g", 100.0, true), radio("two", "g", 200.0, false), radio("three", "g", 300.0, false), radio("other", "h", 400.0, true), name, age, hello];
    p.slides = vec![a];
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
