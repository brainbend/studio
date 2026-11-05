//! A Word (.docx) handout: the same choices as the PDF handout (which slides, orientation, correct answers, space for
//! notes, slide notes, alt text), written as a real Word document so people can edit it.
//!
//! The browser print view the PDF uses cannot be reused here, so this writes the document's XML itself. A slide is
//! shown as a picture when the caller supplies one (the app draws them); otherwise, and for question slides, its text
//! is listed instead. "Slides per page" is a PDF layout choice and does not apply: each slide follows the one before.
use std::collections::BTreeMap;
use std::io::Write;
use brainbend_doc::*;
use brainbend_publish_html::{HandoutOptions, HandoutOrientation};

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Twips (a twentieth of a point) in a millimetre.
const TWIPS_PER_MM: f32 = 56.69;
/// EMUs (what Word sizes pictures in) in a twip.
const EMU_PER_TWIP: i64 = 635;
const MARGIN_MM: f32 = 15.0;
/// Page sizes (A4) in millimetres.
const A4: (f32, f32) = (210.0, 297.0);

fn twips(mm: f32) -> i64 { (mm * TWIPS_PER_MM).round() as i64 }

/// A4 in twips, exactly as Word writes it.
const A4_TWIPS: (i64, i64) = (11906, 16838);

/// The width and height of a PNG, read from its header.
pub fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 || bytes[..8] != [137, 80, 78, 71, 13, 10, 26, 10] { return None; }
    let u = |i: usize| u32::from_be_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
    Some((u(16), u(20))).filter(|&(w, h)| w > 0 && h > 0)
}

/// One paragraph being built.
#[derive(Default)]
struct Para {
    runs: String,
    style: Option<&'static str>,
    keep_next: bool,
    page_break_before: bool,
    /// A line to write on: a rule along the bottom.
    ruled: bool,
    indent: i64,
    spacing_after: Option<i64>,
}

