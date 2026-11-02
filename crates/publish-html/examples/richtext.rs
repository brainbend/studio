// A text box of rich paragraphs for the player tests: richtext <out_dir>
use brainbend_doc::*;
fn main() {
    let out = std::env::args().nth(1).unwrap();
    let mut p = Project::new("Rich");
    let mut a = Slide::new("a", "A", 1920.0, 1080.0);
    let run = |t: &str| Run { text: t.into(), ..Default::default() };
    let mut o = Object::new("t", "Text", Kind::Text, 100.0, 100.0, 800.0, 600.0);
    o.role = TextRole::H2;
    let mut h1 = Object::new("h1", "Main heading", Kind::Text, 100.0, 20.0, 800.0, 60.0);
    h1.text = "Main heading".into();
    h1.role = TextRole::H1;
    o.text = "First\n\nSoft\n• Bullet link".into();
    o.rich = Some(vec![
        Paragraph { runs: vec![run("First")], indent: 2 },
        Paragraph { runs: vec![run("")], ..Default::default() },
        Paragraph { runs: vec![run("Soft\n")], ..Default::default() },
        Paragraph { runs: vec![run("• Bullet "), Run { text: "link".into(), link: Some("https://example.com/".into()), underline: Some(true), color: Some(Rgba(5, 99, 193, 255)), ..Default::default() }, Run { text: " gone".into(), strike: Some(true), highlight: Some(Rgba(255, 240, 0, 255)), spacing: Some(2.0), ..Default::default() }], ..Default::default() },
    ]);
    // A perfectly vertical divider has no width.
    let mut divider = Object::new("d", "Divider", Kind::Line, 900.0, 100.0, 0.0, 80.0);
    divider.stroke = Some(Stroke { color: Rgba(1, 103, 103, 255), width: 2.0 });
    divider.decorative = true;
    let mut dotted = Object::new("dot", "Dotted", Kind::Rect, 100.0, 300.0, 200.0, 100.0);
    dotted.stroke = Some(Stroke { color: Rgba(255, 0, 0, 255), width: 3.0 });
    dotted.dash = LineDash::Dotted;
    a.layers[0].objects = vec![o, divider, dotted, h1];
    p.slides = vec![a];
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
