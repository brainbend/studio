//! Importer for `.story` course packages (OPC-style ZIP of XML parts).
//!
//! Based on observation of sample files; see docs/project-goals.md. Only text,
//! rectangle-like shapes and pictures are converted. Everything else is reported.
pub mod emf;

use md5::{Digest, Md5};
use roxmltree::{Document, Node};
use std::collections::HashMap;
use std::io::{Read, Seek};
use brainbend_doc::*;

const MAX_ENTRY_BYTES: u64 = 256 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 2 * 1024 * 1024 * 1024;
/// Scale used where no slide size is known (tests, helper parsers). See `font_scale_for`.
pub const FONT_SCALE: f32 = 3.4;
/// Source font sizes are points on a slide 540 units tall: canvas size = points * 4/3 * (height / 540).
/// Evidence: a 960x540 file's auto-sized title boxes cap the scale near 1.5 (ruling out growth with
/// width, which would give 1.7), and the 1920x1400 sample measured 3.4 against 3.46 here.
///
/// Slides shorter than 540 (the source app's 16:9 default is 720x405) do not shrink their text: 12pt is 16 px there too.
/// Measured on sample files, whose body text is about 1.25-1.3 times what 1.0 gives.
pub fn font_scale_for(_slide_width: f32, slide_height: f32) -> f32 {
    4.0 / 3.0 * (slide_height / 540.0).max(1.0)
}

mod still;
pub use still::{add_stills, still_asset, video_still};

pub fn import<R: Read + Seek>(reader: R, name: &str) -> Result<Project, String> {
    let parts = read_parts(reader)?;
    let story_xml = text_part(&parts, "story/story.xml")?;
    let story = Document::parse(strip_bom(&story_xml)).map_err(|e| format!("story.xml: {e}"))?;
    let root = story.root_element();
    // Project size sits in a `<prop><sz/></prop>` entry (observed in the sample).
    let (width, height) = root
        .descendants()
        .filter(|n| n.has_tag_name("sz") && num(*n, "w", 0.0) > 0.0)
        .map(|n| (num(n, "w", 0.0), num(n, "h", 0.0)))
        .next()
        .unwrap_or((1920.0, 1080.0));

    let mut project = Project::new(name);
    let assets = load_assets(&parts, root, &mut project);
    // Pixel sizes of the pictures: a crop is stored in source pixels.
    let sizes: HashMap<String, (f32, f32)> = project.assets.iter().filter_map(|(k, a)| Some((k.clone(), image_size(&a.bytes)?))).collect();
    project.variables = parse_variables(root);
    let var_types: HashMap<String, Value> =
        project.variables.iter().map(|v| (v.id.clone(), v.initial.clone())).collect();

    let mut scene_of: HashMap<String, usize> = HashMap::new();
    let mut slide_parts: Vec<String> = Vec::new();
    for (gi, (scene_name, keys)) in scene_groups(&parts, root).into_iter().enumerate() {
        project.scenes.push(Scene { id: format!("scene-{}", gi + 1), name: scene_name, bank: None });
        for k in keys {
            scene_of.insert(k.clone(), gi);
            slide_parts.push(k);
        }
    }
    for key in slide_parts {
        let xml = text_part(&parts, &key)?;
        let doc = Document::parse(strip_bom(&xml)).map_err(|e| format!("{key}: {e}"))?;
        let sld = doc.root_element();
        let (palette, inherited_bg) = master_palette(&parts, sld);
        let hidden_groups = sld
            .descendants()
            .filter(|n| n.has_tag_name("group") && is_hidden(*n))
            .filter_map(|n| n.attribute("g").map(String::from))
            .collect();
        let drawing_text = drawing_shape_text(&parts, &palette);
        let ph_styles = layout_placeholders(&parts, sld, &palette);
        let mut ctx = Ctx { notes: vec![], assets: &assets, hidden_groups, palette, missing_scheme: 0, font_scale: font_scale_for(width, height), var_types: &var_types, triggers: vec![], layer: String::new(), sizes: &sizes, parts: &parts, restores: vec![], drawing_text, ph_styles, emfs: slide_emfs(&parts, &key), drawn: vec![] };
        let mut slide = Slide {
            id: sld.attribute("g").unwrap_or_default().into(),
            // Unnamed slides are common in templates; number them.
            name: match sld.attribute("name") {
                Some(n) if !n.trim().is_empty() => n.into(),
                // The source app shows an unnamed slide under the text of its title.
                _ => title_text(sld).unwrap_or_else(|| format!("Slide {}", project.slides.len() + 1)),
            },
            width,
            height,
            layers: vec![],
            triggers: vec![],
            kind: SlideKind::Normal,
            background: child(sld, "bg")
                .and_then(|bg| child(bg, "solidFill"))
                .and_then(|f| child(f, "clr"))
                .and_then(|c| resolve_clr(c, &mut ctx))
                .or_else(|| {
                    // No fill of its own (and not a picture or gradient): the slide takes its layout's or master's.
                    let own_fill = child(sld, "bg").is_some_and(|bg| ["solidFill", "gradFill", "picFill", "blipFill", "pattFill"].iter().any(|f| child(bg, f).is_some()));
                    if own_fill { None } else { inherited_bg }
                })
                .map(brainbend_doc::Background::Solid),
            layout: None,
            fade_ms: 0, hide_backdrop: false,
            auto_advance: sld.attribute("advMode") == Some("auto"), nav: Default::default(),
            notes: String::new(),
            scene: String::new(),
        };
        ctx.layer = slide.id.clone();
        if let Some(layout) = backdrop_layout(&parts, sld, &mut ctx) {
            slide.layout = Some(layout.id.clone());
            if !project.layouts.iter().any(|l| l.id == layout.id) { project.layouts.push(layout); }
        }
        parse_triggers(sld, Owner::Slide, &mut ctx);
        slide.layers.push(Layer {
            id: slide.id.clone(),
            name: "Base layer".into(),
            objects: convert_shapes(child(sld, "shapeLst"), &mut ctx, false),
            duration_ms: timeline_len(sld),
            modal: true,
            hide_others: false,
            pause_base: false,
            group_names: Default::default(), focus_order: vec![],
        });
        if let Some(list) = child(sld, "sldLayerLst") {
            for l in list.children().filter(|n| n.has_tag_name("sldLayer")) {
                ctx.layer = l.attribute("g").unwrap_or_default().into();
                parse_triggers(l, Owner::Layer(l.attribute("g").unwrap_or_default().into()), &mut ctx);
                slide.layers.push(Layer {
                    id: l.attribute("g").unwrap_or_default().into(),
                    name: l.attribute("name").unwrap_or("Layer").into(),
                    objects: convert_shapes(child(l, "shapeLst"), &mut ctx, false),
                    duration_ms: timeline_len(l),
                    // `modal` blocks the base layer. `stop` is "hide other slide layers": it is true on every layer of every
                    // sample except one file, whose layers stack; `top="true"` is on all of them, so it is not that.
                    modal: l.attribute("modal") != Some("false"),
                    hide_others: l.attribute("stop") == Some("true"),
                    pause_base: l.attribute("pause") == Some("true"),
                    group_names: Default::default(), focus_order: vec![],
                });
            }
        }
        slide.triggers = std::mem::take(&mut ctx.triggers);
        // Hovering out puts the object it changed back into the state it started in.
        for (n, (hovered, target)) in std::mem::take(&mut ctx.restores).into_iter().enumerate() {
            let start = slide.layers.iter().flat_map(|l| l.objects.iter()).find(|o| o.id == target).map(|o| if o.initial_state.is_empty() { "Normal".to_string() } else { o.initial_state.clone() });
            if let Some(state) = start {
                slide.triggers.push(Trigger { id: format!("restore-{n}-{hovered}"), owner: Owner::Object(hovered), event: Event::MouseLeave, action: Action::SetObjectState { object: target, state }, conditions: vec![] });
            }
        }
        for (k, a, _) in std::mem::take(&mut ctx.drawn) {
            project.assets.insert(k, a);
        }
        // The source app keeps a trigger whose layer was later deleted, and it does nothing there either. Leaving it
        // in would make the checker report an error on a slide that is fine, so it is dropped and mentioned.
        let layer_ids: Vec<String> = slide.layers.iter().map(|l| l.id.clone()).collect();
        let before = slide.triggers.len();
        slide.triggers.retain(|t| !matches!(&t.action, Action::ShowLayer(l) | Action::HideLayer(l) if !layer_ids.contains(l)));
        if slide.triggers.len() < before {
            ctx.notes.push(format!("{} trigger(s) pointed at a layer that was deleted in the source app and were left out", before - slide.triggers.len()));
        }
        if ctx.missing_scheme > 0 {
            ctx.notes.push(format!(
                "{} fills use theme colors, which are not resolved yet and were left unfilled",
                ctx.missing_scheme
            ));
        }
        let mut counts: Vec<(String, usize)> = vec![];
        for n in ctx.notes {
            match counts.iter_mut().find(|(k, _)| *k == n) {
                Some((_, c)) => *c += 1,
                None => counts.push((n, 1)),
            }
        }
        project.notes.extend(counts.into_iter().map(|(n, c)| {
            if c > 1 { format!("{}: {n} (x{c})", slide.name) } else { format!("{}: {n}", slide.name) }
        }));
        slide.scene = format!("scene-{}", scene_of.get(&key).copied().unwrap_or(0) + 1);
        project.slides.push(slide);
    }
    if project.slides.is_empty() {
        return Err("no slides found in package".into());
    }
    // Videos the file gives no picture for get a frame of their own, when ffmpeg is there to make it.
    add_stills(&mut project);
    Ok(project)
}

/// Slide parts grouped by scene, in course order: scenes in `sceneLst` order, each scene's `sldId`
/// (a relationship id in `story/_rels/story.xml.rels`) resolved to its part. Slides no scene lists
/// (e.g. question banks) follow in an extra group, sorted by name. Scenes with no resolvable slide
/// are dropped.
fn scene_groups(parts: &HashMap<String, Vec<u8>>, story: Node) -> Vec<(String, Vec<String>)> {
    let mut rels: HashMap<String, String> = HashMap::new();
    if let Some(xml) = parts.get("story/_rels/story.xml.rels").and_then(|b| std::str::from_utf8(b).ok()) {
        if let Ok(doc) = Document::parse(strip_bom(xml)) {
            for r in doc.descendants().filter(|n| n.has_tag_name("Relationship")) {
                if let (Some(id), Some(target)) = (r.attribute("Id"), r.attribute("Target")) {
                    rels.insert(id.into(), target.trim_start_matches('/').into());
                }
            }
        }
    }
    let is_slide = |k: &str| k.starts_with("story/slides/") && k.ends_with(".xml");
    let mut groups: Vec<(String, Vec<String>)> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    let mut scenes: Vec<Node> = story.descendants().filter(|n| n.has_tag_name("scene")).collect();
    if scenes.is_empty() { scenes.push(story); }
    for sc in scenes {
        let mut keys: Vec<String> = Vec::new();
        for sid in sc.descendants().filter(|n| n.has_tag_name("sldId")) {
            if let Some(target) = sid.text().and_then(|id| rels.get(id.trim())) {
                if is_slide(target) && parts.contains_key(target) && !seen.contains(target) {
                    seen.push(target.clone());
                    keys.push(target.clone());
                }
            }
        }
        if !keys.is_empty() {
            let name = sc.attribute("name").map(str::trim).filter(|n| !n.is_empty() && sc != story).map(String::from).unwrap_or_else(|| format!("Scene {}", groups.len() + 1));
            groups.push((name, keys));
        }
    }
    let mut rest: Vec<String> = parts.keys().filter(|k| is_slide(k) && !seen.contains(*k)).cloned().collect();
    rest.sort();
    if !rest.is_empty() {
        let name = if groups.is_empty() { "Scene 1".to_string() } else { "Unlisted slides".to_string() };
        groups.push((name, rest));
    }
    groups
}

struct Ctx<'a> {
    notes: Vec<String>,
    assets: &'a HashMap<String, ()>,
    /// Theme color name (`accent1`, `dk1`, ...) to color, from the slide's master.
    palette: HashMap<String, Rgba>,
    missing_scheme: usize,
    /// Multiplier from source font sizes to canvas units for this project.
    font_scale: f32,
    /// Variable id to its initial value; the value's kind tells how to read trigger operands.
    var_types: &'a HashMap<String, Value>,
    triggers: Vec<Trigger>,
    /// Picture asset id to its size in pixels.
    sizes: &'a HashMap<String, (f32, f32)>,
    /// The layer being converted (the slide's own id for the base layer); "this layer" actions point at it.
    layer: String,
    /// Ids of hidden groups. Group members point at their group via `parentG` (they are not nested).
    hidden_groups: std::collections::HashSet<String>,
    parts: &'a HashMap<String, Vec<u8>>,
    /// The text colour the source app gives shapes drawn from the shape gallery (the "DrawingShape" default style:
    /// usually white), for shapes that do not set one.
    drawing_text: Option<Rgba>,
    /// Hover triggers that change a state and "restore the previous state when the user hovers out":
    /// (the object hovered over, the object whose state changes).
    restores: Vec<(String, String)>,
    /// Text defaults of the slide layout's placeholders, by the placeholder's `g`: a title on a slide
    /// inherits its size, weight and font from the layout when it does not set them itself.
    ph_styles: HashMap<String, PhStyle>,
    /// The metafiles this slide's relationships list, in order, and whether a character has taken each.
    emfs: Vec<(String, bool)>,
    /// Characters drawn from those metafiles: (asset key, asset, size in device pixels).
    drawn: Vec<(String, Asset, (f32, f32))>,
}

impl Ctx<'_> {
    /// Draw the character `n` from the next metafile its slide lists, and return its asset key.
    /// The slide lists its metafiles in the order of its characters; when the picture's crop says
    /// how wide the illustration is, a metafile of that width is preferred.
    fn draw_character(&mut self, n: Node) -> Option<String> {
        let g = n.attribute("assetG").filter(|g| !g.is_empty()).or(n.attribute("g"))?.to_string();
        let wide = child(n, "crop").and_then(|c| child(c, "rect")).map(|r| num(r, "r", 0.0)).unwrap_or(0.0);
        let width_of = |part: &str| self.parts.get(part).and_then(|b| emf::device_size(b)).map(|s| s.0);
        let free: Vec<usize> = (0..self.emfs.len()).filter(|&i| !self.emfs[i].1).collect();
        let fits = |i: usize| wide < 1.0 || width_of(&self.emfs[i].0).is_some_and(|w| (w - wide).abs() <= w * 0.04);
        let pick = free.iter().copied().find(|&i| fits(i)).or_else(|| free.first().copied())?;
        self.emfs[pick].1 = true;
        let part = self.emfs[pick].0.clone();
        let bytes = self.parts.get(&part)?;
        let size = emf::device_size(bytes)?;
        let rendered = emf::render(bytes, 1800)?;
        let ill = child(n, "illConfig");
        let at = |a: &str| ill.and_then(|c| c.attribute(a)).unwrap_or("");
        let name = [at("characterName"), at("expression"), at("pose")].iter().filter(|s| !s.is_empty()).copied().collect::<Vec<_>>().join(", ");
        if !rendered.skipped.is_empty() {
            let who = if name.is_empty() { "a character".to_string() } else { format!("character {name}") };
            self.notes.push(format!("{who} uses {}, which is not drawn (or only roughly), so it may look different from the source app", rendered.skipped.join(", ")));
        }
        self.drawn.push((g.clone(), Asset { name: if name.is_empty() { "Character".into() } else { name }, mime: "image/png".into(), bytes: rendered.png, file: None }, size));
        Some(g)
    }
}

/// The metafiles a slide's relationships list, in order.
fn slide_emfs(parts: &HashMap<String, Vec<u8>>, slide_part: &str) -> Vec<(String, bool)> {
    let (dir, file) = slide_part.rsplit_once('/').unwrap_or(("", slide_part));
    let Some(xml) = parts.get(&format!("{dir}/_rels/{file}.rels")).and_then(|b| std::str::from_utf8(b).ok()) else { return vec![] };
    let Ok(doc) = Document::parse(strip_bom(xml)) else { return vec![] };
    doc.descendants()
        .filter(|n| n.has_tag_name("Relationship"))
        .filter_map(|n| n.attribute("Target"))
        .map(|t| t.trim_start_matches('/').to_string())
        .filter(|t| t.to_ascii_lowercase().ends_with(".emf") && parts.contains_key(t))
        .map(|t| (t, false))
        .collect()
}

fn read_parts<R: Read + Seek>(reader: R) -> Result<HashMap<String, Vec<u8>>, String> {
    let mut zip = zip::ZipArchive::new(reader).map_err(|e| format!("not a ZIP package: {e}"))?;
    let mut parts = HashMap::new();
    let mut total = 0u64;
    for i in 0..zip.len() {
        let f = zip.by_index(i).map_err(|e| e.to_string())?;
        if f.is_dir() {
            continue;
        }
        let name = f.name().map_err(|e| e.to_string())?.trim_start_matches('/').to_string();
        if name.contains("..") {
            return Err(format!("unsafe part name {name:?}"));
        }
        if f.size() > MAX_ENTRY_BYTES {
            return Err(format!("part {name} too large"));
        }
        let mut buf = Vec::new();
        f.take(MAX_ENTRY_BYTES + 1).read_to_end(&mut buf).map_err(|e| e.to_string())?;
        total += buf.len() as u64;
        if buf.len() as u64 > MAX_ENTRY_BYTES || total > MAX_TOTAL_BYTES {
            return Err("package exceeds size limits".into());
        }
        parts.insert(name, buf);
    }
    Ok(parts)
}

fn text_part(parts: &HashMap<String, Vec<u8>>, key: &str) -> Result<String, String> {
    let b = parts.get(key).ok_or_else(|| format!("missing part {key}"))?;
    String::from_utf8(b.clone()).map_err(|_| format!("{key} is not UTF-8"))
}

fn strip_bom(s: &str) -> &str {
    s.trim_start_matches('\u{feff}').trim_start()
}

/// Match `mediaLst` entries to packaged files by MD5 and register them as assets.
fn load_assets(
    parts: &HashMap<String, Vec<u8>>,
    story: Node,
    project: &mut Project,
) -> HashMap<String, ()> {
    let mut by_md5: HashMap<String, &String> = HashMap::new();
    for (k, v) in parts.iter().filter(|(k, _)| k.starts_with("story/media/")) {
        by_md5.insert(hex(&Md5::digest(v)), k);
    }
    let mut known = HashMap::new();
    // Videos and audio are listed as `<video>`/`<audio>` without checksums; match them to the
    // package file of the same size when that is unambiguous.
    let mut by_size: HashMap<usize, Vec<&String>> = HashMap::new();
    for (k, v) in parts.iter().filter(|(k, _)| k.starts_with("story/media/")) {
        by_size.entry(v.len()).or_default().push(k);
    }
    for m in story.descendants().filter(|n| n.has_tag_name("video") || n.has_tag_name("audio")) {
        let (Some(g), Some(bytes)) = (m.attribute("g"), m.attribute("bytes").and_then(|b| b.parse::<usize>().ok())) else { continue };
        match by_size.get(&bytes).map(|v| v.as_slice()) {
            Some([path]) => {
                let data = parts[*path].clone();
                let name = m.attribute("displayName").unwrap_or("media").to_string();
                project.assets.insert(g.to_string(), Asset { name, mime: sniff_mime(&data, path).into(), bytes: data, file: None });
                known.insert(g.to_string(), ());
            }
            _ => project.notes.push(format!(
                "media {:?} could not be matched to a packaged file",
                m.attribute("displayName").unwrap_or(g)
            )),
        }
    }
    // `mediaLst` is nested inside another `mediaLst` in the sample.
    for m in story.descendants().filter(|n| n.has_tag_name("media")) {
        let (Some(g), Some(sum)) = (
            m.attribute("g"),
            child(m, "md5Checksum").and_then(|c| child(c, "stream")).and_then(|s| s.text()),
        ) else {
            continue;
        };
        match by_md5.get(sum) {
            Some(path) => {
                let name = m.attribute("displayName").unwrap_or("media").to_string();
                let mime = match path.rsplit('.').next().unwrap_or("").to_ascii_lowercase().as_str() {
                    "png" => "image/png",
                    "jpg" | "jpeg" => "image/jpeg",
                    "gif" => "image/gif",
                    "svg" => "image/svg+xml",
                    _ => "application/octet-stream",
                };
                project.assets.insert(
                    g.to_string(),
                    Asset { name, mime: mime.into(), bytes: parts[*path].clone(), file: None },
                );
                known.insert(g.to_string(), ());
            }
            None => project.notes.push(format!(
                "media {:?} has no packaged file matching its checksum",
                m.attribute("displayName").unwrap_or(g)
            )),
        }
    }
    known
}

