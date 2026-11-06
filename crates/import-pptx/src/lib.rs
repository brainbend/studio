//! Importer for PowerPoint `.pptx` presentations.
//!
//! Written from the public ECMA-376 (Office Open XML) specification. It carries over slide size,
//! the theme, the slide master and layouts (with backgrounds and decorative shapes), slides with
//! text, shapes, pictures, tables, media, notes, sections and links, resolving the
//! slide → layout → master inheritance of positions, text styles and colors. What it cannot
//! reproduce is listed in the returned project's `notes`.
mod color;
mod geom;
mod shapes;
mod text;

use color::*;
use roxmltree::{Document, Node};
use std::collections::{BTreeMap, HashMap};
use std::io::{Read, Seek};
use std::rc::Rc;
use brainbend_doc::*;
use text::*;

const MAX_ENTRY_BYTES: u64 = 256 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const TARGET_WIDTH: f32 = 1920.0;

/// All parts of the package, by normalized path (no leading slash).
pub(crate) struct Pkg {
    parts: HashMap<String, Vec<u8>>,
}

impl Pkg {
    pub fn text(&self, path: &str) -> Option<String> {
        let b = self.parts.get(path)?;
        let s = String::from_utf8_lossy(b);
        Some(s.trim_start_matches('\u{feff}').to_string())
    }
    pub fn bytes(&self, path: &str) -> Option<&Vec<u8>> {
        self.parts.get(path)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Rel {
    pub kind: String,
    /// Absolute part path (internal) or the URL (external).
    pub target: String,
    pub external: bool,
}

pub(crate) type Rels = HashMap<String, Rel>;

fn resolve_path(base_part: &str, target: &str) -> String {
    let mut segs: Vec<&str> = if let Some(t) = target.strip_prefix('/') {
        return t.to_string();
    } else {
        base_part.rsplit_once('/').map(|(d, _)| d.split('/').collect()).unwrap_or_default()
    };
    for s in target.split('/') {
        match s {
            "." | "" => {}
            ".." => { segs.pop(); }
            s => segs.push(s),
        }
    }
    segs.join("/")
}

pub(crate) fn rels_of(pkg: &Pkg, part: &str) -> Rels {
    let (dir, file) = part.rsplit_once('/').unwrap_or(("", part));
    let path = if dir.is_empty() { format!("_rels/{file}.rels") } else { format!("{dir}/_rels/{file}.rels") };
    let mut out = HashMap::new();
    let Some(xml) = pkg.text(&path) else { return out };
    let Ok(doc) = Document::parse(&xml) else { return out };
    for r in doc.descendants().filter(|n| n.is_element() && n.tag_name().name() == "Relationship") {
        let (Some(id), Some(target)) = (r.attribute("Id"), r.attribute("Target")) else { continue };
        let kind = r.attribute("Type").unwrap_or("").rsplit('/').next().unwrap_or("").to_string();
        let external = r.attribute("TargetMode") == Some("External");
        out.insert(id.to_string(), Rel { kind, target: if external { target.to_string() } else { resolve_path(part, target) }, external });
    }
    out
}

fn read_package<R: Read + Seek>(reader: R) -> Result<Pkg, String> {
    let mut z = zip::ZipArchive::new(reader).map_err(|e| format!("not a PowerPoint file: {e}"))?;
    let mut parts = HashMap::new();
    let mut total = 0u64;
    for i in 0..z.len() {
        let mut f = z.by_index(i).map_err(|e| e.to_string())?;
        if f.is_dir() { continue; }
        if f.size() > MAX_ENTRY_BYTES { return Err(format!("{} is too large", f.name().unwrap_or_default())); }
        total += f.size();
        if total > MAX_TOTAL_BYTES { return Err("the file expands to more than 2 GB".into()); }
        let name = f.name().map_err(|e| e.to_string())?.trim_start_matches('/').to_string();
        let mut buf = Vec::new();
        (&mut f).take(MAX_ENTRY_BYTES + 1).read_to_end(&mut buf).map_err(|e| e.to_string())?;
        parts.insert(name, buf);
    }
    Ok(Pkg { parts })
}

/// Collects import notes, merging repeats ("... (x3)").
#[derive(Default)]
pub(crate) struct Report {
    counts: BTreeMap<String, usize>,
    order: Vec<String>,
    fonts: std::collections::BTreeSet<String>,
}

impl Report {
    pub fn add(&mut self, msg: impl Into<String>) {
        let m = msg.into();
        let c = self.counts.entry(m.clone()).or_insert(0);
        if *c == 0 { self.order.push(m); }
        *c += 1;
    }
    /// A font the file uses that is not available here.
    pub fn font(&mut self, name: &str) {
        self.fonts.insert(name.to_string());
    }
    fn finish(mut self) -> Vec<String> {
        let fonts: Vec<String> = std::mem::take(&mut self.fonts).into_iter().collect();
        let mut out: Vec<String> = self.order.into_iter().map(|m| { let n = self.counts[&m]; if n > 1 { format!("{m} (×{n})") } else { m } }).collect();
        if !fonts.is_empty() {
            out.push(format!("Fonts not available here ({}) are shown in Open Sans. Install them, or add them from Google Fonts, to see them as designed; the published course uses the learner's own copy if they have one.", fonts.join(", ")));
        }
        out
    }
}

/// Media shared by all parts: each package picture is stored once.
#[derive(Default)]
pub(crate) struct Assets {
    pub map: BTreeMap<String, Asset>,
    by_key: HashMap<String, String>,
}

impl Assets {
    pub fn add(&mut self, cache_key: String, name: &str, mime: &str, bytes: impl FnOnce() -> Vec<u8>) -> String {
        if let Some(k) = self.by_key.get(&cache_key) { return k.clone(); }
        let key = format!("asset-{}", self.map.len() + 1);
        self.map.insert(key.clone(), Asset { name: name.to_string(), mime: mime.to_string(), bytes: bytes(), file: None });
        self.by_key.insert(cache_key, key.clone());
        key
    }
}

/// Where a shape's click should go.
#[derive(Clone, Debug)]
pub(crate) enum Link {
    /// Another slide, by its part path.
    Slide(String),
    Url(String),
    Next,
    Previous,
    First,
    Last,
}

/// Placeholder information from a layout or master, kept so slides can inherit from it.
#[derive(Clone, Debug, Default)]
pub(crate) struct PhInfo {
    /// `title`, `ctrTitle`, `subTitle`, `body`, `dt`, `ftr`, `sldNum`, `obj`, `pic`, ...
    pub ty: String,
    pub idx: Option<u32>,
    pub xfrm: Option<shapes::Xfrm>,
    pub lst: Lst,
    pub body: shapes::BodyPr,
    pub fill: Option<FillSpec>,
    pub line: Option<FillSpec>,
    pub line_w: Option<f32>,
}

/// Everything a slide (or layout) inherits from.
pub(crate) struct Chain {
    pub theme: Rc<ThemeData>,
    pub clr_map: Rc<HashMap<String, String>>,
    pub master_phs: Vec<PhInfo>,
    pub layout_phs: Vec<PhInfo>,
    pub tx_title: Lst,
    pub tx_body: Lst,
    pub default_text: Lst,
}

impl Chain {
    pub fn cx(&self) -> ColorCtx<'_> {
        ColorCtx { scheme: &self.theme.scheme, clr_map: &self.clr_map, ph: None }
    }
    pub fn color(&self, c: &ColorSpec, ph: Option<Rgba>) -> Rgba {
        c.resolve(&ColorCtx { scheme: &self.theme.scheme, clr_map: &self.clr_map, ph })
    }
}

fn ph_family(ty: &str) -> &'static str {
    match ty {
        "title" | "ctrTitle" => "title",
        "subTitle" => "subTitle",
        "dt" => "dt",
        "ftr" => "ftr",
        "sldNum" => "sldNum",
        _ => "body",
    }
}

