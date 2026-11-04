//! Writes a BrainBend Studio project as a PowerPoint `.pptx` (Office Open XML, ECMA-376).
//!
//! The theme, slide master, layouts, backgrounds, placeholders, shapes, pictures, media, text,
//! notes, sections, fades and click links are written natively. What PowerPoint has no equivalent
//! for (layers, triggers other than links, variables, quiz scoring, animations) is listed in the
//! returned notes.
mod anim;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Write as _;
use brainbend_doc::*;

/// EMU per slide unit: a 1920-unit-wide slide is 12192000 EMU (13.333 in) wide.
const E: f32 = 6350.0;
const NS: &str = r#"xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main""#;
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

fn emu(v: f32) -> i64 {
    (v * E).round() as i64
}

fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            c if (c as u32) < 0x20 && c != '\t' => {}
            c => o.push(c),
        }
    }
    o
}

fn hex(c: Rgba) -> String {
    format!("{:02X}{:02X}{:02X}", c.0, c.1, c.2)
}

fn color_xml(c: Rgba) -> String {
    if c.3 == 255 { format!("<a:srgbClr val=\"{}\"/>", hex(c)) } else { format!("<a:srgbClr val=\"{}\"><a:alpha val=\"{}\"/></a:srgbClr>", hex(c), (c.3 as f32 / 255.0 * 100000.0).round() as i32) }
}

fn solid(c: Rgba) -> String {
    format!("<a:solidFill>{}</a:solidFill>", color_xml(c))
}

fn gradient_xml(angle: f32, stops: &[(f32, Rgba)]) -> String {
    let gs: String = stops.iter().map(|(p, c)| format!("<a:gs pos=\"{}\">{}</a:gs>", (p * 100000.0).round() as i32, color_xml(*c))).collect();
    // CSS 90deg points right; DrawingML 0 points right.
    let ang = ((angle - 90.0).rem_euclid(360.0) * 60000.0).round() as i64;
    format!("<a:gradFill rotWithShape=\"1\"><a:gsLst>{gs}</a:gsLst><a:lin ang=\"{ang}\" scaled=\"0\"/></a:gradFill>")
}

/// Relationships of one part.
#[derive(Default)]
struct Rels {
    items: Vec<(String, String, String, bool)>,
}

impl Rels {
    fn add(&mut self, ty: &str, target: &str, external: bool) -> String {
        if let Some(i) = self.items.iter().find(|i| i.1 == ty && i.2 == target) { return i.0.clone(); }
        let id = format!("rId{}", self.items.len() + 1);
        self.items.push((id.clone(), ty.to_string(), target.to_string(), external));
        id
    }
    fn xml(&self) -> String {
        let mut s = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">");
        for (id, ty, target, ext) in &self.items {
            let ty = if ty.starts_with("http") { ty.clone() } else { format!("{R}/{ty}") };
            let _ = write!(s, "<Relationship Id=\"{id}\" Type=\"{ty}\" Target=\"{}\"{}/>", esc(target), if *ext { " TargetMode=\"External\"" } else { "" });
        }
        s.push_str("</Relationships>");
        s
    }
}

struct Package {
    files: Vec<(String, Vec<u8>)>,
    /// (part name, content type)
    overrides: Vec<(String, String)>,
    media: BTreeMap<String, String>,
    exts: std::collections::BTreeSet<String>,
}

impl Package {
    fn part(&mut self, name: &str, content_type: &str, xml: String) {
        self.files.push((name.to_string(), format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n{xml}").into_bytes()));
        self.overrides.push((format!("/{name}"), content_type.to_string()));
    }
    fn rels(&mut self, for_part: &str, rels: &Rels) {
        let (dir, file) = for_part.rsplit_once('/').unwrap_or(("", for_part));
        let path = if dir.is_empty() { format!("_rels/{file}.rels") } else { format!("{dir}/_rels/{file}.rels") };
        self.files.push((path, rels.xml().into_bytes()));
    }
    /// Store an asset once; returns its part name (`ppt/media/...`).
    fn media(&mut self, key: &str, a: &Asset) -> String {
        if let Some(n) = self.media.get(key) { return n.clone(); }
        let ext = match a.mime.as_str() {
            "image/png" => "png", "image/jpeg" => "jpg", "image/gif" => "gif", "image/svg+xml" => "svg",
            "video/mp4" => "mp4", "video/webm" => "webm", "audio/mpeg" => "mp3", "audio/mp4" => "m4a", "audio/wav" => "wav", "audio/ogg" => "ogg",
            _ => "bin",
        };
        let name = format!("ppt/media/media{}.{ext}", self.media.len() + 1);
        self.files.push((name.clone(), a.bytes.clone()));
        self.exts.insert(ext.to_string());
        self.media.insert(key.to_string(), name.clone());
        name
    }
}

fn content_type_for(ext: &str) -> &'static str {
    match ext {
        "png" => "image/png", "jpg" => "image/jpeg", "gif" => "image/gif", "svg" => "image/svg+xml",
        "mp4" => "video/mp4", "webm" => "video/webm", "mp3" => "audio/mpeg", "m4a" => "audio/mp4", "wav" => "audio/wav", "ogg" => "audio/ogg",
        _ => "application/octet-stream",
    }
}

/// A small grey picture PowerPoint shows for media before they play.
fn poster_png() -> Vec<u8> {
    let img = image::RgbaImage::from_pixel(16, 9, image::Rgba([40, 40, 40, 255]));
    let mut buf = std::io::Cursor::new(Vec::new());
    let _ = image::DynamicImage::ImageRgba8(img).write_to(&mut buf, image::ImageFormat::Png);
    buf.into_inner()
}

struct Notes(Vec<String>);

impl Notes {
    fn add(&mut self, m: impl Into<String>) {
        let m = m.into();
        if !self.0.contains(&m) { self.0.push(m); }
    }
}

/// Per-part writing state: relationships and shape ids.
struct Part<'a> {
    pkg: &'a mut Package,
    project: &'a Project,
    rels: Rels,
    next_id: u32,
    notes: &'a mut Notes,
    /// Slide part number for each slide id (for jump links).
    slide_no: &'a BTreeMap<String, usize>,
    /// Layout placeholders: how many of each kind were written (to assign idx values).
    ph_count: [u32; 3],
    /// Links to write on objects: object id -> action.
    links: BTreeMap<String, Action>,
    /// Object id -> the shape id written for it (animations target shapes by id).
    spids: BTreeMap<String, u32>,
}

fn ph_idx(kind: PlaceholderKind, n: u32) -> (Option<&'static str>, Option<u32>) {
    match kind {
        PlaceholderKind::Title => (Some("title"), None),
        PlaceholderKind::Subtitle => (Some("subTitle"), Some(20 + n)),
        PlaceholderKind::Body => (Some("body"), Some(10 + n)),
    }
}

