// One object per entrance effect, each leaving with the same effect, for the player tests: effects <out_dir>
use brainbend_doc::*;
fn main() {
    let out = std::env::args().nth(1).unwrap();
    let mut p = Project::new("Effects");
    let mut a = Slide::new("a", "A", 1920.0, 1080.0);
    let kinds = [
        ("spinq", EffectKind::SpinQuarter), ("spinh", EffectKind::SpinHalf), ("spinf", EffectKind::SpinFull), ("wheel", EffectKind::Wheel),
        ("svo", EffectKind::SplitVerticalOut), ("svi", EffectKind::SplitVerticalIn), ("sho", EffectKind::SplitHorizontalOut), ("shi", EffectKind::SplitHorizontalIn),
    ];
    for (i, (id, kind)) in kinds.into_iter().enumerate() {
        let mut o = Object::new(id, id, Kind::Text, 100.0, 60.0 + i as f32 * 80.0, 300.0, 60.0);
        o.text = id.into();
        o.entrance = Some(Entrance { kind, delay_ms: 0, duration_ms: 400 });
        o.exit = Some(Entrance { kind, delay_ms: 0, duration_ms: 400 });
        o.timing = Some(Timing { start_ms: 0, dur_ms: Some(1000) });
        a.layers[0].objects.push(o);
    }
    a.layers[0].duration_ms = Some(3000);
    p.slides = vec![a];
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