impl Chain {
    /// Find the layout and master placeholders a slide placeholder inherits from.
    pub fn find_ph(&self, ty: &str, idx: Option<u32>) -> (Option<&PhInfo>, Option<&PhInfo>) {
        let fam = ph_family(ty);
        let in_layout = self.layout_phs.iter().find(|p| idx.is_some() && p.idx == idx && ph_family(&p.ty) == fam).or_else(|| self.layout_phs.iter().find(|p| ph_family(&p.ty) == fam && (idx.is_none() || p.idx == idx || fam != "body")));
        let want = in_layout.map(|p| ph_family(&p.ty)).unwrap_or(fam);
        let master = self.master_phs.iter().find(|p| ph_family(&p.ty) == want || (want == "subTitle" && ph_family(&p.ty) == "body"));
        (in_layout, master)
    }
}

pub(crate) struct Cx<'a> {
    pub pkg: &'a Pkg,
    pub rels: Rels,
    pub part: String,
    /// Slide units per EMU, and per point.
    pub k: f32,
    pub fk: f32,
    pub ch: &'a Chain,
    pub assets: &'a mut Assets,
    pub report: &'a mut Report,
    pub links: Vec<(String, Link)>,
    /// Shape ids in the source (what animations target) -> ids of the objects made from them.
    pub sp_ids: HashMap<String, Vec<String>>,
    pub id_prefix: String,
    pub counter: usize,
    /// Converting a layout or master (placeholders become templates).
    pub template: bool,
}