impl Part<'_> {
    fn id(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id
    }

    fn media_target(&mut self, key: &str, from_dir_up: bool) -> Option<String> {
        let a = self.project.assets.get(key)?;
        let name = self.pkg.media(key, a);
        let _ = from_dir_up;
        Some(format!("../{}", name.trim_start_matches("ppt/")))
    }

    fn blip(&mut self, key: &str) -> Option<String> {
        let t = self.media_target(key, true)?;
        Some(self.rels.add("image", &t, false))
    }

    fn background(&mut self, bg: &Background) -> String {
        let fill = match bg {
            Background::Solid(c) => solid(*c),
            Background::Gradient { angle, stops } => gradient_xml(*angle, stops),
            Background::Image { asset } => match self.blip(asset) {
                Some(rid) => format!("<a:blipFill dpi=\"0\" rotWithShape=\"1\"><a:blip r:embed=\"{rid}\"/><a:srcRect/><a:stretch><a:fillRect/></a:stretch></a:blipFill>"),
                None => solid(Rgba(255, 255, 255, 255)),
            },
        };
        format!("<p:bg><p:bgPr>{fill}<a:effectLst/></p:bgPr></p:bg>")
    }

    fn link_xml(&mut self, obj: &Object) -> String {
        let Some(action) = self.links.get(&obj.id).cloned() else { return String::new() };
        match action {
            Action::JumpToSlide(id) => match self.slide_no.get(&id) {
                Some(n) => { let rid = self.rels.add("slide", &format!("slide{n}.xml"), false); format!("<a:hlinkClick r:id=\"{rid}\" action=\"ppaction://hlinksldjump\"/>") }
                None => String::new(),
            },
            Action::NextSlide => "<a:hlinkClick r:id=\"\" action=\"ppaction://hlinkshowjump?jump=nextslide\"/>".into(),
            Action::PreviousSlide => "<a:hlinkClick r:id=\"\" action=\"ppaction://hlinkshowjump?jump=previousslide\"/>".into(),
            Action::RunJavaScript(js) => match js.split('"').nth(1).filter(|u| js.starts_with("window.open(") && (u.starts_with("http://") || u.starts_with("https://") || u.starts_with("mailto:"))) {
                Some(url) => { let rid = self.rels.add("hyperlink", url, true); format!("<a:hlinkClick r:id=\"{rid}\"/>") }
                None => { self.notes.add("JavaScript actions are not exported."); String::new() }
            },
            _ => { self.notes.add("Triggers other than links to slides and web pages are not exported."); String::new() }
        }
    }

    fn cnv_pr(&mut self, o: &Object) -> String {
        let id = self.id();
        self.spids.insert(o.id.clone(), id);
        let link = self.link_xml(o);
        let (hidden, descr) = (if o.hidden { " hidden=\"1\"" } else { "" }, { let d = if o.alt.is_empty() { o.equation.as_ref().map(|e| format!("Equation: {}", e.latex)).unwrap_or_default() } else { o.alt.clone() }; if d.is_empty() { String::new() } else { format!(" descr=\"{}\"", esc(&d)) } });
        if link.is_empty() { format!("<p:cNvPr id=\"{id}\" name=\"{}\"{descr}{hidden}/>", esc(&o.name)) } else { format!("<p:cNvPr id=\"{id}\" name=\"{}\"{descr}{hidden}>{link}</p:cNvPr>", esc(&o.name)) }
    }

    fn xfrm(&self, o: &Object, flip_v: bool) -> String {
        let rot = if o.rotation.abs() > 0.01 { format!(" rot=\"{}\"", (o.rotation.rem_euclid(360.0) * 60000.0).round() as i64) } else { String::new() };
        format!("<a:xfrm{rot}{}><a:off x=\"{}\" y=\"{}\"/><a:ext cx=\"{}\" cy=\"{}\"/></a:xfrm>", if flip_v { " flipV=\"1\"" } else { "" }, emu(o.x), emu(o.y), emu(o.w).max(0), emu(o.h).max(0))
    }

    fn fill_xml(&self, o: &Object) -> String {
        if let Some(g) = o.gradient.as_ref().filter(|g| g.stops.len() >= 2) { return gradient_xml(g.angle, &g.stops); }
        match o.fill { Some(c) => solid(c), None => "<a:noFill/>".into() }
    }

    fn line_xml(&self, o: &Object) -> String {
        match o.stroke {
            Some(s) => {
                let dash = match o.dash { LineDash::Solid => "", LineDash::Dotted => "<a:prstDash val=\"sysDot\"/>", LineDash::Dashed => "<a:prstDash val=\"dash\"/>" };
                format!("<a:ln w=\"{}\">{}{dash}</a:ln>", emu(s.width).max(1), solid(s.color))
            }
            None => "<a:ln><a:noFill/></a:ln>".into(),
        }
    }

    fn effects_xml(&self, o: &Object) -> String {
        // The order PowerPoint expects: glow, shadow, reflection, soft edges.
        let mut x = String::new();
        if let Some(g) = o.glow { let _ = write!(x, "<a:glow rad=\"{}\">{}</a:glow>", emu(g.radius), color_xml(g.color)); }
        if let Some(s) = o.shadow {
            let dist = (s.dx * s.dx + s.dy * s.dy).sqrt();
            let dir = if dist > 0.0 { s.dy.atan2(s.dx).to_degrees().rem_euclid(360.0) } else { 0.0 };
            let _ = write!(x, "<a:outerShdw blurRad=\"{}\" dist=\"{}\" dir=\"{}\" algn=\"ctr\" rotWithShape=\"0\">{}</a:outerShdw>", emu(s.blur), emu(dist), (dir * 60000.0).round() as i64, color_xml(s.color));
        }
        if let Some(r) = o.reflection {
            let _ = write!(x, "<a:reflection blurRad=\"0\" stA=\"{}\" endA=\"0\" endPos=\"{}\" dist=\"{}\" dir=\"5400000\" sy=\"-100000\" algn=\"bl\" rotWithShape=\"0\"/>", (r.opacity.clamp(0.0, 1.0) * 100000.0).round() as i64, (r.size.clamp(0.0, 1.0) * 100000.0).round() as i64, emu(r.gap));
        }
        if o.soft_edge > 0.0 { let _ = write!(x, "<a:softEdge rad=\"{}\"/>", emu(o.soft_edge)); }
        if x.is_empty() { x } else { format!("<a:effectLst>{x}</a:effectLst>") }
    }

    fn geometry_xml(&mut self, o: &Object) -> String {
        match &o.kind {
            Kind::Ellipse => "<a:prstGeom prst=\"ellipse\"><a:avLst/></a:prstGeom>".into(),
            Kind::Path(cmds) => {
                let p = |x: f32| (x * 100000.0).round() as i64;
                let mut d = String::new();
                for c in cmds {
                    match c {
                        PathCmd::Move(x, y) => { let _ = write!(d, "<a:moveTo><a:pt x=\"{}\" y=\"{}\"/></a:moveTo>", p(*x), p(*y)); }
                        PathCmd::Line(x, y) => { let _ = write!(d, "<a:lnTo><a:pt x=\"{}\" y=\"{}\"/></a:lnTo>", p(*x), p(*y)); }
                        PathCmd::Curve(a, b, c2, d2, e, f) => { let _ = write!(d, "<a:cubicBezTo><a:pt x=\"{}\" y=\"{}\"/><a:pt x=\"{}\" y=\"{}\"/><a:pt x=\"{}\" y=\"{}\"/></a:cubicBezTo>", p(*a), p(*b), p(*c2), p(*d2), p(*e), p(*f)); }
                        PathCmd::Close => d.push_str("<a:close/>"),
                    }
                }
                format!("<a:custGeom><a:avLst/><a:gdLst/><a:ahLst/><a:cxnLst/><a:rect l=\"0\" t=\"0\" r=\"r\" b=\"b\"/><a:pathLst><a:path w=\"100000\" h=\"100000\">{d}</a:path></a:pathLst></a:custGeom>")
            }
            _ if o.radius > 0.5 => {
                let adj = (o.radius / o.w.min(o.h).max(1.0) * 100000.0).round().clamp(0.0, 50000.0) as i64;
                format!("<a:prstGeom prst=\"roundRect\"><a:avLst><a:gd name=\"adj\" fmla=\"val {adj}\"/></a:avLst></a:prstGeom>")
            }
            _ => "<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom>".into(),
        }
    }

    fn rpr(&mut self, tag: &str, base: &TextStyle, run: Option<&Run>) -> String {
        let size = run.and_then(|r| r.size).unwrap_or(base.size);
        let bold = run.and_then(|r| r.bold).unwrap_or(base.bold);
        let italic = run.and_then(|r| r.italic).unwrap_or(base.italic);
        let under = run.and_then(|r| r.underline).unwrap_or(base.underline);
        let col = run.and_then(|r| r.color).unwrap_or(base.color);
        let strike = run.and_then(|r| r.strike).unwrap_or(false);
        let spc = run.and_then(|r| r.spacing).map(|z| format!(" spc=\"{}\"", (z * 0.5 * 100.0).round() as i64)).unwrap_or_default();
        let hl = run.and_then(|r| r.highlight).map(|c| format!("<a:highlight><a:srgbClr val=\"{:02X}{:02X}{:02X}\"/></a:highlight>", c.0, c.1, c.2)).unwrap_or_default();
        let font = run.and_then(|r| r.font_family.clone()).unwrap_or_else(|| base.font_family.clone());
        let link = match run.and_then(|r| r.link.as_deref()).filter(|u| u.starts_with("http://") || u.starts_with("https://") || u.starts_with("mailto:")) {
            // PowerPoint draws links in the theme's hyperlink colour unless the click asks for the text's own colour.
            Some(url) if run.is_some_and(|r| r.color.is_some()) => { let rid = self.rels.add("hyperlink", url, true); format!("<a:hlinkClick r:id=\"{rid}\"><a:extLst><a:ext uri=\"{{A12FA001-AC4F-418D-AE19-62706E023703}}\"><ahyp:hlinkClr xmlns:ahyp=\"http://schemas.microsoft.com/office/drawing/2018/hyperlinkcolor\" val=\"tx\"/></a:ext></a:extLst></a:hlinkClick>") }
            Some(url) => { let rid = self.rels.add("hyperlink", url, true); format!("<a:hlinkClick r:id=\"{rid}\"/>") }
            None => String::new(),
        };
        format!(
            "<a:{tag} lang=\"en-US\" sz=\"{}\" b=\"{}\" i=\"{}\"{}{}{} dirty=\"0\">{}{hl}<a:latin typeface=\"{f}\"/><a:cs typeface=\"{f}\"/>{link}</a:{tag}>",
            ((size / 2.0 * 100.0).round() as i64).clamp(100, 400000), bold as u8, italic as u8, if under { " u=\"sng\"" } else { "" }, if strike { " strike=\"sngStrike\"" } else { "" }, spc, solid(col), f = esc(&font)
        )
    }

    fn text_body(&mut self, o: &Object, tag: &str) -> String {
        let anchor = match o.style.valign { VAlign::Top => "t", VAlign::Middle => "ctr", VAlign::Bottom => "b" };
        let vert = match o.text_rotation { 90 => " vert=\"vert\"", 270 => " vert=\"vert270\"", _ => "" };
        let m = o.margins;
        let mut s = format!(
            "<p:{tag}><a:bodyPr wrap=\"square\" lIns=\"{}\" tIns=\"{}\" rIns=\"{}\" bIns=\"{}\" rtlCol=\"0\" anchor=\"{anchor}\"{vert}><a:noAutofit/></a:bodyPr><a:lstStyle/>",
            emu(m[0]), emu(m[1]), emu(m[2]), emu(m[3])
        );
        let algn = match o.style.align { Align::Left => "l", Align::Center => "ctr", Align::Right => "r", Align::Justify => "just" };
        let ppr = {
            let ln = if o.line_spacing > 0.0 { format!("<a:lnSpc><a:spcPct val=\"{}\"/></a:lnSpc>", (o.line_spacing / 1.2 * 100000.0).round() as i64) } else { String::new() };
            format!("<a:pPr algn=\"{algn}\">{ln}</a:pPr>")
        };
        let paras: Vec<(u8, Vec<Run>)> = match &o.rich {
            Some(p) => p.iter().map(|p| (p.indent, p.runs.clone())).collect(),
            None => o.text.split('\n').map(|l| (0, vec![Run { text: l.to_string(), ..Default::default() }])).collect(),
        };
        for (indent, runs) in paras {
            s.push_str("<a:p>");
            // A list item hangs: its marker at the indent, wrapped lines in line with the text after it.
            let plain: String = runs.iter().map(|r| r.text.as_str()).collect();
            let m = brainbend_doc::richtext::marker_len(&plain);
            let hang = if m > 0 && plain.len() > m { o.style.size * if plain.trim_start_matches('\u{a0}').starts_with('•') { 1.3 } else { 1.6 } } else { 0.0 };
            let at = indent as f32 * brainbend_doc::INDENT_STEP;
            if indent > 0 || hang > 0.0 {
                s.push_str(&ppr.replacen("<a:pPr ", &format!("<a:pPr marL=\"{}\" indent=\"{}\" ", emu(at + hang), -emu(hang)), 1));
            } else { s.push_str(&ppr); }
            let mut any = false;
            for r in &runs {
                for (i, piece) in r.text.split('\n').enumerate() {
                    if i > 0 { let _ = write!(s, "<a:br>{}</a:br>", self.rpr("rPr", &o.style, Some(r))); }
                    if piece.is_empty() { continue; }
                    let _ = write!(s, "<a:r>{}<a:t>{}</a:t></a:r>", self.rpr("rPr", &o.style, Some(r)), esc(piece));
                    any = true;
                }
            }
            if !any { s.push_str(&self.rpr("endParaRPr", &o.style, None)); }
            s.push_str("</a:p>");
        }
        let _ = write!(s, "</p:{tag}>");
        s
    }

    /// One object as a shape element. `ph` writes it as a placeholder of the layout (template mode
    /// when `template` is set, so its text is the prompt).
    fn shape(&mut self, o: &Object, template: bool, out: &mut String) {
        let has_text = !o.text.is_empty() || template;
        if o.marker.is_some() { self.notes.add("Marker pop-ups (the title on hover and the description card) were left out; the marker itself is a plain shape."); }
        if o.check.is_some() { self.notes.add("Check boxes are exported as text; the box itself is left out."); }
        if o.dial.is_some() { self.notes.add("Dials are exported as ordinary objects; they do not turn in PowerPoint."); }
        match &o.kind {
            Kind::Unsupported(_) => { self.notes.add("Objects the importer could not convert were left out."); return; }
            Kind::Slider(_) => { self.notes.add("Sliders were left out; PowerPoint has no slider."); return; }
            Kind::Input(_) => { self.notes.add("Text and number entry fields were left out; PowerPoint has no fields that keep what is typed."); return; }
            Kind::Web(_) => { self.notes.add("Web objects were left out; PowerPoint cannot show a live web page in a slide."); return; }
            Kind::Pano(_) => { self.notes.add("360° images were left out; PowerPoint cannot show a picture you can look around in."); return; }
            Kind::Line | Kind::LineUp => {
                let cnv = self.cnv_pr(o);
                let ln = match o.stroke { Some(s) => format!("<a:ln w=\"{}\">{}</a:ln>", emu(s.width).max(1), solid(s.color)), None => "<a:ln w=\"12700\"><a:solidFill><a:srgbClr val=\"000000\"/></a:solidFill></a:ln>".into() };
                let _ = write!(out, "<p:cxnSp><p:nvCxnSpPr>{cnv}<p:cNvCxnSpPr/><p:nvPr/></p:nvCxnSpPr><p:spPr>{}<a:prstGeom prst=\"line\"><a:avLst/></a:prstGeom>{ln}{}</p:spPr></p:cxnSp>", self.xfrm(o, matches!(o.kind, Kind::LineUp)), self.effects_xml(o));
                return;
            }
            Kind::Image => {
                if let Some(rid) = o.asset.as_deref().and_then(|k| self.blip(k)) {
                    let cnv = self.cnv_pr(o);
                    let ln = if o.stroke.is_some() { self.line_xml(o) } else { String::new() };
                    // The crop as PowerPoint stores it: thousandths of a percent cut from each edge.
                    let src_rect = o.crop.map(|c| format!("<a:srcRect l=\"{}\" t=\"{}\" r=\"{}\" b=\"{}\"/>", (c[0] * 100000.0).round() as i64, (c[1] * 100000.0).round() as i64, (c[2] * 100000.0).round() as i64, (c[3] * 100000.0).round() as i64)).unwrap_or_default();
                    let _ = write!(out, "<p:pic><p:nvPicPr>{cnv}<p:cNvPicPr><a:picLocks noChangeAspect=\"1\"/></p:cNvPicPr><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed=\"{rid}\"/>{src_rect}<a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr>{}<a:prstGeom prst=\"{}\"><a:avLst/></a:prstGeom>{ln}{}</p:spPr></p:pic>", self.xfrm(o, false), o.mask.map(|m| m.preset()).unwrap_or("rect"), self.effects_xml(o));
                }
                return;
            }
            Kind::Video | Kind::Audio => {
                let Some(a) = o.asset.as_deref().and_then(|k| self.project.assets.get(k).map(|a| (k.to_string(), a.clone()))) else { return };
                let name = self.pkg.media(&a.0, &a.1);
                let target = format!("../{}", name.trim_start_matches("ppt/"));
                let poster = Asset { name: "poster.png".into(), mime: "image/png".into(), bytes: poster_png(), file: None };
                let pname = self.pkg.media("__poster", &poster);
                let poster_rid = self.rels.add("image", &format!("../{}", pname.trim_start_matches("ppt/")), false);
                let is_video = matches!(o.kind, Kind::Video);
                let link_rid = self.rels.add(if is_video { "video" } else { "audio" }, &target, false);
                let media_rid = self.rels.add("http://schemas.microsoft.com/office/2007/relationships/media", &target, false);
                let cnv = self.cnv_pr(o);
                let file = if is_video { format!("<a:videoFile r:link=\"{link_rid}\"/>") } else { format!("<a:audioFile r:link=\"{link_rid}\"/>") };
                let _ = write!(out, "<p:pic><p:nvPicPr>{cnv}<p:cNvPicPr><a:picLocks noChangeAspect=\"1\"/></p:cNvPicPr><p:nvPr>{file}<p:extLst><p:ext uri=\"{{DAA4B4D4-6D71-4841-9C94-3DE7FCFB9230}}\"><p14:media xmlns:p14=\"http://schemas.microsoft.com/office/powerpoint/2010/main\" r:embed=\"{media_rid}\"/></p:ext></p:extLst></p:nvPr></p:nvPicPr><p:blipFill><a:blip r:embed=\"{poster_rid}\"/><a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr>{}<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></p:spPr></p:pic>", self.xfrm(o, false));
                return;
            }
            _ => {}
        }
        let cnv = self.cnv_pr(o);
        let ph = match o.placeholder {
            Some(k) => {
                let slot = match k { PlaceholderKind::Title => 0, PlaceholderKind::Subtitle => 1, PlaceholderKind::Body => 2 };
                let n = self.ph_count[slot];
                self.ph_count[slot] += 1;
                let (ty, idx) = ph_idx(k, n);
                format!("<p:ph{}{}/>", ty.map(|t| format!(" type=\"{t}\"")).unwrap_or_default(), idx.map(|i| format!(" idx=\"{i}\"")).unwrap_or_default())
            }
            None => String::new(),
        };
        let tx_box = if o.placeholder.is_none() && matches!(o.kind, Kind::Text) { " txBox=\"1\"" } else { "" };
        let geom = self.geometry_xml(o);
        let (fill, line) = if o.placeholder.is_some() && o.fill.is_none() && o.gradient.is_none() && o.stroke.is_none() { (String::new(), String::new()) } else { (self.fill_xml(o), self.line_xml(o)) };
        let body = if has_text || o.placeholder.is_some() || matches!(o.kind, Kind::Rect | Kind::Ellipse | Kind::Text) { self.text_body(o, "txBody") } else { String::new() };
        let _ = write!(out, "<p:sp><p:nvSpPr>{cnv}<p:cNvSpPr{tx_box}/><p:nvPr>{ph}</p:nvPr></p:nvSpPr><p:spPr>{}{geom}{fill}{line}{}</p:spPr>{body}</p:sp>", self.xfrm(o, false), self.effects_xml(o));
    }

    /// A table (a complete grid of cells sharing a group) as a native PowerPoint table.
    /// `None` when the members are not exactly one table, so they are written as a plain group.
    fn table_frame(&mut self, members: &[Object], group: &str) -> Option<String> {
        use brainbend_doc::tables as t;
        if members.is_empty() || members.iter().any(|o| o.table.is_none()) { return None; }
        let (rows, cols) = t::dims(members, group);
        let covered: usize = members.iter().map(|o| { let s = t::cell_span(o); s.0 as usize * s.1 as usize }).sum();
        if rows == 0 || cols == 0 || covered != rows as usize * cols as usize { return None; }
        let (xs, ys) = t::grid_edges(members, group);
        let id = self.id();
        let (x0, y0) = (xs[0], ys[0]);
        let mut s = format!(
            "<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id=\"{id}\" name=\"Table {id}\"/><p:cNvGraphicFramePr><a:graphicFrameLocks noGrp=\"1\"/></p:cNvGraphicFramePr><p:nvPr/></p:nvGraphicFramePr>\
<p:xfrm><a:off x=\"{}\" y=\"{}\"/><a:ext cx=\"{}\" cy=\"{}\"/></p:xfrm><a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/table\"><a:tbl><a:tblPr/><a:tblGrid>",
            emu(x0), emu(y0), emu(xs[cols as usize] - x0), emu(ys[rows as usize] - y0)
        );
        for c in 0..cols as usize { let _ = write!(s, "<a:gridCol w=\"{}\"/>", emu(xs[c + 1] - xs[c]).max(1)); }
        s.push_str("</a:tblGrid>");
        for r in 0..rows {
            let _ = write!(s, "<a:tr h=\"{}\">", emu(ys[r as usize + 1] - ys[r as usize]).max(1));
            for c in 0..cols {
                let Some(o) = t::cell_at(members, group, r, c).map(|k| &members[k]) else { s.push_str("<a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:endParaRPr lang=\"en-US\"/></a:p></a:txBody><a:tcPr/></a:tc>"); continue };
                let (or, oc) = o.table.unwrap_or((0, 0));
                let (rs, cs) = t::cell_span(o);
                if (or, oc) != (r, c) {
                    // Covered by a merged cell.
                    let flags = format!("{}{}", if c > oc { " hMerge=\"1\"" } else { "" }, if r > or { " vMerge=\"1\"" } else { "" });
                    let _ = write!(s, "<a:tc{flags}><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:endParaRPr lang=\"en-US\"/></a:p></a:txBody><a:tcPr/></a:tc>");
                    continue;
                }
                let tb = self.text_body(o, "txBody");
                let inner = tb.find("<a:lstStyle/>").map(|i| &tb[i + "<a:lstStyle/>".len()..tb.len() - "</p:txBody>".len()]).unwrap_or("<a:p/>");
                let anchor = match o.style.valign { VAlign::Top => "t", VAlign::Middle => "ctr", VAlign::Bottom => "b" };
                let m = o.margins;
                let sd = t::sides(o);
                let line = |side: &str| match sd[match side { "lnL" => 0, "lnR" => 1, "lnT" => 2, _ => 3 }] {
                    Some(st) => format!("<a:{side} w=\"{}\">{}</a:{side}>", emu(st.width).max(1), solid(st.color)),
                    None => format!("<a:{side} w=\"0\"><a:noFill/></a:{side}>"),
                };
                let spans = format!("{}{}", if cs > 1 { format!(" gridSpan=\"{cs}\"") } else { String::new() }, if rs > 1 { format!(" rowSpan=\"{rs}\"") } else { String::new() });
                let _ = write!(
                    s, "<a:tc{spans}><a:txBody><a:bodyPr/><a:lstStyle/>{inner}</a:txBody><a:tcPr marL=\"{}\" marR=\"{}\" marT=\"{}\" marB=\"{}\" anchor=\"{anchor}\">{}{}{}{}{}</a:tcPr></a:tc>",
                    emu(m[0]), emu(m[2]), emu(m[1]), emu(m[3]), line("lnL"), line("lnR"), line("lnT"), line("lnB"), self.fill_xml(o)
                );
            }
            s.push_str("</a:tr>");
        }
        s.push_str("</a:tbl></a:graphicData></a:graphic></p:graphicFrame>");
        Some(s)
    }

    fn tree(&mut self, objects: &[Object], template: bool) -> String {
        let mut s = String::from("<p:spTree><p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"0\" cy=\"0\"/><a:chOff x=\"0\" y=\"0\"/><a:chExt cx=\"0\" cy=\"0\"/></a:xfrm></p:grpSpPr>");
        let mut i = 0;
        while i < objects.len() {
            let Some(g) = objects[i].group.clone() else { self.shape(&objects[i], template, &mut s); i += 1; continue };
            let mut j = i;
            while j < objects.len() && objects[j].group.as_deref() == Some(g.as_str()) { j += 1; }
            let members = &objects[i..j];
            if let Some(frame) = self.table_frame(members, &g) { s.push_str(&frame); i = j; continue; }
            let (x0, y0) = (members.iter().map(|o| o.x).fold(f32::MAX, f32::min), members.iter().map(|o| o.y).fold(f32::MAX, f32::min));
            let (x1, y1) = (members.iter().map(|o| o.x + o.w).fold(f32::MIN, f32::max), members.iter().map(|o| o.y + o.h).fold(f32::MIN, f32::max));
            let id = self.id();
            let (off, ext) = (format!("<a:off x=\"{}\" y=\"{}\"/>", emu(x0), emu(y0)), format!("<a:ext cx=\"{}\" cy=\"{}\"/>", emu(x1 - x0), emu(y1 - y0)));
            let _ = write!(s, "<p:grpSp><p:nvGrpSpPr><p:cNvPr id=\"{id}\" name=\"Group {id}\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm>{off}{ext}<a:chOff x=\"{}\" y=\"{}\"/><a:chExt cx=\"{}\" cy=\"{}\"/></a:xfrm></p:grpSpPr>", emu(x0), emu(y0), emu(x1 - x0), emu(y1 - y0));
            for o in members { self.shape(o, template, &mut s); }
            s.push_str("</p:grpSp>");
            i = j;
        }
        s.push_str("</p:spTree>");
        s
    }
}

