use super::*;
use std::io::Read;

fn unzip(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("a zip");
    let mut out = BTreeMap::new();
    for i in 0..z.len() {
        let mut f = z.by_index(i).unwrap();
        let name = f.name().unwrap().to_string();
        let mut b = vec![];
        f.read_to_end(&mut b).unwrap();
        out.insert(name, b);
    }
    out
}

fn document(bytes: &[u8]) -> String {
    let files = unzip(bytes);
    let xml = String::from_utf8(files["word/document.xml"].clone()).unwrap();
    roxmltree::Document::parse(&xml).expect("document.xml is well-formed");
    xml
}

fn lesson() -> Project {
    let mut p = Project::new("Safety <101> & more");
    let mut s = Slide::new("s1", "Welcome", 1920.0, 1080.0);
    s.notes = "Say <hello> first".into();
    let mut title = Object::new("t", "Title", Kind::Text, 100.0, 50.0, 800.0, 100.0);
    title.text = "Stay safe".into();
    let mut body = Object::new("b", "Body", Kind::Text, 100.0, 200.0, 800.0, 300.0);
    body.rich = Some(vec![Paragraph { indent: 0, runs: vec![Run { text: "Wear a ".into(), ..Default::default() }, Run { text: "helmet".into(), bold: Some(true), ..Default::default() }] }]);
    body.alt = "A person in a helmet".into();
    s.layers[0].objects = vec![body, title];
    p.slides = vec![s];
    let q = Question { prompt: "Pick one".into(), multiple: false, choices: vec![Choice { id: "a".into(), text: "Right".into(), correct: true }, Choice { id: "b".into(), text: "Wrong".into(), correct: false }], points: 1.0, feedback_correct: String::new(), feedback_incorrect: String::new(), style: QuestionStyle::Choice, area: None };
    let mut qs = Slide::new("s2", "Quiz", 1920.0, 1080.0);
    qs.kind = SlideKind::Question(q);
    p.slides.push(qs);
    p
}

fn png(w: u32, h: u32) -> Vec<u8> {
    let mut b = vec![137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, b'I', b'H', b'D', b'R'];
    b.extend(w.to_be_bytes());
    b.extend(h.to_be_bytes());
    b.extend([8, 6, 0, 0, 0, 0, 0, 0, 0]);
    b
}

#[test]
fn makes_a_word_file_with_the_slides_text_in_reading_order() {
    let p = lesson();
    let bytes = export(&p, &[0, 1], &HandoutOptions::default(), &BTreeMap::new()).unwrap();
    assert_eq!(&bytes[..2], b"PK");
    let files = unzip(&bytes);
    for part in ["[Content_Types].xml", "_rels/.rels", "word/document.xml", "word/styles.xml", "word/_rels/document.xml.rels"] {
        assert!(files.contains_key(part), "{part}");
        roxmltree::Document::parse(std::str::from_utf8(&files[part]).unwrap()).unwrap_or_else(|e| panic!("{part}: {e}"));
    }
    let xml = document(&bytes);
    assert!(xml.contains("Safety &lt;101&gt; &amp; more"), "the title is escaped");
    assert!(xml.contains("1. Welcome") && xml.contains("2. Quiz"));
    assert!(xml.find("Stay safe").unwrap() < xml.find("Wear a ").unwrap(), "top to bottom, not the order they were added");
    assert!(xml.contains("<w:b/></w:rPr><w:t xml:space=\"preserve\">helmet"), "bold text stays bold");
}

#[test]
fn notes_alt_text_and_lines_appear_only_when_asked_for() {
    let p = lesson();
    let make = |o: HandoutOptions| document(&export(&p, &[0], &o, &BTreeMap::new()).unwrap());
    let off = make(HandoutOptions { slide_notes: false, alt_text: false, note_lines: false, ..Default::default() });
    assert!(!off.contains("Say &lt;hello&gt; first") && !off.contains("A person in a helmet") && !off.contains("w:pBdr"));
    assert!(make(HandoutOptions { slide_notes: true, ..Default::default() }).contains("Say &lt;hello&gt; first"));
    assert!(make(HandoutOptions { alt_text: true, ..Default::default() }).contains("Body: A person in a helmet"));
    assert_eq!(make(HandoutOptions { note_lines: true, ..Default::default() }).matches("<w:pBdr>").count(), 6);
}