impl Cx<'_> {
    pub fn next_id(&mut self) -> String {
        self.counter += 1;
        format!("{}-o{}", self.id_prefix, self.counter)
    }
}

fn presentation_parts(pkg: &Pkg) -> Result<(String, Rels), String> {
    let root_rels = rels_of(pkg, "_rels/.rels");
    let _ = root_rels;
    // The root relationships live at `_rels/.rels` (no owning part), so read them directly.
    let xml = pkg.text("_rels/.rels").ok_or("missing _rels/.rels")?;
    let doc = Document::parse(&xml).map_err(|e| e.to_string())?;
    let target = doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "Relationship")
        .find(|r| r.attribute("Type").is_some_and(|t| t.ends_with("/officeDocument")))
        .and_then(|r| r.attribute("Target"))
        .ok_or("this is not a PowerPoint presentation")?;
    let part = resolve_path("x", target);
    let rels = rels_of(pkg, &part);
    Ok((part, rels))
}

fn rgba_of(c: Option<Rgba>) -> Rgba {
    c.unwrap_or(Rgba(0, 0, 0, 255))
}

pub(crate) fn fill_to_background(f: &FillSpec, ch: &Chain, cx: &mut Cx) -> Option<Background> {
    match f {
        FillSpec::NoFill => None,
        FillSpec::Solid(c) | FillSpec::Pattern(c) => Some(Background::Solid(ch.color(c, None))),
        FillSpec::Grad { angle, stops } => {
            Some(Background::Gradient { angle: *angle, stops: stops.iter().map(|(p, c)| (*p, ch.color(c, None))).collect(), kind: Default::default(), geom: Default::default() })
        }
        FillSpec::Blip { embed, crop } => shapes::picture_asset(cx, embed, *crop).map(|asset| Background::Image { asset }),
    }
}

/// `p:cSld/p:bg` of a slide, layout or master.
fn read_background(root: Node, cx: &mut Cx, ch_theme: &ThemeData, ch: &Chain) -> Option<Background> {
    let bg = root.descendants().find(|n| n.is_element() && n.tag_name().name() == "bg")?;
    if let Some(pr) = child(bg, "bgPr") {
        return parse_fill(pr).and_then(|f| fill_to_background(&f, ch, cx));
    }
    let r = child(bg, "bgRef")?;
    let idx = r.attribute("idx").and_then(|v| v.parse::<usize>().ok()).unwrap_or(0);
    let ph = first_color(r).map(|c| ch.color(&c, None));
    let spec = if idx >= 1001 { ch_theme.bg_fills.get(idx - 1001) } else { ch_theme.fills.get(idx.saturating_sub(1)) }?;
    let ctx = ColorCtx { scheme: &ch.theme.scheme, clr_map: &ch.clr_map, ph };
    match spec {
        FillSpec::Solid(c) => Some(Background::Solid(c.resolve(&ctx))),
        FillSpec::Grad { angle, stops } => Some(Background::Gradient { angle: *angle, stops: stops.iter().map(|(p, c)| (*p, c.resolve(&ctx))).collect(), kind: Default::default(), geom: Default::default() }),
        _ => None,
    }
}

struct MasterOut {
    chain_theme: Rc<ThemeData>,
    clr_map: Rc<HashMap<String, String>>,
    master_phs: Vec<PhInfo>,
    tx_title: Lst,
    tx_body: Lst,
    background: Option<Background>,
    objects: Vec<Object>,
    /// (layout part path, layout)
    layouts: Vec<(String, SlideLayout, Vec<PhInfo>)>,
}