fn theme_xml(t: &Theme, name: &str) -> String {
    let c = |i: usize| hex(t.color(i));
    let names = ["dk1", "lt1", "dk2", "lt2", "accent1", "accent2", "accent3", "accent4", "accent5", "accent6"];
    let mut scheme = String::new();
    for (i, n) in names.iter().enumerate() { let _ = write!(scheme, "<a:{n}><a:srgbClr val=\"{}\"/></a:{n}>", c(i)); }
    let _ = write!(scheme, "<a:hlink><a:srgbClr val=\"{}\"/></a:hlink><a:folHlink><a:srgbClr val=\"{}\"/></a:folHlink>", c(4), c(8));
    let ln = |w: u32| format!("<a:ln w=\"{w}\" cap=\"flat\" cmpd=\"sng\" algn=\"ctr\"><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill><a:prstDash val=\"solid\"/><a:miter lim=\"800000\"/></a:ln>");
    format!(
        "<a:theme xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" name=\"{n}\"><a:themeElements><a:clrScheme name=\"{n}\">{scheme}</a:clrScheme>\
<a:fontScheme name=\"{n}\"><a:majorFont><a:latin typeface=\"{h}\"/><a:ea typeface=\"\"/><a:cs typeface=\"\"/></a:majorFont><a:minorFont><a:latin typeface=\"{b}\"/><a:ea typeface=\"\"/><a:cs typeface=\"\"/></a:minorFont></a:fontScheme>\
<a:fmtScheme name=\"BrainBend Studio\"><a:fillStyleLst><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill></a:fillStyleLst>\
<a:lnStyleLst>{l1}{l2}{l3}</a:lnStyleLst><a:effectStyleLst><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle></a:effectStyleLst>\
<a:bgFillStyleLst><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill></a:bgFillStyleLst></a:fmtScheme></a:themeElements><a:objectDefaults/><a:extraClrSchemeLst/></a:theme>",
        n = esc(name), h = esc(&t.heading_font), b = esc(&t.body_font), l1 = ln(9525), l2 = ln(12700), l3 = ln(19050)
    )
}

