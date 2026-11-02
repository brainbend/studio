// Usage: export <file.story> <out> [scorm12|scorm2004]
// Without the third argument, <out> is a folder; with it, <out> is the .zip path.
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let p = brainbend_import_legacy::import(std::fs::File::open(&a[1]).unwrap(), "Course").unwrap();
    let out = std::path::Path::new(&a[2]);
    match a.get(3).map(String::as_str) {
        Some("scorm12") => brainbend_publish_html::export_scorm(&p, brainbend_publish_html::Lms::Scorm12, out).unwrap(),
        Some("scorm2004") => brainbend_publish_html::export_scorm(&p, brainbend_publish_html::Lms::Scorm2004, out).unwrap(),
        _ => brainbend_publish_html::export(&p, out).unwrap(),
    }
    println!("exported to {}", a[2]);
}
