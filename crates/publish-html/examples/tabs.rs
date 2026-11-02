// A tabs interaction, for the player tests: tabs <out_dir>
//
// Slide A: three tab buttons with two-line rich labels and Normal/Hover/Selected states. Each shows its own
// layer (the layers hide each other); the base body text and "character" are hidden while any of them shows.
// Slide B: two layers that stack, and a Previous-button trigger that replaces the move back.
use brainbend_doc::*;

fn rich_label(first: &str, second: &str) -> Vec<Paragraph> {
    vec![
        Paragraph { runs: vec![Run { text: first.into(), bold: Some(true), ..Default::default() }], ..Default::default() },
        Paragraph { runs: vec![Run { text: second.into(), ..Default::default() }], ..Default::default() },
    ]
}

fn tab(id: &str, n: u32, x: f32) -> Object {
    let mut o = Object::new(id, id, Kind::Rect, x, 400.0, 200.0, 60.0);
    let rich = rich_label(&format!("Label {n}"), "Short description");
    o.text = format!("Label {n}\nShort description");
    o.rich = Some(rich.clone());
    o.fill = Some(Rgba(70, 90, 110, 255));
    let state = |name: &str, fill: Rgba| ObjectState { shine: None, name: name.into(), rich: Some(rich.clone()), fill: Some(fill), stroke: None, shadow: None, text: o.text.clone(), style: o.style.clone(), asset: None, size: None };
    o.states = vec![state("Normal", Rgba(70, 90, 110, 255)), state("Hover", Rgba(130, 150, 170, 255)), state("Selected", Rgba(30, 40, 50, 255))];
    o.initial_state = "Normal".into();
    o.button_set = Some("tabs".into());
    o
}

fn text(id: &str, t: &str, y: f32) -> Object {
    let mut o = Object::new(id, id, Kind::Text, 20.0, y, 400.0, 60.0);
    o.text = t.into();
    o
}

fn click(id: &str, owner: &str, layer: &str) -> Trigger {
    Trigger { id: id.into(), owner: Owner::Object(owner.into()), event: Event::Click, action: Action::ShowLayer(layer.into()), conditions: vec![] }
}

fn layer(id: &str, objects: Vec<Object>, hide_others: bool) -> Layer {
    Layer { id: id.into(), name: id.into(), objects, duration_ms: None, modal: false, hide_others, pause_base: false, group_names: Default::default(), focus_order: vec![] }
}

fn main() {
    let out = std::env::args().nth(1).unwrap();
    let mut p = Project::new("Tabs");
    let mut a = Slide::new("a", "A", 1280.0, 720.0);
    let layers: Vec<String> = (1..=3).map(|n| format!("l{n}")).collect();
    let mut body = text("body", "Base body", 100.0);
    body.hide_on_layers = layers.clone();
    let mut character = text("char", "Base character", 200.0);
    character.hide_on_layers = layers.clone();
    a.layers[0].objects = vec![text("title", "Title", 20.0), body, character, tab("t1", 1, 20.0), tab("t2", 2, 240.0), tab("t3", 3, 460.0)];
    for n in 1..=3 {
        a.layers.push(layer(&format!("l{n}"), vec![text(&format!("lb{n}"), &format!("Layer {n} body"), 100.0)], true));
        a.triggers.push(click(&format!("c{n}"), &format!("t{n}"), &format!("l{n}")));
    }
    let mut b = Slide::new("b", "B", 1280.0, 720.0);
    let mut s1 = Object::new("s1", "S1", Kind::Rect, 20.0, 400.0, 200.0, 60.0);
    s1.text = "Show one".into();
    let mut s2 = Object::new("s2", "S2", Kind::Rect, 240.0, 400.0, 200.0, 60.0);
    s2.text = "Show two".into();
    // A button with Normal, Hover and Visited states.
    let mut v = Object::new("v", "V", Kind::Rect, 460.0, 400.0, 200.0, 60.0);
    v.text = "Visit me".into();
    v.fill = Some(Rgba(10, 20, 30, 255));
    let look = |name: &str, fill: Rgba| ObjectState { shine: None, name: name.into(), rich: None, fill: Some(fill), stroke: None, shadow: None, text: "Visit me".into(), style: v.style.clone(), asset: None, size: None };
    v.states = vec![look("Normal", Rgba(10, 20, 30, 255)), look("Hover", Rgba(60, 70, 80, 255)), look("Visited", Rgba(100, 5, 5, 255))];
    v.initial_state = "Normal".into();
    b.layers[0].objects = vec![s1, s2, v, text("btitle", "Stacking layers", 20.0)];
    b.layers.push(layer("k1", vec![text("k1t", "Stacked one", 100.0)], false));
    b.layers.push(layer("k2", vec![text("k2t", "Stacked two", 160.0)], false));
    b.triggers = vec![
        click("sc1", "s1", "k1"),
        click("sc2", "s2", "k2"),
        // The player's Previous button replaces the move back; it is not a click on the slide.
        Trigger { id: "prev".into(), owner: Owner::Slide, event: Event::PreviousButton, action: Action::ShowLayer("k1".into()), conditions: vec![] },
    ];
    p.slides = vec![a, b];
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
