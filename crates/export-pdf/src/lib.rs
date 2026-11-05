//! A PDF handout written directly, without the browser's print dialog.
//!
//! The pages follow the same choices as the web handout (orientation, 1, 2 or 3 slides per page, correct answers, space
//! for notes, slide notes, alt text). Slides are pictures the caller supplies (the app draws them); captions, notes, alt
//! text and question slides are real text in Open Sans. A slide with no picture (a question or results slide) is listed
//! as text inside a frame of the same shape.
use std::collections::BTreeMap;
use std::sync::Arc;
use brainbend_doc::handout_text::{question_lines, slide_text_lines, Line};
use brainbend_doc::*;
use brainbend_publish_html::{HandoutOptions, HandoutOrientation};
use krilla::color::rgb;
use krilla::geom::{PathBuilder, Point, Rect, Size, Transform};
use krilla::image::Image;
use krilla::num::NormalizedF32;
use krilla::page::PageSettings;
use krilla::paint::{Fill, FillRule, Stroke};
use krilla::surface::Surface;
use krilla::text::{Font, TextDirection};
use krilla::Document;

/// Points in a millimetre.
const PT: f32 = 72.0 / 25.4;
const A4: (f32, f32) = (210.0 * PT, 297.0 * PT);
const MARGIN: f32 = 12.0 * PT;
const GAP: f32 = 6.0 * PT;
const CAPTION: f32 = 7.0 * PT;
/// What a slide's notes area needs under it when slides are stacked.
const NOTES_BELOW: f32 = 28.0 * PT;
/// The space between a slide and the notes beside it.
const BESIDE_GAP: f32 = 8.0 * PT;
const RULE_STEP: f32 = 7.5 * PT;

/// One face of Open Sans, for drawing text and measuring it.
struct Face {
    font: Font,
    bytes: Arc<Vec<u8>>,
    wght: Option<f32>,
}

impl Face {
    fn parsed(&self) -> ttf_parser::Face<'_> {
        let mut f = ttf_parser::Face::parse(&self.bytes, 0).expect("the bundled font parses");
        if let Some(w) = self.wght { f.set_variation(ttf_parser::Tag::from_bytes(b"wght"), w); }
        f
    }
}

struct Fonts { regular: Face, bold: Face, italic: Face }

fn load_face(family: &brainbend_fonts::Family, bold: bool, italic: bool) -> Result<Face, String> {
    let face = family.face_for(bold, italic).ok_or("Open Sans has no matching face")?;
    let (bytes, index) = face.data().ok_or("Open Sans could not be read")?;
    let bytes = Arc::new(bytes);
    let (font, wght) = if face.weight_range.is_some() {
        let w = if bold { 700.0 } else { 400.0 };
        (Font::new_variable(bytes.clone().into(), index, &[(krilla::text::Tag::new(b"wght"), w)]), Some(w))
    } else {
        (Font::new(bytes.clone().into(), index), None)
    };
    Ok(Face { font: font.ok_or("Open Sans could not be used in a PDF")?, bytes, wght })
}

fn load_fonts() -> Result<Fonts, String> {
    let family = brainbend_fonts::lookup("Open Sans").ok_or("the Open Sans font is missing")?;
    Ok(Fonts { regular: load_face(&family, false, false)?, bold: load_face(&family, true, false)?, italic: load_face(&family, false, true)? })
}

/// Text the font cannot show is swapped for plain look-alikes so no box or gap appears.
fn printable(face: &ttf_parser::Face, s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        if ch == '\n' || ch == '\r' { out.push(' '); continue; }
        if face.glyph_index(ch).is_some() { out.push(ch); continue; }
        match ch {
            '\u{2610}' => out.push_str("[ ]"),
            '\u{2611}' | '\u{2612}' => out.push_str("[x]"),
            '\u{2192}' => out.push_str("->"),
            '\u{2022}' => out.push('-'),
            '\u{2018}' | '\u{2019}' => out.push('\''),
            '\u{201c}' | '\u{201d}' => out.push('"'),
            c if c.is_whitespace() => out.push(' '),
            _ => out.push('?'),
        }
    }
    out
}

fn width(face: &Face, s: &str, size: f32) -> f32 {
    let f = face.parsed();
    let upem = f.units_per_em() as f32;
    s.chars().map(|c| f.glyph_index(c).and_then(|g| f.glyph_hor_advance(g)).unwrap_or((upem * 0.5) as u16) as f32).sum::<f32>() * size / upem
}

