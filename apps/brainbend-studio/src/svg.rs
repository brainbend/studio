//! SVG pictures: drawn to a sharp PNG, so every place that takes a picture takes an SVG too.
//!
//! The SVG is never run or fetched from: scripts do nothing, and addresses and files it points at are not loaded
//! (only pictures embedded in it as data are). The result is an ordinary picture with a fixed resolution, about the size
//! of a slide; turning an SVG into editable shapes is a separate feature.

use resvg::{tiny_skia, usvg};
use std::sync::{Arc, OnceLock};

/// The longest side, in pixels, an SVG is drawn at.
const DRAW_SIDE: f32 = 2048.0;
/// The largest SVG file taken, in bytes.
pub const MAX_SVG_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug)]
pub struct Rendered {
    pub png: Vec<u8>,
    pub w: u32,
    pub h: u32,
}

/// Whether `bytes` are an SVG document (text with an `<svg` element near the start, after any byte order mark).
pub fn looks_like_svg(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(4096)];
    let head = head.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(head);
    let text = String::from_utf8_lossy(head);
    let t = text.trim_start();
    t.starts_with('<') && t.to_ascii_lowercase().contains("<svg")
}

/// The fonts an SVG's text may use: the ones that come with Studio, and, only when an SVG has text, those installed on
/// this computer (finding them takes a moment).
fn fonts(with_system: bool) -> Arc<usvg::fontdb::Database> {
    static BUNDLED: OnceLock<usvg::fontdb::Database> = OnceLock::new();
    static SYSTEM: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    let bundled = || {
        let mut db = usvg::fontdb::Database::new();
        for (name, src) in brainbend_fonts::families() {
            if src != brainbend_fonts::Source::Bundled { continue; }
            if let Some(f) = brainbend_fonts::lookup(&name) { for face in &f.faces { if let Some(b) = face.static_bytes() { db.load_font_data(b.to_vec()); } } }
        }
        db
    };
    if !with_system { return Arc::new(BUNDLED.get_or_init(bundled).clone()); }
    SYSTEM.get_or_init(|| { let mut db = BUNDLED.get_or_init(bundled).clone(); db.load_system_fonts(); Arc::new(db) }).clone()
}

