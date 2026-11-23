// A button with a chevron in Hover and a check-like shape in Visited, starting in the state given: statelook <out_dir> <Normal|Hover|Visited>
use brainbend_doc::shapes::{preset, Preset};
use brainbend_doc::*;
fn main() {
    let out = std::env::args().nth(1).unwrap();
    let start = std::env::args().nth(2).unwrap_or("Normal".into());
    let mut p = Project::new("State look");
    let mut a = Slide::new("a", "A", 1920.0, 1080.0);
    let mut go = Object::new("go", "Go", Kind::Rect, 600.0, 400.0, 600.0, 200.0);
    go.text = "Button".into();
    go.fill = Some(Rgba(20, 90, 160, 255));
    let shape = |id: &str, name: &str, preset_name: &str, x: f32, color: Rgba| {
        let (w, h) = (160.0, 120.0);
        let Preset::Path(cmds) = preset(preset_name, w, h, &|_, d| d) else { panic!("not a path") };
        let mut o = Object::new(id, name, Kind::Path(cmds), x, 40.0, w, h);
        o.fill = Some(color);
        o
    };
    let state = |name: &str, extras: Vec<Object>| ObjectState { extras, shine: None, gradient: None, name: name.into(), rich: None, fill: go.fill, stroke: None, shadow: None, text: "Button".into(), style: go.style.clone(), asset: None, size: None };
    let arrow = shape("arrow", "Arrow", "chevron", 400.0, Rgba(255, 230, 0, 255));
    let tick = shape("tick", "Tick", "rightArrow", 20.0, Rgba(255, 0, 200, 255));
    go.states = vec![state("Normal", vec![]), state("Hover", vec![arrow]), state("Visited", vec![tick])];
    go.initial_state = start;
    a.layers[0].objects = vec![go];
    p.slides = vec![a];
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
