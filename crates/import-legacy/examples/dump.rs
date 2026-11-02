fn main() {
    let p = std::env::args().nth(1).unwrap();
    let proj = brainbend_import_legacy::import(std::fs::File::open(&p).unwrap(), "x").unwrap();
    println!("slides {} assets {}", proj.slides.len(), proj.assets.len());
    for s in &proj.slides { println!("{} {}x{} layers {} objs {}", s.name, s.width, s.height, s.layers.len(), s.layers.iter().map(|l| l.objects.len()).sum::<usize>()); }
    println!("variables {} triggers {}", proj.variables.len(), proj.slides.iter().map(|s| s.triggers.len()).sum::<usize>());
    let with_states = proj.slides.iter().flat_map(|s| s.layers.iter()).flat_map(|l| l.objects.iter()).filter(|o| !o.states.is_empty()).count();
    println!("objects with states {with_states}");
    println!("slide background: {:?}", proj.slides[0].background);
    if let Some(out) = std::env::var_os("DUMP_JSON") { std::fs::write(out, proj.to_json().unwrap()).unwrap(); }
    { let c = proj.check(); let e = c.iter().filter(|i| i.severity == brainbend_doc::check::Severity::Error).count(); println!("check: {} issues ({} errors)", c.len(), e); }
    if std::env::var_os("CHECK_ALL").is_some() { for i in proj.check().iter().take(12) { println!("  {:?} s{} {}", i.severity, i.at.slide + 1, i.message); } }
    let pkg = proj.to_package().unwrap();
    let json = proj.to_json().unwrap().len();
    let back = brainbend_doc::Project::from_bytes(&pkg).unwrap();
    println!("package {} bytes (legacy JSON would be {} bytes); reopened: {} slides, {} assets, bytes equal: {}", pkg.len(), json, back.slides.len(), back.assets.len(),
        back.assets.iter().all(|(k, a)| proj.assets[k].bytes == a.bytes));
    let mut n = proj.notes.clone(); n.sort(); n.dedup();
    for x in n.iter().take(15) { println!("NOTE {x}"); }
}