fn lvl_styles(tag: &str, theme: &Theme, sizes: &[u32], bullet: bool, major: bool) -> String {
    let mut s = format!("<p:{tag}>");
    for (i, sz) in sizes.iter().enumerate() {
        let n = i + 1;
        let mar = if bullet { 228600 + 457200 * i as i64 } else { 457200 * i as i64 };
        let bu = if bullet { "<a:buFont typeface=\"Arial\"/><a:buChar char=\"•\"/>" } else { "<a:buNone/>" };
        let ind = if bullet { " indent=\"-228600\"" } else { "" };
        let _ = write!(s, "<a:lvl{n}pPr marL=\"{mar}\"{ind} algn=\"l\" defTabSz=\"914400\" rtl=\"0\" eaLnBrk=\"1\" latinLnBrk=\"0\" hangingPunct=\"1\">{bu}<a:defRPr sz=\"{sz}\" kern=\"1200\"><a:solidFill><a:schemeClr val=\"tx1\"/></a:solidFill><a:latin typeface=\"{}\"/><a:ea typeface=\"+mn-ea\"/><a:cs typeface=\"+mn-cs\"/></a:defRPr></a:lvl{n}pPr>", if major { "+mj-lt" } else { "+mn-lt" });
    }
    let _ = theme;
    let _ = write!(s, "</p:{tag}>");
    s
}