fn convert_shapes(list: Option<Node>, ctx: &mut Ctx, hidden: bool) -> Vec<Object> {
    let mut out = Vec::new();
    let Some(list) = list else { return out };
    for n in list.children().filter(|n| n.is_element()) {
        let tag = n.tag_name().name();
        if tag == "group" {
            // Assumption: child frames are in slide coordinates.
            out.extend(convert_shapes(child(n, "childLst"), ctx, hidden || is_hidden(n)));
            continue;
        }
        // A group lists its members as `<g>id</g>` references; the members themselves are in the
        // slide's flat shape list, so there is nothing to convert here.
        if tag == "g" {
            continue;
        }
        let Some(loc) = child(n, "loc") else {
            ctx.notes.push(format!("<{tag}> has no frame; skipped"));
            continue;
        };
        // Frames can be stored flipped (right < left or bottom < top); normalize.
        let (l0, t0, r0, b0) = (num(loc, "l", 0.0), num(loc, "t", 0.0), num(loc, "r", 0.0), num(loc, "b", 0.0));
        let (l, r) = (l0.min(r0), l0.max(r0));
        let (t, b) = (t0.min(b0), t0.max(b0));
        if tag == "table" {
            out.extend(table_cells(n, l, t, hidden || is_hidden(n), ctx));
            continue;
        }
        // A check box keeps its look in its Normal state's inner shape; the outer one is just the frame.
        let check_src = if tag == "checkBox" { check_shape(n) } else { None };
        let src = check_src.unwrap_or(n);
        let base = base_style(src, ctx);
        let mut links = shape_links(n);
        links.extend(shape_links(src));
        let hlink_triggers = child(n, "trigLst").map(|l| l.children().filter(|t| t.has_tag_name("hlinkTrig")).count()).unwrap_or(0);
        if hlink_triggers > links.len() {
            ctx.notes.push("a link in text that does something other than open a web page or write an email was left as plain text".into());
        }
        let text_doc = child(src, "text").and_then(|t| t.text()).map(|t| parse_text_rich_with(t, &base, ctx.font_scale, &ctx.palette, &links));
        let (text, style, rich) = text_doc.unwrap_or_else(|| (String::new(), base, None));
        // A hotspot's click area: the outline of a freeform one, or the ellipse of an oval one.
        let hit_area = match tag {
            "clkAreaFree" => freeform_outline(n),
            "clkAreaOval" => Some(ellipse_outline()),
            _ => None,
        };
        let kind = match tag {
            "rect" | "roundRect" | "btn" | "textBox" => {
                if tag == "textBox" { Kind::Text } else { Kind::Rect }
            }
            "oval" => Kind::Ellipse,
            "checkBox" => Kind::Text,
            "importedVector" => match parse_path(n) {
                Some(cmds) => Kind::Path(cmds),
                None => {
                    ctx.notes.push("<importedVector> has no readable path; skipped".into());
                    Kind::Unsupported("importedVector".into())
                }
            },
            // Frames are stored with the line's real endpoints, so a flip on one axis means it rises.
            "line" => if (l0 <= r0) == (t0 <= b0) { Kind::Line } else { Kind::LineUp },
            t if t.starts_with("clkArea") => {
                if !matches!(t, "clkAreaRect" | "clkAreaOval" | "clkAreaFree") || (t == "clkAreaFree" && hit_area.is_none()) {
                    ctx.notes.push(format!("<{t}> hotspot uses its bounding box"));
                }
                Kind::Hotspot
            }
            // Preset shapes: the file stores each one's tuning in `<prstGeom>`.
            "rightArrow" | "triangle" | "rightTriangle" | "roundSameSideCorner" | "snipDiagCorner" | "lnCall1" | "multiply" | "checkMark" => {
                match preset_path(n, tag, r - l, b - t) {
                    Some(cmds) => Kind::Path(cmds),
                    None => {
                        ctx.notes.push(format!("<{tag}> drawn as a rectangle"));
                        Kind::Rect
                    }
                }
            }
            // A "character" is a picture from the character library.
            "pic" | "char" => Kind::Image,
            "slider" => Kind::Rect, // replaced by the real slider once the object is read
            "video" => Kind::Video,
            other => {
                ctx.notes.push(format!("<{other}> not supported yet"));
                Kind::Unsupported(other.into())
            }
        };
        let mut asset = n.attribute("assetG").filter(|g| ctx.assets.contains_key(*g)).map(String::from);
        // A character is a vector illustration (EMF) with no checksum of its own; it is drawn from the
        // metafile the slide lists for it.
        if tag == "char" && asset.is_none() { asset = ctx.draw_character(n); }
        if matches!(kind, Kind::Image | Kind::Video) && asset.is_none() {
            if tag == "char" {
                let who = child(n, "illConfig").and_then(|c| c.attribute("characterName")).unwrap_or("");
                ctx.notes.push(format!("character {who:?} is a vector illustration (EMF) that could not be drawn"));
            } else {
                ctx.notes.push(format!("{:?} has no resolvable media", n.attribute("name").unwrap_or("")));
            }
        }
        let crop = if matches!(kind, Kind::Image) { asset.as_deref().and_then(|a| ctx.sizes.get(a).copied().or_else(|| ctx.drawn.iter().find(|d| d.0 == a).map(|d| d.2))).and_then(|size| crop_of(n, size)) } else { None };
        let obj_id: String = n.attribute("g").unwrap_or_default().into();
        parse_triggers(n, Owner::Object(obj_id.clone()), ctx);
        let initial_state = n.attribute("initState").unwrap_or_default().to_string();
        // The source app's built-in "Hidden" state keeps an object off the slide until something sets another state.
        let starts_hidden = initial_state == "Hidden";
        let is_slider = tag == "slider";
        // A slider's states only describe its thumb, which the Slider itself holds.
        let is_check = tag == "checkBox";
        let check = is_check.then(|| check_of(src, ctx));
        let states = if is_slider || is_check { vec![] } else { parse_states(n, &text_fallback(&text, &style, &rich), ctx) };
        // Shapes with states draw from the initial state (the shape's own fill is empty).
        let shown = states.iter().find(|s| s.name == initial_state).or(states.first());
        let (text, style, rich, fill, stroke, shadow) = match shown {
            Some(st) => (st.text.clone(), st.style.clone(), st.rich.clone(), st.fill, st.stroke, st.shadow),
            None if is_check => (text, style, rich, None, None, None),
            None => (text, style, rich, fill_of(n, ctx), stroke_of(n, ctx), shadow_of(n, ctx)),
        };
        // A picture shows the picture of its initial state.
        let asset = if matches!(kind, Kind::Image) { shown.and_then(|st| st.asset.clone()).or(asset) } else { asset };
        let dial = dial_of(n, ctx);
        let kind = if is_slider { Kind::Slider(slider_of(n, ctx)) } else { kind };
        out.push(Object {
            id: obj_id,
            name: n.attribute("typeName").filter(|s| !s.is_empty()).or(n.attribute("name")).unwrap_or(tag).into(),
            kind,
            x: l,
            y: t,
            w: r - l,
            h: b - t,
            fill,
            stroke,
            shadow, glow: None, soft_edge: 0.0, reflection: None,
            hidden: hidden || starts_hidden || is_hidden(n) || n.attribute("parentG").is_some_and(|g| ctx.hidden_groups.contains(g)),
            margins: if is_check { child(src, "margins").map(|m| [num(m, "l", 32.0), num(m, "t", 0.0), num(m, "r", 0.0), num(m, "b", 0.0)]).unwrap_or([32.0, 0.0, 0.0, 0.0]) } else { child(n, "textMargin")
                .map(|m| [num(m, "l", 0.0), num(m, "t", 0.0), num(m, "r", 0.0), num(m, "b", 0.0)])
                .unwrap_or([0.0; 4]) },
            radius: corner_radius(n, tag, w_h(l, r, t, b)),
            alt: child(n, "alt").and_then(|a| a.text()).map(|a| a.trim()).filter(|a| !a.is_empty()).map(String::from)
                .or_else(|| (tag == "char").then(|| child(n, "illConfig").map(|c| ["characterName", "expression"].iter().filter_map(|a| c.attribute(*a)).filter(|a| !a.is_empty()).collect::<Vec<_>>().join(", ")).unwrap_or_default()))
                .unwrap_or_default(),
            text_rotation: match n.attribute("textDir") { Some("rot270") => 270, Some("rot90") => 90, _ => 0 },
            // A text style that is a heading level in the source app (`textStyleKey="H1"`) is a heading here too.
            role: match n.descendants().find(|d| d.has_tag_name("defaults")).and_then(|d| d.attribute("textStyleKey")) {
                Some("H1") => TextRole::H1, Some("H2") => TextRole::H2, Some("H3") => TextRole::H3, Some("H4") => TextRole::H4,
                _ => TextRole::Normal,
            },
            marker: None, equation: None,
            captions: None,
            rich,
            entrance: effect_of(n, "entr", ctx),
            exit: effect_of(n, "exit", ctx),
            timing: timing_of(n),
            text,
            style,
            asset,
            states,
            initial_state,
            placeholder: None, gradient: None, rotation: rotation_of(n), line_spacing: 0.0, group: None, table: None, table_span: None, table_borders: None, emphasis: None, motion: None, click_steps: Default::default(), decorative: false, crop, dial, check, hit_area, mask: None, dash: dash_of(n), media: media_of(n), hide_on_layers: hidden_on_layers(n), button_set: n.attribute("grpId").filter(|g| !g.trim().is_empty()).map(String::from), poster: (tag == "video").then(|| n.attribute("thumbG")).flatten().filter(|g| ctx.assets.contains_key(*g)).map(String::from),
        });
    }
    out
}

/// A source table becomes a BrainBend Studio table: one rectangle per cell, all in one group, each knowing its
/// row and column (merged cells carry their span and the cells they cover have no object). Cell fill, the
/// four borders, margins and text carry over, so it edits like a table made in BrainBend Studio.
fn table_cells(n: Node, left: f32, top: f32, hidden: bool, ctx: &mut Ctx) -> Vec<Object> {
    let Some(tbl) = n.children().find(|c| c.has_tag_name("table")) else { return vec![] };
    let cols: Vec<f32> = child(tbl, "columns").map(|c| c.children().filter(|x| x.has_tag_name("tableColumn")).map(|x| num(x, "width", 100.0)).collect()).unwrap_or_default();
    let Some(rows_n) = child(tbl, "rows") else { return vec![] };
    let heights: Vec<f32> = rows_n.children().filter(|x| x.has_tag_name("tableRow")).map(|x| num(x, "height", 40.0)).collect();
    let base_id: String = n.attribute("g").unwrap_or("table").into();
    let base = base_style(n, ctx);
    let mut out = vec![];
    let mut y = top;
    for (ri, row) in rows_n.children().filter(|x| x.has_tag_name("tableRow")).enumerate() {
        let mut x = left;
        let cells: Vec<Node> = child(row, "cells").map(|c| c.children().filter(|x| x.has_tag_name("tableCell")).collect()).unwrap_or_default();
        for (ci, cell) in cells.iter().enumerate() {
            let w = cols.get(ci).copied().unwrap_or(100.0);
            let covered = cell.attribute("verticalMerge") == Some("true") || cell.attribute("horizontalMerge") == Some("true");
            if !covered {
                let span_w: f32 = (ci..ci + num(*cell, "columnSpan", 1.0) as usize).filter_map(|k| cols.get(k)).sum();
                let span_h: f32 = (ri..ri + num(*cell, "rowSpan", 1.0) as usize).filter_map(|k| heights.get(k)).sum();
                let (text, style, rich) = cell.attribute("text").map(|t| parse_text_rich(t, &base, ctx.font_scale, &ctx.palette)).unwrap_or_else(|| (String::new(), base.clone(), None));
                let sty = child(*cell, "style");
                let fill = sty.and_then(|s| child(s, "background")).and_then(|b| child(b, "fillColor")).and_then(|c| resolve_clr(c, ctx));
                // Each side has its own border. A dash type other than solid or dashed (the source app calls "no
                // line" "Custom") draws nothing.
                let mut side = |tag: &str| -> Option<Stroke> {
                    let b = sty.and_then(|s| child(s, tag))?;
                    if b.attribute("dashType") == Some("Custom") { return None; }
                    let c = resolve_clr(child(b, "themeColor")?, ctx)?;
                    Some(Stroke { color: c, width: num(b, "weight", 1.0).clamp(0.5, 8.0) })
                };
                let sides = [side("leftBorder"), side("rightBorder"), side("topBorder"), side("bottomBorder")];
                let all_alike = sides.iter().all(|s| *s == sides[0]);
                let margins = sty.and_then(|s| child(s, "margins")).map(|m| [num(m, "l", 6.0), num(m, "t", 2.0), num(m, "r", 6.0), num(m, "b", 2.0)]).unwrap_or([6.0, 2.0, 6.0, 2.0]);
                let span = (num(*cell, "rowSpan", 1.0).max(1.0) as u16, num(*cell, "columnSpan", 1.0).max(1.0) as u16);
                out.push(Object {
                    id: format!("{base_id}-r{ri}c{ci}"),
                    name: format!("Cell {},{}", ri + 1, ci + 1),
                    kind: Kind::Rect,
                    x: x, y, w: if span_w > 0.0 { span_w } else { w }, h: if span_h > 0.0 { span_h } else { heights.get(ri).copied().unwrap_or(40.0) },
                    fill, stroke: if all_alike { sides[0] } else { None }, shadow: None, glow: None, soft_edge: 0.0, reflection: None, hidden, margins, radius: 0.0,
                    alt: String::new(), text_rotation: 0, role: TextRole::Normal, marker: None, equation: None, captions: None, rich, entrance: None, exit: None, timing: None,
                    text, style, asset: None, states: vec![], initial_state: String::new(), placeholder: None, gradient: None, rotation: 0.0, line_spacing: 0.0, group: Some(base_id.clone()), table: Some((ri as u16, ci as u16)), table_span: (span != (1, 1)).then_some(span), table_borders: (!all_alike).then_some(sides), emphasis: None, motion: None, click_steps: Default::default(), decorative: false, crop: None, dial: None, check: None, hit_area: None, mask: None, dash: LineDash::Solid, media: Default::default(), hide_on_layers: vec![], button_set: None, poster: None,
                });
            }
            x += w;
        }
        y += heights.get(ri).copied().unwrap_or(40.0);
    }
    out
}

fn w_h(l: f32, r: f32, t: f32, b: f32) -> f32 {
    (r - l).min(b - t)
}

const NO_GUID: &str = "00000000-0000-0000-0000-000000000000";

/// The variable a slider or dial sets, if it is linked to one.
fn bound_variable(n: Node) -> Option<String> {
    let g = if n.tag_name().name() == "slider" {
        n.attribute("varG")
    } else {
        child(n, "dialData").filter(|d| d.attribute("dialType") == Some("Dial")).and_then(|d| d.attribute("varG"))
    };
    g.filter(|g| !g.is_empty() && *g != NO_GUID).map(String::from)
}

/// A freeform hotspot's outline: `<pts>` holds points as fractions of the frame, one for every
/// pixel the author's mouse crossed, so the line is thinned to the few that matter.
fn freeform_outline(n: Node) -> Option<Vec<(f32, f32)>> {
    let pts: Vec<(f32, f32)> = child(n, "pts")?.children().filter(|p| p.has_tag_name("pt")).map(|p| (num(p, "x", 0.0), num(p, "y", 0.0))).collect();
    let pts = simplify(&pts, 0.004);
    (pts.len() >= 3).then_some(pts)
}

fn ellipse_outline() -> Vec<(f32, f32)> {
    (0..48).map(|i| { let a = i as f32 / 48.0 * std::f32::consts::TAU; (0.5 + 0.5 * a.cos(), 0.5 + 0.5 * a.sin()) }).collect()
}

/// Ramer-Douglas-Peucker: drop the points that lie within `tol` of the line between those kept.
fn simplify(pts: &[(f32, f32)], tol: f32) -> Vec<(f32, f32)> {
    if pts.len() < 3 { return pts.to_vec(); }
    let (a, b) = (pts[0], pts[pts.len() - 1]);
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len = (dx * dx + dy * dy).sqrt();
    let dist = |p: (f32, f32)| if len < 1e-9 { ((p.0 - a.0).powi(2) + (p.1 - a.1).powi(2)).sqrt() } else { ((p.0 - a.0) * dy - (p.1 - a.1) * dx).abs() / len };
    let (at, far) = pts.iter().enumerate().skip(1).take(pts.len() - 2).map(|(i, p)| (i, dist(*p))).fold((0, 0.0), |m, c| if c.1 > m.1 { c } else { m });
    if far <= tol { return vec![a, b]; }
    let mut left = simplify(&pts[..=at], tol);
    left.pop();
    left.extend(simplify(&pts[at..], tol));
    left
}

/// The shape inside a check box's initial (else Normal) state; it holds the label and the look.
fn check_shape<'a, 'i>(n: Node<'a, 'i>) -> Option<Node<'a, 'i>> {
    let list = child(n, "stateLst")?;
    let wanted = n.attribute("initState").filter(|s| !s.is_empty()).unwrap_or("Normal");
    let state = |name: &str| list.children().find(|c| c.is_element() && c.attribute("name") == Some(name));
    let st = state(wanted).or_else(|| state("Normal"))?;
    child(st, "shapeLst")?.children().find(|c| c.is_element())
}

/// The box and tick of a check box. The file stores only the box's colours (`btnBg`); the source app
/// draws the box itself, in the space the text margin leaves at the left.
fn check_of(src: Node, ctx: &mut Ctx) -> Check {
    let mut c = Check::new();
    // The Selected state's shape has no colours of its own, so the Normal state's are used.
    if let Some(bg) = child(src, "btnBg") {
        if let Some(f) = child(bg, "solidFill").and_then(|f| child(f, "clr")).and_then(|f| resolve_clr(f, ctx)) { c.box_fill = f; }
        if let Some(l) = child(bg, "solidLine").and_then(|f| child(f, "clr")).and_then(|f| resolve_clr(f, ctx)) { c.box_line = Some(l); }
    }
    c.tick = base_style(src, ctx).color;
    c
}