impl Para {
    fn new() -> Self { Para::default() }
    fn styled(style: &'static str) -> Self { Para { style: Some(style), ..Default::default() } }
    fn text(mut self, t: &str, bold: bool, italic: bool) -> Self { self.push(t, bold, italic, false); self }
    fn push(&mut self, t: &str, bold: bool, italic: bool, underline: bool) {
        let mut pr = String::new();
        if bold { pr.push_str("<w:b/>"); }
        if italic { pr.push_str("<w:i/>"); }
        if underline { pr.push_str("<w:u w:val=\"single\"/>"); }
        let rpr = if pr.is_empty() { String::new() } else { format!("<w:rPr>{pr}</w:rPr>") };
        for (i, line) in t.split('\n').enumerate() {
            if i > 0 { self.runs.push_str("<w:r><w:br/></w:r>"); }
            let _ = write!(self.runs, "<w:r>{rpr}<w:t xml:space=\"preserve\">{}</w:t></w:r>", esc(line));
        }
    }
    fn xml(&self) -> String {
        let mut ppr = String::new();
        if let Some(s) = self.style { let _ = write!(ppr, "<w:pStyle w:val=\"{s}\"/>"); }
        if self.keep_next { ppr.push_str("<w:keepNext/>"); }
        if self.page_break_before { ppr.push_str("<w:pageBreakBefore/>"); }
        if self.ruled { ppr.push_str("<w:pBdr><w:bottom w:val=\"single\" w:sz=\"4\" w:space=\"1\" w:color=\"AAAAAA\"/></w:pBdr>"); }
        if let Some(a) = self.spacing_after { let _ = write!(ppr, "<w:spacing w:after=\"{a}\"/>"); }
        if self.indent > 0 { let _ = write!(ppr, "<w:ind w:left=\"{}\"/>", self.indent); }
        let ppr = if ppr.is_empty() { String::new() } else { format!("<w:pPr>{ppr}</w:pPr>") };
        format!("<w:p>{ppr}{}</w:p>", self.runs)
    }
}
use std::fmt::Write as _;

/// A picture to place in the document.
struct Picture { rid: String, w_emu: i64, h_emu: i64, name: String, descr: String, id: usize }

fn picture_para(p: &Picture, keep_next: bool) -> String {
    format!(
        "<w:p><w:pPr>{}</w:pPr><w:r><w:drawing><wp:inline distT=\"0\" distB=\"0\" distL=\"0\" distR=\"0\"><wp:extent cx=\"{w}\" cy=\"{h}\"/><wp:docPr id=\"{id}\" name=\"{name}\" descr=\"{descr}\"/>\
<a:graphic xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/picture\">\
<pic:pic xmlns:pic=\"http://schemas.openxmlformats.org/drawingml/2006/picture\"><pic:nvPicPr><pic:cNvPr id=\"{id}\" name=\"{name}\" descr=\"{descr}\"/><pic:cNvPicPr/></pic:nvPicPr>\
<pic:blipFill><a:blip r:embed=\"{rid}\"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill>\
<pic:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"{w}\" cy=\"{h}\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom><a:ln w=\"9525\"><a:solidFill><a:srgbClr val=\"888888\"/></a:solidFill></a:ln></pic:spPr></pic:pic>\
</a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p>",
        if keep_next { "<w:keepNext/>" } else { "" },
        w = p.w_emu, h = p.h_emu, id = p.id, name = esc(&p.name), descr = esc(&p.descr), rid = p.rid
    )
}

/// The text of a text-bearing object, as paragraphs of (text, bold, italic, underline) runs.
fn object_paragraphs(o: &Object) -> Vec<Para> {
    let mut out = vec![];
    if let Some(rich) = &o.rich {
        for p in rich {
            let mut para = Para::new();
            para.indent = p.indent as i64 * 360;
            para.spacing_after = Some(80);
            for r in &p.runs { para.push(&r.text, r.bold.unwrap_or(false), r.italic.unwrap_or(false), r.underline.unwrap_or(false)); }
            if !p.runs.iter().all(|r| r.text.trim().is_empty()) { out.push(para); }
        }
    } else {
        for line in o.text.lines().filter(|l| !l.trim().is_empty()) {
            let mut para = Para::new();
            para.spacing_after = Some(80);
            para.push(line, false, false, false);
            out.push(para);
        }
    }
    out
}

/// The slide's visible text objects, top to bottom then left to right.
fn text_objects(slide: &Slide) -> Vec<&Object> {
    let mut objects: Vec<&Object> = slide.layers.first().map(|l| l.objects.iter().collect()).unwrap_or_default();
    objects.retain(|o| !o.text.trim().is_empty() || o.rich.as_ref().is_some_and(|r| r.iter().any(|p| p.runs.iter().any(|x| !x.text.trim().is_empty()))));
    objects.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
    objects
}

/// Make the .docx for `slides` (indices into `project.slides`). `pictures` holds a PNG of a slide, by its index, for the
/// slides the caller could draw. Returns the file's bytes.
pub fn export(project: &Project, slides: &[usize], options: &HandoutOptions, pictures: &BTreeMap<usize, Vec<u8>>) -> Result<Vec<u8>, String> {
    let flat = project.flattened();
    let p = &flat;
    let slides: Vec<usize> = slides.iter().copied().filter(|&i| i < p.slides.len()).collect();
    if slides.is_empty() { return Err("there are no slides to put in the document".into()); }
    let portrait = options.orientation == HandoutOrientation::Portrait;
    let (page_w, page_h) = if portrait { A4 } else { (A4.1, A4.0) };
    let text_w = twips(page_w - 2.0 * MARGIN_MM);
    let text_h = twips(page_h - 2.0 * MARGIN_MM);
    let has_extras = options.note_lines || options.slide_notes || options.alt_text;

    let mut body = String::new();
    let mut media: Vec<(String, Vec<u8>)> = vec![];
    let mut rels = String::new();
    let mut first = true;
    // The title, then each slide.
    let mut title = Para::styled("Title");
    title.push(&p.name, false, false, false);
    body.push_str(&title.xml());
    for &i in &slides {
        let slide = &p.slides[i];
        let png = pictures.get(&i).and_then(|b| png_size(b).map(|d| (b, d))).filter(|_| matches!(slide.kind, SlideKind::Normal));
        let mut h = Para::styled("Heading2");
        h.keep_next = true;
        // A slide with a picture or room to write on gets a page of its own; plain text flows on.
        h.page_break_before = !first && (png.is_some() || options.note_lines);
        h.push(&format!("{}. {}", i + 1, slide.name), false, false, false);
        body.push_str(&h.xml());
        first = false;
        match (&slide.kind, png) {
            (SlideKind::Normal, Some((bytes, (w, ht)))) => {
                // As wide as the page allows, and short enough to leave room for what goes with it.
                let max_h = text_h - if has_extras { text_h * 45 / 100 } else { 0 } - twips(14.0);
                let (mut cx, mut cy) = (text_w * EMU_PER_TWIP, text_w * EMU_PER_TWIP * ht as i64 / w as i64);
                if cy > max_h * EMU_PER_TWIP { cx = cx * max_h * EMU_PER_TWIP / cy; cy = max_h * EMU_PER_TWIP; }
                let n = media.len() + 1;
                let rid = format!("rIdImg{n}");
                let file = format!("image{n}.png");
                let _ = write!(rels, "<Relationship Id=\"{rid}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/image\" Target=\"media/{file}\"/>");
                media.push((format!("word/media/{file}"), bytes.clone()));
                body.push_str(&picture_para(&Picture { rid, w_emu: cx, h_emu: cy, name: format!("Slide {}", i + 1), descr: format!("Slide {}: {}", i + 1, slide.name), id: n }, false));
            }
            (SlideKind::Normal, None) => {
                for o in text_objects(slide) { for para in object_paragraphs(o) { body.push_str(&para.xml()); } }
            }
            (SlideKind::Question(q), _) => {
                for line in handout_text::question_lines(q, p, options.correct_answers) {
                    let mut para = Para::new();
                    para.spacing_after = Some(80);
                    para.indent = line.indent as i64 * 360;
                    para.push(&line.text, line.bold, line.italic, false);
                    body.push_str(&para.xml());
                }
            }
            (SlideKind::Results, _) => {
                body.push_str(&Para::new().text("The player shows the learner's score, pass or fail, and a Retry button here.", false, true).xml());
            }
        }
        if options.slide_notes && !slide.notes.trim().is_empty() {
            let mut para = Para::new();
            para.spacing_after = Some(120);
            para.push("Notes: ", true, false, false);
            para.push(slide.notes.trim(), false, false, false);
            body.push_str(&para.xml());
        }
        if options.alt_text {
            let objects = slide.layers.first().map(|l| l.objects.as_slice()).unwrap_or(&[]);
            let items: Vec<String> = objects.iter().filter(|o| !o.decorative && !o.alt.trim().is_empty()).map(|o| format!("{}: {}", o.name, o.alt.trim())).collect();
            if !items.is_empty() {
                body.push_str(&Para::new().text("Alt text", true, false).xml());
                for it in items { let mut para = Para::new(); para.indent = 360; para.push(&format!("\u{2022} {it}"), false, false, false); body.push_str(&para.xml()); }
            }
        }
        if options.note_lines {
            body.push_str(&Para::new().text("Notes", true, false).xml());
            for _ in 0..6 { let mut line = Para::new(); line.ruled = true; line.spacing_after = Some(240); line.push(" ", false, false, false); body.push_str(&line.xml()); }
        }
    }
    let sect = format!(
        "<w:sectPr><w:pgSz w:w=\"{}\" w:h=\"{}\"{}/><w:pgMar w:top=\"{m}\" w:right=\"{m}\" w:bottom=\"{m}\" w:left=\"{m}\" w:header=\"400\" w:footer=\"400\" w:gutter=\"0\"/></w:sectPr>",
        if portrait { A4_TWIPS.0 } else { A4_TWIPS.1 }, if portrait { A4_TWIPS.1 } else { A4_TWIPS.0 }, if portrait { "" } else { " w:orient=\"landscape\"" }, m = twips(MARGIN_MM)
    );
    let document = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:wp=\"http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing\"><w:body>{body}{sect}</w:body></w:document>"
    );

    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut add = |name: &str, data: &[u8]| -> Result<(), String> {
        zip.start_file(name, opts).map_err(|e| e.to_string())?;
        zip.write_all(data).map_err(|e| e.to_string())
    };
    add("[Content_Types].xml", format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Default Extension=\"png\" ContentType=\"image/png\"/>\
<Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/><Override PartName=\"/word/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/></Types>").as_bytes())?;
    add("_rels/.rels", b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/></Relationships>")?;
    add("word/_rels/document.xml.rels", format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rIdStyles\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/>{rels}</Relationships>").as_bytes())?;
    add("word/styles.xml", STYLES.as_bytes())?;
    add("word/document.xml", document.as_bytes())?;
    for (name, bytes) in &media { add(name, bytes)?; }
    let out = zip.finish().map_err(|e| e.to_string())?;
    Ok(out.into_inner())
}

const STYLES: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:styles xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\
<w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii=\"Open Sans\" w:hAnsi=\"Open Sans\" w:cs=\"Open Sans\"/><w:sz w:val=\"22\"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after=\"120\" w:line=\"264\" w:lineRule=\"auto\"/></w:pPr></w:pPrDefault></w:docDefaults>\
<w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\"><w:name w:val=\"Normal\"/></w:style>\
<w:style w:type=\"paragraph\" w:styleId=\"Title\"><w:name w:val=\"Title\"/><w:basedOn w:val=\"Normal\"/><w:next w:val=\"Normal\"/><w:pPr><w:spacing w:after=\"240\"/></w:pPr><w:rPr><w:b/><w:sz w:val=\"44\"/></w:rPr></w:style>\
<w:style w:type=\"paragraph\" w:styleId=\"Heading2\"><w:name w:val=\"heading 2\"/><w:basedOn w:val=\"Normal\"/><w:next w:val=\"Normal\"/><w:pPr><w:keepNext/><w:spacing w:before=\"240\" w:after=\"120\"/><w:outlineLvl w:val=\"1\"/></w:pPr><w:rPr><w:b/><w:sz w:val=\"28\"/></w:rPr></w:style>\
</w:styles>";

#[cfg(test)]
mod tests;