pub fn import<R: Read + Seek>(reader: R, name: &str) -> Result<Project, String> {
    let pkg = read_package(reader)?;
    let (pres_part, pres_rels) = presentation_parts(&pkg)?;
    let pres_xml = pkg.text(&pres_part).ok_or("missing presentation part")?;
    let pres = Document::parse(&pres_xml).map_err(|e| format!("presentation.xml: {e}"))?;
    let root = pres.root_element();

    let sz = root.descendants().find(|n| n.is_element() && n.tag_name().name() == "sldSz").ok_or("missing slide size")?;
    let (cx_emu, cy_emu) = (attr_f(sz, "cx").unwrap_or(12192000.0).max(1.0), attr_f(sz, "cy").unwrap_or(6858000.0).max(1.0));
    let k = TARGET_WIDTH / cx_emu;
    let (width, height) = (TARGET_WIDTH, (cy_emu * k).round());

    let mut project = Project::new(name);
    let mut report = Report::default();
    let mut assets = Assets::default();

    // --- masters, their themes and layouts ------------------------------------------------------
    let master_parts: Vec<String> = root
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "sldMasterId")
        .filter_map(|n| n.attributes().find(|a| a.name() == "id" && a.namespace().is_some()).map(|a| a.value().to_string()))
        .filter_map(|id| pres_rels.get(&id).map(|r| r.target.clone()))
        .collect();
    if master_parts.is_empty() { return Err("the presentation has no slide master".into()); }

    // The first theme gives default text for the presentation.
    let mut masters: Vec<MasterOut> = vec![];
    let mut default_text: Lst = empty_lst();
    for (mi, mpart) in master_parts.iter().enumerate() {
        let mrels = rels_of(&pkg, mpart);
        let theme_part = mrels.values().find(|r| r.kind == "theme").map(|r| r.target.clone());
        let theme = theme_part.and_then(|p| pkg.text(&p)).and_then(|x| Document::parse(&x).ok().map(|d| parse_theme(d.root_element()))).unwrap_or_default();
        let theme = Rc::new(theme);
        if mi == 0 {
            default_text = parse_lst(root.descendants().find(|n| n.is_element() && n.tag_name().name() == "defaultTextStyle"), &theme);
        }
        let mxml = pkg.text(mpart).ok_or("missing slide master")?;
        let mdoc = Document::parse(&mxml).map_err(|e| format!("slide master: {e}"))?;
        let mroot = mdoc.root_element();
        let clr_map: HashMap<String, String> = child(mroot, "clrMap").map(|n| n.attributes().map(|a| (a.name().to_string(), a.value().to_string())).collect()).unwrap_or_default();
        let clr_map = Rc::new(clr_map);
        let tx = child(mroot, "txStyles");
        let tx_title = parse_lst(tx.and_then(|t| child(t, "titleStyle")), &theme);
        let tx_body = parse_lst(tx.and_then(|t| child(t, "bodyStyle")), &theme);
        let mk_chain = |master_phs: Vec<PhInfo>| Chain { theme: theme.clone(), clr_map: clr_map.clone(), master_phs, layout_phs: vec![], tx_title: tx_title.clone(), tx_body: tx_body.clone(), default_text: default_text.clone() };
        let tree = mroot.descendants().find(|n| n.is_element() && n.tag_name().name() == "spTree");
        let master_phs = tree.map(|t| shapes::collect_placeholders(t, &theme)).unwrap_or_default();
        let ch = mk_chain(master_phs.clone());
        let mut cx = Cx { pkg: &pkg, rels: mrels.clone(), part: mpart.clone(), k, fk: 12700.0 * k, ch: &ch, assets: &mut assets, report: &mut report, links: vec![], sp_ids: Default::default(), id_prefix: format!("m{mi}"), counter: 0, template: true };
        let background = read_background(mroot, &mut cx, &theme, &ch);
        let mut objects = vec![];
        if let Some(t) = tree { shapes::convert_children(&mut cx, t, shapes::Tf::base(k), &mut objects); }
        // Layouts of this master, in the master's order.
        let mut layouts = vec![];
        let ids: Vec<String> = child(mroot, "sldLayoutIdLst")
            .map(|l| l.children().filter(|c| c.is_element()).filter_map(|c| c.attributes().find(|a| a.name() == "id" && a.namespace().is_some()).map(|a| a.value().to_string())).collect())
            .unwrap_or_default();
        for rid in ids {
            let Some(lpart) = mrels.get(&rid).map(|r| r.target.clone()) else { continue };
            let Some(lxml) = pkg.text(&lpart) else { continue };
            let Ok(ldoc) = Document::parse(&lxml) else { continue };
            let lroot = ldoc.root_element();
            let lrels = rels_of(&pkg, &lpart);
            let ltree = lroot.descendants().find(|n| n.is_element() && n.tag_name().name() == "spTree");
            let layout_phs = ltree.map(|t| shapes::collect_placeholders(t, &theme)).unwrap_or_default();
            let lch = mk_chain(master_phs.clone());
            let mut lcx = Cx { pkg: &pkg, rels: lrels, part: lpart.clone(), k, fk: 12700.0 * k, ch: &lch, assets: &mut assets, report: &mut report, links: vec![], sp_ids: Default::default(), id_prefix: format!("l{mi}-{}", layouts.len()), counter: 0, template: true };
            let background = read_background(lroot, &mut lcx, &theme, &lch);
            let mut lobjs = vec![];
            if let Some(t) = ltree { shapes::convert_children(&mut lcx, t, shapes::Tf::base(k), &mut lobjs); }
            let lname = child(lroot, "cSld").and_then(|c| c.attribute("name")).unwrap_or("Layout").to_string();
            let show_master = lroot.attribute("showMasterSp") != Some("0");
            let id = format!("pptx-layout-{}-{}", mi, layouts.len() + 1);
            layouts.push((lpart, SlideLayout { id, name: lname, background, objects: lobjs, show_master }, layout_phs));
        }
        masters.push(MasterOut { chain_theme: theme, clr_map, master_phs, tx_title, tx_body, background, objects, layouts });
    }

    // Theme and master of the first slide master; further masters are baked into their layouts.
    {
        let m = &masters[0];
        let resolve = |n: &str| {
            let ctx = ColorCtx { scheme: &m.chain_theme.scheme, clr_map: &m.clr_map, ph: None };
            ColorSpec { base: Base::Scheme(n.into()), mods: vec![] }.resolve(&ctx)
        };
        let t = &m.chain_theme;
        project.theme = Theme {
            name: t.name.clone(),
            colors: vec![resolve("tx1"), resolve("bg1"), resolve("tx2"), resolve("bg2"), resolve("accent1"), resolve("accent2"), resolve("accent3"), resolve("accent4"), resolve("accent5"), resolve("accent6")],
            heading_font: t.major.clone(),
            body_font: t.minor.clone(),
        };
        project.master = Master { background: m.background.clone(), objects: m.objects.clone() };
    }
    let multi = masters.len() > 1;
    let mut layout_ids: HashMap<String, (usize, usize)> = HashMap::new(); // layout part -> (master, layout index)
    for (mi, m) in masters.iter().enumerate() {
        for (li, (part, l, _)) in m.layouts.iter().enumerate() {
            layout_ids.insert(part.clone(), (mi, li));
            let mut l = l.clone();
            if mi > 0 {
                // Another master's look is copied into the layout, since a project has one master.
                let mut objs: Vec<Object> = if l.show_master { m.objects.iter().filter(|o| o.placeholder.is_none()).cloned().collect() } else { vec![] };
                objs.extend(l.objects.drain(..));
                l.objects = objs;
                if l.background.is_none() { l.background = m.background.clone(); }
                l.show_master = false;
            }
            if multi { l.name = format!("{} (master {})", l.name, mi + 1); }
            project.layouts.push(l);
        }
    }
    if masters.len() > 1 { report.add(format!("The file has {} slide masters; the first became the project's master and the others' shapes were copied into their layouts.", masters.len())); }

    // --- slides --------------------------------------------------------------------------------
    let slide_entries: Vec<(String, String)> = root
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "sldId")
        .filter_map(|n| {
            let id = n.attribute("id")?.to_string();
            let rid = n.attributes().find(|a| a.name() == "id" && a.namespace().is_some())?.value().to_string();
            Some((id, pres_rels.get(&rid)?.target.clone()))
        })
        .collect();
    let mut slide_id_of_part: HashMap<String, String> = HashMap::new();
    let mut converted: Vec<(String, String, Slide, Vec<(String, Link)>)> = vec![]; // (sldId, part, slide, links)
    let mut hidden = 0;
    for (sld_id, part) in &slide_entries {
        let Some(xml) = pkg.text(part) else { continue };
        let Ok(doc) = Document::parse(&xml) else { report.add(format!("{part} could not be read")); continue };
        let sroot = doc.root_element();
        if sroot.attribute("show") == Some("0") { hidden += 1; continue; }
        let rels = rels_of(&pkg, part);
        let layout_part = rels.values().find(|r| r.kind == "slideLayout").map(|r| r.target.clone());
        let (mi, li) = layout_part.as_ref().and_then(|p| layout_ids.get(p).copied()).unwrap_or((0, 0));
        let m = &masters[mi];
        let layout_phs = m.layouts.get(li).map(|l| l.2.clone()).unwrap_or_default();
        let ch = Chain { theme: m.chain_theme.clone(), clr_map: m.clr_map.clone(), master_phs: m.master_phs.clone(), layout_phs, tx_title: m.tx_title.clone(), tx_body: m.tx_body.clone(), default_text: default_text.clone() };
        let slide_id = format!("slide-{}", project.slides.len() + converted.len() + 1);
        let mut cx = Cx { pkg: &pkg, rels, part: part.clone(), k, fk: 12700.0 * k, ch: &ch, assets: &mut assets, report: &mut report, links: vec![], sp_ids: Default::default(), id_prefix: slide_id.clone(), counter: 0, template: false };
        let background = read_background(sroot, &mut cx, &m.chain_theme, &ch);
        let mut objects = vec![];
        if let Some(t) = sroot.descendants().find(|n| n.is_element() && n.tag_name().name() == "spTree") {
            shapes::convert_children(&mut cx, t, shapes::Tf::base(k), &mut objects);
        }
        let notes = shapes::read_notes(&cx);
        let title = objects.iter().find(|o| o.placeholder == Some(PlaceholderKind::Title)).map(|o| o.text.lines().next().unwrap_or("").trim().to_string()).filter(|t| !t.is_empty());
        let fade_ms = shapes::read_transition(sroot, &mut cx);
        shapes::read_animations(sroot, &mut cx, &mut objects, (width, height));
        let mut slide = Slide::new(&slide_id, &title.map(|t| t.chars().take(60).collect::<String>()).unwrap_or_else(|| format!("Slide {}", converted.len() + 1)), width, height);
        slide.layers[0].objects = objects;
        slide.background = background;
        slide.layout = m.layouts.get(li).map(|l| l.1.id.clone());
        slide.notes = notes;
        slide.fade_ms = fade_ms;
        let links = std::mem::take(&mut cx.links);
        slide_id_of_part.insert(part.clone(), slide_id.clone());
        converted.push((sld_id.clone(), part.clone(), slide, links));
    }
    if hidden > 0 { report.add(format!("{hidden} hidden slide(s) were skipped.")); }

    // Links become triggers now that every slide has an id.
    let ids_in_order: Vec<String> = converted.iter().map(|c| c.2.id.clone()).collect();
    for (_, _, slide, links) in converted.iter_mut() {
        for (n, (obj, link)) in links.iter().enumerate() {
            let action = match link {
                Link::Slide(p) => match slide_id_of_part.get(p) { Some(id) => Action::JumpToSlide(id.clone()), None => continue },
                Link::Url(u) => Action::RunJavaScript(format!("window.open({}, '_blank', 'noopener');", serde_json_str(u))),
                Link::Next => Action::NextSlide,
                Link::Previous => Action::PreviousSlide,
                Link::First => Action::JumpToSlide(ids_in_order[0].clone()),
                Link::Last => Action::JumpToSlide(ids_in_order[ids_in_order.len() - 1].clone()),
            };
            slide.triggers.push(Trigger { id: format!("{}-link{}", slide.id, n + 1), owner: Owner::Object(obj.clone()), event: Event::Click, action, conditions: vec![] });
        }
    }

    // Sections become scenes.
    let sections: Vec<(String, Vec<String>)> = root
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "section")
        .map(|s| (s.attribute("name").unwrap_or("Section").to_string(), s.descendants().filter(|n| n.is_element() && n.tag_name().name() == "sldId").filter_map(|n| n.attribute("id").map(String::from)).collect()))
        .collect();
    if sections.is_empty() {
        project.scenes.push(Scene { id: "scene-1".into(), name: "Scene 1".into(), bank: None });
    } else {
        for (i, (nm, _)) in sections.iter().enumerate() { project.scenes.push(Scene { id: format!("scene-{}", i + 1), name: nm.clone(), bank: None }); }
    }
    for (sld_id, _, mut slide, _) in converted {
        let si = sections.iter().position(|(_, ids)| ids.contains(&sld_id)).unwrap_or(0);
        slide.scene = project.scenes[si].id.clone();
        project.slides.push(slide);
    }
    if project.slides.is_empty() { return Err("the presentation has no visible slides".into()); }
    project.assets = assets.map;
    project.sanitize();
    project.notes = report.finish();
    let _ = rgba_of(None);
    Ok(project)
}

/// A JSON/JS string literal for `s`.
fn serde_json_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '<' => out.push_str("\\u003c"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