/// An object turned into a dial (`<dialData dialType="Dial">`): the range, step and sweep.
fn dial_of(n: Node, ctx: &mut Ctx) -> Option<Dial> {
    let data = child(n, "dialData").filter(|d| d.attribute("dialType") == Some("Dial"))?;
    let Some(variable) = bound_variable(n) else {
        ctx.notes.push("a dial that is not linked to a variable turns but sets nothing".into());
        return None;
    };
    let values = child(data, "values");
    let at = |a: &str, d: f32| values.map(|v| num(v, a, d)).unwrap_or(d);
    let pivot = child(data, "rotationPoint").map(|p| (num(p, "xOffset", 0.0), num(p, "yOffset", 0.0))).unwrap_or((0.0, 0.0));
    Some(Dial { variable, min: at("start", 0.0), max: at("end", 10.0), step: num(data, "increment", 1.0).max(0.0), sweep: num(data, "rotation", 180.0).clamp(1.0, 360.0), pivot })
}

/// A slider: its range, step and variable, and a thumb picture or colour read from its Normal state.
/// `thumbH` (the thumb's height as a fraction of the frame) is read as written; a slider that does not
/// say gets a thumb a little shorter than its frame.
fn slider_of(n: Node, ctx: &mut Ctx) -> Slider {
    let mut slider = Slider::new();
    slider.variable = bound_variable(n).unwrap_or_default();
    if let Some(t) = child(n, "track") {
        (slider.min, slider.max) = (num(t, "start", 0.0), num(t, "end", 10.0));
        // A track drawn with a line or a fill is visible; the sample's sliders draw none (the thumb is a picture).
        let solid = |name: &str| child(t, "bg").and_then(|bg| child(bg, name)).and_then(|f| child(f, "clr"));
        slider.track = solid("solidFill").or_else(|| solid("solidLine")).and_then(|c| resolve_clr(c, ctx));
    } else {
        slider.track = None;
    }
    slider.step = child(n, "ticks").map(|t| num(t, "step", 1.0)).filter(|s| *s > 0.0).unwrap_or(1.0);
    let normal = child(n, "stateLst").and_then(|l| l.children().find(|c| c.has_tag_name("state") && c.attribute("name") == Some("Normal")));
    if let Some(thumb) = normal.and_then(|st| child(st, "shapeLst")).and_then(|l| l.children().find(|c| c.is_element())) {
        slider.thumb = child(thumb, "bG").and_then(|bg| child(bg, "picFill")).and_then(|p| p.attribute("assetG")).filter(|g| ctx.assets.contains_key(*g)).map(String::from);
        if let Some(c) = fill_of(thumb, ctx) { slider.thumb_color = c; }
        if let Some(h) = child(thumb, "prstGeom").and_then(|g| child(g, "slider")).and_then(|s| s.attribute("thumbH")).and_then(|v| v.parse::<f32>().ok()) {
            slider.thumb_height = h.clamp(0.05, 1.0);
        }
    }
    slider
}

/// Pixel size of a PNG or JPEG, read from its header.
fn image_size(bytes: &[u8]) -> Option<(f32, f32)> {
    let (w, h) = image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format().ok()?.into_dimensions().ok()?;
    Some((w as f32, h as f32))
}

/// A picture's crop as the fractions hidden from each edge. The file stores the visible part as a
/// rectangle in source pixels (`<crop><rect l t r b>`); its size times `scaleX`/`scaleY` is the frame.
fn crop_of(n: Node, (w, h): (f32, f32)) -> Option<[f32; 4]> {
    let rect = child(child(n, "crop")?, "rect")?;
    let (l, t, r, b) = (num(rect, "l", 0.0), num(rect, "t", 0.0), num(rect, "r", 0.0), num(rect, "b", 0.0));
    if r - l < 1.0 || b - t < 1.0 || w < 1.0 || h < 1.0 { return None; }
    let crop = [l / w, t / h, 1.0 - r / w, 1.0 - b / h];
    crop.iter().any(|v| v.abs() > 0.002).then_some(crop)
}

/// The object's turn in degrees, clockwise about the centre of its (unrotated) frame; `-1` means none.
fn rotation_of(n: Node) -> f32 {
    match n.attribute("rot").and_then(|v| v.parse::<f32>().ok()) {
        Some(r) if r >= 0.0 => r % 360.0,
        _ => 0.0,
    }
}

/// The outline of a preset shape from its `<prstGeom>` tuning values (fractions of the frame).
/// `rightArrow`'s `cx`/`cy` are read as head length (a multiple of the shorter side) and shaft
/// thickness; that reading comes from the default values, not from a rendered comparison.
fn preset_path(n: Node, tag: &str, w: f32, h: f32) -> Option<Vec<PathCmd>> {
    use brainbend_doc::shapes::{check_mark, cut_rect, line_callout, multiply, poly, preset, Preset};
    let g = child(n, "prstGeom").and_then(|g| g.children().find(|c| c.is_element() && c.tag_name().name() == tag))?;
    let at = |a: &str, d: f32| num(g, a, d);
    let ss = w.min(h).max(1.0);
    Some(match tag {
        "triangle" => poly(&[(at("v", 0.5), 0.0), (1.0, 1.0), (0.0, 1.0)]),
        "rightTriangle" => poly(&[(0.0, 0.0), (0.0, 1.0), (1.0, 1.0)]),
        "rightArrow" => match preset("rightArrow", w, h, &|name, d| match name { "adj1" => at("cy", d / 100000.0) * 100000.0, "adj2" => at("cx", d / 100000.0) * 100000.0, _ => d }) {
            Preset::Path(p) => p,
            _ => return None,
        },
        "roundSameSideCorner" | "snipDiagCorner" => {
            let round = tag == "roundSameSideCorner";
            let cut = |v: f32| { let v = v.clamp(0.0, 0.5); (round, ss * v / w.max(1.0), ss * v / h.max(1.0)) };
            let (l, r) = (at("l", 0.0), at("r", 0.1667));
            // Same-side: `r` (rounded by default) is the top pair of corners and `l` the bottom pair, as in
            // PowerPoint's round2SameRect (checked against a header bar turned 90 degrees, rounded at its
            // right end by the source app); diagonal: `l` is the top-left/bottom-right pair and `r` the other.
            cut_rect(if round { [cut(r), cut(r), cut(l), cut(l)] } else { [cut(l), cut(r), cut(l), cut(r)] })
        }
        "lnCall1" => {
            let tip = child(g, "v").map(|v| (num(v, "x", 0.0), num(v, "y", 0.0)))?;
            let from = child(g, "pts").and_then(|p| child(p, "pt")).map(|v| (num(v, "x", 0.0), num(v, "y", 0.0)))?;
            line_callout(from, tip)
        }
        "multiply" => multiply(at("cy", 0.1837)),
        "checkMark" => check_mark(),
        _ => return None,
    })
}

/// `roundRect` stores its radius as a fraction of the shorter side (`rndC@rTL`, `usePc`).
/// Buttons carry no radius in the file, only a style: the default style has the source app's border radius of 5
/// and the flat one 0.
fn corner_radius(n: Node, tag: &str, short_side: f32) -> f32 {
    match tag {
        "roundRect" => child(n, "rndC").map(|c| num(c, "rTL", 0.0) * short_side).unwrap_or(0.0),
        "btn" if n.attribute("style") == Some("rectFlat") => 0.0,
        "btn" => 5.0,
        _ => 0.0,
    }
}

type TextParts = (String, TextStyle, Option<Vec<Paragraph>>);

/// The first `tmCtx` of a node's `tmCtxLst` (slides, layers and shapes all have one).
fn first_tm_ctx<'a, 'b>(n: Node<'a, 'b>) -> Option<Node<'a, 'b>> {
    child(n, "tmCtxLst").and_then(|l| l.children().find(|c| c.has_tag_name("tmCtx")))
}

/// Timeline length in ms for a slide or layer (`dur` of its own context).
fn timeline_len(n: Node) -> Option<u32> {
    first_tm_ctx(n).and_then(|c| c.attribute("dur")).and_then(|d| d.parse::<f64>().ok()).map(|d| d.max(0.0) as u32).filter(|d| *d > 0)
}

/// When a shape is on screen: `start`/`dur`, with `untilEnd="true"` meaning until the timeline ends.
fn timing_of(n: Node) -> Option<Timing> {
    let c = first_tm_ctx(n)?;
    let start = num(c, "start", 0.0).max(0.0) as u32;
    let until_end = c.attribute("untilEnd") == Some("true");
    let dur = num(c, "dur", 0.0).max(0.0) as u32;
    if start == 0 && until_end {
        return None;
    }
    Some(Timing { start_ms: start, dur_ms: if until_end { None } else { Some(dur) } })
}

/// ISO-8601 duration like `PT1.5S` or `PT1M2S` to milliseconds.
fn iso_ms(s: &str) -> Option<u32> {
    let rest = s.strip_prefix("PT")?;
    let mut total = 0.0f64;
    let mut num = String::new();
    for ch in rest.chars() {
        match ch {
            'H' => { total += num.parse::<f64>().ok()? * 3_600_000.0; num.clear(); }
            'M' => { total += num.parse::<f64>().ok()? * 60_000.0; num.clear(); }
            'S' => { total += num.parse::<f64>().ok()? * 1000.0; num.clear(); }
            c => num.push(c),
        }
    }
    Some(total.round() as u32)
}

/// `animEffect/entr` or `animEffect/exit`: fade, fly (with direction), wipe, zoom, spin, wheel and split map
/// across; other effects (swivel, blinds, ...) become a fade and are reported.
fn effect_of(n: Node, which: &str, ctx: &mut Ctx) -> Option<Entrance> {
    let fx = child(child(n, "animEffect")?, which)?.children().find(|c| c.is_element())?;
    let kind = match fx.tag_name().name() {
        "fade" => EffectKind::Fade,
        "fly" => match fx.attribute("dir") {
            Some("l") | Some("tl") | Some("bl") => EffectKind::SlideLeft,
            Some("r") | Some("tr") | Some("br") => EffectKind::SlideRight,
            Some("b") => EffectKind::SlideDown,
            _ => EffectKind::SlideUp,
        },
        "wipe" => EffectKind::Wipe,
        "zoom" | "growShrink" => EffectKind::Zoom,
        "spin" => {
            if fx.attribute("sd") == Some("counter") {
                ctx.notes.push(format!("{which} effect <spin> turns the other way in the source app; it turns clockwise here"));
            }
            match fx.attribute("amt") {
                Some("qua") => EffectKind::SpinQuarter,
                Some("half") => EffectKind::SpinHalf,
                Some("one") | None => EffectKind::SpinFull,
                Some(_) => {
                    ctx.notes.push(format!("{which} effect <spin> of more than one turn is approximated as one turn"));
                    EffectKind::SpinFull
                }
            }
        }
        "wheel" => {
            if !matches!(fx.attribute("amt"), Some("one") | None) {
                ctx.notes.push(format!("{which} effect <wheel> with several spokes is approximated as one spoke"));
            }
            EffectKind::Wheel
        }
        "split" => match fx.attribute("sd") {
            Some("vertIn") => EffectKind::SplitVerticalIn,
            Some("horzIn") => EffectKind::SplitHorizontalIn,
            Some("horzOut") => EffectKind::SplitHorizontalOut,
            _ => EffectKind::SplitVerticalOut,
        },
        other => {
            ctx.notes.push(format!("{which} effect <{other}> approximated as a fade"));
            EffectKind::Fade
        }
    };
    Some(Entrance { kind, delay_ms: 0, duration_ms: fx.attribute("dur").and_then(iso_ms).unwrap_or(500).clamp(50, 10_000) })
}

fn text_fallback(text: &str, style: &TextStyle, rich: &Option<Vec<Paragraph>>) -> TextParts {
    (text.to_string(), style.clone(), rich.clone())
}

/// Each `stateLst/state` holds a one-shape `shapeLst` carrying that state's fill and text.
/// States without their own text inherit the object's.
fn parse_states(n: Node, fallback: &TextParts, ctx: &mut Ctx) -> Vec<ObjectState> {
    let mut out = Vec::new();
    let Some(list) = child(n, "stateLst") else { return out };
    // The object's own text can be empty because it lives in the first (Normal) state; the built-in Hover, Down,
    // Visited and Disabled states of a button list no text of their own and show that one.
    let mut inherit = fallback.clone();
    for st in list.children().filter(|c| c.has_tag_name("state")) {
        let Some(shape) = child(st, "shapeLst").and_then(|l| l.children().find(|c| c.is_element())) else { continue };
        let base = base_style(shape, ctx);
        // A state whose text comes out empty (a button's Hover state) is the object's own text, not a blanked one.
        let (text, style, rich) = match child(shape, "text").and_then(|t| t.text()).map(|t| parse_text_rich(t, &base, ctx.font_scale, &ctx.palette)) {
            Some(parsed) if !parsed.0.trim().is_empty() || inherit.0.trim().is_empty() => parsed,
            _ => inherit.clone(),
        };
        if inherit.0.trim().is_empty() { inherit = (text.clone(), style.clone(), rich.clone()); }
        // A state can hold more shapes than the one it restyles: a button's Normal state is its rectangle plus the text
        // boxes that label it ("Button Label", a short description). Their text stacks under the first one's.
        let extra: Vec<TextParts> = child(st, "shapeLst")
            .map(|l| l.children().filter(|c| c.is_element()).skip(1).filter(|c| c.has_tag_name("textBox")).collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|tb| child(tb, "text").and_then(|t| t.text()).map(|t| parse_text_rich(t, &base_style(tb, ctx), ctx.font_scale, &ctx.palette)))
            .filter(|p| !p.0.trim().is_empty())
            .collect();
        let (text, style, rich) = if extra.is_empty() { (text, style, rich) } else { stack_text((text, style, rich), extra) };
        if inherit.0.trim().is_empty() && !text.trim().is_empty() { inherit = (text.clone(), style.clone(), rich.clone()); }
        // A picture's states can each show a different picture, at that picture's own size.
        let asset = (shape.tag_name().name() == "pic").then(|| shape.attribute("assetG")).flatten().filter(|g| ctx.assets.contains_key(*g)).map(String::from);
        let size = asset.as_ref().and_then(|_| child(shape, "loc")).map(|l| ((num(l, "r", 0.0) - num(l, "l", 0.0)).abs(), (num(l, "b", 0.0) - num(l, "t", 0.0)).abs())).filter(|&(w, h)| w >= 1.0 && h >= 1.0);
        // A state only lists what it changes: a fill or line it does not mention is the object's own.
        let says = |tags: &[&str]| child(shape, "bG").is_some_and(|bg| tags.iter().any(|t| child(bg, t).is_some()));
        let fill = if says(&["solidFill", "noFill", "gradFill", "picFill", "pattFill"]) { fill_of(shape, ctx) } else { fill_of(n, ctx) };
        // An empty `<solidFill><clr/></solidFill>` (a button's Visited and Disabled states) sets no colour: it keeps the first state's.
        let fill = match (fill, out.first()) {
            (None, Some(first)) if !says(&["noFill"]) => first.fill,
            (f, _) => f,
        };
        let stroke = if says(&["solidLine", "noLine"]) { stroke_of(shape, ctx) } else { stroke_of(n, ctx) };
        out.push(ObjectState {
            name: st.attribute("name").unwrap_or("").into(),
            rich,
            fill,
            stroke,
            shadow: shadow_of(shape, ctx),
            text,
            style,
            asset,
            size,
            shine: shine_of(shape),
            gradient: None,
        });
    }
    out
}

/// The glossy overlay a state's shape asks for: `<bG shine="true">` with a linear `<gradOvrlyFill blendMode="ovrly" alpha reverse>`.
fn shine_of(shape: Node) -> Option<Shine> {
    let bg = child(shape, "bG").filter(|b| b.attribute("shine") == Some("true"))?;
    let g = child(bg, "gradOvrlyFill").filter(|g| g.attribute("blendMode") == Some("ovrly") && g.attribute("overlayFillType") == Some("Gradient"))?;
    let alpha = g.attribute("alpha")?.parse::<f32>().ok().filter(|a| *a > 0.0)?;
    Some(Shine { alpha: alpha.min(1.0), reverse: g.attribute("reverse") == Some("true") })
}

/// Stack several text boxes' text into one object text: the first part's style is the object's, and the
/// other parts' runs record how they differ from it.
fn stack_text(first: TextParts, more: Vec<TextParts>) -> TextParts {
    let base = first.1.clone();
    let paragraphs_of = |(text, style, rich): TextParts| -> Vec<Paragraph> {
        let rich = rich.unwrap_or_else(|| text.split('\n').map(|l| Paragraph { runs: vec![Run { text: l.into(), ..Default::default() }], ..Default::default() }).collect());
        rich.into_iter()
            .map(|p| Paragraph {
                runs: p
                    .runs
                    .into_iter()
                    .map(|r| {
                        let (bold, italic, underline) = (r.bold.unwrap_or(style.bold), r.italic.unwrap_or(style.italic), r.underline.unwrap_or(style.underline));
                        let (color, size) = (r.color.unwrap_or(style.color), r.size.unwrap_or(style.size));
                        let family = r.font_family.clone().unwrap_or_else(|| style.font_family.clone());
                        Run {
                            text: r.text,
                            bold: (bold != base.bold).then_some(bold),
                            italic: (italic != base.italic).then_some(italic),
                            underline: (underline != base.underline).then_some(underline),
                            color: (color != base.color).then_some(color),
                            size: (size != base.size).then_some(size),
                            font_family: (family != base.font_family).then_some(family),
                            link: r.link,
                            ..Default::default()
                        }
                    })
                    .collect(), ..Default::default() })
            .collect()
    };
    let mut text = first.0.clone();
    let mut paras = if first.0.is_empty() { vec![] } else { paragraphs_of(first.clone()) };
    for part in more {
        if !text.is_empty() { text.push('\n'); }
        text.push_str(&part.0);
        paras.extend(paragraphs_of(part));
    }
    (text, base, Some(paras))
}

fn parse_variables(story: Node) -> Vec<Variable> {
    story
        .descendants()
        .filter(|n| n.has_tag_name("var") && n.attribute("g").is_some())
        .filter_map(|n| {
            let raw = n.attribute("val").unwrap_or("");
            let initial = match n.attribute("dataType")? {
                "bool" => Value::Bool(raw.eq_ignore_ascii_case("true")),
                "num" => Value::Num(raw.parse().unwrap_or(0.0)),
                _ => Value::Text(raw.into()),
            };
            Some(Variable {
                id: n.attribute("g")?.into(),
                name: n.attribute("name")?.into(),
                initial,
                built_in: n.attribute("type") == Some("builtIn"),
            })
        })
        .collect()
}

fn read_value(kind: &Value, bool_v: Option<&str>, num: Option<&str>, text: Option<&str>) -> Value {
    match kind {
        Value::Bool(_) => Value::Bool(bool_v.is_some_and(|b| b.eq_ignore_ascii_case("true"))),
        Value::Num(_) => Value::Num(num.and_then(|v| v.parse().ok()).unwrap_or(0.0)),
        Value::Text(_) => Value::Text(text.unwrap_or("").into()),
    }
}