/// PowerPoint has no layers, so each extra layer becomes a hidden slide right after its slide
/// (the base objects plus the layer's own). Click triggers that show a layer jump to that slide;
/// ones that hide it jump back. Returns the ids of the slides that were added.
fn layers_to_slides(project: &mut Project) -> std::collections::BTreeSet<String> {
    let mut added = std::collections::BTreeSet::new();
    let mut out: Vec<Slide> = Vec::new();
    for slide in std::mem::take(&mut project.slides) {
        if !matches!(slide.kind, SlideKind::Normal) || slide.layers.len() < 2 { out.push(slide); continue; }
        let new_id = |lid: &str| format!("{}::{}", slide.id, lid);
        let jump_for = |action: &Action, from: Option<&str>| -> Option<Action> {
            match action {
                Action::ShowLayer(l) if slide.layers[1..].iter().any(|x| &x.id == l) => Some(Action::JumpToSlide(new_id(l))),
                Action::HideLayer(l) if from == Some(l.as_str()) => Some(Action::JumpToSlide(slide.id.clone())),
                _ => None,
            }
        };
        let rewrite = |from: Option<&str>| -> Vec<Trigger> {
            slide.triggers.iter().cloned().filter_map(|mut t| {
                if matches!(t.action, Action::ShowLayer(_) | Action::HideLayer(_)) {
                    t.action = jump_for(&t.action, from)?;
                }
                Some(t)
            }).collect()
        };
        let mut base = slide.clone();
        base.layers.truncate(1);
        base.triggers = rewrite(None);
        out.push(base);
        for layer in &slide.layers[1..] {
            let mut s = slide.clone();
            s.id = new_id(&layer.id);
            s.name = format!("{} - {}", slide.name, layer.name);
            s.notes = String::new();
            s.auto_advance = false;
            s.fade_ms = 0;
            let mut objects = slide.layers[0].objects.clone();
            objects.extend(layer.objects.iter().cloned());
            s.layers = vec![Layer { id: slide.layers[0].id.clone(), name: slide.layers[0].name.clone(), objects, duration_ms: layer.duration_ms, modal: true, hide_others: false, pause_base: false, group_names: Default::default(), focus_order: vec![] }];
            s.triggers = rewrite(Some(&layer.id));
            added.insert(s.id.clone());
            out.push(s);
        }
    }
    project.slides = out;
    added
}

