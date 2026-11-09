//! Writes a sample handout to the path given, for opening in Word or LibreOffice: cargo run -p brainbend-export-docx --example docx_sample out.docx
use brainbend_doc::*;
use brainbend_publish_html::HandoutOptions;
fn main() {
    let out = std::env::args().nth(1).expect("a path to write");
    let mut p = Project::new("Sample lesson");
    let mut s = Slide::new("s1", "Welcome", 1920.0, 1080.0);
    s.notes = "Greet everyone.".into();
    let mut t = Object::new("t", "Title", Kind::Text, 100.0, 50.0, 800.0, 100.0);
    t.text = "Stay safe".into();
    s.layers[0].objects = vec![t];
    p.slides.push(s);
    let bytes = brainbend_export_docx::export(&p, &[0], &HandoutOptions { slide_notes: true, note_lines: true, ..Default::default() }, &Default::default()).unwrap();
    std::fs::write(out, bytes).unwrap();
}