/// Convert the owner's `trigLst` into triggers (only the observed subset; the rest becomes
/// `Action::Unsupported` so it is reported rather than lost).
fn parse_triggers(owner_node: Node, owner: Owner, ctx: &mut Ctx) {
    let Some(list) = child(owner_node, "trigLst") else { return };
    for t in list.children().filter(|c| c.has_tag_name("trig")) {
        let Some(d) = child(t, "data") else { continue };
        if d.attribute("enabled") == Some("false") {
            continue;
        }
        let other = child(d, "other");
        let oa = |a: &str| other.and_then(|o| o.attribute(a));
        // A state-change trigger watches a list of objects: one event for each.
        let mut more_events: Vec<Event> = Vec::new();
        let event = match d.attribute("event").unwrap_or("") {
            "OnStateChange" => {
                let shape = child(d, "shape");
                let watched: Vec<String> = shape.and_then(|s| child(s, "stateLst")).map(|l| l.children().filter(|g| g.has_tag_name("g")).filter_map(|g| g.text()).map(|g| g.trim().to_string()).collect()).unwrap_or_default();
                let state = shape.and_then(|s| s.attribute("stateName")).unwrap_or("");
                // "any of these are Selected" is exact; "all of" and "are not" are approximated.
                let exact = shape.is_some_and(|s| s.attribute("stateInclude") == Some("any") && s.attribute("statesNot") == Some("are"));
                if !exact {
                    ctx.notes.push("a state-change trigger on \"all\" objects, or on states they are not in, fires when any of them enters or leaves a state".into());
                }
                let mut events = watched.into_iter().map(|g| Event::StateChanged(g, if exact { state.to_string() } else { String::new() }));
                let Some(first) = events.next() else {
                    ctx.notes.push("a state-change trigger that watches no objects is skipped".into());
                    continue;
                };
                more_events = events.collect();
                first
            }
            "OnStart" => match owner {
                Owner::Layer(_) => Event::LayerStart,
                _ => Event::SlideStart,
            },
            // A slide's player triggers ("when the user clicks Previous") are stored as an OnClick on the slide with a
            // navigation intent; they are not clicks on the slide.
            "OnClick" if matches!(owner, Owner::Slide) && d.attribute("NavigationIntent") == Some("Previous") => Event::PreviousButton,
            "OnClick" if matches!(owner, Owner::Slide) && d.attribute("NavigationIntent") == Some("Next") => Event::NextButton,
            "OnClick" => Event::Click,
            "OnEnd" if matches!(owner, Owner::Slide) => Event::SlideEnd,
            "OnEnd" if matches!(owner, Owner::Object(_)) && matches!(owner_node.tag_name().name(), "video" | "audio") => Event::MediaEnded,
            // Moving a slider or turning a dial is its variable changing.
            "OnSliderMoves" | "OnDialTurns" => match bound_variable(owner_node) {
                Some(var) => Event::VariableChanged(var),
                None => {
                    ctx.notes.push("a slider or dial trigger on a control that is not linked to a variable is skipped".into());
                    continue;
                }
            },
            "OnTimelineEvent" => {
                let ms = oa("time").and_then(iso_ms).unwrap_or(0);
                // On a video or audio object the time is a cue point in the media; elsewhere it is on the slide's or layer's timeline.
                if matches!(owner, Owner::Object(_)) && matches!(owner_node.tag_name().name(), "video" | "audio") { Event::MediaReaches(ms) } else { Event::TimelineReaches(ms) }
            }
            "OnHover" | "OnMouseHover" => Event::Hover,
            "OnNextButtonClick" => Event::NextButton,
            "OnDoubleClick" => Event::DoubleClick,
            "OnVariableValueChange" => Event::VariableChanged(oa("varChangeG").unwrap_or("").into()),
            e => {
                ctx.notes.push(format!("trigger event {e} not supported"));
                continue;
            }
        };
        let layer_of = |a: &str| child(d, "sldLayer").and_then(|l| l.attribute(a)).unwrap_or("").to_string();
        // `actSubType` (spec / next / me / prev) does not decide what an action does: the targets
        // live in the child elements, so most actions are read the same way whatever it says.
        let act = d.attribute("action").unwrap_or("");
        let sub = d.attribute("actSubType").unwrap_or("");
        let slide_jump = child(d, "slide").and_then(|s| s.attribute("jumpG")).filter(|g| !g.is_empty());
        let owner_id = if let Owner::Object(id) = &owner { id.clone() } else { String::new() };
        let action = match act {
            "showSubSlide" if !layer_of("showG").is_empty() => Action::ShowLayer(layer_of("showG")),
            // "This layer" is the layer the trigger sits on. Its `hideG` is stale when the trigger was copied from another
            // layer ("Next Layer" buttons), so it is ignored.
            "hideSubSlide" => Action::HideLayer(if sub == "me" || layer_of("hideG").is_empty() { ctx.layer.clone() } else { layer_of("hideG") }),
            "jumpToSlide" if sub == "prev" => Action::PreviousSlide,
            "jumpToSlide" => match slide_jump {
                Some(g) => Action::JumpToSlide(g.into()),
                None if sub == "next" => Action::NextSlide,
                None => Action::JumpToSlide(String::new()),
            },
            "adjustVar" if oa("useVar") == Some("true") => Action::Unsupported("adjustVar with another variable as the value".into()),
            "adjustVar" => {
                let var = oa("varG").unwrap_or("").to_string();
                let op = oa("op").unwrap_or("ass");
                match ctx.var_types.get(&var) {
                    None => Action::Unsupported(format!("adjustVar on unknown variable {var}")),
                    Some(kind) => {
                        let value = read_value(kind, oa("boolVal"), oa("dblVal"), oa("varVal1"));
                        match op {
                            "ass" => Action::SetVariable { value, variable: var },
                            "add" => Action::AdjustVariable { variable: var, op: AdjustOp::Add, value },
                            "minus" | "sub" => Action::AdjustVariable { variable: var, op: AdjustOp::Subtract, value },
                            o => Action::Unsupported(format!("adjustVar operation {o}")),
                        }
                    }
                }
            }
            "changeShapeState" => {
                let shape = child(d, "shape");
                let target = shape.and_then(|s| s.attribute("setStateG")).filter(|g| !g.is_empty()).map(String::from).unwrap_or(owner_id);
                Action::SetObjectState { object: target, state: shape.and_then(|s| s.attribute("setStateName")).unwrap_or("").into() }
            }
            "SeekTimeline" => {
                let seek = child(d, "SeekTimeline");
                let at = |a: &str| seek.and_then(|s| s.attribute(a));
                if at("variableReference") == Some("true") || at("useRelativeTime") == Some("true") {
                    ctx.notes.push("a seek to a time from a variable, or relative to now, jumps to the start instead".into());
                }
                let target = at("target").filter(|g| !g.is_empty() && *g != NO_GUID).map(String::from).unwrap_or_else(|| if matches!(owner, Owner::Layer(_)) { ctx.layer.clone() } else { String::new() });
                let ms = if at("variableReference") == Some("true") || at("useRelativeTime") == Some("true") { 0 } else { (at("timeSeconds").and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.0).max(0.0) * 1000.0).round() as u32 };
                Action::SeekTimeline { target, ms }
            }
            "CompleteCourse" => Action::CompleteCourse,
            "exitProject" => Action::ExitCourse,
            "restartProject" => Action::RestartCourse,
            "executeJavaScript" => Action::RunJavaScript(oa("js").unwrap_or("").into()),
            "jumpToFileUrl" if !oa("open").unwrap_or("").is_empty() => Action::OpenUrl(oa("open").unwrap_or("").into()),
            "stopMedia" => {
                let g = child(d, "video").and_then(|v| v.attribute("stopG")).or_else(|| child(d, "audio").and_then(|v| v.attribute("stopG"))).unwrap_or("");
                if g.is_empty() { Action::Unsupported("stopMedia".into()) } else { Action::StopMedia(g.into()) }
            }
            a => Action::Unsupported(format!("{a}/{sub}")),
        };
        if let Action::Unsupported(what) = &action {
            ctx.notes.push(format!("trigger action {what} not supported"));
        }
        let mut conditions = Vec::new();
        if let Some(cl) = child(t, "condLst") {
            for (i, c) in cl.children().filter(|c| c.has_tag_name("trigCond")).enumerate() {
                let var = c.attribute("varG").unwrap_or("").to_string();
                let cmp = match c.attribute("op").unwrap_or("") {
                    "eq" => Cmp::Eq,
                    "ne" | "neq" => Cmp::Ne,
                    "gt" => Cmp::Gt,
                    "lt" => Cmp::Lt,
                    "gte" | "ge" => Cmp::Ge,
                    "lte" | "le" => Cmp::Le,
                    o => {
                        ctx.notes.push(format!("condition operator {o} treated as equals"));
                        Cmp::Eq
                    }
                };
                let Some(kind) = ctx.var_types.get(&var) else {
                    ctx.notes.push("condition on unknown variable skipped".into());
                    continue;
                };
                conditions.push(Condition {
                    variable: var,
                    cmp,
                    value: read_value(kind, c.attribute("boolVal"), c.attribute("floatVal1"), c.attribute("strVal1")),
                    or_with_previous: i > 0 && c.attribute("andOr") == Some("or"),
                });
            }
        }
        // "Restore previous state when the user hovers out": remembered here, added once the objects are known.
        if let (Event::Hover, Action::SetObjectState { object, .. }, Some(frame), Owner::Object(me)) = (&event, &action, child(d, "frame"), &owner) {
            if frame.attribute("restore") == Some("true") { ctx.restores.push((me.clone(), object.clone())); }
        }
        for (i, event) in std::iter::once(event).chain(more_events).enumerate() {
            let g: String = t.attribute("g").unwrap_or_default().into();
            ctx.triggers.push(Trigger { id: if i == 0 { g } else { format!("{g}-{i}") }, owner: owner.clone(), event, action: action.clone(), conditions: conditions.clone() });
        }
    }
}

/// The text colour of the "DrawingShape" default style in `defaultStyles.xml`.
fn drawing_shape_text(parts: &HashMap<String, Vec<u8>>, palette: &HashMap<String, Rgba>) -> Option<Rgba> {
    let text = std::str::from_utf8(parts.get("story/defaultStyles.xml")?).ok()?;
    let doc = Document::parse(strip_bom(text)).ok()?;
    let style = doc.descendants().find(|n| n.has_tag_name("defStyle") && n.attribute("name") == Some("DrawingShape"))?;
    let clr = child(style, "foreClr")?;
    if let Some(v) = child(clr, "srgbClr").and_then(|n| n.attribute("val")) { return parse_hex(v); }
    palette.get(child(clr, "schemeClr")?.attribute("val")?).copied()
}

/// The first line of the text of the slide's title placeholder, if it has one.
fn title_text(sld: Node) -> Option<String> {
    let shapes = child(sld, "shapeLst")?;
    let title = shapes.children().find(|n| child(*n, "ph").is_some_and(|p| p.attribute("type") == Some("title")))?;
    let text = child(title, "plain")?.text()?;
    let line = text.lines().map(str::trim).find(|l| !l.is_empty())?;
    Some(line.to_string())
}

/// The addresses a shape's hyperlinks open, by the `LinkId` its text refers to: `hLinkLst` ties each
/// id to a `hlinkTrig` in the shape's trigger list, and a "jump to URL" trigger holds the address.
fn shape_links(n: Node) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let Some(list) = child(n, "hLinkLst") else { return out };
    for item in list.children().filter(|c| c.has_tag_name("item")) {
        let (Some(id), Some(trig_g)) = (item.attribute("id"), child(item, "txtHlink").and_then(|t| t.attribute("trigG"))) else { continue };
        let trig = child(n, "trigLst").and_then(|l| l.children().find(|t| t.has_tag_name("hlinkTrig") && t.attribute("g") == Some(trig_g)));
        let other = trig.and_then(|t| child(t, "data")).and_then(|d| child(d, "other"));
        let url = other.and_then(|o| o.attribute("open")).filter(|u| !u.is_empty()).map(String::from)
            .or_else(|| other.and_then(|o| o.attribute("email")).filter(|u| !u.is_empty()).map(|m| format!("mailto:{m}")));
        if let Some(u) = url.filter(|u| u.starts_with("http://") || u.starts_with("https://") || u.starts_with("mailto:")) {
            out.insert(id.to_string(), u);
        }
    }
    out
}

/// The text defaults a layout gives one of its placeholders.
#[derive(Clone, Default)]
struct PhStyle {
    /// Font size in source points.
    size: Option<f32>,
    bold: Option<bool>,
    font: Option<String>,
    valign: Option<VAlign>,
    color: Option<Rgba>,
}

/// The placeholders of the slide's layout (found by `layoutG` = the layout's `internalG`), by `g`.
fn layout_placeholders(parts: &HashMap<String, Vec<u8>>, sld: Node, palette: &HashMap<String, Rgba>) -> HashMap<String, PhStyle> {
    let mut out = HashMap::new();
    let Some(layout_g) = sld.attribute("layoutG") else { return out };
    for (key, bytes) in parts.iter().filter(|(k, _)| k.starts_with("story/slideLayouts/") && k.ends_with(".xml")) {
        let _ = key;
        let Ok(text) = std::str::from_utf8(bytes) else { continue };
        let Ok(doc) = Document::parse(strip_bom(text)) else { continue };
        if doc.root_element().attribute("internalG") != Some(layout_g) { continue; }
        for n in doc.descendants().filter(|n| n.has_tag_name("ph") && n.attribute("g").is_some() && child(*n, "bG").is_some()) {
            let font = child(n, "bG").and_then(|b| child(b, "defaultFont"));
            out.insert(n.attribute("g").unwrap_or_default().to_string(), PhStyle {
                size: font.and_then(|f| f.attribute("sz")).and_then(|z| z.parse().ok()),
                bold: font.and_then(|f| f.attribute("style")).map(|st| st.contains("Bold")),
                font: font.and_then(|f| f.attribute("name")).map(String::from),
                valign: match n.attribute("vertAlign") { Some("t") => Some(VAlign::Top), Some("m") => Some(VAlign::Middle), Some("b") => Some(VAlign::Bottom), _ => None },
                // The text colour the layout gives it, which can be a light one on a dark layout.
                color: font.and_then(|f| child(f, "clr")).and_then(|c| {
                    child(c, "srgbClr").and_then(|v| v.attribute("val")).and_then(parse_hex).or_else(|| child(c, "schemeClr").and_then(|v| v.attribute("val")).and_then(|k| palette.get(k).copied()))
                }),
            });
        }
        break;
    }
    out
}

/// Find the slide master for a slide, then read its palette. Links, in order: the slide's `layoutG` equals a
/// layout's `internalG` and the master's relationships list that layout part (the Content master in all 13 sample
/// files); otherwise the only master with role Content; otherwise `fmGuid` equals the master's `g` (in the samples
/// it names the Feedback master or nothing); otherwise the first master, as a last resort.
/// The part names of the master and the layout a slide inherits from (see `master_palette` for the links).
fn find_master_layout(parts: &HashMap<String, Vec<u8>>, sld: Node) -> (Option<String>, Option<String>) {
    let mut masters: Vec<&String> =
        parts.keys().filter(|k| k.starts_with("story/slideMasters/") && k.ends_with(".xml")).collect();
    masters.sort();
    let parse = |k: &str| -> Option<Document> {
        let text = std::str::from_utf8(parts.get(k)?).ok()?;
        Document::parse(strip_bom(text)).ok()
    };
    let mut chosen: Option<&String> = None;
    let mut layout_part: Option<&String> = None;
    // The slide's layout belongs to its master. (`fmGuid` can name the Feedback master instead, which has its own colours.)
    if let Some(layout_g) = sld.attribute("layoutG") {
        layout_part = parts.keys().filter(|k| k.starts_with("story/slideLayouts/") && k.ends_with(".xml")).find(|k| {
            parse(k).is_some_and(|d| d.root_element().attribute("internalG") == Some(layout_g))
        });
        if let Some(lp) = layout_part {
            let file = lp.rsplit('/').next().unwrap_or("");
            chosen = masters.iter().copied().find(|m| {
                let name = m.rsplit('/').next().unwrap_or("");
                let rels = format!("story/slideMasters/_rels/{name}.rels");
                parts.get(&rels).and_then(|b| std::str::from_utf8(b).ok()).is_some_and(|t| t.contains(&format!("/slideLayouts/{file}\"")))
            });
        }
    }
    // No usable layout link: the master made for content slides (not the Feedback one), then `fmGuid`, then the first.
    if chosen.is_none() {
        let content: Vec<&String> = masters.iter().copied().filter(|k| parse(k).is_some_and(|d| d.root_element().attribute("role") == Some("Content"))).collect();
        if content.len() == 1 { chosen = Some(content[0]); }
    }
    if chosen.is_none() {
        if let Some(guid) = sld.attribute("fmGuid") {
            chosen = masters.iter().copied().find(|k| parse(k).is_some_and(|d| d.root_element().attribute("g") == Some(guid)));
        }
    }
    (chosen.or(masters.first().copied()).cloned(), layout_part.cloned())
}

/// The master's and layout's decorative shapes (the dark frame behind a slide, a logo) as a layout the slide
/// inherits from. Their placeholders are left out: those are templates, not drawn behind the content.
fn backdrop_layout(parts: &HashMap<String, Vec<u8>>, sld: Node, ctx: &mut Ctx) -> Option<brainbend_doc::SlideLayout> {
    let (master, layout) = find_master_layout(parts, sld);
    let parse = |k: &str| -> Option<Document> {
        let text = std::str::from_utf8(parts.get(k)?).ok()?;
        Document::parse(strip_bom(text)).ok()
    };
    let layout_doc = parse(&layout?)?;
    let id = layout_doc.root_element().attribute("internalG")?.to_string();
    let mut objects = vec![];
    // Placeholders (`<ph>` shapes, or shapes with a `<ph>` child) are skipped; what the conversion says about them is not news.
    let notes_before = ctx.notes.len();
    let master_doc = master.and_then(|k| parse(&k));
    for doc in master_doc.iter().chain(std::iter::once(&layout_doc)) {
        let list = child(doc.root_element(), "shapeLst");
        let placeholders: Vec<String> = list.map(|l| l.children().filter(|n| child(*n, "ph").is_some()).filter_map(|n| n.attribute("g").map(String::from)).collect()).unwrap_or_default();
        objects.extend(convert_shapes(list, ctx, false).into_iter().filter(|o| !placeholders.contains(&o.id)));
    }
    ctx.notes.truncate(notes_before);
    if objects.is_empty() { return None; }
    Some(brainbend_doc::SlideLayout { id, name: layout_doc.root_element().attribute("name").unwrap_or("Layout").into(), background: None, objects, show_master: false })
}

fn master_palette(parts: &HashMap<String, Vec<u8>>, sld: Node) -> (HashMap<String, Rgba>, Option<Rgba>) {
    let parse = |k: &str| -> Option<Document> {
        let text = std::str::from_utf8(parts.get(k)?).ok()?;
        Document::parse(strip_bom(text)).ok()
    };
    let (chosen, layout_part) = find_master_layout(parts, sld);
    let layout_doc: Option<Document> = layout_part.and_then(|k| parse(&k));
    let mut out = HashMap::new();
    let Some(doc) = chosen.and_then(|k| parse(&k)) else { return (out, None) };
    if let Some(list) = doc.descendants().find(|n| n.has_tag_name("clrLst")) {
        for c in list.children().filter(|n| n.is_element()) {
            if let Some(rgb) = child(c, "srgbClr").and_then(|x| x.attribute("val")).and_then(parse_hex) {
                out.insert(c.tag_name().name().to_string(), rgb);
            }
        }
    }
    // The background a slide inherits: its layout's if that has a fill, else the master's. Only a plain
    // colour can be drawn; a picture or gradient there is left as the white page.
    let own = |d: &Document| -> Option<Option<Rgba>> {
        let bg = d.descendants().find(|n| n.has_tag_name("bg"))?;
        if let Some(clr) = child(bg, "solidFill").and_then(|f| child(f, "clr")) {
            let mut c = if let Some(v) = child(clr, "srgbClr").and_then(|n| n.attribute("val")) { parse_hex(v)? } else { *out.get(child(clr, "schemeClr")?.attribute("val")?)? };
            if let Some(a) = child(clr, "alpha").and_then(|a| a.attribute("val")).and_then(|v| v.parse::<f32>().ok()) { c.3 = (a / 100000.0 * 255.0).round().clamp(0.0, 255.0) as u8; }
            return Some(Some(c));
        }
        ["gradFill", "picFill", "blipFill", "pattFill"].iter().any(|f| child(bg, f).is_some()).then_some(None)
    };
    let inherited = match layout_doc.as_ref().and_then(own) {
        Some(layout_bg) => layout_bg,
        None => own(&doc).flatten(),
    };
    (out, inherited)
}

