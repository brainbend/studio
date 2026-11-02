// An equation shape that carries MathML, and one whose MathML tries to carry script, for the player tests: equation <out_dir>
use brainbend_doc::*;
fn main() {
    let out = std::env::args().nth(1).unwrap();
    let mut p = Project::new("Equations");
    let mut a = Slide::new("a", "A", 1920.0, 1080.0);
    let shape = |id: &str, x: f32, latex: &str, mathml: &str, alt: &str| {
        let mut o = Object::new(id, id, Kind::Path(vec![PathCmd::Move(0.0, 0.0), PathCmd::Line(1.0, 0.0), PathCmd::Line(1.0, 1.0), PathCmd::Close]), x, 200.0, 400.0, 120.0);
        o.fill = Some(Rgba(0, 0, 0, 255));
        o.alt = alt.into();
        o.equation = Some(EquationInfo { latex: latex.into(), display: true, mathml: mathml.into(), width_pt: 100.0 });
        o
    };
    a.layers[0].objects = vec![
        shape("half", 100.0, r"\frac{1}{2}", "<math display=\"block\"><mfrac><mn>1</mn><mn>2</mn></mfrac></math>", ""),
        shape("bad", 600.0, r"x", "<math><mi onclick=\"alert(1)\" mathvariant=\"italic\">x</mi><script>window.pwned=1</script><mi href=\"javascript:alert(1)\">y</mi></math>", "The letter x"),
        shape("nomath", 1100.0, r"\SI{5}{m}", "", ""),
    ];
    p.slides = vec![a];
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