/// `text` broken into lines no wider than `max`.
fn wrap(face: &Face, text: &str, size: f32, max: f32) -> Vec<String> {
    let clean = printable(&face.parsed(), text);
    let mut lines = vec![];
    let mut line = String::new();
    for word in clean.split(' ').filter(|w| !w.is_empty()) {
        let candidate = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
        if width(face, &candidate, size) <= max || line.is_empty() {
            line = candidate;
        } else {
            lines.push(std::mem::take(&mut line));
            line = word.to_string();
        }
    }
    if !line.is_empty() { lines.push(line); }
    lines
}

fn fill(r: u8, g: u8, b: u8) -> Fill { Fill { paint: rgb::Color::new(r, g, b).into(), opacity: NormalizedF32::ONE, rule: FillRule::default() } }
fn stroke(r: u8, g: u8, b: u8, w: f32) -> Stroke { Stroke { paint: rgb::Color::new(r, g, b).into(), width: w, ..Default::default() } }

fn rect_path(x: f32, y: f32, w: f32, h: f32) -> Option<krilla::geom::Path> {
    let mut pb = PathBuilder::new();
    pb.push_rect(Rect::from_xywh(x, y, w, h)?);
    pb.finish()
}

/// Draws text, a line at a time, down from `y`. Returns where the next line would start.
struct Writer<'a, 'b> { s: &'a mut Surface<'b>, fonts: &'a Fonts, /// Every line of text drawn, for the tests.
    drawn: Vec<String> }

impl Writer<'_, '_> {
    fn text(&mut self, face: fn(&Fonts) -> &Face, x: f32, baseline: f32, size: f32, text: &str, color: (u8, u8, u8)) {
        let f = face(self.fonts);
        let clean = printable(&f.parsed(), text);
        self.s.set_fill(Some(fill(color.0, color.1, color.2)));
        self.s.draw_text(Point::from_xy(x, baseline), f.font.clone(), size, &clean, false, TextDirection::Auto);
        #[cfg(test)]
        self.drawn.push(clean);
    }

    /// A wrapped paragraph; returns the y after it. Stops (and returns) when it would pass `bottom`.
    fn paragraph(&mut self, face: fn(&Fonts) -> &Face, x: f32, y: f32, w: f32, bottom: f32, size: f32, text: &str, color: (u8, u8, u8)) -> f32 {
        let leading = size * 1.35;
        let mut y = y;
        for l in wrap(face(self.fonts), text, size, w) {
            if y + leading > bottom { break; }
            self.text(face, x, y + size, size, &l, color);
            y += leading;
        }
        y
    }

    fn rule(&mut self, x: f32, y: f32, w: f32, gray: u8) {
        if let Some(p) = rect_path(x, y, w, 0.6) {
            self.s.set_stroke(None);
            self.s.set_fill(Some(fill(gray, gray, gray)));
            self.s.draw_path(&p);
        }
    }
}

fn regular(f: &Fonts) -> &Face { &f.regular }
fn bold(f: &Fonts) -> &Face { &f.bold }
fn italic(f: &Fonts) -> &Face { &f.italic }

const INK: (u8, u8, u8) = (34, 34, 34);
const GREY: (u8, u8, u8) = (102, 102, 102);

/// Question and results slides, as text inside a frame of the slide's shape.
fn text_slide(w: &mut Writer, slide: &Slide, project: &Project, answers: bool, x: f32, y: f32, fw: f32, fh: f32) {
    let lines: Vec<Line> = match &slide.kind {
        SlideKind::Question(q) => question_lines(q, project, answers),
        SlideKind::Normal => slide_text_lines(slide),
        SlideKind::Results => vec![Line { text: "Results".into(), bold: true, italic: false, indent: 0 }, Line { text: "The player shows the learner's score, pass or fail, and a Retry button here.".into(), bold: false, italic: true, indent: 0 }],
    };
    let pad = (fw * 0.04).max(6.0);
    let (iw, ibottom) = (fw - 2.0 * pad, y + fh - pad);
    // The biggest text that fits the frame.
    let measure = |w: &mut Writer, size: f32| -> f32 {
        let mut yy = 0.0;
        for l in &lines {
            let face = if l.bold { bold } else if l.italic { italic } else { regular };
            yy += wrap(face(w.fonts), &l.text, size, iw - l.indent as f32 * size * 1.5).len() as f32 * size * 1.35 + size * 0.3;
        }
        yy
    };
    let mut size = 14.0;
    while size > 5.0 && measure(w, size) > fh - 2.0 * pad { size -= 0.5; }
    if let Some(clip) = rect_path(x, y, fw, fh) { w.s.push_clip_path(&clip, &FillRule::default()); }
    let mut yy = y + pad;
    for l in &lines {
        let face: fn(&Fonts) -> &Face = if l.bold { bold } else if l.italic { italic } else { regular };
        let indent = l.indent as f32 * size * 1.5;
        yy = w.paragraph(face, x + pad + indent, yy, iw - indent, ibottom, size, &l.text, if l.italic { GREY } else { INK }) + size * 0.3;
    }
    w.s.pop();
}

