//! Pictures for the AI tools: a search of royalty-free images, and adding a picture to a slide from a web
//! address or from a file the tool made or saved (a photo it fetched, a diagram or chart it drew, an image it generated).
//!
//! The search is Openverse, which gathers openly licensed pictures (Wikimedia Commons, museums, and others) and
//! needs no account. By default only pictures that need no credit (CC0 and public domain) are offered; a picture
//! that asks for a credit is offered only on request, and its credit is put in the slide's notes when it is added.

use super::*;
use crate::mcp_tools::*;
use serde_json::{json, Value};
use std::time::Duration;

pub(super) const SEARCH_URL: &str = "https://api.openverse.org/v1/images/";
/// The largest picture taken, in bytes.
const MAX_BYTES: u64 = 15 * 1024 * 1024;
/// Pictures bigger than this on their longest side are made smaller, so a lesson does not carry a 6000 pixel photo.
const MAX_SIDE: u32 = 2400;

fn agent(secs: u64) -> ureq::Agent {
    ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(secs))).build().into()
}

const USER_AGENT: &str = concat!("BrainBend-Studio/", env!("CARGO_PKG_VERSION"));

/// A picture ready to store: PNG or JPEG bytes, and its size.
struct Picture { bytes: Vec<u8>, mime: &'static str, w: u32, h: u32 }

/// Reads `bytes` as a picture. PNG and JPEG are kept as they are (made smaller if very large); WebP and GIF, which
/// some picture hosts send whatever was asked for, are converted: to PNG when they have see-through parts, else JPEG.
fn prepare(bytes: Vec<u8>) -> Result<Picture, String> {
    let png = bytes.starts_with(&[0x89, b'P', b'N', b'G']);
    let jpeg = bytes.starts_with(&[0xFF, 0xD8]);
    let img = image::load_from_memory(&bytes).map_err(|e| format!("That is not a picture Studio can read (PNG, JPEG, WebP or GIF): {e}"))?;
    let (w, h) = (img.width(), img.height());
    if (png || jpeg) && w.max(h) <= MAX_SIDE { return Ok(Picture { bytes, mime: if png { "image/png" } else { "image/jpeg" }, w, h }); }
    let img = if w.max(h) > MAX_SIDE { img.resize(MAX_SIDE, MAX_SIDE, image::imageops::FilterType::Lanczos3) } else { img };
    let as_png = if png { true } else if jpeg { false } else { img.color().has_alpha() && img.to_rgba8().pixels().any(|p| p[3] < 255) };
    let mut out = std::io::Cursor::new(Vec::new());
    if as_png {
        img.write_to(&mut out, image::ImageFormat::Png).map_err(|e| format!("That picture could not be converted ({e})."))?;
    } else {
        let rgb = img.to_rgb8();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 88).encode_image(&rgb).map_err(|e| format!("That picture could not be converted ({e})."))?;
    }
    Ok(Picture { bytes: out.into_inner(), mime: if as_png { "image/png" } else { "image/jpeg" }, w: img.width(), h: img.height() })
}

fn download(url: &str) -> Result<Vec<u8>, String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) { return Err("`url` must be a web address starting with https://.".into()); }
    let mut reply = agent(25).get(url).header("User-Agent", USER_AGENT).header("Accept", "image/jpeg,image/png,image/*;q=0.8").call().map_err(|e| format!("The picture could not be downloaded from {url}: {e}"))?;
    reply.body_mut().with_config().limit(MAX_BYTES).read_to_vec().map_err(|e| format!("The picture at {url} could not be read (is it over {} MB?): {e}", MAX_BYTES / 1024 / 1024))
}