/// Export `project`. Returns the file's bytes and notes about what could not be carried over.
pub fn export(project: &Project) -> Result<(Vec<u8>, Vec<String>), String> {
    if project.slides.is_empty() { return Err("the project has no slides".into()); }
    let mut project = project.clone();
    project.ensure_layouts();
    let hidden_slides = layers_to_slides(&mut project);
    let (sw, sh) = (project.slides[0].width, project.slides[0].height);
    let mut notes = Notes(vec![]);
    let mut pkg = Package { files: vec![], overrides: vec![], media: BTreeMap::new(), exts: Default::default() };

    // Layouts: the project's, plus a blank one for slides that follow none.
    let mut layouts: Vec<SlideLayout> = project.layouts.clone();
    if project.slides.iter().any(|s| s.layout.as_ref().is_none_or(|l| !layouts.iter().any(|x| &x.id == l))) {
        layouts.push(SlideLayout::new("export-blank", "Blank"));
    }
    let layout_pos = |id: &Option<String>| id.as_ref().and_then(|l| layouts.iter().position(|x| &x.id == l)).unwrap_or(layouts.len() - 1);

    let slide_no: BTreeMap<String, usize> = project.slides.iter().enumerate().map(|(i, s)| (s.id.clone(), i + 1)).collect();
    let any_notes = project.slides.iter().any(|s| !s.notes.trim().is_empty());

    // --- theme ---
    pkg.part("ppt/theme/theme1.xml", "application/vnd.openxmlformats-officedocument.theme+xml", theme_xml(&project.theme, &project.theme.name));

    // --- master ---
    let master_xml;
    let mut master_rels = Rels::default();
    {
        let mut part = Part { pkg: &mut pkg, project: &project, rels: Rels::default(), next_id: 1, notes: &mut notes, slide_no: &slide_no, ph_count: [0; 3], links: BTreeMap::new(), spids: BTreeMap::new() };
        let m = &project.master;
        let mut objs = m.objects.clone();
        let m_pad = sw * 0.06;
        if !objs.iter().any(|o| o.placeholder == Some(PlaceholderKind::Title)) {
            objs.insert(0, brainbend_doc::master::placeholder_object("m-title", PlaceholderKind::Title, &project.theme, m_pad, sh * 0.06, sw - 2.0 * m_pad, sh * 0.16));
        }
        if !objs.iter().any(|o| o.placeholder == Some(PlaceholderKind::Body)) {
            objs.push(brainbend_doc::master::placeholder_object("m-body", PlaceholderKind::Body, &project.theme, m_pad, sh * 0.26, sw - 2.0 * m_pad, sh * 0.62));
        }
        for o in objs.iter_mut() { if o.placeholder.is_some() { o.fill = None; o.gradient = None; o.stroke = None; } }
        let bg = m.background.clone().map(|b| part.background(&b)).unwrap_or_else(|| format!("<p:bg><p:bgPr>{}<a:effectLst/></p:bgPr></p:bg>", solid(project.theme.page())));
        // The master's text placeholder is the standard body placeholder (type body, idx 1).
        let tree = part.tree(&objs, true).replacen("<p:ph type=\"body\" idx=\"10\"/>", "<p:ph type=\"body\" idx=\"1\"/>", 1);
        let lids: String = (0..layouts.len()).map(|i| format!("<p:sldLayoutId id=\"{}\" r:id=\"rId{}\"/>", 2147483649u64 + i as u64, i + 1)).collect();
        master_xml = format!(
            "<p:sldMaster {NS}><p:cSld>{bg}{tree}</p:cSld><p:clrMap bg1=\"lt1\" tx1=\"dk1\" bg2=\"lt2\" tx2=\"dk2\" accent1=\"accent1\" accent2=\"accent2\" accent3=\"accent3\" accent4=\"accent4\" accent5=\"accent5\" accent6=\"accent6\" hlink=\"hlink\" folHlink=\"folHlink\"/><p:sldLayoutIdLst>{lids}</p:sldLayoutIdLst><p:txStyles>{}{}{}</p:txStyles></p:sldMaster>",
            lvl_styles("titleStyle", &project.theme, &[4400], false, true),
            lvl_styles("bodyStyle", &project.theme, &[2800, 2400, 2000, 1800, 1800], true, false),
            lvl_styles("otherStyle", &project.theme, &[1800, 1800], false, false),
        );
        // The master's own relationships: its layouts, then theme and any pictures it uses.
        let extra = std::mem::take(&mut part.rels);
        for (i, _) in layouts.iter().enumerate() { master_rels.items.push((format!("rId{}", i + 1), "slideLayout".into(), format!("../slideLayouts/slideLayout{}.xml", i + 1), false)); }
        let theme_id = format!("rId{}", layouts.len() + 1);
        master_rels.items.push((theme_id, "theme".into(), "../theme/theme1.xml".into(), false));
        for (_, ty, target, ext) in extra.items {
            let id = format!("rId{}", master_rels.items.len() + 1);
            master_rels.items.push((id, ty, target, ext));
        }
    }
    // Picture relationship ids in the master XML were numbered before the layouts were added; renumber.
    let master_xml = renumber_embeds(master_xml, layouts.len() + 1);
    pkg.part("ppt/slideMasters/slideMaster1.xml", "application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml", master_xml);
    pkg.rels("ppt/slideMasters/slideMaster1.xml", &master_rels);

    // --- layouts ---
    for (i, l) in layouts.iter().enumerate() {
        let mut part = Part { pkg: &mut pkg, project: &project, rels: Rels::default(), next_id: 1, notes: &mut notes, slide_no: &slide_no, ph_count: [0; 3], links: BTreeMap::new(), spids: BTreeMap::new() };
        part.rels.add("slideMaster", "../slideMasters/slideMaster1.xml", false);
        let bg = l.background.clone().map(|b| part.background(&b)).unwrap_or_default();
        let mut objs = l.objects.clone();
        for o in objs.iter_mut() { if o.placeholder.is_some() { o.fill = None; o.gradient = None; o.stroke = None; } }
        let tree = part.tree(&objs, true);
        let xml = format!("<p:sldLayout {NS} preserve=\"1\" userDrawn=\"1\"{}><p:cSld name=\"{}\">{bg}{tree}</p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>", if l.show_master { "" } else { " showMasterSp=\"0\"" }, esc(&l.name));
        let rels = std::mem::take(&mut part.rels);
        let name = format!("ppt/slideLayouts/slideLayout{}.xml", i + 1);
        pkg.part(&name, "application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml", xml);
        pkg.rels(&name, &rels);
    }

    // --- slides ---
    let mut slide_ids = String::new();
    let mut pres_rels = Rels::default();
    pres_rels.add("slideMaster", "slideMasters/slideMaster1.xml", false);
    pres_rels.add("theme", "theme/theme1.xml", false);
    let mut notes_master_added = false;
    for (si, slide) in project.slides.iter().enumerate() {
        let n = si + 1;
        let mut part = Part { pkg: &mut pkg, project: &project, rels: Rels::default(), next_id: 1, notes: &mut notes, slide_no: &slide_no, ph_count: [0; 3], links: BTreeMap::new(), spids: BTreeMap::new() };
        part.rels.add("slideLayout", &format!("../slideLayouts/slideLayout{}.xml", layout_pos(&slide.layout) + 1), false);
        for t in &slide.triggers {
            if let (Owner::Object(id), Event::Click) = (&t.owner, &t.event) {
                if t.conditions.is_empty() { part.links.entry(id.clone()).or_insert_with(|| t.action.clone()); continue; }
            }
            part.notes.add("Triggers other than unconditional clicks that jump to a slide or open a web page are not exported.");
        }
        if slide.objects_with_entrance() { part.notes.add("Animations are exported (automatic ones start when the slide opens; click steps become PowerPoint click animations). The Play animation action is not exported."); }
        let objects: Vec<Object> = match &slide.kind {
            SlideKind::Normal => slide.layers[0].objects.clone(),
            SlideKind::Question(q) => { part.notes.add("Question slides are exported as plain text (scoring does not carry over)."); question_objects(q, sw, sh) }
            SlideKind::Results => { part.notes.add("Results slides are exported as a plain text slide."); vec![text_box("Quiz results", sw, sh, 0.35)] }
        };
        let bg = slide.background.clone().map(|b| part.background(&b)).unwrap_or_default();
        let tree = part.tree(&objects, false);
        let timing = anim::timing_xml(&objects, slide.layers[0].duration_ms, &part.spids, sw, sh).unwrap_or_default();
        let transition = if slide.fade_ms > 0 { format!("<p:transition spd=\"{}\"><p:fade/></p:transition>", if slide.fade_ms >= 900 { "slow" } else if slide.fade_ms <= 550 { "fast" } else { "med" }) } else { String::new() };
        if slide.notes.trim().is_empty() == false {
            part.rels.add("notesSlide", &format!("../notesSlides/notesSlide{n}.xml"), false);
        }
        let show = if hidden_slides.contains(&slide.id) { " show=\"0\"" } else { "" };
        let xml = format!("<p:sld {NS}{show}><p:cSld>{bg}{tree}</p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>{transition}{timing}</p:sld>");
        let rels = std::mem::take(&mut part.rels);
        let name = format!("ppt/slides/slide{n}.xml");
        pkg.part(&name, "application/vnd.openxmlformats-officedocument.presentationml.slide+xml", xml);
        pkg.rels(&name, &rels);
        let rid = pres_rels.add("slide", &format!("slides/slide{n}.xml"), false);
        let _ = write!(slide_ids, "<p:sldId id=\"{}\" r:id=\"{rid}\"/>", 255 + n);
        if !slide.notes.trim().is_empty() {
            if !notes_master_added { notes_master_added = true; }
            let body: String = slide.notes.lines().map(|l| format!("<a:p><a:r><a:rPr lang=\"en-US\"/><a:t>{}</a:t></a:r></a:p>", esc(l))).collect();
            let nx = format!("<p:notes {NS}><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"Notes Placeholder\"/><p:cNvSpPr><a:spLocks noGrp=\"1\"/></p:cNvSpPr><p:nvPr><p:ph type=\"body\" idx=\"1\"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/>{body}</p:txBody></p:sp></p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:notes>");
            let mut nr = Rels::default();
            nr.add("notesMaster", "../notesMasters/notesMaster1.xml", false);
            nr.add("slide", &format!("../slides/slide{n}.xml"), false);
            let nn = format!("ppt/notesSlides/notesSlide{n}.xml");
            pkg.part(&nn, "application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml", nx);
            pkg.rels(&nn, &nr);
        }
    }
    if any_notes {
        pres_rels.add("notesMaster", "notesMasters/notesMaster1.xml", false);
        pkg.part("ppt/theme/theme2.xml", "application/vnd.openxmlformats-officedocument.theme+xml", theme_xml(&project.theme, "Notes"));
        let nm = format!("<p:notesMaster {NS}><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"Notes Placeholder\"/><p:cNvSpPr><a:spLocks noGrp=\"1\"/></p:cNvSpPr><p:nvPr><p:ph type=\"body\" sz=\"quarter\" idx=\"1\"/></p:nvPr></p:nvSpPr><p:spPr><a:xfrm><a:off x=\"685800\" y=\"4400550\"/><a:ext cx=\"5486400\" cy=\"3600450\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang=\"en-US\"/><a:t>Click to edit notes</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld><p:clrMap bg1=\"lt1\" tx1=\"dk1\" bg2=\"lt2\" tx2=\"dk2\" accent1=\"accent1\" accent2=\"accent2\" accent3=\"accent3\" accent4=\"accent4\" accent5=\"accent5\" accent6=\"accent6\" hlink=\"hlink\" folHlink=\"folHlink\"/><p:notesStyle><a:lvl1pPr marL=\"0\" algn=\"l\" defTabSz=\"914400\" rtl=\"0\" eaLnBrk=\"1\" latinLnBrk=\"0\" hangingPunct=\"1\"><a:defRPr sz=\"1200\" kern=\"1200\"><a:solidFill><a:schemeClr val=\"tx1\"/></a:solidFill><a:latin typeface=\"+mn-lt\"/><a:ea typeface=\"+mn-ea\"/><a:cs typeface=\"+mn-cs\"/></a:defRPr></a:lvl1pPr></p:notesStyle></p:notesMaster>");
        let mut r = Rels::default();
        r.add("theme", "../theme/theme2.xml", false);
        pkg.part("ppt/notesMasters/notesMaster1.xml", "application/vnd.openxmlformats-officedocument.presentationml.notesMaster+xml", nm);
        pkg.rels("ppt/notesMasters/notesMaster1.xml", &r);
    }

    // --- presentation ---
    let notes_master_id = if any_notes { let rid = pres_rels.items.iter().find(|i| i.1 == "notesMaster").map(|i| i.0.clone()).unwrap_or_default(); format!("<p:notesMasterIdLst><p:notesMasterId r:id=\"{rid}\"/></p:notesMasterIdLst>") } else { String::new() };
    let master_rid = pres_rels.items[0].0.clone();
    let sections = if project.scenes.len() > 1 {
        let mut s = String::from("<p:extLst><p:ext uri=\"{521415D9-36F7-43E2-AB2F-B90AF26B5E84}\"><p14:sectionLst xmlns:p14=\"http://schemas.microsoft.com/office/powerpoint/2010/main\">");
        for (i, sc) in project.scenes.iter().enumerate() {
            let ids: String = project.slides.iter().enumerate().filter(|(_, sl)| sl.scene == sc.id).map(|(k, _)| format!("<p14:sldId id=\"{}\"/>", 256 + k)).collect();
            if ids.is_empty() { continue; }
            let _ = write!(s, "<p14:section name=\"{}\" id=\"{{{:08X}-0000-4000-8000-{:012X}}}\"><p14:sldIdLst>{ids}</p14:sldIdLst></p14:section>", esc(&sc.name), 0x5A000000u32 + i as u32, i);
        }
        s.push_str("</p14:sectionLst></p:ext></p:extLst>");
        s
    } else { String::new() };
    let pres = format!(
        "<p:presentation {NS} saveSubsetFonts=\"1\"><p:sldMasterIdLst><p:sldMasterId id=\"2147483648\" r:id=\"{master_rid}\"/></p:sldMasterIdLst>{notes_master_id}<p:sldIdLst>{slide_ids}</p:sldIdLst><p:sldSz cx=\"{}\" cy=\"{}\"/><p:notesSz cx=\"6858000\" cy=\"9144000\"/>{sections}</p:presentation>",
        emu(sw), emu(sh)
    );
    pkg.part("ppt/presentation.xml", "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml", pres);
    pkg.part("ppt/presProps.xml", "application/vnd.openxmlformats-officedocument.presentationml.presProps+xml", format!("<p:presentationPr {NS}/>"));
    pkg.part("ppt/viewProps.xml", "application/vnd.openxmlformats-officedocument.presentationml.viewProps+xml", format!("<p:viewPr {NS}><p:normalViewPr><p:restoredLeft sz=\"15620\"/><p:restoredTop sz=\"94660\"/></p:normalViewPr><p:gridSpacing cx=\"76200\" cy=\"76200\"/></p:viewPr>"));
    pkg.part("ppt/tableStyles.xml", "application/vnd.openxmlformats-officedocument.presentationml.tableStyles+xml", "<a:tblStyleLst xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" def=\"{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}\"/>".to_string());
    pres_rels.add("presProps", "presProps.xml", false);
    pres_rels.add("viewProps", "viewProps.xml", false);
    pres_rels.add("tableStyles", "tableStyles.xml", false);
    pkg.files.retain(|f| f.0 != "ppt/_rels/presentation.xml.rels");
    pkg.rels("ppt/presentation.xml", &pres_rels);
    let mut root = Rels::default();
    root.add("officeDocument", "ppt/presentation.xml", false);
    pkg.files.push(("_rels/.rels".into(), root.xml().into_bytes()));

    // --- content types and zip ---
    let mut ct = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/>");
    for ext in &pkg.exts { let _ = write!(ct, "<Default Extension=\"{ext}\" ContentType=\"{}\"/>", content_type_for(ext)); }
    for (name, ty) in &pkg.overrides { let _ = write!(ct, "<Override PartName=\"{name}\" ContentType=\"{ty}\"/>"); }
    ct.push_str("</Types>");
    let mut z = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    z.start_file("[Content_Types].xml", opts).map_err(|e| e.to_string())?;
    z.write_all(ct.as_bytes()).map_err(|e| e.to_string())?;
    for (name, bytes) in &pkg.files {
        z.start_file(name, opts).map_err(|e| e.to_string())?;
        z.write_all(bytes).map_err(|e| e.to_string())?;
    }
    let out = z.finish().map_err(|e| e.to_string())?.into_inner();
    if !hidden_slides.is_empty() { notes.add("Extra layers are exported as hidden slides after their slide; clicking a Show layer or Hide layer trigger jumps between them. Layer timelines and other layer rules do not carry over."); }
    if !project.variables.iter().all(|v| v.built_in) { notes.add("Variables are not exported."); }
    Ok((out, notes.0))
}

