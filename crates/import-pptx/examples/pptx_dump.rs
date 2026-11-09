//! `cargo run -p brainbend-import-pptx --example pptx_dump -- file.pptx` prints a summary of the import.
fn main() {
    let path = std::env::args().nth(1).expect("path to a .pptx");
    let f = std::fs::File::open(&path).unwrap();
    let p = brainbend_import_pptx::import(f, "dump").unwrap();
    println!("slides {} scenes {:?} layouts {} assets {}", p.slides.len(), p.scenes.iter().map(|s| &s.name).collect::<Vec<_>>(), p.layouts.len(), p.assets.len());
    println!("theme {} {:?} fonts {}/{}", p.theme.name, p.theme.colors, p.theme.heading_font, p.theme.body_font);
    println!("master bg {:?} objects {}", p.master.background, p.master.objects.len());
    for l in &p.layouts { println!("  layout {} ({} objects, bg {:?}, show_master {})", l.name, l.objects.len(), l.background.is_some(), l.show_master); }
    for (i, s) in p.slides.iter().enumerate() {
        println!("{:>2}. {} [{}x{}] layout={:?} bg={:?} objs={} notes={} fade={}", i + 1, s.name, s.width, s.height, s.layout, s.background.is_some(), s.layers[0].objects.len(), s.notes.len(), s.fade_ms);
        for o in &s.layers[0].objects {
            println!("      {:?} {} ({:.0},{:.0} {:.0}x{:.0}) fill={:?} ph={:?} size={} text={:?}", std::mem::discriminant(&o.kind), o.name, o.x, o.y, o.w, o.h, o.fill.map(|c| (c.0, c.1, c.2)), o.placeholder, o.style.size, o.text.chars().take(40).collect::<String>());
        }
    }
    for n in &p.notes { println!("NOTE: {n}"); }
}