fn read_file(path: &str) -> Result<(Vec<u8>, String), String> {
    let p = std::path::Path::new(path);
    if !p.is_absolute() { return Err("`path` must be the full path of a PNG or JPEG file, starting from the root of the disk.".into()); }
    let len = std::fs::metadata(p).map_err(|e| format!("`path`: {path} could not be found ({e})."))?.len();
    if len > MAX_BYTES { return Err(format!("`path`: that file is over {} MB.", MAX_BYTES / 1024 / 1024)); }
    let bytes = std::fs::read(p).map_err(|e| format!("`path`: {path} could not be read ({e})."))?;
    Ok((bytes, p.file_name().and_then(|n| n.to_str()).unwrap_or("picture").to_string()))
}

impl App {
    /// Looks for openly licensed pictures matching some words.
    pub(super) fn mcp_find_images(&mut self, a: &Value) -> Result<String, String> {
        let query = text_arg(a, "query")?.map(|q| q.trim().to_string()).filter(|q| !q.is_empty()).ok_or("`query` must be some words to look for, such as \"forklift warehouse\".")?;
        let count = num_arg(a, "count", 1.0, 12.0)?.unwrap_or(6.0) as usize;
        let credit_ok = bool_arg(a, "allow_credit_required")?.unwrap_or(false);
        let shape = match text_arg(a, "shape")?.as_deref() {
            None => None,
            Some(s @ ("wide" | "tall" | "square")) => Some(s.to_string()),
            Some(_) => return Err("`shape` must be wide, tall or square.".into()),
        };
        let base = self.mcp.image_search_url.clone().unwrap_or_else(|| SEARCH_URL.to_string());
        let mut req = agent(15).get(&base).header("User-Agent", USER_AGENT)
            .query("q", &query).query("page_size", count.to_string())
            .query("license", if credit_ok { "cc0,pdm,by" } else { "cc0,pdm" }).query("extension", "jpg,png").query("mature", "false");
        if let Some(s) = &shape { req = req.query("aspect_ratio", s); }
        let mut reply = req.call().map_err(|e| format!("The picture search did not answer ({e}). Try again in a moment, or use add_image with a picture file you already have."))?;
        let found: Value = reply.body_mut().read_to_string().ok().and_then(|t| serde_json::from_str(&t).ok()).ok_or("The picture search gave an answer that could not be read.")?;
        let list = found.get("results").and_then(Value::as_array).ok_or("The picture search gave an answer that could not be read.")?;
        let text = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        let results: Vec<Value> = list.iter().filter_map(|r| {
            let url = text(r, "url");
            if url.is_empty() { return None; }
            let license = text(r, "license");
            let needs_credit = !matches!(license.as_str(), "cc0" | "pdm");
            let credit = if needs_credit { Some(text(r, "attribution")).filter(|c| !c.is_empty()).unwrap_or_else(|| format!("{} by {} ({} {})", text(r, "title"), text(r, "creator"), license.to_uppercase(), text(r, "license_version"))) } else { String::new() };
            Some(json!({
                "url": url, "title": text(r, "title"), "creator": text(r, "creator"), "license": format!("{} {}", license.to_uppercase(), text(r, "license_version")).trim(),
                "credit_required": needs_credit, "credit": credit, "width": r.get("width"), "height": r.get("height"), "page": text(r, "foreign_landing_url"),
            }))
        }).take(count).collect();
        self.status = format!("AI tool: looked for pictures of {query}");
        done(json!({
            "found": results.len(), "results": results,
            "next": if results.is_empty() { "Nothing matched. Try other, simpler words." } else { "Choose one that fits and call add_image with its url (and its credit when credit_required). Check the title says what you need: results are matched by words only." },
        }))
    }

