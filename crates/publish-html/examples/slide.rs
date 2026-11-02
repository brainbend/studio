// Publish a single slide of a .story file for a visual check: slide <file.story> <index|list> <out_dir>
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mut p = brainbend_import_legacy::import(std::fs::File::open(&a[1]).unwrap(), "Slide").unwrap();
    if a[2] == "list" {
        for (i, s) in p.slides.iter().enumerate() {
            let objs: Vec<_> = s.layers.iter().flat_map(|l| l.objects.iter()).collect();
            println!("{i}: {} ({} objects, {} rotated, {} cropped, {} paths)", s.name, objs.len(), objs.iter().filter(|o| o.rotation != 0.0).count(), objs.iter().filter(|o| o.crop.is_some()).count(), objs.iter().filter(|o| matches!(o.kind, brainbend_doc::Kind::Path(_))).count());
        }
        return;
    }
    let i: usize = a[2].parse().unwrap();
    let keep = p.slides.remove(i);
    p.slides = vec![keep];
    brainbend_publish_html::export(&p, std::path::Path::new(&a[3])).unwrap();
}