/// Source bit 0x20 of a shape's `state` value is set on exactly the shapes that are absent
/// from the published course in the sample (e.g. a group named "FOR TESTING"). Inferred.
fn is_hidden(n: Node) -> bool {
    n.attribute("state").and_then(|v| v.parse::<u32>().ok()).is_some_and(|v| v & 0x20 != 0)
}

/// Resolve a `<clr>` node: `srgbClr` or a theme `schemeClr`, with `shade` (darken, linear
/// light) and `alpha` modifiers. Other modifiers are ignored.
#[allow(clippy::question_mark)] // the missing-palette branch deliberately counts before returning None
/// `c` in variant `v` of the theme colour picker (see `resolve_clr`). Greys, whose variants are laid out
/// differently, are left as they are.
fn theme_variant(c: Rgba, v: u32) -> Rgba {
    let (r, g, b) = (c.0 as f32 / 255.0, c.1 as f32 / 255.0, c.2 as f32 / 255.0);
    let (hi, lo) = (r.max(g).max(b), r.min(g).min(b));
    let l = (hi + lo) / 2.0;
    let d = hi - lo;
    let s = if d == 0.0 { 0.0 } else { d / (1.0 - (2.0 * l - 1.0).abs()) };
    if d < 0.06 { return c; }
    // 1-5: lighter 80/60/40 %, darker 25/50 %. 6-10 (seen on dark theme colours): a ramp from nearly white down to the colour
    // itself, read off the source app (6 = the card behind the slide, 8 = a button's Hover, 9 = Normal, 10 = Selected).
    let (m, o) = match v { 1 => (0.2, 0.8), 2 => (0.4, 0.6), 3 => (0.6, 0.4), 4 => (0.75, 0.0), 5 => (0.5, 0.0), 6 => (0.1, 0.9), 7 => (0.25, 0.75), 8 => (0.5, 0.5), 9 => (0.75, 0.25), 10 => (1.0, 0.0), _ => return c };
    let h = if hi == r { ((g - b) / d).rem_euclid(6.0) } else if hi == g { (b - r) / d + 2.0 } else { (r - g) / d + 4.0 };
    let l2 = (l * m + o).clamp(0.0, 1.0);
    let k = (1.0 - (2.0 * l2 - 1.0).abs()) * s;
    let x = k * (1.0 - (h.rem_euclid(2.0) - 1.0).abs());
    let (r1, g1, b1) = match h as u32 { 0 => (k, x, 0.0), 1 => (x, k, 0.0), 2 => (0.0, k, x), 3 => (0.0, x, k), 4 => (x, 0.0, k), _ => (k, 0.0, x) };
    let f = |v: f32| ((v + l2 - k / 2.0) * 255.0).round().clamp(0.0, 255.0) as u8;
    Rgba(f(r1), f(g1), f(b1), c.3)
}

fn resolve_clr(clr: Node, ctx: &mut Ctx) -> Option<Rgba> {
    let mut c = if let Some(n) = child(clr, "srgbClr") {
        parse_hex(n.attribute("val")?)?
    } else if let Some(n) = child(clr, "schemeClr") {
        match ctx.palette.get(n.attribute("val")?) {
            Some(c) => *c,
            None => {
                ctx.missing_scheme += 1;
                return None;
            }
        }
    } else {
        return None;
    };
    // Modifiers (`shade`, `alpha`, ...) are siblings of the color element, inside `<clr>`.
    for m in clr.children().filter(|m| m.is_element()) {
        let v: f32 = m.attribute("val").and_then(|v| v.parse().ok()).unwrap_or(100000.0) / 100000.0;
        match m.tag_name().name() {
            "shade" => {
                let f = |ch: u8| {
                    let lin = (ch as f32 / 255.0).powf(2.2) * v;
                    (lin.powf(1.0 / 2.2) * 255.0).round().clamp(0.0, 255.0) as u8
                };
                c = Rgba(f(c.0), f(c.1), f(c.2), c.3);
            }
            "alpha" => c.3 = (v * 255.0).round().clamp(0.0, 255.0) as u8,
            // The source app's colour picker offers each theme colour in lighter and darker variants: `grad` is
            // the variant (1-3 lighter by 80, 60 and 40 percent, 4 and 5 darker by 25 and 50, as in Office).
            "grad" => c = theme_variant(c, m.attribute("val").and_then(|v| v.parse().ok()).unwrap_or(0)),
            _ => {}
        }
    }
    Some(c)
}

fn base64_decode(s: &str) -> Vec<u8> {
    let (mut out, mut acc, mut bits) = (vec![], 0u32, 0);
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => continue,
        };
        acc = (acc << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    out
}

/// The source app freeforms store GDI+ path data: absolute points plus one type byte per point
/// (low bits 0 start, 1 line, 3 bezier; 0x80 closes the figure). The points do not share the
/// frame's coordinates exactly, so they are normalised to their own bounding box and stretched.
fn parse_path(n: Node) -> Option<Vec<PathCmd>> {
    let pts: Vec<(f32, f32)> = child(n, "points")?.children().filter(|c| c.has_tag_name("point")).map(|p| (num(p, "x", 0.0), num(p, "y", 0.0))).collect();
    let types = base64_decode(child(n, "types")?.text()?);
    if pts.len() < 2 || types.len() != pts.len() { return None; }
    let (minx, maxx) = pts.iter().fold((f32::MAX, f32::MIN), |a, p| (a.0.min(p.0), a.1.max(p.0)));
    let (miny, maxy) = pts.iter().fold((f32::MAX, f32::MIN), |a, p| (a.0.min(p.1), a.1.max(p.1)));
    let nx = |x: f32| (x - minx) / (maxx - minx).max(0.001);
    let ny = |y: f32| (y - miny) / (maxy - miny).max(0.001);
    let mut cmds = vec![];
    let mut i = 0;
    while i < pts.len() {
        let t = types[i];
        match t & 7 {
            0 => cmds.push(PathCmd::Move(nx(pts[i].0), ny(pts[i].1))),
            1 => cmds.push(PathCmd::Line(nx(pts[i].0), ny(pts[i].1))),
            3 if i + 2 < pts.len() => {
                cmds.push(PathCmd::Curve(nx(pts[i].0), ny(pts[i].1), nx(pts[i + 1].0), ny(pts[i + 1].1), nx(pts[i + 2].0), ny(pts[i + 2].1)));
                i += 2;
            }
            _ => return None,
        }
        if types[i] & 0x80 != 0 { cmds.push(PathCmd::Close); }
        i += 1;
    }
    Some(cmds)
}

fn fill_of(n: Node, ctx: &mut Ctx) -> Option<Rgba> {
    let solid = child(n, "bG").and_then(|bg| child(bg, "solidFill"))?;
    resolve_clr(child(solid, "clr")?, ctx)
}

/// The outline's pattern: the source app's `lineStyle@dash` (`sqDot` and `rndDot` are dots, other named dashes dashes).
/// The layers that hide this base-layer object while they show: `<extTmLst><extTm id="layer" val="true"/>`.
fn hidden_on_layers(n: Node) -> Vec<String> {
    child(n, "extTmLst").map(|l| l.children().filter(|e| e.has_tag_name("extTm") && e.attribute("val") == Some("true")).filter_map(|e| e.attribute("id").map(String::from)).collect()).unwrap_or_default()
}

/// A video's `play="auto"`, `showCtrls`, `loop`, `mute` and `vol` (0-100). Audio objects carry no such attributes here.
fn media_of(n: Node) -> MediaPlay {
    if !matches!(n.tag_name().name(), "video" | "audio") {
        return MediaPlay::default();
    }
    let on = |a: &str| n.attribute(a) == Some("true");
    MediaPlay {
        autoplay: n.attribute("play") == Some("auto"),
        controls: on("showCtrls"),
        looped: on("loop"),
        muted: on("mute"),
        volume: n.attribute("vol").and_then(|v| v.parse::<f32>().ok()).map_or(1.0, |v| (v / 100.0).clamp(0.0, 1.0)),
    }
}

fn dash_of(n: Node) -> LineDash {
    match child(n, "bG").and_then(|bg| child(bg, "lineStyle")).and_then(|l| l.attribute("dash")) {
        Some("sqDot") | Some("rndDot") => LineDash::Dotted,
        Some(d) if d.to_ascii_lowercase().contains("dash") => LineDash::Dashed,
        _ => LineDash::Solid,
    }
}

fn stroke_of(n: Node, ctx: &mut Ctx) -> Option<Stroke> {
    let bg = child(n, "bG")?;
    let color = resolve_clr(child(child(bg, "solidLine")?, "clr")?, ctx)?;
    let width = child(bg, "lineStyle").map(|l| num(l, "w", 1.0)).unwrap_or(1.0);
    (width > 0.0).then_some(Stroke { color, width })
}

fn shadow_of(n: Node, ctx: &mut Ctx) -> Option<Shadow> {
    let sh = child(child(n, "bG")?, "shdw")?;
    // `<preset val="noShdw">` is the "no shadow" setting; the offsets are still stored.
    if child(sh, "preset").and_then(|p| p.attribute("val")) == Some("noShdw") {
        return None;
    }
    let color = resolve_clr(child(sh, "clr")?, ctx)?;
    let (dist, angle) = (num(sh, "dist", 0.0), num(sh, "angle", 45.0).to_radians());
    Some(Shadow { dx: dist * angle.cos(), dy: dist * angle.sin(), blur: num(sh, "blur", 0.0), spread: 0.0, color })
}

fn default_style() -> TextStyle {
    TextStyle {
        font_family: "Open Sans".into(),
        size: 14.0,
        bold: false,
        italic: false,
        underline: false,
        color: Rgba(0, 0, 0, 255),
        align: Align::Left,
        valign: VAlign::Middle,
    }
}

/// Shape-level text defaults (observed): `bG/foreClr` is the text color, `bG/defaultFont`
/// gives size and weight, and `defHorzAlign`/`vertAlign` position the text. Runs in the text
/// document override these.
/// The alignment `<fmtText>` gives the shape's first paragraph, if it states one.
fn fmt_justification(shape: Node) -> Option<Align> {
    let xml = child(shape, "fmtText")?.text()?;
    let doc = Document::parse(strip_bom(xml)).ok()?;
    let block = doc.descendants().find(|n| n.has_tag_name("Block"))?;
    match child(block, "Style")?.attribute("Justification")? {
        "Center" => Some(Align::Center),
        "Right" => Some(Align::Right),
        "Left" => Some(Align::Left),
        _ => None,
    }
}

fn base_style(shape: Node, ctx: &mut Ctx) -> TextStyle {
    let mut st = default_style();
    // Shapes from the shape gallery start from the "DrawingShape" text colour; text boxes and buttons do not.
    if matches!(shape.tag_name().name(), "rect" | "roundRect" | "oval" | "triangle" | "rightTriangle" | "rightArrow" | "roundSameSideCorner" | "snipDiagCorner" | "lnCall1" | "multiply" | "checkMark") {
        if let Some(c) = ctx.drawing_text { st.color = c; }
    }
    // A placeholder starts from what its layout says.
    let inherited = child(shape, "ph").and_then(|p| p.attribute("g")).and_then(|g| ctx.ph_styles.get(g)).cloned().unwrap_or_default();
    if let Some(c) = inherited.color { st.color = c; }
    if let Some(z) = inherited.size { st.size = z * ctx.font_scale; }
    if let Some(b) = inherited.bold { st.bold = b; }
    if let Some(f) = &inherited.font { st.font_family = f.clone(); }
    // `<resize fontSz="12">` is the size (pt) of text that names none; 0 = not set.
    // A placeholder keeps the size its layout gives it.
    if let Some(z) = child(shape, "resize").and_then(|r| r.attribute("fontSz")).and_then(|z| z.parse::<f32>().ok()).filter(|z| *z > 0.0 && inherited.size.is_none()) {
        st.size = z * ctx.font_scale;
    }
    st.align = match shape.attribute("defHorzAlign") {
        Some("c") => Align::Center,
        Some("r") => Align::Right,
        _ => Align::Left,
    };
    // `<fmtText>` is the text as formatted, with what the shape gives it by default: a button's text is centred there
    // while `defHorzAlign` says left. A paragraph's own `Justification` in `<text>` still wins.
    if let Some(j) = fmt_justification(shape) { st.align = j; }
    st.valign = match shape.attribute("vertAlign") {
        Some("t") => VAlign::Top,
        Some("b") => VAlign::Bottom,
        Some("m") => VAlign::Middle,
        _ => inherited.valign.unwrap_or(VAlign::Middle),
    };
    if let Some(bg) = child(shape, "bG") {
        if let Some(clr) = child(bg, "foreClr") {
            if let Some(c) = child(clr, "srgbClr").and_then(|c| c.attribute("val")).and_then(parse_hex) {
                st.color = c;
            } else if let Some(c) = child(clr, "schemeClr").and_then(|c| c.attribute("val")).and_then(|n| ctx.palette.get(n)) {
                st.color = *c;
            }
        }
        if let Some(f) = child(bg, "defaultFont") {
            if let Some(z) = f.attribute("sz").and_then(|z| z.parse::<f32>().ok()) {
                st.size = z * ctx.font_scale;
            }
            if f.attribute("style").is_some_and(|s| s.contains("Bold")) {
                st.bold = true;
            }
            if let Some(n) = f.attribute("name") {
                st.font_family = n.into();
            }
        }
    }
    st
}

/// Parse the nested `<Document>` XML stored (escaped) inside a shape's `<text>`.
/// Returns plain text (blocks joined by newlines) and a style: `base` overridden by the
/// first run's explicit attributes.
pub fn parse_text_doc(src: &str) -> (String, TextStyle) {
    parse_text_doc_with(src, &default_style())
}

pub fn parse_text_doc_with(src: &str, base: &TextStyle) -> (String, TextStyle) {
    let (t, st, _) = parse_text_rich(src, base, FONT_SCALE, &HashMap::new());
    (t, st)
}

/// Like `parse_text_doc_with`, and also returns per-run styling when runs differ in weight,
/// color, size or font (otherwise `None`: the single object style is enough).
pub fn parse_text_rich(src: &str, base: &TextStyle, scale: f32, palette: &HashMap<String, Rgba>) -> TextParts {
    parse_text_rich_with(src, base, scale, palette, &HashMap::new())
}

/// The colour a hyperlink is drawn in when the theme does not say.
const LINK_BLUE: Rgba = Rgba(5, 99, 193, 255);

/// Like [`parse_text_rich`]. `links` maps a span's `LinkId` to the address it opens.
///
/// The source app stores a paragraph (a `Block`) with its end marker as the last character of its last span
/// (`\n`), and a soft line break as `\r`. The end marker is dropped here, since paragraphs are separate;
/// that leaves `\r\n` at the end of a paragraph as a line break after the last word, which is a blank line.
pub fn parse_text_rich_with(src: &str, base: &TextStyle, scale: f32, palette: &HashMap<String, Rgba>, links: &HashMap<String, String>) -> TextParts {
    let mut style = base.clone();
    let Ok(doc) = Document::parse(strip_bom(src)) else { return (String::new(), style, None) };
    let mut lines = Vec::new();
    let mut styled = false;
    // (text, resolved style, link) for every span, grouped by block.
    // A style of `None` = the span has no style of its own and inherits the object's.
    let mut blocks: Vec<Vec<(String, Option<TextStyle>, Option<String>)>> = Vec::new();
    for block in doc.descendants().filter(|n| n.has_tag_name("Block")) {
        let mut spans: Vec<(String, Option<TextStyle>, Option<String>)> = Vec::new();
        let mut block_style = base.clone();
        if let Some(j) = child(block, "Style").and_then(|s| s.attribute("Justification")) {
            let a = match j {
                "Center" => Align::Center,
                "Right" => Align::Right,
                _ => Align::Left,
            };
            block_style.align = a;
            if !styled {
                style.align = a;
            }
        }
        let span_nodes: Vec<Node> = block.children().filter(|n| n.has_tag_name("Span")).collect();
        for (si, span) in span_nodes.iter().enumerate() {
            let mut raw = span.attribute("Text").unwrap_or("").to_string();
            // The paragraph's end marker closes its last span.
            if si + 1 == span_nodes.len() && raw.ends_with('\n') { raw.pop(); }
            let text = raw.replace("\r\n", "\n").replace('\r', "\n");
            let mut st = block_style.clone();
            if let Some(s) = child(*span, "Style") {
                if let Some(f) = s.attribute("FontFamily") { st.font_family = f.into(); }
                if let Some(z) = s.attribute("FontSize").and_then(|z| z.parse::<f32>().ok()) { st.size = z * scale; }
                if let Some(b) = s.attribute("FontIsBold") { st.bold = b == "True"; }
                if let Some(b) = s.attribute("FontIsItalic") { st.italic = b == "True"; }
                if let Some(b) = s.attribute("FontIsUnderline") { st.underline = b == "True"; }
                if let Some(c) = s.attribute("ForegroundColor").and_then(|v| parse_color(v, palette)) { st.color = c; }
            }
            if !styled && child(*span, "Style").is_some() {
                style = st.clone();
                styled = true;
            }
            // A link is drawn the way the source app draws one: underlined, in the theme's link colour.
            let link = child(*span, "Style").and_then(|s| s.attribute("LinkId")).and_then(|id| links.get(id)).cloned();
            let mut shown = st;
            if link.is_some() {
                shown.underline = true;
                shown.color = palette.get("hlink").copied().unwrap_or(LINK_BLUE);
            }
            // A span with no style of its own is drawn in the shape's default style, not in the first styled run's
            // (an error text: a bold heading, then a plain sentence).
            let own = child(*span, "Style").is_some() || !text.trim().is_empty();
            spans.push((text, own.then_some(shown), link));
        }
        // A bulleted paragraph starts with its bullet (the editor writes lists the same way: as text).
        let list = child(block, "Style").and_then(|s| child(s, "ListStyle"));
        if list.is_some_and(|l| l.attribute("ListType") == Some("Bullet")) && spans.iter().any(|s| !s.0.trim().is_empty()) {
            let ch = list.and_then(|l| l.attribute("BulletChar")).filter(|c| c.chars().count() == 1).unwrap_or("\u{2022}");
            spans.insert(0, (format!("{ch} "), None, None));
        }
        lines.push(spans.iter().map(|s| s.0.as_str()).collect::<String>());
        blocks.push(spans);
    }
    // Object style = first styled run; runs only record how they differ from it.
    let differs = |a: &TextStyle| a.bold != style.bold || a.italic != style.italic || a.underline != style.underline || a.color != style.color || a.size != style.size || a.font_family != style.font_family;
    let rich = if blocks.iter().flatten().any(|(_, s, l)| l.is_some() || s.as_ref().is_some_and(differs)) {
        Some(
            blocks
                .into_iter()
                .map(|spans| Paragraph {
                    runs: spans
                        .into_iter()
                        .map(|(text, s, link)| match s {
                            None => Run { text, link, ..Default::default() },
                            Some(s) => Run {
                                text,
                                bold: (s.bold != style.bold).then_some(s.bold),
                                italic: (s.italic != style.italic).then_some(s.italic),
                                underline: (s.underline != style.underline).then_some(s.underline),
                                color: (s.color != style.color).then_some(s.color),
                                size: (s.size != style.size).then_some(s.size),
                                font_family: (s.font_family != style.font_family).then(|| s.font_family.clone()),
                                link,
                                ..Default::default()
                            },
                        })
                        .collect(), ..Default::default() })
                .collect(),
        )
    } else {
        None
    };
    (lines.join("\n"), style, rich)
}

