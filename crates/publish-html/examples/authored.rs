// Builds a tiny course from scratch (as the editor would) and exports it: authored <out_dir>
use brainbend_doc::*;
fn obj(id: &str, kind: Kind, text: &str) -> Object {
    Object {
        id: id.into(), name: id.into(), kind, x: 100.0, y: 100.0, w: 300.0, h: 100.0,
        fill: Some(Rgba(3, 83, 158, 255)), stroke: None, shadow: None, hidden: false, margins: [0.0; 4], radius: 0.0, alt: String::new(), text_rotation: 0, role: TextRole::Normal, marker: None, equation: None, captions: None, rich: None, entrance: None, exit: None, timing: None,
        text: text.into(),
        style: TextStyle { font_family: "Open Sans".into(), size: 40.0, bold: true, italic: false, underline: false, color: Rgba(255, 255, 255, 255), align: Align::Center, valign: VAlign::Middle },
        asset: None, states: vec![], initial_state: String::new(), placeholder: None, gradient: None, rotation: 0.0, line_spacing: 0.0, group: None, table: None, table_span: None, table_borders: None, emphasis: None, motion: None, click_steps: Default::default(), decorative: false, crop: None, dial: None, check: None, hit_area: None, mask: None, dash: LineDash::Solid, media: Default::default(), hide_on_layers: vec![], button_set: None, poster: None,
    }
}
fn main() {
    let out = std::env::args().nth(1).unwrap();
    let mut p = Project::new("Authored");
    p.variables.push(Variable { id: "v-clicks".into(), name: "Clicked".into(), initial: Value::Bool(false), built_in: false });
    p.variables.push(Variable { id: "v-n".into(), name: "Count".into(), initial: Value::Num(0.0), built_in: false });
    let trig = |id: &str, owner: Owner, event: Event, action: Action, conditions: Vec<Condition>| Trigger { id: id.into(), owner, event, action, conditions };
    p.slides.push(Slide {
        id: "s1".into(), name: "S".into(), width: 1920.0, height: 1080.0,
        layers: vec![
            Layer { id: "base".into(), name: "Base".into(), objects: vec![obj("btn", Kind::Rect, "Click me"), { let mut o = obj("ln", Kind::LineUp, ""); o.decorative = true; o.fill = None; o.stroke = Some(Stroke { color: Rgba(255, 0, 0, 255), width: 6.0 }); o }, { let mut o = obj("lnk", Kind::Text, "See docs now"); o.rich = Some(vec![Paragraph { runs: vec![Run { text: "See ".into(), ..Default::default() }, Run { text: "docs".into(), link: Some("https://example.com/docs".into()), ..Default::default() }, Run { text: " now".into(), ..Default::default() }], ..Default::default() }]); o }], duration_ms: None, modal: true, hide_others: false, pause_base: false, group_names: Default::default(), focus_order: vec![] },
            Layer { id: "pop".into(), name: "Popup".into(), objects: vec![{ let mut o = obj("msg", Kind::Text, "Hello Count=%Count%"); o.entrance = Some(Entrance { kind: EffectKind::Fade, delay_ms: 100, duration_ms: 300 }); o }], duration_ms: None, modal: true, hide_others: false, pause_base: false, group_names: Default::default(), focus_order: vec![] },
        ],
        triggers: vec![
            trig("t1", Owner::Object("btn".into()), Event::Click, Action::SetVariable { variable: "v-clicks".into(), value: Value::Bool(true) }, vec![]),
            // Only shows the popup once Clicked is true (set by the first trigger).
            trig("t2", Owner::Slide, Event::VariableChanged("v-clicks".into()), Action::ShowLayer("pop".into()),
                 vec![Condition { variable: "v-clicks".into(), cmp: Cmp::Eq, value: Value::Bool(true), or_with_previous: false }]),
            trig("t4", Owner::Slide, Event::KeyPress("k".into()), Action::ShowLayer("pop".into()), vec![]),
            trig("t5", Owner::Object("btn".into()), Event::Hover, Action::RunJavaScript("GetPlayer().SetVar('Count', 41);".into()), vec![]),
            trig("t3", Owner::Object("btn".into()), Event::Click, Action::RunJavaScript("var p=GetPlayer(); p.SetVar('Count', p.GetVar('Count')+1);".into()), vec![]),
        ],
        kind: SlideKind::Normal, background: None, layout: None, fade_ms: 0, hide_backdrop: false, auto_advance: false, nav: Default::default(), notes: String::new(), scene: String::new(),
    });
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