/// A paragraph of the notes area: a label ("Notes", "Alt text") or text, with the gap after it.
#[derive(Clone, PartialEq, Debug)]
struct NotePara { text: String, bold: bool, indent: f32, after: f32 }

/// The notes area's content for a slide, as the options ask.
fn note_paras(slide: &Slide, options: &HandoutOptions) -> Vec<NotePara> {
    let mut out = vec![];
    if options.slide_notes && !slide.notes.trim().is_empty() {
        out.push(NotePara { text: "Notes".into(), bold: true, indent: 0.0, after: 0.0 });
        out.push(NotePara { text: slide.notes.trim().to_string(), bold: false, indent: 0.0, after: 4.0 });
    }
    if options.alt_text {
        let objects = slide.layers.first().map(|l| l.objects.as_slice()).unwrap_or(&[]);
        let items: Vec<String> = objects.iter().filter(|o| !o.decorative && !o.alt.trim().is_empty()).map(|o| format!("{}: {}", o.name, o.alt.trim())).collect();
        if !items.is_empty() {
            out.push(NotePara { text: "Alt text".into(), bold: true, indent: 0.0, after: 0.0 });
            let last = items.len() - 1;
            for (n, it) in items.into_iter().enumerate() { out.push(NotePara { text: format!("- {it}"), bold: false, indent: 8.0, after: if n == last { 4.0 } else { 0.0 } }); }
        }
    }
    out
}

/// Draw `paras` down from `y` inside `bottom`. Returns where the next would start, and what did not fit: whole paragraphs,
/// and the rest of one that was cut part way (a label is never left alone at the bottom).
fn fit(w: &mut Writer, paras: Vec<NotePara>, x: f32, y: f32, width: f32, bottom: f32) -> (f32, Vec<NotePara>) {
    let (size, leading) = (10.0, 10.0 * 1.35);
    let mut y = y;
    let mut i = 0;
    while i < paras.len() {
        let para = &paras[i];
        if para.bold {
            // The label needs room for itself and a line of what it labels.
            if y + 14.0 + leading > bottom { break; }
            w.text(bold, x, y + 10.0, size, &para.text, INK);
            y += 14.0;
        } else {
            let lines = wrap(&w.fonts.regular, &para.text, size, width - para.indent);
            let room = (((bottom - y) / leading).floor().max(0.0) as usize).min(lines.len());
            for l in &lines[..room] { w.text(regular, x + para.indent, y + size, size, l, INK); y += leading; }
            if room < lines.len() {
                let mut rest = paras[i + 1..].to_vec();
                rest.insert(0, NotePara { text: lines[room..].join(" "), ..para.clone() });
                return (y, rest);
            }
            y += para.after;
        }
        i += 1;
    }
    (y, paras[i..].to_vec())
}