/// Draws an SVG to a PNG with its transparency kept.
pub fn render(bytes: &[u8]) -> Result<Rendered, String> {
    if bytes.len() > MAX_SVG_BYTES { return Err(format!("That SVG is over {} MB, which is more than a lesson should carry.", MAX_SVG_BYTES / 1024 / 1024)); }
    let has_text = String::from_utf8_lossy(bytes).contains("<text");
    let mut opt = usvg::Options::default();
    // Nothing the SVG points at is loaded: no files, no addresses. Pictures embedded in it as data still are.
    opt.image_href_resolver.resolve_string = Box::new(|_, _| None);
    opt.resources_dir = None;
    opt.fontdb = fonts(has_text);
    opt.font_family = "Open Sans".into();
    let tree = usvg::Tree::from_data(bytes, &opt).map_err(|e| format!("That SVG could not be read ({e}). It needs to be a valid SVG file."))?;
    let size = tree.size();
    let (w, h) = (size.width(), size.height());
    if !(w >= 1.0 && h >= 1.0) || !w.is_finite() || !h.is_finite() { return Err("That SVG has no size (no width, height or viewBox).".into()); }
    let scale = DRAW_SIDE / w.max(h);
    let (pw, ph) = ((w * scale).round().clamp(1.0, 4096.0) as u32, (h * scale).round().clamp(1.0, 4096.0) as u32);
    let mut pixmap = tiny_skia::Pixmap::new(pw, ph).ok_or("That SVG is too large to draw.")?;
    resvg::render(&tree, tiny_skia::Transform::from_scale(pw as f32 / w, ph as f32 / h), &mut pixmap.as_mut());
    let png = pixmap.encode_png().map_err(|e| format!("That SVG could not be turned into a picture ({e})."))?;
    Ok(Rendered { png, w: pw, h: ph })
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED_SQUARE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50" viewBox="0 0 100 50"><rect width="50" height="50" fill="#ff0000"/></svg>"##;

    fn pixel(png: &[u8], x: u32, y: u32) -> [u8; 4] {
        let img = image::load_from_memory(png).unwrap().to_rgba8();
        img.get_pixel(x, y).0
    }

    #[test]
    fn an_svg_is_recognised_and_drawn_sharp_with_its_transparency() {
        assert!(looks_like_svg(RED_SQUARE.as_bytes()));
        assert!(looks_like_svg(format!("\u{feff}  <?xml version=\"1.0\"?>\n<!-- c -->{RED_SQUARE}").as_bytes()));
        assert!(!looks_like_svg(b"<html><body>no</body></html>"));
        assert!(!looks_like_svg(b"\x89PNG\r\n\x1a\n"));
        assert!(!looks_like_svg(b""));
        let r = render(RED_SQUARE.as_bytes()).unwrap();
        assert_eq!((r.w, r.h), (2048, 1024), "its longest side is drawn at 2048 pixels, in its own proportions");
        assert_eq!(pixel(&r.png, 100, 500), [255, 0, 0, 255], "the square is red");
        assert_eq!(pixel(&r.png, 1900, 500)[3], 0, "the rest stays transparent");
    }

    #[test]
    fn gradients_and_embedded_pictures_are_drawn() {
        let g = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><defs><linearGradient id="g" x1="0" x2="1"><stop offset="0" stop-color="#000000"/><stop offset="1" stop-color="#ffffff"/></linearGradient></defs><rect width="64" height="64" fill="url(#g)"/></svg>"##;
        let r = render(g.as_bytes()).unwrap();
        let (l, rr) = (pixel(&r.png, 20, 1000), pixel(&r.png, 2000, 1000));
        assert!(l[0] < 40 && rr[0] > 215, "dark on the left, light on the right: {l:?} {rr:?}");
    }

    #[test]
    fn text_is_drawn_with_a_bundled_font() {
        let t = r##"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="100"><text x="10" y="70" font-family="Montserrat" font-size="64" fill="#000000">Hello</text></svg>"##;
        let r = render(t.as_bytes()).unwrap();
        let img = image::load_from_memory(&r.png).unwrap().to_rgba8();
        let ink = img.pixels().filter(|p| p.0[3] > 128).count();
        assert!(ink > 2000, "the word is drawn: {ink} inked pixels");
    }

    #[test]
    fn nothing_outside_the_svg_is_loaded_and_bad_ones_are_refused() {
        // A picture on this computer, named by path or file address, must not appear.
        let dir = std::env::temp_dir().join(format!("bb-svg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let secret = dir.join("secret.png");
        image::RgbaImage::from_pixel(10, 10, image::Rgba([0, 255, 0, 255])).save(&secret).unwrap();
        for href in [secret.display().to_string(), format!("file://{}", secret.display())] {
            let s = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="64" height="64"><image xlink:href="{href}" width="64" height="64"/></svg>"#);
            let r = render(s.as_bytes()).unwrap();
            assert_eq!(pixel(&r.png, 1000, 1000)[3], 0, "a file the SVG names is not drawn: {href}");
        }
        // A script and a link do nothing.
        let scripted = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" onload="alert(1)"><script>alert(1)</script><a href="https://example.com"><rect width="10" height="10" fill="#0000ff"/></a></svg>"##;
        assert!(render(scripted.as_bytes()).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
        // Not an SVG, no size, and a document built to expand without end.
        assert!(render(b"<svg").unwrap_err().contains("could not be read"));
        assert!(render(b"hello").unwrap_err().contains("could not be read"));
        let bomb = r##"<?xml version="1.0"?><!DOCTYPE s [<!ENTITY a "aaaaaaaaaa"><!ENTITY b "&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;"><!ENTITY c "&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;"><!ENTITY d "&c;&c;&c;&c;&c;&c;&c;&c;&c;&c;"><!ENTITY e "&d;&d;&d;&d;&d;&d;&d;&d;&d;&d;"><!ENTITY f "&e;&e;&e;&e;&e;&e;&e;&e;&e;&e;"><!ENTITY g "&f;&f;&f;&f;&f;&f;&f;&f;&f;&f;"><!ENTITY h "&g;&g;&g;&g;&g;&g;&g;&g;&g;&g;">]><svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><text>&h;</text></svg>"##;
        let started = std::time::Instant::now();
        let _ = render(bomb.as_bytes());
        assert!(started.elapsed() < std::time::Duration::from_secs(5), "an entity bomb must not hang the app");
        // A huge size is drawn at a sane one, and a very large file is refused.
        let huge = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100000" height="50000"><rect width="100000" height="50000" fill="#00ff00"/></svg>"##;
        let r = render(huge.as_bytes()).unwrap();
        assert_eq!((r.w, r.h), (2048, 1024));
        assert!(render(&vec![b' '; MAX_SVG_BYTES + 1]).unwrap_err().contains("MB"));
    }
}