#[test]
fn a_worksheet_hides_the_correct_answers() {
    let p = lesson();
    let make = |answers| document(&export(&p, &[1], &HandoutOptions { correct_answers: answers, ..Default::default() }, &BTreeMap::new()).unwrap());
    assert!(make(true).contains("\u{2611} Right") && make(true).contains("\u{2610} Wrong"));
    assert!(!make(false).contains('\u{2611}'));
}

#[test]
fn the_page_follows_the_orientation() {
    let p = lesson();
    let size = |o| document(&export(&p, &[0], &HandoutOptions { orientation: o, ..Default::default() }, &BTreeMap::new()).unwrap());
    assert!(size(HandoutOrientation::Landscape).contains("w:orient=\"landscape\""));
    let portrait = size(HandoutOrientation::Portrait);
    assert!(!portrait.contains("w:orient") && portrait.contains("w:w=\"11906\" w:h=\"16838\""), "{portrait}");
}

#[test]
fn only_the_chosen_slides_are_included() {
    let p = lesson();
    let xml = document(&export(&p, &[1], &HandoutOptions::default(), &BTreeMap::new()).unwrap());
    assert!(xml.contains("2. Quiz") && !xml.contains("1. Welcome"));
    assert!(export(&p, &[], &HandoutOptions::default(), &BTreeMap::new()).is_err(), "nothing to put in the document");
    assert!(export(&p, &[9], &HandoutOptions::default(), &BTreeMap::new()).is_err(), "a slide that is not there is skipped");
}

#[test]
fn a_slide_picture_goes_in_the_file_and_fits_the_page() {
    let p = lesson();
    let mut pics = BTreeMap::new();
    pics.insert(0usize, png(1920, 1080));
    pics.insert(1usize, png(1920, 1080)); // a question slide is listed as text, so this one is not used
    let bytes = export(&p, &[0, 1], &HandoutOptions::default(), &pics).unwrap();
    let files = unzip(&bytes);
    assert!(files.contains_key("word/media/image1.png") && !files.contains_key("word/media/image2.png"));
    assert!(String::from_utf8_lossy(&files["word/_rels/document.xml.rels"]).contains("media/image1.png"));
    let xml = document(&bytes);
    assert!(xml.contains("r:embed=\"rIdImg1\"") && xml.contains("Slide 1: Welcome"), "alt text for the picture");
    let doc = roxmltree::Document::parse(&xml).unwrap();
    let ext = doc.descendants().find(|n| n.tag_name().name() == "extent").unwrap();
    let (cx, cy): (i64, i64) = (ext.attribute("cx").unwrap().parse().unwrap(), ext.attribute("cy").unwrap().parse().unwrap());
    assert!((cx as f64 / cy as f64 - 16.0 / 9.0).abs() < 0.01, "keeps the slide's shape");
    let page_w_emu = twips(297.0 - 2.0 * MARGIN_MM) * EMU_PER_TWIP;
    assert!(cx <= page_w_emu && cy <= twips(210.0 - 2.0 * MARGIN_MM) * EMU_PER_TWIP, "fits the landscape page");
    assert_eq!(xml.matches("<w:pageBreakBefore/>").count(), 0, "the first slide follows the title, and the question is text that flows on");
    let mut two = lesson();
    two.slides.push(Slide::new("s3", "Third", 1920.0, 1080.0));
    pics.insert(2usize, png(1920, 1080));
    let xml = document(&export(&two, &[0, 2], &HandoutOptions::default(), &pics).unwrap());
    assert_eq!(xml.matches("<w:pageBreakBefore/>").count(), 1, "a slide with a picture starts a new page");
}

#[test]
fn png_size_reads_the_header_and_rejects_anything_else() {
    assert_eq!(png_size(&png(640, 480)), Some((640, 480)));
    assert_eq!(png_size(b"not a picture at all......"), None);
    assert_eq!(png_size(&png(0, 10)), None);
}
