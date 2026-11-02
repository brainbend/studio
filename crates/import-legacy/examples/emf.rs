// Draw one EMF file as a PNG: emf <in.emf> <out.png> [longest side in pixels]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let bytes = std::fs::read(&args[1]).unwrap();
    let side = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(800);
    let t = std::time::Instant::now();
    let png = brainbend_import_legacy::emf::render_png(&bytes, side).expect("a readable metafile");
    std::fs::write(&args[2], png).unwrap();
    eprintln!("drawn in {:?}", t.elapsed());
}