/// Identify media by content (the package stores MP4 video under a `.mpeg` name).
fn sniff_mime(data: &[u8], path: &str) -> &'static str {
    if data.starts_with(&[0x89, b'P', b'N', b'G']) { return "image/png"; }
    if data.starts_with(&[0xFF, 0xD8]) { return "image/jpeg"; }
    if data.starts_with(b"GIF8") { return "image/gif"; }
    if data.len() > 12 && &data[4..8] == b"ftyp" { return "video/mp4"; }
    if data.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) { return "video/webm"; }
    if data.starts_with(b"ID3") || data.starts_with(&[0xFF, 0xFB]) || data.starts_with(&[0xFF, 0xF3]) { return "audio/mpeg"; }
    if data.starts_with(b"OggS") { return "audio/ogg"; }
    if data.starts_with(b"RIFF") { return "audio/wav"; }
    match path.rsplit('.').next().unwrap_or("").to_ascii_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "mp4" | "mpeg" => "video/mp4",
        "mp3" => "audio/mpeg",
        _ => "application/octet-stream",
    }
}

/// A text color is `#RRGGBB`, or a theme reference like `dk2,00` (palette name, then a modifier
/// we do not interpret yet).
fn parse_color(s: &str, palette: &HashMap<String, Rgba>) -> Option<Rgba> {
    parse_hex(s).or_else(|| palette.get(s.split(',').next().unwrap_or("")).copied())
}

fn parse_hex(s: &str) -> Option<Rgba> {
    let h = s.trim_start_matches('#');
    let h = if h.len() == 8 { &h[2..] } else { h }; // tolerate AARRGGBB
    if h.len() != 6 { return None; }
    let v = u32::from_str_radix(h, 16).ok()?;
    Some(Rgba((v >> 16) as u8, (v >> 8) as u8, v as u8, 255))
}

fn child<'a, 'b>(n: Node<'a, 'b>, name: &str) -> Option<Node<'a, 'b>> {
    n.children().find(|c| c.has_tag_name(name))
}