/// Picture relationships written while building the master were numbered from rId1; shift them past the layouts and theme.
fn renumber_embeds(xml: String, shift: usize) -> String {
    let mut out = String::with_capacity(xml.len());
    let mut rest = xml.as_str();
    while let Some(i) = rest.find("r:embed=\"rId") {
        out.push_str(&rest[..i + 12]);
        rest = &rest[i + 12..];
        let end = rest.find('"').unwrap_or(rest.len());
        let n: usize = rest[..end].parse().unwrap_or(0);
        let _ = write!(out, "{}", n + shift);
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

fn text_box(text: &str, sw: f32, sh: f32, y: f32) -> Object {
    let mut o = Object::new("export-text", "Text", Kind::Text, sw * 0.08, sh * y, sw * 0.84, sh * 0.2);
    o.text = text.into();
    o.style.size = 64.0;
    o
}

fn question_objects(q: &Question, sw: f32, sh: f32) -> Vec<Object> {
    let mut title = text_box(&q.prompt, sw, sh, 0.08);
    title.id = "export-q".into();
    title.style.bold = true;
    let mut lines: Vec<String> = vec![];
    match &q.style {
        QuestionStyle::Choice => for c in &q.choices { lines.push(format!("{} {}", if c.correct { "☑" } else { "☐" }, c.text)); },
        QuestionStyle::TextEntry { accepted, .. } => lines.push(format!("Accepted answers: {}", accepted.join(", "))),
        QuestionStyle::Matching { pairs } => for (a, b) in pairs { lines.push(format!("{a} → {b}")); },
        QuestionStyle::Sequence { items } => for (i, it) in items.iter().enumerate() { lines.push(format!("{}. {it}", i + 1)); },
        QuestionStyle::FillBlanks { text, .. } => { lines.push(brainbend_doc::passage::with_gaps(text)); lines.push(format!("Answers: {}", brainbend_doc::passage::blanks(text).iter().map(|a| a.join(" / ")).collect::<Vec<_>>().join(", "))); }
        QuestionStyle::DragWords { text, distractors } => {
            lines.push(brainbend_doc::passage::with_gaps(text));
            let mut bank: Vec<String> = brainbend_doc::passage::blanks(text).into_iter().map(|a| a[0].clone()).collect();
            bank.extend(distractors.iter().cloned());
            lines.push(format!("Word bank: {}", bank.join(", ")));
        }
        QuestionStyle::MarkWords { text } => lines.push(format!("Mark the words in *stars*: {text}")),
        QuestionStyle::Numeric { answer, tolerance } => lines.push(format!("Answer: {answer}{}", if *tolerance > 0.0 { format!(" (within {tolerance})") } else { String::new() })),
        QuestionStyle::Likert { statements, scale, .. } => { lines.push(format!("Scale: {}", scale.join(" / "))); for st in statements { lines.push(st.clone()); } }
        QuestionStyle::Reflection { .. } => lines.push("(Free-text answer)".into()),
        QuestionStyle::Dictation { text, .. } => lines.push(format!("Audio question. Transcript: {text}")),
        QuestionStyle::Dropdowns { text } => lines.push(brainbend_doc::passage::with_gaps(text)),
        QuestionStyle::LabelPicture { zones, distractors, .. } => {
            lines.push(format!("Labels: {}", zones.iter().map(|z| z.label.clone()).chain(distractors.iter().cloned()).collect::<Vec<_>>().join(", ")));
        }
        QuestionStyle::Hotspot { alt, .. } => lines.push(format!("Click the correct spot on the picture. {alt}")),
        QuestionStyle::Categorize { categories } => for (name, items) in categories { lines.push(format!("{name}: {}", items.join(", "))); },
    }
    let mut list = text_box(&lines.join("\n"), sw, sh, 0.32);
    list.id = "export-a".into();
    list.h = sh * 0.6;
    list.style.size = 44.0;
    list.style.valign = VAlign::Top;
    vec![title, list]
}

trait HasEntrance {
    fn objects_with_entrance(&self) -> bool;
}

impl HasEntrance for Slide {
    fn objects_with_entrance(&self) -> bool {
        self.layers.iter().flat_map(|l| l.objects.iter()).any(|o| o.entrance.is_some() || o.exit.is_some() || o.emphasis.is_some() || o.motion.is_some())
    }
}
