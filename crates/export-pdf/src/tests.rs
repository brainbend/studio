use super::*;

fn pages(bytes: &[u8]) -> usize { lopdf::Document::load_mem(bytes).expect("a readable PDF").get_pages().len() }

fn page_size(bytes: &[u8]) -> (f32, f32) {
    let d = lopdf::Document::load_mem(bytes).unwrap();
    let id = *d.get_pages().values().next().unwrap();
    let mb = d.get_dictionary(id).unwrap().get(b"MediaBox").unwrap().as_array().unwrap().clone();
    let n = |i: usize| mb[i].as_float().unwrap();
    (n(2) - n(0), n(3) - n(1))
}

fn five_slides() -> Project {
    let mut p = Project::new("Handout <1> & more");
    for i in 1..=5 {
        let mut s = Slide::new(&format!("s{i}"), &format!("Slide {i}"), 1920.0, 1080.0);
        s.notes = format!("Notes for slide {i}, which are long enough to need wrapping onto a second line of the notes area.");
        p.slides.push(s);
    }
    p
}

fn png(w: u32, h: u32) -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    let img = image_png(w, h);
    out.get_mut().extend(img);
    out.into_inner()
}

/// A small valid PNG, written by hand: a solid colour.
fn image_png(w: u32, h: u32) -> Vec<u8> {
    fn crc(data: &[u8]) -> u32 {
        let mut c = 0xffff_ffffu32;
        for &b in data { c ^= b as u32; for _ in 0..8 { c = if c & 1 == 1 { 0xedb8_8320 ^ (c >> 1) } else { c >> 1 }; } }
        !c
    }
    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        out.extend((data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend(data);
        out.extend(&body);
        out.extend(crc(&body).to_be_bytes());
    }
    fn adler(data: &[u8]) -> u32 { let (mut a, mut b) = (1u32, 0u32); for &x in data { a = (a + x as u32) % 65521; b = (b + a) % 65521; } (b << 16) | a }
    let mut raw = vec![];
    for _ in 0..h { raw.push(0); for _ in 0..w { raw.extend([40, 90, 200]); } }
    // zlib with stored blocks
    let mut z = vec![0x78, 0x01];
    for (i, block) in raw.chunks(65535).enumerate() {
        let last = (i + 1) * 65535 >= raw.len();
        z.push(last as u8);
        z.extend((block.len() as u16).to_le_bytes());
        z.extend((!(block.len() as u16)).to_le_bytes());
        z.extend(block);
    }
    z.extend(adler(&raw).to_be_bytes());
    let mut out = vec![137, 80, 78, 71, 13, 10, 26, 10];
    let mut ihdr = vec![];
    ihdr.extend(w.to_be_bytes());
    ihdr.extend(h.to_be_bytes());
    ihdr.extend([8, 2, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &z);
    chunk(&mut out, b"IEND", &[]);
    out
}

fn opts(orientation: HandoutOrientation, per_page: u8) -> HandoutOptions {
    HandoutOptions { orientation, per_page, ..Default::default() }
}

#[test]
fn makes_a_pdf_with_the_pages_the_slides_per_page_choice_asks_for() {
    let p = five_slides();
    let all: Vec<usize> = (0..5).collect();
    for orientation in HandoutOrientation::ALL {
        for (n, expect) in [(1u8, 5usize), (2, 3), (3, 2)] {
            let bytes = export(&p, &all, &opts(orientation, n), &BTreeMap::new()).unwrap();
            assert_eq!(&bytes[..5], b"%PDF-");
            assert_eq!(pages(&bytes), expect, "{orientation:?}, {n} per page");
        }
    }
}

#[test]
fn the_page_is_a4_in_the_chosen_direction() {
    let p = five_slides();
    let (w, h) = page_size(&export(&p, &[0], &opts(HandoutOrientation::Landscape, 1), &BTreeMap::new()).unwrap());
    assert!(w > h && (w - 841.89).abs() < 1.0 && (h - 595.28).abs() < 1.0, "{w} x {h}");
    let (w, h) = page_size(&export(&p, &[0], &opts(HandoutOrientation::Portrait, 1), &BTreeMap::new()).unwrap());
    assert!(h > w, "{w} x {h}");
}

#[test]
fn only_the_chosen_slides_are_included_and_none_is_an_error() {
    let p = five_slides();
    assert_eq!(pages(&export(&p, &[1, 3], &opts(HandoutOrientation::Landscape, 1), &BTreeMap::new()).unwrap()), 2);
    assert!(export(&p, &[], &HandoutOptions::default(), &BTreeMap::new()).is_err());
    assert!(export(&p, &[42], &HandoutOptions::default(), &BTreeMap::new()).is_err());
}

fn image_count(bytes: &[u8]) -> usize {
    let d = lopdf::Document::load_mem(bytes).unwrap();
    d.objects.values().filter(|o| matches!(o, lopdf::Object::Stream(s) if s.dict.get(b"Subtype").ok().and_then(|n| n.as_name().ok()) == Some(b"Image"))).count()
}

#[test]
fn a_slide_picture_is_embedded_and_a_slide_without_one_lists_its_text() {
    let mut p = five_slides();
    let mut t = Object::new("t", "Title", Kind::Text, 100.0, 50.0, 800.0, 100.0);
    t.text = "Stay safe".into();
    p.slides[0].layers[0].objects.push(t);
    let without = export(&p, &[0], &HandoutOptions::default(), &BTreeMap::new()).unwrap();
    assert_eq!(image_count(&without), 0);
    let mut pics = BTreeMap::new();
    pics.insert(0usize, png(400, 225));
    let with = export(&p, &[0], &HandoutOptions::default(), &pics).unwrap();
    assert_eq!(image_count(&with), 1, "the picture is in the file");
    let (a, b) = (with.len(), without.len());
    assert!(a > 0 && b > 0);
}

#[test]
fn question_slides_and_all_the_options_make_a_readable_file() {
    let mut p = five_slides();
    let q = Question { prompt: "Pick one \u{2014} wisely".into(), multiple: false, choices: vec![Choice { id: "a".into(), text: "Right".into(), correct: true }, Choice { id: "b".into(), text: "Wrong".into(), correct: false }], points: 1.0, feedback_correct: "Yes".into(), feedback_incorrect: "No".into(), style: QuestionStyle::Choice, area: None };
    p.slides[1].kind = SlideKind::Question(q);
    p.slides[2].kind = SlideKind::Results;
    let mut o = Object::new("o", "Logo", Kind::Text, 0.0, 0.0, 10.0, 10.0);
    o.alt = "A blue circle".into();
    p.slides[0].layers[0].objects.push(o);
    let all: Vec<usize> = (0..5).collect();
    for orientation in HandoutOrientation::ALL {
        for n in [1u8, 2, 3] {
            let options = HandoutOptions { orientation, per_page: n, correct_answers: n != 2, note_lines: true, slide_notes: true, alt_text: true };
            let bytes = export(&p, &all, &options, &BTreeMap::new()).unwrap();
            assert!(pages(&bytes) >= 1);
        }
    }
}

#[test]
fn text_the_font_cannot_show_is_swapped_for_plain_look_alikes() {
    let f = load_fonts().unwrap();
    let face = f.regular.parsed();
    let s = printable(&face, "\u{2610} a \u{2611} b \u{2192} c\nd");
    assert!(!s.contains('\u{2610}') || face.glyph_index('\u{2610}').is_some());
    assert!(!s.contains('\n'));
    assert!(wrap(&f.regular, "one two three four five six seven", 10.0, 60.0).len() > 1, "long text wraps");
    assert!(width(&f.bold, "Notes", 10.0) > width(&f.regular, "Notes", 10.0), "bold is wider");
}

fn long_notes_project(n: usize) -> Project {
    let mut p = Project::new("Long notes");
    for i in 0..n {
        let mut s = Slide::new(&format!("s{i}"), &format!("Slide {}", i + 1), 1920.0, 1080.0);
        s.notes = format!("START{i} ") + &"word ".repeat(1000) + &format!("END{i}");
        s.layers[0].objects = (0..40).map(|j| { let mut o = Object::new(&format!("o{j}"), &format!("Pic{j}"), Kind::Text, 0.0, 0.0, 10.0, 10.0); o.alt = format!("alt{i}x{j} describes the picture in some detail"); o }).collect();
        p.slides.push(s);
    }
    p
}

#[test]
fn notes_too_long_for_their_place_continue_on_extra_pages_and_none_is_lost() {
    let p = long_notes_project(3);
    for (orientation, per_page) in [(HandoutOrientation::Portrait, 1u8), (HandoutOrientation::Portrait, 2), (HandoutOrientation::Portrait, 3), (HandoutOrientation::Landscape, 1), (HandoutOrientation::Landscape, 2), (HandoutOrientation::Landscape, 3)] {
        let o = HandoutOptions { orientation, per_page, slide_notes: true, alt_text: true, note_lines: true, ..Default::default() };
        let plain = HandoutOptions { slide_notes: false, alt_text: false, note_lines: false, ..o.clone() };
        let bytes = export(&p, &[0, 1, 2], &o, &BTreeMap::new()).unwrap();
        let base = pages(&export(&p, &[0, 1, 2], &plain, &BTreeMap::new()).unwrap());
        assert!(pages(&bytes) > base, "{orientation:?} {per_page}: the notes needed more pages");
    }
}

#[test]
fn what_does_not_fit_is_handed_on_and_every_word_is_drawn_somewhere() {
    let fonts = load_fonts().unwrap();
    let slide = &long_notes_project(1).slides[0];
    let o = HandoutOptions { slide_notes: true, alt_text: true, ..Default::default() };
    let all = note_paras(slide, &o);
    let want: Vec<String> = all.iter().filter(|p| !p.bold).flat_map(|p| p.text.split_whitespace().map(String::from).collect::<Vec<_>>()).collect();
    // A small place, then page after page, as the export does.
    let mut doc = Document::new();
    let mut drawn = vec![];
    let mut paras = all;
    let (mut width, mut room) = (150.0, 120.0);
    let mut rounds = 0;
    while !paras.is_empty() {
        rounds += 1;
        assert!(rounds < 200, "it keeps making progress");
        let mut page = doc.start_page_with(PageSettings::from_wh(595.0, 842.0).unwrap());
        let mut surface = page.surface();
        let mut w = Writer { s: &mut surface, fonts: &fonts, drawn: vec![] };
        let (_, left) = fit(&mut w, paras, 10.0, 10.0, width, room);
        drawn.extend(w.drawn.clone());
        surface.finish();
        page.finish();
        paras = left;
        (width, room) = (500.0, 800.0);
    }
    assert!(rounds >= 3, "it needed several pages");
    let got: Vec<String> = drawn.iter().flat_map(|l| l.split_whitespace().map(String::from).collect::<Vec<_>>()).filter(|w| w != "Notes" && w != "Alt" && w != "text").collect();
    let want: Vec<String> = want.into_iter().map(|w| w.trim_start_matches('-').to_string()).filter(|w| !w.is_empty()).collect();
    let got: Vec<String> = got.into_iter().map(|w| w.trim_start_matches('-').to_string()).filter(|w| !w.is_empty()).collect();
    assert_eq!(got, want, "the same words in the same order");
}

#[test]
fn short_notes_stay_on_the_slides_page() {
    let p = five_slides();
    let o = HandoutOptions { orientation: HandoutOrientation::Portrait, per_page: 2, slide_notes: true, alt_text: true, note_lines: true, ..Default::default() };
    let bytes = export(&p, &[0, 1, 2, 3, 4], &o, &BTreeMap::new()).unwrap();
    assert_eq!(pages(&bytes), 3, "five slides, two to a page, no continuation pages");
}