    /// Adds a picture to a slide, from a web address or a file.
    pub(super) fn mcp_add_image(&mut self, a: &Value, count: usize) -> Result<String, String> {
        let i = slide_arg(a, "slide", count)?;
        let (url, path) = (text_arg(a, "url")?, text_arg(a, "path")?);
        let alt = text_arg(a, "alt")?.map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
        let decorative = bool_arg(a, "decorative")?.unwrap_or(false);
        if alt.is_none() && !decorative { return Err("`alt` is needed: a short description of the picture for people who cannot see it (or set decorative to true for a picture that only decorates).".into()); }
        let credit = text_arg(a, "credit")?.map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
        let (sw, sh) = { let s = &self.project.as_ref().unwrap().slides[i]; (s.width, s.height) };
        let (x, y) = (num_arg(a, "x", -20_000.0, 20_000.0)?, num_arg(a, "y", -20_000.0, 20_000.0)?);
        let (bw, bh) = (num_arg(a, "width", 1.0, 20_000.0)?, num_arg(a, "height", 1.0, 20_000.0)?);
        let to_back = bool_arg(a, "send_to_back")?.unwrap_or(false);
        let (bytes, source_name) = match (url.as_deref(), path.as_deref()) {
            (Some(_), Some(_)) | (None, None) => return Err("Give either `url` (a web address of a picture) or `path` (a picture file on this computer), one of them.".into()),
            (Some(u), None) => (download(u)?, u.split(['?', '#']).next().and_then(|u| u.rsplit('/').next()).filter(|n| !n.is_empty()).unwrap_or("picture").to_string()),
            (None, Some(p)) => read_file(p)?,
        };
        let pic = prepare(bytes)?;
        // Fit inside the box given, keeping the picture's proportions. Without a box it is up to 60% of the slide, centred.
        let ratio = pic.w as f32 / pic.h as f32;
        let (mut fw, mut fh) = match (bw, bh) {
            (Some(w), Some(h)) => if w / h > ratio { (h * ratio, h) } else { (w, w / ratio) },
            (Some(w), None) => (w, w / ratio),
            (None, Some(h)) => (h * ratio, h),
            (None, None) => if sw * 0.6 / (sh * 0.6) > ratio { (sh * 0.6 * ratio, sh * 0.6) } else { (sw * 0.6, sw * 0.6 / ratio) },
        };
        fw = fw.round().max(1.0);
        fh = fh.round().max(1.0);
        let (bx, by) = (x.unwrap_or(((sw - bw.unwrap_or(fw)) / 2.0).max(0.0)), y.unwrap_or(((sh - bh.unwrap_or(fh)) / 2.0).max(0.0)));
        // Centred in the box when both sides of it were given.
        let (ox, oy) = (bx + (bw.unwrap_or(fw) - fw) / 2.0, by + (bh.unwrap_or(fh) - fh) / 2.0);
        let key = self.new_id("asset");
        let mut o = blank_object(self.new_id("obj"), text_arg(a, "name")?.as_deref().unwrap_or("Picture"), Kind::Image, ox.round(), oy.round(), fw, fh);
        o.asset = Some(key.clone());
        o.alt = alt.unwrap_or_default();
        let described = describe_object(&o);
        self.snapshot();
        let slide_no = i + 1;
        let p = self.project.as_mut().unwrap();
        p.assets.insert(key, Asset { name: source_name, mime: pic.mime.into(), bytes: pic.bytes, file: None });
        let slide = &mut p.slides[i];
        if to_back { slide.layers[0].objects.insert(0, o); self.selected = Some((0, 0)); } else { slide.layers[0].objects.push(o); self.selected = Some((0, slide.layers[0].objects.len() - 1)); }
        if let Some(c) = &credit {
            let line = format!("Image credit: {c}");
            if !slide.notes.contains(&line) { if !slide.notes.is_empty() { slide.notes.push('\n'); } slide.notes.push_str(&line); }
        }
        self.slide = i;
        self.layer = 0;
        self.dirty = true;
        self.status = format!("AI tool: added a picture to slide {slide_no}");
        done(json!({ "added": described, "picture_pixels": [pic.w, pic.h], "credit_in_notes": credit.is_some() }))
    }
}

// ---- icons ----