fn num(n: Node, attr: &str, default: f32) -> f32 {
    n.attribute(attr).and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn italic_and_underline_runs_are_kept() {
        let src = r##"<Document><Content><Block><Span Text="Plain "><Style FontSize="10"/></Span><Span Text="slanted"><Style FontSize="10" FontIsItalic="True" FontIsUnderline="True"/></Span></Block></Content></Document>"##;
        let (_, style, rich) = parse_text_rich(src, &default_style(), 1.0, &HashMap::new());
        assert!(!style.italic);
        let runs = &rich.unwrap()[0].runs;
        assert_eq!((runs[1].italic, runs[1].underline), (Some(true), Some(true)));
    }

    #[test]
    fn freeform_path_is_decoded_and_normalised() {
        // Types: start, line, bezier(3 points, last closes) -> AAEDAwOD is [0,1,3,3,3|0x80]
        let xml = r#"<importedVector><points><point x="10" y="20"/><point x="30" y="20"/><point x="30" y="30"/><point x="20" y="40"/><point x="10" y="40"/></points><types>AAEDA4M=</types></importedVector>"#;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let cmds = parse_path(doc.root_element()).unwrap();
        assert_eq!(cmds[0], PathCmd::Move(0.0, 0.0));
        assert_eq!(cmds[1], PathCmd::Line(1.0, 0.0));
        assert_eq!(cmds[2], PathCmd::Curve(1.0, 0.5, 0.5, 1.0, 0.0, 1.0));
        assert_eq!(cmds[3], PathCmd::Close);
    }

    #[test]
    fn text_document_is_parsed() {
        let src = r##"<Document><Content><Block><Style Justification="Center"/><Span Text="Hello"><Style FontFamily="Montserrat" FontSize="12" FontIsBold="True" ForegroundColor="#03539E"/></Span></Block><Block><Span Text="World"/></Block></Content></Document>"##;
        let (t, s) = parse_text_doc(src);
        assert_eq!(t, "Hello\nWorld");
        assert_eq!(s.font_family, "Montserrat");
        assert!(s.bold);
        assert_eq!(s.align, Align::Center);
        assert_eq!(s.color, Rgba(3, 83, 158, 255));
        assert!((s.size - 12.0 * FONT_SCALE).abs() < 0.01);
    }

    fn package(files: &[(&str, &str)]) -> std::io::Cursor<Vec<u8>> {
        use std::io::Write;
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut buf);
            let o = zip::write::SimpleFileOptions::default();
            for (name, body) in files {
                z.start_file(*name, o).unwrap();
                z.write_all(body.as_bytes()).unwrap();
            }
            z.finish().unwrap();
        }
        buf.set_position(0);
        buf
    }

    const SLIDE: &str = r#"<sld g="G" name="NAME"><shapeLst/><sldLayerLst/></sld>"#;

    #[test]
    fn slides_follow_scene_order_not_file_name_order() {
        let story = r#"<story><propLst><prop><sz w="1280" h="720"/></prop></propLst>
            <sceneLst><scene name="s"><sldIdLst><sldId>R2</sldId><sldId>R1</sldId></sldIdLst></scene></sceneLst></story>"#;
        let rels = r#"<Relationships xmlns="x"><Relationship Type="slide" Target="/story/slides/slide.xml" Id="R1"/>
            <Relationship Type="slide" Target="/story/slides/slide2.xml" Id="R2"/></Relationships>"#;
        let pkg = package(&[
            ("story/story.xml", story),
            ("story/_rels/story.xml.rels", rels),
            ("story/slides/slide.xml", &SLIDE.replace("NAME", "First by name")),
            ("story/slides/slide2.xml", &SLIDE.replace("NAME", "First in course")),
            ("story/slides/slide3.xml", &SLIDE.replace("NAME", "Unlisted")),
        ]);
        let p = import(pkg, "t").unwrap();
        let names: Vec<_> = p.slides.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["First in course", "First by name", "Unlisted"]);
        assert_eq!((p.slides[0].width, p.slides[0].height), (1280.0, 720.0));
    }

    #[test]
    fn scenes_are_imported_with_their_names_and_slides() {
        let story = r#"<story><propLst><prop><sz w="1280" h="720"/></prop></propLst>
            <sceneLst><scene name="Intro"><sldIdLst><sldId>R1</sldId></sldIdLst></scene>
            <scene name="Quiz"><sldIdLst><sldId>R2</sldId></sldIdLst></scene></sceneLst></story>"#;
        let rels = r#"<Relationships xmlns="x"><Relationship Type="slide" Target="/story/slides/slide.xml" Id="R1"/>
            <Relationship Type="slide" Target="/story/slides/slide2.xml" Id="R2"/></Relationships>"#;
        let pkg = package(&[
            ("story/story.xml", story),
            ("story/_rels/story.xml.rels", rels),
            ("story/slides/slide.xml", &SLIDE.replace("NAME", "A")),
            ("story/slides/slide2.xml", &SLIDE.replace("NAME", "B")),
        ]);
        let p = import(pkg, "t").unwrap();
        let names: Vec<_> = p.scenes.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["Intro", "Quiz"]);
        assert_eq!(p.slides[0].scene, p.scenes[0].id);
        assert_eq!(p.slides[1].scene, p.scenes[1].id);
    }

    #[test]
    fn mixed_runs_become_rich_text() {
        let src = r##"<Document><Content><Block><Span Text="Plain "><Style FontSize="10" ForegroundColor="#000000"/></Span><Span Text="bold red"><Style FontSize="10" FontIsBold="True" ForegroundColor="#FF0000"/></Span></Block></Content></Document>"##;
        let (text, style, rich) = parse_text_rich(src, &default_style(), FONT_SCALE, &HashMap::new());
        assert_eq!(text, "Plain bold red");
        assert!(!style.bold);
        let rich = rich.expect("runs differ, so rich text is kept");
        assert_eq!(rich[0].runs.len(), 2);
        assert_eq!(rich[0].runs[0].bold, None);
        assert_eq!(rich[0].runs[1].bold, Some(true));
        assert_eq!(rich[0].runs[1].color, Some(Rgba(255, 0, 0, 255)));
        // A uniform text document stays plain.
        // Unstyled spans (e.g. plain line breaks) inherit and do not make the text "rich".
        let breaks = r##"<Document><Content><Block><Span Text="a"><Style FontSize="10"/></Span><Span Text="&#xD;"/></Block></Content></Document>"##;
        assert!(parse_text_rich(breaks, &default_style(), FONT_SCALE, &HashMap::new()).2.is_none());
        let uniform = r##"<Document><Content><Block><Span Text="a"><Style FontSize="10"/></Span><Span Text="b"><Style FontSize="10"/></Span></Block></Content></Document>"##;
        assert!(parse_text_rich(uniform, &default_style(), FONT_SCALE, &HashMap::new()).2.is_none());
    }

    #[test]
    fn buttons_in_a_set_share_its_name() {
        let slide = format!(r#"<sld g="s" name="A"><shapeLst>
            <rect g="a" name="a" grpId="Button set 1">{LOC}</rect><rect g="b" name="b" grpId="Button set 1">{LOC}</rect>
            <rect g="c" name="c" grpId="">{LOC}</rect></shapeLst></sld>"#);
        let p = import(package(&[("story/story.xml", "<story/>"), ("story/slides/slide.xml", &slide)]), "t").unwrap();
        let sets: Vec<Option<&str>> = p.slides[0].layers[0].objects.iter().map(|o| o.button_set.as_deref()).collect();
        assert_eq!(sets, [Some("Button set 1"), Some("Button set 1"), None]);
    }

    #[test]
    fn base_objects_remember_which_layers_hide_them() {
        let slide = format!(r#"<sld g="s" name="A"><shapeLst>
            <rect g="body" name="Body">{LOC}<extTmLst><extTm id="L1" val="true"/><extTm id="L2" val="false"/></extTmLst></rect>
            <rect g="title" name="Title">{LOC}</rect></shapeLst></sld>"#);
        let p = import(package(&[("story/story.xml", "<story/>"), ("story/slides/slide.xml", &slide)]), "t").unwrap();
        let o = &p.slides[0].layers[0].objects;
        assert_eq!(o[0].hide_on_layers, ["L1"], "only the layers marked true hide it");
        assert!(o[1].hide_on_layers.is_empty());
    }

    #[test]
    fn a_layers_stop_setting_is_hide_other_layers() {
        let slide = r#"<sld g="s" name="A"><shapeLst/><sldLayerLst>
            <sldLayer g="one" name="One" modal="false" stop="true" top="true"><shapeLst/></sldLayer>
            <sldLayer g="two" name="Two" modal="false" stop="false" top="true"><shapeLst/></sldLayer></sldLayerLst></sld>"#;
        let p = import(package(&[("story/story.xml", "<story/>"), ("story/slides/slide.xml", slide)]), "t").unwrap();
        let hides: Vec<(&str, bool)> = p.slides[0].layers.iter().map(|l| (l.name.as_str(), l.hide_others)).collect();
        assert_eq!(hides, [("Base layer", false), ("One", true), ("Two", false)], "top=true on both says nothing; stop does");
    }

    #[test]
    fn a_slide_takes_the_colours_of_the_master_its_layout_belongs_to_not_the_feedback_one() {
        let mk = |g: &str, role: &str, dk2: &str| format!(r#"<master g="{g}" role="{role}"><clrLst><dk2><srgbClr val="{dk2}"/></dk2></clrLst></master>"#);
        let feedback = mk("F", "Feedback", "FF0000");
        let content = mk("C", "Content", "00FF00");
        let layout = r#"<layout internalG="L"/>"#;
        let rels = r#"<Relationships><Relationship Target="/story/slideLayouts/layout.xml" Id="R1"/></Relationships>"#;
        let story = "<story/>";
        let slide = |attrs: &str| format!(r#"<sld g="s" name="A" {attrs}><shapeLst><rect g="a" name="a"><bG><solidFill><clr><schemeClr val="dk2"/></clr></solidFill></bG>{LOC}</rect></shapeLst></sld>"#);
        let fill = |slide: String| {
            let p = import(package(&[
                ("story/story.xml", story),
                ("story/slideMasters/slideMaster.xml", &feedback),
                ("story/slideMasters/slideMaster2.xml", &content),
                ("story/slideMasters/_rels/slideMaster2.xml.rels", rels),
                ("story/slideLayouts/layout.xml", layout),
                ("story/slides/slide.xml", &slide),
            ]), "t").unwrap();
            p.slides[0].layers[0].objects[0].fill
        };
        let green = Some(Rgba(0, 255, 0, 255));
        assert_eq!(fill(slide(r#"fmGuid="F" layoutG="L""#)), green, "the layout link wins over fmGuid, which names the Feedback master");
        assert_eq!(fill(slide(r#"fmGuid="F""#)), green, "with no layout link, the Content master beats fmGuid and the first master");
    }

    #[test]
    fn iso_durations_parse() {
        assert_eq!(iso_ms("PT1.5S"), Some(1500));
        assert_eq!(iso_ms("PT1M2S"), Some(62_000));
        assert_eq!(iso_ms("1.5"), None);
    }

    #[test]
    fn timing_effects_and_slide_timeline_are_imported() {
        let story = r#"<story><propLst><prop><sz w="960" h="540"/></prop></propLst>
            <sceneLst><scene><sldIdLst><sldId>R1</sldId></sldIdLst></scene></sceneLst></story>"#;
        let rels = r#"<Relationships xmlns="x"><Relationship Type="slide" Target="/story/slides/slide.xml" Id="R1"/></Relationships>"#;
        let slide = r#"<sld g="s" name="T" advMode="auto"><tmCtxLst><tmCtx start="0" dur="4000"/></tmCtxLst>
          <shapeLst>
            <rect g="a" name="timed"><tmCtxLst><tmCtx start="500" dur="1000" untilEnd="false"/></tmCtxLst>
              <animEffect><entr><fly dur="PT0.75S" dir="l"/></entr><exit><fade dur="PT0.25S"/></exit></animEffect>
              <loc l="0" t="0" r="10" b="10"/></rect>
            <rect g="b" name="always"><tmCtxLst><tmCtx start="0" dur="4000" untilEnd="true"/></tmCtxLst>
              <animEffect><entr><swivel dur="PT1S"/></entr></animEffect><loc l="0" t="0" r="10" b="10"/></rect>
          </shapeLst><sldLayerLst/></sld>"#;
        let p = import(package(&[("story/story.xml", story), ("story/_rels/story.xml.rels", rels), ("story/slides/slide.xml", slide)]), "t").unwrap();
        let s = &p.slides[0];
        assert!(s.auto_advance);
        assert_eq!(s.layers[0].duration_ms, Some(4000));
        let (a, b) = (&s.layers[0].objects[0], &s.layers[0].objects[1]);
        assert_eq!(a.timing, Some(Timing { start_ms: 500, dur_ms: Some(1000) }));
        assert_eq!(a.entrance, Some(Entrance { kind: EffectKind::SlideLeft, delay_ms: 0, duration_ms: 750 }));
        assert_eq!(a.exit.map(|e| (e.kind, e.duration_ms)), Some((EffectKind::Fade, 250)));
        assert_eq!(b.timing, None, "on screen for the whole timeline");
        assert_eq!(b.entrance.map(|e| e.kind), Some(EffectKind::Fade), "unknown effects fall back to fade");
        assert!(p.notes.iter().any(|n| n.contains("swivel")));
    }

    #[test]
    fn spin_wheel_and_split_effects_are_kept() {
        let fx = |e: &str| {
            let shape = format!(r#"<rect g="a" name="x"><animEffect>{e}</animEffect><loc l="0" t="0" r="10" b="10"/></rect>"#);
            let p = slide_with(&shape, "");
            (p.slides[0].layers[0].objects[0].entrance.map(|x| x.kind), p.slides[0].layers[0].objects[0].exit.map(|x| x.kind), p.notes.clone())
        };
        let (en, _, notes) = fx(r#"<entr><spin dur="PT0.5S" amt="qua" sd="clock"/></entr>"#);
        assert_eq!(en, Some(EffectKind::SpinQuarter));
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(fx(r#"<entr><spin dur="PT0.5S" amt="half" sd="clock"/></entr>"#).0, Some(EffectKind::SpinHalf));
        let (en, _, notes) = fx(r#"<entr><spin dur="PT0.5S" amt="two" sd="counter"/></entr>"#);
        assert_eq!(en, Some(EffectKind::SpinFull));
        assert!(notes.iter().any(|n| n.contains("more than one turn")) && notes.iter().any(|n| n.contains("other way")), "{notes:?}");
        let (en, ex, notes) = fx(r#"<entr><wheel dur="PT0.25S" amt="one"/></entr><exit><wheel dur="PT0.25S" amt="one"/></exit>"#);
        assert_eq!((en, ex), (Some(EffectKind::Wheel), Some(EffectKind::Wheel)));
        assert!(notes.is_empty(), "{notes:?}");
        assert!(fx(r#"<entr><wheel dur="PT0.25S" amt="four"/></entr>"#).2.iter().any(|n| n.contains("spokes")));
        assert_eq!(fx(r#"<entr><split dur="PT0.75S" sd="vertOut"/></entr>"#).0, Some(EffectKind::SplitVerticalOut));
        assert_eq!(fx(r#"<exit><split dur="PT0.75S" sd="vertIn"/></exit>"#).1, Some(EffectKind::SplitVerticalIn));
        assert_eq!(fx(r#"<entr><split dur="PT0.75S" sd="horzOut"/></entr>"#).0, Some(EffectKind::SplitHorizontalOut));
        assert_eq!(fx(r#"<entr><split dur="PT0.75S" sd="horzIn"/></entr>"#).0, Some(EffectKind::SplitHorizontalIn));
    }

    #[test]
    fn font_scale_follows_slide_height() {
        assert!((font_scale_for(960.0, 540.0) - 4.0 / 3.0).abs() < 1e-4);
        assert!((font_scale_for(720.0, 405.0) - 4.0 / 3.0).abs() < 1e-4); // short slides are not shrunk
        assert!((font_scale_for(1920.0, 1400.0) - 3.4568).abs() < 1e-3); // measured 3.4
        // The same text is the same share of the slide whatever its width.
        assert_eq!(font_scale_for(720.0, 540.0), font_scale_for(960.0, 540.0));
    }

    #[test]
    fn rejects_non_zip() {
        assert!(import(std::io::Cursor::new(b"nope".to_vec()), "x").is_err());
    }

    #[test]
    fn sample_file_imports_when_present() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/safety-model.story");
        let Ok(f) = std::fs::File::open(p) else { return };
        let proj = import(f, "sample").unwrap();
        assert!(proj.slides[0].layers.len() > 10);
        assert!(proj.assets.len() >= 7);
    }

    /// GIF pictures (ELH 283's speech bubbles) must be decodable, or the editor draws nothing for them.
    #[test]
    fn gif_assets_decode_when_present() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/ELH_283_-_Interactive_graphic_2.story");
        let Ok(f) = std::fs::File::open(p) else { return };
        let proj = import(f, "sample").unwrap();
        let gifs: Vec<_> = proj.assets.values().filter(|a| a.mime == "image/gif").collect();
        assert!(!gifs.is_empty());
        for g in gifs { assert!(image::load_from_memory(&g.bytes).is_ok()); }
    }

    /// A one-slide package whose story declares one number variable `V`.
    fn slide_with(shapes: &str, layers: &str) -> Project {
        let story = r#"<story><propLst><prop><sz w="960" h="540"/></prop></propLst><varLst><var g="V" name="n" dataType="num" val="0"/></varLst></story>"#;
        let slide = format!(r#"<sld g="S" name="S"><shapeLst>{shapes}</shapeLst><sldLayerLst>{layers}</sldLayerLst></sld>"#);
        import(package(&[("story/story.xml", story), ("story/slides/slide.xml", &slide)]), "t").unwrap()
    }

    fn objects(p: &Project) -> Vec<&Object> {
        p.slides[0].layers.iter().flat_map(|l| l.objects.iter()).collect()
    }

    const LOC: &str = r#"<loc l="0" t="0" r="100" b="50"/>"#;

    #[test]
    fn rotation_is_imported_and_minus_one_means_none() {
        let p = slide_with(&format!(r#"<rect g="a" name="A" rot="90">{LOC}</rect><rect g="b" name="B" rot="-1">{LOC}</rect>"#), "");
        let o = objects(&p);
        assert_eq!((o[0].rotation, o[1].rotation), (90.0, 0.0));
    }

    #[test]
    fn group_member_references_are_not_reported() {
        let p = slide_with(&format!(r#"<group g="G" name="G"><childLst><g>a</g></childLst></group><rect g="a" name="A">{LOC}</rect>"#), "");
        assert!(p.notes.iter().all(|n| !n.contains("no frame")), "{:?}", p.notes);
        assert_eq!(objects(&p).len(), 1);
    }

    #[test]
    fn preset_shapes_use_their_geometry() {
        let p = slide_with(&format!(r#"<triangle g="t" name="T">{LOC}<prstGeom><triangle vertexSet="true" v="0.25"/></prstGeom></triangle>
            <char g="c" name="C" assetG="missing">{LOC}<illConfig characterName="Lily"/></char>"#), "");
        let o = objects(&p);
        match &o[0].kind {
            Kind::Path(c) => assert_eq!(c[0], PathCmd::Move(0.25, 0.0)),
            k => panic!("expected a path, got {k:?}"),
        }
        assert!(matches!(o[1].kind, Kind::Image), "a character is a picture");
        assert!(p.notes.iter().any(|n| n.contains("Lily") && n.contains("EMF")), "{:?}", p.notes);
        assert!(p.notes.iter().all(|n| !n.contains("drawn as a rectangle")));
    }

    fn trigger(event: &str, action: &str, sub: &str, body: &str) -> String {
        format!(r#"<trigLst><trig g="T"><data enabled="true" event="{event}" action="{action}" actSubType="{sub}">{body}</data></trig></trigLst>"#)
    }

    #[test]
    fn actions_are_read_by_what_they_do_not_by_their_sub_type() {
        let shape = |t: String| format!(r#"<rect g="a" name="A">{LOC}{t}</rect>"#);
        let one = |t: String| slide_with(&shape(t), "").slides[0].triggers[0].action.clone();
        for sub in ["spec", "next"] {
            let a = one(trigger("OnClick", "changeShapeState", sub, r#"<shape setStateG="a" setStateName="Hover"/>"#));
            assert!(matches!(&a, Action::SetObjectState { object, state } if object == "a" && state == "Hover"), "{a:?}");
        }
        let a = one(trigger("OnClick", "adjustVar", "next", r#"<other varG="V" op="add" dblVal="2"/>"#));
        assert!(matches!(&a, Action::AdjustVariable { op: AdjustOp::Add, value: Value::Num(v), .. } if *v == 2.0), "{a:?}");
        let a = one(trigger("OnClick", "adjustVar", "spec", r#"<other varG="V" op="minus" dblVal="1"/>"#));
        assert!(matches!(&a, Action::AdjustVariable { op: AdjustOp::Subtract, .. }), "{a:?}");
        let a = one(trigger("OnClick", "jumpToFileUrl", "next", r#"<other open="https://example.com"/>"#));
        assert!(matches!(&a, Action::OpenUrl(u) if u == "https://example.com"), "{a:?}");
    }

    #[test]
    fn hiding_this_layer_points_at_the_layer_the_trigger_is_on() {
        let t = trigger("OnClick", "hideSubSlide", "me", "<sldLayer/>");
        let layer = format!(r#"<sldLayer g="L1" name="L"><shapeLst><rect g="x" name="X">{LOC}{t}</rect></shapeLst></sldLayer>"#);
        let p = slide_with("", &layer);
        assert!(matches!(&p.slides[0].triggers[0].action, Action::HideLayer(l) if l == "L1"), "{:?}", p.slides[0].triggers);
    }

    #[test]
    fn timeline_events_become_cue_points_and_timeline_times() {
        let t = trigger("OnTimelineEvent", "showSubSlide", "spec", r#"<sldLayer showG="L1"/><other time="PT3.5S"/>"#);
        let p = slide_with(&format!(r#"<video g="v" name="V">{LOC}{t}</video>"#), r#"<sldLayer g="L1" name="L"><shapeLst/></sldLayer>"#);
        assert!(matches!(p.slides[0].triggers[0].event, Event::MediaReaches(3500)), "{:?}", p.slides[0].triggers);
    }

    #[test]
    fn picture_crop_becomes_fractions_of_the_source_image() {
        use std::io::Write;
        let mut png = Vec::new();
        image::RgbaImage::new(100, 50).write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
        let sum = hex(&Md5::digest(&png));
        let story = format!(r#"<story><propLst><prop><sz w="960" h="540"/></prop></propLst><mediaLst><media g="M" displayName="p.png"><md5Checksum><stream>{sum}</stream></md5Checksum></media></mediaLst></story>"#);
        let pic = |crop: &str| format!(r#"<pic g="p" name="P" assetG="M">{LOC}{crop}</pic>"#);
        let slide = format!(r#"<sld g="S" name="S"><shapeLst>{}{}</shapeLst><sldLayerLst/></sld>"#,
            pic(r#"<crop scaleX="1" scaleY="1"><rect l="10" t="5" r="90" b="50"/></crop>"#),
            pic(r#"<crop scaleX="1" scaleY="1"><rect l="0" t="0" r="100" b="50"/></crop>"#));
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut buf);
            let o = zip::write::SimpleFileOptions::default();
            for (name, body) in [("story/story.xml", story.as_bytes()), ("story/slides/slide.xml", slide.as_bytes()), ("story/media/p.png", &png[..])] {
                z.start_file(name, o).unwrap();
                z.write_all(body).unwrap();
            }
            z.finish().unwrap();
        }
        buf.set_position(0);
        let p = import(buf, "t").unwrap();
        let o = objects(&p);
        let c = o[0].crop.expect("a cropped picture");
        for (got, want) in c.iter().zip([0.1, 0.1, 0.1, 0.0]) { assert!((got - want).abs() < 1e-4, "{c:?}"); }
        assert_eq!(o[1].crop, None, "a crop that shows the whole picture is no crop");
    }

    /// A one-slide package with two pictures (`A` and `B`) and one number variable `V`.
    fn controls_project(shapes: &str) -> Project {
        use std::io::Write;
        let png = |shade: u8| {
            let mut b = Vec::new();
            image::RgbaImage::from_pixel(4, 2, image::Rgba([shade, 0, 0, 255])).write_to(&mut std::io::Cursor::new(&mut b), image::ImageFormat::Png).unwrap();
            b
        };
        let (a, b) = (png(10), png(200));
        let media = |g: &str, bytes: &[u8]| format!(r#"<media g="{g}" displayName="{g}.png"><md5Checksum><stream>{}</stream></md5Checksum></media>"#, hex(&Md5::digest(bytes)));
        let story = format!(r#"<story><propLst><prop><sz w="960" h="540"/></prop></propLst><varLst><var g="V" name="n" dataType="num" val="9"/></varLst><mediaLst>{}{}</mediaLst></story>"#, media("A", &a), media("B", &b));
        let slide = format!(r#"<sld g="S" name="S"><shapeLst>{shapes}</shapeLst><sldLayerLst/></sld>"#);
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut buf);
            let o = zip::write::SimpleFileOptions::default();
            for (name, body) in [("story/story.xml", story.as_bytes()), ("story/slides/slide.xml", slide.as_bytes()), ("story/media/a.png", &a[..]), ("story/media/b.png", &b[..])] {
                z.start_file(name, o).unwrap();
                z.write_all(body).unwrap();
            }
            z.finish().unwrap();
        }
        buf.set_position(0);
        import(buf, "t").unwrap()
    }

    #[test]
    fn a_slider_keeps_its_range_variable_and_thumb_picture() {
        let slider = format!(r#"<slider g="s" name="Slider" varG="V" action="drag">{LOC}
            <track start="1" end="17"><bg><noFill/><noLine/></bg></track><ticks step="2" style="none"/>
            <stateLst><state name="Normal"><shapeLst><slider name="Slider"><prstGeom><slider thumbH="0.8"/></prstGeom>{LOC}<bG><picFill assetG="A"/></bG></slider></shapeLst></state>
                <state name="Hover"><shapeLst><slider name="Slider"><bG/></slider></shapeLst></state></stateLst>
            {}</slider>"#, trigger("OnSliderMoves", "changeShapeState", "spec", r#"<shape setStateG="s" setStateName="B"/>"#));
        let p = controls_project(&slider);
        let o = &objects(&p)[0];
        let Kind::Slider(sl) = &o.kind else { panic!("a slider, not {:?}", o.kind) };
        assert_eq!((sl.variable.as_str(), sl.min, sl.max, sl.step), ("V", 1.0, 17.0, 2.0));
        assert_eq!(sl.thumb.as_deref(), Some("A"), "the thumb picture comes from the Normal state");
        assert!((sl.thumb_height - 0.8).abs() < 1e-6);
        assert_eq!(sl.track, None, "the track is drawn with no fill and no line");
        assert!(o.states.is_empty(), "the states only describe the thumb");
        // Moving the slider is its variable changing.
        let t = &p.slides[0].triggers[0];
        assert!(matches!(&t.event, Event::VariableChanged(v) if v == "V"), "{:?}", t.event);
        assert!(p.notes.iter().all(|n| !n.contains("not supported")), "{:?}", p.notes);
    }

    #[test]
    fn a_dial_is_an_object_that_turns_and_a_slider_without_a_variable_is_reported() {
        let wheel = format!(r#"<pic g="w" name="Wheel" assetG="A">{LOC}<dialData dialType="Dial" varG="V" increment="1" rotation="270"><values start="1" end="17"/><rotationPoint xOffset="5" yOffset="-3"/></dialData>
            {}</pic>"#, trigger("OnDialTurns", "adjustVar", "spec", r#"<other varG="V" op="ass" dblVal="1"/>"#));
        let plain = format!(r#"<pic g="p" name="Plain" assetG="A">{LOC}<dialData dialType="None" varG="00000000-0000-0000-0000-000000000000"><values start="0" end="12"/></dialData></pic>"#);
        let loose = format!(r#"<slider g="l" name="Loose" varG="00000000-0000-0000-0000-000000000000">{LOC}<track start="0" end="10"/>{}</slider>"#, trigger("OnSliderMoves", "adjustVar", "spec", r#"<other varG="V" op="ass" dblVal="1"/>"#));
        let p = controls_project(&format!("{wheel}{plain}{loose}"));
        let o = objects(&p);
        let d = o[0].dial.as_ref().expect("a dial");
        assert_eq!((d.variable.as_str(), d.min, d.max, d.step, d.sweep, d.pivot), ("V", 1.0, 17.0, 1.0, 270.0, (5.0, -3.0)));
        assert_eq!(o[1].dial, None, "dialType None is an ordinary picture");
        assert!(matches!(&p.slides[0].triggers[0].event, Event::VariableChanged(v) if v == "V"));
        assert_eq!(p.slides[0].triggers.len(), 1, "the loose slider's trigger has nothing to listen to");
        assert!(p.notes.iter().any(|n| n.contains("not linked to a variable")), "{:?}", p.notes);
    }

    #[test]
    fn a_picture_state_can_show_another_picture_at_its_own_size() {
        let state = |name: &str, asset: &str, w: u32| format!(r#"<state name="{name}"><shapeLst><pic name="P" assetG="{asset}"><loc l="0" t="0" r="{w}" b="20"/></pic></shapeLst></state>"#);
        let car = format!(r#"<pic g="c" name="Car" assetG="A" initState="Two">{LOC}<stateLst>{}{}</stateLst></pic>"#, state("One", "A", 40), state("Two", "B", 30));
        let p = controls_project(&car);
        let o = &objects(&p)[0];
        assert_eq!(o.states.len(), 2);
        assert_eq!((o.states[0].asset.as_deref(), o.states[0].size), (Some("A"), Some((40.0, 20.0))));
        assert_eq!((o.states[1].asset.as_deref(), o.states[1].size), (Some("B"), Some((30.0, 20.0))));
        assert_eq!(o.asset.as_deref(), Some("B"), "the object shows the picture of its initial state");
    }

    #[test]
    fn a_check_box_takes_its_label_and_colours_from_its_normal_state_and_state_triggers_fan_out() {
        let inner = |text: &str, fill: &str| format!(r#"<checkBox name="Check Box">{LOC}<text>{text}</text><plain>{text}</plain><margins l="32" t="0" r="0" b="0"/><btnBg>{fill}</btnBg></checkBox>"#);
        let fill = r#"<solidFill><clr><srgbClr val="F0F0F0"/></clr></solidFill><solidLine><clr><srgbClr val="102030"/></clr></solidLine>"#;
        let boxed = |g: &str| format!(r#"<checkBox g="{g}" name="Check Box" initState="Normal">{LOC}<stateLst>
            <checkShapeState name="Normal"><shapeLst>{}</shapeLst></checkShapeState>
            <checkShapeState name="Selected"><shapeLst>{}</shapeLst></checkShapeState></stateLst></checkBox>"#, inner("", fill), inner("", ""));
        let submit = format!(r#"<rect g="go" name="Submit">{LOC}{}</rect>"#, trigger("OnStateChange", "changeShapeState", "next",
            r#"<shape setStateG="go" setStateName="Normal" stateInclude="any" statesNot="are" stateName="Selected"><stateLst><g>c1</g><g>c2</g></stateLst></shape>"#));
        let p = controls_project(&format!("{}{}{submit}", boxed("c1"), boxed("c2")));
        let o = &objects(&p)[0];
        let ck = o.check.as_ref().expect("a check box");
        assert_eq!(ck.box_fill, Rgba(0xF0, 0xF0, 0xF0, 255));
        assert_eq!(ck.box_line, Some(Rgba(0x10, 0x20, 0x30, 255)));
        assert_eq!(o.margins[0], 32.0, "the label starts after the box");
        assert!(o.states.is_empty() && o.fill.is_none(), "the box colours are not the object's fill");
        // "any of c1, c2 are Selected" watches each of them.
        let events: Vec<_> = p.slides[0].triggers.iter().map(|t| format!("{:?}", t.event)).collect();
        assert_eq!(events, ["StateChanged(\"c1\", \"Selected\")", "StateChanged(\"c2\", \"Selected\")"], "{events:?}");
        assert!(p.notes.iter().all(|n| !n.contains("not supported")), "{:?}", p.notes);
    }

    #[test]
    fn freeform_and_oval_hotspots_are_clickable_only_inside_their_outline() {
        // A long straight run of points thins out to its corners; the outline is a right triangle.
        let pts: String = (0..=50).map(|i| format!(r#"<pt x="{}" y="0"/>"#, i as f32 / 50.0)).chain((1..=50).map(|i| format!(r#"<pt x="1" y="{}"/>"#, i as f32 / 50.0))).chain((1..50).map(|i| format!(r#"<pt x="{0}" y="{0}"/>"#, 1.0 - i as f32 / 50.0))).collect();
        let free = format!(r#"<clkAreaFree g="f" name="Free">{LOC}<pts>{pts}</pts></clkAreaFree>"#);
        let oval = format!(r#"<clkAreaOval g="o" name="Oval">{LOC}</clkAreaOval>"#);
        let rect = format!(r#"<clkAreaRect g="r" name="Rect">{LOC}</clkAreaRect>"#);
        let p = controls_project(&format!("{free}{oval}{rect}"));
        let o = objects(&p);
        let tri = o[0].hit_area.as_ref().expect("an outline");
        assert!(tri.len() <= 5, "thinned to its corners: {} points", tri.len());
        let near = |x: f32, y: f32| tri.iter().any(|p| (p.0 - x).abs() < 0.03 && (p.1 - y).abs() < 0.03);
        assert!(near(0.0, 0.0) && near(1.0, 0.0) && near(1.0, 1.0), "the corners are kept: {tri:?}");
        let ell = o[1].hit_area.as_ref().expect("an ellipse");
        assert!(ell.iter().all(|&(x, y)| ((x - 0.5).powi(2) + (y - 0.5).powi(2)).sqrt() <= 0.5001));
        assert_eq!(o[2].hit_area, None, "a rectangle needs no outline");
        assert!(p.notes.iter().all(|n| !n.contains("bounding box")), "{:?}", p.notes);
    }

    #[test]
    fn seeking_and_the_course_actions_are_read() {
        let on = |act: &str, body: &str| format!(r#"<trigLst><trig g="T"><data enabled="true" event="OnClick" action="{act}" actSubType="me">{body}</data></trig></trigLst>"#);
        let seek = |target: &str, secs: &str, extra: &str| on("SeekTimeline", &format!(r#"<SeekTimeline target="{target}" timeSeconds="{secs}" cuePoint="1" variableReference="false" useRelativeTime="false" {extra}/>"#));
        let shape = |g: &str, t: String| format!(r#"<rect g="{g}" name="{g}">{LOC}{t}</rect>"#);
        let xml = [shape("a", seek("L", "2.5", "")), shape("b", seek("00000000-0000-0000-0000-000000000000", "0", "")), shape("c", on("CompleteCourse", "")), shape("d", on("exitProject", "")), shape("e", on("restartProject", "")),
            shape("f", seek("L", "0", r#"useRelativeTime="true""#).replace(r#"useRelativeTime="false" useRelativeTime="true""#, r#"useRelativeTime="true""#))].concat();
        let p = controls_project(&xml);
        let a: Vec<String> = p.slides[0].triggers.iter().map(|t| format!("{:?}", t.action)).collect();
        assert_eq!(a[0], "SeekTimeline { target: \"L\", ms: 2500 }");
        assert!(a[1].starts_with("SeekTimeline { target: \"\", ms: 0 }"), "no target means the layer the trigger sits on: {}", a[1]);
        assert_eq!(&a[2..5], ["CompleteCourse", "ExitCourse", "RestartCourse"]);
        assert!(p.notes.iter().all(|n| !n.contains("not supported")), "{:?}", p.notes);
    }

    #[test]
    fn a_trigger_for_a_deleted_layer_is_left_out_and_mentioned() {
        let show = |g: &str| format!(r#"<trig g="T{g}"><data enabled="true" event="OnClick" action="showSubSlide" actSubType="spec"><sldLayer showG="{g}"/></data></trig>"#);
        let p = slide_with(&format!(r#"<rect g="a" name="a">{LOC}<trigLst>{}{}</trigLst></rect>"#, show("L"), show("GONE")), r#"<sldLayer g="L" name="Layer"><shapeLst/></sldLayer>"#);
        let acts: Vec<String> = p.slides[0].triggers.iter().map(|t| format!("{:?}", t.action)).collect();
        assert_eq!(acts, ["ShowLayer(\"L\")"], "{acts:?}");
        assert!(p.notes.iter().any(|n| n.contains("deleted in the source app")), "{:?}", p.notes);
    }

    #[test]
    fn paragraph_ends_bullets_and_links_are_read_the_way_source_app_shows_them() {
        let doc = r##"<Document><Content>
            <Block><Span Text="Intro with a "><Style FontSize="12"/></Span><Span Text="link"><Style FontSize="12" LinkId="L1"/></Span><Span Text="&#xA;"><Style FontSize="12"/></Span></Block>
            <Block><Span Text="&#xA;"><Style FontSize="12"/></Span></Block>
            <Block><Span Text="Choices:&#xD;&#xA;"><Style FontSize="12"/></Span></Block>
            <Block><Style LeadingMargin="18"><ListStyle ListType="Bullet" BulletChar="&#x2022;"/></Style><Span Text="First&#xA;"><Style FontSize="12" FontIsBold="True"/></Span></Block>
            <Block><Style><ListStyle ListType="Bullet" BulletChar="&#x2022;"/></Style><Span Text="&#xA;"><Style FontSize="12"/></Span></Block>
            <Block><Span Text="Last"><Style FontSize="12"/></Span></Block>
        </Content></Document>"##;
        let links: HashMap<String, String> = [("L1".to_string(), "https://example.com/".to_string())].into();
        let pal: HashMap<String, Rgba> = [("hlink".to_string(), Rgba(1, 2, 3, 255))].into();
        let (text, _, rich) = parse_text_rich_with(doc, &default_style(), 1.0, &pal, &links);
        // The end marker of each paragraph is not a line of its own; a line break before it is (a blank line).
        assert_eq!(text, "Intro with a link\n\nChoices:\n\n• First\n\nLast");
        let paras = rich.expect("runs differ, so the text is rich");
        assert_eq!(paras.len(), 6);
        let plain = |i: usize| paras[i].runs.iter().map(|r| r.text.as_str()).collect::<String>();
        assert_eq!((plain(0).as_str(), plain(1).as_str(), plain(2).as_str(), plain(3).as_str(), plain(4).as_str()), ("Intro with a link", "", "Choices:\n", "• First", ""), "an empty bulleted paragraph gets no bullet");
        let link = paras[0].runs.iter().find(|r| r.text == "link").unwrap();
        assert_eq!(link.link.as_deref(), Some("https://example.com/"));
        assert_eq!((link.underline, link.color), (Some(true), Some(Rgba(1, 2, 3, 255))), "underlined in the theme's link colour");
        assert!(paras[0].runs[0].link.is_none());
    }

    #[test]
    fn a_title_takes_its_size_from_the_layout_placeholder_it_stands_on() {
        let story = r#"<story><propLst><prop><sz w="960" h="540"/></prop></propLst></story>"#;
        let layout = r#"<slideLayout internalG="LAY"><shapeLst><ph g="PH1" vertAlign="m"><bG><defaultFont sz="28" style="Bold"/></bG><ph type="title"/></ph></shapeLst></slideLayout>"#;
        let title = |g: &str| format!(r#"<textBox g="t" name="Title"><bG/><loc l="0" t="0" r="100" b="50"/><text>&lt;Document&gt;&lt;Content&gt;&lt;Block&gt;&lt;Span Text="Hello"/&gt;&lt;/Block&gt;&lt;/Content&gt;&lt;/Document&gt;</text><ph g="{g}" type="title"/></textBox>"#);
        let slide = |g: &str| format!(r#"<sld g="S" name="S" layoutG="LAY"><shapeLst>{}</shapeLst><sldLayerLst/></sld>"#, title(g));
        let k = font_scale_for(960.0, 540.0);
        for (g, size, bold) in [("PH1", 28.0 * k, true), ("OTHER", default_style().size, false)] {
            let p = import(package(&[("story/story.xml", story), ("story/slideLayouts/l.xml", layout), ("story/slides/slide.xml", &slide(g))]), "t").unwrap();
            let o = &p.slides[0].layers[0].objects[0];
            assert!((o.style.size - size).abs() < 0.01 && o.style.bold == bold, "{g}: {} {}", o.style.size, o.style.bold);
        }
    }

    #[test]
    fn the_same_side_rounded_bar_is_rounded_at_the_end_the_layout_says() {
        let xml = r#"<roundSameSideCorner><prstGeom><roundSameSideCorner l="0" r="0.1667"/></prstGeom></roundSameSideCorner>"#;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let cmds = preset_path(doc.root_element(), "roundSameSideCorner", 100.0, 100.0).unwrap();
        // `r` rounds the top pair of corners: the outline starts below the top-left corner, and the bottom corners are square.
        let PathCmd::Move(x0, y0) = cmds[0] else { panic!("starts with a move") };
        assert!(x0 > 0.1 && y0 == 0.0, "the top-left corner is rounded: the outline starts {x0} along the top edge");
        assert!(cmds.iter().any(|c| matches!(c, PathCmd::Line(x, y) if *x == 1.0 && *y == 1.0)) && cmds.iter().any(|c| matches!(c, PathCmd::Line(x, y) if *x == 0.0 && *y == 1.0)), "square bottom corners: {cmds:?}");
    }

    #[test]
    fn a_slide_without_its_own_fill_takes_the_colour_of_its_master() {
        let story = r#"<story><propLst><prop><sz w="960" h="540"/></prop></propLst></story>"#;
        let master = r#"<slideMaster g="M"><clrLst><lt1><srgbClr val="FAFEFF"/></lt1></clrLst><bg><solidFill><clr><schemeClr val="lt1"/><alpha val="71000"/></clr></solidFill></bg></slideMaster>"#;
        let plain = r#"<sld g="S" name="S" fmGuid="M"><bg/><shapeLst/><sldLayerLst/></sld>"#;
        let own = r#"<sld g="S" name="S" fmGuid="M"><bg><solidFill><clr><srgbClr val="102030"/></clr></solidFill></bg><shapeLst/><sldLayerLst/></sld>"#;
        let pic = r#"<sld g="S" name="S" fmGuid="M"><bg><picFill/></bg><shapeLst/><sldLayerLst/></sld>"#;
        let bg = |slide: &str| import(package(&[("story/story.xml", story), ("story/slideMasters/m.xml", master), ("story/slides/slide.xml", slide)]), "t").unwrap().slides[0].background.clone();
        assert!(matches!(bg(plain), Some(brainbend_doc::Background::Solid(c)) if c == Rgba(250, 254, 255, 181)), "{:?}", bg(plain));
        assert!(matches!(bg(own), Some(brainbend_doc::Background::Solid(c)) if c == Rgba(16, 32, 48, 255)));
        assert!(bg(pic).is_none(), "a picture fill is not drawn, and the master's colour must not stand in for it");
    }

    #[test]
    fn an_unnamed_slide_is_called_by_its_title() {
        let title = r#"<textBox g="t" name="Title"><loc l="0" t="0" r="100" b="50"/><ph g="P" type="title"/><plain>Accessible Color Theme 2: Contrast Grid</plain></textBox>"#;
        let body = r#"<textBox g="b" name="Body"><loc l="0" t="60" r="100" b="90"/><plain>Just text</plain></textBox>"#;
        let story = r#"<story><propLst><prop><sz w="960" h="540"/></prop></propLst></story>"#;
        let name = |sld: &str| import(package(&[("story/story.xml", story), ("story/slides/slide.xml", sld)]), "t").unwrap().slides[0].name.clone();
        assert_eq!(name(&format!(r#"<sld g="S" name=""><shapeLst>{body}{title}</shapeLst><sldLayerLst/></sld>"#)), "Accessible Color Theme 2: Contrast Grid");
        assert_eq!(name(&format!(r#"<sld g="S" name="Named"><shapeLst>{title}</shapeLst><sldLayerLst/></sld>"#)), "Named", "a name of its own wins");
        assert_eq!(name(&format!(r#"<sld g="S" name=""><shapeLst>{body}</shapeLst><sldLayerLst/></sld>"#)), "Slide 1", "no title, no name");
    }

    #[test]
    fn a_source_app_table_becomes_a_table_with_per_side_borders_spans_and_margins() {
        let cell = |extra: &str, borders: &str| format!(r#"<tableCell rowSpan="1" columnSpan="1" verticalMerge="false" horizontalMerge="false" {extra}><style><background><fillColor><srgbClr val="FFFFFF"/></fillColor></background>{borders}<margins l="10" t="7" r="10" b="7"/></style></tableCell>"#);
        let solid = |tag: &str, w: u32| format!(r#"<{tag} dashType="Solid" weight="{w}"><themeColor><srgbClr val="212121"/></themeColor></{tag}>"#);
        let custom = |tag: &str| format!(r#"<{tag} dashType="Custom" weight="2"><themeColor><srgbClr val="212121"/></themeColor></{tag}>"#);
        let alike = [solid("leftBorder", 1), solid("rightBorder", 1), solid("topBorder", 1), solid("bottomBorder", 1)].concat();
        let open_corner = [custom("leftBorder"), solid("rightBorder", 2), custom("topBorder"), solid("bottomBorder", 2)].concat();
        let merged = r#"<tableCell rowSpan="2" columnSpan="1" verticalMerge="false" horizontalMerge="false"><style/></tableCell>"#;
        let covered = r#"<tableCell rowSpan="1" columnSpan="1" verticalMerge="true" horizontalMerge="false"><style/></tableCell>"#;
        let table = format!(r#"<table g="T" name="Table"><loc l="10" t="20" r="210" b="120"/><table><columns><tableColumn width="100"/><tableColumn width="100"/></columns><rows>
            <tableRow height="40"><cells>{}{}</cells></tableRow>
            <tableRow height="30"><cells>{merged}{}</cells></tableRow>
            <tableRow height="30"><cells>{covered}{}</cells></tableRow></rows></table></table>"#, cell("", &open_corner), cell("", &alike), cell("", &alike), cell("", &alike));
        let p = slide_with(&table, "");
        let cells = objects(&p);
        assert!(cells.iter().all(|o| o.group.as_deref() == Some("T")), "every cell is in the table's group");
        assert_eq!(cells.len(), 5, "the covered cell has no object of its own");
        let first = cells.iter().find(|o| o.table == Some((0, 0))).unwrap();
        assert_eq!(first.margins, [10.0, 7.0, 10.0, 7.0]);
        let b = first.table_borders.expect("its sides differ");
        assert_eq!((b[0], b[2]), (None, None), "no line on the open sides");
        assert_eq!((b[1].unwrap().width, b[3].unwrap().width), (2.0, 2.0));
        let plain = cells.iter().find(|o| o.table == Some((0, 1))).unwrap();
        assert!(plain.table_borders.is_none() && plain.stroke.map(|s| s.width) == Some(1.0), "four alike sides are just the stroke");
        let tall = cells.iter().find(|o| o.table == Some((1, 0))).unwrap();
        assert_eq!(tall.table_span, Some((2, 1)));
        assert!((tall.h - 60.0).abs() < 0.01);
        // The editor's table tools see it as a table: two columns, three rows.
        let objs: Vec<Object> = cells.iter().map(|o| (*o).clone()).collect();
        assert_eq!(brainbend_doc::tables::dims(&objs, "T"), (3, 2));
    }

    #[test]
    fn theme_colour_variants_are_the_lighter_and_darker_shades_of_the_picker() {
        let accent = Rgba(0x4B, 0xAC, 0xC6, 255);
        let near = |a: Rgba, b: (u8, u8, u8)| (a.0 as i32 - b.0 as i32).abs() <= 2 && (a.1 as i32 - b.1 as i32).abs() <= 2 && (a.2 as i32 - b.2 as i32).abs() <= 2;
        assert!(near(theme_variant(accent, 1), (0xDA, 0xEE, 0xF3)), "{:?}", theme_variant(accent, 1));
        assert!(near(theme_variant(accent, 2), (0xB7, 0xDE, 0xE8)), "{:?}", theme_variant(accent, 2));
        assert!(near(theme_variant(accent, 3), (0x92, 0xCD, 0xDC)), "{:?}", theme_variant(accent, 3));
        assert!(near(theme_variant(accent, 4), (0x31, 0x85, 0x9B)), "{:?}", theme_variant(accent, 4));
        assert!(near(theme_variant(accent, 5), (0x20, 0x58, 0x67)), "{:?}", theme_variant(accent, 5));
        assert_eq!(theme_variant(Rgba(250, 254, 255, 255), 4), Rgba(250, 254, 255, 255), "greys are left alone");
        assert_eq!(theme_variant(accent, 11), accent, "unknown variants change nothing");
        // 6-10 on a dark slate (the theme's dk2): the card behind a slide, Hover, Normal and Selected of its buttons.
        let slate = Rgba(0x24, 0x2E, 0x37, 255);
        assert!(near(theme_variant(slate, 8), (128, 151, 172)) && near(theme_variant(slate, 9), (77, 99, 118)), "{:?}", theme_variant(slate, 9));
        assert_eq!(theme_variant(slate, 10), slate, "10 is the colour itself");
        assert!(theme_variant(slate, 6).0 > 220, "6 is nearly white");
    }

    #[test]
    fn a_hidden_object_starts_off_the_slide_and_hover_triggers_restore_it_when_the_pointer_leaves() {
        let hover = |target: &str| format!(r#"<trigLst><trig g="T"><data enabled="true" event="OnMouseHover" action="changeShapeState" actSubType="next"><shape setStateG="{target}" setStateName="Normal"/><frame restore="true"/></data></trig></trigLst>"#);
        let shapes = format!(r#"<rect g="box" name="box" initState="Normal">{LOC}{}</rect><rect g="tip" name="tip" initState="Hidden">{LOC}</rect>"#, hover("tip"));
        let p = slide_with(&shapes, "");
        let o = objects(&p);
        assert!(!o[0].hidden && o[1].hidden && o[1].initial_state == "Hidden", "the tip starts hidden");
        let acts: Vec<String> = p.slides[0].triggers.iter().map(|t| format!("{:?} {:?}", t.event, t.action)).collect();
        assert_eq!(acts.len(), 2, "{acts:?}");
        assert!(acts[0].starts_with("Hover SetObjectState") && acts[0].contains("\"Normal\""));
        assert!(acts[1].starts_with("MouseLeave SetObjectState") && acts[1].contains("\"Hidden\"") && acts[1].contains("\"tip\""), "back to Hidden: {acts:?}");
    }

    #[test]
    fn a_state_that_does_not_mention_its_fill_keeps_the_objects_own() {
        let solid = r#"<bG><solidFill><clr><srgbClr val="DBEEF4"/></clr></solidFill><noLine/></bG>"#;
        let nested = |bg: &str| format!(r#"<stateLst><state name="Normal"><shapeLst><rect g="in" name="in">{bg}{LOC}</rect></shapeLst></state></stateLst>"#);
        let shape = |states: &str| format!(r#"<rect g="a" name="a" initState="Normal">{states}{solid}{LOC}</rect>"#);
        let p = slide_with(&shape(&nested(r#"<bG><noLine/></bG>"#)), "");
        assert_eq!(objects(&p)[0].fill, Some(Rgba(0xDB, 0xEE, 0xF4, 255)), "the Normal state says nothing about fill, so the shape's own shows");
        let p = slide_with(&shape(&nested(r#"<bG><solidFill><clr><srgbClr val="102030"/></clr></solidFill></bG>"#)), "");
        assert_eq!(objects(&p)[0].fill, Some(Rgba(0x10, 0x20, 0x30, 255)), "a fill the state does set wins");
    }

    #[test]
    fn dotted_and_dashed_outlines_are_kept() {
        let line = |dash: &str| format!(r#"<rect g="a" name="a"><bG><noFill/><solidLine><clr><srgbClr val="FFFFFF"/></clr></solidLine><lineStyle w="3" dash="{dash}"/></bG>{LOC}</rect>"#);
        for (name, want) in [("sqDot", LineDash::Dotted), ("rndDot", LineDash::Dotted), ("dash", LineDash::Dashed), ("lgDash", LineDash::Dashed), ("sld", LineDash::Solid), ("cstm", LineDash::Solid)] {
            let p = slide_with(&line(name), "");
            assert_eq!(objects(&p)[0].dash, want, "{name}");
        }
    }

    #[test]
    fn stop_media_stops_rather_than_pauses() {
        let trig = r#"<trigLst><trig g="T"><data enabled="true" event="OnClick" action="stopMedia" actSubType="spec"><video stopG="V1"/></data></trig></trigLst>"#;
        let p = controls_project(&format!(r#"<rect g="a" name="a">{LOC}{trig}</rect>"#));
        assert_eq!(format!("{:?}", p.slides[0].triggers[0].action), "StopMedia(\"V1\")");
        assert!(p.notes.iter().all(|n| !n.contains("pause") && !n.contains("not supported")), "{:?}", p.notes);
    }

    #[test]
    fn a_character_is_drawn_from_the_metafile_its_slide_lists_and_cropped_to_its_frame() {
        use std::io::Write;
        // Two metafiles, 100 and 200 device pixels wide, both 400 tall. The slide lists the wide one first,
        // but its first character is cropped to 100 pixels wide, so the width decides, not the order.
        let (narrow, wide) = (crate::emf::blank_emf(100, 400), crate::emf::blank_emf(200, 400));
        let char_xml = |g: &str, r: u32| format!(r#"<char g="{g}" assetG="m{g}" name="Character"><loc l="0" t="0" r="50" b="100"/><crop scaleX="0.5" scaleY="0.5"><rect l="0" t="0" r="{r}" b="200"/></crop><illConfig characterName="Lily" expression="Happy"/></char>"#);
        let story = r#"<story><propLst><prop><sz w="960" h="540"/></prop></propLst><mediaLst><media g="ma"/><media g="mb"/></mediaLst></story>"#;
        let slide = format!(r#"<sld g="S" name="S"><shapeLst>{}{}</shapeLst><sldLayerLst/></sld>"#, char_xml("a", 100), char_xml("b", 200));
        let rels = r#"<Relationships><Relationship Type="media" Target="/story/media/wide.emf" Id="1"/><Relationship Type="media" Target="/story/media/narrow.emf" Id="2"/></Relationships>"#;
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut buf);
            let o = zip::write::SimpleFileOptions::default();
            for (name, body) in [("story/story.xml", story.as_bytes()), ("story/slides/slide.xml", slide.as_bytes()), ("story/slides/_rels/slide.xml.rels", rels.as_bytes()), ("story/media/wide.emf", &wide[..]), ("story/media/narrow.emf", &narrow[..])] {
                z.start_file(name, o).unwrap();
                z.write_all(body).unwrap();
            }
            z.finish().unwrap();
        }
        buf.set_position(0);
        let p = import(buf, "t").unwrap();
        let o = objects(&p);
        let size_of = |o: &Object| image_size(&p.assets[o.asset.as_deref().unwrap()].bytes).unwrap();
        // The drawn picture has the metafile's own shape: 1800 pixels on its longer side.
        assert_eq!(size_of(&o[0]), (450.0, 1800.0), "the first character got the 100-wide metafile");
        assert_eq!(size_of(&o[1]), (900.0, 1800.0), "the second got the 200-wide one");
        assert_eq!(o[0].crop.map(|c| c.map(|v| (v * 100.0).round())), Some([0.0, 0.0, 0.0, 50.0]), "the lower half is hidden");
        assert_eq!(o[0].alt, "Lily, Happy");
        assert!(p.notes.iter().all(|n| !n.contains("character")), "{:?}", p.notes);
    }
}