/// Make the PDF for `slides` (indices into `project.slides`). `pictures` holds a PNG of a slide, by its index, for the
/// slides the caller could draw. Returns the file's bytes.
pub fn export(project: &Project, slides: &[usize], options: &HandoutOptions, pictures: &BTreeMap<usize, Vec<u8>>) -> Result<Vec<u8>, String> {
    let flat = project.flattened();
    let p = &flat;
    let slides: Vec<usize> = slides.iter().copied().filter(|&i| i < p.slides.len()).collect();
    if slides.is_empty() { return Err("there are no slides to put in the document".into()); }
    let fonts = load_fonts()?;
    let per_page = options.per_page.clamp(1, 3) as usize;
    let portrait = options.orientation == HandoutOrientation::Portrait;
    let (page_w, page_h) = if portrait { A4 } else { (A4.1, A4.0) };
    let (area_w, area_h) = (page_w - 2.0 * MARGIN, page_h - 2.0 * MARGIN);
    let has_notes_area = options.note_lines || options.slide_notes || options.alt_text;
    let (cols, rows) = if portrait { (1, per_page) } else { (per_page, 1) };
    let beside = if portrait { per_page == 3 } else { per_page == 1 && has_notes_area };
    let cell_w = (area_w - GAP * (cols as f32 - 1.0)) / cols as f32;
    let cell_h = (area_h - GAP * (rows as f32 - 1.0)) / rows as f32;
    let alone = !beside && !has_notes_area && !portrait;

    let mut doc = Document::new();
    doc.set_metadata(krilla::metadata::Metadata::new().title(p.name.clone()));
    for group in slides.chunks(per_page) {
        let mut overflow: Vec<(usize, Vec<NotePara>)> = vec![];
        let mut page = doc.start_page_with(PageSettings::from_wh(page_w, page_h).ok_or("bad page size")?);
        let mut surface = page.surface();
        for (k, &i) in group.iter().enumerate() {
            let slide = &p.slides[i];
            let (col, row) = if portrait { (0, k) } else { (k, 0) };
            let (cx, cy) = (MARGIN + col as f32 * (cell_w + GAP), MARGIN + row as f32 * (cell_h + GAP));
            let aspect = slide.height / slide.width;
            // Room under the slide for its notes: a third of the cell at least, so notes, alt text and lines have somewhere to go.
            let below = if has_notes_area { NOTES_BELOW.max(cell_h * 0.34) } else { 0.0 };
            let fw = if beside { (cell_w * 0.58).min((cell_h - CAPTION) / aspect) } else { cell_w.min((cell_h - CAPTION - below) / aspect) };
            let fh = fw * aspect;
            let fx = if alone { cx + (cell_w - fw) / 2.0 } else { cx };
            let fy = cy + CAPTION;
            let mut w = Writer { s: &mut surface, fonts: &fonts, drawn: vec![] };
            w.text(regular, fx + 1.0, cy + CAPTION - 4.0, 9.0, &format!("{}. {}", i + 1, slide.name), GREY);
            // The slide: its picture if it has one, else its text.
            let picture = pictures.get(&i).and_then(|b| Image::from_png(b.clone().into(), true).ok());
            match picture {
                Some(img) => {
                    let (iw, ih) = img.size();
                    if iw > 0 && ih > 0 && fw > 0.0 {
                        w.s.push_transform(&Transform::from_translate(fx, fy));
                        if let Some(size) = Size::from_wh(fw, fw * ih as f32 / iw as f32) { w.s.draw_image(img, size); }
                        w.s.pop();
                    }
                }
                None => {
                    if let Some(frame) = rect_path(fx, fy, fw, fh) { w.s.set_stroke(None); w.s.set_fill(Some(fill(255, 255, 255))); w.s.draw_path(&frame); }
                    text_slide(&mut w, slide, p, options.correct_answers, fx, fy, fw, fh);
                }
            }
            if let Some(frame) = rect_path(fx, fy, fw, fh) { w.s.set_fill(None); w.s.set_stroke(Some(stroke(136, 136, 136, 0.75))); w.s.draw_path(&frame); w.s.set_stroke(None); }
            // The notes area: beside the slide, or under it. What does not fit goes on pages after this one.
            if has_notes_area {
                let (nx, ny, nw, nh) = if beside { (fx + fw + BESIDE_GAP, fy, cell_w - fw - BESIDE_GAP, fh) } else { (fx, fy + fh + GAP * 0.7, fw, cy + cell_h - (fy + fh + GAP * 0.7)) };
                let bottom = ny + nh;
                let (y, left) = fit(&mut w, note_paras(slide, options), nx, ny, nw, bottom);
                if !left.is_empty() {
                    overflow.push((i, left));
                } else if options.note_lines {
                    let mut ly = y + RULE_STEP;
                    while ly <= bottom { w.rule(nx, ly, nw, 170); ly += RULE_STEP; }
                }
            }
        }
        surface.finish();
        page.finish();
        // Notes that did not fit beside or under their slide continue on pages of their own, in the full width of the page.
        for (i, mut paras) in overflow {
            let title = format!("{}. {} (notes continued)", i + 1, p.slides[i].name);
            while !paras.is_empty() {
                let mut page = doc.start_page_with(PageSettings::from_wh(page_w, page_h).ok_or("bad page size")?);
                let mut surface = page.surface();
                let mut w = Writer { s: &mut surface, fonts: &fonts, drawn: vec![] };
                w.text(bold, MARGIN, MARGIN + 11.0, 11.0, &title, INK);
                let before = paras.len();
                let (_, left) = fit(&mut w, paras.clone(), MARGIN, MARGIN + 24.0, area_w, page_h - MARGIN);
                surface.finish();
                page.finish();
                // A page that took nothing would repeat for ever.
                if left.len() == before && left == paras { break; }
                paras = left;
            }
        }
    }
    doc.finish().map_err(|e| format!("{e:?}"))
}

#[cfg(test)]
mod tests;