impl App {
    /// Searches the icon gallery (Phosphor icons) by words in their names.
    pub(super) fn mcp_find_icons(&mut self, a: &Value) -> Result<String, String> {
        let query = text_arg(a, "query")?.map(|q| q.trim().to_string()).filter(|q| !q.is_empty()).ok_or("`query` must be a word or two, such as \"house\" or \"arrow right\".")?;
        let count = num_arg(a, "count", 1.0, 40.0)?.unwrap_or(12.0) as usize;
        let all = crate::icons::search(&query);
        let names: Vec<&str> = all.iter().take(count).map(|i| i.names[0]).collect();
        done(json!({ "found": all.len(), "icons": names, "next": if names.is_empty() { "Nothing matched. Try one simpler word." } else { "Use add_icon with one of these names." } }))
    }

    /// Adds an icon from the gallery to a slide as a vector shape.
    pub(super) fn mcp_add_icon(&mut self, a: &Value, count: usize) -> Result<String, String> {
        let i = slide_arg(a, "slide", count)?;
        let name = text_arg(a, "icon")?.map(|n| n.trim().to_lowercase().replace(' ', "-")).filter(|n| !n.is_empty()).ok_or("`icon` must be an icon's name, such as \"house\" (find_icons searches the names).")?;
        let icon = crate::icons::all().iter().find(|ic| ic.names.contains(&name.as_str())).ok_or_else(|| {
            let near: Vec<&str> = crate::icons::search(&name.replace('-', " ")).iter().take(8).map(|i| i.names[0]).collect();
            if near.is_empty() { format!("There is no icon called {name:?}. Use find_icons to search the names.") } else { format!("There is no icon called {name:?}. Close names: {}.", near.join(", ")) }
        })?;
        let (cmds, aspect) = crate::icons::outline(icon).ok_or("That icon could not be drawn.")?;
        let (sw, sh) = { let s = &self.project.as_ref().unwrap().slides[i]; (s.width, s.height) };
        let size = num_arg(a, "size", 8.0, 4000.0)?.unwrap_or(120.0);
        let (w, h) = if aspect >= 1.0 { (size, size / aspect) } else { (size * aspect, size) };
        let (w, h) = (w.round().max(1.0), h.round().max(1.0));
        let (x, y) = (num_arg(a, "x", -20_000.0, 20_000.0)?, num_arg(a, "y", -20_000.0, 20_000.0)?);
        let color = color_arg(a, "color")?.unwrap_or_else(|| self.project.as_ref().map(|p| p.theme.text()).unwrap_or(Rgba(34, 34, 34, 255)));
        let decorative = bool_arg(a, "decorative")?.unwrap_or(false);
        let alt = text_arg(a, "alt")?.map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
        let mut o = blank_object(self.new_id("obj"), text_arg(a, "name")?.as_deref().unwrap_or(&icon.title()), Kind::Path(cmds), 0.0, 0.0, w, h);
        // Lines in the outline color (default the lesson's text color) and, if asked, the inside in the fill color.
        o.stroke = Some(Stroke { color, width: 0.0 });
        o.fill = color_arg(a, "fill")?;
        o.icon = Some(brainbend_doc::shapes::silhouette(match &o.kind { Kind::Path(c) => c, _ => unreachable!() }));
        o.decorative = decorative;
        o.alt = if decorative { String::new() } else { alt.unwrap_or_else(|| icon.title()) };
        o.x = x.unwrap_or(((sw - w) / 2.0).max(0.0)).round();
        o.y = y.unwrap_or(((sh - h) / 2.0).max(0.0)).round();
        let described = describe_object(&o);
        self.snapshot();
        let slide = &mut self.project.as_mut().unwrap().slides[i];
        if bool_arg(a, "send_to_back")?.unwrap_or(false) { slide.layers[0].objects.insert(0, o); self.selected = Some((0, 0)); } else { slide.layers[0].objects.push(o); self.selected = Some((0, slide.layers[0].objects.len() - 1)); }
        self.slide = i;
        self.layer = 0;
        self.dirty = true;
        self.status = format!("AI tool: added the {} icon to slide {}", icon.title(), i + 1);
        done(described)
    }
}
