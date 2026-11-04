// A button in a gallery style (a gradient in every state, a drop shadow, an outline when pressed): buttonstyle <out_dir>
use brainbend_doc::*;
fn main() {
    let out = std::env::args().nth(1).unwrap();
    let mut p = Project::new("Button style");
    let mut s = Slide::new("a", "Style", 1920.0, 1080.0);
    let mut o = Object::new("b", "b", Kind::Rect, 100.0, 100.0, 156.0, 68.0);
    o.text = "Go".into();
    let drop = Some(Shadow { dx: 0.0, dy: 4.0, blur: 3.5, spread: 0.0, color: Rgba(0, 0, 0, 74) });
    let grad = |top: Rgba, bottom: Rgba| Some(Gradient { angle: 180.0, stops: vec![(0.0, top), (1.0, bottom)] });
    let state = |name: &str, g: Option<Gradient>, stroke: Option<Stroke>| ObjectState {
        shine: None, gradient: g, name: name.into(), rich: None, fill: Some(Rgba(79, 129, 189, 255)), stroke, shadow: drop,
        text: "Go".into(), style: o.style.clone(), asset: None, size: None,
    };
    o.radius = 5.0;
    o.states = vec![
        state("Normal", grad(Rgba(110, 179, 215, 255), Rgba(47, 78, 162, 255)), Some(Stroke { color: Rgba(0, 0, 0, 102), width: 1.0 })),
        state("Hover", grad(Rgba(121, 198, 225, 255), Rgba(35, 59, 152, 255)), None),
    ];
    o.initial_state = "Normal".into();
    s.layers[0].objects = vec![o];
    p.slides = vec![s];
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
